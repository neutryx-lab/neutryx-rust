//! Conundrum integrand convexity adjuster for CMS rates.
//!
//! Implements CMS convexity adjustments via three methods:
//! - Numerical integration (replication via OTM option prices)
//! - Normal analytic (closed-form under Bachelier dynamics)
//! - Shifted log-normal analytic (closed-form under SLN dynamics)

use pricer_core::{
    math::{normal_dist::norm_cdf, numeric::from_f64},
    traits::Float,
};

use super::{
    params::{ConvexityAdjustCalcMethod, ConvexityAdjusterParams},
    support, ConvexityAdjuster, ConvexityAdjustmentError,
};

/// CMS convexity adjuster using the conundrum integrand (replication) method.
///
/// Supports three calculation methods:
/// - **NumericalIntegration**: Full replication via adaptive integration of OTM
///   option prices.
/// - **NormalAnalytic**: Closed-form under Normal (Bachelier) vol.
/// - **SlnAnalytic**: Closed-form under Shifted Log-Normal vol.
#[derive(Debug, Clone)]
pub struct ConundrumIntegrandConvexityAdjuster<T: Float> {
    params: ConvexityAdjusterParams<T>,
    calc_method: ConvexityAdjustCalcMethod,
}

impl<T: Float> ConundrumIntegrandConvexityAdjuster<T> {
    /// Creates a new conundrum integrand convexity adjuster.
    pub fn new(params: ConvexityAdjusterParams<T>, calc_method: ConvexityAdjustCalcMethod) -> Self {
        Self {
            params,
            calc_method,
        }
    }

    /// Creates with default parameters and numerical integration.
    pub fn default_numerical() -> Self {
        Self::new(
            ConvexityAdjusterParams::default(),
            ConvexityAdjustCalcMethod::NumericalIntegration,
        )
    }

    /// Creates with default parameters and Normal analytic method.
    pub fn default_normal_analytic() -> Self {
        Self::new(
            ConvexityAdjusterParams::default(),
            ConvexityAdjustCalcMethod::NormalAnalytic,
        )
    }

    /// Creates with default parameters and Shifted Log-Normal analytic method.
    pub fn default_sln_analytic() -> Self {
        Self::new(
            ConvexityAdjusterParams::default(),
            ConvexityAdjustCalcMethod::SlnAnalytic,
        )
    }

    /// Returns the calculation method.
    pub fn calc_method(&self) -> ConvexityAdjustCalcMethod { self.calc_method }
}

impl<T: Float> ConvexityAdjuster<T> for ConundrumIntegrandConvexityAdjuster<T> {
    fn does_apply(&self, end_date_yf: T, payment_date_yf: T) -> bool {
        let grace_years: T = from_f64(self.params.grace_period_days as f64 / 365.0);
        (payment_date_yf - end_date_yf).abs() > grace_years
    }

    fn compute_adjustment(
        &self,
        ref_term: T,
        pay_freq: T,
        fwd_swap: T,
        lo_spread: T,
        effective_date_yf: T,
        end_date_yf: T,
        pay_date_yf: T,
        option_price_fn: &dyn Fn(T, bool) -> T,
        time_value_fn: &dyn Fn(T) -> T,
        normal_vol: T,
        option_term: T,
        daycount_adjust: T,
    ) -> Result<T, ConvexityAdjustmentError> {
        if option_term <= T::zero() {
            return Ok(T::zero());
        }

        let two: T = from_f64(2.0);

        let ratio = support::calc_log_numeraire_ratio_derivative(
            ref_term,
            pay_freq,
            fwd_swap,
            lo_spread,
            effective_date_yf,
            end_date_yf,
            pay_date_yf,
        );

        let fwd_vol_dc = fwd_swap * daycount_adjust;
        let stdev = if normal_vol * option_term.sqrt() > from_f64(1e-5) {
            normal_vol * option_term.sqrt()
        } else {
            from_f64(1e-5)
        };

        let call_integral = support::integrate_option_price(
            option_price_fn,
            time_value_fn,
            true,
            fwd_vol_dc,
            stdev,
            daycount_adjust,
            self.params.integral_tolerance,
            self.params.time_value_tolerance,
        );

        let put_integral = support::integrate_option_price(
            option_price_fn,
            time_value_fn,
            false,
            fwd_vol_dc,
            stdev,
            daycount_adjust,
            self.params.integral_tolerance,
            self.params.time_value_tolerance,
        );

        let integral = two * (call_integral + put_integral);

        Ok(ratio * integral)
    }

    fn calc_swaplet_value(
        &self,
        ref_term: T,
        pay_freq: T,
        fwd_swap: T,
        lo_spread: T,
        effective_date_yf: T,
        first_payment_date_yf: T,
        pay_date_yf: T,
        annuity: T,
        discount_factor_pay: T,
        normal_vol: T,
        sln_vol: T,
        shift_size: T,
        option_term: T,
        daycount_adjust: T,
        option_price_fn: &dyn Fn(T, bool) -> T,
        time_value_fn: &dyn Fn(T) -> T,
    ) -> Result<T, ConvexityAdjustmentError> {
        match self.calc_method {
            ConvexityAdjustCalcMethod::NumericalIntegration => {
                let two: T = from_f64(2.0);

                if option_term <= T::zero() {
                    return Ok(fwd_swap);
                }

                let fwd_vol_dc = fwd_swap * daycount_adjust;
                let stdev = if normal_vol * option_term.sqrt() > from_f64(1e-5) {
                    normal_vol * option_term.sqrt()
                } else {
                    from_f64(1e-5)
                };

                let ratio = support::calc_log_numeraire_ratio_derivative(
                    ref_term,
                    pay_freq,
                    fwd_swap,
                    lo_spread,
                    effective_date_yf,
                    first_payment_date_yf,
                    pay_date_yf,
                );

                let call_integral = support::integrate_option_price(
                    option_price_fn,
                    time_value_fn,
                    true,
                    fwd_vol_dc,
                    stdev,
                    daycount_adjust,
                    self.params.integral_tolerance,
                    self.params.time_value_tolerance,
                );

                let put_integral = support::integrate_option_price(
                    option_price_fn,
                    time_value_fn,
                    false,
                    fwd_vol_dc,
                    stdev,
                    daycount_adjust,
                    self.params.integral_tolerance,
                    self.params.time_value_tolerance,
                );

                let integral = two * (call_integral + put_integral);

                Ok(fwd_swap + ratio * integral)
            }

            ConvexityAdjustCalcMethod::NormalAnalytic => {
                let delta =
                    (pay_date_yf - effective_date_yf) / (first_payment_date_yf - effective_date_yf);
                let nrd = support::numeraire_ratio_derivative(fwd_swap, delta, pay_freq, ref_term);
                let convexity_adjust =
                    nrd * annuity * normal_vol * normal_vol * option_term / discount_factor_pay;

                Ok(fwd_swap + convexity_adjust)
            }

            ConvexityAdjustCalcMethod::SlnAnalytic => {
                let delta =
                    (pay_date_yf - effective_date_yf) / (first_payment_date_yf - effective_date_yf);
                let nrd = support::numeraire_ratio_derivative(fwd_swap, delta, pay_freq, ref_term);
                let shifted_fwd = fwd_swap + shift_size;
                let sigma_sq_tau = sln_vol * sln_vol * option_term;
                // expm1(x) = exp(x) - 1
                let expm1_val = sigma_sq_tau.exp() - T::one();
                let convexity_adjust =
                    nrd * annuity * shifted_fwd * shifted_fwd * expm1_val / discount_factor_pay;

                Ok(fwd_swap + convexity_adjust)
            }
        }
    }

    fn calc_caplet_value(
        &self,
        ref_term: T,
        pay_freq: T,
        fwd_swap: T,
        strike: T,
        lo_spread: T,
        effective_date_yf: T,
        first_payment_date_yf: T,
        pay_date_yf: T,
        annuity: T,
        discount_factor_pay: T,
        normal_vol: T,
        sln_vol: T,
        shift_size: T,
        option_term: T,
        daycount_adjust: T,
        call_price_at_strike: T,
        option_price_fn: &dyn Fn(T, bool) -> T,
        time_value_fn: &dyn Fn(T) -> T,
    ) -> Result<T, ConvexityAdjustmentError> {
        match self.calc_method {
            ConvexityAdjustCalcMethod::NumericalIntegration => {
                let two: T = from_f64(2.0);

                let fwd_vol_dc = fwd_swap * daycount_adjust;
                let stk_vol_dc = strike * daycount_adjust;

                if option_term <= T::zero() {
                    // max(fwd - strike, 0)
                    let intrinsic = if fwd_swap > strike {
                        fwd_swap - strike
                    } else {
                        T::zero()
                    };
                    return Ok(intrinsic);
                }

                let stdev = if normal_vol * option_term.sqrt() > from_f64(1e-5) {
                    normal_vol * option_term.sqrt()
                } else {
                    from_f64(1e-5)
                };

                let ratio = support::calc_log_numeraire_ratio_derivative(
                    ref_term,
                    pay_freq,
                    fwd_swap,
                    lo_spread,
                    effective_date_yf,
                    first_payment_date_yf,
                    pay_date_yf,
                );

                let integral = if fwd_swap <= strike {
                    // integral = (K - F) * Call(K) + 2 * integrate_call(K -> +inf)
                    let call_at_stk = option_price_fn(stk_vol_dc, true) / daycount_adjust;
                    let base = (strike - fwd_swap) * call_at_stk;

                    let call_integral = support::integrate_option_price(
                        option_price_fn,
                        time_value_fn,
                        true,
                        stk_vol_dc,
                        stdev,
                        daycount_adjust,
                        self.params.integral_tolerance,
                        self.params.time_value_tolerance,
                    );

                    base + two * call_integral
                } else {
                    // integral = (K - F) * Put(K) + 2 * integrate_call(F -> +inf) + 2 *
                    // integrate_put(K, F)
                    let put_at_stk = option_price_fn(stk_vol_dc, false) / daycount_adjust;
                    let base = (strike - fwd_swap) * put_at_stk;

                    let call_integral = support::integrate_option_price(
                        option_price_fn,
                        time_value_fn,
                        true,
                        fwd_vol_dc,
                        stdev,
                        daycount_adjust,
                        self.params.integral_tolerance,
                        self.params.time_value_tolerance,
                    );

                    let put_integral = support::integrate_put_price(
                        option_price_fn,
                        stk_vol_dc,
                        fwd_vol_dc,
                        daycount_adjust,
                    );

                    base + two * call_integral + two * put_integral
                };

                Ok(call_price_at_strike + ratio * integral)
            }

            ConvexityAdjustCalcMethod::NormalAnalytic => {
                let delta =
                    (pay_date_yf - effective_date_yf) / (first_payment_date_yf - effective_date_yf);
                let nrd = support::numeraire_ratio_derivative(fwd_swap, delta, pay_freq, ref_term);
                let sqrt_tau = option_term.sqrt();
                let d = (fwd_swap - strike) / (normal_vol * sqrt_tau);
                let convexity_adjust = nrd * annuity * normal_vol * normal_vol * option_term
                    / discount_factor_pay
                    * norm_cdf(d);

                Ok(call_price_at_strike + convexity_adjust)
            }

            ConvexityAdjustCalcMethod::SlnAnalytic => {
                let delta =
                    (pay_date_yf - effective_date_yf) / (first_payment_date_yf - effective_date_yf);
                let nrd = support::numeraire_ratio_derivative(fwd_swap, delta, pay_freq, ref_term);

                let shifted_fwd = fwd_swap + shift_size;
                let shifted_strike = strike + shift_size;
                let sigma_sq_tau = sln_vol * sln_vol * option_term;
                let sqrt_tau = option_term.sqrt();

                let d = |x: T| -> T {
                    ((shifted_fwd / shifted_strike).ln() + x * sigma_sq_tau) / (sln_vol * sqrt_tau)
                };

                let convexity_adjust = nrd * annuity / discount_factor_pay
                    * (shifted_fwd * shifted_fwd * sigma_sq_tau.exp() * norm_cdf(d(from_f64(1.5)))
                        - shifted_fwd
                            * (shifted_fwd + shifted_strike)
                            * norm_cdf(d(from_f64(0.5)))
                        + shifted_fwd * shifted_strike * norm_cdf(d(from_f64(-0.5))));

                Ok(call_price_at_strike + convexity_adjust)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn does_apply_within_grace_period() {
        let adj = ConundrumIntegrandConvexityAdjuster::<f64>::default_numerical();
        // 14 days ~ 0.0384 years; end=10.0, pay=10.02 is within grace
        assert!(!adj.does_apply(10.0, 10.02));
    }

    #[test]
    fn does_apply_outside_grace_period() {
        let adj = ConundrumIntegrandConvexityAdjuster::<f64>::default_numerical();
        // end=10.0, pay=10.5 is well outside 14 days
        assert!(adj.does_apply(10.0, 10.5));
    }

    #[test]
    fn normal_analytic_positive_convexity() {
        let adj = ConundrumIntegrandConvexityAdjuster::<f64>::default_normal_analytic();
        let dummy_price_fn = |_k: f64, _is_call: bool| 0.0;
        let dummy_tv_fn = |_k: f64| 0.0;

        // CMS 10Y SA swap: effective=0, first_pay=0.5, CMS coupon pay=1.0
        // delta = (1.0 - 0.0) / (0.5 - 0.0) = 2.0 (typical for CMS)
        let result = adj
            .calc_swaplet_value(
                10.0,  // ref_term (10Y underlying)
                2.0,   // pay_freq (SA)
                0.03,  // fwd_swap
                0.0,   // lo_spread
                0.0,   // effective_date_yf
                0.5,   // first_payment_date_yf
                1.0,   // pay_date_yf (CMS coupon pays ~1Y from effective)
                9.5,   // annuity
                0.97,  // discount_factor_pay
                0.005, // normal_vol (50bps)
                0.0,   // sln_vol (unused)
                0.0,   // shift_size (unused)
                1.0,   // option_term
                1.0,   // daycount_adjust
                &dummy_price_fn,
                &dummy_tv_fn,
            )
            .unwrap();

        // Convexity adjustment should be positive: result > forward
        assert!(result > 0.03, "CMS swaplet should exceed the forward rate");
        // And the adjustment should be small but meaningful
        assert!(result < 0.04, "Adjustment should be reasonable");
    }

    #[test]
    fn sln_analytic_positive_convexity() {
        let adj = ConundrumIntegrandConvexityAdjuster::<f64>::default_sln_analytic();
        let dummy_price_fn = |_k: f64, _is_call: bool| 0.0;
        let dummy_tv_fn = |_k: f64| 0.0;

        // CMS coupon pay_date_yf=1.0 => delta=2.0
        let result = adj
            .calc_swaplet_value(
                10.0,
                2.0,
                0.03,
                0.0,
                0.0,
                0.5,
                1.0,
                9.5,
                0.97,
                0.0,
                0.20,
                0.0,
                1.0,
                1.0,
                &dummy_price_fn,
                &dummy_tv_fn,
            )
            .unwrap();

        assert!(result > 0.03, "SLN swaplet should exceed forward");
    }

    #[test]
    fn sln_converges_to_normal_for_small_vol() {
        let normal_adj = ConundrumIntegrandConvexityAdjuster::<f64>::default_normal_analytic();
        let sln_adj = ConundrumIntegrandConvexityAdjuster::<f64>::default_sln_analytic();
        let dummy_price_fn = |_k: f64, _is_call: bool| 0.0;
        let dummy_tv_fn = |_k: f64| 0.0;

        let fwd = 0.03;
        let normal_vol = 0.003; // small vol
                                // For SLN with shift=0: sigma_SLN * F ≈ sigma_N => sigma_SLN ≈ sigma_N / F
        let sln_vol = normal_vol / fwd;

        // pay_date_yf=1.0 => delta=2.0
        let normal_result = normal_adj
            .calc_swaplet_value(
                10.0,
                2.0,
                fwd,
                0.0,
                0.0,
                0.5,
                1.0,
                9.5,
                0.97,
                normal_vol,
                0.0,
                0.0,
                1.0,
                1.0,
                &dummy_price_fn,
                &dummy_tv_fn,
            )
            .unwrap();

        let sln_result = sln_adj
            .calc_swaplet_value(
                10.0,
                2.0,
                fwd,
                0.0,
                0.0,
                0.5,
                1.0,
                9.5,
                0.97,
                0.0,
                sln_vol,
                0.0,
                1.0,
                1.0,
                &dummy_price_fn,
                &dummy_tv_fn,
            )
            .unwrap();

        // For small vol, Normal and SLN should give close results
        assert_relative_eq!(normal_result, sln_result, epsilon = 1e-5);
    }

    #[test]
    fn caplet_normal_analytic_basic() {
        let adj = ConundrumIntegrandConvexityAdjuster::<f64>::default_normal_analytic();
        let dummy_price_fn = |_k: f64, _is_call: bool| 0.0;
        let dummy_tv_fn = |_k: f64| 0.0;

        let call_price = 0.005; // intrinsic call price
                                // pay_date_yf=1.0 => delta=2.0
        let result = adj
            .calc_caplet_value(
                10.0,
                2.0,
                0.03,
                0.025,
                0.0,
                0.0,
                0.5,
                1.0,
                9.5,
                0.97,
                0.005,
                0.0,
                0.0,
                1.0,
                1.0,
                call_price,
                &dummy_price_fn,
                &dummy_tv_fn,
            )
            .unwrap();

        // Caplet value >= call price (convexity adds value)
        assert!(result >= call_price);
    }

    #[test]
    fn zero_option_term_returns_forward() {
        let adj = ConundrumIntegrandConvexityAdjuster::<f64>::default_numerical();
        let dummy_price_fn = |_k: f64, _is_call: bool| 0.0;
        let dummy_tv_fn = |_k: f64| 0.0;

        let result = adj
            .calc_swaplet_value(
                10.0,
                2.0,
                0.03,
                0.0,
                0.0,
                0.5,
                10.5,
                9.5,
                0.97,
                0.005,
                0.0,
                0.0,
                0.0, // option_term = 0
                1.0,
                &dummy_price_fn,
                &dummy_tv_fn,
            )
            .unwrap();

        assert_eq!(result, 0.03);
    }
}
