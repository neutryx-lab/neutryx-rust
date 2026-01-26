//! Callable/Bermudan pricing engine for CallableKernel IR.
//!
//! This module provides `CallableEngine` which prices products with early
//! exercise features using Longstaff-Schwartz Monte Carlo (LSMC).
//!
//! # Algorithm
//!
//! 1. **Forward Pass**: Simulate paths and accumulate cashflow values
//! 2. **Backward Pass**: LSMC regression to determine optimal exercise
//!
//! # Example
//!
//! ```ignore
//! use pricer_pricing::kernel::{CallableEngine, CallableContext, KernelContext};
//! use pricer_core::ir::CallableKernel;
//!
//! let kernel = /* compiled callable kernel */;
//! let context = KernelContext::new(&curves);
//! let paths = SimulatedPaths::generate(num_paths, num_steps);
//!
//! let npv = CallableEngine::price(&kernel, &context, &paths);
//! ```

use pricer_core::ir::{CallableBlock, CallableKernel, PricingKernel};

use super::{days_to_years, CurveProvider, KernelContext, LinearEngine};

/// State at an exercise point during forward pass.
///
/// Contains the accumulated value and market state at each exercise date
/// for each simulation path.
#[derive(Clone, Debug)]
pub struct ExerciseState {
    /// Exercise date (days from epoch).
    pub exercise_date: i32,

    /// Accumulated cashflow value (one per path).
    /// This is the value of cashflows received/paid up to this exercise point.
    pub accumulated_values: Vec<f64>,

    /// Intrinsic value at exercise (one per path).
    /// This is the value of exercising immediately.
    pub intrinsic_values: Vec<f64>,

    /// Short rate at this exercise point (one per path).
    /// Used as regression variable for LSMC.
    pub short_rates: Vec<f64>,
}

impl ExerciseState {
    /// Creates a new exercise state.
    ///
    /// # Arguments
    ///
    /// * `exercise_date` - Exercise date
    /// * `num_paths` - Number of simulation paths
    #[must_use]
    pub fn new(exercise_date: i32, num_paths: usize) -> Self {
        Self {
            exercise_date,
            accumulated_values: vec![0.0; num_paths],
            intrinsic_values: vec![0.0; num_paths],
            short_rates: vec![0.0; num_paths],
        }
    }

    /// Returns the number of paths.
    #[must_use]
    pub fn num_paths(&self) -> usize { self.accumulated_values.len() }
}

/// Simulated paths for Monte Carlo pricing.
///
/// Stores rate paths for forward rate simulation.
#[derive(Clone, Debug)]
pub struct SimulatedPaths {
    /// Number of simulation paths.
    num_paths: usize,

    /// Time grid (years from valuation).
    time_grid: Vec<f64>,

    /// Short rate paths: paths[path_idx][time_idx].
    short_rate_paths: Vec<Vec<f64>>,

    /// Discount factor paths (cumulative): df_paths[path_idx][time_idx].
    discount_factor_paths: Vec<Vec<f64>>,
}

impl SimulatedPaths {
    /// Creates new simulated paths.
    ///
    /// # Arguments
    ///
    /// * `num_paths` - Number of Monte Carlo paths
    /// * `time_grid` - Time points (years from valuation date)
    #[must_use]
    pub fn new(num_paths: usize, time_grid: Vec<f64>) -> Self {
        let num_steps = time_grid.len();
        Self {
            num_paths,
            time_grid,
            short_rate_paths: vec![vec![0.0; num_steps]; num_paths],
            discount_factor_paths: vec![vec![1.0; num_steps]; num_paths],
        }
    }

    /// Creates paths with given short rate and discount factor data.
    #[must_use]
    pub fn with_data(
        time_grid: Vec<f64>,
        short_rate_paths: Vec<Vec<f64>>,
        discount_factor_paths: Vec<Vec<f64>>,
    ) -> Self {
        let num_paths = short_rate_paths.len();
        Self {
            num_paths,
            time_grid,
            short_rate_paths,
            discount_factor_paths,
        }
    }

    /// Returns the number of paths.
    #[must_use]
    pub fn num_paths(&self) -> usize { self.num_paths }

    /// Returns the number of time steps.
    #[must_use]
    pub fn num_steps(&self) -> usize { self.time_grid.len() }

    /// Returns the time grid.
    #[must_use]
    pub fn time_grid(&self) -> &[f64] { &self.time_grid }

    /// Returns the short rate for a given path and time index.
    #[must_use]
    pub fn short_rate(&self, path_idx: usize, time_idx: usize) -> f64 {
        self.short_rate_paths[path_idx][time_idx]
    }

    /// Returns the discount factor for a given path and time index.
    #[must_use]
    pub fn discount_factor(&self, path_idx: usize, time_idx: usize) -> f64 {
        self.discount_factor_paths[path_idx][time_idx]
    }

    /// Finds the time index closest to the given time.
    #[must_use]
    pub fn find_time_index(&self, target_time: f64) -> usize {
        self.time_grid
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                ((*a - target_time).abs())
                    .partial_cmp(&((*b - target_time).abs()))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    /// Sets short rate paths (for testing or external path generation).
    pub fn set_short_rate_paths(&mut self, paths: Vec<Vec<f64>>) { self.short_rate_paths = paths; }

    /// Sets discount factor paths.
    pub fn set_discount_factor_paths(&mut self, paths: Vec<Vec<f64>>) {
        self.discount_factor_paths = paths;
    }
}

/// Pricing engine for callable/Bermudan products.
///
/// Uses Longstaff-Schwartz Monte Carlo (LSMC) for optimal exercise
/// determination.
pub struct CallableEngine;

impl CallableEngine {
    /// Performs the forward pass: accumulate cashflow values to exercise
    /// points.
    ///
    /// # Arguments
    ///
    /// * `kernel` - Compiled callable kernel
    /// * `context` - Market data context
    /// * `paths` - Simulated rate paths
    /// * `valuation_date_days` - Valuation date as days from epoch
    ///
    /// # Returns
    ///
    /// Vector of `ExerciseState` for each exercise point, containing
    /// accumulated values and market state.
    pub fn forward_pass<P: CurveProvider>(
        kernel: &CallableKernel,
        context: &KernelContext<P>,
        paths: &SimulatedPaths,
        valuation_date_days: i32,
    ) -> Vec<ExerciseState> {
        let num_paths = paths.num_paths();
        let mut exercise_states: Vec<ExerciseState> = Vec::new();

        // Process blocks in forward (chronological) order
        for block in kernel.iter() {
            // Evaluate cashflows within this block for each path
            let block_values = Self::evaluate_block_cashflows(
                &block.core_flows,
                context,
                paths,
                valuation_date_days,
            );

            // If this block has an exercise, create exercise state
            if let Some(exercise) = &block.exercise {
                let mut state = ExerciseState::new(exercise.exercise_date, num_paths);

                // Calculate accumulated values up to this point
                let exercise_time =
                    days_to_years(exercise.exercise_date, valuation_date_days);
                let time_idx = paths.find_time_index(exercise_time);

                for path_idx in 0..num_paths {
                    // Accumulated value is sum of block cashflows discounted to
                    // exercise date
                    state.accumulated_values[path_idx] = block_values[path_idx];

                    // Add values from previous exercise states
                    if let Some(prev_state) = exercise_states.last() {
                        state.accumulated_values[path_idx] += prev_state.accumulated_values[path_idx];
                    }

                    // Record short rate at exercise point
                    state.short_rates[path_idx] = paths.short_rate(path_idx, time_idx);

                    // Intrinsic value is the value of underlying if exercised
                    // For a swaption, this is the value of the remaining swap
                    state.intrinsic_values[path_idx] = Self::calculate_intrinsic_value(
                        kernel,
                        block,
                        context,
                        paths,
                        path_idx,
                        valuation_date_days,
                    );
                }

                exercise_states.push(state);
            }
        }

        exercise_states
    }

    /// Evaluates cashflows within a block for all paths.
    ///
    /// Returns a vector of present values (one per path).
    fn evaluate_block_cashflows<P: CurveProvider>(
        kernel: &PricingKernel,
        context: &KernelContext<P>,
        paths: &SimulatedPaths,
        valuation_date_days: i32,
    ) -> Vec<f64> {
        let num_paths = paths.num_paths();
        let mut values = vec![0.0; num_paths];

        if kernel.is_empty() {
            return values;
        }

        // For each path, evaluate cashflows
        for path_idx in 0..num_paths {
            let mut path_value = 0.0;

            for i in 0..kernel.len() {
                let payment_date = kernel.payment_dates[i];
                let fixing_date = kernel.fixing_dates[i];
                let year_fraction = kernel.year_fractions[i];
                let notional = kernel.notionals[i];
                let spread = kernel.spreads[i];
                let gearing = kernel.gearings[i];
                let fwd_index_id = kernel.fwd_index_ids[i];
                let discount_curve_id = kernel.discount_curve_ids[i];
                let fx_index_id = kernel.fx_index_ids[i];

                // Get forward rate (for floating, use path-dependent rate)
                let fwd_rate = if fwd_index_id == 0 {
                    0.0 // Fixed leg
                } else {
                    // Use short rate from path at fixing date
                    let fixing_time = days_to_years(fixing_date, valuation_date_days);
                    let time_idx = paths.find_time_index(fixing_time.max(0.0));
                    paths.short_rate(path_idx, time_idx)
                };

                // Calculate rate: L * α + β
                let rate = fwd_rate * gearing + spread;

                // Calculate amount: N * τ * rate
                let amount = notional * year_fraction * rate;

                // Get discount factor
                let payment_time = days_to_years(payment_date, valuation_date_days);
                let df = if payment_time <= 0.0 {
                    1.0 // Already paid
                } else {
                    let time_idx = paths.find_time_index(payment_time);
                    paths.discount_factor(path_idx, time_idx)
                };

                // Get FX rate (use deterministic for now)
                let fx_rate = context.fx_rate(fx_index_id);

                path_value += amount * df * fx_rate;
            }

            values[path_idx] = path_value;
        }

        values
    }

    /// Calculates intrinsic value for exercise at given block.
    ///
    /// For a Bermudan swaption, this is the value of the remaining swap
    /// if exercised at this point.
    fn calculate_intrinsic_value<P: CurveProvider>(
        kernel: &CallableKernel,
        current_block: &CallableBlock,
        context: &KernelContext<P>,
        paths: &SimulatedPaths,
        path_idx: usize,
        valuation_date_days: i32,
    ) -> f64 {
        let mut intrinsic = 0.0;

        // Sum value of all remaining blocks (including current)
        let current_start = current_block.start_date;
        let mut found_current = false;

        for block in kernel.iter() {
            if block.start_date == current_start {
                found_current = true;
            }

            if found_current {
                // Evaluate this block's cashflows
                for i in 0..block.core_flows.len() {
                    let payment_date = block.core_flows.payment_dates[i];
                    let fixing_date = block.core_flows.fixing_dates[i];
                    let year_fraction = block.core_flows.year_fractions[i];
                    let notional = block.core_flows.notionals[i];
                    let spread = block.core_flows.spreads[i];
                    let gearing = block.core_flows.gearings[i];
                    let fwd_index_id = block.core_flows.fwd_index_ids[i];
                    let fx_index_id = block.core_flows.fx_index_ids[i];

                    // Get forward rate
                    let fwd_rate = if fwd_index_id == 0 {
                        0.0
                    } else {
                        let fixing_time = days_to_years(fixing_date, valuation_date_days);
                        let time_idx = paths.find_time_index(fixing_time.max(0.0));
                        paths.short_rate(path_idx, time_idx)
                    };

                    let rate = fwd_rate * gearing + spread;
                    let amount = notional * year_fraction * rate;

                    // Discount from payment date
                    let payment_time = days_to_years(payment_date, valuation_date_days);
                    let df = if payment_time <= 0.0 {
                        1.0
                    } else {
                        let time_idx = paths.find_time_index(payment_time);
                        paths.discount_factor(path_idx, time_idx)
                    };

                    let fx_rate = context.fx_rate(fx_index_id);

                    intrinsic += amount * df * fx_rate;
                }
            }
        }

        intrinsic
    }

    /// Prices a callable kernel using deterministic (non-MC) valuation.
    ///
    /// This is a simplified pricing that doesn't consider exercise
    /// optionality.
    /// Useful for testing and comparison.
    pub fn price_deterministic<P: CurveProvider>(
        kernel: &CallableKernel,
        context: &KernelContext<P>,
    ) -> f64 {
        let mut total_pv = 0.0;

        for block in kernel.iter() {
            total_pv += LinearEngine::price(&block.core_flows, context);
        }

        total_pv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pricer_core::ir::{CallableBlock, CallableKernel, ExerciseDef, ExerciseStyle, PricingKernel};

    fn create_test_kernel() -> CallableKernel {
        CallableKernel::new(
            vec![
                CallableBlock::new(
                    19000,
                    19365,
                    PricingKernel::empty(),
                    Some(ExerciseDef::bermudan(19365)),
                ),
                CallableBlock::new(
                    19365,
                    19730,
                    PricingKernel::empty(),
                    Some(ExerciseDef::bermudan(19730)),
                ),
            ],
            0,
        )
    }

    fn create_test_paths() -> SimulatedPaths {
        let time_grid = vec![0.0, 0.5, 1.0, 1.5, 2.0];
        let num_paths = 100;

        let mut paths = SimulatedPaths::new(num_paths, time_grid.clone());

        // Set constant short rates and discount factors for testing
        let short_rate_paths: Vec<Vec<f64>> = (0..num_paths)
            .map(|_| vec![0.03; time_grid.len()])
            .collect();

        let df_paths: Vec<Vec<f64>> = (0..num_paths)
            .map(|_| {
                time_grid
                    .iter()
                    .map(|&t| (-0.03 * t).exp())
                    .collect()
            })
            .collect();

        paths.set_short_rate_paths(short_rate_paths);
        paths.set_discount_factor_paths(df_paths);

        paths
    }

    // =========================================================================
    // SimulatedPaths Tests
    // =========================================================================

    #[test]
    fn test_simulated_paths_new() {
        let paths = SimulatedPaths::new(100, vec![0.0, 0.5, 1.0]);
        assert_eq!(paths.num_paths(), 100);
        assert_eq!(paths.num_steps(), 3);
    }

    #[test]
    fn test_simulated_paths_time_grid() {
        let time_grid = vec![0.0, 0.5, 1.0, 2.0];
        let paths = SimulatedPaths::new(10, time_grid.clone());
        assert_eq!(paths.time_grid(), &time_grid[..]);
    }

    #[test]
    fn test_simulated_paths_find_time_index() {
        let paths = SimulatedPaths::new(10, vec![0.0, 0.5, 1.0, 2.0]);

        assert_eq!(paths.find_time_index(0.0), 0);
        assert_eq!(paths.find_time_index(0.5), 1);
        assert_eq!(paths.find_time_index(0.4), 1); // Closest to 0.5
        assert_eq!(paths.find_time_index(1.5), 2); // Closest to 1.0 or 2.0
        assert_eq!(paths.find_time_index(2.0), 3);
    }

    #[test]
    fn test_simulated_paths_set_rates() {
        let mut paths = SimulatedPaths::new(2, vec![0.0, 1.0]);

        let short_rates = vec![vec![0.03, 0.04], vec![0.035, 0.045]];
        paths.set_short_rate_paths(short_rates);

        assert!((paths.short_rate(0, 0) - 0.03).abs() < 1e-10);
        assert!((paths.short_rate(0, 1) - 0.04).abs() < 1e-10);
        assert!((paths.short_rate(1, 0) - 0.035).abs() < 1e-10);
    }

    #[test]
    fn test_simulated_paths_set_discount_factors() {
        let mut paths = SimulatedPaths::new(2, vec![0.0, 1.0]);

        let df_paths = vec![vec![1.0, 0.95], vec![1.0, 0.96]];
        paths.set_discount_factor_paths(df_paths);

        assert!((paths.discount_factor(0, 0) - 1.0).abs() < 1e-10);
        assert!((paths.discount_factor(0, 1) - 0.95).abs() < 1e-10);
    }

    // =========================================================================
    // ExerciseState Tests
    // =========================================================================

    #[test]
    fn test_exercise_state_new() {
        let state = ExerciseState::new(19365, 100);

        assert_eq!(state.exercise_date, 19365);
        assert_eq!(state.num_paths(), 100);
        assert_eq!(state.accumulated_values.len(), 100);
        assert_eq!(state.intrinsic_values.len(), 100);
        assert_eq!(state.short_rates.len(), 100);
    }

    #[test]
    fn test_exercise_state_initialised_zero() {
        let state = ExerciseState::new(19365, 10);

        for i in 0..10 {
            assert!(state.accumulated_values[i].abs() < 1e-10);
            assert!(state.intrinsic_values[i].abs() < 1e-10);
            assert!(state.short_rates[i].abs() < 1e-10);
        }
    }

    // =========================================================================
    // Forward Pass Tests
    // =========================================================================

    #[test]
    fn test_forward_pass_empty_kernel() {
        let kernel = CallableKernel::empty();
        let paths = create_test_paths();

        // Create a mock context
        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let states = CallableEngine::forward_pass(&kernel, &context, &paths, 19000);

        assert!(states.is_empty());
    }

    #[test]
    fn test_forward_pass_creates_exercise_states() {
        let kernel = create_test_kernel();
        let paths = create_test_paths();

        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let states = CallableEngine::forward_pass(&kernel, &context, &paths, 19000);

        // Should have 2 exercise states (one per exercise date)
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].exercise_date, 19365);
        assert_eq!(states[1].exercise_date, 19730);
    }

    #[test]
    fn test_forward_pass_path_count() {
        let kernel = create_test_kernel();
        let paths = create_test_paths();

        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let states = CallableEngine::forward_pass(&kernel, &context, &paths, 19000);

        for state in &states {
            assert_eq!(state.num_paths(), paths.num_paths());
        }
    }

    #[test]
    fn test_forward_pass_records_short_rates() {
        let kernel = create_test_kernel();
        let paths = create_test_paths();

        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let states = CallableEngine::forward_pass(&kernel, &context, &paths, 19000);

        // Short rates should be recorded (3% constant in our test paths)
        for path_idx in 0..10 {
            assert!((states[0].short_rates[path_idx] - 0.03).abs() < 1e-10);
        }
    }

    // =========================================================================
    // Deterministic Pricing Tests
    // =========================================================================

    #[test]
    fn test_price_deterministic_empty() {
        let kernel = CallableKernel::empty();

        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let pv = CallableEngine::price_deterministic(&kernel, &context);

        assert!(pv.abs() < 1e-10);
    }

    #[test]
    fn test_price_deterministic_with_cashflows() {
        // Create kernel with some cashflows
        let inner_kernel = PricingKernel::new(
            vec![19365],
            vec![19363],
            vec![1.0],
            vec![1_000_000.0],
            vec![0.03],
            vec![0.0], // Fixed
            vec![0],
            vec![0],
            vec![0],
            vec![0],
        )
        .unwrap();

        let kernel = CallableKernel::new(
            vec![CallableBlock::new(19000, 19365, inner_kernel, None)],
            0,
        );

        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let pv = CallableEngine::price_deterministic(&kernel, &context);

        // Should have some value
        assert!(pv.abs() > 0.0);
    }
}
