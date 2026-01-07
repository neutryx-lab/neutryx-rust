//! Curve enumeration types for multi-curve framework.
//!
//! This module provides:
//! - [`CurveName`]: Enumeration of standard curve names (OIS, SOFR, TONAR, etc.)
//! - [`CurveEnum`]: Static dispatch enum wrapping FlatCurve and InterpolatedCurve

use super::{FlatCurve, InterpolatedCurve, YieldCurve};
use crate::market_data::error::MarketDataError;
use num_traits::Float;

/// Standard curve names for the multi-curve framework.
///
/// This enum represents the various types of yield curves used in
/// derivatives pricing and risk management.
///
/// # Variants
///
/// - `Ois`: Overnight Index Swap curve (e.g., EONIA, Fed Funds)
/// - `Sofr`: Secured Overnight Financing Rate curve
/// - `Tonar`: Tokyo Overnight Average Rate curve
/// - `Euribor`: Euro Interbank Offered Rate curve
/// - `Forward`: Generic forward curve
/// - `Discount`: Discount curve for present value calculations
/// - `Custom`: User-defined curve with custom name
///
/// # Example
///
/// ```
/// use pricer_core::market_data::curves::CurveName;
///
/// let sofr = CurveName::Sofr;
/// assert_eq!(sofr.as_str(), "SOFR");
///
/// let custom = CurveName::Custom("MY_CURVE");
/// assert_eq!(custom.as_str(), "MY_CURVE");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurveName {
    /// Overnight Index Swap curve
    Ois,
    /// Secured Overnight Financing Rate curve
    Sofr,
    /// Tokyo Overnight Average Rate curve
    Tonar,
    /// Euro Interbank Offered Rate curve
    Euribor,
    /// Generic forward curve
    Forward,
    /// Discount curve
    Discount,
    /// Custom curve with user-defined name
    Custom(&'static str),
}

impl CurveName {
    /// Return the string representation of the curve name.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::CurveName;
    ///
    /// assert_eq!(CurveName::Ois.as_str(), "OIS");
    /// assert_eq!(CurveName::Sofr.as_str(), "SOFR");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &'static str {
        match self {
            CurveName::Ois => "OIS",
            CurveName::Sofr => "SOFR",
            CurveName::Tonar => "TONAR",
            CurveName::Euribor => "EURIBOR",
            CurveName::Forward => "FORWARD",
            CurveName::Discount => "DISCOUNT",
            CurveName::Custom(name) => name,
        }
    }
}

impl std::fmt::Display for CurveName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Static dispatch enum wrapping concrete yield curve implementations.
///
/// This enum provides efficient static dispatch for yield curve operations,
/// avoiding the overhead of trait objects while maintaining AD compatibility
/// through generic type parameter `T: Float`.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Variants
///
/// - `Flat`: Constant rate yield curve
/// - `Interpolated`: Pillar-based interpolated yield curve
///
/// # Example
///
/// ```
/// use pricer_core::market_data::curves::{CurveEnum, FlatCurve, YieldCurve};
///
/// // Create a flat curve wrapped in CurveEnum
/// let flat = FlatCurve::new(0.05_f64);
/// let curve = CurveEnum::Flat(flat);
///
/// // Use YieldCurve trait methods
/// let df = curve.discount_factor(1.0).unwrap();
/// assert!((df - 0.951229).abs() < 1e-5);
/// ```
#[derive(Debug, Clone)]
pub enum CurveEnum<T: Float> {
    /// Flat (constant rate) yield curve
    Flat(FlatCurve<T>),
    /// Interpolated yield curve with pillar points
    Interpolated(InterpolatedCurve<T>),
}

impl<T: Float> CurveEnum<T> {
    /// Create a flat curve variant.
    ///
    /// # Arguments
    ///
    /// * `rate` - The constant interest rate
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::{CurveEnum, YieldCurve};
    ///
    /// let curve = CurveEnum::flat(0.05_f64);
    /// let df = curve.discount_factor(1.0).unwrap();
    /// ```
    #[inline]
    pub fn flat(rate: T) -> Self {
        CurveEnum::Flat(FlatCurve::new(rate))
    }
}

impl<T: Float> YieldCurve<T> for CurveEnum<T> {
    /// Return the discount factor for maturity `t`.
    ///
    /// Delegates to the underlying curve implementation.
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError> {
        match self {
            CurveEnum::Flat(curve) => curve.discount_factor(t),
            CurveEnum::Interpolated(curve) => curve.discount_factor(t),
        }
    }

    /// Return the zero rate for maturity `t`.
    ///
    /// Delegates to the underlying curve implementation.
    fn zero_rate(&self, t: T) -> Result<T, MarketDataError> {
        match self {
            CurveEnum::Flat(curve) => curve.zero_rate(t),
            CurveEnum::Interpolated(curve) => curve.zero_rate(t),
        }
    }

    /// Return the forward rate between t1 and t2.
    ///
    /// Delegates to the underlying curve implementation.
    fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError> {
        match self {
            CurveEnum::Flat(curve) => curve.forward_rate(t1, t2),
            CurveEnum::Interpolated(curve) => curve.forward_rate(t1, t2),
        }
    }
}

impl<T: Float> From<FlatCurve<T>> for CurveEnum<T> {
    fn from(curve: FlatCurve<T>) -> Self {
        CurveEnum::Flat(curve)
    }
}

impl<T: Float> From<InterpolatedCurve<T>> for CurveEnum<T> {
    fn from(curve: InterpolatedCurve<T>) -> Self {
        CurveEnum::Interpolated(curve)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::curves::CurveInterpolation;

    // ========================================
    // CurveName Tests
    // ========================================

    #[test]
    fn test_curve_name_as_str() {
        assert_eq!(CurveName::Ois.as_str(), "OIS");
        assert_eq!(CurveName::Sofr.as_str(), "SOFR");
        assert_eq!(CurveName::Tonar.as_str(), "TONAR");
        assert_eq!(CurveName::Euribor.as_str(), "EURIBOR");
        assert_eq!(CurveName::Forward.as_str(), "FORWARD");
        assert_eq!(CurveName::Discount.as_str(), "DISCOUNT");
        assert_eq!(CurveName::Custom("MY_CURVE").as_str(), "MY_CURVE");
    }

    #[test]
    fn test_curve_name_display() {
        assert_eq!(format!("{}", CurveName::Sofr), "SOFR");
        assert_eq!(format!("{}", CurveName::Custom("CUSTOM")), "CUSTOM");
    }

    #[test]
    fn test_curve_name_equality() {
        assert_eq!(CurveName::Sofr, CurveName::Sofr);
        assert_ne!(CurveName::Sofr, CurveName::Tonar);
        assert_eq!(CurveName::Custom("A"), CurveName::Custom("A"));
        assert_ne!(CurveName::Custom("A"), CurveName::Custom("B"));
    }

    #[test]
    fn test_curve_name_clone() {
        let name = CurveName::Sofr;
        let cloned = name;
        assert_eq!(name, cloned);
    }

    #[test]
    fn test_curve_name_debug() {
        let debug_str = format!("{:?}", CurveName::Sofr);
        assert!(debug_str.contains("Sofr"));
    }

    #[test]
    fn test_curve_name_hash() {
        use std::collections::HashMap;
        let mut map: HashMap<CurveName, i32> = HashMap::new();
        map.insert(CurveName::Sofr, 1);
        map.insert(CurveName::Tonar, 2);
        assert_eq!(map.get(&CurveName::Sofr), Some(&1));
        assert_eq!(map.get(&CurveName::Tonar), Some(&2));
    }

    // ========================================
    // CurveEnum Tests - Flat
    // ========================================

    #[test]
    fn test_curve_enum_flat_creation() {
        let curve = CurveEnum::flat(0.05_f64);
        match curve {
            CurveEnum::Flat(_) => {}
            _ => panic!("Expected Flat variant"),
        }
    }

    #[test]
    fn test_curve_enum_flat_discount_factor() {
        let curve = CurveEnum::flat(0.05_f64);
        let df = curve.discount_factor(1.0).unwrap();
        let expected = (-0.05_f64).exp();
        assert!((df - expected).abs() < 1e-10);
    }

    #[test]
    fn test_curve_enum_flat_zero_rate() {
        let curve = CurveEnum::flat(0.05_f64);
        let rate = curve.zero_rate(1.0).unwrap();
        assert!((rate - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_curve_enum_flat_forward_rate() {
        let curve = CurveEnum::flat(0.05_f64);
        let fwd = curve.forward_rate(1.0, 2.0).unwrap();
        assert!((fwd - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_curve_enum_from_flat_curve() {
        let flat = FlatCurve::new(0.03_f64);
        let curve: CurveEnum<f64> = flat.into();
        let df = curve.discount_factor(1.0).unwrap();
        let expected = (-0.03_f64).exp();
        assert!((df - expected).abs() < 1e-10);
    }

    // ========================================
    // CurveEnum Tests - Interpolated
    // ========================================

    #[test]
    fn test_curve_enum_interpolated_discount_factor() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.03, 0.04];
        let interp =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();
        let curve = CurveEnum::Interpolated(interp);

        let df = curve.discount_factor(1.0).unwrap();
        let expected = (-0.03_f64).exp();
        assert!((df - expected).abs() < 1e-10);
    }

    #[test]
    fn test_curve_enum_interpolated_zero_rate() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.03, 0.04];
        let interp =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();
        let curve = CurveEnum::Interpolated(interp);

        let rate = curve.zero_rate(1.0).unwrap();
        assert!((rate - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_curve_enum_from_interpolated_curve() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.03, 0.04];
        let interp =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();
        let curve: CurveEnum<f64> = interp.into();

        match curve {
            CurveEnum::Interpolated(_) => {}
            _ => panic!("Expected Interpolated variant"),
        }
    }

    // ========================================
    // Error Handling Tests
    // ========================================

    #[test]
    fn test_curve_enum_flat_invalid_maturity() {
        let curve = CurveEnum::flat(0.05_f64);
        let result = curve.discount_factor(-1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_curve_enum_interpolated_out_of_bounds() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.03, 0.04];
        let interp =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();
        let curve = CurveEnum::Interpolated(interp);

        let result = curve.discount_factor(0.25);
        assert!(result.is_err());
    }

    // ========================================
    // Generic Type Tests
    // ========================================

    #[test]
    fn test_curve_enum_with_f32() {
        let curve = CurveEnum::flat(0.05_f32);
        let df = curve.discount_factor(1.0_f32).unwrap();
        let expected = (-0.05_f32).exp();
        assert!((df - expected).abs() < 1e-6);
    }

    #[test]
    fn test_curve_enum_clone() {
        let curve = CurveEnum::flat(0.05_f64);
        let cloned = curve.clone();
        let df1 = curve.discount_factor(1.0).unwrap();
        let df2 = cloned.discount_factor(1.0).unwrap();
        assert!((df1 - df2).abs() < 1e-10);
    }

    #[test]
    fn test_curve_enum_debug() {
        let curve = CurveEnum::flat(0.05_f64);
        let debug_str = format!("{:?}", curve);
        assert!(debug_str.contains("Flat"));
    }
}
