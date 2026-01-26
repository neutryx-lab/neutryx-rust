//! FX Curve API type definitions for the WebApp.
//!
//! This module provides request/response types for the FX curve construction
//! API, integrating with the new FxMarketBuilder calibration framework.
//!
//! # API Endpoints Coverage
//!
//! - `POST /api/fxcurve/build` → `FxCurveBuildRequest`, `FxCurveBuildResponse`
//! - `POST /api/fxcurve/market` → `FxMarketBuildRequest`,
//!   `FxMarketBuildResponse`
//!
//! # Requirements Coverage
//!
//! - Requirement 12.1: FXカーブ構築APIエンドポイント
//! - Requirement 12.2: テナーごとのフォワードポイント返却
//! - Requirement 12.6: カリブレーション診断
//! - Requirement 12.7: 失敗時詳細エラーメッセージ

use serde::{Deserialize, Serialize};

// =============================================================================
// FX Swap Input Types
// =============================================================================

/// FX swap input data for curve construction.
///
/// Represents an FX swap instrument with spot rate and swap points.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxSwapInput {
    /// Tenor label (e.g., "1W", "1M", "3M", "6M", "1Y")
    pub tenor: String,
    /// Time to maturity in years
    pub expiry: f64,
    /// Swap points (forward points)
    pub swap_points: f64,
    /// Scaling factor (default: 10000 for EURUSD, 100 for USDJPY)
    #[serde(default = "default_scaling_factor")]
    pub scaling_factor: f64,
}

fn default_scaling_factor() -> f64 { 10000.0 }

/// XCCY basis swap input for long-tenor curve construction.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XccyBasisInput {
    /// Tenor label (e.g., "2Y", "5Y", "10Y")
    pub tenor: String,
    /// Time to maturity in years
    pub expiry: f64,
    /// Basis spread in basis points
    pub basis_spread_bps: f64,
}

/// Prebuilt discount curve reference.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscountCurveRef {
    /// Curve identifier
    pub curve_id: String,
    /// Currency code
    pub currency: String,
}

// =============================================================================
// FX Curve Build Request/Response
// =============================================================================

/// Request for `POST /api/fxcurve/build`.
///
/// # Requirements Coverage
///
/// - Requirement 12.1: FXカーブ構築APIエンドポイント
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxCurveBuildRequest {
    /// Currency pair (e.g., "EURUSD", "USDJPY")
    pub currency_pair: String,
    /// Reference date (ISO 8601 format)
    pub reference_date: String,
    /// Spot FX rate
    pub spot_rate: f64,
    /// FX swap instruments (short-term, typically ≤1Y)
    #[serde(default)]
    pub fx_swaps: Vec<FxSwapInput>,
    /// XCCY basis swaps (long-term, typically 2Y-30Y)
    #[serde(default)]
    pub xccy_basis_swaps: Vec<XccyBasisInput>,
    /// Domestic discount curve reference (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domestic_curve: Option<DiscountCurveRef>,
    /// Foreign discount curve reference (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_curve: Option<DiscountCurveRef>,
    /// Domestic OIS rate (used if no curve reference)
    #[serde(default)]
    pub domestic_rate: f64,
    /// Foreign OIS rate (used if no curve reference)
    #[serde(default)]
    pub foreign_rate: f64,
    /// Transition region start (years, default 1.0)
    #[serde(default = "default_transition_start")]
    pub transition_start: f64,
    /// Transition region end (years, default 2.0)
    #[serde(default = "default_transition_end")]
    pub transition_end: f64,
}

fn default_transition_start() -> f64 { 1.0 }

fn default_transition_end() -> f64 { 2.0 }

/// Forward point data at a specific tenor.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardPointData {
    /// Tenor label (e.g., "1M", "6M", "2Y")
    pub tenor: String,
    /// Time to maturity in years
    pub expiry: f64,
    /// Forward points
    pub forward_points: f64,
    /// Forward FX rate
    pub forward_rate: f64,
    /// Source of this point (FxSwap, XccyBasis, Interpolated)
    pub source: String,
}

/// Calibration diagnostics for FX curve construction.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxCurveDiagnostics {
    /// Number of FX swap inputs
    pub fx_swap_count: usize,
    /// Number of XCCY basis inputs
    pub xccy_basis_count: usize,
    /// Total calibration time in milliseconds
    pub calibration_time_ms: f64,
    /// Maximum repricing error (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_repricing_error: Option<f64>,
    /// Convergence status
    pub converged: bool,
    /// Warnings (e.g., extrapolation, missing data)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Response for `POST /api/fxcurve/build`.
///
/// # Requirements Coverage
///
/// - Requirement 12.2: テナーごとのフォワードポイント返却
/// - Requirement 12.6: カリブレーション診断
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxCurveBuildResponse {
    /// Unique curve identifier
    pub curve_id: String,
    /// Currency pair
    pub currency_pair: String,
    /// Reference date
    pub reference_date: String,
    /// Spot rate
    pub spot_rate: f64,
    /// Forward point data by tenor
    pub forward_points: Vec<ForwardPointData>,
    /// Calibration diagnostics
    pub diagnostics: FxCurveDiagnostics,
}

// =============================================================================
// FX Market Build Request/Response (Full Pipeline)
// =============================================================================

/// Volatility quote input for market construction.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolQuoteInput {
    /// Time to expiry in years
    pub expiry: f64,
    /// ATM volatility
    pub atm_vol: f64,
    /// 25D Risk Reversal
    pub rr_25d: f64,
    /// 25D Butterfly
    pub bf_25d: f64,
    /// 10D Risk Reversal (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rr_10d: Option<f64>,
    /// 10D Butterfly (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bf_10d: Option<f64>,
}

/// Request for `POST /api/fxcurve/market`.
///
/// Full FX market construction including curves and volatility surface.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxMarketBuildRequest {
    /// Currency pair
    pub currency_pair: String,
    /// Reference date
    pub reference_date: String,
    /// Spot rate
    pub spot_rate: f64,
    /// Domestic OIS rate
    pub domestic_rate: f64,
    /// Foreign OIS rate
    pub foreign_rate: f64,
    /// FX swap instruments
    #[serde(default)]
    pub fx_swaps: Vec<FxSwapInput>,
    /// Volatility quotes
    #[serde(default)]
    pub vol_quotes: Vec<VolQuoteInput>,
    /// Use lazy volatility surface calibration
    #[serde(default)]
    pub lazy_vol_surface: bool,
}

/// Calibrated smile data at a specific expiry.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibratedSmileData {
    /// Time to expiry
    pub expiry: f64,
    /// Expiry label (e.g., "3M", "1Y")
    pub label: String,
    /// ATM volatility
    pub atm_vol: f64,
    /// SABR alpha parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sabr_alpha: Option<f64>,
    /// SABR beta parameter (usually fixed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sabr_beta: Option<f64>,
    /// SABR rho parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sabr_rho: Option<f64>,
    /// SABR nu parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sabr_nu: Option<f64>,
    /// Calibration residual (fit error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration_residual: Option<f64>,
}

/// Full market diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxMarketDiagnosticsResponse {
    /// Domestic curve construction time (ms)
    pub domestic_curve_time_ms: f64,
    /// Foreign curve construction time (ms)
    pub foreign_curve_time_ms: f64,
    /// FX curve construction time (ms)
    pub fx_curve_time_ms: f64,
    /// Vol surface calibration time (ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_surface_time_ms: Option<f64>,
    /// Total construction time (ms)
    pub total_time_ms: f64,
    /// Vol surface calibration iterations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_calibration_iterations: Option<usize>,
    /// Vol surface is lazy (deferred calibration)
    pub vol_surface_lazy: bool,
}

/// Response for `POST /api/fxcurve/market`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxMarketBuildResponse {
    /// Unique market identifier
    pub market_id: String,
    /// Currency pair
    pub currency_pair: String,
    /// Reference date
    pub reference_date: String,
    /// Spot rate
    pub spot_rate: f64,
    /// Forward points data
    pub forward_points: Vec<ForwardPointData>,
    /// Calibrated smile data (if vol surface was built)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calibrated_smiles: Vec<CalibratedSmileData>,
    /// Diagnostics
    pub diagnostics: FxMarketDiagnosticsResponse,
}

// =============================================================================
// Error Types
// =============================================================================

/// FX curve construction error details.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxCurveErrorDetail {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Failed construction stage
    pub stage: String,
    /// Partial results (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_result: Option<serde_json::Value>,
}
