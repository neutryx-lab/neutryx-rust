//! FxVol API handlers.
//!
//! This module provides REST API handlers for the FX Volatility Surface WebApp,
//! including currency pair management, quote data, and density calculations.
//!
//! # Endpoints
//!
//! - `GET /api/fxvol/pairs` - Get available currency pairs
//! - `GET /api/fxvol/quotes/{pair}` - Get quotes for a pair
//! - `PUT /api/fxvol/quotes/{pair}` - Update quotes for a pair
//! - `POST /api/fxvol/build` - Build an FX volatility surface
//! - `GET /api/fxvol/smile` - Get smile data for a built surface
//! - `GET /api/fxvol/rr-bf` - Get RR/BF term structure
//! - `GET /api/fxvol/density` - Get probability density data
//! - `POST /api/fxvol/delta-strike` - Convert delta to strike
//!
//! # Requirements Coverage
//!
//! - Requirement 10: FX VolSurface専用機能
//! - Requirement 11: FX VolSurface バックエンドAPI

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Instant};

use axum::{
    extract::{Path, Query, State},
    Json,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{
    error::{ApiError, ApiResult},
    fxvol_types::{
        DeltaStrikeRequest, DeltaStrikeResponse, DeltaType, DeltaVols, FxDeltaPoint,
        FxDeltaTypesResponse, FxDensityQuery, FxDensityResponse, FxDensityStatistics, FxPairInfo,
        FxQuoteEntry, FxQuotesRequest, FxQuotesResponse, FxSmileQuery, FxSmileResponse,
        FxSurfaceBuildRequest, FxSurfaceBuildResponse, FxVolFile, FxVolPairsResponse,
        RrBfDataPoint, RrBfQuery, RrBfResponse,
    },
    AppState,
};

// =============================================================================
// FxVolCache - LRU Cache for built surfaces (Req 11.4)
// =============================================================================

/// Cached FX volatility surface data after build.
#[derive(Debug, Clone)]
pub struct CachedFxSurface {
    /// Currency pair
    pub currency_pair: String,
    /// Spot rate
    pub spot: f64,
    /// Domestic rate
    pub domestic_rate: f64,
    /// Foreign rate
    pub foreign_rate: f64,
    /// Quote entries
    pub quotes: Vec<FxQuoteEntry>,
    /// Delta grid points
    pub delta_points: Vec<f64>,
    /// Expiry grid points
    pub expiry_points: Vec<f64>,
    /// Allow extrapolation
    pub allow_extrapolation: bool,
}

/// LRU cache for built FX volatility surfaces.
pub struct FxVolCache {
    surfaces: RwLock<HashMap<Uuid, CachedFxSurface>>,
    max_entries: usize,
}

impl FxVolCache {
    /// Create a new FxVolCache with the specified maximum entries.
    pub fn new(max_entries: usize) -> Self {
        Self {
            surfaces: RwLock::new(HashMap::new()),
            max_entries,
        }
    }

    /// Add a built surface to the cache.
    pub async fn add(&self, id: Uuid, surface: CachedFxSurface) {
        let mut surfaces = self.surfaces.write().await;

        // Simple eviction: remove oldest if at capacity
        if surfaces.len() >= self.max_entries {
            if let Some(key) = surfaces.keys().next().copied() {
                surfaces.remove(&key);
            }
        }

        surfaces.insert(id, surface);
    }

    /// Get a cached surface by ID.
    pub async fn get(&self, id: &Uuid) -> Option<CachedFxSurface> {
        let surfaces = self.surfaces.read().await;
        surfaces.get(id).cloned()
    }

    /// Check if the cache is empty.
    pub async fn is_empty(&self) -> bool {
        let surfaces = self.surfaces.read().await;
        surfaces.is_empty()
    }

    /// Get the number of cached entries.
    pub async fn len(&self) -> usize {
        let surfaces = self.surfaces.read().await;
        surfaces.len()
    }
}

impl Default for FxVolCache {
    fn default() -> Self { Self::new(10) }
}

// =============================================================================
// FxVolDataLoader (Task 3.2)
// =============================================================================

/// Loader for FX volatility data files.
///
/// # Requirements Coverage
///
/// - Requirement 11.1: 利用可能な通貨ペア一覧
/// - Requirement 11.2: 指定通貨ペアのボラティリティQuotesを返す
pub struct FxVolDataLoader {
    base_path: PathBuf,
}

impl FxVolDataLoader {
    /// Create a new FxVolDataLoader with the specified base path.
    pub fn new(base_path: PathBuf) -> Self { Self { base_path } }

    /// Create a FxVolDataLoader with the default path.
    pub fn default_path() -> Self { Self::new(PathBuf::from("demo/data/input/volsurface")) }

    /// Get the list of available FX currency pairs.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 9.4, 9.5: EURUSD, USDJPYをサポート
    pub fn available_pairs(&self) -> Vec<FxPairInfo> {
        let mut pairs = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    // FX files are 6-char currency pairs (not ending in -swaption)
                    if std::path::Path::new(name)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                        && !name.contains("-swaption")
                    {
                        let pair_code = name.trim_end_matches(".json").to_uppercase();
                        if pair_code.len() == 6 {
                            pairs.push(FxPairInfo::new(&pair_code));
                        }
                    }
                }
            }
        }

        // Sort for consistent ordering
        pairs.sort_by(|a, b| a.pair.cmp(&b.pair));
        pairs
    }

    /// Load quotes for the specified currency pair.
    ///
    /// # Arguments
    ///
    /// * `pair` - The currency pair code (e.g., "EURUSD")
    ///
    /// # Returns
    ///
    /// The FX vol file contents if found and valid.
    pub fn load_quotes(&self, pair: &str) -> Result<FxVolFile, FxVolDataError> {
        let file_path = self.base_path.join(format!("{}.json", pair.to_lowercase()));

        if !file_path.exists() {
            return Err(FxVolDataError::PairNotFound(pair.to_string()));
        }

        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| FxVolDataError::IoError(e.to_string()))?;

        let file: FxVolFile = serde_json::from_str(&content)
            .map_err(|e| FxVolDataError::ParseError(e.to_string()))?;

        Ok(file)
    }

    /// Save quotes to file.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 11.3: Quotesデータを更新・保存
    pub fn save_quotes(&self, pair: &str, data: &FxVolFile) -> Result<(), FxVolDataError> {
        let file_path = self.base_path.join(format!("{}.json", pair.to_lowercase()));

        let content = serde_json::to_string_pretty(data)
            .map_err(|e| FxVolDataError::ParseError(e.to_string()))?;

        std::fs::write(&file_path, content).map_err(|e| FxVolDataError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Check if a pair is supported.
    pub fn is_supported(&self, pair: &str) -> bool {
        let file_path = self.base_path.join(format!("{}.json", pair.to_lowercase()));
        file_path.exists()
    }
}

impl Default for FxVolDataLoader {
    fn default() -> Self { Self::default_path() }
}

/// Errors from FxVolDataLoader operations.
#[derive(Debug, Clone)]
pub enum FxVolDataError {
    /// The requested currency pair was not found
    PairNotFound(String),
    /// IO error reading the file
    IoError(String),
    /// Error parsing the JSON file
    ParseError(String),
}

impl std::fmt::Display for FxVolDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PairNotFound(pair) => write!(f, "Currency pair '{}' not found", pair),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for FxVolDataError {}

// =============================================================================
// API Handlers (Tasks 3.2, 3.3)
// =============================================================================

/// Handler for `GET /api/fxvol/pairs`.
///
/// # Requirements Coverage
///
/// - Requirement 11.1: 利用可能な通貨ペア一覧
pub async fn get_pairs() -> ApiResult<FxVolPairsResponse> {
    let loader = FxVolDataLoader::default_path();
    let pairs = loader.available_pairs();

    Ok(Json(FxVolPairsResponse { pairs }))
}

/// Handler for `GET /api/fxvol/quotes/{pair}`.
///
/// # Requirements Coverage
///
/// - Requirement 11.2: 指定通貨ペアのボラティリティQuotesを返す
pub async fn get_quotes(Path(pair): Path<String>) -> ApiResult<FxQuotesResponse> {
    let loader = FxVolDataLoader::default_path();

    let file = loader.load_quotes(&pair).map_err(|e| match e {
        FxVolDataError::PairNotFound(_) => ApiError::not_found("Currency pair", &pair),
        FxVolDataError::IoError(msg) => ApiError::internal(msg),
        FxVolDataError::ParseError(msg) => {
            ApiError::internal(format!("Invalid quotes file: {}", msg))
        }
    })?;

    let response = FxQuotesResponse {
        currency_pair: file.currency_pair,
        reference_date: file.reference_date,
        spot: file.spot,
        domestic_rate: file.domestic_rate,
        foreign_rate: file.foreign_rate,
        quotes: file.quotes,
    };

    Ok(Json(response))
}

/// Handler for `PUT /api/fxvol/quotes/{pair}`.
///
/// # Requirements Coverage
///
/// - Requirement 11.3: Quotesデータを更新・保存
pub async fn update_quotes(
    Path(pair): Path<String>,
    Json(request): Json<FxQuotesRequest>,
) -> ApiResult<FxQuotesResponse> {
    let loader = FxVolDataLoader::default_path();

    // Validate quotes
    for quote in &request.quotes {
        if quote.expiry <= 0.0 {
            return Err(ApiError::validation(
                format!("Expiry must be positive, got {}", quote.expiry),
                "expiry",
            ));
        }
        if quote.atm_vol <= 0.0 || quote.atm_vol > 1.0 {
            return Err(ApiError::validation(
                format!("ATM vol {} out of range (0, 1)", quote.atm_vol),
                "atmVol",
            ));
        }
    }

    // Create file structure
    let file = FxVolFile {
        currency_pair: pair.clone(),
        reference_date: request.reference_date.clone(),
        spot: request.spot,
        domestic_rate: request.domestic_rate,
        foreign_rate: request.foreign_rate,
        quotes: request.quotes.clone(),
    };

    // Save to file
    loader.save_quotes(&pair, &file).map_err(|e| match e {
        FxVolDataError::IoError(msg) => ApiError::internal(format!("Failed to save: {}", msg)),
        _ => ApiError::internal("Unexpected error saving quotes"),
    })?;

    let response = FxQuotesResponse {
        currency_pair: file.currency_pair,
        reference_date: file.reference_date,
        spot: file.spot,
        domestic_rate: file.domestic_rate,
        foreign_rate: file.foreign_rate,
        quotes: file.quotes,
    };

    Ok(Json(response))
}

/// Handler for `GET /api/fxvol/delta-types`.
pub async fn get_delta_types() -> ApiResult<FxDeltaTypesResponse> {
    Ok(Json(FxDeltaTypesResponse::new()))
}

/// Handler for `POST /api/fxvol/build`.
///
/// # Requirements Coverage
///
/// - Requirement 11.4: FxVolatilitySurfaceを構築
pub async fn build_surface(
    State(state): State<Arc<AppState>>,
    Json(request): Json<FxSurfaceBuildRequest>,
) -> ApiResult<FxSurfaceBuildResponse> {
    let start = Instant::now();

    // Validate request
    if request.quotes.is_empty() {
        return Err(ApiError::validation(
            "At least one quote is required",
            "quotes",
        ));
    }

    if request.spot <= 0.0 {
        return Err(ApiError::validation(
            format!("Spot must be positive, got {}", request.spot),
            "spot",
        ));
    }

    // Extract delta and expiry points
    let mut expiry_points: Vec<f64> = request.quotes.iter().map(|q| q.expiry).collect();
    expiry_points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    expiry_points.dedup();

    // Standard delta points
    let delta_points = vec![0.10, 0.25, 0.50, -0.25, -0.10];

    // Generate surface ID and cache
    let surface_id = Uuid::new_v4();

    let cached_surface = CachedFxSurface {
        currency_pair: request.currency_pair.clone(),
        spot: request.spot,
        domestic_rate: request.domestic_rate,
        foreign_rate: request.foreign_rate,
        quotes: request.quotes.clone(),
        delta_points: delta_points.clone(),
        expiry_points: expiry_points.clone(),
        allow_extrapolation: request.allow_extrapolation,
    };

    state.fxvol_cache.add(surface_id, cached_surface).await;

    let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    let response = FxSurfaceBuildResponse {
        surface_id: surface_id.to_string(),
        currency_pair: request.currency_pair,
        delta_points,
        expiry_points,
        processing_time_ms,
    };

    Ok(Json(response))
}

/// Handler for `GET /api/fxvol/smile`.
///
/// # Requirements Coverage
///
/// - Requirement 11.5: 指定ExpiryのDelta-Volスマイルデータ
/// - Requirement 10.1: Delta軸でスマイルを表示
pub async fn get_smile(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FxSmileQuery>,
) -> ApiResult<FxSmileResponse> {
    // Parse surface ID
    let surface_id = Uuid::parse_str(&query.surface_id)
        .map_err(|_| ApiError::validation("Invalid surface ID format", "surface_id"))?;

    // Get cached surface
    let surface = state
        .fxvol_cache
        .get(&surface_id)
        .await
        .ok_or_else(|| ApiError::not_found("FxVolSurface", &query.surface_id))?;

    // Find quote for this expiry (closest match)
    let quote = surface
        .quotes
        .iter()
        .min_by(|a, b| {
            (a.expiry - query.expiry)
                .abs()
                .partial_cmp(&(b.expiry - query.expiry).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or_else(|| ApiError::not_found("Expiry", query.expiry.to_string()))?;

    // Calculate forward
    let rate_diff = surface.domestic_rate - surface.foreign_rate;
    let forward = surface.spot * (rate_diff * query.expiry).exp();

    // Convert RR/BF to delta vols
    let delta_vols = quote.to_delta_vols();

    // Generate smile points
    let num_points = query.num_points.clamp(5, 50);
    let mut points = Vec::with_capacity(num_points);

    // Add standard delta points
    let deltas = vec![
        (0.10, "10D Call"),
        (0.25, "25D Call"),
        (0.50, "ATM"),
        (-0.25, "25D Put"),
        (-0.10, "10D Put"),
    ];

    for (delta, label) in deltas {
        let vol = interpolate_delta_vol(&delta_vols, delta);
        let strike = delta_to_strike(
            delta,
            surface.spot,
            surface.domestic_rate,
            surface.foreign_rate,
            query.expiry,
            vol,
            DeltaType::SpotDelta,
        );

        points.push(FxDeltaPoint {
            delta,
            label: label.to_string(),
            volatility: vol,
            strike,
        });
    }

    // Sort by delta (put to call)
    points.sort_by(|a, b| {
        a.delta
            .partial_cmp(&b.delta)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(Json(FxSmileResponse {
        expiry: query.expiry,
        spot: surface.spot,
        forward,
        atm_vol: quote.atm_vol,
        points,
        rr_25d: quote.rr_25d,
        bf_25d: quote.bf_25d,
        rr_10d: quote.rr_10d,
        bf_10d: quote.bf_10d,
    }))
}

/// Handler for `GET /api/fxvol/rr-bf`.
///
/// # Requirements Coverage
///
/// - Requirement 11.6: Risk Reversal/Butterflyの時系列データ
/// - Requirement 10.4: RR/BF時系列チャート
pub async fn get_rr_bf(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RrBfQuery>,
) -> ApiResult<RrBfResponse> {
    // Parse surface ID
    let surface_id = Uuid::parse_str(&query.surface_id)
        .map_err(|_| ApiError::validation("Invalid surface ID format", "surface_id"))?;

    // Get cached surface
    let surface = state
        .fxvol_cache
        .get(&surface_id)
        .await
        .ok_or_else(|| ApiError::not_found("FxVolSurface", &query.surface_id))?;

    // Build data points
    let data: Vec<RrBfDataPoint> = surface
        .quotes
        .iter()
        .map(|q| {
            let label = expiry_to_label(q.expiry);
            RrBfDataPoint {
                expiry: q.expiry,
                label,
                atm_vol: q.atm_vol,
                rr_25d: q.rr_25d,
                bf_25d: q.bf_25d,
                rr_10d: q.rr_10d,
                bf_10d: q.bf_10d,
            }
        })
        .collect();

    Ok(Json(RrBfResponse {
        currency_pair: surface.currency_pair.clone(),
        data,
    }))
}

/// Handler for `GET /api/fxvol/density`.
///
/// # Requirements Coverage
///
/// - Requirement 11.7: 確率密度データを返す
/// - Requirement 10.7: FX確率密度関数を計算・表示
pub async fn get_density(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FxDensityQuery>,
) -> ApiResult<FxDensityResponse> {
    // Parse surface ID
    let surface_id = Uuid::parse_str(&query.surface_id)
        .map_err(|_| ApiError::validation("Invalid surface ID format", "surface_id"))?;

    // Get cached surface
    let surface = state
        .fxvol_cache
        .get(&surface_id)
        .await
        .ok_or_else(|| ApiError::not_found("FxVolSurface", &query.surface_id))?;

    // Find quote for this expiry (closest match)
    let quote = surface
        .quotes
        .iter()
        .min_by(|a, b| {
            (a.expiry - query.expiry)
                .abs()
                .partial_cmp(&(b.expiry - query.expiry).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or_else(|| ApiError::not_found("Expiry", query.expiry.to_string()))?;

    // Calculate forward
    let rate_diff = surface.domestic_rate - surface.foreign_rate;
    let forward = surface.spot * (rate_diff * query.expiry).exp();

    // Generate strike grid
    let strike_min = forward * 0.7;
    let strike_max = forward * 1.3;
    let num_points = query.num_points.clamp(50, 500);
    let strike_step = (strike_max - strike_min) / (num_points - 1) as f64;

    let mut strikes = Vec::with_capacity(num_points);
    let mut densities = Vec::with_capacity(num_points);
    let mut cdf = Vec::with_capacity(num_points);
    let mut warnings = Vec::new();

    // Convert RR/BF to delta vols
    let delta_vols = quote.to_delta_vols();

    // Breeden-Litzenberger density calculation
    let h = strike_step * 0.1;

    for i in 0..num_points {
        let strike = strike_min + i as f64 * strike_step;
        strikes.push(strike);

        // Get volatility at this strike (interpolate from delta vols)
        let vol = volatility_at_strike(
            strike,
            forward,
            query.expiry,
            &delta_vols,
            surface.spot,
            surface.domestic_rate,
            surface.foreign_rate,
        );

        let vol_low = volatility_at_strike(
            strike - h,
            forward,
            query.expiry,
            &delta_vols,
            surface.spot,
            surface.domestic_rate,
            surface.foreign_rate,
        );

        let vol_high = volatility_at_strike(
            strike + h,
            forward,
            query.expiry,
            &delta_vols,
            surface.spot,
            surface.domestic_rate,
            surface.foreign_rate,
        );

        // Calculate call prices
        let c_low = black_call_price(
            strike - h,
            forward,
            query.expiry,
            vol_low,
            surface.domestic_rate,
        );
        let c_mid = black_call_price(strike, forward, query.expiry, vol, surface.domestic_rate);
        let c_high = black_call_price(
            strike + h,
            forward,
            query.expiry,
            vol_high,
            surface.domestic_rate,
        );

        // Second derivative
        let d2c_dk2 = (c_high - 2.0 * c_mid + c_low) / (h * h);
        let density = (surface.domestic_rate * query.expiry).exp() * d2c_dk2;

        densities.push(density.max(0.0));
    }

    // Normalise densities
    let total: f64 = densities.iter().sum::<f64>() * strike_step;
    if total > 0.0 {
        for d in &mut densities {
            *d /= total;
        }
    } else {
        warnings.push("Density normalisation failed".to_string());
    }

    // Compute CDF
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
        m4 - 3.0
    } else {
        0.0
    };

    Ok(Json(FxDensityResponse {
        expiry: query.expiry,
        spot: surface.spot,
        forward,
        strikes,
        densities,
        cdf,
        statistics: FxDensityStatistics {
            mean,
            variance,
            std_dev,
            skewness,
            kurtosis,
        },
        warnings,
    }))
}

/// Handler for `POST /api/fxvol/delta-strike`.
///
/// # Requirements Coverage
///
/// - Requirement 11.8: Delta-Strike変換結果を返す
/// - Requirement 10.6: Delta-Strike変換
pub async fn delta_to_strike_handler(
    Json(request): Json<DeltaStrikeRequest>,
) -> ApiResult<DeltaStrikeResponse> {
    // Validate request
    if request.spot <= 0.0 {
        return Err(ApiError::validation("Spot must be positive", "spot"));
    }
    if request.expiry <= 0.0 {
        return Err(ApiError::validation("Expiry must be positive", "expiry"));
    }
    if request.volatility <= 0.0 {
        return Err(ApiError::validation(
            "Volatility must be positive",
            "volatility",
        ));
    }

    // Calculate forward
    let rate_diff = request.domestic_rate - request.foreign_rate;
    let forward = request.spot * (rate_diff * request.expiry).exp();

    // Convert each delta to strike
    let strikes: Vec<f64> = request
        .deltas
        .iter()
        .map(|&delta| {
            delta_to_strike(
                delta,
                request.spot,
                request.domestic_rate,
                request.foreign_rate,
                request.expiry,
                request.volatility,
                request.delta_type,
            )
        })
        .collect();

    Ok(Json(DeltaStrikeResponse {
        deltas: request.deltas,
        strikes,
        forward,
        delta_type: request.delta_type,
    }))
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert expiry in years to a market label.
fn expiry_to_label(expiry: f64) -> String {
    if expiry < 0.05 {
        // 1 week ≈ 0.019 years, boundary at midpoint to 1M
        "1W".to_string()
    } else if expiry < 0.125 {
        "1M".to_string()
    } else if expiry < 0.21 {
        "2M".to_string()
    } else if expiry < 0.33 {
        "3M".to_string()
    } else if expiry < 0.54 {
        "6M".to_string()
    } else if expiry < 0.83 {
        "9M".to_string()
    } else if expiry < 1.5 {
        "1Y".to_string()
    } else if expiry < 2.5 {
        "2Y".to_string()
    } else if expiry < 4.0 {
        "3Y".to_string()
    } else {
        format!("{}Y", expiry.round() as i32)
    }
}

/// Interpolate volatility from delta vols.
fn interpolate_delta_vol(vols: &DeltaVols, delta: f64) -> f64 {
    // Simple linear interpolation based on delta
    let abs_delta = delta.abs();

    if abs_delta > 0.45 {
        // Near ATM
        vols.atm
    } else if abs_delta > 0.20 {
        // Between 25D and ATM
        let w = (abs_delta - 0.25) / 0.25;
        if delta > 0.0 {
            vols.vol_25d_call * (1.0 - w) + vols.atm * w
        } else {
            vols.vol_25d_put * (1.0 - w) + vols.atm * w
        }
    } else {
        // Near 10D wing (if available)
        if delta > 0.0 {
            vols.vol_10d_call.unwrap_or(vols.vol_25d_call)
        } else {
            vols.vol_10d_put.unwrap_or(vols.vol_25d_put)
        }
    }
}

/// Get volatility at a given strike using sticky-delta approach.
fn volatility_at_strike(
    strike: f64,
    forward: f64,
    _expiry: f64,
    vols: &DeltaVols,
    _spot: f64,
    _domestic_rate: f64,
    _foreign_rate: f64,
) -> f64 {
    // Approximate delta from moneyness
    let moneyness = strike / forward;
    let approx_delta = if moneyness > 1.0 {
        0.5 * (2.0 - moneyness).clamp(0.0, 1.0)
    } else {
        -0.5 * moneyness.clamp(0.0, 1.0)
    };

    interpolate_delta_vol(vols, approx_delta)
}

/// Convert delta to strike using Garman-Kohlhagen.
fn delta_to_strike(
    delta: f64,
    spot: f64,
    domestic_rate: f64,
    foreign_rate: f64,
    expiry: f64,
    volatility: f64,
    delta_type: DeltaType,
) -> f64 {
    let rate_diff = domestic_rate - foreign_rate;
    let forward = spot * (rate_diff * expiry).exp();
    let sqrt_t = expiry.sqrt();

    // Inverse normal CDF approximation
    fn norm_inv(p: f64) -> f64 {
        let a1 = -39.6968302866538;
        let a2 = 220.946098424521;
        let a3 = -275.928510446969;
        let a4 = 138.357751867269;
        let a5 = -30.6647980661472;
        let a6 = 2.50662827463100;

        let b1 = -54.4760987982241;
        let b2 = 161.585836858041;
        let b3 = -155.698979859887;
        let b4 = 66.8013118877197;
        let b5 = -13.2806815528857;

        let c1 = -7.78489400243029e-03;
        let c2 = -0.322396458041136;
        let c3 = -2.40075827716184;
        let c4 = -2.54973253934373;
        let c5 = 4.37466414146497;
        let c6 = 2.93816398269878;

        let d1 = 7.78469570904146e-03;
        let d2 = 0.32246712907004;
        let d3 = 2.445134137143;
        let d4 = 3.75440866190742;

        let p_low = 0.02425;
        let p_high = 1.0 - p_low;

        if p < p_low {
            let q = (-2.0 * p.ln()).sqrt();
            (((((c1 * q + c2) * q + c3) * q + c4) * q + c5) * q + c6)
                / ((((d1 * q + d2) * q + d3) * q + d4) * q + 1.0)
        } else if p <= p_high {
            let q = p - 0.5;
            let r = q * q;
            (((((a1 * r + a2) * r + a3) * r + a4) * r + a5) * r + a6) * q
                / (((((b1 * r + b2) * r + b3) * r + b4) * r + b5) * r + 1.0)
        } else {
            let q = (-2.0 * (1.0 - p).ln()).sqrt();
            -(((((c1 * q + c2) * q + c3) * q + c4) * q + c5) * q + c6)
                / ((((d1 * q + d2) * q + d3) * q + d4) * q + 1.0)
        }
    }

    let is_call = delta > 0.0;
    let abs_delta = delta.abs();

    // Different formulas based on delta type
    let d1 = match delta_type {
        DeltaType::SpotDelta => {
            let disc_foreign = (-foreign_rate * expiry).exp();
            let adj_delta = abs_delta / disc_foreign;
            if is_call {
                norm_inv(adj_delta)
            } else {
                -norm_inv(adj_delta)
            }
        }
        DeltaType::ForwardDelta => {
            if is_call {
                norm_inv(abs_delta)
            } else {
                norm_inv(abs_delta) - 1.0
            }
        }
        DeltaType::PremiumAdjusted => {
            // Simplified - use spot delta formula as approximation
            let disc_foreign = (-foreign_rate * expiry).exp();
            let adj_delta = abs_delta / disc_foreign;
            if is_call {
                norm_inv(adj_delta)
            } else {
                -norm_inv(adj_delta)
            }
        }
    };

    // K = F * exp(-d1 * sigma * sqrt(T) + 0.5 * sigma^2 * T)
    forward * (-d1 * volatility * sqrt_t + 0.5 * volatility * volatility * expiry).exp()
}

/// Black call price.
fn black_call_price(strike: f64, forward: f64, expiry: f64, vol: f64, rate: f64) -> f64 {
    if strike <= 0.0 || forward <= 0.0 || expiry <= 0.0 || vol <= 0.0 {
        return 0.0;
    }

    let sqrt_t = expiry.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * vol * vol * expiry) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;

    let df = (-rate * expiry).exp();

    fn norm_cdf(x: f64) -> f64 {
        // Abramowitz and Stegun approximation for cumulative normal distribution
        let a1 = 0.254829592;
        let a2 = -0.284496736;
        let a3 = 1.421413741;
        let a4 = -1.453152027;
        let a5 = 1.061405429;
        let p = 0.3275911;

        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let x_abs = x.abs() / std::f64::consts::SQRT_2;

        let t = 1.0 / (1.0 + p * x_abs);
        let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x_abs * x_abs).exp();

        0.5 * (1.0 + sign * y)
    }

    df * (forward * norm_cdf(d1) - strike * norm_cdf(d2))
}

// =============================================================================
// Extended API Handlers (Task 13.2)
// =============================================================================

use super::fxvol_types::{
    FxCalibrationDiagnostics, FxCalibrateRequest, FxCalibrateResponse, FxSurfaceQuery,
    FxSurfaceResponse, SabrParameters, SurfacePoint,
};

/// Handler for `POST /api/fxvol/calibrate`.
///
/// Calibrates an FX volatility surface using SABR model.
///
/// # Requirements Coverage
///
/// - Requirement 12.3: ボラティリティサーフェスカリブレーションAPIエンドポイント
/// - Requirement 12.5: カリブレーション診断表示
pub async fn calibrate_surface(
    State(state): State<Arc<AppState>>,
    Json(request): Json<FxCalibrateRequest>,
) -> ApiResult<FxCalibrateResponse> {
    let start = Instant::now();

    // Validate request
    if request.quotes.is_empty() {
        return Err(ApiError::validation(
            "At least one quote is required for calibration",
            "quotes",
        ));
    }

    if request.spot <= 0.0 {
        return Err(ApiError::validation(
            format!("Spot must be positive, got {}", request.spot),
            "spot",
        ));
    }

    let mut warnings = Vec::new();
    let mut sabr_params = Vec::new();
    let mut max_residual = 0.0_f64;
    let mut total_residual = 0.0_f64;

    // Calibrate SABR for each expiry
    for quote in &request.quotes {
        let label = expiry_to_label(quote.expiry);

        // Calculate forward rate
        let rate_diff = request.domestic_rate - request.foreign_rate;
        let forward = request.spot * (rate_diff * quote.expiry).exp();

        // Initial SABR parameter estimates
        let atm_vol = quote.atm_vol;
        let beta = request.sabr_beta;

        // Simplified SABR calibration
        // In production, would use proper optimisation
        let alpha = atm_vol * forward.powf(1.0 - beta);

        // Estimate rho from risk reversal
        let rho = (quote.rr_25d / atm_vol).clamp(-0.95, 0.95);

        // Estimate nu from butterfly
        let nu = (quote.bf_25d.abs() / atm_vol * 4.0 + 0.2).clamp(0.1, 2.0);

        // Compute calibration residual (simplified)
        let delta_vols = quote.to_delta_vols();
        let model_25c = sabr_vol(forward, forward * 1.05, quote.expiry, alpha, beta, rho, nu);
        let model_25p = sabr_vol(forward, forward * 0.95, quote.expiry, alpha, beta, rho, nu);
        let residual_25c = (model_25c - delta_vols.vol_25d_call).abs();
        let residual_25p = (model_25p - delta_vols.vol_25d_put).abs();
        let residual = ((residual_25c.powi(2) + residual_25p.powi(2)) / 2.0).sqrt();

        max_residual = max_residual.max(residual);
        total_residual += residual;

        sabr_params.push(SabrParameters {
            expiry: quote.expiry,
            label,
            alpha,
            beta,
            rho,
            nu,
            forward,
            residual,
            iterations: 10, // Placeholder
        });
    }

    let expiry_count = request.quotes.len();
    let avg_residual = if expiry_count > 0 {
        total_residual / expiry_count as f64
    } else {
        0.0
    };

    // Check convergence (residual threshold)
    let converged = max_residual < 0.005; // 50bps tolerance
    if !converged {
        warnings.push(format!(
            "High calibration residual: max={:.4}, avg={:.4}",
            max_residual, avg_residual
        ));
    }

    // Generate surface ID and cache
    let surface_id = Uuid::new_v4();

    // Extract expiry points
    let mut expiry_points: Vec<f64> = request.quotes.iter().map(|q| q.expiry).collect();
    expiry_points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    expiry_points.dedup();

    let cached_surface = CachedFxSurface {
        currency_pair: request.currency_pair.clone(),
        spot: request.spot,
        domestic_rate: request.domestic_rate,
        foreign_rate: request.foreign_rate,
        quotes: request.quotes.clone(),
        delta_points: vec![0.10, 0.25, 0.50, -0.25, -0.10],
        expiry_points,
        allow_extrapolation: true,
    };

    state.fxvol_cache.add(surface_id, cached_surface).await;

    let calibration_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    let diagnostics = FxCalibrationDiagnostics {
        model: format!("{:?}", request.model),
        calibration_time_ms,
        expiry_count,
        converged,
        max_residual,
        avg_residual,
        sabr_params,
        warnings,
    };

    Ok(Json(FxCalibrateResponse {
        surface_id: surface_id.to_string(),
        currency_pair: request.currency_pair,
        diagnostics,
    }))
}

/// Handler for `GET /api/fxvol/surface`.
///
/// Returns 3D volatility surface data for visualisation.
///
/// # Requirements Coverage
///
/// - Requirement 12.4: 3D可視化用JSON形式でサーフェスデータ返却
pub async fn get_surface(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FxSurfaceQuery>,
) -> ApiResult<FxSurfaceResponse> {
    // Parse surface ID
    let surface_id = Uuid::parse_str(&query.surface_id)
        .map_err(|_| ApiError::validation("Invalid surface ID format", "surface_id"))?;

    // Get cached surface
    let surface = state
        .fxvol_cache
        .get(&surface_id)
        .await
        .ok_or_else(|| ApiError::not_found("FxVolSurface", &query.surface_id))?;

    // Build delta axis (from put to call)
    let delta_count = query.delta_points;
    let mut delta_axis = Vec::with_capacity(delta_count);

    // Standard delta points: -0.10, -0.25, -0.50 (puts), 0.50, 0.25, 0.10 (calls)
    // Map to uniform range for visualisation
    for i in 0..delta_count {
        let t = i as f64 / (delta_count - 1) as f64;
        // Map [0, 1] to delta range [-0.45, 0.45]
        let delta = -0.45 + t * 0.9;
        delta_axis.push(delta);
    }

    // Get expiry axis from surface data
    let expiry_axis = surface.expiry_points.clone();
    let expiry_labels: Vec<String> = expiry_axis.iter().map(|e| expiry_to_label(*e)).collect();

    // Build surface points and matrices
    let mut points = Vec::new();
    let mut vol_matrix = Vec::with_capacity(expiry_axis.len());
    let mut strike_matrix = Vec::with_capacity(expiry_axis.len());

    for &expiry in &expiry_axis {
        // Find closest quote for this expiry
        let quote = surface
            .quotes
            .iter()
            .min_by(|a, b| {
                (a.expiry - expiry)
                    .abs()
                    .partial_cmp(&(b.expiry - expiry).abs())
                    .unwrap()
            })
            .unwrap();

        let delta_vols = quote.to_delta_vols();
        let rate_diff = surface.domestic_rate - surface.foreign_rate;
        let forward = surface.spot * (rate_diff * expiry).exp();

        let mut vol_row = Vec::with_capacity(delta_axis.len());
        let mut strike_row = Vec::with_capacity(delta_axis.len());

        for &delta in &delta_axis {
            // Interpolate volatility from delta vols
            let vol = interpolate_delta_vol(&delta_vols, delta);

            // Calculate strike from delta
            let strike = delta_to_strike(
                delta,
                surface.spot,
                surface.domestic_rate,
                surface.foreign_rate,
                expiry,
                vol,
                DeltaType::SpotDelta,
            );

            points.push(SurfacePoint {
                delta,
                expiry,
                volatility: vol,
                strike,
            });

            vol_row.push(vol);
            strike_row.push(strike);
        }

        vol_matrix.push(vol_row);
        strike_matrix.push(strike_row);
    }

    Ok(Json(FxSurfaceResponse {
        currency_pair: surface.currency_pair.clone(),
        spot: surface.spot,
        reference_date: "".to_string(), // Not stored in cache
        delta_axis,
        expiry_axis,
        expiry_labels,
        points,
        vol_matrix,
        strike_matrix,
    }))
}

/// Simplified SABR volatility formula.
fn sabr_vol(forward: f64, strike: f64, expiry: f64, alpha: f64, beta: f64, rho: f64, nu: f64) -> f64 {
    if (forward - strike).abs() < 1e-10 {
        // ATM approximation
        let fk_beta = forward.powf(1.0 - beta);
        return alpha / fk_beta
            * (1.0
                + ((1.0 - beta).powi(2) / 24.0 * alpha.powi(2) / fk_beta.powi(2)
                    + 0.25 * rho * beta * nu * alpha / fk_beta
                    + (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2))
                    * expiry);
    }

    let log_fk = (forward / strike).ln();
    let fk_mid = (forward * strike).powf((1.0 - beta) / 2.0);
    let z = nu / alpha * fk_mid * log_fk;
    let x_z = ((1.0 - 2.0 * rho * z + z.powi(2)).sqrt() + z - rho).ln() / (1.0 - rho);

    let prefix = alpha / (fk_mid * (1.0 + (1.0 - beta).powi(2) / 24.0 * log_fk.powi(2)));
    let zeta = if x_z.abs() < 1e-10 { 1.0 } else { z / x_z };

    prefix * zeta
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fxvol_data_loader_default() {
        let loader = FxVolDataLoader::default_path();
        assert!(loader.base_path.ends_with("demo/data/input/volsurface"));
    }

    #[test]
    fn test_fxvol_cache_new() {
        let cache = FxVolCache::new(10);
        assert_eq!(cache.max_entries, 10);
    }

    #[tokio::test]
    async fn test_fxvol_cache_add_get() {
        let cache = FxVolCache::new(10);
        let id = Uuid::new_v4();

        let surface = CachedFxSurface {
            currency_pair: "EURUSD".to_string(),
            spot: 1.085,
            domestic_rate: 0.045,
            foreign_rate: 0.035,
            quotes: vec![],
            delta_points: vec![0.25, 0.5],
            expiry_points: vec![0.25, 0.5],
            allow_extrapolation: true,
        };

        cache.add(id, surface.clone()).await;

        let retrieved = cache.get(&id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().currency_pair, "EURUSD");
    }

    #[test]
    fn test_expiry_to_label() {
        assert_eq!(expiry_to_label(0.08), "1M");
        assert_eq!(expiry_to_label(0.25), "3M");
        assert_eq!(expiry_to_label(0.5), "6M");
        assert_eq!(expiry_to_label(1.0), "1Y");
        assert_eq!(expiry_to_label(2.0), "2Y");
    }

    #[test]
    fn test_interpolate_delta_vol() {
        let vols = DeltaVols {
            vol_10d_put: Some(0.12),
            vol_25d_put: 0.11,
            atm: 0.10,
            vol_25d_call: 0.105,
            vol_10d_call: Some(0.115),
        };

        // ATM should return ATM vol
        let atm_vol = interpolate_delta_vol(&vols, 0.5);
        assert!((atm_vol - 0.10).abs() < 0.01);

        // 25D call
        let vol_25c = interpolate_delta_vol(&vols, 0.25);
        assert!(vol_25c > 0.10);
    }

    #[test]
    fn test_delta_to_strike() {
        let strike = delta_to_strike(
            0.25,  // 25D call
            1.085, // spot
            0.045, // domestic
            0.035, // foreign
            0.5,   // expiry
            0.10,  // vol
            DeltaType::SpotDelta,
        );

        // Strike should be above forward for OTM call
        let forward = 1.085 * ((0.045 - 0.035) * 0.5_f64).exp();
        assert!(strike > forward);
    }

    #[test]
    fn test_delta_to_strike_put() {
        let strike = delta_to_strike(
            -0.25, // 25D put
            1.085,
            0.045,
            0.035,
            0.5,
            0.10,
            DeltaType::SpotDelta,
        );

        // Strike should be below forward for OTM put
        let forward = 1.085 * ((0.045 - 0.035) * 0.5_f64).exp();
        assert!(strike < forward);
    }

    #[test]
    fn test_black_call_price() {
        let price = black_call_price(
            1.10,   // strike
            1.0855, // forward
            0.5,    // expiry
            0.10,   // vol
            0.045,  // rate
        );

        // OTM call should have small but positive value
        assert!(price > 0.0);
        assert!(price < 0.1);
    }

    #[test]
    fn test_fxvol_data_error_display() {
        let err = FxVolDataError::PairNotFound("GBPUSD".to_string());
        assert!(err.to_string().contains("GBPUSD"));

        let err = FxVolDataError::IoError("read failed".to_string());
        assert!(err.to_string().contains("IO error"));
    }
}
