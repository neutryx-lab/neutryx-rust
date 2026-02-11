//! Slice-based pricing kernels for Enzyme AAD.

#[cfg(feature = "enzyme-ad")]
use std::autodiff::autodiff;

/// Interest Rate Swap (IRS) pricing kernel.
#[cfg(feature = "enzyme-ad")]
#[autodiff(
    d_pricing_kernel_irs,
    Reverse,
    Duplicated,
    Const,
    Const,
    Const,
    Const,
    Duplicated
)]
pub fn pricing_kernel_irs(
    rates: &[f64],
    times: &[f64],
    notionals: &[f64],
    year_fractions: &[f64],
    fixed_rate: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), times.len());
    debug_assert_eq!(rates.len(), notionals.len());
    debug_assert_eq!(rates.len(), year_fractions.len());

    let n = rates.len();
    let mut pv = 0.0;

    for i in 0..n {
        let df = (-rates[i] * times[i]).exp();
        let floating_cf = notionals[i] * rates[i] * year_fractions[i] * df;
        let fixed_cf = notionals[i] * fixed_rate * year_fractions[i] * df;
        pv += floating_cf - fixed_cf;
    }

    *output = pv;
}

#[cfg(not(feature = "enzyme-ad"))]
pub fn pricing_kernel_irs(
    rates: &[f64],
    times: &[f64],
    notionals: &[f64],
    year_fractions: &[f64],
    fixed_rate: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), times.len());
    debug_assert_eq!(rates.len(), notionals.len());
    debug_assert_eq!(rates.len(), year_fractions.len());

    let n = rates.len();
    let mut pv = 0.0;

    for i in 0..n {
        let df = (-rates[i] * times[i]).exp();
        let floating_cf = notionals[i] * rates[i] * year_fractions[i] * df;
        let fixed_cf = notionals[i] * fixed_rate * year_fractions[i] * df;
        pv += floating_cf - fixed_cf;
    }

    *output = pv;
}

/// Simple discount factor computation kernel.
#[cfg(feature = "enzyme-ad")]
#[autodiff(d_discount_kernel, Reverse, Duplicated, Const, Const, Duplicated)]
pub fn discount_kernel(rate: &f64, time: f64, cashflow: f64, output: &mut f64) {
    *output = cashflow * (-*rate * time).exp();
}

#[cfg(not(feature = "enzyme-ad"))]
pub fn discount_kernel(rate: &f64, time: f64, cashflow: f64, output: &mut f64) {
    *output = cashflow * (-*rate * time).exp();
}

/// Bond pricing kernel computing PV of coupons plus principal.
#[cfg(feature = "enzyme-ad")]
#[autodiff(
    d_bond_pricing_kernel,
    Reverse,
    Duplicated,
    Const,
    Const,
    Const,
    Duplicated
)]
pub fn bond_pricing_kernel(
    rates: &[f64],
    times: &[f64],
    coupon_rate: f64,
    face_value: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), times.len());
    debug_assert!(!rates.is_empty());

    let n = rates.len();
    let mut pv = 0.0;

    for i in 0..n {
        let df = (-rates[i] * times[i]).exp();
        let coupon = face_value * coupon_rate;
        pv += coupon * df;
    }

    let df_maturity = (-rates[n - 1] * times[n - 1]).exp();
    pv += face_value * df_maturity;

    *output = pv;
}

#[cfg(not(feature = "enzyme-ad"))]
pub fn bond_pricing_kernel(
    rates: &[f64],
    times: &[f64],
    coupon_rate: f64,
    face_value: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), times.len());
    debug_assert!(!rates.is_empty());

    let n = rates.len();
    let mut pv = 0.0;

    for i in 0..n {
        let df = (-rates[i] * times[i]).exp();
        let coupon = face_value * coupon_rate;
        pv += coupon * df;
    }

    let df_maturity = (-rates[n - 1] * times[n - 1]).exp();
    pv += face_value * df_maturity;

    *output = pv;
}

/// FRA (Forward Rate Agreement) pricing kernel.
#[cfg(feature = "enzyme-ad")]
#[autodiff(
    d_fra_pricing_kernel,
    Reverse,
    Duplicated,
    Const,
    Const,
    Const,
    Duplicated
)]
pub fn fra_pricing_kernel(
    rates: &[f64],
    times: &[f64],
    notional: f64,
    fra_rate: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), 2);
    debug_assert_eq!(times.len(), 2);

    let df_short = (-rates[0] * times[0]).exp();
    let df_long = (-rates[1] * times[1]).exp();

    let accrual = times[1] - times[0];
    let forward_rate = (df_short / df_long - 1.0) / accrual;

    *output = notional * (forward_rate - fra_rate) * accrual * df_long;
}

#[cfg(not(feature = "enzyme-ad"))]
pub fn fra_pricing_kernel(
    rates: &[f64],
    times: &[f64],
    notional: f64,
    fra_rate: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), 2);
    debug_assert_eq!(times.len(), 2);

    let df_short = (-rates[0] * times[0]).exp();
    let df_long = (-rates[1] * times[1]).exp();

    let accrual = times[1] - times[0];
    let forward_rate = (df_short / df_long - 1.0) / accrual;

    *output = notional * (forward_rate - fra_rate) * accrual * df_long;
}

/// Compute gradients using central finite differences.
pub fn finite_difference_gradients<F>(
    kernel: F,
    rates: &[f64],
    times: &[f64],
    notionals: &[f64],
    year_fractions: &[f64],
    fixed_rate: f64,
    bump_size: f64,
) -> Vec<f64>
where
    F: Fn(&[f64], &[f64], &[f64], &[f64], f64, &mut f64),
{
    let n = rates.len();
    let mut gradients = vec![0.0; n];
    let mut rates_bumped = rates.to_vec();

    for i in 0..n {
        rates_bumped[i] = rates[i] + bump_size;
        let mut pv_up = 0.0;
        kernel(
            &rates_bumped,
            times,
            notionals,
            year_fractions,
            fixed_rate,
            &mut pv_up,
        );

        rates_bumped[i] = rates[i] - bump_size;
        let mut pv_down = 0.0;
        kernel(
            &rates_bumped,
            times,
            notionals,
            year_fractions,
            fixed_rate,
            &mut pv_down,
        );

        gradients[i] = (pv_up - pv_down) / (2.0 * bump_size);

        rates_bumped[i] = rates[i];
    }

    gradients
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_kernel_irs_basic() {
        let rates = [0.03, 0.03, 0.03];
        let times = [1.0, 2.0, 3.0];
        let notionals = [1_000_000.0, 1_000_000.0, 1_000_000.0];
        let year_fractions = [1.0, 1.0, 1.0];
        let fixed_rate = 0.03;

        let mut pv = 0.0;
        pricing_kernel_irs(
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            &mut pv,
        );

        assert!(
            pv.abs() < 1.0,
            "ATM swap should have near-zero PV, got {}",
            pv
        );
    }

    #[test]
    fn test_pricing_kernel_irs_positive_pv() {
        let rates = [0.04, 0.04, 0.04];
        let times = [1.0, 2.0, 3.0];
        let notionals = [1_000_000.0, 1_000_000.0, 1_000_000.0];
        let year_fractions = [1.0, 1.0, 1.0];
        let fixed_rate = 0.02;

        let mut pv = 0.0;
        pricing_kernel_irs(
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            &mut pv,
        );

        assert!(pv > 0.0, "Expected positive PV, got {}", pv);
    }

    #[test]
    fn test_pricing_kernel_irs_negative_pv() {
        let rates = [0.02, 0.02, 0.02];
        let times = [1.0, 2.0, 3.0];
        let notionals = [1_000_000.0, 1_000_000.0, 1_000_000.0];
        let year_fractions = [1.0, 1.0, 1.0];
        let fixed_rate = 0.04;

        let mut pv = 0.0;
        pricing_kernel_irs(
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            &mut pv,
        );

        assert!(pv < 0.0, "Expected negative PV, got {}", pv);
    }

    #[test]
    fn test_discount_kernel_basic() {
        let rate = 0.05;
        let time = 1.0;
        let cashflow = 100.0;

        let mut pv = 0.0;
        discount_kernel(&rate, time, cashflow, &mut pv);

        let expected = 100.0 * (-0.05_f64).exp();
        assert!(
            (pv - expected).abs() < 1e-10,
            "Expected {}, got {}",
            expected,
            pv
        );
    }

    #[test]
    fn test_discount_kernel_zero_rate() {
        let rate = 0.0;
        let time = 1.0;
        let cashflow = 100.0;

        let mut pv = 0.0;
        discount_kernel(&rate, time, cashflow, &mut pv);

        assert!((pv - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_bond_pricing_kernel_basic() {
        let rates = [0.03, 0.03, 0.03];
        let times = [1.0, 2.0, 3.0];
        let coupon_rate = 0.05;
        let face_value = 100.0;

        let mut pv = 0.0;
        bond_pricing_kernel(&rates, &times, coupon_rate, face_value, &mut pv);

        assert!(pv > face_value, "Expected PV > {}, got {}", face_value, pv);
    }

    #[test]
    fn test_bond_pricing_kernel_par_bond() {
        let rates = [0.05, 0.05, 0.05];
        let times = [1.0, 2.0, 3.0];
        let coupon_rate = 0.05;
        let face_value = 100.0;

        let mut pv = 0.0;
        bond_pricing_kernel(&rates, &times, coupon_rate, face_value, &mut pv);

        assert!(
            (pv - face_value).abs() < 5.0,
            "Expected PV close to {}, got {}",
            face_value,
            pv
        );
    }

    #[test]
    fn test_fra_pricing_kernel_basic() {
        let rates = [0.02, 0.03];
        let times = [0.25, 0.5];
        let notional = 1_000_000.0;
        let fra_rate = 0.04;

        let mut pv = 0.0;
        fra_pricing_kernel(&rates, &times, notional, fra_rate, &mut pv);

        assert!(pv.abs() > 0.0);
    }

    #[test]
    fn test_fra_pricing_kernel_atm() {
        let rates = [0.02_f64, 0.02];
        let times = [0.25_f64, 0.5];
        let notional = 1_000_000.0;

        let df_short = (-rates[0] * times[0]).exp();
        let df_long = (-rates[1] * times[1]).exp();
        let accrual = times[1] - times[0];
        let implied_forward = (df_short / df_long - 1.0) / accrual;

        let mut pv = 0.0;
        fra_pricing_kernel(&rates, &times, notional, implied_forward, &mut pv);

        assert!(
            pv.abs() < 1.0,
            "ATM FRA should have near-zero PV, got {}",
            pv
        );
    }

    #[test]
    fn test_finite_difference_gradients() {
        let rates = [0.03, 0.03, 0.03];
        let times = [1.0, 2.0, 3.0];
        let notionals = [1_000_000.0, 1_000_000.0, 1_000_000.0];
        let year_fractions = [1.0, 1.0, 1.0];
        let fixed_rate = 0.025;

        let gradients = finite_difference_gradients(
            pricing_kernel_irs,
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            1e-7,
        );

        for (i, &grad) in gradients.iter().enumerate() {
            assert!(
                grad.abs() > 0.0,
                "Gradient {} should be non-zero, got {}",
                i,
                grad
            );
        }
    }

    #[test]
    fn test_gradients_sign() {
        let rates = [0.02, 0.02, 0.02];
        let times = [1.0, 2.0, 3.0];
        let notionals = [1_000_000.0, 1_000_000.0, 1_000_000.0];
        let year_fractions = [1.0, 1.0, 1.0];
        let fixed_rate = 0.02;

        let gradients = finite_difference_gradients(
            pricing_kernel_irs,
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            1e-7,
        );

        for (i, &grad) in gradients.iter().enumerate() {
            assert!(
                grad > 0.0,
                "ATM swap gradient {} should be positive, got {}",
                i,
                grad
            );
        }
    }

    #[test]
    fn test_empty_inputs() {
        let rates: [f64; 0] = [];
        let times: [f64; 0] = [];
        let notionals: [f64; 0] = [];
        let year_fractions: [f64; 0] = [];
        let fixed_rate = 0.03;

        let mut pv = 0.0;
        pricing_kernel_irs(
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            &mut pv,
        );

        assert_eq!(pv, 0.0);
    }

    #[test]
    fn test_single_period() {
        let rates = [0.05];
        let times = [1.0];
        let notionals = [1_000_000.0];
        let year_fractions = [1.0];
        let fixed_rate = 0.03;

        let mut pv = 0.0;
        pricing_kernel_irs(
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            &mut pv,
        );

        let df = (-0.05_f64).exp();
        let expected = 1_000_000.0 * (0.05 - 0.03) * 1.0 * df;

        assert!(
            (pv - expected).abs() < 1e-6,
            "Expected {}, got {}",
            expected,
            pv
        );
    }
}
