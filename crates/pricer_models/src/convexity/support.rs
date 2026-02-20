//! Support functions for convexity adjustment computation.
//!
//! Provides numeraire ratio calculations, derivatives, and numerical
//! integration utilities for CMS convexity adjustments.

use pricer_core::math::numeric::from_f64;
use pricer_core::traits::Float;

// ─── Gauss-Kronrod G7-K15 nodes and weights ─────────────────────

/// Gauss-Kronrod G7-K15 quadrature nodes (positive half, symmetric).
const GK_NODES: [f64; 8] = [
    0.0,
    0.207784955007898,
    0.405845151377397,
    0.586087235467691,
    0.741531185599394,
    0.864864423359769,
    0.949107912342759,
    0.991455371120813,
];

/// Kronrod weights for the 15-point rule.
const K15_WEIGHTS: [f64; 8] = [
    0.209482141084728,
    0.204432940075298,
    0.190350578064785,
    0.169004726639268,
    0.140653259715525,
    0.104790010322250,
    0.063092092629979,
    0.022935322010529,
];

/// Gauss weights for the 7-point rule (zero for Kronrod-only nodes).
const G7_WEIGHTS: [f64; 8] = [
    0.417959183673469,
    0.0,
    0.381830050505119,
    0.0,
    0.279705391489277,
    0.0,
    0.129484966168870,
    0.0,
];

/// Gauss-Kronrod G7-K15 adaptive quadrature over `[a, b]`.
///
/// Returns the Kronrod estimate of the integral.
pub fn gauss_kronrod_integrate<T: Float>(f: &dyn Fn(T) -> T, a: T, b: T) -> T {
    let half = from_f64::<T>(0.5);
    let mid = half * (a + b);
    let half_len = half * (b - a);

    let mut result_k15 = T::zero();

    for i in 0..8 {
        let node: T = from_f64(GK_NODES[i]);
        let wk: T = from_f64(K15_WEIGHTS[i]);

        if i == 0 {
            result_k15 = result_k15 + wk * f(mid);
        } else {
            let x_plus = mid + half_len * node;
            let x_minus = mid - half_len * node;
            result_k15 = result_k15 + wk * (f(x_plus) + f(x_minus));
        }
    }

    result_k15 * half_len
}

// ─── Numeraire ratio ────────────────────────────────────

/// Computes the numeraire ratio G(K).
///
/// ```text
/// G(K) = K / (1 + K * dcf)^delta / (1 - 1/(1 + K * dcf)^payments)
/// ```
///
/// Uses Taylor expansion when `(dcf * K)^2 < 10 * epsilon` to avoid
/// numerical instability near K = 0.
pub fn numeraire_ratio<T: Float>(strike: T, delta: T, freq: T, tenor: T) -> T {
    let eps = T::epsilon();
    let m: T = from_f64(10.0);
    let one = T::one();
    let two: T = from_f64(2.0);
    let three: T = from_f64(3.0);
    let half: T = from_f64(0.5);

    let dcf = one / freq;
    let payments = freq * tenor;

    let threshold = dcf * strike * dcf * strike;

    if threshold.abs() > eps * m {
        let tmp_a = strike * dcf;
        let tmp_b = one + tmp_a;
        let tmp_c = tmp_b.powf(delta);
        let tmp_d = one - one / tmp_b.powf(payments);

        strike / tmp_c / tmp_d
    } else {
        // Taylor expansion near strike = 0
        let p1 = (delta - payments) * dcf;
        let p2 = p1 * (delta - payments - one) * dcf;

        let q1 = payments * dcf;
        let q2 = q1 * (payments - one) * dcf;
        let q3 = q2 * (payments - two) * dcf;

        let g1 = q1;
        let g2 = q2 + two * p1 * q1;
        let g3 = q3 + three * p1 * q2 + three * p2 * q1;

        let f0 = one / g1;
        let f1 = -g2 / two / g1 / g1;
        let f2 = g2 * g2 / two / g1 / g1 / g1 - g3 / three / g1 / g1;

        f0 + f1 * strike + half * f2 * strike * strike
    }
}

/// First derivative of the numeraire ratio w.r.t. strike: G'(K).
///
/// Uses Taylor expansion near K = 0 for numerical stability.
pub fn numeraire_ratio_derivative<T: Float>(strike: T, delta: T, freq: T, tenor: T) -> T {
    let eps = T::epsilon();
    let m: T = from_f64(10.0);
    let one = T::one();
    let two: T = from_f64(2.0);
    let three: T = from_f64(3.0);
    let four: T = from_f64(4.0);
    let six: T = from_f64(6.0);
    let half: T = from_f64(0.5);

    let dcf = one / freq;
    let payments = freq * tenor;

    let threshold = dcf * strike * dcf * strike;

    if threshold.abs() > eps * m {
        let tmp_a = strike * dcf;
        let tmp_b = one + tmp_a;
        let tmp_c = tmp_b.powf(delta);
        let tmp_d = one - one / tmp_b.powf(payments);
        let tmp_e = one - delta + payments;
        let tmp_f = (delta - payments) / tmp_b;

        (tmp_e + tmp_f - payments * tmp_a / tmp_b / tmp_d) / tmp_c / tmp_d
    } else {
        // Taylor expansion near strike = 0
        let p1 = (delta - payments) * dcf;
        let p2 = p1 * (delta - payments - one) * dcf;
        let p3 = p2 * (delta - payments - two) * dcf;

        let q1 = payments * dcf;
        let q2 = q1 * (payments - one) * dcf;
        let q3 = q2 * (payments - two) * dcf;
        let q4 = q3 * (payments - three) * dcf;

        let g1 = q1;
        let g2 = q2 + two * p1 * q1;
        let g3 = q3 + three * p1 * q2 + three * p2 * q1;
        let g4 = q4 + four * p1 * q3 + six * p2 * q2 + four * p3 * q1;

        let f1 = -g2 / two / g1 / g1;
        let f2 = g2 * g2 / two / g1 / g1 / g1 - g3 / three / g1 / g1;
        let f3 = g2 * g3 / g1 / g1 / g1
            - g4 / four / g1 / g1
            - three * g2 * g2 * g2 / four / g1 / g1 / g1 / g1;

        f1 + f2 * strike + half * f3 * strike * strike
    }
}

/// Computes the log numeraire ratio derivative: G'(F + spread) / G(F + spread).
///
/// `delta = (pay_date - effective_date) / (first_payment_date - effective_date)`.
pub fn calc_log_numeraire_ratio_derivative<T: Float>(
    ref_term: T,
    pay_freq: T,
    fwd_swap: T,
    lo_spread: T,
    effective_date_yf: T,
    first_payment_date_yf: T,
    pay_date_yf: T,
) -> T {
    let delta = (pay_date_yf - effective_date_yf) / (first_payment_date_yf - effective_date_yf);

    let shifted = fwd_swap + lo_spread;
    let div = numeraire_ratio(shifted, delta, pay_freq, ref_term);
    let num = numeraire_ratio_derivative(shifted, delta, pay_freq, ref_term);

    num / div
}

// ─── Option price integration ───────────────────────────

/// Adaptive integration of OTM option prices from `start_strike` outward.
///
/// Translates the C++ `integrateOptionPriceImpl` which integrates call prices
/// from the starting strike towards +infinity (or put prices towards -infinity)
/// using adaptive Gauss-Kronrod quadrature with convergence checks.
///
/// # Arguments
/// * `option_price_fn` - Returns the option price: `fn(strike, is_call) -> price`
/// * `time_value_fn` - Returns the time value at a given strike
/// * `is_call` - `true` for call integration (towards +inf), `false` for put
/// * `start_strike` - Starting strike for integration
/// * `normal_stdev` - Normal standard deviation for determining step width
/// * `daycount_adjust` - Daycount adjustment divisor
/// * `integral_tolerance` - Relative convergence tolerance
/// * `time_value_tolerance` - Absolute tolerance for time value cutoff
pub fn integrate_option_price<T: Float>(
    option_price_fn: &dyn Fn(T, bool) -> T,
    time_value_fn: &dyn Fn(T) -> T,
    is_call: bool,
    start_strike: T,
    normal_stdev: T,
    daycount_adjust: T,
    integral_tolerance: T,
    time_value_tolerance: T,
) -> T {
    let five: T = from_f64(5.0);
    let two: T = from_f64(2.0);
    let min_stdev: T = from_f64(1e-5);

    let cp_sign = if is_call { T::one() } else { -T::one() };

    let f = |x: T| -> T { option_price_fn(x, is_call) / daycount_adjust };

    let clamped_stdev = if normal_stdev > min_stdev {
        normal_stdev
    } else {
        min_stdev
    };
    let mut width = five * clamped_stdev;

    let mut integral = T::zero();
    let mut prev_integral = T::infinity();

    let mut prev_strike = start_strike;
    let mut next_strike = start_strike + cp_sign * width;

    let max_iter = 1000usize;
    let sub_max_iter = 10usize;
    let mut n_iter = 0usize;
    let mut sub_n_iter = 0usize;

    loop {
        if sub_n_iter > sub_max_iter {
            sub_n_iter = 0;
            width = width * two;
        }

        let (lower, upper) = if is_call {
            (prev_strike, next_strike)
        } else {
            (next_strike, prev_strike)
        };

        integral = integral + gauss_kronrod_integrate(&f, lower, upper);

        if (integral - prev_integral).abs() < integral_tolerance * prev_integral.abs() {
            break;
        }

        let time_value = time_value_fn(next_strike);
        if time_value < time_value_tolerance {
            break;
        }

        prev_integral = integral;
        prev_strike = next_strike;
        next_strike = next_strike + cp_sign * width;

        sub_n_iter += 1;
        n_iter += 1;

        if n_iter > max_iter {
            break;
        }
    }

    integral
}

/// Fixed-range call price integration using Gauss-Kronrod quadrature.
pub fn integrate_call_price<T: Float>(
    option_price_fn: &dyn Fn(T, bool) -> T,
    min_strike: T,
    max_strike: T,
    daycount_adjust: T,
) -> T {
    let f = |x: T| -> T { option_price_fn(x, true) / daycount_adjust };
    gauss_kronrod_integrate(&f, min_strike, max_strike)
}

/// Fixed-range put price integration using Gauss-Kronrod quadrature.
pub fn integrate_put_price<T: Float>(
    option_price_fn: &dyn Fn(T, bool) -> T,
    min_strike: T,
    max_strike: T,
    daycount_adjust: T,
) -> T {
    let f = |x: T| -> T { option_price_fn(x, false) / daycount_adjust };
    gauss_kronrod_integrate(&f, min_strike, max_strike)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    // ─── Gauss-Kronrod tests ──────────────────────────

    #[test]
    fn gk_integrates_constant() {
        // ∫₀¹ 3 dx = 3
        let result = gauss_kronrod_integrate(&|_x: f64| 3.0, 0.0, 1.0);
        assert_relative_eq!(result, 3.0, epsilon = 1e-14);
    }

    #[test]
    fn gk_integrates_polynomial() {
        // ∫₀¹ x² dx = 1/3
        let result = gauss_kronrod_integrate(&|x: f64| x * x, 0.0, 1.0);
        assert_relative_eq!(result, 1.0 / 3.0, epsilon = 1e-14);
    }

    #[test]
    fn gk_integrates_higher_polynomial() {
        // G7-K15 is exact for polynomials up to degree 29
        // ∫₋₁¹ x⁶ dx = 2/7
        let result = gauss_kronrod_integrate(&|x: f64| x.powi(6), -1.0, 1.0);
        assert_relative_eq!(result, 2.0 / 7.0, epsilon = 1e-13);
    }

    #[test]
    fn gk_integrates_exp() {
        // ∫₀¹ e^x dx = e - 1
        let result = gauss_kronrod_integrate(&|x: f64| x.exp(), 0.0, 1.0);
        assert_relative_eq!(result, 1.0_f64.exp() - 1.0, epsilon = 1e-14);
    }

    // ─── Numeraire ratio tests ────────────────────────

    #[test]
    fn numeraire_ratio_typical_10y_sa() {
        // 10Y semi-annual swap, delta=1 (spot start), forward=3%
        let g = numeraire_ratio(0.03_f64, 1.0, 2.0, 10.0);
        // G(K) must be positive and close to 1/payments for small K
        assert!(g > 0.0);
        assert!(g < 1.0);
    }

    #[test]
    fn numeraire_ratio_near_zero_strike() {
        // Taylor branch: very small strike
        let g_taylor = numeraire_ratio(1e-12_f64, 1.5, 2.0, 10.0);
        // Should equal f0 = 1/g1 = 1/(payments * dcf) = freq/payments = 1/tenor = 0.1
        assert_relative_eq!(g_taylor, 0.1, epsilon = 1e-6);
    }

    #[test]
    fn numeraire_ratio_continuity_at_threshold() {
        // Values just above and below Taylor threshold should be close
        let eps = f64::EPSILON;
        let dcf = 0.5_f64; // freq=2
        // threshold = (dcf * strike)^2 ~ 10 * eps => strike ~ sqrt(10*eps) / dcf
        let critical_strike = (10.0 * eps).sqrt() / dcf;

        let g_above = numeraire_ratio(critical_strike * 1.1, 1.5, 2.0, 10.0);
        let g_below = numeraire_ratio(critical_strike * 0.9, 1.5, 2.0, 10.0);

        // Should be continuous
        assert_relative_eq!(g_above, g_below, epsilon = 1e-8);
    }

    #[test]
    fn numeraire_ratio_derivative_finite_difference() {
        let strike = 0.03_f64;
        let delta = 1.5;
        let freq = 2.0;
        let tenor = 10.0;

        let h = 1e-7;
        let fd = (numeraire_ratio(strike + h, delta, freq, tenor)
            - numeraire_ratio(strike - h, delta, freq, tenor))
            / (2.0 * h);

        let analytical = numeraire_ratio_derivative(strike, delta, freq, tenor);

        assert_relative_eq!(fd, analytical, epsilon = 1e-5);
    }

    #[test]
    fn log_numeraire_ratio_derivative_basic() {
        let ratio = calc_log_numeraire_ratio_derivative(
            10.0, 2.0, 0.03, 0.001, 0.0, 0.5, 10.5,
        );
        // Should be finite and non-zero for typical CMS parameters
        assert!(ratio.is_finite());
        assert!(ratio.abs() > 0.0);
    }
}
