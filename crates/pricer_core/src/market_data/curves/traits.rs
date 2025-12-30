//! Yield curve trait definition.

use crate::market_data::error::MarketDataError;
use num_traits::Float;

/// Generic yield curve trait for discount factor and rate calculations.
///
/// All implementations must be generic over `T: Float` for AD compatibility.
/// This ensures the curve can be used with both standard floating-point types
/// (f64, f32) and automatic differentiation types (Dual64).
///
/// # Contract
///
/// - `discount_factor(t)` returns the discount factor D(t) for maturity t
/// - `zero_rate(t)` returns the continuously compounded zero rate r(t)
/// - `forward_rate(t1, t2)` returns the forward rate between t1 and t2
///
/// # Invariants
///
/// - D(0) = 1 (discount factor at time 0 is 1)
/// - D(t) > 0 for all t >= 0 (discount factors are positive)
/// - D(t1) >= D(t2) for t1 <= t2 (no arbitrage condition)
///
/// # Example
///
/// ```
/// use pricer_core::market_data::curves::{YieldCurve, FlatCurve};
///
/// let curve = FlatCurve::new(0.05_f64);
///
/// // Get discount factor for 1 year
/// let df = curve.discount_factor(1.0).unwrap();
/// assert!((df - 0.951229).abs() < 1e-5);
///
/// // Get zero rate
/// let rate = curve.zero_rate(1.0).unwrap();
/// assert!((rate - 0.05).abs() < 1e-10);
///
/// // Get forward rate
/// let fwd = curve.forward_rate(1.0, 2.0).unwrap();
/// assert!((fwd - 0.05).abs() < 1e-10);
/// ```
pub trait YieldCurve<T: Float> {
    /// Return the discount factor for maturity `t`.
    ///
    /// The discount factor D(t) represents the present value of 1 unit
    /// of currency received at time t.
    ///
    /// # Arguments
    ///
    /// * `t` - Time to maturity in years (must be >= 0)
    ///
    /// # Returns
    ///
    /// * `Ok(D(t))` - Discount factor at time t
    /// * `Err(MarketDataError::InvalidMaturity)` - If t < 0
    ///
    /// # Mathematical Definition
    ///
    /// For a continuously compounded rate r(t):
    /// ```text
    /// D(t) = exp(-r(t) * t)
    /// ```
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError>;

    /// Return the continuously compounded zero rate for maturity `t`.
    ///
    /// The zero rate r(t) is the constant rate that, when compounded
    /// continuously, gives the discount factor D(t).
    ///
    /// # Arguments
    ///
    /// * `t` - Time to maturity in years (must be > 0)
    ///
    /// # Returns
    ///
    /// * `Ok(r(t))` - Zero rate at time t
    /// * `Err(MarketDataError::InvalidMaturity)` - If t <= 0
    ///
    /// # Default Implementation
    ///
    /// ```text
    /// r(t) = -ln(D(t)) / t
    /// ```
    fn zero_rate(&self, t: T) -> Result<T, MarketDataError> {
        let df = self.discount_factor(t)?;
        if t <= T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }
        Ok(-df.ln() / t)
    }

    /// Return the forward rate between t1 and t2.
    ///
    /// The forward rate f(t1, t2) is the rate applicable for the period
    /// from t1 to t2, as implied by the current yield curve.
    ///
    /// # Arguments
    ///
    /// * `t1` - Start time in years (must be >= 0)
    /// * `t2` - End time in years (must be > t1)
    ///
    /// # Returns
    ///
    /// * `Ok(f(t1, t2))` - Forward rate between t1 and t2
    /// * `Err(MarketDataError::InvalidMaturity)` - If t2 <= t1
    ///
    /// # Default Implementation
    ///
    /// ```text
    /// f(t1, t2) = -ln(D(t2) / D(t1)) / (t2 - t1)
    /// ```
    fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError> {
        let df1 = self.discount_factor(t1)?;
        let df2 = self.discount_factor(t2)?;
        let dt = t2 - t1;
        if dt <= T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: dt.to_f64().unwrap_or(0.0),
            });
        }
        Ok(-(df2 / df1).ln() / dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock implementation for testing default methods
    struct MockCurve {
        rate: f64,
    }

    impl YieldCurve<f64> for MockCurve {
        fn discount_factor(&self, t: f64) -> Result<f64, MarketDataError> {
            if t < 0.0 {
                return Err(MarketDataError::InvalidMaturity { t });
            }
            Ok((-self.rate * t).exp())
        }
    }

    #[test]
    fn test_default_zero_rate() {
        let curve = MockCurve { rate: 0.05 };
        let r = curve.zero_rate(1.0).unwrap();
        assert!((r - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_default_zero_rate_invalid_maturity() {
        let curve = MockCurve { rate: 0.05 };
        let result = curve.zero_rate(0.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InvalidMaturity { t } => assert_eq!(t, 0.0),
            _ => panic!("Expected InvalidMaturity error"),
        }
    }

    #[test]
    fn test_default_forward_rate() {
        let curve = MockCurve { rate: 0.05 };
        let f = curve.forward_rate(1.0, 2.0).unwrap();
        assert!((f - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_default_forward_rate_invalid() {
        let curve = MockCurve { rate: 0.05 };
        let result = curve.forward_rate(2.0, 1.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InvalidMaturity { .. } => {}
            _ => panic!("Expected InvalidMaturity error"),
        }
    }
}
