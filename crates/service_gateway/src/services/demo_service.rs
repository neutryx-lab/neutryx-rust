//! Demo service wrapping data loading and demo-specific operations
//!
//! Provides endpoints for the demo_gui frontend.

use std::{collections::HashMap, path::Path, sync::Arc, time::Instant};

use crate::{
    error::ServerError,
    rest::dto::demo::{
        AppConfigResponse, AvailableCurvesResponse, CalibrationMetadata, CalibrationParameters,
        Cashflow, Convention, ConventionField, ConventionsResponse, CurveIndicesResponse,
        CurveInstrument, CurveInstrumentsResponse, DemoGreeksRequest, DemoGreeksResult,
        DemoPricingRequest, DemoPricingResult, EnumValue, EventType, EventTypesResponse,
        EventsResponse, ExpandedTrade, ExportFormat, FieldType, FxVolCalibrateRequest, FxVolPair,
        FxVolPairsResponse, FxVolQuote, FxVolQuotesResponse, Importance, InstrumentDef,
        InstrumentsResponse, IrVolCurrenciesResponse, IrVolCurrency, IrVolQuote,
        IrVolQuotesResponse, MarketConfigResponse, MarketEvent, MarketRate,
        MarketRateDetailResponse, MarketRatesResponse, ParameterDef, SmilePoint,
        SwaptionInstrument, TradeExpandRequest, TradeLeg, TradeMetadata, VolcubeCalibrateRequest,
        VolcubeCalibrateResponse, VolcubeIndicesResponse, VolcubeInstrumentsResponse,
        VolcubeModelsResponse,
    },
    state::AppState,
};

/// Demo service providing API endpoints for demo_gui
pub struct DemoService;

impl DemoService {
    // =========================================================================
    // Configuration API
    // =========================================================================

    /// Get application configuration
    pub fn get_config(_state: &Arc<AppState>) -> Result<AppConfigResponse, ServerError> {
        // Load from indices.json
        let indices_path = Path::new("demo/data/input/indices.json");
        let indices_content = std::fs::read_to_string(indices_path)
            .map_err(|e| ServerError::Internal(format!("Failed to read indices.json: {e}")))?;
        let indices: serde_json::Value = serde_json::from_str(&indices_content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse indices.json: {e}")))?;

        // Build enums
        let mut enums: HashMap<String, Vec<EnumValue>> = HashMap::new();

        // Add currencies from indices
        if let Some(currencies) = indices.get("currencies").and_then(|c| c.as_array()) {
            let currency_values: Vec<EnumValue> = currencies
                .iter()
                .filter_map(|c| {
                    let code = c.get("code")?.as_str()?.to_string();
                    let name = c.get("name").and_then(|n| n.as_str()).map(String::from);
                    Some(EnumValue::Object { code, name })
                })
                .collect();
            enums.insert("currencies".to_string(), currency_values);
        }

        // Add rate indices
        if let Some(rates_indices) = indices
            .get("indices")
            .and_then(|i| i.get("rates"))
            .and_then(|r| r.get("items"))
            .and_then(|i| i.as_array())
        {
            let index_values: Vec<EnumValue> = rates_indices
                .iter()
                .filter_map(|i| Some(EnumValue::Simple(i.get("index")?.as_str()?.to_string())))
                .collect();
            enums.insert("rateIndices".to_string(), index_values);
        }

        // Build rate_index_by_currency
        let mut rate_index_by_currency: HashMap<String, String> = HashMap::new();
        if let Some(currencies) = indices.get("currencies").and_then(|c| c.as_array()) {
            for currency in currencies {
                if let (Some(code), Some(index)) = (
                    currency.get("code").and_then(|c| c.as_str()),
                    currency.get("index").and_then(|i| i.as_str()),
                ) {
                    rate_index_by_currency.insert(code.to_string(), index.to_string());
                }
            }
        }

        // Build defaults
        let defaults: HashMap<String, serde_json::Value> = [
            ("currency".to_string(), serde_json::json!("USD")),
            ("notional".to_string(), serde_json::json!(1_000_000)),
            ("paymentFrequency".to_string(), serde_json::json!("3M")),
        ]
        .into_iter()
        .collect();

        Ok(AppConfigResponse {
            enums,
            defaults,
            rate_index_by_currency,
        })
    }

    // =========================================================================
    // Instruments API
    // =========================================================================

    /// Get available instruments
    pub fn get_instruments(_state: &Arc<AppState>) -> Result<InstrumentsResponse, ServerError> {
        // Return hardcoded instrument definitions
        let instruments = vec![
            InstrumentDef {
                instrument_type: "IRS".to_string(),
                id: Some("irs".to_string()),
                display_name: Some("Interest Rate Swap".to_string()),
                asset_class_name: Some("Rates".to_string()),
                required_params: vec![
                    ParameterDef {
                        name: "notional".to_string(),
                        label: Some("Notional".to_string()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(1_000_000)),
                        options: None,
                        validation: Some(crate::rest::dto::demo::ParameterValidation {
                            min: Some(0.0),
                            max: None,
                        }),
                    },
                    ParameterDef {
                        name: "currency".to_string(),
                        label: Some("Currency".to_string()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("USD")),
                        options: Some(vec![
                            crate::rest::dto::demo::ParameterOption {
                                value: "USD".to_string(),
                                label: "USD".to_string(),
                            },
                            crate::rest::dto::demo::ParameterOption {
                                value: "EUR".to_string(),
                                label: "EUR".to_string(),
                            },
                            crate::rest::dto::demo::ParameterOption {
                                value: "JPY".to_string(),
                                label: "JPY".to_string(),
                            },
                        ]),
                        validation: None,
                    },
                    ParameterDef {
                        name: "startDate".to_string(),
                        label: Some("Start Date".to_string()),
                        field_type: FieldType::Date,
                        default_value: None,
                        options: None,
                        validation: None,
                    },
                    ParameterDef {
                        name: "endDate".to_string(),
                        label: Some("End Date".to_string()),
                        field_type: FieldType::Date,
                        default_value: None,
                        options: None,
                        validation: None,
                    },
                ],
                optional_params: vec![ParameterDef {
                    name: "fixedRate".to_string(),
                    label: Some("Fixed Rate".to_string()),
                    field_type: FieldType::Number,
                    default_value: Some(serde_json::json!(0.05)),
                    options: None,
                    validation: Some(crate::rest::dto::demo::ParameterValidation {
                        min: Some(-0.1),
                        max: Some(0.5),
                    }),
                }],
            },
            InstrumentDef {
                instrument_type: "FxForward".to_string(),
                id: Some("fx-forward".to_string()),
                display_name: Some("FX Forward".to_string()),
                asset_class_name: Some("FX".to_string()),
                required_params: vec![
                    ParameterDef {
                        name: "notional".to_string(),
                        label: Some("Notional".to_string()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(1_000_000)),
                        options: None,
                        validation: None,
                    },
                    ParameterDef {
                        name: "currencyPair".to_string(),
                        label: Some("Currency Pair".to_string()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("EURUSD")),
                        options: Some(vec![
                            crate::rest::dto::demo::ParameterOption {
                                value: "EURUSD".to_string(),
                                label: "EUR/USD".to_string(),
                            },
                            crate::rest::dto::demo::ParameterOption {
                                value: "USDJPY".to_string(),
                                label: "USD/JPY".to_string(),
                            },
                        ]),
                        validation: None,
                    },
                ],
                optional_params: vec![],
            },
            InstrumentDef {
                instrument_type: "FxVanillaOption".to_string(),
                id: Some("fx-vanilla-option".to_string()),
                display_name: Some("FX Vanilla Option".to_string()),
                asset_class_name: Some("FX".to_string()),
                required_params: vec![
                    ParameterDef {
                        name: "notional".to_string(),
                        label: Some("Notional".to_string()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(1_000_000)),
                        options: None,
                        validation: None,
                    },
                    ParameterDef {
                        name: "strike".to_string(),
                        label: Some("Strike".to_string()),
                        field_type: FieldType::Number,
                        default_value: None,
                        options: None,
                        validation: None,
                    },
                ],
                optional_params: vec![],
            },
        ];

        Ok(InstrumentsResponse { instruments })
    }

    // =========================================================================
    // Trade Expansion API
    // =========================================================================

    /// Expand a trade request into cashflows
    pub fn expand_trade(
        request: &TradeExpandRequest,
        _state: &Arc<AppState>,
    ) -> Result<ExpandedTrade, ServerError> {
        let start = Instant::now();

        // Generate mock expanded trade based on instrument type
        let (legs, trade_type) = match request.instrument_type.as_str() {
            "IRS" => {
                let fixed_leg = TradeLeg {
                    direction: "Payer".to_string(),
                    currency: request
                        .params
                        .get("currency")
                        .and_then(|c| c.as_str())
                        .unwrap_or("USD")
                        .to_string(),
                    leg_type: "Fixed".to_string(),
                    rate_index: None,
                    cashflows: vec![Cashflow {
                        payment_date: "2027-01-30".to_string(),
                        accrual_start: "2026-01-30".to_string(),
                        accrual_end: "2027-01-30".to_string(),
                        year_fraction: 1.0,
                        notional: request
                            .params
                            .get("notional")
                            .and_then(|n| n.as_f64())
                            .unwrap_or(1_000_000.0),
                        rate: Some(0.05),
                        payoff_type: "Fixed".to_string(),
                        rate_index: None,
                    }],
                };
                let float_leg = TradeLeg {
                    direction: "Receiver".to_string(),
                    currency: request
                        .params
                        .get("currency")
                        .and_then(|c| c.as_str())
                        .unwrap_or("USD")
                        .to_string(),
                    leg_type: "Float".to_string(),
                    rate_index: Some("SOFR".to_string()),
                    cashflows: vec![Cashflow {
                        payment_date: "2027-01-30".to_string(),
                        accrual_start: "2026-01-30".to_string(),
                        accrual_end: "2027-01-30".to_string(),
                        year_fraction: 1.0,
                        notional: request
                            .params
                            .get("notional")
                            .and_then(|n| n.as_f64())
                            .unwrap_or(1_000_000.0),
                        rate: None,
                        payoff_type: "Linear".to_string(),
                        rate_index: Some("SOFR".to_string()),
                    }],
                };
                (vec![fixed_leg, float_leg], "IRS")
            }
            "FxForward" => {
                let leg = TradeLeg {
                    direction: "Payer".to_string(),
                    currency: "EUR".to_string(),
                    leg_type: "FxForward".to_string(),
                    rate_index: None,
                    cashflows: vec![Cashflow {
                        payment_date: "2026-07-30".to_string(),
                        accrual_start: "2026-01-30".to_string(),
                        accrual_end: "2026-07-30".to_string(),
                        year_fraction: 0.5,
                        notional: request
                            .params
                            .get("notional")
                            .and_then(|n| n.as_f64())
                            .unwrap_or(1_000_000.0),
                        rate: None,
                        payoff_type: "Forward".to_string(),
                        rate_index: None,
                    }],
                };
                (vec![leg], "FxForward")
            }
            _ => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unknown instrument type: {}",
                    request.instrument_type
                )))
            }
        };

        let total_cashflows = legs.iter().map(|l| l.cashflows.len()).sum();
        let elapsed = start.elapsed();

        Ok(ExpandedTrade {
            trade_id: uuid::Uuid::new_v4().to_string(),
            trade_type: trade_type.to_string(),
            legs,
            metadata: TradeMetadata {
                total_legs: 2,
                total_cashflows,
                processing_time_ms: elapsed.as_secs_f64() * 1000.0,
            },
        })
    }

    // =========================================================================
    // Pricing API
    // =========================================================================

    /// Price a trade
    pub fn price_trade(
        request: &DemoPricingRequest,
        _state: &Arc<AppState>,
    ) -> Result<DemoPricingResult, ServerError> {
        // Simple mock pricing
        let total_pv: f64 = request
            .legs
            .iter()
            .map(|leg| {
                let sign = if leg.direction == "payer" { -1.0 } else { 1.0 };
                sign * leg
                    .cashflows
                    .iter()
                    .map(|cf| cf.amount * 0.98) // Simple discount
                    .sum::<f64>()
            })
            .sum();

        Ok(DemoPricingResult {
            total_pv: Some(total_pv),
            pv: Some(total_pv),
            currency: request.reporting_currency.clone(),
            legs: None,
        })
    }

    /// Calculate Greeks
    pub fn calculate_greeks(
        request: &DemoGreeksRequest,
        _state: &Arc<AppState>,
    ) -> Result<DemoGreeksResult, ServerError> {
        // Simple mock Greeks calculation
        let base_pv: f64 = request
            .legs
            .iter()
            .map(|leg| leg.cashflows.iter().map(|cf| cf.amount * 0.98).sum::<f64>())
            .sum();

        let delta = base_pv * (request.bump_sizes.rate_bump_bp / 10000.0);

        Ok(DemoGreeksResult {
            currency: request.reporting_currency.clone(),
            delta,
            gamma: Some(delta * 0.1),
            theta: Some(-base_pv * 0.001),
            vega: Some(base_pv * (request.bump_sizes.vol_bump_pct / 100.0)),
        })
    }

    // =========================================================================
    // Market Data API
    // =========================================================================

    /// Get market rates
    pub fn get_market_rates(_state: &Arc<AppState>) -> Result<MarketRatesResponse, ServerError> {
        let rates_path = Path::new("demo/data/input/rates/market_quotes.json");
        let content = std::fs::read_to_string(rates_path).map_err(|e| {
            ServerError::Internal(format!("Failed to read market_quotes.json: {e}"))
        })?;
        let data: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            ServerError::Internal(format!("Failed to parse market_quotes.json: {e}"))
        })?;

        let mut rates = Vec::new();
        let timestamp = chrono::Utc::now().to_rfc3339();

        // Load IR rates
        if let Some(rates_by_currency) = data.get("rates").and_then(|r| r.as_object()) {
            for (currency, rate_types) in rates_by_currency {
                if let Some(rate_types_obj) = rate_types.as_object() {
                    for (rate_type, quotes) in rate_types_obj {
                        if let Some(quotes_arr) = quotes.as_array() {
                            for quote in quotes_arr {
                                let tenor =
                                    quote.get("tenor").and_then(|t| t.as_str()).unwrap_or("");
                                let value =
                                    quote.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let index = quote.get("index").and_then(|i| i.as_str());

                                rates.push(MarketRate {
                                    id: format!("{}-{}-{}", currency, rate_type, tenor),
                                    currency: currency.clone(),
                                    tenor: tenor.to_string(),
                                    rate_type: rate_type.clone(),
                                    value,
                                    rate_index: index.map(String::from),
                                    quote_type: Some("Mid".to_string()),
                                    source: "Internal".to_string(),
                                    timestamp: timestamp.clone(),
                                    is_stale: false,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Load FX spot rates
        let fx_path = Path::new("demo/data/input/fx/fx_spots.json");
        if let Ok(fx_content) = std::fs::read_to_string(fx_path) {
            if let Ok(fx_data) = serde_json::from_str::<serde_json::Value>(&fx_content) {
                if let Some(spots) = fx_data.get("spots").and_then(|s| s.as_array()) {
                    for spot in spots {
                        let pair = spot.get("pair").and_then(|p| p.as_str()).unwrap_or("");
                        let value = spot.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let base_ccy = if pair.len() >= 3 { &pair[..3] } else { pair };

                        rates.push(MarketRate {
                            id: format!("FX-{}", pair),
                            currency: base_ccy.to_string(),
                            tenor: "SPOT".to_string(),
                            rate_type: "fx_spot".to_string(),
                            value,
                            rate_index: None,
                            quote_type: Some("Mid".to_string()),
                            source: "Internal".to_string(),
                            timestamp: timestamp.clone(),
                            is_stale: false,
                        });
                    }
                }
            }
        }

        Ok(MarketRatesResponse {
            rates,
            last_updated: timestamp,
        })
    }

    /// Get market config
    pub fn get_market_config(_state: &Arc<AppState>) -> Result<MarketConfigResponse, ServerError> {
        Ok(MarketConfigResponse {
            tenor_order: vec![
                "ON".to_string(),
                "1W".to_string(),
                "1M".to_string(),
                "3M".to_string(),
                "6M".to_string(),
                "1Y".to_string(),
                "2Y".to_string(),
                "3Y".to_string(),
                "5Y".to_string(),
                "7Y".to_string(),
                "10Y".to_string(),
                "15Y".to_string(),
                "20Y".to_string(),
                "30Y".to_string(),
            ],
        })
    }

    /// Get rate detail
    pub fn get_rate_detail(
        rate_id: &str,
        state: &Arc<AppState>,
    ) -> Result<MarketRateDetailResponse, ServerError> {
        let rates_response = Self::get_market_rates(state)?;
        let rate = rates_response
            .rates
            .into_iter()
            .find(|r| r.id == rate_id)
            .ok_or_else(|| ServerError::NotFound(format!("Rate {} not found", rate_id)))?;

        // Build instrument description based on rate type
        let instrument = Self::build_instrument_description(&rate);

        // Find matching convention
        let convention = Self::find_convention_for_rate(&rate, state);

        Ok(MarketRateDetailResponse {
            rate,
            instrument,
            convention,
        })
    }

    /// Build instrument description for a rate
    fn build_instrument_description(rate: &MarketRate) -> Option<serde_json::Value> {
        let description = match rate.rate_type.as_str() {
            "deposit" => serde_json::json!({
                "type": "Money Market Deposit",
                "description": format!(
                    "{} {} deposit rate. A money market instrument representing the cost of \
                     borrowing or lending {} for the {} period.",
                    rate.currency, rate.tenor, rate.currency, rate.tenor
                ),
                "usage": "Used for short-end curve construction and discounting",
                "quoteConvention": "Simple rate, typically ACT/360 (USD/EUR) or ACT/365F (GBP/JPY)",
                "settlementDays": if rate.currency == "GBP" { 0 } else { 2 }
            }),
            "ois" => serde_json::json!({
                "type": "Overnight Index Swap",
                "description": format!(
                    "{} {} OIS rate. An interest rate swap where the floating leg pays the \
                     compounded overnight rate ({}) versus a fixed rate.",
                    rate.currency, rate.tenor,
                    rate.rate_index.as_deref().unwrap_or("overnight index")
                ),
                "usage": "Primary instrument for building risk-free discount curves",
                "quoteConvention": "Par swap rate quoted as annual rate",
                "index": rate.rate_index.clone()
            }),
            "swap" => serde_json::json!({
                "type": "Interest Rate Swap",
                "description": format!(
                    "{} {} IRS rate. A vanilla interest rate swap exchanging fixed rate payments \
                     for floating rate payments indexed to {}.",
                    rate.currency, rate.tenor,
                    rate.rate_index.as_deref().unwrap_or("floating index")
                ),
                "usage": "Key instrument for yield curve construction at longer tenors",
                "quoteConvention": "Par swap rate, fixed leg vs floating leg",
                "index": rate.rate_index.clone()
            }),
            "fra" => serde_json::json!({
                "type": "Forward Rate Agreement",
                "description": format!(
                    "{} {} FRA. A forward contract to exchange a fixed rate for a floating rate \
                     on a notional amount for the specified period.",
                    rate.currency, rate.tenor
                ),
                "usage": "Used for constructing the forward curve between deposit and swap tenors",
                "quoteConvention": "FRA rate quoted as simple forward rate",
                "index": rate.rate_index.clone()
            }),
            "fx_spot" => {
                let pair = rate.id.strip_prefix("FX-").unwrap_or(&rate.id);
                let base = if pair.len() >= 3 { &pair[..3] } else { "" };
                let quote = if pair.len() >= 6 { &pair[3..6] } else { "" };
                serde_json::json!({
                    "type": "FX Spot Rate",
                    "description": format!(
                        "{}/{} spot exchange rate. The current market rate to exchange {} for {}. \
                         Quote convention: 1 {} = {} {}.",
                        base, quote, base, quote, base, rate.value, quote
                    ),
                    "usage": "Used for FX conversions, cross-currency discounting, and FX derivative pricing",
                    "quoteConvention": format!("{}/{} (1 {} = x {})", base, quote, base, quote),
                    "settlementDays": 2
                })
            }
            _ => return None,
        };
        Some(description)
    }

    /// Find matching convention for a rate
    fn find_convention_for_rate(rate: &MarketRate, state: &Arc<AppState>) -> Option<Convention> {
        let conventions_result = Self::get_conventions(state).ok()?;

        // Try to find exact match first
        let convention_id = match rate.rate_type.as_str() {
            "deposit" => format!("{}-DEPO", rate.currency),
            "ois" => format!(
                "{}-{}-OIS",
                rate.currency,
                rate.rate_index.as_deref().unwrap_or("OIS")
            ),
            "swap" => {
                let index = rate.rate_index.as_deref().unwrap_or("SWAP");
                format!("{}-{}-SWAP", rate.currency, index)
            }
            "fx_spot" => "FX-SPOT".to_string(),
            _ => return None,
        };

        // Try exact match
        if let Some(conv) = conventions_result
            .conventions
            .iter()
            .find(|c| c.id == convention_id)
        {
            return Some(conv.clone());
        }

        // Try currency match with default
        conventions_result
            .conventions
            .into_iter()
            .find(|c| {
                c.currency == rate.currency
                    && c.convention_type.to_lowercase().contains(&rate.rate_type)
                    && c.is_default == Some(true)
            })
    }

    /// Refresh market rates (mock - just returns success)
    pub fn refresh_market_rates(_state: &Arc<AppState>) -> Result<(), ServerError> {
        // In a real implementation, this would refresh from data sources
        Ok(())
    }

    // =========================================================================
    // Conventions API
    // =========================================================================

    /// Get conventions
    pub fn get_conventions(_state: &Arc<AppState>) -> Result<ConventionsResponse, ServerError> {
        let conv_path = Path::new("demo/data/input/conventions/conventions.json");
        let content = std::fs::read_to_string(conv_path)
            .map_err(|e| ServerError::Internal(format!("Failed to read conventions.json: {e}")))?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse conventions.json: {e}")))?;

        let mut conventions = Vec::new();

        if let Some(conv_obj) = data.get("conventions").and_then(|c| c.as_object()) {
            for (id, conv) in conv_obj {
                let convention_type = conv
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let currency = conv
                    .get("currency")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let is_default = conv.get("is_default").and_then(|d| d.as_bool());

                // Convert fields object to ConventionField vec
                let fields = conv.get("fields").and_then(|f| f.as_object()).map(|obj| {
                    obj.iter()
                        .map(|(key, value)| ConventionField {
                            label: key
                                .replace('_', " ")
                                .split_whitespace()
                                .map(|w| {
                                    let mut c = w.chars();
                                    match c.next() {
                                        None => String::new(),
                                        Some(f) => {
                                            f.to_uppercase().collect::<String>() + c.as_str()
                                        }
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(" "),
                            value: value.as_str().map(String::from).unwrap_or_else(|| {
                                if let Some(n) = value.as_i64() {
                                    n.to_string()
                                } else if let Some(n) = value.as_f64() {
                                    n.to_string()
                                } else {
                                    value.to_string()
                                }
                            }),
                        })
                        .collect()
                });

                conventions.push(Convention {
                    id: id.clone(),
                    convention_type,
                    currency,
                    is_default,
                    fields,
                });
            }
        }

        // Sort conventions by id for consistency
        conventions.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(ConventionsResponse { conventions })
    }

    /// Get convention detail
    pub fn get_convention_detail(
        id: &str,
        state: &Arc<AppState>,
    ) -> Result<Convention, ServerError> {
        let conventions_response = Self::get_conventions(state)?;
        conventions_response
            .conventions
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| ServerError::NotFound(format!("Convention {} not found", id)))
    }

    // =========================================================================
    // IR Volatility API
    // =========================================================================

    /// Get IR vol currencies
    pub fn get_ir_vol_currencies(
        _state: &Arc<AppState>,
    ) -> Result<IrVolCurrenciesResponse, ServerError> {
        Ok(IrVolCurrenciesResponse {
            currencies: vec![
                IrVolCurrency {
                    currency: "USD".to_string(),
                },
                IrVolCurrency {
                    currency: "EUR".to_string(),
                },
                IrVolCurrency {
                    currency: "JPY".to_string(),
                },
                IrVolCurrency {
                    currency: "GBP".to_string(),
                },
            ],
        })
    }

    /// Get IR vol quotes for a currency
    pub fn get_ir_vol_quotes(
        currency: &str,
        _state: &Arc<AppState>,
    ) -> Result<IrVolQuotesResponse, ServerError> {
        // Try to load from file
        let file_path = format!("demo/data/input/irvol/{}.json", currency.to_lowercase());
        let path = Path::new(&file_path);

        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| ServerError::Internal(format!("Failed to read IR vol file: {e}")))?;
            let data: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| ServerError::Internal(format!("Failed to parse IR vol file: {e}")))?;

            let mut quotes = Vec::new();
            if let Some(quotes_arr) = data.get("quotes").and_then(|q| q.as_array()) {
                for quote in quotes_arr {
                    quotes.push(IrVolQuote {
                        expiry: quote
                            .get("expiry")
                            .and_then(|e| e.as_str())
                            .unwrap_or("")
                            .to_string(),
                        tenor: quote
                            .get("tenor")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string(),
                        atm_vol: quote.get("atmVol").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        smile: None,
                    });
                }
            }

            return Ok(IrVolQuotesResponse {
                quotes,
                vol_type: Some("normal".to_string()),
                source: Some("Internal".to_string()),
            });
        }

        // Return mock data if file not found
        Ok(IrVolQuotesResponse {
            quotes: vec![
                IrVolQuote {
                    expiry: "1M".to_string(),
                    tenor: "1Y".to_string(),
                    atm_vol: 0.0050,
                    smile: Some(vec![
                        SmilePoint {
                            strike_offset_bp: -50.0,
                            vol: 0.0055,
                        },
                        SmilePoint {
                            strike_offset_bp: 50.0,
                            vol: 0.0045,
                        },
                    ]),
                },
                IrVolQuote {
                    expiry: "1Y".to_string(),
                    tenor: "5Y".to_string(),
                    atm_vol: 0.0065,
                    smile: None,
                },
            ],
            vol_type: Some("normal".to_string()),
            source: Some("Internal".to_string()),
        })
    }

    // =========================================================================
    // FX Volatility API
    // =========================================================================

    /// Get FX vol pairs
    pub fn get_fx_vol_pairs(_state: &Arc<AppState>) -> Result<FxVolPairsResponse, ServerError> {
        // Read from indices.json
        let indices_path = Path::new("demo/data/input/indices.json");
        if indices_path.exists() {
            let content = std::fs::read_to_string(indices_path)
                .map_err(|e| ServerError::Internal(format!("Failed to read indices.json: {e}")))?;
            let indices: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| ServerError::Internal(format!("Failed to parse indices.json: {e}")))?;

            if let Some(fxvol_items) = indices
                .get("indices")
                .and_then(|i| i.get("fxvol"))
                .and_then(|f| f.get("items"))
                .and_then(|i| i.as_array())
            {
                let pairs: Vec<FxVolPair> = fxvol_items
                    .iter()
                    .filter_map(|item| {
                        item.get("currencyPair")
                            .and_then(|p| p.as_str())
                            .map(|pair| FxVolPair {
                                pair: pair.to_string(),
                            })
                    })
                    .collect();

                return Ok(FxVolPairsResponse { pairs });
            }
        }

        // Fallback
        Ok(FxVolPairsResponse {
            pairs: vec![
                FxVolPair {
                    pair: "EURUSD".to_string(),
                },
                FxVolPair {
                    pair: "USDJPY".to_string(),
                },
            ],
        })
    }

    /// Get FX vol quotes for a pair
    pub fn get_fx_vol_quotes(
        pair: &str,
        _state: &Arc<AppState>,
    ) -> Result<FxVolQuotesResponse, ServerError> {
        let file_path = format!("demo/data/input/fxvol/{}.json", pair.to_lowercase());
        let path = Path::new(&file_path);

        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| ServerError::Internal(format!("Failed to read FX vol file: {e}")))?;
            let data: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| ServerError::Internal(format!("Failed to parse FX vol file: {e}")))?;

            let mut quotes = Vec::new();
            if let Some(quotes_arr) = data.get("quotes").and_then(|q| q.as_array()) {
                for quote in quotes_arr {
                    quotes.push(FxVolQuote {
                        expiry: quote.get("expiry").and_then(|e| e.as_f64()).unwrap_or(0.0),
                        atm_vol: quote.get("atmVol").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        rr25d: quote.get("rr25d").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        bf25d: quote.get("bf25d").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        rr10d: quote.get("rr10d").and_then(|v| v.as_f64()),
                        bf10d: quote.get("bf10d").and_then(|v| v.as_f64()),
                    });
                }
            }

            let spot = data.get("spot").and_then(|s| s.as_f64());

            return Ok(FxVolQuotesResponse { quotes, spot });
        }

        Err(ServerError::NotFound(format!(
            "FX vol data not found for pair: {}",
            pair
        )))
    }

    // =========================================================================
    // Events API
    // =========================================================================

    /// Get market events
    pub fn get_events(_state: &Arc<AppState>) -> Result<EventsResponse, ServerError> {
        let mut events = Vec::new();

        // Helper to parse event type from string
        fn parse_event_type(s: &str) -> EventType {
            match s {
                "central_bank_meeting" => EventType::CentralBankMeeting,
                "economic_release" => EventType::EconomicRelease,
                "holiday" => EventType::Holiday,
                "news" => EventType::News,
                "expiry" => EventType::Expiry,
                _ => EventType::Other,
            }
        }

        // Helper to parse importance from string
        fn parse_importance(s: &str) -> Importance {
            match s.to_lowercase().as_str() {
                "critical" => Importance::Critical,
                "high" => Importance::High,
                "medium" => Importance::Medium,
                _ => Importance::Low,
            }
        }

        // Helper to parse event from JSON value
        fn parse_event(event: &serde_json::Value) -> Option<MarketEvent> {
            let id = event.get("id")?.as_str()?.to_string();
            let date = event.get("date")?.as_str()?.to_string();
            let event_type_str = event.get("eventType")?.as_str()?;
            let title = event.get("title")?.as_str()?.to_string();

            let central_bank = event.get("centralBank").and_then(|cb| {
                Some(crate::rest::dto::demo::CentralBank {
                    name: cb.get("name")?.as_str()?.to_string(),
                    code: cb.get("code")?.as_str()?.to_string(),
                    currency: cb.get("currency")?.as_str()?.to_string(),
                })
            });

            let tags = event.get("tags").and_then(|t| t.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

            Some(MarketEvent {
                id,
                date,
                event_type: parse_event_type(event_type_str),
                title,
                description: event
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(String::from),
                currency: event
                    .get("currency")
                    .and_then(|c| c.as_str())
                    .map(String::from),
                region: event
                    .get("region")
                    .and_then(|r| r.as_str())
                    .map(String::from),
                importance: event
                    .get("importance")
                    .and_then(|i| i.as_str())
                    .map(parse_importance)
                    .unwrap_or(Importance::Medium),
                time: event.get("time").and_then(|t| t.as_str()).map(String::from),
                timezone: event
                    .get("timezone")
                    .and_then(|t| t.as_str())
                    .map(String::from),
                source: event
                    .get("source")
                    .and_then(|s| s.as_str())
                    .unwrap_or("Internal")
                    .to_string(),
                tags,
                central_bank,
                previous: event
                    .get("previous")
                    .and_then(|p| p.as_str())
                    .map(String::from),
                forecast: event
                    .get("forecast")
                    .and_then(|f| f.as_str())
                    .map(String::from),
                actual: event
                    .get("actual")
                    .and_then(|a| a.as_str())
                    .map(String::from),
            })
        }

        // Load central bank meetings
        let cb_path = Path::new("demo/data/input/events/central_bank_meetings.json");
        if let Ok(content) = std::fs::read_to_string(cb_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(event_arr) = data.get("events").and_then(|e| e.as_array()) {
                    for event in event_arr {
                        if let Some(parsed) = parse_event(event) {
                            events.push(parsed);
                        }
                    }
                }
            }
        }

        // Load economic releases
        let econ_path = Path::new("demo/data/input/events/economic_releases.json");
        if let Ok(content) = std::fs::read_to_string(econ_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(event_arr) = data.get("events").and_then(|e| e.as_array()) {
                    for event in event_arr {
                        if let Some(parsed) = parse_event(event) {
                            events.push(parsed);
                        }
                    }
                }
            }
        }

        // Load holidays
        let hol_path = Path::new("demo/data/input/events/holidays.json");
        if let Ok(content) = std::fs::read_to_string(hol_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(event_arr) = data.get("events").and_then(|e| e.as_array()) {
                    for event in event_arr {
                        if let Some(parsed) = parse_event(event) {
                            events.push(parsed);
                        }
                    }
                }
            }
        }

        // Sort events by date
        events.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(EventsResponse { events })
    }

    /// Get event types
    pub fn get_event_types(_state: &Arc<AppState>) -> Result<EventTypesResponse, ServerError> {
        Ok(EventTypesResponse {
            types: vec![
                "central_bank_meeting".to_string(),
                "economic_release".to_string(),
                "holiday".to_string(),
                "news".to_string(),
                "expiry".to_string(),
                "other".to_string(),
            ],
        })
    }

    // =========================================================================
    // Curves API (additional endpoints)
    // =========================================================================

    /// Get available curves
    pub fn get_available_curves(
        state: &Arc<AppState>,
    ) -> Result<AvailableCurvesResponse, ServerError> {
        // Return curves from cache
        let curves: Vec<String> = state
            .curve_cache
            .list_ids()
            .iter()
            .map(|id| id.to_string())
            .collect();

        Ok(AvailableCurvesResponse { curves })
    }

    /// Get curve indices for bootstrapping
    pub fn get_curve_indices(_state: &Arc<AppState>) -> Result<CurveIndicesResponse, ServerError> {
        // Load from indices.json
        let indices_path = Path::new("demo/data/input/indices.json");
        let content = std::fs::read_to_string(indices_path)
            .map_err(|e| ServerError::Internal(format!("Failed to read indices.json: {e}")))?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse indices.json: {e}")))?;

        let indices: Vec<String> = data
            .get("indices")
            .and_then(|i| i.get("rates"))
            .and_then(|r| r.get("items"))
            .and_then(|i| i.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("index").and_then(|i| i.as_str()))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        Ok(CurveIndicesResponse { indices })
    }

    /// Get instruments for a specific curve index
    pub fn get_curve_instruments(
        index: &str,
        _state: &Arc<AppState>,
    ) -> Result<CurveInstrumentsResponse, ServerError> {
        // Load market quotes and filter by index
        let rates_path = Path::new("demo/data/input/rates/market_quotes.json");
        let content = std::fs::read_to_string(rates_path).map_err(|e| {
            ServerError::Internal(format!("Failed to read market_quotes.json: {e}"))
        })?;
        let data: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            ServerError::Internal(format!("Failed to parse market_quotes.json: {e}"))
        })?;

        // Map index to currency
        let currency = match index {
            "SOFR" => "USD",
            "ESTR" => "EUR",
            "TONA" => "JPY",
            "SONIA" => "GBP",
            _ => return Err(ServerError::NotFound(format!("Unknown index: {}", index))),
        };

        let mut instruments = Vec::new();

        if let Some(rates_by_currency) = data.get("rates").and_then(|r| r.as_object()) {
            if let Some(rate_types) = rates_by_currency.get(currency).and_then(|r| r.as_object()) {
                for (rate_type, quotes) in rate_types {
                    if let Some(quotes_arr) = quotes.as_array() {
                        for quote in quotes_arr {
                            let quote_index = quote.get("index").and_then(|i| i.as_str());
                            // Filter by index if specified in quote, or include all if no index
                            if quote_index.is_none() || quote_index == Some(index) {
                                let tenor = quote
                                    .get("tenor")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let rate =
                                    quote.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);

                                instruments.push(CurveInstrument {
                                    instrument_type: rate_type.clone(),
                                    tenor,
                                    rate,
                                    enabled: true,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(CurveInstrumentsResponse { instruments })
    }

    // =========================================================================
    // Volcube API
    // =========================================================================

    /// Get volcube indices (swaption currencies)
    pub fn get_volcube_indices(
        _state: &Arc<AppState>,
    ) -> Result<VolcubeIndicesResponse, ServerError> {
        // Load from indices.json
        let indices_path = Path::new("demo/data/input/indices.json");
        let content = std::fs::read_to_string(indices_path)
            .map_err(|e| ServerError::Internal(format!("Failed to read indices.json: {e}")))?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse indices.json: {e}")))?;

        let indices: Vec<String> = data
            .get("indices")
            .and_then(|i| i.get("irvol"))
            .and_then(|r| r.get("items"))
            .and_then(|i| i.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("currency").and_then(|c| c.as_str()))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        Ok(VolcubeIndicesResponse { indices })
    }

    /// Get available volcube calibration models
    pub fn get_volcube_models(
        _state: &Arc<AppState>,
    ) -> Result<VolcubeModelsResponse, ServerError> {
        Ok(VolcubeModelsResponse {
            models: vec![
                "SABR".to_string(),
                "SABR-LMM".to_string(),
                "Heston".to_string(),
                "Local Vol".to_string(),
            ],
        })
    }

    /// Get swaption instruments for volcube calibration
    pub fn get_volcube_instruments(
        currency: &str,
        _state: &Arc<AppState>,
    ) -> Result<VolcubeInstrumentsResponse, ServerError> {
        // Load IR vol data from irvol directory
        let vol_path =
            Path::new("demo/data/input/irvol").join(format!("{}.json", currency.to_lowercase()));

        let content = std::fs::read_to_string(&vol_path).map_err(|_| {
            ServerError::NotFound(format!("Swaption vol data not found for: {}", currency))
        })?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse vol data: {e}")))?;

        let mut instruments = Vec::new();

        if let Some(quotes) = data.get("quotes").and_then(|q| q.as_array()) {
            for quote in quotes {
                let expiry = quote
                    .get("expiry")
                    .and_then(|e| e.as_str())
                    .unwrap_or("")
                    .to_string();
                let tenor = quote
                    .get("tenor")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                // Use atmVol field from the data
                let vol = quote.get("atmVol").and_then(|v| v.as_f64()).unwrap_or(0.0);

                instruments.push(SwaptionInstrument {
                    expiry,
                    tenor,
                    strike: "ATM".to_string(),
                    vol,
                    enabled: true,
                });
            }
        }

        Ok(VolcubeInstrumentsResponse { instruments })
    }

    /// Calibrate volcube (swaption vol surface)
    pub fn calibrate_volcube(
        request: &VolcubeCalibrateRequest,
        _state: &Arc<AppState>,
    ) -> Result<VolcubeCalibrateResponse, ServerError> {
        let start = std::time::Instant::now();

        // Load vol data to get instrument count
        let vol_path = Path::new("demo/data/input/irvol")
            .join(format!("{}.json", request.index.to_lowercase()));

        let content = std::fs::read_to_string(&vol_path).map_err(|_| {
            ServerError::NotFound(format!("Vol data not found for: {}", request.index))
        })?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse vol data: {e}")))?;

        let instrument_count = data
            .get("quotes")
            .and_then(|q| q.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        // Get SABR parameters from the file or use defaults
        let params = data.get("smileParameters");
        let alpha = params
            .and_then(|p| p.get("defaultAlpha"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.02);
        let beta = params
            .and_then(|p| p.get("defaultBeta"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        let rho = params
            .and_then(|p| p.get("defaultRho"))
            .and_then(|v| v.as_f64())
            .unwrap_or(-0.15);
        let nu = params
            .and_then(|p| p.get("defaultNu"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.4);

        let elapsed = start.elapsed();
        let model = request.model.clone().unwrap_or_else(|| "SABR".to_string());

        Ok(VolcubeCalibrateResponse {
            model,
            metadata: CalibrationMetadata {
                instrument_count,
                processing_time_ms: elapsed.as_secs_f64() * 1000.0,
            },
            parameters: CalibrationParameters {
                alpha,
                beta,
                rho,
                nu,
            },
        })
    }

    /// Calibrate FX vol surface
    pub fn calibrate_fxvol(
        request: &FxVolCalibrateRequest,
        _state: &Arc<AppState>,
    ) -> Result<VolcubeCalibrateResponse, ServerError> {
        let start = std::time::Instant::now();

        // Load FX vol data
        let vol_path =
            Path::new("demo/data/input/fxvol").join(format!("{}.json", request.pair.to_lowercase()));

        let content = std::fs::read_to_string(&vol_path).map_err(|_| {
            ServerError::NotFound(format!("FX vol data not found for: {}", request.pair))
        })?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse FX vol data: {e}")))?;

        let instrument_count = data
            .get("smiles")
            .and_then(|s| s.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        let elapsed = start.elapsed();

        // Mock SABR parameters for FX vol
        Ok(VolcubeCalibrateResponse {
            model: "SABR".to_string(),
            metadata: CalibrationMetadata {
                instrument_count,
                processing_time_ms: elapsed.as_secs_f64() * 1000.0,
            },
            parameters: CalibrationParameters {
                alpha: 0.15,
                beta: 0.5,
                rho: -0.20,
                nu: 0.35,
            },
        })
    }

    // =========================================================================
    // Export API
    // =========================================================================

    /// Export market data
    pub fn export_market_data(
        format: ExportFormat,
        state: &Arc<AppState>,
    ) -> Result<Vec<u8>, ServerError> {
        let rates_response = Self::get_market_rates(state)?;

        match format {
            ExportFormat::Csv => {
                let mut csv = String::new();
                csv.push_str("id,currency,tenor,rate_type,value,rate_index,source\n");
                for rate in &rates_response.rates {
                    csv.push_str(&format!(
                        "{},{},{},{},{},{},{}\n",
                        rate.id,
                        rate.currency,
                        rate.tenor,
                        rate.rate_type,
                        rate.value,
                        rate.rate_index.as_deref().unwrap_or(""),
                        rate.source
                    ));
                }
                Ok(csv.into_bytes())
            }
            ExportFormat::Json => {
                let json = serde_json::to_vec(&rates_response).map_err(|e| {
                    ServerError::Internal(format!("JSON serialization failed: {e}"))
                })?;
                Ok(json)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state() -> Arc<AppState> { Arc::new(AppState::new()) }

    #[test]
    fn test_get_instruments() {
        let state = create_test_state();
        let result = DemoService::get_instruments(&state);
        assert!(result.is_ok());
        let instruments = result.unwrap();
        assert!(!instruments.instruments.is_empty());
    }

    #[test]
    fn test_get_fx_vol_pairs() {
        let state = create_test_state();
        let result = DemoService::get_fx_vol_pairs(&state);
        assert!(result.is_ok());
        let pairs = result.unwrap();
        assert!(!pairs.pairs.is_empty());
    }

    #[test]
    fn test_get_ir_vol_currencies() {
        let state = create_test_state();
        let result = DemoService::get_ir_vol_currencies(&state);
        assert!(result.is_ok());
        let currencies = result.unwrap();
        assert!(!currencies.currencies.is_empty());
    }

    #[test]
    fn test_get_conventions() {
        let state = create_test_state();
        let result = DemoService::get_conventions(&state);
        assert!(result.is_ok());
        let conventions = result.unwrap();
        assert!(!conventions.conventions.is_empty());
    }

    #[test]
    fn test_get_market_config() {
        let state = create_test_state();
        let result = DemoService::get_market_config(&state);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.tenor_order.is_empty());
    }

    #[test]
    fn test_get_events() {
        let state = create_test_state();
        let result = DemoService::get_events(&state);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert!(!events.events.is_empty());
    }

    #[test]
    fn test_get_event_types() {
        let state = create_test_state();
        let result = DemoService::get_event_types(&state);
        assert!(result.is_ok());
        let types = result.unwrap();
        assert!(!types.types.is_empty());
    }
}
