//! Enzyme-compatible smooth approximations of discontinuous functions.

use num_traits::Float;

/// Default smoothing parameter (epsilon).
pub const DEFAULT_EPSILON: f64 = 1e-6;

/// Smooth approximation of `max(a, b)` using `(a + b + sqrt((a-b)^2 + e^2)) / 2`.
#[inline]
pub fn smooth_max<T: Float>(a: T, b: T, epsilon: T) -> T {
    let diff = a - b;
    let sum = a + b;
    let two = T::from(2.0).unwrap();
    (sum + (diff * diff + epsilon * epsilon).sqrt()) / two
}

/// Smooth approximation of `min(a, b)` using `min(a, b) = -max(-a, -b)`.
#[inline]
pub fn smooth_min<T: Float>(a: T, b: T, epsilon: T) -> T { -smooth_max(-a, -b, epsilon) }

/// Smooth approximation of `max(x, 0)` (soft-plus): `e * ln(1 + exp(x/e))`.
#[inline]
pub fn smooth_relu<T: Float>(x: T, epsilon: T) -> T {
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

/// Derivative of smooth_relu (sigmoid function).
#[inline]
pub fn smooth_relu_derivative<T: Float>(x: T, epsilon: T) -> T { smooth_indicator(x, epsilon) }

/// Smooth indicator (sigmoid): `1 / (1 + exp(-x/e))`.
#[inline]
pub fn smooth_indicator<T: Float>(x: T, epsilon: T) -> T {
    let scaled = x / epsilon;
    let twenty = T::from(20.0).unwrap();

    if scaled > twenty {
        T::one()
    } else if scaled < -twenty {
        T::zero()
    } else {
        T::one() / (T::one() + (-scaled).exp())
    }
}

/// Derivative of smooth_indicator (sigmoid derivative).
#[inline]
pub fn smooth_indicator_derivative<T: Float>(x: T, epsilon: T) -> T {
    let sig = smooth_indicator(x, epsilon);
    sig * (T::one() - sig) / epsilon
}

/// Smooth absolute value: `sqrt(x^2 + e^2)`.
#[inline]
pub fn smooth_abs<T: Float>(x: T, epsilon: T) -> T { (x * x + epsilon * epsilon).sqrt() }

/// Derivative of smooth_abs: `x / sqrt(x^2 + e^2)`.
#[inline]
pub fn smooth_abs_derivative<T: Float>(x: T, epsilon: T) -> T { x / smooth_abs(x, epsilon) }

/// Smooth European call payoff: `max(S - K, 0)`.
#[inline]
pub fn smooth_call_payoff<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    smooth_relu(spot - strike, epsilon)
}

/// Smooth call delta: `sigmoid((S - K) / e)`.
#[inline]
pub fn smooth_call_delta<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    smooth_indicator(spot - strike, epsilon)
}

/// Smooth European put payoff: `max(K - S, 0)`.
#[inline]
pub fn smooth_put_payoff<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    smooth_relu(strike - spot, epsilon)
}

/// Smooth put delta: `-sigmoid((K - S) / e)`.
#[inline]
pub fn smooth_put_delta<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    -smooth_indicator(strike - spot, epsilon)
}

/// Smooth digital call payoff: `1 if S > K else 0`.
#[inline]
pub fn smooth_digital_call<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    smooth_indicator(spot - strike, epsilon)
}

/// Smooth digital put payoff: `1 if S < K else 0`.
#[inline]
pub fn smooth_digital_put<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    smooth_indicator(strike - spot, epsilon)
}

/// Smooth up barrier alive indicator: ~1 if spot < barrier, ~0 otherwise.
#[inline]
pub fn smooth_up_barrier_alive<T: Float>(spot: T, barrier: T, epsilon: T) -> T {
    smooth_indicator(barrier - spot, epsilon)
}

/// Smooth down barrier alive indicator: ~1 if spot > barrier, ~0 otherwise.
#[inline]
pub fn smooth_down_barrier_alive<T: Float>(spot: T, barrier: T, epsilon: T) -> T {
    smooth_indicator(spot - barrier, epsilon)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    const EPS: f64 = 1e-6;

    #[test]
    fn test_smooth_max_positive_dominates() {
        let result = smooth_max(5.0, 3.0, EPS);
        assert_relative_eq!(result, 5.0, epsilon = 1e-5);
    }

    #[test]
    fn test_smooth_max_negative_vs_positive() {
        let result = smooth_max(-1.0, 1.0, EPS);
        assert_relative_eq!(result, 1.0, epsilon = 1e-5);
    }

    #[test]
    fn test_smooth_max_equal_values() {
        let result = smooth_max(3.0, 3.0, EPS);
        assert_relative_eq!(result, 3.0, epsilon = 1e-5);
    }

    #[test]
    fn test_smooth_max_with_zero() {
        let result = smooth_max(5.0, 0.0, EPS);
        assert_relative_eq!(result, 5.0, epsilon = 1e-5);

        let result = smooth_max(-5.0, 0.0, EPS);
        assert_relative_eq!(result, 0.0, epsilon = 1e-5);
    }

    #[test]
    fn test_smooth_min_basic() {
        let result = smooth_min(5.0, 3.0, EPS);
        assert_relative_eq!(result, 3.0, epsilon = 1e-5);

        let result = smooth_min(-1.0, 1.0, EPS);
        assert_relative_eq!(result, -1.0, epsilon = 1e-5);
    }

    #[test]
    fn test_smooth_relu_positive() {
        let result = smooth_relu(5.0, EPS);
        assert_relative_eq!(result, 5.0, epsilon = 1e-5);
    }

    #[test]
    fn test_smooth_relu_negative() {
        let result = smooth_relu(-5.0, EPS);
        assert!(result < 1e-5);
        assert!(result >= 0.0);
    }

    #[test]
    fn test_smooth_relu_at_zero() {
        let result = smooth_relu(0.0, 1.0);
        assert_relative_eq!(result, 1.0_f64.ln_1p(), epsilon = 1e-10);
    }

    #[test]
    fn test_smooth_indicator_positive() {
        let result = smooth_indicator(10.0, 1.0);
        assert!(result > 0.99);
    }

    #[test]
    fn test_smooth_indicator_negative() {
        let result = smooth_indicator(-10.0, 1.0);
        assert!(result < 0.01);
    }

    #[test]
    fn test_smooth_indicator_at_zero() {
        let result = smooth_indicator(0.0, 1.0);
        assert_relative_eq!(result, 0.5, epsilon = 0.01);
    }

    #[test]
    fn test_smooth_abs_positive() {
        let result = smooth_abs(5.0, EPS);
        assert_relative_eq!(result, 5.0, epsilon = 1e-5);
    }

    #[test]
    fn test_smooth_abs_negative() {
        let result = smooth_abs(-5.0, EPS);
        assert_relative_eq!(result, 5.0, epsilon = 1e-5);
    }

    #[test]
    fn test_smooth_abs_at_zero() {
        let result = smooth_abs(0.0, EPS);
        assert_relative_eq!(result, EPS, epsilon = 1e-10);
    }

    #[test]
    fn test_smooth_call_payoff_itm() {
        let payoff = smooth_call_payoff(110.0, 100.0, EPS);
        assert_relative_eq!(payoff, 10.0, epsilon = 1e-5);
    }

    #[test]
    fn test_smooth_call_payoff_otm() {
        let payoff = smooth_call_payoff(90.0, 100.0, EPS);
        assert!(payoff < 1e-5);
    }

    #[test]
    fn test_smooth_call_payoff_atm() {
        let payoff = smooth_call_payoff(100.0, 100.0, 1.0);
        assert!(payoff > 0.0);
        assert!(payoff < 1.0);
    }

    #[test]
    fn test_smooth_put_payoff_itm() {
        let payoff = smooth_put_payoff(90.0, 100.0, EPS);
        assert_relative_eq!(payoff, 10.0, epsilon = 1e-5);
    }

    #[test]
    fn test_smooth_put_payoff_otm() {
        let payoff = smooth_put_payoff(110.0, 100.0, EPS);
        assert!(payoff < 1e-5);
    }

    #[test]
    fn test_smooth_call_delta_itm() {
        let delta = smooth_call_delta(110.0, 100.0, 1.0);
        assert!(delta > 0.99);
    }

    #[test]
    fn test_smooth_call_delta_otm() {
        let delta = smooth_call_delta(90.0, 100.0, 1.0);
        assert!(delta < 0.01);
    }

    #[test]
    fn test_smooth_put_delta_itm() {
        let delta = smooth_put_delta(90.0, 100.0, 1.0);
        assert!(delta < -0.99);
    }

    #[test]
    fn test_smooth_put_delta_otm() {
        let delta = smooth_put_delta(110.0, 100.0, 1.0);
        assert!(delta > -0.01);
    }

    #[test]
    fn test_smooth_digital_call_itm() {
        let payoff = smooth_digital_call(110.0, 100.0, 1.0);
        assert!(payoff > 0.99);
    }

    #[test]
    fn test_smooth_digital_call_otm() {
        let payoff = smooth_digital_call(90.0, 100.0, 1.0);
        assert!(payoff < 0.01);
    }

    #[test]
    fn test_smooth_digital_put_itm() {
        let payoff = smooth_digital_put(90.0, 100.0, 1.0);
        assert!(payoff > 0.99);
    }

    #[test]
    fn test_smooth_digital_put_otm() {
        let payoff = smooth_digital_put(110.0, 100.0, 1.0);
        assert!(payoff < 0.01);
    }

    #[test]
    fn test_smooth_up_barrier_alive_below() {
        let alive = smooth_up_barrier_alive(100.0, 120.0, 1.0);
        assert!(alive > 0.99);
    }

    #[test]
    fn test_smooth_up_barrier_alive_above() {
        let alive = smooth_up_barrier_alive(130.0, 120.0, 1.0);
        assert!(alive < 0.01);
    }

    #[test]
    fn test_smooth_down_barrier_alive_above() {
        let alive = smooth_down_barrier_alive(100.0, 80.0, 1.0);
        assert!(alive > 0.99);
    }

    #[test]
    fn test_smooth_down_barrier_alive_below() {
        let alive = smooth_down_barrier_alive(70.0, 80.0, 1.0);
        assert!(alive < 0.01);
    }

    #[test]
    fn test_smooth_relu_derivative_positive() {
        let deriv = smooth_relu_derivative(10.0, 1.0);
        assert!(deriv > 0.99);
    }

    #[test]
    fn test_smooth_relu_derivative_negative() {
        let deriv = smooth_relu_derivative(-10.0, 1.0);
        assert!(deriv < 0.01);
    }

    #[test]
    fn test_smooth_indicator_derivative_at_zero() {
        let deriv = smooth_indicator_derivative(0.0, 1.0);
        assert_relative_eq!(deriv, 0.25, epsilon = 0.01);
    }

    #[test]
    fn test_smooth_abs_derivative() {
        let deriv = smooth_abs_derivative(5.0, EPS);
        assert_relative_eq!(deriv, 1.0, epsilon = 1e-5);

        let deriv = smooth_abs_derivative(-5.0, EPS);
        assert_relative_eq!(deriv, -1.0, epsilon = 1e-5);
    }

    #[test]
    fn test_put_call_parity_smooth() {
        let spot = 100.0;
        let strike = 100.0;
        let eps = 1e-6;

        let call = smooth_call_payoff(spot, strike, eps);
        let put = smooth_put_payoff(spot, strike, eps);

        assert_relative_eq!(call, put, epsilon = 1e-5);
    }

    #[test]
    fn test_digital_put_call_parity() {
        let spot = 100.0;
        let strike = 100.0;
        let eps = 1.0;

        let dig_call = smooth_digital_call(spot, strike, eps);
        let dig_put = smooth_digital_put(spot, strike, eps);

        assert_relative_eq!(dig_call + dig_put, 1.0, epsilon = 0.01);
    }
}
