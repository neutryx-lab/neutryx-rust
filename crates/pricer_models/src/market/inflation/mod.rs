//! Inflation curve abstractions and implementations.
//!
//! Provides:
//! * [`InflationCurve`] — trait for inflation forward rate queries
//! * [`InflationCurveItp`] — cubic-spline / linear interpolated curve with
//!   seasonal adjustment
//! * [`InflationSeasonalFactor`] — monthly CPI seasonal factors
//! * [`ShiftRange`] / [`make_zero_rate_shifter`] — zero-rate bumping utilities

pub mod interpolated;
pub mod seasonality;
pub mod shift;

use enum_dispatch::enum_dispatch;
use num_traits::Float;

use super::MarketDataError;
use infra_domain::time::Date;

pub use interpolated::{InflationCurveItp, InflationInterpolation};
pub use seasonality::InflationSeasonalFactor;
pub use shift::{make_zero_rate_shifter, ShiftRange, ZeroRateShiftMode};

// ─── Absolute month utility ──────────────────────────────────────────

/// Converts a [`Date`] to an absolute month number.
///
/// Formula: `year * 12 + (month - 1)`.
///
/// This provides a monotonic integer axis for monthly inflation data,
/// suitable as the X-axis for interpolation.
#[inline]
#[must_use]
pub fn absolute_month(date: Date) -> i32 {
    date.year() * 12 + (date.month() as i32 - 1)
}

/// Inverse of [`absolute_month`]: reconstructs the (year, month) pair.
///
/// Returns month in `1..=12` range.
#[inline]
#[must_use]
pub fn year_month_from_absolute(abs_month: i32) -> (i32, u32) {
    let year = abs_month.div_euclid(12);
    let month = (abs_month.rem_euclid(12) + 1) as u32;
    (year, month)
}

// ─── Trait ───────────────────────────────────────────────────────────

/// Core trait for inflation forward curves.
///
/// Provides forward inflation rates (annualised) and the base index value
/// at the curve's reference date.  Two rate methods are exposed: one with
/// seasonal adjustment applied, one without.
#[enum_dispatch]
pub trait InflationCurve<T: Float> {
    /// Returns the seasonally adjusted forward inflation rate for the
    /// month containing `date`.
    fn forward_rate(&self, date: Date) -> Result<T, MarketDataError>;

    /// Returns the unadjusted (base) forward inflation rate before
    /// seasonal adjustment.
    fn unadjusted_forward_rate(&self, date: Date) -> Result<T, MarketDataError>;

    /// Returns the base CPI/RPI index value at the curve's reference date.
    fn base_index_value(&self) -> T;

    /// Returns the reference (base) date of this curve.
    fn reference_date(&self) -> Date;
}

// ─── Dispatch enum ──────────────────────────────────────────────────

/// Static-dispatch enum wrapping all supported inflation curve types.
///
/// Uses `enum_dispatch` for zero-cost polymorphism (Enzyme-friendly).
#[derive(Debug, Clone)]
#[enum_dispatch(InflationCurve<T>)]
pub enum InflationCurveEnum<T: Float> {
    /// Interpolated inflation curve with optional seasonality.
    Interpolated(InflationCurveItp<T>),
}

impl<T: Float> InflationCurveEnum<T> {
    /// Wraps an interpolated inflation curve.
    pub fn interpolated(curve: InflationCurveItp<T>) -> Self {
        Self::Interpolated(curve)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_absolute_month_jan_2024() {
        let d = Date::from_ymd(2024, 1, 15).unwrap();
        assert_eq!(absolute_month(d), 2024 * 12);
    }

    #[test]
    fn test_absolute_month_dec_2024() {
        let d = Date::from_ymd(2024, 12, 1).unwrap();
        assert_eq!(absolute_month(d), 2024 * 12 + 11);
    }

    #[test]
    fn test_absolute_month_roundtrip() {
        for year in [2020, 2024, 2030] {
            for month in 1..=12u32 {
                let d = Date::from_ymd(year, month, 1).unwrap();
                let abs = absolute_month(d);
                let (y, m) = year_month_from_absolute(abs);
                assert_eq!(y, year);
                assert_eq!(m, month);
            }
        }
    }

    #[test]
    fn test_year_month_from_absolute_edge() {
        let (y, m) = year_month_from_absolute(0);
        assert_eq!(y, 0);
        assert_eq!(m, 1);
    }

    #[test]
    fn test_enum_dispatch_forward_rate() {
        let d = Date::from_ymd(2024, 6, 15).unwrap();
        let seasonal = InflationSeasonalFactor::<f64>::identity();
        let curve = InflationCurveItp::new(
            vec![absolute_month(d)],
            vec![0.025],
            300.0,
            d,
            seasonal,
            InflationInterpolation::Linear,
        )
        .unwrap();
        let e: InflationCurveEnum<f64> = InflationCurveEnum::interpolated(curve);
        let rate = e.forward_rate(d).unwrap();
        assert!((rate - 0.025).abs() < 1e-12);
    }
}
