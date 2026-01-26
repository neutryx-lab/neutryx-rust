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
