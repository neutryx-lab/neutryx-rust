//! Demo service wrapping data loading and demo-specific operations
//!
//! Provides endpoints for the demo_gui frontend.

use std::{collections::HashMap, path::Path, sync::Arc, time::Instant};

use crate::{
    error::ServerError,
    rest::dto::demo::{
        AppConfigResponse, AvailableCurvesResponse, Cashflow, Convention, ConventionField,
        ConventionsResponse, CurveIndicesResponse, CurveInstrument, CurveInstrumentsResponse,
        DemoGreeksRequest, DemoGreeksResult, DemoPricingRequest, DemoPricingResult, EnumValue,
        EventType, EventTypesResponse, EventsResponse, ExpandedTrade, ExportFormat, FieldType,
        FxVolPair, FxVolPairsResponse, FxVolQuote, FxVolQuotesResponse, Importance, InstrumentDef,
        InstrumentsResponse, IrVolCurrenciesResponse, IrVolCurrency, IrVolQuote, IrVolQuotesResponse,
        MarketConfigResponse, MarketEvent, MarketRate, MarketRateDetailResponse, MarketRatesResponse,
        ParameterDef, SmilePoint, SwaptionInstrument, TradeExpandRequest, TradeLeg, TradeMetadata,
        VolcubeIndicesResponse, VolcubeInstrumentsResponse, VolcubeModelsResponse,
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

        Ok(MarketRateDetailResponse {
            rate,
            instrument: None,
            convention: None,
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
        // Return hardcoded conventions
        let conventions = vec![
            Convention {
                id: "USD-OIS".to_string(),
                convention_type: "OIS".to_string(),
                currency: "USD".to_string(),
                is_default: Some(true),
                fields: Some(vec![
                    ConventionField {
                        label: "Day Count".to_string(),
                        value: "ACT/360".to_string(),
                    },
                    ConventionField {
                        label: "Payment Frequency".to_string(),
                        value: "Annual".to_string(),
                    },
                ]),
            },
            Convention {
                id: "EUR-OIS".to_string(),
                convention_type: "OIS".to_string(),
                currency: "EUR".to_string(),
                is_default: Some(true),
                fields: Some(vec![
                    ConventionField {
                        label: "Day Count".to_string(),
                        value: "ACT/360".to_string(),
                    },
                    ConventionField {
                        label: "Payment Frequency".to_string(),
                        value: "Annual".to_string(),
                    },
                ]),
            },
            Convention {
                id: "USD-SWAP".to_string(),
                convention_type: "Swap".to_string(),
                currency: "USD".to_string(),
                is_default: Some(true),
                fields: Some(vec![
                    ConventionField {
                        label: "Fixed Day Count".to_string(),
                        value: "30/360".to_string(),
                    },
                    ConventionField {
                        label: "Float Day Count".to_string(),
                        value: "ACT/360".to_string(),
                    },
                ]),
            },
        ];

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
        // Return mock events
        Ok(EventsResponse {
            events: vec![
                MarketEvent {
                    id: "fed-2026-02".to_string(),
                    date: "2026-02-15".to_string(),
                    event_type: EventType::CentralBankMeeting,
                    title: "FOMC Meeting".to_string(),
                    description: Some("Federal Reserve FOMC meeting".to_string()),
                    currency: Some("USD".to_string()),
                    region: Some("US".to_string()),
                    importance: Importance::Critical,
                    time: Some("14:00".to_string()),
                    timezone: Some("America/New_York".to_string()),
                    source: "Federal Reserve".to_string(),
                    tags: Some(vec!["monetary_policy".to_string()]),
                    central_bank: Some(crate::rest::dto::demo::CentralBank {
                        name: "Federal Reserve".to_string(),
                        code: "FED".to_string(),
                        currency: "USD".to_string(),
                    }),
                    previous: None,
                    forecast: None,
                    actual: None,
                },
                MarketEvent {
                    id: "ecb-2026-02".to_string(),
                    date: "2026-02-20".to_string(),
                    event_type: EventType::CentralBankMeeting,
                    title: "ECB Meeting".to_string(),
                    description: Some("European Central Bank meeting".to_string()),
                    currency: Some("EUR".to_string()),
                    region: Some("EU".to_string()),
                    importance: Importance::Critical,
                    time: Some("13:45".to_string()),
                    timezone: Some("Europe/Frankfurt".to_string()),
                    source: "ECB".to_string(),
                    tags: Some(vec!["monetary_policy".to_string()]),
                    central_bank: Some(crate::rest::dto::demo::CentralBank {
                        name: "European Central Bank".to_string(),
                        code: "ECB".to_string(),
                        currency: "EUR".to_string(),
                    }),
                    previous: None,
                    forecast: None,
                    actual: None,
                },
            ],
        })
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
        let content = std::fs::read_to_string(rates_path)
            .map_err(|e| ServerError::Internal(format!("Failed to read market_quotes.json: {e}")))?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse market_quotes.json: {e}")))?;

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
                                let rate = quote.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);

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
    pub fn get_volcube_indices(_state: &Arc<AppState>) -> Result<VolcubeIndicesResponse, ServerError> {
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
    pub fn get_volcube_models(_state: &Arc<AppState>) -> Result<VolcubeModelsResponse, ServerError> {
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
        // Load IR vol data
        let vol_path = Path::new("demo/data/input/vol/swaption")
            .join(format!("{}_swaption_vol.json", currency.to_lowercase()));

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
                let vol = quote.get("vol").and_then(|v| v.as_f64()).unwrap_or(0.0);

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
