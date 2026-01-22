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

// =============================================================================
// Task 1.1: TradeInstrumentType Enum
// =============================================================================

/// Trade instrument type for trade expansion requests.
///
/// Represents all supported instrument types organised by asset class:
/// - Rates: Deposit, FRA, Futures, ParSwap, OIS, BasisSwap, IRS
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
    /// Par swap (ATM swap)
    ParSwap,
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
            Self::Deposit
            | Self::Fra
            | Self::Futures
            | Self::ParSwap
            | Self::Ois
            | Self::BasisSwap
            | Self::Irs => AssetClass::Rates,

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
            Self::ParSwap,
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
            Self::ParSwap => "Par Swap",
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

/// Asset class categorisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    /// Interest rates
    Rates,
    /// Foreign exchange
    Fx,
    /// Equity
    Equity,
    /// Credit
    Credit,
    /// Commodity
    Commodity,
}

impl AssetClass {
    /// Returns the display name for this asset class.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Rates => "Rates",
            Self::Fx => "FX",
            Self::Equity => "Equity",
            Self::Credit => "Credit",
            Self::Commodity => "Commodity",
        }
    }
}

// =============================================================================
// Task 1.2: Instrument Parameter Types
// =============================================================================

/// Rates instrument parameters (Deposit, FRA, Futures, ParSwap, OIS).
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
    /// Cashflows in this leg
    pub cashflows: Vec<CashflowDto>,
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 1.1: TradeInstrumentType Tests
    // =========================================================================

    mod trade_instrument_type_tests {
        use super::*;

        #[test]
        fn test_all_instrument_types_exist() {
            let all = TradeInstrumentType::all();

            // Rates (7)
            assert!(all.contains(&TradeInstrumentType::Deposit));
            assert!(all.contains(&TradeInstrumentType::Fra));
            assert!(all.contains(&TradeInstrumentType::Futures));
            assert!(all.contains(&TradeInstrumentType::ParSwap));
            assert!(all.contains(&TradeInstrumentType::Ois));
            assert!(all.contains(&TradeInstrumentType::BasisSwap));
            assert!(all.contains(&TradeInstrumentType::Irs));

            // FX (3)
            assert!(all.contains(&TradeInstrumentType::FxForward));
            assert!(all.contains(&TradeInstrumentType::FxOption));
            assert!(all.contains(&TradeInstrumentType::CrossCurrencySwap));

            // Equity (2)
            assert!(all.contains(&TradeInstrumentType::EquityVanillaOption));
            assert!(all.contains(&TradeInstrumentType::EquityForward));

            // Credit (1)
            assert!(all.contains(&TradeInstrumentType::Cds));

            // Commodity (1)
            assert!(all.contains(&TradeInstrumentType::CommodityForward));
        }

        #[test]
        fn test_asset_class_categorisation() {
            // Rates instruments
            assert_eq!(
                TradeInstrumentType::Deposit.asset_class(),
                AssetClass::Rates
            );
            assert_eq!(TradeInstrumentType::Fra.asset_class(), AssetClass::Rates);
            assert_eq!(TradeInstrumentType::Irs.asset_class(), AssetClass::Rates);

            // FX instruments
            assert_eq!(TradeInstrumentType::FxForward.asset_class(), AssetClass::Fx);
            assert_eq!(TradeInstrumentType::FxOption.asset_class(), AssetClass::Fx);

            // Equity instruments
            assert_eq!(
                TradeInstrumentType::EquityVanillaOption.asset_class(),
                AssetClass::Equity
            );

            // Credit instruments
            assert_eq!(TradeInstrumentType::Cds.asset_class(), AssetClass::Credit);

            // Commodity instruments
            assert_eq!(
                TradeInstrumentType::CommodityForward.asset_class(),
                AssetClass::Commodity
            );
        }

        #[test]
        fn test_is_rates() {
            assert!(TradeInstrumentType::Deposit.is_rates());
            assert!(TradeInstrumentType::Irs.is_rates());
            assert!(!TradeInstrumentType::FxForward.is_rates());
        }

        #[test]
        fn test_is_fx() {
            assert!(TradeInstrumentType::FxForward.is_fx());
            assert!(TradeInstrumentType::FxOption.is_fx());
            assert!(!TradeInstrumentType::Deposit.is_fx());
        }

        #[test]
        fn test_is_equity() {
            assert!(TradeInstrumentType::EquityVanillaOption.is_equity());
            assert!(TradeInstrumentType::EquityForward.is_equity());
            assert!(!TradeInstrumentType::Irs.is_equity());
        }

        #[test]
        fn test_display_name() {
            assert_eq!(TradeInstrumentType::Deposit.display_name(), "Deposit");
            assert_eq!(TradeInstrumentType::Fra.display_name(), "FRA");
            assert_eq!(TradeInstrumentType::Irs.display_name(), "IRS");
            assert_eq!(TradeInstrumentType::FxForward.display_name(), "FX Forward");
            assert_eq!(
                TradeInstrumentType::EquityVanillaOption.display_name(),
                "Equity Vanilla Option"
            );
        }

        #[test]
        fn test_serde_serialisation() {
            let instrument = TradeInstrumentType::Deposit;
            let json = serde_json::to_string(&instrument).unwrap();
            assert_eq!(json, "\"deposit\"");

            let instrument = TradeInstrumentType::FxForward;
            let json = serde_json::to_string(&instrument).unwrap();
            assert_eq!(json, "\"fx_forward\"");
        }

        #[test]
        fn test_serde_deserialisation() {
            let instrument: TradeInstrumentType = serde_json::from_str("\"deposit\"").unwrap();
            assert_eq!(instrument, TradeInstrumentType::Deposit);

            let instrument: TradeInstrumentType = serde_json::from_str("\"fx_forward\"").unwrap();
            assert_eq!(instrument, TradeInstrumentType::FxForward);
        }
    }

    // =========================================================================
    // Task 1.2: Instrument Parameter Tests
    // =========================================================================

    mod instrument_params_tests {
        use super::*;

        #[test]
        fn test_rates_params_serialisation() {
            let params = RatesParams {
                currency: "USD".to_string(),
                start_date: "2024-01-15".to_string(),
                tenor: "3M".to_string(),
                rate: 0.05,
                notional: 1_000_000.0,
            };

            let json = serde_json::to_string(&params).unwrap();
            assert!(json.contains("\"currency\":\"USD\""));
            assert!(json.contains("\"startDate\":\"2024-01-15\""));
            assert!(json.contains("\"tenor\":\"3M\""));
        }

        #[test]
        fn test_swap_params_serialisation() {
            let params = SwapParams {
                currency: "EUR".to_string(),
                start_date: "2024-01-15".to_string(),
                tenor: "5Y".to_string(),
                notional: 10_000_000.0,
                fixed_rate: Some(0.03),
                spread: None,
                payment_frequency: "SemiAnnual".to_string(),
                day_count: "Act360".to_string(),
            };

            let json = serde_json::to_string(&params).unwrap();
            assert!(json.contains("\"fixedRate\":0.03"));
            assert!(json.contains("\"paymentFrequency\":\"SemiAnnual\""));
        }

        #[test]
        fn test_fx_params_serialisation() {
            let params = FxParams {
                base_currency: "EUR".to_string(),
                quote_currency: "USD".to_string(),
                spot_rate: 1.10,
                forward_rate: Some(1.12),
                strike: None,
                expiry: "2024-07-15".to_string(),
                notional: 1_000_000.0,
                option_type: None,
            };

            let json = serde_json::to_string(&params).unwrap();
            assert!(json.contains("\"baseCurrency\":\"EUR\""));
            assert!(json.contains("\"quoteCurrency\":\"USD\""));
            assert!(json.contains("\"forwardRate\":1.12"));
        }

        #[test]
        fn test_equity_params_serialisation() {
            let params = EquityParams {
                underlying: "AAPL".to_string(),
                spot_price: 175.0,
                strike: 180.0,
                expiry: "2024-12-20".to_string(),
                volatility: 0.25,
                risk_free_rate: 0.05,
                option_type: Some("call".to_string()),
                direction: None,
            };

            let json = serde_json::to_string(&params).unwrap();
            assert!(json.contains("\"underlying\":\"AAPL\""));
            assert!(json.contains("\"spotPrice\":175"));
            assert!(json.contains("\"optionType\":\"call\""));
        }

        #[test]
        fn test_instrument_params_union_tagged_serialisation() {
            let params = InstrumentParamsUnion::Rates(RatesParams {
                currency: "USD".to_string(),
                start_date: "2024-01-15".to_string(),
                tenor: "3M".to_string(),
                rate: 0.05,
                notional: 1_000_000.0,
            });

            let json = serde_json::to_string(&params).unwrap();
            assert!(json.contains("\"type\":\"rates\""));
        }

        #[test]
        fn test_instrument_params_union_deserialisation() {
            let json = r#"{"type":"rates","currency":"USD","startDate":"2024-01-15","tenor":"3M","rate":0.05,"notional":1000000}"#;
            let params: InstrumentParamsUnion = serde_json::from_str(json).unwrap();

            match params {
                InstrumentParamsUnion::Rates(p) => {
                    assert_eq!(p.currency, "USD");
                    assert_eq!(p.tenor, "3M");
                }
                _ => panic!("Expected Rates variant"),
            }
        }
    }

    // =========================================================================
    // Task 1.3: Request/Response Tests
    // =========================================================================

    mod request_response_tests {
        use super::*;

        #[test]
        fn test_trade_expand_request_deserialisation() {
            let json = r#"{
                "instrumentType": "deposit",
                "params": {
                    "type": "rates",
                    "currency": "USD",
                    "startDate": "2024-01-15",
                    "tenor": "3M",
                    "rate": 0.05,
                    "notional": 1000000
                }
            }"#;

            let request: TradeExpandRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.instrument_type, TradeInstrumentType::Deposit);
        }

        #[test]
        fn test_trade_expand_response_serialisation() {
            let response = TradeExpandResponse {
                trade_id: "T001".to_string(),
                trade_type: "Swap".to_string(),
                legs: vec![LegDto {
                    leg_number: 1,
                    direction: "Receiver".to_string(),
                    currency: "USD".to_string(),
                    leg_type: "Fixed".to_string(),
                    cashflows: vec![CashflowDto {
                        payment_date: "2024-07-15".to_string(),
                        accrual_start: "2024-01-15".to_string(),
                        accrual_end: "2024-07-15".to_string(),
                        year_fraction: 0.5,
                        notional: 1_000_000.0,
                        payoff_type: "Fixed".to_string(),
                        rate: Some(0.05),
                        spread: None,
                    }],
                }],
                metadata: TradeMetadataDto {
                    total_legs: 1,
                    total_cashflows: 1,
                    processing_time_ms: 5.0,
                },
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"tradeId\":\"T001\""));
            assert!(json.contains("\"tradeType\":\"Swap\""));
            assert!(json.contains("\"legNumber\":1"));
            assert!(json.contains("\"totalCashflows\":1"));
        }

        #[test]
        fn test_leg_dto_serialisation() {
            let leg = LegDto {
                leg_number: 1,
                direction: "Payer".to_string(),
                currency: "EUR".to_string(),
                leg_type: "Floating".to_string(),
                cashflows: vec![],
            };

            let json = serde_json::to_string(&leg).unwrap();
            assert!(json.contains("\"legNumber\":1"));
            assert!(json.contains("\"direction\":\"Payer\""));
            assert!(json.contains("\"legType\":\"Floating\""));
        }

        #[test]
        fn test_cashflow_dto_serialisation() {
            let cf = CashflowDto {
                payment_date: "2024-07-15".to_string(),
                accrual_start: "2024-01-15".to_string(),
                accrual_end: "2024-07-15".to_string(),
                year_fraction: 0.5,
                notional: 1_000_000.0,
                payoff_type: "Fixed".to_string(),
                rate: Some(0.05),
                spread: None,
            };

            let json = serde_json::to_string(&cf).unwrap();
            assert!(json.contains("\"paymentDate\":\"2024-07-15\""));
            assert!(json.contains("\"yearFraction\":0.5"));
            assert!(json.contains("\"payoffType\":\"Fixed\""));
        }
    }

    // =========================================================================
    // Error Response Tests
    // =========================================================================

    mod error_tests {
        use super::*;

        #[test]
        fn test_validation_error() {
            let error = TradeExpandError::validation("tenor", "Invalid tenor format");
            assert_eq!(error.error, "invalid_parameter");
            assert_eq!(error.field, Some("tenor".to_string()));
        }

        #[test]
        fn test_unsupported_instrument_error() {
            let error = TradeExpandError::unsupported_instrument(TradeInstrumentType::Cds);
            assert_eq!(error.error, "unsupported_instrument");
            assert!(error.message.contains("Cds"));
        }

        #[test]
        fn test_schedule_error() {
            let error = TradeExpandError::schedule_error("Failed to generate schedule");
            assert_eq!(error.error, "schedule_error");
        }

        #[test]
        fn test_internal_error() {
            let error = TradeExpandError::internal("Unexpected error");
            assert_eq!(error.error, "internal_error");
        }
    }
}
