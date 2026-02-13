//! Service for exotic product pricing.

use std::time::Instant;

use pricer_core::kernel::BarrierType;
use pricer_models::compiler::{
    DownsideProtection, ExoticCompiler, IndexMapper, ObservationAction, ObservationSchedule,
    ScriptProduct, ScriptProductType, TargetConfig,
};
use pricer_pricing::kernel::{FlatSpotProvider, ScriptEngine};

use crate::rest::dto::exotic::{
    AutocallableRequest, ExoticPricingResponse, ExoticProductDef, ExoticProductRequest,
    ParameterDef, TarfRequest,
};

/// Service for exotic product pricing via ScriptKernel.
pub struct ExoticService;

impl ExoticService {
    /// Prices an exotic product request using the ScriptEngine.
    pub fn price_exotic(request: &ExoticProductRequest) -> Result<ExoticPricingResponse, String> {
        let start = Instant::now();

        match request {
            ExoticProductRequest::Tarf(req) => Self::price_tarf(req, start),
            ExoticProductRequest::Autocallable(req) => Self::price_autocallable(req, start),
        }
    }

    /// Prices a TARF product.
    fn price_tarf(req: &TarfRequest, start: Instant) -> Result<ExoticPricingResponse, String> {
        let fixing_dates = req.effective_fixing_dates();
        let observations: Vec<ObservationSchedule> = fixing_dates
            .iter()
            .map(|&t| ObservationSchedule {
                time: t,
                action: ObservationAction::TarfAccrual {
                    strike: req.strike,
                    notional_per_fixing: req.notional_per_fixing,
                    leverage_ratio: req.leverage,
                },
            })
            .collect();

        let product = ScriptProduct {
            product_type: ScriptProductType::Tarf,
            trade_id: "TARF-API".to_string(),
            underlying_index: 1,
            currency_id: 0,
            discount_curve_id: 0,
            notional: req.notional_per_fixing * fixing_dates.len() as f64,
            observations,
            target: Some(TargetConfig {
                target_level: req.target_profit,
                cap_final_settlement: true,
            }),
            downside: None,
            memory_coupon: None,
        };

        let mut compiler = ExoticCompiler::new(IndexMapper::new());
        let kernel = compiler
            .compile_script_product(&product)
            .map_err(|e| format!("{e}"))?;

        let provider = FlatSpotProvider::new(req.domestic_rate, req.foreign_rate, req.spot);
        let pv = ScriptEngine::price(&kernel, &provider);

        Ok(ExoticPricingResponse {
            price: pv,
            currency: req.currency_pair.clone(),
            product_type: "tarf".to_string(),
            mc_stats: None,
            calculation_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Prices an Autocallable product.
    fn price_autocallable(
        req: &AutocallableRequest,
        start: Instant,
    ) -> Result<ExoticPricingResponse, String> {
        let obs_dates = req.effective_observation_dates();
        let mut observations: Vec<ObservationSchedule> = obs_dates
            .iter()
            .map(|&t| ObservationSchedule {
                time: t,
                action: ObservationAction::AutocallCheck {
                    barrier_level: req.autocall_barrier,
                    coupon_amount: req.notional * req.coupon_rate,
                    principal_return: req.notional,
                },
            })
            .collect();

        // Final payoff observation at maturity.
        observations.push(ObservationSchedule {
            time: req.maturity,
            action: ObservationAction::FinalPayoff {
                strike: req.ki_barrier,
                is_call: false,
                notional: req.notional,
            },
        });

        let product = ScriptProduct {
            product_type: ScriptProductType::Autocallable,
            trade_id: "AUTOCALL-API".to_string(),
            underlying_index: 1,
            currency_id: 0,
            discount_curve_id: 0,
            notional: req.notional,
            observations,
            target: None,
            downside: Some(DownsideProtection {
                barrier_level: req.ki_barrier,
                barrier_type: BarrierType::DownIn,
                put_strike: req.ki_barrier,
            }),
            memory_coupon: None,
        };

        let mut compiler = ExoticCompiler::new(IndexMapper::new());
        let kernel = compiler
            .compile_script_product(&product)
            .map_err(|e| format!("{e}"))?;

        let provider = FlatSpotProvider::new(req.rate, 0.0, req.spot);
        let pv = ScriptEngine::price(&kernel, &provider);

        Ok(ExoticPricingResponse {
            price: pv,
            currency: req.underlying.clone(),
            product_type: "autocallable".to_string(),
            mc_stats: None,
            calculation_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Returns the list of available exotic product definitions for UI rendering.
    pub fn get_exotic_products() -> Vec<ExoticProductDef> {
        vec![
            ExoticProductDef {
                product_type: "tarf".to_string(),
                display_name: "TARF".to_string(),
                description: "Target Accrual Redemption Forward".to_string(),
                parameters: vec![
                    ParameterDef {
                        name: "currencyPair".to_string(),
                        display_name: "Currency Pair".to_string(),
                        field_type: "string".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!("EURUSD")),
                        description: Some("e.g. EURUSD".to_string()),
                    },
                    ParameterDef {
                        name: "notionalPerFixing".to_string(),
                        display_name: "Notional per Fixing".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(100000)),
                        description: None,
                    },
                    ParameterDef {
                        name: "strike".to_string(),
                        display_name: "Strike".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(1.10)),
                        description: None,
                    },
                    ParameterDef {
                        name: "targetProfit".to_string(),
                        display_name: "Target Profit".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(50000)),
                        description: None,
                    },
                    ParameterDef {
                        name: "leverage".to_string(),
                        display_name: "Leverage".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(2.0)),
                        description: None,
                    },
                    ParameterDef {
                        name: "maturity".to_string(),
                        display_name: "Maturity (years)".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(1.0)),
                        description: None,
                    },
                    ParameterDef {
                        name: "numFixings".to_string(),
                        display_name: "Number of Fixings".to_string(),
                        field_type: "number".to_string(),
                        required: false,
                        default_value: Some(serde_json::json!(12)),
                        description: Some("Monthly fixings by default".to_string()),
                    },
                    ParameterDef {
                        name: "spot".to_string(),
                        display_name: "Spot".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(1.08)),
                        description: None,
                    },
                    ParameterDef {
                        name: "domesticRate".to_string(),
                        display_name: "Domestic Rate".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(0.05)),
                        description: None,
                    },
                    ParameterDef {
                        name: "foreignRate".to_string(),
                        display_name: "Foreign Rate".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(0.03)),
                        description: None,
                    },
                    ParameterDef {
                        name: "volatility".to_string(),
                        display_name: "Volatility".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(0.10)),
                        description: None,
                    },
                ],
            },
            ExoticProductDef {
                product_type: "autocallable".to_string(),
                display_name: "Autocallable".to_string(),
                description: "Autocallable Structured Note".to_string(),
                parameters: vec![
                    ParameterDef {
                        name: "underlying".to_string(),
                        display_name: "Underlying".to_string(),
                        field_type: "string".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!("SPX")),
                        description: None,
                    },
                    ParameterDef {
                        name: "notional".to_string(),
                        display_name: "Notional".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(1000000)),
                        description: None,
                    },
                    ParameterDef {
                        name: "spot".to_string(),
                        display_name: "Spot".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(100.0)),
                        description: None,
                    },
                    ParameterDef {
                        name: "autocallBarrier".to_string(),
                        display_name: "Autocall Barrier".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(105.0)),
                        description: Some("Absolute level".to_string()),
                    },
                    ParameterDef {
                        name: "couponRate".to_string(),
                        display_name: "Coupon Rate".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(0.08)),
                        description: None,
                    },
                    ParameterDef {
                        name: "kiBarrier".to_string(),
                        display_name: "KI Barrier".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(70.0)),
                        description: Some("Down-and-in put barrier".to_string()),
                    },
                    ParameterDef {
                        name: "maturity".to_string(),
                        display_name: "Maturity (years)".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(3.0)),
                        description: None,
                    },
                    ParameterDef {
                        name: "numObservations".to_string(),
                        display_name: "Observations".to_string(),
                        field_type: "number".to_string(),
                        required: false,
                        default_value: Some(serde_json::json!(4)),
                        description: Some("Quarterly by default".to_string()),
                    },
                    ParameterDef {
                        name: "rate".to_string(),
                        display_name: "Risk-Free Rate".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(0.05)),
                        description: None,
                    },
                    ParameterDef {
                        name: "volatility".to_string(),
                        display_name: "Volatility".to_string(),
                        field_type: "number".to_string(),
                        required: true,
                        default_value: Some(serde_json::json!(0.20)),
                        description: None,
                    },
                ],
            },
        ]
    }
}
