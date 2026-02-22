//! Pre-computed MtM grid cache for exotic products.
//!
//! Stores mark-to-market values computed via MFM tree pricing at each
//! node of the Gaussian tree. During XVA Monte Carlo simulation, the
//! HW1F path state is mapped to a grid position and the MtM is retrieved
//! via O(1) linear interpolation instead of re-pricing.

// ─── Types ──────────────────────────────────────────────────────────────────

/// Product type for the cached exotic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExoticProductType {
    /// Bermudan swaption (callable or puttable).
    Bermudan,
    /// Target Redemption Note.
    Tarn,
    /// Callable Inverse Floater.
    Cif,
}

/// A single time-slice of cached MtM values on the Gaussian grid.
#[derive(Debug, Clone)]
pub struct MfmMtmSlice {
    /// Time (year fraction) of this slice.
    pub time: f64,
    /// Gaussian state variable grid (ascending).
    pub x_grid: Vec<f64>,
    /// Swap rate at each grid point (monotone, for Approach A lookup).
    pub swap_rates: Vec<f64>,
    /// Mark-to-market value at each grid point.
    pub mtm_values: Vec<f64>,
}

impl MfmMtmSlice {
    /// Number of grid points.
    #[inline]
    pub fn num_nodes(&self) -> usize { self.x_grid.len() }

    /// Linearly interpolate the MtM for a given state variable value `x`.
    ///
    /// Flat-extrapolates outside the grid range.
    pub fn interpolate_mtm_by_x(&self, x: f64) -> f64 {
        linear_interp(&self.x_grid, &self.mtm_values, x)
    }

    /// Linearly interpolate the MtM for a given swap rate value.
    ///
    /// Uses binary search on the (monotone) `swap_rates` array to locate
    /// the position, then interpolates the corresponding `mtm_values`.
    /// This implements **Approach A** (Market Observable Mapping).
    pub fn interpolate_mtm_by_swap_rate(&self, swap_rate: f64) -> f64 {
        if self.swap_rates.is_empty() {
            return 0.0;
        }
        if self.swap_rates.len() == 1 {
            return self.mtm_values[0];
        }

        // swap_rates is monotone (increasing with x for normal models).
        // Use binary search to find the interval.
        let n = self.swap_rates.len();

        // Flat extrapolation
        if swap_rate <= self.swap_rates[0] {
            return self.mtm_values[0];
        }
        if swap_rate >= self.swap_rates[n - 1] {
            return self.mtm_values[n - 1];
        }

        // Binary search for the interval [i, i+1] such that
        // swap_rates[i] <= swap_rate < swap_rates[i+1]
        let mut lo = 0usize;
        let mut hi = n - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if self.swap_rates[mid] <= swap_rate {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        let s0 = self.swap_rates[lo];
        let s1 = self.swap_rates[hi];
        let ds = s1 - s0;
        if ds.abs() < 1e-30 {
            return self.mtm_values[lo];
        }

        let w = (swap_rate - s0) / ds;
        self.mtm_values[lo] * (1.0 - w) + self.mtm_values[hi] * w
    }
}

/// Full grid cache for one exotic product.
///
/// Contains pre-computed MtM slices at each observation time, plus the
/// MFM model parameters used to build the grid (needed for Z-score
/// mapping in Approach B).
#[derive(Debug, Clone)]
pub struct MfmGridCache {
    /// Identifier for the product.
    pub product_id: String,
    /// Type of exotic product.
    pub product_type: ExoticProductType,
    /// Cached slices, one per observation time (chronological order).
    pub slices: Vec<MfmMtmSlice>,
    /// MFM mean reversion used to build this grid.
    pub mfm_mean_reversion: f64,
    /// MFM volatility used to build this grid.
    pub mfm_volatility: f64,
}

impl MfmGridCache {
    /// Create a new grid cache.
    pub fn new(
        product_id: String,
        product_type: ExoticProductType,
        slices: Vec<MfmMtmSlice>,
        mfm_mean_reversion: f64,
        mfm_volatility: f64,
    ) -> Self {
        Self {
            product_id,
            product_type,
            slices,
            mfm_mean_reversion,
            mfm_volatility,
        }
    }

    /// Number of time slices in the cache.
    #[inline]
    pub fn num_slices(&self) -> usize { self.slices.len() }

    /// Find the closest slice index for a given time.
    ///
    /// Returns the index of the slice whose time is closest to `t`.
    pub fn find_closest_slice(&self, t: f64) -> Option<usize> {
        if self.slices.is_empty() {
            return None;
        }

        let mut best_idx = 0;
        let mut best_dist = (self.slices[0].time - t).abs();

        for (i, slice) in self.slices.iter().enumerate().skip(1) {
            let dist = (slice.time - t).abs();
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }

        Some(best_idx)
    }

    /// Interpolate MtM at a given time and x-grid position.
    ///
    /// Finds the two bracketing time slices and interpolates in both
    /// time and space dimensions. Falls back to nearest-slice lookup
    /// if `t` is outside the cached time range.
    pub fn interpolate_mtm(&self, t: f64, x: f64) -> f64 {
        if self.slices.is_empty() {
            return 0.0;
        }
        if self.slices.len() == 1 {
            return self.slices[0].interpolate_mtm_by_x(x);
        }

        let n = self.slices.len();

        // Before first slice
        if t <= self.slices[0].time {
            return self.slices[0].interpolate_mtm_by_x(x);
        }
        // After last slice
        if t >= self.slices[n - 1].time {
            return self.slices[n - 1].interpolate_mtm_by_x(x);
        }

        // Find bracketing slices
        let mut lo = 0;
        for i in 0..n - 1 {
            if self.slices[i].time <= t && t < self.slices[i + 1].time {
                lo = i;
                break;
            }
        }
        let hi = lo + 1;

        let t0 = self.slices[lo].time;
        let t1 = self.slices[hi].time;
        let dt = t1 - t0;

        if dt.abs() < 1e-30 {
            return self.slices[lo].interpolate_mtm_by_x(x);
        }

        let w = (t - t0) / dt;
        let v0 = self.slices[lo].interpolate_mtm_by_x(x);
        let v1 = self.slices[hi].interpolate_mtm_by_x(x);

        v0 * (1.0 - w) + v1 * w
    }

    /// Interpolate MtM by swap rate at a given slice index.
    ///
    /// This is the core lookup for **Approach A** (Market Observable Mapping):
    /// the HW1F engine computes a benchmark swap rate and uses it to look up
    /// the MtM in the MFM grid.
    pub fn interpolate_mtm_by_swap_rate(&self, time_idx: usize, swap_rate: f64) -> f64 {
        if time_idx >= self.slices.len() {
            return 0.0;
        }
        self.slices[time_idx].interpolate_mtm_by_swap_rate(swap_rate)
    }

    /// Build a grid cache from Bermudan tree pricing results.
    ///
    /// Extracts MtM values at each exercise time from the backward
    /// induction node values, paired with the MFM calibration data
    /// for swap rate mappings.
    pub fn from_bermudan_node_values(
        product_id: String,
        exercise_times: &[f64],
        x_grids: &[Vec<f64>],
        swap_rate_grids: &[Vec<f64>],
        mtm_grids: &[Vec<f64>],
        mean_reversion: f64,
        volatility: f64,
    ) -> Self {
        let slices = exercise_times
            .iter()
            .zip(x_grids.iter())
            .zip(swap_rate_grids.iter())
            .zip(mtm_grids.iter())
            .map(|(((&t, x_grid), sr_grid), mtm)| MfmMtmSlice {
                time: t,
                x_grid: x_grid.clone(),
                swap_rates: sr_grid.clone(),
                mtm_values: mtm.clone(),
            })
            .collect();

        Self::new(
            product_id,
            ExoticProductType::Bermudan,
            slices,
            mean_reversion,
            volatility,
        )
    }

    /// Build a grid cache from TARN tree pricing results.
    pub fn from_tarn_node_values(
        product_id: String,
        observation_times: &[f64],
        x_grids: &[Vec<f64>],
        swap_rate_grids: &[Vec<f64>],
        mtm_grids: &[Vec<f64>],
        mean_reversion: f64,
        volatility: f64,
    ) -> Self {
        let slices = observation_times
            .iter()
            .zip(x_grids.iter())
            .zip(swap_rate_grids.iter())
            .zip(mtm_grids.iter())
            .map(|(((&t, x_grid), sr_grid), mtm)| MfmMtmSlice {
                time: t,
                x_grid: x_grid.clone(),
                swap_rates: sr_grid.clone(),
                mtm_values: mtm.clone(),
            })
            .collect();

        Self::new(
            product_id,
            ExoticProductType::Tarn,
            slices,
            mean_reversion,
            volatility,
        )
    }
}

// ─── Utility ────────────────────────────────────────────────────────────────

/// Linear interpolation with flat extrapolation.
fn linear_interp(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let n = xs.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return ys[0];
    }

    // Flat extrapolation
    if x <= xs[0] {
        return ys[0];
    }
    if x >= xs[n - 1] {
        return ys[n - 1];
    }

    // Linear scan (grids are typically small, ~41 points)
    let mut i = 0;
    while i < n - 1 && xs[i + 1] < x {
        i += 1;
    }

    let dx = xs[i + 1] - xs[i];
    if dx.abs() < 1e-30 {
        return ys[i];
    }

    let w = (x - xs[i]) / dx;
    ys[i] * (1.0 - w) + ys[i + 1] * w
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_slice(time: f64) -> MfmMtmSlice {
        MfmMtmSlice {
            time,
            x_grid: vec![-2.0, -1.0, 0.0, 1.0, 2.0],
            swap_rates: vec![0.01, 0.02, 0.03, 0.04, 0.05],
            mtm_values: vec![-100.0, -50.0, 0.0, 50.0, 100.0],
        }
    }

    fn sample_cache() -> MfmGridCache {
        MfmGridCache::new(
            "TEST_BERM".to_string(),
            ExoticProductType::Bermudan,
            vec![sample_slice(1.0), sample_slice(2.0), sample_slice(3.0)],
            0.05,
            0.01,
        )
    }

    // ── MfmMtmSlice tests ──────────────────────────────────────────

    #[test]
    fn slice_interp_at_grid_points() {
        let s = sample_slice(1.0);
        assert!((s.interpolate_mtm_by_x(0.0) - 0.0).abs() < 1e-12);
        assert!((s.interpolate_mtm_by_x(-2.0) - (-100.0)).abs() < 1e-12);
        assert!((s.interpolate_mtm_by_x(2.0) - 100.0).abs() < 1e-12);
    }

    #[test]
    fn slice_interp_midpoint() {
        let s = sample_slice(1.0);
        let v = s.interpolate_mtm_by_x(0.5);
        assert!((v - 25.0).abs() < 1e-12);
    }

    #[test]
    fn slice_interp_flat_extrap() {
        let s = sample_slice(1.0);
        assert!((s.interpolate_mtm_by_x(-5.0) - (-100.0)).abs() < 1e-12);
        assert!((s.interpolate_mtm_by_x(5.0) - 100.0).abs() < 1e-12);
    }

    #[test]
    fn slice_interp_by_swap_rate() {
        let s = sample_slice(1.0);
        // swap_rate=0.03 maps to x=0.0, mtm=0.0
        assert!((s.interpolate_mtm_by_swap_rate(0.03) - 0.0).abs() < 1e-12);
        // swap_rate=0.025 maps to midpoint between x=-1 and x=0
        let v = s.interpolate_mtm_by_swap_rate(0.025);
        assert!((v - (-25.0)).abs() < 1e-12);
    }

    #[test]
    fn slice_interp_by_swap_rate_extrap() {
        let s = sample_slice(1.0);
        assert!((s.interpolate_mtm_by_swap_rate(0.001) - (-100.0)).abs() < 1e-12);
        assert!((s.interpolate_mtm_by_swap_rate(0.1) - 100.0).abs() < 1e-12);
    }

    // ── MfmGridCache tests ─────────────────────────────────────────

    #[test]
    fn cache_find_closest_slice() {
        let cache = sample_cache();
        assert_eq!(cache.find_closest_slice(1.1), Some(0));
        assert_eq!(cache.find_closest_slice(1.9), Some(1));
        assert_eq!(cache.find_closest_slice(2.8), Some(2));
    }

    #[test]
    fn cache_interpolate_at_slice_time() {
        let cache = sample_cache();
        let v = cache.interpolate_mtm(1.0, 0.0);
        assert!((v - 0.0).abs() < 1e-12);
    }

    #[test]
    fn cache_interpolate_between_slices() {
        let cache = sample_cache();
        // At t=1.5, x=1.0: both slices give mtm=50.0, so interpolation = 50.0
        let v = cache.interpolate_mtm(1.5, 1.0);
        assert!((v - 50.0).abs() < 1e-12);
    }

    #[test]
    fn cache_interpolate_by_swap_rate() {
        let cache = sample_cache();
        let v = cache.interpolate_mtm_by_swap_rate(0, 0.03);
        assert!((v - 0.0).abs() < 1e-12);
    }

    #[test]
    fn cache_empty() {
        let cache = MfmGridCache::new(
            "EMPTY".to_string(),
            ExoticProductType::Bermudan,
            vec![],
            0.05,
            0.01,
        );
        assert_eq!(cache.interpolate_mtm(1.0, 0.0), 0.0);
        assert_eq!(cache.find_closest_slice(1.0), None);
    }

    #[test]
    fn from_bermudan_node_values() {
        let cache = MfmGridCache::from_bermudan_node_values(
            "BERM_1".to_string(),
            &[1.0, 2.0],
            &[vec![-1.0, 0.0, 1.0], vec![-1.0, 0.0, 1.0]],
            &[vec![0.02, 0.03, 0.04], vec![0.02, 0.03, 0.04]],
            &[vec![-10.0, 0.0, 10.0], vec![-20.0, 0.0, 20.0]],
            0.05,
            0.01,
        );
        assert_eq!(cache.num_slices(), 2);
        assert_eq!(cache.product_type, ExoticProductType::Bermudan);
        // At t=1, x=0.5 -> interp between 0 and 10 = 5.0
        assert!((cache.slices[0].interpolate_mtm_by_x(0.5) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn linear_interp_basic() {
        let xs = vec![0.0, 1.0, 2.0];
        let ys = vec![0.0, 10.0, 20.0];
        assert!((linear_interp(&xs, &ys, 0.5) - 5.0).abs() < 1e-12);
        assert!((linear_interp(&xs, &ys, 1.5) - 15.0).abs() < 1e-12);
    }
}
