//! Pricer API types for the FrictionalBank WebApp.
//!
//! This module defines request/response types for the pricing API endpoint.
//! All types support JSON serialisation with camelCase field names for
//! JavaScript interoperability.
//!
//! # Task Coverage
//!
//! - Task 1.1: プライサーAPI用リクエスト/レスポンス型の定義
//!
//! # Requirements Coverage
//!
//! - Requirement 2.1, 2.2, 2.3: 商品固有パラメータ型
//! - Requirement 3.2, 3.5: 価格計算レスポンス型
//! - Requirement 4.1, 4.2: Greeks結果型

use serde::{Deserialize, Serialize};

// =============================================================================
// Instrument Type Enum
// =============================================================================

/// Instrument type for pricing requests.
///
/// Represents the type of derivative instrument to price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentType {
    /// Equity vanilla option (European style)
    EquityVanillaOption,
    /// FX option (Garman-Kohlhagen model)
    FxOption,
    /// Interest Rate Swap
    Irs,
}

// =============================================================================
// Option Type Enum
// =============================================================================

/// Option type (Call or Put).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptionType {
    /// Call option (right to buy)
    Call,
    /// Put option (right to sell)
    Put,
}

// =============================================================================
// Instrument Parameters
// =============================================================================

/// Equity vanilla option parameters.
///
/// Parameters required to price a European-style equity option
/// using the Black-Scholes model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityOptionParams {
    /// Current spot price of the underlying
    pub spot: f64,
    /// Strike price of the option
    pub strike: f64,
    /// Time to expiry in years
    pub expiry_years: f64,
    /// Annualised volatility (e.g., 0.2 for 20%)
    pub volatility: f64,
    /// Risk-free interest rate (e.g., 0.05 for 5%)
    pub rate: f64,
    /// Option type (Call or Put)
    pub option_type: OptionType,
}

/// FX option parameters.
///
/// Parameters required to price an FX option using the
/// Garman-Kohlhagen model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxOptionParams {
    /// Current spot exchange rate
    pub spot: f64,
    /// Strike exchange rate
    pub strike: f64,
    /// Time to expiry in years
    pub expiry_years: f64,
    /// Domestic risk-free interest rate
    pub domestic_rate: f64,
    /// Foreign risk-free interest rate
    pub foreign_rate: f64,
    /// Annualised volatility
    pub volatility: f64,
    /// Option type (Call or Put)
    pub option_type: OptionType,
}

/// Interest Rate Swap parameters.
///
/// Parameters required to price an IRS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrsParams {
    /// Notional principal amount
    pub notional: f64,
    /// Fixed leg rate (e.g., 0.03 for 3%)
    pub fixed_rate: f64,
    /// Swap tenor in years
    pub tenor_years: f64,
}

/// Instrument parameters (discriminated union).
///
/// Uses `#[serde(untagged)]` for flexible JSON parsing where the
/// structure determines the variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InstrumentParams {
    /// Equity option parameters
    EquityOption(EquityOptionParams),
    /// FX option parameters
    FxOption(FxOptionParams),
    /// IRS parameters
    Irs(IrsParams),
}

// =============================================================================
// Market Data Configuration
// =============================================================================

/// Market data source configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketDataSource {
    /// Use demo/sample market data
    Demo,
    /// Use custom market data from request
    Custom,
}

/// Market data configuration for pricing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataConfig {
    /// Market data source
    pub source: MarketDataSource,
    /// Custom yield curve rate (for Custom source)
    pub curve_rate: Option<f64>,
    /// Custom volatility (for Custom source)
    pub volatility: Option<f64>,
}

// =============================================================================
// Demo Market Data (Task 1.2)
// =============================================================================

/// Default demo market data values.
///
/// Provides standard market data for demonstration purposes.
#[derive(Debug, Clone, PartialEq)]
pub struct DemoMarketData {
    /// Flat yield curve rate (5%)
    pub curve_rate: f64,
    /// Default volatility (20%)
    pub volatility: f64,
}

impl Default for DemoMarketData {
    fn default() -> Self {
        Self::new()
    }
}

impl DemoMarketData {
    /// Default flat curve rate (5% p.a.)
    pub const DEFAULT_CURVE_RATE: f64 = 0.05;
    /// Default volatility (20% annualised)
    pub const DEFAULT_VOLATILITY: f64 = 0.20;

    /// Create new demo market data with default values.
    pub fn new() -> Self {
        Self {
            curve_rate: Self::DEFAULT_CURVE_RATE,
            volatility: Self::DEFAULT_VOLATILITY,
        }
    }

    /// Create demo market data with custom curve rate.
    pub fn with_curve_rate(mut self, rate: f64) -> Self {
        self.curve_rate = rate;
        self
    }

    /// Create demo market data with custom volatility.
    pub fn with_volatility(mut self, vol: f64) -> Self {
        self.volatility = vol;
        self
    }

    /// Get the effective curve rate from market data config.
    ///
    /// Uses custom rate if provided and source is Custom,
    /// otherwise returns demo default.
    pub fn get_curve_rate(config: Option<&MarketDataConfig>) -> f64 {
        config
            .and_then(|c| {
                if c.source == MarketDataSource::Custom {
                    c.curve_rate
                } else {
                    None
                }
            })
            .unwrap_or(Self::DEFAULT_CURVE_RATE)
    }

    /// Get the effective volatility from market data config.
    ///
    /// Uses custom volatility if provided and source is Custom,
    /// otherwise returns demo default.
    pub fn get_volatility(config: Option<&MarketDataConfig>) -> f64 {
        config
            .and_then(|c| {
                if c.source == MarketDataSource::Custom {
                    c.volatility
                } else {
                    None
                }
            })
            .unwrap_or(Self::DEFAULT_VOLATILITY)
    }
}

// =============================================================================
// Pricing Request
// =============================================================================

/// Pricing calculation request.
///
/// Sent by the client to request a price calculation for an instrument.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingRequest {
    /// Type of instrument to price
    pub instrument_type: InstrumentType,
    /// Instrument-specific parameters
    pub params: InstrumentParams,
    /// Market data configuration (optional, defaults to Demo)
    pub market_data: Option<MarketDataConfig>,
    /// Whether to compute Greeks
    pub compute_greeks: bool,
}

// =============================================================================
// Greeks Data
// =============================================================================

/// Greeks calculation results.
///
/// Contains first-order sensitivities (Delta, Gamma, Vega, Theta, Rho).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GreeksData {
    /// Delta: sensitivity to underlying price
    pub delta: f64,
    /// Gamma: sensitivity of delta to underlying price
    pub gamma: f64,
    /// Vega: sensitivity to volatility (per 1% move)
    pub vega: f64,
    /// Theta: time decay (per day)
    pub theta: f64,
    /// Rho: sensitivity to interest rate (per 1% move)
    pub rho: f64,
}

// =============================================================================
// Pricing Response
// =============================================================================

/// Pricing calculation response.
///
/// Returned by the server after a successful price calculation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingResponse {
    /// Unique calculation ID (for history tracking)
    pub calculation_id: String,
    /// Type of instrument that was priced
    pub instrument_type: InstrumentType,
    /// Present value / price
    pub pv: f64,
    /// Greeks (if compute_greeks was true)
    pub greeks: Option<GreeksData>,
    /// Calculation timestamp (Unix epoch milliseconds)
    pub timestamp: i64,
}

// =============================================================================
// Error Response
// =============================================================================

/// Pricing error response.
///
/// Returned when a pricing request fails.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingErrorResponse {
    /// Error type identifier
    pub error_type: String,
    /// Human-readable error message
    pub message: String,
    /// Field that caused the error (for validation errors)
    pub field: Option<String>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // InstrumentType Serialisation Tests
    // =========================================================================

    mod instrument_type_tests {
        use super::*;

        #[test]
        fn test_serialize_equity_vanilla_option() {
            let t = InstrumentType::EquityVanillaOption;
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, "\"equity_vanilla_option\"");
        }

        #[test]
        fn test_serialize_fx_option() {
            let t = InstrumentType::FxOption;
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, "\"fx_option\"");
        }

        #[test]
        fn test_serialize_irs() {
            let t = InstrumentType::Irs;
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, "\"irs\"");
        }

        #[test]
        fn test_deserialize_instrument_types() {
            let eq: InstrumentType = serde_json::from_str("\"equity_vanilla_option\"").unwrap();
            assert_eq!(eq, InstrumentType::EquityVanillaOption);

            let fx: InstrumentType = serde_json::from_str("\"fx_option\"").unwrap();
            assert_eq!(fx, InstrumentType::FxOption);

            let irs: InstrumentType = serde_json::from_str("\"irs\"").unwrap();
            assert_eq!(irs, InstrumentType::Irs);
        }
    }

    // =========================================================================
    // OptionType Serialisation Tests
    // =========================================================================

    mod option_type_tests {
        use super::*;

        #[test]
        fn test_serialize_call() {
            let t = OptionType::Call;
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, "\"call\"");
        }

        #[test]
        fn test_serialize_put() {
            let t = OptionType::Put;
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, "\"put\"");
        }

        #[test]
        fn test_deserialize_option_types() {
            let call: OptionType = serde_json::from_str("\"call\"").unwrap();
            assert_eq!(call, OptionType::Call);

            let put: OptionType = serde_json::from_str("\"put\"").unwrap();
            assert_eq!(put, OptionType::Put);
        }
    }

    // =========================================================================
    // EquityOptionParams Tests
    // =========================================================================

    mod equity_option_params_tests {
        use super::*;

        fn sample_equity_params() -> EquityOptionParams {
            EquityOptionParams {
                spot: 100.0,
                strike: 105.0,
                expiry_years: 1.0,
                volatility: 0.2,
                rate: 0.05,
                option_type: OptionType::Call,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let params = sample_equity_params();
            let json = serde_json::to_string(&params).unwrap();

            assert!(json.contains("\"spot\":100"));
            assert!(json.contains("\"strike\":105"));
            assert!(json.contains("\"expiryYears\":1"));
            assert!(json.contains("\"volatility\":0.2"));
            assert!(json.contains("\"rate\":0.05"));
            assert!(json.contains("\"optionType\":\"call\""));
        }

        #[test]
        fn test_deserialize_camel_case() {
            let json = r#"{
                "spot": 100.0,
                "strike": 105.0,
                "expiryYears": 1.0,
                "volatility": 0.2,
                "rate": 0.05,
                "optionType": "call"
            }"#;

            let params: EquityOptionParams = serde_json::from_str(json).unwrap();
            assert!((params.spot - 100.0).abs() < 1e-10);
            assert!((params.strike - 105.0).abs() < 1e-10);
            assert!((params.expiry_years - 1.0).abs() < 1e-10);
            assert!((params.volatility - 0.2).abs() < 1e-10);
            assert!((params.rate - 0.05).abs() < 1e-10);
            assert_eq!(params.option_type, OptionType::Call);
        }
    }

    // =========================================================================
    // FxOptionParams Tests
    // =========================================================================

    mod fx_option_params_tests {
        use super::*;

        fn sample_fx_params() -> FxOptionParams {
            FxOptionParams {
                spot: 1.10,
                strike: 1.15,
                expiry_years: 0.5,
                domestic_rate: 0.05,
                foreign_rate: 0.02,
                volatility: 0.15,
                option_type: OptionType::Put,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let params = sample_fx_params();
            let json = serde_json::to_string(&params).unwrap();

            assert!(json.contains("\"spot\":1.1"));
            assert!(json.contains("\"strike\":1.15"));
            assert!(json.contains("\"expiryYears\":0.5"));
            assert!(json.contains("\"domesticRate\":0.05"));
            assert!(json.contains("\"foreignRate\":0.02"));
            assert!(json.contains("\"volatility\":0.15"));
            assert!(json.contains("\"optionType\":\"put\""));
        }

        #[test]
        fn test_deserialize_camel_case() {
            let json = r#"{
                "spot": 1.10,
                "strike": 1.15,
                "expiryYears": 0.5,
                "domesticRate": 0.05,
                "foreignRate": 0.02,
                "volatility": 0.15,
                "optionType": "put"
            }"#;

            let params: FxOptionParams = serde_json::from_str(json).unwrap();
            assert!((params.spot - 1.10).abs() < 1e-10);
            assert!((params.domestic_rate - 0.05).abs() < 1e-10);
            assert!((params.foreign_rate - 0.02).abs() < 1e-10);
            assert_eq!(params.option_type, OptionType::Put);
        }
    }

    // =========================================================================
    // IrsParams Tests
    // =========================================================================

    mod irs_params_tests {
        use super::*;

        fn sample_irs_params() -> IrsParams {
            IrsParams {
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let params = sample_irs_params();
            let json = serde_json::to_string(&params).unwrap();

            assert!(json.contains("\"notional\":10000000"));
            assert!(json.contains("\"fixedRate\":0.03"));
            assert!(json.contains("\"tenorYears\":5"));
        }

        #[test]
        fn test_deserialize_camel_case() {
            let json = r#"{
                "notional": 10000000.0,
                "fixedRate": 0.03,
                "tenorYears": 5.0
            }"#;

            let params: IrsParams = serde_json::from_str(json).unwrap();
            assert!((params.notional - 10_000_000.0).abs() < 1e-10);
            assert!((params.fixed_rate - 0.03).abs() < 1e-10);
            assert!((params.tenor_years - 5.0).abs() < 1e-10);
        }
    }

    // =========================================================================
    // InstrumentParams (Untagged Union) Tests
    // =========================================================================

    mod instrument_params_tests {
        use super::*;

        #[test]
        fn test_deserialize_equity_option() {
            let json = r#"{
                "spot": 100.0,
                "strike": 105.0,
                "expiryYears": 1.0,
                "volatility": 0.2,
                "rate": 0.05,
                "optionType": "call"
            }"#;

            let params: InstrumentParams = serde_json::from_str(json).unwrap();
            match params {
                InstrumentParams::EquityOption(p) => {
                    assert!((p.spot - 100.0).abs() < 1e-10);
                }
                _ => panic!("Expected EquityOption variant"),
            }
        }

        #[test]
        fn test_deserialize_fx_option() {
            let json = r#"{
                "spot": 1.10,
                "strike": 1.15,
                "expiryYears": 0.5,
                "domesticRate": 0.05,
                "foreignRate": 0.02,
                "volatility": 0.15,
                "optionType": "put"
            }"#;

            let params: InstrumentParams = serde_json::from_str(json).unwrap();
            match params {
                InstrumentParams::FxOption(p) => {
                    assert!((p.domestic_rate - 0.05).abs() < 1e-10);
                    assert!((p.foreign_rate - 0.02).abs() < 1e-10);
                }
                _ => panic!("Expected FxOption variant"),
            }
        }

        #[test]
        fn test_deserialize_irs() {
            let json = r#"{
                "notional": 10000000.0,
                "fixedRate": 0.03,
                "tenorYears": 5.0
            }"#;

            let params: InstrumentParams = serde_json::from_str(json).unwrap();
            match params {
                InstrumentParams::Irs(p) => {
                    assert!((p.notional - 10_000_000.0).abs() < 1e-10);
                }
                _ => panic!("Expected Irs variant"),
            }
        }
    }

    // =========================================================================
    // PricingRequest Tests
    // =========================================================================

    mod pricing_request_tests {
        use super::*;

        #[test]
        fn test_deserialize_full_request() {
            let json = r#"{
                "instrumentType": "equity_vanilla_option",
                "params": {
                    "spot": 100.0,
                    "strike": 105.0,
                    "expiryYears": 1.0,
                    "volatility": 0.2,
                    "rate": 0.05,
                    "optionType": "call"
                },
                "computeGreeks": true
            }"#;

            let request: PricingRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.instrument_type, InstrumentType::EquityVanillaOption);
            assert!(request.compute_greeks);
            assert!(request.market_data.is_none());
        }

        #[test]
        fn test_deserialize_with_market_data() {
            let json = r#"{
                "instrumentType": "fx_option",
                "params": {
                    "spot": 1.10,
                    "strike": 1.15,
                    "expiryYears": 0.5,
                    "domesticRate": 0.05,
                    "foreignRate": 0.02,
                    "volatility": 0.15,
                    "optionType": "put"
                },
                "marketData": {
                    "source": "demo"
                },
                "computeGreeks": false
            }"#;

            let request: PricingRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.instrument_type, InstrumentType::FxOption);
            assert!(!request.compute_greeks);
            assert!(request.market_data.is_some());
            assert_eq!(
                request.market_data.unwrap().source,
                MarketDataSource::Demo
            );
        }

        #[test]
        fn test_deserialize_irs_request() {
            let json = r#"{
                "instrumentType": "irs",
                "params": {
                    "notional": 10000000.0,
                    "fixedRate": 0.03,
                    "tenorYears": 5.0
                },
                "computeGreeks": true
            }"#;

            let request: PricingRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.instrument_type, InstrumentType::Irs);
            assert!(request.compute_greeks);
        }
    }

    // =========================================================================
    // GreeksData Tests
    // =========================================================================

    mod greeks_data_tests {
        use super::*;

        fn sample_greeks() -> GreeksData {
            GreeksData {
                delta: 0.55,
                gamma: 0.02,
                vega: 0.35,
                theta: -0.05,
                rho: 0.12,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let greeks = sample_greeks();
            let json = serde_json::to_string(&greeks).unwrap();

            assert!(json.contains("\"delta\":0.55"));
            assert!(json.contains("\"gamma\":0.02"));
            assert!(json.contains("\"vega\":0.35"));
            assert!(json.contains("\"theta\":-0.05"));
            assert!(json.contains("\"rho\":0.12"));
        }

        #[test]
        fn test_deserialize_greeks() {
            let json = r#"{
                "delta": 0.55,
                "gamma": 0.02,
                "vega": 0.35,
                "theta": -0.05,
                "rho": 0.12
            }"#;

            let greeks: GreeksData = serde_json::from_str(json).unwrap();
            assert!((greeks.delta - 0.55).abs() < 1e-10);
            assert!((greeks.theta - (-0.05)).abs() < 1e-10);
        }
    }

    // =========================================================================
    // PricingResponse Tests
    // =========================================================================

    mod pricing_response_tests {
        use super::*;

        #[test]
        fn test_serialize_without_greeks() {
            let response = PricingResponse {
                calculation_id: "calc-001".to_string(),
                instrument_type: InstrumentType::Irs,
                pv: 125_000.5,
                greeks: None,
                timestamp: 1700000000000,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"calculationId\":\"calc-001\""));
            assert!(json.contains("\"instrumentType\":\"irs\""));
            assert!(json.contains("\"pv\":125000.5"));
            assert!(json.contains("\"greeks\":null"));
            assert!(json.contains("\"timestamp\":1700000000000"));
        }

        #[test]
        fn test_serialize_with_greeks() {
            let greeks = GreeksData {
                delta: 0.55,
                gamma: 0.02,
                vega: 0.35,
                theta: -0.05,
                rho: 0.12,
            };

            let response = PricingResponse {
                calculation_id: "calc-002".to_string(),
                instrument_type: InstrumentType::EquityVanillaOption,
                pv: 10.25,
                greeks: Some(greeks),
                timestamp: 1700000000000,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"calculationId\":\"calc-002\""));
            assert!(json.contains("\"instrumentType\":\"equity_vanilla_option\""));
            assert!(json.contains("\"delta\":0.55"));
        }
    }

    // =========================================================================
    // PricingErrorResponse Tests
    // =========================================================================

    mod pricing_error_response_tests {
        use super::*;

        #[test]
        fn test_serialize_validation_error() {
            let error = PricingErrorResponse {
                error_type: "ValidationError".to_string(),
                message: "Strike must be positive".to_string(),
                field: Some("strike".to_string()),
            };

            let json = serde_json::to_string(&error).unwrap();
            assert!(json.contains("\"errorType\":\"ValidationError\""));
            assert!(json.contains("\"message\":\"Strike must be positive\""));
            assert!(json.contains("\"field\":\"strike\""));
        }

        #[test]
        fn test_serialize_pricing_error() {
            let error = PricingErrorResponse {
                error_type: "PricingError".to_string(),
                message: "Numerical instability in pricing".to_string(),
                field: None,
            };

            let json = serde_json::to_string(&error).unwrap();
            assert!(json.contains("\"errorType\":\"PricingError\""));
            assert!(json.contains("\"field\":null"));
        }
    }

    // =========================================================================
    // MarketDataConfig Tests
    // =========================================================================

    mod market_data_config_tests {
        use super::*;

        #[test]
        fn test_serialize_demo_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Demo,
                curve_rate: None,
                volatility: None,
            };

            let json = serde_json::to_string(&config).unwrap();
            assert!(json.contains("\"source\":\"demo\""));
        }

        #[test]
        fn test_serialize_custom_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Custom,
                curve_rate: Some(0.05),
                volatility: Some(0.2),
            };

            let json = serde_json::to_string(&config).unwrap();
            assert!(json.contains("\"source\":\"custom\""));
            assert!(json.contains("\"curveRate\":0.05"));
            assert!(json.contains("\"volatility\":0.2"));
        }

        #[test]
        fn test_deserialize_market_data_config() {
            let json = r#"{
                "source": "custom",
                "curveRate": 0.04,
                "volatility": 0.25
            }"#;

            let config: MarketDataConfig = serde_json::from_str(json).unwrap();
            assert_eq!(config.source, MarketDataSource::Custom);
            assert!((config.curve_rate.unwrap() - 0.04).abs() < 1e-10);
            assert!((config.volatility.unwrap() - 0.25).abs() < 1e-10);
        }
    }

    // =========================================================================
    // DemoMarketData Tests (Task 1.2)
    // =========================================================================

    mod demo_market_data_tests {
        use super::*;

        #[test]
        fn test_demo_market_data_new() {
            let data = DemoMarketData::new();
            assert!((data.curve_rate - 0.05).abs() < 1e-10);
            assert!((data.volatility - 0.20).abs() < 1e-10);
        }

        #[test]
        fn test_demo_market_data_default() {
            let data = DemoMarketData::default();
            assert_eq!(data, DemoMarketData::new());
        }

        #[test]
        fn test_demo_market_data_with_curve_rate() {
            let data = DemoMarketData::new().with_curve_rate(0.03);
            assert!((data.curve_rate - 0.03).abs() < 1e-10);
            assert!((data.volatility - 0.20).abs() < 1e-10);
        }

        #[test]
        fn test_demo_market_data_with_volatility() {
            let data = DemoMarketData::new().with_volatility(0.30);
            assert!((data.curve_rate - 0.05).abs() < 1e-10);
            assert!((data.volatility - 0.30).abs() < 1e-10);
        }

        #[test]
        fn test_demo_market_data_builder_chain() {
            let data = DemoMarketData::new()
                .with_curve_rate(0.04)
                .with_volatility(0.25);
            assert!((data.curve_rate - 0.04).abs() < 1e-10);
            assert!((data.volatility - 0.25).abs() < 1e-10);
        }

        #[test]
        fn test_get_curve_rate_none_config() {
            let rate = DemoMarketData::get_curve_rate(None);
            assert!((rate - 0.05).abs() < 1e-10);
        }

        #[test]
        fn test_get_curve_rate_demo_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Demo,
                curve_rate: Some(0.10),
                volatility: Some(0.40),
            };
            let rate = DemoMarketData::get_curve_rate(Some(&config));
            // Demo source should use default, ignoring custom values
            assert!((rate - 0.05).abs() < 1e-10);
        }

        #[test]
        fn test_get_curve_rate_custom_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Custom,
                curve_rate: Some(0.08),
                volatility: Some(0.30),
            };
            let rate = DemoMarketData::get_curve_rate(Some(&config));
            assert!((rate - 0.08).abs() < 1e-10);
        }

        #[test]
        fn test_get_curve_rate_custom_source_no_value() {
            let config = MarketDataConfig {
                source: MarketDataSource::Custom,
                curve_rate: None,
                volatility: Some(0.30),
            };
            let rate = DemoMarketData::get_curve_rate(Some(&config));
            // Falls back to default when Custom but no value provided
            assert!((rate - 0.05).abs() < 1e-10);
        }

        #[test]
        fn test_get_volatility_none_config() {
            let vol = DemoMarketData::get_volatility(None);
            assert!((vol - 0.20).abs() < 1e-10);
        }

        #[test]
        fn test_get_volatility_demo_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Demo,
                curve_rate: Some(0.10),
                volatility: Some(0.40),
            };
            let vol = DemoMarketData::get_volatility(Some(&config));
            // Demo source should use default, ignoring custom values
            assert!((vol - 0.20).abs() < 1e-10);
        }

        #[test]
        fn test_get_volatility_custom_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Custom,
                curve_rate: Some(0.08),
                volatility: Some(0.35),
            };
            let vol = DemoMarketData::get_volatility(Some(&config));
            assert!((vol - 0.35).abs() < 1e-10);
        }

        #[test]
        fn test_get_volatility_custom_source_no_value() {
            let config = MarketDataConfig {
                source: MarketDataSource::Custom,
                curve_rate: Some(0.08),
                volatility: None,
            };
            let vol = DemoMarketData::get_volatility(Some(&config));
            // Falls back to default when Custom but no value provided
            assert!((vol - 0.20).abs() < 1e-10);
        }

        #[test]
        fn test_constants() {
            assert!((DemoMarketData::DEFAULT_CURVE_RATE - 0.05).abs() < 1e-10);
            assert!((DemoMarketData::DEFAULT_VOLATILITY - 0.20).abs() < 1e-10);
        }
    }
}
