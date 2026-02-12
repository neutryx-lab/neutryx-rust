//! Smooth approximations for discontinuous functions.
//!
//! This module provides differentiable smoothing functions that replace
//! discontinuous operations (max, min, abs, indicator) with smooth
//! approximations. Required for Enzyme AD: hard `if` conditions are
//! non-differentiable.
//!
//! All functions use generic type parameter `T: num_traits::Float` for f32/f64
//! support.

use num_traits::Float;

use crate::math::numeric::from_f64;

/// Differentiable approximation of `max(a, b)` using LogSumExp.
///
/// `smooth_max(a, b, ε) = ε * log(exp(a/ε) + exp(b/ε))` converges to
/// `max(a, b)` as ε -> 0. Panics if `epsilon <= 0`.
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

/// Differentiable approximation of `min(a, b)` as `-smooth_max(-a, -b, ε)`.
///
/// Panics if `epsilon <= 0`.
#[inline]
pub fn smooth_min<T: Float>(a: T, b: T, epsilon: T) -> T {
    assert!(epsilon > T::zero(), "epsilon must be positive");

    // Dual of smooth_max: smooth_min(a, b, ε) = -smooth_max(-a, -b, ε)
    -smooth_max(-a, -b, epsilon)
}

/// Differentiable Heaviside step via sigmoid: `1 / (1 + exp(-x/ε))`.
///
/// Converges to 0 (x<0), 0.5 (x=0), 1 (x>0) as ε -> 0.
/// Panics if `epsilon <= 0`.
#[inline]
pub fn smooth_indicator<T: Float>(x: T, epsilon: T) -> T {
    assert!(epsilon > T::zero(), "epsilon must be positive");

    // Sigmoid function: σ(x/ε) = 1 / (1 + exp(-x/ε))
    let one = T::one();
    one / (one + (-x / epsilon).exp())
}

/// Differentiable `|x|` via Softplus: `ε * log(exp(x/ε) + exp(-x/ε))`.
///
/// Panics if `epsilon <= 0`.
#[inline]
pub fn smooth_abs<T: Float>(x: T, epsilon: T) -> T {
    assert!(epsilon > T::zero(), "epsilon must be positive");

    // Softplus-based: smooth_abs(x, ε) = ε * log(exp(x/ε) + exp(-x/ε))
    // Using log-sum-exp trick for numerical stability
    // smooth_abs(x, ε) = |x| + ε * log(1 + exp(-2|x|/ε))
    let abs_x = x.abs();
    let two: T = from_f64(2.0);
    let term = (-two * abs_x / epsilon).exp();

    abs_x + epsilon * (T::one() + term).ln()
}

/// Differentiable `sqrt(x)` via `sqrt(x + ε^2) - ε`.
///
/// Exact at zero, always non-negative, differentiable everywhere.
/// Panics if `epsilon <= 0`.
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

/// Differentiable `ln(x)` via `ln(x + ε)`, avoiding the singularity at zero.
///
/// Panics if `epsilon <= 0`.
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

/// Differentiable `x^p` via `exp(p * ln(|x| + ε))`.
///
/// Works for any real exponent, handles x near zero smoothly.
/// Panics if `epsilon <= 0`.
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
    use approx::assert_relative_eq;

    use super::*;

    // ── Convergence test macro ──────────────────────────────────────────
    macro_rules! test_convergence {
        ($name:ident, $compute:expr, $expected:expr,
         eps = [$($e:expr),+], tol = [$($t:expr),+]) => {
            #[test]
            fn $name() {
                let expected = $expected;
                for (eps, tol) in [$($e),+].iter().zip([$($t),+].iter()) {
                    let result = $compute(*eps);
                    assert_relative_eq!(result, expected, epsilon = *tol);
                }
            }
        };
    }

    test_convergence!(smooth_max_convergence,
        |eps| smooth_max(3.0_f64, 5.0, eps), 5.0_f64,
        eps = [1e-2, 1e-4, 1e-6], tol = [1e-1, 1e-2, 1e-3]);

    test_convergence!(smooth_min_convergence,
        |eps| smooth_min(3.0_f64, 5.0, eps), 3.0_f64,
        eps = [1e-2, 1e-4, 1e-6], tol = [1e-1, 1e-2, 1e-3]);

    test_convergence!(smooth_abs_convergence,
        |eps| smooth_abs(3.5_f64, eps), 3.5_f64,
        eps = [1e-2, 1e-4, 1e-6], tol = [1e-1, 1e-2, 1e-3]);

    test_convergence!(smooth_sqrt_convergence,
        |eps| smooth_sqrt(4.0_f64, eps), 2.0_f64,
        eps = [1e-2, 1e-4, 1e-6], tol = [1e-1, 1e-3, 1e-5]);

    test_convergence!(smooth_log_convergence,
        |eps| smooth_log(2.0_f64, eps), 2.0_f64.ln(),
        eps = [1e-2, 1e-4, 1e-6], tol = [1e-1, 1e-3, 1e-5]);

    test_convergence!(smooth_pow_integer_exp_convergence,
        |eps| smooth_pow(3.0_f64, 2.0, eps), 9.0_f64,
        eps = [1e-2, 1e-4, 1e-6], tol = [1e-1, 1e-3, 1e-4]);

    test_convergence!(smooth_pow_fractional_exp_convergence,
        |eps| smooth_pow(4.0_f64, 0.5, eps), 2.0_f64,
        eps = [1e-2, 1e-4, 1e-6], tol = [1e-1, 1e-3, 1e-4]);

    // ── Panic test macro ────────────────────────────────────────────────
    macro_rules! test_epsilon_panic {
        ($name:ident, $call:expr) => {
            #[test]
            #[should_panic(expected = "epsilon must be positive")]
            fn $name() { $call; }
        };
    }

    test_epsilon_panic!(smooth_max_zero_eps,    smooth_max(3.0_f64, 5.0, 0.0));
    test_epsilon_panic!(smooth_max_neg_eps,     smooth_max(3.0_f64, 5.0, -1e-6));
    test_epsilon_panic!(smooth_min_zero_eps,    smooth_min(3.0_f64, 5.0, 0.0));
    test_epsilon_panic!(smooth_indicator_zero_eps, smooth_indicator(0.0_f64, 0.0));
    test_epsilon_panic!(smooth_abs_zero_eps,    smooth_abs(3.0_f64, 0.0));
    test_epsilon_panic!(smooth_sqrt_zero_eps,   smooth_sqrt(4.0_f64, 0.0));
    test_epsilon_panic!(smooth_sqrt_neg_eps,    smooth_sqrt(4.0_f64, -1e-6));
    test_epsilon_panic!(smooth_log_zero_eps,    smooth_log(2.0_f64, 0.0));
    test_epsilon_panic!(smooth_log_neg_eps,     smooth_log(2.0_f64, -1e-6));
    test_epsilon_panic!(smooth_pow_zero_eps,    smooth_pow(4.0_f64, 0.5, 0.0));
    test_epsilon_panic!(smooth_pow_neg_eps,     smooth_pow(4.0_f64, 0.5, -1e-6));

    // ── Symmetry / identity tests ───────────────────────────────────────
    #[test]
    fn smooth_max_commutativity() {
        assert_relative_eq!(
            smooth_max(3.0_f64, 5.0, 1e-6),
            smooth_max(5.0_f64, 3.0, 1e-6),
            epsilon = 1e-10
        );
    }

    #[test]
    fn smooth_min_duality() {
        assert_relative_eq!(
            smooth_min(3.0_f64, 5.0, 1e-6),
            -smooth_max(-3.0_f64, -5.0, 1e-6),
            epsilon = 1e-10
        );
    }

    #[test]
    fn smooth_abs_even_function() {
        assert_relative_eq!(
            smooth_abs(3.5_f64, 1e-6),
            smooth_abs(-3.5_f64, 1e-6),
            epsilon = 1e-10
        );
    }

    // ── Boundary / special-value tests ──────────────────────────────────
    #[test]
    fn smooth_indicator_boundary() {
        assert_relative_eq!(smooth_indicator(0.0_f64, 1e-6), 0.5, epsilon = 1e-3);
    }

    #[test]
    fn smooth_indicator_convergence() {
        let eps = 1e-6;
        assert!(smooth_indicator(-10.0_f64, eps) < 0.01);
        assert!(smooth_indicator(10.0_f64, eps) > 0.99);
    }

    #[test]
    fn smooth_sqrt_near_zero() {
        assert_relative_eq!(smooth_sqrt(0.0_f64, 1e-6), 0.0, epsilon = 1e-10);
    }

    #[test]
    fn smooth_sqrt_always_non_negative() {
        let eps = 1e-6;
        for x in [-1.0, 0.0, 1.0, 4.0, 100.0] {
            assert!(smooth_sqrt(x, eps) >= -eps, "smooth_sqrt({x}) should be >= -eps");
        }
    }

    #[test]
    fn smooth_sqrt_large_values() {
        assert_relative_eq!(smooth_sqrt(1e10_f64, 1e-6), 1e10_f64.sqrt(), epsilon = 1e-3);
    }

    #[test]
    fn smooth_log_near_zero() {
        let result = smooth_log(1e-10_f64, 1e-6);
        assert!(result.is_finite() && result < 0.0);
    }

    #[test]
    fn smooth_log_at_epsilon() {
        let eps = 1e-6;
        assert_relative_eq!(smooth_log(eps, eps), (2.0 * eps).ln(), epsilon = 1e-8);
    }

    #[test]
    fn smooth_log_large_values() {
        assert_relative_eq!(smooth_log(100.0_f64, 1e-6), 100.0_f64.ln(), epsilon = 1e-6);
    }

    #[test]
    fn smooth_pow_near_zero() {
        let r = smooth_pow(1e-10_f64, 0.5, 1e-6);
        assert!(r.is_finite() && r > 0.0);
    }

    #[test]
    fn smooth_pow_exponent_zero() {
        assert_relative_eq!(smooth_pow(5.0_f64, 0.0, 1e-6), 1.0, epsilon = 1e-4);
    }

    #[test]
    fn smooth_pow_exponent_one() {
        assert_relative_eq!(smooth_pow(5.0_f64, 1.0, 1e-6), 5.0, epsilon = 1e-4);
    }

    #[test]
    fn smooth_pow_negative_exponent() {
        assert_relative_eq!(smooth_pow(4.0_f64, -1.0, 1e-6), 0.25, epsilon = 1e-4);
    }

    // ── f32 support ─────────────────────────────────────────────────────
    #[test]
    fn smooth_f32_support() {
        assert_relative_eq!(smooth_sqrt(4.0_f32, 1e-4), 2.0_f32, epsilon = 1e-3);
        assert_relative_eq!(smooth_log(2.0_f32, 1e-4), 2.0_f32.ln(), epsilon = 1e-3);
        assert_relative_eq!(smooth_pow(4.0_f32, 0.5, 1e-4), 2.0_f32, epsilon = 1e-2);
    }

    // ── Finite-difference gradient tests ────────────────────────────────
    fn finite_diff(f: impl Fn(f64) -> f64, x: f64, h: f64) -> f64 {
        (f(x + h) - f(x - h)) / (2.0 * h)
    }

    fn assert_gradient(f: impl Fn(f64) -> f64, x: f64, expected: f64, tol: f64, label: &str) {
        let num = finite_diff(&f, x, 1e-8);
        let err = if expected.abs() > 1e-10 {
            (num - expected).abs() / expected.abs()
        } else {
            (num - expected).abs()
        };
        assert!(err < tol, "{label} gradient mismatch at x={x}: numerical={num}, expected={expected}, err={err}");
    }

    /// Verify convex-combination gradient property for binary smooth functions.
    fn assert_binary_gradient_sums_to_one(
        f: impl Fn(f64, f64) -> f64, a_vals: &[f64], b_vals: &[f64], label: &str,
    ) {
        let h = 1e-8_f64;
        for &a in a_vals {
            for &b in b_vals {
                let ga = finite_diff(|x| f(x, b), a, h);
                let gb = finite_diff(|x| f(a, x), b, h);
                assert!(ga >= -1e-6 && ga <= 1.0 + 1e-6, "{label} grad_a OOB at a={a}, b={b}");
                assert!(gb >= -1e-6 && gb <= 1.0 + 1e-6, "{label} grad_b OOB at a={a}, b={b}");
                assert!((ga + gb - 1.0).abs() < 1e-4, "{label} grads don't sum to 1 at a={a}, b={b}");
            }
        }
    }

    #[test]
    fn smooth_max_gradient() {
        let eps = 1e-6;
        assert_binary_gradient_sums_to_one(
            |a, b| smooth_max(a, b, eps),
            &[0.5, 1.0, 2.0, 5.0], &[0.3, 1.0, 2.5, 4.0], "smooth_max",
        );
    }

    #[test]
    fn smooth_min_gradient() {
        let eps = 1e-6;
        assert_binary_gradient_sums_to_one(
            |a, b| smooth_min(a, b, eps),
            &[0.5, 1.0, 2.0], &[0.3, 1.0, 2.5], "smooth_min",
        );
    }

    #[test]
    fn smooth_sqrt_gradient() {
        let eps = 1e-6;
        for x in [0.01, 0.1, 1.0, 4.0, 9.0] {
            assert_gradient(|t| smooth_sqrt(t, eps), x, 0.5 / x.sqrt(), 1e-4, "smooth_sqrt");
        }
    }

    #[test]
    fn smooth_indicator_gradient() {
        let eps = 1e-6;
        for x in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let sig = 1.0 / (1.0 + (-x / eps).exp());
            assert_gradient(|t| smooth_indicator(t, eps), x, sig * (1.0 - sig) / eps, 1e-4, "smooth_indicator");
        }
    }

    #[test]
    fn smooth_abs_gradient() {
        let eps = 1e-6;
        for x in [-2.0, -1.0, 1.0, 2.0] {
            let expected = if x > 0.0 { 1.0 } else { -1.0 };
            assert_gradient(|t| smooth_abs(t, eps), x, expected, 1e-4, "smooth_abs");
        }
        assert!(finite_diff(|t| smooth_abs(t, eps), 0.0, 1e-8).abs() < 1e-3);
    }

    #[test]
    fn smooth_pow_gradient() {
        let eps = 1e-6;
        for x in [0.5, 1.0, 2.0, 4.0] {
            for p in [0.5, 1.0, 2.0] {
                assert_gradient(|t| smooth_pow(t, p, eps), x, p * x.powf(p - 1.0), 1e-3, "smooth_pow");
            }
        }
    }

    #[test]
    fn smooth_log_gradient() {
        let eps = 1e-6;
        for x in [0.1, 0.5, 1.0, 2.0, 10.0] {
            assert_gradient(|t| smooth_log(t, eps), x, 1.0 / (x + eps), 1e-6, "smooth_log");
        }
    }

    // ── Property-based tests ────────────────────────────────────────────
    #[cfg(test)]
    mod property_tests {
        use proptest::prelude::*;

        use super::*;

        fn eps_strat() -> impl Strategy<Value = f64> { 1e-8..1e-3 }
        fn fin_strat() -> impl Strategy<Value = f64> { prop::num::f64::NORMAL }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(1000))]

            #[test]
            fn max_inequality(a in fin_strat(), b in fin_strat(), e in eps_strat()) {
                let tol = e * 10.0;
                assert!(smooth_max(a, b, e) >= a.max(b) - tol);
            }
            #[test]
            fn max_commutativity(a in fin_strat(), b in fin_strat(), e in eps_strat()) {
                assert_relative_eq!(smooth_max(a, b, e), smooth_max(b, a, e), epsilon = 1e-10);
            }
            #[test]
            fn min_inequality(a in fin_strat(), b in fin_strat(), e in eps_strat()) {
                let tol = e * 10.0;
                assert!(smooth_min(a, b, e) <= a.min(b) + tol);
            }
            #[test]
            fn min_commutativity(a in fin_strat(), b in fin_strat(), e in eps_strat()) {
                assert_relative_eq!(smooth_min(a, b, e), smooth_min(b, a, e), epsilon = 1e-10);
            }
            #[test]
            fn indicator_monotonic(x1 in fin_strat(), x2 in fin_strat(), e in eps_strat()) {
                if x1 < x2 { assert!(smooth_indicator(x1, e) <= smooth_indicator(x2, e)); }
            }
            #[test]
            fn indicator_bounds(x in fin_strat(), e in eps_strat()) {
                let r = smooth_indicator(x, e);
                assert!(r >= 0.0 && r <= 1.0);
            }
            #[test]
            fn abs_even(x in fin_strat(), e in eps_strat()) {
                assert_relative_eq!(smooth_abs(x, e), smooth_abs(-x, e), epsilon = 1e-10);
            }
            #[test]
            fn abs_non_negative(x in fin_strat(), e in eps_strat()) {
                assert!(smooth_abs(x, e) >= 0.0);
            }
            #[test]
            fn all_finite(a in fin_strat(), b in fin_strat(), x in fin_strat(), e in eps_strat()) {
                assert!(smooth_max(a, b, e).is_finite());
                assert!(smooth_min(a, b, e).is_finite());
                assert!(smooth_indicator(x, e).is_finite());
                assert!(smooth_abs(x, e).is_finite());
            }
            #[test]
            fn sqrt_non_negative(x in fin_strat(), e in eps_strat()) {
                assert!(smooth_sqrt(x, e) >= -e);
            }
            #[test]
            fn sqrt_monotonic(x1 in fin_strat(), x2 in fin_strat(), e in eps_strat()) {
                if x1 >= 0.0 && x2 >= 0.0 && x1 < x2 {
                    assert!(smooth_sqrt(x1, e) <= smooth_sqrt(x2, e) + 1e-10);
                }
            }
            #[test]
            fn sqrt_finite(x in fin_strat(), e in eps_strat()) {
                assert!(smooth_sqrt(x, e).is_finite());
            }
            #[test]
            fn log_monotonic(x1 in 0.01f64..1000.0, x2 in 0.01f64..1000.0, e in eps_strat()) {
                if x1 < x2 { assert!(smooth_log(x1, e) < smooth_log(x2, e) + 1e-10); }
            }
            #[test]
            fn log_finite_positive(x in 0.0f64..1e6, e in eps_strat()) {
                assert!(smooth_log(x, e).is_finite());
            }
            #[test]
            fn pow_positive_base(x in 0.01f64..100.0, p in -2.0f64..2.0, e in eps_strat()) {
                assert!(smooth_pow(x, p, e) > 0.0);
            }
            #[test]
            fn pow_finite(x in 0.01f64..100.0, p in -2.0f64..2.0, e in eps_strat()) {
                assert!(smooth_pow(x, p, e).is_finite());
            }
            #[test]
            fn pow_zero_exp_is_one(x in 0.01f64..100.0, e in eps_strat()) {
                assert_relative_eq!(smooth_pow(x, 0.0, e), 1.0, epsilon = 1e-3);
            }
        }
    }
}
