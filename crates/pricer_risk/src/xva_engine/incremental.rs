//! Incremental XVA engine with HW1F outer MC and analytical/grid pricing.
//!
//! The pipeline:
//! 1. Generate HW1F short-rate paths (Outer MC, Euler-Maruyama).
//! 2. At each `(time, path)`:
//!    - Price vanilla swaps analytically via HW1F affine formulas.
//!    - Price exotics via pre-computed MFM grid caches (O(1) interpolation).
//! 3. Net all MtMs per path to form base and full portfolio exposures.
//! 4. Compute EPE/ENE, bilateral CVA/DVA, FVA for both base and full.
//! 5. Incremental XVA = full XVA − base XVA.

use std::time::Instant;

use pricer_models::process::{
    hull_white::{HullWhiteModel, HullWhiteParams},
    hw1f_analytical,
    stochastic::StochasticModel,
};
use pricer_pricing::methods::tree::grid_cache::MfmGridCache;

use super::{
    error::XvaEngineError,
    model_coupler::{CouplingMethod, ModelCoupler},
};
use crate::portfolio::{
    xva::{BilateralXvaCalculator, OwnCreditParams},
    CreditParams,
};

// ─── Trade definitions ──────────────────────────────────────────────────────

/// A vanilla interest rate swap for analytical pricing on HW1F paths.
#[derive(Debug, Clone)]
pub struct VanillaSwapDef {
    /// Trade identifier.
    pub trade_id: String,
    /// Notional amount.
    pub notional: f64,
    /// Contractual fixed rate.
    pub fixed_rate: f64,
    /// Remaining payment dates (absolute year fractions).
    pub payment_times: Vec<f64>,
    /// True = payer swap (pay fixed, receive floating).
    pub is_payer: bool,
}

/// An exotic product with a pre-computed MFM grid cache.
#[derive(Debug, Clone)]
pub struct ExoticTradeDef {
    /// Trade identifier.
    pub trade_id: String,
    /// Notional amount (applied as scaling factor to grid MtM).
    pub notional: f64,
    /// Pre-computed MtM grid from MFM tree pricing.
    pub grid_cache: MfmGridCache,
}

/// Discriminated union for the incremental trade.
#[derive(Debug, Clone)]
pub enum IncrementalTrade {
    Swap(VanillaSwapDef),
    Exotic(ExoticTradeDef),
}

// ─── Portfolio ──────────────────────────────────────────────────────────────

/// Portfolio for incremental XVA: base trades + one incremental trade.
#[derive(Debug, Clone)]
pub struct IncrementalPortfolio {
    /// Existing vanilla swaps.
    pub base_swaps: Vec<VanillaSwapDef>,
    /// Existing exotic trades.
    pub base_exotics: Vec<ExoticTradeDef>,
    /// The new trade being evaluated for incremental impact.
    pub incremental_trade: IncrementalTrade,
}

// ─── Configuration ──────────────────────────────────────────────────────────

/// Configuration for the incremental XVA engine.
#[derive(Debug, Clone)]
pub struct IncrementalXvaConfig {
    /// Number of Monte Carlo paths.
    pub n_paths: usize,
    /// Time grid for exposure computation (year fractions).
    pub time_grid: Vec<f64>,
    /// Random seed for reproducibility.
    pub seed: Option<u64>,
    /// Use antithetic variates.
    pub antithetic: bool,
    /// HW1F model parameters.
    pub hw_mean_reversion: f64,
    pub hw_volatility: f64,
    pub hw_initial_rate: f64,
    /// Model coupling method.
    pub coupling_method: CouplingMethod,
    /// Bilateral CVA/DVA flag.
    pub bilateral: bool,
    /// Compute FVA flag.
    pub compute_fva: bool,
    /// Funding spreads.
    pub funding_spread_borrow: f64,
    pub funding_spread_lend: f64,
}

// ─── Results ────────────────────────────────────────────────────────────────

/// XVA metrics for one portfolio configuration.
#[derive(Debug, Clone, Default)]
pub struct XvaMetrics {
    pub ucva: f64,
    pub bcva: f64,
    pub udva: f64,
    pub bdva: f64,
    pub fca: f64,
    pub fba: f64,
    pub fva: f64,
    pub total: f64,
}

/// Full result of the incremental XVA computation.
#[derive(Debug, Clone)]
pub struct IncrementalXvaResult {
    /// Time grid used.
    pub time_grid: Vec<f64>,
    /// Actual number of paths.
    pub n_paths: usize,
    /// Base portfolio XVA (without incremental trade).
    pub base_xva: XvaMetrics,
    /// Full portfolio XVA (with incremental trade).
    pub full_xva: XvaMetrics,
    /// Incremental XVA = full − base.
    pub incremental_xva: XvaMetrics,
    /// Base portfolio EPE profile.
    pub base_epe: Vec<f64>,
    /// Base portfolio ENE profile.
    pub base_ene: Vec<f64>,
    /// Full portfolio EPE profile.
    pub full_epe: Vec<f64>,
    /// Full portfolio ENE profile.
    pub full_ene: Vec<f64>,
    /// Wall-clock time in milliseconds.
    pub computation_time_ms: f64,
}

// ─── Engine ─────────────────────────────────────────────────────────────────

/// The incremental XVA engine.
pub struct IncrementalXvaEngine;

impl IncrementalXvaEngine {
    /// Run the full incremental XVA pipeline.
    pub fn run(
        config: &IncrementalXvaConfig,
        portfolio: &IncrementalPortfolio,
        credit_params: &CreditParams,
        own_credit: &OwnCreditParams,
    ) -> Result<IncrementalXvaResult, XvaEngineError> {
        let start = Instant::now();

        let n_times = config.time_grid.len();
        let n_paths_requested = config.n_paths;

        // Effective paths (doubled if antithetic)
        let n_paths = if config.antithetic {
            n_paths_requested * 2
        } else {
            n_paths_requested
        };

        // ── 1. Generate HW1F short-rate paths ──
        let rate_paths = Self::generate_hw1f_paths(config, n_paths_requested)?;

        // ── 2. Build the model coupler ──
        let coupler = ModelCoupler::new(
            config.coupling_method.clone(),
            config.hw_mean_reversion,
            config.hw_volatility,
            config.hw_initial_rate,
        );

        // ── 3. Compute MtM on each (time, path) ──
        // base_mtm[t][p] = sum of all base trades
        // incr_mtm[t][p] = incremental trade only
        let mut base_mtm = vec![vec![0.0_f64; n_paths]; n_times];
        let mut incr_mtm = vec![vec![0.0_f64; n_paths]; n_times];

        let a = config.hw_mean_reversion;
        let sigma = config.hw_volatility;
        let r_star = config.hw_initial_rate;

        for t_idx in 0..n_times {
            let t = config.time_grid[t_idx];

            for p in 0..n_paths {
                let r_t = rate_paths[t_idx][p];

                // ── Base vanilla swaps (analytical) ──
                let mut base_total = 0.0;
                for swap in &portfolio.base_swaps {
                    let mtm = hw1f_analytical::hw_swap_mtm(
                        a,
                        sigma,
                        r_star,
                        t,
                        r_t,
                        swap.fixed_rate,
                        swap.notional,
                        &swap.payment_times,
                        swap.is_payer,
                    );
                    base_total += mtm;
                }

                // ── Base exotics (grid cache lookup) ──
                for exotic in &portfolio.base_exotics {
                    let slice_idx = exotic.grid_cache.find_closest_slice(t).unwrap_or(0);
                    let mtm = coupler.lookup_exotic_mtm(t, r_t, &exotic.grid_cache, slice_idx);
                    base_total += exotic.notional * mtm;
                }

                base_mtm[t_idx][p] = base_total;

                // ── Incremental trade ──
                let incr = match &portfolio.incremental_trade {
                    IncrementalTrade::Swap(swap) => hw1f_analytical::hw_swap_mtm(
                        a,
                        sigma,
                        r_star,
                        t,
                        r_t,
                        swap.fixed_rate,
                        swap.notional,
                        &swap.payment_times,
                        swap.is_payer,
                    ),
                    IncrementalTrade::Exotic(exotic) => {
                        let slice_idx = exotic.grid_cache.find_closest_slice(t).unwrap_or(0);
                        let mtm = coupler.lookup_exotic_mtm(t, r_t, &exotic.grid_cache, slice_idx);
                        exotic.notional * mtm
                    }
                };

                incr_mtm[t_idx][p] = incr;
            }
        }

        // ── 4. Compute full portfolio MtM = base + incremental ──
        let mut full_mtm = vec![vec![0.0_f64; n_paths]; n_times];
        for t_idx in 0..n_times {
            for p in 0..n_paths {
                full_mtm[t_idx][p] = base_mtm[t_idx][p] + incr_mtm[t_idx][p];
            }
        }

        // ── 5. Compute exposure profiles ──
        let base_epe = Self::compute_epe(&base_mtm, n_times, n_paths);
        let base_ene = Self::compute_ene(&base_mtm, n_times, n_paths);
        let full_epe = Self::compute_epe(&full_mtm, n_times, n_paths);
        let full_ene = Self::compute_ene(&full_mtm, n_times, n_paths);

        // ── 6. Compute XVA for base and full ──
        let base_xva = Self::compute_xva_metrics(
            &base_epe,
            &base_ene,
            &config.time_grid,
            credit_params,
            own_credit,
            config,
        )?;
        let full_xva = Self::compute_xva_metrics(
            &full_epe,
            &full_ene,
            &config.time_grid,
            credit_params,
            own_credit,
            config,
        )?;

        // ── 7. Incremental = full - base ──
        let incremental_xva = XvaMetrics {
            ucva: full_xva.ucva - base_xva.ucva,
            bcva: full_xva.bcva - base_xva.bcva,
            udva: full_xva.udva - base_xva.udva,
            bdva: full_xva.bdva - base_xva.bdva,
            fca: full_xva.fca - base_xva.fca,
            fba: full_xva.fba - base_xva.fba,
            fva: full_xva.fva - base_xva.fva,
            total: full_xva.total - base_xva.total,
        };

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(IncrementalXvaResult {
            time_grid: config.time_grid.clone(),
            n_paths,
            base_xva,
            full_xva,
            incremental_xva,
            base_epe,
            base_ene,
            full_epe,
            full_ene,
            computation_time_ms: elapsed_ms,
        })
    }

    // ── Path generation ─────────────────────────────────────────────────

    /// Generate HW1F short-rate paths via Euler-Maruyama.
    ///
    /// Returns `rate_paths[time_idx][path_idx]`.
    fn generate_hw1f_paths(
        config: &IncrementalXvaConfig,
        n_paths_base: usize,
    ) -> Result<Vec<Vec<f64>>, XvaEngineError> {
        use pricer_models::market::curves::FlatCurve;

        let n_times = config.time_grid.len();
        if n_times == 0 {
            return Err(XvaEngineError::InvalidTimeGrid(
                "Time grid must not be empty".to_string(),
            ));
        }

        let hw_params = HullWhiteParams::new(
            config.hw_mean_reversion,
            config.hw_volatility,
            FlatCurve::new(config.hw_initial_rate),
        )
        .ok_or_else(|| XvaEngineError::ConfigError("Invalid HW1F parameters".to_string()))?;

        let n_paths = if config.antithetic {
            n_paths_base * 2
        } else {
            n_paths_base
        };

        // Seed the RNG
        let seed = config.seed.unwrap_or(42);

        // Simple LCG-based normal generator (Box-Muller)
        // For production, you'd use a proper RNG. This is sufficient for the demo.
        let mut rng_state = seed;

        let mut rate_paths = vec![vec![0.0_f64; n_paths]; n_times];

        // For each path, evolve the HW1F model
        for p in 0..n_paths_base {
            let mut params = hw_params.clone();
            let mut state = HullWhiteModel::initial_state(&params);
            let mut prev_time = 0.0;

            for t_idx in 0..n_times {
                let t = config.time_grid[t_idx];
                let dt = t - prev_time;

                if dt > 0.0 {
                    // Generate a standard normal random variate (Box-Muller)
                    let z = next_normal(&mut rng_state);
                    let dw = [z];
                    params.current_time = prev_time;
                    state = HullWhiteModel::evolve_step(state, dt, &dw, &params);
                }

                rate_paths[t_idx][p] = state.0;

                // Antithetic path: use -z
                if config.antithetic {
                    let _anti_p = n_paths_base + p;
                    if t_idx == 0 {
                        // Reset for antithetic
                    }
                    // We need to re-evolve with negated random numbers
                    // This is handled below
                }

                prev_time = t;
            }
        }

        // Generate antithetic paths separately
        if config.antithetic {
            rng_state = seed; // Reset to same seed

            for p in 0..n_paths_base {
                let mut params = hw_params.clone();
                let mut state = HullWhiteModel::initial_state(&params);
                let mut prev_time = 0.0;
                let anti_p = n_paths_base + p;

                for t_idx in 0..n_times {
                    let t = config.time_grid[t_idx];
                    let dt = t - prev_time;

                    if dt > 0.0 {
                        let z = next_normal(&mut rng_state);
                        let dw = [-z]; // Antithetic: negate
                        params.current_time = prev_time;
                        state = HullWhiteModel::evolve_step(state, dt, &dw, &params);
                    }

                    rate_paths[t_idx][anti_p] = state.0;
                    prev_time = t;
                }
            }
        }

        Ok(rate_paths)
    }

    // ── Exposure profile computation ────────────────────────────────────

    /// Compute Expected Positive Exposure: EPE(t) = E[max(V(t), 0)].
    fn compute_epe(mtm: &[Vec<f64>], n_times: usize, n_paths: usize) -> Vec<f64> {
        let mut epe = vec![0.0; n_times];
        for t in 0..n_times {
            let sum: f64 = mtm[t].iter().map(|&v| v.max(0.0)).sum();
            epe[t] = sum / n_paths as f64;
        }
        epe
    }

    /// Compute Expected Negative Exposure: ENE(t) = E[max(-V(t), 0)].
    fn compute_ene(mtm: &[Vec<f64>], n_times: usize, n_paths: usize) -> Vec<f64> {
        let mut ene = vec![0.0; n_times];
        for t in 0..n_times {
            let sum: f64 = mtm[t].iter().map(|&v| (-v).max(0.0)).sum();
            ene[t] = sum / n_paths as f64;
        }
        ene
    }

    // ── XVA computation ─────────────────────────────────────────────────

    /// Compute CVA/DVA/FVA from exposure profiles.
    fn compute_xva_metrics(
        epe: &[f64],
        ene: &[f64],
        time_grid: &[f64],
        credit_params: &CreditParams,
        own_credit: &OwnCreditParams,
        config: &IncrementalXvaConfig,
    ) -> Result<XvaMetrics, XvaEngineError> {
        let bilateral_result = BilateralXvaCalculator::compute_bilateral_cva(
            epe,
            ene,
            time_grid,
            credit_params,
            own_credit,
        );

        let mut metrics = XvaMetrics {
            ucva: bilateral_result.ucva,
            bcva: bilateral_result.bcva,
            udva: bilateral_result.udva,
            bdva: bilateral_result.bdva,
            ..Default::default()
        };

        if config.compute_fva {
            let discount_factors: Vec<f64> = time_grid
                .iter()
                .map(|&t| (-config.hw_initial_rate * t).exp())
                .collect();

            let survival_both: Vec<f64> = time_grid
                .iter()
                .map(|&t| credit_params.survival_prob(t) * own_credit.survival_prob(t))
                .collect();

            let fva_result = BilateralXvaCalculator::compute_fva_with_basis(
                epe,
                ene,
                time_grid,
                config.funding_spread_borrow,
                config.funding_spread_lend,
                &discount_factors,
                &survival_both,
                None,
            );

            metrics.fca = fva_result.fca;
            metrics.fba = fva_result.fba;
            metrics.fva = fva_result.fva;
        }

        metrics.total = metrics.bcva - metrics.bdva + metrics.fva;

        Ok(metrics)
    }
}

// ─── RNG utility ────────────────────────────────────────────────────────────

/// Simple xorshift64 PRNG.
#[inline]
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Generate a standard normal variate using Box-Muller transform.
#[inline]
fn next_normal(state: &mut u64) -> f64 {
    loop {
        let u1 = (xorshift64(state) as f64) / (u64::MAX as f64);
        let u2 = (xorshift64(state) as f64) / (u64::MAX as f64);

        if u1 > 1e-30 {
            let r = (-2.0 * u1.ln()).sqrt();
            let theta = 2.0 * std::f64::consts::PI * u2;
            return r * theta.cos();
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use pricer_pricing::methods::tree::grid_cache::{ExoticProductType, MfmMtmSlice};

    use super::*;

    fn make_config(n_paths: usize) -> IncrementalXvaConfig {
        IncrementalXvaConfig {
            n_paths,
            time_grid: vec![0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0],
            seed: Some(42),
            antithetic: true,
            hw_mean_reversion: 0.05,
            hw_volatility: 0.01,
            hw_initial_rate: 0.03,
            coupling_method: CouplingMethod::default(),
            bilateral: true,
            compute_fva: true,
            funding_spread_borrow: 0.005,
            funding_spread_lend: 0.003,
        }
    }

    fn make_payer_swap(fixed_rate: f64, notional: f64) -> VanillaSwapDef {
        VanillaSwapDef {
            trade_id: "IRS_PAY".to_string(),
            notional,
            fixed_rate,
            payment_times: (1..=10).map(|i| i as f64 * 0.5).collect(),
            is_payer: true,
        }
    }

    fn make_receiver_swap(fixed_rate: f64, notional: f64) -> VanillaSwapDef {
        VanillaSwapDef {
            trade_id: "IRS_RCV".to_string(),
            notional,
            fixed_rate,
            payment_times: (1..=10).map(|i| i as f64 * 0.5).collect(),
            is_payer: false,
        }
    }

    fn make_exotic_cache() -> MfmGridCache {
        let slices: Vec<MfmMtmSlice> = vec![1.0, 2.0, 3.0, 5.0]
            .into_iter()
            .map(|t| MfmMtmSlice {
                time: t,
                x_grid: vec![-0.03, -0.015, 0.0, 0.015, 0.03],
                swap_rates: vec![0.01, 0.02, 0.03, 0.04, 0.05],
                mtm_values: vec![-500.0, -200.0, 0.0, 200.0, 500.0],
            })
            .collect();

        MfmGridCache::new(
            "TEST_BERM".to_string(),
            ExoticProductType::Bermudan,
            slices,
            0.05,
            0.01,
        )
    }

    #[test]
    fn basic_swap_only_portfolio() {
        let config = make_config(1000);
        let portfolio = IncrementalPortfolio {
            base_swaps: vec![make_payer_swap(0.03, 1_000_000.0)],
            base_exotics: vec![],
            incremental_trade: IncrementalTrade::Swap(make_receiver_swap(0.03, 500_000.0)),
        };

        let credit = CreditParams::new(0.02, 0.6).unwrap();
        let own = OwnCreditParams::new(0.01, 0.4).unwrap();

        let result = IncrementalXvaEngine::run(&config, &portfolio, &credit, &own).unwrap();

        assert_eq!(result.time_grid.len(), 8);
        assert!(result.n_paths > 0);
        assert!(result.base_xva.bcva.is_finite());
        assert!(result.full_xva.bcva.is_finite());
        assert!(result.incremental_xva.total.is_finite());
        assert!(result.computation_time_ms > 0.0);
    }

    #[test]
    fn swap_plus_exotic_portfolio() {
        let config = make_config(500);
        let portfolio = IncrementalPortfolio {
            base_swaps: vec![
                make_payer_swap(0.03, 1_000_000.0),
                make_receiver_swap(0.03, 800_000.0),
            ],
            base_exotics: vec![ExoticTradeDef {
                trade_id: "BERM_1".to_string(),
                notional: 1.0, // Grid already has notional baked in
                grid_cache: make_exotic_cache(),
            }],
            incremental_trade: IncrementalTrade::Exotic(ExoticTradeDef {
                trade_id: "TARN_1".to_string(),
                notional: 1.0,
                grid_cache: make_exotic_cache(),
            }),
        };

        let credit = CreditParams::new(0.02, 0.6).unwrap();
        let own = OwnCreditParams::new(0.01, 0.4).unwrap();

        let result = IncrementalXvaEngine::run(&config, &portfolio, &credit, &own).unwrap();

        assert!(result.base_epe.len() == 8);
        assert!(result.full_epe.len() == 8);
        assert!(result.incremental_xva.total.is_finite());
    }

    #[test]
    fn incremental_xva_is_difference() {
        let config = make_config(500);
        let portfolio = IncrementalPortfolio {
            base_swaps: vec![make_payer_swap(0.03, 1_000_000.0)],
            base_exotics: vec![],
            incremental_trade: IncrementalTrade::Swap(make_payer_swap(0.035, 500_000.0)),
        };

        let credit = CreditParams::new(0.02, 0.6).unwrap();
        let own = OwnCreditParams::new(0.01, 0.4).unwrap();

        let result = IncrementalXvaEngine::run(&config, &portfolio, &credit, &own).unwrap();

        // Verify: incremental = full - base
        let diff =
            (result.incremental_xva.bcva - (result.full_xva.bcva - result.base_xva.bcva)).abs();
        assert!(diff < 1e-10, "BCVA diff: {}", diff);

        let diff_fva =
            (result.incremental_xva.fva - (result.full_xva.fva - result.base_xva.fva)).abs();
        assert!(diff_fva < 1e-10, "FVA diff: {}", diff_fva);
    }

    #[test]
    fn empty_base_portfolio() {
        let config = make_config(500);
        let portfolio = IncrementalPortfolio {
            base_swaps: vec![],
            base_exotics: vec![],
            incremental_trade: IncrementalTrade::Swap(make_payer_swap(0.03, 1_000_000.0)),
        };

        let credit = CreditParams::new(0.02, 0.6).unwrap();
        let own = OwnCreditParams::new(0.01, 0.4).unwrap();

        let result = IncrementalXvaEngine::run(&config, &portfolio, &credit, &own).unwrap();

        // Base XVA should be zero (empty portfolio)
        assert!(result.base_xva.bcva.abs() < 1e-10);
        assert!(result.base_xva.bdva.abs() < 1e-10);

        // Full = incremental when base is empty
        let diff = (result.full_xva.total - result.incremental_xva.total).abs();
        assert!(diff < 1e-10);
    }

    #[test]
    fn rng_produces_finite_values() {
        let mut state = 42u64;
        for _ in 0..1000 {
            let z = next_normal(&mut state);
            assert!(z.is_finite());
            assert!(z.abs() < 10.0); // Very unlikely to exceed 10 sigma
        }
    }
}
