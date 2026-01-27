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
//! use pricer_core::kernel::CallableKernel;
//!
//! let kernel = /* compiled callable kernel */;
//! let context = KernelContext::new(&curves);
//! let paths = SimulatedPaths::generate(num_paths, num_steps);
//!
//! let npv = CallableEngine::price(&kernel, &context, &paths);
//! ```

use pricer_core::kernel::{CallableBlock, CallableKernel, PricingKernel};

use super::{
    days_to_years,
    lsmc::{LSMCRegressor, RegressionResult},
    CurveProvider, KernelContext, LinearEngine,
};

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
                let exercise_time = days_to_years(exercise.exercise_date, valuation_date_days);
                let time_idx = paths.find_time_index(exercise_time);

                for path_idx in 0..num_paths {
                    // Accumulated value is sum of block cashflows discounted to
                    // exercise date
                    state.accumulated_values[path_idx] = block_values[path_idx];

                    // Add values from previous exercise states
                    if let Some(prev_state) = exercise_states.last() {
                        state.accumulated_values[path_idx] +=
                            prev_state.accumulated_values[path_idx];
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

    /// Performs the backward pass: LSMC regression for optimal exercise.
    ///
    /// Walks backwards through exercise points, using regression to
    /// estimate continuation values and determine optimal exercise.
    ///
    /// # Algorithm
    ///
    /// For each exercise date t (backward from last to first):
    /// 1. Regress discounted future value against basis functions of state
    /// 2. Compare exercise value vs estimated continuation value
    /// 3. If exercise is optimal, update path value to exercise value
    ///
    /// # Arguments
    ///
    /// * `exercise_states` - Forward pass results (from `forward_pass`)
    /// * `paths` - Simulated paths (for discount factors)
    /// * `valuation_date_days` - Valuation date as days from epoch
    /// * `regressor` - LSMC regressor configuration
    ///
    /// # Returns
    ///
    /// `BackwardPassResult` containing option values and exercise decisions.
    pub fn backward_pass(
        exercise_states: &[ExerciseState],
        paths: &SimulatedPaths,
        valuation_date_days: i32,
        regressor: &LSMCRegressor,
    ) -> BackwardPassResult {
        if exercise_states.is_empty() {
            return BackwardPassResult::empty();
        }

        let num_paths = paths.num_paths();
        let num_exercise_dates = exercise_states.len();

        // Cashflow values for each path (updated during backward pass)
        let mut cashflow_values = vec![0.0; num_paths];

        // Track exercise decisions: (exercise_date, exercised paths)
        let mut exercise_decisions: Vec<ExerciseDecision> = Vec::with_capacity(num_exercise_dates);

        // Track regression results for diagnostics
        let mut regression_results: Vec<RegressionResult> = Vec::with_capacity(num_exercise_dates);

        // Process exercise dates in reverse order
        for (state_idx, state) in exercise_states.iter().enumerate().rev() {
            let exercise_time = days_to_years(state.exercise_date, valuation_date_days);
            let time_idx = paths.find_time_index(exercise_time.max(0.0));

            // Determine in-the-money paths (intrinsic > 0)
            let itm_mask: Vec<bool> = state.intrinsic_values.iter().map(|&v| v > 0.0).collect();

            // Calculate future values (discounted from next exercise or final)
            let future_values: Vec<f64> = if state_idx == exercise_states.len() - 1 {
                // Last exercise date: future value is intrinsic at maturity
                state.intrinsic_values.clone()
            } else {
                // Discount cashflow values from next exercise date
                let next_state = &exercise_states[state_idx + 1];
                let next_time = days_to_years(next_state.exercise_date, valuation_date_days);
                let next_time_idx = paths.find_time_index(next_time.max(0.0));

                (0..num_paths)
                    .map(|path_idx| {
                        let df_ratio = if paths.discount_factor(path_idx, next_time_idx) > 0.0 {
                            paths.discount_factor(path_idx, time_idx)
                                / paths.discount_factor(path_idx, next_time_idx)
                        } else {
                            1.0
                        };
                        cashflow_values[path_idx] * df_ratio
                    })
                    .collect()
            };

            // Fit regression for continuation value estimation
            let regression_result =
                regressor.fit(&state.short_rates, &future_values, Some(&itm_mask));

            // Determine exercise decisions
            let exercise_now = regressor.determine_exercise(
                &state.short_rates,
                &state.intrinsic_values,
                &regression_result,
            );

            // Update cashflow values based on exercise decision
            for path_idx in 0..num_paths {
                if exercise_now[path_idx] {
                    // Exercise: take intrinsic value
                    cashflow_values[path_idx] = state.intrinsic_values[path_idx];
                } else if state_idx == exercise_states.len() - 1 {
                    // Last exercise date, don't exercise: take future value
                    cashflow_values[path_idx] = future_values[path_idx];
                }
                // Otherwise: keep previously computed cashflow value
            }

            // Record decisions (reverse order, will be reversed later)
            exercise_decisions.push(ExerciseDecision {
                exercise_date: state.exercise_date,
                exercised: exercise_now,
            });

            regression_results.push(regression_result);
        }

        // Reverse to chronological order
        exercise_decisions.reverse();
        regression_results.reverse();

        // Calculate option value as mean of discounted cashflows
        let final_time = days_to_years(
            exercise_states.last().unwrap().exercise_date,
            valuation_date_days,
        );
        let final_time_idx = paths.find_time_index(final_time.max(0.0));

        let option_value: f64 = (0..num_paths)
            .map(|path_idx| {
                cashflow_values[path_idx] * paths.discount_factor(path_idx, final_time_idx)
            })
            .sum::<f64>()
            / num_paths as f64;

        BackwardPassResult {
            option_value,
            cashflow_values,
            exercise_decisions,
            regression_results,
        }
    }

    /// Full LSMC pricing: forward pass + backward pass.
    ///
    /// # Arguments
    ///
    /// * `kernel` - Compiled callable kernel
    /// * `context` - Market data context
    /// * `paths` - Simulated rate paths
    /// * `valuation_date_days` - Valuation date as days from epoch
    /// * `regressor` - LSMC regressor configuration
    ///
    /// # Returns
    ///
    /// Option value (present value).
    pub fn price_lsmc<P: CurveProvider>(
        kernel: &CallableKernel,
        context: &KernelContext<P>,
        paths: &SimulatedPaths,
        valuation_date_days: i32,
        regressor: &LSMCRegressor,
    ) -> f64 {
        // Forward pass: accumulate values to exercise points
        let exercise_states = Self::forward_pass(kernel, context, paths, valuation_date_days);

        if exercise_states.is_empty() {
            return Self::price_deterministic(kernel, context);
        }

        // Backward pass: LSMC regression for optimal exercise
        let result = Self::backward_pass(&exercise_states, paths, valuation_date_days, regressor);

        result.option_value
    }
}

/// Result of backward pass LSMC algorithm.
#[derive(Clone, Debug)]
pub struct BackwardPassResult {
    /// Option value (mean of discounted cashflows).
    pub option_value: f64,

    /// Final cashflow value for each path.
    pub cashflow_values: Vec<f64>,

    /// Exercise decisions at each exercise date.
    pub exercise_decisions: Vec<ExerciseDecision>,

    /// Regression results at each exercise date.
    pub regression_results: Vec<RegressionResult>,
}

impl BackwardPassResult {
    /// Creates an empty result.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            option_value: 0.0,
            cashflow_values: Vec::new(),
            exercise_decisions: Vec::new(),
            regression_results: Vec::new(),
        }
    }

    /// Returns the number of exercise dates.
    #[must_use]
    pub fn num_exercise_dates(&self) -> usize { self.exercise_decisions.len() }

    /// Returns the exercise probability at each exercise date.
    #[must_use]
    pub fn exercise_probabilities(&self) -> Vec<f64> {
        self.exercise_decisions
            .iter()
            .map(|d| {
                let exercised = d.exercised.iter().filter(|&&e| e).count();
                exercised as f64 / d.exercised.len() as f64
            })
            .collect()
    }
}

/// Exercise decision at a specific date.
#[derive(Clone, Debug)]
pub struct ExerciseDecision {
    /// Exercise date (days from epoch).
    pub exercise_date: i32,

    /// Whether each path exercised (true = exercised).
    pub exercised: Vec<bool>,
}

#[cfg(test)]
mod tests {
    use pricer_core::kernel::{CallableBlock, CallableKernel, ExerciseDef, PricingKernel};

    use super::*;

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
            .map(|_| time_grid.iter().map(|&t| (-0.03 * t).exp()).collect())
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

    // =========================================================================
    // Backward Pass Tests
    // =========================================================================

    #[test]
    fn test_backward_pass_empty_states() {
        let paths = create_test_paths();
        let regressor = LSMCRegressor::default();

        let result = CallableEngine::backward_pass(&[], &paths, 19000, &regressor);

        assert!(result.option_value.abs() < 1e-10);
        assert!(result.cashflow_values.is_empty());
        assert!(result.exercise_decisions.is_empty());
    }

    #[test]
    fn test_backward_pass_creates_exercise_decisions() {
        let kernel = create_test_kernel();
        let paths = create_test_paths();
        let regressor = LSMCRegressor::default();

        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let states = CallableEngine::forward_pass(&kernel, &context, &paths, 19000);
        let result = CallableEngine::backward_pass(&states, &paths, 19000, &regressor);

        // Should have 2 exercise decisions (one per exercise date)
        assert_eq!(result.num_exercise_dates(), 2);
        assert_eq!(result.exercise_decisions.len(), 2);
    }

    #[test]
    fn test_backward_pass_exercise_decision_paths() {
        let kernel = create_test_kernel();
        let paths = create_test_paths();
        let regressor = LSMCRegressor::default();

        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let states = CallableEngine::forward_pass(&kernel, &context, &paths, 19000);
        let result = CallableEngine::backward_pass(&states, &paths, 19000, &regressor);

        // Each decision should have the same number of paths
        for decision in &result.exercise_decisions {
            assert_eq!(decision.exercised.len(), paths.num_paths());
        }
    }

    #[test]
    fn test_backward_pass_result_empty() {
        let result = BackwardPassResult::empty();

        assert!(result.option_value.abs() < 1e-10);
        assert!(result.cashflow_values.is_empty());
        assert_eq!(result.num_exercise_dates(), 0);
    }

    #[test]
    fn test_backward_pass_exercise_probabilities() {
        let kernel = create_test_kernel();
        let paths = create_test_paths();
        let regressor = LSMCRegressor::default();

        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let states = CallableEngine::forward_pass(&kernel, &context, &paths, 19000);
        let result = CallableEngine::backward_pass(&states, &paths, 19000, &regressor);

        let probs = result.exercise_probabilities();

        // Should have probability for each exercise date
        assert_eq!(probs.len(), 2);

        // Probabilities should be between 0 and 1
        for prob in &probs {
            assert!(*prob >= 0.0 && *prob <= 1.0);
        }
    }

    #[test]
    fn test_backward_pass_with_intrinsic_values() {
        // Create exercise states with known intrinsic values
        let num_paths = 100;
        let mut state = ExerciseState::new(19365, num_paths);

        // Half paths are ITM with value 1.0
        for i in 0..num_paths {
            if i < 50 {
                state.intrinsic_values[i] = 1.0;
            }
            state.short_rates[i] = 0.03;
        }

        let paths = create_test_paths();
        let regressor = LSMCRegressor::default();

        let result = CallableEngine::backward_pass(&[state], &paths, 19000, &regressor);

        // Should have one exercise decision
        assert_eq!(result.num_exercise_dates(), 1);
    }

    // =========================================================================
    // Full LSMC Pricing Tests
    // =========================================================================

    #[test]
    fn test_price_lsmc_empty_kernel() {
        let kernel = CallableKernel::empty();
        let paths = create_test_paths();
        let regressor = LSMCRegressor::default();

        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let price = CallableEngine::price_lsmc(&kernel, &context, &paths, 19000, &regressor);

        // Empty kernel should have zero price
        assert!(price.abs() < 1e-10);
    }

    #[test]
    fn test_price_lsmc_with_exercise() {
        let kernel = create_test_kernel();
        let paths = create_test_paths();
        let regressor = LSMCRegressor::default();

        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let price = CallableEngine::price_lsmc(&kernel, &context, &paths, 19000, &regressor);

        // Price should be finite
        assert!(price.is_finite());
    }

    #[test]
    fn test_exercise_decision_new() {
        let decision = ExerciseDecision {
            exercise_date: 19365,
            exercised: vec![true, false, true, false],
        };

        assert_eq!(decision.exercise_date, 19365);
        assert_eq!(decision.exercised.len(), 4);
        assert!(decision.exercised[0]);
        assert!(!decision.exercised[1]);
    }

    #[test]
    fn test_backward_pass_regression_results() {
        let kernel = create_test_kernel();
        let paths = create_test_paths();
        let regressor = LSMCRegressor::default();

        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);

        let states = CallableEngine::forward_pass(&kernel, &context, &paths, 19000);
        let result = CallableEngine::backward_pass(&states, &paths, 19000, &regressor);

        // Should have regression result for each exercise date
        assert_eq!(result.regression_results.len(), 2);
    }
}
