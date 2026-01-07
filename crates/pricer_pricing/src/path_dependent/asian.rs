//! Asian option payoff implementations.
//!
//! This module provides payoff implementations for Asian options that
//! use path averages:
//!
//! - **Arithmetic Asian**: Payoff based on arithmetic mean of prices
//! - **Geometric Asian**: Payoff based on geometric mean of prices
//!
//! # Smooth Approximations
//!
//! All payoffs use smooth approximations (soft-plus) for AD compatibility.
//! When the `l1l2-integration` feature is enabled, payoffs use
//! `pricer_core::math::smoothing::smooth_max`.

use super::{ObservationType, PathDependentPayoff, PathObserver};
use num_traits::Float;

/// Parameters for Asian option payoffs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AsianParams<T: Float> {
    /// Strike price
    pub strike: T,
    /// Whether this is a call (true) or put (false)
    pub is_call: bool,
    /// Smoothing epsilon for soft approximations
    pub smoothing_epsilon: T,
}

impl<T: Float> AsianParams<T> {
    /// Creates parameters for an Asian call option.
    #[inline]
    pub fn call(strike: T, epsilon: T) -> Self {
        Self {
            strike,
            is_call: true,
            smoothing_epsilon: epsilon,
        }
    }

    /// Creates parameters for an Asian put option.
    #[inline]
    pub fn put(strike: T, epsilon: T) -> Self {
        Self {
            strike,
            is_call: false,
            smoothing_epsilon: epsilon,
        }
    }
}

/// Soft-plus function: smooth approximation of max(x, 0).
///
/// Uses the log-sum-exp formulation for numerical stability.
#[inline]
fn soft_plus<T: Float>(x: T, epsilon: T) -> T {
    let scaled = x / epsilon;
    let twenty = T::from(20.0).unwrap();
    if scaled > twenty {
        x
    } else if scaled < -twenty {
        epsilon * scaled.exp()
    } else {
        epsilon * (T::one() + scaled.exp()).ln()
    }
}

/// Arithmetic average Asian option payoff.
///
/// Payoff is based on the arithmetic mean of observed prices:
/// - Call: max(A - K, 0) where A = (1/n) Σ S_i
/// - Put: max(K - A, 0)
#[derive(Clone, Copy, Debug)]
pub struct AsianArithmeticPayoff<T: Float> {
    params: AsianParams<T>,
}

impl<T: Float> AsianArithmeticPayoff<T> {
    /// Creates a new arithmetic Asian payoff.
    #[inline]
    pub fn new(params: AsianParams<T>) -> Self {
        Self { params }
    }

    /// Creates a call option payoff.
    #[inline]
    pub fn call(strike: T, epsilon: T) -> Self {
        Self::new(AsianParams::call(strike, epsilon))
    }

    /// Creates a put option payoff.
    #[inline]
    pub fn put(strike: T, epsilon: T) -> Self {
        Self::new(AsianParams::put(strike, epsilon))
    }
}

impl<T: Float + Send + Sync> PathDependentPayoff<T> for AsianArithmeticPayoff<T> {
    fn compute(&self, _path: &[T], observer: &PathObserver<T>) -> T {
        let avg = observer.arithmetic_average();
        let intrinsic = if self.params.is_call {
            avg - self.params.strike
        } else {
            self.params.strike - avg
        };
        soft_plus(intrinsic, self.params.smoothing_epsilon)
    }

    fn required_observations(&self) -> ObservationType {
        ObservationType::arithmetic_asian()
    }

    fn smoothing_epsilon(&self) -> T {
        self.params.smoothing_epsilon
    }
}

/// Geometric average Asian option payoff.
///
/// Payoff is based on the geometric mean of observed prices:
/// - Call: max(G - K, 0) where G = (Π S_i)^(1/n)
/// - Put: max(K - G, 0)
///
/// Geometric Asian options have closed-form solutions under GBM,
/// making them useful for testing.
#[derive(Clone, Copy, Debug)]
pub struct AsianGeometricPayoff<T: Float> {
    params: AsianParams<T>,
}

impl<T: Float> AsianGeometricPayoff<T> {
    /// Creates a new geometric Asian payoff.
    #[inline]
    pub fn new(params: AsianParams<T>) -> Self {
        Self { params }
    }

    /// Creates a call option payoff.
    #[inline]
    pub fn call(strike: T, epsilon: T) -> Self {
        Self::new(AsianParams::call(strike, epsilon))
    }

    /// Creates a put option payoff.
    #[inline]
    pub fn put(strike: T, epsilon: T) -> Self {
        Self::new(AsianParams::put(strike, epsilon))
    }
}

impl<T: Float + Send + Sync> PathDependentPayoff<T> for AsianGeometricPayoff<T> {
    fn compute(&self, _path: &[T], observer: &PathObserver<T>) -> T {
        let geo_avg = observer.geometric_average();
        let intrinsic = if self.params.is_call {
            geo_avg - self.params.strike
        } else {
            self.params.strike - geo_avg
        };
        soft_plus(intrinsic, self.params.smoothing_epsilon)
    }

    fn required_observations(&self) -> ObservationType {
        ObservationType::geometric_asian()
    }

    fn smoothing_epsilon(&self) -> T {
        self.params.smoothing_epsilon
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ========================================================================
    // AsianParams Tests
    // ========================================================================

    #[test]
    fn test_asian_params_call() {
        let params = AsianParams::call(100.0_f64, 1e-6);
        assert_eq!(params.strike, 100.0);
        assert!(params.is_call);
        assert_eq!(params.smoothing_epsilon, 1e-6);
    }

    #[test]
    fn test_asian_params_put() {
        let params = AsianParams::put(100.0_f64, 1e-6);
        assert_eq!(params.strike, 100.0);
        assert!(!params.is_call);
    }

    // ========================================================================
    // Soft-plus Tests
    // ========================================================================

    #[test]
    fn test_soft_plus_positive() {
        let result = soft_plus(10.0_f64, 0.01);
        assert_relative_eq!(result, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_soft_plus_negative() {
        let result = soft_plus(-10.0_f64, 0.01);
        assert!(result < 0.01);
        assert!(result >= 0.0);
    }

    #[test]
    fn test_soft_plus_at_zero() {
        let epsilon = 1.0_f64;
        let result = soft_plus(0.0, epsilon);
        // softplus(0) = ε * ln(2) ≈ 0.693
        assert_relative_eq!(result, 2.0_f64.ln(), epsilon = 1e-10);
    }

    // ========================================================================
    // AsianArithmeticPayoff Tests
    // ========================================================================

    #[test]
    fn test_arithmetic_asian_call_itm() {
        let payoff = AsianArithmeticPayoff::call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Observe prices: [100, 110, 120] -> avg = 110
        observer.observe(100.0);
        observer.observe(110.0);
        observer.observe(120.0);

        let result = payoff.compute(&[], &observer);
        // avg - strike = 110 - 100 = 10
        assert_relative_eq!(result, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_arithmetic_asian_call_otm() {
        let payoff = AsianArithmeticPayoff::call(120.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Observe prices: [100, 110] -> avg = 105
        observer.observe(100.0);
        observer.observe(110.0);

        let result = payoff.compute(&[], &observer);
        // avg - strike = 105 - 120 = -15 -> ~0
        assert!(result < 0.01);
        assert!(result >= 0.0);
    }

    #[test]
    fn test_arithmetic_asian_put_itm() {
        let payoff = AsianArithmeticPayoff::put(120.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Observe prices: [100, 110] -> avg = 105
        observer.observe(100.0);
        observer.observe(110.0);

        let result = payoff.compute(&[], &observer);
        // strike - avg = 120 - 105 = 15
        assert_relative_eq!(result, 15.0, epsilon = 0.01);
    }

    #[test]
    fn test_arithmetic_asian_required_observations() {
        let payoff = AsianArithmeticPayoff::call(100.0_f64, 1e-6);
        let obs = payoff.required_observations();
        assert!(obs.needs_average);
        assert!(!obs.needs_geometric_average);
        assert!(obs.needs_terminal);
    }

    // ========================================================================
    // AsianGeometricPayoff Tests
    // ========================================================================

    #[test]
    fn test_geometric_asian_call_equal_prices() {
        let payoff = AsianGeometricPayoff::call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // All prices equal: geometric mean = 110
        observer.observe(110.0);
        observer.observe(110.0);
        observer.observe(110.0);

        let result = payoff.compute(&[], &observer);
        // geo_avg - strike = 110 - 100 = 10
        assert_relative_eq!(result, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_geometric_asian_call_varied_prices() {
        let payoff = AsianGeometricPayoff::call(3.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Geometric mean of [2, 8] = sqrt(16) = 4
        observer.observe(2.0);
        observer.observe(8.0);

        let result = payoff.compute(&[], &observer);
        // geo_avg - strike = 4 - 3 = 1
        assert_relative_eq!(result, 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_geometric_asian_put_itm() {
        let payoff = AsianGeometricPayoff::put(5.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Geometric mean of [2, 8] = 4
        observer.observe(2.0);
        observer.observe(8.0);

        let result = payoff.compute(&[], &observer);
        // strike - geo_avg = 5 - 4 = 1
        assert_relative_eq!(result, 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_geometric_asian_required_observations() {
        let payoff = AsianGeometricPayoff::call(100.0_f64, 1e-6);
        let obs = payoff.required_observations();
        assert!(!obs.needs_average);
        assert!(obs.needs_geometric_average);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_geometric_asian_smoothing_epsilon() {
        let epsilon = 1e-4_f64;
        let payoff = AsianGeometricPayoff::call(100.0, epsilon);
        assert_eq!(payoff.smoothing_epsilon(), epsilon);
    }

    // ========================================================================
    // Put-Call Parity Tests
    // ========================================================================

    #[test]
    fn test_arithmetic_asian_put_call_relation() {
        let strike = 100.0_f64;
        let epsilon = 1e-8;
        let call = AsianArithmeticPayoff::call(strike, epsilon);
        let put = AsianArithmeticPayoff::put(strike, epsilon);

        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(90.0);
        observer.observe(100.0);
        observer.observe(130.0); // avg = 320/3 ≈ 106.67

        let call_payoff = call.compute(&[], &observer);
        let put_payoff = put.compute(&[], &observer);
        let avg = observer.arithmetic_average();

        // Approximate put-call parity: C - P ≈ A - K for deep ITM/OTM
        assert_relative_eq!(call_payoff - put_payoff, avg - strike, epsilon = 0.01);
    }
}
