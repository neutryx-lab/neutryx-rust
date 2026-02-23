//! Incremental XVA engine service for demo GUI.
//!
//! Orchestrates the incremental XVA pipeline: converts DTOs to domain types,
//! builds MFM grid caches for exotic products, and runs the engine.

use std::time::Instant;

use pricer_pricing::methods::tree::grid_cache::{ExoticProductType, MfmGridCache, MfmMtmSlice};
use pricer_risk::{
    portfolio::{xva::OwnCreditParams, CreditParams},
    xva_engine::{
        incremental::{
            ExoticTradeDef, InflationSwapDef, IncrementalPortfolio, IncrementalTrade,
            IncrementalXvaConfig, IncrementalXvaEngine, JyInflationConfig, VanillaSwapDef,
        },
        model_coupler::CouplingMethod,
    },
};

use crate::{
    error::ServerError,
    rest::dto::incremental_xva::{
        ExoticDefinitionDto, InflationSwapDefinitionDto, IncrementalTradeDto,
        IncrementalXvaDefaultConfig, IncrementalXvaRequest, IncrementalXvaResponse,
        SwapDefinitionDto, XvaMetricsDto,
    },
};

/// Stateless incremental XVA service.
pub struct IncrementalXvaService;

impl IncrementalXvaService {
    /// Returns default demo configuration with a pre-populated portfolio.
    ///
    /// Demo portfolio:
    /// - Base: 2 IRS (one payer, one receiver) + 1 TARN + 1 Bermudan
    /// - Incremental: 1 CIF (Callable Inverse Floater)
    pub fn get_default_config() -> Result<IncrementalXvaDefaultConfig, ServerError> {
        let base_swaps = vec![
            SwapDefinitionDto {
                trade_id: "IRS_PAY_5Y".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: "semi-annual".to_string(),
                is_payer: true,
            },
            SwapDefinitionDto {
                trade_id: "IRS_RCV_10Y".to_string(),
                notional: 5_000_000.0,
                fixed_rate: 0.035,
                tenor_years: 10.0,
                payment_frequency: "semi-annual".to_string(),
                is_payer: false,
            },
        ];

        let base_exotics = vec![
            ExoticDefinitionDto {
                trade_id: "TARN_7Y".to_string(),
                product_type: "tarn".to_string(),
                notional: 5_000_000.0,
                mfm_mean_reversion: 0.05,
                mfm_volatility: 0.01,
                mfm_grid_points: Some(41),
                mfm_num_std_devs: Some(5.0),
                exercise_times: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
                swap_tenors: vec![1.0; 7],
                payment_frequency: Some(0.5),
                funding_rate: Some(0.03),
                coupon_rate: Some(0.03),
                flat_vol_bps: Some(50.0),
                fixed_rate: Some(0.05),
                is_callable: None,
                tarn_target: Some(0.15),
                tarn_coupon_grid_points: Some(20),
                leverage: None,
                floor_rate: None,
                cap_rate: None,
            },
            ExoticDefinitionDto {
                trade_id: "BERM_10Y".to_string(),
                product_type: "bermudan".to_string(),
                notional: 8_000_000.0,
                mfm_mean_reversion: 0.05,
                mfm_volatility: 0.01,
                mfm_grid_points: Some(41),
                mfm_num_std_devs: Some(5.0),
                exercise_times: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
                swap_tenors: vec![9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.5],
                payment_frequency: Some(0.5),
                funding_rate: Some(0.03),
                coupon_rate: Some(0.03),
                flat_vol_bps: Some(50.0),
                fixed_rate: Some(0.03),
                is_callable: Some(true),
                tarn_target: None,
                tarn_coupon_grid_points: None,
                leverage: None,
                floor_rate: None,
                cap_rate: None,
            },
        ];

        let incremental_trade = IncrementalTradeDto::Exotic(ExoticDefinitionDto {
            trade_id: "CIF_5Y".to_string(),
            product_type: "cif".to_string(),
            notional: 3_000_000.0,
            mfm_mean_reversion: 0.05,
            mfm_volatility: 0.01,
            mfm_grid_points: Some(41),
            mfm_num_std_devs: Some(5.0),
            exercise_times: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            swap_tenors: vec![1.0; 5],
            payment_frequency: Some(0.5),
            funding_rate: Some(0.03),
            coupon_rate: Some(0.03),
            flat_vol_bps: Some(50.0),
            fixed_rate: Some(0.06),
            is_callable: Some(true),
            tarn_target: None,
            tarn_coupon_grid_points: None,
            leverage: Some(1.5),
            floor_rate: Some(0.0),
            cap_rate: Some(0.08),
        });

        Ok(IncrementalXvaDefaultConfig {
            n_paths: 10_000,
            horizon_years: 10.0,
            time_step: "quarterly".to_string(),
            antithetic: true,
            bilateral: true,
            compute_fva: true,
            hw_mean_reversion: 0.05,
            hw_volatility: 0.01,
            hw_initial_rate: 0.03,
            coupling_method: "swap_rate".to_string(),
            hazard_rate: 0.02,
            lgd: 0.6,
            jy_real_mean_reversion: Some(0.03),
            jy_real_volatility: Some(0.005),
            jy_initial_real_rate: Some(0.01),
            jy_inflation_volatility: Some(0.02),
            jy_initial_index: Some(100.0),
            jy_rho_nominal_real: Some(0.3),
            jy_rho_nominal_inflation: Some(-0.1),
            jy_rho_real_inflation: Some(-0.2),
            base_swaps,
            base_exotics,
            base_inflation_swaps: vec![],
            incremental_trade,
        })
    }

    /// Run the incremental XVA computation.
    pub fn run(request: &IncrementalXvaRequest) -> Result<IncrementalXvaResponse, ServerError> {
        let start = Instant::now();

        // ── Parse configuration ──
        let n_paths = request.n_paths.unwrap_or(10_000);
        let horizon = request.horizon_years.unwrap_or(10.0);
        let antithetic = request.antithetic.unwrap_or(true);
        let bilateral = request.bilateral.unwrap_or(true);
        let compute_fva = request.compute_fva.unwrap_or(true);

        let time_grid =
            Self::build_time_grid(horizon, request.time_step.as_deref().unwrap_or("quarterly"));

        let coupling_method = match request.coupling_method.as_str() {
            "zscore" => CouplingMethod::ZScoreMatching,
            _ => CouplingMethod::MarketObservableMapping {
                swap_tenor: request.coupling_swap_tenor.unwrap_or(10.0),
                payment_freq: request.coupling_payment_freq.unwrap_or(0.5),
            },
        };

        // ── JY inflation config (optional) ──
        let jy_inflation = request.jy_real_mean_reversion.map(|a_r| JyInflationConfig {
            real_mean_reversion: a_r,
            real_volatility: request.jy_real_volatility.unwrap_or(0.005),
            initial_real_rate: request.jy_initial_real_rate.unwrap_or(0.01),
            inflation_volatility: request.jy_inflation_volatility.unwrap_or(0.02),
            initial_index: request.jy_initial_index.unwrap_or(100.0),
            rho_nominal_real: request.jy_rho_nominal_real.unwrap_or(0.3),
            rho_nominal_inflation: request.jy_rho_nominal_inflation.unwrap_or(-0.1),
            rho_real_inflation: request.jy_rho_real_inflation.unwrap_or(-0.2),
        });

        let config = IncrementalXvaConfig {
            n_paths,
            time_grid: time_grid.clone(),
            seed: request.seed,
            antithetic,
            hw_mean_reversion: request.hw_mean_reversion,
            hw_volatility: request.hw_volatility,
            hw_initial_rate: request.hw_initial_rate,
            coupling_method,
            bilateral,
            compute_fva,
            funding_spread_borrow: request.funding_spread.unwrap_or(0.005),
            funding_spread_lend: request.funding_spread.unwrap_or(0.005) * 0.6,
            jy_inflation,
        };

        // ── Build base vanilla swaps ──
        let base_swaps: Vec<VanillaSwapDef> = request
            .base_swaps
            .iter()
            .map(|dto| Self::build_swap_def(dto))
            .collect();

        // ── Build base exotic grid caches ──
        let base_exotics: Vec<ExoticTradeDef> = request
            .base_exotics
            .iter()
            .map(|dto| Self::build_exotic_def(dto))
            .collect::<Result<Vec<_>, _>>()?;

        // ── Build base inflation swaps ──
        let base_inflation_swaps: Vec<InflationSwapDef> = request
            .base_inflation_swaps
            .iter()
            .map(|dto| Self::build_inflation_swap_def(dto))
            .collect();

        // ── Build incremental trade ──
        let incremental_trade = match &request.incremental_trade {
            IncrementalTradeDto::Swap(dto) => IncrementalTrade::Swap(Self::build_swap_def(dto)),
            IncrementalTradeDto::Exotic(dto) => {
                IncrementalTrade::Exotic(Self::build_exotic_def(dto)?)
            }
            IncrementalTradeDto::InflationSwap(dto) => {
                IncrementalTrade::InflationSwap(Self::build_inflation_swap_def(dto))
            }
        };

        let portfolio = IncrementalPortfolio {
            base_swaps,
            base_exotics,
            base_inflation_swaps,
            incremental_trade,
        };

        // ── Credit parameters ──
        let credit_params = CreditParams::new(request.hazard_rate, request.lgd)
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid credit params: {e}")))?;

        let own_credit = OwnCreditParams::new(
            request.own_hazard_rate.unwrap_or(0.01),
            request.own_lgd.unwrap_or(0.4),
        )
        .map_err(|e| ServerError::InvalidRequest(format!("Invalid own credit params: {e}")))?;

        // ── Run engine ──
        let result = IncrementalXvaEngine::run(&config, &portfolio, &credit_params, &own_credit)
            .map_err(|e| ServerError::Pricing(format!("Incremental XVA engine error: {e}")))?;

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        // ── Convert to response ──
        Ok(IncrementalXvaResponse {
            time_grid: result.time_grid,
            n_paths: result.n_paths,
            base_xva: Self::metrics_to_dto(&result.base_xva),
            full_xva: Self::metrics_to_dto(&result.full_xva),
            incremental_xva: Self::metrics_to_dto(&result.incremental_xva),
            base_epe: result.base_epe,
            base_ene: result.base_ene,
            full_epe: result.full_epe,
            full_ene: result.full_ene,
            coupling_method: request.coupling_method.clone(),
            computation_time_ms: elapsed_ms,
        })
    }

    // ── Private helpers ──

    fn build_time_grid(horizon_years: f64, time_step: &str) -> Vec<f64> {
        let step = match time_step {
            "monthly" => 1.0 / 12.0,
            "semi-annual" => 0.5,
            _ => 0.25,
        };

        let n_steps = (horizon_years / step).ceil() as usize;
        (1..=n_steps)
            .map(|i| (i as f64 * step).min(horizon_years))
            .collect()
    }

    fn build_swap_def(dto: &SwapDefinitionDto) -> VanillaSwapDef {
        let payment_freq = match dto.payment_frequency.as_str() {
            "quarterly" => 0.25,
            "annual" => 1.0,
            _ => 0.5, // semi-annual default
        };

        let n_periods = (dto.tenor_years / payment_freq).round() as usize;
        let payment_times: Vec<f64> = (1..=n_periods).map(|i| i as f64 * payment_freq).collect();

        VanillaSwapDef {
            trade_id: dto.trade_id.clone(),
            notional: dto.notional,
            fixed_rate: dto.fixed_rate,
            payment_times,
            is_payer: dto.is_payer,
        }
    }

    fn build_inflation_swap_def(dto: &InflationSwapDefinitionDto) -> InflationSwapDef {
        InflationSwapDef {
            trade_id: dto.trade_id.clone(),
            notional: dto.notional,
            fixed_rate: dto.fixed_rate,
            maturity: dto.maturity_years,
            base_index: dto.base_index,
        }
    }

    fn build_exotic_def(dto: &ExoticDefinitionDto) -> Result<ExoticTradeDef, ServerError> {
        // Build a synthetic MFM grid cache from the exotic definition.
        // In a production system, this would run full MFM calibration + tree pricing.
        // For the demo, we build a synthetic grid that maps swap rates to reasonable
        // MtM.
        let grid_cache = Self::build_synthetic_grid_cache(dto)?;

        Ok(ExoticTradeDef {
            trade_id: dto.trade_id.clone(),
            notional: dto.notional,
            grid_cache,
        })
    }

    /// Build a synthetic MFM grid cache for demo purposes.
    ///
    /// Creates a realistic-looking grid where MtM varies with the Gaussian
    /// state variable, mimicking what full MFM calibration + tree pricing
    /// would produce.
    fn build_synthetic_grid_cache(dto: &ExoticDefinitionDto) -> Result<MfmGridCache, ServerError> {
        let a = dto.mfm_mean_reversion;
        let sigma = dto.mfm_volatility;
        let n_grid = dto.mfm_grid_points.unwrap_or(41);
        let n_std = dto.mfm_num_std_devs.unwrap_or(5.0);
        let fixed_rate = dto.fixed_rate.unwrap_or(0.03);
        let notional = dto.notional;

        let product_type = match dto.product_type.as_str() {
            "tarn" => ExoticProductType::Tarn,
            "cif" => ExoticProductType::Cif,
            _ => ExoticProductType::Bermudan,
        };

        let mut slices = Vec::with_capacity(dto.exercise_times.len());

        for (i, &t) in dto.exercise_times.iter().enumerate() {
            // Terminal variance for grid extent
            let var_t = if a.abs() > 1e-10 {
                (sigma * sigma / (2.0 * a)) * (1.0 - (-2.0 * a * t).exp())
            } else {
                sigma * sigma * t
            };
            let std_t = var_t.sqrt().max(1e-10);

            // Build x_grid
            let center = n_grid / 2;
            let dx = 2.0 * n_std * std_t / (n_grid as f64 - 1.0).max(1.0);
            let x_grid: Vec<f64> = (0..n_grid)
                .map(|j| (j as f64 - center as f64) * dx)
                .collect();

            // Build swap rates: monotonically map x to swap rate
            // s(x) = funding_rate + sensitivity * x
            let base_rate = dto.funding_rate.unwrap_or(0.03);
            let sensitivity = 0.5; // swap rate sensitivity to x
            let swap_rates: Vec<f64> = x_grid
                .iter()
                .map(|&x| (base_rate + sensitivity * x).max(0.001))
                .collect();

            // Build MtM values
            // For Bermudan: optionality value increases with distance from strike
            // For TARN: similar but with auto-redemption cap
            // For CIF: inverse floater payoff
            let tenor_remaining = dto.swap_tenors.get(i).copied().unwrap_or(5.0);
            let mtm_values: Vec<f64> = swap_rates
                .iter()
                .map(|&sr| {
                    let diff = sr - fixed_rate;
                    match product_type {
                        ExoticProductType::Bermudan => {
                            // Callable swaption-like payoff
                            let annuity = tenor_remaining * 0.98; // approximate
                            let intrinsic = if dto.is_callable.unwrap_or(true) {
                                (-diff * annuity).max(0.0) // callable: value
                                                           // when rates drop
                            } else {
                                (diff * annuity).max(0.0)
                            };
                            intrinsic * notional / notional // normalised by
                                                            // notional
                        }
                        ExoticProductType::Tarn => {
                            // TARN: coupon value with target cap
                            let tarn_target = dto.tarn_target.unwrap_or(0.15);
                            let coupon = (fixed_rate - sr).max(0.0);
                            let capped = coupon.min(tarn_target / tenor_remaining.max(1.0));
                            capped * tenor_remaining * notional / notional
                        }
                        ExoticProductType::Cif => {
                            // CIF: leverage * (fixed - floating) with floor/cap
                            let leverage = dto.leverage.unwrap_or(1.5);
                            let floor = dto.floor_rate.unwrap_or(0.0);
                            let cap = dto.cap_rate.unwrap_or(0.08);
                            let raw = leverage * (fixed_rate - sr);
                            let capped = raw.max(floor).min(cap);
                            capped * tenor_remaining * notional / notional
                        }
                    }
                })
                .collect();

            slices.push(MfmMtmSlice {
                time: t,
                x_grid,
                swap_rates,
                mtm_values,
            });
        }

        Ok(MfmGridCache::new(
            dto.trade_id.clone(),
            product_type,
            slices,
            a,
            sigma,
        ))
    }

    fn metrics_to_dto(metrics: &pricer_risk::xva_engine::incremental::XvaMetrics) -> XvaMetricsDto {
        XvaMetricsDto {
            ucva: metrics.ucva,
            bcva: metrics.bcva,
            udva: metrics.udva,
            bdva: metrics.bdva,
            fca: metrics.fca,
            fba: metrics.fba,
            fva: metrics.fva,
            total: metrics.total,
        }
    }
}
