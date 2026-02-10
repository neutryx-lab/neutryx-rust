//! Curve-related DTOs

use pricer_models::market::BootstrapInterpolation;
use serde::{Deserialize, Serialize};

/// Bootstrap method for curve building
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapMethod {
    /// Iterative bootstrapping (default)
    #[default]
    Bootstrapping,
    /// Global optimisation
    Global,
}

/// Single instrument input for curve building
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CurveInstrumentInput {
    /// Instrument type (e.g., "deposit", "fra", "swap", "event")
    #[serde(alias = "type")]
    pub instrument_type: String,
    /// Tenor string (e.g., "1M", "3M", "1Y") - optional for event type
    #[serde(default)]
    pub tenor: String,
    /// Par rate (as decimal, e.g., 0.05 for 5%) - optional for event type
    #[serde(default)]
    pub rate: f64,
    /// Event date for CB meetings/turn-of-year (ISO format, e.g., "2026-03-18")
    #[serde(default)]
    pub event_date: Option<String>,
    /// Expected rate spike at event (e.g., -0.0025 for -25bp cut)
    #[serde(default)]
    pub expected_rate_spike: Option<f64>,
    /// End date for turn events — spike reverts after this date (ISO format).
    /// If present, the event is treated as a temporary turn (paired entries).
    /// If absent, the event is treated as a permanent jump.
    #[serde(default)]
    pub end_date: Option<String>,
}

/// Request to build a yield curve
#[derive(Debug, Clone, Deserialize)]
pub struct CurveBuildRequest {
    /// Index name (e.g., "USD-SOFR", "EUR-EURIBOR-6M")
    pub index: String,
    /// Currency code (optional, extracted from index if not provided)
    #[serde(default)]
    pub currency: String,
    /// Reference date for curve building (ISO format, e.g., "2026-01-29").
    /// Defaults to today if not provided.
    #[serde(default)]
    pub reference_date: Option<String>,
    /// Market instruments for bootstrapping
    pub instruments: Vec<CurveInstrumentInput>,
    /// Interpolation method
    #[serde(default)]
    pub interpolation: BootstrapInterpolation,
    /// Bootstrap method
    #[serde(default)]
    pub bootstrap_method: BootstrapMethod,
    /// Tolerance for bootstrap convergence
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    /// Maximum iterations for bootstrap
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}

fn default_tolerance() -> f64 { 1e-10 }

fn default_max_iterations() -> usize { 100 }

/// Pillar point in a bootstrapped curve
#[derive(Debug, Clone, Serialize)]
pub struct CurvePillar {
    /// Date (ISO 8601, e.g. "2026-03-18")
    pub date: String,
    /// Time in years from reference date
    pub time: f64,
    /// Discount factor at this pillar
    pub discount_factor: f64,
    /// Zero rate (continuously compounded) at this pillar
    pub zero_rate: f64,
    /// Instantaneous forward rate (simple, annualised) at this pillar
    pub forward_rate: f64,
}

/// Forward rate point on a daily grid
#[derive(Debug, Clone, Serialize)]
pub struct ForwardRatePoint {
    /// Date (ISO 8601)
    pub date: String,
    /// Time in years from reference date
    pub time: f64,
    /// Forward rate (simple, annualised) for the next day
    pub forward_rate: f64,
}

/// A single point on a pre-computed chart display grid.
///
/// Contains all data needed for frontend rendering without further computation.
#[derive(Debug, Clone, Serialize)]
pub struct ChartGridPoint {
    /// Date (ISO 8601, e.g. "2026-03-18")
    pub date: String,
    /// Time in years from reference date (ACT/365)
    pub time: f64,
    /// Discount factor at this grid point
    pub discount_factor: f64,
    /// Forward rate (simple, annualised) at this grid point
    pub forward_rate: f64,
    /// Date label for chart axis (e.g. "15-Jan" or "Mar-26")
    pub label: String,
}

/// Jacobian matrix data for curve sensitivity analysis.
///
/// Contains the normalised Jacobian `[d(log DF_i) / T_i] / dr_j  ≈  −dz_i /
/// dr_j` where:
/// - Row i corresponds to pillar i (zero-rate sensitivity)
/// - Column j corresponds to instrument j (market rate `r_j`)
/// - `T_i` is the year-fraction to pillar i
///
/// Dividing by `T_i` converts raw log-DF sensitivity into zero-rate
/// units, giving comparable magnitudes across all maturities.
///
/// For bootstrapping, this matrix is lower-triangular.
#[derive(Debug, Clone, Serialize)]
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

/// Response for curve building
#[derive(Debug, Clone, Serialize)]
pub struct CurveBuildResponse {
    /// Generated curve ID for caching
    pub curve_id: String,
    /// Index name
    pub index: String,
    /// Currency code
    pub currency: String,
    /// Pillar points (bootstrap nodes)
    pub pillars: Vec<CurvePillar>,
    /// Forward rate curve on daily grid
    pub forward_curve: Vec<ForwardRatePoint>,
    /// Short-term chart grid (0-1Y): daily up to 3M, weekly 3M-1Y
    pub short_term_grid: Vec<ChartGridPoint>,
    /// Long-term chart grid (0-30Y): quarterly 3M-10Y, semi-annual 10.5Y-20Y,
    /// annual 21Y-30Y
    pub long_term_grid: Vec<ChartGridPoint>,
    /// Number of instruments used
    pub instrument_count: usize,
    /// Interpolation method used (for display)
    pub interpolation: String,
    /// Bootstrap convergence achieved
    pub converged: bool,
    /// Calculation time in milliseconds
    pub calculation_time_ms: f64,
    /// Actual bootstrap method used (may differ from request if fallback
    /// occurred)
    pub bootstrap_method: String,
    /// Jacobian matrix d(log DF)/dr (finite-difference)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jacobian: Option<JacobianData>,
}

/// Request to get discount factor from a cached curve
#[derive(Debug, Clone, Deserialize)]
pub struct DiscountFactorRequest {
    /// Curve ID from previous build
    pub curve_id: String,
    /// Time in years
    pub time: f64,
}

/// Response with discount factor
#[derive(Debug, Clone, Serialize)]
pub struct DiscountFactorResponse {
    /// Curve ID used for the lookup
    pub curve_id: String,
    /// Time in years
    pub time: f64,
    /// Discount factor at the given time
    pub discount_factor: f64,
    /// Zero rate (continuously compounded) at the given time
    pub zero_rate: f64,
}

/// Request to get forward rate from a cached curve
#[derive(Debug, Clone, Deserialize)]
pub struct ForwardRateRequest {
    /// Curve ID from previous build
    pub curve_id: String,
    /// Start time in years
    pub start_time: f64,
    /// End time in years
    pub end_time: f64,
}

/// Response with forward rate
#[derive(Debug, Clone, Serialize)]
pub struct ForwardRateResponse {
    /// Curve ID used for the lookup
    pub curve_id: String,
    /// Start time in years
    pub start_time: f64,
    /// End time in years
    pub end_time: f64,
    /// Forward rate for the period
    pub forward_rate: f64,
}

/// Request to compute forward swap rates from a cached curve
#[derive(Debug, Clone, Deserialize)]
pub struct ForwardSwapRateRequest {
    /// Curve ID from previous build
    pub curve_id: String,
    /// Expiry tenor strings (e.g., "1M", "3M", "1Y", "5Y", "10Y")
    pub expiries: Vec<String>,
    /// Swap tenor strings (e.g., "1Y", "2Y", "5Y", "10Y", "30Y")
    pub tenors: Vec<String>,
}

/// Response with forward swap rate matrix
#[derive(Debug, Clone, Serialize)]
pub struct ForwardSwapRateResponse {
    /// Curve ID used
    pub curve_id: String,
    /// Forward swap rates keyed by "expiry|tenor" (e.g., "1Y|5Y" -> 0.045)
    pub rates: std::collections::HashMap<String, f64>,
    /// Calculation time in milliseconds
    pub calculation_time_ms: f64,
}
