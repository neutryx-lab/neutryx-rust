//! Interest Rate Swap (IRS) implementation.
//!
//! This module provides the [`InterestRateSwap`] structure for vanilla IRS
//! with fixed and floating legs.
//!
//! # Structure
//!
//! A vanilla IRS consists of:
//! - Fixed leg: Pays/receives fixed rate payments
//! - Floating leg: Pays/receives floating rate (indexed to a benchmark)
//!
//! # Example
//!
//! ```
//! use pricer_models::instruments::rates::{
//!     InterestRateSwap, FixedLeg, FloatingLeg, RateIndex, SwapDirection,
//! };
//! use pricer_models::schedules::{ScheduleBuilder, Frequency};
//! use pricer_core::types::{Currency, time::{Date, DayCountConvention}};
//!
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
//! assert_eq!(swap.notional(), 1_000_000.0);
//! assert_eq!(swap.direction(), SwapDirection::PayFixed);
//! ```

use num_traits::Float;
use pricer_core::types::time::DayCountConvention;
use pricer_core::types::Currency;
use std::fmt;
use std::str::FromStr;

use crate::schedules::Schedule;

/// Interest rate benchmark index.
///
/// Defines the reference rate used for floating leg calculations.
/// Modern benchmarks (SOFR, TONAR) are RFR-based, while legacy
/// benchmarks (EURIBOR) are IBOR-based.
///
/// # Examples
///
/// ```
/// use pricer_models::instruments::rates::RateIndex;
///
/// let index = RateIndex::Sofr;
/// assert_eq!(index.name(), "SOFR");
/// assert_eq!(index.tenor_months(), 0); // overnight
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateIndex {
    /// Secured Overnight Financing Rate (USD).
    /// US dollar overnight rate based on Treasury repo transactions.
    Sofr,
    /// Tokyo Overnight Average Rate (JPY).
    /// Japanese yen overnight unsecured call rate.
    Tonar,
    /// Euro Interbank Offered Rate - 3 Month (EUR).
    /// 3-month term rate for EUR deposits.
    Euribor3M,
    /// Euro Interbank Offered Rate - 6 Month (EUR).
    /// 6-month term rate for EUR deposits.
    Euribor6M,
    /// Sterling Overnight Index Average (GBP).
    /// UK pound overnight unsecured rate.
    Sonia,
    /// Swiss Average Rate Overnight (CHF).
    /// Swiss franc overnight secured rate.
    Saron,
}

impl RateIndex {
    /// Returns the standard name for this index.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            RateIndex::Sofr => "SOFR",
            RateIndex::Tonar => "TONAR",
            RateIndex::Euribor3M => "EURIBOR3M",
            RateIndex::Euribor6M => "EURIBOR6M",
            RateIndex::Sonia => "SONIA",
            RateIndex::Saron => "SARON",
        }
    }

    /// Returns the tenor in months (0 for overnight rates).
    #[inline]
    pub fn tenor_months(&self) -> u32 {
        match self {
            RateIndex::Sofr => 0,
            RateIndex::Tonar => 0,
            RateIndex::Euribor3M => 3,
            RateIndex::Euribor6M => 6,
            RateIndex::Sonia => 0,
            RateIndex::Saron => 0,
        }
    }

    /// Returns whether this is an overnight rate.
    #[inline]
    pub fn is_overnight(&self) -> bool {
        self.tenor_months() == 0
    }

    /// Returns the default currency for this index.
    #[inline]
    pub fn currency(&self) -> Currency {
        match self {
            RateIndex::Sofr => Currency::USD,
            RateIndex::Tonar => Currency::JPY,
            RateIndex::Euribor3M | RateIndex::Euribor6M => Currency::EUR,
            RateIndex::Sonia => Currency::GBP,
            RateIndex::Saron => Currency::CHF,
        }
    }

    /// Returns the default day count convention for this index.
    #[inline]
    pub fn default_day_count(&self) -> DayCountConvention {
        match self {
            RateIndex::Sofr => DayCountConvention::ActualActual360,
            RateIndex::Tonar => DayCountConvention::ActualActual365,
            RateIndex::Euribor3M | RateIndex::Euribor6M => DayCountConvention::ActualActual360,
            RateIndex::Sonia => DayCountConvention::ActualActual365,
            RateIndex::Saron => DayCountConvention::ActualActual360,
        }
    }
}

impl fmt::Display for RateIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for RateIndex {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SOFR" => Ok(RateIndex::Sofr),
            "TONAR" | "TONA" => Ok(RateIndex::Tonar),
            "EURIBOR3M" | "EURIBOR_3M" | "EUR3M" => Ok(RateIndex::Euribor3M),
            "EURIBOR6M" | "EURIBOR_6M" | "EUR6M" => Ok(RateIndex::Euribor6M),
            "SONIA" => Ok(RateIndex::Sonia),
            "SARON" => Ok(RateIndex::Saron),
            _ => Err(format!("Unknown rate index: {}", s)),
        }
    }
}

/// Swap direction (payer or receiver of fixed rate).
///
/// In swap market convention:
/// - **PayFixed** (Payer swap): Pay fixed rate, receive floating rate
/// - **ReceiveFixed** (Receiver swap): Receive fixed rate, pay floating rate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwapDirection {
    /// Pay fixed rate, receive floating rate.
    PayFixed,
    /// Receive fixed rate, pay floating rate.
    ReceiveFixed,
}

impl SwapDirection {
    /// Returns the multiplier for fixed leg cashflows.
    ///
    /// - PayFixed: -1 (negative = pay)
    /// - ReceiveFixed: +1 (positive = receive)
    #[inline]
    pub fn fixed_multiplier<T: Float>(&self) -> T {
        match self {
            SwapDirection::PayFixed => -T::one(),
            SwapDirection::ReceiveFixed => T::one(),
        }
    }

    /// Returns the multiplier for floating leg cashflows.
    ///
    /// - PayFixed: +1 (positive = receive)
    /// - ReceiveFixed: -1 (negative = pay)
    #[inline]
    pub fn floating_multiplier<T: Float>(&self) -> T {
        match self {
            SwapDirection::PayFixed => T::one(),
            SwapDirection::ReceiveFixed => -T::one(),
        }
    }
}

impl fmt::Display for SwapDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwapDirection::PayFixed => write!(f, "Pay Fixed"),
            SwapDirection::ReceiveFixed => write!(f, "Receive Fixed"),
        }
    }
}

/// Fixed leg of an interest rate swap.
///
/// The fixed leg pays/receives a predetermined fixed rate on each
/// payment date according to the schedule.
///
/// # Cashflow Calculation
///
/// For each period i:
/// ```text
/// CF_i = Notional × FixedRate × YearFraction_i
/// ```
#[derive(Debug, Clone)]
pub struct FixedLeg<T: Float> {
    /// Payment schedule.
    schedule: Schedule,
    /// Fixed rate (annual, e.g., 0.03 for 3%).
    fixed_rate: T,
    /// Day count convention for accrual calculation.
    day_count: DayCountConvention,
}

impl<T: Float> FixedLeg<T> {
    /// Create a new fixed leg.
    ///
    /// # Arguments
    ///
    /// * `schedule` - Payment schedule
    /// * `fixed_rate` - Annual fixed rate (e.g., 0.03 for 3%)
    /// * `day_count` - Day count convention for accrual
    pub fn new(schedule: Schedule, fixed_rate: T, day_count: DayCountConvention) -> Self {
        Self {
            schedule,
            fixed_rate,
            day_count,
        }
    }

    /// Returns the payment schedule.
    #[inline]
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Returns the fixed rate.
    #[inline]
    pub fn fixed_rate(&self) -> T {
        self.fixed_rate
    }

    /// Returns the day count convention.
    #[inline]
    pub fn day_count(&self) -> DayCountConvention {
        self.day_count
    }

    /// Returns the number of payment periods.
    #[inline]
    pub fn num_periods(&self) -> usize {
        self.schedule.periods().len()
    }
}

/// Floating leg of an interest rate swap.
///
/// The floating leg pays/receives a floating rate indexed to a
/// benchmark rate (e.g., SOFR, EURIBOR) plus an optional spread.
///
/// # Cashflow Calculation
///
/// For each period i:
/// ```text
/// CF_i = Notional × (ForwardRate_i + Spread) × YearFraction_i
/// ```
#[derive(Debug, Clone)]
pub struct FloatingLeg<T: Float> {
    /// Payment schedule.
    schedule: Schedule,
    /// Spread over the index rate (annual, e.g., 0.001 for 10bp).
    spread: T,
    /// Reference rate index.
    index: RateIndex,
    /// Day count convention for accrual calculation.
    day_count: DayCountConvention,
}

impl<T: Float> FloatingLeg<T> {
    /// Create a new floating leg.
    ///
    /// # Arguments
    ///
    /// * `schedule` - Payment schedule
    /// * `spread` - Spread over index rate (e.g., 0.001 for 10bp)
    /// * `index` - Reference rate index (SOFR, EURIBOR, etc.)
    /// * `day_count` - Day count convention for accrual
    pub fn new(
        schedule: Schedule,
        spread: T,
        index: RateIndex,
        day_count: DayCountConvention,
    ) -> Self {
        Self {
            schedule,
            spread,
            index,
            day_count,
        }
    }

    /// Returns the payment schedule.
    #[inline]
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Returns the spread over the index rate.
    #[inline]
    pub fn spread(&self) -> T {
        self.spread
    }

    /// Returns the reference rate index.
    #[inline]
    pub fn index(&self) -> RateIndex {
        self.index
    }

    /// Returns the day count convention.
    #[inline]
    pub fn day_count(&self) -> DayCountConvention {
        self.day_count
    }

    /// Returns the number of payment periods.
    #[inline]
    pub fn num_periods(&self) -> usize {
        self.schedule.periods().len()
    }
}

/// Plain vanilla Interest Rate Swap (IRS).
///
/// An IRS is an agreement to exchange fixed and floating rate payments
/// on a notional principal over a specified term.
///
/// # Structure
///
/// - **Fixed Leg**: Pays/receives fixed rate at predetermined intervals
/// - **Floating Leg**: Pays/receives floating rate indexed to a benchmark
///
/// # Valuation
///
/// The swap value is the difference between the present values of the
/// two legs:
/// ```text
/// V = PV(ReceiveLeg) - PV(PayLeg)
/// ```
///
/// # Examples
///
/// ```
/// use pricer_models::instruments::rates::{
///     InterestRateSwap, FixedLeg, FloatingLeg, RateIndex, SwapDirection,
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
/// assert_eq!(swap.notional(), 1_000_000.0);
/// assert_eq!(swap.currency(), Currency::USD);
/// ```
#[derive(Debug, Clone)]
pub struct InterestRateSwap<T: Float> {
    /// Notional principal amount.
    notional: T,
    /// Fixed rate leg.
    fixed_leg: FixedLeg<T>,
    /// Floating rate leg.
    floating_leg: FloatingLeg<T>,
    /// Settlement currency.
    currency: Currency,
    /// Swap direction (pay or receive fixed).
    direction: SwapDirection,
}

impl<T: Float> InterestRateSwap<T> {
    /// Create a new interest rate swap.
    ///
    /// # Arguments
    ///
    /// * `notional` - Notional principal amount
    /// * `fixed_leg` - Fixed rate leg
    /// * `floating_leg` - Floating rate leg
    /// * `currency` - Settlement currency
    /// * `direction` - Pay or receive fixed rate
    pub fn new(
        notional: T,
        fixed_leg: FixedLeg<T>,
        floating_leg: FloatingLeg<T>,
        currency: Currency,
        direction: SwapDirection,
    ) -> Self {
        Self {
            notional,
            fixed_leg,
            floating_leg,
            currency,
            direction,
        }
    }

    /// Returns the notional principal amount.
    #[inline]
    pub fn notional(&self) -> T {
        self.notional
    }

    /// Returns the fixed leg.
    #[inline]
    pub fn fixed_leg(&self) -> &FixedLeg<T> {
        &self.fixed_leg
    }

    /// Returns the floating leg.
    #[inline]
    pub fn floating_leg(&self) -> &FloatingLeg<T> {
        &self.floating_leg
    }

    /// Returns the settlement currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns the swap direction.
    #[inline]
    pub fn direction(&self) -> SwapDirection {
        self.direction
    }

    /// Returns the fixed rate.
    #[inline]
    pub fn fixed_rate(&self) -> T {
        self.fixed_leg.fixed_rate()
    }

    /// Returns the floating leg spread.
    #[inline]
    pub fn spread(&self) -> T {
        self.floating_leg.spread()
    }

    /// Returns the rate index for the floating leg.
    #[inline]
    pub fn index(&self) -> RateIndex {
        self.floating_leg.index()
    }

    /// Returns the maturity time in years (end of final period).
    ///
    /// This calculates the time from the start of the swap to the
    /// latest payment date across both legs.
    pub fn maturity(&self) -> T {
        // Get the first period start date as reference
        let start_date = self
            .fixed_leg
            .schedule()
            .periods()
            .first()
            .map(|p| p.start())
            .or_else(|| {
                self.floating_leg
                    .schedule()
                    .periods()
                    .first()
                    .map(|p| p.start())
            });

        // Get the last period end date
        let fixed_end = self.fixed_leg.schedule().periods().last().map(|p| p.end());

        let floating_end = self
            .floating_leg
            .schedule()
            .periods()
            .last()
            .map(|p| p.end());

        match (start_date, fixed_end, floating_end) {
            (Some(start), Some(fe), Some(fle)) => {
                let end = if fe > fle { fe } else { fle };
                let year_frac = DayCountConvention::ActualActual365.year_fraction_dates(start, end);
                T::from(year_frac).unwrap_or_else(T::zero)
            }
            (Some(start), Some(end), None) | (Some(start), None, Some(end)) => {
                let year_frac = DayCountConvention::ActualActual365.year_fraction_dates(start, end);
                T::from(year_frac).unwrap_or_else(T::zero)
            }
            _ => T::zero(),
        }
    }

    /// Returns whether this is a payer swap (pay fixed, receive floating).
    #[inline]
    pub fn is_payer(&self) -> bool {
        self.direction == SwapDirection::PayFixed
    }

    /// Returns whether this is a receiver swap (receive fixed, pay floating).
    #[inline]
    pub fn is_receiver(&self) -> bool {
        self.direction == SwapDirection::ReceiveFixed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedules::{Frequency, ScheduleBuilder};
    use pricer_core::types::time::Date;

    // ========================================
    // RateIndex Tests
    // ========================================

    #[test]
    fn test_rate_index_name() {
        assert_eq!(RateIndex::Sofr.name(), "SOFR");
        assert_eq!(RateIndex::Tonar.name(), "TONAR");
        assert_eq!(RateIndex::Euribor3M.name(), "EURIBOR3M");
        assert_eq!(RateIndex::Euribor6M.name(), "EURIBOR6M");
        assert_eq!(RateIndex::Sonia.name(), "SONIA");
        assert_eq!(RateIndex::Saron.name(), "SARON");
    }

    #[test]
    fn test_rate_index_tenor() {
        assert_eq!(RateIndex::Sofr.tenor_months(), 0);
        assert_eq!(RateIndex::Tonar.tenor_months(), 0);
        assert_eq!(RateIndex::Euribor3M.tenor_months(), 3);
        assert_eq!(RateIndex::Euribor6M.tenor_months(), 6);
        assert_eq!(RateIndex::Sonia.tenor_months(), 0);
        assert_eq!(RateIndex::Saron.tenor_months(), 0);
    }

    #[test]
    fn test_rate_index_is_overnight() {
        assert!(RateIndex::Sofr.is_overnight());
        assert!(RateIndex::Tonar.is_overnight());
        assert!(!RateIndex::Euribor3M.is_overnight());
        assert!(!RateIndex::Euribor6M.is_overnight());
        assert!(RateIndex::Sonia.is_overnight());
        assert!(RateIndex::Saron.is_overnight());
    }

    #[test]
    fn test_rate_index_currency() {
        assert_eq!(RateIndex::Sofr.currency(), Currency::USD);
        assert_eq!(RateIndex::Tonar.currency(), Currency::JPY);
        assert_eq!(RateIndex::Euribor3M.currency(), Currency::EUR);
        assert_eq!(RateIndex::Euribor6M.currency(), Currency::EUR);
        assert_eq!(RateIndex::Sonia.currency(), Currency::GBP);
        assert_eq!(RateIndex::Saron.currency(), Currency::CHF);
    }

    #[test]
    fn test_rate_index_from_str() {
        assert_eq!("SOFR".parse::<RateIndex>().unwrap(), RateIndex::Sofr);
        assert_eq!("sofr".parse::<RateIndex>().unwrap(), RateIndex::Sofr);
        assert_eq!("TONAR".parse::<RateIndex>().unwrap(), RateIndex::Tonar);
        assert_eq!("TONA".parse::<RateIndex>().unwrap(), RateIndex::Tonar);
        assert_eq!(
            "EURIBOR3M".parse::<RateIndex>().unwrap(),
            RateIndex::Euribor3M
        );
        assert_eq!("EUR6M".parse::<RateIndex>().unwrap(), RateIndex::Euribor6M);
        assert_eq!("SONIA".parse::<RateIndex>().unwrap(), RateIndex::Sonia);
        assert_eq!("SARON".parse::<RateIndex>().unwrap(), RateIndex::Saron);
    }

    #[test]
    fn test_rate_index_from_str_invalid() {
        assert!("INVALID".parse::<RateIndex>().is_err());
        assert!("LIBOR".parse::<RateIndex>().is_err());
    }

    #[test]
    fn test_rate_index_display() {
        assert_eq!(format!("{}", RateIndex::Sofr), "SOFR");
        assert_eq!(format!("{}", RateIndex::Euribor6M), "EURIBOR6M");
    }

    #[test]
    fn test_rate_index_default_day_count() {
        assert_eq!(
            RateIndex::Sofr.default_day_count(),
            DayCountConvention::ActualActual360
        );
        assert_eq!(
            RateIndex::Tonar.default_day_count(),
            DayCountConvention::ActualActual365
        );
    }

    // ========================================
    // SwapDirection Tests
    // ========================================

    #[test]
    fn test_swap_direction_multipliers() {
        assert_eq!(SwapDirection::PayFixed.fixed_multiplier::<f64>(), -1.0);
        assert_eq!(SwapDirection::PayFixed.floating_multiplier::<f64>(), 1.0);
        assert_eq!(SwapDirection::ReceiveFixed.fixed_multiplier::<f64>(), 1.0);
        assert_eq!(
            SwapDirection::ReceiveFixed.floating_multiplier::<f64>(),
            -1.0
        );
    }

    #[test]
    fn test_swap_direction_display() {
        assert_eq!(format!("{}", SwapDirection::PayFixed), "Pay Fixed");
        assert_eq!(format!("{}", SwapDirection::ReceiveFixed), "Receive Fixed");
    }

    // ========================================
    // FixedLeg Tests
    // ========================================

    fn create_test_schedule() -> Schedule {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2026, 1, 15).unwrap();

        ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::SemiAnnual)
            .day_count(DayCountConvention::Thirty360)
            .build()
            .unwrap()
    }

    #[test]
    fn test_fixed_leg_new() {
        let schedule = create_test_schedule();
        let fixed_leg = FixedLeg::new(schedule, 0.03, DayCountConvention::Thirty360);

        assert!((fixed_leg.fixed_rate() - 0.03).abs() < 1e-10);
        assert_eq!(fixed_leg.day_count(), DayCountConvention::Thirty360);
        assert_eq!(fixed_leg.num_periods(), 4); // 2 years, semi-annual = 4 periods
    }

    #[test]
    fn test_fixed_leg_accessors() {
        let schedule = create_test_schedule();
        let fixed_leg = FixedLeg::new(schedule, 0.05, DayCountConvention::ActualActual365);

        assert!((fixed_leg.fixed_rate() - 0.05).abs() < 1e-10);
        assert_eq!(fixed_leg.day_count(), DayCountConvention::ActualActual365);
        assert!(!fixed_leg.schedule().periods().is_empty());
    }

    // ========================================
    // FloatingLeg Tests
    // ========================================

    #[test]
    fn test_floating_leg_new() {
        let schedule = create_test_schedule();
        let floating_leg = FloatingLeg::new(
            schedule,
            0.001, // 10bp spread
            RateIndex::Sofr,
            DayCountConvention::ActualActual360,
        );

        assert!((floating_leg.spread() - 0.001).abs() < 1e-10);
        assert_eq!(floating_leg.index(), RateIndex::Sofr);
        assert_eq!(
            floating_leg.day_count(),
            DayCountConvention::ActualActual360
        );
        assert_eq!(floating_leg.num_periods(), 4);
    }

    #[test]
    fn test_floating_leg_accessors() {
        let schedule = create_test_schedule();
        let floating_leg = FloatingLeg::new(
            schedule,
            0.005,
            RateIndex::Euribor6M,
            DayCountConvention::ActualActual360,
        );

        assert!((floating_leg.spread() - 0.005).abs() < 1e-10);
        assert_eq!(floating_leg.index(), RateIndex::Euribor6M);
        assert!(!floating_leg.schedule().periods().is_empty());
    }

    // ========================================
    // InterestRateSwap Tests
    // ========================================

    fn create_test_swap() -> InterestRateSwap<f64> {
        let schedule = create_test_schedule();

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

    #[test]
    fn test_irs_new() {
        let swap = create_test_swap();

        assert!((swap.notional() - 1_000_000.0).abs() < 1e-6);
        assert_eq!(swap.currency(), Currency::USD);
        assert_eq!(swap.direction(), SwapDirection::PayFixed);
    }

    #[test]
    fn test_irs_fixed_rate() {
        let swap = create_test_swap();
        assert!((swap.fixed_rate() - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_irs_spread() {
        let swap = create_test_swap();
        assert!((swap.spread() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_irs_index() {
        let swap = create_test_swap();
        assert_eq!(swap.index(), RateIndex::Sofr);
    }

    #[test]
    fn test_irs_maturity() {
        let swap = create_test_swap();
        // 2 years from schedule
        let maturity = swap.maturity();
        assert!(maturity > 1.0 && maturity < 3.0);
    }

    #[test]
    fn test_irs_is_payer_receiver() {
        let payer_swap = create_test_swap();
        assert!(payer_swap.is_payer());
        assert!(!payer_swap.is_receiver());

        let schedule = create_test_schedule();
        let fixed_leg = FixedLeg::new(schedule.clone(), 0.03, DayCountConvention::Thirty360);
        let floating_leg = FloatingLeg::new(
            schedule,
            0.0,
            RateIndex::Sofr,
            DayCountConvention::ActualActual360,
        );

        let receiver_swap = InterestRateSwap::new(
            1_000_000.0,
            fixed_leg,
            floating_leg,
            Currency::USD,
            SwapDirection::ReceiveFixed,
        );

        assert!(!receiver_swap.is_payer());
        assert!(receiver_swap.is_receiver());
    }

    #[test]
    fn test_irs_legs() {
        let swap = create_test_swap();

        assert!((swap.fixed_leg().fixed_rate() - 0.03).abs() < 1e-10);
        assert_eq!(swap.floating_leg().index(), RateIndex::Sofr);
    }

    #[test]
    fn test_irs_clone() {
        let swap1 = create_test_swap();
        let swap2 = swap1.clone();

        assert!((swap1.notional() - swap2.notional()).abs() < 1e-10);
        assert_eq!(swap1.currency(), swap2.currency());
        assert_eq!(swap1.direction(), swap2.direction());
    }

    #[test]
    fn test_irs_debug() {
        let swap = create_test_swap();
        let debug_str = format!("{:?}", swap);
        assert!(debug_str.contains("InterestRateSwap"));
        assert!(debug_str.contains("notional"));
    }
}
