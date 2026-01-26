//! Trade expansion API types for the FrictionalBank WebApp.
//!
//! This module defines request/response types for the trade expansion API
//! endpoint. All types support JSON serialisation with camelCase field names
//! for JavaScript interoperability.
//!
//! # Task Coverage
//!
//! - Task 1.1: TradeInstrumentType enum
//! - Task 1.2: Instrument parameter types (RatesParams, SwapParams, FxParams,
//!   EquityParams)
//! - Task 1.3: TradeExpandRequest/Response types
//!
//! # Requirements Coverage
//!
//! - Requirement 1.1: Instrument セレクタの拡張
//! - Requirement 2.1, 2.2, 2.3, 2.4: Instrument別入力フォーム
//! - Requirement 3.3, 5.2, 5.3: Trade展開リクエスト/レスポンス型

use serde::{Deserialize, Serialize};

use infra_master::market::{Currency, RateIndex};

// =============================================================================
// Task 4.1: Rate Index Validation (using infra_master::RateIndex)
// =============================================================================

/// Validates a rate index string.
///
/// Returns `Ok(())` if the rate index is valid or None, `Err` otherwise.
/// Uses `RateIndex::all_codes()` from infra_master.
pub fn validate_rate_index(rate_index: &Option<String>) -> Result<(), String> {
    if let Some(idx) = rate_index {
        let normalised = idx.to_uppercase();
        let valid_codes = RateIndex::all_codes();
        if !valid_codes.contains(&normalised.as_str()) {
            return Err(format!(
                "Invalid rate_index '{}'. Supported values: {}",
                idx,
                valid_codes.join(", ")
            ));
        }
    }
    Ok(())
}

/// Returns the default rate index for a given currency.
///
/// Uses `RateIndex::default_for_currency()` from infra_master.
#[must_use]
pub fn default_rate_index_for_currency(currency: &str) -> &'static str {
    let currency_enum = currency.parse::<Currency>().unwrap_or(Currency::USD);
    RateIndex::default_for_currency(currency_enum).api_code()
}

// =============================================================================
// Task 1.1: TradeInstrumentType Enum
// =============================================================================

/// Trade instrument type for trade expansion requests.
///
/// Represents all supported instrument types organised by asset class:
/// - Rates: Deposit, FRA, Futures, OIS, BasisSwap, IRS
/// - FX: FxForward, FxOption, CrossCurrencySwap
/// - Equity: EquityVanillaOption, EquityForward
/// - Credit: CDS (placeholder)
/// - Commodity: CommodityForward (placeholder)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeInstrumentType {
    // Rates instruments
    /// Money market deposit
    Deposit,
    /// Forward Rate Agreement
    Fra,
    /// Interest rate futures
    Futures,
    /// Overnight Index Swap
    Ois,
    /// Basis swap (two floating legs)
    BasisSwap,
    /// Interest Rate Swap (fixed vs floating)
    Irs,

    // FX instruments
    /// FX forward
    FxForward,
    /// FX option
    FxOption,
    /// Cross-currency swap
    CrossCurrencySwap,

    // Equity instruments
    /// Equity vanilla option
    EquityVanillaOption,
    /// Equity forward
    EquityForward,

    // Credit instruments (placeholder)
    /// Credit Default Swap
    Cds,

    // Commodity instruments (placeholder)
    /// Commodity forward
    CommodityForward,
}

impl TradeInstrumentType {
    /// Returns the asset class for this instrument type.
    #[must_use]
    pub fn asset_class(&self) -> AssetClass {
        match self {
            Self::Deposit | Self::Fra | Self::Futures | Self::Ois | Self::BasisSwap | Self::Irs => {
                AssetClass::Rates
            }

            Self::FxForward | Self::FxOption | Self::CrossCurrencySwap => AssetClass::Fx,

            Self::EquityVanillaOption | Self::EquityForward => AssetClass::Equity,

            Self::Cds => AssetClass::Credit,

            Self::CommodityForward => AssetClass::Commodity,
        }
    }

    /// Returns true if this is a Rates instrument.
    #[must_use]
    pub fn is_rates(&self) -> bool { matches!(self.asset_class(), AssetClass::Rates) }

    /// Returns true if this is an FX instrument.
    #[must_use]
    pub fn is_fx(&self) -> bool { matches!(self.asset_class(), AssetClass::Fx) }

    /// Returns true if this is an Equity instrument.
    #[must_use]
    pub fn is_equity(&self) -> bool { matches!(self.asset_class(), AssetClass::Equity) }

    /// Returns all supported instrument types.
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![
            // Rates
            Self::Deposit,
            Self::Fra,
            Self::Futures,
            Self::Ois,
            Self::BasisSwap,
            Self::Irs,
            // FX
            Self::FxForward,
            Self::FxOption,
            Self::CrossCurrencySwap,
            // Equity
            Self::EquityVanillaOption,
            Self::EquityForward,
            // Credit (placeholder)
            Self::Cds,
            // Commodity (placeholder)
            Self::CommodityForward,
        ]
    }

    /// Returns the display name for this instrument type.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Deposit => "Deposit",
            Self::Fra => "FRA",
            Self::Futures => "Futures",
            Self::Ois => "OIS",
            Self::BasisSwap => "Basis Swap",
            Self::Irs => "IRS",
            Self::FxForward => "FX Forward",
            Self::FxOption => "FX Option",
            Self::CrossCurrencySwap => "Cross Currency Swap",
            Self::EquityVanillaOption => "Equity Vanilla Option",
            Self::EquityForward => "Equity Forward",
            Self::Cds => "CDS",
            Self::CommodityForward => "Commodity Forward",
        }
    }
}

/// Asset class categorisation (re-exported from infra_master).
pub use infra_master::trade::AssetClass;

// =============================================================================
// Task 1.2: Instrument Parameter Types
// =============================================================================

/// Rates instrument parameters (Deposit, FRA, Futures, OIS).
///
/// Basic parameters for simple rates instruments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RatesParams {
    /// Currency code (e.g., "USD", "EUR")
    pub currency: String,
    /// Start date (ISO 8601: "2024-01-15")
    pub start_date: String,
    /// Tenor (e.g., "3M", "1Y", "5Y")
    pub tenor: String,
    /// Rate or price
    pub rate: f64,
    /// Notional principal amount
    pub notional: f64,
    /// Rate index for floating leg (e.g., "SOFR", "EURIBOR3M", "SONIA")
    /// If not specified, defaults to currency-appropriate index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
}

/// Swap instrument parameters (IRS, BasisSwap).
///
/// Extended parameters for swap instruments with leg details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapParams {
    /// Currency code
    pub currency: String,
    /// Start date (ISO 8601)
    pub start_date: String,
    /// Tenor (e.g., "5Y", "10Y")
    pub tenor: String,
    /// Notional principal amount
    pub notional: f64,
    /// Fixed rate (for IRS)
    pub fixed_rate: Option<f64>,
    /// Spread (for BasisSwap)
    pub spread: Option<f64>,
    /// Payment frequency (e.g., "Quarterly", "SemiAnnual")
    pub payment_frequency: String,
    /// Day count convention (e.g., "Act360", "Thirty360")
    pub day_count: String,
    /// Rate index for floating leg (e.g., "SOFR", "EURIBOR3M", "SONIA")
    /// If not specified, defaults to currency-appropriate index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
}

/// FX instrument parameters (FxForward, FxOption, CrossCurrencySwap).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxParams {
    /// Base currency (e.g., "EUR")
    pub base_currency: String,
    /// Quote currency (e.g., "USD")
    pub quote_currency: String,
    /// Spot exchange rate
    pub spot_rate: f64,
    /// Forward rate (for Forward)
    pub forward_rate: Option<f64>,
    /// Strike (for Option)
    pub strike: Option<f64>,
    /// Expiry date (ISO 8601)
    pub expiry: String,
    /// Notional amount
    pub notional: f64,
    /// Option type: "call" or "put" (for FxOption)
    pub option_type: Option<String>,
}

/// Equity instrument parameters (VanillaOption, Forward).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityParams {
    /// Underlying asset ticker (e.g., "AAPL")
    pub underlying: String,
    /// Current spot price
    pub spot_price: f64,
    /// Strike price
    pub strike: f64,
    /// Expiry date (ISO 8601)
    pub expiry: String,
    /// Annualised volatility (e.g., 0.2 for 20%)
    pub volatility: f64,
    /// Risk-free interest rate
    pub risk_free_rate: f64,
    /// Option type: "call" or "put" (for VanillaOption)
    pub option_type: Option<String>,
    /// Direction: "long" or "short" (for Forward)
    pub direction: Option<String>,
}

/// Instrument parameters union (tagged union).
///
/// Uses `#[serde(tag = "type")]` for discriminated JSON parsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstrumentParamsUnion {
    /// Rates instrument parameters
    Rates(RatesParams),
    /// Swap instrument parameters
    Swap(SwapParams),
    /// FX instrument parameters
    Fx(FxParams),
    /// Equity instrument parameters
    Equity(EquityParams),
}

// =============================================================================
// Task 1.3: Trade Expand Request/Response Types
// =============================================================================

/// Trade expansion request.
///
/// Sent by the client to expand an instrument into a Trade with cashflows.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeExpandRequest {
    /// Type of instrument to expand
    pub instrument_type: TradeInstrumentType,
    /// Instrument-specific parameters
    pub params: InstrumentParamsUnion,
}

/// Trade expansion response.
///
/// Returned after successful trade expansion.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeExpandResponse {
    /// Generated trade ID
    pub trade_id: String,
    /// Trade type (e.g., "Swap", "Forward", "Option")
    pub trade_type: String,
    /// Legs comprising the trade
    pub legs: Vec<LegDto>,
    /// Additional metadata
    pub metadata: TradeMetadataDto,
}

/// Leg DTO for API response.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegDto {
    /// Leg number (1-indexed)
    pub leg_number: usize,
    /// Direction: "Payer" or "Receiver"
    pub direction: String,
    /// Currency code
    pub currency: String,
    /// Leg type: "Fixed", "Floating", etc.
    pub leg_type: String,
    /// Rate index for floating legs (e.g., "SOFR", "EURIBOR3M")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
    /// Cashflows in this leg
    pub cashflows: Vec<CashflowDto>,
}

/// Daily accrual detail for OIS compounding.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyAccrualDto {
    /// Date of this accrual (ISO 8601)
    pub date: String,
    /// Overnight rate for this day
    pub overnight_rate: f64,
    /// Day count fraction (typically 1/360 or 1/365)
    pub day_fraction: f64,
    /// Compounded notional up to this date
    pub compounded_notional: f64,
}

/// Cashflow DTO for API response.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashflowDto {
    /// Payment date (ISO 8601)
    pub payment_date: String,
    /// Accrual period start (ISO 8601)
    pub accrual_start: String,
    /// Accrual period end (ISO 8601)
    pub accrual_end: String,
    /// Year fraction for the period
    pub year_fraction: f64,
    /// Notional amount
    pub notional: f64,
    /// Payoff type: "Fixed", "Linear", "VanillaOption", "Digital"
    pub payoff_type: String,
    /// Fixed rate (if applicable)
    pub rate: Option<f64>,
    /// Spread (if applicable)
    pub spread: Option<f64>,
    /// Rate index for Linear/VanillaOption payoffs (e.g., "SOFR", "EURIBOR3M")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
    /// Daily accrual details for OIS floating leg (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_accruals: Option<Vec<DailyAccrualDto>>,
}

/// Trade metadata DTO for API response.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeMetadataDto {
    /// Total number of legs
    pub total_legs: usize,
    /// Total number of cashflows
    pub total_cashflows: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
}

// =============================================================================
// Task 4.2: Instruments API Response Types
// =============================================================================

/// Parameter field metadata for dynamic form generation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterFieldMeta {
    /// Field name
    pub name: String,
    /// Display label
    pub label: String,
    /// Field type: "string", "number", "date", "select"
    pub field_type: String,
    /// Whether this field is required
    pub required: bool,
    /// Default value (if any)
    pub default_value: Option<serde_json::Value>,
    /// Validation rules (if any)
    pub validation: Option<ValidationRules>,
    /// Options for select fields
    pub options: Option<Vec<SelectOption>>,
}

/// Validation rules for a parameter field.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRules {
    /// Minimum value (for numbers)
    pub min: Option<f64>,
    /// Maximum value (for numbers)
    pub max: Option<f64>,
    /// Pattern (for strings)
    pub pattern: Option<String>,
}

/// Select option for dropdown fields.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectOption {
    /// Option value
    pub value: String,
    /// Display label
    pub label: String,
}

/// Instrument metadata for the instruments list API.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentMeta {
    /// Instrument type identifier
    pub instrument_type: TradeInstrumentType,
    /// Display name
    pub display_name: String,
    /// Asset class
    pub asset_class: AssetClass,
    /// Asset class display name
    pub asset_class_name: String,
    /// Required parameters
    pub required_params: Vec<ParameterFieldMeta>,
    /// Optional parameters
    pub optional_params: Vec<ParameterFieldMeta>,
}

/// Response for GET /api/instruments endpoint.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentsResponse {
    /// List of available instruments
    pub instruments: Vec<InstrumentMeta>,
}

// =============================================================================
// Error Response Types
// =============================================================================

/// Error response for trade expansion failures.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeExpandError {
    /// Error code
    pub error: String,
    /// Error message
    pub message: String,
    /// Field that caused the error (if applicable)
    pub field: Option<String>,
}

impl TradeExpandError {
    /// Creates a new validation error.
    pub fn validation(field: &str, message: &str) -> Self {
        Self {
            error: "invalid_parameter".to_string(),
            message: message.to_string(),
            field: Some(field.to_string()),
        }
    }

    /// Creates a new unsupported instrument error.
    pub fn unsupported_instrument(instrument_type: TradeInstrumentType) -> Self {
        Self {
            error: "unsupported_instrument".to_string(),
            message: format!(
                "Instrument type '{:?}' is not yet supported",
                instrument_type
            ),
            field: None,
        }
    }

    /// Creates a new schedule generation error.
    pub fn schedule_error(message: &str) -> Self {
        Self {
            error: "schedule_error".to_string(),
            message: message.to_string(),
            field: None,
        }
    }

    /// Creates a new internal error.
    pub fn internal(message: &str) -> Self {
        Self {
            error: "internal_error".to_string(),
            message: message.to_string(),
            field: None,
        }
    }
}
