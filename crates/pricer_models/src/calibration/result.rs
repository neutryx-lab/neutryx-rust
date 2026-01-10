//! Calibration result types.
//!
//! # Requirement: 6.2
//!
//! This module defines calibration result types that contain
//! calibrated parameters and diagnostic information.

use std::time::Duration;

/// Calibration diagnostics.
///
/// Contains detailed information about the calibration process
/// for analysis and debugging.
#[derive(Debug, Clone)]
pub struct CalibrationDiagnostics {
    /// Number of iterations performed
    pub iterations: usize,
    /// Final objective function value (sum of squared errors)
    pub final_residual: f64,
    /// Root mean squared error
    pub rmse: f64,
    /// Maximum absolute error
    pub max_error: f64,
    /// Calibration duration
    pub duration: Duration,
    /// Whether AD was used for gradients
    pub used_ad: bool,
    /// Individual errors per instrument (if available)
    pub instrument_errors: Option<Vec<f64>>,
    /// Parameter sensitivities (Jacobian diagonal)
    pub parameter_sensitivities: Option<Vec<f64>>,
}

impl Default for CalibrationDiagnostics {
    fn default() -> Self {
        Self {
            iterations: 0,
            final_residual: f64::MAX,
            rmse: f64::MAX,
            max_error: f64::MAX,
            duration: Duration::ZERO,
            used_ad: false,
            instrument_errors: None,
            parameter_sensitivities: None,
        }
    }
}

impl CalibrationDiagnostics {
    /// Create new diagnostics with basic information.
    pub fn new(iterations: usize, final_residual: f64, duration: Duration) -> Self {
        let rmse = (final_residual / iterations.max(1) as f64).sqrt();
        Self {
            iterations,
            final_residual,
            rmse,
            max_error: f64::MAX,
            duration,
            used_ad: false,
            instrument_errors: None,
            parameter_sensitivities: None,
        }
    }

    /// Set the maximum absolute error.
    pub fn with_max_error(mut self, max_error: f64) -> Self {
        self.max_error = max_error;
        self
    }

    /// Set whether AD was used.
    pub fn with_ad_flag(mut self, used_ad: bool) -> Self {
        self.used_ad = used_ad;
        self
    }

    /// Set individual instrument errors.
    pub fn with_instrument_errors(mut self, errors: Vec<f64>) -> Self {
        // Compute RMSE and max error from instrument errors
        if !errors.is_empty() {
            let n = errors.len() as f64;
            self.rmse = (errors.iter().map(|e| e * e).sum::<f64>() / n).sqrt();
            self.max_error = errors.iter().map(|e| e.abs()).fold(0.0_f64, f64::max);
        }
        self.instrument_errors = Some(errors);
        self
    }

    /// Set parameter sensitivities.
    pub fn with_sensitivities(mut self, sensitivities: Vec<f64>) -> Self {
        self.parameter_sensitivities = Some(sensitivities);
        self
    }

    /// Check if calibration quality is acceptable.
    ///
    /// # Arguments
    ///
    /// * `tolerance` - Maximum acceptable RMSE
    pub fn is_quality_acceptable(&self, tolerance: f64) -> bool {
        self.rmse <= tolerance
    }
}

/// Calibration result.
///
/// # Requirement: 6.2
///
/// Contains calibrated parameters and diagnostic information.
/// Generic over the parameter type P to support model-specific results.
#[derive(Debug, Clone)]
pub struct CalibrationResult<P> {
    /// Calibrated parameters
    pub parameters: P,
    /// Whether calibration converged successfully
    pub converged: bool,
    /// Calibration diagnostics
    pub diagnostics: CalibrationDiagnostics,
}

impl<P> CalibrationResult<P> {
    /// Create a successful calibration result.
    pub fn success(parameters: P, diagnostics: CalibrationDiagnostics) -> Self {
        Self {
            parameters,
            converged: true,
            diagnostics,
        }
    }

    /// Create a failed calibration result.
    ///
    /// Used when calibration did not converge but still returns best-effort parameters.
    pub fn failure(parameters: P, diagnostics: CalibrationDiagnostics) -> Self {
        Self {
            parameters,
            converged: false,
            diagnostics,
        }
    }

    /// Get the calibrated parameters.
    pub fn params(&self) -> &P {
        &self.parameters
    }

    /// Get the calibration diagnostics.
    pub fn diagnostics(&self) -> &CalibrationDiagnostics {
        &self.diagnostics
    }

    /// Check if calibration was successful.
    pub fn is_success(&self) -> bool {
        self.converged
    }

    /// Get the RMSE of the calibration.
    pub fn rmse(&self) -> f64 {
        self.diagnostics.rmse
    }

    /// Map the parameter type to a different type.
    pub fn map<Q, F>(self, f: F) -> CalibrationResult<Q>
    where
        F: FnOnce(P) -> Q,
    {
        CalibrationResult {
            parameters: f(self.parameters),
            converged: self.converged,
            diagnostics: self.diagnostics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_default() {
        let diag = CalibrationDiagnostics::default();
        assert_eq!(diag.iterations, 0);
        assert_eq!(diag.final_residual, f64::MAX);
    }

    #[test]
    fn test_diagnostics_new() {
        let diag = CalibrationDiagnostics::new(100, 1e-6, Duration::from_millis(500));
        assert_eq!(diag.iterations, 100);
        assert_eq!(diag.final_residual, 1e-6);
        assert_eq!(diag.duration, Duration::from_millis(500));
    }

    #[test]
    fn test_diagnostics_with_instrument_errors() {
        let errors = vec![0.01, 0.02, 0.03];
        let diag = CalibrationDiagnostics::default().with_instrument_errors(errors.clone());

        assert!(diag.instrument_errors.is_some());
        assert!((diag.max_error - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_result_success() {
        let params = vec![0.2, 0.4, -0.3];
        let diag = CalibrationDiagnostics::new(50, 1e-8, Duration::from_millis(100));
        let result = CalibrationResult::success(params.clone(), diag);

        assert!(result.is_success());
        assert_eq!(result.params(), &params);
    }

    #[test]
    fn test_result_failure() {
        let params = vec![0.2, 0.4, -0.3];
        let diag = CalibrationDiagnostics::new(1000, 1e-2, Duration::from_secs(5));
        let result = CalibrationResult::failure(params, diag);

        assert!(!result.is_success());
    }

    #[test]
    fn test_result_map() {
        let params = vec![1.0, 2.0];
        let diag = CalibrationDiagnostics::default();
        let result = CalibrationResult::success(params, diag);

        let mapped = result.map(|p| p.iter().sum::<f64>());
        assert_eq!(mapped.parameters, 3.0);
    }
}
