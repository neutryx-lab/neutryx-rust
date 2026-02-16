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
                optional_params: vec![
                    ParameterDef {
                        name: "fixedRate".to_string(),
                        label: Some("Fixed Rate".to_string()),
                        field_type: FieldType::Number,
                        default_value: Some(serde_json::json!(0.05)),
                        options: None,
                        validation: Some(crate::rest::dto::demo::ParameterValidation {
                            min: Some(-0.1),
                            max: Some(0.5),
                        }),
                    },
                    ParameterDef {
                        name: "rateIndex".to_string(),
                        label: Some("Float Index".to_string()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("SOFR")),
                        options: Some(vec![
                            crate::rest::dto::demo::ParameterOption {
                                value: "SOFR".to_string(),
                                label: "USD SOFR".to_string(),
                            },
                            crate::rest::dto::demo::ParameterOption {
                                value: "ESTR".to_string(),
                                label: "EUR ESTR".to_string(),
                            },
                            crate::rest::dto::demo::ParameterOption {
                                value: "EURIBOR3M".to_string(),
                                label: "EUR EURIBOR 3M".to_string(),
                            },
                            crate::rest::dto::demo::ParameterOption {
                                value: "EURIBOR6M".to_string(),
                                label: "EUR EURIBOR 6M".to_string(),
                            },
                            crate::rest::dto::demo::ParameterOption {
                                value: "SONIA".to_string(),
                                label: "GBP SONIA".to_string(),
                            },
                            crate::rest::dto::demo::ParameterOption {
                                value: "TONA".to_string(),
                                label: "JPY TONA".to_string(),
                            },
                            crate::rest::dto::demo::ParameterOption {
                                value: "SARON".to_string(),
                                label: "CHF SARON".to_string(),
                            },
                        ]),
                        validation: None,
                    },
                    ParameterDef {
                        name: "swapType".to_string(),
                        label: Some("Swap Type".to_string()),
                        field_type: FieldType::Select,
                        default_value: Some(serde_json::json!("Vanilla")),
                        options: Some(vec![
                            crate::rest::dto::demo::ParameterOption {
                                value: "Vanilla".to_string(),
                                label: "Vanilla".to_string(),
                            },
                            crate::rest::dto::demo::ParameterOption {
                                value: "OIS".to_string(),
                                label: "OIS".to_string(),
                            },
                        ]),
                        validation: None,
                    },
                ],
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
