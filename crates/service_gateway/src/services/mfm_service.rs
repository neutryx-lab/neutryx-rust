//! Service for Markov Functional Model calibration and pricing.

use std::time::Instant;

use pricer_models::markov_functional::{
    CalibratedSlice, FlatSwaptionVolCube, MarkovFunctionalNonParametric1F,
    MfmCalibrationResult, MfmConfig, MfmRateIndex, MfmVolType, RateIndexCalibration,
    SabrSwaptionVolCube, SwaptionVolCubeEnum,
};
use pricer_models::markov_functional::cif_evaluator::{
    CifInstrument, compute_cif_node_info,
};
use pricer_models::market::{CurveEnum, YieldCurve};
use pricer_pricing::tree::{
    BermudanTreeConfig, BermudanTreeEngine, CouponInfo, ExerciseInfo,
    GaussianTree, GaussianTreeConfig, TarnConfig, TarnCouponInfo, TarnExerciseInfo, TarnTreeEngine,
};

use crate::rest::dto::mfm::*;

/// MFM service providing calibration and pricing endpoints.
pub struct MfmService;

impl MfmService {
    // ─── Calibration ────────────────────────────────────────────────────

    /// Calibrate the MFM model and return calibration results.
    pub fn calibrate(request: &MfmCalibrateRequest) -> Result<MfmCalibrateResponse, String> {
        let start = Instant::now();

        let vol_type = match request.vol_type {
            MfmVolTypeDto::Normal => MfmVolType::Normal,
            MfmVolTypeDto::Lognormal => MfmVolType::Lognormal,
        };

        let config = MfmConfig {
            mean_reversion: request.mean_reversion,
            volatility: request.volatility,
            num_grid_points: request.num_grid_points,
            num_std_devs: request.num_std_devs,
            vol_type,
            exercise_times: request.exercise_times.clone(),
            swap_tenors: request.swap_tenors.clone(),
            payment_frequencies: request.payment_frequencies.clone(),
            ..MfmConfig::default()
        };

        let model = MarkovFunctionalNonParametric1F::new(config)
            .map_err(|e| format!("MFM config error: {e}"))?;

        let funding_curve = CurveEnum::flat(request.funding_curve.rate);
        let coupon_curve = CurveEnum::flat(request.coupon_curve.rate);
        let vol_cube = Self::build_vol_cube(request)?;

        // Wrap curves as discount factor functions Fn(f64) -> f64
        let funding_df = |t: f64| -> f64 {
            funding_curve.discount_factor(t).unwrap_or(1.0)
        };
        let coupon_df = |t: f64| -> f64 {
            coupon_curve.discount_factor(t).unwrap_or(1.0)
        };

        let result = model
            .calibrate(&funding_df, &coupon_df, &vol_cube)
            .map_err(|e| format!("MFM calibration error: {e}"))?;

        Ok(Self::convert_calibration_result(&result, start))
    }

    // ─── Gaussian Tree ──────────────────────────────────────────────────

    /// Build a Gaussian tree and return its structure.
    pub fn build_gaussian_tree(
        request: &GaussianTreeRequest,
    ) -> Result<GaussianTreeResponse, String> {
        let start = Instant::now();

        let config = GaussianTreeConfig {
            mean_reversion: request.mean_reversion,
            volatility: request.volatility,
            times: request.times.clone(),
            num_std_devs: request.num_std_devs,
            num_grid_points: request.num_grid_points,
        };

        let tree =
            GaussianTree::build(config).map_err(|e| format!("Gaussian tree error: {e}"))?;

        let ad_prices = tree.arrow_debreu_prices();

        let slices: Vec<GaussianTreeSliceDto> = tree
            .slices
            .iter()
            .map(|s| GaussianTreeSliceDto {
                time: s.time,
                x_grid: s.x_grid.clone(),
                dx: s.dx,
                conditional_variance: s.conditional_variance,
            })
            .collect();

        Ok(GaussianTreeResponse {
            num_steps: tree.num_steps(),
            num_nodes: tree.num_nodes(),
            slices,
            arrow_debreu_prices: ad_prices,
            computation_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        })
    }

    // ─── CIF Evaluation ─────────────────────────────────────────────────

    /// Evaluate CIF coupon components.
    pub fn evaluate_cif(request: &CifEvaluateRequest) -> Result<CifEvaluateResponse, String> {
        let start = Instant::now();

        let n_coupons = request.coupon_dates.len();
        if n_coupons == 0 {
            return Err("coupon_dates must not be empty".to_string());
        }
        if request.payment_dates.len() != n_coupons
            || request.year_fractions.len() != n_coupons
            || request.swap_rates.len() != n_coupons
            || request.libor_rates.len() != n_coupons
            || request.discount_factors.len() != n_coupons
            || request.forward_swap_rates.len() != n_coupons
            || request.forward_libors.len() != n_coupons
            || request.normal_vols.len() != n_coupons
        {
            return Err("All arrays must have the same length as coupon_dates".to_string());
        }

        let instrument = CifInstrument {
            fixed_rate: request.instrument.fixed_rate,
            leverage: request.instrument.leverage,
            floor_rate: request.instrument.floor_rate,
            cap_rate: request.instrument.cap_rate,
            notional: request.instrument.notional,
            coupon_dates: request.coupon_dates.clone(),
            payment_dates: request.payment_dates.clone(),
            year_fractions: request.year_fractions.clone(),
        };

        let mut coupons = Vec::with_capacity(n_coupons);

        for i in 0..n_coupons {
            let node_info = compute_cif_node_info(
                &instrument,
                i,
                &request.swap_rates[i],
                &request.libor_rates[i],
                &request.discount_factors[i],
                request.forward_swap_rates[i],
                request.forward_libors[i],
                request.normal_vols[i],
                request.coupon_dates[i],
            )
            .map_err(|e| format!("CIF evaluation error at coupon {i}: {e}"))?;

            let n_nodes = request.swap_rates[i].len();
            let mut d_e = Vec::with_capacity(n_nodes);
            let mut d_r = Vec::with_capacity(n_nodes);
            let mut d_i = Vec::with_capacity(n_nodes);
            let mut d_q = Vec::with_capacity(n_nodes);
            let mut total = Vec::with_capacity(n_nodes);

            for c in &node_info.components {
                d_e.push(c.d_e);
                d_r.push(c.d_r);
                d_i.push(c.d_i);
                d_q.push(c.d_q);
                total.push(c.total());
            }

            coupons.push(CifCouponInfoDto {
                coupon_idx: node_info.coupon_idx,
                coupon_date_yf: node_info.coupon_date_yf,
                payment_date_yf: node_info.payment_date_yf,
                year_fraction: node_info.year_fraction,
                forward_swap_rate: node_info.forward_swap_rate,
                forward_libor: node_info.forward_libor,
                normal_vol: node_info.normal_vol,
                components: CifComponentsDto {
                    d_e,
                    d_r,
                    d_i,
                    d_q,
                    total,
                },
                discounted_values: node_info.discounted_values.clone(),
            });
        }

        Ok(CifEvaluateResponse {
            coupons,
            computation_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        })
    }

    // ─── Bermudan Pricing ───────────────────────────────────────────────

    /// Price a Bermudan swaption using MFM + Gaussian tree.
    pub fn price_bermudan(request: &BermudanPriceRequest) -> Result<BermudanPriceResponse, String> {
        let start = Instant::now();

        // 1. Build Gaussian tree
        let tree_config = GaussianTreeConfig {
            mean_reversion: request.mean_reversion,
            volatility: request.volatility,
            times: request.exercise_times.clone(),
            num_std_devs: request.num_std_devs,
            num_grid_points: request.num_grid_points,
        };
        let tree =
            GaussianTree::build(tree_config).map_err(|e| format!("Tree build error: {e}"))?;

        let n_nodes = tree.num_nodes();
        let n_steps = tree.num_steps();

        // 2. Build coupon info (simplified: flat coupon value at each step)
        let flat_coupon = request.flat_coupon.unwrap_or(0.0);
        let mut coupons: Vec<(usize, CouponInfo<f64>)> = Vec::new();
        for step in 0..n_steps {
            coupons.push((
                step,
                CouponInfo {
                    values: vec![flat_coupon; n_nodes],
                },
            ));
        }

        // 3. Build exercise info (simplified: par value at exercise dates)
        let mut exercises: Vec<(usize, ExerciseInfo<f64>)> = Vec::new();
        for step in 0..n_steps {
            // Exercise value = par (1.0) for a swaption
            exercises.push((
                step,
                ExerciseInfo {
                    values: vec![0.0; n_nodes],
                },
            ));
        }

        // 4. Build Bermudan config
        let bermudan_config = BermudanTreeConfig {
            is_callable: request.is_callable,
            exercise_times: request.exercise_times.clone(),
            exercise_costs: vec![0.0; n_steps],
            coupon_times: request.exercise_times.clone(),
        };

        // 5. Price
        let result = BermudanTreeEngine::price(&tree, &bermudan_config, &coupons, &exercises);

        Ok(BermudanPriceResponse {
            pv: result.pv,
            continuation_value: result.continuation_value,
            option_value: result.option_value,
            exercise_boundary: result.exercise_boundary,
            computation_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        })
    }

    // ─── TARN Pricing ───────────────────────────────────────────────────

    /// Price a TARN using MFM + Gaussian tree + 2D state space.
    pub fn price_tarn(request: &TarnPriceRequest) -> Result<TarnPriceResponse, String> {
        let start = Instant::now();

        // 1. Build Gaussian tree
        let tree_config = GaussianTreeConfig {
            mean_reversion: request.mean_reversion,
            volatility: request.volatility,
            times: request.exercise_times.clone(),
            num_std_devs: request.num_std_devs,
            num_grid_points: request.num_grid_points,
        };
        let tree =
            GaussianTree::build(tree_config).map_err(|e| format!("Tree build error: {e}"))?;

        let n_nodes = tree.num_nodes();
        let n_steps = tree.num_steps();

        // 2. Build TARN config
        let tarn_config = TarnConfig {
            tarn_amount: request.tarn_amount,
            num_coupon_grid_points: request.num_coupon_grid_points,
            excess_coupon_flag: request.excess_coupon_flag,
            has_bermudan_exercise: request.has_bermudan_exercise,
            is_callable: request.is_callable,
        };

        // 3. Build coupon info
        let flat_coupon = request.flat_coupon.unwrap_or(0.0);
        let mut coupons: Vec<(usize, TarnCouponInfo<f64>)> = Vec::new();
        for step in 0..n_steps {
            coupons.push((
                step,
                TarnCouponInfo {
                    node_coupons: vec![flat_coupon; n_nodes],
                },
            ));
        }

        // 4. Build exercise info
        let mut exercises: Vec<(usize, TarnExerciseInfo<f64>)> = Vec::new();
        if request.has_bermudan_exercise {
            for step in 0..n_steps {
                exercises.push((
                    step,
                    TarnExerciseInfo {
                        node_values: vec![0.0; n_nodes],
                    },
                ));
            }
        }

        // 5. Price
        let redemption_value = 0.0;
        let result = TarnTreeEngine::price(
            &tree,
            &tarn_config,
            &coupons,
            &exercises,
            redemption_value,
        );

        Ok(TarnPriceResponse {
            pv: result.pv,
            auto_redemption_probability: result.auto_redemption_probability,
            expected_redemption_time: result.expected_redemption_time,
            computation_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        })
    }

    // ─── Product Definitions ────────────────────────────────────────────

    /// Returns available MFM product definitions for UI rendering.
    pub fn get_products() -> Vec<MfmProductDef> {
        vec![
            Self::calibration_product_def(),
            Self::gaussian_tree_product_def(),
            Self::bermudan_product_def(),
            Self::tarn_product_def(),
            Self::cif_product_def(),
        ]
    }

    // ─── Helpers ────────────────────────────────────────────────────────

    fn build_vol_cube(
        request: &MfmCalibrateRequest,
    ) -> Result<SwaptionVolCubeEnum<f64>, String> {
        match request.vol_surface_type {
            MfmVolSurfaceType::Flat => {
                let vol_bp = request
                    .flat_vol
                    .as_ref()
                    .map(|v| v.normal_vol_bp)
                    .unwrap_or(80.0);
                let vol = vol_bp / 10000.0;
                let cube = FlatSwaptionVolCube::from_normal_vol(vol)
                    .map_err(|e| format!("Flat vol cube error: {e}"))?;
                Ok(SwaptionVolCubeEnum::Flat(cube))
            }
            MfmVolSurfaceType::Sabr => {
                let sabr = request
                    .sabr_vol
                    .as_ref()
                    .ok_or_else(|| "sabr_vol must be provided when vol_surface_type is sabr".to_string())?;
                let cube = SabrSwaptionVolCube::new(
                    sabr.expiries.clone(),
                    sabr.tenors.clone(),
                    sabr.alphas.clone(),
                    sabr.betas.clone(),
                    sabr.rhos.clone(),
                    sabr.nus.clone(),
                )
                .map_err(|e| format!("SABR vol cube error: {e}"))?;
                Ok(SwaptionVolCubeEnum::Sabr(cube))
            }
        }
    }

    fn convert_calibration_result(
        result: &MfmCalibrationResult<f64>,
        start: Instant,
    ) -> MfmCalibrateResponse {
        MfmCalibrateResponse {
            funding_calibration: Self::convert_rate_index_calibration(
                &result.funding_calibration,
            ),
            coupon_swap_calibration: Self::convert_rate_index_calibration(
                &result.coupon_swap_calibration,
            ),
            coupon_libor_calibration: Self::convert_rate_index_calibration(
                &result.coupon_libor_calibration,
            ),
            adjuster: IntegralAdjusterDto {
                adders: result.adjuster.adders.clone(),
                multipliers: result.adjuster.multipliers.clone(),
            },
            max_nr_iterations_used: result.max_nr_iterations_used,
            max_calibration_error: result.max_calibration_error,
            computation_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        }
    }

    fn convert_rate_index_calibration(
        cal: &RateIndexCalibration<f64>,
    ) -> RateIndexCalibrationDto {
        let rate_index = match cal.rate_index {
            MfmRateIndex::FundingIndexSwapRate => "FundingIndexSwapRate",
            MfmRateIndex::CouponIndexSwapRate => "CouponIndexSwapRate",
            MfmRateIndex::CouponLibor => "CouponLibor",
        };
        RateIndexCalibrationDto {
            rate_index: rate_index.to_string(),
            slices: cal
                .slices
                .iter()
                .map(Self::convert_calibrated_slice)
                .collect(),
        }
    }

    fn convert_calibrated_slice(slice: &CalibratedSlice<f64>) -> CalibratedSliceDto {
        CalibratedSliceDto {
            exercise_time: slice.exercise_time,
            x_grid: slice.x_grid.clone(),
            swap_rates: slice.swap_rates.clone(),
            discount_factors: slice.discount_factors.clone(),
            annuities: slice.annuities.clone(),
        }
    }

    // ─── Product Definition Builders ────────────────────────────────────

    fn calibration_product_def() -> MfmProductDef {
        MfmProductDef {
            product_type: "calibration".to_string(),
            display_name: "MFM Calibration".to_string(),
            description: "Calibrate 1F Non-Parametric Markov Functional Model".to_string(),
            parameters: vec![
                Self::param("meanReversion", "Mean Reversion (a)", "number", true, Some(serde_json::json!(0.03)), None, Some("Model")),
                Self::param("volatility", "Gaussian Vol (σ)", "number", true, Some(serde_json::json!(0.01)), None, Some("Model")),
                Self::param("numGridPoints", "Grid Points", "number", false, Some(serde_json::json!(41)), Some("Odd number (e.g. 41)"), Some("Model")),
                Self::param("numStdDevs", "Std Devs", "number", false, Some(serde_json::json!(5.0)), None, Some("Model")),
                Self::param("volType", "Vol Type", "select", false, Some(serde_json::json!("normal")), None, Some("Model")),
                Self::param("fundingRate", "Funding Rate", "number", true, Some(serde_json::json!(0.03)), Some("OIS flat rate"), Some("Curves")),
                Self::param("couponRate", "Coupon Rate", "number", true, Some(serde_json::json!(0.035)), Some("Libor/EURIBOR flat rate"), Some("Curves")),
                Self::param("normalVolBp", "Normal Vol (bp)", "number", true, Some(serde_json::json!(80.0)), Some("Swaption normal vol in bp"), Some("Vol")),
                Self::param("numExercises", "Exercise Dates", "number", true, Some(serde_json::json!(5)), Some("Number of annual exercise dates"), Some("Schedule")),
                Self::param("swapTenor", "Swap Tenor", "number", true, Some(serde_json::json!(5.0)), Some("Underlying swap tenor (years)"), Some("Schedule")),
                Self::param("paymentFreq", "Payment Freq", "number", true, Some(serde_json::json!(0.5)), Some("Year fraction (0.5 = semi-annual)"), Some("Schedule")),
            ],
        }
    }

    fn gaussian_tree_product_def() -> MfmProductDef {
        MfmProductDef {
            product_type: "gaussianTree".to_string(),
            display_name: "Gaussian Tree".to_string(),
            description: "Build and visualise a Gaussian recombining trinomial tree".to_string(),
            parameters: vec![
                Self::param("meanReversion", "Mean Reversion (a)", "number", true, Some(serde_json::json!(0.03)), None, Some("Tree")),
                Self::param("volatility", "Gaussian Vol (σ)", "number", true, Some(serde_json::json!(0.01)), None, Some("Tree")),
                Self::param("numGridPoints", "Grid Points", "number", false, Some(serde_json::json!(21)), None, Some("Tree")),
                Self::param("numStdDevs", "Std Devs", "number", false, Some(serde_json::json!(4.0)), None, Some("Tree")),
                Self::param("numSteps", "Time Steps", "number", true, Some(serde_json::json!(5)), Some("Number of time steps"), Some("Tree")),
                Self::param("maturity", "Maturity (years)", "number", true, Some(serde_json::json!(5.0)), None, Some("Tree")),
            ],
        }
    }

    fn bermudan_product_def() -> MfmProductDef {
        MfmProductDef {
            product_type: "bermudanSwaption".to_string(),
            display_name: "Bermudan Swaption".to_string(),
            description: "Price Bermudan swaption via MFM + backward induction".to_string(),
            parameters: vec![
                Self::param("meanReversion", "Mean Reversion", "number", true, Some(serde_json::json!(0.03)), None, Some("Model")),
                Self::param("volatility", "Gaussian Vol", "number", true, Some(serde_json::json!(0.01)), None, Some("Model")),
                Self::param("numGridPoints", "Grid Points", "number", false, Some(serde_json::json!(41)), None, Some("Model")),
                Self::param("fundingRate", "Funding Rate", "number", true, Some(serde_json::json!(0.03)), None, Some("Curves")),
                Self::param("couponRate", "Coupon Rate", "number", true, Some(serde_json::json!(0.035)), None, Some("Curves")),
                Self::param("normalVolBp", "Normal Vol (bp)", "number", true, Some(serde_json::json!(80.0)), None, Some("Vol")),
                Self::param("numExercises", "Exercise Dates", "number", true, Some(serde_json::json!(5)), None, Some("Schedule")),
                Self::param("swapTenor", "Swap Tenor", "number", true, Some(serde_json::json!(5.0)), None, Some("Schedule")),
                Self::param("isCallable", "Callable", "boolean", false, Some(serde_json::json!(true)), None, Some("Exercise")),
                Self::param("flatCoupon", "Flat Coupon", "number", false, Some(serde_json::json!(0.01)), Some("Flat coupon per period"), Some("Coupon")),
            ],
        }
    }

    fn tarn_product_def() -> MfmProductDef {
        MfmProductDef {
            product_type: "tarn".to_string(),
            display_name: "TARN".to_string(),
            description: "Target Accrual Redemption Note with 2D state space".to_string(),
            parameters: vec![
                Self::param("meanReversion", "Mean Reversion", "number", true, Some(serde_json::json!(0.03)), None, Some("Model")),
                Self::param("volatility", "Gaussian Vol", "number", true, Some(serde_json::json!(0.01)), None, Some("Model")),
                Self::param("numGridPoints", "Grid Points", "number", false, Some(serde_json::json!(41)), None, Some("Model")),
                Self::param("fundingRate", "Funding Rate", "number", true, Some(serde_json::json!(0.03)), None, Some("Curves")),
                Self::param("couponRate", "Coupon Rate", "number", true, Some(serde_json::json!(0.035)), None, Some("Curves")),
                Self::param("normalVolBp", "Normal Vol (bp)", "number", true, Some(serde_json::json!(80.0)), None, Some("Vol")),
                Self::param("numExercises", "Exercise Dates", "number", true, Some(serde_json::json!(10)), None, Some("Schedule")),
                Self::param("swapTenor", "Swap Tenor", "number", true, Some(serde_json::json!(5.0)), None, Some("Schedule")),
                Self::param("tarnAmount", "TARN Amount", "number", true, Some(serde_json::json!(0.10)), Some("Target cumulative coupon"), Some("TARN")),
                Self::param("numCouponGridPoints", "Coupon Grid", "number", false, Some(serde_json::json!(10)), None, Some("TARN")),
                Self::param("excessCouponFlag", "Excess Coupon", "boolean", false, Some(serde_json::json!(false)), None, Some("TARN")),
                Self::param("hasBermudanExercise", "Bermudan Exercise", "boolean", false, Some(serde_json::json!(false)), None, Some("TARN")),
                Self::param("flatCoupon", "Flat Coupon", "number", false, Some(serde_json::json!(0.02)), None, Some("Coupon")),
            ],
        }
    }

    fn cif_product_def() -> MfmProductDef {
        MfmProductDef {
            product_type: "cifEvaluation".to_string(),
            display_name: "CIF Evaluation".to_string(),
            description: "Callable Inverse Floater coupon decomposition (dE + dR + dI + dQ)"
                .to_string(),
            parameters: vec![
                Self::param("fixedRate", "Fixed Rate", "number", true, Some(serde_json::json!(0.07)), None, Some("CIF")),
                Self::param("leverage", "Leverage", "number", true, Some(serde_json::json!(1.0)), None, Some("CIF")),
                Self::param("floorRate", "Floor Rate", "number", true, Some(serde_json::json!(0.0)), None, Some("CIF")),
                Self::param("capRate", "Cap Rate", "number", false, None, None, Some("CIF")),
                Self::param("notional", "Notional", "number", true, Some(serde_json::json!(1000000.0)), None, Some("CIF")),
            ],
        }
    }

    fn param(
        name: &str,
        display_name: &str,
        field_type: &str,
        required: bool,
        default_value: Option<serde_json::Value>,
        description: Option<&str>,
        group: Option<&str>,
    ) -> MfmParameterDef {
        MfmParameterDef {
            name: name.to_string(),
            display_name: display_name.to_string(),
            field_type: field_type.to_string(),
            required,
            default_value,
            description: description.map(|s| s.to_string()),
            group: group.map(|s| s.to_string()),
        }
    }
}
