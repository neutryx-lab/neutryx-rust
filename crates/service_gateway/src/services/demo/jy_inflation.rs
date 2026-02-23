//! Jarrow-Yildirim inflation model service methods.
//!
//! Provides five API endpoints for the JY demo GUI:
//! - Curve building (nominal + real + breakeven)
//! - Instrument cashflow generation
//! - Monte Carlo simulation of the 3-factor model
//! - Analytical ZCIS pricing with bump-and-revalue Greeks
//! - XVA computation (CVA/DVA/FVA) via outer MC

use pricer_models::process::{
    stochastic::StochasticModel, JarrowYildirimModel, JarrowYildirimParams, ZcisAnalyticalPricer,
};

use super::DemoService;
use crate::{
    error::ServerError,
    rest::dto::jy_inflation::{
        ExposureProfile, JyCashflow, JyCurveBuildRequest, JyCurveBuildResponse, JyCurvePoint,
        JyGreeks, JyInstrumentRequest, JyInstrumentResponse, JyInstrumentSummary, JyPricingRequest,
        JyPricingResponse, JySamplePath, JySimulationRequest, JySimulationResponse, JyXvaRequest,
        JyXvaResponse, SimulationPathStats,
    },
};

// ─── Simple PRNG (no rand dependency) ───────────────────────────────────────

/// Xorshift64 state for reproducible Monte Carlo.
struct Xorshift64(u64);

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0xDEAD_BEEF_CAFE_BABE
        } else {
            seed
        })
    }

    /// Seed from system time.
    fn from_time() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self::new(nanos ^ 0x517CC1B727220A95)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in (0, 1).
    fn next_f64(&mut self) -> f64 { (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64 }

    /// Standard normal via Box-Muller transform.
    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-300);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Build JY params from DTO request fields.
fn build_jy_params(
    model: &crate::rest::dto::jy_inflation::JyModelParams,
    corr: &crate::rest::dto::jy_inflation::JyCorrelation,
    initial_nominal: f64,
    initial_real: f64,
    initial_index: f64,
) -> Result<JarrowYildirimParams<f64>, ServerError> {
    JarrowYildirimParams::new(
        model.a_n,
        model.sigma_n,
        initial_nominal,
        model.a_r,
        model.sigma_r,
        initial_real,
        model.sigma_i,
        initial_index,
        corr.rho_nr,
        corr.rho_ni,
        corr.rho_ri,
    )
    .map_err(|e| ServerError::Pricing(e.to_string()))
}

/// Compute percentile from a sorted slice using linear interpolation.
fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len();
    let idx = p * (n - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = (lo + 1).min(n - 1);
    let frac = idx - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

/// Parse tenor string (e.g., "3M", "1Y", "6M") to years.
fn tenor_to_years(tenor: &str) -> f64 {
    let s = tenor.trim().to_uppercase();
    if let Some(y) = s.strip_suffix('Y') {
        y.parse::<f64>().unwrap_or(1.0)
    } else if let Some(m) = s.strip_suffix('M') {
        m.parse::<f64>().unwrap_or(12.0) / 12.0
    } else if let Some(w) = s.strip_suffix('W') {
        w.parse::<f64>().unwrap_or(1.0) / 52.0
    } else if let Some(d) = s.strip_suffix('D') {
        d.parse::<f64>().unwrap_or(1.0) / 365.0
    } else {
        s.parse::<f64>().unwrap_or(1.0)
    }
}

// ─── Service Methods ────────────────────────────────────────────────────────

impl DemoService {
    /// Build nominal and real yield curves from market rate inputs.
    ///
    /// Uses flat-rate interpolation for the demo: each instrument's tenor maps
    /// to a zero rate, and discount factors are computed as exp(-r * t).
    /// The nominal curve is resolved from the Rates system via
    /// `nominal_curve_ref`.
    pub fn jy_build_curves(
        request: &JyCurveBuildRequest,
    ) -> Result<JyCurveBuildResponse, ServerError> {
        let nominal_rates = Self::resolve_nominal_curve(&request.nominal_curve_ref)?;

        let mut nominal_curve = Vec::with_capacity(nominal_rates.len());
        let mut nominal_df = Vec::with_capacity(nominal_rates.len());
        for pt in &nominal_rates {
            let t = tenor_to_years(&pt.tenor);
            nominal_curve.push(JyCurvePoint {
                tenor: t,
                value: pt.rate,
            });
            nominal_df.push(JyCurvePoint {
                tenor: t,
                value: (-pt.rate * t).exp(),
            });
        }

        let mut real_curve = Vec::with_capacity(request.real_rates.len());
        let mut real_df = Vec::with_capacity(request.real_rates.len());
        for pt in &request.real_rates {
            let t = tenor_to_years(&pt.tenor);
            real_curve.push(JyCurvePoint {
                tenor: t,
                value: pt.rate,
            });
            real_df.push(JyCurvePoint {
                tenor: t,
                value: (-pt.rate * t).exp(),
            });
        }

        // Breakeven = nominal - real at common tenors
        let mut breakeven_curve = Vec::new();
        for np in &nominal_curve {
            if let Some(rp) = real_curve
                .iter()
                .find(|rp| (rp.tenor - np.tenor).abs() < 0.01)
            {
                breakeven_curve.push(JyCurvePoint {
                    tenor: np.tenor,
                    value: np.value - rp.value,
                });
            }
        }

        // Sort all curves by tenor
        nominal_curve.sort_by(|a, b| a.tenor.partial_cmp(&b.tenor).unwrap());
        real_curve.sort_by(|a, b| a.tenor.partial_cmp(&b.tenor).unwrap());
        breakeven_curve.sort_by(|a, b| a.tenor.partial_cmp(&b.tenor).unwrap());
        nominal_df.sort_by(|a, b| a.tenor.partial_cmp(&b.tenor).unwrap());
        real_df.sort_by(|a, b| a.tenor.partial_cmp(&b.tenor).unwrap());

        Ok(JyCurveBuildResponse {
            nominal_curve,
            real_curve,
            breakeven_curve,
            nominal_df,
            real_df,
        })
    }

    /// Generate cashflow schedule for a ZCIS or YoY inflation swap.
    pub fn jy_instrument_cashflows(
        request: &JyInstrumentRequest,
    ) -> Result<JyInstrumentResponse, ServerError> {
        let maturity_years = parse_maturity_years(&request.start_date, &request.maturity_date)?;

        let freq_years = match request.payment_frequency.to_lowercase().as_str() {
            "semiannual" | "semi-annual" => 0.5,
            "quarterly" => 0.25,
            _ => 1.0, // annual default
        };

        let is_zcis = request.instrument_type.to_uppercase() == "ZCIS";
        let mut cashflows = Vec::new();
        let mut total_fixed_pv = 0.0;
        let mut total_inflation_pv = 0.0;

        if is_zcis {
            // ZCIS: single cashflow at maturity
            let df_nominal = (-request.nominal_curve_rate * maturity_years).exp();
            let df_real = (-request.real_curve_rate * maturity_years).exp();

            // Fixed leg: N * [(1+K)^T - 1]
            let fixed_amount =
                request.notional * ((1.0 + request.fixed_rate).powf(maturity_years) - 1.0);
            let fixed_pv = fixed_amount * df_nominal;

            // Inflation leg: N * [I(T)/I(0) - 1] ≈ use forward breakeven
            let breakeven = request.nominal_curve_rate - request.real_curve_rate;
            let expected_index_ratio = (breakeven * maturity_years).exp();
            let inflation_amount = request.notional * (expected_index_ratio - 1.0);
            let inflation_pv = inflation_amount * df_real;

            cashflows.push(JyCashflow {
                date: request.maturity_date.clone(),
                year_fraction: maturity_years,
                nominal_amount: fixed_amount,
                real_amount: Some(inflation_amount),
                discount_factor: df_nominal,
                present_value: inflation_pv - fixed_pv,
            });

            total_fixed_pv = fixed_pv;
            total_inflation_pv = inflation_pv;
        } else {
            // YoY: periodic payments
            let num_periods = (maturity_years / freq_years).round() as usize;
            let breakeven = request.nominal_curve_rate - request.real_curve_rate;

            for i in 1..=num_periods {
                let t = freq_years * i as f64;
                let df = (-request.nominal_curve_rate * t).exp();

                // Fixed leg coupon
                let fixed_coupon = request.notional * request.fixed_rate * freq_years;

                // YoY inflation coupon: N * [I(t)/I(t-1) - 1]
                let yoy_rate = breakeven; // approximate
                let inflation_coupon = request.notional * yoy_rate * freq_years;

                let pv = (inflation_coupon - fixed_coupon) * df;

                cashflows.push(JyCashflow {
                    date: format_date_offset(&request.start_date, t),
                    year_fraction: freq_years,
                    nominal_amount: fixed_coupon,
                    real_amount: Some(inflation_coupon),
                    discount_factor: df,
                    present_value: pv,
                });

                total_fixed_pv += fixed_coupon * df;
                total_inflation_pv += inflation_coupon * df;
            }
        }

        let net_pv = total_inflation_pv - total_fixed_pv;

        Ok(JyInstrumentResponse {
            instrument_type: request.instrument_type.clone(),
            cashflows,
            summary: JyInstrumentSummary {
                total_fixed_pv,
                total_inflation_pv,
                net_pv,
                num_cashflows: if is_zcis {
                    1
                } else {
                    (maturity_years / freq_years).round() as usize
                },
                maturity_years,
            },
        })
    }

    /// Run Monte Carlo simulation of the JY 3-factor model.
    pub fn jy_simulate(request: &JySimulationRequest) -> Result<JySimulationResponse, ServerError> {
        let num_paths = request.num_paths as usize;
        let num_steps = request.num_steps as usize;
        let dt = request.horizon / num_steps as f64;
        let num_sample = (request.num_sample_paths as usize).min(num_paths);

        let params = build_jy_params(
            &request.model_params,
            &request.correlation,
            request.initial_nominal_rate,
            request.initial_real_rate,
            request.initial_index,
        )?;

        // Verify correlation matrix is PD
        let psd_enforced = params.correlation_matrix().is_err();

        let mut rng = Xorshift64::from_time();

        // Storage: [path][step] for each factor
        let mut nominal_paths: Vec<Vec<f64>> = vec![vec![0.0; num_steps + 1]; num_paths];
        let mut real_paths: Vec<Vec<f64>> = vec![vec![0.0; num_steps + 1]; num_paths];
        let mut index_paths: Vec<Vec<f64>> = vec![vec![0.0; num_steps + 1]; num_paths];

        // Simulate paths
        for path_idx in 0..num_paths {
            let mut state = JarrowYildirimModel::initial_state(&params);
            nominal_paths[path_idx][0] = state.first;
            real_paths[path_idx][0] = state.second;
            index_paths[path_idx][0] = state.third;

            let mut sim_params = params.clone();
            sim_params.reset_time();

            for step in 1..=num_steps {
                let dw = [rng.next_normal(), rng.next_normal(), rng.next_normal()];
                state = JarrowYildirimModel::evolve_step(state, dt, &dw, &sim_params);
                sim_params.advance_time(dt);

                nominal_paths[path_idx][step] = state.first;
                real_paths[path_idx][step] = state.second;
                index_paths[path_idx][step] = state.third;
            }
        }

        // Compute statistics per time step
        let time_grid: Vec<f64> = (0..=num_steps).map(|s| s as f64 * dt).collect();

        let nominal_stats = compute_path_stats(&nominal_paths, num_steps);
        let real_stats = compute_path_stats(&real_paths, num_steps);
        let index_stats = compute_path_stats(&index_paths, num_steps);

        // Extract sample paths
        let sample_paths: Vec<JySamplePath> = (0..num_sample)
            .map(|i| JySamplePath {
                nominal_rate: nominal_paths[i].clone(),
                real_rate: real_paths[i].clone(),
                inflation_index: index_paths[i].clone(),
            })
            .collect();

        // Realized correlation (empirical from increments)
        let correlation_realized = compute_empirical_correlation(
            &nominal_paths,
            &real_paths,
            &index_paths,
            num_paths,
            num_steps,
        );

        Ok(JySimulationResponse {
            time_grid,
            nominal_rate: nominal_stats,
            real_rate: real_stats,
            inflation_index: index_stats,
            sample_paths,
            correlation_realized,
            psd_enforced,
        })
    }

    /// Price a ZCIS using the analytical JY formula with bump-and-revalue
    /// Greeks.
    pub fn jy_price(request: &JyPricingRequest) -> Result<JyPricingResponse, ServerError> {
        let params = build_jy_params(
            &request.model_params,
            &request.correlation,
            request.initial_nominal_rate,
            request.initial_real_rate,
            request.initial_index,
        )?;

        let n_t = request.initial_nominal_rate;
        let r_t = request.initial_real_rate;
        let i_t = request.initial_index;
        let t = 0.0;
        let maturity = request.maturity;
        let notional = request.notional;
        let fixed_rate = request.fixed_rate;
        let base_index = request.initial_index;

        // Base price
        let mtm = ZcisAnalyticalPricer::price(
            &params, n_t, r_t, i_t, t, maturity, notional, fixed_rate, base_index,
        );

        let (inflation_leg_pv, fixed_leg_pv) = ZcisAnalyticalPricer::leg_pvs(
            &params, n_t, r_t, i_t, t, maturity, notional, fixed_rate, base_index,
        );

        // ── Greeks via bump-and-revalue ──
        let bp = 0.0001; // 1 basis point
        let vol_bump = 0.01; // 1% relative

        // DV01 nominal: bump initial nominal rate by 1bp
        let params_n_up = build_jy_params(
            &request.model_params,
            &request.correlation,
            request.initial_nominal_rate + bp,
            request.initial_real_rate,
            request.initial_index,
        )?;
        let mtm_n_up = ZcisAnalyticalPricer::price(
            &params_n_up,
            n_t + bp,
            r_t,
            i_t,
            t,
            maturity,
            notional,
            fixed_rate,
            base_index,
        );
        let dv01_nominal = mtm_n_up - mtm;

        // DV01 real: bump initial real rate by 1bp
        let params_r_up = build_jy_params(
            &request.model_params,
            &request.correlation,
            request.initial_nominal_rate,
            request.initial_real_rate + bp,
            request.initial_index,
        )?;
        let mtm_r_up = ZcisAnalyticalPricer::price(
            &params_r_up,
            n_t,
            r_t + bp,
            i_t,
            t,
            maturity,
            notional,
            fixed_rate,
            base_index,
        );
        let dv01_real = mtm_r_up - mtm;

        // Vega nominal: bump σ_n by 1%
        let mut model_n_vol = request.model_params.clone();
        model_n_vol.sigma_n *= 1.0 + vol_bump;
        let params_vn = build_jy_params(
            &model_n_vol,
            &request.correlation,
            request.initial_nominal_rate,
            request.initial_real_rate,
            request.initial_index,
        )?;
        let mtm_vn = ZcisAnalyticalPricer::price(
            &params_vn, n_t, r_t, i_t, t, maturity, notional, fixed_rate, base_index,
        );
        let vega_nominal = mtm_vn - mtm;

        // Vega real: bump σ_r by 1%
        let mut model_r_vol = request.model_params.clone();
        model_r_vol.sigma_r *= 1.0 + vol_bump;
        let params_vr = build_jy_params(
            &model_r_vol,
            &request.correlation,
            request.initial_nominal_rate,
            request.initial_real_rate,
            request.initial_index,
        )?;
        let mtm_vr = ZcisAnalyticalPricer::price(
            &params_vr, n_t, r_t, i_t, t, maturity, notional, fixed_rate, base_index,
        );
        let vega_real = mtm_vr - mtm;

        // Vega inflation: bump σ_I by 1%
        let mut model_i_vol = request.model_params.clone();
        model_i_vol.sigma_i *= 1.0 + vol_bump;
        let params_vi = build_jy_params(
            &model_i_vol,
            &request.correlation,
            request.initial_nominal_rate,
            request.initial_real_rate,
            request.initial_index,
        )?;
        let mtm_vi = ZcisAnalyticalPricer::price(
            &params_vi, n_t, r_t, i_t, t, maturity, notional, fixed_rate, base_index,
        );
        let vega_inflation = mtm_vi - mtm;

        // Theta: shift time forward by 1 day
        let day = 1.0 / 365.0;
        let mut params_theta = params.clone();
        params_theta.advance_time(day);
        let mtm_theta = ZcisAnalyticalPricer::price(
            &params_theta,
            n_t,
            r_t,
            i_t,
            day,
            maturity,
            notional,
            fixed_rate,
            base_index,
        );
        let theta = mtm_theta - mtm;

        Ok(JyPricingResponse {
            mtm,
            inflation_leg_pv,
            fixed_leg_pv,
            greeks: JyGreeks {
                dv01_nominal,
                dv01_real,
                vega_nominal,
                vega_real,
                vega_inflation,
                theta,
            },
        })
    }

    /// Compute XVA adjustments (CVA/DVA/FVA) using outer MC simulation.
    ///
    /// At each time step, the ZCIS is re-priced using the analytical formula
    /// given the simulated nominal rate, real rate, and inflation index.
    ///
    /// DEPRECATED: Use `POST /api/incremental-xva/run` with an
    /// `inflationSwap` incremental trade instead. This endpoint will be
    /// removed in a future release.
    pub fn jy_xva(request: &JyXvaRequest) -> Result<JyXvaResponse, ServerError> {
        let num_paths = request.num_paths as usize;
        let num_steps = request.num_steps as usize;
        let dt = request.maturity / num_steps as f64;

        let params = build_jy_params(
            &request.model_params,
            &request.correlation,
            request.initial_nominal_rate,
            request.initial_real_rate,
            request.initial_index,
        )?;

        // Clean MtM at t=0
        let clean_mtm = ZcisAnalyticalPricer::price(
            &params,
            request.initial_nominal_rate,
            request.initial_real_rate,
            request.initial_index,
            0.0,
            request.maturity,
            request.notional,
            request.fixed_rate,
            request.initial_index,
        );

        let mut rng = Xorshift64::from_time();

        // MtM matrix: [path][step]
        let mut mtm_matrix: Vec<Vec<f64>> = vec![vec![0.0; num_steps + 1]; num_paths];

        for path_idx in 0..num_paths {
            let mut state = JarrowYildirimModel::initial_state(&params);
            let mut sim_params = params.clone();
            sim_params.reset_time();

            mtm_matrix[path_idx][0] = clean_mtm;

            for step in 1..=num_steps {
                let dw = [rng.next_normal(), rng.next_normal(), rng.next_normal()];
                state = JarrowYildirimModel::evolve_step(state, dt, &dw, &sim_params);
                sim_params.advance_time(dt);

                let t = step as f64 * dt;

                // Re-price ZCIS at this state
                let mtm_t = ZcisAnalyticalPricer::price(
                    &sim_params,
                    state.first,
                    state.second,
                    state.third,
                    t,
                    request.maturity,
                    request.notional,
                    request.fixed_rate,
                    request.initial_index,
                );

                mtm_matrix[path_idx][step] = mtm_t;
            }
        }

        // Compute exposure profile
        let time_grid: Vec<f64> = (0..=num_steps).map(|s| s as f64 * dt).collect();
        let mut ee = vec![0.0; num_steps + 1]; // Expected Exposure
        let mut ene = vec![0.0; num_steps + 1]; // Negative Expected Exposure
        let mut pfe_95 = vec![0.0; num_steps + 1];
        let mut pfe_99 = vec![0.0; num_steps + 1];

        for step in 0..=num_steps {
            let mut values: Vec<f64> = (0..num_paths).map(|p| mtm_matrix[p][step]).collect();
            let n = values.len() as f64;

            // EE = E[max(V, 0)]
            ee[step] = values.iter().map(|v| v.max(0.0)).sum::<f64>() / n;

            // ENE = E[min(V, 0)]
            ene[step] = values.iter().map(|v| v.min(0.0)).sum::<f64>() / n;

            // PFE at percentiles
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            pfe_95[step] = percentile_sorted(&values, 0.95);
            pfe_99[step] = percentile_sorted(&values, 0.99);
        }

        // Integrate CVA/DVA/FVA using trapezoidal rule
        let lgd_cpty = 1.0 - request.counterparty_recovery;
        let lgd_own = 1.0 - request.own_recovery;

        let mut cva = 0.0;
        let mut dva = 0.0;
        let mut fva = 0.0;

        for step in 1..=num_steps {
            let t = time_grid[step];
            let df = (-request.nominal_curve_rate * t).exp();

            // Hazard rate from annual PD: λ = -ln(1 - PD)
            let hazard_cpty = -(1.0 - request.counterparty_pd).max(1e-15).ln();
            let hazard_own = -(1.0 - request.own_pd).max(1e-15).ln();

            // Survival probabilities
            let surv_cpty = (-hazard_cpty * t).exp();
            let surv_own = (-hazard_own * t).exp();

            // Marginal default probability in [t-dt, t]
            let dp_cpty = surv_cpty * hazard_cpty * dt;
            let dp_own = surv_own * hazard_own * dt;

            // CVA = ΣDP_cpty × LGD_cpty × EE × DF
            cva += dp_cpty * lgd_cpty * ee[step] * df;

            // DVA = ΣDP_own × LGD_own × |ENE| × DF (benefit, so negative)
            dva += dp_own * lgd_own * (-ene[step]) * df;

            // FVA = Σfunding_spread × (EE + ENE) × DF × dt
            let net_exposure = ee[step] + ene[step]; // net funding exposure
            fva += request.funding_spread * net_exposure * df * dt;
        }

        // Convention: CVA is a cost (negative), DVA is a benefit (positive)
        let cva = -cva;
        let dva = dva;
        let total_xva = cva + dva + fva;
        let adjusted_mtm = clean_mtm + total_xva;

        Ok(JyXvaResponse {
            cva,
            dva,
            fva,
            total_xva,
            clean_mtm,
            adjusted_mtm,
            exposure_profile: ExposureProfile {
                time_grid,
                expected_exposure: ee,
                negative_expected_exposure: ene,
                pfe_95,
                pfe_99,
            },
        })
    }
}

// ─── Path Statistics ────────────────────────────────────────────────────────

fn compute_path_stats(paths: &[Vec<f64>], num_steps: usize) -> SimulationPathStats {
    let num_paths = paths.len();
    let mut mean = vec![0.0; num_steps + 1];
    let mut p5 = vec![0.0; num_steps + 1];
    let mut p25 = vec![0.0; num_steps + 1];
    let mut p75 = vec![0.0; num_steps + 1];
    let mut p95 = vec![0.0; num_steps + 1];

    for step in 0..=num_steps {
        let mut vals: Vec<f64> = (0..num_paths).map(|p| paths[p][step]).collect();
        let n = vals.len() as f64;
        mean[step] = vals.iter().sum::<f64>() / n;

        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        p5[step] = percentile_sorted(&vals, 0.05);
        p25[step] = percentile_sorted(&vals, 0.25);
        p75[step] = percentile_sorted(&vals, 0.75);
        p95[step] = percentile_sorted(&vals, 0.95);
    }

    SimulationPathStats {
        mean,
        percentile_5: p5,
        percentile_25: p25,
        percentile_75: p75,
        percentile_95: p95,
    }
}

/// Compute empirical correlation between the three factors from path
/// increments.
fn compute_empirical_correlation(
    nominal: &[Vec<f64>],
    real: &[Vec<f64>],
    index: &[Vec<f64>],
    num_paths: usize,
    num_steps: usize,
) -> crate::rest::dto::jy_inflation::JyCorrelation {
    let mut sum_n = 0.0;
    let mut sum_r = 0.0;
    let mut sum_i = 0.0;
    let mut sum_nr = 0.0;
    let mut sum_ni = 0.0;
    let mut sum_ri = 0.0;
    let mut sum_nn = 0.0;
    let mut sum_rr = 0.0;
    let mut sum_ii = 0.0;
    let mut count = 0.0;

    for p in 0..num_paths {
        for s in 1..=num_steps {
            let dn = nominal[p][s] - nominal[p][s - 1];
            let dr = real[p][s] - real[p][s - 1];
            // Use log returns for index
            let di = if index[p][s - 1] > 0.0 {
                (index[p][s] / index[p][s - 1]).ln()
            } else {
                0.0
            };

            sum_n += dn;
            sum_r += dr;
            sum_i += di;
            sum_nr += dn * dr;
            sum_ni += dn * di;
            sum_ri += dr * di;
            sum_nn += dn * dn;
            sum_rr += dr * dr;
            sum_ii += di * di;
            count += 1.0;
        }
    }

    let mean_n = sum_n / count;
    let mean_r = sum_r / count;
    let mean_i = sum_i / count;

    let var_n = (sum_nn / count - mean_n * mean_n).max(1e-20);
    let var_r = (sum_rr / count - mean_r * mean_r).max(1e-20);
    let var_i = (sum_ii / count - mean_i * mean_i).max(1e-20);

    let cov_nr = sum_nr / count - mean_n * mean_r;
    let cov_ni = sum_ni / count - mean_n * mean_i;
    let cov_ri = sum_ri / count - mean_r * mean_i;

    crate::rest::dto::jy_inflation::JyCorrelation {
        rho_nr: (cov_nr / (var_n * var_r).sqrt()).clamp(-1.0, 1.0),
        rho_ni: (cov_ni / (var_n * var_i).sqrt()).clamp(-1.0, 1.0),
        rho_ri: (cov_ri / (var_r * var_i).sqrt()).clamp(-1.0, 1.0),
    }
}

// ─── Date Helpers ───────────────────────────────────────────────────────────

/// Parse ISO dates to compute maturity in years.
fn parse_maturity_years(start: &str, end: &str) -> Result<f64, ServerError> {
    let parse = |s: &str| -> Result<(i32, u32, u32), ServerError> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return Err(ServerError::InvalidRequest(format!(
                "Invalid date format: {s}"
            )));
        }
        let y = parts[0]
            .parse::<i32>()
            .map_err(|_| ServerError::InvalidRequest(format!("Invalid year in: {s}")))?;
        let m = parts[1]
            .parse::<u32>()
            .map_err(|_| ServerError::InvalidRequest(format!("Invalid month in: {s}")))?;
        let d = parts[2]
            .parse::<u32>()
            .map_err(|_| ServerError::InvalidRequest(format!("Invalid day in: {s}")))?;
        Ok((y, m, d))
    };

    let (y1, m1, d1) = parse(start)?;
    let (y2, m2, d2) = parse(end)?;

    // ACT/365 approximation
    let days1 = y1 as f64 * 365.25 + m1 as f64 * 30.44 + d1 as f64;
    let days2 = y2 as f64 * 365.25 + m2 as f64 * 30.44 + d2 as f64;
    let years = (days2 - days1) / 365.25;

    if years <= 0.0 {
        return Err(ServerError::InvalidRequest(
            "Maturity date must be after start date".to_string(),
        ));
    }

    Ok(years)
}

/// Format a date string offset from a start date by `years`.
fn format_date_offset(start: &str, years: f64) -> String {
    let parts: Vec<&str> = start.split('-').collect();
    if parts.len() != 3 {
        return start.to_string();
    }
    let y = parts[0].parse::<i32>().unwrap_or(2024);
    let m = parts[1].parse::<u32>().unwrap_or(1);
    let d = parts[2].parse::<u32>().unwrap_or(1);

    let total_months = (m as f64 + years * 12.0).round() as i32;
    let new_y = y + (total_months - 1) / 12;
    let new_m = ((total_months - 1) % 12 + 1) as u32;
    let new_d = d.min(28); // safe day

    format!("{new_y:04}-{new_m:02}-{new_d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::dto::jy_inflation::*;

    fn sample_model_params() -> JyModelParams {
        JyModelParams {
            a_n: 0.03,
            sigma_n: 0.01,
            a_r: 0.02,
            sigma_r: 0.008,
            sigma_i: 0.02,
        }
    }

    fn sample_correlation() -> JyCorrelation {
        JyCorrelation {
            rho_nr: 0.5,
            rho_ni: -0.2,
            rho_ri: -0.3,
        }
    }

    #[test]
    fn test_tenor_to_years() {
        assert!((tenor_to_years("1Y") - 1.0).abs() < 1e-10);
        assert!((tenor_to_years("6M") - 0.5).abs() < 1e-10);
        assert!((tenor_to_years("3M") - 0.25).abs() < 1e-10);
        assert!((tenor_to_years("1W") - 1.0 / 52.0).abs() < 1e-6);
    }

    #[test]
    #[ignore = "requires demo data files"]
    fn test_build_curves() {
        let req = JyCurveBuildRequest {
            nominal_curve_ref: "USD-SOFR".to_string(),
            real_rates: vec![
                CurveRatePoint {
                    instrument_type: "TIPS".to_string(),
                    tenor: "1Y".to_string(),
                    rate: 0.01,
                },
                CurveRatePoint {
                    instrument_type: "TIPS".to_string(),
                    tenor: "5Y".to_string(),
                    rate: 0.015,
                },
            ],
            valuation_date: "2024-01-15".to_string(),
            model_params: sample_model_params(),
            correlation: sample_correlation(),
        };

        let resp = DemoService::jy_build_curves(&req).unwrap();
        assert!(!resp.nominal_curve.is_empty());
        assert_eq!(resp.real_curve.len(), 2);
        assert_eq!(resp.nominal_curve.len(), resp.nominal_df.len());
    }

    #[test]
    fn test_instrument_cashflows_zcis() {
        let req = JyInstrumentRequest {
            instrument_type: "ZCIS".to_string(),
            notional: 1_000_000.0,
            fixed_rate: 0.02,
            start_date: "2024-01-15".to_string(),
            maturity_date: "2029-01-15".to_string(),
            payment_frequency: "annual".to_string(),
            nominal_curve_rate: 0.035,
            real_curve_rate: 0.01,
        };

        let resp = DemoService::jy_instrument_cashflows(&req).unwrap();
        assert_eq!(resp.instrument_type, "ZCIS");
        assert_eq!(resp.cashflows.len(), 1); // ZCIS = single payment at maturity
        assert!(resp.summary.maturity_years > 4.5 && resp.summary.maturity_years < 5.5);
    }

    #[test]
    fn test_instrument_cashflows_yoy() {
        let req = JyInstrumentRequest {
            instrument_type: "YoYIS".to_string(),
            notional: 1_000_000.0,
            fixed_rate: 0.02,
            start_date: "2024-01-15".to_string(),
            maturity_date: "2029-01-15".to_string(),
            payment_frequency: "annual".to_string(),
            nominal_curve_rate: 0.035,
            real_curve_rate: 0.01,
        };

        let resp = DemoService::jy_instrument_cashflows(&req).unwrap();
        assert_eq!(resp.instrument_type, "YoYIS");
        assert!(resp.cashflows.len() >= 4); // ~5 annual payments
    }

    #[test]
    fn test_pricing() {
        let req = JyPricingRequest {
            model_params: sample_model_params(),
            correlation: sample_correlation(),
            initial_nominal_rate: 0.03,
            initial_real_rate: 0.01,
            initial_index: 100.0,
            notional: 1_000_000.0,
            fixed_rate: 0.02,
            maturity: 5.0,
            nominal_curve_rate: 0.03,
            real_curve_rate: 0.01,
        };

        let resp = DemoService::jy_price(&req).unwrap();
        assert!(resp.mtm.is_finite());
        assert!(resp.inflation_leg_pv > 0.0);
        assert!(resp.fixed_leg_pv > 0.0);
        // Greeks should be finite
        assert!(resp.greeks.dv01_nominal.is_finite());
        assert!(resp.greeks.dv01_real.is_finite());
        assert!(resp.greeks.theta.is_finite());
    }

    #[test]
    fn test_simulation_small() {
        let req = JySimulationRequest {
            model_params: sample_model_params(),
            correlation: sample_correlation(),
            num_paths: 100,
            num_steps: 50,
            horizon: 5.0,
            initial_nominal_rate: 0.03,
            initial_real_rate: 0.01,
            initial_index: 100.0,
            num_sample_paths: 3,
        };

        let resp = DemoService::jy_simulate(&req).unwrap();
        assert_eq!(resp.time_grid.len(), 51); // 50 steps + initial
        assert_eq!(resp.nominal_rate.mean.len(), 51);
        assert_eq!(resp.sample_paths.len(), 3);
        // Mean nominal rate should be roughly near initial
        assert!((resp.nominal_rate.mean[0] - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_xva() {
        let req = JyXvaRequest {
            model_params: sample_model_params(),
            correlation: sample_correlation(),
            initial_nominal_rate: 0.03,
            initial_real_rate: 0.01,
            initial_index: 100.0,
            notional: 1_000_000.0,
            fixed_rate: 0.02,
            maturity: 5.0,
            nominal_curve_rate: 0.03,
            real_curve_rate: 0.01,
            counterparty_pd: 0.01,
            counterparty_recovery: 0.4,
            own_pd: 0.005,
            own_recovery: 0.4,
            funding_spread: 0.005,
            num_paths: 100,
            num_steps: 25,
        };

        let resp = DemoService::jy_xva(&req).unwrap();
        assert!(resp.clean_mtm.is_finite());
        assert!(resp.cva.is_finite());
        assert!(resp.dva.is_finite());
        assert!(resp.fva.is_finite());
        assert!((resp.total_xva - (resp.cva + resp.dva + resp.fva)).abs() < 1e-10);
        assert_eq!(resp.exposure_profile.time_grid.len(), 26);
    }

    #[test]
    fn test_xorshift_basic() {
        let mut rng = Xorshift64::new(42);
        let mut values = Vec::new();
        for _ in 0..1000 {
            let v = rng.next_f64();
            assert!(v >= 0.0 && v < 1.0);
            values.push(v);
        }
        // Should not be all the same
        let first = values[0];
        assert!(values.iter().any(|&v| (v - first).abs() > 0.01));
    }

    #[test]
    fn test_box_muller_normal() {
        let mut rng = Xorshift64::new(123);
        let n = 10000;
        let mut sum = 0.0;
        let mut sum_sq = 0.0;
        for _ in 0..n {
            let z = rng.next_normal();
            assert!(z.is_finite());
            sum += z;
            sum_sq += z * z;
        }
        let mean = sum / n as f64;
        let var = sum_sq / n as f64 - mean * mean;
        // Mean ≈ 0, Variance ≈ 1
        assert!(mean.abs() < 0.1, "Mean = {mean}");
        assert!((var - 1.0).abs() < 0.2, "Variance = {var}");
    }

    #[test]
    fn test_percentile_sorted() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile_sorted(&data, 0.0) - 1.0).abs() < 1e-10);
        assert!((percentile_sorted(&data, 1.0) - 5.0).abs() < 1e-10);
        assert!((percentile_sorted(&data, 0.5) - 3.0).abs() < 1e-10);
    }
}
