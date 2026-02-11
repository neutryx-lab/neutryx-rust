//! Callable/Bermudan pricing engine for CallableKernel IR.

use pricer_core::kernel::{CallableBlock, CallableKernel, PricingKernel};

use super::{
    days_to_years,
    lsmc::{LSMCRegressor, RegressionResult},
    CurveProvider, KernelContext, LinearEngine,
};

/// State at an exercise point during forward pass.
#[derive(Clone, Debug)]
pub struct ExerciseState {
    /// Date of the exercise opportunity.
    pub exercise_date: i32,
    /// Accumulated cashflow values per path.
    pub accumulated_values: Vec<f64>,
    /// Intrinsic values at this exercise point per path.
    pub intrinsic_values: Vec<f64>,
    /// Short rates at this exercise point per path.
    pub short_rates: Vec<f64>,
}

impl ExerciseState {
    /// Creates a new exercise state.
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
#[derive(Clone, Debug)]
pub struct SimulatedPaths {
    num_paths: usize,
    time_grid: Vec<f64>,
    short_rate_paths: Vec<Vec<f64>>,
    discount_factor_paths: Vec<Vec<f64>>,
}

impl SimulatedPaths {
    /// Creates new simulated paths.
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
pub struct CallableEngine;

impl CallableEngine {
    /// Performs the forward pass: accumulate cashflow values to exercise points.
    pub fn forward_pass<P: CurveProvider>(
        kernel: &CallableKernel,
        context: &KernelContext<P>,
        paths: &SimulatedPaths,
        valuation_date_days: i32,
    ) -> Vec<ExerciseState> {
        let num_paths = paths.num_paths();
        let mut exercise_states: Vec<ExerciseState> = Vec::new();

        for block in kernel.iter() {
            let block_values = Self::evaluate_block_cashflows(
                &block.core_flows,
                context,
                paths,
                valuation_date_days,
            );

            if let Some(exercise) = &block.exercise {
                let mut state = ExerciseState::new(exercise.exercise_date, num_paths);
                let exercise_time = days_to_years(exercise.exercise_date, valuation_date_days);
                let time_idx = paths.find_time_index(exercise_time);

                for path_idx in 0..num_paths {
                    state.accumulated_values[path_idx] = block_values[path_idx];

                    if let Some(prev_state) = exercise_states.last() {
                        state.accumulated_values[path_idx] +=
                            prev_state.accumulated_values[path_idx];
                    }

                    state.short_rates[path_idx] = paths.short_rate(path_idx, time_idx);

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

                let fwd_rate = if fwd_index_id == 0 {
                    0.0
                } else {
                    let fixing_time = days_to_years(fixing_date, valuation_date_days);
                    let time_idx = paths.find_time_index(fixing_time.max(0.0));
                    paths.short_rate(path_idx, time_idx)
                };

                let rate = fwd_rate * gearing + spread;
                let amount = notional * year_fraction * rate;

                let payment_time = days_to_years(payment_date, valuation_date_days);
                let df = if payment_time <= 0.0 {
                    1.0
                } else {
                    let time_idx = paths.find_time_index(payment_time);
                    paths.discount_factor(path_idx, time_idx)
                };

                let fx_rate = context.fx_rate(fx_index_id);

                path_value += amount * df * fx_rate;
            }

            values[path_idx] = path_value;
        }

        values
    }

    /// Calculates intrinsic value for exercise at given block.
    fn calculate_intrinsic_value<P: CurveProvider>(
        kernel: &CallableKernel,
        current_block: &CallableBlock,
        context: &KernelContext<P>,
        paths: &SimulatedPaths,
        path_idx: usize,
        valuation_date_days: i32,
    ) -> f64 {
        let mut intrinsic = 0.0;
        let current_start = current_block.start_date;
        let mut found_current = false;

        for block in kernel.iter() {
            if block.start_date == current_start {
                found_current = true;
            }

            if found_current {
                for i in 0..block.core_flows.len() {
                    let payment_date = block.core_flows.payment_dates[i];
                    let fixing_date = block.core_flows.fixing_dates[i];
                    let year_fraction = block.core_flows.year_fractions[i];
                    let notional = block.core_flows.notionals[i];
                    let spread = block.core_flows.spreads[i];
                    let gearing = block.core_flows.gearings[i];
                    let fwd_index_id = block.core_flows.fwd_index_ids[i];
                    let fx_index_id = block.core_flows.fx_index_ids[i];

                    let fwd_rate = if fwd_index_id == 0 {
                        0.0
                    } else {
                        let fixing_time = days_to_years(fixing_date, valuation_date_days);
                        let time_idx = paths.find_time_index(fixing_time.max(0.0));
                        paths.short_rate(path_idx, time_idx)
                    };

                    let rate = fwd_rate * gearing + spread;
                    let amount = notional * year_fraction * rate;

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

        let mut cashflow_values = vec![0.0; num_paths];
        let mut exercise_decisions: Vec<ExerciseDecision> = Vec::with_capacity(num_exercise_dates);
        let mut regression_results: Vec<RegressionResult> = Vec::with_capacity(num_exercise_dates);

        for (state_idx, state) in exercise_states.iter().enumerate().rev() {
            let exercise_time = days_to_years(state.exercise_date, valuation_date_days);
            let time_idx = paths.find_time_index(exercise_time.max(0.0));

            let itm_mask: Vec<bool> = state.intrinsic_values.iter().map(|&v| v > 0.0).collect();

            let future_values: Vec<f64> = if state_idx == exercise_states.len() - 1 {
                state.intrinsic_values.clone()
            } else {
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

            let regression_result =
                regressor.fit(&state.short_rates, &future_values, Some(&itm_mask));

            let exercise_now = regressor.determine_exercise(
                &state.short_rates,
                &state.intrinsic_values,
                &regression_result,
            );

            for path_idx in 0..num_paths {
                if exercise_now[path_idx] {
                    cashflow_values[path_idx] = state.intrinsic_values[path_idx];
                } else if state_idx == exercise_states.len() - 1 {
                    cashflow_values[path_idx] = future_values[path_idx];
                }
            }

            exercise_decisions.push(ExerciseDecision {
                exercise_date: state.exercise_date,
                exercised: exercise_now,
            });

            regression_results.push(regression_result);
        }

        exercise_decisions.reverse();
        regression_results.reverse();

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
    pub fn price_lsmc<P: CurveProvider>(
        kernel: &CallableKernel,
        context: &KernelContext<P>,
        paths: &SimulatedPaths,
        valuation_date_days: i32,
        regressor: &LSMCRegressor,
    ) -> f64 {
        let exercise_states = Self::forward_pass(kernel, context, paths, valuation_date_days);

        if exercise_states.is_empty() {
            return Self::price_deterministic(kernel, context);
        }

        let result = Self::backward_pass(&exercise_states, paths, valuation_date_days, regressor);

        result.option_value
    }
}

/// Result of backward pass LSMC algorithm.
#[derive(Clone, Debug)]
pub struct BackwardPassResult {
    /// Computed option value (discounted expected payoff).
    pub option_value: f64,
    /// Cashflow values per path.
    pub cashflow_values: Vec<f64>,
    /// Exercise decisions at each exercise date.
    pub exercise_decisions: Vec<ExerciseDecision>,
    /// Regression results from LSMC at each exercise date.
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
    /// Date of this exercise opportunity.
    pub exercise_date: i32,
    /// Whether each path exercised at this date.
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
        assert_eq!(paths.find_time_index(0.4), 1);
        assert_eq!(paths.find_time_index(1.5), 2);
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

    #[test]
    fn test_forward_pass_empty_kernel() {
        let kernel = CallableKernel::empty();
        let paths = create_test_paths();
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
        for path_idx in 0..10 {
            assert!((states[0].short_rates[path_idx] - 0.03).abs() < 1e-10);
        }
    }

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
        let inner_kernel = PricingKernel::new(
            vec![19365],
            vec![19363],
            vec![1.0],
            vec![1_000_000.0],
            vec![0.03],
            vec![0.0],
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
        assert!(pv.abs() > 0.0);
    }

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
        assert_eq!(probs.len(), 2);
        for prob in &probs {
            assert!(*prob >= 0.0 && *prob <= 1.0);
        }
    }

    #[test]
    fn test_backward_pass_with_intrinsic_values() {
        let num_paths = 100;
        let mut state = ExerciseState::new(19365, num_paths);
        for i in 0..num_paths {
            if i < 50 {
                state.intrinsic_values[i] = 1.0;
            }
            state.short_rates[i] = 0.03;
        }
        let paths = create_test_paths();
        let regressor = LSMCRegressor::default();
        let result = CallableEngine::backward_pass(&[state], &paths, 19000, &regressor);
        assert_eq!(result.num_exercise_dates(), 1);
    }

    #[test]
    fn test_price_lsmc_empty_kernel() {
        let kernel = CallableKernel::empty();
        let paths = create_test_paths();
        let regressor = LSMCRegressor::default();
        let curves = super::super::FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&curves);
        let price = CallableEngine::price_lsmc(&kernel, &context, &paths, 19000, &regressor);
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
        assert_eq!(result.regression_results.len(), 2);
    }
}
