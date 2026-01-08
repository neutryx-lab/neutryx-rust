//! Interest rate derivative instruments.
//!
//! This module provides interest rate derivative instruments including:
//! - [`InterestRateSwap`]: Plain vanilla IRS with fixed and floating legs
//! - [`Swaption`]: Option on interest rate swaps
//! - [`Cap`] and [`Floor`]: Interest rate caps and floors
//! - [`Collar`]: Combination of cap and floor
//!
//! # Feature Flag
//!
//! This module is available when the `rates` feature is enabled.
//!
//! # Architecture
//!
//! All rate instruments are wrapped in the [`RatesInstrument`] enum for
//! Enzyme AD-compatible static dispatch.
//!
//! # Examples
//!
//! ```
//! use pricer_models::instruments::rates::{
//!     InterestRateSwap, FixedLeg, FloatingLeg, RateIndex, SwapDirection,
//! };
//! use pricer_models::schedules::{ScheduleBuilder, Frequency};
//! use pricer_core::types::{Currency, time::{Date, DayCountConvention}};
//!
//! // Create a 5-year USD IRS
//! let start = Date::from_ymd(2024, 1, 15).unwrap();
//! let end = Date::from_ymd(2029, 1, 15).unwrap();
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
//!     0.0,  // no spread
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
//! ```

mod capfloor;
pub mod pricing;
mod swap;
mod swaption;

pub use capfloor::{Cap, Collar, Floor};
pub use pricing::{
    par_swap_rate, price_fixed_leg, price_floating_leg, price_irs, price_swaption_bachelier,
    price_swaption_black76,
};
pub use swap::{FixedLeg, FloatingLeg, InterestRateSwap, RateIndex, SwapDirection};
pub use swaption::{Swaption, SwaptionStyle, SwaptionType};

use num_traits::Float;
use pricer_core::types::Currency;

use super::InstrumentTrait;

/// Interest rate instruments sub-enum for static dispatch.
///
/// This enum wraps all interest rate derivative instruments and enables
/// Enzyme AD-compatible static dispatch without trait objects.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Variants
///
/// - `Swap`: Plain vanilla interest rate swap
/// - `Swaption`: Option on interest rate swap
/// - `Cap`: Interest rate cap (series of caplets)
/// - `Floor`: Interest rate floor (series of floorlets)
///
/// # Examples
///
/// ```
/// use pricer_models::instruments::rates::{
///     RatesInstrument, InterestRateSwap, FixedLeg, FloatingLeg, RateIndex, SwapDirection,
/// };
/// use pricer_models::schedules::{ScheduleBuilder, Frequency};
/// use pricer_core::types::{Currency, time::{Date, DayCountConvention}};
///
/// let start = Date::from_ymd(2024, 1, 15).unwrap();
/// let end = Date::from_ymd(2026, 1, 15).unwrap();
///
/// let schedule = ScheduleBuilder::new()
///     .start(start)
///     .end(end)
///     .frequency(Frequency::SemiAnnual)
///     .day_count(DayCountConvention::Thirty360)
///     .build()
///     .unwrap();
///
/// let fixed_leg = FixedLeg::new(schedule.clone(), 0.03, DayCountConvention::Thirty360);
/// let floating_leg = FloatingLeg::new(
///     schedule,
///     0.0,
///     RateIndex::Sofr,
///     DayCountConvention::ActualActual360,
/// );
///
/// let swap = InterestRateSwap::new(
///     1_000_000.0,
///     fixed_leg,
///     floating_leg,
///     Currency::USD,
///     SwapDirection::PayFixed,
/// );
///
/// let instrument: RatesInstrument<f64> = swap.into();
/// assert_eq!(instrument.type_name(), "RatesSwap");
/// ```
#[derive(Debug, Clone)]
pub enum RatesInstrument<T: Float> {
    /// Plain vanilla interest rate swap.
    Swap(InterestRateSwap<T>),
    /// Swaption (option on swap).
    Swaption(Swaption<T>),
    /// Interest rate cap.
    Cap(Cap<T>),
    /// Interest rate floor.
    Floor(Floor<T>),
}

impl<T: Float> RatesInstrument<T> {
    /// Compute payoff at given spot (placeholder for rate instruments).
    ///
    /// Note: Rate instruments don't have spot-based payoffs in the
    /// traditional sense. This returns zero. Use curve-based valuation
    /// in the pricing layer instead.
    #[inline]
    pub fn payoff(&self, _spot: T) -> T {
        // Rate instruments use curve-based valuation, not spot-based
        T::zero()
    }

    /// Return time to final maturity in years.
    #[inline]
    pub fn expiry(&self) -> T {
        match self {
            RatesInstrument::Swap(swap) => swap.maturity(),
            RatesInstrument::Swaption(swaption) => swaption.expiry(),
            RatesInstrument::Cap(cap) => cap.maturity(),
            RatesInstrument::Floor(floor) => floor.maturity(),
        }
    }

    /// Return the settlement currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        match self {
            RatesInstrument::Swap(swap) => swap.currency(),
            RatesInstrument::Swaption(swaption) => swaption.currency(),
            RatesInstrument::Cap(cap) => cap.currency(),
            RatesInstrument::Floor(floor) => floor.currency(),
        }
    }

    /// Return the instrument type name.
    pub fn type_name(&self) -> &'static str {
        match self {
            RatesInstrument::Swap(_) => "RatesSwap",
            RatesInstrument::Swaption(_) => "RatesSwaption",
            RatesInstrument::Cap(_) => "RatesCap",
            RatesInstrument::Floor(_) => "RatesFloor",
        }
    }

    /// Check if this is a swap instrument.
    #[inline]
    pub fn is_swap(&self) -> bool {
        matches!(self, RatesInstrument::Swap(_))
    }

    /// Check if this is a swaption instrument.
    #[inline]
    pub fn is_swaption(&self) -> bool {
        matches!(self, RatesInstrument::Swaption(_))
    }

    /// Check if this is a cap instrument.
    #[inline]
    pub fn is_cap(&self) -> bool {
        matches!(self, RatesInstrument::Cap(_))
    }

    /// Check if this is a floor instrument.
    #[inline]
    pub fn is_floor(&self) -> bool {
        matches!(self, RatesInstrument::Floor(_))
    }

    /// Get reference to swap if this is a Swap variant.
    pub fn as_swap(&self) -> Option<&InterestRateSwap<T>> {
        match self {
            RatesInstrument::Swap(swap) => Some(swap),
            _ => None,
        }
    }

    /// Get reference to swaption if this is a Swaption variant.
    pub fn as_swaption(&self) -> Option<&Swaption<T>> {
        match self {
            RatesInstrument::Swaption(swaption) => Some(swaption),
            _ => None,
        }
    }

    /// Get reference to cap if this is a Cap variant.
    pub fn as_cap(&self) -> Option<&Cap<T>> {
        match self {
            RatesInstrument::Cap(cap) => Some(cap),
            _ => None,
        }
    }

    /// Get reference to floor if this is a Floor variant.
    pub fn as_floor(&self) -> Option<&Floor<T>> {
        match self {
            RatesInstrument::Floor(floor) => Some(floor),
            _ => None,
        }
    }
}

impl<T: Float> InstrumentTrait<T> for RatesInstrument<T> {
    #[inline]
    fn payoff(&self, spot: T) -> T {
        self.payoff(spot)
    }

    #[inline]
    fn expiry(&self) -> T {
        self.expiry()
    }

    #[inline]
    fn currency(&self) -> Currency {
        self.currency()
    }

    fn type_name(&self) -> &'static str {
        self.type_name()
    }
}

// Conversion from individual instruments to RatesInstrument
impl<T: Float> From<InterestRateSwap<T>> for RatesInstrument<T> {
    fn from(swap: InterestRateSwap<T>) -> Self {
        RatesInstrument::Swap(swap)
    }
}

impl<T: Float> From<Swaption<T>> for RatesInstrument<T> {
    fn from(swaption: Swaption<T>) -> Self {
        RatesInstrument::Swaption(swaption)
    }
}

impl<T: Float> From<Cap<T>> for RatesInstrument<T> {
    fn from(cap: Cap<T>) -> Self {
        RatesInstrument::Cap(cap)
    }
}

impl<T: Float> From<Floor<T>> for RatesInstrument<T> {
    fn from(floor: Floor<T>) -> Self {
        RatesInstrument::Floor(floor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedules::{Frequency, ScheduleBuilder};
    use pricer_core::types::time::{Date, DayCountConvention};

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

    fn create_test_cap() -> Cap<f64> {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2027, 1, 15).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        Cap::new(1_000_000.0, schedule, 0.04, RateIndex::Sofr, Currency::USD)
    }

    fn create_test_floor() -> Floor<f64> {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2027, 1, 15).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        Floor::new(1_000_000.0, schedule, 0.02, RateIndex::Sofr, Currency::USD)
    }

    fn create_test_swaption() -> Swaption<f64> {
        let swap = create_test_swap();
        Swaption::new(
            swap,
            1.0,
            0.03,
            SwaptionType::Payer,
            SwaptionStyle::European,
        )
    }

    // ========================================
    // RatesInstrument Swap Tests
    // ========================================

    #[test]
    fn test_rates_instrument_swap() {
        let swap = create_test_swap();
        let instrument = RatesInstrument::Swap(swap);

        assert_eq!(instrument.payoff(100.0), 0.0);
        assert_eq!(instrument.currency(), Currency::USD);
        assert_eq!(instrument.type_name(), "RatesSwap");
        assert!(instrument.is_swap());
        assert!(!instrument.is_swaption());
        assert!(!instrument.is_cap());
        assert!(!instrument.is_floor());
    }

    #[test]
    fn test_rates_instrument_from_swap() {
        let swap = create_test_swap();
        let instrument: RatesInstrument<f64> = swap.into();

        assert!(matches!(instrument, RatesInstrument::Swap(_)));
        assert!(instrument.as_swap().is_some());
        assert!(instrument.as_swaption().is_none());
    }

    // ========================================
    // RatesInstrument Swaption Tests
    // ========================================

    #[test]
    fn test_rates_instrument_swaption() {
        let swaption = create_test_swaption();
        let instrument = RatesInstrument::Swaption(swaption);

        assert_eq!(instrument.payoff(100.0), 0.0);
        assert_eq!(instrument.currency(), Currency::USD);
        assert_eq!(instrument.type_name(), "RatesSwaption");
        assert!(instrument.is_swaption());
        assert!(!instrument.is_swap());
    }

    #[test]
    fn test_rates_instrument_from_swaption() {
        let swaption = create_test_swaption();
        let instrument: RatesInstrument<f64> = swaption.into();

        assert!(matches!(instrument, RatesInstrument::Swaption(_)));
        assert!(instrument.as_swaption().is_some());
    }

    // ========================================
    // RatesInstrument Cap Tests
    // ========================================

    #[test]
    fn test_rates_instrument_cap() {
        let cap = create_test_cap();
        let instrument = RatesInstrument::Cap(cap);

        assert_eq!(instrument.payoff(100.0), 0.0);
        assert_eq!(instrument.currency(), Currency::USD);
        assert_eq!(instrument.type_name(), "RatesCap");
        assert!(instrument.is_cap());
        assert!(!instrument.is_floor());
    }

    #[test]
    fn test_rates_instrument_from_cap() {
        let cap = create_test_cap();
        let instrument: RatesInstrument<f64> = cap.into();

        assert!(matches!(instrument, RatesInstrument::Cap(_)));
        assert!(instrument.as_cap().is_some());
    }

    // ========================================
    // RatesInstrument Floor Tests
    // ========================================

    #[test]
    fn test_rates_instrument_floor() {
        let floor = create_test_floor();
        let instrument = RatesInstrument::Floor(floor);

        assert_eq!(instrument.payoff(100.0), 0.0);
        assert_eq!(instrument.currency(), Currency::USD);
        assert_eq!(instrument.type_name(), "RatesFloor");
        assert!(instrument.is_floor());
        assert!(!instrument.is_cap());
    }

    #[test]
    fn test_rates_instrument_from_floor() {
        let floor = create_test_floor();
        let instrument: RatesInstrument<f64> = floor.into();

        assert!(matches!(instrument, RatesInstrument::Floor(_)));
        assert!(instrument.as_floor().is_some());
    }

    // ========================================
    // InstrumentTrait Implementation Tests
    // ========================================

    #[test]
    fn test_rates_instrument_trait() {
        let swap = create_test_swap();
        let instrument = RatesInstrument::Swap(swap);

        // Use InstrumentTrait methods
        let payoff = <RatesInstrument<f64> as InstrumentTrait<f64>>::payoff(&instrument, 100.0);
        assert_eq!(payoff, 0.0);

        let currency = <RatesInstrument<f64> as InstrumentTrait<f64>>::currency(&instrument);
        assert_eq!(currency, Currency::USD);

        let type_name = <RatesInstrument<f64> as InstrumentTrait<f64>>::type_name(&instrument);
        assert_eq!(type_name, "RatesSwap");
    }

    // ========================================
    // Clone and Debug Tests
    // ========================================

    #[test]
    fn test_rates_instrument_clone() {
        let swap = create_test_swap();
        let inst1 = RatesInstrument::Swap(swap);
        let inst2 = inst1.clone();

        assert_eq!(inst1.currency(), inst2.currency());
        assert_eq!(inst1.type_name(), inst2.type_name());
    }

    #[test]
    fn test_rates_instrument_debug() {
        let swap = create_test_swap();
        let instrument = RatesInstrument::Swap(swap);
        let debug_str = format!("{:?}", instrument);

        assert!(debug_str.contains("Swap"));
    }
}
