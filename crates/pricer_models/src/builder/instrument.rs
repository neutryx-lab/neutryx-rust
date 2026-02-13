//! Calibration instrument trait for global curve calibration.
//!
//! This module provides the `CalibrationInstrument<T>` trait that abstracts
//! market instruments for use in global curve calibration. Unlike sequential
//! bootstrapping which solves one discount factor at a time, global calibration
//! solves all discount factors simultaneously.

use num_traits::Float;
use pricer_core::math::numeric::{from_f64, from_usize};

use crate::market::{
    curves::{Frequency, MarketInstrument, YieldCurve},
    MarketDataError,
};

/// Trait for instruments used in global curve calibration.
pub trait CalibrationInstrument<T: Float>: Clone {
    /// Returns the market-quoted rate for this instrument.
    fn market_rate(&self) -> T;

    /// Computes the theoretical rate implied by the given yield curve.
    fn theoretical_rate<C: YieldCurve<T>>(&self, curve: &C) -> Result<T, MarketDataError>;

    /// Returns the instrument's maturity in years from today.
    fn maturity(&self) -> T;

    /// Computes the pricing error: theoretical_rate - market_rate.
    fn pricing_error<C: YieldCurve<T>>(&self, curve: &C) -> Result<T, MarketDataError> {
        Ok(self.theoretical_rate(curve)? - self.market_rate())
    }

    /// Returns a descriptive name for the instrument type.
    fn instrument_type(&self) -> &'static str;
}

impl<T: Float> CalibrationInstrument<T> for MarketInstrument<T> {
    fn market_rate(&self) -> T { self.rate() }

    fn theoretical_rate<C: YieldCurve<T>>(&self, curve: &C) -> Result<T, MarketDataError> {
        match self {
            Self::Ois {
                maturity,
                payment_frequency,
                ..
            } => compute_ois_par_rate::<T, C>(*maturity, *payment_frequency, curve),

            Self::Irs {
                maturity,
                fixed_frequency,
                ..
            } => compute_irs_par_rate::<T, C>(*maturity, *fixed_frequency, curve),

            Self::Fra { start, end, .. } => compute_fra_rate::<T, C>(*start, *end, curve),

            Self::Future {
                maturity,
                convexity_adjustment,
                ..
            } => {
                let fra_rate = compute_fra_rate::<T, C>(T::zero(), *maturity, curve)?;
                Ok(fra_rate + *convexity_adjustment)
            }

            Self::Event { maturity, .. } => compute_event_jump::<T, C>(*maturity, curve),

            Self::Bond {
                maturity,
                coupon_rate,
                payment_frequency,
                ..
            } => compute_bond_ytm::<T, C>(*maturity, *coupon_rate, *payment_frequency, curve),

            Self::Cds {
                maturity,
                recovery_rate,
                risk_free_dfs,
                ..
            } => compute_cds_par_spread::<T, C>(*maturity, *recovery_rate, risk_free_dfs, curve),
        }
    }

    fn maturity(&self) -> T { MarketInstrument::maturity(self) }

    fn instrument_type(&self) -> &'static str { MarketInstrument::instrument_type(self) }
}

/// Compute the OIS par swap rate from a yield curve.
fn compute_ois_par_rate<T: Float, C: YieldCurve<T>>(
    maturity: T,
    frequency: Frequency,
    curve: &C,
) -> Result<T, MarketDataError> {
    let dt = frequency.period_years::<T>();
    let num_periods = (maturity / dt).ceil().to_usize().unwrap_or(1).max(1);

    let df_maturity = curve.discount_factor(maturity)?;

    if num_periods == 1 {
        Ok((T::one() / df_maturity - T::one()) / maturity)
    } else {
        let mut annuity = T::zero();
        for i in 1..num_periods {
            let t_i = dt * from_usize::<T>(i);
            if t_i < maturity {
                annuity = annuity + curve.discount_factor(t_i)? * dt;
            }
        }
        let final_dt = maturity - dt * from_usize::<T>(num_periods - 1);
        annuity = annuity + df_maturity * final_dt;

        if annuity > T::zero() {
            Ok((T::one() - df_maturity) / annuity)
        } else {
            Err(MarketDataError::InterpolationFailed {
                reason: "annuity is zero or negative".to_string(),
            })
        }
    }
}

/// Compute the IRS par swap rate from a yield curve.
fn compute_irs_par_rate<T: Float, C: YieldCurve<T>>(
    maturity: T,
    fixed_frequency: Frequency,
    curve: &C,
) -> Result<T, MarketDataError> {
    let dt = fixed_frequency.period_years::<T>();
    let num_periods = (maturity / dt).ceil().to_usize().unwrap_or(1).max(1);

    let df_maturity = curve.discount_factor(maturity)?;

    let mut annuity = T::zero();
    for i in 1..num_periods {
        let t_i = dt * from_usize::<T>(i);
        if t_i < maturity {
            annuity = annuity + curve.discount_factor(t_i)? * dt;
        }
    }
    let final_dt = maturity - dt * from_usize::<T>(num_periods - 1);
    annuity = annuity + df_maturity * final_dt;

    if annuity > T::zero() {
        Ok((T::one() - df_maturity) / annuity)
    } else {
        Err(MarketDataError::InterpolationFailed {
            reason: "IRS annuity is zero or negative".to_string(),
        })
    }
}

/// Compute the FRA (forward) rate from a yield curve.
fn compute_fra_rate<T: Float, C: YieldCurve<T>>(
    start: T,
    end: T,
    curve: &C,
) -> Result<T, MarketDataError> {
    let df_start = if start <= T::zero() {
        T::one()
    } else {
        curve.discount_factor(start)?
    };
    let df_end = curve.discount_factor(end)?;
    let tau = end - start;

    if tau <= T::zero() {
        return Err(MarketDataError::InterpolationFailed {
            reason: "FRA period must be positive".to_string(),
        });
    }

    if df_end <= T::zero() {
        return Err(MarketDataError::InterpolationFailed {
            reason: "discount factor must be positive".to_string(),
        });
    }

    Ok((df_start / df_end - T::one()) / tau)
}

/// Compute the instantaneous forward rate jump at an event date.
///
/// For central bank meetings and scheduled events, we measure the
/// difference in instantaneous forward rates just before and after
/// the event time. This is computed as:
///
///   jump = f(t+ε) - f(t-ε)
///
/// where f(t) is the instantaneous forward rate at time t.
fn compute_event_jump<T: Float, C: YieldCurve<T>>(
    maturity: T,
    curve: &C,
) -> Result<T, MarketDataError> {
    use pricer_core::math::numeric::from_f64;

    // Small time step for numerical differentiation
    let dt = from_f64::<T>(1e-5);

    // Ensure we don't go negative for very short maturities
    let t_before = if maturity > dt {
        maturity - dt
    } else {
        T::zero()
    };
    let t_after = maturity + dt;

    // Compute forward rates just before and after the event
    let rate_before = curve.forward_rate(t_before, maturity)?;
    let rate_after = curve.forward_rate(maturity, t_after)?;

    Ok(rate_after - rate_before)
}

/// Compute the CDS par spread from a survival probability curve.
///
/// Uses the ISDA Standard Model with quarterly premium payments:
///   Protection Leg PV = (1-R) × Σ [DF_rf(t_i) × (SP(t_{i-1}) - SP(t_i))]
///   Risky Annuity     = Σ [DF_rf(t_i) × SP(t_i) × Δt_i]
///   Par Spread        = Protection Leg PV / Risky Annuity
///
/// `curve` is the survival probability curve being calibrated
/// (discount_factor = survival_probability), and `risk_free_dfs`
/// contains pre-sampled risk-free discount factors.
fn compute_cds_par_spread<T: Float, C: YieldCurve<T>>(
    maturity: T,
    recovery_rate: T,
    risk_free_dfs: &[(T, T)],
    curve: &C,
) -> Result<T, MarketDataError> {
    let one = T::one();
    let loss_given_default = one - recovery_rate;
    let quarter: T = from_f64(0.25);

    let n_periods = (maturity / quarter).ceil().to_usize().unwrap_or(1).max(1);

    let mut protection_leg = T::zero();
    let mut risky_annuity = T::zero();
    let mut prev_sp = one; // SP(0) = 1

    for i in 1..=n_periods {
        let t_i = {
            let t = quarter * from_usize::<T>(i);
            if t > maturity { maturity } else { t }
        };

        let sp_i = curve.discount_factor(t_i)?;
        let df_rf_i = interpolate_rf_df(t_i, risk_free_dfs);

        let dt = if i == 1 {
            t_i
        } else {
            let t_prev = quarter * from_usize::<T>(i - 1);
            if t_i > t_prev { t_i - t_prev } else { quarter }
        };

        // Protection leg: (1-R) × DF_rf × (SP_{i-1} - SP_i)
        protection_leg = protection_leg + loss_given_default * df_rf_i * (prev_sp - sp_i);

        // Premium leg (risky annuity): DF_rf × SP × dt
        risky_annuity = risky_annuity + df_rf_i * sp_i * dt;

        prev_sp = sp_i;
    }

    if risky_annuity <= T::zero() {
        return Err(MarketDataError::InterpolationFailed {
            reason: "CDS risky annuity is zero or negative".to_string(),
        });
    }

    Ok(protection_leg / risky_annuity)
}

/// Log-linear interpolation of risk-free discount factors from pre-sampled pairs.
fn interpolate_rf_df<T: Float>(t: T, dfs: &[(T, T)]) -> T {
    if dfs.is_empty() {
        return T::one();
    }
    if t <= dfs[0].0 {
        // Extrapolate left using log-linear from origin (DF(0)=1)
        let log_rate = -dfs[0].1.ln() / dfs[0].0;
        return (-log_rate * t).exp();
    }
    if t >= dfs[dfs.len() - 1].0 {
        return dfs[dfs.len() - 1].1;
    }

    for i in 0..dfs.len() - 1 {
        if t >= dfs[i].0 && t <= dfs[i + 1].0 {
            let w = (t - dfs[i].0) / (dfs[i + 1].0 - dfs[i].0);
            let log_df = dfs[i].1.ln() * (T::one() - w) + dfs[i + 1].1.ln() * w;
            return log_df.exp();
        }
    }
    dfs[dfs.len() - 1].1
}

/// Compute the bond yield-to-maturity implied by a yield curve.
///
/// 1. Price the bond using curve discount factors:
///    `P = Σ(c × τ_i × DF(t_i)) + DF(T)`
/// 2. Solve for YTM `y` via Newton-Raphson:
///    `P(y) = Σ(c × τ_i × exp(-y × t_i)) + exp(-y × T)`
fn compute_bond_ytm<T: Float, C: YieldCurve<T>>(
    maturity: T,
    coupon_rate: T,
    frequency: Frequency,
    curve: &C,
) -> Result<T, MarketDataError> {
    use pricer_core::math::numeric::from_f64;

    let dt = frequency.period_years::<T>();
    let num_periods = (maturity / dt).ceil().to_usize().unwrap_or(1).max(1);

    // Collect cashflow schedule: (time, year_fraction, includes_principal)
    let mut cashflows: Vec<(T, T, bool)> = Vec::with_capacity(num_periods);
    for i in 1..=num_periods {
        let t_i = dt * from_usize::<T>(i);
        let t = if t_i > maturity { maturity } else { t_i };
        let tau = if i == 1 {
            t
        } else if i == num_periods {
            maturity - dt * from_usize::<T>(num_periods - 1)
        } else {
            dt
        };
        cashflows.push((t, tau, i == num_periods));
    }

    // Step 1: Dirty price from curve discount factors
    let mut dirty_price = T::zero();
    for &(t, tau, is_final) in &cashflows {
        let df = curve.discount_factor(t)?;
        let coupon_cf = coupon_rate * tau;
        if is_final {
            dirty_price = dirty_price + (coupon_cf + T::one()) * df;
        } else {
            dirty_price = dirty_price + coupon_cf * df;
        }
    }

    // Step 2: Newton-Raphson to convert price → YTM
    // Initial guess: use coupon rate as starting point
    let mut ytm = coupon_rate;
    if ytm <= T::zero() {
        ytm = from_f64::<T>(0.01);
    }
    let max_iter = 50;
    let tol = from_f64::<T>(1e-12);

    for _ in 0..max_iter {
        let mut p = T::zero();
        let mut dp = T::zero();
        for &(t, tau, is_final) in &cashflows {
            let disc = (-ytm * t).exp();
            let coupon_cf = coupon_rate * tau;
            if is_final {
                let cf = coupon_cf + T::one();
                p = p + cf * disc;
                dp = dp - cf * t * disc;
            } else {
                p = p + coupon_cf * disc;
                dp = dp - coupon_cf * t * disc;
            }
        }

        let f_val = p - dirty_price;
        if f_val.abs() < tol {
            return Ok(ytm);
        }
        if dp.abs() < from_f64::<T>(1e-30) {
            return Err(MarketDataError::InterpolationFailed {
                reason: "Bond YTM derivative near zero".to_string(),
            });
        }
        ytm = ytm - f_val / dp;
        if !ytm.is_finite() {
            return Err(MarketDataError::InterpolationFailed {
                reason: "Bond YTM iteration produced non-finite value".to_string(),
            });
        }
    }

    Err(MarketDataError::InterpolationFailed {
        reason: "Bond YTM solver did not converge".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::market::curves::{BootstrapInterpolation, BootstrappedCurve};

    fn create_test_curve() -> BootstrappedCurve<f64> {
        let pillars = vec![0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
        let discount_factors: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        BootstrappedCurve::new(
            pillars,
            discount_factors,
            BootstrapInterpolation::LogLinear,
            true,
        )
        .unwrap()
    }

    #[test]
    fn test_calibration_instrument_market_rate() {
        let ois: MarketInstrument<f64> = MarketInstrument::ois(5.0, 0.03);
        assert_relative_eq!(ois.market_rate(), 0.03, epsilon = 1e-10);

        let irs: MarketInstrument<f64> = MarketInstrument::irs(10.0, 0.035);
        assert_relative_eq!(irs.market_rate(), 0.035, epsilon = 1e-10);
    }

    #[test]
    fn test_calibration_instrument_maturity() {
        let ois: MarketInstrument<f64> = MarketInstrument::ois(5.0, 0.03);
        assert_relative_eq!(CalibrationInstrument::maturity(&ois), 5.0, epsilon = 1e-10);

        let fra: MarketInstrument<f64> = MarketInstrument::fra(0.5, 1.0, 0.025);
        assert_relative_eq!(CalibrationInstrument::maturity(&fra), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_calibration_instrument_type() {
        let ois: MarketInstrument<f64> = MarketInstrument::ois(5.0, 0.03);
        assert_eq!(CalibrationInstrument::instrument_type(&ois), "OIS");

        let irs: MarketInstrument<f64> = MarketInstrument::irs(10.0, 0.035);
        assert_eq!(CalibrationInstrument::instrument_type(&irs), "IRS");
    }

    #[test]
    fn test_theoretical_rate_ois() {
        let curve = create_test_curve();
        let ois: MarketInstrument<f64> = MarketInstrument::ois(1.0, 0.03);
        let theoretical = ois.theoretical_rate(&curve).unwrap();

        assert_relative_eq!(theoretical, 0.0305, epsilon = 1e-3);
        assert!(theoretical > 0.03);
    }

    #[test]
    fn test_theoretical_rate_fra() {
        let curve = create_test_curve();
        let fra: MarketInstrument<f64> = MarketInstrument::fra(0.5, 1.0, 0.03);
        let theoretical = fra.theoretical_rate(&curve).unwrap();

        assert_relative_eq!(theoretical, 0.03, epsilon = 5e-3);
    }

    #[test]
    fn test_pricing_error() {
        let curve = create_test_curve();

        let ois: MarketInstrument<f64> = MarketInstrument::ois(1.0, 0.03);
        let error = ois.pricing_error(&curve).unwrap();
        assert!(error.abs() < 0.01, "expected small error, got {}", error);

        let ois_higher: MarketInstrument<f64> = MarketInstrument::ois(1.0, 0.04);
        let error_higher = ois_higher.pricing_error(&curve).unwrap();
        assert!(error_higher < 0.0, "expected negative error");
    }

    #[test]
    fn test_compute_fra_rate_zero_start() {
        let curve = create_test_curve();
        let rate = compute_fra_rate::<f64, _>(0.0, 1.0, &curve).unwrap();
        assert_relative_eq!(rate, 0.03, epsilon = 1e-3);
    }

    #[test]
    fn test_calibration_instrument_clone() {
        let ois: MarketInstrument<f64> = MarketInstrument::ois(5.0, 0.03);
        let cloned = ois.clone();
        assert_relative_eq!(ois.market_rate(), cloned.market_rate(), epsilon = 1e-15);
    }

    #[test]
    fn test_cds_theoretical_spread() {
        // Flat survival probability curve: SP(t) = exp(-h*t) with h = 200bp
        let hazard_rate = 0.02;
        let recovery = 0.40;
        let pillars = vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
        let sps: Vec<f64> = pillars.iter().map(|&t| (-hazard_rate * t).exp()).collect();
        let sp_curve = BootstrappedCurve::new(
            pillars,
            sps,
            BootstrapInterpolation::LogLinear,
            true,
        )
        .unwrap();

        // Pre-sample risk-free DFs at 3% flat
        let rf_dfs: Vec<(f64, f64)> = (1..=40)
            .map(|i| {
                let t = 0.25 * i as f64;
                (t, (-0.03 * t).exp())
            })
            .collect();

        let cds_5y: MarketInstrument<f64> =
            MarketInstrument::cds(5.0, 0.012, recovery, rf_dfs.clone());
        let spread = cds_5y.theoretical_rate(&sp_curve).unwrap();

        // For flat hazard rate: CDS spread ≈ (1-R) * h = 0.6 * 0.02 = 0.012
        assert_relative_eq!(spread, 0.012, epsilon = 2e-3);
    }

    #[test]
    fn test_cds_instrument_type_and_maturity() {
        let cds: MarketInstrument<f64> = MarketInstrument::cds(5.0, 0.01, 0.40, vec![]);
        assert_eq!(CalibrationInstrument::instrument_type(&cds), "CDS");
        assert_relative_eq!(CalibrationInstrument::maturity(&cds), 5.0, epsilon = 1e-10);
        assert_relative_eq!(cds.market_rate(), 0.01, epsilon = 1e-10);
    }

    #[test]
    fn test_interpolate_rf_df_basic() {
        let dfs = vec![(1.0, 0.97_f64), (2.0, 0.94_f64)];
        // At t=1.0, should return exactly 0.97
        let df1 = interpolate_rf_df(1.0, &dfs);
        assert_relative_eq!(df1, 0.97, epsilon = 1e-10);
        // At t=2.0, should return exactly 0.94
        let df2 = interpolate_rf_df(2.0, &dfs);
        assert_relative_eq!(df2, 0.94, epsilon = 1e-10);
        // At t=1.5, log-linear interpolation
        let df15 = interpolate_rf_df(1.5, &dfs);
        let expected = (0.97_f64.ln() * 0.5 + 0.94_f64.ln() * 0.5).exp();
        assert_relative_eq!(df15, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_theoretical_rate_bond_par() {
        let curve = create_test_curve();
        // Par bond: coupon ≈ YTM on a flat 3% curve
        let bond: MarketInstrument<f64> = MarketInstrument::bond(5.0, 0.03, 0.03);
        let theoretical = bond.theoretical_rate(&curve).unwrap();
        // Implied YTM should be close to 3%
        assert_relative_eq!(theoretical, 0.03, epsilon = 5e-3);
    }

    #[test]
    fn test_theoretical_rate_bond_premium() {
        let curve = create_test_curve();
        // Premium bond: high coupon → trades above par → implied YTM < coupon
        let bond: MarketInstrument<f64> = MarketInstrument::bond(5.0, 0.04, 0.05);
        let theoretical = bond.theoretical_rate(&curve).unwrap();
        assert!(
            theoretical < 0.05,
            "premium bond YTM should be below coupon, got {}",
            theoretical
        );
    }

    #[test]
    fn test_theoretical_rate_bond_discount() {
        let curve = create_test_curve();
        // Discount bond: low coupon → trades below par → implied YTM > coupon
        let bond: MarketInstrument<f64> = MarketInstrument::bond(5.0, 0.02, 0.01);
        let theoretical = bond.theoretical_rate(&curve).unwrap();
        assert!(
            theoretical > 0.01,
            "discount bond YTM should be above coupon, got {}",
            theoretical
        );
    }

    #[test]
    fn test_bond_instrument_type() {
        let bond: MarketInstrument<f64> = MarketInstrument::bond(10.0, 0.04, 0.035);
        assert_eq!(CalibrationInstrument::instrument_type(&bond), "Bond");
        assert_relative_eq!(CalibrationInstrument::maturity(&bond), 10.0, epsilon = 1e-10);
        assert_relative_eq!(CalibrationInstrument::market_rate(&bond), 0.04, epsilon = 1e-10);
    }
}
