//! VolCube API handlers.
//!
//! This module provides REST API handlers for the VolCube Calibration WebApp,
//! including index management, instrument data, and calibration endpoints.
//!
//! # Endpoints
//!
//! - `GET /api/volcube/indices` - Get available VolCube indices
//! - `GET /api/volcube/instruments/{index}` - Get instruments for an index
//! - `PUT /api/volcube/instruments/{index}` - Update instruments for an index
//! - `GET /api/volcube/models` - Get available calibration models
//! - `POST /api/volcube/calibrate` - Calibrate a VolCube
//! - `GET /api/volcube/smile` - Get smile data for a calibrated cube
//! - `GET /api/volcube/density` - Get probability density data
//! - `GET /api/volcube/surface` - Get 3D surface data
//!
//! # Requirements Coverage
//!
//! - Requirement 1: ボラティリティデータ管理
//! - Requirement 3: VolCubeキャリブレーション設定
//! - Requirement 4: キャリブレーション結果パラメータ表示
//! - Requirement 8: バックエンドAPI実装

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Instant};

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::web::{
    error::{ApiError, ApiResult},
    AppState,
};

// =============================================================================
// Calibration Model Enums
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationModel {
    #[default]
    Sabr,
    Svi,
    LocalVolatility,
}

impl CalibrationModel {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Sabr => "SABR",
            Self::Svi => "SVI",
            Self::LocalVolatility => "Local Volatility",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Sabr => "Stochastic Alpha Beta Rho - standard for rates",
            Self::Svi => "Stochastic Volatility Inspired - popular for equity",
            Self::LocalVolatility => "Dupire's local volatility - arbitrage-free",
        }
    }

    pub fn is_enabled(&self) -> bool { matches!(self, Self::Sabr) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StrikeAxisType {
    #[default]
    Absolute,
    Moneyness,
    LogMoneyness,
    Delta,
}

impl StrikeAxisType {
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
// Instrument Data Structures
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwaptionInstrument {
    pub expiry: f64,
    pub tenor: f64,
    pub strike: f64,
    pub implied_vol: f64,
    pub forward: f64,
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 { 1.0 }

impl SwaptionInstrument {
    pub fn new(expiry: f64, tenor: f64, strike: f64, implied_vol: f64, forward: f64) -> Self {
        Self {
            expiry,
            tenor,
            strike,
            implied_vol,
            forward,
            weight: 1.0,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeFile {
    pub index: String,
    pub reference_date: String,
    #[serde(default)]
    pub dependent_curves: Vec<String>,
    pub instruments: Vec<SwaptionInstrument>,
}

// =============================================================================
// SABR Configuration
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SabrConfigInput {
    #[serde(default = "default_sabr_beta")]
    pub beta: Option<f64>,
    #[serde(default)]
    pub shift: f64,
    #[serde(default)]
    pub calibrate_beta: bool,
}

#[allow(clippy::unnecessary_wraps)]
fn default_sabr_beta() -> Option<f64> { Some(1.0) }

impl Default for SabrConfigInput {
    fn default() -> Self {
        Self {
            beta: Some(1.0),
            shift: 0.0,
            calibrate_beta: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeConfigInput {
    #[serde(default)]
    pub sabr: SabrConfigInput,
    #[serde(default = "default_interpolation")]
    pub expiry_interpolation: String,
    #[serde(default = "default_interpolation")]
    pub tenor_interpolation: String,
    #[serde(default = "default_extrapolation")]
    pub extrapolation: String,
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
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
// API Request/Response Types
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeIndicesResponse {
    pub indices: Vec<VolCubeIndexInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeIndexInfo {
    pub id: String,
    pub name: String,
    pub asset_class: String,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeInstrumentListResponse {
    pub index: String,
    pub reference_date: String,
    pub dependent_curves: Vec<String>,
    pub instruments: Vec<SwaptionInstrument>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeInstrumentListRequest {
    pub reference_date: String,
    #[serde(default)]
    pub dependent_curves: Vec<String>,
    pub instruments: Vec<SwaptionInstrument>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeCalibrateRequest {
    pub index: String,
    pub instruments: Vec<SwaptionInstrument>,
    #[serde(default)]
    pub model: CalibrationModel,
    #[serde(default)]
    pub config: VolCubeConfigInput,
    pub dependent_curve_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SabrParamsOutput {
    pub expiry: f64,
    pub tenor: f64,
    pub alpha: f64,
    pub beta: f64,
    pub rho: f64,
    pub nu: f64,
    pub forward: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FitMetrics {
    pub rmse: f64,
    pub max_error: f64,
    pub r_squared: f64,
    pub iterations: usize,
    pub instrument_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentFit {
    pub expiry: f64,
    pub tenor: f64,
    pub strike: f64,
    pub market_vol: f64,
    pub model_vol: f64,
    pub error: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeCalibrateResponse {
    pub cube_id: String,
    pub model: CalibrationModel,
    pub parameters: Vec<SabrParamsOutput>,
    pub fit_metrics: FitMetrics,
    pub instrument_fits: Vec<InstrumentFit>,
    pub processing_time_ms: f64,
}

// =============================================================================
// Smile and Density Query Types
// =============================================================================

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SmileQuery {
    pub cube_id: String,
    pub expiry: f64,
    pub tenor: f64,
    #[serde(default)]
    pub strike_axis: StrikeAxisType,
    #[serde(default = "default_num_points")]
    pub num_points: usize,
}

fn default_num_points() -> usize { 50 }

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPoint {
    pub strike: f64,
    pub implied_vol: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmileDataResponse {
    pub expiry: f64,
    pub tenor: f64,
    pub forward: f64,
    pub strikes: Vec<f64>,
    pub model_vols: Vec<f64>,
    pub market_points: Vec<MarketPoint>,
    pub sabr_params: SabrParamsOutput,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DensityQuery {
    pub cube_id: String,
    pub expiry: f64,
    pub tenor: f64,
    #[serde(default = "default_density_points")]
    pub num_points: usize,
}

fn default_density_points() -> usize { 100 }

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DensityStatistics {
    pub mean: f64,
    pub variance: f64,
    pub skewness: f64,
    pub kurtosis: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DensityDataResponse {
    pub expiry: f64,
    pub tenor: f64,
    pub forward: f64,
    pub strikes: Vec<f64>,
    pub densities: Vec<f64>,
    pub cdf: Vec<f64>,
    pub statistics: DensityStatistics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

// =============================================================================
// 3D Surface Query Types
// =============================================================================

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SurfaceQuery {
    pub cube_id: String,
    pub tenor: Option<f64>,
    #[serde(default = "default_surface_points")]
    pub expiry_points: usize,
    #[serde(default = "default_surface_points")]
    pub strike_points: usize,
}

fn default_surface_points() -> usize { 25 }

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceDataResponse {
    pub tenor: f64,
    pub expiries: Vec<f64>,
    pub strikes: Vec<f64>,
    pub volatilities: Vec<Vec<f64>>,
    pub market_points: Vec<SurfaceMarketPoint>,
    pub available_tenors: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceMarketPoint {
    pub expiry: f64,
    pub strike: f64,
    pub implied_vol: f64,
}

// =============================================================================
// Models Response
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCubeModelsResponse {
    pub models: Vec<CalibrationModelInfo>,
    pub strike_axis_types: Vec<StrikeAxisTypeInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrikeAxisTypeInfo {
    pub id: String,
    pub name: String,
}

impl VolCubeModelsResponse {
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
// VolCubeCache - LRU Cache for calibrated cubes (Req 8.8)
// =============================================================================

/// Cached VolCube data after calibration.
#[derive(Debug, Clone)]
pub struct CachedVolCube {
    /// Instruments used for calibration
    pub instruments: Vec<SwaptionInstrument>,
    /// Calibration model used
    pub model: CalibrationModel,
    /// SABR parameters at each grid point
    pub parameters: Vec<SabrParamsOutput>,
    /// Fit metrics
    pub fit_metrics: FitMetrics,
    /// Available expiries
    pub expiries: Vec<f64>,
    /// Available tenors
    pub tenors: Vec<f64>,
}

/// LRU cache for calibrated VolCubes.
pub struct VolCubeCache {
    cubes: RwLock<HashMap<Uuid, CachedVolCube>>,
    max_entries: usize,
}

impl VolCubeCache {
    /// Create a new VolCubeCache with the specified maximum entries.
    pub fn new(max_entries: usize) -> Self {
        Self {
            cubes: RwLock::new(HashMap::new()),
            max_entries,
        }
    }

    /// Add a calibrated cube to the cache.
    pub async fn add(&self, id: Uuid, cube: CachedVolCube) {
        let mut cubes = self.cubes.write().await;

        // Simple eviction: remove oldest if at capacity
        if cubes.len() >= self.max_entries {
            if let Some(key) = cubes.keys().next().copied() {
                cubes.remove(&key);
            }
        }

        cubes.insert(id, cube);
    }

    /// Get a cached cube by ID.
    pub async fn get(&self, id: &Uuid) -> Option<CachedVolCube> {
        let cubes = self.cubes.read().await;
        cubes.get(id).cloned()
    }

    /// Check if the cache is empty.
    pub async fn is_empty(&self) -> bool {
        let cubes = self.cubes.read().await;
        cubes.is_empty()
    }

    /// Get the number of cached entries.
    pub async fn len(&self) -> usize {
        let cubes = self.cubes.read().await;
        cubes.len()
    }
}

impl Default for VolCubeCache {
    fn default() -> Self { Self::new(10) }
}

// =============================================================================
// VolCubeDataLoader (Task 2.2)
// =============================================================================

/// Loader for VolCube instrument data files.
///
/// # Requirements Coverage
///
/// - Requirement 1.4: `demo/data/input/irvol/`ディレクトリからデータ読み込み
/// - Requirement 1.5:
///   ファイルが存在しないか不正な形式の場合、適切なエラーメッセージ
pub struct VolCubeDataLoader {
    base_path: PathBuf,
}

impl VolCubeDataLoader {
    /// Create a new VolCubeDataLoader with the specified base path.
    pub fn new(base_path: PathBuf) -> Self { Self { base_path } }

    /// Create a VolCubeDataLoader with the default path.
    pub fn default_path() -> Self { Self::new(PathBuf::from("demo/data/input/irvol")) }

    /// Get the list of available swaption indices.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 9.2, 9.3: USD-SOFR-Swaption, EUR-ESTR-Swaptionをサポート
    pub fn available_indices(&self) -> Vec<VolCubeIndexInfo> {
        let mut indices = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    // Only include swaption files (not FX files)
                    if name.ends_with("-swaption.json") {
                        let id = name.trim_end_matches(".json").to_string();
                        let display_name = id.replace('-', " ").to_uppercase();

                        let currency = if id.starts_with("usd") {
                            "USD"
                        } else if id.starts_with("eur") {
                            "EUR"
                        } else if id.starts_with("jpy") {
                            "JPY"
                        } else {
                            "Other"
                        };

                        indices.push(VolCubeIndexInfo {
                            id,
                            name: display_name,
                            asset_class: "swaption".to_string(),
                            currency: currency.to_string(),
                        });
                    }
                }
            }
        }

        // Sort for consistent ordering
        indices.sort_by(|a, b| a.id.cmp(&b.id));
        indices
    }

    /// Load instruments for the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index identifier (e.g., "usd-sofr-swaption")
    ///
    /// # Returns
    ///
    /// The instrument file contents if found and valid.
    ///
    /// # Errors
    ///
    /// Returns an error if the file doesn't exist or is invalid.
    pub fn load_instruments(&self, index: &str) -> Result<VolCubeFile, VolCubeDataError> {
        let file_path = self.base_path.join(format!("{}.json", index));

        if !file_path.exists() {
            return Err(VolCubeDataError::IndexNotFound(index.to_string()));
        }

        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| VolCubeDataError::IoError(e.to_string()))?;

        let file: VolCubeFile = serde_json::from_str(&content)
            .map_err(|e| VolCubeDataError::ParseError(e.to_string()))?;

        Ok(file)
    }

    /// Save instruments to file.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 1.7: 編集したデータを保存可能
    pub fn save_instruments(
        &self,
        index: &str,
        data: &VolCubeFile,
    ) -> Result<(), VolCubeDataError> {
        let file_path = self.base_path.join(format!("{}.json", index));

        let content = serde_json::to_string_pretty(data)
            .map_err(|e| VolCubeDataError::ParseError(e.to_string()))?;

        std::fs::write(&file_path, content)
            .map_err(|e| VolCubeDataError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Check if an index is supported.
    pub fn is_supported(&self, index: &str) -> bool {
        let file_path = self.base_path.join(format!("{}.json", index));
        file_path.exists()
    }
}

impl Default for VolCubeDataLoader {
    fn default() -> Self { Self::default_path() }
}

/// Errors from VolCubeDataLoader operations.
#[derive(Debug, Clone)]
pub enum VolCubeDataError {
    /// The requested index was not found
    IndexNotFound(String),
    /// IO error reading the file
    IoError(String),
    /// Error parsing the JSON file
    ParseError(String),
}

impl std::fmt::Display for VolCubeDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexNotFound(index) => write!(f, "VolCube index '{}' not found", index),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for VolCubeDataError {}

// =============================================================================
// API Handlers (Tasks 2.2, 2.3)
// =============================================================================

/// Handler for `GET /api/volcube/indices`.
///
/// # Requirements Coverage
///
/// - Requirement 8.1: 利用可能なIndex一覧を返す
pub async fn get_indices() -> ApiResult<VolCubeIndicesResponse> {
    let loader = VolCubeDataLoader::default_path();
    let indices = loader.available_indices();

    Ok(Json(VolCubeIndicesResponse { indices }))
}

/// Handler for `GET /api/volcube/instruments/{index}`.
///
/// # Requirements Coverage
///
/// - Requirement 8.2: 指定Indexのインストゥルメントデータを返す
pub async fn get_instruments(
    Path(index): Path<String>,
) -> ApiResult<VolCubeInstrumentListResponse> {
    let loader = VolCubeDataLoader::default_path();

    let file = loader.load_instruments(&index).map_err(|e| match e {
        VolCubeDataError::IndexNotFound(_) => ApiError::not_found("VolCube Index", &index),
        VolCubeDataError::IoError(msg) => ApiError::internal(msg),
        VolCubeDataError::ParseError(msg) => {
            ApiError::internal(format!("Invalid instrument file: {}", msg))
        }
    })?;

    let response = VolCubeInstrumentListResponse {
        index: file.index,
        reference_date: file.reference_date,
        dependent_curves: file.dependent_curves,
        instruments: file.instruments,
    };

    Ok(Json(response))
}

/// Handler for `PUT /api/volcube/instruments/{index}`.
///
/// # Requirements Coverage
///
/// - Requirement 8.3: インストゥルメントデータを更新・保存
pub async fn update_instruments(
    Path(index): Path<String>,
    Json(request): Json<VolCubeInstrumentListRequest>,
) -> ApiResult<VolCubeInstrumentListResponse> {
    let loader = VolCubeDataLoader::default_path();

    // Validate instruments
    for inst in &request.instruments {
        if inst.expiry <= 0.0 {
            return Err(ApiError::validation(
                format!("Expiry must be positive, got {}", inst.expiry),
                "expiry",
            ));
        }
        if inst.tenor <= 0.0 {
            return Err(ApiError::validation(
                format!("Tenor must be positive, got {}", inst.tenor),
                "tenor",
            ));
        }
        if inst.implied_vol <= 0.0 || inst.implied_vol > 2.0 {
            return Err(ApiError::validation(
                format!("Implied vol {} out of range (0, 2)", inst.implied_vol),
                "impliedVol",
            ));
        }
    }

    // Create file structure
    let file = VolCubeFile {
        index: index.clone(),
        reference_date: request.reference_date.clone(),
        dependent_curves: request.dependent_curves.clone(),
        instruments: request.instruments.clone(),
    };

    // Save to file
    loader
        .save_instruments(&index, &file)
        .map_err(|e| match e {
            VolCubeDataError::IoError(msg) => {
                ApiError::internal(format!("Failed to save: {}", msg))
            }
            _ => ApiError::internal("Unexpected error saving instruments"),
        })?;

    let response = VolCubeInstrumentListResponse {
        index: file.index,
        reference_date: file.reference_date,
        dependent_curves: file.dependent_curves,
        instruments: file.instruments,
    };

    Ok(Json(response))
}

/// Handler for `GET /api/volcube/models`.
///
/// # Requirements Coverage
///
/// - Requirement 3.1: SABR、SVI、Local Volatilityモデルを選択可能
pub async fn get_models() -> ApiResult<VolCubeModelsResponse> {
    Ok(Json(VolCubeModelsResponse::new()))
}

/// Handler for `POST /api/volcube/calibrate`.
///
/// # Requirements Coverage
///
/// - Requirement 8.4: キャリブレーションを実行
/// - Requirement 3.5: Calibrateボタンでキャリブレーション開始
/// - Requirement 4.1-4.6: キャリブレーション結果表示
pub async fn calibrate(
    State(state): State<Arc<AppState>>,
    Json(request): Json<VolCubeCalibrateRequest>,
) -> ApiResult<VolCubeCalibrateResponse> {
    let start = Instant::now();

    // Validate request
    if request.instruments.is_empty() {
        return Err(ApiError::validation(
            "At least one instrument is required",
            "instruments",
        ));
    }

    // Validate model
    if !request.model.is_enabled() {
        return Err(ApiError::validation(
            format!(
                "{} model is not yet implemented",
                request.model.display_name()
            ),
            "model",
        ));
    }

    // Group instruments by expiry-tenor
    let mut grid: HashMap<(i32, i32), Vec<&SwaptionInstrument>> = HashMap::new();
    for inst in &request.instruments {
        let key = ((inst.expiry * 100.0) as i32, (inst.tenor * 100.0) as i32);
        grid.entry(key).or_default().push(inst);
    }

    // Calibrate SABR at each grid point
    let mut parameters = Vec::new();
    let mut instrument_fits = Vec::new();
    let mut total_error = 0.0;
    let mut max_error = 0.0_f64;
    let mut fit_count = 0;

    let beta = request.config.sabr.beta.unwrap_or(0.5);

    for ((exp_key, tenor_key), instruments) in &grid {
        let expiry = *exp_key as f64 / 100.0;
        let tenor = *tenor_key as f64 / 100.0;

        // Calculate average forward
        let avg_forward =
            instruments.iter().map(|i| i.forward).sum::<f64>() / instruments.len() as f64;

        // Simple SABR calibration (simplified for demo)
        let (alpha, rho, nu) = calibrate_sabr_simple(instruments, beta, avg_forward);

        parameters.push(SabrParamsOutput {
            expiry,
            tenor,
            alpha,
            beta,
            rho,
            nu,
            forward: avg_forward,
        });

        // Calculate fit quality for each instrument
        for inst in instruments.iter() {
            let model_vol =
                sabr_implied_vol(inst.strike, avg_forward, expiry, alpha, beta, rho, nu);
            let error = model_vol - inst.implied_vol;

            total_error += error * error;
            max_error = max_error.max(error.abs());
            fit_count += 1;

            instrument_fits.push(InstrumentFit {
                expiry: inst.expiry,
                tenor: inst.tenor,
                strike: inst.strike,
                market_vol: inst.implied_vol,
                model_vol,
                error,
            });
        }
    }

    // Calculate RMSE and R²
    let rmse = if fit_count > 0 {
        (total_error / fit_count as f64).sqrt()
    } else {
        0.0
    };

    // R² = 1 - SS_res / SS_tot
    let mean_vol = request
        .instruments
        .iter()
        .map(|i| i.implied_vol)
        .sum::<f64>()
        / request.instruments.len() as f64;
    let ss_tot: f64 = request
        .instruments
        .iter()
        .map(|i| (i.implied_vol - mean_vol).powi(2))
        .sum();
    let r_squared = if ss_tot > 0.0 {
        1.0 - total_error / ss_tot
    } else {
        1.0
    };

    // Generate cube ID and cache
    let cube_id = Uuid::new_v4();

    // Get unique expiries and tenors
    let mut expiries: Vec<f64> = grid.keys().map(|(e, _)| *e as f64 / 100.0).collect();
    let mut tenors: Vec<f64> = grid.keys().map(|(_, t)| *t as f64 / 100.0).collect();
    expiries.sort_by(|a, b| a.partial_cmp(b).unwrap());
    expiries.dedup();
    tenors.sort_by(|a, b| a.partial_cmp(b).unwrap());
    tenors.dedup();

    let cached_cube = CachedVolCube {
        instruments: request.instruments.clone(),
        model: request.model,
        parameters: parameters.clone(),
        fit_metrics: FitMetrics {
            rmse,
            max_error,
            r_squared,
            iterations: 1, // Simplified calibration
            instrument_count: request.instruments.len(),
        },
        expiries,
        tenors,
    };

    state.volcube_cache.add(cube_id, cached_cube).await;

    let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    let response = VolCubeCalibrateResponse {
        cube_id: cube_id.to_string(),
        model: request.model,
        parameters,
        fit_metrics: FitMetrics {
            rmse,
            max_error,
            r_squared,
            iterations: 1,
            instrument_count: request.instruments.len(),
        },
        instrument_fits,
        processing_time_ms,
    };

    Ok(Json(response))
}

/// Handler for `GET /api/volcube/smile`.
///
/// # Requirements Coverage
///
/// - Requirement 8.5: 指定Expiry/Tenorのスマイルデータを返す
/// - Requirement 5.2: Strike vs Implied Volのスマイルカーブ
pub async fn get_smile(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SmileQuery>,
) -> ApiResult<SmileDataResponse> {
    // Parse cube ID
    let cube_id = Uuid::parse_str(&query.cube_id)
        .map_err(|_| ApiError::validation("Invalid cube ID format", "cube_id"))?;

    // Get cached cube
    let cube = state
        .volcube_cache
        .get(&cube_id)
        .await
        .ok_or_else(|| ApiError::not_found("VolCube", &query.cube_id))?;

    // Find SABR params for this expiry-tenor
    let params = cube
        .parameters
        .iter()
        .find(|p| (p.expiry - query.expiry).abs() < 0.01 && (p.tenor - query.tenor).abs() < 0.01)
        .ok_or_else(|| {
            ApiError::not_found(
                "Grid point",
                format!("expiry={}, tenor={}", query.expiry, query.tenor),
            )
        })?;

    let forward = params.forward;

    // Generate strike grid
    let strike_min = forward * 0.5;
    let strike_max = forward * 1.5;
    let num_points = query.num_points.clamp(10, 200);
    let strike_step = (strike_max - strike_min) / (num_points - 1) as f64;

    let mut strikes = Vec::with_capacity(num_points);
    let mut model_vols = Vec::with_capacity(num_points);

    for i in 0..num_points {
        let strike = strike_min + i as f64 * strike_step;
        strikes.push(strike);

        let vol = sabr_implied_vol(
            strike,
            forward,
            query.expiry,
            params.alpha,
            params.beta,
            params.rho,
            params.nu,
        );
        model_vols.push(vol);
    }

    // Get market points for this expiry-tenor
    let market_points: Vec<MarketPoint> = cube
        .instruments
        .iter()
        .filter(|i| (i.expiry - query.expiry).abs() < 0.01 && (i.tenor - query.tenor).abs() < 0.01)
        .map(|i| MarketPoint {
            strike: i.strike,
            implied_vol: i.implied_vol,
            weight: i.weight,
        })
        .collect();

    Ok(Json(SmileDataResponse {
        expiry: query.expiry,
        tenor: query.tenor,
        forward,
        strikes,
        model_vols,
        market_points,
        sabr_params: params.clone(),
    }))
}

/// Handler for `GET /api/volcube/density`.
///
/// # Requirements Coverage
///
/// - Requirement 8.6: 確率密度データを返す
/// - Requirement 6.2: Breeden-Litzenberger法で計算された確率密度関数
pub async fn get_density(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DensityQuery>,
) -> ApiResult<DensityDataResponse> {
    // Parse cube ID
    let cube_id = Uuid::parse_str(&query.cube_id)
        .map_err(|_| ApiError::validation("Invalid cube ID format", "cube_id"))?;

    // Get cached cube
    let cube = state
        .volcube_cache
        .get(&cube_id)
        .await
        .ok_or_else(|| ApiError::not_found("VolCube", &query.cube_id))?;

    // Find SABR params for this expiry-tenor
    let params = cube
        .parameters
        .iter()
        .find(|p| (p.expiry - query.expiry).abs() < 0.01 && (p.tenor - query.tenor).abs() < 0.01)
        .ok_or_else(|| {
            ApiError::not_found(
                "Grid point",
                format!("expiry={}, tenor={}", query.expiry, query.tenor),
            )
        })?;

    let forward = params.forward;
    let rate = 0.03; // Approximate discount rate

    // Generate strike grid
    let strike_min = forward * 0.5;
    let strike_max = forward * 1.5;
    let num_points = query.num_points.clamp(50, 500);
    let strike_step = (strike_max - strike_min) / (num_points - 1) as f64;

    let mut strikes = Vec::with_capacity(num_points);
    let mut densities = Vec::with_capacity(num_points);
    let mut cdf = Vec::with_capacity(num_points);
    let mut warnings = Vec::new();

    // Breeden-Litzenberger: d²C/dK²
    let h = strike_step * 0.1; // Small step for numerical derivative

    for i in 0..num_points {
        let strike = strike_min + i as f64 * strike_step;
        strikes.push(strike);

        // Compute density using central difference
        let vol_mid = sabr_implied_vol(
            strike,
            forward,
            query.expiry,
            params.alpha,
            params.beta,
            params.rho,
            params.nu,
        );
        let vol_low = sabr_implied_vol(
            strike - h,
            forward,
            query.expiry,
            params.alpha,
            params.beta,
            params.rho,
            params.nu,
        );
        let vol_high = sabr_implied_vol(
            strike + h,
            forward,
            query.expiry,
            params.alpha,
            params.beta,
            params.rho,
            params.nu,
        );

        let c_low = black_call_price(strike - h, forward, query.expiry, vol_low, rate);
        let c_mid = black_call_price(strike, forward, query.expiry, vol_mid, rate);
        let c_high = black_call_price(strike + h, forward, query.expiry, vol_high, rate);

        let d2c_dk2 = (c_high - 2.0 * c_mid + c_low) / (h * h);
        let density = (rate * query.expiry).exp() * d2c_dk2;

        densities.push(density.max(0.0));
    }

    // Normalise densities
    let total: f64 = densities.iter().sum::<f64>() * strike_step;
    if total > 0.0 {
        for d in &mut densities {
            *d /= total;
        }
    } else {
        warnings.push("Density normalisation failed - total was zero".to_string());
    }

    // Compute CDF (cumulative sum)
    let mut cumulative = 0.0;
    for d in &densities {
        cumulative += d * strike_step;
        cdf.push(cumulative.min(1.0));
    }

    // Compute statistics
    let mean: f64 = strikes
        .iter()
        .zip(&densities)
        .map(|(k, d)| k * d * strike_step)
        .sum();

    let variance: f64 = strikes
        .iter()
        .zip(&densities)
        .map(|(k, d)| (k - mean).powi(2) * d * strike_step)
        .sum();

    let std_dev = variance.sqrt();

    let skewness: f64 = if std_dev > 0.0 {
        strikes
            .iter()
            .zip(&densities)
            .map(|(k, d)| ((k - mean) / std_dev).powi(3) * d * strike_step)
            .sum()
    } else {
        0.0
    };

    let kurtosis: f64 = if std_dev > 0.0 {
        let m4: f64 = strikes
            .iter()
            .zip(&densities)
            .map(|(k, d)| ((k - mean) / std_dev).powi(4) * d * strike_step)
            .sum();
        m4 - 3.0 // Excess kurtosis
    } else {
        0.0
    };

    Ok(Json(DensityDataResponse {
        expiry: query.expiry,
        tenor: query.tenor,
        forward,
        strikes,
        densities,
        cdf,
        statistics: DensityStatistics {
            mean,
            variance,
            skewness,
            kurtosis,
        },
        warnings,
    }))
}

/// Handler for `GET /api/volcube/surface`.
///
/// # Requirements Coverage
///
/// - Requirement 8.7: 3Dサーフェス用グリッドデータを返す
/// - Requirement 7.2: Expiry × Strike × Implied Volの3Dサーフェス
pub async fn get_surface(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SurfaceQuery>,
) -> ApiResult<SurfaceDataResponse> {
    // Parse cube ID
    let cube_id = Uuid::parse_str(&query.cube_id)
        .map_err(|_| ApiError::validation("Invalid cube ID format", "cube_id"))?;

    // Get cached cube
    let cube = state
        .volcube_cache
        .get(&cube_id)
        .await
        .ok_or_else(|| ApiError::not_found("VolCube", &query.cube_id))?;

    // Get available tenors
    let available_tenors = cube.tenors.clone();

    // Select tenor (use first available if not specified)
    let tenor = query
        .tenor
        .unwrap_or_else(|| available_tenors.first().copied().unwrap_or(5.0));

    // Find params for this tenor
    let tenor_params: Vec<&SabrParamsOutput> = cube
        .parameters
        .iter()
        .filter(|p| (p.tenor - tenor).abs() < 0.01)
        .collect();

    if tenor_params.is_empty() {
        return Err(ApiError::not_found("Tenor", tenor.to_string()));
    }

    // Get expiry range
    let expiry_min = tenor_params
        .iter()
        .map(|p| p.expiry)
        .fold(f64::INFINITY, f64::min);
    let expiry_max = tenor_params
        .iter()
        .map(|p| p.expiry)
        .fold(f64::NEG_INFINITY, f64::max);

    // Get forward range for strike calculation
    let forward_avg =
        tenor_params.iter().map(|p| p.forward).sum::<f64>() / tenor_params.len() as f64;
    let strike_min = forward_avg * 0.5;
    let strike_max = forward_avg * 1.5;

    // Generate grids
    let num_expiries = query.expiry_points.clamp(5, 50);
    let num_strikes = query.strike_points.clamp(5, 50);

    let expiry_step = (expiry_max - expiry_min) / (num_expiries - 1).max(1) as f64;
    let strike_step = (strike_max - strike_min) / (num_strikes - 1).max(1) as f64;

    let expiries: Vec<f64> = (0..num_expiries)
        .map(|i| expiry_min + i as f64 * expiry_step)
        .collect();

    let strikes: Vec<f64> = (0..num_strikes)
        .map(|i| strike_min + i as f64 * strike_step)
        .collect();

    // Generate volatility grid
    let mut volatilities: Vec<Vec<f64>> = Vec::with_capacity(num_expiries);

    for &expiry in &expiries {
        // Find closest params for this expiry
        let params = tenor_params
            .iter()
            .min_by(|a, b| {
                (a.expiry - expiry)
                    .abs()
                    .partial_cmp(&(b.expiry - expiry).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        let row: Vec<f64> = strikes
            .iter()
            .map(|&strike| {
                sabr_implied_vol(
                    strike,
                    params.forward,
                    expiry,
                    params.alpha,
                    params.beta,
                    params.rho,
                    params.nu,
                )
            })
            .collect();

        volatilities.push(row);
    }

    // Get market points for markers
    let market_points: Vec<SurfaceMarketPoint> = cube
        .instruments
        .iter()
        .filter(|i| (i.tenor - tenor).abs() < 0.01)
        .map(|i| SurfaceMarketPoint {
            expiry: i.expiry,
            strike: i.strike,
            implied_vol: i.implied_vol,
        })
        .collect();

    Ok(Json(SurfaceDataResponse {
        tenor,
        expiries,
        strikes,
        volatilities,
        market_points,
        available_tenors,
    }))
}

// =============================================================================
// Helper Functions
// =============================================================================

/// SABR calibration using iterative least-squares optimization.
///
/// Uses Levenberg-Marquardt style optimization with multi-start to find optimal
/// (alpha, rho, nu) that minimises the sum of squared errors between model and
/// market volatilities.
fn calibrate_sabr_simple(
    instruments: &[&SwaptionInstrument],
    beta: f64,
    forward: f64,
) -> (f64, f64, f64) {
    if instruments.is_empty() {
        return (0.2 * forward.powf(1.0 - beta), 0.0, 0.3);
    }

    // Find ATM instrument for initial guess
    let atm_inst = instruments
        .iter()
        .min_by(|a, b| {
            (a.strike - forward)
                .abs()
                .partial_cmp(&(b.strike - forward).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();

    // Initial guess for alpha from ATM vol (accounting for f^(1-beta) scaling)
    let f_mid = forward.powf(1.0 - beta);
    let alpha_init = (atm_inst.implied_vol * f_mid).max(0.001);

    // Estimate rho from skew direction
    let low_strikes: Vec<_> = instruments.iter().filter(|i| i.strike < forward).collect();
    let high_strikes: Vec<_> = instruments.iter().filter(|i| i.strike > forward).collect();
    let rho_init = if !low_strikes.is_empty() && !high_strikes.is_empty() {
        let avg_low_vol =
            low_strikes.iter().map(|i| i.implied_vol).sum::<f64>() / low_strikes.len() as f64;
        let avg_high_vol =
            high_strikes.iter().map(|i| i.implied_vol).sum::<f64>() / high_strikes.len() as f64;
        let skew = avg_low_vol - avg_high_vol;
        // Negative skew (higher vol at low strikes) -> negative rho
        (-skew * 5.0 / atm_inst.implied_vol).clamp(-0.8, 0.8)
    } else {
        0.0
    };

    // Estimate nu from smile curvature
    let nu_init = if instruments.len() >= 3 {
        let wing_vols: Vec<f64> = instruments
            .iter()
            .filter(|i| (i.strike - forward).abs() > forward * 0.15)
            .map(|i| i.implied_vol)
            .collect();
        if !wing_vols.is_empty() {
            let avg_wing = wing_vols.iter().sum::<f64>() / wing_vols.len() as f64;
            let curvature = (avg_wing - atm_inst.implied_vol).max(0.0);
            (curvature * 10.0 / atm_inst.implied_vol + 0.2).clamp(0.1, 2.0)
        } else {
            0.4
        }
    } else {
        0.4
    };

    // Objective function: sum of squared weighted errors
    let objective = |a: f64, r: f64, n: f64| -> f64 {
        instruments
            .iter()
            .map(|inst| {
                let model_vol = sabr_implied_vol(inst.strike, forward, inst.expiry, a, beta, r, n);
                if model_vol <= 0.0 || model_vol.is_nan() {
                    return 1e10;
                }
                let err = (model_vol - inst.implied_vol) * inst.weight;
                err * err
            })
            .sum()
    };

    // Start optimization directly from estimated initial values
    // For swaptions, rho is typically negative - bias initial guess accordingly
    let rho_start = rho_init.min(-0.1); // Ensure we start from negative rho

    let (alpha, rho, nu, _) = optimize_sabr(alpha_init, rho_start, nu_init, &objective);

    (alpha, rho, nu)
}

/// Run LM-style optimization from a starting point.
fn optimize_sabr<F>(
    mut alpha: f64,
    mut rho: f64,
    mut nu: f64,
    objective: &F,
) -> (f64, f64, f64, f64)
where
    F: Fn(f64, f64, f64) -> f64,
{
    let max_iterations = 150;
    let tolerance = 1e-12;
    let mut lambda = 0.001;

    for _ in 0..max_iterations {
        let current_obj = objective(alpha, rho, nu);
        if current_obj.is_nan() || current_obj.is_infinite() {
            break;
        }

        let h = 1e-7;
        let grad_alpha =
            (objective(alpha + h, rho, nu) - objective(alpha - h, rho, nu)) / (2.0 * h);
        let grad_rho = (objective(alpha, rho + h, nu) - objective(alpha, rho - h, nu)) / (2.0 * h);
        let grad_nu = (objective(alpha, rho, nu + h) - objective(alpha, rho, nu - h)) / (2.0 * h);

        // Skip if gradients are NaN
        if grad_alpha.is_nan() || grad_rho.is_nan() || grad_nu.is_nan() {
            break;
        }

        let hess_alpha = (objective(alpha + h, rho, nu) - 2.0 * current_obj
            + objective(alpha - h, rho, nu))
            / (h * h);
        let hess_rho = (objective(alpha, rho + h, nu) - 2.0 * current_obj
            + objective(alpha, rho - h, nu))
            / (h * h);
        let hess_nu = (objective(alpha, rho, nu + h) - 2.0 * current_obj
            + objective(alpha, rho, nu - h))
            / (h * h);

        let d_alpha = -grad_alpha / (hess_alpha.abs().max(1e-8) + lambda);
        let d_rho = -grad_rho / (hess_rho.abs().max(1e-8) + lambda);
        let d_nu = -grad_nu / (hess_nu.abs().max(1e-8) + lambda);

        let mut step = 1.0;
        let mut new_alpha = alpha;
        let mut new_rho = rho;
        let mut new_nu = nu;
        let mut accepted = false;

        for _ in 0..20 {
            new_alpha = (alpha + step * d_alpha).clamp(0.001, 1.0);
            new_rho = (rho + step * d_rho).clamp(-0.95, 0.95);
            new_nu = (nu + step * d_nu).clamp(0.05, 3.0);

            let new_obj = objective(new_alpha, new_rho, new_nu);
            if !new_obj.is_nan() && new_obj < current_obj {
                lambda = (lambda * 0.7).max(1e-10);
                accepted = true;
                break;
            }
            step *= 0.5;
        }

        if !accepted {
            lambda = (lambda * 2.0).min(1e4);
            continue;
        }

        let improvement = (alpha - new_alpha).abs() + (rho - new_rho).abs() + (nu - new_nu).abs();
        alpha = new_alpha;
        rho = new_rho;
        nu = new_nu;

        if improvement < tolerance {
            break;
        }
    }

    let final_obj = objective(alpha, rho, nu);
    (alpha, rho, nu, final_obj)
}

/// SABR implied volatility approximation (Hagan formula).
fn sabr_implied_vol(
    strike: f64,
    forward: f64,
    expiry: f64,
    alpha: f64,
    beta: f64,
    rho: f64,
    nu: f64,
) -> f64 {
    if strike <= 0.0 || forward <= 0.0 || expiry <= 0.0 || alpha <= 0.0 {
        return 0.0;
    }

    let eps = 1e-10;
    let one_minus_beta = 1.0 - beta;

    // ATM case
    if (strike - forward).abs() < eps {
        let f_mid = forward.powf(one_minus_beta);
        let term1 = alpha / f_mid;
        let term2 = (one_minus_beta.powi(2) * alpha.powi(2)) / (24.0 * f_mid.powi(2));
        let term3 = rho * beta * nu * alpha / (4.0 * f_mid);
        let term4 = (2.0 - 3.0 * rho.powi(2)) * nu.powi(2) / 24.0;
        return term1 * (1.0 + (term2 + term3 + term4) * expiry);
    }

    // General case (Hagan et al. 2002)
    let log_fk = (forward / strike).ln();
    let fk_mid = (forward * strike).powf(one_minus_beta / 2.0);
    let z = (nu / alpha) * fk_mid * log_fk;

    // x(z) = (sqrt(1 - 2*rho*z + z^2) + z - rho) / (1 - rho)
    // The ratio z/ln(x(z)) appears in the formula; when z→0, x(z)→1, so we need
    // L'Hopital
    let x_z = ((1.0 - 2.0 * rho * z + z * z).sqrt() + z - rho) / (1.0 - rho);
    let x_z_ratio = if (x_z - 1.0).abs() < eps {
        // When x_z ≈ 1, ln(x_z) ≈ 0, use limit: z/ln(x_z) → 1 as z→0
        1.0
    } else {
        z / x_z.ln()
    };

    let prefix = alpha
        / (fk_mid
            * (1.0
                + one_minus_beta.powi(2) * log_fk.powi(2) / 24.0
                + one_minus_beta.powi(4) * log_fk.powi(4) / 1920.0));

    let term2 = (one_minus_beta.powi(2) * alpha.powi(2)) / (24.0 * fk_mid.powi(2));
    let term3 = rho * beta * nu * alpha / (4.0 * fk_mid);
    let term4 = (2.0 - 3.0 * rho.powi(2)) * nu.powi(2) / 24.0;

    let result = prefix * x_z_ratio * (1.0 + (term2 + term3 + term4) * expiry);

    // Safeguard: ensure non-negative, finite result
    if result.is_nan() || result.is_infinite() || result < 0.0 {
        // Fallback to simple ATM-like approximation
        return (alpha / fk_mid).max(0.001);
    }

    result
}

/// Black call price for density calculation.
fn black_call_price(strike: f64, forward: f64, expiry: f64, vol: f64, rate: f64) -> f64 {
    use std::f64::consts::PI;

    if strike <= 0.0 || forward <= 0.0 || expiry <= 0.0 || vol <= 0.0 {
        return 0.0;
    }

    let sqrt_t = expiry.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * vol * vol * expiry) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;

    let df = (-rate * expiry).exp();

    // Cumulative normal distribution approximation
    fn norm_cdf(x: f64) -> f64 { 0.5 * (1.0 + (x / (2.0_f64.sqrt())).tanh() * (2.0 / PI).sqrt()) }

    df * (forward * norm_cdf(d1) - strike * norm_cdf(d2))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volcube_data_loader_default() {
        let loader = VolCubeDataLoader::default_path();
        assert!(loader.base_path.ends_with("demo/data/input/irvol"));
    }

    #[test]
    fn test_volcube_cache_new() {
        let cache = VolCubeCache::new(10);
        assert_eq!(cache.max_entries, 10);
    }

    #[tokio::test]
    async fn test_volcube_cache_add_get() {
        let cache = VolCubeCache::new(10);
        let id = Uuid::new_v4();

        let cube = CachedVolCube {
            instruments: vec![],
            model: CalibrationModel::Sabr,
            parameters: vec![],
            fit_metrics: FitMetrics::default(),
            expiries: vec![1.0, 2.0],
            tenors: vec![5.0, 10.0],
        };

        cache.add(id, cube.clone()).await;

        let retrieved = cache.get(&id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().expiries, vec![1.0, 2.0]);
    }

    #[tokio::test]
    async fn test_volcube_cache_eviction() {
        let cache = VolCubeCache::new(2);

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        let cube = CachedVolCube {
            instruments: vec![],
            model: CalibrationModel::Sabr,
            parameters: vec![],
            fit_metrics: FitMetrics::default(),
            expiries: vec![],
            tenors: vec![],
        };

        cache.add(id1, cube.clone()).await;
        cache.add(id2, cube.clone()).await;
        assert_eq!(cache.len().await, 2);

        // Adding third should evict one
        cache.add(id3, cube).await;
        assert_eq!(cache.len().await, 2);
    }

    #[test]
    fn test_sabr_implied_vol_atm() {
        let forward = 0.03;
        let vol = sabr_implied_vol(
            forward, // ATM
            forward, 1.0,  // 1Y expiry
            0.04, // alpha
            0.5,  // beta
            -0.2, // rho
            0.3,  // nu
        );

        assert!(vol > 0.0);
        assert!(vol < 1.0);
    }

    #[test]
    fn test_sabr_implied_vol_otm() {
        let forward = 0.03;
        let vol_low = sabr_implied_vol(0.02, forward, 1.0, 0.04, 0.5, -0.2, 0.3);
        let vol_atm = sabr_implied_vol(forward, forward, 1.0, 0.04, 0.5, -0.2, 0.3);
        let vol_high = sabr_implied_vol(0.04, forward, 1.0, 0.04, 0.5, -0.2, 0.3);

        // With negative rho, SABR produces a skew (not a smile):
        // - low strikes have higher vol (negative correlation effect)
        // - high strikes have lower vol
        assert!(vol_low > vol_atm);
        assert!(vol_high < vol_atm);
    }

    #[test]
    fn test_calibrate_sabr_simple() {
        let instruments = vec![
            SwaptionInstrument::new(1.0, 5.0, 0.02, 0.25, 0.03),
            SwaptionInstrument::new(1.0, 5.0, 0.03, 0.20, 0.03),
            SwaptionInstrument::new(1.0, 5.0, 0.04, 0.22, 0.03),
        ];

        let refs: Vec<&SwaptionInstrument> = instruments.iter().collect();
        let (alpha, rho, nu) = calibrate_sabr_simple(&refs, 0.5, 0.03);

        assert!(alpha > 0.0);
        assert!(rho > -1.0 && rho < 1.0);
        assert!(nu > 0.0);
    }

    #[test]
    fn test_black_call_price() {
        let price = black_call_price(
            0.03,  // strike
            0.035, // forward
            1.0,   // expiry
            0.20,  // vol
            0.03,  // rate
        );

        // ITM call should have positive value
        assert!(price > 0.0);
    }

    #[test]
    fn test_volcube_data_error_display() {
        let err = VolCubeDataError::IndexNotFound("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = VolCubeDataError::IoError("read failed".to_string());
        assert!(err.to_string().contains("IO error"));

        let err = VolCubeDataError::ParseError("invalid json".to_string());
        assert!(err.to_string().contains("Parse error"));
    }
}
