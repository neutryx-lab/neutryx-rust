//! Chart grid generation helpers for curve visualisation.

use chrono::{Datelike, Months, NaiveDate};
use infra_domain::{
    market::RateIndex,
    time::{Date, DayCounter},
};
use pricer_models::market::YieldCurve;

use crate::rest::dto::ChartGridPoint;

const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Internal model time axis: ACT/365 Fixed.
///
/// All time conversions in this module use this day counter,
/// ensuring consistency with the `pricer_models` internal basis.
pub(crate) const MODEL_DAY_COUNTER: DayCounter = DayCounter::Actual365Fixed;

/// Format date for short-term chart axis: "15-Jan"
fn format_short_term_label(date: NaiveDate) -> String {
    format!("{}-{}", date.day(), MONTH_ABBR[date.month0() as usize])
}

/// Format date for long-term chart axis: "Mar-26"
fn format_long_term_label(date: NaiveDate) -> String {
    format!(
        "{}-{:02}",
        MONTH_ABBR[date.month0() as usize],
        date.year() % 100
    )
}

/// Resolve the day count convention from the request index name.
///
/// Falls back to ACT/365 Fixed if the index is not recognised.
pub(crate) fn resolve_day_counter(index: &str) -> DayCounter {
    RateIndex::from_index_name(index)
        .map(|ri| ri.day_counter())
        .unwrap_or(DayCounter::Actual365Fixed)
}

/// Compute the overnight forward rate at a given date.
///
/// Forward rate F = (DF₁ / DF₂ − 1) / δ where δ uses the index day counter.
pub(crate) fn overnight_forward_rate<C: YieldCurve<f64>>(
    curve: &C,
    ref_date: NaiveDate,
    date: NaiveDate,
    day_counter: DayCounter,
) -> Option<f64> {
    let d = (date - ref_date).num_days();
    let next_date = date + chrono::Duration::days(1);
    let t1 = MODEL_DAY_COUNTER.year_fraction_from_days(d);
    let t2 = MODEL_DAY_COUNTER.year_fraction_from_days(d + 1);
    let df1 = if t1 <= 0.0 {
        1.0
    } else {
        curve.discount_factor(t1).ok()?
    };
    let df2 = curve.discount_factor(t2).ok()?;
    let delta = day_counter.year_fraction(Date::from(date), Date::from(next_date));
    if delta <= 0.0 {
        return None;
    }
    Some((df1 / df2 - 1.0) / delta)
}

/// Generate short-term grid dates: daily up to 3M, weekly 3M→1Y.
fn generate_short_term_dates(ref_date: NaiveDate) -> Vec<NaiveDate> {
    let three_months = ref_date
        .checked_add_months(Months::new(3))
        .unwrap_or(ref_date);
    let one_year = ref_date
        .checked_add_months(Months::new(12))
        .unwrap_or(ref_date);

    let mut dates = Vec::new();

    // Daily up to 3M
    let mut d = ref_date + chrono::Duration::days(1);
    while d <= three_months {
        dates.push(d);
        d += chrono::Duration::days(1);
    }

    // Weekly from 3M to 1Y
    d = three_months + chrono::Duration::days(7);
    while d <= one_year {
        dates.push(d);
        d += chrono::Duration::days(7);
    }

    dates
}

/// Generate long-term grid dates: quarterly 3M→10Y, semi-annual 10.5Y→20Y,
/// annual 21Y→30Y.
fn generate_long_term_dates(ref_date: NaiveDate) -> Vec<NaiveDate> {
    let mut dates = Vec::new();

    // Quarterly from 3M (q=1) to 10Y (q=40)
    for q in 1..=40u32 {
        if let Some(d) = ref_date.checked_add_months(Months::new(q * 3)) {
            dates.push(d);
        }
    }

    // Semi-annual from 10.5Y (h=21) to 20Y (h=40)
    for h in 21..=40u32 {
        if let Some(d) = ref_date.checked_add_months(Months::new(h * 6)) {
            dates.push(d);
        }
    }

    // Annual from 21Y to 30Y
    for y in 21..=30u32 {
        if let Some(d) = ref_date.checked_add_months(Months::new(y * 12)) {
            dates.push(d);
        }
    }

    dates
}

/// Build `ChartGridPoint` vec from grid dates.
fn build_chart_grid<C: YieldCurve<f64>>(
    ref_date: NaiveDate,
    dates: &[NaiveDate],
    curve: &C,
    label_fn: fn(NaiveDate) -> String,
    day_counter: DayCounter,
) -> Vec<ChartGridPoint> {
    dates
        .iter()
        .filter_map(|date| {
            let time = MODEL_DAY_COUNTER.year_fraction_from_days((*date - ref_date).num_days());
            let df = curve.discount_factor(time).ok()?;
            let fwd = if time > 0.0 {
                overnight_forward_rate(curve, ref_date, *date, day_counter)?
            } else {
                0.0
            };
            Some(ChartGridPoint {
                date: date.format("%Y-%m-%d").to_string(),
                time,
                discount_factor: df,
                forward_rate: fwd,
                label: label_fn(*date),
            })
        })
        .collect()
}

/// Generate short-term and long-term chart grids for a curve.
pub(crate) fn generate_chart_grids<C: YieldCurve<f64>>(
    ref_date: NaiveDate,
    curve: &C,
    day_counter: DayCounter,
) -> (Vec<ChartGridPoint>, Vec<ChartGridPoint>) {
    let short_term_dates = generate_short_term_dates(ref_date);
    let long_term_dates = generate_long_term_dates(ref_date);
    let short_term_grid = build_chart_grid(
        ref_date,
        &short_term_dates,
        curve,
        format_short_term_label,
        day_counter,
    );
    let long_term_grid = build_chart_grid(
        ref_date,
        &long_term_dates,
        curve,
        format_long_term_label,
        day_counter,
    );
    (short_term_grid, long_term_grid)
}
