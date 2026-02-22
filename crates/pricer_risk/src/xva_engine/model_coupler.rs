//! Model coupler for linking HW1F outer MC scenarios to MFM grid caches.
//!
//! During XVA Monte Carlo, the outer simulation evolves the HW1F state
//! variable (short rate). To price exotic products, the state must be
//! mapped to the MFM tree grid and the pre-computed MtM interpolated.
//!
//! Two coupling approaches are supported:
//!
//! - **Approach A** (Market Observable Mapping): maps via benchmark swap rate.
//! - **Approach B** (Z-Score Matching): maps via normalised Gaussian quantile.

use pricer_models::process::hw1f_analytical;
use pricer_pricing::methods::tree::grid_cache::{MfmGridCache, MfmMtmSlice};

// ─── Configuration ──────────────────────────────────────────────────────────

/// Coupling method for linking HW1F to MFM.
#[derive(Debug, Clone)]
pub enum CouplingMethod {
    /// Approach A: map via a benchmark swap rate observed at each time step.
    ///
    /// The HW1F analytical formula is used to compute the par swap rate,
    /// which is then looked up in the MFM grid's swap-rate array.
    MarketObservableMapping {
        /// Tenor of the benchmark swap (e.g. 10.0 for 10Y).
        swap_tenor: f64,
        /// Payment frequency of the benchmark swap (e.g. 0.5 for semi-annual).
        payment_freq: f64,
    },
    /// Approach B: map via the Z-score (normalised quantile) of the HW1F state.
    ///
    /// Uses the fact that both HW1F and MFM are driven by 1-factor Gaussian
    /// processes to directly translate the state position.
    ZScoreMatching,
}

impl Default for CouplingMethod {
    fn default() -> Self {
        CouplingMethod::MarketObservableMapping {
            swap_tenor: 10.0,
            payment_freq: 0.5,
        }
    }
}

// ─── Coupler ────────────────────────────────────────────────────────────────

/// Maps HW1F Monte Carlo state to MFM grid position for exotic MtM lookup.
#[derive(Debug, Clone)]
pub struct ModelCoupler {
    /// Selected coupling method.
    method: CouplingMethod,
    /// HW1F mean reversion speed.
    hw_mean_reversion: f64,
    /// HW1F volatility.
    hw_volatility: f64,
    /// HW1F initial short rate (r*).
    hw_initial_rate: f64,
}

impl ModelCoupler {
    /// Create a new coupler.
    pub fn new(
        method: CouplingMethod,
        hw_mean_reversion: f64,
        hw_volatility: f64,
        hw_initial_rate: f64,
    ) -> Self {
        Self {
            method,
            hw_mean_reversion,
            hw_volatility,
            hw_initial_rate,
        }
    }

    /// Look up the exotic MtM from a grid cache for a given HW1F state.
    ///
    /// # Arguments
    ///
    /// * `t`        — current time (year fraction)
    /// * `r_t`      — HW1F short rate at time t
    /// * `cache`    — pre-computed MFM grid cache for the exotic product
    /// * `time_idx` — index into `cache.slices` for the current time step
    ///
    /// # Returns
    ///
    /// The interpolated MtM value from the MFM grid.
    pub fn lookup_exotic_mtm(
        &self,
        t: f64,
        r_t: f64,
        cache: &MfmGridCache,
        time_idx: usize,
    ) -> f64 {
        if time_idx >= cache.slices.len() {
            return 0.0;
        }

        let slice = &cache.slices[time_idx];

        match &self.method {
            CouplingMethod::MarketObservableMapping {
                swap_tenor,
                payment_freq,
            } => self.map_via_swap_rate(t, r_t, slice, *swap_tenor, *payment_freq),
            CouplingMethod::ZScoreMatching => self.map_via_zscore(t, r_t, slice, cache),
        }
    }

    /// Look up exotic MtM using the closest time slice (for non-uniform grids).
    ///
    /// Finds the nearest slice by time and delegates to `lookup_exotic_mtm`.
    pub fn lookup_exotic_mtm_by_time(&self, t: f64, r_t: f64, cache: &MfmGridCache) -> f64 {
        match cache.find_closest_slice(t) {
            Some(idx) => self.lookup_exotic_mtm(t, r_t, cache, idx),
            None => 0.0,
        }
    }

    // ── Approach A: Market Observable Mapping ───────────────────────────

    /// Maps the HW1F state to the MFM grid via a benchmark swap rate.
    ///
    /// 1. Compute the par swap rate `S_HW(t)` analytically from `r_t`.
    /// 2. Look up `S_HW(t)` in the MFM slice's swap-rate array via binary
    ///    search.
    /// 3. Interpolate the MtM at that position.
    fn map_via_swap_rate(
        &self,
        t: f64,
        r_t: f64,
        slice: &MfmMtmSlice,
        swap_tenor: f64,
        payment_freq: f64,
    ) -> f64 {
        // Step 1: compute the benchmark swap rate from HW1F state
        let s_hw = hw1f_analytical::hw_swap_rate(
            self.hw_mean_reversion,
            self.hw_volatility,
            self.hw_initial_rate,
            t,
            r_t,
            swap_tenor,
            payment_freq,
        );

        // Step 2-3: binary-search and interpolate in the MFM grid
        slice.interpolate_mtm_by_swap_rate(s_hw)
    }

    // ── Approach B: Z-Score Matching ────────────────────────────────────

    /// Maps the HW1F state to the MFM grid via Z-score (quantile matching).
    ///
    /// 1. Compute Z = (r_t - E[r(t)]) / sqrt(V(t)) for HW1F.
    /// 2. Scale Z by the MFM grid's standard deviation to get x_mfm.
    /// 3. Interpolate the MtM by x on the MFM grid.
    fn map_via_zscore(&self, t: f64, r_t: f64, slice: &MfmMtmSlice, cache: &MfmGridCache) -> f64 {
        let eps = 1e-30;

        // HW1F conditional mean: E[r(t)] = r_star * exp(-a*t) + ...
        // For the simple case with flat curve, E[r(t)] ≈ r_star (long-term)
        // More precisely: E[x(t)] = x(0) * exp(-a*t) under the OU process
        let hw_mean = hw1f_analytical::hw_conditional_mean(
            self.hw_mean_reversion,
            self.hw_initial_rate,
            0.0,
            t,
        );

        // HW1F conditional variance: V(t) = σ²/(2a) * (1 - exp(-2at))
        let hw_var =
            hw1f_analytical::hw_conditional_variance(self.hw_mean_reversion, self.hw_volatility, t);

        let hw_std = hw_var.sqrt().max(eps);

        // Z-score of the current HW1F state
        let z = (r_t - hw_mean) / hw_std;

        // MFM conditional variance at the same time
        let mfm_var = hw1f_analytical::hw_conditional_variance(
            cache.mfm_mean_reversion,
            cache.mfm_volatility,
            t,
        );
        let mfm_std = mfm_var.sqrt().max(eps);

        // Map Z to MFM x-grid coordinate
        // The MFM grid is centred at 0, so x_mfm = Z * mfm_std
        let x_mfm = z * mfm_std;

        // Interpolate MtM on the MFM grid
        slice.interpolate_mtm_by_x(x_mfm)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use pricer_pricing::methods::tree::grid_cache::{ExoticProductType, MfmMtmSlice};

    use super::*;

    fn sample_cache() -> MfmGridCache {
        let slices = vec![
            MfmMtmSlice {
                time: 1.0,
                x_grid: vec![-0.02, -0.01, 0.0, 0.01, 0.02],
                swap_rates: vec![0.01, 0.02, 0.03, 0.04, 0.05],
                mtm_values: vec![-200.0, -100.0, 0.0, 100.0, 200.0],
            },
            MfmMtmSlice {
                time: 2.0,
                x_grid: vec![-0.02, -0.01, 0.0, 0.01, 0.02],
                swap_rates: vec![0.01, 0.02, 0.03, 0.04, 0.05],
                mtm_values: vec![-300.0, -150.0, 0.0, 150.0, 300.0],
            },
        ];

        MfmGridCache::new(
            "TEST_BERM".to_string(),
            ExoticProductType::Bermudan,
            slices,
            0.05,
            0.01,
        )
    }

    #[test]
    fn coupling_method_default() {
        let m = CouplingMethod::default();
        match m {
            CouplingMethod::MarketObservableMapping {
                swap_tenor,
                payment_freq,
            } => {
                assert!((swap_tenor - 10.0).abs() < 1e-12);
                assert!((payment_freq - 0.5).abs() < 1e-12);
            }
            _ => panic!("Expected MarketObservableMapping"),
        }
    }

    #[test]
    fn zscore_at_mean_returns_center() {
        let coupler = ModelCoupler::new(
            CouplingMethod::ZScoreMatching,
            0.05, // a
            0.01, // sigma
            0.03, // r_star
        );
        let cache = sample_cache();

        // At the conditional mean, Z=0, so x_mfm=0, and mtm at x=0 is 0.0
        let hw_mean = hw1f_analytical::hw_conditional_mean(0.05, 0.03, 0.0, 1.0);
        let mtm = coupler.lookup_exotic_mtm(1.0, hw_mean, &cache, 0);
        assert!(
            mtm.abs() < 50.0,
            "MtM at mean should be near center: {}",
            mtm
        );
    }

    #[test]
    fn zscore_above_mean_positive_mtm() {
        let coupler = ModelCoupler::new(CouplingMethod::ZScoreMatching, 0.05, 0.01, 0.03);
        let cache = sample_cache();

        // Well above the mean
        let r_high = 0.06;
        let mtm = coupler.lookup_exotic_mtm(1.0, r_high, &cache, 0);
        assert!(mtm > 0.0, "MtM should be positive for high rates: {}", mtm);
    }

    #[test]
    fn swap_rate_mapping_basic() {
        let coupler = ModelCoupler::new(
            CouplingMethod::MarketObservableMapping {
                swap_tenor: 5.0,
                payment_freq: 0.5,
            },
            0.05,
            0.01,
            0.03,
        );
        let cache = sample_cache();

        // At r_t = r_star, the swap rate should be close to r_star
        // and the MtM should be near the center of the grid
        let mtm = coupler.lookup_exotic_mtm(1.0, 0.03, &cache, 0);
        assert!(mtm.is_finite());
    }

    #[test]
    fn lookup_out_of_bounds_returns_zero() {
        let coupler = ModelCoupler::new(CouplingMethod::default(), 0.05, 0.01, 0.03);
        let cache = sample_cache();

        // time_idx beyond slices
        let mtm = coupler.lookup_exotic_mtm(1.0, 0.03, &cache, 100);
        assert!((mtm - 0.0).abs() < 1e-15);
    }

    #[test]
    fn lookup_by_time_finds_nearest() {
        let coupler = ModelCoupler::new(CouplingMethod::ZScoreMatching, 0.05, 0.01, 0.03);
        let cache = sample_cache();

        // t=1.1 should find slice 0 (time=1.0), t=1.9 should find slice 1 (time=2.0)
        let mtm_near_1 = coupler.lookup_exotic_mtm_by_time(1.1, 0.03, &cache);
        let mtm_near_2 = coupler.lookup_exotic_mtm_by_time(1.9, 0.03, &cache);
        assert!(mtm_near_1.is_finite());
        assert!(mtm_near_2.is_finite());
    }

    #[test]
    fn empty_cache_returns_zero() {
        let coupler = ModelCoupler::new(CouplingMethod::default(), 0.05, 0.01, 0.03);
        let cache = MfmGridCache::new(
            "EMPTY".to_string(),
            ExoticProductType::Bermudan,
            vec![],
            0.05,
            0.01,
        );

        let mtm = coupler.lookup_exotic_mtm_by_time(1.0, 0.03, &cache);
        assert!((mtm - 0.0).abs() < 1e-15);
    }
}
