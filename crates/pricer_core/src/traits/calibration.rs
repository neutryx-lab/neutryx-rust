//! Calibration traits and result types.
//!
//! This module defines the core abstractions for model calibration:
//! - [`Calibrator`]: Trait for calibrating model parameters
//! - [`CalibrationResult`]: Result of a calibration run
//! - [`Constraint`]: Parameter constraints during calibration
//!
//! # Example
//!
//! ```
//! use pricer_core::traits::calibration::{
//!     Calibrator, CalibrationResult, CalibrationConfig,
//!     ParameterBounds, Constraint,
//! };
//!
//! // Define a simple model calibrator
//! struct SimpleCalibrator {
//!     target_values: Vec<f64>,
//! }
//!
//! impl Calibrator for SimpleCalibrator {
//!     type MarketData = Vec<f64>;
//!     type ModelParams = Vec<f64>;
//!
//!     fn calibrate(
//!         &self,
//!         market_data: &Self::MarketData,
//!         initial_params: Self::ModelParams,
//!         config: &CalibrationConfig,
//!     ) -> CalibrationResult<Self::ModelParams> {
//!         // Simple calibration that just returns initial params
//!         CalibrationResult::converged(initial_params, 1, 0.0)
//!     }
//!
//!     fn objective_function(
//!         &self,
//!         params: &Self::ModelParams,
//!         market_data: &Self::MarketData,
//!     ) -> Vec<f64> {
//!         // Return residuals
//!         params.iter().zip(market_data).map(|(p, m)| p - m).collect()
//!     }
//!
//!     fn constraints(&self) -> Vec<Constraint> {
//!         vec![
//!             Constraint::bounds(0, 0.0, 1.0), // param 0 in [0, 1]
//!         ]
//!     }
//! }
//! ```

use std::fmt;

/// Configuration for calibration process.
///
/// Controls convergence criteria and iteration limits.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationConfig {
    /// Maximum number of iterations.
    pub max_iterations: usize,
    /// Convergence tolerance for residual norm.
    pub tolerance: f64,
    /// Tolerance for parameter change.
    pub param_tolerance: f64,
    /// Whether to use finite differences for Jacobian (vs AD).
    pub use_finite_differences: bool,
    /// Step size for finite differences.
    pub finite_diff_step: f64,
    /// Verbosity level (0 = silent, 1 = summary, 2 = iterations).
    pub verbosity: u8,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-8,
            param_tolerance: 1e-10,
            use_finite_differences: true,
            finite_diff_step: 1e-8,
            verbosity: 0,
        }
    }
}

impl CalibrationConfig {
    /// Create a new configuration with specified tolerance and iterations.
    pub fn new(tolerance: f64, max_iterations: usize) -> Self {
        Self {
            tolerance,
            max_iterations,
            ..Default::default()
        }
    }

    /// Create a fast configuration for quick calibrations.
    pub fn fast() -> Self {
        Self {
            max_iterations: 50,
            tolerance: 1e-6,
            param_tolerance: 1e-8,
            ..Default::default()
        }
    }

    /// Create a high precision configuration.
    pub fn high_precision() -> Self {
        Self {
            max_iterations: 500,
            tolerance: 1e-12,
            param_tolerance: 1e-14,
            ..Default::default()
        }
    }
}

/// Result of a calibration run.
///
/// Contains the final parameters, convergence status, and diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationResult<P> {
    /// Final calibrated parameters.
    pub params: P,
    /// Whether calibration converged.
    pub converged: bool,
    /// Number of iterations performed.
    pub iterations: usize,
    /// Final residual sum of squares.
    pub residual_ss: f64,
    /// Individual residuals at final params.
    pub residuals: Vec<f64>,
    /// Optional message with convergence details.
    pub message: Option<String>,
}

impl<P> CalibrationResult<P> {
    /// Create a successful (converged) result.
    pub fn converged(params: P, iterations: usize, residual_ss: f64) -> Self {
        Self {
            params,
            converged: true,
            iterations,
            residual_ss,
            residuals: Vec::new(),
            message: None,
        }
    }

    /// Create a failed (not converged) result.
    pub fn not_converged(params: P, iterations: usize, residual_ss: f64, reason: String) -> Self {
        Self {
            params,
            converged: false,
            iterations,
            residual_ss,
            residuals: Vec::new(),
            message: Some(reason),
        }
    }

    /// Create a result with full details.
    pub fn with_details(
        params: P,
        converged: bool,
        iterations: usize,
        residual_ss: f64,
        residuals: Vec<f64>,
    ) -> Self {
        Self {
            params,
            converged,
            iterations,
            residual_ss,
            residuals,
            message: None,
        }
    }

    /// Add a message to the result.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Get root mean square error.
    pub fn rmse(&self) -> f64 {
        if self.residuals.is_empty() {
            self.residual_ss.sqrt()
        } else {
            (self.residual_ss / self.residuals.len() as f64).sqrt()
        }
    }
}

impl<P: fmt::Debug> fmt::Display for CalibrationResult<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CalibrationResult {{ converged: {}, iterations: {}, RMSE: {:.6e} }}",
            self.converged,
            self.iterations,
            self.rmse()
        )
    }
}

/// Bounds for a single parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterBounds {
    /// Minimum allowed value.
    pub min: f64,
    /// Maximum allowed value.
    pub max: f64,
}

impl ParameterBounds {
    /// Create new bounds.
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    /// Create bounds for a strictly positive parameter.
    pub fn positive() -> Self {
        Self {
            min: 1e-10,
            max: f64::INFINITY,
        }
    }

    /// Create bounds for a non-negative parameter.
    pub fn non_negative() -> Self {
        Self {
            min: 0.0,
            max: f64::INFINITY,
        }
    }

    /// Create bounds for a parameter in [0, 1].
    pub fn unit_interval() -> Self {
        Self { min: 0.0, max: 1.0 }
    }

    /// Create unbounded.
    pub fn unbounded() -> Self {
        Self {
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
        }
    }

    /// Check if a value is within bounds.
    pub fn contains(&self, value: f64) -> bool {
        value >= self.min && value <= self.max
    }

    /// Clamp a value to bounds.
    pub fn clamp(&self, value: f64) -> f64 {
        value.clamp(self.min, self.max)
    }
}

impl Default for ParameterBounds {
    fn default() -> Self {
        Self::unbounded()
    }
}

/// Constraint types for calibration.
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// Parameter bounds constraint.
    Bounds {
        /// Parameter index.
        param_index: usize,
        /// Bounds for the parameter.
        bounds: ParameterBounds,
    },
    /// Linear constraint: sum(coefficients * params) <= rhs.
    LinearInequality {
        /// Coefficients for each parameter.
        coefficients: Vec<f64>,
        /// Right-hand side value.
        rhs: f64,
    },
    /// Linear equality: sum(coefficients * params) = rhs.
    LinearEquality {
        /// Coefficients for each parameter.
        coefficients: Vec<f64>,
        /// Right-hand side value.
        rhs: f64,
    },
    /// Custom constraint function (returns violation amount, 0 = satisfied).
    Custom {
        /// Name for debugging.
        name: String,
        /// Constraint function (params -> violation).
        #[allow(clippy::type_complexity)]
        constraint_fn: fn(&[f64]) -> f64,
    },
}

impl Constraint {
    /// Create a bounds constraint.
    pub fn bounds(param_index: usize, min: f64, max: f64) -> Self {
        Self::Bounds {
            param_index,
            bounds: ParameterBounds::new(min, max),
        }
    }

    /// Create a positive parameter constraint.
    pub fn positive(param_index: usize) -> Self {
        Self::Bounds {
            param_index,
            bounds: ParameterBounds::positive(),
        }
    }

    /// Create a non-negative parameter constraint.
    pub fn non_negative(param_index: usize) -> Self {
        Self::Bounds {
            param_index,
            bounds: ParameterBounds::non_negative(),
        }
    }

    /// Create a unit interval constraint.
    pub fn unit_interval(param_index: usize) -> Self {
        Self::Bounds {
            param_index,
            bounds: ParameterBounds::unit_interval(),
        }
    }

    /// Check if a parameter set satisfies this constraint.
    pub fn is_satisfied(&self, params: &[f64]) -> bool {
        match self {
            Constraint::Bounds { param_index, bounds } => {
                params.get(*param_index).map_or(false, |&v| bounds.contains(v))
            }
            Constraint::LinearInequality { coefficients, rhs } => {
                let sum: f64 = coefficients
                    .iter()
                    .zip(params)
                    .map(|(c, p)| c * p)
                    .sum();
                sum <= *rhs
            }
            Constraint::LinearEquality { coefficients, rhs } => {
                let sum: f64 = coefficients
                    .iter()
                    .zip(params)
                    .map(|(c, p)| c * p)
                    .sum();
                (sum - *rhs).abs() < 1e-10
            }
            Constraint::Custom { constraint_fn, .. } => constraint_fn(params) <= 0.0,
        }
    }

    /// Get the violation amount (0 = satisfied, positive = violated).
    pub fn violation(&self, params: &[f64]) -> f64 {
        match self {
            Constraint::Bounds { param_index, bounds } => {
                params.get(*param_index).map_or(f64::INFINITY, |&v| {
                    if v < bounds.min {
                        bounds.min - v
                    } else if v > bounds.max {
                        v - bounds.max
                    } else {
                        0.0
                    }
                })
            }
            Constraint::LinearInequality { coefficients, rhs } => {
                let sum: f64 = coefficients
                    .iter()
                    .zip(params)
                    .map(|(c, p)| c * p)
                    .sum();
                (sum - *rhs).max(0.0)
            }
            Constraint::LinearEquality { coefficients, rhs } => {
                let sum: f64 = coefficients
                    .iter()
                    .zip(params)
                    .map(|(c, p)| c * p)
                    .sum();
                (sum - *rhs).abs()
            }
            Constraint::Custom { constraint_fn, .. } => constraint_fn(params).max(0.0),
        }
    }
}

/// Trait for model calibrators.
///
/// Defines the interface for calibrating model parameters to market data.
///
/// # Type Parameters
///
/// * `MarketData` - The type of market data used for calibration
/// * `ModelParams` - The type of model parameters being calibrated
///
/// # Example
///
/// ```
/// use pricer_core::traits::calibration::{
///     Calibrator, CalibrationResult, CalibrationConfig, Constraint,
/// };
///
/// struct MyCalibrator;
///
/// impl Calibrator for MyCalibrator {
///     type MarketData = Vec<f64>;
///     type ModelParams = [f64; 2];
///
///     fn calibrate(
///         &self,
///         market_data: &Self::MarketData,
///         initial_params: Self::ModelParams,
///         config: &CalibrationConfig,
///     ) -> CalibrationResult<Self::ModelParams> {
///         // Calibration logic here...
///         CalibrationResult::converged(initial_params, 0, 0.0)
///     }
///
///     fn objective_function(
///         &self,
///         params: &Self::ModelParams,
///         market_data: &Self::MarketData,
///     ) -> Vec<f64> {
///         // Return residuals
///         vec![0.0; market_data.len()]
///     }
///
///     fn constraints(&self) -> Vec<Constraint> {
///         vec![]
///     }
/// }
/// ```
pub trait Calibrator {
    /// Type of market data used for calibration.
    type MarketData;
    /// Type of model parameters.
    type ModelParams;

    /// Calibrate model parameters to match market data.
    ///
    /// # Arguments
    ///
    /// * `market_data` - Market observations to fit
    /// * `initial_params` - Starting point for optimization
    /// * `config` - Calibration configuration
    ///
    /// # Returns
    ///
    /// Calibration result with final parameters and diagnostics.
    fn calibrate(
        &self,
        market_data: &Self::MarketData,
        initial_params: Self::ModelParams,
        config: &CalibrationConfig,
    ) -> CalibrationResult<Self::ModelParams>;

    /// Compute the objective function residuals.
    ///
    /// Returns a vector of residuals (model - market) for each observation.
    ///
    /// # Arguments
    ///
    /// * `params` - Current model parameters
    /// * `market_data` - Market observations
    ///
    /// # Returns
    ///
    /// Vector of residuals.
    fn objective_function(
        &self,
        params: &Self::ModelParams,
        market_data: &Self::MarketData,
    ) -> Vec<f64>;

    /// Get parameter constraints.
    ///
    /// Returns a vector of constraints that must be satisfied.
    fn constraints(&self) -> Vec<Constraint>;

    /// Calibrate with default configuration.
    fn calibrate_default(
        &self,
        market_data: &Self::MarketData,
        initial_params: Self::ModelParams,
    ) -> CalibrationResult<Self::ModelParams> {
        self.calibrate(market_data, initial_params, &CalibrationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // CalibrationConfig Tests
    // ========================================

    #[test]
    fn test_config_default() {
        let config = CalibrationConfig::default();
        assert_eq!(config.max_iterations, 100);
        assert!(config.tolerance > 0.0);
    }

    #[test]
    fn test_config_new() {
        let config = CalibrationConfig::new(1e-6, 50);
        assert_eq!(config.max_iterations, 50);
        assert!((config.tolerance - 1e-6).abs() < 1e-15);
    }

    #[test]
    fn test_config_fast() {
        let config = CalibrationConfig::fast();
        assert!(config.max_iterations <= 50);
        assert!(config.tolerance >= 1e-8);
    }

    #[test]
    fn test_config_high_precision() {
        let config = CalibrationConfig::high_precision();
        assert!(config.max_iterations >= 500);
        assert!(config.tolerance <= 1e-10);
    }

    // ========================================
    // CalibrationResult Tests
    // ========================================

    #[test]
    fn test_result_converged() {
        let result: CalibrationResult<Vec<f64>> =
            CalibrationResult::converged(vec![1.0, 2.0], 10, 0.001);
        assert!(result.converged);
        assert_eq!(result.iterations, 10);
        assert!((result.residual_ss - 0.001).abs() < 1e-15);
    }

    #[test]
    fn test_result_not_converged() {
        let result: CalibrationResult<Vec<f64>> = CalibrationResult::not_converged(
            vec![1.0],
            100,
            1.0,
            "Max iterations reached".to_string(),
        );
        assert!(!result.converged);
        assert!(result.message.is_some());
    }

    #[test]
    fn test_result_with_details() {
        let result: CalibrationResult<Vec<f64>> = CalibrationResult::with_details(
            vec![1.0, 2.0],
            true,
            15,
            0.004,
            vec![0.01, 0.02, 0.01, 0.02],
        );
        assert!(result.converged);
        assert_eq!(result.residuals.len(), 4);
    }

    #[test]
    fn test_result_with_message() {
        let result: CalibrationResult<f64> =
            CalibrationResult::converged(1.0, 5, 0.0).with_message("Success!");
        assert_eq!(result.message, Some("Success!".to_string()));
    }

    #[test]
    fn test_result_rmse() {
        let result: CalibrationResult<f64> =
            CalibrationResult::with_details(1.0, true, 10, 4.0, vec![1.0, 1.0, 1.0, 1.0]);
        let rmse = result.rmse();
        assert!((rmse - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_result_display() {
        let result: CalibrationResult<f64> = CalibrationResult::converged(1.0, 10, 0.0001);
        let display = format!("{}", result);
        assert!(display.contains("converged: true"));
        assert!(display.contains("iterations: 10"));
    }

    // ========================================
    // ParameterBounds Tests
    // ========================================

    #[test]
    fn test_bounds_new() {
        let bounds = ParameterBounds::new(0.0, 1.0);
        assert_eq!(bounds.min, 0.0);
        assert_eq!(bounds.max, 1.0);
    }

    #[test]
    fn test_bounds_positive() {
        let bounds = ParameterBounds::positive();
        assert!(bounds.min > 0.0);
    }

    #[test]
    fn test_bounds_non_negative() {
        let bounds = ParameterBounds::non_negative();
        assert_eq!(bounds.min, 0.0);
    }

    #[test]
    fn test_bounds_unit_interval() {
        let bounds = ParameterBounds::unit_interval();
        assert_eq!(bounds.min, 0.0);
        assert_eq!(bounds.max, 1.0);
    }

    #[test]
    fn test_bounds_unbounded() {
        let bounds = ParameterBounds::unbounded();
        assert!(bounds.min == f64::NEG_INFINITY);
        assert!(bounds.max == f64::INFINITY);
    }

    #[test]
    fn test_bounds_contains() {
        let bounds = ParameterBounds::new(0.0, 1.0);
        assert!(bounds.contains(0.5));
        assert!(bounds.contains(0.0));
        assert!(bounds.contains(1.0));
        assert!(!bounds.contains(-0.1));
        assert!(!bounds.contains(1.1));
    }

    #[test]
    fn test_bounds_clamp() {
        let bounds = ParameterBounds::new(0.0, 1.0);
        assert_eq!(bounds.clamp(0.5), 0.5);
        assert_eq!(bounds.clamp(-0.5), 0.0);
        assert_eq!(bounds.clamp(1.5), 1.0);
    }

    // ========================================
    // Constraint Tests
    // ========================================

    #[test]
    fn test_constraint_bounds() {
        let c = Constraint::bounds(0, 0.0, 1.0);
        assert!(c.is_satisfied(&[0.5]));
        assert!(!c.is_satisfied(&[1.5]));
    }

    #[test]
    fn test_constraint_positive() {
        let c = Constraint::positive(0);
        assert!(c.is_satisfied(&[0.001]));
        assert!(!c.is_satisfied(&[-0.001]));
    }

    #[test]
    fn test_constraint_non_negative() {
        let c = Constraint::non_negative(0);
        assert!(c.is_satisfied(&[0.0]));
        assert!(!c.is_satisfied(&[-0.001]));
    }

    #[test]
    fn test_constraint_unit_interval() {
        let c = Constraint::unit_interval(0);
        assert!(c.is_satisfied(&[0.5]));
        assert!(!c.is_satisfied(&[1.1]));
    }

    #[test]
    fn test_constraint_linear_inequality() {
        // x + y <= 1
        let c = Constraint::LinearInequality {
            coefficients: vec![1.0, 1.0],
            rhs: 1.0,
        };
        assert!(c.is_satisfied(&[0.3, 0.3]));
        assert!(!c.is_satisfied(&[0.6, 0.6]));
    }

    #[test]
    fn test_constraint_linear_equality() {
        // x + y = 1
        let c = Constraint::LinearEquality {
            coefficients: vec![1.0, 1.0],
            rhs: 1.0,
        };
        assert!(c.is_satisfied(&[0.5, 0.5]));
        assert!(!c.is_satisfied(&[0.5, 0.6]));
    }

    #[test]
    fn test_constraint_custom() {
        // x^2 + y^2 <= 1
        let c = Constraint::Custom {
            name: "unit_circle".to_string(),
            constraint_fn: |params: &[f64]| params[0] * params[0] + params[1] * params[1] - 1.0,
        };
        assert!(c.is_satisfied(&[0.5, 0.5]));
        assert!(!c.is_satisfied(&[1.0, 1.0]));
    }

    #[test]
    fn test_constraint_violation() {
        let c = Constraint::bounds(0, 0.0, 1.0);
        assert_eq!(c.violation(&[0.5]), 0.0);
        assert!((c.violation(&[1.5]) - 0.5).abs() < 1e-10);
        assert!((c.violation(&[-0.5]) - 0.5).abs() < 1e-10);
    }

    // ========================================
    // Calibrator Trait Tests
    // ========================================

    struct MockCalibrator {
        target: Vec<f64>,
    }

    impl Calibrator for MockCalibrator {
        type MarketData = Vec<f64>;
        type ModelParams = Vec<f64>;

        fn calibrate(
            &self,
            market_data: &Self::MarketData,
            initial_params: Self::ModelParams,
            _config: &CalibrationConfig,
        ) -> CalibrationResult<Self::ModelParams> {
            // Mock: just return market data as params
            CalibrationResult::converged(market_data.clone(), 1, 0.0)
        }

        fn objective_function(
            &self,
            params: &Self::ModelParams,
            market_data: &Self::MarketData,
        ) -> Vec<f64> {
            params
                .iter()
                .zip(market_data)
                .map(|(p, m)| p - m)
                .collect()
        }

        fn constraints(&self) -> Vec<Constraint> {
            vec![Constraint::positive(0)]
        }
    }

    #[test]
    fn test_calibrator_calibrate() {
        let calibrator = MockCalibrator {
            target: vec![1.0, 2.0],
        };
        let market_data = vec![1.0, 2.0];
        let initial = vec![0.5, 0.5];
        let config = CalibrationConfig::default();

        let result = calibrator.calibrate(&market_data, initial, &config);
        assert!(result.converged);
    }

    #[test]
    fn test_calibrator_objective_function() {
        let calibrator = MockCalibrator {
            target: vec![1.0, 2.0],
        };
        let market_data = vec![1.0, 2.0];
        let params = vec![1.1, 2.1];

        let residuals = calibrator.objective_function(&params, &market_data);
        assert_eq!(residuals.len(), 2);
        assert!((residuals[0] - 0.1).abs() < 1e-10);
        assert!((residuals[1] - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_calibrator_constraints() {
        let calibrator = MockCalibrator {
            target: vec![1.0],
        };
        let constraints = calibrator.constraints();
        assert_eq!(constraints.len(), 1);
    }

    #[test]
    fn test_calibrator_calibrate_default() {
        let calibrator = MockCalibrator {
            target: vec![1.0],
        };
        let market_data = vec![1.0];
        let initial = vec![0.5];

        let result = calibrator.calibrate_default(&market_data, initial);
        assert!(result.converged);
    }
}
