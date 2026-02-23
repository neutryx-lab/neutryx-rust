//! Analytical pricing functions for the Hull-White 1-Factor model.
//!
//! Provides closed-form formulas for:
//! - Zero-coupon bond prices (`hw_bond_price`, `hw_b_factor`)
//! - Conditional mean and variance of the state variable
//! - Par swap rate computation from the short rate state
//! - Swap mark-to-market (MtM) valuation
//! - Discount curve reconstruction
//!
//! These functions leverage the affine yield property of HW1F:
//! given the short rate `r(t)` at time `t`, the price of a zero-coupon
//! bond maturing at `T` is:
//!
//! ```text
//! P(t,T) = exp( A(t,T) - B(t,T) * r(t) )
//! ```
//!
//! where `B(t,T) = (1 - exp(-a*(T-t))) / a` and `A(t,T)` depends on the
//! initial yield curve and model parameters.

use pricer_core::traits::Float;

// Re-export the core bond-pricing helpers that live in the Jarrow-Yildirim module
// so that callers can use a single import path.
pub use super::jarrow_yildirim::{hw_b_factor, hw_bond_price, hw_log_bond_price};

// ─── Conditional moments ────────────────────────────────────────────────────

/// Conditional variance of the HW1F state variable at time `t`.
///
/// ```text
/// V(t) = σ² / (2a) * (1 - exp(-2at))
/// ```
///
/// For `a ≈ 0` falls back to `σ² * t`.
#[inline]
pub fn hw_conditional_variance<T: Float>(a: T, sigma: T, t: T) -> T {
    let two = T::from(2.0).unwrap_or(T::one() + T::one());
    let eps = T::from(1e-10).unwrap_or(T::zero());

    if a.abs() < eps {
        sigma * sigma * t
    } else {
        (sigma * sigma / (two * a)) * (T::one() - (-two * a * t).exp())
    }
}

/// Conditional mean of `x(t)` given `x(s)` under HW1F dynamics.
///
/// ```text
/// E[x(t) | x(s)] = x(s) * exp(-a * (t - s))
/// ```
///
/// Here the "state variable" `x` is the short rate `r` when using the
/// standard HW1F SDE.
#[inline]
pub fn hw_conditional_mean<T: Float>(a: T, x_s: T, s: T, t: T) -> T { x_s * (-(a * (t - s))).exp() }

// ─── Discount curve reconstruction ──────────────────────────────────────────

/// Build a vector of zero-coupon bond prices from the HW1F state.
///
/// Returns `P(t, T_i)` for each maturity `T_i` in `maturities`.
/// Uses the HW1F affine formula via [`hw_bond_price`].
pub fn hw_discount_curve<T: Float>(
    a: T,
    sigma: T,
    r_star: T,
    t: T,
    r_t: T,
    maturities: &[T],
) -> Vec<T> {
    maturities
        .iter()
        .map(|&mat| hw_bond_price(a, sigma, r_star, t, mat, r_t))
        .collect()
}

// ─── Par swap rate ──────────────────────────────────────────────────────────

/// Compute the par swap rate observed at time `t` from the HW1F state.
///
/// The par swap rate for a swap starting at `t` with tenor `swap_tenor` and
/// payment frequency `payment_freq` (in year fractions, e.g. 0.5 for
/// semi-annual) is:
///
/// ```text
/// S(t) = (P(t, t) - P(t, t + tenor)) / Annuity(t)
/// ```
///
/// where `Annuity(t) = Σ_i dcf_i * P(t, t_i)` summed over the payment dates.
///
/// Note: `P(t, t) = 1` by definition, so effectively:
///
/// ```text
/// S(t) = (1 - P(t, t + tenor)) / Annuity(t)
/// ```
pub fn hw_swap_rate<T: Float>(
    a: T,
    sigma: T,
    r_star: T,
    t: T,
    r_t: T,
    swap_tenor: T,
    payment_freq: T,
) -> T {
    let eps = T::from(1e-15).unwrap_or(T::zero());

    // Number of payment periods
    let n_periods_f = (swap_tenor / payment_freq).round();
    let n_periods = n_periods_f.to_usize().unwrap_or(1).max(1);

    // Terminal discount factor
    let df_end = hw_bond_price(a, sigma, r_star, t, t + swap_tenor, r_t);

    // Annuity: sum of dcf * DF for each payment date
    let mut annuity = T::zero();
    for i in 1..=n_periods {
        let t_i = t + payment_freq * T::from(i).unwrap_or(T::one());
        let df_i = hw_bond_price(a, sigma, r_star, t, t_i, r_t);
        annuity = annuity + payment_freq * df_i;
    }

    if annuity.abs() < eps {
        return T::zero();
    }

    (T::one() - df_end) / annuity
}

// ─── Swap MtM ───────────────────────────────────────────────────────────────

/// Compute the mark-to-market of a vanilla interest rate swap from the
/// HW1F state.
///
/// The swap MtM for a **payer** swap (pay fixed, receive floating) is:
///
/// ```text
/// MtM = Notional * (S_current - S_fixed) * Annuity
/// ```
///
/// where `S_current` is the current par swap rate and `Annuity` is the
/// present value of the remaining fixed-leg basis points.
///
/// For a **receiver** swap the sign is flipped:
///
/// ```text
/// MtM = Notional * (S_fixed - S_current) * Annuity
/// ```
///
/// # Arguments
///
/// * `a`, `sigma`, `r_star` — HW1F model parameters
/// * `t` — current time (year fraction)
/// * `r_t` — short rate at time `t`
/// * `fixed_rate` — contractual fixed rate of the swap
/// * `notional` — swap notional
/// * `payment_times` — remaining payment dates (absolute year fractions, `> t`)
/// * `is_payer` — `true` for payer swap (pay fixed), `false` for receiver
pub fn hw_swap_mtm<T: Float>(
    a: T,
    sigma: T,
    r_star: T,
    t: T,
    r_t: T,
    fixed_rate: T,
    notional: T,
    payment_times: &[T],
    is_payer: bool,
) -> T {
    if payment_times.is_empty() {
        return T::zero();
    }

    // Filter to remaining payments (payment_time > t)
    let remaining: Vec<T> = payment_times.iter().copied().filter(|&pt| pt > t).collect();

    if remaining.is_empty() {
        return T::zero();
    }

    // Floating leg value = P(t, t_start) - P(t, t_end)
    // For a swap that has already started, t_start = t, so P(t,t) = 1.
    // t_end = last payment date
    let t_end = remaining[remaining.len() - 1];
    let df_end = hw_bond_price(a, sigma, r_star, t, t_end, r_t);
    let float_leg = T::one() - df_end;

    // Fixed leg value = fixed_rate * Annuity
    let mut annuity = T::zero();
    let mut prev = t;
    for &t_pay in &remaining {
        let dcf = t_pay - prev;
        let df = hw_bond_price(a, sigma, r_star, t, t_pay, r_t);
        annuity = annuity + dcf * df;
        prev = t_pay;
    }
    let fixed_leg = fixed_rate * annuity;

    if is_payer {
        // Pay fixed, receive floating
        notional * (float_leg - fixed_leg)
    } else {
        // Receive fixed, pay floating
        notional * (fixed_leg - float_leg)
    }
}

/// Compute the swap MtM using a simplified interface where the remaining
/// tenor and payment frequency are specified instead of explicit dates.
///
/// Internally generates the payment schedule and delegates to [`hw_swap_mtm`].
pub fn hw_swap_mtm_from_tenor<T: Float>(
    a: T,
    sigma: T,
    r_star: T,
    t: T,
    r_t: T,
    fixed_rate: T,
    notional: T,
    remaining_tenor: T,
    payment_freq: T,
    is_payer: bool,
) -> T {
    let n_periods_f = (remaining_tenor / payment_freq).round();
    let n_periods = n_periods_f.to_usize().unwrap_or(1).max(1);

    let payment_times: Vec<T> = (1..=n_periods)
        .map(|i| t + payment_freq * T::from(i).unwrap_or(T::one()))
        .collect();

    hw_swap_mtm(
        a,
        sigma,
        r_star,
        t,
        r_t,
        fixed_rate,
        notional,
        &payment_times,
        is_payer,
    )
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const A: f64 = 0.05;
    const SIGMA: f64 = 0.01;
    const R_STAR: f64 = 0.03;

    #[test]
    fn conditional_variance_at_zero() {
        let v = hw_conditional_variance(A, SIGMA, 0.0);
        assert!(v.abs() < 1e-15);
    }

    #[test]
    fn conditional_variance_positive() {
        let v = hw_conditional_variance(A, SIGMA, 1.0);
        // V(1) = 0.01^2 / (2*0.05) * (1 - exp(-0.1)) ≈ 0.001 * 0.09516 ≈ 9.516e-5
        assert!(v > 0.0);
        assert!((v - 9.516e-5).abs() < 1e-6);
    }

    #[test]
    fn conditional_variance_limit_large_t() {
        let v = hw_conditional_variance(A, SIGMA, 100.0);
        // Limit: σ²/(2a) = 0.0001/0.1 = 0.001
        assert!((v - 0.001).abs() < 1e-6);
    }

    #[test]
    fn conditional_mean_no_reversion() {
        // At s=t, E[x(t)|x(s)] = x(s)
        let m = hw_conditional_mean(A, 0.05, 1.0, 1.0);
        assert!((m - 0.05).abs() < 1e-15);
    }

    #[test]
    fn conditional_mean_decays() {
        let m = hw_conditional_mean(A, 0.05, 0.0, 1.0);
        // E = 0.05 * exp(-0.05) ≈ 0.04756
        assert!((m - 0.05 * (-0.05_f64).exp()).abs() < 1e-10);
    }

    #[test]
    fn discount_curve_monotone() {
        let mats = vec![1.0, 2.0, 5.0, 10.0];
        let dfs = hw_discount_curve(A, SIGMA, R_STAR, 0.0, R_STAR, &mats);
        // DFs should be decreasing (positive rates)
        for i in 1..dfs.len() {
            assert!(dfs[i] < dfs[i - 1]);
        }
    }

    #[test]
    fn swap_rate_at_par() {
        // At t=0 with r_t = r*, the swap rate should be close to the flat rate
        let sr = hw_swap_rate(A, SIGMA, R_STAR, 0.0, R_STAR, 5.0, 0.5);
        // For a flat curve, par swap rate ≈ the flat rate (with small convexity
        // correction)
        assert!((sr - R_STAR).abs() < 0.005);
    }

    #[test]
    fn swap_rate_positive() {
        let sr = hw_swap_rate(A, SIGMA, R_STAR, 0.0, R_STAR, 10.0, 0.25);
        assert!(sr > 0.0);
    }

    #[test]
    fn swap_mtm_at_par_near_zero() {
        // A swap struck at par should have near-zero MtM
        let sr = hw_swap_rate(A, SIGMA, R_STAR, 0.0, R_STAR, 5.0, 0.5);
        let payments: Vec<f64> = (1..=10).map(|i| i as f64 * 0.5).collect();
        let mtm = hw_swap_mtm(
            A,
            SIGMA,
            R_STAR,
            0.0,
            R_STAR,
            sr,
            1_000_000.0,
            &payments,
            true,
        );
        assert!(mtm.abs() < 1.0); // Should be very close to zero
    }

    #[test]
    fn swap_mtm_payer_receiver_symmetry() {
        let payments: Vec<f64> = (1..=10).map(|i| i as f64 * 0.5).collect();
        let payer = hw_swap_mtm(A, SIGMA, R_STAR, 0.0, 0.04, 0.03, 1e6, &payments, true);
        let receiver = hw_swap_mtm(A, SIGMA, R_STAR, 0.0, 0.04, 0.03, 1e6, &payments, false);
        assert!((payer + receiver).abs() < 1e-6);
    }

    #[test]
    fn swap_mtm_rate_up_payer_positive() {
        // If rates go up, a payer swap (pay fixed, receive floating) gains
        let payments: Vec<f64> = (1..=20).map(|i| i as f64 * 0.25).collect();
        let mtm = hw_swap_mtm(A, SIGMA, R_STAR, 0.0, 0.05, R_STAR, 1e6, &payments, true);
        assert!(mtm > 0.0);
    }

    #[test]
    fn swap_mtm_rate_down_receiver_positive() {
        // If rates go down, a receiver swap (receive fixed) gains
        let payments: Vec<f64> = (1..=20).map(|i| i as f64 * 0.25).collect();
        let mtm = hw_swap_mtm(A, SIGMA, R_STAR, 0.0, 0.01, R_STAR, 1e6, &payments, false);
        assert!(mtm > 0.0);
    }

    #[test]
    fn swap_mtm_empty_payments() {
        let mtm = hw_swap_mtm(A, SIGMA, R_STAR, 0.0, R_STAR, 0.03, 1e6, &[], true);
        assert!(mtm.abs() < 1e-15);
    }

    #[test]
    fn swap_mtm_from_tenor_matches() {
        let payments: Vec<f64> = (1..=10).map(|i| i as f64 * 0.5).collect();
        let mtm1 = hw_swap_mtm(A, SIGMA, R_STAR, 0.0, 0.04, 0.03, 1e6, &payments, true);
        let mtm2 = hw_swap_mtm_from_tenor(A, SIGMA, R_STAR, 0.0, 0.04, 0.03, 1e6, 5.0, 0.5, true);
        assert!((mtm1 - mtm2).abs() < 1e-6);
    }

    #[test]
    fn swap_mtm_future_time_filters_payments() {
        // At t=2.5, only payments after 2.5 should count
        let payments: Vec<f64> = (1..=10).map(|i| i as f64 * 0.5).collect();
        let mtm = hw_swap_mtm(A, SIGMA, R_STAR, 2.5, R_STAR, 0.03, 1e6, &payments, true);
        // Should still be finite and reasonable
        assert!(mtm.is_finite());
    }
}
