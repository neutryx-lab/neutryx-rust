//! Demo DTOs for the demo_gui frontend integration.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeExpandRequest {
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

/// Pricing request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoPricingRequest {
    pub valuation_date: String,
    pub reporting_currency: String,
    pub legs: Vec<PricingLeg>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_config: Option<DemoModelConfig>,
}

/// Pricing leg.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingLeg {
    pub currency: String,
    pub direction: String,
    pub cashflows: Vec<PricingCashflow>,
}

/// Pricing cashflow.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingCashflow {
    pub payment_date: String,
    pub amount: f64,
}

/// Model configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoModelConfig {
    pub num_paths: usize,
    pub num_steps: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
}

/// Pricing result.
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
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolcubeCalibrateRequest {
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
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxVolCalibrateRequest {
    pub pair: String,
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

/// Request to compute SABR smile and implied density from calibrated parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct SabrSmileRequest {
    /// SABR alpha (vol-of-vol backbone).
    pub alpha: f64,
    /// SABR beta (CEV exponent).
    pub beta: f64,
    /// SABR rho (correlation).
    pub rho: f64,
    /// SABR nu (vol-of-vol).
    pub nu: f64,
    /// Forward rate.
    pub forward: f64,
    /// Time to expiry in years.
    pub expiry_years: f64,
    /// Number of output points (default: 101).
    #[serde(default = "default_sabr_n_points")]
    pub n_points: usize,
    /// Strike range in basis points (default: 200, i.e., -200 to +200).
    #[serde(default = "default_sabr_range_bp")]
    pub range_bp: f64,
}

fn default_sabr_n_points() -> usize { 101 }
fn default_sabr_range_bp() -> f64 { 200.0 }

/// Response with SABR smile and implied density.
#[derive(Debug, Clone, Serialize)]
pub struct SabrSmileResponse {
    /// Strike offsets in basis points.
    pub offsets: Vec<f64>,
    /// Normal volatilities (percentage, same scale as market data).
    pub vols: Vec<f64>,
    /// Implied probability density (Breeden-Litzenberger).
    pub density: Vec<f64>,
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
    /// Tenor (e.g., "O/N", "3M").
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
#[derive(Debug, Clone, Deserialize)]
pub struct ImpliedPdfRequest {
    /// Time to expiry in years.
    pub expiry_years: f64,
    /// ATM normal volatility (percentage, e.g., 80.0 for 80bp).
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
}
