//! Calibration engine using Levenberg-Marquardt solver.
//!
//! This module provides the core calibration engine that wraps the LM solver
//! from `pricer_core` and integrates with the Calibrator trait.
//!
//! ## Architecture
//!
//! ```text
//! CalibrationEngine
//!   └─> pricer_core::math::solvers::LevenbergMarquardtSolver
//! ```
//!
//! ## Calibration Scope
//!
//! The engine supports different calibration scopes:
//! - `Global`: Calibrate all parameters simultaneously
//! - `TermByTerm`: Calibrate parameters grouped by term
//! - `Piecewise`: Calibrate parameters in segments

use pricer_core::{
    math::solvers::{LMConfig, LMResult, LevenbergMarquardtSolver},
    traits::calibration::{
        CalibrationConfig, CalibrationResult, Calibrator, Constraint, ParameterBounds,
    },
    types::CalibrationError,
};

/// Calibration scope defining how parameters are grouped during calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalibrationScope {
    /// Calibrate all parameters simultaneously (default).
    #[default]
    Global,
    /// Calibrate parameters grouped by term/maturity.
    TermByTerm,
    /// Calibrate parameters in piecewise segments.
    Piecewise,
}

impl CalibrationScope {
    /// Returns the name of the scope.
    pub fn name(&self) -> &'static str {
        match self {
            CalibrationScope::Global => "Global",
            CalibrationScope::TermByTerm => "Term-by-Term",
            CalibrationScope::Piecewise => "Piecewise",
        }
    }

    /// Returns true if this is global calibration.
    pub fn is_global(&self) -> bool { matches!(self, CalibrationScope::Global) }
}

/// Configuration for the calibration engine.
#[derive(Debug, Clone)]
pub struct CalibrationEngineConfig {
    /// LM solver configuration.
    pub lm_config: LMConfig,
    /// Parameter bounds.
    pub bounds: Vec<ParameterBounds>,
    /// Whether to apply bounds constraints.
    pub enforce_bounds: bool,
    /// Calibration scope.
    pub scope: CalibrationScope,
}

impl Default for CalibrationEngineConfig {
    fn default() -> Self {
        Self {
            lm_config: LMConfig::default(),
            bounds: Vec::new(),
            enforce_bounds: true,
            scope: CalibrationScope::Global,
        }
    }
}

impl CalibrationEngineConfig {
    /// Create a new configuration.
    pub fn new(lm_config: LMConfig) -> Self {
        Self {
            lm_config,
            bounds: Vec::new(),
            enforce_bounds: true,
            scope: CalibrationScope::Global,
        }
    }

    /// Set parameter bounds.
    pub fn with_bounds(mut self, bounds: Vec<ParameterBounds>) -> Self {
        self.bounds = bounds;
        self
    }

    /// Set whether to enforce bounds.
    pub fn with_enforce_bounds(mut self, enforce: bool) -> Self {
        self.enforce_bounds = enforce;
        self
    }

    /// Set calibration scope.
    pub fn with_scope(mut self, scope: CalibrationScope) -> Self {
        self.scope = scope;
        self
    }

    /// Create from CalibrationConfig.
    pub fn from_calibration_config(config: &CalibrationConfig) -> Self {
        Self {
            lm_config: LMConfig {
                tolerance: config.tolerance,
                max_iterations: config.max_iterations,
                param_tolerance: config.param_tolerance,
                ..LMConfig::default()
            },
            bounds: Vec::new(),
            enforce_bounds: true,
            scope: CalibrationScope::Global,
        }
    }
}

/// Backward compatibility alias for CalibrationEngineConfig.
pub type ModelCalibratorConfig = CalibrationEngineConfig;

/// Calibration engine using Levenberg-Marquardt.
///
/// This engine wraps the LM solver and provides a convenient
/// interface for calibrating model parameters to market data.
///
/// # Example
///
/// ```
/// use pricer_models::market::calibration::{CalibrationEngine, CalibrationEngineConfig};
/// use pricer_core::math::solvers::LMConfig;
///
/// let config = CalibrationEngineConfig::new(LMConfig::default());
/// let engine = CalibrationEngine::new(config);
///
/// // Define residual function: model(params) - market_data
/// let residuals = |params: &[f64]| -> Vec<f64> {
///     vec![params[0] - 1.0, params[1] - 2.0]
/// };
///
/// let result = engine.calibrate_with_residuals(residuals, vec![0.5, 0.5]);
/// assert!(result.converged);
/// ```
#[derive(Debug, Clone)]
pub struct CalibrationEngine {
    config: CalibrationEngineConfig,
}

impl CalibrationEngine {
    /// Create a new calibration engine.
    pub fn new(config: CalibrationEngineConfig) -> Self { Self { config } }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self {
            config: CalibrationEngineConfig::default(),
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &CalibrationEngineConfig { &self.config }

    /// Get the calibration scope.
    pub fn scope(&self) -> CalibrationScope { self.config.scope }

    /// Calibrate using a residual function.
    ///
    /// The residual function should return a vector of residuals
    /// (model - market) for each observation.
    ///
    /// # Arguments
    ///
    /// * `residuals` - Function computing residuals from parameters
    /// * `initial_params` - Starting parameter values
    ///
    /// # Returns
    ///
    /// Calibration result with final parameters.
    pub fn calibrate_with_residuals<F>(
        &self,
        residuals: F,
        initial_params: Vec<f64>,
    ) -> CalibrationResult<Vec<f64>>
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        // Apply bounds to initial params if needed
        let initial = if self.config.enforce_bounds && !self.config.bounds.is_empty() {
            self.apply_bounds(&initial_params)
        } else {
            initial_params
        };

        // Create constrained residual function
        let bounds = self.config.bounds.clone();
        let enforce = self.config.enforce_bounds && !bounds.is_empty();

        let constrained_residuals = move |params: &[f64]| {
            if enforce {
                let clamped: Vec<f64> = params
                    .iter()
                    .enumerate()
                    .map(|(i, &p)| bounds.get(i).map_or(p, |b| b.clamp(p)))
                    .collect();
                residuals(&clamped)
            } else {
                residuals(params)
            }
        };

        // Run LM solver
        let solver = LevenbergMarquardtSolver::new(self.config.lm_config);
        match solver.solve(constrained_residuals, initial) {
            Ok(lm_result) => {
                let mut result = self.convert_lm_result(lm_result);
                // Apply bounds to final result if enabled
                if enforce {
                    result.params = self.apply_bounds(&result.params);
                }
                result
            }
            Err(e) => {
                let calib_err: CalibrationError = e.into();
                CalibrationResult::not_converged(
                    Vec::new(),
                    calib_err.iterations,
                    calib_err.residual_ss,
                    format!("{}", calib_err),
                )
            }
        }
    }

    /// Calibrate and return CalibrationError on failure.
    pub fn calibrate_or_error<F>(
        &self,
        residuals: F,
        initial_params: Vec<f64>,
    ) -> Result<CalibrationResult<Vec<f64>>, CalibrationError>
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        let result = self.calibrate_with_residuals(residuals, initial_params);
        if result.converged {
            Ok(result)
        } else {
            Err(
                CalibrationError::not_converged(result.iterations, result.residual_ss)
                    .with_parameters(result.params.clone())
                    .with_message(result.message.unwrap_or_default()),
            )
        }
    }

    /// Apply bounds to parameters.
    fn apply_bounds(&self, params: &[f64]) -> Vec<f64> {
        params
            .iter()
            .enumerate()
            .map(|(i, &p)| self.config.bounds.get(i).map_or(p, |b| b.clamp(p)))
            .collect()
    }

    /// Convert LM result to CalibrationResult.
    fn convert_lm_result(&self, lm_result: LMResult) -> CalibrationResult<Vec<f64>> {
        CalibrationResult::with_details(
            lm_result.params,
            lm_result.converged,
            lm_result.iterations,
            lm_result.residual_ss,
            Vec::new(), // LM doesn't return individual residuals
        )
    }
}

/// Backward compatibility alias for CalibrationEngine.
pub type ModelCalibrator = CalibrationEngine;

/// Implement Calibrator trait for a specific market data type.
///
/// This is a helper struct that wraps CalibrationEngine for use with
/// the Calibrator trait.
pub struct GenericCalibrator<F, M> {
    calibrator: CalibrationEngine,
    residual_fn: F,
    constraints: Vec<Constraint>,
    _phantom: std::marker::PhantomData<M>,
}

#[allow(dead_code)]
impl<F, M> GenericCalibrator<F, M>
where
    F: Fn(&[f64], &M) -> Vec<f64>,
{
    /// Create a new generic calibrator.
    pub fn new(config: CalibrationEngineConfig, residual_fn: F) -> Self {
        Self {
            calibrator: CalibrationEngine::new(config),
            residual_fn,
            constraints: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add constraints.
    pub fn with_constraints(mut self, constraints: Vec<Constraint>) -> Self {
        self.constraints = constraints;
        self
    }
}

impl<F, M> Calibrator for GenericCalibrator<F, M>
where
    F: Fn(&[f64], &M) -> Vec<f64>,
{
    type MarketData = M;
    type ModelParams = Vec<f64>;

    fn calibrate(
        &self,
        market_data: &Self::MarketData,
        initial_params: Self::ModelParams,
        _config: &CalibrationConfig,
    ) -> CalibrationResult<Self::ModelParams> {
        let residuals = |params: &[f64]| (self.residual_fn)(params, market_data);
        self.calibrator
            .calibrate_with_residuals(residuals, initial_params)
    }

    fn objective_function(
        &self,
        params: &Self::ModelParams,
        market_data: &Self::MarketData,
    ) -> Vec<f64> {
        (self.residual_fn)(params, market_data)
    }

    fn constraints(&self) -> Vec<Constraint> { self.constraints.clone() }
}

#[cfg(test)]
mod tests {
    use pricer_core::math::solvers::LMConfig;

    use super::*;

    #[test]
    fn test_calibration_scope() {
        assert_eq!(CalibrationScope::default(), CalibrationScope::Global);
        assert!(CalibrationScope::Global.is_global());
        assert!(!CalibrationScope::TermByTerm.is_global());
        assert!(!CalibrationScope::Piecewise.is_global());
        assert_eq!(CalibrationScope::Global.name(), "Global");
        assert_eq!(CalibrationScope::TermByTerm.name(), "Term-by-Term");
        assert_eq!(CalibrationScope::Piecewise.name(), "Piecewise");
    }

    #[test]
    fn test_calibration_engine_new() {
        let config = CalibrationEngineConfig::new(LMConfig::default());
        let engine = CalibrationEngine::new(config);
        assert!(engine.config().lm_config.max_iterations > 0);
        assert!(engine.scope().is_global());
    }

    #[test]
    fn test_calibration_engine_with_defaults() {
        let engine = CalibrationEngine::with_defaults();
        assert!(engine.config().lm_config.tolerance > 0.0);
    }

    #[test]
    fn test_config_with_scope() {
        let config = CalibrationEngineConfig::default().with_scope(CalibrationScope::TermByTerm);
        assert_eq!(config.scope, CalibrationScope::TermByTerm);
    }

    #[test]
    fn test_calibrate_simple_linear() {
        let engine = CalibrationEngine::with_defaults();

        // Minimize (p[0] - 2)^2 + (p[1] - 3)^2
        let residuals = |params: &[f64]| vec![params[0] - 2.0, params[1] - 3.0];

        let result = engine.calibrate_with_residuals(residuals, vec![0.0, 0.0]);

        assert!(result.converged);
        assert!((result.params[0] - 2.0).abs() < 1e-6);
        assert!((result.params[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_calibrate_with_bounds() {
        let config = CalibrationEngineConfig::default()
            .with_bounds(vec![
                ParameterBounds::new(0.0, 1.0),  // param 0 in [0, 1]
                ParameterBounds::new(0.0, 10.0), // param 1 in [0, 10]
            ])
            .with_enforce_bounds(true);

        let engine = CalibrationEngine::new(config);

        // Try to fit to (2, 3), but param 0 is bounded to [0, 1]
        let residuals = |params: &[f64]| vec![params[0] - 2.0, params[1] - 3.0];

        let result = engine.calibrate_with_residuals(residuals, vec![0.5, 0.5]);

        // param[0] should be clamped to upper bound (1.0)
        assert!(result.params[0] <= 1.0 + 1e-10);
        // param[1] should converge to 3.0
        assert!((result.params[1] - 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_calibrate_or_error_success() {
        let engine = CalibrationEngine::with_defaults();

        let residuals = |params: &[f64]| vec![params[0] - 1.0];

        let result = engine.calibrate_or_error(residuals, vec![0.5]);

        assert!(result.is_ok());
        let calib_result = result.unwrap();
        assert!(calib_result.converged);
    }

    #[test]
    fn test_config_from_calibration_config() {
        let calib_config = CalibrationConfig::new(1e-8, 50);
        let config = CalibrationEngineConfig::from_calibration_config(&calib_config);

        assert!((config.lm_config.tolerance - 1e-8).abs() < 1e-15);
        assert_eq!(config.lm_config.max_iterations, 50);
    }

    #[test]
    fn test_generic_calibrator() {
        let config = CalibrationEngineConfig::default();
        let residual_fn = |params: &[f64], target: &Vec<f64>| {
            params.iter().zip(target).map(|(p, t)| p - t).collect()
        };

        let calibrator = GenericCalibrator::new(config, residual_fn);

        let market_data = vec![1.0, 2.0, 3.0];
        let initial = vec![0.0, 0.0, 0.0];

        let result = calibrator.calibrate(&market_data, initial, &CalibrationConfig::default());

        assert!(result.converged);
        for (p, t) in result.params.iter().zip(&market_data) {
            assert!((p - t).abs() < 1e-6);
        }
    }

    #[test]
    fn test_generic_calibrator_objective_function() {
        let config = CalibrationEngineConfig::default();
        let residual_fn = |params: &[f64], target: &Vec<f64>| {
            params.iter().zip(target).map(|(p, t)| p - t).collect()
        };

        let calibrator = GenericCalibrator::new(config, residual_fn);

        let market_data = vec![1.0, 2.0];
        let params = vec![1.5, 2.5];

        let residuals = calibrator.objective_function(&params, &market_data);

        assert_eq!(residuals.len(), 2);
        assert!((residuals[0] - 0.5).abs() < 1e-10);
        assert!((residuals[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_generic_calibrator_constraints() {
        let config = CalibrationEngineConfig::default();
        let residual_fn = |params: &[f64], _target: &Vec<f64>| params.iter().copied().collect();

        let calibrator = GenericCalibrator::new(config, residual_fn)
            .with_constraints(vec![Constraint::positive(0)]);

        let constraints = calibrator.constraints();
        assert_eq!(constraints.len(), 1);
    }

    #[test]
    fn test_exponential_fit() {
        let engine = CalibrationEngine::with_defaults();

        // Fit y = a * exp(-b * x) where true a = 2.0, b = 0.5
        let x_data: [f64; 5] = [0.0, 1.0, 2.0, 3.0, 4.0];
        let y_data: [f64; 5] = [
            2.0,
            2.0 * (-0.5_f64).exp(),
            2.0 * (-1.0_f64).exp(),
            2.0 * (-1.5_f64).exp(),
            2.0 * (-2.0_f64).exp(),
        ];

        let residuals = |params: &[f64]| {
            let a = params[0];
            let b = params[1];
            x_data
                .iter()
                .zip(y_data.iter())
                .map(|(&x, &y)| a * (-b * x).exp() - y)
                .collect()
        };

        let result = engine.calibrate_with_residuals(residuals, vec![1.0, 1.0]);

        assert!(result.converged);
        assert!((result.params[0] - 2.0).abs() < 0.01);
        assert!((result.params[1] - 0.5).abs() < 0.01);
    }

    // Backward compatibility tests
    #[test]
    fn test_backward_compatibility_model_calibrator() {
        let config = ModelCalibratorConfig::new(LMConfig::default());
        let calibrator = ModelCalibrator::new(config);
        assert!(calibrator.config().lm_config.max_iterations > 0);
    }
}
