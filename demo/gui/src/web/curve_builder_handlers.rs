//! Curve Builder API handlers.
//!
//! This module provides REST API handlers for the Curve Builder WebApp,
//! including instrument list retrieval, curve construction, and parameter
//! queries.
//!
//! # Endpoints
//!
//! - `GET /api/curves/instruments/{index}` - Get instruments for an index
//! - `GET /api/curves/builders` - Get available builder methods
//! - `POST /api/curves/build` - Build a yield curve
//! - `GET /api/curves/{curveId}/parameters` - Get curve parameters
//!
//! # Requirements Coverage
//!
//! - Requirement 1: Index別Instrument入力データ管理
//! - Requirement 3: カーブBuilderモデル選択
//! - Requirement 4: カーブ構築実行
//! - Requirement 5: Parameterカーブ表示
//! - Requirement 7: API設計

use std::{path::PathBuf, sync::Arc, time::Instant};

use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

use super::{
    curve_builder_types::{
        BuildStatus, BuilderListResponse, CurveBuildRequest, CurveBuildResponse, CurveParameter,
        InstrumentFile, InstrumentInfo, InstrumentListResponse, ParameterPoint, ParameterQuery,
        ParameterResponse, ParameterType,
    },
    error::{ApiError, ApiResult},
    AppState,
};

// =============================================================================
// CurveDataLoader (Task 3.1)
// =============================================================================

/// Loader for index-based instrument data files.
///
/// # Requirements Coverage
///
/// - Requirement 1.1: `demo/data/input/curves/`
///   ディレクトリにIndex別のInstrumentリストJSONファイルを格納
/// - Requirement 1.5:
///   ファイルが存在しないか不正な形式の場合、適切なエラーメッセージを表示
pub struct CurveDataLoader {
    base_path: PathBuf,
}

impl CurveDataLoader {
    /// Create a new CurveDataLoader with the specified base path.
    pub fn new(base_path: PathBuf) -> Self { Self { base_path } }

    /// Create a CurveDataLoader with the default path.
    pub fn default_path() -> Self { Self::new(PathBuf::from("demo/data/input/curves")) }

    /// Get the list of available indices.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 1.4: USD-SOFR, EUR-ESTR,
    ///   JPY-TONAの3通貨をデフォルトでサポート
    pub fn available_indices(&self) -> Vec<String> {
        // Scan the directory for JSON files
        let mut indices = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if std::path::Path::new(name)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                    {
                        indices.push(name.trim_end_matches(".json").to_string());
                    }
                }
            }
        }

        // Sort for consistent ordering
        indices.sort();
        indices
    }

    /// Load instruments for the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index identifier (e.g., "usd-sofr")
    ///
    /// # Returns
    ///
    /// The instrument file contents if found and valid.
    ///
    /// # Errors
    ///
    /// Returns an error if the file doesn't exist or is invalid.
    pub fn load_instruments(&self, index: &str) -> Result<InstrumentFile, CurveDataError> {
        let file_path = self.base_path.join(format!("{}.json", index));

        if !file_path.exists() {
            return Err(CurveDataError::IndexNotFound(index.to_string()));
        }

        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| CurveDataError::IoError(e.to_string()))?;

        let file: InstrumentFile = serde_json::from_str(&content)
            .map_err(|e| CurveDataError::ParseError(e.to_string()))?;

        Ok(file)
    }

    /// Check if an index is supported.
    pub fn is_supported(&self, index: &str) -> bool {
        let file_path = self.base_path.join(format!("{}.json", index));
        file_path.exists()
    }
}

impl Default for CurveDataLoader {
    fn default() -> Self { Self::default_path() }
}

/// Errors from CurveDataLoader operations.
#[derive(Debug, Clone)]
pub enum CurveDataError {
    /// The requested index was not found
    IndexNotFound(String),
    /// IO error reading the file
    IoError(String),
    /// Error parsing the JSON file
    ParseError(String),
}

impl std::fmt::Display for CurveDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexNotFound(index) => write!(f, "Index '{}' not found", index),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for CurveDataError {}

// =============================================================================
// API Handlers (Tasks 3.2, 4.1, 5.1-5.3, 6.1-6.2)
// =============================================================================

/// Handler for `GET /api/curves/instruments/{index}`.
///
/// # Requirements Coverage
///
/// - Requirement 1.1-1.5: Index別Instrument入力データ管理
/// - Requirement 7.1: GET /api/curves/instruments/{index} エンドポイント
pub async fn get_instruments(Path(index): Path<String>) -> ApiResult<InstrumentListResponse> {
    let loader = CurveDataLoader::default_path();

    let file = loader.load_instruments(&index).map_err(|e| match e {
        CurveDataError::IndexNotFound(_) => ApiError::not_found("Index", &index),
        CurveDataError::IoError(msg) => ApiError::internal(msg),
        CurveDataError::ParseError(msg) => {
            ApiError::internal(format!("Invalid instrument file: {}", msg))
        }
    })?;

    let response = InstrumentListResponse {
        index: file.index,
        currency: file.currency,
        instruments: file
            .instruments
            .into_iter()
            .map(InstrumentInfo::from)
            .collect(),
    };

    Ok(Json(response))
}

/// Handler for `GET /api/curves/builders`.
///
/// # Requirements Coverage
///
/// - Requirement 3.1-3.2: 補間手法とブートストラップ手法の一覧
/// - Requirement 7.4: GET /api/curves/builders エンドポイント
pub async fn get_builders() -> ApiResult<BuilderListResponse> {
    Ok(Json(BuilderListResponse::new()))
}

/// Handler for `POST /api/curves/build`.
///
/// # Requirements Coverage
///
/// - Requirement 4.1-4.5: カーブ構築実行
/// - Requirement 7.2: POST /api/curves/build エンドポイント
pub async fn build_curve(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CurveBuildRequest>,
) -> ApiResult<CurveBuildResponse> {
    let start = Instant::now();

    // Validate request
    if request.instruments.is_empty() {
        return Err(ApiError::validation(
            "At least one instrument is required",
            "instruments",
        ));
    }

    // Check rate ranges (-10% to +50%)
    for inst in &request.instruments {
        if inst.rate < -0.10 || inst.rate > 0.50 {
            return Err(ApiError::validation(
                format!(
                    "Rate {} for {} is out of range (-10% to +50%)",
                    inst.rate, inst.tenor
                ),
                "rate",
            ));
        }
    }

    // Build the curve using existing bootstrap infrastructure
    // For now, we'll use a simplified approach that constructs pillar points
    let mut pillars: Vec<f64> = Vec::new();
    let mut discount_factors: Vec<f64> = Vec::new();
    let mut zero_rates: Vec<f64> = Vec::new();

    // Sort instruments by tenor for bootstrap
    let mut sorted_instruments = request.instruments.clone();
    sorted_instruments.sort_by(|a, b| {
        let ta = parse_tenor_years(&a.tenor);
        let tb = parse_tenor_years(&b.tenor);
        ta.partial_cmp(&tb).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Simple bootstrap: calculate discount factors from par rates
    for inst in &sorted_instruments {
        let tenor_years = parse_tenor_years(&inst.tenor);
        let rate = inst.rate;

        // For deposits/OIS, use simple discounting
        // DF = 1 / (1 + r * t)
        let df = 1.0 / (1.0 + rate * tenor_years);

        // Zero rate from discount factor
        // z = -ln(DF) / t
        let zero_rate = if tenor_years > 0.0 {
            -df.ln() / tenor_years
        } else {
            rate
        };

        pillars.push(tenor_years);
        discount_factors.push(df);
        zero_rates.push(zero_rate);
    }

    // Generate curve ID and cache
    let curve_id = Uuid::new_v4();

    // Store in the curve cache (using existing CachedCurve)
    use super::pricer_types::{CachedCurve, ParRateInput};

    let par_rates: Vec<ParRateInput> = sorted_instruments
        .iter()
        .map(|i| ParRateInput {
            tenor: i.tenor.clone(),
            rate: i.rate,
        })
        .collect();

    let cached_curve = CachedCurve::new(
        pillars.clone(),
        discount_factors.clone(),
        zero_rates.clone(),
        par_rates,
    );

    state.curve_cache.add(curve_id, cached_curve);

    let build_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Build parameters array for visualisation with forward rates
    let parameters: Vec<CurveParameter> = pillars
        .iter()
        .enumerate()
        .map(|(i, &tenor)| {
            let df = discount_factors[i];
            let zr = zero_rates[i];

            // Calculate forward rate from consecutive discount factors
            // f(t1, t2) = (ln(DF1) - ln(DF2)) / (t2 - t1)
            let forward_rate = if i > 0 {
                let prev_tenor = pillars[i - 1];
                let prev_df = discount_factors[i - 1];
                let dt = tenor - prev_tenor;
                if dt > 0.0 {
                    Some((prev_df.ln() - df.ln()) / dt)
                } else {
                    Some(zr) // Fallback to zero rate
                }
            } else {
                // First point: forward rate equals zero rate
                Some(zr)
            };

            CurveParameter {
                tenor_years: tenor,
                discount_factor: df,
                zero_rate: zr,
                forward_rate,
            }
        })
        .collect();

    let response = CurveBuildResponse {
        curve_id: curve_id.to_string(),
        status: BuildStatus::Success,
        index: request.index.clone(),
        interpolation_method: request.interpolation.display_name().to_string(),
        parameters,
        pillars,
        discount_factors,
        zero_rates,
        build_time_ms,
        instrument_count: sorted_instruments.len(),
    };

    Ok(Json(response))
}

/// Handler for `GET /api/curves/{curveId}/parameters`.
///
/// # Requirements Coverage
///
/// - Requirement 5.1-5.6: Parameterカーブ表示
/// - Requirement 7.3: GET /api/curves/{curveId}/parameters エンドポイント
pub async fn get_parameters(
    State(state): State<Arc<AppState>>,
    Path(curve_id): Path<String>,
    Query(query): Query<ParameterQuery>,
) -> ApiResult<ParameterResponse> {
    // Parse curve ID
    let uuid = Uuid::parse_str(&curve_id)
        .map_err(|_| ApiError::validation("Invalid curve ID format", "curveId"))?;

    // Get curve from cache
    let cached_curve = state
        .curve_cache
        .get(&uuid)
        .ok_or_else(|| ApiError::not_found("Curve", &curve_id))?;

    // Generate grid points
    let mut data = Vec::new();
    let mut t = query.start_year;

    while t <= query.end_year {
        let value = match query.r#type {
            ParameterType::DiscountFactor => interpolate_df(&cached_curve, t),
            ParameterType::ZeroRate => interpolate_zero_rate(&cached_curve, t),
            ParameterType::ForwardRate => {
                interpolate_forward_rate(&cached_curve, t, query.grid_interval)
            }
        };

        data.push(ParameterPoint { tenor: t, value });
        t += query.grid_interval;
    }

    let response = ParameterResponse {
        curve_id,
        parameter_type: query.r#type,
        data,
    };

    Ok(Json(response))
}

/// Handler for `GET /api/curves/indices`.
///
/// Returns the list of available index identifiers.
pub async fn get_indices() -> ApiResult<Vec<String>> {
    let loader = CurveDataLoader::default_path();
    Ok(Json(loader.available_indices()))
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Parse tenor string to years.
fn parse_tenor_years(tenor: &str) -> f64 {
    let tenor = tenor.to_uppercase();

    if let Some(num) = tenor.strip_suffix('Y') {
        num.parse::<f64>().unwrap_or(0.0)
    } else if let Some(num) = tenor.strip_suffix('M') {
        num.parse::<f64>().unwrap_or(0.0) / 12.0
    } else if let Some(num) = tenor.strip_suffix('W') {
        num.parse::<f64>().unwrap_or(0.0) / 52.0
    } else if let Some(num) = tenor.strip_suffix('D') {
        num.parse::<f64>().unwrap_or(0.0) / 365.0
    } else {
        // Try parsing as a number (already in years)
        tenor.parse::<f64>().unwrap_or(0.0)
    }
}

/// Interpolate discount factor at time t using log-linear interpolation.
fn interpolate_df(curve: &super::pricer_types::CachedCurve, t: f64) -> f64 {
    if t <= 0.0 {
        return 1.0;
    }

    let pillars = &curve.pillars;
    let dfs = &curve.discount_factors;

    if pillars.is_empty() {
        return 1.0;
    }

    if t <= pillars[0] {
        // Extrapolate from first point
        let log_df = dfs[0].ln() * t / pillars[0];
        return log_df.exp();
    }

    if t >= *pillars.last().unwrap() {
        // Extrapolate from last point
        let n = pillars.len();
        let log_df_last = dfs[n - 1].ln();
        let log_df_prev = dfs[n - 2].ln();
        let slope = (log_df_last - log_df_prev) / (pillars[n - 1] - pillars[n - 2]);
        let log_df = log_df_last + slope * (t - pillars[n - 1]);
        return log_df.exp();
    }

    // Find bracketing points
    for i in 1..pillars.len() {
        if t <= pillars[i] {
            let t0 = pillars[i - 1];
            let t1 = pillars[i];
            let log_df0 = dfs[i - 1].ln();
            let log_df1 = dfs[i].ln();

            // Log-linear interpolation
            let w = (t - t0) / (t1 - t0);
            let log_df = log_df0 + w * (log_df1 - log_df0);
            return log_df.exp();
        }
    }

    1.0
}

/// Interpolate zero rate at time t.
fn interpolate_zero_rate(curve: &super::pricer_types::CachedCurve, t: f64) -> f64 {
    if t <= 0.0 {
        // Return the short-end rate
        if !curve.zero_rates.is_empty() {
            return curve.zero_rates[0];
        }
        return 0.0;
    }

    let df = interpolate_df(curve, t);
    -df.ln() / t
}

/// Interpolate forward rate at time t with given interval.
fn interpolate_forward_rate(
    curve: &super::pricer_types::CachedCurve,
    t: f64,
    interval: f64,
) -> f64 {
    let df_t = interpolate_df(curve, t);
    let df_t_dt = interpolate_df(curve, t + interval);

    if interval > 0.0 && df_t_dt > 0.0 {
        (df_t / df_t_dt - 1.0) / interval
    } else {
        interpolate_zero_rate(curve, t)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod curve_data_loader_tests {
        use super::*;

        #[test]
        fn test_parse_tenor_years() {
            assert!((parse_tenor_years("1Y") - 1.0).abs() < 1e-10);
            assert!((parse_tenor_years("6M") - 0.5).abs() < 1e-10);
            assert!((parse_tenor_years("3M") - 0.25).abs() < 1e-10);
            assert!((parse_tenor_years("1W") - 1.0 / 52.0).abs() < 1e-10);
            assert!((parse_tenor_years("30Y") - 30.0).abs() < 1e-10);
        }

        #[test]
        fn test_loader_available_indices() {
            let loader = CurveDataLoader::default_path();
            // Skip if data directory doesn't exist (CI environment)
            if !loader.base_path.exists() {
                return;
            }

            let indices = loader.available_indices();

            // Should find the files we created
            assert!(indices.contains(&"usd-sofr".to_string()));
            assert!(indices.contains(&"eur-estr".to_string()));
            assert!(indices.contains(&"jpy-tona".to_string()));
        }

        #[test]
        fn test_loader_load_instruments() {
            let loader = CurveDataLoader::default_path();
            // Skip if data directory doesn't exist (CI environment)
            if !loader.base_path.exists() {
                return;
            }

            let result = loader.load_instruments("usd-sofr");

            assert!(result.is_ok());
            let file = result.unwrap();
            assert_eq!(file.index, "usd-sofr");
            assert_eq!(file.currency, "USD");
            assert!(!file.instruments.is_empty());
        }

        #[test]
        fn test_loader_not_found() {
            let loader = CurveDataLoader::default_path();
            let result = loader.load_instruments("nonexistent-index");

            assert!(result.is_err());
            match result.unwrap_err() {
                CurveDataError::IndexNotFound(idx) => assert_eq!(idx, "nonexistent-index"),
                _ => panic!("Expected IndexNotFound error"),
            }
        }

        #[test]
        fn test_loader_is_supported() {
            let loader = CurveDataLoader::default_path();
            // Skip if data directory doesn't exist (CI environment)
            if !loader.base_path.exists() {
                return;
            }

            assert!(loader.is_supported("usd-sofr"));
            assert!(loader.is_supported("eur-estr"));
            assert!(loader.is_supported("jpy-tona"));
            assert!(!loader.is_supported("nonexistent"));
        }
    }

    mod interpolation_tests {
        use super::*;
        use crate::web::pricer_types::{CachedCurve, ParRateInput};

        fn sample_curve() -> CachedCurve {
            let pillars = vec![0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
            let discount_factors = vec![0.9875, 0.9750, 0.9500, 0.9000, 0.8000, 0.6500];
            let zero_rates: Vec<f64> = pillars
                .iter()
                .zip(discount_factors.iter())
                .map(|(t, df): (&f64, &f64)| -df.ln() / t)
                .collect();
            let par_rates = pillars
                .iter()
                .map(|t| ParRateInput {
                    tenor: format!("{}Y", t),
                    rate: 0.05,
                })
                .collect();

            CachedCurve::new(pillars, discount_factors, zero_rates, par_rates)
        }

        #[test]
        fn test_interpolate_df_at_zero() {
            let curve = sample_curve();
            let df = interpolate_df(&curve, 0.0);
            assert!((df - 1.0).abs() < 1e-10);
        }

        #[test]
        fn test_interpolate_df_at_pillar() {
            let curve = sample_curve();
            let df = interpolate_df(&curve, 1.0);
            assert!((df - 0.9500).abs() < 1e-10);
        }

        #[test]
        fn test_interpolate_df_between_pillars() {
            let curve = sample_curve();
            let df = interpolate_df(&curve, 0.75);
            // Should be between 0.9750 and 0.9500
            assert!(df > 0.9500 && df < 0.9750);
        }

        #[test]
        fn test_interpolate_zero_rate() {
            let curve = sample_curve();
            let zr = interpolate_zero_rate(&curve, 1.0);
            // From DF = 0.95, zero rate should be -ln(0.95)/1 ≈ 0.0513
            assert!((zr - 0.0513).abs() < 0.001);
        }

        #[test]
        fn test_interpolate_forward_rate() {
            let curve = sample_curve();
            let fr = interpolate_forward_rate(&curve, 1.0, 0.25);
            // Forward rate should be positive
            assert!(fr > 0.0);
        }
    }
}
