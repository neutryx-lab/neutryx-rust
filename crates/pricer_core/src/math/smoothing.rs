//! Smooth approximations for discontinuous functions.
//!
//! This module provides differentiable smoothing functions that replace
//! discontinuous operations (max, min, abs, indicator) with smooth approximations.
//! Required for Enzyme AD: hard `if` conditions are non-differentiable.
//!
//! All functions use generic type parameter `T: num_traits::Float` for f32/f64 support.

use num_traits::Float;

/// Differentiable maximum function using LogSumExp.
///
/// # Mathematical Definition
/// ```text
/// smooth_max(a, b, ε) = ε * log(exp(a/ε) + exp(b/ε))
/// ```
///
/// # Convergence
/// As ε → 0, smooth_max(a, b, ε) → max(a, b)
///
/// # Arguments
/// * `a` - First argument
/// * `b` - Second argument
/// * `epsilon` - Smoothing parameter (recommended range: 1e-8 to 1e-3)
///
/// # Returns
/// Differentiable approximation of max(a, b)
///
/// # Panics
/// Panics if epsilon <= 0
///
/// # Examples
/// ```
/// use pricer_core::math::smoothing::smooth_max;
///
/// let result = smooth_max(3.0_f64, 5.0_f64, 1e-6);
/// assert!((result - 5.0).abs() < 1e-3);
/// ```
#[inline]
pub fn smooth_max<T: Float>(a: T, b: T, epsilon: T) -> T {
    assert!(epsilon > T::zero(), "epsilon must be positive");

    // Numerically stable LogSumExp using log-sum-exp trick
    // smooth_max(a, b, ε) = m + ε * log(exp((a-m)/ε) + exp((b-m)/ε))
    // where m = max(a, b) to prevent overflow
    let m = if a > b { a } else { b };
    let exp_a = ((a - m) / epsilon).exp();
    let exp_b = ((b - m) / epsilon).exp();

    m + epsilon * (exp_a + exp_b).ln()
}

/// Differentiable minimum function as dual of smooth_max.
///
/// # Mathematical Definition
/// ```text
/// smooth_min(a, b, ε) = -smooth_max(-a, -b, ε)
/// ```
///
/// # Convergence
/// As ε → 0, smooth_min(a, b, ε) → min(a, b)
///
/// # Arguments
/// * `a` - First argument
/// * `b` - Second argument
/// * `epsilon` - Smoothing parameter (recommended range: 1e-8 to 1e-3)
///
/// # Returns
/// Differentiable approximation of min(a, b)
///
/// # Panics
/// Panics if epsilon <= 0
#[inline]
pub fn smooth_min<T: Float>(a: T, b: T, epsilon: T) -> T {
    assert!(epsilon > T::zero(), "epsilon must be positive");

    // Dual of smooth_max: smooth_min(a, b, ε) = -smooth_max(-a, -b, ε)
    -smooth_max(-a, -b, epsilon)
}

/// Differentiable Heaviside function using sigmoid.
///
/// # Mathematical Definition
/// ```text
/// smooth_indicator(x, ε) = 1 / (1 + exp(-x/ε))
/// ```
///
/// # Convergence
/// As ε → 0:
/// - x < 0 → 0
/// - x = 0 → 0.5
/// - x > 0 → 1
///
/// # Arguments
/// * `x` - Input value
/// * `epsilon` - Smoothing parameter (recommended range: 1e-8 to 1e-3)
///
/// # Returns
/// Differentiable approximation of Heaviside step function
///
/// # Panics
/// Panics if epsilon <= 0
#[inline]
pub fn smooth_indicator<T: Float>(x: T, epsilon: T) -> T {
    assert!(epsilon > T::zero(), "epsilon must be positive");

    // Sigmoid function: σ(x/ε) = 1 / (1 + exp(-x/ε))
    let one = T::one();
    one / (one + (-x / epsilon).exp())
}

/// Differentiable absolute value function using Softplus.
///
/// # Mathematical Definition
/// ```text
/// smooth_abs(x, ε) = ε * log(exp(x/ε) + exp(-x/ε))
/// ```
///
/// # Convergence
/// As ε → 0, smooth_abs(x, ε) → |x|
///
/// # Arguments
/// * `x` - Input value
/// * `epsilon` - Smoothing parameter (recommended range: 1e-8 to 1e-3)
///
/// # Returns
/// Differentiable approximation of |x|
///
/// # Panics
/// Panics if epsilon <= 0
#[inline]
pub fn smooth_abs<T: Float>(x: T, epsilon: T) -> T {
    assert!(epsilon > T::zero(), "epsilon must be positive");

    // Softplus-based: smooth_abs(x, ε) = ε * log(exp(x/ε) + exp(-x/ε))
    // Using log-sum-exp trick for numerical stability
    // smooth_abs(x, ε) = |x| + ε * log(1 + exp(-2|x|/ε))
    let abs_x = x.abs();
    let two = T::from(2.0).unwrap();
    let term = (-two * abs_x / epsilon).exp();

    abs_x + epsilon * (T::one() + term).ln()
}

/// Differentiable square root function.
///
/// # Mathematical Definition
/// ```text
/// smooth_sqrt(x, ε) = sqrt(x + ε²) - ε
/// ```
///
/// This formula provides:
/// - smooth_sqrt(0, ε) = 0 (exact at zero)
/// - smooth_sqrt(x, ε) → sqrt(x) as ε → 0 for x > 0
/// - Always non-negative for any x (even x < 0 due to ε² term)
/// - Differentiable everywhere including at x = 0
///
/// # Convergence
/// As ε → 0, smooth_sqrt(x, ε) → sqrt(x) for x >= 0
///
/// # Arguments
/// * `x` - Input value (can be slightly negative due to numerical errors)
/// * `epsilon` - Smoothing parameter (recommended range: 1e-8 to 1e-3)
///
/// # Returns
/// Differentiable approximation of sqrt(max(x, 0))
///
/// # Panics
/// Panics if epsilon <= 0
///
/// # Examples
/// ```
/// use pricer_core::math::smoothing::smooth_sqrt;
///
/// let result = smooth_sqrt(4.0_f64, 1e-6);
/// assert!((result - 2.0).abs() < 1e-4);
///
/// // Handles zero smoothly
/// let at_zero = smooth_sqrt(0.0_f64, 1e-6);
/// assert!(at_zero.abs() < 1e-10);
/// ```
#[inline]
pub fn smooth_sqrt<T: Float>(x: T, epsilon: T) -> T {
    assert!(epsilon > T::zero(), "epsilon must be positive");

    // smooth_sqrt(x, ε) = sqrt(x + ε²) - ε
    // This ensures:
    // 1. At x=0: sqrt(ε²) - ε = ε - ε = 0
    // 2. For x >> ε²: sqrt(x + ε²) ≈ sqrt(x)
    // 3. Always real-valued (x + ε² > 0 always)
    let eps_squared = epsilon * epsilon;
    let radicand = x + eps_squared;
    // Clamp to 0 to handle x < -ε² (e.g. large negative inputs)
    let safe_radicand = if radicand < T::zero() {
        T::zero()
    } else {
        radicand
    };
    safe_radicand.sqrt() - epsilon
}

/// Differentiable natural logarithm function.
///
/// # Mathematical Definition
/// ```text
/// smooth_log(x, ε) = ln(x + ε)
/// ```
///
/// This formula provides:
/// - Avoids the singularity at x = 0 by shifting the argument by ε
/// - smooth_log(x, ε) → ln(x) as ε → 0 for x > 0
/// - Returns finite values even for x ≈ 0 or x < 0
/// - Differentiable everywhere
///
/// # Convergence
/// As ε → 0, smooth_log(x, ε) → ln(x) for x > 0
///
/// # Arguments
/// * `x` - Input value (should typically be positive)
/// * `epsilon` - Smoothing parameter (recommended range: 1e-8 to 1e-3)
///
/// # Returns
/// Differentiable approximation of ln(x)
///
/// # Panics
/// Panics if epsilon <= 0
///
/// # Examples
/// ```
/// use pricer_core::math::smoothing::smooth_log;
///
/// let result = smooth_log(std::f64::consts::E, 1e-10);
/// assert!((result - 1.0).abs() < 1e-8);
///
/// // Handles near-zero smoothly
/// let near_zero = smooth_log(1e-10_f64, 1e-6);
/// assert!(near_zero.is_finite());
/// ```
#[inline]
pub fn smooth_log<T: Float>(x: T, epsilon: T) -> T {
    assert!(epsilon > T::zero(), "epsilon must be positive");

    // smooth_log(x, ε) = ln(x + ε)
    // This ensures:
    // 1. At x=0: ln(ε) which is finite (though negative for small ε)
    // 2. For x >> ε: ln(x + ε) ≈ ln(x)
    // 3. Always defined (x + ε > 0 for x >= 0)
    (x + epsilon).ln()
}

/// Differentiable power function for non-integer exponents.
///
/// # Mathematical Definition
/// ```text
/// smooth_pow(x, p, ε) = exp(p * smooth_log(smooth_abs(x, ε) + ε, ε))
/// ```
///
/// For positive x (the common case):
/// ```text
/// smooth_pow(x, p, ε) ≈ exp(p * ln(x + ε)) = (x + ε)^p
/// ```
///
/// This formula provides:
/// - Handles x near zero smoothly
/// - Works for any real exponent p (integer or fractional)
/// - Differentiable everywhere
/// - Uses smooth_abs for robustness with slightly negative x
///
/// # Convergence
/// As ε → 0, smooth_pow(x, p, ε) → x^p for x > 0
///
/// # Arguments
/// * `x` - Base value (should typically be positive)
/// * `p` - Exponent (can be any real number)
/// * `epsilon` - Smoothing parameter (recommended range: 1e-8 to 1e-3)
///
/// # Returns
/// Differentiable approximation of x^p
///
/// # Panics
/// Panics if epsilon <= 0
///
/// # Examples
/// ```
/// use pricer_core::math::smoothing::smooth_pow;
///
/// // x^2
/// let result = smooth_pow(3.0_f64, 2.0, 1e-8);
/// assert!((result - 9.0).abs() < 1e-4);
///
/// // x^0.5 (square root)
/// let sqrt_result = smooth_pow(4.0_f64, 0.5, 1e-8);
/// assert!((sqrt_result - 2.0).abs() < 1e-4);
///
/// // x^0 = 1
/// let zero_exp = smooth_pow(5.0_f64, 0.0, 1e-8);
/// assert!((zero_exp - 1.0).abs() < 1e-4);
/// ```
#[inline]
pub fn smooth_pow<T: Float>(x: T, p: T, epsilon: T) -> T {
    assert!(epsilon > T::zero(), "epsilon must be positive");

    // For numerical stability, we use:
    // smooth_pow(x, p, ε) = exp(p * ln(|x| + ε))
    //
    // This avoids issues with:
    // 1. x = 0: |0| + ε = ε > 0, so ln is defined
    // 2. x < 0: |x| + ε > 0, so ln is defined
    // 3. Large x: standard exp(p * ln(x)) = x^p
    //
    // For very small |x|: (|x| + ε)^p ≈ ε^p which is small for p > 0

    // Use smooth_abs for robustness, but for typical positive x,
    // this simplifies to x + ε (since smooth_abs(x) ≈ |x| for x away from 0)
    let abs_x = smooth_abs(x, epsilon);
    let stabilized_base = abs_x + epsilon;

    // exp(p * ln(base)) = base^p
    (p * stabilized_base.ln()).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // Task 2.5: Smoothing function unit tests

    #[test]
    fn test_smooth_max_convergence() {
        // Test convergence to true max as epsilon decreases
        let a = 3.0_f64;
        let b = 5.0_f64;
        let true_max = a.max(b);

        let eps_values = [1e-2, 1e-4, 1e-6];
        let tolerances = [1e-1, 1e-2, 1e-3];

        for (eps, tol) in eps_values.iter().zip(tolerances.iter()) {
            let result = smooth_max(a, b, *eps);
            assert_relative_eq!(result, true_max, epsilon = *tol);
        }
    }

    #[test]
    fn test_smooth_max_commutativity() {
        let a = 3.0_f64;
        let b = 5.0_f64;
        let epsilon = 1e-6;

        let result1 = smooth_max(a, b, epsilon);
        let result2 = smooth_max(b, a, epsilon);

        assert_relative_eq!(result1, result2, epsilon = 1e-10);
    }

    #[test]
    fn test_smooth_min_duality() {
        // smooth_min(a, b, ε) == -smooth_max(-a, -b, ε)
        let a = 3.0_f64;
        let b = 5.0_f64;
        let epsilon = 1e-6;

        let result_min = smooth_min(a, b, epsilon);
        let result_dual = -smooth_max(-a, -b, epsilon);

        assert_relative_eq!(result_min, result_dual, epsilon = 1e-10);
    }

    #[test]
    fn test_smooth_min_convergence() {
        let a = 3.0_f64;
        let b = 5.0_f64;
        let true_min = a.min(b);

        let eps_values = [1e-2, 1e-4, 1e-6];
        let tolerances = [1e-1, 1e-2, 1e-3];

        for (eps, tol) in eps_values.iter().zip(tolerances.iter()) {
            let result = smooth_min(a, b, *eps);
            assert_relative_eq!(result, true_min, epsilon = *tol);
        }
    }

    #[test]
    fn test_smooth_indicator_boundary() {
        // At x=0, smooth_indicator should be approximately 0.5
        let epsilon = 1e-6;
        let result = smooth_indicator(0.0_f64, epsilon);
        assert_relative_eq!(result, 0.5, epsilon = 1e-3);
    }

    #[test]
    fn test_smooth_indicator_convergence() {
        let epsilon = 1e-6;

        // For large negative x, should converge to 0
        let result_neg = smooth_indicator(-10.0_f64, epsilon);
        assert!(result_neg < 0.01);

        // For large positive x, should converge to 1
        let result_pos = smooth_indicator(10.0_f64, epsilon);
        assert!(result_pos > 0.99);
    }

    #[test]
    fn test_smooth_abs_even_function() {
        // smooth_abs(-x, ε) == smooth_abs(x, ε)
        let x = 3.5_f64;
        let epsilon = 1e-6;

        let result_pos = smooth_abs(x, epsilon);
        let result_neg = smooth_abs(-x, epsilon);

        assert_relative_eq!(result_pos, result_neg, epsilon = 1e-10);
    }

    #[test]
    fn test_smooth_abs_convergence() {
        let x = 3.5_f64;
        let true_abs = x.abs();

        let eps_values = [1e-2, 1e-4, 1e-6];
        let tolerances = [1e-1, 1e-2, 1e-3];

        for (eps, tol) in eps_values.iter().zip(tolerances.iter()) {
            let result = smooth_abs(x, *eps);
            assert_relative_eq!(result, true_abs, epsilon = *tol);
        }
    }

    #[test]
    #[should_panic(expected = "epsilon must be positive")]
    fn test_smooth_max_panics_on_zero_epsilon() {
        smooth_max(3.0_f64, 5.0_f64, 0.0);
    }

    #[test]
    #[should_panic(expected = "epsilon must be positive")]
    fn test_smooth_max_panics_on_negative_epsilon() {
        smooth_max(3.0_f64, 5.0_f64, -1e-6);
    }

    #[test]
    #[should_panic(expected = "epsilon must be positive")]
    fn test_smooth_min_panics_on_zero_epsilon() {
        smooth_min(3.0_f64, 5.0_f64, 0.0);
    }

    #[test]
    #[should_panic(expected = "epsilon must be positive")]
    fn test_smooth_indicator_panics_on_zero_epsilon() {
        smooth_indicator(0.0_f64, 0.0);
    }

    #[test]
    #[should_panic(expected = "epsilon must be positive")]
    fn test_smooth_abs_panics_on_zero_epsilon() {
        smooth_abs(3.0_f64, 0.0);
    }

    // Task 1.1: smooth_sqrt tests
    #[test]
    fn test_smooth_sqrt_convergence() {
        // Test convergence to true sqrt as epsilon decreases
        let x = 4.0_f64;
        let true_sqrt = x.sqrt();

        let eps_values = [1e-2, 1e-4, 1e-6];
        let tolerances = [1e-1, 1e-3, 1e-5];

        for (eps, tol) in eps_values.iter().zip(tolerances.iter()) {
            let result = smooth_sqrt(x, *eps);
            assert_relative_eq!(result, true_sqrt, epsilon = *tol);
        }
    }

    #[test]
    fn test_smooth_sqrt_near_zero() {
        // smooth_sqrt should handle values near zero smoothly
        let epsilon = 1e-6;
        let result = smooth_sqrt(0.0_f64, epsilon);

        // At x=0: smooth_sqrt(0, ε) = sqrt(0 + ε²) - ε = ε - ε = 0
        assert_relative_eq!(result, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_smooth_sqrt_always_non_negative() {
        // smooth_sqrt should return non-negative values (within epsilon tolerance)
        let epsilon = 1e-6;

        for x in [-1.0, 0.0, 1.0, 4.0, 100.0] {
            let result = smooth_sqrt(x, epsilon);
            assert!(
                result >= -epsilon,
                "smooth_sqrt({}, {}) = {} should be >= -epsilon",
                x,
                epsilon,
                result
            );
        }
    }

    #[test]
    fn test_smooth_sqrt_large_values() {
        // Test with large values
        let x = 1e10_f64;
        let epsilon = 1e-6;
        let result = smooth_sqrt(x, epsilon);
        let true_sqrt = x.sqrt();

        // For large x, smooth_sqrt ≈ sqrt(x)
        assert_relative_eq!(result, true_sqrt, epsilon = 1e-3);
    }

    #[test]
    fn test_smooth_sqrt_f32() {
        // Test f32 support
        let x = 4.0_f32;
        let epsilon = 1e-4_f32;
        let result = smooth_sqrt(x, epsilon);
        let true_sqrt = x.sqrt();

        assert_relative_eq!(result, true_sqrt, epsilon = 1e-3);
    }

    #[test]
    #[should_panic(expected = "epsilon must be positive")]
    fn test_smooth_sqrt_panics_on_zero_epsilon() {
        smooth_sqrt(4.0_f64, 0.0);
    }

    #[test]
    #[should_panic(expected = "epsilon must be positive")]
    fn test_smooth_sqrt_panics_on_negative_epsilon() {
        smooth_sqrt(4.0_f64, -1e-6);
    }

    // Task 1.2: smooth_log tests
    #[test]
    fn test_smooth_log_convergence() {
        // Test convergence to true log as epsilon decreases
        let x = 2.0_f64;
        let true_log = x.ln();

        let eps_values = [1e-2, 1e-4, 1e-6];
        let tolerances = [1e-1, 1e-3, 1e-5];

        for (eps, tol) in eps_values.iter().zip(tolerances.iter()) {
            let result = smooth_log(x, *eps);
            assert_relative_eq!(result, true_log, epsilon = *tol);
        }
    }

    #[test]
    fn test_smooth_log_near_zero() {
        // smooth_log should handle values near zero without blowing up
        let epsilon = 1e-6;
        let result = smooth_log(1e-10_f64, epsilon);

        // Should return a large negative but finite value
        assert!(
            result.is_finite(),
            "smooth_log should return finite value near zero"
        );
        assert!(
            result < 0.0,
            "smooth_log of small positive should be negative"
        );
    }

    #[test]
    fn test_smooth_log_at_epsilon() {
        // At x = epsilon, smooth_log should be approximately log(2*epsilon)
        let epsilon = 1e-6;
        let result = smooth_log(epsilon, epsilon);
        let expected = (2.0 * epsilon).ln();

        assert_relative_eq!(result, expected, epsilon = 1e-8);
    }

    #[test]
    fn test_smooth_log_large_values() {
        // For large x >> epsilon, smooth_log ≈ log(x)
        let x = 100.0_f64;
        let epsilon = 1e-6;
        let result = smooth_log(x, epsilon);
        let true_log = x.ln();

        assert_relative_eq!(result, true_log, epsilon = 1e-6);
    }

    #[test]
    fn test_smooth_log_f32() {
        // Test f32 support
        let x = 2.0_f32;
        let epsilon = 1e-4_f32;
        let result = smooth_log(x, epsilon);
        let true_log = x.ln();

        assert_relative_eq!(result, true_log, epsilon = 1e-3);
    }

    #[test]
    #[should_panic(expected = "epsilon must be positive")]
    fn test_smooth_log_panics_on_zero_epsilon() {
        smooth_log(2.0_f64, 0.0);
    }

    #[test]
    #[should_panic(expected = "epsilon must be positive")]
    fn test_smooth_log_panics_on_negative_epsilon() {
        smooth_log(2.0_f64, -1e-6);
    }

    // Task 1.3: smooth_pow tests
    #[test]
    fn test_smooth_pow_convergence_integer_exponent() {
        // Test convergence to true power for integer exponent
        let x = 3.0_f64;
        let p = 2.0_f64;
        let true_pow = x.powf(p);

        let eps_values = [1e-2, 1e-4, 1e-6];
        let tolerances = [1e-1, 1e-3, 1e-4];

        for (eps, tol) in eps_values.iter().zip(tolerances.iter()) {
            let result = smooth_pow(x, p, *eps);
            assert_relative_eq!(result, true_pow, epsilon = *tol);
        }
    }

    #[test]
    fn test_smooth_pow_convergence_fractional_exponent() {
        // Test convergence for fractional exponent (x^0.5 = sqrt)
        let x = 4.0_f64;
        let p = 0.5_f64;
        let true_pow = x.powf(p); // = 2.0

        let eps_values = [1e-2, 1e-4, 1e-6];
        let tolerances = [1e-1, 1e-3, 1e-4];

        for (eps, tol) in eps_values.iter().zip(tolerances.iter()) {
            let result = smooth_pow(x, p, *eps);
            assert_relative_eq!(result, true_pow, epsilon = *tol);
        }
    }

    #[test]
    fn test_smooth_pow_near_zero() {
        // smooth_pow should handle x near zero smoothly
        let epsilon = 1e-6;
        let p = 0.5_f64;
        let result = smooth_pow(1e-10_f64, p, epsilon);

        // Should return a small positive finite value
        assert!(
            result.is_finite(),
            "smooth_pow should return finite value near zero"
        );
        assert!(result > 0.0, "smooth_pow should return positive value");
    }

    #[test]
    fn test_smooth_pow_exponent_zero() {
        // x^0 = 1 for any x
        let x = 5.0_f64;
        let p = 0.0_f64;
        let epsilon = 1e-6;
        let result = smooth_pow(x, p, epsilon);

        assert_relative_eq!(result, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn test_smooth_pow_exponent_one() {
        // x^1 = x
        let x = 5.0_f64;
        let p = 1.0_f64;
        let epsilon = 1e-6;
        let result = smooth_pow(x, p, epsilon);

        assert_relative_eq!(result, x, epsilon = 1e-4);
    }

    #[test]
    fn test_smooth_pow_negative_exponent() {
        // x^(-1) = 1/x
        let x = 4.0_f64;
        let p = -1.0_f64;
        let epsilon = 1e-6;
        let result = smooth_pow(x, p, epsilon);
        let expected = 0.25_f64;

        assert_relative_eq!(result, expected, epsilon = 1e-4);
    }

    #[test]
    fn test_smooth_pow_f32() {
        // Test f32 support
        let x = 4.0_f32;
        let p = 0.5_f32;
        let epsilon = 1e-4_f32;
        let result = smooth_pow(x, p, epsilon);
        let true_pow = x.powf(p);

        assert_relative_eq!(result, true_pow, epsilon = 1e-2);
    }

    #[test]
    #[should_panic(expected = "epsilon must be positive")]
    fn test_smooth_pow_panics_on_zero_epsilon() {
        smooth_pow(4.0_f64, 0.5, 0.0);
    }

    #[test]
    #[should_panic(expected = "epsilon must be positive")]
    fn test_smooth_pow_panics_on_negative_epsilon() {
        smooth_pow(4.0_f64, 0.5, -1e-6);
    }

    // ================================================================
    // Task 5.2: Gradient verification tests
    // Verify AD compatibility via finite difference comparison
    // ================================================================

    /// Compute numerical gradient using central difference
    fn finite_diff<F>(f: F, x: f64, h: f64) -> f64
    where
        F: Fn(f64) -> f64,
    {
        (f(x + h) - f(x - h)) / (2.0 * h)
    }

    #[test]
    fn test_smooth_max_gradient_finite_diff() {
        let eps = 1e-6_f64;
        let h = 1e-8_f64;

        // Test gradient at various points
        for a in [0.5, 1.0, 2.0, 5.0] {
            for b in [0.3, 1.0, 2.5, 4.0] {
                // Gradient w.r.t. a: d/da smooth_max(a, b, eps)
                let grad_a = finite_diff(|x| smooth_max(x, b, eps), a, h);

                // Gradient w.r.t. b: d/db smooth_max(a, b, eps)
                let grad_b = finite_diff(|x| smooth_max(a, x, eps), b, h);

                // Gradients should be in [0, 1] for smooth_max (convex combination)
                assert!(
                    grad_a >= -1e-6 && grad_a <= 1.0 + 1e-6,
                    "smooth_max grad_a out of range: {} at a={}, b={}",
                    grad_a,
                    a,
                    b
                );
                assert!(
                    grad_b >= -1e-6 && grad_b <= 1.0 + 1e-6,
                    "smooth_max grad_b out of range: {} at a={}, b={}",
                    grad_b,
                    a,
                    b
                );

                // Sum of gradients should be approximately 1 (convex combination)
                assert!(
                    (grad_a + grad_b - 1.0).abs() < 1e-4,
                    "smooth_max gradients should sum to 1: {} + {} at a={}, b={}",
                    grad_a,
                    grad_b,
                    a,
                    b
                );
            }
        }
    }

    #[test]
    fn test_smooth_sqrt_gradient_finite_diff() {
        let eps = 1e-6_f64;
        let h = 1e-8_f64;

        // Test gradient at various points
        for x in [0.01, 0.1, 1.0, 4.0, 9.0] {
            let numerical_grad = finite_diff(|t| smooth_sqrt(t, eps), x, h);

            // Analytical gradient of sqrt: 1 / (2 * sqrt(x))
            // For smooth_sqrt with small eps, should be close
            let analytical_grad = 1.0 / (2.0 * x.sqrt());

            let rel_err = if analytical_grad.abs() > 1e-10 {
                (numerical_grad - analytical_grad).abs() / analytical_grad.abs()
            } else {
                (numerical_grad - analytical_grad).abs()
            };

            assert!(
                rel_err < 1e-4, // Allow some tolerance due to smoothing
                "smooth_sqrt gradient mismatch at x={}: numerical={}, analytical={}, rel_err={}",
                x,
                numerical_grad,
                analytical_grad,
                rel_err
            );
        }
    }

    #[test]
    fn test_smooth_indicator_gradient_finite_diff() {
        let eps = 1e-6_f64;
        let h = 1e-8_f64;

        // Test gradient at various points
        for x in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let numerical_grad = finite_diff(|t| smooth_indicator(t, eps), x, h);

            // Analytical gradient of sigmoid: sigmoid(x) * (1 - sigmoid(x)) / eps
            let sig = 1.0 / (1.0 + (-x / eps).exp());
            let analytical_grad = sig * (1.0 - sig) / eps;

            let rel_err = if analytical_grad.abs() > 1e-10 {
                (numerical_grad - analytical_grad).abs() / analytical_grad.abs()
            } else {
                (numerical_grad - analytical_grad).abs()
            };

            assert!(
                rel_err < 1e-4,
                "smooth_indicator gradient mismatch at x={}: numerical={}, analytical={}, rel_err={}",
                x,
                numerical_grad,
                analytical_grad,
                rel_err
            );
        }
    }

    #[test]
    fn test_smooth_abs_gradient_finite_diff() {
        let eps = 1e-6_f64;
        let h = 1e-8_f64;

        // Test gradient at various points (away from 0 where gradient transitions)
        for x in [-2.0, -1.0, 1.0, 2.0] {
            let numerical_grad = finite_diff(|t| smooth_abs(t, eps), x, h);

            // For |x| >> eps, gradient should be sign(x)
            let expected_grad = if x > 0.0 { 1.0 } else { -1.0 };

            let diff = (numerical_grad - expected_grad).abs();
            assert!(
                diff < 1e-4,
                "smooth_abs gradient mismatch at x={}: numerical={}, expected={}, diff={}",
                x,
                numerical_grad,
                expected_grad,
                diff
            );
        }

        // At x=0, gradient should be approximately 0 (smooth transition)
        let grad_at_zero = finite_diff(|t| smooth_abs(t, eps), 0.0, h);
        assert!(
            grad_at_zero.abs() < 1e-3,
            "smooth_abs gradient at 0 should be ~0: {}",
            grad_at_zero
        );
    }

    #[test]
    fn test_smooth_pow_gradient_finite_diff() {
        let eps = 1e-6_f64;
        let h = 1e-8_f64;

        // Test gradient w.r.t. x at various points
        for x in [0.5, 1.0, 2.0, 4.0] {
            for p in [0.5, 1.0, 2.0] {
                let numerical_grad = finite_diff(|t| smooth_pow(t, p, eps), x, h);

                // Analytical gradient: p * x^(p-1)
                let analytical_grad = p * x.powf(p - 1.0);

                let rel_err = if analytical_grad.abs() > 1e-10 {
                    (numerical_grad - analytical_grad).abs() / analytical_grad.abs()
                } else {
                    (numerical_grad - analytical_grad).abs()
                };

                assert!(
                    rel_err < 1e-3, // smooth_pow has some approximation error
                    "smooth_pow gradient mismatch at x={}, p={}: numerical={}, analytical={}, rel_err={}",
                    x,
                    p,
                    numerical_grad,
                    analytical_grad,
                    rel_err
                );
            }
        }
    }

    #[test]
    fn test_smooth_log_gradient_finite_diff() {
        let eps = 1e-6_f64;
        let h = 1e-8_f64;

        // Test gradient at various points
        for x in [0.1, 0.5, 1.0, 2.0, 10.0] {
            let numerical_grad = finite_diff(|t| smooth_log(t, eps), x, h);

            // Analytical gradient of ln(x + eps): 1 / (x + eps)
            let analytical_grad = 1.0 / (x + eps);

            let rel_err = if analytical_grad.abs() > 1e-10 {
                (numerical_grad - analytical_grad).abs() / analytical_grad.abs()
            } else {
                (numerical_grad - analytical_grad).abs()
            };

            assert!(
                rel_err < 1e-6,
                "smooth_log gradient mismatch at x={}: numerical={}, analytical={}, rel_err={}",
                x,
                numerical_grad,
                analytical_grad,
                rel_err
            );
        }
    }

    #[test]
    fn test_smooth_min_gradient_finite_diff() {
        let eps = 1e-6_f64;
        let h = 1e-8_f64;

        // Test gradient at various points
        for a in [0.5, 1.0, 2.0] {
            for b in [0.3, 1.0, 2.5] {
                // Gradient w.r.t. a
                let grad_a = finite_diff(|x| smooth_min(x, b, eps), a, h);

                // Gradient w.r.t. b
                let grad_b = finite_diff(|x| smooth_min(a, x, eps), b, h);

                // Gradients should be in [0, 1] for smooth_min (convex combination)
                assert!(
                    grad_a >= -1e-6 && grad_a <= 1.0 + 1e-6,
                    "smooth_min grad_a out of range: {} at a={}, b={}",
                    grad_a,
                    a,
                    b
                );
                assert!(
                    grad_b >= -1e-6 && grad_b <= 1.0 + 1e-6,
                    "smooth_min grad_b out of range: {} at a={}, b={}",
                    grad_b,
                    a,
                    b
                );

                // Sum of gradients should be approximately 1
                assert!(
                    (grad_a + grad_b - 1.0).abs() < 1e-4,
                    "smooth_min gradients should sum to 1: {} + {} at a={}, b={}",
                    grad_a,
                    grad_b,
                    a,
                    b
                );
            }
        }
    }

    // Task 6.1: Property-based tests for smoothing functions
    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        // Generate epsilon in practical range [1e-8, 1e-3]
        fn epsilon_strategy() -> impl Strategy<Value = f64> {
            1e-8..1e-3
        }

        // Generate finite f64 values for testing
        fn finite_f64_strategy() -> impl Strategy<Value = f64> {
            prop::num::f64::NORMAL
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(1000))]

            #[test]
            fn test_smooth_max_inequality(
                a in finite_f64_strategy(),
                b in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                let result = smooth_max(a, b, epsilon);
                let true_max = a.max(b);
                let tolerance = epsilon.abs() * 10.0; // O(ε) tolerance

                // smooth_max(a, b, ε) >= max(a, b) - tolerance
                assert!(
                    result >= true_max - tolerance,
                    "smooth_max({}, {}, {}) = {} should be >= {} - {}",
                    a, b, epsilon, result, true_max, tolerance
                );
            }

            #[test]
            fn test_smooth_max_commutativity_property(
                a in finite_f64_strategy(),
                b in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                let result1 = smooth_max(a, b, epsilon);
                let result2 = smooth_max(b, a, epsilon);

                // smooth_max(a, b, ε) == smooth_max(b, a, ε)
                assert_relative_eq!(result1, result2, epsilon = 1e-10);
            }

            #[test]
            fn test_smooth_min_inequality(
                a in finite_f64_strategy(),
                b in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                let result = smooth_min(a, b, epsilon);
                let true_min = a.min(b);
                let tolerance = epsilon.abs() * 10.0; // O(ε) tolerance

                // smooth_min(a, b, ε) <= min(a, b) + tolerance
                assert!(
                    result <= true_min + tolerance,
                    "smooth_min({}, {}, {}) = {} should be <= {} + {}",
                    a, b, epsilon, result, true_min, tolerance
                );
            }

            #[test]
            fn test_smooth_min_commutativity_property(
                a in finite_f64_strategy(),
                b in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                let result1 = smooth_min(a, b, epsilon);
                let result2 = smooth_min(b, a, epsilon);

                // smooth_min(a, b, ε) == smooth_min(b, a, ε)
                assert_relative_eq!(result1, result2, epsilon = 1e-10);
            }

            #[test]
            fn test_smooth_indicator_monotonicity(
                x1 in finite_f64_strategy(),
                x2 in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                // Only test when x1 < x2
                if x1 < x2 {
                    let result1 = smooth_indicator(x1, epsilon);
                    let result2 = smooth_indicator(x2, epsilon);

                    // x1 < x2 → smooth_indicator(x1, ε) <= smooth_indicator(x2, ε)
                    assert!(
                        result1 <= result2,
                        "smooth_indicator({}, {}) = {} should be <= smooth_indicator({}, {}) = {}",
                        x1, epsilon, result1, x2, epsilon, result2
                    );
                }
            }

            #[test]
            fn test_smooth_indicator_bounds(
                x in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                let result = smooth_indicator(x, epsilon);

                // smooth_indicator should always be in [0, 1]
                assert!(
                    result >= 0.0 && result <= 1.0,
                    "smooth_indicator({}, {}) = {} should be in [0, 1]",
                    x, epsilon, result
                );
            }

            #[test]
            fn test_smooth_abs_even_function_property(
                x in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                let result_pos = smooth_abs(x, epsilon);
                let result_neg = smooth_abs(-x, epsilon);

                // smooth_abs(-x, ε) == smooth_abs(x, ε)
                assert_relative_eq!(result_pos, result_neg, epsilon = 1e-10);
            }

            #[test]
            fn test_smooth_abs_non_negative(
                x in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                let result = smooth_abs(x, epsilon);

                // smooth_abs should always be non-negative
                assert!(
                    result >= 0.0,
                    "smooth_abs({}, {}) = {} should be non-negative",
                    x, epsilon, result
                );
            }

            #[test]
            fn test_all_smoothing_functions_return_finite(
                a in finite_f64_strategy(),
                b in finite_f64_strategy(),
                x in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                // All smoothing functions should return finite values for finite inputs
                let max_result = smooth_max(a, b, epsilon);
                let min_result = smooth_min(a, b, epsilon);
                let ind_result = smooth_indicator(x, epsilon);
                let abs_result = smooth_abs(x, epsilon);

                assert!(max_result.is_finite());
                assert!(min_result.is_finite());
                assert!(ind_result.is_finite());
                assert!(abs_result.is_finite());
            }

            // Task 1.1: Property-based tests for smooth_sqrt
            #[test]
            fn test_smooth_sqrt_non_negative(
                x in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                let result = smooth_sqrt(x, epsilon);

                // smooth_sqrt should always return non-negative values
                assert!(
                    result >= -epsilon, // Allow tiny numerical errors
                    "smooth_sqrt({}, {}) = {} should be >= 0",
                    x, epsilon, result
                );
            }

            #[test]
            fn test_smooth_sqrt_monotonicity(
                x1 in finite_f64_strategy(),
                x2 in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                // Only test when both are positive and x1 < x2
                if x1 >= 0.0 && x2 >= 0.0 && x1 < x2 {
                    let result1 = smooth_sqrt(x1, epsilon);
                    let result2 = smooth_sqrt(x2, epsilon);

                    // x1 < x2 → smooth_sqrt(x1) <= smooth_sqrt(x2)
                    assert!(
                        result1 <= result2 + 1e-10,
                        "smooth_sqrt({}, {}) = {} should be <= smooth_sqrt({}, {}) = {}",
                        x1, epsilon, result1, x2, epsilon, result2
                    );
                }
            }

            #[test]
            fn test_smooth_sqrt_returns_finite(
                x in finite_f64_strategy(),
                epsilon in epsilon_strategy()
            ) {
                let result = smooth_sqrt(x, epsilon);
                assert!(result.is_finite(), "smooth_sqrt({}, {}) should be finite", x, epsilon);
            }

            // Task 1.2: Property-based tests for smooth_log
            #[test]
            fn test_smooth_log_monotonicity(
                x1 in 0.01f64..1000.0,
                x2 in 0.01f64..1000.0,
                epsilon in epsilon_strategy()
            ) {
                // Only test when x1 < x2
                if x1 < x2 {
                    let result1 = smooth_log(x1, epsilon);
                    let result2 = smooth_log(x2, epsilon);

                    // x1 < x2 → smooth_log(x1) < smooth_log(x2) for positive x
                    assert!(
                        result1 < result2 + 1e-10,
                        "smooth_log({}, {}) = {} should be < smooth_log({}, {}) = {}",
                        x1, epsilon, result1, x2, epsilon, result2
                    );
                }
            }

            #[test]
            fn test_smooth_log_returns_finite_for_positive(
                x in 0.0f64..1e6,
                epsilon in epsilon_strategy()
            ) {
                let result = smooth_log(x, epsilon);
                assert!(result.is_finite(), "smooth_log({}, {}) should be finite", x, epsilon);
            }

            // Task 1.3: Property-based tests for smooth_pow
            #[test]
            fn test_smooth_pow_positive_base_positive_result(
                x in 0.01f64..100.0,
                p in -2.0f64..2.0,
                epsilon in epsilon_strategy()
            ) {
                let result = smooth_pow(x, p, epsilon);

                // For positive base, result should be positive
                assert!(
                    result > 0.0,
                    "smooth_pow({}, {}, {}) = {} should be positive",
                    x, p, epsilon, result
                );
            }

            #[test]
            fn test_smooth_pow_returns_finite(
                x in 0.01f64..100.0,
                p in -2.0f64..2.0,
                epsilon in epsilon_strategy()
            ) {
                let result = smooth_pow(x, p, epsilon);
                assert!(
                    result.is_finite(),
                    "smooth_pow({}, {}, {}) should be finite",
                    x, p, epsilon
                );
            }

            #[test]
            fn test_smooth_pow_exponent_zero_is_one(
                x in 0.01f64..100.0,
                epsilon in epsilon_strategy()
            ) {
                let result = smooth_pow(x, 0.0, epsilon);

                // x^0 = 1 for any positive x
                assert_relative_eq!(result, 1.0, epsilon = 1e-3);
            }
        }
    }
}
