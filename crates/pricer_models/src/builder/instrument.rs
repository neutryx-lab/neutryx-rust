//! Calibration instrument trait for global curve calibration.
//!
//! This module provides the `CalibrationInstrument<T>` trait that abstracts
//! market instruments for use in global curve calibration. Unlike sequential
//! bootstrapping which solves one discount factor at a time, global calibration
//! solves all discount factors simultaneously.

use num_traits::Float;
use pricer_core::math::numeric::from_usize;

use crate::market::{
    curves::{Frequency, MarketInstrument, YieldCurve},
    MarketDataError,
};

/// Trait for instruments used in global curve calibration.
///
/// This trait provides a uniform interface for computing pricing errors
/// across different instrument types, enabling global optimisation approaches
/// where all discount factors are solved simultaneously.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
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

// =============================================================================
// Implementation for MarketInstrument
// =============================================================================

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
        }
    }

    fn maturity(&self) -> T { MarketInstrument::maturity(self) }

    fn instrument_type(&self) -> &'static str { MarketInstrument::instrument_type(self) }
}

// =============================================================================
// Helper Functions for Rate Computation
// =============================================================================

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
    let t_before = if maturity > dt { maturity - dt } else { T::zero() };
    let t_after = maturity + dt;

    // Compute forward rates just before and after the event
    let rate_before = curve.forward_rate(t_before, maturity)?;
    let rate_after = curve.forward_rate(maturity, t_after)?;

    Ok(rate_after - rate_before)
}

// =============================================================================
// Tests
// =============================================================================

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
}
