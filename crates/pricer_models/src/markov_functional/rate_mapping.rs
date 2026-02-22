//! Rate index mapping types for multi-curve MFM calibration.
//!
//! In a multi-curve framework the Markov Functional Model calibrates
//! separate rate mappings for each index:
//!
//! - **Funding swap rate** -- used for discounting.
//! - **Coupon swap rate** -- used for coupon projection on the swap curve.
//! - **Coupon LIBOR** -- used for coupon projection on the LIBOR curve.
//!
//! Each mapping is stored as a sequence of [`CalibratedSlice`] instances,
//! one per exercise date, containing the grid-point-level swap rates,
//! discount factors, and annuities.

use pricer_core::traits::Float;

// ─── Rate index enum ────────────────────────────────────────────────

/// Identifies which rate index a calibrated mapping belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfmRateIndex {
    /// Funding-curve swap rate (used for discounting).
    FundingIndexSwapRate,
    /// Coupon-curve swap rate (used for coupon swap projection).
    CouponIndexSwapRate,
    /// Coupon-curve LIBOR rate (used for LIBOR projection).
    CouponLibor,
}

// ─── Calibrated slice ───────────────────────────────────────────────

/// A single calibrated time-slice of the Gaussian grid.
///
/// For one exercise date, stores the mapping from Gaussian state variable
/// `x` to swap rates, discount factors, and annuities.
#[derive(Debug, Clone)]
pub struct CalibratedSlice<T: Float> {
    /// Exercise time (year fraction) for this slice.
    pub exercise_time: T,
    /// Gaussian grid points (ascending order).
    pub x_grid: Vec<T>,
    /// Swap rate at each grid point.
    pub swap_rates: Vec<T>,
    /// Discount factor at each grid point.
    pub discount_factors: Vec<T>,
    /// Annuity at each grid point.
    pub annuities: Vec<T>,
}

impl<T: Float> CalibratedSlice<T> {
    /// Returns the number of grid nodes in this slice.
    pub fn num_nodes(&self) -> usize {
        self.x_grid.len()
    }

    /// Linearly interpolates the swap rate for a given state variable value `x`.
    ///
    /// If `x` is outside the grid range the swap rate is extrapolated flat
    /// (clamped to the boundary value).
    pub fn interpolate_swap_rate(&self, x: T) -> T {
        let n = self.x_grid.len();
        if n == 0 {
            return T::zero();
        }
        if n == 1 {
            return self.swap_rates[0];
        }

        // Flat extrapolation below
        if x <= self.x_grid[0] {
            return self.swap_rates[0];
        }
        // Flat extrapolation above
        if x >= self.x_grid[n - 1] {
            return self.swap_rates[n - 1];
        }

        // Find the interval [x_grid[i], x_grid[i+1]] containing x
        // using a simple linear scan (grids are typically small, ~41 points).
        let mut i = 0;
        while i < n - 1 && self.x_grid[i + 1] < x {
            i += 1;
        }

        let x0 = self.x_grid[i];
        let x1 = self.x_grid[i + 1];
        let y0 = self.swap_rates[i];
        let y1 = self.swap_rates[i + 1];

        let dx = x1 - x0;
        if dx == T::zero() {
            return y0;
        }

        let t = (x - x0) / dx;
        y0 + t * (y1 - y0)
    }
}

// ─── Rate index calibration ─────────────────────────────────────────

/// Collection of calibrated slices for a single rate index.
///
/// Holds one [`CalibratedSlice`] per exercise date, in chronological order.
#[derive(Debug, Clone)]
pub struct RateIndexCalibration<T: Float> {
    /// Which rate index this calibration corresponds to.
    pub rate_index: MfmRateIndex,
    /// Calibrated slices, one per exercise date, in chronological order.
    pub slices: Vec<CalibratedSlice<T>>,
}

impl<T: Float> RateIndexCalibration<T> {
    /// Returns the number of exercise dates (slices) in this calibration.
    pub fn num_exercise_dates(&self) -> usize {
        self.slices.len()
    }

    /// Returns a reference to the calibrated slice at index `idx`.
    ///
    /// # Panics
    /// Panics if `idx >= self.slices.len()`.
    pub fn slice(&self, idx: usize) -> &CalibratedSlice<T> {
        &self.slices[idx]
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_slice() -> CalibratedSlice<f64> {
        CalibratedSlice {
            exercise_time: 1.0,
            x_grid: vec![-2.0, -1.0, 0.0, 1.0, 2.0],
            swap_rates: vec![0.02, 0.03, 0.04, 0.05, 0.06],
            discount_factors: vec![0.98, 0.97, 0.96, 0.95, 0.94],
            annuities: vec![4.5, 4.4, 4.3, 4.2, 4.1],
        }
    }

    // ── CalibratedSlice tests ───────────────────────────────────────

    #[test]
    fn num_nodes() {
        let s = sample_slice();
        assert_eq!(s.num_nodes(), 5);
    }

    #[test]
    fn interpolate_at_grid_points() {
        let s = sample_slice();
        // At exact grid points the interpolation should return the stored value.
        assert!((s.interpolate_swap_rate(-2.0) - 0.02).abs() < 1e-12);
        assert!((s.interpolate_swap_rate(0.0) - 0.04).abs() < 1e-12);
        assert!((s.interpolate_swap_rate(2.0) - 0.06).abs() < 1e-12);
    }

    #[test]
    fn interpolate_midpoint() {
        let s = sample_slice();
        // Midpoint between x=-1.0 (rate=0.03) and x=0.0 (rate=0.04)
        // Expected: 0.035
        let rate = s.interpolate_swap_rate(-0.5);
        assert!((rate - 0.035).abs() < 1e-12);
    }

    #[test]
    fn interpolate_quarter_point() {
        let s = sample_slice();
        // x = 0.5, between x=0.0 (rate=0.04) and x=1.0 (rate=0.05)
        // t = 0.5, expected: 0.04 + 0.5*(0.05 - 0.04) = 0.045
        let rate = s.interpolate_swap_rate(0.5);
        assert!((rate - 0.045).abs() < 1e-12);
    }

    #[test]
    fn interpolate_flat_extrapolation_below() {
        let s = sample_slice();
        // Below the grid: should return leftmost value
        assert!((s.interpolate_swap_rate(-10.0) - 0.02).abs() < 1e-12);
        assert!((s.interpolate_swap_rate(-2.5) - 0.02).abs() < 1e-12);
    }

    #[test]
    fn interpolate_flat_extrapolation_above() {
        let s = sample_slice();
        // Above the grid: should return rightmost value
        assert!((s.interpolate_swap_rate(10.0) - 0.06).abs() < 1e-12);
        assert!((s.interpolate_swap_rate(2.5) - 0.06).abs() < 1e-12);
    }

    #[test]
    fn interpolate_single_node() {
        let s = CalibratedSlice {
            exercise_time: 1.0,
            x_grid: vec![0.0],
            swap_rates: vec![0.05],
            discount_factors: vec![0.95],
            annuities: vec![4.0],
        };
        assert!((s.interpolate_swap_rate(-1.0) - 0.05).abs() < 1e-12);
        assert!((s.interpolate_swap_rate(0.0) - 0.05).abs() < 1e-12);
        assert!((s.interpolate_swap_rate(1.0) - 0.05).abs() < 1e-12);
    }

    #[test]
    fn interpolate_empty_grid() {
        let s = CalibratedSlice::<f64> {
            exercise_time: 1.0,
            x_grid: vec![],
            swap_rates: vec![],
            discount_factors: vec![],
            annuities: vec![],
        };
        assert!((s.interpolate_swap_rate(0.0) - 0.0).abs() < 1e-12);
    }

    #[test]
    fn interpolate_two_nodes() {
        let s = CalibratedSlice {
            exercise_time: 1.0,
            x_grid: vec![-1.0, 1.0],
            swap_rates: vec![0.03, 0.07],
            discount_factors: vec![0.97, 0.93],
            annuities: vec![4.5, 4.1],
        };
        // Midpoint: 0.03 + 0.5*(0.07-0.03) = 0.05
        assert!((s.interpolate_swap_rate(0.0) - 0.05).abs() < 1e-12);
        // Quarter point: 0.03 + 0.25*(0.07-0.03) = 0.04
        assert!((s.interpolate_swap_rate(-0.5) - 0.04).abs() < 1e-12);
    }

    // ── RateIndexCalibration tests ──────────────────────────────────

    #[test]
    fn rate_index_calibration_basic() {
        let cal = RateIndexCalibration {
            rate_index: MfmRateIndex::FundingIndexSwapRate,
            slices: vec![sample_slice()],
        };
        assert_eq!(cal.num_exercise_dates(), 1);
        assert_eq!(cal.slice(0).num_nodes(), 5);
        assert_eq!(cal.rate_index, MfmRateIndex::FundingIndexSwapRate);
    }

    #[test]
    fn rate_index_calibration_multiple_slices() {
        let mut s1 = sample_slice();
        s1.exercise_time = 1.0;

        let mut s2 = sample_slice();
        s2.exercise_time = 2.0;
        s2.swap_rates = vec![0.025, 0.035, 0.045, 0.055, 0.065];

        let cal = RateIndexCalibration {
            rate_index: MfmRateIndex::CouponIndexSwapRate,
            slices: vec![s1, s2],
        };

        assert_eq!(cal.num_exercise_dates(), 2);
        assert!((cal.slice(0).exercise_time - 1.0).abs() < 1e-12);
        assert!((cal.slice(1).exercise_time - 2.0).abs() < 1e-12);
        assert!((cal.slice(1).swap_rates[2] - 0.045).abs() < 1e-12);
    }

    #[test]
    fn rate_index_calibration_empty() {
        let cal = RateIndexCalibration::<f64> {
            rate_index: MfmRateIndex::CouponLibor,
            slices: vec![],
        };
        assert_eq!(cal.num_exercise_dates(), 0);
    }

    // ── MfmRateIndex tests ──────────────────────────────────────────

    #[test]
    fn rate_index_clone_copy_eq() {
        let idx = MfmRateIndex::CouponLibor;
        let idx2 = idx;
        assert_eq!(idx, idx2);
        assert_eq!(idx.clone(), MfmRateIndex::CouponLibor);
    }

    #[test]
    fn rate_index_variants_distinct() {
        assert_ne!(
            MfmRateIndex::FundingIndexSwapRate,
            MfmRateIndex::CouponIndexSwapRate
        );
        assert_ne!(
            MfmRateIndex::CouponIndexSwapRate,
            MfmRateIndex::CouponLibor
        );
        assert_ne!(
            MfmRateIndex::FundingIndexSwapRate,
            MfmRateIndex::CouponLibor
        );
    }
}
