//! Demo DTOs for the demo_gui frontend integration.
#![allow(dead_code)]
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Application configuration response.
#[derive(Debug, Clone, Serialize)]
pub struct AppConfigResponse {
    /// Enum values for dropdowns.
    pub enums: std::collections::HashMap<String, Vec<EnumValue>>,
    /// Default values for forms.
    pub defaults: std::collections::HashMap<String, serde_json::Value>,
    /// Rate index by currency.
    pub rate_index_by_currency: std::collections::HashMap<String, String>,
}

/// Enum value (string or object with code/name).
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum EnumValue {
    /// Simple string value.
    Simple(String),
    /// Object with code and optional name.
    Object { code: String, name: Option<String> },
}

/// Instruments response.
#[derive(Debug, Clone, Serialize)]
pub struct InstrumentsResponse {
    pub instruments: Vec<InstrumentDef>,
}

/// Instrument definition.
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

/// Parameter definition.
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

/// Field type.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Number,
    Date,
    Select,
}

/// Parameter option for select fields.
#[derive(Debug, Clone, Serialize)]
pub struct ParameterOption {
    pub value: String,
    pub label: String,
}

/// Parameter validation.
#[derive(Debug, Clone, Serialize)]
pub struct ParameterValidation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

/// Trade expansion request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TradeExpandRequest {
    #[validate(length(min = 1))]
    pub instrument_type: String,
    pub params: serde_json::Value,
}

/// Expanded trade response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpandedTrade {
    pub trade_id: String,
    pub trade_type: String,
    pub legs: Vec<TradeLeg>,
    pub metadata: TradeMetadata,
}

/// Trade leg.
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

/// Cashflow.
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

/// Trade metadata.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeMetadata {
    pub total_legs: usize,
    pub total_cashflows: usize,
    pub processing_time_ms: f64,
}

/// Pricing method hint (mirrors `PricingMethodHint`).
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DemoPricingMethod {
    #[default]
    Auto,
    Analytical,
    MonteCarlo,
    Tree,
}

/// Tree type (mirrors `TreeType`).
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DemoTreeType {
    #[default]
    Binomial,
    Trinomial,
}

/// Tree configuration (mirrors `TreeSetting`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoTreeConfig {
    pub num_steps: Option<usize>,
    pub tree_type: Option<DemoTreeType>,
}

/// Pricing request (mirrors `CalcSetting` + trade data).
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DemoPricingRequest {
    #[validate(length(min = 1))]
    pub valuation_date: String,
    #[validate(length(min = 1))]
    pub reporting_currency: String,
    #[validate(length(min = 1))]
    pub legs: Vec<PricingLeg>,
    #[serde(default)]
    pub method: DemoPricingMethod,
    #[serde(default)]
    pub compute_greeks: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mc_config: Option<DemoModelConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_config: Option<DemoTreeConfig>,
}

/// Pricing leg.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingLeg {
    pub currency: String,
    pub direction: String,
    pub cashflows: Vec<PricingCashflow>,
}

/// Pricing cashflow.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingCashflow {
    pub payment_date: String,
    pub notional: f64,
    pub rate: Option<f64>,
    pub year_fraction: f64,
    #[serde(default = "default_payoff_type")]
    pub payoff_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accrual_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accrual_end: Option<String>,
}

fn default_payoff_type() -> String { "Fixed".to_string() }

/// Model configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoModelConfig {
    pub num_paths: usize,
    pub num_steps: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
}

/// Pricing result (mirrors `PricingResult` from result.rs).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoPricingResult {
    pub total_pv: f64,
    pub reporting_currency: String,
    pub legs: Vec<LegResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_distribution: Option<DemoPathDistribution>,
    /// Pricing method used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Greeks (if compute_greeks was true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeks: Option<DemoGreeksInline>,
    /// Computation time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computation_time_ms: Option<f64>,
}

/// Inline Greeks returned with pricing result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoGreeksInline {
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub theta: Option<f64>,
    pub rho: Option<f64>,
}

/// Path distribution for Monte Carlo pricing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoPathDistribution {
    pub mean: f64,
    pub std_dev: f64,
    pub percentiles: Vec<(f64, f64)>,
    pub path_count: usize,
}

/// Leg result with detailed breakdown.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegResult {
    pub direction: String,
    pub pv: f64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pv_original: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fx_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cashflows: Option<Vec<CashflowPvResult>>,
}

/// Cashflow-level PV result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashflowPvResult {
    pub pv: f64,
    pub discount_factor: f64,
    pub payment_date: String,
}

/// Greeks request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DemoGreeksRequest {
    #[validate(length(min = 1))]
    pub valuation_date: String,
    #[validate(length(min = 1))]
    pub reporting_currency: String,
    #[validate(length(min = 1))]
    pub legs: Vec<PricingLeg>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_config: Option<DemoModelConfig>,
    pub bump_sizes: BumpSizes,
}

/// Bump sizes for Greeks.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BumpSizes {
    pub rate_bump_bp: f64,
    pub fx_bump_pct: f64,
    pub vol_bump_pct: f64,
}

/// Greeks result.
#[derive(Debug, Clone, Serialize)]
pub struct DemoGreeksResult {
    pub currency: String,
    pub delta: f64,
    pub gamma: Option<f64>,
    pub theta: Option<f64>,
    pub vega: Option<f64>,
}

/// Advanced Greeks configuration — tagged enum keyed on `mode`.
///
/// `BumpRevalue` carries user-specified bump sizes; `EnzymeAad` uses
/// `pricer_risk::GreeksConfig::default()` internally.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum AdvancedGreeksConfig {
    /// Bump-and-revalue with user-specified bump sizes.
    #[serde(rename_all = "camelCase")]
    BumpRevalue {
        #[serde(default = "default_spot_bump")]
        spot_bump_relative: f64,
        #[serde(default = "default_vol_bump")]
        vol_bump_absolute: f64,
        #[serde(default = "default_time_bump")]
        time_bump_years: f64,
        #[serde(default = "default_rate_bump")]
        rate_bump_absolute: f64,
    },
    /// Enzyme AAD (or FD fallback when `enzyme-ad` feature is disabled).
    EnzymeAad,
}

impl AdvancedGreeksConfig {
    /// Returns `(spot, vol, time, rate)` bump sizes.
    ///
    /// `BumpRevalue` returns user-specified values; `EnzymeAad` returns
    /// `pricer_risk::GreeksConfig::default()` values (used only in FD
    /// fallback).
    pub fn effective_bumps(&self) -> (f64, f64, f64, f64) {
        match self {
            Self::BumpRevalue {
                spot_bump_relative,
                vol_bump_absolute,
                time_bump_years,
                rate_bump_absolute,
            } => (
                *spot_bump_relative,
                *vol_bump_absolute,
                *time_bump_years,
                *rate_bump_absolute,
            ),
            Self::EnzymeAad => {
                let d = pricer_risk::GreeksConfig::default();
                (
                    d.spot_bump_relative,
                    d.vol_bump_absolute,
                    d.time_bump_years,
                    d.rate_bump_absolute,
                )
            }
        }
    }

    /// Returns `true` if this is the `EnzymeAad` variant.
    pub fn is_enzyme_aad(&self) -> bool { matches!(self, Self::EnzymeAad) }
}

fn default_spot_bump() -> f64 { 0.01 }
fn default_vol_bump() -> f64 { 0.01 }
fn default_time_bump() -> f64 { 1.0 / 365.0 }
fn default_rate_bump() -> f64 { 0.0001 }

/// Advanced Greeks request (mirrors `pricer_risk::GreeksConfig` + trade).
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DemoAdvancedGreeksRequest {
    #[validate(length(min = 1))]
    pub valuation_date: String,
    #[validate(length(min = 1))]
    pub reporting_currency: String,
    #[validate(length(min = 1))]
    pub legs: Vec<PricingLeg>,
    pub config: AdvancedGreeksConfig,
}

/// Risk factor identifier (mirrors `pricer_risk::RiskFactorId`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskFactor {
    pub factor_type: String,
    pub name: String,
}

/// Greeks for a single risk factor (mirrors `pricer_risk::GreeksResult`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FactorGreeks {
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub theta: Option<f64>,
    pub rho: Option<f64>,
    pub vanna: Option<f64>,
    pub volga: Option<f64>,
}

/// Per-factor Greeks entry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FactorGreeksEntry {
    pub factor: RiskFactor,
    pub greeks: FactorGreeks,
}

/// Advanced Greeks result by factor (mirrors `pricer_risk::GreeksResultByFactor`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoAdvancedGreeksResult {
    pub price: f64,
    pub currency: String,
    pub mode: String,
    pub computation_time_ms: f64,
    pub factors: Vec<FactorGreeksEntry>,
    pub totals: FactorGreeks,
}

/// Market rate.
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

/// Market rates response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRatesResponse {
    pub rates: Vec<MarketRate>,
    pub last_updated: String,
}

/// Market config response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketConfigResponse {
    pub tenor_order: Vec<String>,
}

/// Market rate detail response.
#[derive(Debug, Clone, Serialize)]
pub struct MarketRateDetailResponse {
    pub rate: MarketRate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrument: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub convention: Option<Convention>,
}

/// Convention.
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

/// Convention field.
#[derive(Debug, Clone, Serialize)]
pub struct ConventionField {
    pub label: String,
    pub value: String,
}

/// Conventions response.
#[derive(Debug, Clone, Serialize)]
pub struct ConventionsResponse {
    pub conventions: Vec<Convention>,
}

/// IR vol currency.
#[derive(Debug, Clone, Serialize)]
pub struct IrVolCurrency {
    pub currency: String,
}

/// IR vol currencies response.
#[derive(Debug, Clone, Serialize)]
pub struct IrVolCurrenciesResponse {
    pub currencies: Vec<IrVolCurrency>,
}

/// IR vol quote.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolQuote {
    pub expiry: String,
    pub tenor: String,
    pub atm_vol: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smile: Option<Vec<SmilePoint>>,
}

/// Smile point.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmilePoint {
    pub strike_offset_bp: f64,
    pub vol: f64,
}

/// IR vol quotes response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolQuotesResponse {
    pub quotes: Vec<IrVolQuote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// FX vol pair.
#[derive(Debug, Clone, Serialize)]
pub struct FxVolPair {
    pub pair: String,
}

/// FX vol pairs response.
#[derive(Debug, Clone, Serialize)]
pub struct FxVolPairsResponse {
    pub pairs: Vec<FxVolPair>,
}

/// FX vol quote.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxVolQuote {
    pub expiry: f64,
    /// Human-readable label for the expiry (e.g., "9M", "15M", "1Y").
    pub expiry_label: String,
    pub atm_vol: f64,
    pub rr25d: f64,
    pub bf25d: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rr10d: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bf10d: Option<f64>,
    /// FX forward rate at this tenor: F(T) = Spot × DF_foreign(T) /.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward: Option<f64>,
}

/// FX vol quotes response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxVolQuotesResponse {
    pub quotes: Vec<FxVolQuote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spot: Option<f64>,
    /// Domestic rate (quote currency) used for forward computation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domestic_rate: Option<f64>,
    /// Foreign rate (base currency) used for forward computation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_rate: Option<f64>,
}

/// Market event.
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
    /// Expected rate spike in basis points (for turn events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_spike_bp: Option<f64>,
}

/// Event type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    CentralBankMeeting,
    EconomicRelease,
    Holiday,
    News,
    Expiry,
    /// Turn of Year (TOY) - year-end rate spike.
    TurnOfYear,
    /// Turn of Quarter (TOQ) - quarter-end rate spike.
    TurnOfQuarter,
    /// Turn of Month (TOM) - month-end rate spike.
    TurnOfMonth,
    /// Generic turn event.
    Turn,
    Other,
}

/// Importance level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Importance {
    Critical,
    High,
    Medium,
    Low,
}

/// Central bank.
#[derive(Debug, Clone, Serialize)]
pub struct CentralBank {
    pub name: String,
    pub code: String,
    pub currency: String,
}

/// Events response.
#[derive(Debug, Clone, Serialize)]
pub struct EventsResponse {
    pub events: Vec<MarketEvent>,
}

/// Event types response.
#[derive(Debug, Clone, Serialize)]
pub struct EventTypesResponse {
    pub types: Vec<String>,
}

/// Holiday data.
#[derive(Debug, Clone, Serialize)]
pub struct Holiday {
    pub id: String,
    pub date: String,
    pub name: String,
    pub country: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Holiday type: "bank", "market", "settlement".
    #[serde(rename = "type")]
    pub holiday_type: String,
    pub importance: Importance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Holidays response.
#[derive(Debug, Clone, Serialize)]
pub struct HolidaysResponse {
    pub holidays: Vec<Holiday>,
}

/// Available curves response.
#[derive(Debug, Clone, Serialize)]
pub struct AvailableCurvesResponse {
    pub curves: Vec<String>,
}

/// Curve indices response.
#[derive(Debug, Clone, Serialize)]
pub struct CurveIndicesResponse {
    pub indices: Vec<String>,
}

/// Curve instrument for bootstrapping.
#[derive(Debug, Clone, Serialize)]
pub struct CurveInstrument {
    #[serde(rename = "type")]
    pub instrument_type: String,
    pub tenor: String,
    pub rate: f64,
    pub enabled: bool,
}

/// Curve instruments response.
#[derive(Debug, Clone, Serialize)]
pub struct CurveInstrumentsResponse {
    pub instruments: Vec<CurveInstrument>,
}

/// Volcube indices response.
#[derive(Debug, Clone, Serialize)]
pub struct VolcubeIndicesResponse {
    pub indices: Vec<String>,
}

/// Volcube calibration models response.
#[derive(Debug, Clone, Serialize)]
pub struct VolcubeModelsResponse {
    pub models: Vec<String>,
}

/// Swaption instrument for volcube.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwaptionInstrument {
    pub expiry: String,
    pub tenor: String,
    pub strike: String,
    pub atm_vol: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub smile: Vec<SmilePoint>,
    pub enabled: bool,
}

/// Volcube instruments response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolcubeInstrumentsResponse {
    pub instruments: Vec<SwaptionInstrument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_date: Option<String>,
}

/// Initial SABR parameter guesses for calibration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SabrInitialParams {
    pub alpha: Option<f64>,
    pub beta: Option<f64>,
    pub rho: Option<f64>,
    pub nu: Option<f64>,
}

/// Flags indicating which SABR parameters should be held fixed during.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SabrFixedParams {
    pub alpha: Option<bool>,
    pub beta: Option<bool>,
    pub rho: Option<bool>,
    pub nu: Option<bool>,
}

/// Volcube calibration request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct VolcubeCalibrateRequest {
    #[validate(length(min = 1))]
    pub index: String,
    pub reference_date: Option<String>,
    pub model: Option<String>,
    /// Forward swap rates keyed by "expiry|tenor" (e.g.
    #[serde(default)]
    pub forward_rates: Option<std::collections::HashMap<String, f64>>,
    /// Initial SABR parameter guesses.
    #[serde(default)]
    pub initial_params: Option<SabrInitialParams>,
    /// Which parameters to hold fixed during calibration.
    #[serde(default)]
    pub fixed_params: Option<SabrFixedParams>,
}

/// Volcube calibration response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolcubeCalibrateResponse {
    pub model: String,
    pub metadata: CalibrationMetadata,
    pub parameters: CalibrationParameters,
    /// Per-cell SABR parameters keyed by "expiry|tenor".
    pub cell_parameters: std::collections::HashMap<String, CalibrationParameters>,
    /// Per-cell calibration diagnostics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell_diagnostics: Option<std::collections::HashMap<String, CellDiagnostics>>,
    /// Per-cell Jacobian `∂σ_model / ∂θ_k`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell_jacobians: Option<std::collections::HashMap<String, CellJacobian>>,
}

/// Per-cell calibration diagnostics.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CellDiagnostics {
    pub converged: bool,
    pub iterations: usize,
    pub rmse: f64,
}

/// Per-cell Jacobian: `∂σ_model / ∂θ_k` at each instrument strike.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CellJacobian {
    /// Row labels (strike descriptions, e.g., "ATM", "+50bp").
    pub row_labels: Vec<String>,
    /// Column labels (parameter names, e.g., "α", "ρ", "ν").
    pub col_labels: Vec<String>,
    /// m × n matrix of `∂σ_model / ∂θ_k` values.
    pub matrix: Vec<Vec<f64>>,
}

/// FX vol calibration request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct FxVolCalibrateRequest {
    #[validate(length(min = 1))]
    pub pair: String,
    #[validate(range(exclusive_min = 0.0))]
    pub spot: f64,
    pub domestic_rate: f64,
    pub foreign_rate: f64,
    /// FX forward rates keyed by tenor label (e.g.
    #[serde(default)]
    pub forward_rates: Option<std::collections::HashMap<String, f64>>,
    /// Initial SABR parameter guesses.
    #[serde(default)]
    pub initial_params: Option<SabrInitialParams>,
    /// Which parameters to hold fixed during calibration.
    #[serde(default)]
    pub fixed_params: Option<SabrFixedParams>,
}

/// Calibration metadata.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationMetadata {
    pub instrument_count: usize,
    pub processing_time_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converged_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_rmse: Option<f64>,
}

/// Calibration parameters (SABR model).
#[derive(Debug, Clone, Serialize)]
pub struct CalibrationParameters {
    pub alpha: f64,
    pub beta: f64,
    pub rho: f64,
    pub nu: f64,
}

/// Request to compute SABR smile and implied density from calibrated
/// parameters.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct SabrSmileRequest {
    /// SABR alpha (vol-of-vol backbone).
    #[validate(range(exclusive_min = 0.0))]
    pub alpha: f64,
    /// SABR beta (CEV exponent).
    #[validate(range(min = 0.0, max = 1.0))]
    pub beta: f64,
    /// SABR rho (correlation).
    #[validate(range(min = -1.0, max = 1.0))]
    pub rho: f64,
    /// SABR nu (vol-of-vol).
    #[validate(range(min = 0.0))]
    pub nu: f64,
    /// Forward rate.
    #[validate(range(exclusive_min = 0.0))]
    pub forward: f64,
    /// Time to expiry in years.
    #[validate(range(exclusive_min = 0.0))]
    pub expiry_years: f64,
    /// Number of output points (default: 101).
    #[serde(default = "default_smile_n_points")]
    pub n_points: usize,
    /// Strike range in basis points (default: 200, i.e., -200 to +200).
    #[serde(default = "default_smile_range_bp")]
    pub range_bp: f64,
}

fn default_smile_n_points() -> usize { 101 }
fn default_smile_range_bp() -> f64 { 200.0 }

/// Response with smile and implied density (shared across all models).
#[derive(Debug, Clone, Serialize)]
pub struct SmileResponse {
    /// Strike offsets in basis points.
    pub offsets: Vec<f64>,
    /// Implied volatilities (percentage).
    pub vols: Vec<f64>,
    /// Implied probability density (Breeden-Litzenberger).
    pub density: Vec<f64>,
}

/// Backward-compatible alias.
pub type SabrSmileResponse = SmileResponse;

/// Generic vol smile request for any model.
///
/// `model` selects the model ("sabr", "svi", "ssvi", etc.).
/// `params` carries model-specific parameters as a JSON object.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct VolSmileRequest {
    /// Model identifier (e.g., "sabr", "svi", "ssvi", "vanna_volga",
    /// "zabr", "mixture_lognormal", "polynomial", "variance_gamma").
    pub model: String,
    /// Forward rate.
    pub forward: f64,
    /// Time to expiry in years.
    pub expiry_years: f64,
    /// Number of output points (default: 101).
    #[serde(default = "default_smile_n_points")]
    pub n_points: usize,
    /// Strike range in basis points (default: 200).
    #[serde(default = "default_smile_range_bp")]
    pub range_bp: f64,
    /// Model-specific parameters.
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Tenor resolution request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ResolveTenorRequest {
    /// Tenor string (e.g. "TD", "3M", "1Y") or ISO date "YYYY-MM-DD".
    #[validate(length(min = 1))]
    pub tenor: String,
    /// Optional base date (ISO "YYYY-MM-DD"). Defaults to today.
    pub base: Option<String>,
}

/// Tenor resolution response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveTenorResponse {
    /// Resolved ISO date string "YYYY-MM-DD".
    pub date: String,
}

/// Pricer graph request (single-instrument computation graph).
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PricerGraphRequest {
    #[validate(length(min = 1))]
    pub instrument_type: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub detail_level: Option<String>,
}

/// Graph node DTO.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricerGraphNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    pub is_sensitivity_target: bool,
    pub group: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trade_ids: Vec<String>,
}

/// Graph edge DTO.
#[derive(Debug, Clone, Serialize)]
pub struct PricerGraphEdge {
    pub source: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

/// Pricer graph response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricerGraphResponse {
    pub nodes: Vec<PricerGraphNode>,
    #[serde(rename = "links")]
    pub edges: Vec<PricerGraphEdge>,
    pub metadata: PricerGraphMetadata,
}

/// Pricer graph metadata.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricerGraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub depth: usize,
    pub generated_at: String,
    pub trade_count: usize,
    pub shared_node_count: usize,
    pub optimisation_ratio: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_locations: Option<std::collections::HashMap<String, String>>,
}

/// Export format.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Csv,
    Json,
}

/// Rate instrument response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateInstrumentResponse {
    /// Rate identifier.
    pub rate_id: String,
    /// Rate value.
    pub rate_value: f64,
    /// Instrument type name.
    pub instrument_type: String,
    /// Convention details.
    pub convention: Option<ConventionDetail>,
    /// Effective date.
    pub effective_date: String,
    /// Maturity date.
    pub maturity_date: String,
    /// Notional amount.
    pub notional: f64,
    /// Processing time in milliseconds.
    pub processing_time_ms: f64,
}

/// Convention detail for instrument response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConventionDetail {
    /// Convention type.
    pub convention_type: String,
    /// Day count convention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_count: Option<String>,
    /// Payment frequency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
    /// Business day convention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_day_convention: Option<String>,
    /// Settlement days (spot lag).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spot_lag: Option<u32>,
    /// Calendar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar: Option<String>,
}

/// Rate cashflows response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateCashflowsResponse {
    /// Rate identifier.
    pub rate_id: String,
    /// Legs with cashflows.
    pub legs: Vec<LegCashflows>,
    /// Processing time in milliseconds.
    pub processing_time_ms: f64,
}

/// Leg with cashflows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegCashflows {
    /// Leg type (Fixed, Floating).
    pub leg_type: String,
    /// Direction (Payer, Receiver).
    pub direction: String,
    /// Currency.
    pub currency: String,
    /// Rate index (for floating legs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
    /// Cashflows.
    pub cashflows: Vec<CashflowDetail>,
}

/// Cashflow detail.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashflowDetail {
    /// Payment date.
    pub payment_date: String,
    /// Accrual start date.
    pub accrual_start: String,
    /// Accrual end date.
    pub accrual_end: String,
    /// Year fraction.
    pub year_fraction: f64,
    /// Notional amount.
    pub notional: f64,
    /// Fixed rate (for fixed legs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,
    /// Spread (for floating legs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread: Option<f64>,
    /// Payoff type.
    pub payoff_type: String,
}

/// Rate index info.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateIndexInfo {
    /// Index code (e.g., "SOFR", "ESTR").
    pub code: String,
    /// Display name.
    pub name: String,
    /// Currency.
    pub currency: String,
    /// Tenor (e.g., "ON", "3M").
    pub tenor: String,
    /// Day counter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_counter: Option<String>,
    /// Is overnight RFR.
    pub is_overnight: bool,
    /// Number of associated market rates.
    pub associated_rates_count: usize,
    /// Number of associated conventions.
    pub associated_conventions_count: usize,
}

/// Rate indices response.
#[derive(Debug, Clone, Serialize)]
pub struct RateIndicesResponse {
    /// List of rate indices.
    pub indices: Vec<RateIndexInfo>,
}

/// Rate index detail response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateIndexDetailResponse {
    /// Index code.
    pub code: String,
    /// Display name.
    pub name: String,
    /// Currency.
    pub currency: String,
    /// Tenor.
    pub tenor: String,
    /// Index metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<RateIndexMetadata>,
    /// Associated rate IDs.
    pub associated_rates: Vec<String>,
    /// Associated convention IDs.
    pub associated_conventions: Vec<String>,
}

/// Rate index metadata.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateIndexMetadata {
    /// Fixing lag in days.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixing_lag: Option<u32>,
    /// Settlement lag in days.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_lag: Option<u32>,
    /// Compounding method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compounding_method: Option<String>,
    /// Fixing calendar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixing_calendar: Option<String>,
}

/// Index rates response.
#[derive(Debug, Clone, Serialize)]
pub struct IndexRatesResponse {
    /// List of market rates for this index.
    pub rates: Vec<MarketRate>,
}

/// Index conventions response.
#[derive(Debug, Clone, Serialize)]
pub struct IndexConventionsResponse {
    /// List of conventions using this index.
    pub conventions: Vec<Convention>,
}

/// A smile point for implied PDF computation.
#[derive(Debug, Clone, Deserialize)]
pub struct ImpliedPdfSmilePoint {
    /// Strike offset in basis points from ATM.
    pub strike_offset_bp: f64,
    /// Normal volatility (percentage, e.g., 80.0 for 80bp).
    pub vol: f64,
}

/// Request to compute implied probability density function via.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ImpliedPdfRequest {
    /// Time to expiry in years.
    #[validate(range(exclusive_min = 0.0))]
    pub expiry_years: f64,
    /// ATM normal volatility (percentage, e.g., 80.0 for 80bp).
    #[validate(range(exclusive_min = 0.0))]
    pub atm_vol: f64,
    /// Smile points (strike offset in bp, vol in percentage).
    pub smile: Vec<ImpliedPdfSmilePoint>,
    /// Strike range in bp (default: 150, i.e., -150 to +150).
    #[serde(default = "default_pdf_range_bp")]
    pub range_bp: f64,
    /// Strike step in bp (default: 2).
    #[serde(default = "default_pdf_step_bp")]
    pub step_bp: f64,
}

fn default_pdf_range_bp() -> f64 { 150.0 }
fn default_pdf_step_bp() -> f64 { 2.0 }

/// Response with implied probability density.
#[derive(Debug, Clone, Serialize)]
pub struct ImpliedPdfResponse {
    /// Strike offsets in basis points.
    pub offsets: Vec<f64>,
    /// Probability density values.
    pub density: Vec<f64>,
}

/// Bond market data quote.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BondQuote {
    pub id: String,
    pub currency: String,
    pub issuer: String,
    pub maturity: String,
    pub coupon_rate: f64,
    pub ytm: f64,
    pub price: f64,
    pub duration: f64,
    pub convexity: f64,
    pub coupon_frequency: String,
    pub rating: String,
    /// "government", "corporate", or "agency".
    pub bond_type: String,
    pub source: String,
    pub is_stale: bool,
}

/// Bond quotes response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BondQuotesResponse {
    pub quotes: Vec<BondQuote>,
    pub last_updated: String,
}

/// Credit market data quote (CDS spread).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditQuote {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub tenor: String,
    pub spread: f64,
    pub upfront: f64,
    pub recovery_rate: f64,
    pub index_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<String>,
    pub source: String,
    pub is_stale: bool,
}

/// Credit quotes response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditQuotesResponse {
    pub quotes: Vec<CreditQuote>,
    pub last_updated: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enum_value_untagged_serialization() {
        let simple = EnumValue::Simple("USD".to_string());
        assert_eq!(serde_json::to_string(&simple).unwrap(), "\"USD\"");

        let object = EnumValue::Object {
            code: "USD".to_string(),
            name: Some("US Dollar".to_string()),
        };
        let json = serde_json::to_string(&object).unwrap();
        assert!(json.contains("US Dollar"));
    }

    #[test]
    fn test_trade_expand_request_deserialization() {
        let json = r#"{"instrumentType": "IRS", "params": {"type": "VanillaIRS"}}"#;
        let request: TradeExpandRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.instrument_type, "IRS");
    }

    #[test]
    fn test_advanced_greeks_config_bump_revalue() {
        let json = r#"{"mode": "bumpRevalue", "rateBumpAbsolute": 0.001}"#;
        let cfg: AdvancedGreeksConfig = serde_json::from_str(json).unwrap();
        let (spot, _vol, _time, rate) = cfg.effective_bumps();
        assert!((rate - 0.001).abs() < 1e-10);
        // spot falls back to default
        assert!((spot - 0.01).abs() < 1e-10);
        assert!(!cfg.is_enzyme_aad());
    }

    #[test]
    fn test_advanced_greeks_config_bump_revalue_defaults() {
        let json = r#"{"mode": "bumpRevalue"}"#;
        let cfg: AdvancedGreeksConfig = serde_json::from_str(json).unwrap();
        let (spot, vol, _time, rate) = cfg.effective_bumps();
        assert!((spot - 0.01).abs() < 1e-10);
        assert!((vol - 0.01).abs() < 1e-10);
        assert!((rate - 0.0001).abs() < 1e-10);
    }

    #[test]
    fn test_advanced_greeks_config_enzyme_aad() {
        let json = r#"{"mode": "enzymeAad"}"#;
        let cfg: AdvancedGreeksConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.is_enzyme_aad());
        // effective_bumps returns pricer_risk defaults
        let (spot, vol, _time, rate) = cfg.effective_bumps();
        assert!(spot > 0.0);
        assert!(vol > 0.0);
        assert!(rate > 0.0);
    }
}
