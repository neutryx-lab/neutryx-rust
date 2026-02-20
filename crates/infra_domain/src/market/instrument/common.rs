//! Common types shared across instrument definitions.

use crate::time::{AccrualPeriod, Date, EndOfMonthRule, Frequency, Period, Tenor, TimeUnit};

/// Asset class categorisation for financial instruments.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    strum::Display,
    strum::AsRefStr,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    /// Interest rate instruments (swaps, swaptions, caps/floors, etc.).
    Rates,
    /// Foreign exchange instruments (spots, forwards, options).
    #[strum(serialize = "FX")]
    Fx,
    /// Equity instruments (forwards, options, swaps).
    Equity,
    /// Credit instruments (CDS, CDX, etc.).
    Credit,
    /// Commodity instruments (forwards, swaps, options).
    Commodity,
}

/// Exercise style for option instruments.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, serde::Serialize, serde::Deserialize,
)]
pub enum ExerciseStyle {
    /// European: exercise only at expiry.
    European,
    /// American: exercise at any time until expiry.
    American,
    /// Bermudan: exercise at specific dates.
    Bermudan,
    /// Asian: payoff depends on average price.
    Asian,
}

impl ExerciseStyle {
    /// Returns true if this is European exercise.
    #[inline]
    #[must_use]
    pub fn is_european(&self) -> bool { matches!(self, Self::European) }

    /// Returns true if this is American exercise.
    #[inline]
    #[must_use]
    pub fn is_american(&self) -> bool { matches!(self, Self::American) }

    /// Returns true if this is Bermudan exercise.
    #[inline]
    #[must_use]
    pub fn is_bermudan(&self) -> bool { matches!(self, Self::Bermudan) }

    /// Returns true if this is Asian exercise.
    #[inline]
    #[must_use]
    pub fn is_asian(&self) -> bool { matches!(self, Self::Asian) }
}

/// Payer/Receiver position indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PayerReceiver {
    /// Payer position (pay fixed, receive floating in swap context).
    Payer,
    /// Receiver position (receive fixed, pay floating in swap context).
    Receiver,
}

impl PayerReceiver {
    /// Returns the opposite position.
    #[must_use]
    pub fn opposite(&self) -> Self {
        match self {
            PayerReceiver::Payer => PayerReceiver::Receiver,
            PayerReceiver::Receiver => PayerReceiver::Payer,
        }
    }

    /// Returns 1.0 for Payer, -1.0 for Receiver.
    #[must_use]
    pub fn sign(&self) -> f64 {
        match self {
            PayerReceiver::Payer => 1.0,
            PayerReceiver::Receiver => -1.0,
        }
    }
}

/// Barrier type for barrier options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BarrierType {
    /// Knock-in: option becomes active when barrier is breached.
    KnockIn,
    /// Knock-out: option becomes void when barrier is breached.
    KnockOut,
}

/// Barrier direction for barrier options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BarrierDirection {
    /// Up barrier: triggered when spot rises above barrier level.
    Up,
    /// Down barrier: triggered when spot falls below barrier level.
    Down,
}

/// ATM strike convention for FX options (from FxStraddleConvention).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtmConvention {
    /// ATM Forward strike.
    AtmForward,
    /// Delta-neutral straddle in percentage terms.
    DeltaNeutralPercent,
    /// Delta-neutral straddle in pips terms.
    DeltaNeutralPips,
}

/// Expiry/delivery date adjustment convention for FX options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpiryDeliveryAdjust {
    /// Forward adjustment (standard: expiry and delivery derived from forward).
    Forward,
    /// Premium adjustment (expiry/delivery adjusted for premium payment).
    Premium,
}

/// Inflation index calculation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InflationIndexType {
    /// Monthly index (no interpolation within month).
    Monthly,
    /// Interpolated index (linear interpolation between months).
    Interpolated,
}

/// CDS convention type (ISDA vs SMBC/CREDIS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CdsType {
    /// ISDA standard single-name CDS.
    Isda,
    /// SMBC/CREDIS convention CDS.
    Smbc,
}

/// Notional schedule for amortising instruments.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NotionalSchedule {
    /// Notional amounts for each period.
    pub notionals: Vec<f64>,
}

impl NotionalSchedule {
    /// Creates a constant notional schedule.
    #[must_use]
    pub fn constant(notional: f64) -> Self {
        Self {
            notionals: vec![notional],
        }
    }

    /// Creates a notional schedule from a vector of amounts.
    #[must_use]
    pub fn from_schedule(notionals: Vec<f64>) -> Self { Self { notionals } }

    /// Returns the notional for a given period index.
    #[must_use]
    pub fn notional_at(&self, period_index: usize) -> f64 {
        self.notionals
            .get(period_index)
            .or_else(|| self.notionals.last())
            .copied()
            .unwrap_or(0.0)
    }

    /// Returns true if this is a constant notional schedule.
    #[must_use]
    pub fn is_constant(&self) -> bool {
        self.notionals.len() <= 1
            || self
                .notionals
                .windows(2)
                .all(|w| (w[0] - w[1]).abs() < 1e-10)
    }
}

impl Default for NotionalSchedule {
    fn default() -> Self {
        Self {
            notionals: vec![1_000_000.0],
        }
    }
}

/// Payment schedule for fixed income instruments.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentSchedule {
    /// Accrual periods in the schedule.
    pub periods: Vec<AccrualPeriod>,
}

impl PaymentSchedule {
    /// Creates a new payment schedule from a list of accrual periods.
    #[must_use]
    pub fn new(periods: Vec<AccrualPeriod>) -> Self { Self { periods } }

    /// Creates an empty payment schedule.
    #[must_use]
    pub fn empty() -> Self { Self { periods: vec![] } }

    /// Generates a payment schedule from start/end dates and frequency.
    #[must_use]
    pub fn generate(start: Date, end: Date, frequency: Frequency, payment_lag: u32) -> Self {
        let mut periods = Vec::new();
        let period_months = frequency.months_per_period();

        if period_months == 0 {
            let payment = add_business_days(end, payment_lag);
            periods.push(AccrualPeriod::new(start, end, payment));
            return Self { periods };
        }

        let mut current_start = start;
        while current_start < end {
            let current_end = add_months_to_date(current_start, period_months);
            let actual_end = if current_end > end { end } else { current_end };
            let payment = add_business_days(actual_end, payment_lag);

            periods.push(AccrualPeriod::new(current_start, actual_end, payment));
            current_start = actual_end;

            if actual_end >= end {
                break;
            }
        }

        Self { periods }
    }

    /// Generates a schedule from a start date and tenor.
    #[must_use]
    pub fn generate_from_tenor(
        start: Date,
        tenor: Tenor,
        frequency: Frequency,
        payment_lag: u32,
    ) -> Self {
        let end = tenor.add_to_date(start, EndOfMonthRule::Adjust);
        Self::generate(start, end, frequency, payment_lag)
    }

    /// Returns the number of periods in the schedule.
    #[must_use]
    pub fn num_periods(&self) -> usize { self.periods.len() }

    /// Returns true if the schedule is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.periods.is_empty() }

    /// Returns the start date of the first period.
    #[must_use]
    pub fn start_date(&self) -> Option<Date> { self.periods.first().map(|p| p.start) }

    /// Returns the end date of the last period.
    #[must_use]
    pub fn end_date(&self) -> Option<Date> { self.periods.last().map(|p| p.end) }

    /// Returns all payment dates in the schedule.
    #[must_use]
    pub fn payment_dates(&self) -> Vec<Date> { self.periods.iter().map(|p| p.payment).collect() }
}

/// Helper function to add months to a date.
fn add_months_to_date(date: Date, months: u32) -> Date {
    date + Period::new(months as i32, TimeUnit::Months)
}

/// Helper function to add business days (simplified - ignores holidays).
fn add_business_days(date: Date, days: u32) -> Date {
    if days == 0 {
        return date;
    }
    date + Period::days(days as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enums_and_traits() {
        assert_eq!(AssetClass::Rates.to_string(), "Rates");
        assert_eq!(AssetClass::Fx.to_string(), "FX");
        assert_eq!(AssetClass::Equity.to_string(), "Equity");
        assert_eq!(AssetClass::Credit.to_string(), "Credit");
        assert_eq!(AssetClass::Commodity.to_string(), "Commodity");
        assert_eq!(AssetClass::Rates.as_ref(), "Rates");
        assert_eq!(AssetClass::Fx.as_ref(), "FX");
        assert_eq!(AssetClass::Rates, AssetClass::Rates);
        assert_ne!(AssetClass::Rates, AssetClass::Fx);
        let mut set = std::collections::HashSet::new();
        set.insert(AssetClass::Rates);
        set.insert(AssetClass::Fx);
        set.insert(AssetClass::Rates);
        assert_eq!(set.len(), 2);

        assert_eq!(ExerciseStyle::European, ExerciseStyle::European);
        assert_ne!(ExerciseStyle::European, ExerciseStyle::American);
        assert_eq!(PayerReceiver::Payer.opposite(), PayerReceiver::Receiver);
        assert_eq!(PayerReceiver::Receiver.opposite(), PayerReceiver::Payer);
        assert_eq!(PayerReceiver::Payer.sign(), 1.0);
        assert_eq!(PayerReceiver::Receiver.sign(), -1.0);
        assert_eq!(BarrierType::KnockIn, BarrierType::KnockIn);
        assert_ne!(BarrierType::KnockIn, BarrierType::KnockOut);
        assert_eq!(BarrierDirection::Up, BarrierDirection::Up);
        assert_ne!(BarrierDirection::Up, BarrierDirection::Down);
    }

    #[test]
    fn test_notional_schedule() {
        let c = NotionalSchedule::constant(1_000_000.0);
        assert!(c.is_constant());
        assert_eq!(c.notional_at(0), 1_000_000.0);
        assert_eq!(c.notional_at(10), 1_000_000.0);

        let a = NotionalSchedule::from_schedule(vec![1_000_000.0, 800_000.0, 600_000.0]);
        assert!(!a.is_constant());
        assert_eq!(a.notional_at(0), 1_000_000.0);
        assert_eq!(a.notional_at(1), 800_000.0);
        assert_eq!(a.notional_at(2), 600_000.0);
        assert_eq!(a.notional_at(5), 600_000.0);

        let d = NotionalSchedule::default();
        assert!(d.is_constant());
        assert_eq!(d.notional_at(0), 1_000_000.0);
        let cloned = NotionalSchedule::from_schedule(vec![100.0, 200.0]);
        assert_eq!(cloned.clone(), cloned);
    }

    #[test]
    fn test_payment_schedule() {
        let s = Date::from_ymd(2025, 1, 1).unwrap();

        let ann =
            PaymentSchedule::generate(s, Date::from_ymd(2030, 1, 1).unwrap(), Frequency::Annual, 0);
        assert_eq!(ann.num_periods(), 5);
        assert_eq!(ann.start_date(), Some(s));
        assert_eq!(ann.end_date(), Some(Date::from_ymd(2030, 1, 1).unwrap()));

        let semi = PaymentSchedule::generate(
            s,
            Date::from_ymd(2027, 1, 1).unwrap(),
            Frequency::SemiAnnual,
            0,
        );
        assert_eq!(semi.num_periods(), 4);

        let q = PaymentSchedule::generate(
            s,
            Date::from_ymd(2026, 1, 1).unwrap(),
            Frequency::Quarterly,
            0,
        );
        assert_eq!(q.num_periods(), 4);

        let lag =
            PaymentSchedule::generate(s, Date::from_ymd(2026, 1, 1).unwrap(), Frequency::Annual, 2);
        assert_eq!(lag.num_periods(), 1);
        assert_eq!(lag.payment_dates()[0], Date::from_ymd(2026, 1, 3).unwrap());

        let tenor = PaymentSchedule::generate_from_tenor(s, Tenor::FiveYears, Frequency::Annual, 0);
        assert_eq!(tenor.num_periods(), 5);

        let e = PaymentSchedule::empty();
        assert!(e.is_empty());
        assert_eq!(e.num_periods(), 0);
        assert_eq!(e.start_date(), None);
        assert_eq!(e.end_date(), None);
        assert_eq!(ann.clone(), ann);
    }
}
