//! FX Volatility Surface Configuration.
//!
//! This module provides configuration types for FX volatility surface
//! calibration.
//!
//! Note: FxVolSurfaceConfig has been consolidated into `surfaces/fx.rs`.
//! This module re-exports it for backward compatibility.

// Re-export from the canonical location (surfaces/fx.rs)
pub use crate::market::surfaces::FxVolSurfaceConfig;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::surfaces::ExpiryInterpolation;
    use crate::market::volcube::{ExtrapolationMethod, InterpolationMethod, StrikeAxisType};

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
