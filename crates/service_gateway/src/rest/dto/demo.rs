//! Demo DTOs for the demo_gui frontend integration
//!
//! These DTOs match the TypeScript API types defined in:
//! `demo/gui/static/src/types/api.ts`

use serde::{Deserialize, Serialize};

// =============================================================================
// Configuration Types
// =============================================================================

/// Application configuration response
#[derive(Debug, Clone, Serialize)]
pub struct AppConfigResponse {
    /// Enum values for dropdowns
    pub enums: std::collections::HashMap<String, Vec<EnumValue>>,
    /// Default values for forms
    pub defaults: std::collections::HashMap<String, serde_json::Value>,
    /// Rate index by currency
    pub rate_index_by_currency: std::collections::HashMap<String, String>,
}

/// Enum value (string or object with code/name)
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum EnumValue {
    /// Simple string value
    Simple(String),
    /// Object with code and optional name
    Object { code: String, name: Option<String> },
}

// =============================================================================
// Instrument Types
// =============================================================================

/// Instruments response
#[derive(Debug, Clone, Serialize)]
pub struct InstrumentsResponse {
    pub instruments: Vec<InstrumentDef>,
}

/// Instrument definition
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentDef {
    pub instrument_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_class_name: Option<String>,
    pub required_params: Vec<ParameterDef>,
    pub optional_params: Vec<ParameterDef>,
}

/// Parameter definition
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub field_type: FieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ParameterOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<ParameterValidation>,
}

/// Field type
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Number,
    Date,
    Select,
}

/// Parameter option for select fields
#[derive(Debug, Clone, Serialize)]
pub struct ParameterOption {
    pub value: String,
    pub label: String,
}

/// Parameter validation
#[derive(Debug, Clone, Serialize)]
pub struct ParameterValidation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

// =============================================================================
// Trade Expansion Types
// =============================================================================

/// Trade expansion request
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeExpandRequest {
    pub instrument_type: String,
    pub params: serde_json::Value,
}

/// Expanded trade response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpandedTrade {
    pub trade_id: String,
    pub trade_type: String,
    pub legs: Vec<TradeLeg>,
    pub metadata: TradeMetadata,
}

/// Trade leg
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeLeg {
    pub direction: String,
    pub currency: String,
    pub leg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
    pub cashflows: Vec<Cashflow>,
}

/// Cashflow
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cashflow {
    pub payment_date: String,
    pub accrual_start: String,
    pub accrual_end: String,
    pub year_fraction: f64,
    pub notional: f64,
    pub rate: Option<f64>,
    pub payoff_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
}

/// Trade metadata
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeMetadata {
    pub total_legs: usize,
    pub total_cashflows: usize,
    pub processing_time_ms: f64,
}

// =============================================================================
// Pricing Types
// =============================================================================

/// Pricing request
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoPricingRequest {
    pub valuation_date: String,
    pub reporting_currency: String,
    pub legs: Vec<PricingLeg>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_config: Option<DemoModelConfig>,
}

/// Pricing leg
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingLeg {
    pub currency: String,
    pub direction: String,
    pub cashflows: Vec<PricingCashflow>,
}

/// Pricing cashflow
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingCashflow {
    pub payment_date: String,
    pub amount: f64,
}

/// Model configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoModelConfig {
    pub num_paths: usize,
    pub num_steps: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
}

/// Pricing result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoPricingResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_pv: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pv: Option<f64>,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legs: Option<Vec<LegResult>>,
}

/// Leg result
#[derive(Debug, Clone, Serialize)]
pub struct LegResult {
    pub direction: String,
    pub pv: f64,
}

// =============================================================================
// Greeks Types
// =============================================================================

/// Greeks request
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoGreeksRequest {
    pub valuation_date: String,
    pub reporting_currency: String,
    pub legs: Vec<PricingLeg>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_config: Option<DemoModelConfig>,
    pub bump_sizes: BumpSizes,
}

/// Bump sizes for Greeks
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BumpSizes {
    pub rate_bump_bp: f64,
    pub fx_bump_pct: f64,
    pub vol_bump_pct: f64,
}

/// Greeks result
#[derive(Debug, Clone, Serialize)]
pub struct DemoGreeksResult {
    pub currency: String,
    pub delta: f64,
    pub gamma: Option<f64>,
    pub theta: Option<f64>,
    pub vega: Option<f64>,
}

// =============================================================================
// Market Data Types
// =============================================================================

/// Market rate
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRate {
    pub id: String,
    pub currency: String,
    pub tenor: String,
    pub rate_type: String,
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_type: Option<String>,
    pub source: String,
    pub timestamp: String,
    pub is_stale: bool,
}

/// Market rates response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRatesResponse {
    pub rates: Vec<MarketRate>,
    pub last_updated: String,
}

/// Market config response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketConfigResponse {
    pub tenor_order: Vec<String>,
}

/// Market rate detail response
#[derive(Debug, Clone, Serialize)]
pub struct MarketRateDetailResponse {
    pub rate: MarketRate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrument: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub convention: Option<Convention>,
}

// =============================================================================
// Convention Types
// =============================================================================

/// Convention
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Convention {
    pub id: String,
    pub convention_type: String,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<ConventionField>>,
}

/// Convention field
#[derive(Debug, Clone, Serialize)]
pub struct ConventionField {
    pub label: String,
    pub value: String,
}

/// Conventions response
#[derive(Debug, Clone, Serialize)]
pub struct ConventionsResponse {
    pub conventions: Vec<Convention>,
}

// =============================================================================
// IR Volatility Types
// =============================================================================

/// IR vol currency
#[derive(Debug, Clone, Serialize)]
pub struct IrVolCurrency {
    pub currency: String,
}

/// IR vol currencies response
#[derive(Debug, Clone, Serialize)]
pub struct IrVolCurrenciesResponse {
    pub currencies: Vec<IrVolCurrency>,
}

/// IR vol quote
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolQuote {
    pub expiry: String,
    pub tenor: String,
    pub atm_vol: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smile: Option<Vec<SmilePoint>>,
}

/// Smile point
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmilePoint {
    pub strike_offset_bp: f64,
    pub vol: f64,
}

/// IR vol quotes response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolQuotesResponse {
    pub quotes: Vec<IrVolQuote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

// =============================================================================
// FX Volatility Types
// =============================================================================

/// FX vol pair
#[derive(Debug, Clone, Serialize)]
pub struct FxVolPair {
    pub pair: String,
}

/// FX vol pairs response
#[derive(Debug, Clone, Serialize)]
pub struct FxVolPairsResponse {
    pub pairs: Vec<FxVolPair>,
}

/// FX vol quote
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxVolQuote {
    pub expiry: f64,
    pub atm_vol: f64,
    pub rr25d: f64,
    pub bf25d: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rr10d: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bf10d: Option<f64>,
}

/// FX vol quotes response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxVolQuotesResponse {
    pub quotes: Vec<FxVolQuote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spot: Option<f64>,
}

// =============================================================================
// Events Types
// =============================================================================

/// Market event
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketEvent {
    pub id: String,
    pub date: String,
    pub event_type: EventType,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub importance: Importance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub central_bank: Option<CentralBank>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forecast: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
}

/// Event type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    CentralBankMeeting,
    EconomicRelease,
    Holiday,
    News,
    Expiry,
    Other,
}

/// Importance level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Importance {
    Critical,
    High,
    Medium,
    Low,
}

/// Central bank
#[derive(Debug, Clone, Serialize)]
pub struct CentralBank {
    pub name: String,
    pub code: String,
    pub currency: String,
}

/// Events response
#[derive(Debug, Clone, Serialize)]
pub struct EventsResponse {
    pub events: Vec<MarketEvent>,
}

/// Event types response
#[derive(Debug, Clone, Serialize)]
pub struct EventTypesResponse {
    pub types: Vec<String>,
}

// =============================================================================
// Curves Types (additional to existing curves module)
// =============================================================================

/// Available curves response
#[derive(Debug, Clone, Serialize)]
pub struct AvailableCurvesResponse {
    pub curves: Vec<String>,
}

// =============================================================================
// Export Types
// =============================================================================

/// Export format
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Csv,
    Json,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_serialization() {
        let config = AppConfigResponse {
            enums: std::collections::HashMap::new(),
            defaults: std::collections::HashMap::new(),
            rate_index_by_currency: std::collections::HashMap::new(),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("enums"));
        assert!(json.contains("defaults"));
    }

    #[test]
    fn test_enum_value_serialization() {
        let simple = EnumValue::Simple("USD".to_string());
        let json = serde_json::to_string(&simple).unwrap();
        assert_eq!(json, "\"USD\"");

        let object = EnumValue::Object {
            code: "USD".to_string(),
            name: Some("US Dollar".to_string()),
        };
        let json = serde_json::to_string(&object).unwrap();
        assert!(json.contains("USD"));
        assert!(json.contains("US Dollar"));
    }

    #[test]
    fn test_instrument_def_serialization() {
        let instrument = InstrumentDef {
            instrument_type: "IRS".to_string(),
            id: Some("irs-1".to_string()),
            display_name: Some("Interest Rate Swap".to_string()),
            asset_class_name: Some("Rates".to_string()),
            required_params: vec![],
            optional_params: vec![],
        };
        let json = serde_json::to_string(&instrument).unwrap();
        assert!(json.contains("instrumentType"));
        assert!(json.contains("IRS"));
    }

    #[test]
    fn test_market_rate_serialization() {
        let rate = MarketRate {
            id: "USD-SOFR-3M".to_string(),
            currency: "USD".to_string(),
            tenor: "3M".to_string(),
            rate_type: "deposit".to_string(),
            value: 0.05,
            rate_index: Some("SOFR".to_string()),
            quote_type: Some("Mid".to_string()),
            source: "Reuters".to_string(),
            timestamp: "2026-01-30T10:00:00Z".to_string(),
            is_stale: false,
        };
        let json = serde_json::to_string(&rate).unwrap();
        assert!(json.contains("USD-SOFR-3M"));
        assert!(json.contains("isStale"));
    }

    #[test]
    fn test_fx_vol_quote_serialization() {
        let quote = FxVolQuote {
            expiry: 0.25,
            atm_vol: 0.10,
            rr25d: -0.005,
            bf25d: 0.002,
            rr10d: Some(-0.01),
            bf10d: Some(0.003),
        };
        let json = serde_json::to_string(&quote).unwrap();
        assert!(json.contains("atmVol"));
        assert!(json.contains("rr25d"));
    }

    #[test]
    fn test_event_type_serialization() {
        let event_type = EventType::CentralBankMeeting;
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, "\"central_bank_meeting\"");
    }

    #[test]
    fn test_importance_serialization() {
        let importance = Importance::Critical;
        let json = serde_json::to_string(&importance).unwrap();
        assert_eq!(json, "\"critical\"");
    }

    #[test]
    fn test_trade_expand_request_deserialization() {
        let json = r#"{
            "instrumentType": "IRS",
            "params": {
                "type": "VanillaIRS",
                "notional": 1000000
            }
        }"#;
        let request: TradeExpandRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.instrument_type, "IRS");
    }

    #[test]
    fn test_pricing_request_deserialization() {
        let json = r#"{
            "valuationDate": "2026-01-30",
            "reportingCurrency": "USD",
            "legs": [],
            "modelConfig": null
        }"#;
        let request: DemoPricingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.valuation_date, "2026-01-30");
        assert_eq!(request.reporting_currency, "USD");
    }
}
