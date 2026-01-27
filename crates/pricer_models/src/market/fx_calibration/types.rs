//! FX Calibration core types.
//!
//! This module provides newtypes for type-safe FX volatility surface
//! operations.
//!
//! Note: These types have been consolidated into `surfaces/fx.rs`.
//! This module re-exports them for backward compatibility.

// Re-export from the canonical location (surfaces/fx.rs)
pub use crate::market::surfaces::{ExpiryInterpolation, Strike, Vol};

// Note: ForwardPoints has been moved to curves/fx.rs

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === Strike Tests ===

    #[test]
    fn test_strike_new() {
        let strike = Strike::new(1.1050);
        assert!((strike.value() - 1.1050).abs() < 1e-10);
    }

    #[test]
    fn test_strike_log_moneyness() {
        let strike = Strike::new(110.0);
        let forward = 100.0;
        let lm = strike.log_moneyness(forward);
        // ln(110/100) = ln(1.1) ≈ 0.0953
        assert!((lm - (1.1_f64).ln()).abs() < 1e-10);
    }

    #[test]
    fn test_strike_moneyness() {
        let strike = Strike::new(110.0);
        let forward = 100.0;
        let m = strike.moneyness(forward);
        assert!((m - 1.1).abs() < 1e-10);
    }

    #[test]
    fn test_strike_from_f64() {
        let strike: Strike = 1.1050.into();
        assert!((strike.value() - 1.1050).abs() < 1e-10);
    }

    #[test]
    fn test_strike_display() {
        let strike = Strike::new(1.105);
        assert_eq!(strike.to_string(), "1.105000");
    }

    // === Vol Tests ===

    #[test]
    fn test_vol_from_decimal() {
        let vol = Vol::from_decimal(0.10);
        assert!((vol.as_decimal() - 0.10).abs() < 1e-10);
    }

    #[test]
    fn test_vol_from_percent() {
        let vol = Vol::from_percent(10.0);
        assert!((vol.as_decimal() - 0.10).abs() < 1e-10);
        assert!((vol.as_percent() - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_vol_from_bps() {
        let vol = Vol::from_bps(1000.0);
        assert!((vol.as_decimal() - 0.10).abs() < 1e-10);
        assert!((vol.as_bps() - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_vol_is_valid() {
        let valid = Vol::from_decimal(0.10);
        let invalid = Vol::from_decimal(-0.10);
        let zero = Vol::from_decimal(0.0);

        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
        assert!(!zero.is_valid());
    }

    #[test]
    fn test_vol_display() {
        let vol = Vol::from_decimal(0.1234);
        assert_eq!(vol.to_string(), "12.34%");
    }

    // Note: ForwardPoints tests have been moved to curves/fx.rs

    // === ExpiryInterpolation Tests ===

    #[test]
    fn test_expiry_interpolation_default() {
        let interp = ExpiryInterpolation::default();
        assert_eq!(interp, ExpiryInterpolation::Linear);
    }

    #[test]
    fn test_expiry_interpolation_description() {
        assert!(ExpiryInterpolation::Linear.description().contains("Linear"));
        assert!(ExpiryInterpolation::FlatForward
            .description()
            .contains("forward"));
        assert!(ExpiryInterpolation::CubicSpline
            .description()
            .contains("spline"));
        assert!(ExpiryInterpolation::LinearVariance
            .description()
            .contains("variance"));
    }

    #[test]
    fn test_expiry_interpolation_display() {
        assert_eq!(ExpiryInterpolation::Linear.to_string(), "Linear");
        assert_eq!(ExpiryInterpolation::FlatForward.to_string(), "FlatForward");
        assert_eq!(ExpiryInterpolation::CubicSpline.to_string(), "CubicSpline");
        assert_eq!(
            ExpiryInterpolation::LinearVariance.to_string(),
            "LinearVariance"
        );
    }
}
