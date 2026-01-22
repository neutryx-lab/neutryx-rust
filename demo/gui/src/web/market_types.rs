//! Market Data API DTO types.
//!
//! This module provides request/response types for the Market Data Viewer API.
//!
//! # Task Coverage
//!
//! - Task 1.1: MarketRateResponse, MarketRatesListResponse, MarketRateQuery
//! - Task 1.2: InstrumentResponse, ConventionResponse, MarketRateDetailResponse
//! - Task 1.3: ApiError
//!
//! # Requirements Coverage
//!
//! - Requirement 1.1-1.5: マーケットレートデータセット定義
//! - Requirement 2.1-2.5: Instrument 紐付け情報表示
//! - Requirement 3.1-3.5: Convention 情報表示
//! - Requirement 5.1-5.7: REST API エンドポイント

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// =============================================================================
// Task 1.1: Market Rate Response Types
// =============================================================================

/// Market rate API response.
///
/// Represents a single market rate quote for the API response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketRateResponse {
    /// Unique rate identifier (e.g., "USD-3M-DEPO").
    pub id: String,
    /// Currency code (e.g., "USD", "EUR", "JPY").
    pub currency: String,
    /// Tenor string (e.g., "3M", "1Y", "5Y").
    pub tenor: String,
    /// Rate type (e.g., "Deposit", "Swap", "Ois").
    pub rate_type: String,
    /// Rate value.
    pub value: f64,
    /// Quote type (e.g., "Mid", "Bid", "Ask").
    pub quote_type: String,
    /// Unix timestamp in milliseconds.
    pub timestamp: i64,
    /// Data source (e.g., "Bloomberg", "Reuters").
    pub source: String,
    /// Whether the rate is considered stale.
    pub is_stale: bool,
    /// Optional rate index (e.g., "SOFR", "EURIBOR").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
}

/// Market rates list response with metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketRatesListResponse {
    /// List of market rates.
    pub rates: Vec<MarketRateResponse>,
    /// Last update timestamp in Unix milliseconds.
    pub last_updated: i64,
    /// Total count of rates.
    pub total_count: usize,
}

/// Query parameters for market rate filtering.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MarketRateQuery {
    /// Filter by currency code.
    pub currency: Option<String>,
    /// Filter by rate type.
    pub rate_type: Option<String>,
    /// Filter by rate index.
    pub index: Option<String>,
}

// =============================================================================
// Task 1.2: Instrument and Convention Response Types
// =============================================================================

/// Instrument response for rate detail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentResponse {
    /// Instrument type (e.g., "Deposit", "ParSwap", "Ois").
    pub instrument_type: String,
    /// Currency code.
    pub currency: String,
    /// Start date (ISO 8601 format).
    pub start_date: String,
    /// End date / maturity (ISO 8601 format).
    pub end_date: String,
    /// Rate value.
    pub rate: f64,
    /// Additional instrument parameters.
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Convention field (key/value pair).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConventionField {
    /// Display label (e.g., "Day Count").
    pub label: String,
    /// Value string (e.g., "ACT/360").
    pub value: String,
}

/// Convention response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConventionResponse {
    /// Convention type (e.g., "SwapConvention", "FxConvention").
    pub convention_type: String,
    /// List of convention fields.
    pub fields: Vec<ConventionField>,
}

/// Market rate detail response with instrument and convention.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketRateDetailResponse {
    /// Base rate information.
    pub rate: MarketRateResponse,
    /// Linked instrument (if mappable).
    pub instrument: Option<InstrumentResponse>,
    /// Applicable convention.
    pub convention: Option<ConventionResponse>,
}

/// Convention summary for list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConventionSummary {
    /// Convention identifier (e.g., "USD-SOFR-SWAP").
    pub id: String,
    /// Currency code.
    pub currency: String,
    /// Convention type.
    pub convention_type: String,
    /// Whether this is the default convention for the currency.
    pub is_default: bool,
}

/// Conventions list response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConventionsListResponse {
    /// List of convention summaries.
    pub conventions: Vec<ConventionSummary>,
}

// =============================================================================
// Task 1.3: API Error Response
// =============================================================================

/// API error response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataApiError {
    /// Error code (e.g., "not_found", "invalid_parameter").
    pub error: String,
    /// Human-readable error message.
    pub message: String,
    /// Additional error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl MarketDataApiError {
    /// Creates a not found error.
    pub fn not_found(resource: &str, id: &str) -> Self {
        Self {
            error: "not_found".to_string(),
            message: format!("{} with ID '{}' not found", resource, id),
            details: None,
        }
    }

    /// Creates an invalid parameter error.
    pub fn invalid_parameter(param: &str, reason: &str) -> Self {
        Self {
            error: "invalid_parameter".to_string(),
            message: format!("Invalid parameter '{}': {}", param, reason),
            details: None,
        }
    }

    /// Creates an internal error.
    pub fn internal(message: &str) -> Self {
        Self {
            error: "internal_error".to_string(),
            message: message.to_string(),
            details: None,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 1.1 Tests: MarketRateResponse serialisation
    // =========================================================================

    mod market_rate_response_tests {
        use super::*;

        #[test]
        fn test_market_rate_response_serialise_camel_case() {
            let response = MarketRateResponse {
                id: "USD-3M-DEPO".to_string(),
                currency: "USD".to_string(),
                tenor: "3M".to_string(),
                rate_type: "Deposit".to_string(),
                value: 0.05,
                quote_type: "Mid".to_string(),
                timestamp: 1700000000000,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: None,
            };

            let json = serde_json::to_string(&response).unwrap();

            // Verify camelCase field names
            assert!(json.contains("\"rateType\""));
            assert!(json.contains("\"quoteType\""));
            assert!(json.contains("\"isStale\""));
            assert!(!json.contains("\"rate_type\""));
            assert!(!json.contains("\"quote_type\""));
            assert!(!json.contains("\"is_stale\""));
        }

        #[test]
        fn test_market_rate_response_deserialise_camel_case() {
            let json = r#"{
                "id": "USD-3M-DEPO",
                "currency": "USD",
                "tenor": "3M",
                "rateType": "Deposit",
                "value": 0.05,
                "quoteType": "Mid",
                "timestamp": 1700000000000,
                "source": "Bloomberg",
                "isStale": false
            }"#;

            let response: MarketRateResponse = serde_json::from_str(json).unwrap();

            assert_eq!(response.id, "USD-3M-DEPO");
            assert_eq!(response.rate_type, "Deposit");
            assert_eq!(response.quote_type, "Mid");
            assert!(!response.is_stale);
        }

        #[test]
        fn test_market_rate_response_with_rate_index() {
            let response = MarketRateResponse {
                id: "USD-1Y-OIS".to_string(),
                currency: "USD".to_string(),
                tenor: "1Y".to_string(),
                rate_type: "Ois".to_string(),
                value: 0.0425,
                quote_type: "Mid".to_string(),
                timestamp: 1700000000000,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: Some("SOFR".to_string()),
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"rateIndex\":\"SOFR\""));
        }

        #[test]
        fn test_market_rate_response_skips_none_rate_index() {
            let response = MarketRateResponse {
                id: "USD-5Y-SWAP".to_string(),
                currency: "USD".to_string(),
                tenor: "5Y".to_string(),
                rate_type: "Swap".to_string(),
                value: 0.04,
                quote_type: "Mid".to_string(),
                timestamp: 1700000000000,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: None,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(!json.contains("rateIndex"));
        }
    }

    // =========================================================================
    // Task 1.1 Tests: MarketRatesListResponse
    // =========================================================================

    mod market_rates_list_response_tests {
        use super::*;

        #[test]
        fn test_market_rates_list_response_serialise() {
            let response = MarketRatesListResponse {
                rates: vec![],
                last_updated: 1700000000000,
                total_count: 0,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"lastUpdated\""));
            assert!(json.contains("\"totalCount\""));
        }

        #[test]
        fn test_market_rates_list_response_with_rates() {
            let rate = MarketRateResponse {
                id: "USD-3M-DEPO".to_string(),
                currency: "USD".to_string(),
                tenor: "3M".to_string(),
                rate_type: "Deposit".to_string(),
                value: 0.05,
                quote_type: "Mid".to_string(),
                timestamp: 1700000000000,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: None,
            };

            let response = MarketRatesListResponse {
                rates: vec![rate],
                last_updated: 1700000000000,
                total_count: 1,
            };

            let json = serde_json::to_string(&response).unwrap();
            let parsed: MarketRatesListResponse = serde_json::from_str(&json).unwrap();

            assert_eq!(parsed.rates.len(), 1);
            assert_eq!(parsed.total_count, 1);
        }
    }

    // =========================================================================
    // Task 1.1 Tests: MarketRateQuery
    // =========================================================================

    mod market_rate_query_tests {
        use super::*;

        #[test]
        fn test_market_rate_query_deserialise_empty() {
            let json = "{}";
            let query: MarketRateQuery = serde_json::from_str(json).unwrap();

            assert!(query.currency.is_none());
            assert!(query.rate_type.is_none());
            assert!(query.index.is_none());
        }

        #[test]
        fn test_market_rate_query_deserialise_with_filters() {
            let json = r#"{
                "currency": "USD",
                "rateType": "Swap",
                "index": "SOFR"
            }"#;

            let query: MarketRateQuery = serde_json::from_str(json).unwrap();

            assert_eq!(query.currency, Some("USD".to_string()));
            assert_eq!(query.rate_type, Some("Swap".to_string()));
            assert_eq!(query.index, Some("SOFR".to_string()));
        }

        #[test]
        fn test_market_rate_query_default() {
            let query = MarketRateQuery::default();

            assert!(query.currency.is_none());
            assert!(query.rate_type.is_none());
            assert!(query.index.is_none());
        }
    }

    // =========================================================================
    // Task 1.2 Tests: InstrumentResponse
    // =========================================================================

    mod instrument_response_tests {
        use super::*;

        #[test]
        fn test_instrument_response_serialise() {
            let mut params = HashMap::new();
            params.insert("notional".to_string(), serde_json::json!(1_000_000));

            let response = InstrumentResponse {
                instrument_type: "Deposit".to_string(),
                currency: "USD".to_string(),
                start_date: "2024-01-15".to_string(),
                end_date: "2024-04-15".to_string(),
                rate: 0.05,
                parameters: params,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"instrumentType\""));
            assert!(json.contains("\"startDate\""));
            assert!(json.contains("\"endDate\""));
        }

        #[test]
        fn test_instrument_response_empty_parameters() {
            let response = InstrumentResponse {
                instrument_type: "ParSwap".to_string(),
                currency: "EUR".to_string(),
                start_date: "2024-01-15".to_string(),
                end_date: "2029-01-15".to_string(),
                rate: 0.03,
                parameters: HashMap::new(),
            };

            let json = serde_json::to_string(&response).unwrap();
            let parsed: InstrumentResponse = serde_json::from_str(&json).unwrap();

            assert!(parsed.parameters.is_empty());
        }
    }

    // =========================================================================
    // Task 1.2 Tests: ConventionResponse
    // =========================================================================

    mod convention_response_tests {
        use super::*;

        #[test]
        fn test_convention_field_serialise() {
            let field = ConventionField {
                label: "Day Count".to_string(),
                value: "ACT/360".to_string(),
            };

            let json = serde_json::to_string(&field).unwrap();
            assert!(json.contains("\"label\""));
            assert!(json.contains("\"value\""));
        }

        #[test]
        fn test_convention_response_serialise() {
            let response = ConventionResponse {
                convention_type: "SwapConvention".to_string(),
                fields: vec![
                    ConventionField {
                        label: "Fixed Leg Day Count".to_string(),
                        value: "30/360".to_string(),
                    },
                    ConventionField {
                        label: "Float Leg Day Count".to_string(),
                        value: "ACT/360".to_string(),
                    },
                ],
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"conventionType\""));
            assert!(json.contains("SwapConvention"));
        }
    }

    // =========================================================================
    // Task 1.2 Tests: MarketRateDetailResponse
    // =========================================================================

    mod market_rate_detail_response_tests {
        use super::*;

        #[test]
        fn test_market_rate_detail_response_full() {
            let rate = MarketRateResponse {
                id: "USD-5Y-SWAP".to_string(),
                currency: "USD".to_string(),
                tenor: "5Y".to_string(),
                rate_type: "Swap".to_string(),
                value: 0.04,
                quote_type: "Mid".to_string(),
                timestamp: 1700000000000,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: None,
            };

            let instrument = InstrumentResponse {
                instrument_type: "ParSwap".to_string(),
                currency: "USD".to_string(),
                start_date: "2024-01-15".to_string(),
                end_date: "2029-01-15".to_string(),
                rate: 0.04,
                parameters: HashMap::new(),
            };

            let convention = ConventionResponse {
                convention_type: "SwapConvention".to_string(),
                fields: vec![ConventionField {
                    label: "Day Count".to_string(),
                    value: "30/360".to_string(),
                }],
            };

            let response = MarketRateDetailResponse {
                rate,
                instrument: Some(instrument),
                convention: Some(convention),
            };

            let json = serde_json::to_string(&response).unwrap();
            let parsed: MarketRateDetailResponse = serde_json::from_str(&json).unwrap();

            assert!(parsed.instrument.is_some());
            assert!(parsed.convention.is_some());
        }

        #[test]
        fn test_market_rate_detail_response_no_instrument() {
            let rate = MarketRateResponse {
                id: "USD-USDJPY-FXSPOT".to_string(),
                currency: "USD".to_string(),
                tenor: "SPOT".to_string(),
                rate_type: "FxSpot".to_string(),
                value: 150.0,
                quote_type: "Mid".to_string(),
                timestamp: 1700000000000,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: None,
            };

            let response = MarketRateDetailResponse {
                rate,
                instrument: None,
                convention: None,
            };

            let json = serde_json::to_string(&response).unwrap();
            let parsed: MarketRateDetailResponse = serde_json::from_str(&json).unwrap();

            assert!(parsed.instrument.is_none());
            assert!(parsed.convention.is_none());
        }
    }

    // =========================================================================
    // Task 1.2 Tests: ConventionSummary and ConventionsListResponse
    // =========================================================================

    mod conventions_list_response_tests {
        use super::*;

        #[test]
        fn test_convention_summary_serialise() {
            let summary = ConventionSummary {
                id: "USD-SOFR-SWAP".to_string(),
                currency: "USD".to_string(),
                convention_type: "SwapConvention".to_string(),
                is_default: true,
            };

            let json = serde_json::to_string(&summary).unwrap();
            assert!(json.contains("\"isDefault\""));
            assert!(json.contains("\"conventionType\""));
        }

        #[test]
        fn test_conventions_list_response_serialise() {
            let response = ConventionsListResponse {
                conventions: vec![
                    ConventionSummary {
                        id: "USD-SOFR-SWAP".to_string(),
                        currency: "USD".to_string(),
                        convention_type: "SwapConvention".to_string(),
                        is_default: true,
                    },
                    ConventionSummary {
                        id: "EUR-EURIBOR-SWAP".to_string(),
                        currency: "EUR".to_string(),
                        convention_type: "SwapConvention".to_string(),
                        is_default: true,
                    },
                ],
            };

            let json = serde_json::to_string(&response).unwrap();
            let parsed: ConventionsListResponse = serde_json::from_str(&json).unwrap();

            assert_eq!(parsed.conventions.len(), 2);
        }
    }

    // =========================================================================
    // Task 1.3 Tests: MarketDataApiError
    // =========================================================================

    mod api_error_tests {
        use super::*;

        #[test]
        fn test_api_error_not_found() {
            let error = MarketDataApiError::not_found("Rate", "USD-99Y-SWAP");

            assert_eq!(error.error, "not_found");
            assert!(error.message.contains("Rate"));
            assert!(error.message.contains("USD-99Y-SWAP"));
            assert!(error.details.is_none());
        }

        #[test]
        fn test_api_error_invalid_parameter() {
            let error = MarketDataApiError::invalid_parameter("currency", "must be 3 characters");

            assert_eq!(error.error, "invalid_parameter");
            assert!(error.message.contains("currency"));
            assert!(error.message.contains("must be 3 characters"));
        }

        #[test]
        fn test_api_error_internal() {
            let error = MarketDataApiError::internal("Database connection failed");

            assert_eq!(error.error, "internal_error");
            assert_eq!(error.message, "Database connection failed");
        }

        #[test]
        fn test_api_error_serialise_camel_case() {
            let error = MarketDataApiError::not_found("Rate", "TEST-001");

            let json = serde_json::to_string(&error).unwrap();
            // All field names should be lowercase in this case
            assert!(json.contains("\"error\""));
            assert!(json.contains("\"message\""));
        }

        #[test]
        fn test_api_error_skips_none_details() {
            let error = MarketDataApiError::not_found("Rate", "TEST-001");

            let json = serde_json::to_string(&error).unwrap();
            assert!(!json.contains("details"));
        }

        #[test]
        fn test_api_error_with_details() {
            let error = MarketDataApiError {
                error: "validation_error".to_string(),
                message: "Validation failed".to_string(),
                details: Some("Field 'value' must be numeric".to_string()),
            };

            let json = serde_json::to_string(&error).unwrap();
            assert!(json.contains("\"details\""));
        }
    }
}
