//! Enzyme-compatible smooth functions for automatic differentiation.
//!
//! This module provides smooth approximations of discontinuous functions
//! that are compatible with Enzyme LLVM-level automatic differentiation.
//!
//! # Why Smoothing?
//!
//! Enzyme AD requires differentiable functions. Common financial functions
//! like `max(0, x)` and `if x > 0 then 1 else 0` have discontinuous derivatives.
//! These smooth approximations replace step functions with sigmoids and
//! max functions with soft-plus, ensuring well-defined gradients everywhere.
//!
//! # Requirements Coverage
//!
//! - Requirement 6.1: smooth_payoff (call, put, digital)
//! - Requirement 6.2: smooth approximation patterns
//! - Requirement 6.3: smooth_indicator, smooth_max usage
//!
//! # Functions
//!
//! | Function | Approximates | Formula |
//! |----------|--------------|---------|
//! | `smooth_max` | `max(a, b)` | `(a + b + sqrt((a-b)² + ε²)) / 2` |
//! | `smooth_relu` | `max(x, 0)` | `ε × ln(1 + exp(x/ε))` |
//! | `smooth_indicator` | `1 if x > 0 else 0` | `1 / (1 + exp(-x/ε))` |
//! | `smooth_call_payoff` | `max(S - K, 0)` | `smooth_relu(S - K)` |
//! | `smooth_put_payoff` | `max(K - S, 0)` | `smooth_relu(K - S)` |
//! | `smooth_digital_call` | `1 if S > K else 0` | `smooth_indicator(S - K)` |

use num_traits::Float;

/// Default smoothing parameter (epsilon).
///
/// This value provides good accuracy while ensuring numerical stability.
/// For most option pricing applications, 1e-6 is sufficient.
pub const DEFAULT_EPSILON: f64 = 1e-6;

// =============================================================================
// Core Smooth Functions
// =============================================================================

/// Smooth approximation of `max(a, b)`.
///
/// Uses the formula: `(a + b + sqrt((a-b)² + ε²)) / 2`
///
/// As ε → 0, this converges to the exact max function.
/// The function is differentiable everywhere, suitable for Enzyme AD.
///
/// # Arguments
///
/// * `a` - First value
/// * `b` - Second value
/// * `epsilon` - Smoothing parameter (smaller = sharper transition)
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::enzyme::smooth::smooth_max;
///
/// let result: f64 = smooth_max(5.0, 3.0, 1e-6);
/// assert!((result - 5.0).abs() < 1e-5);
///
/// let result: f64 = smooth_max(-1.0, 1.0, 1e-6);
/// assert!((result - 1.0).abs() < 1e-5);
/// ```
#[inline]
pub fn smooth_max<T: Float>(a: T, b: T, epsilon: T) -> T {
    let diff = a - b;
    let sum = a + b;
    let two = T::from(2.0).unwrap();
    (sum + (diff * diff + epsilon * epsilon).sqrt()) / two
}

/// Smooth approximation of `min(a, b)`.
///
/// Uses the identity: `min(a, b) = -max(-a, -b)`
///
/// # Arguments
///
/// * `a` - First value
/// * `b` - Second value
/// * `epsilon` - Smoothing parameter
#[inline]
pub fn smooth_min<T: Float>(a: T, b: T, epsilon: T) -> T {
    -smooth_max(-a, -b, epsilon)
}

/// Smooth approximation of `max(x, 0)` (ReLU / soft-plus).
///
/// Uses the formula: `ε × ln(1 + exp(x/ε))`
///
/// As ε → 0, this converges to `max(x, 0)`.
/// Also known as the soft-plus function in machine learning.
///
/// # Arguments
///
/// * `x` - Input value
/// * `epsilon` - Smoothing parameter
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::enzyme::smooth::smooth_relu;
///
/// // Positive input
/// let result: f64 = smooth_relu(5.0, 1e-6);
/// assert!((result - 5.0).abs() < 1e-5);
///
/// // Negative input
/// let result: f64 = smooth_relu(-5.0, 1e-6);
/// assert!(result < 1e-5);
/// ```
#[inline]
pub fn smooth_relu<T: Float>(x: T, epsilon: T) -> T {
    let scaled = x / epsilon;
    let twenty = T::from(20.0).unwrap();

    if scaled > twenty {
        // Avoid overflow: ln(1 + exp(20)) ≈ 20
        x
    } else if scaled < -twenty {
        // Avoid underflow: use exp directly
        epsilon * scaled.exp()
    } else {
        epsilon * (T::one() + scaled.exp()).ln()
    }
}

/// Derivative of smooth_relu (sigmoid function).
///
/// ```text
/// d/dx smooth_relu(x, ε) = sigmoid(x/ε) = 1 / (1 + exp(-x/ε))
/// ```
///
/// This is the smooth approximation of the Heaviside step function.
///
/// # Arguments
///
/// * `x` - Input value
/// * `epsilon` - Smoothing parameter
///
/// # Returns
///
/// Value in (0, 1), approximating `1 if x > 0 else 0`.
#[inline]
pub fn smooth_relu_derivative<T: Float>(x: T, epsilon: T) -> T {
    smooth_indicator(x, epsilon)
}

/// Smooth indicator function (sigmoid).
///
/// Approximates the Heaviside step function:
/// - Returns ~0 for x << 0
/// - Returns ~0.5 for x ≈ 0
/// - Returns ~1 for x >> 0
///
/// # Formula
///
/// ```text
/// sigmoid(x/ε) = 1 / (1 + exp(-x/ε))
/// ```
///
/// # Arguments
///
/// * `x` - Input value
/// * `epsilon` - Smoothing parameter (smaller = sharper transition)
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::enzyme::smooth::smooth_indicator;
///
/// // Far positive
/// let result: f64 = smooth_indicator(10.0, 1.0);
/// assert!(result > 0.99);
///
/// // Far negative
/// let result: f64 = smooth_indicator(-10.0, 1.0);
/// assert!(result < 0.01);
///
/// // At zero
/// let result: f64 = smooth_indicator(0.0, 1.0);
/// assert!((result - 0.5).abs() < 0.01);
/// ```
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
///
/// ```text
/// d/dx sigmoid(x/ε) = sigmoid(x/ε) × (1 - sigmoid(x/ε)) / ε
/// ```
///
/// This is the smooth approximation of the Dirac delta function.
#[inline]
pub fn smooth_indicator_derivative<T: Float>(x: T, epsilon: T) -> T {
    let sig = smooth_indicator(x, epsilon);
    sig * (T::one() - sig) / epsilon
}

/// Smooth absolute value function.
///
/// Uses the formula: `sqrt(x² + ε²)`
///
/// As ε → 0, this converges to `|x|`.
/// Differentiable at x = 0 (unlike true absolute value).
///
/// # Arguments
///
/// * `x` - Input value
/// * `epsilon` - Smoothing parameter
#[inline]
pub fn smooth_abs<T: Float>(x: T, epsilon: T) -> T {
    (x * x + epsilon * epsilon).sqrt()
}

/// Derivative of smooth_abs.
///
/// ```text
/// d/dx sqrt(x² + ε²) = x / sqrt(x² + ε²)
/// ```
#[inline]
pub fn smooth_abs_derivative<T: Float>(x: T, epsilon: T) -> T {
    x / smooth_abs(x, epsilon)
}

// =============================================================================
// Option Payoff Functions
// =============================================================================

/// Smooth European call option payoff.
///
/// Approximates `max(S - K, 0)` using soft-plus.
///
/// # Arguments
///
/// * `spot` - Current/terminal spot price S
/// * `strike` - Strike price K
/// * `epsilon` - Smoothing parameter
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::enzyme::smooth::smooth_call_payoff;
///
/// // ITM call
/// let payoff: f64 = smooth_call_payoff(110.0, 100.0, 1e-6);
/// assert!((payoff - 10.0).abs() < 1e-5);
///
/// // OTM call
/// let payoff: f64 = smooth_call_payoff(90.0, 100.0, 1e-6);
/// assert!(payoff < 1e-5);
/// ```
#[inline]
pub fn smooth_call_payoff<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    smooth_relu(spot - strike, epsilon)
}

/// Derivative of smooth call payoff with respect to spot.
///
/// This is Delta for a digital-approximated call.
/// ```text
/// d/dS smooth_call(S, K, ε) = sigmoid((S - K) / ε)
/// ```
#[inline]
pub fn smooth_call_delta<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    smooth_indicator(spot - strike, epsilon)
}

/// Smooth European put option payoff.
///
/// Approximates `max(K - S, 0)` using soft-plus.
///
/// # Arguments
///
/// * `spot` - Current/terminal spot price S
/// * `strike` - Strike price K
/// * `epsilon` - Smoothing parameter
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::enzyme::smooth::smooth_put_payoff;
///
/// // ITM put
/// let payoff: f64 = smooth_put_payoff(90.0, 100.0, 1e-6);
/// assert!((payoff - 10.0).abs() < 1e-5);
///
/// // OTM put
/// let payoff: f64 = smooth_put_payoff(110.0, 100.0, 1e-6);
/// assert!(payoff < 1e-5);
/// ```
#[inline]
pub fn smooth_put_payoff<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    smooth_relu(strike - spot, epsilon)
}

/// Derivative of smooth put payoff with respect to spot.
///
/// ```text
/// d/dS smooth_put(S, K, ε) = -sigmoid((K - S) / ε)
/// ```
#[inline]
pub fn smooth_put_delta<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    -smooth_indicator(strike - spot, epsilon)
}

/// Smooth digital (binary) call option payoff.
///
/// Pays 1 if S > K at expiry, 0 otherwise.
/// Uses sigmoid as smooth approximation.
///
/// # Arguments
///
/// * `spot` - Current/terminal spot price S
/// * `strike` - Strike price K
/// * `epsilon` - Smoothing parameter
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::enzyme::smooth::smooth_digital_call;
///
/// // Deep ITM
/// let payoff = smooth_digital_call(110.0, 100.0, 1.0);
/// assert!(payoff > 0.99);
///
/// // Deep OTM
/// let payoff = smooth_digital_call(90.0, 100.0, 1.0);
/// assert!(payoff < 0.01);
/// ```
#[inline]
pub fn smooth_digital_call<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    smooth_indicator(spot - strike, epsilon)
}

/// Smooth digital (binary) put option payoff.
///
/// Pays 1 if S < K at expiry, 0 otherwise.
///
/// # Arguments
///
/// * `spot` - Current/terminal spot price S
/// * `strike` - Strike price K
/// * `epsilon` - Smoothing parameter
#[inline]
pub fn smooth_digital_put<T: Float>(spot: T, strike: T, epsilon: T) -> T {
    smooth_indicator(strike - spot, epsilon)
}

// =============================================================================
// Barrier Functions
// =============================================================================

/// Smooth barrier indicator (up barrier).
///
/// Returns ~0 if S has crossed barrier from below, ~1 otherwise.
/// Used for up-and-out barrier options.
///
/// # Arguments
///
/// * `spot` - Current spot price
/// * `barrier` - Barrier level (above spot)
/// * `epsilon` - Smoothing parameter
#[inline]
pub fn smooth_up_barrier_alive<T: Float>(spot: T, barrier: T, epsilon: T) -> T {
    smooth_indicator(barrier - spot, epsilon)
}

/// Smooth barrier indicator (down barrier).
///
/// Returns ~0 if S has crossed barrier from above, ~1 otherwise.
/// Used for down-and-out barrier options.
///
/// # Arguments
///
/// * `spot` - Current spot price
/// * `barrier` - Barrier level (below spot)
/// * `epsilon` - Smoothing parameter
#[inline]
pub fn smooth_down_barrier_alive<T: Float>(spot: T, barrier: T, epsilon: T) -> T {
    smooth_indicator(spot - barrier, epsilon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    const EPS: f64 = 1e-6;

    // =============================================================================
    // smooth_max tests
    // =============================================================================

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

    // =============================================================================
    // smooth_min tests
    // =============================================================================

    #[test]
    fn test_smooth_min_basic() {
        let result = smooth_min(5.0, 3.0, EPS);
        assert_relative_eq!(result, 3.0, epsilon = 1e-5);

        let result = smooth_min(-1.0, 1.0, EPS);
        assert_relative_eq!(result, -1.0, epsilon = 1e-5);
    }

    // =============================================================================
    // smooth_relu tests
    // =============================================================================

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
        // At x=0, smooth_relu(0, ε) = ε × ln(2) ≈ 0.693ε
        assert_relative_eq!(result, 1.0_f64.ln_1p(), epsilon = 1e-10);
    }

    // =============================================================================
    // smooth_indicator tests
    // =============================================================================

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

    // =============================================================================
    // smooth_abs tests
    // =============================================================================

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

    // =============================================================================
    // Call/Put payoff tests
    // =============================================================================

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
        // At ATM, payoff ≈ ε × ln(2)
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

    // =============================================================================
    // Delta tests
    // =============================================================================

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

    // =============================================================================
    // Digital payoff tests
    // =============================================================================

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

    // =============================================================================
    // Barrier tests
    // =============================================================================

    #[test]
    fn test_smooth_up_barrier_alive_below() {
        // Spot below barrier = alive
        let alive = smooth_up_barrier_alive(100.0, 120.0, 1.0);
        assert!(alive > 0.99);
    }

    #[test]
    fn test_smooth_up_barrier_alive_above() {
        // Spot above barrier = knocked out
        let alive = smooth_up_barrier_alive(130.0, 120.0, 1.0);
        assert!(alive < 0.01);
    }

    #[test]
    fn test_smooth_down_barrier_alive_above() {
        // Spot above barrier = alive
        let alive = smooth_down_barrier_alive(100.0, 80.0, 1.0);
        assert!(alive > 0.99);
    }

    #[test]
    fn test_smooth_down_barrier_alive_below() {
        // Spot below barrier = knocked out
        let alive = smooth_down_barrier_alive(70.0, 80.0, 1.0);
        assert!(alive < 0.01);
    }

    // =============================================================================
    // Derivative tests
    // =============================================================================

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
        // At x=0, derivative is 0.5 * 0.5 / 1.0 = 0.25
        assert_relative_eq!(deriv, 0.25, epsilon = 0.01);
    }

    #[test]
    fn test_smooth_abs_derivative() {
        let deriv = smooth_abs_derivative(5.0, EPS);
        assert_relative_eq!(deriv, 1.0, epsilon = 1e-5);

        let deriv = smooth_abs_derivative(-5.0, EPS);
        assert_relative_eq!(deriv, -1.0, epsilon = 1e-5);
    }

    // =============================================================================
    // Put-Call parity for smooth payoffs
    // =============================================================================

    #[test]
    fn test_put_call_parity_smooth() {
        // Put-Call parity: C - P = S - K (for smooth approximations at large moneyness)
        let spot = 100.0;
        let strike = 100.0;
        let eps = 1e-6;

        let call = smooth_call_payoff(spot, strike, eps);
        let put = smooth_put_payoff(spot, strike, eps);

        // For ATM, both should be approximately equal
        assert_relative_eq!(call, put, epsilon = 1e-5);
    }

    #[test]
    fn test_digital_put_call_parity() {
        // Digital call + Digital put ≈ 1
        let spot = 100.0;
        let strike = 100.0;
        let eps = 1.0;

        let dig_call = smooth_digital_call(spot, strike, eps);
        let dig_put = smooth_digital_put(spot, strike, eps);

        assert_relative_eq!(dig_call + dig_put, 1.0, epsilon = 0.01);
    }
}
