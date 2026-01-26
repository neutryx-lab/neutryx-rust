//! FX Volatility Surface Configuration.
//!
//! This module provides configuration types for FX volatility surface
//! calibration.

use super::types::ExpiryInterpolation;
use crate::market::volcube::{ExtrapolationMethod, InterpolationMethod, StrikeAxisType};

// ============================================================================
// FxVolSurfaceConfig
// ============================================================================

/// FX Volatility Surface Configuration.
///
/// Comprehensive configuration for FX vol surface calibration and
/// interpolation. Uses the builder pattern for ergonomic configuration.
///
/// # Example
///
/// ```rust
/// use pricer_models::market::fx_calibration::FxVolSurfaceConfig;
/// use pricer_models::market::volcube::InterpolationMethod;
///
/// let config = FxVolSurfaceConfig::default()
///     .with_smile_interpolation(InterpolationMethod::Sabr)
///     .with_sabr_beta(0.5)
///     .with_allow_extrapolation(true);
///
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FxVolSurfaceConfig {
    // === Smile (Strike) Dimension ===
    /// Smile interpolation method (SABR, SVI, etc.).
    pub smile_interpolation: InterpolationMethod,

    /// Strike axis type (delta, moneyness, etc.).
    pub strike_axis: StrikeAxisType,

    /// Extrapolation method for strikes outside the quoted range.
    pub strike_extrapolation: ExtrapolationMethod,

    // === Expiry (Time) Dimension ===
    /// Expiry interpolation method.
    pub expiry_interpolation: ExpiryInterpolation,

    /// Whether to allow extrapolation beyond the expiry range.
    pub allow_extrapolation: bool,

    // === SABR Parameters ===
    /// SABR beta (fixed). None = calibrate beta.
    pub sabr_beta: Option<f64>,

    /// SABR shift for negative rates (shifted SABR).
    pub sabr_shift: f64,

    // === Calibration Settings ===
    /// Maximum iterations for calibration optimiser.
    pub max_iterations: usize,

    /// Convergence tolerance.
    pub tolerance: f64,

    /// Enable arbitrage-free constraint checking.
    pub check_arbitrage_free: bool,

    // === Forward Curve ===
    /// Use forward points for forward curve (vs discount curve ratio).
    pub use_forward_points: bool,
}

impl Default for FxVolSurfaceConfig {
    fn default() -> Self {
        Self {
            smile_interpolation: InterpolationMethod::Sabr,
            strike_axis: StrikeAxisType::Delta,
            strike_extrapolation: ExtrapolationMethod::Flat,
            expiry_interpolation: ExpiryInterpolation::Linear,
            allow_extrapolation: true,
            sabr_beta: Some(0.5),
            sabr_shift: 0.0,
            max_iterations: 100,
            tolerance: 1e-8,
            check_arbitrage_free: false,
            use_forward_points: true,
        }
    }
}

impl FxVolSurfaceConfig {
    /// Creates a new config with default settings.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Sets the smile interpolation method.
    #[must_use]
    pub fn with_smile_interpolation(mut self, method: InterpolationMethod) -> Self {
        self.smile_interpolation = method;
        self
    }

    /// Sets the strike axis type.
    #[must_use]
    pub fn with_strike_axis(mut self, axis: StrikeAxisType) -> Self {
        self.strike_axis = axis;
        self
    }

    /// Sets the strike extrapolation method.
    #[must_use]
    pub fn with_strike_extrapolation(mut self, method: ExtrapolationMethod) -> Self {
        self.strike_extrapolation = method;
        self
    }

    /// Sets the expiry interpolation method.
    #[must_use]
    pub fn with_expiry_interpolation(mut self, method: ExpiryInterpolation) -> Self {
        self.expiry_interpolation = method;
        self
    }

    /// Sets whether extrapolation is allowed.
    #[must_use]
    pub fn with_allow_extrapolation(mut self, allow: bool) -> Self {
        self.allow_extrapolation = allow;
        self
    }

    /// Sets the SABR beta parameter.
    #[must_use]
    pub fn with_sabr_beta(mut self, beta: f64) -> Self {
        self.sabr_beta = Some(beta);
        self
    }

    /// Sets the SABR shift for negative rates.
    #[must_use]
    pub fn with_sabr_shift(mut self, shift: f64) -> Self {
        self.sabr_shift = shift;
        self
    }

    /// Sets the maximum iterations for calibration.
    #[must_use]
    pub fn with_max_iterations(mut self, iterations: usize) -> Self {
        self.max_iterations = iterations;
        self
    }

    /// Sets the convergence tolerance.
    #[must_use]
    pub fn with_tolerance(mut self, tol: f64) -> Self {
        self.tolerance = tol;
        self
    }

    /// Enables or disables arbitrage-free checking.
    #[must_use]
    pub fn with_check_arbitrage_free(mut self, check: bool) -> Self {
        self.check_arbitrage_free = check;
        self
    }

    /// Sets whether to use forward points for forward curve.
    #[must_use]
    pub fn with_use_forward_points(mut self, use_points: bool) -> Self {
        self.use_forward_points = use_points;
        self
    }

    /// Creates a config preset for EURUSD-like pairs.
    ///
    /// - SABR interpolation
    /// - Spot delta convention
    /// - Beta = 0.5
    #[must_use]
    pub fn eurusd_preset() -> Self {
        Self::default()
            .with_smile_interpolation(InterpolationMethod::Sabr)
            .with_strike_axis(StrikeAxisType::Delta)
            .with_sabr_beta(0.5)
    }

    /// Creates a config preset for USDJPY-like pairs.
    ///
    /// - SABR interpolation
    /// - Premium-adjusted delta
    /// - Beta = 0.5
    #[must_use]
    pub fn usdjpy_preset() -> Self {
        Self::default()
            .with_smile_interpolation(InterpolationMethod::Sabr)
            .with_strike_axis(StrikeAxisType::Delta)
            .with_sabr_beta(0.5)
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), String> {
        // Validate SABR beta
        if let Some(beta) = self.sabr_beta {
            if !(0.0..=1.0).contains(&beta) {
                return Err(format!("SABR beta must be in [0, 1], got: {}", beta));
            }
        }

        // Validate tolerance
        if self.tolerance <= 0.0 {
            return Err(format!(
                "Tolerance must be positive, got: {}",
                self.tolerance
            ));
        }

        // Validate max iterations
        if self.max_iterations == 0 {
            return Err("Max iterations must be at least 1".to_string());
        }

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = FxVolSurfaceConfig::default();

        assert_eq!(config.smile_interpolation, InterpolationMethod::Sabr);
        assert_eq!(config.strike_axis, StrikeAxisType::Delta);
        assert_eq!(config.strike_extrapolation, ExtrapolationMethod::Flat);
        assert_eq!(config.expiry_interpolation, ExpiryInterpolation::Linear);
        assert!(config.allow_extrapolation);
        assert_eq!(config.sabr_beta, Some(0.5));
        assert!((config.sabr_shift).abs() < 1e-10);
        assert_eq!(config.max_iterations, 100);
        assert!((config.tolerance - 1e-8).abs() < 1e-15);
        assert!(!config.check_arbitrage_free);
        assert!(config.use_forward_points);
    }

    #[test]
    fn test_config_builder_smile_interpolation() {
        let config =
            FxVolSurfaceConfig::default().with_smile_interpolation(InterpolationMethod::Svi);
        assert_eq!(config.smile_interpolation, InterpolationMethod::Svi);
    }

    #[test]
    fn test_config_builder_strike_axis() {
        let config = FxVolSurfaceConfig::default().with_strike_axis(StrikeAxisType::LogMoneyness);
        assert_eq!(config.strike_axis, StrikeAxisType::LogMoneyness);
    }

    #[test]
    fn test_config_builder_expiry_interpolation() {
        let config = FxVolSurfaceConfig::default()
            .with_expiry_interpolation(ExpiryInterpolation::FlatForward);
        assert_eq!(
            config.expiry_interpolation,
            ExpiryInterpolation::FlatForward
        );
    }

    #[test]
    fn test_config_builder_sabr_beta() {
        let config = FxVolSurfaceConfig::default().with_sabr_beta(0.25);
        assert_eq!(config.sabr_beta, Some(0.25));
    }

    #[test]
    fn test_config_builder_sabr_shift() {
        let config = FxVolSurfaceConfig::default().with_sabr_shift(0.03);
        assert!((config.sabr_shift - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_config_builder_chain() {
        let config = FxVolSurfaceConfig::default()
            .with_smile_interpolation(InterpolationMethod::Svi)
            .with_expiry_interpolation(ExpiryInterpolation::CubicSpline)
            .with_allow_extrapolation(false)
            .with_sabr_beta(1.0)
            .with_max_iterations(200)
            .with_tolerance(1e-10)
            .with_check_arbitrage_free(true);

        assert_eq!(config.smile_interpolation, InterpolationMethod::Svi);
        assert_eq!(
            config.expiry_interpolation,
            ExpiryInterpolation::CubicSpline
        );
        assert!(!config.allow_extrapolation);
        assert_eq!(config.sabr_beta, Some(1.0));
        assert_eq!(config.max_iterations, 200);
        assert!((config.tolerance - 1e-10).abs() < 1e-15);
        assert!(config.check_arbitrage_free);
    }

    #[test]
    fn test_config_preset_eurusd() {
        let config = FxVolSurfaceConfig::eurusd_preset();
        assert_eq!(config.smile_interpolation, InterpolationMethod::Sabr);
        assert_eq!(config.strike_axis, StrikeAxisType::Delta);
        assert_eq!(config.sabr_beta, Some(0.5));
    }

    #[test]
    fn test_config_preset_usdjpy() {
        let config = FxVolSurfaceConfig::usdjpy_preset();
        assert_eq!(config.smile_interpolation, InterpolationMethod::Sabr);
        assert_eq!(config.strike_axis, StrikeAxisType::Delta);
    }

    #[test]
    fn test_config_validate_valid() {
        let config = FxVolSurfaceConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_invalid_beta_high() {
        let config = FxVolSurfaceConfig::default().with_sabr_beta(1.5);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("beta"));
    }

    #[test]
    fn test_config_validate_invalid_beta_negative() {
        let config = FxVolSurfaceConfig::default().with_sabr_beta(-0.1);
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validate_invalid_tolerance() {
        let config = FxVolSurfaceConfig::default().with_tolerance(-1e-8);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Tolerance"));
    }

    #[test]
    fn test_config_validate_invalid_max_iterations() {
        let config = FxVolSurfaceConfig::default().with_max_iterations(0);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("iterations"));
    }
}
