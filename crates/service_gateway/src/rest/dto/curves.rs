//! Curve-related DTOs.

use pricer_models::market::BootstrapInterpolation;
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;
use validator::Validate;

/// Bootstrap method for curve building.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum BootstrapMethod {
    /// Iterative bootstrapping (default).
    #[default]
    Bootstrapping,
    /// Global optimisation.
    Global,
    /// Levenberg-Marquardt non-linear least squares.
    LevenbergMarquardt,
    /// Penalised (regularised) global calibration with forward smoothness penalty.
    Penalised,
    /// Best fit via QR least squares (for overdetermined systems).
    BestFit,
}

/// Curve type discriminator.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum CurveType {
    /// Interest rate curve (default).
    #[default]
    Rate,
    /// Credit (survival probability) curve bootstrapped from CDS spreads.
    Credit,
    /// FX forward curve.
    Fx,
}

/// FX curve construction method.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum FxCurveMethod {
    /// Flat forward points.
    #[default]
    Flat,
    /// Interest Rate Parity using bootstrapped yield curves.
    IrpGeneric,
}

/// Single instrument input for curve building.
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CurveInstrumentInput {
    /// Instrument type (e.g., "deposit", "fra", "swap", "event").
    #[serde(alias = "type")]
    #[validate(length(min = 1))]
    pub instrument_type: String,
    /// Tenor string (e.g., "1M", "3M", "1Y") - optional for event type.
    #[serde(default)]
    pub tenor: String,
    /// Par rate (as decimal, e.g., 0.05 for 5%) - optional for event type.
    #[serde(default)]
    pub rate: f64,
    /// Event date for CB meetings/turn-of-year (ISO format, e.g.,
    /// "2026-03-18").
    #[serde(default)]
    pub event_date: Option<String>,
    /// Expected rate spike at event (e.g., -0.0025 for -25bp cut).
    #[serde(default)]
    pub expected_rate_spike: Option<f64>,
    /// End date for turn events — spike reverts after this date (ISO format).
    #[serde(default)]
    pub end_date: Option<String>,
    /// Coupon rate for Bond instruments (as decimal, e.g., 0.04 for 4%).
    #[serde(default)]
    pub coupon_rate: Option<f64>,
}

/// Request to build a yield curve.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CurveBuildRequest {
    /// Index name (e.g., "USD-SOFR", "EUR-EURIBOR-6M").
    #[validate(length(min = 1))]
    pub index: String,
    /// Currency code (optional, extracted from index if not provided).
    #[serde(default)]
    pub currency: String,
    /// Reference date for curve building (ISO format, e.g., "2026-01-29").
    #[serde(default)]
    pub reference_date: Option<String>,
    /// Market instruments for bootstrapping (may be empty for FX IRP curves).
    #[validate(nested)]
    pub instruments: Vec<CurveInstrumentInput>,
    /// Interpolation method.
    #[serde(default)]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub interpolation: BootstrapInterpolation,
    /// Bootstrap method.
    #[serde(default)]
    pub bootstrap_method: BootstrapMethod,
    /// Tolerance for bootstrap convergence.
    #[serde(default = "default_tolerance")]
    #[validate(range(exclusive_min = 0.0))]
    pub tolerance: f64,
    /// Maximum iterations for bootstrap.
    #[serde(default = "default_max_iterations")]
    #[validate(range(min = 1))]
    pub max_iterations: usize,
    /// Type of curve to build (rate or credit).
    #[serde(default)]
    pub curve_type: CurveType,
    /// ID of a previously built risk-free discount curve (required for credit
    /// curves).
    #[serde(default)]
    pub discount_curve_id: Option<String>,
    /// Recovery rate for CDS instruments (default 0.40).
    #[serde(default = "default_recovery_rate")]
    pub recovery_rate: f64,
    /// Tension parameter for tension spline interpolation (default 1.0).
    #[serde(default)]
    pub tension: Option<f64>,
    /// Penalty weight for penalised calibration (default 1e-4).
    #[serde(default)]
    pub penalty_weight: Option<f64>,
    /// FX curve construction method (required for FX curves).
    #[serde(default)]
    pub fx_curve_method: FxCurveMethod,
    /// Currency pair (e.g., "EURUSD") for FX curves.
    #[serde(default)]
    pub currency_pair: Option<String>,
    /// Spot FX rate (required for FX curves).
    #[serde(default)]
    pub spot: Option<f64>,
    /// ID of a previously built domestic yield curve (required for IRP
    /// methods).
    #[serde(default)]
    pub domestic_curve_id: Option<String>,
    /// ID of a previously built foreign yield curve (required for IRP
    /// methods).
    #[serde(default)]
    pub foreign_curve_id: Option<String>,
}

fn default_tolerance() -> f64 { 1e-10 }

fn default_max_iterations() -> usize { 100 }

fn default_recovery_rate() -> f64 { 0.40 }

/// Pillar point in a bootstrapped curve.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CurvePillar {
    /// Date (ISO 8601, e.g.
    pub date: String,
    /// Time in years from reference date.
    pub time: f64,
    /// Discount factor at this pillar.
    pub discount_factor: f64,
    /// Zero rate (continuously compounded) at this pillar.
    pub zero_rate: f64,
    /// Instantaneous forward rate (simple, annualised) at this pillar.
    pub forward_rate: f64,
    /// Survival probability at this pillar (credit curves only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub survival_probability: Option<f64>,
    /// Hazard rate at this pillar (credit curves only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hazard_rate: Option<f64>,
    /// FX forward rate at this pillar (FX curves only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fx_forward: Option<f64>,
}

/// Forward rate point on a daily grid.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ForwardRatePoint {
    /// Date (ISO 8601).
    pub date: String,
    /// Time in years from reference date.
    pub time: f64,
    /// Forward rate (simple, annualised) for the next day.
    pub forward_rate: f64,
}

/// A single point on a pre-computed chart display grid.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ChartGridPoint {
    /// Date (ISO 8601, e.g.
    pub date: String,
    /// Time in years from reference date (ACT/365).
    pub time: f64,
    /// Discount factor at this grid point.
    pub discount_factor: f64,
    /// Forward rate (simple, annualised) at this grid point.
    pub forward_rate: f64,
    /// Date label for chart axis (e.g.
    pub label: String,
    /// FX forward rate at this grid point (FX curves only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fx_forward: Option<f64>,
}

/// Jacobian matrix data for curve sensitivity analysis.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct JacobianData {
    /// Row labels (pillar descriptions, e.g., "Depo-1M", "IRS-5Y").
    pub row_labels: Vec<String>,
    /// Column labels (instrument descriptions).
    pub col_labels: Vec<String>,
    /// Row-major n x n matrix values.
    pub matrix: Vec<Vec<f64>>,
    /// Size of the matrix (n).
    pub size: usize,
}

/// Response for curve building.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CurveBuildResponse {
    /// Generated curve ID for caching.
    pub curve_id: String,
    /// Index name.
    pub index: String,
    /// Currency code.
    pub currency: String,
    /// Pillar points (bootstrap nodes).
    pub pillars: Vec<CurvePillar>,
    /// Forward rate curve on daily grid.
    pub forward_curve: Vec<ForwardRatePoint>,
    /// Short-term chart grid (0-1Y): daily up to 3M, weekly 3M-1Y.
    pub short_term_grid: Vec<ChartGridPoint>,
    /// Long-term chart grid (0-30Y): quarterly 3M-10Y, semi-annual 10.5Y-20Y,.
    pub long_term_grid: Vec<ChartGridPoint>,
    /// Number of instruments used.
    pub instrument_count: usize,
    /// Interpolation method used (for display).
    pub interpolation: String,
    /// Bootstrap convergence achieved.
    pub converged: bool,
    /// Calculation time in milliseconds.
    pub calculation_time_ms: f64,
    /// Actual bootstrap method used (may differ from request if fallback.
    pub bootstrap_method: String,
    /// Jacobian matrix d(log DF)/dr (finite-difference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jacobian: Option<JacobianData>,
    /// Type of curve that was built ("rate", "credit", or "fx").
    pub curve_type: String,
    /// Spot FX rate (FX curves only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spot: Option<f64>,
    /// Currency pair (FX curves only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_pair: Option<String>,
}

/// Request to get discount factor from a cached curve.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct DiscountFactorRequest {
    /// Curve ID from previous build.
    #[validate(length(min = 1))]
    pub curve_id: String,
    /// Time in years.
    #[validate(range(min = 0.0))]
    pub time: f64,
}

/// Response with discount factor.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct DiscountFactorResponse {
    /// Curve ID used for the lookup.
    pub curve_id: String,
    /// Time in years.
    pub time: f64,
    /// Discount factor at the given time.
    pub discount_factor: f64,
    /// Zero rate (continuously compounded) at the given time.
    pub zero_rate: f64,
}

/// Request to get forward rate from a cached curve.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ForwardRateRequest {
    /// Curve ID from previous build.
    #[validate(length(min = 1))]
    pub curve_id: String,
    /// Start time in years.
    #[validate(range(min = 0.0))]
    pub start_time: f64,
    /// End time in years.
    #[validate(range(exclusive_min = 0.0))]
    pub end_time: f64,
}

/// Response with forward rate.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ForwardRateResponse {
    /// Curve ID used for the lookup.
    pub curve_id: String,
    /// Start time in years.
    pub start_time: f64,
    /// End time in years.
    pub end_time: f64,
    /// Forward rate for the period.
    pub forward_rate: f64,
}

/// Request to compute forward swap rates from a cached curve.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ForwardSwapRateRequest {
    /// Curve ID from previous build.
    #[validate(length(min = 1))]
    pub curve_id: String,
    /// Expiry tenor strings (e.g., "1M", "3M", "1Y", "5Y", "10Y").
    #[validate(length(min = 1))]
    pub expiries: Vec<String>,
    /// Swap tenor strings (e.g., "1Y", "2Y", "5Y", "10Y", "30Y").
    #[validate(length(min = 1))]
    pub tenors: Vec<String>,
}

/// Response with forward swap rate matrix.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ForwardSwapRateResponse {
    /// Curve ID used.
    pub curve_id: String,
    /// Forward swap rates keyed by "expiry|tenor" (e.g., "1Y|5Y" -> 0.045).
    pub rates: std::collections::HashMap<String, f64>,
    /// Calculation time in milliseconds.
    pub calculation_time_ms: f64,
}
