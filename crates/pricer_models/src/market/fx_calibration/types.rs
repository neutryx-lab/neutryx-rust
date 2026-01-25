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

// ============================================================================
// ForwardPoints Newtype
// ============================================================================

/// Forward points for FX forward rate calculation.
///
/// Forward points represent the difference between forward and spot rates,
/// quoted with a scaling factor. Provides convenience methods for forward
/// rate calculation.
///
/// # Example
///
/// ```rust
/// use pricer_models::market::fx_calibration::ForwardPoints;
///
/// // EURUSD: 50 points with scaling factor 10000
/// let points = ForwardPoints::new(50.0, 10000.0);
/// let forward = points.to_forward_rate(1.1000);
/// assert!((forward - 1.1050).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ForwardPoints {
    /// Raw points value.
    points: f64,
    /// Scaling factor (e.g., 10000 for EURUSD, 100 for USDJPY).
    scaling_factor: f64,
}

impl ForwardPoints {
    /// Creates new forward points with explicit scaling factor.
    #[must_use]
    pub fn new(points: f64, scaling_factor: f64) -> Self {
        Self {
            points,
            scaling_factor,
        }
    }

    /// Creates forward points for EURUSD-like pairs (scaling = 10000).
    #[must_use]
    pub fn for_major_pairs(points: f64) -> Self { Self::new(points, 10000.0) }

    /// Creates forward points for USDJPY-like pairs (scaling = 100).
    #[must_use]
    pub fn for_jpy_pairs(points: f64) -> Self { Self::new(points, 100.0) }

    /// Returns the raw points value.
    #[inline]
    #[must_use]
    pub fn points(&self) -> f64 { self.points }

    /// Returns the scaling factor.
    #[inline]
    #[must_use]
    pub fn scaling_factor(&self) -> f64 { self.scaling_factor }

    /// Calculates forward rate from spot rate.
    ///
    /// Formula: F = S + points / scaling_factor
    #[inline]
    #[must_use]
    pub fn to_forward_rate(&self, spot: f64) -> f64 { spot + self.points / self.scaling_factor }

    /// Calculates points from spot and forward rates.
    #[must_use]
    pub fn from_rates(spot: f64, forward: f64, scaling_factor: f64) -> Self {
        let points = (forward - spot) * scaling_factor;
        Self::new(points, scaling_factor)
    }
}

impl std::fmt::Display for ForwardPoints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1} pts", self.points)
    }
}

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

    // === ForwardPoints Tests ===

    #[test]
    fn test_forward_points_new() {
        let fp = ForwardPoints::new(50.0, 10000.0);
        assert!((fp.points() - 50.0).abs() < 1e-10);
        assert!((fp.scaling_factor() - 10000.0).abs() < 1e-10);
    }

    #[test]
    fn test_forward_points_to_forward_rate_eurusd() {
        let fp = ForwardPoints::for_major_pairs(50.0);
        let forward = fp.to_forward_rate(1.1000);
        // F = 1.1000 + 50/10000 = 1.1050
        assert!((forward - 1.1050).abs() < 1e-10);
    }

    #[test]
    fn test_forward_points_to_forward_rate_usdjpy() {
        let fp = ForwardPoints::for_jpy_pairs(-25.0);
        let forward = fp.to_forward_rate(150.0);
        // F = 150.0 + (-25)/100 = 149.75
        assert!((forward - 149.75).abs() < 1e-10);
    }

    #[test]
    fn test_forward_points_from_rates() {
        let fp = ForwardPoints::from_rates(1.1000, 1.1050, 10000.0);
        assert!((fp.points() - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_forward_points_display() {
        let fp = ForwardPoints::new(50.5, 10000.0);
        assert_eq!(fp.to_string(), "50.5 pts");
    }

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
