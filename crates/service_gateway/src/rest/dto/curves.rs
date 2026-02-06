//! Curve-related DTOs

use serde::{Deserialize, Serialize};

/// Interpolation method for curve building
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationMethod {
    /// Linear interpolation on discount factors
    Linear,
    /// Log-linear interpolation (linear on log of discount factors)
    #[default]
    LogLinear,
    /// Cubic spline interpolation
    CubicSpline,
}

/// Bootstrap method for curve building
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapMethod {
    #[default]
    Sequential,
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
    pub interpolation: InterpolationMethod,
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
    /// Time in years
    pub time: f64,
    /// Discount factor at this pillar
    pub discount_factor: f64,
    /// Zero rate (continuously compounded) at this pillar
    pub zero_rate: f64,
}

/// Forward rate point on a daily grid
#[derive(Debug, Clone, Serialize)]
pub struct ForwardRatePoint {
    /// Time in years
    pub time: f64,
    /// Forward rate (simple, annualised) for the next day
    pub forward_rate: f64,
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
    /// Number of instruments used
    pub instrument_count: usize,
    /// Bootstrap convergence achieved
    pub converged: bool,
    /// Calculation time in milliseconds
    pub calculation_time_ms: f64,
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
    pub curve_id: String,
    pub time: f64,
    pub discount_factor: f64,
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
    pub curve_id: String,
    pub start_time: f64,
    pub end_time: f64,
    pub forward_rate: f64,
}

/// Request to compute bucket DV01
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct BucketDv01Request {
    /// Curve ID from previous build
    pub curve_id: String,
    /// Notional amount
    pub notional: f64,
    /// Fixed rate of the swap
    pub fixed_rate: f64,
    /// Tenor in years
    pub tenor_years: f64,
    /// Bump size in basis points
    #[serde(default = "default_bump_bps")]
    pub bump_size_bps: f64,
}

fn default_bump_bps() -> f64 { 1.0 }

/// Single bucket DV01 result
#[derive(Debug, Clone, Serialize)]
pub struct BucketDv01Result {
    /// Tenor bucket label (e.g., "1Y", "2Y")
    pub tenor: String,
    /// Time in years
    pub time: f64,
    /// DV01 for this bucket
    pub dv01: f64,
}

/// Response for bucket DV01 calculation
#[derive(Debug, Clone, Serialize)]
pub struct BucketDv01Response {
    /// Curve ID
    pub curve_id: String,
    /// Total DV01 (sum of all buckets)
    pub total_dv01: f64,
    /// Per-bucket DV01 results
    pub buckets: Vec<BucketDv01Result>,
    /// Calculation time in milliseconds
    pub calculation_time_ms: f64,
}
