//! Interpolated inflation forward curve with seasonal adjustment and
//! in-place bump/reset support.
//!
//! The curve stores (absolute_month, annualised_forward_rate) pairs and
//! interpolates between them.  The seasonal factor is applied as a
//! multiplicative overlay on the unadjusted rate.
//!
//! Forward rate semantics: the rate at absolute month `m` represents the
//! annualised inflation rate for the period `[m, m+1]`.

use num_traits::Float;

use pricer_core::math::{interpolation::CubicSpline, numeric::from_f64};

use super::{
    absolute_month,
    seasonality::InflationSeasonalFactor,
    shift::{ShiftRange, ZeroRateShiftMode},
    MarketDataError,
};
use infra_domain::time::Date;

// ─── Interpolation method ───────────────────────────────────────────

/// Interpolation method for the inflation forward rate curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InflationInterpolation {
    /// Linear interpolation on absolute month axis.
    #[default]
    Linear,
    /// Natural cubic spline on absolute month axis.
    CubicSpline,
}

// ─── InflationCurveItp ──────────────────────────────────────────────

/// Interpolated inflation forward curve with seasonal adjustment.
///
/// ## Zero-Rate Conversion for Bumping
///
/// Instead of bumping forward rates directly (which introduces compounding
/// distortion across maturities), we convert to zero rates:
///
/// ```text
/// Z_i = ln(rate_i / base_rate) / t_i
/// ```
///
/// Apply the shift in zero-rate space, then restore:
///
/// ```text
/// rate_new_i = base_rate * exp(Z_shifted_i * t_i)
/// ```
///
/// ## State Backup/Restore
///
/// On the first bump, original `grid_rates` are copied to `org_rates`.
/// `reset_rate()` swaps them back via `std::mem::swap` (no reallocation).
#[derive(Debug, Clone)]
pub struct InflationCurveItp<T: Float> {
    /// Sorted grid of absolute months.
    grid_months: Vec<i32>,
    /// Forward rates corresponding to each grid month.
    grid_rates: Vec<T>,
    /// Backup of original rates before any shift.
    org_rates: Option<Vec<T>>,
    /// Base CPI/RPI value at the reference date.
    base_index: T,
    /// Reference date of the curve.
    reference_date: Date,
    /// Seasonal adjustment factors.
    seasonal: InflationSeasonalFactor<T>,
    /// Interpolation method.
    interpolation: InflationInterpolation,
    /// Pre-computed cubic spline (populated when interpolation = CubicSpline).
    spline: Option<CubicSpline<T>>,
}

impl<T: Float> InflationCurveItp<T> {
    /// Constructs a new interpolated inflation curve.
    ///
    /// # Errors
    ///
    /// Returns `MarketDataError::InvalidInput` if `grid_months` and
    /// `grid_rates` have different lengths, are empty, or if
    /// `grid_months` is not strictly increasing.
    pub fn new(
        grid_months: Vec<i32>,
        grid_rates: Vec<T>,
        base_index: T,
        reference_date: Date,
        seasonal: InflationSeasonalFactor<T>,
        interpolation: InflationInterpolation,
    ) -> Result<Self, MarketDataError> {
        if grid_months.len() != grid_rates.len() {
            return Err(MarketDataError::InvalidInput {
                message: "grid_months and grid_rates must have same length".into(),
            });
        }
        if grid_months.is_empty() {
            return Err(MarketDataError::InvalidInput {
                message: "inflation curve must have at least one grid point".into(),
            });
        }
        for w in grid_months.windows(2) {
            if w[1] <= w[0] {
                return Err(MarketDataError::InvalidInput {
                    message: "grid_months must be strictly increasing".into(),
                });
            }
        }

        let spline = if interpolation == InflationInterpolation::CubicSpline
            && grid_months.len() >= 2
        {
            let xs: Vec<T> = grid_months.iter().map(|&m| from_f64(m as f64)).collect();
            let ys = grid_rates.clone();
            Some(CubicSpline::natural(&xs, &ys).map_err(|e| {
                MarketDataError::InterpolationFailed {
                    reason: format!("Cubic spline construction failed: {}", e),
                }
            })?)
        } else {
            None
        };

        Ok(Self {
            grid_months,
            grid_rates,
            org_rates: None,
            base_index,
            reference_date,
            seasonal,
            interpolation,
            spline,
        })
    }

    /// Returns the grid of absolute months.
    pub fn grid_months(&self) -> &[i32] {
        &self.grid_months
    }

    /// Returns the current grid of forward rates (possibly shifted).
    pub fn grid_rates(&self) -> &[T] {
        &self.grid_rates
    }

    /// Returns a reference to the seasonal factor.
    pub fn seasonal(&self) -> &InflationSeasonalFactor<T> {
        &self.seasonal
    }

    /// Returns `true` if the curve is in a bumped state.
    pub fn is_bumped(&self) -> bool {
        self.org_rates.is_some()
    }

    // ── Interpolation ──────────────────────────────────────────────

    /// Interpolates the unadjusted rate at the given absolute month.
    fn interpolate_rate(&self, abs_month: i32) -> Result<T, MarketDataError> {
        let n = self.grid_months.len();
        let x = from_f64::<T>(abs_month as f64);

        if n == 1 {
            return Ok(self.grid_rates[0]);
        }

        match self.interpolation {
            InflationInterpolation::CubicSpline => {
                if let Some(ref spline) = self.spline {
                    Ok(spline.evaluate(x))
                } else {
                    self.linear_interpolate(abs_month)
                }
            }
            InflationInterpolation::Linear => self.linear_interpolate(abs_month),
        }
    }

    /// Linear interpolation with flat extrapolation at boundaries.
    fn linear_interpolate(&self, abs_month: i32) -> Result<T, MarketDataError> {
        let n = self.grid_months.len();

        // Flat extrapolation below
        if abs_month <= self.grid_months[0] {
            return Ok(self.grid_rates[0]);
        }
        // Flat extrapolation above
        if abs_month >= self.grid_months[n - 1] {
            return Ok(self.grid_rates[n - 1]);
        }

        // Find bracketing interval via binary search
        let idx = self.grid_months.partition_point(|&m| m < abs_month);
        let i = if idx > 0 { idx - 1 } else { 0 };

        let m0 = from_f64::<T>(self.grid_months[i] as f64);
        let m1 = from_f64::<T>(self.grid_months[i + 1] as f64);
        let r0 = self.grid_rates[i];
        let r1 = self.grid_rates[i + 1];
        let x = from_f64::<T>(abs_month as f64);

        let w = (x - m0) / (m1 - m0);
        Ok(r0 * (T::one() - w) + r1 * w)
    }

    // ── Shift/Bump operations ──────────────────────────────────────

    /// Applies a zero-rate shift to grid points selected by `range`.
    ///
    /// On first call, backs up `grid_rates` into `org_rates`.  Subsequent
    /// calls shift from the *original* values (not cumulatively).
    ///
    /// # Algorithm
    ///
    /// For each grid point `i` in `range`:
    /// 1. `Z_i = ln(org_rates[i] / base_index) / t_i`
    /// 2. Apply shift (Absolute: `Z + shift`, Relative: `Z * (1 + shift)`)
    /// 3. Restore: `grid_rates[i] = base_index * exp(Z_shifted * t_i)`
    ///
    /// Points outside the range keep their original values.
    pub fn apply_zero_rate_shift(
        &mut self,
        range: ShiftRange,
        shift_val: T,
        mode: ZeroRateShiftMode,
    ) {
        // Backup on first bump
        if self.org_rates.is_none() {
            self.org_rates = Some(self.grid_rates.clone());
        }
        let org = self.org_rates.as_ref().expect("org_rates just set");
        let base_abs = absolute_month(self.reference_date);

        for i in 0..self.grid_months.len() {
            let y_orig = org[i];
            if range.contains(i) {
                let t_months = self.grid_months[i] - base_abs;
                if t_months <= 0 || y_orig <= T::zero() || self.base_index <= T::zero() {
                    self.grid_rates[i] = y_orig;
                    continue;
                }
                let t = T::from(t_months).unwrap_or_else(|| T::one());
                let z = (y_orig / self.base_index).ln() / t;
                let z_shifted = match mode {
                    ZeroRateShiftMode::Absolute => z + shift_val,
                    ZeroRateShiftMode::Relative => z * (T::one() + shift_val),
                };
                self.grid_rates[i] = self.base_index * (z_shifted * t).exp();
            } else {
                self.grid_rates[i] = y_orig;
            }
        }

        // Rebuild spline if needed
        if self.interpolation == InflationInterpolation::CubicSpline
            && self.grid_months.len() >= 2
        {
            let xs: Vec<T> = self
                .grid_months
                .iter()
                .map(|&m| from_f64(m as f64))
                .collect();
            self.spline = CubicSpline::natural(&xs, &self.grid_rates).ok();
        }
    }

    /// Resets the curve to its original (pre-bump) state.
    ///
    /// Uses `std::mem::swap` for zero-allocation restoration.
    /// No-op if no bump has been applied.
    pub fn reset_rate(&mut self) {
        if let Some(mut org) = self.org_rates.take() {
            std::mem::swap(&mut self.grid_rates, &mut org);
            // Rebuild spline from restored rates
            if self.interpolation == InflationInterpolation::CubicSpline
                && self.grid_months.len() >= 2
            {
                let xs: Vec<T> = self
                    .grid_months
                    .iter()
                    .map(|&m| from_f64(m as f64))
                    .collect();
                self.spline = CubicSpline::natural(&xs, &self.grid_rates).ok();
            }
        }
    }
}

// ─── InflationCurve trait implementation ─────────────────────────────

impl<T: Float> super::InflationCurve<T> for InflationCurveItp<T> {
    fn forward_rate(&self, date: Date) -> Result<T, MarketDataError> {
        let unadjusted = self.unadjusted_forward_rate(date)?;
        let target_abs = absolute_month(date);
        // Extract calendar month (1-indexed)
        let month_in_year = (target_abs.rem_euclid(12) as u32) + 1;
        let seasonal_factor = self.seasonal.monthly_factor(month_in_year);
        Ok(unadjusted * seasonal_factor)
    }

    fn unadjusted_forward_rate(&self, date: Date) -> Result<T, MarketDataError> {
        let abs_month = absolute_month(date);
        self.interpolate_rate(abs_month)
    }

    fn base_index_value(&self) -> T {
        self.base_index
    }

    fn reference_date(&self) -> Date {
        self.reference_date
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_curve() -> InflationCurveItp<f64> {
        let ref_date = Date::from_ymd(2024, 1, 1).unwrap();
        let base_abs = absolute_month(ref_date);
        // Grid: 1Y, 2Y, 5Y, 10Y from reference
        let grid_months = vec![base_abs + 12, base_abs + 24, base_abs + 60, base_abs + 120];
        let grid_rates = vec![0.028, 0.026, 0.024, 0.022];
        let seasonal = InflationSeasonalFactor::identity();
        InflationCurveItp::new(
            grid_months,
            grid_rates,
            300.0,
            ref_date,
            seasonal,
            InflationInterpolation::Linear,
        )
        .unwrap()
    }

    fn sample_curve_with_seasonality() -> InflationCurveItp<f64> {
        let ref_date = Date::from_ymd(2024, 1, 1).unwrap();
        let base_abs = absolute_month(ref_date);
        let grid_months = vec![base_abs + 12, base_abs + 24, base_abs + 60, base_abs + 120];
        let grid_rates = vec![0.028, 0.026, 0.024, 0.022];
        let seasonal = InflationSeasonalFactor::new([
            0.9982, 0.9996, 1.0028, 1.0028, 1.0025, 1.0010, 0.9990, 0.9992, 1.0000, 0.9995,
            0.9972, 0.9982,
        ]);
        InflationCurveItp::new(
            grid_months,
            grid_rates,
            300.0,
            ref_date,
            seasonal,
            InflationInterpolation::Linear,
        )
        .unwrap()
    }

    #[test]
    fn test_unadjusted_rate_at_grid_point() {
        let curve = sample_curve();
        let ref_date = Date::from_ymd(2024, 1, 1).unwrap();
        let base_abs = absolute_month(ref_date);
        // 1Y point
        let d = Date::from_ymd(2025, 1, 1).unwrap();
        assert_eq!(absolute_month(d), base_abs + 12);
        let rate = curve.unadjusted_forward_rate(d).unwrap();
        assert!((rate - 0.028).abs() < 1e-12);
    }

    #[test]
    fn test_unadjusted_rate_interpolated() {
        let curve = sample_curve();
        // Between 1Y and 2Y (18 months from base = midpoint)
        let d = Date::from_ymd(2025, 7, 1).unwrap();
        let rate = curve.unadjusted_forward_rate(d).unwrap();
        // Midpoint of 0.028 and 0.026 = 0.027
        assert!((rate - 0.027).abs() < 1e-12);
    }

    #[test]
    fn test_flat_extrapolation_below() {
        let curve = sample_curve();
        // Before first grid point
        let d = Date::from_ymd(2024, 6, 1).unwrap();
        let rate = curve.unadjusted_forward_rate(d).unwrap();
        assert!((rate - 0.028).abs() < 1e-12);
    }

    #[test]
    fn test_flat_extrapolation_above() {
        let curve = sample_curve();
        // After last grid point
        let d = Date::from_ymd(2040, 1, 1).unwrap();
        let rate = curve.unadjusted_forward_rate(d).unwrap();
        assert!((rate - 0.022).abs() < 1e-12);
    }

    #[test]
    fn test_forward_rate_with_identity_seasonal() {
        let curve = sample_curve();
        let d = Date::from_ymd(2025, 1, 1).unwrap();
        let adj = curve.forward_rate(d).unwrap();
        let unadj = curve.unadjusted_forward_rate(d).unwrap();
        // Identity seasonal => adjusted == unadjusted
        assert!((adj - unadj).abs() < 1e-15);
    }

    #[test]
    fn test_forward_rate_with_seasonality() {
        use super::super::InflationCurve;
        let curve = sample_curve_with_seasonality();
        let d = Date::from_ymd(2025, 1, 1).unwrap();
        let adj = curve.forward_rate(d).unwrap();
        let unadj = curve.unadjusted_forward_rate(d).unwrap();
        // January factor = 0.9982 => adj = unadj * 0.9982
        let expected = unadj * 0.9982;
        assert!((adj - expected).abs() < 1e-12);
    }

    #[test]
    fn test_base_index_and_reference_date() {
        use super::super::InflationCurve;
        let curve = sample_curve();
        assert!((curve.base_index_value() - 300.0).abs() < 1e-15);
        assert_eq!(
            curve.reference_date(),
            Date::from_ymd(2024, 1, 1).unwrap()
        );
    }

    #[test]
    fn test_apply_zero_rate_shift_parallel() {
        let mut curve = sample_curve();
        let original_rates = curve.grid_rates().to_vec();
        let shift = 0.0001; // 1bp

        curve.apply_zero_rate_shift(ShiftRange::all(), shift, ZeroRateShiftMode::Absolute);
        assert!(curve.is_bumped());

        // All rates should have changed
        for i in 0..original_rates.len() {
            assert!(
                (curve.grid_rates()[i] - original_rates[i]).abs() > 1e-10,
                "Rate at index {} should have changed",
                i,
            );
        }
    }

    #[test]
    fn test_apply_zero_rate_shift_key_rate() {
        let mut curve = sample_curve();
        let original_rates = curve.grid_rates().to_vec();

        curve.apply_zero_rate_shift(ShiftRange::EQ(2), 0.001, ZeroRateShiftMode::Absolute);

        // Points 0, 1, 3 unchanged
        assert!((curve.grid_rates()[0] - original_rates[0]).abs() < 1e-14);
        assert!((curve.grid_rates()[1] - original_rates[1]).abs() < 1e-14);
        assert!((curve.grid_rates()[3] - original_rates[3]).abs() < 1e-14);
        // Point 2 changed
        assert!((curve.grid_rates()[2] - original_rates[2]).abs() > 1e-8);
    }

    #[test]
    fn test_reset_rate_restores_original() {
        let mut curve = sample_curve();
        let original_rates = curve.grid_rates().to_vec();

        curve.apply_zero_rate_shift(ShiftRange::all(), 0.01, ZeroRateShiftMode::Absolute);
        assert!(curve.is_bumped());

        curve.reset_rate();
        assert!(!curve.is_bumped());

        for i in 0..original_rates.len() {
            assert!(
                (curve.grid_rates()[i] - original_rates[i]).abs() < 1e-14,
                "Point {} not restored",
                i,
            );
        }
    }

    #[test]
    fn test_multiple_bumps_from_original() {
        let mut curve = sample_curve();
        let original_rates = curve.grid_rates().to_vec();

        // First bump
        curve.apply_zero_rate_shift(ShiftRange::all(), 0.001, ZeroRateShiftMode::Absolute);
        let rates_after_first = curve.grid_rates().to_vec();

        // Second bump (should bump from original, not from first bump)
        curve.apply_zero_rate_shift(ShiftRange::all(), 0.002, ZeroRateShiftMode::Absolute);
        let rates_after_second = curve.grid_rates().to_vec();

        // Second bump should differ from first
        for i in 0..original_rates.len() {
            assert!((rates_after_second[i] - rates_after_first[i]).abs() > 1e-10);
        }

        // Reset
        curve.reset_rate();
        for i in 0..original_rates.len() {
            assert!((curve.grid_rates()[i] - original_rates[i]).abs() < 1e-14);
        }
    }

    #[test]
    fn test_construction_validation_empty() {
        let result = InflationCurveItp::<f64>::new(
            vec![],
            vec![],
            100.0,
            Date::from_ymd(2024, 1, 1).unwrap(),
            InflationSeasonalFactor::identity(),
            InflationInterpolation::Linear,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_construction_validation_mismatched_lengths() {
        let result = InflationCurveItp::<f64>::new(
            vec![1, 2, 3],
            vec![0.02, 0.03],
            100.0,
            Date::from_ymd(2024, 1, 1).unwrap(),
            InflationSeasonalFactor::identity(),
            InflationInterpolation::Linear,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_construction_validation_non_increasing() {
        let result = InflationCurveItp::<f64>::new(
            vec![10, 5, 20],
            vec![0.02, 0.03, 0.04],
            100.0,
            Date::from_ymd(2024, 1, 1).unwrap(),
            InflationSeasonalFactor::identity(),
            InflationInterpolation::Linear,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cubic_spline_interpolation() {
        let ref_date = Date::from_ymd(2024, 1, 1).unwrap();
        let base_abs = absolute_month(ref_date);
        let grid_months = vec![base_abs + 12, base_abs + 24, base_abs + 60, base_abs + 120];
        let grid_rates = vec![0.028, 0.026, 0.024, 0.022];
        let seasonal = InflationSeasonalFactor::identity();
        let curve = InflationCurveItp::new(
            grid_months,
            grid_rates,
            300.0,
            ref_date,
            seasonal,
            InflationInterpolation::CubicSpline,
        )
        .unwrap();

        // At grid point, spline should reproduce exactly
        let d = Date::from_ymd(2025, 1, 1).unwrap();
        let rate = curve.unadjusted_forward_rate(d).unwrap();
        assert!((rate - 0.028).abs() < 1e-10);

        // Between grid points, should be smooth
        let d_mid = Date::from_ymd(2025, 7, 1).unwrap();
        let rate_mid = curve.unadjusted_forward_rate(d_mid).unwrap();
        assert!(rate_mid > 0.020 && rate_mid < 0.030);
    }
}
