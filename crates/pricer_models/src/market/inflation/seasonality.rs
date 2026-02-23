//! Monthly CPI seasonality adjustments for inflation curves.
//!
//! Stores 12 month-on-month (MoM) multiplicative factors and provides
//! cumulative seasonal multipliers for arbitrary dates via linear
//! interpolation on the cumulative factor curve.

use infra_domain::time::Date;
use num_traits::Float;
use pricer_core::math::numeric::from_f64;

use super::absolute_month;

/// Monthly seasonal multiplicative factors for an inflation index.
///
/// Each factor represents the multiplicative seasonal effect for that
/// calendar month relative to a non-seasonal baseline.  Typical range:
/// `[0.98, 1.02]`.
///
/// The cumulative factor from `base_month` to `target_month` is the
/// product of individual monthly factors over that interval.
#[derive(Debug, Clone)]
pub struct InflationSeasonalFactor<T: Float> {
    /// 12 MoM seasonal factors indexed 0..12 (Jan=0, Dec=11).
    monthly_factors: [T; 12],
}

impl<T: Float> InflationSeasonalFactor<T> {
    /// Creates a new seasonal factor from 12 MoM factors.
    ///
    /// # Panics
    ///
    /// Panics if any factor is non-positive.
    pub fn new(monthly_factors: [T; 12]) -> Self {
        for (i, &f) in monthly_factors.iter().enumerate() {
            assert!(
                f > T::zero(),
                "Monthly factor at index {} must be positive",
                i,
            );
        }
        Self { monthly_factors }
    }

    /// Returns a neutral (identity) seasonal factor where all months = 1.0.
    pub fn identity() -> Self {
        Self {
            monthly_factors: [T::one(); 12],
        }
    }

    /// Returns the MoM factor for a given month (1=Jan, 12=Dec).
    #[inline]
    pub fn monthly_factor(&self, month: u32) -> T {
        self.monthly_factors[((month - 1) % 12) as usize]
    }

    /// Returns the raw monthly factors array.
    pub fn monthly_factors(&self) -> &[T; 12] { &self.monthly_factors }

    /// Computes the cumulative seasonal multiplier from `base_month`
    /// (absolute month) to `target_month` (absolute month).
    ///
    /// The cumulative factor is the product of MoM factors for each
    /// month in `[base_month, target_month)`.
    pub fn cumulative_factor(&self, base_month: i32, target_month: i32) -> T {
        if target_month <= base_month {
            return T::one();
        }

        let mut product = T::one();
        for m in base_month..target_month {
            let month_in_year = (m.rem_euclid(12)) as usize;
            product = product * self.monthly_factors[month_in_year];
        }
        product
    }

    /// Returns the interpolated seasonal multiplier for a date, using
    /// linear interpolation on cumulative factors within the month.
    ///
    /// For a date falling on day `d` of month `m`, interpolates between
    /// the cumulative factor at the start of `m` and the start of `m+1`:
    ///
    /// ```text
    /// factor(d) = cum(base, m) * (1 - alpha) + cum(base, m+1) * alpha
    /// ```
    ///
    /// where `alpha = (d - 1) / days_in_month`.
    pub fn interpolated_multiplier(&self, base_month: i32, date: Date) -> T {
        let target_abs = absolute_month(date);
        let cum_start = self.cumulative_factor(base_month, target_abs);
        let cum_end = self.cumulative_factor(base_month, target_abs + 1);

        let day = date.day() as f64;
        let days_in_month = date.last_day_of_month().day() as f64;
        let alpha = from_f64::<T>((day - 1.0) / days_in_month);

        cum_start * (T::one() - alpha) + cum_end * alpha
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn us_cpi_factors() -> InflationSeasonalFactor<f64> {
        // Approximate US CPI-U seasonal pattern.
        InflationSeasonalFactor::new([
            0.9982, // Jan
            0.9996, // Feb
            1.0028, // Mar
            1.0028, // Apr
            1.0025, // May
            1.0010, // Jun
            0.9990, // Jul
            0.9992, // Aug
            1.0000, // Sep
            0.9995, // Oct
            0.9972, // Nov
            0.9982, // Dec
        ])
    }

    #[test]
    fn test_identity_factor() {
        let sf = InflationSeasonalFactor::<f64>::identity();
        for m in 1..=12 {
            assert!((sf.monthly_factor(m) - 1.0).abs() < 1e-15);
        }
    }

    #[test]
    fn test_monthly_factor_lookup() {
        let sf = us_cpi_factors();
        assert!((sf.monthly_factor(1) - 0.9982).abs() < 1e-10);
        assert!((sf.monthly_factor(6) - 1.0010).abs() < 1e-10);
        assert!((sf.monthly_factor(12) - 0.9982).abs() < 1e-10);
    }

    #[test]
    fn test_cumulative_factor_one_month() {
        let sf = us_cpi_factors();
        // base=0 (Jan), target=1 (Feb) => product of Jan factor only
        let cum = sf.cumulative_factor(0, 1);
        assert!((cum - 0.9982).abs() < 1e-10);
    }

    #[test]
    fn test_cumulative_factor_full_year() {
        let sf = us_cpi_factors();
        // Product of all 12 factors
        let expected: f64 = sf.monthly_factors().iter().product();
        let cum = sf.cumulative_factor(0, 12);
        assert!((cum - expected).abs() < 1e-12);
    }

    #[test]
    fn test_cumulative_factor_target_le_base() {
        let sf = us_cpi_factors();
        assert!((sf.cumulative_factor(5, 5) - 1.0).abs() < 1e-15);
        assert!((sf.cumulative_factor(5, 3) - 1.0).abs() < 1e-15);
    }

    #[test]
    fn test_interpolated_multiplier_first_day() {
        let sf = us_cpi_factors();
        // On first day of month, alpha ~ 0 => close to cum(base, target)
        let d = Date::from_ymd(2024, 3, 1).unwrap();
        let base_month = super::absolute_month(Date::from_ymd(2024, 1, 1).unwrap());
        let target_month = super::absolute_month(d);
        let cum = sf.cumulative_factor(base_month, target_month);
        let interp = sf.interpolated_multiplier(base_month, d);
        assert!((interp - cum).abs() < 0.001);
    }

    #[test]
    fn test_interpolated_multiplier_last_day() {
        let sf = us_cpi_factors();
        // On last day of month, alpha ~ 1 => close to cum(base, target+1)
        let d = Date::from_ymd(2024, 3, 31).unwrap();
        let base_month = super::absolute_month(Date::from_ymd(2024, 1, 1).unwrap());
        let target_month = super::absolute_month(d);
        let cum_end = sf.cumulative_factor(base_month, target_month + 1);
        let interp = sf.interpolated_multiplier(base_month, d);
        assert!((interp - cum_end).abs() < 0.01);
    }

    #[test]
    #[should_panic(expected = "must be positive")]
    fn test_non_positive_factor_panics() {
        InflationSeasonalFactor::new([
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0_f64,
        ]);
    }
}
