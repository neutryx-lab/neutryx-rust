//! Interest rate derivatives pricing module.
//!
//! This module provides pricing logic for interest rate derivatives:
//! - IRS (Interest Rate Swap) valuation
//! - Swaption pricing using Black76 and Bachelier models
//!
//! # IRS Pricing
//!
//! IRS is priced by computing the present value of both legs:
//! - Fixed Leg: Sum of discounted fixed rate payments
//! - Floating Leg: Sum of discounted projected floating rate payments
//!
//! # Example
//!
//! ```
//! use pricer_models::instruments::rates::pricing::price_irs;
//! use pricer_models::instruments::rates::{
//!     InterestRateSwap, FixedLeg, FloatingLeg, RateIndex, SwapDirection,
//! };
//! use pricer_models::schedules::{ScheduleBuilder, Frequency};
//! use pricer_core::types::{Currency, time::{Date, DayCountConvention}};
//! use pricer_core::market_data::curves::{CurveSet, CurveName, CurveEnum};
//!
//! // Create a simple swap
//! let start = Date::from_ymd(2024, 1, 15).unwrap();
//! let end = Date::from_ymd(2026, 1, 15).unwrap();
//!
//! let schedule = ScheduleBuilder::new()
//!     .start(start)
//!     .end(end)
//!     .frequency(Frequency::SemiAnnual)
//!     .day_count(DayCountConvention::Thirty360)
//!     .build()
//!     .unwrap();
//!
//! let fixed_leg = FixedLeg::new(schedule.clone(), 0.03, DayCountConvention::Thirty360);
//! let floating_leg = FloatingLeg::new(
//!     schedule,
//!     0.0,
//!     RateIndex::Sofr,
//!     DayCountConvention::ActualActual360,
//! );
//!
//! let swap = InterestRateSwap::new(
//!     1_000_000.0,
//!     fixed_leg,
//!     floating_leg,
//!     Currency::USD,
//!     SwapDirection::PayFixed,
//! );
//!
//! // Create curve set with discount and forward curves
//! let mut curves = CurveSet::new();
//! curves.insert(CurveName::Discount, CurveEnum::flat(0.03));
//! curves.insert(CurveName::Sofr, CurveEnum::flat(0.035));
//! curves.set_discount_curve(CurveName::Discount);
//!
//! let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
//!
//! // Price the swap
//! let pv = price_irs(&swap, &curves, valuation_date);
//! ```

use num_traits::Float;
use pricer_core::market_data::curves::{CurveName, CurveSet, YieldCurve};
use pricer_core::types::time::{Date, DayCountConvention};

use super::{InterestRateSwap, RateIndex, SwapDirection};
use crate::analytical::error::AnalyticalError;

/// Price an Interest Rate Swap.
///
/// Computes the present value of the swap by calculating:
/// - PV(Fixed Leg): Sum of discounted fixed rate payments
/// - PV(Floating Leg): Sum of discounted projected floating rate payments
///
/// The swap value depends on the direction:
/// - PayFixed: PV(Floating) - PV(Fixed)
/// - ReceiveFixed: PV(Fixed) - PV(Floating)
///
/// # Arguments
///
/// * `swap` - The interest rate swap to price
/// * `curves` - Curve set containing discount and forward curves
/// * `valuation_date` - The valuation date
///
/// # Returns
///
/// Present value of the swap.
///
/// # Panics
///
/// Panics if required curves are not found in the curve set.
pub fn price_irs<T: Float>(
    swap: &InterestRateSwap<T>,
    curves: &CurveSet<T>,
    valuation_date: Date,
) -> T {
    let fixed_pv = price_fixed_leg(swap, curves, valuation_date);
    let floating_pv = price_floating_leg(swap, curves, valuation_date);

    match swap.direction() {
        SwapDirection::PayFixed => floating_pv - fixed_pv,
        SwapDirection::ReceiveFixed => fixed_pv - floating_pv,
    }
}

/// Price the fixed leg of a swap.
///
/// Computes the present value of all fixed rate payments:
/// PV = Sum_i(Notional × FixedRate × YearFraction_i × DF_i)
///
/// # Arguments
///
/// * `swap` - The interest rate swap
/// * `curves` - Curve set containing the discount curve
/// * `valuation_date` - The valuation date
///
/// # Returns
///
/// Present value of the fixed leg.
pub fn price_fixed_leg<T: Float>(
    swap: &InterestRateSwap<T>,
    curves: &CurveSet<T>,
    valuation_date: Date,
) -> T {
    let discount_curve = curves
        .discount_curve()
        .expect("Discount curve not found in curve set");

    let fixed_leg = swap.fixed_leg();
    let notional = swap.notional();
    let fixed_rate = fixed_leg.fixed_rate();
    let day_count = fixed_leg.day_count();

    let mut pv = T::zero();

    for period in fixed_leg.schedule().periods() {
        // Skip periods that have already ended
        if period.end() <= valuation_date {
            continue;
        }

        // Calculate year fraction for this period
        let year_frac = day_count.year_fraction_dates(period.start(), period.end());
        let year_frac_t = T::from(year_frac).unwrap_or_else(T::zero);

        // Calculate time to payment in years
        let time_to_payment = DayCountConvention::ActualActual365
            .year_fraction_dates(valuation_date, period.payment());
        let time_to_payment_t = T::from(time_to_payment).unwrap_or_else(T::zero);

        // Get discount factor
        let df = discount_curve
            .discount_factor(time_to_payment_t)
            .unwrap_or_else(|_| T::one());

        // Calculate cashflow: Notional × FixedRate × YearFraction
        let cashflow = notional * fixed_rate * year_frac_t;

        // Add discounted cashflow
        pv = pv + cashflow * df;
    }

    pv
}

/// Price the floating leg of a swap.
///
/// Computes the present value of all floating rate payments:
/// PV = Sum_i(Notional × (ForwardRate_i + Spread) × YearFraction_i × DF_i)
///
/// The forward rates are projected from the forward curve.
///
/// # Arguments
///
/// * `swap` - The interest rate swap
/// * `curves` - Curve set containing discount and forward curves
/// * `valuation_date` - The valuation date
///
/// # Returns
///
/// Present value of the floating leg.
pub fn price_floating_leg<T: Float>(
    swap: &InterestRateSwap<T>,
    curves: &CurveSet<T>,
    valuation_date: Date,
) -> T {
    let discount_curve = curves
        .discount_curve()
        .expect("Discount curve not found in curve set");

    // Get forward curve based on the index
    let forward_curve_name = get_curve_name_for_index(swap.floating_leg().index());
    let forward_curve = curves
        .get(&forward_curve_name)
        .or_else(|| curves.discount_curve())
        .expect("Forward curve not found in curve set");

    let floating_leg = swap.floating_leg();
    let notional = swap.notional();
    let spread = floating_leg.spread();
    let day_count = floating_leg.day_count();

    let mut pv = T::zero();

    for period in floating_leg.schedule().periods() {
        // Skip periods that have already ended
        if period.end() <= valuation_date {
            continue;
        }

        // Calculate times in years from valuation date
        let t_start =
            DayCountConvention::ActualActual365.year_fraction_dates(valuation_date, period.start());
        let t_end =
            DayCountConvention::ActualActual365.year_fraction_dates(valuation_date, period.end());
        let t_payment = DayCountConvention::ActualActual365
            .year_fraction_dates(valuation_date, period.payment());

        let t_start_t = T::from(t_start).unwrap_or_else(T::zero);
        let t_end_t = T::from(t_end).unwrap_or_else(T::zero);
        let t_payment_t = T::from(t_payment).unwrap_or_else(T::zero);

        // Calculate year fraction for accrual
        let year_frac = day_count.year_fraction_dates(period.start(), period.end());
        let year_frac_t = T::from(year_frac).unwrap_or_else(T::zero);

        // Get forward rate from the forward curve
        let forward_rate = if t_start_t <= T::zero() {
            // For periods starting on or before valuation date, use zero rate
            forward_curve
                .zero_rate(t_end_t)
                .unwrap_or_else(|_| T::zero())
        } else {
            forward_curve
                .forward_rate(t_start_t, t_end_t)
                .unwrap_or_else(|_| T::zero())
        };

        // Get discount factor
        let df = discount_curve
            .discount_factor(t_payment_t)
            .unwrap_or_else(|_| T::one());

        // Calculate cashflow: Notional × (ForwardRate + Spread) × YearFraction
        let cashflow = notional * (forward_rate + spread) * year_frac_t;

        // Add discounted cashflow
        pv = pv + cashflow * df;
    }

    pv
}

/// Calculate the par swap rate.
///
/// The par swap rate is the fixed rate that makes the swap have zero
/// present value at inception.
///
/// ParRate = Sum_i(DF_i × ForwardRate_i × YearFrac_i) / Sum_i(DF_i × YearFrac_i)
///
/// # Arguments
///
/// * `swap` - The interest rate swap (fixed rate is ignored)
/// * `curves` - Curve set containing discount and forward curves
/// * `valuation_date` - The valuation date
///
/// # Returns
///
/// The par swap rate.
pub fn par_swap_rate<T: Float>(
    swap: &InterestRateSwap<T>,
    curves: &CurveSet<T>,
    valuation_date: Date,
) -> T {
    let discount_curve = curves
        .discount_curve()
        .expect("Discount curve not found in curve set");

    let forward_curve_name = get_curve_name_for_index(swap.floating_leg().index());
    let forward_curve = curves
        .get(&forward_curve_name)
        .or_else(|| curves.discount_curve())
        .expect("Forward curve not found in curve set");

    let fixed_leg = swap.fixed_leg();
    let floating_leg = swap.floating_leg();
    let fixed_day_count = fixed_leg.day_count();
    let floating_day_count = floating_leg.day_count();

    let mut annuity = T::zero(); // Sum_i(DF_i × YearFrac_i)
    let mut floating_pv = T::zero(); // Sum_i(DF_i × ForwardRate_i × YearFrac_i)

    // Calculate annuity from fixed leg
    for period in fixed_leg.schedule().periods() {
        if period.end() <= valuation_date {
            continue;
        }

        let year_frac = fixed_day_count.year_fraction_dates(period.start(), period.end());
        let year_frac_t = T::from(year_frac).unwrap_or_else(T::zero);

        let t_payment = DayCountConvention::ActualActual365
            .year_fraction_dates(valuation_date, period.payment());
        let t_payment_t = T::from(t_payment).unwrap_or_else(T::zero);

        let df = discount_curve
            .discount_factor(t_payment_t)
            .unwrap_or_else(|_| T::one());

        annuity = annuity + df * year_frac_t;
    }

    // Calculate floating leg PV (normalized by notional)
    for period in floating_leg.schedule().periods() {
        if period.end() <= valuation_date {
            continue;
        }

        let t_start =
            DayCountConvention::ActualActual365.year_fraction_dates(valuation_date, period.start());
        let t_end =
            DayCountConvention::ActualActual365.year_fraction_dates(valuation_date, period.end());
        let t_payment = DayCountConvention::ActualActual365
            .year_fraction_dates(valuation_date, period.payment());

        let t_start_t = T::from(t_start).unwrap_or_else(T::zero);
        let t_end_t = T::from(t_end).unwrap_or_else(T::zero);
        let t_payment_t = T::from(t_payment).unwrap_or_else(T::zero);

        let year_frac = floating_day_count.year_fraction_dates(period.start(), period.end());
        let year_frac_t = T::from(year_frac).unwrap_or_else(T::zero);

        let forward_rate = if t_start_t <= T::zero() {
            forward_curve
                .zero_rate(t_end_t)
                .unwrap_or_else(|_| T::zero())
        } else {
            forward_curve
                .forward_rate(t_start_t, t_end_t)
                .unwrap_or_else(|_| T::zero())
        };

        let df = discount_curve
            .discount_factor(t_payment_t)
            .unwrap_or_else(|_| T::one());

        floating_pv = floating_pv + df * forward_rate * year_frac_t;
    }

    // Par rate = Floating PV / Annuity
    if annuity > T::zero() {
        floating_pv / annuity
    } else {
        T::zero()
    }
}

/// Map rate index to curve name.
fn get_curve_name_for_index(index: RateIndex) -> CurveName {
    match index {
        RateIndex::Sofr => CurveName::Sofr,
        RateIndex::Tonar => CurveName::Tonar,
        RateIndex::Euribor3M | RateIndex::Euribor6M => CurveName::Euribor,
        RateIndex::Sonia => CurveName::Custom("SONIA"),
        RateIndex::Saron => CurveName::Custom("SARON"),
    }
}

/// Price a Swaption using the Black76 model.
///
/// The Black76 model assumes log-normal forward swap rates.
///
/// # Arguments
///
/// * `swaption` - The swaption to price
/// * `curves` - Curve set containing discount and forward curves
/// * `volatility` - Annualized log-normal volatility of the forward swap rate
/// * `valuation_date` - The valuation date
///
/// # Returns
///
/// Present value of the swaption.
pub fn price_swaption_black76<T: Float>(
    swaption: &super::Swaption<T>,
    curves: &CurveSet<T>,
    volatility: T,
    valuation_date: Date,
) -> Result<T, AnalyticalError> {
    use crate::analytical::distributions::norm_cdf;

    // Validate inputs
    if volatility <= T::zero() {
        return Err(AnalyticalError::InvalidVolatility {
            volatility: volatility.to_f64().unwrap_or(0.0),
        });
    }

    let expiry = swaption.expiry();
    if expiry <= T::zero() {
        return Err(AnalyticalError::InvalidSpot {
            spot: expiry.to_f64().unwrap_or(0.0),
        });
    }

    let underlying = swaption.underlying();
    let strike = swaption.strike();

    // Calculate forward swap rate (par rate)
    let forward = par_swap_rate(underlying, curves, valuation_date);

    // Calculate swap annuity
    let annuity = calculate_annuity(underlying, curves, valuation_date);

    // Black76 formula
    let sqrt_t = expiry.sqrt();
    let vol_sqrt_t = volatility * sqrt_t;

    // d1 = (ln(F/K) + 0.5 * sigma^2 * T) / (sigma * sqrt(T))
    // d2 = d1 - sigma * sqrt(T)
    let d1 = ((forward / strike).ln() + volatility * volatility * expiry / (T::one() + T::one()))
        / vol_sqrt_t;
    let d2 = d1 - vol_sqrt_t;

    let notional = underlying.notional();

    // Price based on swaption type
    let price = match swaption.swaption_type() {
        super::SwaptionType::Payer => {
            // Payer swaption: max(S - K, 0) * Annuity
            // Price = Annuity * [F * N(d1) - K * N(d2)]
            notional * annuity * (forward * norm_cdf(d1) - strike * norm_cdf(d2))
        }
        super::SwaptionType::Receiver => {
            // Receiver swaption: max(K - S, 0) * Annuity
            // Price = Annuity * [K * N(-d2) - F * N(-d1)]
            notional * annuity * (strike * norm_cdf(-d2) - forward * norm_cdf(-d1))
        }
    };

    Ok(price)
}

/// Price a Swaption using the Bachelier (normal) model.
///
/// The Bachelier model assumes normally distributed forward swap rates.
/// This is often preferred in low/negative rate environments.
///
/// # Arguments
///
/// * `swaption` - The swaption to price
/// * `curves` - Curve set containing discount and forward curves
/// * `volatility` - Annualized normal volatility (in absolute terms)
/// * `valuation_date` - The valuation date
///
/// # Returns
///
/// Present value of the swaption.
pub fn price_swaption_bachelier<T: Float>(
    swaption: &super::Swaption<T>,
    curves: &CurveSet<T>,
    volatility: T,
    valuation_date: Date,
) -> Result<T, AnalyticalError> {
    use crate::analytical::distributions::{norm_cdf, norm_pdf};

    // Validate inputs
    if volatility <= T::zero() {
        return Err(AnalyticalError::InvalidVolatility {
            volatility: volatility.to_f64().unwrap_or(0.0),
        });
    }

    let expiry = swaption.expiry();
    if expiry <= T::zero() {
        return Err(AnalyticalError::InvalidSpot {
            spot: expiry.to_f64().unwrap_or(0.0),
        });
    }

    let underlying = swaption.underlying();
    let strike = swaption.strike();

    // Calculate forward swap rate (par rate)
    let forward = par_swap_rate(underlying, curves, valuation_date);

    // Calculate swap annuity
    let annuity = calculate_annuity(underlying, curves, valuation_date);

    // Bachelier formula
    let sqrt_t = expiry.sqrt();
    let vol_sqrt_t = volatility * sqrt_t;

    // d = (F - K) / (sigma * sqrt(T))
    let d = (forward - strike) / vol_sqrt_t;

    let notional = underlying.notional();

    // Price based on swaption type
    let price = match swaption.swaption_type() {
        super::SwaptionType::Payer => {
            // Payer: (F - K) * N(d) + sigma * sqrt(T) * n(d)
            notional * annuity * ((forward - strike) * norm_cdf(d) + vol_sqrt_t * norm_pdf(d))
        }
        super::SwaptionType::Receiver => {
            // Receiver: (K - F) * N(-d) + sigma * sqrt(T) * n(d)
            notional * annuity * ((strike - forward) * norm_cdf(-d) + vol_sqrt_t * norm_pdf(d))
        }
    };

    Ok(price)
}

/// Calculate the annuity (PV01) of a swap.
///
/// Annuity = Sum_i(DF_i × YearFrac_i)
fn calculate_annuity<T: Float>(
    swap: &InterestRateSwap<T>,
    curves: &CurveSet<T>,
    valuation_date: Date,
) -> T {
    let discount_curve = curves
        .discount_curve()
        .expect("Discount curve not found in curve set");

    let fixed_leg = swap.fixed_leg();
    let day_count = fixed_leg.day_count();

    let mut annuity = T::zero();

    for period in fixed_leg.schedule().periods() {
        if period.end() <= valuation_date {
            continue;
        }

        let year_frac = day_count.year_fraction_dates(period.start(), period.end());
        let year_frac_t = T::from(year_frac).unwrap_or_else(T::zero);

        let t_payment = DayCountConvention::ActualActual365
            .year_fraction_dates(valuation_date, period.payment());
        let t_payment_t = T::from(t_payment).unwrap_or_else(T::zero);

        let df = discount_curve
            .discount_factor(t_payment_t)
            .unwrap_or_else(|_| T::one());

        annuity = annuity + df * year_frac_t;
    }

    annuity
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::rates::{FixedLeg, FloatingLeg, Swaption, SwaptionStyle, SwaptionType};
    use crate::schedules::{Frequency, ScheduleBuilder};
    use pricer_core::market_data::curves::CurveEnum;
    use pricer_core::types::Currency;

    fn create_test_swap() -> InterestRateSwap<f64> {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2026, 1, 15).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::SemiAnnual)
            .day_count(DayCountConvention::Thirty360)
            .build()
            .unwrap();

        let fixed_leg = FixedLeg::new(schedule.clone(), 0.03, DayCountConvention::Thirty360);
        let floating_leg = FloatingLeg::new(
            schedule,
            0.0,
            RateIndex::Sofr,
            DayCountConvention::ActualActual360,
        );

        InterestRateSwap::new(
            1_000_000.0,
            fixed_leg,
            floating_leg,
            Currency::USD,
            SwapDirection::PayFixed,
        )
    }

    fn create_test_curves() -> CurveSet<f64> {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Discount, CurveEnum::flat(0.03));
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035));
        curves.set_discount_curve(CurveName::Discount);
        curves
    }

    // ========================================
    // IRS Pricing Tests
    // ========================================

    #[test]
    fn test_price_irs_payer_swap() {
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let pv = price_irs(&swap, &curves, valuation_date);

        // Payer swap (pay fixed, receive floating)
        // Forward rate > Fixed rate, so PV should be positive
        assert!(
            pv > 0.0,
            "Payer swap should have positive PV when forward rate > fixed rate"
        );
    }

    #[test]
    fn test_price_irs_receiver_swap() {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2026, 1, 15).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::SemiAnnual)
            .day_count(DayCountConvention::Thirty360)
            .build()
            .unwrap();

        let fixed_leg = FixedLeg::new(schedule.clone(), 0.03, DayCountConvention::Thirty360);
        let floating_leg = FloatingLeg::new(
            schedule,
            0.0,
            RateIndex::Sofr,
            DayCountConvention::ActualActual360,
        );

        let swap = InterestRateSwap::new(
            1_000_000.0,
            fixed_leg,
            floating_leg,
            Currency::USD,
            SwapDirection::ReceiveFixed,
        );

        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let pv = price_irs(&swap, &curves, valuation_date);

        // Receiver swap should have opposite sign to payer swap
        assert!(
            pv < 0.0,
            "Receiver swap should have negative PV when forward rate > fixed rate"
        );
    }

    #[test]
    fn test_price_fixed_leg() {
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let fixed_pv = price_fixed_leg(&swap, &curves, valuation_date);

        // Fixed leg PV should be positive and approximately
        // Notional × FixedRate × Maturity × AvgDF
        assert!(fixed_pv > 0.0);
        // Rough check: ~1M × 3% × 2 years = ~60k (discounted)
        assert!(fixed_pv > 50_000.0 && fixed_pv < 70_000.0);
    }

    #[test]
    fn test_price_floating_leg() {
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let floating_pv = price_floating_leg(&swap, &curves, valuation_date);

        // Floating leg PV should be positive
        assert!(floating_pv > 0.0);
        // Rough check: ~1M × 3.5% × 2 years = ~70k (discounted)
        assert!(floating_pv > 60_000.0 && floating_pv < 80_000.0);
    }

    #[test]
    fn test_par_swap_rate() {
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let par_rate = par_swap_rate(&swap, &curves, valuation_date);

        // Par rate should be close to the forward curve rate
        assert!(par_rate > 0.03 && par_rate < 0.04);
    }

    #[test]
    fn test_swap_at_par_has_zero_pv() {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2026, 1, 15).unwrap();
        let valuation_date = start;

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::SemiAnnual)
            .day_count(DayCountConvention::Thirty360)
            .build()
            .unwrap();

        // Use a flat curve where discount = forward
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Discount, CurveEnum::flat(0.035));
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035));
        curves.set_discount_curve(CurveName::Discount);

        // Create a swap with a dummy fixed rate
        let fixed_leg = FixedLeg::new(schedule.clone(), 0.035, DayCountConvention::Thirty360);
        let floating_leg = FloatingLeg::new(
            schedule,
            0.0,
            RateIndex::Sofr,
            DayCountConvention::Thirty360, // Use same day count for simplicity
        );

        let swap = InterestRateSwap::new(
            1_000_000.0,
            fixed_leg,
            floating_leg,
            Currency::USD,
            SwapDirection::PayFixed,
        );

        // Get par rate and verify it's close to 3.5%
        let par_rate = par_swap_rate(&swap, &curves, valuation_date);
        assert!((par_rate - 0.035).abs() < 0.001);
    }

    // ========================================
    // Swaption Pricing Tests
    // ========================================

    #[test]
    fn test_price_swaption_black76() {
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let swaption = Swaption::new(
            swap,
            0.5,
            0.03,
            SwaptionType::Payer,
            SwaptionStyle::European,
        );

        let price = price_swaption_black76(&swaption, &curves, 0.20, valuation_date);

        assert!(price.is_ok());
        let pv = price.unwrap();
        assert!(pv > 0.0, "Swaption should have positive value");
    }

    #[test]
    fn test_price_swaption_bachelier() {
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let swaption = Swaption::new(
            swap,
            0.5,
            0.03,
            SwaptionType::Payer,
            SwaptionStyle::European,
        );

        // Bachelier vol is in absolute terms (e.g., 50bp = 0.005)
        let price = price_swaption_bachelier(&swaption, &curves, 0.005, valuation_date);

        assert!(price.is_ok());
        let pv = price.unwrap();
        assert!(pv > 0.0, "Swaption should have positive value");
    }

    #[test]
    fn test_payer_receiver_parity() {
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let strike = 0.035;
        let vol = 0.20;

        let payer = Swaption::new(
            swap.clone(),
            0.5,
            strike,
            SwaptionType::Payer,
            SwaptionStyle::European,
        );
        let receiver = Swaption::new(
            swap.clone(),
            0.5,
            strike,
            SwaptionType::Receiver,
            SwaptionStyle::European,
        );

        let payer_price = price_swaption_black76(&payer, &curves, vol, valuation_date).unwrap();
        let receiver_price =
            price_swaption_black76(&receiver, &curves, vol, valuation_date).unwrap();

        // Calculate forward swap rate
        let forward = par_swap_rate(&swap, &curves, valuation_date);
        let annuity = calculate_annuity(&swap, &curves, valuation_date);
        let notional = swap.notional();

        // Payer - Receiver = Notional × Annuity × (Forward - Strike)
        let intrinsic = notional * annuity * (forward - strike);

        let parity_diff = (payer_price - receiver_price - intrinsic).abs();

        assert!(
            parity_diff < 1000.0,
            "Put-call parity should hold: diff = {}",
            parity_diff
        );
    }

    #[test]
    fn test_swaption_invalid_volatility() {
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();

        let swaption = Swaption::new(
            swap,
            0.5,
            0.03,
            SwaptionType::Payer,
            SwaptionStyle::European,
        );

        let result = price_swaption_black76(&swaption, &curves, 0.0, valuation_date);
        assert!(result.is_err());

        let result = price_swaption_black76(&swaption, &curves, -0.1, valuation_date);
        assert!(result.is_err());
    }
}
