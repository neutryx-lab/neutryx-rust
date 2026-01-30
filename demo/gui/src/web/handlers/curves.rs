//! Curve Builder API handlers and types.

use std::{path::PathBuf, sync::Arc, time::Instant};

use adapter_loader::{parse_instruments, validate_rates, InstrumentParseError, InstrumentSpec};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Local, NaiveDate};
// Conditional imports for global bootstrap with jump calibration
#[cfg(feature = "global-bootstrap")]
use pricer_models::builder::{GlobalBootstrapConfig, GlobalBootstrapper, JumpPillar};
use pricer_models::{
    builder::{
        BootstrapConfig, BootstrapError, CurveBootstrapper,
        InterpolationMethod as BuilderInterpolation,
    },
    market::curves::YieldCurve,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::web::{
    error::{ApiError, ApiResult},
    market_data::get_config,
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

    /// Convert to pricer_models builder interpolation method.
    fn to_builder_interpolation(self) -> BuilderInterpolation {
        match self {
            Self::LinearOnZeroRate => BuilderInterpolation::Linear,
            Self::LinearOnLogDf => BuilderInterpolation::LogLinear,
            Self::CubicSplineOnZeroRate => BuilderInterpolation::CubicSpline,
            Self::MonotonicOnZeroRate => BuilderInterpolation::LogLinear, // Fallback
        }
    }
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

// =============================================================================
// CB Meeting Jump Types (Task 9.1, 9.2, 9.3)
// =============================================================================

/// CB meeting event input.
///
/// Represents a central bank meeting date with expected jump size.
/// Used in curve calibration requests to enable jump-aware bootstrapping.
///
/// # Requirements Coverage
///
/// - Requirement 1.5: JSON format for cb_events
/// - Requirement 4.2: Event parsing with date and expected jump
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CbEventInput {
    /// Meeting date in ISO format (YYYY-MM-DD).
    pub date: String,
    /// Expected jump in basis points (-100 to +100).
    pub expected_jump_bps: f64,
    /// Central bank code (optional, for display).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub central_bank: Option<String>,
}

impl CbEventInput {
    /// Validate the CB event input.
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, `Err(String)` with validation error message.
    pub fn validate(&self) -> Result<(), String> {
        // Validate date format (ISO 8601)
        if NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").is_err() {
            return Err(format!(
                "Invalid date format '{}': expected YYYY-MM-DD",
                self.date
            ));
        }

        // Validate jump range (±100bps)
        if self.expected_jump_bps < -100.0 || self.expected_jump_bps > 100.0 {
            return Err(format!(
                "Expected jump {} bps is out of range (-100 to +100)",
                self.expected_jump_bps
            ));
        }

        Ok(())
    }
}

/// Realised jump information in calibration response.
///
/// Contains the details of a calibrated jump at a CB meeting date.
///
/// # Requirements Coverage
///
/// - Requirement 4.3: Response includes realised jump values
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RealizedJumpInfo {
    /// Meeting date in ISO format.
    pub date: String,
    /// Central bank code (if provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub central_bank: Option<String>,
    /// Expected jump in basis points (input).
    pub expected_bps: f64,
    /// Realised jump in basis points (calibrated).
    pub realized_bps: f64,
    /// Time to jump in years.
    pub time_years: f64,
}

impl BootstrapMethod {
    /// Get the display name for this bootstrap method.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Sequential => "Sequential",
            Self::Global => "Global",
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
    pub fn is_enabled(&self) -> bool { matches!(self, Self::Sequential | Self::Global) }
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
///
/// # Requirements Coverage
///
/// - Requirement 4.1: cb_events optional parameter
/// - Requirement 7.2: New parameters are optional for backward compatibility
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
    /// CB meeting events with expected jumps (Task 9.1).
    ///
    /// When provided, the bootstrapper will include jump parameters
    /// at these dates during calibration.
    #[serde(default)]
    pub cb_events: Option<Vec<CbEventInput>>,
    /// Enable jump calibration (Task 9.1).
    ///
    /// When true and cb_events are provided, the global bootstrapper
    /// will calibrate jump parameters at CB meeting dates.
    #[serde(default)]
    pub enable_jumps: bool,
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

impl InstrumentInput {
    /// Convert to an `InstrumentSpec` for use with `parse_instruments`.
    fn to_spec(&self) -> InstrumentSpec {
        InstrumentSpec::new(&self.instrument_type, &self.tenor, self.rate)
    }
}

/// Response for `POST /api/curves/build`.
///
/// # Requirements Coverage
///
/// - Requirement 4.3: Response includes realised jump values
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
    /// Realised jumps after calibration (Task 9.3).
    ///
    /// Present only when jump calibration was enabled and completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realized_jumps: Option<Vec<RealizedJumpInfo>>,
    /// Whether jump calibration fallback was used (Task 9.3).
    ///
    /// True if jump calibration failed and the system fell back
    /// to non-jump calibration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jump_fallback_used: Option<bool>,
    /// Jump-related warnings (Task 9.3).
    ///
    /// Warnings generated during jump calibration, such as
    /// events outside instrument tenor range.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jump_warnings: Vec<String>,
    /// Jacobian inverse matrix from Global Bootstrap calibration.
    ///
    /// Present only when Global Bootstrap method is used and
    /// `store_jacobian_inverse` is enabled. Used for AAD sensitivity
    /// computation via implicit function theorem.
    ///
    /// Format: Row-major 2D array where J⁻¹[i][j] = ∂log(DF_i)/∂(market_rate_j)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jacobian_inverse: Option<JacobianInverseData>,
}

/// Jacobian inverse matrix data for API response.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JacobianInverseData {
    /// Number of rows (pillars).
    pub rows: usize,
    /// Number of columns (instruments).
    pub cols: usize,
    /// Matrix data in row-major order (rows × cols).
    pub data: Vec<Vec<f64>>,
    /// Row labels (pillar tenors in years).
    pub row_labels: Vec<f64>,
    /// Column labels (instrument tenors in years).
    pub col_labels: Vec<f64>,
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
///
/// # Task Coverage
///
/// - Task 10.1: CB Meeting parameter parsing and validation
/// - Task 10.2: JumpPillar conversion and range filtering
/// - Task 10.3: GlobalBootstrapper call integration
pub async fn build_curve(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CurveBuildRequest>,
) -> ApiResult<CurveBuildResponse> {
    let start = Instant::now();

    // Convert to InstrumentSpec for parsing
    let specs: Vec<InstrumentSpec> = request.instruments.iter().map(|i| i.to_spec()).collect();

    // Validate rates using adapter_loader
    validate_rates(&specs, -0.10, 0.50).map_err(|e| convert_parse_error(&e))?;

    // Filter duplicate maturities - when multiple instruments have the same
    // effective maturity (e.g., FRA 3x6 and Future 6M both at 0.5Y), keep only
    // the highest priority instrument
    let filtered_specs = filter_duplicate_maturities(specs);

    // Parse instruments using adapter_loader (handles sorting by tenor)
    let market_instruments =
        parse_instruments(&filtered_specs).map_err(|e| convert_parse_error(&e))?;

    // Convert filtered_specs back to InstrumentInput for par rates
    // This ensures par_rates match the filtered market_instruments
    let sorted_instruments: Vec<InstrumentInput> = filtered_specs
        .iter()
        .map(|spec| InstrumentInput {
            instrument_type: spec.instrument_type.clone(),
            tenor: spec.tenor.clone(),
            rate: spec.rate,
        })
        .collect();

    use super::types::{CachedCurve, ParRateInput};

    // Variables to store jump calibration results
    let mut realized_jumps: Option<Vec<RealizedJumpInfo>> = None;
    let mut jump_fallback_used: Option<bool> = None;
    let mut jump_warnings: Vec<String> = Vec::new();
    let mut jacobian_inverse_data: Option<JacobianInverseData> = None;

    // Build curve - with jump calibration when global-bootstrap feature is enabled
    #[cfg(feature = "global-bootstrap")]
    let curve = {
        // Get max tenor for filtering CB events (use filtered specs)
        let max_tenor = filtered_specs
            .iter()
            .filter_map(|s| s.tenor_years().ok())
            .fold(0.0_f64, |a, b| a.max(b));

        // Task 10.1: Parse and validate CB events
        let jump_pillars = if request.enable_jumps {
            parse_and_validate_cb_events(request.cb_events.as_ref(), max_tenor, &mut jump_warnings)?
        } else {
            Vec::new()
        };

        // Determine if we should use global bootstrap with jumps
        let use_jump_calibration = request.enable_jumps
            && !jump_pillars.is_empty()
            && request.bootstrap_method == BootstrapMethod::Global;

        if use_jump_calibration {
            // Use GlobalBootstrapper with jump calibration
            let global_config = GlobalBootstrapConfig::default()
                .with_tolerance(request.tolerance)
                .with_max_iterations(request.max_iterations)
                .with_jacobian_inverse(true)
                .with_interpolation(
                    pricer_models::market::curves::BootstrapInterpolation::LogLinear,
                );

            let global_bootstrapper = GlobalBootstrapper::new(global_config);

            match global_bootstrapper
                .calibrate_with_jumps(&market_instruments, jump_pillars.clone())
            {
                Ok(result) => {
                    jump_fallback_used =
                        Some(result.realised_jumps.as_ref().is_some_and(|j| j.is_empty()));

                    // Extract realised jumps for response
                    if let Some(ref calibrated_jumps) = result.realised_jumps {
                        let cb_events_ref = request.cb_events.as_ref();
                        let empty_vec = Vec::new();
                        let events = cb_events_ref.unwrap_or(&empty_vec);
                        realized_jumps = Some(
                            calibrated_jumps
                                .iter()
                                .zip(events.iter())
                                .map(|(jp, input)| RealizedJumpInfo {
                                    date: input.date.clone(),
                                    central_bank: input.central_bank.clone(),
                                    expected_bps: input.expected_jump_bps,
                                    realized_bps: jp.realised_jump_bps().unwrap_or(0.0),
                                    time_years: jp.time,
                                })
                                .collect(),
                        );
                    }

                    // Extract Jacobian inverse for AAD sensitivity display
                    if let Some(ref j_inv) = result.jacobian_inverse {
                        let rows = j_inv.nrows();
                        let cols = j_inv.ncols();
                        let data: Vec<Vec<f64>> = (0..rows)
                            .map(|i| (0..cols).map(|j| j_inv[(i, j)]).collect())
                            .collect();
                        let row_labels: Vec<f64> = result.pillars.clone();
                        // Use filtered specs for column labels (instrument tenors)
                        let col_labels: Vec<f64> = filtered_specs
                            .iter()
                            .filter_map(|s| s.tenor_years().ok())
                            .collect();
                        jacobian_inverse_data = Some(JacobianInverseData {
                            rows,
                            cols,
                            data,
                            row_labels,
                            col_labels,
                        });
                    }

                    result.curve
                }
                Err(e) => {
                    // Fallback to sequential bootstrap
                    jump_warnings.push(format!(
                        "Jump calibration failed: {}. Falling back to sequential bootstrap.",
                        e
                    ));
                    jump_fallback_used = Some(true);

                    let config = BootstrapConfig::new(request.tolerance, request.max_iterations)
                        .with_interpolation(request.interpolation.to_builder_interpolation());
                    let bootstrapper = CurveBootstrapper::with_config(config);
                    bootstrapper
                        .bootstrap_to_curve(&market_instruments)
                        .map_err(|e| convert_bootstrap_error(&e))?
                }
            }
        } else if request.bootstrap_method == BootstrapMethod::Global {
            // Global Bootstrap without jumps - still return Jacobian inverse
            let global_config = GlobalBootstrapConfig::default()
                .with_tolerance(request.tolerance)
                .with_max_iterations(request.max_iterations)
                .with_jacobian_inverse(true)
                .with_interpolation(
                    pricer_models::market::curves::BootstrapInterpolation::LogLinear,
                );

            let global_bootstrapper = GlobalBootstrapper::new(global_config);

            match global_bootstrapper.calibrate(&market_instruments) {
                Ok(result) => {
                    // Extract Jacobian inverse for AAD sensitivity display
                    if let Some(ref j_inv) = result.jacobian_inverse {
                        let rows = j_inv.nrows();
                        let cols = j_inv.ncols();
                        let data: Vec<Vec<f64>> = (0..rows)
                            .map(|i| (0..cols).map(|j| j_inv[(i, j)]).collect())
                            .collect();
                        let row_labels: Vec<f64> = result.pillars.clone();
                        let col_labels: Vec<f64> = filtered_specs
                            .iter()
                            .filter_map(|s| s.tenor_years().ok())
                            .collect();
                        jacobian_inverse_data = Some(JacobianInverseData {
                            rows,
                            cols,
                            data,
                            row_labels,
                            col_labels,
                        });
                    }

                    result.curve
                }
                Err(e) => {
                    // Fallback to sequential bootstrap
                    jump_warnings.push(format!(
                        "Global bootstrap failed: {}. Falling back to sequential bootstrap.",
                        e
                    ));

                    let config = BootstrapConfig::new(request.tolerance, request.max_iterations)
                        .with_interpolation(request.interpolation.to_builder_interpolation());
                    let bootstrapper = CurveBootstrapper::with_config(config);
                    bootstrapper
                        .bootstrap_to_curve(&market_instruments)
                        .map_err(|e| convert_bootstrap_error(&e))?
                }
            }
        } else {
            // Standard sequential bootstrapping with Jacobian inverse
            let config = BootstrapConfig::new(request.tolerance, request.max_iterations)
                .with_interpolation(request.interpolation.to_builder_interpolation());

            let bootstrapper = CurveBootstrapper::with_config(config);

            // Try to get Jacobian inverse (may fail if matrix isn't lower triangular)
            match bootstrapper.bootstrap_with_jacobian(&market_instruments) {
                Ok((curve, Some(j_inv))) => {
                    let rows = j_inv.nrows();
                    let cols = j_inv.ncols();
                    let data: Vec<Vec<f64>> = (0..rows)
                        .map(|i| (0..cols).map(|j| j_inv[(i, j)]).collect())
                        .collect();
                    // For sequential bootstrap, pillars = instrument tenors
                    let row_labels: Vec<f64> = filtered_specs
                        .iter()
                        .filter_map(|s| s.tenor_years().ok())
                        .collect();
                    let col_labels = row_labels.clone();
                    jacobian_inverse_data = Some(JacobianInverseData {
                        rows,
                        cols,
                        data,
                        row_labels,
                        col_labels,
                    });
                    curve
                }
                Ok((curve, None)) => {
                    // Jacobian computation failed but curve is valid
                    curve
                }
                Err(_) => {
                    // Fall back to standard bootstrap without Jacobian
                    bootstrapper
                        .bootstrap_to_curve(&market_instruments)
                        .map_err(|e| convert_bootstrap_error(&e))?
                }
            }
        }
    };

    // Fallback when global-bootstrap feature is not enabled
    #[cfg(not(feature = "global-bootstrap"))]
    let curve = {
        // Standard sequential bootstrapping only
        let config = BootstrapConfig::new(request.tolerance, request.max_iterations)
            .with_interpolation(request.interpolation.to_builder_interpolation());

        let bootstrapper = CurveBootstrapper::with_config(config);
        bootstrapper
            .bootstrap_to_curve(&market_instruments)
            .map_err(|e| convert_bootstrap_error(&e))?
    };

    // Create par_rates for CachedCurve (needed for bump-and-revalue)
    let par_rates: Vec<ParRateInput> = sorted_instruments
        .iter()
        .map(|i| ParRateInput {
            tenor: i.tenor.clone(),
            rate: i.rate,
        })
        .collect();

    // Create CachedCurve with the BootstrappedCurve
    let cached_curve = CachedCurve::new(curve, par_rates);

    // Extract data for API response from the cached curve
    let pillars = cached_curve.pillars().to_vec();
    let discount_factors = cached_curve.discount_factors().to_vec();
    let zero_rates = cached_curve.zero_rates();

    // Cache the curve
    let curve_id = Uuid::new_v4();
    state.curve_cache.add(curve_id, cached_curve);

    let build_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Build response parameters for visualisation
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
        realized_jumps,
        jump_fallback_used,
        jump_warnings,
        jacobian_inverse: jacobian_inverse_data,
    };

    Ok(Json(response))
}

/// Parse and validate CB events, converting to JumpPillars.
///
/// # Task Coverage
///
/// - Task 10.1: Date format validation (ISO 8601), numeric validation
/// - Task 10.2: JumpPillar conversion, range filtering
///
/// # Arguments
///
/// * `cb_events` - Optional vector of CB event inputs
/// * `max_tenor` - Maximum instrument tenor for filtering
/// * `warnings` - Vector to collect warnings for out-of-range events
///
/// # Returns
///
/// Vector of `JumpPillar` for in-range events.
#[cfg(feature = "global-bootstrap")]
fn parse_and_validate_cb_events(
    cb_events: Option<&Vec<CbEventInput>>,
    max_tenor: f64,
    warnings: &mut Vec<String>,
) -> Result<Vec<JumpPillar<f64>>, ApiError> {
    let events = match cb_events {
        Some(events) if !events.is_empty() => events,
        _ => return Ok(Vec::new()),
    };

    // Use today as reference date for year fraction calculation
    let today = Local::now().naive_local().date();

    let mut jump_pillars = Vec::with_capacity(events.len());

    for event in events {
        // Task 10.1: Validate each event
        event
            .validate()
            .map_err(|msg| ApiError::validation(msg, "cb_events"))?;

        // Parse the event date
        let event_date = NaiveDate::parse_from_str(&event.date, "%Y-%m-%d").map_err(|e| {
            ApiError::validation(format!("Invalid date '{}': {}", event.date, e), "cb_events")
        })?;

        // Calculate time to event in years
        let days_to_event = (event_date - today).num_days();
        let time_years = days_to_event as f64 / 365.0;

        // Task 10.2: Filter out events outside instrument tenor range
        if time_years <= 0.0 {
            warnings.push(format!(
                "CB event on {} is in the past, ignoring",
                event.date
            ));
            continue;
        }

        if time_years > max_tenor {
            warnings.push(format!(
                "CB event on {} ({:.2}Y) exceeds max instrument tenor ({:.2}Y), ignoring",
                event.date, time_years, max_tenor
            ));
            continue;
        }

        // Create JumpPillar
        jump_pillars.push(JumpPillar::new(time_years, event.expected_jump_bps));
    }

    // Sort by time
    jump_pillars.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(jump_pillars)
}

/// Convert BootstrapError to ApiError.
fn convert_bootstrap_error(error: &BootstrapError) -> ApiError {
    match error {
        BootstrapError::ConvergenceFailure {
            maturity,
            residual,
            iterations,
        } => ApiError::calculation(format!(
            "Bootstrap convergence failure at maturity {:.4}Y: residual {:.2e} after {} iterations",
            maturity, residual, iterations
        )),
        BootstrapError::InsufficientData { required, provided } => ApiError::validation(
            format!(
                "Insufficient data: {} instruments required, {} provided",
                required, provided
            ),
            "instruments",
        ),
        BootstrapError::NegativeRate { maturity, rate } => ApiError::calculation(format!(
            "Negative rate {:.6} at maturity {:.4}Y",
            rate, maturity
        )),
        BootstrapError::ArbitrageDetected { maturity } => ApiError::calculation(format!(
            "Arbitrage detected at maturity {:.4}Y (non-monotonic curve)",
            maturity
        )),
        BootstrapError::DuplicateMaturity { maturity } => ApiError::validation(
            format!("Duplicate maturity: {:.4}Y", maturity),
            "instruments",
        ),
        BootstrapError::InvalidMaturity {
            maturity,
            max_maturity,
        } => ApiError::validation(
            format!(
                "Invalid maturity {:.4}Y (max: {:.4}Y)",
                maturity, max_maturity
            ),
            "instruments",
        ),
        BootstrapError::Solver(e) => ApiError::calculation(format!("Solver error: {}", e)),
        BootstrapError::MarketData(e) => ApiError::calculation(format!("Market data error: {}", e)),
        BootstrapError::InvalidInput(msg) => ApiError::validation(msg.clone(), "instruments"),
    }
}

/// Convert InstrumentParseError to ApiError.
fn convert_parse_error(error: &InstrumentParseError) -> ApiError {
    match error {
        InstrumentParseError::InvalidTenor { tenor, reason } => {
            ApiError::validation(format!("Invalid tenor '{}': {}", tenor, reason), "tenor")
        }
        InstrumentParseError::UnknownType { instrument_type } => ApiError::validation(
            format!("Unknown instrument type: {}", instrument_type),
            "instrumentType",
        ),
        InstrumentParseError::InvalidRate { rate, reason } => {
            ApiError::validation(format!("Invalid rate {}: {}", rate, reason), "rate")
        }
        InstrumentParseError::EmptyInstruments => {
            ApiError::validation("At least one instrument is required", "instruments")
        }
    }
}

/// Get the priority of an instrument type for duplicate maturity resolution.
///
/// Lower numbers have higher priority. When multiple instruments have the same
/// maturity, the one with the highest priority (lowest number) will be
/// selected.
///
/// Priority order (highest to lowest):
/// 1. deposit - Most direct short-term rate
/// 2. ois - Overnight index swap, highly liquid
/// 3. swap/irs - Standard benchmark instruments
/// 4. fra - Forward rate agreements
/// 5. future - Futures (may require convexity adjustment)
fn instrument_type_priority(instrument_type: &str) -> u8 {
    match instrument_type.to_lowercase().as_str() {
        "deposit" | "depo" => 1,
        "ois" => 2,
        "swap" | "irs" => 3,
        "fra" => 4,
        "future" | "futures" => 5,
        _ => 6,
    }
}

/// Filter duplicate maturities from instruments, keeping the highest priority
/// instrument for each maturity.
///
/// This function resolves conflicts when multiple instruments have the same
/// effective maturity (e.g., FRA 3x6 and Future 6M both mature at 0.5Y).
///
/// # Arguments
///
/// * `specs` - Vector of instrument specifications
///
/// # Returns
///
/// A filtered vector with at most one instrument per maturity, selected by
/// priority.
fn filter_duplicate_maturities(specs: Vec<InstrumentSpec>) -> Vec<InstrumentSpec> {
    use std::collections::HashMap;

    // Group instruments by maturity (rounded to avoid floating point issues)
    let mut maturity_groups: HashMap<i64, Vec<InstrumentSpec>> = HashMap::new();

    for spec in specs {
        if let Ok(tenor) = spec.tenor_years() {
            // Round to 6 decimal places for grouping (1e-6 year ≈ 31 seconds)
            let key = (tenor * 1_000_000.0).round() as i64;
            maturity_groups.entry(key).or_default().push(spec);
        }
    }

    // Select the highest priority instrument from each group
    let mut result: Vec<InstrumentSpec> = Vec::with_capacity(maturity_groups.len());

    for (_key, mut group) in maturity_groups {
        // Sort by priority (ascending = higher priority first)
        group.sort_by_key(|s| instrument_type_priority(&s.instrument_type));
        // Take the first (highest priority) instrument
        if let Some(best) = group.into_iter().next() {
            result.push(best);
        }
    }

    // Sort by maturity for consistent output
    result.sort_by(|a, b| {
        let ta = a.tenor_years().unwrap_or(0.0);
        let tb = b.tenor_years().unwrap_or(0.0);
        ta.partial_cmp(&tb).unwrap_or(std::cmp::Ordering::Equal)
    });

    result
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

    // Use the YieldCurve trait methods from the underlying BootstrappedCurve
    let curve = cached_curve.curve();

    let mut data = Vec::new();
    let mut t = query.start_year;

    while t <= query.end_year {
        let value = match query.r#type {
            ParameterType::DiscountFactor => curve.discount_factor(t).unwrap_or(1.0),
            ParameterType::ZeroRate => curve.zero_rate(t).unwrap_or(0.0),
            ParameterType::ForwardRate => curve
                .forward_rate(t, t + query.grid_interval)
                .unwrap_or(0.0),
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
///
/// Uses the shared config from `market_data_config.json` for file paths.
pub async fn get_central_bank_meetings() -> ApiResult<serde_json::Value> {
    let config = get_config();
    let default_path = "demo/data/input/events/central_bank_meetings.json";
    let file_path = PathBuf::from(
        config
            .paths
            .events
            .as_ref()
            .and_then(|e| e.central_bank_meetings.as_ref())
            .map(String::as_str)
            .unwrap_or(default_path),
    );

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

#[cfg(test)]
mod tests {
    use adapter_loader::{parse_fra_tenor, parse_tenor_string};
    use pricer_models::market::curves::MarketInstrument;

    use super::*;

    #[test]
    fn test_parse_tenor_string() {
        // Uses adapter_loader::parse_tenor_string
        assert!((parse_tenor_string("1Y").unwrap() - 1.0).abs() < 1e-10);
        assert!((parse_tenor_string("6M").unwrap() - 0.5).abs() < 1e-10);
        assert!((parse_tenor_string("3M").unwrap() - 0.25).abs() < 1e-10);
        assert!((parse_tenor_string("1W").unwrap() - 1.0 / 52.0).abs() < 1e-10);
    }

    #[test]
    fn test_fra_tenor_parsing() {
        // Test "3x6" format (3M start, 6M end)
        let result = parse_fra_tenor("3x6");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!((start - 0.25).abs() < 1e-10); // 3M = 0.25Y
        assert!((end - 0.5).abs() < 1e-10); // 6M = 0.5Y

        // Test "6x12" format
        let result = parse_fra_tenor("6x12");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!((start - 0.5).abs() < 1e-10); // 6M = 0.5Y
        assert!((end - 1.0).abs() < 1e-10); // 12M = 1.0Y

        // Test "3Mx6M" format
        let result = parse_fra_tenor("3Mx6M");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!((start - 0.25).abs() < 1e-10);
        assert!((end - 0.5).abs() < 1e-10);

        // Test case insensitivity
        assert!(parse_fra_tenor("3X6").is_some());

        // Test invalid formats
        assert!(parse_fra_tenor("6M").is_none()); // Not FRA format

        // Test invalid: end <= start
        assert!(parse_fra_tenor("6x3").is_none());
    }

    #[test]
    fn test_fra_to_market_instrument() {
        // Test FRA with proper "3x6" format using InstrumentSpec
        let spec = InstrumentSpec::new("fra", "3x6", 0.025);
        let instrument = spec.to_market_instrument().unwrap();

        match instrument {
            MarketInstrument::Fra { start, end, rate } => {
                assert!((start - 0.25).abs() < 1e-10);
                assert!((end - 0.5).abs() < 1e-10);
                assert!((rate - 0.025).abs() < 1e-10);
            }
            _ => panic!("Expected FRA instrument"),
        }

        // Test FRA fallback with standard tenor (e.g., "6M" treated as 0x6M)
        let spec = InstrumentSpec::new("fra", "6M", 0.028);
        let instrument = spec.to_market_instrument().unwrap();

        match instrument {
            MarketInstrument::Fra { start, end, rate } => {
                assert!((start - 0.0).abs() < 1e-10); // Start at 0
                assert!((end - 0.5).abs() < 1e-10); // End at 6M
                assert!((rate - 0.028).abs() < 1e-10);
            }
            _ => panic!("Expected FRA instrument"),
        }
    }

    #[test]
    fn test_loader_not_found() {
        let loader = CurveDataLoader::default_path();
        let result = loader.load_instruments("nonexistent-index");
        assert!(result.is_err());
    }

    // =========================================================================
    // Duplicate Maturity Filter Tests
    // =========================================================================

    #[test]
    fn test_instrument_type_priority() {
        assert!(instrument_type_priority("deposit") < instrument_type_priority("ois"));
        assert!(instrument_type_priority("ois") < instrument_type_priority("fra"));
        assert!(instrument_type_priority("fra") < instrument_type_priority("future"));
        assert!(instrument_type_priority("DEPOSIT") < instrument_type_priority("OIS"));
        assert!(instrument_type_priority("swap") < instrument_type_priority("fra"));
    }

    #[test]
    fn test_filter_duplicate_maturities_keeps_higher_priority() {
        // FRA 3x6 and Future 6M both have maturity 0.5Y
        // FRA (priority 4) should be preferred over Future (priority 5)
        let specs = vec![
            InstrumentSpec::new("fra", "3x6", 0.0405),
            InstrumentSpec::new("future", "6M", 0.0395),
        ];

        let filtered = filter_duplicate_maturities(specs);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].instrument_type, "fra");
        assert!((filtered[0].rate - 0.0405).abs() < 1e-10);
    }

    #[test]
    fn test_filter_duplicate_maturities_preserves_unique() {
        // All instruments have unique maturities
        let specs = vec![
            InstrumentSpec::new("deposit", "1M", 0.0430),
            InstrumentSpec::new("fra", "3x6", 0.0405),
            InstrumentSpec::new("ois", "1Y", 0.0358),
        ];

        let filtered = filter_duplicate_maturities(specs);

        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_filter_duplicate_maturities_sorted_by_maturity() {
        // Test that output is sorted by maturity
        let specs = vec![
            InstrumentSpec::new("ois", "1Y", 0.0358),
            InstrumentSpec::new("deposit", "1M", 0.0430),
            InstrumentSpec::new("fra", "3x6", 0.0405),
        ];

        let filtered = filter_duplicate_maturities(specs);

        assert_eq!(filtered.len(), 3);
        // Should be sorted: 1M (0.0833), 3x6 (0.5), 1Y (1.0)
        assert_eq!(filtered[0].tenor, "1M");
        assert_eq!(filtered[1].tenor, "3x6");
        assert_eq!(filtered[2].tenor, "1Y");
    }

    #[test]
    fn test_filter_duplicate_maturities_ois_vs_swap_at_1y() {
        // OIS 1Y and FRA 9x12 both have maturity 1.0Y
        // OIS (priority 2) should be preferred over FRA (priority 4)
        let specs = vec![
            InstrumentSpec::new("fra", "9x12", 0.0368),
            InstrumentSpec::new("ois", "1Y", 0.0358),
        ];

        let filtered = filter_duplicate_maturities(specs);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].instrument_type, "ois");
    }

    // =========================================================================
    // Task 9: CB Event Input Tests
    // =========================================================================

    #[test]
    fn test_cb_event_input_valid() {
        let event = CbEventInput {
            date: "2025-03-15".to_string(),
            expected_jump_bps: 25.0,
            central_bank: Some("Fed".to_string()),
        };
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_cb_event_input_invalid_date() {
        let event = CbEventInput {
            date: "invalid-date".to_string(),
            expected_jump_bps: 25.0,
            central_bank: None,
        };
        let result = event.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid date format"));
    }

    #[test]
    fn test_cb_event_input_jump_out_of_range() {
        // Test positive out of range
        let event = CbEventInput {
            date: "2025-03-15".to_string(),
            expected_jump_bps: 150.0,
            central_bank: None,
        };
        let result = event.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of range"));

        // Test negative out of range
        let event = CbEventInput {
            date: "2025-03-15".to_string(),
            expected_jump_bps: -150.0,
            central_bank: None,
        };
        let result = event.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_cb_event_input_boundary_values() {
        // Test -100bps (valid)
        let event = CbEventInput {
            date: "2025-03-15".to_string(),
            expected_jump_bps: -100.0,
            central_bank: None,
        };
        assert!(event.validate().is_ok());

        // Test +100bps (valid)
        let event = CbEventInput {
            date: "2025-03-15".to_string(),
            expected_jump_bps: 100.0,
            central_bank: None,
        };
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_cb_event_input_serialization() {
        let event = CbEventInput {
            date: "2025-03-15".to_string(),
            expected_jump_bps: 25.0,
            central_bank: Some("Fed".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"date\":\"2025-03-15\""));
        assert!(json.contains("\"expectedJumpBps\":25.0"));
        assert!(json.contains("\"centralBank\":\"Fed\""));
    }

    #[test]
    fn test_cb_event_input_deserialization() {
        let json = r#"{
            "date": "2025-06-18",
            "expectedJumpBps": -25.0,
            "centralBank": "ECB"
        }"#;

        let event: CbEventInput = serde_json::from_str(json).unwrap();
        assert_eq!(event.date, "2025-06-18");
        assert!((event.expected_jump_bps - (-25.0)).abs() < 1e-10);
        assert_eq!(event.central_bank, Some("ECB".to_string()));
    }

    #[test]
    fn test_realized_jump_info_serialization() {
        let jump = RealizedJumpInfo {
            date: "2025-03-15".to_string(),
            central_bank: Some("Fed".to_string()),
            expected_bps: 25.0,
            realized_bps: 23.5,
            time_years: 0.125,
        };

        let json = serde_json::to_string(&jump).unwrap();
        assert!(json.contains("\"date\":\"2025-03-15\""));
        assert!(json.contains("\"expectedBps\":25.0"));
        assert!(json.contains("\"realizedBps\":23.5"));
    }

    // =========================================================================
    // Task 10: parse_and_validate_cb_events Tests
    // =========================================================================

    #[cfg(feature = "global-bootstrap")]
    mod jump_calibration_tests {
        use super::*;

        #[test]
        fn test_parse_cb_events_empty() {
            let mut warnings = Vec::new();
            let result = parse_and_validate_cb_events(None, 1.0, &mut warnings);
            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
        }

        #[test]
        fn test_parse_cb_events_empty_vec() {
            let mut warnings = Vec::new();
            let empty_vec = Vec::new();
            let result = parse_and_validate_cb_events(Some(&empty_vec), 1.0, &mut warnings);
            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
        }
    }
}
