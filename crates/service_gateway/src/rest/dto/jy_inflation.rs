//! Data Transfer Objects for Jarrow-Yildirim inflation model endpoints.

use serde::{Deserialize, Serialize};
use validator::Validate;

// ─── Shared Types ────────────────────────────────────────────────────────────

/// A single rate point for curve construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurveRatePoint {
    pub instrument_type: String,
    pub tenor: String,
    pub rate: f64,
}

/// A single inflation index observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InflationIndexPoint {
    pub date: String,
    pub level: f64,
}

/// JY model parameters (5 core parameters).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct JyModelParams {
    /// Nominal rate mean reversion speed.
    #[validate(range(min = 0.001, max = 10.0))]
    pub a_n: f64,
    /// Nominal rate volatility.
    #[validate(range(min = 0.0001, max = 1.0))]
    pub sigma_n: f64,
    /// Real rate mean reversion speed.
    #[validate(range(min = 0.001, max = 10.0))]
    pub a_r: f64,
    /// Real rate volatility.
    #[validate(range(min = 0.0001, max = 1.0))]
    pub sigma_r: f64,
    /// Inflation index volatility.
    #[validate(range(min = 0.0001, max = 1.0))]
    pub sigma_i: f64,
}

/// JY correlation structure (3 pairwise correlations).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct JyCorrelation {
    /// Nominal-real rate correlation.
    #[validate(range(min = -1.0, max = 1.0))]
    pub rho_nr: f64,
    /// Nominal rate-inflation correlation.
    #[validate(range(min = -1.0, max = 1.0))]
    pub rho_ni: f64,
    /// Real rate-inflation correlation.
    #[validate(range(min = -1.0, max = 1.0))]
    pub rho_ri: f64,
}

/// A point on a constructed curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JyCurvePoint {
    /// Tenor in years.
    pub tenor: f64,
    /// Rate or discount factor value.
    pub value: f64,
}

// ─── Curve Build ─────────────────────────────────────────────────────────────

/// Request to build nominal and real yield curves.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct JyCurveBuildRequest {
    /// Nominal curve market rates (e.g., USD swap rates).
    #[validate(length(min = 1))]
    pub nominal_rates: Vec<CurveRatePoint>,
    /// Real curve market rates (e.g., TIPS yields).
    #[validate(length(min = 1))]
    pub real_rates: Vec<CurveRatePoint>,
    /// Valuation date (ISO 8601).
    #[validate(length(min = 1))]
    pub valuation_date: String,
    /// Model parameters.
    #[validate(nested)]
    pub model_params: JyModelParams,
    /// Correlation structure.
    #[validate(nested)]
    pub correlation: JyCorrelation,
}

/// Response from curve building.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JyCurveBuildResponse {
    /// Nominal zero rates by tenor.
    pub nominal_curve: Vec<JyCurvePoint>,
    /// Real zero rates by tenor.
    pub real_curve: Vec<JyCurvePoint>,
    /// Breakeven inflation rates (nominal - real).
    pub breakeven_curve: Vec<JyCurvePoint>,
    /// Nominal discount factors.
    pub nominal_df: Vec<JyCurvePoint>,
    /// Real discount factors.
    pub real_df: Vec<JyCurvePoint>,
}

// ─── Instrument ──────────────────────────────────────────────────────────────

/// Request to generate instrument cashflows.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct JyInstrumentRequest {
    /// Instrument type: "ZCIS" or "YoYIS".
    #[validate(length(min = 1))]
    pub instrument_type: String,
    /// Notional amount.
    #[validate(range(min = 1.0))]
    pub notional: f64,
    /// Fixed rate (annual, e.g., 0.02 for 2%).
    pub fixed_rate: f64,
    /// Start date (ISO 8601).
    #[validate(length(min = 1))]
    pub start_date: String,
    /// Maturity date (ISO 8601).
    #[validate(length(min = 1))]
    pub maturity_date: String,
    /// Payment frequency: "annual", "semiannual", "quarterly".
    #[serde(default = "default_frequency")]
    pub payment_frequency: String,
    /// Nominal curve rate for discounting.
    #[serde(default = "default_nominal_rate")]
    pub nominal_curve_rate: f64,
    /// Real curve rate for real discounting.
    #[serde(default = "default_real_rate")]
    pub real_curve_rate: f64,
}

fn default_frequency() -> String {
    "annual".to_string()
}
fn default_nominal_rate() -> f64 {
    0.03
}
fn default_real_rate() -> f64 {
    0.01
}

/// A single cashflow in the instrument schedule.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JyCashflow {
    /// Payment date (ISO 8601).
    pub date: String,
    /// Year fraction from previous date.
    pub year_fraction: f64,
    /// Nominal cashflow amount.
    pub nominal_amount: f64,
    /// Real (inflation-adjusted) cashflow amount, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_amount: Option<f64>,
    /// Discount factor to valuation date.
    pub discount_factor: f64,
    /// Present value of this cashflow.
    pub present_value: f64,
}

/// Instrument cashflow response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JyInstrumentResponse {
    /// Instrument type.
    pub instrument_type: String,
    /// Full cashflow schedule.
    pub cashflows: Vec<JyCashflow>,
    /// Summary statistics.
    pub summary: JyInstrumentSummary,
}

/// Summary of instrument cashflows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JyInstrumentSummary {
    pub total_fixed_pv: f64,
    pub total_inflation_pv: f64,
    pub net_pv: f64,
    pub num_cashflows: usize,
    pub maturity_years: f64,
}

// ─── Simulation ──────────────────────────────────────────────────────────────

/// Request to run Monte Carlo simulation of JY factors.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct JySimulationRequest {
    /// Model parameters.
    #[validate(nested)]
    pub model_params: JyModelParams,
    /// Correlation structure.
    #[validate(nested)]
    pub correlation: JyCorrelation,
    /// Number of Monte Carlo paths.
    #[validate(range(min = 100, max = 100000))]
    pub num_paths: u32,
    /// Number of time steps.
    #[validate(range(min = 10, max = 5000))]
    pub num_steps: u32,
    /// Simulation horizon in years.
    #[validate(range(min = 0.1, max = 50.0))]
    pub horizon: f64,
    /// Initial nominal short rate.
    pub initial_nominal_rate: f64,
    /// Initial real short rate.
    pub initial_real_rate: f64,
    /// Initial inflation index level.
    #[validate(range(min = 0.01))]
    pub initial_index: f64,
    /// Number of sample paths to return (for charting).
    #[serde(default = "default_sample_paths")]
    #[validate(range(max = 20))]
    pub num_sample_paths: u32,
}

fn default_sample_paths() -> u32 {
    5
}

/// Statistics for a simulated factor across time steps.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationPathStats {
    pub mean: Vec<f64>,
    pub percentile_5: Vec<f64>,
    pub percentile_25: Vec<f64>,
    pub percentile_75: Vec<f64>,
    pub percentile_95: Vec<f64>,
}

/// A single sample path across all three factors.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JySamplePath {
    pub nominal_rate: Vec<f64>,
    pub real_rate: Vec<f64>,
    pub inflation_index: Vec<f64>,
}

/// Simulation response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JySimulationResponse {
    /// Time grid (years).
    pub time_grid: Vec<f64>,
    /// Nominal rate statistics.
    pub nominal_rate: SimulationPathStats,
    /// Real rate statistics.
    pub real_rate: SimulationPathStats,
    /// Inflation index statistics.
    pub inflation_index: SimulationPathStats,
    /// Sample paths for charting.
    pub sample_paths: Vec<JySamplePath>,
    /// Realized correlation matrix (empirical from paths).
    pub correlation_realized: JyCorrelation,
    /// PSD enforcement applied flag.
    pub psd_enforced: bool,
}

// ─── Pricing ─────────────────────────────────────────────────────────────────

/// Request to price a ZCIS using analytical formulas.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct JyPricingRequest {
    #[validate(nested)]
    pub model_params: JyModelParams,
    #[validate(nested)]
    pub correlation: JyCorrelation,
    pub initial_nominal_rate: f64,
    pub initial_real_rate: f64,
    #[validate(range(min = 0.01))]
    pub initial_index: f64,
    #[validate(range(min = 1.0))]
    pub notional: f64,
    pub fixed_rate: f64,
    #[validate(range(min = 0.1, max = 50.0))]
    pub maturity: f64,
    pub nominal_curve_rate: f64,
    pub real_curve_rate: f64,
}

/// Pricing response with MtM and Greeks.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JyPricingResponse {
    /// Mark-to-Market value.
    pub mtm: f64,
    /// Inflation leg present value.
    pub inflation_leg_pv: f64,
    /// Fixed leg present value.
    pub fixed_leg_pv: f64,
    /// Risk sensitivities.
    pub greeks: JyGreeks,
}

/// Greeks (risk sensitivities) for a JY instrument.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JyGreeks {
    /// DV01 with respect to nominal curve (1bp bump).
    pub dv01_nominal: f64,
    /// DV01 with respect to real curve (1bp bump).
    pub dv01_real: f64,
    /// Vega: sensitivity to nominal volatility (1% bump).
    pub vega_nominal: f64,
    /// Vega: sensitivity to real volatility (1% bump).
    pub vega_real: f64,
    /// Vega: sensitivity to inflation volatility (1% bump).
    pub vega_inflation: f64,
    /// Theta: time decay (1 day).
    pub theta: f64,
}

// ─── XVA ─────────────────────────────────────────────────────────────────────

/// Request to compute XVA adjustments.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct JyXvaRequest {
    #[validate(nested)]
    pub model_params: JyModelParams,
    #[validate(nested)]
    pub correlation: JyCorrelation,
    pub initial_nominal_rate: f64,
    pub initial_real_rate: f64,
    #[validate(range(min = 0.01))]
    pub initial_index: f64,
    #[validate(range(min = 1.0))]
    pub notional: f64,
    pub fixed_rate: f64,
    #[validate(range(min = 0.1, max = 50.0))]
    pub maturity: f64,
    pub nominal_curve_rate: f64,
    pub real_curve_rate: f64,
    /// Counterparty annual default probability.
    #[validate(range(min = 0.0, max = 1.0))]
    pub counterparty_pd: f64,
    /// Counterparty recovery rate.
    #[validate(range(min = 0.0, max = 1.0))]
    pub counterparty_recovery: f64,
    /// Own annual default probability.
    #[validate(range(min = 0.0, max = 1.0))]
    pub own_pd: f64,
    /// Own recovery rate.
    #[validate(range(min = 0.0, max = 1.0))]
    pub own_recovery: f64,
    /// Funding spread (annual).
    pub funding_spread: f64,
    /// Number of Monte Carlo paths.
    #[validate(range(min = 100, max = 100000))]
    pub num_paths: u32,
    /// Number of time steps.
    #[validate(range(min = 10, max = 5000))]
    pub num_steps: u32,
}

/// XVA computation response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JyXvaResponse {
    /// Credit Valuation Adjustment.
    pub cva: f64,
    /// Debit Valuation Adjustment.
    pub dva: f64,
    /// Funding Valuation Adjustment.
    pub fva: f64,
    /// Total XVA (CVA + DVA + FVA).
    pub total_xva: f64,
    /// Clean (risk-free) MtM.
    pub clean_mtm: f64,
    /// Adjusted MtM (clean + total XVA).
    pub adjusted_mtm: f64,
    /// Exposure profile over time.
    pub exposure_profile: ExposureProfile,
}

/// Exposure profile for XVA visualisation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExposureProfile {
    /// Time grid (years).
    pub time_grid: Vec<f64>,
    /// Expected Exposure (EE).
    pub expected_exposure: Vec<f64>,
    /// Negative Expected Exposure (ENE).
    pub negative_expected_exposure: Vec<f64>,
    /// Potential Future Exposure at 95th percentile.
    pub pfe_95: Vec<f64>,
    /// Potential Future Exposure at 99th percentile.
    pub pfe_99: Vec<f64>,
}
