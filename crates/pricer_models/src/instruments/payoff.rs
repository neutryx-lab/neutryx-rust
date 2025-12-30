//! Payoff type definitions with smooth approximations.
//!
//! This module provides payoff types (Call, Put, Digital) with
//! AD-compatible smooth approximations for Enzyme compatibility.

use num_traits::Float;
use pricer_core::math::smoothing::{smooth_indicator, smooth_max};

/// Type of option payoff.
///
/// Provides AD-compatible payoff evaluation using smooth approximations.
/// All payoff computations use `smooth_max` or `smooth_indicator` to maintain
/// differentiability for Enzyme automatic differentiation.
///
/// # Variants
/// - `Call`: max(S - K, 0) payoff using smooth approximation
/// - `Put`: max(K - S, 0) payoff using smooth approximation
/// - `DigitalCall`: 1 if S > K else 0, using smooth indicator
/// - `DigitalPut`: 1 if S < K else 0, using smooth indicator
///
/// # Examples
/// ```
/// use pricer_models::instruments::PayoffType;
///
/// let call = PayoffType::Call;
/// let payoff = call.evaluate(110.0_f64, 100.0, 1e-6);
/// assert!((payoff - 10.0).abs() < 0.01);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PayoffType {
    /// Call option: max(S - K, 0)
    Call,
    /// Put option: max(K - S, 0)
    Put,
    /// Digital call: 1 if S > K else 0
    DigitalCall,
    /// Digital put: 1 if S < K else 0
    DigitalPut,
}

impl PayoffType {
    /// Evaluate the payoff for given spot and strike.
    ///
    /// Uses smooth approximations for AD compatibility:
    /// - Call/Put use `smooth_max` for differentiable max(x, 0)
    /// - Digital options use `smooth_indicator` for differentiable step function
    ///
    /// # Arguments
    /// * `spot` - Current spot price (S)
    /// * `strike` - Strike price (K)
    /// * `epsilon` - Smoothing parameter for AD compatibility
    ///
    /// # Returns
    /// Smooth payoff value maintaining AD tape consistency.
    ///
    /// # Panics
    /// Panics if epsilon <= 0 (delegated to smoothing functions).
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::PayoffType;
    ///
    /// // In-the-money call
    /// let call = PayoffType::Call;
    /// let payoff = call.evaluate(110.0_f64, 100.0, 1e-6);
    /// assert!((payoff - 10.0).abs() < 0.01);
    ///
    /// // Out-of-the-money put
    /// let put = PayoffType::Put;
    /// let payoff = put.evaluate(110.0_f64, 100.0, 1e-6);
    /// assert!(payoff < 0.01);
    /// ```
    #[inline]
    pub fn evaluate<T: Float>(&self, spot: T, strike: T, epsilon: T) -> T {
        let zero = T::zero();
        match self {
            PayoffType::Call => {
                // max(S - K, 0)
                smooth_max(spot - strike, zero, epsilon)
            }
            PayoffType::Put => {
                // max(K - S, 0)
                smooth_max(strike - spot, zero, epsilon)
            }
            PayoffType::DigitalCall => {
                // 1 if S > K else 0 (smoothed)
                smooth_indicator(spot - strike, epsilon)
            }
            PayoffType::DigitalPut => {
                // 1 if S < K else 0 (smoothed)
                smooth_indicator(strike - spot, epsilon)
            }
        }
    }

    /// Returns whether this payoff is a call-type (Call or DigitalCall).
    #[inline]
    pub fn is_call(&self) -> bool {
        matches!(self, PayoffType::Call | PayoffType::DigitalCall)
    }

    /// Returns whether this payoff is a put-type (Put or DigitalPut).
    #[inline]
    pub fn is_put(&self) -> bool {
        matches!(self, PayoffType::Put | PayoffType::DigitalPut)
    }

    /// Returns whether this payoff is digital (DigitalCall or DigitalPut).
    #[inline]
    pub fn is_digital(&self) -> bool {
        matches!(self, PayoffType::DigitalCall | PayoffType::DigitalPut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // Call payoff tests

    #[test]
    fn test_call_payoff_in_the_money() {
        let call = PayoffType::Call;
        let payoff = call.evaluate(110.0_f64, 100.0, 1e-6);
        // Should be approximately 10.0
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_call_payoff_out_of_the_money() {
        let call = PayoffType::Call;
        let payoff = call.evaluate(90.0_f64, 100.0, 1e-6);
        // Should be approximately 0.0
        assert!(payoff < 0.01);
        assert!(payoff >= 0.0); // Should be non-negative
    }

    #[test]
    fn test_call_payoff_at_the_money() {
        let call = PayoffType::Call;
        let payoff = call.evaluate(100.0_f64, 100.0, 1e-6);
        // Should be approximately 0.0
        assert!(payoff < 0.01);
    }

    // Put payoff tests

    #[test]
    fn test_put_payoff_in_the_money() {
        let put = PayoffType::Put;
        let payoff = put.evaluate(90.0_f64, 100.0, 1e-6);
        // Should be approximately 10.0
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_put_payoff_out_of_the_money() {
        let put = PayoffType::Put;
        let payoff = put.evaluate(110.0_f64, 100.0, 1e-6);
        // Should be approximately 0.0
        assert!(payoff < 0.01);
        assert!(payoff >= 0.0); // Should be non-negative
    }

    #[test]
    fn test_put_payoff_at_the_money() {
        let put = PayoffType::Put;
        let payoff = put.evaluate(100.0_f64, 100.0, 1e-6);
        // Should be approximately 0.0
        assert!(payoff < 0.01);
    }

    // Digital call tests

    #[test]
    fn test_digital_call_in_the_money() {
        let digital = PayoffType::DigitalCall;
        let payoff = digital.evaluate(110.0_f64, 100.0, 1e-6);
        // Should be approximately 1.0
        assert!(payoff > 0.99);
    }

    #[test]
    fn test_digital_call_out_of_the_money() {
        let digital = PayoffType::DigitalCall;
        let payoff = digital.evaluate(90.0_f64, 100.0, 1e-6);
        // Should be approximately 0.0
        assert!(payoff < 0.01);
    }

    #[test]
    fn test_digital_call_at_the_money() {
        let digital = PayoffType::DigitalCall;
        let payoff = digital.evaluate(100.0_f64, 100.0, 1e-6);
        // Should be approximately 0.5
        assert_relative_eq!(payoff, 0.5, epsilon = 0.01);
    }

    // Digital put tests

    #[test]
    fn test_digital_put_in_the_money() {
        let digital = PayoffType::DigitalPut;
        let payoff = digital.evaluate(90.0_f64, 100.0, 1e-6);
        // Should be approximately 1.0
        assert!(payoff > 0.99);
    }

    #[test]
    fn test_digital_put_out_of_the_money() {
        let digital = PayoffType::DigitalPut;
        let payoff = digital.evaluate(110.0_f64, 100.0, 1e-6);
        // Should be approximately 0.0
        assert!(payoff < 0.01);
    }

    // Helper function tests

    #[test]
    fn test_is_call() {
        assert!(PayoffType::Call.is_call());
        assert!(PayoffType::DigitalCall.is_call());
        assert!(!PayoffType::Put.is_call());
        assert!(!PayoffType::DigitalPut.is_call());
    }

    #[test]
    fn test_is_put() {
        assert!(PayoffType::Put.is_put());
        assert!(PayoffType::DigitalPut.is_put());
        assert!(!PayoffType::Call.is_put());
        assert!(!PayoffType::DigitalCall.is_put());
    }

    #[test]
    fn test_is_digital() {
        assert!(PayoffType::DigitalCall.is_digital());
        assert!(PayoffType::DigitalPut.is_digital());
        assert!(!PayoffType::Call.is_digital());
        assert!(!PayoffType::Put.is_digital());
    }

    // f32 compatibility test

    #[test]
    fn test_f32_compatibility() {
        let call = PayoffType::Call;
        let payoff = call.evaluate(110.0_f32, 100.0_f32, 1e-4_f32);
        assert!((payoff - 10.0_f32).abs() < 0.1);
    }

    // Clone and equality tests

    #[test]
    fn test_clone_and_equality() {
        let call1 = PayoffType::Call;
        let call2 = call1;
        assert_eq!(call1, call2);
    }

    #[test]
    fn test_debug() {
        let call = PayoffType::Call;
        assert_eq!(format!("{:?}", call), "Call");
    }

    // AD compatibility test with Dual64
    #[test]
    fn test_dual64_compatibility() {
        use num_dual::Dual64;

        let call = PayoffType::Call;
        // Create dual numbers with derivative tracking
        let spot = Dual64::new(110.0, 1.0); // d/dS
        let strike = Dual64::new(100.0, 0.0);
        let epsilon = Dual64::new(1e-6, 0.0);

        let payoff = call.evaluate(spot, strike, epsilon);

        // Payoff should be approximately 10.0
        assert!((payoff.re - 10.0).abs() < 0.01);

        // Delta (derivative w.r.t. spot) should be approximately 1.0 for deep ITM
        assert!(payoff.eps > 0.9);
    }
}
