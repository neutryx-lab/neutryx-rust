//! Configuration and instrument definition endpoints.

use std::{collections::HashMap, path::Path, sync::Arc};

use super::DemoService;
use crate::{
    error::ServerError,
    rest::dto::demo::{
        AppConfigResponse, AvailableCurvesResponse, CurveIndicesResponse, EnumValue,
        EventTypesResponse, FieldType, InstrumentDef, InstrumentsResponse, MarketConfigResponse,
        ParameterDef,
    },
    services::helpers,
    state::AppState,
};

impl DemoService {
    /// Get application configuration.
    pub fn get_config(_state: &Arc<AppState>) -> Result<AppConfigResponse, ServerError> {
        let currencies_path = Path::new("demo/data/config/currencies.json");
        let currencies_data: serde_json::Value =
            helpers::load_json_value(currencies_path, "currencies.json")?;

        let rate_indices_path = Path::new("demo/data/config/rate_indices.json");
        let rate_indices_data: serde_json::Value =
            helpers::load_json_value(rate_indices_path, "rate_indices.json")?;

        let mut enums: HashMap<String, Vec<EnumValue>> = HashMap::new();

        if let Some(currencies) = currencies_data.get("currencies").and_then(|c| c.as_array()) {
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

        if let Some(rate_indices) = rate_indices_data
            .get("rateIndices")
            .and_then(|i| i.as_array())
        {
            let index_values: Vec<EnumValue> = rate_indices
                .iter()
                .filter_map(|i| Some(EnumValue::Simple(i.get("indexType")?.as_str()?.to_string())))
                .collect();
            enums.insert("rateIndices".to_string(), index_values);
        }

        let mut rate_index_by_currency: HashMap<String, String> = HashMap::new();
        if let Some(currencies) = currencies_data.get("currencies").and_then(|c| c.as_array()) {
            for currency in currencies {
                if let (Some(code), Some(index)) = (
                    currency.get("code").and_then(|c| c.as_str()),
                    currency.get("index").and_then(|i| i.as_str()),
                ) {
                    rate_index_by_currency.insert(code.to_string(), index.to_string());
                }
            }
        }

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

    /// Get available instruments.
    pub fn get_instruments(_state: &Arc<AppState>) -> Result<InstrumentsResponse, ServerError> {
        use crate::rest::dto::demo::{ParameterOption, ParameterValidation};

        // --- Reusable param builders -------------------------------------------
        let notional_param = || ParameterDef {
            name: "notional".into(),
            label: Some("Notional".into()),
            field_type: FieldType::Number,
            default_value: Some(serde_json::json!(1_000_000)),
            options: None,
            validation: Some(ParameterValidation {
                min: Some(0.0),
                max: None,
            }),
        };
        let currency_param = || ParameterDef {
            name: "currency".into(),
            label: Some("Currency".into()),
            field_type: FieldType::Select,
            default_value: Some(serde_json::json!("USD")),
            options: Some(vec![
                ParameterOption {
                    value: "USD".into(),
                    label: "USD".into(),
                },
                ParameterOption {
                    value: "EUR".into(),
                    label: "EUR".into(),
                },
                ParameterOption {
                    value: "GBP".into(),
                    label: "GBP".into(),
                },
                ParameterOption {
                    value: "JPY".into(),
                    label: "JPY".into(),
                },
                ParameterOption {
                    value: "CHF".into(),
                    label: "CHF".into(),
                },
            ]),
            validation: None,
        };
        let start_date_param = || ParameterDef {
            name: "startDate".into(),
            label: Some("Start Date".into()),
            field_type: FieldType::Date,
            default_value: None,
            options: None,
            validation: None,
        };
        let end_date_param = || ParameterDef {
            name: "endDate".into(),
            label: Some("End Date".into()),
            field_type: FieldType::Date,
            default_value: None,
            options: None,
            validation: None,
        };
        let rate_index_param = || ParameterDef {
            name: "rateIndex".into(),
            label: Some("Float Index".into()),
            field_type: FieldType::Select,
            default_value: Some(serde_json::json!("SOFR")),
            options: Some(vec![
                ParameterOption {
                    value: "SOFR".into(),
                    label: "USD SOFR".into(),
                },
                ParameterOption {
                    value: "ESTR".into(),
                    label: "EUR ESTR".into(),
                },
                ParameterOption {
                    value: "EURIBOR3M".into(),
                    label: "EUR EURIBOR 3M".into(),
                },
                ParameterOption {
                    value: "EURIBOR6M".into(),
                    label: "EUR EURIBOR 6M".into(),
                },
                ParameterOption {
                    value: "SONIA".into(),
                    label: "GBP SONIA".into(),
                },
                ParameterOption {
                    value: "TONA".into(),
                    label: "JPY TONA".into(),
                },
                ParameterOption {
                    value: "SARON".into(),
                    label: "CHF SARON".into(),
                },
            ]),
            validation: None,
        };
        let fixed_rate_param = |default: f64| ParameterDef {
            name: "fixedRate".into(),
            label: Some("Fixed Rate".into()),
            field_type: FieldType::Number,
            default_value: Some(serde_json::json!(default)),
            options: None,
            validation: Some(ParameterValidation {
                min: Some(-0.1),
                max: Some(0.5),
            }),
        };
        let strike_param = || ParameterDef {
            name: "strike".into(),
            label: Some("Strike".into()),
            field_type: FieldType::Number,
            default_value: None,
            options: None,
            validation: None,
        };
        let currency_pair_param = || ParameterDef {
            name: "currencyPair".into(),
            label: Some("Currency Pair".into()),
            field_type: FieldType::Select,
            default_value: Some(serde_json::json!("EURUSD")),
            options: Some(vec![
                ParameterOption {
                    value: "EURUSD".into(),
                    label: "EUR/USD".into(),
                },
                ParameterOption {
                    value: "USDJPY".into(),
                    label: "USD/JPY".into(),
                },
                ParameterOption {
                    value: "GBPUSD".into(),
                    label: "GBP/USD".into(),
                },
                ParameterOption {
                    value: "USDCHF".into(),
                    label: "USD/CHF".into(),
                },
                ParameterOption {
                    value: "USDJPY".into(),
                    label: "USD/JPY".into(),
                },
            ]),
            validation: None,
        };
        let option_type_param = || ParameterDef {
            name: "optionType".into(),
            label: Some("Option Type".into()),
            field_type: FieldType::Select,
            default_value: Some(serde_json::json!("Call")),
            options: Some(vec![
                ParameterOption {
                    value: "Call".into(),
                    label: "Call".into(),
                },
                ParameterOption {
                    value: "Put".into(),
                    label: "Put".into(),
                },
            ]),
            validation: None,
        };
        let expiry_param = || ParameterDef {
            name: "expiry".into(),
            label: Some("Expiry".into()),
            field_type: FieldType::Date,
            default_value: None,
            options: None,
            validation: None,
        };
        let spread_param = |default: f64| ParameterDef {
            name: "spread".into(),
            label: Some("Spread (bp)".into()),
            field_type: FieldType::Number,
            default_value: Some(serde_json::json!(default)),
            options: None,
            validation: None,
        };

        let barrier_level_param = || ParameterDef {
            name: "barrierLevel".into(),
            label: Some("Barrier Level".into()),
            field_type: FieldType::Number,
            default_value: None,
            options: None,
            validation: None,
        };
        let barrier_type_param = || ParameterDef {
            name: "barrierType".into(),
            label: Some("Barrier Type".into()),
            field_type: FieldType::Select,
            default_value: Some(serde_json::json!("KnockOut")),
            options: Some(vec![
                ParameterOption {
                    value: "KnockOut".into(),
                    label: "Knock-Out".into(),
                },
                ParameterOption {
                    value: "KnockIn".into(),
                    label: "Knock-In".into(),
                },
            ]),
            validation: None,
        };
        let barrier_direction_param = || ParameterDef {
            name: "barrierDirection".into(),
            label: Some("Barrier Direction".into()),
            field_type: FieldType::Select,
            default_value: Some(serde_json::json!("Up")),
            options: Some(vec![
                ParameterOption {
                    value: "Up".into(),
                    label: "Up".into(),
                },
                ParameterOption {
                    value: "Down".into(),
                    label: "Down".into(),
                },
            ]),
            validation: None,
        };
        let quantity_param = || ParameterDef {
            name: "quantity".into(),
            label: Some("Quantity".into()),
            field_type: FieldType::Number,
            default_value: Some(serde_json::json!(1000)),
            options: None,
            validation: Some(ParameterValidation {
                min: Some(0.0),
                max: None,
            }),
        };
        let commodity_param = || ParameterDef {
            name: "commodityType".into(),
            label: Some("Commodity".into()),
            field_type: FieldType::Select,
            default_value: Some(serde_json::json!("CrudeOil")),
            options: Some(vec![
                ParameterOption {
                    value: "CrudeOil".into(),
                    label: "Crude Oil".into(),
                },
                ParameterOption {
                    value: "NaturalGas".into(),
                    label: "Natural Gas".into(),
                },
                ParameterOption {
                    value: "Gold".into(),
                    label: "Gold".into(),
                },
                ParameterOption {
                    value: "Silver".into(),
                    label: "Silver".into(),
                },
                ParameterOption {
                    value: "Copper".into(),
                    label: "Copper".into(),
                },
                ParameterOption {
                    value: "Wheat".into(),
                    label: "Wheat".into(),
                },
                ParameterOption {
                    value: "Corn".into(),
                    label: "Corn".into(),
                },
            ]),
            validation: None,
        };
        let forward_price_param = |default: f64| ParameterDef {
            name: "forwardPrice".into(),
            label: Some("Forward Price".into()),
            field_type: FieldType::Number,
            default_value: Some(serde_json::json!(default)),
            options: None,
            validation: Some(ParameterValidation {
                min: Some(0.0),
                max: None,
            }),
        };

        let def = |typ: &str,
                   id: &str,
                   name: &str,
                   class: &str,
                   req: Vec<ParameterDef>,
                   opt: Vec<ParameterDef>| InstrumentDef {
            instrument_type: typ.into(),
            id: Some(id.into()),
            display_name: Some(name.into()),
            asset_class_name: Some(class.into()),
            required_params: req,
            optional_params: opt,
        };

        let instruments = vec![
            // ── Rates ──────────────────────────────────────────────────────
            def(
                "IRS",
                "irs",
                "Interest Rate Swap",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![
                    fixed_rate_param(0.05),
                    rate_index_param(),
                    ParameterDef {
                        name: "swapType".into(),
                        label: Some("Swap Type".into()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("Vanilla")),
                        options: Some(vec![
                            ParameterOption {
                                value: "Vanilla".into(),
                                label: "Vanilla".into(),
                            },
                            ParameterOption {
                                value: "OIS".into(),
                                label: "OIS".into(),
                            },
                        ]),
                        validation: None,
                    },
                ],
            ),
            def(
                "BasisSwap",
                "basis-swap",
                "Basis Swap",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![rate_index_param(), spread_param(0.0)],
            ),
            def(
                "Swaption",
                "swaption",
                "Swaption",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![option_type_param()],
            ),
            def(
                "CapFloor",
                "cap-floor",
                "Cap / Floor",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    strike_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![rate_index_param()],
            ),
            def(
                "Deposit",
                "deposit",
                "Deposit",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![fixed_rate_param(0.05)],
            ),
            def(
                "Fra",
                "fra",
                "FRA",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                    strike_param(),
                ],
                vec![rate_index_param()],
            ),
            def(
                "Bond",
                "bond",
                "Bond",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![fixed_rate_param(0.03)],
            ),
            def(
                "Futures",
                "futures",
                "IR Futures",
                "Rates",
                vec![notional_param(), currency_param(), expiry_param()],
                vec![
                    rate_index_param(),
                    ParameterDef {
                        name: "futuresPrice".into(),
                        label: Some("Futures Price".into()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(95.0)),
                        options: None,
                        validation: Some(ParameterValidation {
                            min: Some(80.0),
                            max: Some(110.0),
                        }),
                    },
                ],
            ),
            def(
                "Frn",
                "frn",
                "Floating Rate Note",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![rate_index_param(), spread_param(10.0)],
            ),
            def(
                "CmsSwap",
                "cms-swap",
                "CMS Swap",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![
                    spread_param(0.0),
                    ParameterDef {
                        name: "cmsTenor".into(),
                        label: Some("CMS Tenor".into()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("10Y")),
                        options: Some(vec![
                            ParameterOption {
                                value: "5Y".into(),
                                label: "5Y".into(),
                            },
                            ParameterOption {
                                value: "10Y".into(),
                                label: "10Y".into(),
                            },
                            ParameterOption {
                                value: "30Y".into(),
                                label: "30Y".into(),
                            },
                        ]),
                        validation: None,
                    },
                ],
            ),
            def(
                "InflationSwap",
                "inflation-swap",
                "Inflation Swap",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![
                    fixed_rate_param(0.025),
                    ParameterDef {
                        name: "inflationIndex".into(),
                        label: Some("Inflation Index".into()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("CPI")),
                        options: Some(vec![
                            ParameterOption {
                                value: "CPI".into(),
                                label: "US CPI".into(),
                            },
                            ParameterOption {
                                value: "HICP".into(),
                                label: "EUR HICP".into(),
                            },
                            ParameterOption {
                                value: "RPI".into(),
                                label: "UK RPI".into(),
                            },
                        ]),
                        validation: None,
                    },
                    ParameterDef {
                        name: "inflationSwapType".into(),
                        label: Some("Swap Type".into()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("ZeroCoupon")),
                        options: Some(vec![
                            ParameterOption {
                                value: "ZeroCoupon".into(),
                                label: "Zero Coupon".into(),
                            },
                            ParameterOption {
                                value: "YearOnYear".into(),
                                label: "Year-on-Year".into(),
                            },
                        ]),
                        validation: None,
                    },
                ],
            ),
            // ── FX ─────────────────────────────────────────────────────────
            def(
                "FxSpot",
                "fx-spot",
                "FX Spot",
                "FX",
                vec![notional_param(), currency_pair_param()],
                vec![],
            ),
            def(
                "FxForward",
                "fx-forward",
                "FX Forward",
                "FX",
                vec![notional_param(), currency_pair_param()],
                vec![end_date_param()],
            ),
            def(
                "FxVanillaOption",
                "fx-vanilla-option",
                "FX Vanilla Option",
                "FX",
                vec![
                    notional_param(),
                    currency_pair_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![option_type_param()],
            ),
            def(
                "FxBarrierOption",
                "fx-barrier-option",
                "FX Barrier Option",
                "FX",
                vec![
                    notional_param(),
                    currency_pair_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![
                    option_type_param(),
                    barrier_level_param(),
                    barrier_type_param(),
                    barrier_direction_param(),
                ],
            ),
            def(
                "FxSwap",
                "fx-swap",
                "FX Swap",
                "FX",
                vec![notional_param(), currency_pair_param()],
                vec![
                    ParameterDef {
                        name: "nearDate".into(),
                        label: Some("Near Leg Date".into()),
                        field_type: FieldType::Date,
                        default_value: None,
                        options: None,
                        validation: None,
                    },
                    ParameterDef {
                        name: "farDate".into(),
                        label: Some("Far Leg Date".into()),
                        field_type: FieldType::Date,
                        default_value: None,
                        options: None,
                        validation: None,
                    },
                ],
            ),
            def(
                "CrossCurrencyBasisSwap",
                "xccy-basis-swap",
                "Cross-Currency Basis Swap",
                "FX",
                vec![notional_param(), start_date_param(), end_date_param()],
                vec![
                    ParameterDef {
                        name: "domesticCurrency".into(),
                        label: Some("Domestic Currency".into()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("USD")),
                        options: Some(vec![
                            ParameterOption {
                                value: "USD".into(),
                                label: "USD".into(),
                            },
                            ParameterOption {
                                value: "EUR".into(),
                                label: "EUR".into(),
                            },
                            ParameterOption {
                                value: "GBP".into(),
                                label: "GBP".into(),
                            },
                            ParameterOption {
                                value: "JPY".into(),
                                label: "JPY".into(),
                            },
                        ]),
                        validation: None,
                    },
                    ParameterDef {
                        name: "foreignCurrency".into(),
                        label: Some("Foreign Currency".into()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("EUR")),
                        options: Some(vec![
                            ParameterOption {
                                value: "USD".into(),
                                label: "USD".into(),
                            },
                            ParameterOption {
                                value: "EUR".into(),
                                label: "EUR".into(),
                            },
                            ParameterOption {
                                value: "GBP".into(),
                                label: "GBP".into(),
                            },
                            ParameterOption {
                                value: "JPY".into(),
                                label: "JPY".into(),
                            },
                        ]),
                        validation: None,
                    },
                    spread_param(-15.0),
                ],
            ),
            // ── Equity ─────────────────────────────────────────────────────
            def(
                "EquityForward",
                "equity-forward",
                "Equity Forward",
                "Equity",
                vec![notional_param(), currency_param(), end_date_param()],
                vec![],
            ),
            def(
                "EquityVanillaOption",
                "equity-vanilla-option",
                "Equity Vanilla Option",
                "Equity",
                vec![
                    notional_param(),
                    currency_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![option_type_param()],
            ),
            def(
                "EquityBarrierOption",
                "equity-barrier-option",
                "Equity Barrier Option",
                "Equity",
                vec![
                    notional_param(),
                    currency_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![
                    option_type_param(),
                    barrier_level_param(),
                    barrier_type_param(),
                    barrier_direction_param(),
                ],
            ),
            def(
                "AsianOption",
                "asian-option",
                "Asian Option",
                "Equity",
                vec![
                    notional_param(),
                    currency_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![option_type_param()],
            ),
            def(
                "LookbackOption",
                "lookback-option",
                "Lookback Option",
                "Equity",
                vec![notional_param(), currency_param(), expiry_param()],
                vec![
                    option_type_param(),
                    strike_param(),
                    ParameterDef {
                        name: "lookbackType".into(),
                        label: Some("Lookback Type".into()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("FloatingStrike")),
                        options: Some(vec![
                            ParameterOption {
                                value: "FixedStrike".into(),
                                label: "Fixed Strike".into(),
                            },
                            ParameterOption {
                                value: "FloatingStrike".into(),
                                label: "Floating Strike".into(),
                            },
                        ]),
                        validation: None,
                    },
                ],
            ),
            def(
                "EquitySwap",
                "equity-swap",
                "Equity Swap",
                "Equity",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![
                    spread_param(10.0),
                    ParameterDef {
                        name: "returnType".into(),
                        label: Some("Return Type".into()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("TotalReturn")),
                        options: Some(vec![
                            ParameterOption {
                                value: "TotalReturn".into(),
                                label: "Total Return".into(),
                            },
                            ParameterOption {
                                value: "Price".into(),
                                label: "Price Return".into(),
                            },
                        ]),
                        validation: None,
                    },
                ],
            ),
            def(
                "BasketOption",
                "basket-option",
                "Basket Option",
                "Equity",
                vec![
                    notional_param(),
                    currency_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![option_type_param()],
            ),
            // ── Credit ─────────────────────────────────────────────────────
            def(
                "CDS",
                "cds",
                "Credit Default Swap",
                "Credit",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![spread_param(100.0)],
            ),
            def(
                "CdsIndex",
                "cds-index",
                "CDS Index",
                "Credit",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![
                    spread_param(60.0),
                    ParameterDef {
                        name: "indexName".into(),
                        label: Some("Index Name".into()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("CDX.NA.IG")),
                        options: Some(vec![
                            ParameterOption {
                                value: "CDX.NA.IG".into(),
                                label: "CDX NA IG".into(),
                            },
                            ParameterOption {
                                value: "CDX.NA.HY".into(),
                                label: "CDX NA HY".into(),
                            },
                            ParameterOption {
                                value: "iTraxx Europe".into(),
                                label: "iTraxx Europe".into(),
                            },
                            ParameterOption {
                                value: "iTraxx Xover".into(),
                                label: "iTraxx Crossover".into(),
                            },
                        ]),
                        validation: None,
                    },
                ],
            ),
            def(
                "CdsOption",
                "cds-option",
                "CDS Option",
                "Credit",
                vec![notional_param(), currency_param(), expiry_param()],
                vec![
                    spread_param(100.0),
                    end_date_param(),
                    ParameterDef {
                        name: "isPayer".into(),
                        label: Some("Payer/Receiver".into()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("true")),
                        options: Some(vec![
                            ParameterOption {
                                value: "true".into(),
                                label: "Payer (Buy Protection)".into(),
                            },
                            ParameterOption {
                                value: "false".into(),
                                label: "Receiver (Sell Protection)".into(),
                            },
                        ]),
                        validation: None,
                    },
                ],
            ),
            def(
                "NtdBasket",
                "ntd-basket",
                "Nth-to-Default Basket",
                "Credit",
                vec![
                    notional_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![
                    spread_param(150.0),
                    ParameterDef {
                        name: "nthToDefault".into(),
                        label: Some("N (Nth-to-Default)".into()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(1)),
                        options: None,
                        validation: Some(ParameterValidation {
                            min: Some(1.0),
                            max: Some(10.0),
                        }),
                    },
                ],
            ),
            // ── Commodity ──────────────────────────────────────────────────
            def(
                "CommodityForward",
                "commodity-forward",
                "Commodity Forward",
                "Commodity",
                vec![quantity_param(), currency_param(), end_date_param()],
                vec![commodity_param(), forward_price_param(75.0)],
            ),
            def(
                "CommoditySwap",
                "commodity-swap",
                "Commodity Swap",
                "Commodity",
                vec![
                    quantity_param(),
                    currency_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![
                    commodity_param(),
                    ParameterDef {
                        name: "fixedPrice".into(),
                        label: Some("Fixed Price".into()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(75.0)),
                        options: None,
                        validation: Some(ParameterValidation {
                            min: Some(0.0),
                            max: None,
                        }),
                    },
                ],
            ),
            def(
                "CommodityVanillaOption",
                "commodity-vanilla-option",
                "Commodity Option",
                "Commodity",
                vec![
                    quantity_param(),
                    currency_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![commodity_param(), option_type_param()],
            ),
            def(
                "CommodityAsianOption",
                "commodity-asian-option",
                "Commodity Asian Option",
                "Commodity",
                vec![
                    quantity_param(),
                    currency_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![commodity_param(), option_type_param()],
            ),
            def(
                "SpreadOption",
                "spread-option",
                "Spread Option",
                "Commodity",
                vec![quantity_param(), currency_param(), expiry_param()],
                vec![
                    ParameterDef {
                        name: "spreadStrike".into(),
                        label: Some("Spread Strike".into()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(10.0)),
                        options: None,
                        validation: None,
                    },
                    option_type_param(),
                ],
            ),
            // ── Rates (New) ──────────────────────────────────────────────
            def(
                "BondFuture",
                "bond-future",
                "Bond Future",
                "Rates",
                vec![notional_param(), currency_param(), expiry_param()],
                vec![
                    ParameterDef {
                        name: "price".into(),
                        label: Some("Futures Price".into()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(100.0)),
                        options: None,
                        validation: Some(ParameterValidation {
                            min: Some(0.0),
                            max: None,
                        }),
                    },
                    ParameterDef {
                        name: "bondCoupon".into(),
                        label: Some("Bond Coupon".into()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(0.06)),
                        options: None,
                        validation: Some(ParameterValidation {
                            min: Some(0.0),
                            max: Some(0.5),
                        }),
                    },
                    ParameterDef {
                        name: "bondTermYears".into(),
                        label: Some("Bond Term (Years)".into()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(10)),
                        options: None,
                        validation: Some(ParameterValidation {
                            min: Some(1.0),
                            max: Some(50.0),
                        }),
                    },
                ],
            ),
            def(
                "BondFutureOption",
                "bond-future-option",
                "Bond Future Option",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![option_type_param()],
            ),
            def(
                "IrFutureOption",
                "ir-future-option",
                "IR Future Option",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![option_type_param(), rate_index_param()],
            ),
            def(
                "SwaptionStraddle",
                "swaption-straddle",
                "Swaption Straddle",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![],
            ),
            def(
                "CapFloorStraddle",
                "cap-floor-straddle",
                "Cap/Floor Straddle",
                "Rates",
                vec![
                    notional_param(),
                    currency_param(),
                    strike_param(),
                    start_date_param(),
                    end_date_param(),
                ],
                vec![rate_index_param()],
            ),
            // ── Commodity (New) ───────────────────────────────────────────
            def(
                "CommodityFuture",
                "commodity-future",
                "Commodity Future",
                "Commodity",
                vec![quantity_param(), currency_param(), expiry_param()],
                vec![
                    commodity_param(),
                    ParameterDef {
                        name: "price".into(),
                        label: Some("Futures Price".into()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(75.0)),
                        options: None,
                        validation: Some(ParameterValidation {
                            min: Some(0.0),
                            max: None,
                        }),
                    },
                ],
            ),
            def(
                "CommodityFutureOption",
                "commodity-future-option",
                "Commodity Future Option",
                "Commodity",
                vec![
                    quantity_param(),
                    currency_param(),
                    strike_param(),
                    expiry_param(),
                ],
                vec![commodity_param(), option_type_param()],
            ),
        ];

        Ok(InstrumentsResponse { instruments })
    }

    /// Get market config.
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

    /// Get event types.
    pub fn get_event_types(_state: &Arc<AppState>) -> Result<EventTypesResponse, ServerError> {
        Ok(EventTypesResponse {
            types: vec![
                "central_bank_meeting".to_string(),
                "economic_release".to_string(),
                "holiday".to_string(),
                "news".to_string(),
                "expiry".to_string(),
                "turn_of_year".to_string(),
                "turn_of_quarter".to_string(),
                "turn_of_month".to_string(),
                "turn".to_string(),
                "other".to_string(),
            ],
        })
    }

    /// Get curve indices for bootstrapping.
    pub fn get_curve_indices(_state: &Arc<AppState>) -> Result<CurveIndicesResponse, ServerError> {
        let rate_indices_path = Path::new("demo/data/config/rate_indices.json");
        let data: serde_json::Value =
            helpers::load_json_value(rate_indices_path, "rate_indices.json")?;

        let indices: Vec<String> = data
            .get("rateIndices")
            .and_then(|i| i.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("indexType").and_then(|i| i.as_str()))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        Ok(CurveIndicesResponse { indices })
    }

    /// Get available curves.
    pub fn get_available_curves(
        state: &Arc<AppState>,
    ) -> Result<AvailableCurvesResponse, ServerError> {
        let curves: Vec<String> = state
            .curve_cache
            .list_ids()
            .iter()
            .map(|id| id.to_string())
            .collect();

        Ok(AvailableCurvesResponse { curves })
    }
}
