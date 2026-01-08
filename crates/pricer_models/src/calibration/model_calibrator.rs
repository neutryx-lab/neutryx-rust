//! Generic model calibrator using Levenberg-Marquardt solver.
//!
//! This module provides a generic calibrator that wraps the LM solver
//! and integrates with the Calibrator trait from pricer_core.

use pricer_core::math::solvers::{LMConfig, LMResult, LevenbergMarquardtSolver};
use pricer_core::traits::calibration::{
    CalibrationConfig, CalibrationResult, Calibrator, Constraint, ParameterBounds,
};
use pricer_core::types::CalibrationError;

/// Configuration for the model calibrator.
#[derive(Debug, Clone)]
pub struct ModelCalibratorConfig {
    /// LM solver configuration.
    pub lm_config: LMConfig,
    /// Parameter bounds.
    pub bounds: Vec<ParameterBounds>,
    /// Whether to apply bounds constraints.
    pub enforce_bounds: bool,
}

impl Default for ModelCalibratorConfig {
    fn default() -> Self {
        Self {
            lm_config: LMConfig::default(),
            bounds: Vec::new(),
            enforce_bounds: true,
        }
    }
}

impl ModelCalibratorConfig {
    /// Create a new configuration.
    pub fn new(lm_config: LMConfig) -> Self {
        Self {
            lm_config,
            bounds: Vec::new(),
            enforce_bounds: true,
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
        }
    }
}

/// Generic model calibrator using Levenberg-Marquardt.
///
/// This calibrator wraps the LM solver and provides a convenient
/// interface for calibrating model parameters to market data.
///
/// # Example
///
/// ```
/// use pricer_models::calibration::{ModelCalibrator, ModelCalibratorConfig};
/// use pricer_core::math::solvers::LMConfig;
///
/// let config = ModelCalibratorConfig::new(LMConfig::default());
/// let calibrator = ModelCalibrator::new(config);
///
/// // Define residual function: model(params) - market_data
/// let residuals = |params: &[f64]| -> Vec<f64> {
///     vec![params[0] - 1.0, params[1] - 2.0]
/// };
///
/// let result = calibrator.calibrate_with_residuals(residuals, vec![0.5, 0.5]);
/// assert!(result.converged);
/// ```
#[derive(Debug, Clone)]
pub struct ModelCalibrator {
    config: ModelCalibratorConfig,
}

impl ModelCalibrator {
    /// Create a new model calibrator.
    pub fn new(config: ModelCalibratorConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self {
            config: ModelCalibratorConfig::default(),
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &ModelCalibratorConfig {
        &self.config
    }

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
        // Always use bounds-aware version (handles empty bounds correctly)
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
            Err(CalibrationError::not_converged(
                result.iterations,
                result.residual_ss,
            )
            .with_parameters(result.params.clone())
            .with_message(result.message.unwrap_or_default()))
        }
    }

    /// Apply bounds to parameters.
    fn apply_bounds(&self, params: &[f64]) -> Vec<f64> {
        params
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                self.config.bounds.get(i).map_or(p, |b| b.clamp(p))
            })
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

/// Implement Calibrator trait for a specific market data type.
///
/// This is a helper struct that wraps ModelCalibrator for use with
/// the Calibrator trait.
pub struct GenericCalibrator<F, M> {
    calibrator: ModelCalibrator,
    residual_fn: F,
    constraints: Vec<Constraint>,
    _phantom: std::marker::PhantomData<M>,
}

impl<F, M> GenericCalibrator<F, M>
where
    F: Fn(&[f64], &M) -> Vec<f64>,
{
    /// Create a new generic calibrator.
    pub fn new(config: ModelCalibratorConfig, residual_fn: F) -> Self {
        Self {
            calibrator: ModelCalibrator::new(config),
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
        self.calibrator.calibrate_with_residuals(residuals, initial_params)
    }

    fn objective_function(
        &self,
        params: &Self::ModelParams,
        market_data: &Self::MarketData,
    ) -> Vec<f64> {
        (self.residual_fn)(params, market_data)
    }

    fn constraints(&self) -> Vec<Constraint> {
        self.constraints.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pricer_core::math::solvers::LMConfig;

    #[test]
    fn test_model_calibrator_new() {
        let config = ModelCalibratorConfig::new(LMConfig::default());
        let calibrator = ModelCalibrator::new(config);
        assert!(calibrator.config().lm_config.max_iterations > 0);
    }

    #[test]
    fn test_model_calibrator_with_defaults() {
        let calibrator = ModelCalibrator::with_defaults();
        assert!(calibrator.config().lm_config.tolerance > 0.0);
    }

    #[test]
    fn test_calibrate_simple_linear() {
        let calibrator = ModelCalibrator::with_defaults();

        // Minimize (p[0] - 2)^2 + (p[1] - 3)^2
        let residuals = |params: &[f64]| vec![params[0] - 2.0, params[1] - 3.0];

        let result = calibrator.calibrate_with_residuals(residuals, vec![0.0, 0.0]);

        assert!(result.converged);
        assert!((result.params[0] - 2.0).abs() < 1e-6);
        assert!((result.params[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_calibrate_with_bounds() {
        let config = ModelCalibratorConfig::default()
            .with_bounds(vec![
                ParameterBounds::new(0.0, 1.0), // param 0 in [0, 1]
                ParameterBounds::new(0.0, 10.0), // param 1 in [0, 10]
            ])
            .with_enforce_bounds(true);

        let calibrator = ModelCalibrator::new(config);

        // Try to fit to (2, 3), but param 0 is bounded to [0, 1]
        let residuals = |params: &[f64]| vec![params[0] - 2.0, params[1] - 3.0];

        let result = calibrator.calibrate_with_residuals(residuals, vec![0.5, 0.5]);

        // param[0] should be clamped to upper bound (1.0)
        assert!(result.params[0] <= 1.0 + 1e-10);
        // param[1] should converge to 3.0
        assert!((result.params[1] - 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_calibrate_or_error_success() {
        let calibrator = ModelCalibrator::with_defaults();

        let residuals = |params: &[f64]| vec![params[0] - 1.0];

        let result = calibrator.calibrate_or_error(residuals, vec![0.5]);

        assert!(result.is_ok());
        let calib_result = result.unwrap();
        assert!(calib_result.converged);
    }

    #[test]
    fn test_config_from_calibration_config() {
        let calib_config = CalibrationConfig::new(1e-8, 50);
        let config = ModelCalibratorConfig::from_calibration_config(&calib_config);

        assert!((config.lm_config.tolerance - 1e-8).abs() < 1e-15);
        assert_eq!(config.lm_config.max_iterations, 50);
    }

    #[test]
    fn test_generic_calibrator() {
        let config = ModelCalibratorConfig::default();
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
        let config = ModelCalibratorConfig::default();
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
        let config = ModelCalibratorConfig::default();
        let residual_fn = |params: &[f64], _target: &Vec<f64>| {
            params.iter().map(|p| *p).collect()
        };

        let calibrator = GenericCalibrator::new(config, residual_fn)
            .with_constraints(vec![Constraint::positive(0)]);

        let constraints = calibrator.constraints();
        assert_eq!(constraints.len(), 1);
    }

    #[test]
    fn test_exponential_fit() {
        let calibrator = ModelCalibrator::with_defaults();

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

        let result = calibrator.calibrate_with_residuals(residuals, vec![1.0, 1.0]);

        assert!(result.converged);
        assert!((result.params[0] - 2.0).abs() < 0.01);
        assert!((result.params[1] - 0.5).abs() < 0.01);
    }
}
