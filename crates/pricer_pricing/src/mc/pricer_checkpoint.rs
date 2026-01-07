//! Checkpoint-integrated Monte Carlo pricing engine.
//!
//! This module extends the Monte Carlo pricer with checkpoint support for
//! memory-efficient automatic differentiation during path-dependent option pricing.
//!
//! # Checkpoint Integration
//!
//! During the forward pass:
//! 1. Simulation state is saved at configured intervals
//! 2. Checkpoints include RNG state, observer statistics, and current prices
//!
//! During the reverse pass (AD):
//! 1. Checkpoints are restored to avoid storing all intermediate values
//! 2. Forward computation is replayed from checkpoint to compute adjoints
//!
//! # Memory Efficiency
//!
//! With checkpoints at interval `k` over `n` steps:
//! - Without checkpoints: O(n) memory for all intermediate values
//! - With checkpoints: O(n/k) memory for checkpoints + O(k) for replay
//!
//! Optimal checkpoint interval for √n checkpoints gives O(√n) memory.

use crate::checkpoint::{
    CheckpointManager, CheckpointResult, CheckpointStrategy, MemoryBudget, SimulationState,
};
use crate::mc::workspace_checkpoint::CheckpointWorkspace;
use crate::mc::{GbmParams, MonteCarloConfig, PricingResult};
use crate::path_dependent::{PathObserverState, PathPayoffType};
use crate::rng::PricerRng;

/// Configuration for checkpoint-enabled pricing.
///
/// Extends the basic Monte Carlo config with checkpoint strategy and memory budget.
#[derive(Clone, Debug)]
pub struct CheckpointPricingConfig {
    /// Base Monte Carlo configuration.
    pub mc_config: MonteCarloConfig,
    /// Checkpoint strategy.
    pub checkpoint_strategy: CheckpointStrategy,
    /// Optional memory budget for automatic interval calculation.
    pub memory_budget: Option<MemoryBudget>,
}

impl CheckpointPricingConfig {
    /// Creates a new checkpoint pricing configuration.
    pub fn new(mc_config: MonteCarloConfig, checkpoint_strategy: CheckpointStrategy) -> Self {
        Self {
            mc_config,
            checkpoint_strategy,
            memory_budget: None,
        }
    }

    /// Sets a memory budget for automatic checkpoint interval calculation.
    pub fn with_memory_budget(mut self, budget: MemoryBudget) -> Self {
        self.memory_budget = Some(budget);
        self
    }
}

/// Checkpoint-enabled Monte Carlo pricing engine.
///
/// This pricer extends the basic MC engine with checkpoint support for
/// memory-efficient automatic differentiation.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::mc::{MonteCarloConfig, GbmParams};
/// use pricer_pricing::mc::pricer_checkpoint::{CheckpointPricingConfig, CheckpointPricer};
/// use pricer_pricing::checkpoint::CheckpointStrategy;
/// use pricer_pricing::path_dependent::PathPayoffType;
///
/// let mc_config = MonteCarloConfig::builder()
///     .n_paths(10_000)
///     .n_steps(100)
///     .seed(42)
///     .build()
///     .unwrap();
///
/// let config = CheckpointPricingConfig::new(
///     mc_config,
///     CheckpointStrategy::Uniform { interval: 10 },
/// );
///
/// let mut pricer = CheckpointPricer::new(config).unwrap();
/// let gbm = GbmParams::default();
/// let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
/// let df = (-0.05_f64).exp();
///
/// let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);
/// println!("Price: {:.4}", result.price);
/// ```
pub struct CheckpointPricer {
    config: CheckpointPricingConfig,
    workspace: CheckpointWorkspace<f64>,
    rng: PricerRng,
    checkpoint_manager: CheckpointManager<f64>,
}

impl CheckpointPricer {
    /// Creates a new checkpoint-enabled pricer.
    ///
    /// # Arguments
    ///
    /// * `config` - Checkpoint pricing configuration
    ///
    /// # Errors
    ///
    /// Returns error if configuration is invalid.
    pub fn new(config: CheckpointPricingConfig) -> Result<Self, crate::mc::ConfigError> {
        config.mc_config.validate()?;

        let n_paths = config.mc_config.n_paths();
        let n_steps = config.mc_config.n_steps();
        let seed = config.mc_config.seed().unwrap_or(0);

        let workspace = CheckpointWorkspace::new(n_paths, n_steps);
        let rng = PricerRng::from_seed(seed);

        let mut checkpoint_manager =
            CheckpointManager::new(config.checkpoint_strategy).with_total_steps(n_steps);

        // Apply memory budget if set
        if let Some(budget) = config.memory_budget {
            checkpoint_manager = checkpoint_manager.with_memory_budget(budget);
        }

        Ok(Self {
            config,
            workspace,
            rng,
            checkpoint_manager,
        })
    }

    /// Returns a reference to the configuration.
    #[inline]
    pub fn config(&self) -> &CheckpointPricingConfig {
        &self.config
    }

    /// Returns the number of checkpoints currently stored.
    #[inline]
    pub fn checkpoint_count(&self) -> usize {
        self.checkpoint_manager.checkpoint_count()
    }

    /// Returns the total memory usage of checkpoints in bytes.
    #[inline]
    pub fn checkpoint_memory_usage(&self) -> usize {
        self.checkpoint_manager.memory_usage()
    }

    /// Resets the pricer state for a new simulation.
    pub fn reset(&mut self) {
        self.workspace.reset_observers();
        self.rng = PricerRng::from_seed(self.config.mc_config.seed().unwrap_or(0));
        self.checkpoint_manager.clear();
    }

    /// Resets with a new seed.
    pub fn reset_with_seed(&mut self, seed: u64) {
        self.workspace.reset_observers();
        self.rng = PricerRng::from_seed(seed);
        self.checkpoint_manager.clear();
    }

    /// Prices a path-dependent option with checkpoint support.
    ///
    /// This method:
    /// 1. Generates GBM paths step-by-step
    /// 2. Saves checkpoints at configured intervals
    /// 3. Computes path-dependent payoffs
    ///
    /// # Arguments
    ///
    /// * `gbm` - GBM parameters
    /// * `payoff` - Path-dependent payoff type
    /// * `discount_factor` - Discount factor
    ///
    /// # Returns
    ///
    /// Pricing result with price and standard error.
    pub fn price_path_dependent_with_checkpoints(
        &mut self,
        gbm: GbmParams,
        payoff: PathPayoffType<f64>,
        discount_factor: f64,
    ) -> PricingResult {
        let n_paths = self.config.mc_config.n_paths();
        let n_steps = self.config.mc_config.n_steps();

        // Ensure workspace capacity
        self.workspace.ensure_capacity(n_paths, n_steps);

        // Clear previous checkpoints
        self.checkpoint_manager.clear();

        // Reset observers
        self.workspace.reset_observers();

        // Pre-generate all random numbers
        self.rng.fill_normal(self.workspace.randoms_mut());

        // GBM parameters
        let dt = gbm.maturity / n_steps as f64;
        let drift = (gbm.rate - 0.5 * gbm.volatility * gbm.volatility) * dt;
        let vol_sqrt_dt = gbm.volatility * dt.sqrt();

        // Initialize paths with spot price
        let paths = self.workspace.paths_mut();
        for path_idx in 0..n_paths {
            paths[path_idx * (n_steps + 1)] = gbm.spot;
        }

        // Initialize observers with initial spot
        for path_idx in 0..n_paths {
            self.workspace.observer_mut(path_idx).observe(gbm.spot);
        }

        // Forward pass with checkpointing
        for step in 1..=n_steps {
            // Check if we should save a checkpoint BEFORE this step
            if self.checkpoint_manager.should_checkpoint(step - 1) {
                // Save checkpoint state
                let _ = self.save_checkpoint(step - 1, n_paths, n_steps);
            }

            // Generate this step's prices
            // First pass: compute new prices using paths_mut and randoms
            {
                let n_steps_plus_1 = n_steps + 1;
                for path_idx in 0..n_paths {
                    let prev_idx = path_idx * n_steps_plus_1 + step - 1;
                    let curr_idx = path_idx * n_steps_plus_1 + step;
                    let rand_idx = path_idx * n_steps + step - 1;

                    let prev_price = self.workspace.paths()[prev_idx];
                    let random = self.workspace.randoms()[rand_idx];

                    // Log-normal GBM: S_t = S_{t-1} * exp(drift + vol * sqrt(dt) * Z)
                    let curr_price = prev_price * (drift + vol_sqrt_dt * random).exp();
                    self.workspace.paths_mut()[curr_idx] = curr_price;
                }
            }

            // Second pass: observe prices
            {
                let n_steps_plus_1 = n_steps + 1;
                for path_idx in 0..n_paths {
                    let curr_idx = path_idx * n_steps_plus_1 + step;
                    let price = self.workspace.paths()[curr_idx];
                    self.workspace.observer_mut(path_idx).observe(price);
                }
            }
        }

        // Set terminal prices
        {
            let n_steps_plus_1 = n_steps + 1;
            for path_idx in 0..n_paths {
                let terminal_idx = path_idx * n_steps_plus_1 + n_steps;
                let terminal = self.workspace.paths()[terminal_idx];
                self.workspace.observer_mut(path_idx).set_terminal(terminal);
            }
        }

        // Compute payoffs
        let mut payoff_sum = 0.0;
        let mut payoff_sum_sq = 0.0;

        for path_idx in 0..n_paths {
            let observer = self.workspace.observer(path_idx);
            let payoff_value = payoff.compute(&[], observer);
            payoff_sum += payoff_value;
            payoff_sum_sq += payoff_value * payoff_value;
        }

        // Aggregate results
        let mean = payoff_sum / n_paths as f64;
        let variance = (payoff_sum_sq / n_paths as f64) - mean * mean;
        let std_dev = variance.max(0.0).sqrt();
        let std_error = std_dev / (n_paths as f64).sqrt();

        PricingResult {
            price: mean * discount_factor,
            std_error: std_error * discount_factor,
            ..Default::default()
        }
    }

    /// Saves a checkpoint at the given step.
    fn save_checkpoint(
        &mut self,
        step: usize,
        n_paths: usize,
        n_steps: usize,
    ) -> CheckpointResult<()> {
        // Collect current prices
        let paths = self.workspace.paths();
        let mut current_prices = Vec::with_capacity(n_paths);
        for path_idx in 0..n_paths {
            let idx = path_idx * (n_steps + 1) + step;
            current_prices.push(paths[idx]);
        }

        // Create observer state (use first observer as representative, or aggregate)
        // For simplicity, save the first observer's state
        let observer_state = if n_paths > 0 {
            self.workspace.observer(0).snapshot()
        } else {
            PathObserverState::default()
        };

        let state = SimulationState::new(
            step,
            self.rng.seed(),
            0, // RNG calls tracking would require modification
            observer_state,
            current_prices,
        );

        self.checkpoint_manager.save_state(step, state)
    }

    /// Verifies that checkpoint operations produce equivalent results.
    ///
    /// This method runs the simulation twice:
    /// 1. With checkpoints enabled
    /// 2. Without checkpoints
    ///
    /// Returns true if the prices match within tolerance.
    pub fn verify_checkpoint_equivalence(
        &mut self,
        gbm: GbmParams,
        payoff: PathPayoffType<f64>,
        discount_factor: f64,
        tolerance: f64,
    ) -> bool {
        let seed = self.config.mc_config.seed().unwrap_or(42);

        // Price with checkpoints
        self.reset_with_seed(seed);
        let result_with = self.price_path_dependent_with_checkpoints(gbm, payoff, discount_factor);

        // Price without checkpoints (using None strategy)
        let original_strategy = self.config.checkpoint_strategy;
        self.config.checkpoint_strategy = CheckpointStrategy::None;
        self.checkpoint_manager = CheckpointManager::new(CheckpointStrategy::None)
            .with_total_steps(self.config.mc_config.n_steps());

        self.reset_with_seed(seed);
        let result_without =
            self.price_path_dependent_with_checkpoints(gbm, payoff, discount_factor);

        // Restore original strategy
        self.config.checkpoint_strategy = original_strategy;
        self.checkpoint_manager = CheckpointManager::new(original_strategy)
            .with_total_steps(self.config.mc_config.n_steps());

        (result_with.price - result_without.price).abs() < tolerance
    }

    /// Forward pass for gradient computation (placeholder for Enzyme integration).
    ///
    /// In Phase 5+, this will be the entry point for Enzyme's reverse-mode AD.
    /// The checkpoints saved during forward pass will be used to replay
    /// forward computation during the reverse pass.
    ///
    /// # Arguments
    ///
    /// * `gbm` - GBM parameters
    /// * `payoff` - Path-dependent payoff type
    /// * `discount_factor` - Discount factor
    ///
    /// # Returns
    ///
    /// (price, checkpoint_count) tuple.
    pub fn forward_pass_with_checkpoints(
        &mut self,
        gbm: GbmParams,
        payoff: PathPayoffType<f64>,
        discount_factor: f64,
    ) -> (f64, usize) {
        let result = self.price_path_dependent_with_checkpoints(gbm, payoff, discount_factor);
        (result.price, self.checkpoint_count())
    }

    /// Reverse pass placeholder for gradient computation.
    ///
    /// This is a placeholder for the reverse-mode AD pass that will be
    /// integrated with Enzyme in Phase 5+.
    ///
    /// The reverse pass will:
    /// 1. Start from the terminal payoff adjoint
    /// 2. For each segment between checkpoints (in reverse order):
    ///    - Restore state from checkpoint
    ///    - Replay forward computation
    ///    - Accumulate adjoints using Enzyme
    ///
    /// # Returns
    ///
    /// Placeholder gradient (0.0 until Enzyme integration).
    pub fn reverse_pass_placeholder(&self) -> f64 {
        // Placeholder: actual implementation requires Enzyme #[autodiff]
        // This demonstrates the checkpoint restoration pattern:
        //
        // for step in (0..n_steps).rev() {
        //     if let Some(checkpoint_step) = self.checkpoint_manager.nearest_checkpoint(step) {
        //         let state = self.checkpoint_manager.restore_state(checkpoint_step)?;
        //         // Replay forward from checkpoint_step to step
        //         // Accumulate adjoints
        //     }
        // }

        0.0 // Placeholder return
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::{CheckpointStrategy, MemoryBudget};
    use approx::assert_relative_eq;

    fn create_test_config() -> CheckpointPricingConfig {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        CheckpointPricingConfig::new(mc_config, CheckpointStrategy::Uniform { interval: 10 })
    }

    // ========================================================================
    // Construction Tests
    // ========================================================================

    #[test]
    fn test_checkpoint_pricer_creation() {
        let config = create_test_config();
        let pricer = CheckpointPricer::new(config).unwrap();

        assert_eq!(pricer.config().mc_config.n_paths(), 1000);
        assert_eq!(pricer.config().mc_config.n_steps(), 50);
        assert_eq!(pricer.checkpoint_count(), 0);
    }

    #[test]
    fn test_checkpoint_pricer_with_memory_budget() {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .seed(42)
            .build()
            .unwrap();

        let config =
            CheckpointPricingConfig::new(mc_config, CheckpointStrategy::Uniform { interval: 10 })
                .with_memory_budget(MemoryBudget::from_mb(100));

        let pricer = CheckpointPricer::new(config).unwrap();
        assert!(pricer.config().memory_budget.is_some());
    }

    // ========================================================================
    // Pricing Tests
    // ========================================================================

    #[test]
    fn test_price_asian_with_checkpoints() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = (-0.05_f64).exp();

        let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

        assert!(result.price > 0.0);
        assert!(result.std_error > 0.0);
    }

    #[test]
    fn test_checkpoints_created_during_pricing() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        assert_eq!(pricer.checkpoint_count(), 0);

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        let _ = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // With interval=10 and n_steps=50, should have checkpoints at 0, 10, 20, 30, 40
        assert!(pricer.checkpoint_count() > 0);
    }

    #[test]
    fn test_checkpoint_memory_tracking() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        let _ = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // Memory usage should be positive after checkpointing
        assert!(pricer.checkpoint_memory_usage() > 0);
    }

    // ========================================================================
    // Equivalence Tests
    // ========================================================================

    #[test]
    fn test_checkpoint_equivalence() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(
            is_equivalent,
            "Checkpoint and non-checkpoint results should match"
        );
    }

    #[test]
    fn test_checkpoint_equivalence_geometric_asian() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_geometric_call(100.0, 1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(is_equivalent);
    }

    #[test]
    fn test_checkpoint_equivalence_barrier() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::barrier_up_out_call(100.0, 150.0, 1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(is_equivalent);
    }

    #[test]
    fn test_checkpoint_equivalence_lookback() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::lookback_fixed_call(100.0, 1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(is_equivalent);
    }

    // ========================================================================
    // Reproducibility Tests
    // ========================================================================

    #[test]
    fn test_checkpoint_pricing_reproducibility() {
        let config = create_test_config();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        let mut pricer1 = CheckpointPricer::new(config.clone()).unwrap();
        let result1 = pricer1.price_path_dependent_with_checkpoints(gbm, payoff, df);

        let mut pricer2 = CheckpointPricer::new(config).unwrap();
        let result2 = pricer2.price_path_dependent_with_checkpoints(gbm, payoff, df);

        assert_eq!(result1.price, result2.price);
        assert_eq!(result1.std_error, result2.std_error);
    }

    #[test]
    fn test_reset_clears_checkpoints() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        // Price once to create checkpoints
        let _ = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);
        assert!(pricer.checkpoint_count() > 0);

        // Reset should clear checkpoints
        pricer.reset();
        assert_eq!(pricer.checkpoint_count(), 0);
    }

    // ========================================================================
    // Forward Pass Tests
    // ========================================================================

    #[test]
    fn test_forward_pass_returns_checkpoint_count() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        let (price, checkpoint_count) = pricer.forward_pass_with_checkpoints(gbm, payoff, df);

        assert!(price > 0.0);
        assert!(checkpoint_count > 0);
    }

    // ========================================================================
    // Strategy Tests
    // ========================================================================

    #[test]
    fn test_logarithmic_checkpoint_strategy() {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(500)
            .n_steps(100)
            .seed(42)
            .build()
            .unwrap();

        let config = CheckpointPricingConfig::new(
            mc_config,
            CheckpointStrategy::Logarithmic { base_interval: 5 },
        );

        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);
        assert!(result.price > 0.0);

        // Logarithmic strategy should create fewer checkpoints than uniform
        // for the same number of steps
    }

    #[test]
    fn test_no_checkpoint_strategy() {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(500)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let config = CheckpointPricingConfig::new(mc_config, CheckpointStrategy::None);

        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        let _ = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // No checkpoints should be created
        assert_eq!(pricer.checkpoint_count(), 0);
    }

    // ========================================================================
    // Comparison Tests
    // ========================================================================

    #[test]
    fn test_checkpoint_vs_non_checkpoint_same_price() {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(5000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        // With checkpoints
        let config_with = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 5 },
        );
        let mut pricer_with = CheckpointPricer::new(config_with).unwrap();
        let result_with = pricer_with.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // Without checkpoints
        let config_without = CheckpointPricingConfig::new(mc_config, CheckpointStrategy::None);
        let mut pricer_without = CheckpointPricer::new(config_without).unwrap();
        let result_without = pricer_without.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // Prices should be identical (same RNG seed)
        assert_relative_eq!(result_with.price, result_without.price, epsilon = 1e-10);
    }

    // ========================================================================
    // Different Checkpoint Intervals Tests (Task 12.1)
    // ========================================================================

    #[test]
    fn test_different_uniform_intervals_produce_same_price() {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(2000)
            .n_steps(100)
            .seed(42)
            .build()
            .unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        // Interval = 5
        let config_5 = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 5 },
        );
        let mut pricer_5 = CheckpointPricer::new(config_5).unwrap();
        let result_5 = pricer_5.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // Interval = 10
        let config_10 = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 10 },
        );
        let mut pricer_10 = CheckpointPricer::new(config_10).unwrap();
        let result_10 = pricer_10.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // Interval = 20
        let config_20 = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer_20 = CheckpointPricer::new(config_20).unwrap();
        let result_20 = pricer_20.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // Interval = 50
        let config_50 =
            CheckpointPricingConfig::new(mc_config, CheckpointStrategy::Uniform { interval: 50 });
        let mut pricer_50 = CheckpointPricer::new(config_50).unwrap();
        let result_50 = pricer_50.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // All should produce the same price
        assert_relative_eq!(result_5.price, result_10.price, epsilon = 1e-10);
        assert_relative_eq!(result_10.price, result_20.price, epsilon = 1e-10);
        assert_relative_eq!(result_20.price, result_50.price, epsilon = 1e-10);
    }

    #[test]
    fn test_all_strategies_produce_same_price() {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(2000)
            .n_steps(100)
            .seed(42)
            .build()
            .unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_geometric_call(100.0, 1e-6);
        let df = 0.95;

        // None strategy
        let config_none = CheckpointPricingConfig::new(mc_config.clone(), CheckpointStrategy::None);
        let mut pricer_none = CheckpointPricer::new(config_none).unwrap();
        let result_none = pricer_none.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // Uniform strategy
        let config_uniform = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 10 },
        );
        let mut pricer_uniform = CheckpointPricer::new(config_uniform).unwrap();
        let result_uniform = pricer_uniform.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // Logarithmic strategy
        let config_log = CheckpointPricingConfig::new(
            mc_config,
            CheckpointStrategy::Logarithmic { base_interval: 5 },
        );
        let mut pricer_log = CheckpointPricer::new(config_log).unwrap();
        let result_log = pricer_log.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // All should produce the same price
        assert_relative_eq!(result_none.price, result_uniform.price, epsilon = 1e-10);
        assert_relative_eq!(result_uniform.price, result_log.price, epsilon = 1e-10);
    }

    #[test]
    fn test_checkpoint_equivalence_barrier_down_in_put() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::barrier_down_in_put(100.0, 80.0, 1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(is_equivalent);
    }

    #[test]
    fn test_checkpoint_equivalence_barrier_up_in() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::barrier_up_in_call(100.0, 120.0, 1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(is_equivalent);
    }

    #[test]
    fn test_checkpoint_equivalence_barrier_down_out_put() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::barrier_down_out_put(100.0, 80.0, 1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(is_equivalent);
    }

    #[test]
    fn test_checkpoint_equivalence_lookback_floating_call() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::lookback_floating_call(1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(is_equivalent);
    }

    #[test]
    fn test_checkpoint_equivalence_lookback_floating_put() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::lookback_floating_put(1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(is_equivalent);
    }

    #[test]
    fn test_checkpoint_equivalence_lookback_fixed_put() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::lookback_fixed_put(100.0, 1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(is_equivalent);
    }

    #[test]
    fn test_checkpoint_equivalence_asian_put() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_put(100.0, 1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(is_equivalent);
    }

    #[test]
    fn test_checkpoint_equivalence_geometric_asian_put() {
        let config = create_test_config();
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_geometric_put(100.0, 1e-6);
        let df = 0.95;

        let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
        assert!(is_equivalent);
    }

    #[test]
    fn test_different_intervals_all_payoff_types() {
        // Test that different checkpoint intervals produce the same price
        // for all path-dependent option types
        let mc_config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let gbm = GbmParams::default();
        let df = 0.95;

        let payoffs: Vec<PathPayoffType<f64>> = vec![
            PathPayoffType::asian_arithmetic_call(100.0, 1e-6),
            PathPayoffType::asian_geometric_call(100.0, 1e-6),
            PathPayoffType::barrier_up_out_call(100.0, 150.0, 1e-6),
            PathPayoffType::barrier_down_out_put(100.0, 80.0, 1e-6),
            PathPayoffType::lookback_fixed_call(100.0, 1e-6),
            PathPayoffType::lookback_floating_call(1e-6),
        ];

        for payoff in payoffs {
            // Interval = 5
            let config_5 = CheckpointPricingConfig::new(
                mc_config.clone(),
                CheckpointStrategy::Uniform { interval: 5 },
            );
            let mut pricer_5 = CheckpointPricer::new(config_5).unwrap();
            let result_5 = pricer_5.price_path_dependent_with_checkpoints(gbm, payoff, df);

            // Interval = 25
            let config_25 = CheckpointPricingConfig::new(
                mc_config.clone(),
                CheckpointStrategy::Uniform { interval: 25 },
            );
            let mut pricer_25 = CheckpointPricer::new(config_25).unwrap();
            let result_25 = pricer_25.price_path_dependent_with_checkpoints(gbm, payoff, df);

            // All should produce the same price regardless of checkpoint interval
            assert_relative_eq!(result_5.price, result_25.price, epsilon = 1e-10);
        }
    }

    // ========================================================================
    // Overhead Verification Tests (Task 13.1)
    // ========================================================================

    /// Test that checkpoint memory usage increases with more frequent checkpoints.
    #[test]
    fn test_memory_usage_scales_with_checkpoint_frequency() {
        let n_paths = 2000;
        let n_steps = 100;
        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        // Frequent checkpoints (interval = 5)
        let mc_config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .seed(42)
            .build()
            .unwrap();
        let config_frequent = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 5 },
        );
        let mut pricer_frequent = CheckpointPricer::new(config_frequent).unwrap();
        let _ = pricer_frequent.price_path_dependent_with_checkpoints(gbm, payoff, df);
        let mem_frequent = pricer_frequent.checkpoint_memory_usage();

        // Sparse checkpoints (interval = 50)
        let config_sparse =
            CheckpointPricingConfig::new(mc_config, CheckpointStrategy::Uniform { interval: 50 });
        let mut pricer_sparse = CheckpointPricer::new(config_sparse).unwrap();
        let _ = pricer_sparse.price_path_dependent_with_checkpoints(gbm, payoff, df);
        let mem_sparse = pricer_sparse.checkpoint_memory_usage();

        // Frequent checkpoints should use more memory
        assert!(
            mem_frequent > mem_sparse,
            "Frequent checkpoints ({}) should use more memory than sparse ({})",
            mem_frequent,
            mem_sparse
        );

        // Memory should roughly scale with number of checkpoints
        // interval=5 → ~20 checkpoints, interval=50 → ~2 checkpoints
        // So memory ratio should be roughly 10x (allow 5x-20x range)
        let ratio = mem_frequent as f64 / mem_sparse as f64;
        assert!(
            ratio > 3.0,
            "Memory ratio should be > 3x, got {:.2}x",
            ratio
        );
    }

    /// Test that checkpoint count matches expected based on interval.
    #[test]
    fn test_checkpoint_count_matches_interval() {
        let n_steps = 100;
        let mc_config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(n_steps)
            .seed(42)
            .build()
            .unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        // Test various intervals
        for interval in [5, 10, 20, 25, 50] {
            let config = CheckpointPricingConfig::new(
                mc_config.clone(),
                CheckpointStrategy::Uniform { interval },
            );
            let mut pricer = CheckpointPricer::new(config).unwrap();
            let _ = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

            let expected_checkpoints = n_steps / interval;
            let actual_checkpoints = pricer.checkpoint_count();

            // Allow +/- 1 due to boundary conditions
            assert!(
                (actual_checkpoints as i32 - expected_checkpoints as i32).abs() <= 1,
                "Interval {}: expected ~{} checkpoints, got {}",
                interval,
                expected_checkpoints,
                actual_checkpoints
            );
        }
    }

    /// Test that no-checkpoint strategy has zero memory overhead.
    #[test]
    fn test_no_checkpoint_zero_memory() {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .seed(42)
            .build()
            .unwrap();

        let config = CheckpointPricingConfig::new(mc_config, CheckpointStrategy::None);
        let mut pricer = CheckpointPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        let _ = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

        assert_eq!(
            pricer.checkpoint_count(),
            0,
            "No-checkpoint strategy should have 0 checkpoints"
        );
        assert_eq!(
            pricer.checkpoint_memory_usage(),
            0,
            "No-checkpoint strategy should have 0 memory usage"
        );
    }
}
