//! Pricing kernel implementation.

use infra_domain::time::Date;

/// Returns the number of days between two dates.
fn days_between(start: Date, end: Date) -> i32 {
    (end.into_inner() - start.into_inner()).num_days() as i32
}

/// Returns a date value suitable for date_to_ymd conversion.
fn date_to_days(date: Date) -> i32 {
    let epoch = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    (date.into_inner() - epoch).num_days() as i32
}

/// Adds days to a date.
fn add_days_to_date(date: Date, days: i32) -> Date {
    let new_naive = date.into_inner() + chrono::Duration::days(i64::from(days));
    Date::from_ymd(new_naive.year(), new_naive.month(), new_naive.day()).expect("valid date")
}

use chrono::Datelike;

/// Day count convention for year fraction calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DayCountConvention {
    /// Actual/365 Fixed (ACT/365F)
    #[default]
    Actual365Fixed,

    /// Actual/360 (ACT/360)
    Actual360,

    /// 30/360 (Bond basis)
    Thirty360,

    /// Actual/Actual ISDA
    ActualActualIsda,
}

impl DayCountConvention {
    /// Calculates the year fraction between two dates.
    pub fn year_fraction(&self, start: Date, end: Date) -> f64 {
        let days = self.day_count(start, end);

        match self {
            Self::Actual365Fixed => days as f64 / 365.0,
            Self::Actual360 => days as f64 / 360.0,
            Self::Thirty360 => {
                days as f64 / 360.0
            }
            Self::ActualActualIsda => {
                days as f64 / 365.25
            }
        }
    }

    /// Calculates the day count between two dates.
    fn day_count(&self, start: Date, end: Date) -> i32 {
        match self {
            Self::Thirty360 => self.thirty360_days(start, end),
            _ => days_between(start, end),
        }
    }

    /// Calculates 30/360 day count.
    fn thirty360_days(&self, start: Date, end: Date) -> i32 {
        let (y1, m1, d1) = date_to_ymd(start);
        let (y2, m2, d2) = date_to_ymd(end);

        let d1_adj = d1.min(30);
        let d2_adj = if d1_adj == 30 { d2.min(30) } else { d2 };

        360 * (y2 - y1) + 30 * (m2 - m1) + (d2_adj - d1_adj)
    }
}

/// Converts a Date to (year, month, day).
fn date_to_ymd(date: Date) -> (i32, i32, i32) {
    let days = date_to_days(date);
    let year = 2000 + days / 365;
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    (year, month.min(12), day.min(28))
}

/// Payment frequency for scheduled cashflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Frequency {
    /// Once (bullet payment)
    Once,
    /// Annual payments
    #[default]
    Annual,
    /// Semi-annual payments
    SemiAnnual,
    /// Quarterly payments
    Quarterly,
    /// Monthly payments
    Monthly,
}

impl Frequency {
    /// Returns the number of periods per year.
    pub fn periods_per_year(&self) -> u32 {
        match self {
            Self::Once => 1,
            Self::Annual => 1,
            Self::SemiAnnual => 2,
            Self::Quarterly => 4,
            Self::Monthly => 12,
        }
    }

    /// Returns the period length in months.
    pub fn period_months(&self) -> u32 {
        match self {
            Self::Once => 12,
            Self::Annual => 12,
            Self::SemiAnnual => 6,
            Self::Quarterly => 3,
            Self::Monthly => 1,
        }
    }
}

/// Business day adjustment convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BusinessDayConvention {
    /// No adjustment
    None,
    /// Move to the next business day
    #[default]
    Following,
    /// Move to the previous business day
    Preceding,
    /// Move to the next business day unless it changes the month,
    ModifiedFollowing,
    /// Move to the previous business day unless it changes the month,
    ModifiedPreceding,
}

impl BusinessDayConvention {
    /// Adjusts a date according to this convention.
    pub fn adjust(&self, date: Date) -> Date {
        match self {
            Self::None => date,
            Self::Following => self.next_business_day(date),
            Self::Preceding => self.prev_business_day(date),
            Self::ModifiedFollowing => {
                let adjusted = self.next_business_day(date);
                if month_of(adjusted) != month_of(date) {
                    self.prev_business_day(date)
                } else {
                    adjusted
                }
            }
            Self::ModifiedPreceding => {
                let adjusted = self.prev_business_day(date);
                if month_of(adjusted) != month_of(date) {
                    self.next_business_day(date)
                } else {
                    adjusted
                }
            }
        }
    }

    /// Finds the next business day (skipping weekends).
    fn next_business_day(&self, date: Date) -> Date {
        let mut d = date;
        while is_weekend(d) {
            d = add_days_to_date(d, 1);
        }
        d
    }

    /// Finds the previous business day (skipping weekends).
    fn prev_business_day(&self, date: Date) -> Date {
        let mut d = date;
        while is_weekend(d) {
            d = add_days_to_date(d, -1);
        }
        d
    }
}

/// Checks if a date is a weekend (Saturday or Sunday).
fn is_weekend(date: Date) -> bool {
    let days = date_to_days(date);
    let day_of_week = (days + 5) % 7;
    day_of_week >= 5
}

/// Gets the month of a date (1-12).
fn month_of(date: Date) -> i32 {
    let (_, m, _) = date_to_ymd(date);
    m
}

/// Discount factor calculator.
#[derive(Debug, Clone, Copy)]
pub struct DiscountCalculator {
    /// The continuously compounded rate.
    rate: f64,
}

impl DiscountCalculator {
    /// Creates a new discount calculator with a flat rate.
    pub fn with_flat_rate(rate: f64) -> Self { Self { rate } }

    /// Calculates the discount factor for a given time to maturity.
    pub fn discount_factor(&self, time_to_maturity: f64) -> f64 {
        (-self.rate * time_to_maturity).exp()
    }

    /// Calculates the discount factor between two dates.
    pub fn discount_factor_dates(
        &self,
        valuation_date: Date,
        payment_date: Date,
        day_count: DayCountConvention,
    ) -> f64 {
        let year_frac = day_count.year_fraction(valuation_date, payment_date);
        self.discount_factor(year_frac)
    }
}

/// Prices a single cashflow.
pub fn price_cashflow(
    amount: f64,
    payment_date: Date,
    valuation_date: Date,
    discount_rate: f64,
    day_count: DayCountConvention,
) -> f64 {
    if payment_date <= valuation_date {
        return 0.0;
    }

    let calculator = DiscountCalculator::with_flat_rate(discount_rate);
    let df = calculator.discount_factor_dates(valuation_date, payment_date, day_count);
    amount * df
}

/// Prices a stream of cashflows.
pub fn price_cashflow_stream<I>(
    cashflows: I,
    valuation_date: Date,
    discount_rate: f64,
    day_count: DayCountConvention,
) -> f64
where
    I: Iterator<Item = (f64, Date)>,
{
    cashflows
        .map(|(amount, payment_date)| {
            price_cashflow(
                amount,
                payment_date,
                valuation_date,
                discount_rate,
                day_count,
            )
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_count_actual365() {
        let dc = DayCountConvention::Actual365Fixed;
        let (start, end) = (
            Date::from_ymd(2023, 1, 1).unwrap(),
            Date::from_ymd(2024, 1, 1).unwrap(),
        );
        let yf = dc.year_fraction(start, end);
        assert!((yf - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_day_count_actual360() {
        let dc = DayCountConvention::Actual360;
        let (start, end) = (
            Date::from_ymd(2023, 1, 1).unwrap(),
            Date::from_ymd(2023, 12, 27).unwrap(),
        );
        let yf = dc.year_fraction(start, end);
        assert!((yf - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_day_count_half_year() {
        let dc = DayCountConvention::Actual365Fixed;
        let start = Date::from_ymd(2024, 1, 1).unwrap();
        let end = Date::from_ymd(2024, 7, 1).unwrap();
        let yf = dc.year_fraction(start, end);
        assert!(yf > 0.49 && yf < 0.51);
    }

    #[test]
    fn test_frequency_periods() {
        assert_eq!(Frequency::Annual.periods_per_year(), 1);
        assert_eq!(Frequency::SemiAnnual.periods_per_year(), 2);
        assert_eq!(Frequency::Quarterly.periods_per_year(), 4);
        assert_eq!(Frequency::Monthly.periods_per_year(), 12);
    }

    #[test]
    fn test_frequency_months() {
        assert_eq!(Frequency::Annual.period_months(), 12);
        assert_eq!(Frequency::SemiAnnual.period_months(), 6);
        assert_eq!(Frequency::Quarterly.period_months(), 3);
        assert_eq!(Frequency::Monthly.period_months(), 1);
    }

    #[test]
    fn test_discount_calculator_flat() {
        let calc = DiscountCalculator::with_flat_rate(0.05);

        let df = calc.discount_factor(1.0);
        assert!((df - 0.9512).abs() < 0.001);

        let df0 = calc.discount_factor(0.0);
        assert!((df0 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_discount_calculator_dates() {
        let calc = DiscountCalculator::with_flat_rate(0.05);
        let dc = DayCountConvention::Actual365Fixed;

        let val_date = Date::from_ymd(2024, 1, 1).unwrap();
        let pay_date = Date::from_ymd(2025, 1, 1).unwrap();

        let df = calc.discount_factor_dates(val_date, pay_date, dc);
        assert!((df - 0.9512).abs() < 0.001);
    }

    #[test]
    fn test_price_cashflow() {
        let amount = 100_000.0;
        let payment_date = Date::from_ymd(2025, 1, 1).unwrap();
        let valuation_date = Date::from_ymd(2024, 1, 1).unwrap();
        let rate = 0.05;
        let dc = DayCountConvention::Actual365Fixed;

        let pv = price_cashflow(amount, payment_date, valuation_date, rate, dc);

        assert!(pv > 95_000.0 && pv < 96_000.0);
    }

    #[test]
    fn test_price_cashflow_past() {
        let amount = 100_000.0;
        let payment_date = Date::from_ymd(2024, 2, 20).unwrap();
        let valuation_date = Date::from_ymd(2024, 4, 10).unwrap();
        let rate = 0.05;
        let dc = DayCountConvention::Actual365Fixed;

        let pv = price_cashflow(amount, payment_date, valuation_date, rate, dc);

        assert!(pv.abs() < 1e-10);
    }

    #[test]
    fn test_price_cashflow_stream() {
        let valuation_date = Date::from_ymd(2024, 1, 1).unwrap();
        let rate = 0.05;
        let dc = DayCountConvention::Actual365Fixed;

        let cashflows = vec![
            (100_000.0, Date::from_ymd(2025, 1, 1).unwrap()),
            (100_000.0, Date::from_ymd(2026, 1, 1).unwrap()),
        ];

        let pv = price_cashflow_stream(cashflows.into_iter(), valuation_date, rate, dc);

        assert!(pv > 185_000.0 && pv < 186_500.0);
    }

    #[test]
    fn test_business_day_following() {
        let conv = BusinessDayConvention::Following;

        let monday = Date::from_ymd(2024, 1, 8).unwrap();
        let adjusted = conv.adjust(monday);
        assert_eq!(adjusted, monday);
    }

    #[test]
    fn test_business_day_none() {
        let conv = BusinessDayConvention::None;

        let date = Date::from_ymd(2024, 1, 1).unwrap();
        let adjusted = conv.adjust(date);
        assert_eq!(adjusted, date);
    }

    #[test]
    fn test_day_count_default() {
        let dc = DayCountConvention::default();
        assert_eq!(dc, DayCountConvention::Actual365Fixed);
    }

    #[test]
    fn test_frequency_default() {
        let freq = Frequency::default();
        assert_eq!(freq, Frequency::Annual);
    }

    #[test]
    fn test_business_day_default() {
        let conv = BusinessDayConvention::default();
        assert_eq!(conv, BusinessDayConvention::Following);
    }
}
