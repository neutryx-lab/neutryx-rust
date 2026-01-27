//! FX Calibration core types.
//!
//! This module provides newtypes for type-safe FX volatility surface
//! operations.

// ============================================================================
// Strike Newtype
// ============================================================================

/// Strike price newtype for FX options.
///
/// Stores the strike as an absolute FX rate (quote currency per base currency).
/// Provides type safety and prevents confusion with other f64 values.
///
/// # Example
///
/// ```rust
/// use pricer_models::market::fx_calibration::Strike;
///
/// let strike = Strike::new(1.1050);
/// assert!((strike.value() - 1.1050).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Strike(f64);

impl Strike {
    /// Creates a new Strike value.
    ///
    /// # Arguments
    ///
    /// * `value` - The strike price (should be positive)
    #[must_use]
    pub fn new(value: f64) -> Self { Self(value) }

    /// Returns the strike value.
    #[inline]
    #[must_use]
    pub fn value(&self) -> f64 { self.0 }

    /// Converts strike to log-moneyness given forward rate.
    ///
    /// Log-moneyness = ln(K/F)
    #[inline]
    #[must_use]
    pub fn log_moneyness(&self, forward: f64) -> f64 { (self.0 / forward).ln() }

    /// Converts strike to moneyness given forward rate.
    ///
    /// Moneyness = K/F
    #[inline]
    #[must_use]
    pub fn moneyness(&self, forward: f64) -> f64 { self.0 / forward }
}

impl From<f64> for Strike {
    fn from(value: f64) -> Self { Self(value) }
}

impl std::fmt::Display for Strike {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.6}", self.0)
    }
}

// ============================================================================
// Vol Newtype
// ============================================================================

/// Implied volatility newtype.
///
/// Stores volatility as an annualised decimal (e.g., 0.10 for 10%).
/// Ensures type safety and clear intent in function signatures.
///
/// # Example
///
/// ```rust
/// use pricer_models::market::fx_calibration::Vol;
///
/// let vol = Vol::from_decimal(0.10);
/// assert!((vol.as_percent() - 10.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vol(f64);

impl Vol {
    /// Creates volatility from decimal value.
    ///
    /// # Arguments
    ///
    /// * `decimal` - Volatility as decimal (e.g., 0.10 for 10%)
    #[must_use]
    pub fn from_decimal(decimal: f64) -> Self { Self(decimal) }

    /// Creates volatility from percentage value.
    ///
    /// # Arguments
    ///
    /// * `percent` - Volatility as percentage (e.g., 10.0 for 10%)
    #[must_use]
    pub fn from_percent(percent: f64) -> Self { Self(percent / 100.0) }

    /// Creates volatility from basis points.
    ///
    /// # Arguments
    ///
    /// * `bps` - Volatility in basis points (e.g., 1000 for 10%)
    #[must_use]
    pub fn from_bps(bps: f64) -> Self { Self(bps / 10000.0) }

    /// Returns the volatility as a decimal.
    #[inline]
    #[must_use]
    pub fn as_decimal(&self) -> f64 { self.0 }

    /// Returns the volatility as a percentage.
    #[inline]
    #[must_use]
    pub fn as_percent(&self) -> f64 { self.0 * 100.0 }

    /// Returns the volatility in basis points.
    #[inline]
    #[must_use]
    pub fn as_bps(&self) -> f64 { self.0 * 10000.0 }

    /// Validates that the volatility is positive.
    #[must_use]
    pub fn is_valid(&self) -> bool { self.0 > 0.0 }
}

impl From<f64> for Vol {
    fn from(value: f64) -> Self { Self(value) }
}

impl std::fmt::Display for Vol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}%", self.0 * 100.0)
    }
}

// Note: ForwardPoints has been moved to curves/fx.rs

// ============================================================================
// ExpiryInterpolation Enum
// ============================================================================

/// Expiry (time) dimension interpolation method.
///
/// Controls how volatility is interpolated between expiry pillar dates.
/// Different methods have different smoothness and arbitrage properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExpiryInterpolation {
    /// Linear interpolation in time.
    ///
    /// Simple and fast, but may produce non-smooth term structure.
    #[default]
    Linear,

    /// Flat forward volatility interpolation.
    ///
    /// Assumes constant forward volatility between pillars.
    /// Produces more stable hedging behaviour.
    FlatForward,

    /// Cubic spline interpolation in time.
    ///
    /// Smooth C2 continuous term structure.
    /// May require monotonicity constraints.
    CubicSpline,

    /// Linear variance interpolation.
    ///
    /// Interpolates total variance (sigma^2 * T) linearly.
    /// Ensures no calendar arbitrage for vanilla options.
    LinearVariance,
}

impl ExpiryInterpolation {
    /// Returns a description of the interpolation method.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Linear => "Linear interpolation in time",
            Self::FlatForward => "Flat forward volatility",
            Self::CubicSpline => "Cubic spline (C2 continuous)",
            Self::LinearVariance => "Linear total variance",
        }
    }
}

impl std::fmt::Display for ExpiryInterpolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Linear => "Linear",
            Self::FlatForward => "FlatForward",
            Self::CubicSpline => "CubicSpline",
            Self::LinearVariance => "LinearVariance",
        };
        write!(f, "{}", name)
    }
}

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
