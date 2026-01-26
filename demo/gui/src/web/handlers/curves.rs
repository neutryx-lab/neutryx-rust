//! Curve Builder API handlers and types.

use std::{path::PathBuf, sync::Arc, time::Instant};

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::web::{
    error::{ApiError, ApiResult},
    AppState,
};

// =============================================================================
// Type Definitions
// =============================================================================

/// Interpolation method for yield curve construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationMethod {
    /// Linear interpolation on zero rates.
    LinearOnZeroRate,
    /// Linear interpolation on log discount factors (recommended).
    #[default]
    LinearOnLogDf,
    /// Cubic spline interpolation on zero rates.
    CubicSplineOnZeroRate,
    /// Monotonic cubic interpolation on zero rates.
    MonotonicOnZeroRate,
}

impl InterpolationMethod {
    /// Get the display name for this interpolation method.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::LinearOnZeroRate => "Linear on Zero Rate",
            Self::LinearOnLogDf => "Linear on ln(DF)",
            Self::CubicSplineOnZeroRate => "Cubic Spline on Zero Rate",
            Self::MonotonicOnZeroRate => "Monotonic on Zero Rate",
        }
    }

    /// Get a description of this interpolation method.
    pub fn description(&self) -> &'static str {
        match self {
            Self::LinearOnZeroRate => "Linear interpolation on continuously compounded zero rate",
            Self::LinearOnLogDf => {
                "Linear interpolation on ln(DF), constant forward between pillars"
            }
            Self::CubicSplineOnZeroRate => {
                "Cubic spline on zero rate with continuous 1st/2nd derivatives"
            }
            Self::MonotonicOnZeroRate => "Monotone-preserving cubic interpolation on zero rate",
        }
    }

    /// Check if this method is recommended.
    pub fn is_recommended(&self) -> bool { matches!(self, Self::LinearOnLogDf) }
}

/// Bootstrap method for curve construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapMethod {
    /// Sequential bootstrapping (instrument by instrument)
    #[default]
    Sequential,
    /// Global optimization (all instruments simultaneously)
    Global,
}

impl BootstrapMethod {
    /// Get the display name for this bootstrap method.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Sequential => "Sequential",
            Self::Global => "Global (Coming Soon)",
        }
    }

    /// Get a description of this bootstrap method.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Sequential => "Bootstraps instruments one by one in tenor order",
            Self::Global => "Solves all instruments simultaneously using global optimization",
        }
    }

    /// Check if this method is currently available.
    pub fn is_enabled(&self) -> bool { matches!(self, Self::Sequential) }
}

/// Parameter type for curve output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    /// Discount factor
    #[default]
    DiscountFactor,
    /// Continuously compounded zero rate
    ZeroRate,
    /// Instantaneous forward rate
    ForwardRate,
}

impl ParameterType {
    /// Get the display name for this parameter type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::DiscountFactor => "Discount Factor",
            Self::ZeroRate => "Zero Rate",
            Self::ForwardRate => "Forward Rate",
        }
    }
}

/// Build status for curve construction result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildStatus {
    /// Curve built successfully
    Success,
    /// Curve built with some warnings
    PartialSuccess,
    /// Curve construction failed
    Failed,
}

/// Instrument definition from JSON file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InstrumentFileEntry {
    /// Instrument type (deposit, ois, swap)
    pub r#type: String,
    /// Tenor string (e.g., "1M", "5Y")
    pub tenor: String,
    /// Tenor in years
    pub tenor_years: f64,
    /// Par rate
    pub rate: f64,
    /// Payment frequency
    pub frequency: String,
}

/// Complete instrument file structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InstrumentFile {
    /// Index identifier (e.g., "usd-sofr")
    pub index: String,
    /// Currency code (e.g., "USD")
    pub currency: String,
    /// Reference date for the curve
    pub reference_date: String,
    /// List of instruments
    pub instruments: Vec<InstrumentFileEntry>,
}

/// Response for `GET /api/curves/instruments/{index}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentListResponse {
    /// Index identifier
    pub index: String,
    /// Currency code
    pub currency: String,
    /// List of instruments
    pub instruments: Vec<InstrumentInfo>,
}

/// Instrument information for API responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentInfo {
    /// Instrument type
    pub instrument_type: String,
    /// Tenor string
    pub tenor: String,
    /// Tenor in years
    pub tenor_years: f64,
    /// Par rate
    pub rate: f64,
    /// Payment frequency
    pub frequency: String,
}

impl From<InstrumentFileEntry> for InstrumentInfo {
    fn from(entry: InstrumentFileEntry) -> Self {
        Self {
            instrument_type: entry.r#type,
            tenor: entry.tenor,
            tenor_years: entry.tenor_years,
            rate: entry.rate,
            frequency: entry.frequency,
        }
    }
}

/// Request for `POST /api/curves/build`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurveBuildRequest {
    /// Index identifier
    pub index: String,
    /// Instruments with rates
    pub instruments: Vec<InstrumentInput>,
    /// Interpolation method
    #[serde(alias = "interpolationMethod")]
    pub interpolation: InterpolationMethod,
    /// Bootstrap method
    #[serde(default)]
    pub bootstrap_method: BootstrapMethod,
    /// Tolerance for convergence
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    /// Maximum iterations for solver
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}

fn default_tolerance() -> f64 { 1e-10 }
fn default_max_iterations() -> usize { 100 }

/// Instrument input from client.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentInput {
    /// Instrument type
    pub instrument_type: String,
    /// Tenor string
    pub tenor: String,
    /// Par rate
    pub rate: f64,
}

/// Response for `POST /api/curves/build`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurveBuildResponse {
    /// UUID of the constructed curve
    pub curve_id: String,
    /// Build status
    pub status: BuildStatus,
    /// Index name
    pub index: String,
    /// Interpolation method used
    pub interpolation_method: String,
    /// Parameter points for visualisation
    pub parameters: Vec<CurveParameter>,
    /// Pillar points (years)
    pub pillars: Vec<f64>,
    /// Discount factors at pillar points
    pub discount_factors: Vec<f64>,
    /// Zero rates at pillar points
    pub zero_rates: Vec<f64>,
    /// Build time in milliseconds
    pub build_time_ms: f64,
    /// Number of instruments used
    pub instrument_count: usize,
}

/// A single curve parameter point for visualisation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurveParameter {
    /// Tenor in years
    pub tenor_years: f64,
    /// Discount factor at this tenor
    pub discount_factor: f64,
    /// Zero rate at this tenor
    pub zero_rate: f64,
    /// Forward rate at this tenor (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward_rate: Option<f64>,
}

/// Query parameters for `GET /api/curves/{curveId}/parameters`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ParameterQuery {
    /// Parameter type
    pub r#type: ParameterType,
    /// Start year (default: 0)
    #[serde(default)]
    pub start_year: f64,
    /// End year (default: 30)
    #[serde(default = "default_end_year")]
    pub end_year: f64,
    /// Grid interval in years (default: 0.25)
    #[serde(default = "default_grid_interval")]
    pub grid_interval: f64,
}

fn default_end_year() -> f64 { 30.0 }
fn default_grid_interval() -> f64 { 0.25 }

/// Response for `GET /api/curves/{curveId}/parameters`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterResponse {
    /// Curve identifier
    pub curve_id: String,
    /// Parameter type
    pub parameter_type: ParameterType,
    /// Data points
    pub data: Vec<ParameterPoint>,
}

/// Single point on the parameter curve.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterPoint {
    /// Tenor in years
    pub tenor: f64,
    /// Parameter value
    pub value: f64,
}

/// Response for `GET /api/curves/builders`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderListResponse {
    /// Available interpolation methods
    pub interpolation_methods: Vec<InterpolationMethodInfo>,
    /// Available bootstrap methods
    pub bootstrap_methods: Vec<BootstrapMethodInfo>,
}

/// Information about an interpolation method.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterpolationMethodInfo {
    /// Method identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Whether this method is recommended
    pub recommended: bool,
}

/// Information about a bootstrap method.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapMethodInfo {
    /// Method identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Whether this method is currently enabled
    pub enabled: bool,
}

impl BuilderListResponse {
    /// Create a new builder list response with all available methods.
    pub fn new() -> Self {
        Self {
            interpolation_methods: vec![
                InterpolationMethodInfo {
                    id: "linear_on_zero_rate".to_string(),
                    name: InterpolationMethod::LinearOnZeroRate
                        .display_name()
                        .to_string(),
                    description: InterpolationMethod::LinearOnZeroRate
                        .description()
                        .to_string(),
                    recommended: InterpolationMethod::LinearOnZeroRate.is_recommended(),
                },
                InterpolationMethodInfo {
                    id: "linear_on_log_df".to_string(),
                    name: InterpolationMethod::LinearOnLogDf
                        .display_name()
                        .to_string(),
                    description: InterpolationMethod::LinearOnLogDf.description().to_string(),
                    recommended: InterpolationMethod::LinearOnLogDf.is_recommended(),
                },
                InterpolationMethodInfo {
                    id: "cubic_spline_on_zero_rate".to_string(),
                    name: InterpolationMethod::CubicSplineOnZeroRate
                        .display_name()
                        .to_string(),
                    description: InterpolationMethod::CubicSplineOnZeroRate
                        .description()
                        .to_string(),
                    recommended: InterpolationMethod::CubicSplineOnZeroRate.is_recommended(),
                },
                InterpolationMethodInfo {
                    id: "monotonic_on_zero_rate".to_string(),
                    name: InterpolationMethod::MonotonicOnZeroRate
                        .display_name()
                        .to_string(),
                    description: InterpolationMethod::MonotonicOnZeroRate
                        .description()
                        .to_string(),
                    recommended: InterpolationMethod::MonotonicOnZeroRate.is_recommended(),
                },
            ],
            bootstrap_methods: vec![
                BootstrapMethodInfo {
                    id: "sequential".to_string(),
                    name: BootstrapMethod::Sequential.display_name().to_string(),
                    description: BootstrapMethod::Sequential.description().to_string(),
                    enabled: BootstrapMethod::Sequential.is_enabled(),
                },
                BootstrapMethodInfo {
                    id: "global".to_string(),
                    name: BootstrapMethod::Global.display_name().to_string(),
                    description: BootstrapMethod::Global.description().to_string(),
                    enabled: BootstrapMethod::Global.is_enabled(),
                },
            ],
        }
    }
}

impl Default for BuilderListResponse {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// CurveDataLoader
// =============================================================================

/// Loader for index-based instrument data files.
pub struct CurveDataLoader {
    base_path: PathBuf,
}

impl CurveDataLoader {
    /// Create a new CurveDataLoader with the specified base path.
    pub fn new(base_path: PathBuf) -> Self { Self { base_path } }

    /// Create a CurveDataLoader with the default path.
    pub fn default_path() -> Self { Self::new(PathBuf::from("demo/data/input/rates")) }

    /// Get the list of available indices.
    pub fn available_indices(&self) -> Vec<String> {
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
        indices.sort();
        indices
    }

    /// Load instruments for the specified index.
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
// API Handlers
// =============================================================================

/// Handler for `GET /api/curves/instruments/{index}`.
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
pub async fn get_builders() -> ApiResult<BuilderListResponse> {
    Ok(Json(BuilderListResponse::new()))
}

/// Handler for `POST /api/curves/build`.
pub async fn build_curve(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CurveBuildRequest>,
) -> ApiResult<CurveBuildResponse> {
    let start = Instant::now();

    if request.instruments.is_empty() {
        return Err(ApiError::validation(
            "At least one instrument is required",
            "instruments",
        ));
    }

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

    let mut pillars: Vec<f64> = Vec::new();
    let mut discount_factors: Vec<f64> = Vec::new();
    let mut zero_rates: Vec<f64> = Vec::new();

    let mut sorted_instruments = request.instruments.clone();
    sorted_instruments.sort_by(|a, b| {
        let ta = parse_tenor_years(&a.tenor);
        let tb = parse_tenor_years(&b.tenor);
        ta.partial_cmp(&tb).unwrap_or(std::cmp::Ordering::Equal)
    });

    for inst in &sorted_instruments {
        let tenor_years = parse_tenor_years(&inst.tenor);
        let rate = inst.rate;
        let df = 1.0 / (1.0 + rate * tenor_years);
        let zero_rate = if tenor_years > 0.0 {
            -df.ln() / tenor_years
        } else {
            rate
        };

        pillars.push(tenor_years);
        discount_factors.push(df);
        zero_rates.push(zero_rate);
    }

    let curve_id = Uuid::new_v4();

    use super::types::{CachedCurve, ParRateInput};

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

    let parameters: Vec<CurveParameter> = pillars
        .iter()
        .enumerate()
        .map(|(i, &tenor)| {
            let df = discount_factors[i];
            let zr = zero_rates[i];

            let forward_rate = if i > 0 {
                let prev_tenor = pillars[i - 1];
                let prev_df = discount_factors[i - 1];
                let dt = tenor - prev_tenor;
                if dt > 0.0 {
                    Some((prev_df.ln() - df.ln()) / dt)
                } else {
                    Some(zr)
                }
            } else {
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
pub async fn get_parameters(
    State(state): State<Arc<AppState>>,
    Path(curve_id): Path<String>,
    Query(query): Query<ParameterQuery>,
) -> ApiResult<ParameterResponse> {
    let uuid = Uuid::parse_str(&curve_id)
        .map_err(|_| ApiError::validation("Invalid curve ID format", "curveId"))?;

    let cached_curve = state
        .curve_cache
        .get(&uuid)
        .ok_or_else(|| ApiError::not_found("Curve", &curve_id))?;

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

    Ok(Json(ParameterResponse {
        curve_id,
        parameter_type: query.r#type,
        data,
    }))
}

/// Handler for `GET /api/curves/indices`.
pub async fn get_indices() -> ApiResult<Vec<String>> {
    let loader = CurveDataLoader::default_path();
    let mut indices = loader.available_indices();
    indices.retain(|idx| !idx.contains("market_quotes"));
    Ok(Json(indices))
}

/// Handler for `GET /api/curves/central-bank-meetings`.
/// Transforms the events array into a currency-keyed meetings object for
/// frontend consumption.
pub async fn get_central_bank_meetings() -> ApiResult<serde_json::Value> {
    let file_path = std::path::PathBuf::from("demo/data/input/events/central_bank_meetings.json");

    if !file_path.exists() {
        return Ok(Json(serde_json::json!({ "meetings": {} })));
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| ApiError::internal(format!("Failed to read central bank meetings: {}", e)))?;

    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| ApiError::internal(format!("Failed to parse central bank meetings: {}", e)))?;

    // Transform events array into currency-keyed meetings object
    // Expected input: { "events": [{ "currency": "USD", "date": "2025-01-29",
    // "centralBank": {...} }, ...] } Expected output: { "meetings": { "USD": {
    // "centralBank": "FED", "dates": ["2025-01-29", ...] }, ... } }
    let mut meetings: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();

    if let Some(events) = data.get("events").and_then(|e| e.as_array()) {
        for event in events {
            let currency = event
                .get("currency")
                .and_then(|c| c.as_str())
                .unwrap_or("UNKNOWN");
            let date = event.get("date").and_then(|d| d.as_str()).unwrap_or("");
            let central_bank_code = event
                .get("centralBank")
                .and_then(|cb| cb.get("code"))
                .and_then(|c| c.as_str())
                .unwrap_or("");

            let entry = meetings.entry(currency.to_string()).or_insert_with(
                || serde_json::json!({ "centralBank": central_bank_code, "dates": [] }),
            );

            if let Some(dates) = entry.get_mut("dates").and_then(|d| d.as_array_mut()) {
                if !date.is_empty() {
                    dates.push(serde_json::Value::String(date.to_string()));
                }
            }
        }
    }

    Ok(Json(serde_json::json!({ "meetings": meetings })))
}

// =============================================================================
// Helper Functions
// =============================================================================

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
        tenor.parse::<f64>().unwrap_or(0.0)
    }
}

fn interpolate_df(curve: &super::types::CachedCurve, t: f64) -> f64 {
    if t <= 0.0 {
        return 1.0;
    }

    let pillars = &curve.pillars;
    let dfs = &curve.discount_factors;

    if pillars.is_empty() {
        return 1.0;
    }

    if t <= pillars[0] {
        let log_df = dfs[0].ln() * t / pillars[0];
        return log_df.exp();
    }

    if t >= *pillars.last().unwrap() {
        let n = pillars.len();
        let log_df_last = dfs[n - 1].ln();
        let log_df_prev = dfs[n - 2].ln();
        let slope = (log_df_last - log_df_prev) / (pillars[n - 1] - pillars[n - 2]);
        let log_df = log_df_last + slope * (t - pillars[n - 1]);
        return log_df.exp();
    }

    for i in 1..pillars.len() {
        if t <= pillars[i] {
            let t0 = pillars[i - 1];
            let t1 = pillars[i];
            let log_df0 = dfs[i - 1].ln();
            let log_df1 = dfs[i].ln();
            let w = (t - t0) / (t1 - t0);
            let log_df = log_df0 + w * (log_df1 - log_df0);
            return log_df.exp();
        }
    }

    1.0
}

fn interpolate_zero_rate(curve: &super::types::CachedCurve, t: f64) -> f64 {
    if t <= 0.0 {
        if !curve.zero_rates.is_empty() {
            return curve.zero_rates[0];
        }
        return 0.0;
    }

    let df = interpolate_df(curve, t);
    -df.ln() / t
}

fn interpolate_forward_rate(curve: &super::types::CachedCurve, t: f64, interval: f64) -> f64 {
    let df_t = interpolate_df(curve, t);
    let df_t_dt = interpolate_df(curve, t + interval);

    if interval > 0.0 && df_t_dt > 0.0 {
        (df_t / df_t_dt - 1.0) / interval
    } else {
        interpolate_zero_rate(curve, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tenor_years() {
        assert!((parse_tenor_years("1Y") - 1.0).abs() < 1e-10);
        assert!((parse_tenor_years("6M") - 0.5).abs() < 1e-10);
        assert!((parse_tenor_years("3M") - 0.25).abs() < 1e-10);
        assert!((parse_tenor_years("1W") - 1.0 / 52.0).abs() < 1e-10);
    }

    #[test]
    fn test_loader_not_found() {
        let loader = CurveDataLoader::default_path();
        let result = loader.load_instruments("nonexistent-index");
        assert!(result.is_err());
    }
}
