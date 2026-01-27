//! Calibration instrument trait for global curve calibration.
//!
//! This module provides the `CalibrationInstrument<T>` trait that abstracts
//! market instruments for use in global curve calibration. Unlike sequential
//! bootstrapping which solves one discount factor at a time, global calibration
//! solves all discount factors simultaneously.
//!
//! ## Key Concepts
//!
//! - **Market Rate**: The observed market quote for the instrument
//! - **Theoretical Rate**: The rate implied by a given yield curve
//! - **Pricing Error**: Difference between theoretical and market rates
//!
//! ## Usage
//!
//! The trait is implemented for `BootstrapInstrument<T>` and can be used
//! with the `GlobalBootstrapper` for simultaneous curve calibration.
//!
//! ```ignore
//! use pricer_models::market::calibration::bootstrapping::CalibrationInstrument;
//!
//! let instrument = BootstrapInstrument::ois(5.0, 0.03);
//! let market_rate = instrument.market_rate();
//! let theoretical = instrument.theoretical_rate(&curve)?;
//! let error = instrument.pricing_error(&curve)?;
//! ```

use num_traits::Float;

use crate::market::curves::YieldCurve;
use crate::market::error::MarketDataError;

use super::instrument::BootstrapInstrument;
use super::Frequency;
use pricer_core::math::numeric::{from_f64, from_usize};

/// Trait for instruments used in global curve calibration.
///
/// This trait provides a uniform interface for computing pricing errors
/// across different instrument types, enabling global optimisation approaches
/// where all discount factors are solved simultaneously.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
///
/// # Required Methods
///
/// - [`market_rate`](Self::market_rate): Returns the market-quoted rate
/// - [`theoretical_rate`](Self::theoretical_rate): Computes implied rate from curve
/// - [`maturity`](Self::maturity): Returns the instrument's maturity
///
/// # Provided Methods
///
/// - [`pricing_error`](Self::pricing_error): Returns theoretical - market rate
///
/// # Example
///
/// ```ignore
/// use pricer_models::market::calibration::bootstrapping::{
///     CalibrationInstrument, BootstrapInstrument,
/// };
///
/// let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
///
/// // Get market rate
/// assert!((ois.market_rate() - 0.03).abs() < 1e-10);
///
/// // Compute theoretical rate from a curve
/// let theoretical = ois.theoretical_rate(&curve)?;
///
/// // Compute pricing error
/// let error = ois.pricing_error(&curve)?; // theoretical - market
/// ```
pub trait CalibrationInstrument<T: Float>: Clone {
    /// Returns the market-quoted rate for this instrument.
    ///
    /// For OIS/IRS, this is the fixed rate. For futures, this is
    /// derived from the price (100 - price) / 100.
    fn market_rate(&self) -> T;

    /// Computes the theoretical rate implied by the given yield curve.
    ///
    /// # Arguments
    ///
    /// * `curve` - The yield curve to evaluate against
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - The implied rate from the curve
    /// * `Err(MarketDataError)` - If the curve cannot be evaluated
    fn theoretical_rate<C: YieldCurve<T>>(&self, curve: &C) -> Result<T, MarketDataError>;

    /// Returns the instrument's maturity in years from today.
    fn maturity(&self) -> T;

    /// Computes the pricing error: theoretical_rate - market_rate.
    ///
    /// This is the residual that global calibration seeks to minimise.
    ///
    /// # Arguments
    ///
    /// * `curve` - The yield curve to evaluate against
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - The pricing error (positive means curve implies higher rate)
    /// * `Err(MarketDataError)` - If the curve cannot be evaluated
    fn pricing_error<C: YieldCurve<T>>(&self, curve: &C) -> Result<T, MarketDataError> {
        Ok(self.theoretical_rate(curve)? - self.market_rate())
    }

    /// Returns a descriptive name for the instrument type.
    fn instrument_type(&self) -> &'static str;
}

// =============================================================================
// Implementation for BootstrapInstrument
// =============================================================================

impl<T: Float> CalibrationInstrument<T> for BootstrapInstrument<T> {
    fn market_rate(&self) -> T {
        self.rate()
    }

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
                // Future rate is essentially a forward rate with convexity adjustment
                let fra_rate = compute_fra_rate::<T, C>(T::zero(), *maturity, curve)?;
                Ok(fra_rate + *convexity_adjustment)
            }
        }
    }

    fn maturity(&self) -> T {
        BootstrapInstrument::maturity(self)
    }

    fn instrument_type(&self) -> &'static str {
        BootstrapInstrument::instrument_type(self)
    }
}

// =============================================================================
// Helper Functions for Rate Computation
// =============================================================================

/// Compute the OIS par swap rate from a yield curve.
///
/// For a single period: rate = (1/DF(T) - 1) / T
/// For multiple periods: rate = (1 - DF(T)) / Annuity
fn compute_ois_par_rate<T: Float, C: YieldCurve<T>>(
    maturity: T,
    frequency: Frequency,
    curve: &C,
) -> Result<T, MarketDataError> {
    let dt = frequency.period_years::<T>();
    let num_periods = (maturity / dt).ceil().to_usize().unwrap_or(1).max(1);

    let df_maturity = curve.discount_factor(maturity)?;

    if num_periods == 1 {
        // Single period: rate = (1/DF - 1) / T
        Ok((T::one() / df_maturity - T::one()) / maturity)
    } else {
        // Multi-period: rate = (1 - DF(T)) / Annuity
        let mut annuity = T::zero();
        for i in 1..num_periods {
            let t_i = dt * from_usize::<T>(i);
            if t_i < maturity {
                annuity = annuity + curve.discount_factor(t_i)? * dt;
            }
        }
        // Add final period with adjusted dt
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
///
/// rate = (1 - DF(T)) / Annuity
fn compute_irs_par_rate<T: Float, C: YieldCurve<T>>(
    maturity: T,
    fixed_frequency: Frequency,
    curve: &C,
) -> Result<T, MarketDataError> {
    let dt = fixed_frequency.period_years::<T>();
    let num_periods = (maturity / dt).ceil().to_usize().unwrap_or(1).max(1);

    let df_maturity = curve.discount_factor(maturity)?;

    // Compute annuity
    let mut annuity = T::zero();
    for i in 1..num_periods {
        let t_i = dt * from_usize::<T>(i);
        if t_i < maturity {
            annuity = annuity + curve.discount_factor(t_i)? * dt;
        }
    }
    // Add final period
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
///
/// forward_rate = (DF(start) / DF(end) - 1) / tau
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::calibration::bootstrapping::{
        BootstrapInterpolation, BootstrappedCurve,
    };
    use approx::assert_relative_eq;

    fn create_test_curve() -> BootstrappedCurve<f64> {
        // Create a simple flat 3% curve
        let pillars = vec![0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
        let discount_factors: Vec<f64> = pillars
            .iter()
            .map(|&t| (-0.03 * t).exp())
            .collect();

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
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
        assert_relative_eq!(ois.market_rate(), 0.03, epsilon = 1e-10);

        let irs: BootstrapInstrument<f64> = BootstrapInstrument::irs(10.0, 0.035);
        assert_relative_eq!(irs.market_rate(), 0.035, epsilon = 1e-10);
    }

    #[test]
    fn test_calibration_instrument_maturity() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
        assert_relative_eq!(ois.maturity(), 5.0, epsilon = 1e-10);

        let fra: BootstrapInstrument<f64> = BootstrapInstrument::fra(0.5, 1.0, 0.025);
        assert_relative_eq!(fra.maturity(), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_calibration_instrument_type() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
        assert_eq!(
            CalibrationInstrument::instrument_type(&ois),
            "OIS"
        );

        let irs: BootstrapInstrument<f64> = BootstrapInstrument::irs(10.0, 0.035);
        assert_eq!(
            CalibrationInstrument::instrument_type(&irs),
            "IRS"
        );
    }

    #[test]
    fn test_theoretical_rate_ois() {
        let curve = create_test_curve();
        // Single period OIS at 1 year
        // Note: Curve is built from continuously compounded rate (exp(-0.03*t))
        // but OIS par rate is simple rate: (1/DF - 1) / T
        // For DF = exp(-0.03) ≈ 0.9704, simple rate ≈ (1/0.9704 - 1) / 1 ≈ 0.0305
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(1.0, 0.03);
        let theoretical = ois.theoretical_rate(&curve).unwrap();

        // For flat 3% continuously compounded curve, simple rate is slightly higher
        assert_relative_eq!(theoretical, 0.0305, epsilon = 1e-3);
        assert!(theoretical > 0.03); // Simple rate > continuous rate
    }

    #[test]
    fn test_theoretical_rate_fra() {
        let curve = create_test_curve();
        // FRA from 0.5 to 1.0 years
        let fra: BootstrapInstrument<f64> = BootstrapInstrument::fra(0.5, 1.0, 0.03);
        let theoretical = fra.theoretical_rate(&curve).unwrap();

        // Forward rate from 0.5 to 1.0 for flat curve is close to 3%
        // but simple rate differs from continuous rate
        assert_relative_eq!(theoretical, 0.03, epsilon = 5e-3);
    }

    #[test]
    fn test_pricing_error() {
        let curve = create_test_curve();

        // OIS at 3% - error will be positive since simple rate > continuous rate
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(1.0, 0.03);
        let error = ois.pricing_error(&curve).unwrap();
        // The error is expected to be small but positive (~0.0005 for 1 year)
        assert!(error.abs() < 0.01, "expected small-ish error, got {}", error);

        // OIS at 4% - should have negative pricing error (market rate > theoretical)
        let ois_higher: BootstrapInstrument<f64> = BootstrapInstrument::ois(1.0, 0.04);
        let error_higher = ois_higher.pricing_error(&curve).unwrap();
        assert!(error_higher < 0.0, "expected negative error");
        // Theoretical is ~0.0305, so error is ~0.0305 - 0.04 ≈ -0.0095
        assert_relative_eq!(error_higher, -0.0095, epsilon = 5e-3);
    }

    #[test]
    fn test_compute_fra_rate_zero_start() {
        let curve = create_test_curve();
        // FRA starting at 0 is essentially a zero rate
        let rate = compute_fra_rate::<f64, _>(0.0, 1.0, &curve).unwrap();
        assert_relative_eq!(rate, 0.03, epsilon = 1e-3);
    }

    #[test]
    fn test_calibration_instrument_clone() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
        let cloned = ois.clone();
        assert_relative_eq!(ois.market_rate(), cloned.market_rate(), epsilon = 1e-15);
    }
}
