//! VolCube calibrator trait and implementations.
//!
//! # Requirements: 10.2, 10.3, 10.4
//!
//! This module provides the trait-based abstraction for VolCube calibrators,
//! enabling extensibility with custom calibration strategies.
//!
//! # Extensibility
//!
//! New calibrators can be implemented by:
//! 1. Implementing the `VolCubeCalibrator` trait
//! 2. Registering with `VolCubeBuilder::with_calibrator()`
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::market::volcube::{VolCubeCalibrator, VolCubeConfig, VolCube};
//!
//! struct CustomCalibrator;
//!
//! impl<T: Float + Send + Sync> VolCubeCalibrator<T> for CustomCalibrator {
//!     fn calibrate(
//!         &self,
//!         instruments: &[VolInstrument<T>],
//!         config: &VolCubeConfig,
//!     ) -> Result<VolCube<T>, VolCubeError> {
//!         // Custom calibration logic
//!     }
//! }
//! ```

use num_traits::Float;

use super::config::VolCubeConfig;
use super::cube::VolCube;
use super::error::VolCubeError;
use super::types::VolInstrument;

/// Result type for calibration operations.
pub type CalibrationResult<T> = Result<CalibratorOutput<T>, VolCubeError>;

/// Calibrator output containing the calibrated cube and diagnostics.
#[derive(Debug, Clone)]
pub struct CalibratorOutput<T: Float> {
    /// The calibrated VolCube.
    pub cube: VolCube<T>,
    /// Number of iterations used.
    pub iterations: usize,
    /// Final residual (calibration error).
    pub final_residual: f64,
    /// Per-instrument calibration errors.
    pub instrument_errors: Vec<f64>,
}

impl<T: Float> CalibratorOutput<T> {
    /// Create a new calibrator output.
    pub fn new(cube: VolCube<T>, iterations: usize, final_residual: f64) -> Self {
        Self {
            cube,
            iterations,
            final_residual,
            instrument_errors: Vec::new(),
        }
    }

    /// Add instrument-level errors.
    pub fn with_instrument_errors(mut self, errors: Vec<f64>) -> Self {
        self.instrument_errors = errors;
        self
    }
}

/// Trait for VolCube calibrators.
///
/// # Requirements: 10.2, 10.3
///
/// This trait defines the interface for calibration strategies.
/// Implement this trait to create custom calibrators for different
/// volatility models (SABR, SVI, LocalVol, etc.).
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` for parallel calibration.
pub trait VolCubeCalibrator<T: Float + Send + Sync>: Send + Sync {
    /// Calibrate a VolCube from market instruments.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Market volatility instruments
    /// * `config` - Calibration configuration
    ///
    /// # Returns
    ///
    /// * `Ok(CalibratorOutput)` - Calibrated cube with diagnostics
    /// * `Err(VolCubeError)` - Calibration failure
    fn calibrate(
        &self,
        instruments: &[VolInstrument<T>],
        config: &VolCubeConfig,
    ) -> CalibrationResult<T>;

    /// Get the calibrator name for logging/diagnostics.
    fn name(&self) -> &'static str;

    /// Check if this calibrator supports the given interpolation method.
    fn supports_interpolation(&self, method: &super::config::InterpolationMethod) -> bool;
}

/// Default SABR calibrator.
///
/// Uses Hagan's SABR formula with Levenberg-Marquardt optimisation.
#[derive(Debug, Clone, Copy, Default)]
pub struct SabrCalibrator;

impl SabrCalibrator {
    /// Create a new SABR calibrator.
    pub fn new() -> Self {
        Self
    }
}

impl<T: Float + Send + Sync> VolCubeCalibrator<T> for SabrCalibrator {
    fn calibrate(
        &self,
        instruments: &[VolInstrument<T>],
        config: &VolCubeConfig,
    ) -> CalibrationResult<T> {
        use super::builder::VolCubeBuilder;

        // Delegate to the existing builder logic
        let cube = VolCubeBuilder::new()
            .with_instruments(instruments.to_vec())
            .with_config(config.clone())
            .build()?;

        Ok(CalibratorOutput::new(cube, config.max_iterations, 0.0))
    }

    fn name(&self) -> &'static str {
        "SABR"
    }

    fn supports_interpolation(&self, method: &super::config::InterpolationMethod) -> bool {
        matches!(
            method,
            super::config::InterpolationMethod::Sabr
                | super::config::InterpolationMethod::Linear
                | super::config::InterpolationMethod::FlatVol
        )
    }
}

/// Placeholder for SVI calibrator.
///
/// SVI (Stochastic Volatility Inspired) parameterisation by Jim Gatheral.
/// Provides a smooth, arbitrage-free smile representation.
#[derive(Debug, Clone, Copy, Default)]
pub struct SviCalibrator;

impl SviCalibrator {
    /// Create a new SVI calibrator.
    pub fn new() -> Self {
        Self
    }
}

impl<T: Float + Send + Sync> VolCubeCalibrator<T> for SviCalibrator {
    fn calibrate(
        &self,
        _instruments: &[VolInstrument<T>],
        _config: &VolCubeConfig,
    ) -> CalibrationResult<T> {
        // Placeholder: SVI calibration not yet implemented
        Err(VolCubeError::InvalidInput {
            message: "SVI calibrator not yet implemented".to_string(),
        })
    }

    fn name(&self) -> &'static str {
        "SVI"
    }

    fn supports_interpolation(&self, method: &super::config::InterpolationMethod) -> bool {
        matches!(method, super::config::InterpolationMethod::Svi)
    }
}

/// Placeholder for Local Volatility calibrator.
///
/// Available with the `local-vol` feature flag.
#[cfg(feature = "local-vol")]
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalVolCalibrator;

#[cfg(feature = "local-vol")]
impl LocalVolCalibrator {
    /// Create a new Local Volatility calibrator.
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "local-vol")]
impl<T: Float + Send + Sync> VolCubeCalibrator<T> for LocalVolCalibrator {
    fn calibrate(
        &self,
        _instruments: &[VolInstrument<T>],
        _config: &VolCubeConfig,
    ) -> CalibrationResult<T> {
        // Placeholder: Local Vol calibration not yet implemented
        Err(VolCubeError::InvalidInput {
            message: "Local Volatility calibrator not yet implemented".to_string(),
        })
    }

    fn name(&self) -> &'static str {
        "LocalVol"
    }

    fn supports_interpolation(&self, _method: &super::config::InterpolationMethod) -> bool {
        true // LocalVol can work with any smile representation
    }
}

/// Placeholder for Stochastic Local Volatility calibrator.
///
/// Available with the `stochastic-local-vol` feature flag.
#[cfg(feature = "stochastic-local-vol")]
#[derive(Debug, Clone, Copy, Default)]
pub struct StochasticLocalVolCalibrator;

#[cfg(feature = "stochastic-local-vol")]
impl StochasticLocalVolCalibrator {
    /// Create a new Stochastic Local Volatility calibrator.
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "stochastic-local-vol")]
impl<T: Float + Send + Sync> VolCubeCalibrator<T> for StochasticLocalVolCalibrator {
    fn calibrate(
        &self,
        _instruments: &[VolInstrument<T>],
        _config: &VolCubeConfig,
    ) -> CalibrationResult<T> {
        // Placeholder: SLV calibration not yet implemented
        Err(VolCubeError::InvalidInput {
            message: "Stochastic Local Volatility calibrator not yet implemented".to_string(),
        })
    }

    fn name(&self) -> &'static str {
        "StochasticLocalVol"
    }

    fn supports_interpolation(&self, _method: &super::config::InterpolationMethod) -> bool {
        true // SLV can work with any smile representation
    }
}

/// Type-erased calibrator for dynamic dispatch.
///
/// Use this when you need to store different calibrator types
/// in a collection or pass them across API boundaries.
pub type BoxedCalibrator<T> = Box<dyn VolCubeCalibrator<T>>;

/// Create the default calibrator based on configuration.
pub fn default_calibrator<T: Float + Send + Sync>(_config: &VolCubeConfig) -> BoxedCalibrator<T> {
    Box::new(SabrCalibrator::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // CalibratorOutput Tests
    // ========================================

    fn create_test_cube() -> VolCube<f64> {
        use super::super::{SabrParameterSurface, SabrParams, InstrumentId};

        let expiries = vec![0.5_f64, 1.0];
        let tenors = vec![2.0, 5.0];
        let params = vec![
            vec![SabrParams::new(0.04, 0.5, -0.3, 0.4), SabrParams::new(0.05, 0.5, -0.25, 0.35)],
            vec![SabrParams::new(0.045, 0.5, -0.35, 0.45), SabrParams::new(0.055, 0.5, -0.2, 0.3)],
        ];
        let sabr_surface = SabrParameterSurface::new(expiries, tenors, &params, 0.5).unwrap();

        VolCube::new(
            sabr_surface,
            vec![vec![0.03, 0.035], vec![0.032, 0.038]],
            VolCubeConfig::default(),
            vec![InstrumentId::new("TEST-1"), InstrumentId::new("TEST-2")],
            (0.01, 0.10),
        )
    }

    #[test]
    fn test_calibrator_output_new() {
        let cube = create_test_cube();
        let output = CalibratorOutput::new(cube, 50, 1e-10);
        assert_eq!(output.iterations, 50);
        assert!((output.final_residual - 1e-10).abs() < 1e-15);
        assert!(output.instrument_errors.is_empty());
    }

    #[test]
    fn test_calibrator_output_with_errors() {
        let cube = create_test_cube();
        let errors = vec![0.001, 0.002, 0.003];
        let output = CalibratorOutput::new(cube, 50, 1e-10)
            .with_instrument_errors(errors.clone());
        assert_eq!(output.instrument_errors, errors);
    }

    // ========================================
    // SabrCalibrator Tests
    // ========================================

    #[test]
    fn test_sabr_calibrator_name() {
        let calibrator = SabrCalibrator::new();
        assert_eq!(<SabrCalibrator as VolCubeCalibrator<f64>>::name(&calibrator), "SABR");
    }

    #[test]
    fn test_sabr_calibrator_supports_interpolation() {
        let calibrator = SabrCalibrator::new();
        assert!(<SabrCalibrator as VolCubeCalibrator<f64>>::supports_interpolation(&calibrator, &super::super::config::InterpolationMethod::Sabr));
        assert!(<SabrCalibrator as VolCubeCalibrator<f64>>::supports_interpolation(&calibrator, &super::super::config::InterpolationMethod::Linear));
        assert!(<SabrCalibrator as VolCubeCalibrator<f64>>::supports_interpolation(&calibrator, &super::super::config::InterpolationMethod::FlatVol));
        assert!(!<SabrCalibrator as VolCubeCalibrator<f64>>::supports_interpolation(&calibrator, &super::super::config::InterpolationMethod::Svi));
    }

    // ========================================
    // SviCalibrator Tests
    // ========================================

    #[test]
    fn test_svi_calibrator_name() {
        let calibrator = SviCalibrator::new();
        assert_eq!(<SviCalibrator as VolCubeCalibrator<f64>>::name(&calibrator), "SVI");
    }

    #[test]
    fn test_svi_calibrator_supports_interpolation() {
        let calibrator = SviCalibrator::new();
        assert!(<SviCalibrator as VolCubeCalibrator<f64>>::supports_interpolation(&calibrator, &super::super::config::InterpolationMethod::Svi));
        assert!(!<SviCalibrator as VolCubeCalibrator<f64>>::supports_interpolation(&calibrator, &super::super::config::InterpolationMethod::Sabr));
    }

    #[test]
    fn test_svi_calibrator_not_implemented() {
        let calibrator = SviCalibrator::new();
        let config = VolCubeConfig::default();
        let instruments: Vec<VolInstrument<f64>> = vec![];

        let result: CalibrationResult<f64> = calibrator.calibrate(&instruments, &config);
        assert!(result.is_err());
    }

    // ========================================
    // Default Calibrator Tests
    // ========================================

    #[test]
    fn test_default_calibrator() {
        let config = VolCubeConfig::default();
        let calibrator: BoxedCalibrator<f64> = default_calibrator(&config);
        assert_eq!(calibrator.name(), "SABR");
    }

    // ========================================
    // Feature Flag Tests
    // ========================================

    #[cfg(feature = "local-vol")]
    #[test]
    fn test_local_vol_calibrator_name() {
        let calibrator = LocalVolCalibrator::new();
        assert_eq!(<LocalVolCalibrator as VolCubeCalibrator<f64>>::name(&calibrator), "LocalVol");
    }

    #[cfg(feature = "stochastic-local-vol")]
    #[test]
    fn test_slv_calibrator_name() {
        let calibrator = StochasticLocalVolCalibrator::new();
        assert_eq!(<StochasticLocalVolCalibrator as VolCubeCalibrator<f64>>::name(&calibrator), "StochasticLocalVol");
    }
}
