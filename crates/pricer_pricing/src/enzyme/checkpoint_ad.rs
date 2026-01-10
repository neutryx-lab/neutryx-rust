//! Checkpointing integration for path-dependent AD.
//!
//! This module provides the integration between Enzyme automatic differentiation
//! and the checkpointing infrastructure for memory-efficient Greeks computation
//! on path-dependent options (Asian, Barrier, etc.).
//!
//! # Memory Efficiency
//!
//! For path-dependent options with N steps, computing Greeks via reverse-mode AD
//! would normally require O(N) memory to store all intermediate values.
//! Checkpointing reduces this to O(√N) by:
//!
//! 1. Saving state at strategic checkpoints during forward pass
//! 2. During reverse pass, restoring from nearest checkpoint and recomputing
//!
//! # Usage
//!
//! ```rust,ignore
//! use pricer_pricing::enzyme::checkpoint_ad::{CheckpointedAD, CheckpointADConfig};
//! use pricer_pricing::checkpoint::CheckpointStrategy;
//!
//! let config = CheckpointADConfig::new(252, CheckpointStrategy::binomial_optimal(252, 16));
//! let mut ad = CheckpointedAD::new(config);
//!
//! // Forward pass with checkpointing
//! for step in 0..252 {
//!     if ad.should_checkpoint(step) {
//!         ad.save_forward_state(step, &state);
//!     }
//!     // ... simulation step with AD ...
//! }
//!
//! // Reverse pass
//! let greeks = ad.compute_reverse_pass();
//! ```

use num_traits::Float;

use crate::checkpoint::{CheckpointManager, CheckpointStrategy};

use super::loops::AdjointAccumulator;
use super::reverse::ReverseAD;

/// Configuration for checkpointed AD.
#[derive(Clone, Debug)]
pub struct CheckpointADConfig {
    /// Total number of simulation steps.
    n_steps: usize,

    /// Checkpointing strategy.
    strategy: CheckpointStrategy,

    /// Whether to use smooth payoffs for barrier options.
    use_smooth_payoffs: bool,

    /// Smoothing epsilon for barrier/digital payoffs.
    smooth_epsilon: f64,
}

impl CheckpointADConfig {
    /// Creates a new checkpoint AD configuration.
    ///
    /// # Arguments
    ///
    /// * `n_steps` - Total number of simulation steps
    /// * `strategy` - Checkpointing strategy to use
    #[inline]
    pub fn new(n_steps: usize, strategy: CheckpointStrategy) -> Self {
        Self {
            n_steps,
            strategy,
            use_smooth_payoffs: true,
            smooth_epsilon: 1e-6,
        }
    }

    /// Creates configuration with automatic optimal checkpointing.
    ///
    /// Uses binomial optimal strategy with approximately √N checkpoints.
    #[inline]
    pub fn with_optimal_checkpoints(n_steps: usize) -> Self {
        let strategy = CheckpointStrategy::binomial_optimal(n_steps);
        Self::new(n_steps, strategy)
    }

    /// Creates configuration with uniform checkpointing.
    #[inline]
    pub fn with_uniform_checkpoints(n_steps: usize, interval: usize) -> Self {
        let strategy = CheckpointStrategy::Uniform { interval };
        Self::new(n_steps, strategy)
    }

    /// Sets whether to use smooth payoffs.
    #[inline]
    pub fn with_smooth_payoffs(mut self, use_smooth: bool) -> Self {
        self.use_smooth_payoffs = use_smooth;
        self
    }

    /// Sets the smoothing epsilon.
    #[inline]
    pub fn with_smooth_epsilon(mut self, epsilon: f64) -> Self {
        self.smooth_epsilon = epsilon;
        self
    }

    /// Returns the number of steps.
    #[inline]
    pub fn n_steps(&self) -> usize {
        self.n_steps
    }

    /// Returns the checkpointing strategy.
    #[inline]
    pub fn strategy(&self) -> &CheckpointStrategy {
        &self.strategy
    }

    /// Returns whether smooth payoffs are enabled.
    #[inline]
    pub fn use_smooth_payoffs(&self) -> bool {
        self.use_smooth_payoffs
    }

    /// Returns the smoothing epsilon.
    #[inline]
    pub fn smooth_epsilon(&self) -> f64 {
        self.smooth_epsilon
    }
}

/// State saved at a checkpoint for AD reverse pass.
#[derive(Clone, Debug)]
pub struct ADCheckpointState<T: Float> {
    /// Step index where checkpoint was saved.
    pub step: usize,

    /// Current spot prices for all paths.
    pub spots: Vec<T>,

    /// Running average (for Asian options).
    pub running_avg: Option<Vec<T>>,

    /// Barrier status (for barrier options).
    pub barrier_alive: Option<Vec<bool>>,

    /// Random numbers used from this step onwards.
    pub randoms_from_step: Vec<T>,
}

impl<T: Float> ADCheckpointState<T> {
    /// Creates a new checkpoint state.
    pub fn new(step: usize, n_paths: usize) -> Self {
        Self {
            step,
            spots: Vec::with_capacity(n_paths),
            running_avg: None,
            barrier_alive: None,
            randoms_from_step: Vec::new(),
        }
    }

    /// Creates a checkpoint for Asian option simulation.
    pub fn for_asian(step: usize, spots: Vec<T>, running_avg: Vec<T>) -> Self {
        Self {
            step,
            spots,
            running_avg: Some(running_avg),
            barrier_alive: None,
            randoms_from_step: Vec::new(),
        }
    }

    /// Creates a checkpoint for barrier option simulation.
    pub fn for_barrier(step: usize, spots: Vec<T>, barrier_alive: Vec<bool>) -> Self {
        Self {
            step,
            spots,
            running_avg: None,
            barrier_alive: Some(barrier_alive),
            randoms_from_step: Vec::new(),
        }
    }
}

/// Checkpointed AD coordinator for path-dependent options.
///
/// Manages the integration between checkpointing and automatic differentiation
/// for memory-efficient Greeks computation.
#[derive(Debug)]
pub struct CheckpointedAD<T: Float> {
    config: CheckpointADConfig,
    #[allow(dead_code)]
    manager: CheckpointManager<T>,
    adjoint_accumulator: AdjointAccumulator<T>,
    checkpoints: Vec<ADCheckpointState<T>>,
    current_step: usize,
}

impl<T: Float> CheckpointedAD<T> {
    /// Creates a new checkpointed AD coordinator.
    pub fn new(config: CheckpointADConfig) -> Self {
        let manager = CheckpointManager::new(config.strategy);
        Self {
            config,
            manager,
            adjoint_accumulator: AdjointAccumulator::new(),
            checkpoints: Vec::new(),
            current_step: 0,
        }
    }

    /// Returns the configuration.
    #[inline]
    pub fn config(&self) -> &CheckpointADConfig {
        &self.config
    }

    /// Checks if a checkpoint should be saved at the given step.
    #[inline]
    pub fn should_checkpoint(&self, step: usize) -> bool {
        self.config
            .strategy
            .should_checkpoint(step, self.config.n_steps)
    }

    /// Saves an AD checkpoint state.
    pub fn save_checkpoint(&mut self, state: ADCheckpointState<T>) {
        self.checkpoints.push(state);
    }

    /// Finds the nearest checkpoint before or at the given step.
    pub fn nearest_checkpoint(&self, step: usize) -> Option<&ADCheckpointState<T>> {
        self.checkpoints.iter().rev().find(|cp| cp.step <= step)
    }

    /// Clears all saved checkpoints.
    pub fn clear_checkpoints(&mut self) {
        self.checkpoints.clear();
    }

    /// Returns the number of saved checkpoints.
    #[inline]
    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    /// Returns the adjoint accumulator.
    #[inline]
    pub fn accumulator(&self) -> &AdjointAccumulator<T> {
        &self.adjoint_accumulator
    }

    /// Returns a mutable reference to the adjoint accumulator.
    #[inline]
    pub fn accumulator_mut(&mut self) -> &mut AdjointAccumulator<T> {
        &mut self.adjoint_accumulator
    }

    /// Resets the coordinator for a new simulation.
    pub fn reset(&mut self) {
        self.checkpoints.clear();
        self.adjoint_accumulator.reset();
        self.current_step = 0;
    }
}

/// Trait for path-dependent payoffs that support AD.
///
/// Implementations must provide methods that are compatible with
/// Enzyme automatic differentiation.
pub trait PathDependentAD<T: Float> {
    /// Computes the payoff value at termination.
    ///
    /// This must be a smooth function for AD to work correctly.
    fn payoff(&self, terminal_spot: T, path_data: &PathData<T>) -> T;

    /// Computes the delta contribution from this path.
    ///
    /// For forward-mode AD, this is the tangent of the payoff with respect to spot.
    fn delta_contribution(&self, terminal_spot: T, path_data: &PathData<T>) -> T;

    /// Returns whether this payoff type requires path history.
    fn requires_path_history(&self) -> bool;
}

/// Path data collected during simulation.
#[derive(Clone, Debug, Default)]
pub struct PathData<T: Float> {
    /// Running sum for average calculation.
    pub running_sum: T,

    /// Number of observations.
    pub n_observations: usize,

    /// Maximum spot seen (for lookback).
    pub max_spot: T,

    /// Minimum spot seen (for lookback).
    pub min_spot: T,

    /// Whether barrier has been breached.
    pub barrier_breached: bool,

    /// Step at which barrier was breached (if any).
    pub breach_step: Option<usize>,
}

impl<T: Float> PathData<T> {
    /// Creates new empty path data.
    pub fn new() -> Self {
        Self {
            running_sum: T::zero(),
            n_observations: 0,
            max_spot: T::neg_infinity(),
            min_spot: T::infinity(),
            barrier_breached: false,
            breach_step: None,
        }
    }

    /// Updates path data with a new observation.
    pub fn observe(&mut self, spot: T) {
        self.running_sum = self.running_sum + spot;
        self.n_observations += 1;

        if spot > self.max_spot {
            self.max_spot = spot;
        }
        if spot < self.min_spot {
            self.min_spot = spot;
        }
    }

    /// Computes the arithmetic average.
    pub fn average(&self) -> T {
        if self.n_observations == 0 {
            T::zero()
        } else {
            self.running_sum / T::from(self.n_observations).unwrap()
        }
    }

    /// Checks and updates barrier breach status.
    pub fn check_barrier(&mut self, spot: T, barrier: T, is_up: bool, step: usize) {
        if !self.barrier_breached {
            let breached = if is_up {
                spot >= barrier
            } else {
                spot <= barrier
            };

            if breached {
                self.barrier_breached = true;
                self.breach_step = Some(step);
            }
        }
    }
}

/// Result of checkpointed AD computation.
#[derive(Clone, Debug)]
pub struct CheckpointedGreeks<T: Float> {
    /// Computed Delta.
    pub delta: T,

    /// Computed Gamma (if available).
    pub gamma: Option<T>,

    /// Computed Vega.
    pub vega: T,

    /// Computed Theta.
    pub theta: T,

    /// Computed Rho.
    pub rho: T,

    /// Number of paths used.
    pub n_paths: usize,

    /// Peak memory usage estimate (in checkpoints).
    pub peak_checkpoints: usize,

    /// Total recomputation steps (for performance analysis).
    pub recomputation_steps: usize,
}

impl<T: Float> Default for CheckpointedGreeks<T> {
    fn default() -> Self {
        Self {
            delta: T::zero(),
            gamma: None,
            vega: T::zero(),
            theta: T::zero(),
            rho: T::zero(),
            n_paths: 0,
            peak_checkpoints: 0,
            recomputation_steps: 0,
        }
    }
}

impl<T: Float> CheckpointedGreeks<T> {
    /// Creates Greeks from a ReverseAD result.
    pub fn from_reverse_ad(ad: &ReverseAD<T>, n_paths: usize) -> Self {
        Self {
            delta: ad.delta,
            gamma: None,
            vega: ad.vega,
            theta: ad.theta,
            rho: ad.rho,
            n_paths,
            peak_checkpoints: 0,
            recomputation_steps: 0,
        }
    }

    /// Sets memory statistics.
    pub fn with_stats(mut self, peak_checkpoints: usize, recomputation_steps: usize) -> Self {
        self.peak_checkpoints = peak_checkpoints;
        self.recomputation_steps = recomputation_steps;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_ad_config_new() {
        let config = CheckpointADConfig::new(252, CheckpointStrategy::Uniform { interval: 25 });

        assert_eq!(config.n_steps(), 252);
        assert!(config.use_smooth_payoffs());
    }

    #[test]
    fn test_checkpoint_ad_config_optimal() {
        let config = CheckpointADConfig::with_optimal_checkpoints(100);

        assert_eq!(config.n_steps(), 100);
    }

    #[test]
    fn test_checkpoint_ad_config_uniform() {
        let config = CheckpointADConfig::with_uniform_checkpoints(252, 25);

        assert_eq!(config.n_steps(), 252);
    }

    #[test]
    fn test_checkpoint_ad_config_builder() {
        let config = CheckpointADConfig::with_optimal_checkpoints(100)
            .with_smooth_payoffs(false)
            .with_smooth_epsilon(1e-8);

        assert!(!config.use_smooth_payoffs());
        assert!((config.smooth_epsilon() - 1e-8).abs() < 1e-15);
    }

    #[test]
    fn test_ad_checkpoint_state_new() {
        let state: ADCheckpointState<f64> = ADCheckpointState::new(10, 100);

        assert_eq!(state.step, 10);
        assert!(state.running_avg.is_none());
        assert!(state.barrier_alive.is_none());
    }

    #[test]
    fn test_ad_checkpoint_state_for_asian() {
        let spots = vec![100.0, 101.0, 102.0];
        let running_avg = vec![100.0, 100.5, 101.0];
        let state = ADCheckpointState::for_asian(5, spots.clone(), running_avg.clone());

        assert_eq!(state.step, 5);
        assert_eq!(state.spots, spots);
        assert_eq!(state.running_avg.unwrap(), running_avg);
    }

    #[test]
    fn test_ad_checkpoint_state_for_barrier() {
        let spots = vec![100.0, 101.0, 102.0];
        let alive = vec![true, true, false];
        let state = ADCheckpointState::for_barrier(5, spots.clone(), alive.clone());

        assert_eq!(state.step, 5);
        assert_eq!(state.barrier_alive.unwrap(), alive);
    }

    #[test]
    fn test_checkpointed_ad_basic() {
        let config = CheckpointADConfig::with_uniform_checkpoints(100, 10);
        let ad: CheckpointedAD<f64> = CheckpointedAD::new(config);

        assert_eq!(ad.checkpoint_count(), 0);
        assert!(ad.should_checkpoint(0));
        assert!(ad.should_checkpoint(10));
        assert!(!ad.should_checkpoint(5));
    }

    #[test]
    fn test_checkpointed_ad_save_and_find() {
        let config = CheckpointADConfig::with_uniform_checkpoints(100, 10);
        let mut ad: CheckpointedAD<f64> = CheckpointedAD::new(config);

        ad.save_checkpoint(ADCheckpointState::new(0, 10));
        ad.save_checkpoint(ADCheckpointState::new(10, 10));
        ad.save_checkpoint(ADCheckpointState::new(20, 10));

        assert_eq!(ad.checkpoint_count(), 3);

        let nearest = ad.nearest_checkpoint(15);
        assert!(nearest.is_some());
        assert_eq!(nearest.unwrap().step, 10);

        let nearest = ad.nearest_checkpoint(25);
        assert_eq!(nearest.unwrap().step, 20);
    }

    #[test]
    fn test_checkpointed_ad_reset() {
        let config = CheckpointADConfig::with_uniform_checkpoints(100, 10);
        let mut ad: CheckpointedAD<f64> = CheckpointedAD::new(config);

        ad.save_checkpoint(ADCheckpointState::new(0, 10));
        ad.accumulator_mut().add_delta(1.0);

        assert_eq!(ad.checkpoint_count(), 1);
        assert_eq!(ad.accumulator().count(), 1);

        ad.reset();

        assert_eq!(ad.checkpoint_count(), 0);
        assert_eq!(ad.accumulator().count(), 0);
    }

    #[test]
    fn test_path_data_new() {
        let data: PathData<f64> = PathData::new();

        assert_eq!(data.n_observations, 0);
        assert_eq!(data.running_sum, 0.0);
        assert!(!data.barrier_breached);
    }

    #[test]
    fn test_path_data_observe() {
        let mut data: PathData<f64> = PathData::new();

        data.observe(100.0);
        data.observe(110.0);
        data.observe(90.0);

        assert_eq!(data.n_observations, 3);
        assert!((data.running_sum - 300.0).abs() < 1e-10);
        assert!((data.max_spot - 110.0).abs() < 1e-10);
        assert!((data.min_spot - 90.0).abs() < 1e-10);
    }

    #[test]
    fn test_path_data_average() {
        let mut data: PathData<f64> = PathData::new();

        data.observe(100.0);
        data.observe(110.0);
        data.observe(90.0);

        assert!((data.average() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_path_data_barrier_check_up() {
        let mut data: PathData<f64> = PathData::new();

        data.check_barrier(100.0, 105.0, true, 0);
        assert!(!data.barrier_breached);

        data.check_barrier(106.0, 105.0, true, 1);
        assert!(data.barrier_breached);
        assert_eq!(data.breach_step, Some(1));
    }

    #[test]
    fn test_path_data_barrier_check_down() {
        let mut data: PathData<f64> = PathData::new();

        data.check_barrier(100.0, 95.0, false, 0);
        assert!(!data.barrier_breached);

        data.check_barrier(94.0, 95.0, false, 1);
        assert!(data.barrier_breached);
        assert_eq!(data.breach_step, Some(1));
    }

    #[test]
    fn test_checkpointed_greeks_default() {
        let greeks: CheckpointedGreeks<f64> = CheckpointedGreeks::default();

        assert_eq!(greeks.delta, 0.0);
        assert!(greeks.gamma.is_none());
        assert_eq!(greeks.n_paths, 0);
    }

    #[test]
    fn test_checkpointed_greeks_from_reverse_ad() {
        let ad = ReverseAD::new(10.0, 0.5, 0.1, 20.0, -5.0);
        let greeks = CheckpointedGreeks::from_reverse_ad(&ad, 1000);

        assert!((greeks.delta - 0.5).abs() < 1e-10);
        assert!((greeks.vega - 20.0).abs() < 1e-10);
        assert_eq!(greeks.n_paths, 1000);
    }

    #[test]
    fn test_checkpointed_greeks_with_stats() {
        let greeks: CheckpointedGreeks<f64> = CheckpointedGreeks::default().with_stats(16, 1000);

        assert_eq!(greeks.peak_checkpoints, 16);
        assert_eq!(greeks.recomputation_steps, 1000);
    }
}
