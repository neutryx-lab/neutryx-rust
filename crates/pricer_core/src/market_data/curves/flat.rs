//! Flat yield curve implementation.

use super::YieldCurve;
use crate::market_data::error::MarketDataError;
use num_traits::Float;

/// Flat yield curve with constant interest rate.
///
/// A simple yield curve where the same rate applies to all maturities.
/// Useful for prototyping, testing, and scenarios with flat term structures.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Example
///
/// ```
/// use pricer_core::market_data::curves::{YieldCurve, FlatCurve};
///
/// let curve = FlatCurve::new(0.05_f64);
///
/// // Discount factor at t=1: exp(-0.05 * 1) ≈ 0.9512
/// let df = curve.discount_factor(1.0).unwrap();
/// assert!((df - 0.951229).abs() < 1e-5);
///
/// // Zero rate is constant
/// assert_eq!(curve.zero_rate(1.0).unwrap(), 0.05);
/// assert_eq!(curve.zero_rate(5.0).unwrap(), 0.05);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatCurve<T: Float> {
    /// The constant interest rate
    rate: T,
}

impl<T: Float> FlatCurve<T> {
    /// Construct a flat curve with the given constant rate.
    ///
    /// # Arguments
    ///
    /// * `rate` - The constant interest rate (continuously compounded)
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::FlatCurve;
    ///
    /// let curve = FlatCurve::new(0.05_f64);
    /// assert_eq!(curve.rate(), 0.05);
    /// ```
    #[inline]
    pub fn new(rate: T) -> Self {
        Self { rate }
    }

    /// Return the constant rate.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::FlatCurve;
    ///
    /// let curve = FlatCurve::new(0.03_f64);
    /// assert_eq!(curve.rate(), 0.03);
    /// ```
    #[inline]
    pub fn rate(&self) -> T {
        self.rate
    }
}

impl<T: Float> YieldCurve<T> for FlatCurve<T> {
    /// Return the discount factor for maturity `t`.
    ///
    /// For a flat curve with rate r:
    /// ```text
    /// D(t) = exp(-r * t)
    /// ```
    ///
    /// # Arguments
    ///
    /// * `t` - Time to maturity in years (must be >= 0)
    ///
    /// # Returns
    ///
    /// * `Ok(D(t))` - Discount factor at time t
    /// * `Err(MarketDataError::InvalidMaturity)` - If t < 0
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }
        Ok((-self.rate * t).exp())
    }

    /// Return the zero rate for maturity `t`.
    ///
    /// For a flat curve, the zero rate is simply the constant rate.
    ///
    /// # Arguments
    ///
    /// * `t` - Time to maturity in years (must be > 0)
    ///
    /// # Returns
    ///
    /// * `Ok(r)` - The constant rate
    /// * `Err(MarketDataError::InvalidMaturity)` - If t <= 0
    fn zero_rate(&self, t: T) -> Result<T, MarketDataError> {
        if t <= T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }
        Ok(self.rate)
    }

    /// Return the forward rate between t1 and t2.
    ///
    /// For a flat curve, the forward rate equals the constant rate.
    ///
    /// # Arguments
    ///
    /// * `t1` - Start time in years (must be >= 0)
    /// * `t2` - End time in years (must be > t1)
    ///
    /// # Returns
    ///
    /// * `Ok(r)` - The constant rate
    /// * `Err(MarketDataError::InvalidMaturity)` - If t2 <= t1
    fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError> {
        if t2 <= t1 {
            return Err(MarketDataError::InvalidMaturity {
                t: (t2 - t1).to_f64().unwrap_or(0.0),
            });
        }
        Ok(self.rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Construction Tests
    // ========================================

    #[test]
    fn test_new() {
        let curve = FlatCurve::new(0.05_f64);
        assert_eq!(curve.rate(), 0.05);
    }

    #[test]
    fn test_new_negative_rate() {
        // Negative rates are valid (e.g., negative interest rate environment)
        let curve = FlatCurve::new(-0.01_f64);
        assert_eq!(curve.rate(), -0.01);
    }

    #[test]
    fn test_new_zero_rate() {
        let curve = FlatCurve::new(0.0_f64);
        assert_eq!(curve.rate(), 0.0);
    }

    #[test]
    fn test_clone() {
        let curve = FlatCurve::new(0.05_f64);
        let cloned = curve.clone();
        assert_eq!(curve.rate(), cloned.rate());
    }

    #[test]
    fn test_copy() {
        let curve = FlatCurve::new(0.05_f64);
        let copied = curve;
        assert_eq!(curve.rate(), copied.rate());
    }

    #[test]
    fn test_debug() {
        let curve = FlatCurve::new(0.05_f64);
        let debug_str = format!("{:?}", curve);
        assert!(debug_str.contains("FlatCurve"));
    }

    // ========================================
    // Discount Factor Tests
    // ========================================

    #[test]
    fn test_discount_factor_at_zero() {
        let curve = FlatCurve::new(0.05_f64);
        let df = curve.discount_factor(0.0).unwrap();
        assert!((df - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_discount_factor_at_one_year() {
        let curve = FlatCurve::new(0.05_f64);
        let df = curve.discount_factor(1.0).unwrap();
        let expected = (-0.05_f64).exp();
        assert!((df - expected).abs() < 1e-10);
    }

    #[test]
    fn test_discount_factor_at_multiple_years() {
        let curve = FlatCurve::new(0.05_f64);

        for t in [0.5, 1.0, 2.0, 5.0, 10.0] {
            let df = curve.discount_factor(t).unwrap();
            let expected = (-0.05 * t).exp();
            assert!(
                (df - expected).abs() < 1e-10,
                "Failed at t={}: got {}, expected {}",
                t,
                df,
                expected
            );
        }
    }

    #[test]
    fn test_discount_factor_negative_maturity() {
        let curve = FlatCurve::new(0.05_f64);
        let result = curve.discount_factor(-1.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InvalidMaturity { t } => assert_eq!(t, -1.0),
            _ => panic!("Expected InvalidMaturity error"),
        }
    }

    #[test]
    fn test_discount_factor_with_zero_rate() {
        let curve = FlatCurve::new(0.0_f64);
        let df = curve.discount_factor(5.0).unwrap();
        assert!((df - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_discount_factor_with_negative_rate() {
        let curve = FlatCurve::new(-0.01_f64);
        let df = curve.discount_factor(1.0).unwrap();
        let expected = 0.01_f64.exp(); // exp(0.01)
        assert!((df - expected).abs() < 1e-10);
    }

    // ========================================
    // Zero Rate Tests
    // ========================================

    #[test]
    fn test_zero_rate() {
        let curve = FlatCurve::new(0.05_f64);
        let r = curve.zero_rate(1.0).unwrap();
        assert!((r - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_zero_rate_various_maturities() {
        let curve = FlatCurve::new(0.03_f64);

        for t in [0.25, 0.5, 1.0, 2.0, 10.0] {
            let r = curve.zero_rate(t).unwrap();
            assert!(
                (r - 0.03).abs() < 1e-10,
                "Failed at t={}: got {}, expected 0.03",
                t,
                r
            );
        }
    }

    #[test]
    fn test_zero_rate_invalid_maturity_zero() {
        let curve = FlatCurve::new(0.05_f64);
        let result = curve.zero_rate(0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_rate_invalid_maturity_negative() {
        let curve = FlatCurve::new(0.05_f64);
        let result = curve.zero_rate(-1.0);
        assert!(result.is_err());
    }

    // ========================================
    // Forward Rate Tests
    // ========================================

    #[test]
    fn test_forward_rate() {
        let curve = FlatCurve::new(0.05_f64);
        let f = curve.forward_rate(1.0, 2.0).unwrap();
        assert!((f - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_forward_rate_various_periods() {
        let curve = FlatCurve::new(0.04_f64);

        let periods = [(0.0, 1.0), (1.0, 2.0), (0.5, 1.5), (2.0, 5.0)];
        for (t1, t2) in periods {
            let f = curve.forward_rate(t1, t2).unwrap();
            assert!(
                (f - 0.04).abs() < 1e-10,
                "Failed for ({}, {}): got {}, expected 0.04",
                t1,
                t2,
                f
            );
        }
    }

    #[test]
    fn test_forward_rate_invalid_t2_less_than_t1() {
        let curve = FlatCurve::new(0.05_f64);
        let result = curve.forward_rate(2.0, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_forward_rate_invalid_t2_equals_t1() {
        let curve = FlatCurve::new(0.05_f64);
        let result = curve.forward_rate(1.0, 1.0);
        assert!(result.is_err());
    }

    // ========================================
    // Generic Type Tests
    // ========================================

    #[test]
    fn test_with_f32() {
        let curve = FlatCurve::new(0.05_f32);
        let df = curve.discount_factor(1.0_f32).unwrap();
        let expected = (-0.05_f32).exp();
        assert!((df - expected).abs() < 1e-6);
    }
}
