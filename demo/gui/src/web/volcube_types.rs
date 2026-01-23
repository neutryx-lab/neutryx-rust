//! VolCube API type definitions for the WebApp.
//!
//! This module provides request/response types for the VolCube calibration API,
//! including instrument data, calibration parameters, and result visualisation.
//!
//! # API Endpoints Coverage
//!
//! - `GET /api/volcube/indices` → `IndicesResponse`
//! - `GET /api/volcube/instruments/{index}` → `VolCubeInstrumentListResponse`
//! - `PUT /api/volcube/instruments/{index}` → `VolCubeInstrumentListRequest`
//! - `POST /api/volcube/calibrate` → `VolCubeCalibrateRequest`,
//!   `VolCubeCalibrateResponse`
//! - `GET /api/volcube/smile` → `SmileDataResponse`
//! - `GET /api/volcube/density` → `DensityDataResponse`
//! - `GET /api/volcube/surface` → `SurfaceDataResponse`
//!
//! # Requirements Coverage
//!
//! - Requirement 1: ボラティリティデータ管理
//! - Requirement 3: VolCubeキャリブレーション設定
//! - Requirement 4: キャリブレーション結果パラメータ表示
//! - Requirement 5: スマイルカーブ可視化
//! - Requirement 6: 確率密度関数可視化
//! - Requirement 8: バックエンドAPI実装

use serde::{Deserialize, Serialize};

// =============================================================================
// Calibration Model Enums (Req 3.1)
// =============================================================================

/// Calibration model selection for VolCube construction.
///
/// Determines the parametric model used to fit the volatility smile
/// at each expiry-tenor point.
///
/// # Requirements Coverage
///
/// - Requirement 3.1: SABR、SVI、Local Volatilityモデルを選択可能
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationModel {
    /// SABR model (Stochastic Alpha Beta Rho).
    /// Standard for interest rate options and swaptions.
    #[default]
    Sabr,
    /// SVI model (Stochastic Volatility Inspired).
    /// Popular for equity volatility surfaces.
    Svi,
    /// Local Volatility model (Dupire).
    /// Non-parametric, arbitrage-free by construction.
    LocalVolatility,
}

impl CalibrationModel {
    /// Get the display name for this calibration model.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Sabr => "SABR",
            Self::Svi => "SVI",
            Self::LocalVolatility => "Local Volatility",
        }
    }

    /// Get a description of this calibration model.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Sabr => "Stochastic Alpha Beta Rho - standard for rates",
            Self::Svi => "Stochastic Volatility Inspired - popular for equity",
            Self::LocalVolatility => "Dupire's local volatility - arbitrage-free",
        }
    }

    /// Check if this model is currently implemented.
    pub fn is_enabled(&self) -> bool { matches!(self, Self::Sabr) }
}

/// Strike axis type for volatility visualisation.
///
/// Determines how strikes are represented in charts and data output.
///
/// # Requirements Coverage
///
/// - Requirement 3.3: Strike軸タイプ（Absolute、Moneyness、Log-Moneyness、Delta）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StrikeAxisType {
    /// Absolute strike value (e.g., 0.03 for 3% rate)
    #[default]
    Absolute,
    /// Strike / Forward ratio
    Moneyness,
    /// ln(Strike / Forward)
    LogMoneyness,
    /// Option delta (for FX, more intuitive)
    Delta,
}

impl StrikeAxisType {
    /// Get the display name for this strike axis type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Absolute => "Absolute Strike",
            Self::Moneyness => "Moneyness (K/F)",
            Self::LogMoneyness => "Log-Moneyness",
            Self::Delta => "Delta",
        }
    }
}

// =============================================================================
// Instrument Data Structures (Req 1.5)
// =============================================================================

/// Swaption instrument from JSON data file.
///
/// Represents a single market observation point for VolCube construction.
/// Used for deserialising from `demo/data/input/volsurface/{index}.json`.
///
/// # Requirements Coverage
///
/// - Requirement 1.5: expiry, tenor, strike, implied_vol, forward, weight fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwaptionInstrument {
    /// Time to option expiry in years
    pub expiry: f64,
    /// Underlying swap tenor in years
    pub tenor: f64,
    /// Absolute strike rate (e.g., 0.03 for 3%)
    pub strike: f64,
    /// Market implied volatility (e.g., 0.20 for 20%)
    pub implied_vol: f64,
    /// Forward swap rate at the reference date
    pub forward: f64,
    /// Calibration weight (default 1.0)
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 { 1.0 }

impl SwaptionInstrument {
    /// Create a new swaption instrument.
    pub fn new(
        expiry: f64,
        tenor: f64,
        strike: f64,
        implied_vol: f64,
        forward: f64,
    ) -> Self {
        Self {
            expiry,
            tenor,
            strike,
            implied_vol,
            forward,
            weight: 1.0,
        }
    }

    /// Create with custom weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Complete VolCube data file structure.
///
/// JSON schema for files in `demo/data/input/volsurface/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeFile {
    /// Index identifier (e.g., "usd-sofr-swaption")
    pub index: String,
    /// Reference date (ISO 8601 format)
    pub reference_date: String,
    /// List of dependent curve IDs
    #[serde(default)]
    pub dependent_curves: Vec<String>,
    /// List of swaption instruments
    pub instruments: Vec<SwaptionInstrument>,
}

// =============================================================================
// SABR Configuration (Req 3.2)
// =============================================================================

/// SABR-specific configuration parameters.
///
/// # Requirements Coverage
///
/// - Requirement 3.2: Beta固定値またはキャリブレーション、Shift値
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SabrConfigInput {
    /// Beta parameter (0.0 = normal, 0.5 = CIR, 1.0 = log-normal)
    /// If None, beta will be calibrated
    #[serde(default = "default_sabr_beta")]
    pub beta: Option<f64>,
    /// Shift for negative rates (shifted SABR)
    #[serde(default)]
    pub shift: f64,
    /// Whether to calibrate beta or use fixed value
    #[serde(default)]
    pub calibrate_beta: bool,
}

fn default_sabr_beta() -> Option<f64> { Some(0.5) }

impl Default for SabrConfigInput {
    fn default() -> Self {
        Self {
            beta: Some(0.5),
            shift: 0.0,
            calibrate_beta: false,
        }
    }
}

/// General VolCube configuration input from client.
///
/// # Requirements Coverage
///
/// - Requirement 3.3: 補間方法、外挿方法、最適化手法、許容誤差、最大反復回数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeConfigInput {
    /// SABR-specific settings
    #[serde(default)]
    pub sabr: SabrConfigInput,
    /// Interpolation method for expiry axis
    #[serde(default = "default_interpolation")]
    pub expiry_interpolation: String,
    /// Interpolation method for tenor axis
    #[serde(default = "default_interpolation")]
    pub tenor_interpolation: String,
    /// Extrapolation method
    #[serde(default = "default_extrapolation")]
    pub extrapolation: String,
    /// Optimisation tolerance
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    /// Maximum iterations
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}

fn default_interpolation() -> String { "linear".to_string() }

fn default_extrapolation() -> String { "flat".to_string() }

fn default_tolerance() -> f64 { 1e-8 }

fn default_max_iterations() -> usize { 100 }

impl Default for VolCubeConfigInput {
    fn default() -> Self {
        Self {
            sabr: SabrConfigInput::default(),
            expiry_interpolation: default_interpolation(),
            tenor_interpolation: default_interpolation(),
            extrapolation: default_extrapolation(),
            tolerance: default_tolerance(),
            max_iterations: default_max_iterations(),
        }
    }
}

// =============================================================================
// API Request Types (Req 8.2-8.4)
// =============================================================================

/// Response for `GET /api/volcube/indices`.
///
/// # Requirements Coverage
///
/// - Requirement 8.1: 利用可能なIndex一覧を返す
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeIndicesResponse {
    /// List of available index identifiers
    pub indices: Vec<VolCubeIndexInfo>,
}

/// Information about a VolCube index.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeIndexInfo {
    /// Index identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Asset class (swaption, fx_options, equity_options)
    pub asset_class: String,
    /// Currency
    pub currency: String,
}

/// Response for `GET /api/volcube/instruments/{index}`.
///
/// # Requirements Coverage
///
/// - Requirement 8.2: 指定Indexのインストゥルメントデータを返す
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeInstrumentListResponse {
    /// Index identifier
    pub index: String,
    /// Reference date
    pub reference_date: String,
    /// Dependent curve IDs
    pub dependent_curves: Vec<String>,
    /// List of instruments
    pub instruments: Vec<SwaptionInstrument>,
}

/// Request for `PUT /api/volcube/instruments/{index}`.
///
/// # Requirements Coverage
///
/// - Requirement 8.3: インストゥルメントデータを更新・保存
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeInstrumentListRequest {
    /// Reference date
    pub reference_date: String,
    /// Dependent curve IDs
    #[serde(default)]
    pub dependent_curves: Vec<String>,
    /// List of instruments
    pub instruments: Vec<SwaptionInstrument>,
}

/// Request for `POST /api/volcube/calibrate`.
///
/// # Requirements Coverage
///
/// - Requirement 8.4: キャリブレーションを実行
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeCalibrateRequest {
    /// Index identifier
    pub index: String,
    /// Instruments with rates (potentially edited by user)
    pub instruments: Vec<SwaptionInstrument>,
    /// Calibration model
    #[serde(default)]
    pub model: CalibrationModel,
    /// Configuration settings
    #[serde(default)]
    pub config: VolCubeConfigInput,
    /// Optional dependent curve ID for forward calculation
    pub dependent_curve_id: Option<String>,
}

// =============================================================================
// API Response Types (Req 4, 8.4-8.7)
// =============================================================================

/// SABR parameters output for a single expiry-tenor point.
///
/// # Requirements Coverage
///
/// - Requirement 4.2: Alpha、Beta、Rho、Nuを各(Expiry, Tenor)で表示
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SabrParamsOutput {
    /// Option expiry in years
    pub expiry: f64,
    /// Swap tenor in years
    pub tenor: f64,
    /// SABR alpha (vol of vol at ATM)
    pub alpha: f64,
    /// SABR beta (CEV exponent, 0 = normal, 1 = log-normal)
    pub beta: f64,
    /// SABR rho (correlation between spot and vol)
    pub rho: f64,
    /// SABR nu (vol of vol)
    pub nu: f64,
    /// Forward rate at this point
    pub forward: f64,
}

/// Fit quality metrics for calibration.
///
/// # Requirements Coverage
///
/// - Requirement 4.3: RMSE、最大誤差、R²、反復回数、処理時間
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitMetrics {
    /// Root Mean Square Error (vol units)
    pub rmse: f64,
    /// Maximum absolute error (vol units)
    pub max_error: f64,
    /// R-squared (coefficient of determination)
    pub r_squared: f64,
    /// Number of iterations used
    pub iterations: usize,
    /// Number of instruments used
    pub instrument_count: usize,
}

impl Default for FitMetrics {
    fn default() -> Self {
        Self {
            rmse: 0.0,
            max_error: 0.0,
            r_squared: 1.0,
            iterations: 0,
            instrument_count: 0,
        }
    }
}

/// Per-instrument fit comparison.
///
/// # Requirements Coverage
///
/// - Requirement 4.4: 市場vol vs モデルvolの比較
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentFit {
    /// Option expiry
    pub expiry: f64,
    /// Swap tenor
    pub tenor: f64,
    /// Strike
    pub strike: f64,
    /// Market implied volatility
    pub market_vol: f64,
    /// Model implied volatility
    pub model_vol: f64,
    /// Fit error (model - market)
    pub error: f64,
}

/// Response for `POST /api/volcube/calibrate`.
///
/// # Requirements Coverage
///
/// - Requirement 4.1: モデルパラメータをテーブル形式で表示
/// - Requirement 8.4: キャリブレーション結果を返す
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeCalibrateResponse {
    /// Unique cube identifier (for subsequent queries)
    pub cube_id: String,
    /// Calibration model used
    pub model: CalibrationModel,
    /// SABR parameters at each grid point
    pub parameters: Vec<SabrParamsOutput>,
    /// Overall fit quality metrics
    pub fit_metrics: FitMetrics,
    /// Per-instrument fit comparison
    pub instrument_fits: Vec<InstrumentFit>,
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
}

// =============================================================================
// Smile and Density Query Types (Req 5, 6, 8.5-8.6)
// =============================================================================

/// Query parameters for `GET /api/volcube/smile`.
///
/// # Requirements Coverage
///
/// - Requirement 5.1: Expiry/Tenor選択
/// - Requirement 8.5: スマイルデータを返す
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SmileQuery {
    /// Cube ID from calibration
    pub cube_id: String,
    /// Option expiry in years
    pub expiry: f64,
    /// Swap tenor in years
    pub tenor: f64,
    /// Strike axis type for output
    #[serde(default)]
    pub strike_axis: StrikeAxisType,
    /// Number of strike points (default 50)
    #[serde(default = "default_num_points")]
    pub num_points: usize,
}

fn default_num_points() -> usize { 50 }

/// Market observation point for smile chart.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPoint {
    /// Strike value
    pub strike: f64,
    /// Market implied volatility
    pub implied_vol: f64,
    /// Calibration weight
    pub weight: f64,
}

/// Response for `GET /api/volcube/smile`.
///
/// # Requirements Coverage
///
/// - Requirement 5.2: スマイルカーブ（Strike vs Implied Vol）を返す
/// - Requirement 5.3: 市場観測点とモデル曲線を含む
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmileDataResponse {
    /// Option expiry
    pub expiry: f64,
    /// Swap tenor
    pub tenor: f64,
    /// Forward rate at this expiry-tenor
    pub forward: f64,
    /// Strike values (model curve)
    pub strikes: Vec<f64>,
    /// Model volatilities
    pub model_vols: Vec<f64>,
    /// Market observation points
    pub market_points: Vec<MarketPoint>,
    /// SABR parameters for this slice
    pub sabr_params: SabrParamsOutput,
}

/// Query parameters for `GET /api/volcube/density`.
///
/// # Requirements Coverage
///
/// - Requirement 6.1: 確率密度表示モード
/// - Requirement 8.6: 確率密度データを返す
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DensityQuery {
    /// Cube ID from calibration
    pub cube_id: String,
    /// Option expiry in years
    pub expiry: f64,
    /// Swap tenor in years
    pub tenor: f64,
    /// Number of strike points (default 100)
    #[serde(default = "default_density_points")]
    pub num_points: usize,
}

fn default_density_points() -> usize { 100 }

/// Density statistics.
///
/// # Requirements Coverage
///
/// - Requirement 6.3: 期待値、分散、歪度、尖度を表示
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DensityStatistics {
    /// Expected value (mean)
    pub mean: f64,
    /// Variance
    pub variance: f64,
    /// Skewness (third standardised moment)
    pub skewness: f64,
    /// Excess kurtosis (normal = 0)
    pub kurtosis: f64,
}

impl Default for DensityStatistics {
    fn default() -> Self {
        Self {
            mean: 0.0,
            variance: 0.0,
            skewness: 0.0,
            kurtosis: 0.0,
        }
    }
}

/// Response for `GET /api/volcube/density`.
///
/// # Requirements Coverage
///
/// - Requirement 6.2: Breeden-Litzenberger法で計算された確率密度関数
/// - Requirement 6.4: 累積分布関数（CDF）表示オプション
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DensityDataResponse {
    /// Option expiry
    pub expiry: f64,
    /// Swap tenor
    pub tenor: f64,
    /// Forward rate
    pub forward: f64,
    /// Strike values
    pub strikes: Vec<f64>,
    /// Probability density values
    pub densities: Vec<f64>,
    /// Cumulative distribution function values
    pub cdf: Vec<f64>,
    /// Distribution statistics
    pub statistics: DensityStatistics,
    /// Warnings about numerical issues
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

// =============================================================================
// 3D Surface Query Types (Req 7, 8.7)
// =============================================================================

/// Query parameters for `GET /api/volcube/surface`.
///
/// # Requirements Coverage
///
/// - Requirement 7.2: Expiry × Strike × Implied Volの3Dサーフェス
/// - Requirement 8.7: 3Dサーフェスデータを返す
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SurfaceQuery {
    /// Cube ID from calibration
    pub cube_id: String,
    /// Fixed tenor for 3D slice (default: first available)
    pub tenor: Option<f64>,
    /// Number of expiry points
    #[serde(default = "default_surface_points")]
    pub expiry_points: usize,
    /// Number of strike points
    #[serde(default = "default_surface_points")]
    pub strike_points: usize,
}

fn default_surface_points() -> usize { 25 }

/// Response for `GET /api/volcube/surface`.
///
/// # Requirements Coverage
///
/// - Requirement 7.2: 3Dサーフェスデータ
/// - Requirement 7.6: 市場観測点をマーカーとして含む
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceDataResponse {
    /// Fixed tenor for this surface slice
    pub tenor: f64,
    /// Expiry axis values
    pub expiries: Vec<f64>,
    /// Strike axis values
    pub strikes: Vec<f64>,
    /// 2D grid of volatilities [expiry][strike]
    pub volatilities: Vec<Vec<f64>>,
    /// Market observation points for markers
    pub market_points: Vec<SurfaceMarketPoint>,
    /// Available tenors for switching
    pub available_tenors: Vec<f64>,
}

/// Market point for 3D surface visualisation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceMarketPoint {
    /// Expiry
    pub expiry: f64,
    /// Strike
    pub strike: f64,
    /// Implied volatility
    pub implied_vol: f64,
}

// =============================================================================
// Builder Method List Response
// =============================================================================

/// Response for `GET /api/volcube/models`.
///
/// Lists available calibration models and configuration options.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeModelsResponse {
    /// Available calibration models
    pub models: Vec<CalibrationModelInfo>,
    /// Available strike axis types
    pub strike_axis_types: Vec<StrikeAxisTypeInfo>,
}

/// Information about a calibration model.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationModelInfo {
    /// Model identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Whether currently enabled
    pub enabled: bool,
}

/// Information about a strike axis type.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrikeAxisTypeInfo {
    /// Type identifier
    pub id: String,
    /// Display name
    pub name: String,
}

impl VolCubeModelsResponse {
    /// Create a new models response with all available options.
    pub fn new() -> Self {
        Self {
            models: vec![
                CalibrationModelInfo {
                    id: "sabr".to_string(),
                    name: CalibrationModel::Sabr.display_name().to_string(),
                    description: CalibrationModel::Sabr.description().to_string(),
                    enabled: CalibrationModel::Sabr.is_enabled(),
                },
                CalibrationModelInfo {
                    id: "svi".to_string(),
                    name: CalibrationModel::Svi.display_name().to_string(),
                    description: CalibrationModel::Svi.description().to_string(),
                    enabled: CalibrationModel::Svi.is_enabled(),
                },
                CalibrationModelInfo {
                    id: "local_volatility".to_string(),
                    name: CalibrationModel::LocalVolatility.display_name().to_string(),
                    description: CalibrationModel::LocalVolatility.description().to_string(),
                    enabled: CalibrationModel::LocalVolatility.is_enabled(),
                },
            ],
            strike_axis_types: vec![
                StrikeAxisTypeInfo {
                    id: "absolute".to_string(),
                    name: StrikeAxisType::Absolute.display_name().to_string(),
                },
                StrikeAxisTypeInfo {
                    id: "moneyness".to_string(),
                    name: StrikeAxisType::Moneyness.display_name().to_string(),
                },
                StrikeAxisTypeInfo {
                    id: "log_moneyness".to_string(),
                    name: StrikeAxisType::LogMoneyness.display_name().to_string(),
                },
                StrikeAxisTypeInfo {
                    id: "delta".to_string(),
                    name: StrikeAxisType::Delta.display_name().to_string(),
                },
            ],
        }
    }
}

impl Default for VolCubeModelsResponse {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // CalibrationModel Tests
    // =========================================================================

    mod calibration_model_tests {
        use super::*;

        #[test]
        fn test_calibration_model_default() {
            let model = CalibrationModel::default();
            assert_eq!(model, CalibrationModel::Sabr);
        }

        #[test]
        fn test_calibration_model_serde() {
            let model = CalibrationModel::Svi;
            let json = serde_json::to_string(&model).unwrap();
            assert_eq!(json, "\"svi\"");

            let parsed: CalibrationModel = serde_json::from_str("\"sabr\"").unwrap();
            assert_eq!(parsed, CalibrationModel::Sabr);

            let parsed: CalibrationModel = serde_json::from_str("\"local_volatility\"").unwrap();
            assert_eq!(parsed, CalibrationModel::LocalVolatility);
        }

        #[test]
        fn test_calibration_model_display_name() {
            assert_eq!(CalibrationModel::Sabr.display_name(), "SABR");
            assert_eq!(CalibrationModel::Svi.display_name(), "SVI");
            assert_eq!(
                CalibrationModel::LocalVolatility.display_name(),
                "Local Volatility"
            );
        }

        #[test]
        fn test_calibration_model_is_enabled() {
            assert!(CalibrationModel::Sabr.is_enabled());
            assert!(!CalibrationModel::Svi.is_enabled());
            assert!(!CalibrationModel::LocalVolatility.is_enabled());
        }
    }

    // =========================================================================
    // StrikeAxisType Tests
    // =========================================================================

    mod strike_axis_type_tests {
        use super::*;

        #[test]
        fn test_strike_axis_type_default() {
            let axis = StrikeAxisType::default();
            assert_eq!(axis, StrikeAxisType::Absolute);
        }

        #[test]
        fn test_strike_axis_type_serde() {
            let axis = StrikeAxisType::Moneyness;
            let json = serde_json::to_string(&axis).unwrap();
            assert_eq!(json, "\"moneyness\"");

            let parsed: StrikeAxisType = serde_json::from_str("\"log_moneyness\"").unwrap();
            assert_eq!(parsed, StrikeAxisType::LogMoneyness);

            let parsed: StrikeAxisType = serde_json::from_str("\"delta\"").unwrap();
            assert_eq!(parsed, StrikeAxisType::Delta);
        }

        #[test]
        fn test_strike_axis_type_display_name() {
            assert_eq!(StrikeAxisType::Absolute.display_name(), "Absolute Strike");
            assert_eq!(StrikeAxisType::Moneyness.display_name(), "Moneyness (K/F)");
            assert_eq!(StrikeAxisType::LogMoneyness.display_name(), "Log-Moneyness");
            assert_eq!(StrikeAxisType::Delta.display_name(), "Delta");
        }
    }

    // =========================================================================
    // SwaptionInstrument Tests
    // =========================================================================

    mod swaption_instrument_tests {
        use super::*;

        #[test]
        fn test_swaption_instrument_new() {
            let inst = SwaptionInstrument::new(1.0, 5.0, 0.03, 0.20, 0.035);
            assert_eq!(inst.expiry, 1.0);
            assert_eq!(inst.tenor, 5.0);
            assert_eq!(inst.strike, 0.03);
            assert_eq!(inst.implied_vol, 0.20);
            assert_eq!(inst.forward, 0.035);
            assert_eq!(inst.weight, 1.0);
        }

        #[test]
        fn test_swaption_instrument_with_weight() {
            let inst = SwaptionInstrument::new(1.0, 5.0, 0.03, 0.20, 0.035).with_weight(0.5);
            assert_eq!(inst.weight, 0.5);
        }

        #[test]
        fn test_swaption_instrument_serde() {
            let inst = SwaptionInstrument::new(1.0, 5.0, 0.03, 0.20, 0.035);
            let json = serde_json::to_string(&inst).unwrap();

            assert!(json.contains("\"expiry\":1.0"));
            assert!(json.contains("\"tenor\":5.0"));
            assert!(json.contains("\"strike\":0.03"));
            assert!(json.contains("\"impliedVol\":0.2"));
            assert!(json.contains("\"forward\":0.035"));

            let parsed: SwaptionInstrument = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.expiry, 1.0);
            assert_eq!(parsed.tenor, 5.0);
        }

        #[test]
        fn test_swaption_instrument_deserialise_with_default_weight() {
            let json = r#"{
                "expiry": 1.0,
                "tenor": 5.0,
                "strike": 0.03,
                "impliedVol": 0.20,
                "forward": 0.035
            }"#;

            let inst: SwaptionInstrument = serde_json::from_str(json).unwrap();
            assert_eq!(inst.weight, 1.0);
        }
    }

    // =========================================================================
    // VolCubeFile Tests
    // =========================================================================

    mod volcube_file_tests {
        use super::*;

        #[test]
        fn test_volcube_file_deserialise() {
            let json = r#"{
                "index": "usd-sofr-swaption",
                "referenceDate": "2026-01-23",
                "dependentCurves": ["usd-sofr"],
                "instruments": [
                    {
                        "expiry": 1.0,
                        "tenor": 5.0,
                        "strike": 0.03,
                        "impliedVol": 0.20,
                        "forward": 0.035
                    }
                ]
            }"#;

            let file: VolCubeFile = serde_json::from_str(json).unwrap();
            assert_eq!(file.index, "usd-sofr-swaption");
            assert_eq!(file.reference_date, "2026-01-23");
            assert_eq!(file.dependent_curves, vec!["usd-sofr"]);
            assert_eq!(file.instruments.len(), 1);
        }
    }

    // =========================================================================
    // Configuration Tests
    // =========================================================================

    mod config_tests {
        use super::*;

        #[test]
        fn test_sabr_config_default() {
            let config = SabrConfigInput::default();
            assert_eq!(config.beta, Some(0.5));
            assert_eq!(config.shift, 0.0);
            assert!(!config.calibrate_beta);
        }

        #[test]
        fn test_volcube_config_default() {
            let config = VolCubeConfigInput::default();
            assert_eq!(config.expiry_interpolation, "linear");
            assert_eq!(config.tenor_interpolation, "linear");
            assert_eq!(config.extrapolation, "flat");
            assert_eq!(config.tolerance, 1e-8);
            assert_eq!(config.max_iterations, 100);
        }
    }

    // =========================================================================
    // Request/Response Tests
    // =========================================================================

    mod request_response_tests {
        use super::*;

        #[test]
        fn test_calibrate_request_deserialise() {
            let json = r#"{
                "index": "usd-sofr-swaption",
                "instruments": [
                    {
                        "expiry": 1.0,
                        "tenor": 5.0,
                        "strike": 0.03,
                        "impliedVol": 0.20,
                        "forward": 0.035
                    }
                ],
                "model": "sabr"
            }"#;

            let req: VolCubeCalibrateRequest = serde_json::from_str(json).unwrap();
            assert_eq!(req.index, "usd-sofr-swaption");
            assert_eq!(req.model, CalibrationModel::Sabr);
            assert_eq!(req.instruments.len(), 1);
        }

        #[test]
        fn test_calibrate_request_defaults() {
            let json = r#"{
                "index": "test",
                "instruments": []
            }"#;

            let req: VolCubeCalibrateRequest = serde_json::from_str(json).unwrap();
            assert_eq!(req.model, CalibrationModel::Sabr);
            assert_eq!(req.config.tolerance, 1e-8);
            assert!(req.dependent_curve_id.is_none());
        }

        #[test]
        fn test_calibrate_response_serialise() {
            let response = VolCubeCalibrateResponse {
                cube_id: "abc-123".to_string(),
                model: CalibrationModel::Sabr,
                parameters: vec![SabrParamsOutput {
                    expiry: 1.0,
                    tenor: 5.0,
                    alpha: 0.04,
                    beta: 0.5,
                    rho: -0.2,
                    nu: 0.3,
                    forward: 0.035,
                }],
                fit_metrics: FitMetrics {
                    rmse: 0.001,
                    max_error: 0.002,
                    r_squared: 0.98,
                    iterations: 10,
                    instrument_count: 5,
                },
                instrument_fits: vec![],
                processing_time_ms: 15.5,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"cubeId\":\"abc-123\""));
            assert!(json.contains("\"model\":\"sabr\""));
            assert!(json.contains("\"alpha\":0.04"));
            assert!(json.contains("\"rmse\":0.001"));
            assert!(json.contains("\"processingTimeMs\":15.5"));
        }
    }

    // =========================================================================
    // Smile and Density Tests
    // =========================================================================

    mod smile_density_tests {
        use super::*;

        #[test]
        fn test_smile_query_defaults() {
            let json = r#"{
                "cube_id": "abc",
                "expiry": 1.0,
                "tenor": 5.0
            }"#;

            let query: SmileQuery = serde_json::from_str(json).unwrap();
            assert_eq!(query.strike_axis, StrikeAxisType::Absolute);
            assert_eq!(query.num_points, 50);
        }

        #[test]
        fn test_smile_response_serialise() {
            let response = SmileDataResponse {
                expiry: 1.0,
                tenor: 5.0,
                forward: 0.035,
                strikes: vec![0.02, 0.03, 0.04],
                model_vols: vec![0.22, 0.20, 0.21],
                market_points: vec![MarketPoint {
                    strike: 0.03,
                    implied_vol: 0.20,
                    weight: 1.0,
                }],
                sabr_params: SabrParamsOutput {
                    expiry: 1.0,
                    tenor: 5.0,
                    alpha: 0.04,
                    beta: 0.5,
                    rho: -0.2,
                    nu: 0.3,
                    forward: 0.035,
                },
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"expiry\":1.0"));
            assert!(json.contains("\"forward\":0.035"));
            assert!(json.contains("\"modelVols\""));
            assert!(json.contains("\"marketPoints\""));
        }

        #[test]
        fn test_density_query_defaults() {
            let json = r#"{
                "cube_id": "abc",
                "expiry": 1.0,
                "tenor": 5.0
            }"#;

            let query: DensityQuery = serde_json::from_str(json).unwrap();
            assert_eq!(query.num_points, 100);
        }

        #[test]
        fn test_density_response_serialise() {
            let response = DensityDataResponse {
                expiry: 1.0,
                tenor: 5.0,
                forward: 0.035,
                strikes: vec![0.02, 0.03, 0.04],
                densities: vec![0.1, 0.5, 0.1],
                cdf: vec![0.1, 0.6, 0.7],
                statistics: DensityStatistics {
                    mean: 0.035,
                    variance: 0.001,
                    skewness: -0.1,
                    kurtosis: 0.5,
                },
                warnings: vec![],
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"densities\""));
            assert!(json.contains("\"cdf\""));
            assert!(json.contains("\"statistics\""));
            // warnings should be skipped if empty
            assert!(!json.contains("\"warnings\""));
        }

        #[test]
        fn test_density_response_with_warnings() {
            let response = DensityDataResponse {
                expiry: 1.0,
                tenor: 5.0,
                forward: 0.035,
                strikes: vec![],
                densities: vec![],
                cdf: vec![],
                statistics: DensityStatistics::default(),
                warnings: vec!["Numerical instability at low strikes".to_string()],
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"warnings\""));
            assert!(json.contains("Numerical instability"));
        }
    }

    // =========================================================================
    // Surface Tests
    // =========================================================================

    mod surface_tests {
        use super::*;

        #[test]
        fn test_surface_query_defaults() {
            let json = r#"{
                "cube_id": "abc"
            }"#;

            let query: SurfaceQuery = serde_json::from_str(json).unwrap();
            assert!(query.tenor.is_none());
            assert_eq!(query.expiry_points, 25);
            assert_eq!(query.strike_points, 25);
        }

        #[test]
        fn test_surface_response_serialise() {
            let response = SurfaceDataResponse {
                tenor: 5.0,
                expiries: vec![1.0, 2.0, 5.0],
                strikes: vec![0.02, 0.03, 0.04],
                volatilities: vec![
                    vec![0.22, 0.20, 0.21],
                    vec![0.21, 0.19, 0.20],
                    vec![0.20, 0.18, 0.19],
                ],
                market_points: vec![SurfaceMarketPoint {
                    expiry: 1.0,
                    strike: 0.03,
                    implied_vol: 0.20,
                }],
                available_tenors: vec![5.0, 10.0],
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"tenor\":5.0"));
            assert!(json.contains("\"volatilities\""));
            assert!(json.contains("\"availableTenors\""));
        }
    }

    // =========================================================================
    // Models Response Tests
    // =========================================================================

    mod models_response_tests {
        use super::*;

        #[test]
        fn test_models_response_new() {
            let response = VolCubeModelsResponse::new();

            assert_eq!(response.models.len(), 3);
            assert_eq!(response.strike_axis_types.len(), 4);

            let sabr = response.models.iter().find(|m| m.id == "sabr").unwrap();
            assert!(sabr.enabled);

            let svi = response.models.iter().find(|m| m.id == "svi").unwrap();
            assert!(!svi.enabled);
        }

        #[test]
        fn test_models_response_serialise() {
            let response = VolCubeModelsResponse::new();
            let json = serde_json::to_string(&response).unwrap();

            assert!(json.contains("\"models\""));
            assert!(json.contains("\"strikeAxisTypes\""));
            assert!(json.contains("\"enabled\""));
        }
    }
}
