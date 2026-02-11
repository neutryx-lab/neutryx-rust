//! Asian option payoff implementations.

use num_traits::Float;

use super::{ObservationType, PathDependentPayoff, PathObserver};

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
#[derive(Clone, Copy, Debug)]
pub struct AsianArithmeticPayoff<T: Float> {
    params: AsianParams<T>,
}

impl<T: Float> AsianArithmeticPayoff<T> {
    /// Creates a new arithmetic Asian payoff.
    #[inline]
    pub fn new(params: AsianParams<T>) -> Self { Self { params } }

    /// Creates a call option payoff.
    #[inline]
    pub fn call(strike: T, epsilon: T) -> Self { Self::new(AsianParams::call(strike, epsilon)) }

    /// Creates a put option payoff.
    #[inline]
    pub fn put(strike: T, epsilon: T) -> Self { Self::new(AsianParams::put(strike, epsilon)) }
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

    fn required_observations(&self) -> ObservationType { ObservationType::arithmetic_asian() }

    fn smoothing_epsilon(&self) -> T { self.params.smoothing_epsilon }
}

/// Geometric average Asian option payoff.
#[derive(Clone, Copy, Debug)]
pub struct AsianGeometricPayoff<T: Float> {
    params: AsianParams<T>,
}

impl<T: Float> AsianGeometricPayoff<T> {
    /// Creates a new geometric Asian payoff.
    #[inline]
    pub fn new(params: AsianParams<T>) -> Self { Self { params } }

    /// Creates a call option payoff.
    #[inline]
    pub fn call(strike: T, epsilon: T) -> Self { Self::new(AsianParams::call(strike, epsilon)) }

    /// Creates a put option payoff.
    #[inline]
    pub fn put(strike: T, epsilon: T) -> Self { Self::new(AsianParams::put(strike, epsilon)) }
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

    fn required_observations(&self) -> ObservationType { ObservationType::geometric_asian() }

    fn smoothing_epsilon(&self) -> T { self.params.smoothing_epsilon }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

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
        assert_relative_eq!(result, 2.0_f64.ln(), epsilon = 1e-10);
    }

    #[test]
    fn test_arithmetic_asian_call_itm() {
        let payoff = AsianArithmeticPayoff::call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(110.0);
        observer.observe(120.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_arithmetic_asian_call_otm() {
        let payoff = AsianArithmeticPayoff::call(120.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(110.0);

        let result = payoff.compute(&[], &observer);
        assert!(result < 0.01);
        assert!(result >= 0.0);
    }

    #[test]
    fn test_arithmetic_asian_put_itm() {
        let payoff = AsianArithmeticPayoff::put(120.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(110.0);

        let result = payoff.compute(&[], &observer);
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

    #[test]
    fn test_geometric_asian_call_equal_prices() {
        let payoff = AsianGeometricPayoff::call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(110.0);
        observer.observe(110.0);
        observer.observe(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_geometric_asian_call_varied_prices() {
        let payoff = AsianGeometricPayoff::call(3.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(2.0);
        observer.observe(8.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_geometric_asian_put_itm() {
        let payoff = AsianGeometricPayoff::put(5.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(2.0);
        observer.observe(8.0);

        let result = payoff.compute(&[], &observer);
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

    #[test]
    fn test_arithmetic_asian_put_call_relation() {
        let strike = 100.0_f64;
        let epsilon = 1e-8;
        let call = AsianArithmeticPayoff::call(strike, epsilon);
        let put = AsianArithmeticPayoff::put(strike, epsilon);

        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(90.0);
        observer.observe(100.0);
        observer.observe(130.0);

        let call_payoff = call.compute(&[], &observer);
        let put_payoff = put.compute(&[], &observer);
        let avg = observer.arithmetic_average();

        assert_relative_eq!(call_payoff - put_payoff, avg - strike, epsilon = 0.01);
    }
}
