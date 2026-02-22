//! DTOs for Markov Functional Model (MFM) demo endpoints.

use serde::{Deserialize, Serialize};
use validator::Validate;

// ─── Enums ──────────────────────────────────────────────────────────────────

/// Vol type for MFM calibration.
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum MfmVolTypeDto {
    #[default]
    Normal,
    Lognormal,
}

/// Vol surface type for calibration.
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum MfmVolSurfaceType {
    #[default]
    Flat,
    Sabr,
}

/// Product type for pricing.
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum MfmProductType {
    #[default]
    BermudanSwaption,
    CallableInverseFloater,
    Tarn,
}

// ─── Calibration Request/Response ───────────────────────────────────────────

/// Flat vol surface parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatVolParams {
    /// Normal vol in basis points (e.g. 80.0 for 80bp).
    pub normal_vol_bp: f64,
}

/// SABR vol surface parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SabrVolParams {
    pub expiries: Vec<f64>,
    pub tenors: Vec<f64>,
    pub alphas: Vec<f64>,
    pub betas: Vec<f64>,
    pub rhos: Vec<f64>,
    pub nus: Vec<f64>,
}

/// Yield curve specification (flat rate for demo).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatCurveSpec {
    /// Continuous zero rate (e.g. 0.03 for 3%).
    pub rate: f64,
}

/// MFM calibration request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct MfmCalibrateRequest {
    // ── Model parameters ──
    /// Mean reversion speed (a > 0).
    pub mean_reversion: f64,
    /// Gaussian volatility (sigma > 0).
    pub volatility: f64,
    /// Number of grid points (odd, e.g. 41).
    #[serde(default = "default_num_grid_points")]
    pub num_grid_points: usize,
    /// Number of standard deviations for grid truncation (e.g. 5.0).
    #[serde(default = "default_num_std_devs")]
    pub num_std_devs: f64,
    /// Vol type: normal (Bachelier) or lognormal (Black).
    #[serde(default)]
    pub vol_type: MfmVolTypeDto,

    // ── Schedule ──
    /// Exercise times (year fractions from today).
    pub exercise_times: Vec<f64>,
    /// Swap tenors per exercise date.
    pub swap_tenors: Vec<f64>,
    /// Payment frequencies per exercise date (year fractions, e.g. 0.5 for semi-annual).
    pub payment_frequencies: Vec<f64>,

    // ── Curves ──
    /// Funding (OIS) curve specification.
    pub funding_curve: FlatCurveSpec,
    /// Coupon (Libor/EURIBOR) curve specification.
    pub coupon_curve: FlatCurveSpec,

    // ── Vol surface ──
    /// Vol surface type.
    #[serde(default)]
    pub vol_surface_type: MfmVolSurfaceType,
    /// Flat vol parameters (when vol_surface_type = flat).
    pub flat_vol: Option<FlatVolParams>,
    /// SABR vol parameters (when vol_surface_type = sabr).
    pub sabr_vol: Option<SabrVolParams>,
}

fn default_num_grid_points() -> usize { 41 }
fn default_num_std_devs() -> f64 { 5.0 }

/// Calibrated slice data for one exercise date.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibratedSliceDto {
    pub exercise_time: f64,
    pub x_grid: Vec<f64>,
    pub swap_rates: Vec<f64>,
    pub discount_factors: Vec<f64>,
    pub annuities: Vec<f64>,
}

/// Rate index calibration result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateIndexCalibrationDto {
    pub rate_index: String,
    pub slices: Vec<CalibratedSliceDto>,
}

/// Integral adjuster result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegralAdjusterDto {
    pub adders: Vec<f64>,
    pub multipliers: Vec<f64>,
}

/// MFM calibration response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MfmCalibrateResponse {
    pub funding_calibration: RateIndexCalibrationDto,
    pub coupon_swap_calibration: RateIndexCalibrationDto,
    pub coupon_libor_calibration: RateIndexCalibrationDto,
    pub adjuster: IntegralAdjusterDto,
    pub max_nr_iterations_used: usize,
    pub max_calibration_error: f64,
    pub computation_time_ms: f64,
}

// ─── Gaussian Tree Request/Response ─────────────────────────────────────────

/// Gaussian tree construction request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GaussianTreeRequest {
    pub mean_reversion: f64,
    pub volatility: f64,
    /// Time grid (year fractions).
    pub times: Vec<f64>,
    #[serde(default = "default_num_std_devs")]
    pub num_std_devs: f64,
    #[serde(default = "default_num_grid_points")]
    pub num_grid_points: usize,
}

/// Transition probabilities for a single node.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionDto {
    pub p_down: f64,
    pub p_mid: f64,
    pub p_up: f64,
    pub j_center: usize,
}

/// Gaussian tree slice response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GaussianTreeSliceDto {
    pub time: f64,
    pub x_grid: Vec<f64>,
    pub dx: f64,
    pub conditional_variance: f64,
}

/// Gaussian tree response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GaussianTreeResponse {
    pub num_steps: usize,
    pub num_nodes: usize,
    pub slices: Vec<GaussianTreeSliceDto>,
    pub arrow_debreu_prices: Vec<Vec<f64>>,
    pub computation_time_ms: f64,
}

// ─── CIF Evaluation Request/Response ────────────────────────────────────────

/// CIF instrument definition.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CifInstrumentDto {
    /// Fixed rate (e.g. 0.07 for 7%).
    pub fixed_rate: f64,
    /// Leverage on floating rate (e.g. 1.0).
    pub leverage: f64,
    /// Floor rate (e.g. 0.0).
    pub floor_rate: f64,
    /// Optional cap rate.
    pub cap_rate: Option<f64>,
    /// Notional amount.
    pub notional: f64,
}

/// CIF evaluation request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CifEvaluateRequest {
    /// CIF instrument definition.
    pub instrument: CifInstrumentDto,
    /// Coupon dates (year fractions).
    pub coupon_dates: Vec<f64>,
    /// Payment dates (year fractions).
    pub payment_dates: Vec<f64>,
    /// Year fractions for each coupon period.
    pub year_fractions: Vec<f64>,
    /// Swap rates per node at each coupon date (flattened: n_coupons × n_nodes).
    pub swap_rates: Vec<Vec<f64>>,
    /// Libor rates per node at each coupon date.
    pub libor_rates: Vec<Vec<f64>>,
    /// Discount factors per node at each coupon date.
    pub discount_factors: Vec<Vec<f64>>,
    /// Forward swap rate per coupon date.
    pub forward_swap_rates: Vec<f64>,
    /// Forward Libor per coupon date.
    pub forward_libors: Vec<f64>,
    /// Normal vol per coupon date.
    pub normal_vols: Vec<f64>,
}

/// 4-component decomposition per coupon.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CifComponentsDto {
    pub d_e: Vec<f64>,
    pub d_r: Vec<f64>,
    pub d_i: Vec<f64>,
    pub d_q: Vec<f64>,
    pub total: Vec<f64>,
}

/// CIF node info for one coupon date.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CifCouponInfoDto {
    pub coupon_idx: usize,
    pub coupon_date_yf: f64,
    pub payment_date_yf: f64,
    pub year_fraction: f64,
    pub forward_swap_rate: f64,
    pub forward_libor: f64,
    pub normal_vol: f64,
    pub components: CifComponentsDto,
    pub discounted_values: Vec<f64>,
}

/// CIF evaluation response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CifEvaluateResponse {
    pub coupons: Vec<CifCouponInfoDto>,
    pub computation_time_ms: f64,
}

// ─── Bermudan Pricing Request/Response ──────────────────────────────────────

/// Bermudan swaption pricing request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct BermudanPriceRequest {
    // ── MFM calibration params (reused) ──
    pub mean_reversion: f64,
    pub volatility: f64,
    #[serde(default = "default_num_grid_points")]
    pub num_grid_points: usize,
    #[serde(default = "default_num_std_devs")]
    pub num_std_devs: f64,
    #[serde(default)]
    pub vol_type: MfmVolTypeDto,

    // ── Schedule ──
    pub exercise_times: Vec<f64>,
    pub swap_tenors: Vec<f64>,
    pub payment_frequencies: Vec<f64>,

    // ── Curves ──
    pub funding_curve: FlatCurveSpec,
    pub coupon_curve: FlatCurveSpec,

    // ── Vol surface ──
    #[serde(default)]
    pub vol_surface_type: MfmVolSurfaceType,
    pub flat_vol: Option<FlatVolParams>,
    pub sabr_vol: Option<SabrVolParams>,

    // ── Bermudan options ──
    /// true = callable (issuer exercises), false = puttable.
    #[serde(default = "default_true")]
    pub is_callable: bool,

    // ── Coupon node values (pre-computed or flat) ──
    /// Flat coupon value per period (simplified). If absent, zero coupons assumed.
    pub flat_coupon: Option<f64>,
}

fn default_true() -> bool { true }

/// Bermudan pricing response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BermudanPriceResponse {
    pub pv: f64,
    pub continuation_value: f64,
    pub option_value: f64,
    pub exercise_boundary: Vec<f64>,
    pub computation_time_ms: f64,
}

// ─── TARN Pricing Request/Response ──────────────────────────────────────────

/// TARN pricing request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TarnPriceRequest {
    // ── MFM calibration params ──
    pub mean_reversion: f64,
    pub volatility: f64,
    #[serde(default = "default_num_grid_points")]
    pub num_grid_points: usize,
    #[serde(default = "default_num_std_devs")]
    pub num_std_devs: f64,
    #[serde(default)]
    pub vol_type: MfmVolTypeDto,

    // ── Schedule ──
    pub exercise_times: Vec<f64>,
    pub swap_tenors: Vec<f64>,
    pub payment_frequencies: Vec<f64>,

    // ── Curves ──
    pub funding_curve: FlatCurveSpec,
    pub coupon_curve: FlatCurveSpec,

    // ── Vol surface ──
    #[serde(default)]
    pub vol_surface_type: MfmVolSurfaceType,
    pub flat_vol: Option<FlatVolParams>,
    pub sabr_vol: Option<SabrVolParams>,

    // ── TARN config ──
    /// Target cumulative coupon amount.
    pub tarn_amount: f64,
    /// Number of cumulative coupon grid points.
    #[serde(default = "default_tarn_grid_points")]
    pub num_coupon_grid_points: usize,
    /// Whether to pay excess above target.
    #[serde(default)]
    pub excess_coupon_flag: bool,
    /// Whether Bermudan exercise is also available.
    #[serde(default)]
    pub has_bermudan_exercise: bool,
    /// Callable vs puttable (for Bermudan).
    #[serde(default = "default_true")]
    pub is_callable: bool,

    // ── Coupon ──
    /// Flat coupon rate per period for simplified demo.
    pub flat_coupon: Option<f64>,
}

fn default_tarn_grid_points() -> usize { 10 }

/// TARN pricing response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TarnPriceResponse {
    pub pv: f64,
    pub auto_redemption_probability: f64,
    pub expected_redemption_time: f64,
    pub computation_time_ms: f64,
}

// ─── Product definitions for dynamic form ───────────────────────────────────

/// MFM product definition for the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MfmProductDef {
    pub product_type: String,
    pub display_name: String,
    pub description: String,
    pub parameters: Vec<MfmParameterDef>,
}

/// Parameter definition for MFM forms.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MfmParameterDef {
    pub name: String,
    pub display_name: String,
    pub field_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub group: Option<String>,
}
