//! Curve Builder WebApp type definitions.
//!
//! This module provides request/response types for the Curve Builder API,
//! including instrument lists, curve build parameters, and parameter curve
//! outputs.
//!
//! # API Endpoints Coverage
//!
//! - `GET /api/curves/instruments/{index}` → `InstrumentListResponse`
//! - `POST /api/curves/build` → `CurveBuildRequest`, `CurveBuildResponse`
//! - `GET /api/curves/{curveId}/parameters` → `ParameterResponse`
//! - `GET /api/curves/builders` → `BuilderListResponse`
//!
//! # Requirements Coverage
//!
//! - Requirement 1: Index別Instrument入力データ管理
//! - Requirement 3: カーブBuilderモデル選択
//! - Requirement 4: カーブ構築実行
//! - Requirement 5: Parameterカーブ表示
//! - Requirement 7: API設計

use serde::{Deserialize, Serialize};

// =============================================================================
// Interpolation and Bootstrap Method Enums (Task 2.2)
// =============================================================================

/// Interpolation method for yield curve construction.
///
/// The naming convention indicates the interpolation target:
/// - `*OnZeroRate`: Interpolates the continuously compounded zero rate
/// - `*OnLogDf`: Interpolates the natural log of discount factor (ln(DF))
///
/// # Requirements Coverage
///
/// - Requirement 3.1: LinearOnZeroRate, LinearOnLogDf, CubicSplineOnZeroRate,
///   MonotonicOnZeroRate補間手法を選択肢として提供
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationMethod {
    /// Linear interpolation on zero rates.
    /// Interpolates r(t) linearly, then DF(t) = exp(-r(t) * t).
    LinearOnZeroRate,
    /// Linear interpolation on log discount factors (recommended).
    /// Interpolates ln(DF(t)) linearly, equivalent to constant forward rate
    /// between pillars.
    #[default]
    LinearOnLogDf,
    /// Cubic spline interpolation on zero rates.
    /// Smooth interpolation with continuous first and second derivatives.
    CubicSplineOnZeroRate,
    /// Monotonic cubic interpolation on zero rates.
    /// Preserves monotonicity of the curve.
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
///
/// # Requirements Coverage
///
/// - Requirement 3.2: ブートストラップ手法（Sequential, Global）を選択可能
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapMethod {
    /// Sequential bootstrapping (instrument by instrument)
    #[default]
    Sequential,
    /// Global optimization (all instruments simultaneously) - Coming Soon
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
///
/// # Requirements Coverage
///
/// - Requirement 5.1: Discount Factor, Zero Rate, Forward Rateの表示モード切替
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    /// Discount factor (e.g., 0.95 for 5% discount over time)
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

/// Instrument type for curve construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentType {
    /// Money market deposit
    Deposit,
    /// Overnight Index Swap
    Ois,
    /// Interest Rate Swap
    Swap,
}

impl InstrumentType {
    /// Get the display name for this instrument type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Deposit => "Deposit",
            Self::Ois => "OIS",
            Self::Swap => "Swap",
        }
    }
}

// =============================================================================
// Instrument File Structure (Task 2.1)
// =============================================================================

/// Instrument definition from JSON file.
///
/// Used for deserializing instrument data from
/// `demo/data/input/rates/{index}.json`.
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
    /// Payment frequency (annual, semi_annual, quarterly)
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

// =============================================================================
// API Request/Response Types (Task 2.1)
// =============================================================================

/// Response for `GET /api/curves/instruments/{index}`.
///
/// # Requirements Coverage
///
/// - Requirement 1.3: Tenor, Rate Value, Index名,
///   Instrumentタイプを含む完全なInstrument定義
/// - Requirement 7.1: `/api/curves/instruments/{index}` エンドポイント
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
    /// Instrument type (deposit, ois, swap)
    pub instrument_type: String,
    /// Tenor string (e.g., "1M", "5Y")
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
/// - Requirement 4.1: 入力レートと選択されたBuilderモデルを使用してカーブ構築
/// - Requirement 7.2: `/api/curves/build` エンドポイント
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurveBuildRequest {
    /// Index identifier (e.g., "usd-sofr")
    pub index: String,
    /// Instruments with rates (potentially edited by user)
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
    /// Par rate (potentially edited by user)
    pub rate: f64,
}

/// Response for `POST /api/curves/build`.
///
/// # Requirements Coverage
///
/// - Requirement 4.3:
///   構築結果（成功/失敗、処理時間、使用Instrument数）をサマリとして表示
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurveBuildResponse {
    /// UUID of the constructed curve
    pub curve_id: String,
    /// Build status
    pub status: BuildStatus,
    /// Index name (e.g., "usd-sofr")
    pub index: String,
    /// Interpolation method used
    pub interpolation_method: String,
    /// Parameter points for visualisation
    pub parameters: Vec<CurveParameter>,
    /// Pillar points (years) - legacy field
    pub pillars: Vec<f64>,
    /// Discount factors at pillar points - legacy field
    pub discount_factors: Vec<f64>,
    /// Zero rates at pillar points - legacy field
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
///
/// # Requirements Coverage
///
/// - Requirement 5.5: Tenor範囲（開始日、終了日、グリッド間隔）をカスタマイズ
/// - Requirement 7.3: `/api/curves/{curveId}/parameters` エンドポイント
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ParameterQuery {
    /// Parameter type (discount_factor, zero_rate, forward_rate)
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
///
/// # Requirements Coverage
///
/// - Requirement 3.1, 3.2: 補間手法とブートストラップ手法の一覧
/// - Requirement 7.4: `/api/curves/builders` エンドポイント
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
// RFC 7807 Problem Details (Task 2.3)
// =============================================================================

/// RFC 7807 Problem Details error response.
///
/// # Requirements Coverage
///
/// - Requirement 7.5: RFC 7807準拠のProblem Details形式でエラーレスポンス
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemDetails {
    /// URI reference identifying the problem type
    pub r#type: String,
    /// Short human-readable summary
    pub title: String,
    /// HTTP status code
    pub status: u16,
    /// Detailed explanation
    pub detail: String,
    /// URI reference for this occurrence (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl ProblemDetails {
    /// Create a validation error (400 Bad Request).
    pub fn validation(detail: impl Into<String>) -> Self {
        Self {
            r#type: "https://api.neutryx.io/problems/validation-error".to_string(),
            title: "Validation Error".to_string(),
            status: 400,
            detail: detail.into(),
            instance: None,
        }
    }

    /// Create a not found error (404 Not Found).
    pub fn not_found(resource: &str, id: impl std::fmt::Display) -> Self {
        Self {
            r#type: "https://api.neutryx.io/problems/not-found".to_string(),
            title: "Resource Not Found".to_string(),
            status: 404,
            detail: format!("{} '{}' not found", resource, id),
            instance: None,
        }
    }

    /// Create a calculation error (422 Unprocessable Entity).
    pub fn calculation(detail: impl Into<String>) -> Self {
        Self {
            r#type: "https://api.neutryx.io/problems/calculation-error".to_string(),
            title: "Calculation Error".to_string(),
            status: 422,
            detail: detail.into(),
            instance: None,
        }
    }

    /// Create an internal server error (500).
    pub fn internal(detail: impl Into<String>) -> Self {
        Self {
            r#type: "https://api.neutryx.io/problems/internal-error".to_string(),
            title: "Internal Server Error".to_string(),
            status: 500,
            detail: detail.into(),
            instance: None,
        }
    }

    /// Add an instance URI to the problem details.
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }
}
