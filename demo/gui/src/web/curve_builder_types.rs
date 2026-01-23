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
/// `demo/data/input/curves/{index}.json`.
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
    /// Pillar points (years)
    pub pillars: Vec<f64>,
    /// Discount factors at pillar points
    pub discount_factors: Vec<f64>,
    /// Zero rates at pillar points
    pub zero_rates: Vec<f64>,
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
    /// Number of instruments used
    pub instrument_count: usize,
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Enum Tests (Task 2.2)
    // =========================================================================

    mod interpolation_method_tests {
        use super::*;

        #[test]
        fn test_interpolation_method_default() {
            let method = InterpolationMethod::default();
            assert_eq!(method, InterpolationMethod::LinearOnLogDf);
        }

        #[test]
        fn test_interpolation_method_serde() {
            let method = InterpolationMethod::CubicSplineOnZeroRate;
            let json = serde_json::to_string(&method).unwrap();
            assert_eq!(json, "\"cubic_spline_on_zero_rate\"");

            let parsed: InterpolationMethod =
                serde_json::from_str("\"linear_on_zero_rate\"").unwrap();
            assert_eq!(parsed, InterpolationMethod::LinearOnZeroRate);
        }

        #[test]
        fn test_interpolation_method_display_name() {
            assert_eq!(
                InterpolationMethod::LinearOnZeroRate.display_name(),
                "Linear on Zero Rate"
            );
            assert_eq!(
                InterpolationMethod::LinearOnLogDf.display_name(),
                "Linear on ln(DF)"
            );
        }

        #[test]
        fn test_interpolation_method_is_recommended() {
            assert!(InterpolationMethod::LinearOnLogDf.is_recommended());
            assert!(!InterpolationMethod::LinearOnZeroRate.is_recommended());
        }
    }

    mod bootstrap_method_tests {
        use super::*;

        #[test]
        fn test_bootstrap_method_default() {
            let method = BootstrapMethod::default();
            assert_eq!(method, BootstrapMethod::Sequential);
        }

        #[test]
        fn test_bootstrap_method_serde() {
            let method = BootstrapMethod::Global;
            let json = serde_json::to_string(&method).unwrap();
            assert_eq!(json, "\"global\"");
        }

        #[test]
        fn test_bootstrap_method_is_enabled() {
            assert!(BootstrapMethod::Sequential.is_enabled());
            assert!(!BootstrapMethod::Global.is_enabled());
        }
    }

    mod parameter_type_tests {
        use super::*;

        #[test]
        fn test_parameter_type_default() {
            let param_type = ParameterType::default();
            assert_eq!(param_type, ParameterType::DiscountFactor);
        }

        #[test]
        fn test_parameter_type_serde() {
            let param_type = ParameterType::ZeroRate;
            let json = serde_json::to_string(&param_type).unwrap();
            assert_eq!(json, "\"zero_rate\"");
        }

        #[test]
        fn test_parameter_type_display_name() {
            assert_eq!(
                ParameterType::DiscountFactor.display_name(),
                "Discount Factor"
            );
            assert_eq!(ParameterType::ZeroRate.display_name(), "Zero Rate");
            assert_eq!(ParameterType::ForwardRate.display_name(), "Forward Rate");
        }
    }

    // =========================================================================
    // Request/Response Tests (Task 2.1)
    // =========================================================================

    mod instrument_list_tests {
        use super::*;

        #[test]
        fn test_instrument_file_deserialise() {
            let json = r#"{
                "index": "usd-sofr",
                "currency": "USD",
                "reference_date": "2026-01-23",
                "instruments": [
                    {
                        "type": "deposit",
                        "tenor": "1M",
                        "tenor_years": 0.0833,
                        "rate": 0.0525,
                        "frequency": "annual"
                    }
                ]
            }"#;

            let file: InstrumentFile = serde_json::from_str(json).unwrap();
            assert_eq!(file.index, "usd-sofr");
            assert_eq!(file.currency, "USD");
            assert_eq!(file.instruments.len(), 1);
            assert_eq!(file.instruments[0].r#type, "deposit");
        }

        #[test]
        fn test_instrument_info_from_entry() {
            let entry = InstrumentFileEntry {
                r#type: "ois".to_string(),
                tenor: "5Y".to_string(),
                tenor_years: 5.0,
                rate: 0.0405,
                frequency: "annual".to_string(),
            };

            let info: InstrumentInfo = entry.into();
            assert_eq!(info.instrument_type, "ois");
            assert_eq!(info.tenor, "5Y");
            assert_eq!(info.tenor_years, 5.0);
        }

        #[test]
        fn test_instrument_list_response_serialise() {
            let response = InstrumentListResponse {
                index: "usd-sofr".to_string(),
                currency: "USD".to_string(),
                instruments: vec![InstrumentInfo {
                    instrument_type: "deposit".to_string(),
                    tenor: "1M".to_string(),
                    tenor_years: 0.0833,
                    rate: 0.0525,
                    frequency: "annual".to_string(),
                }],
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"instrumentType\""));
            assert!(json.contains("\"tenorYears\""));
        }
    }

    mod curve_build_tests {
        use super::*;

        #[test]
        fn test_curve_build_request_deserialise() {
            let json = r#"{
                "index": "usd-sofr",
                "instruments": [
                    {
                        "instrumentType": "deposit",
                        "tenor": "1M",
                        "rate": 0.0525
                    }
                ],
                "interpolation": "linear_on_log_df"
            }"#;

            let request: CurveBuildRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.index, "usd-sofr");
            assert_eq!(request.interpolation, InterpolationMethod::LinearOnLogDf);
            assert_eq!(request.bootstrap_method, BootstrapMethod::Sequential);
            assert_eq!(request.tolerance, 1e-10);
            assert_eq!(request.max_iterations, 100);
        }

        #[test]
        fn test_curve_build_response_serialise() {
            let response = CurveBuildResponse {
                curve_id: "abc-123".to_string(),
                status: BuildStatus::Success,
                pillars: vec![0.25, 0.5, 1.0],
                discount_factors: vec![0.9875, 0.975, 0.95],
                zero_rates: vec![0.05, 0.0505, 0.051],
                processing_time_ms: 15.5,
                instrument_count: 3,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"curveId\""));
            assert!(json.contains("\"processingTimeMs\""));
            assert!(json.contains("\"instrumentCount\""));
        }
    }

    mod parameter_tests {
        use super::*;

        #[test]
        fn test_parameter_query_defaults() {
            let json = r#"{"type": "zero_rate"}"#;
            let query: ParameterQuery = serde_json::from_str(json).unwrap();

            assert_eq!(query.r#type, ParameterType::ZeroRate);
            assert_eq!(query.start_year, 0.0);
            assert_eq!(query.end_year, 30.0);
            assert_eq!(query.grid_interval, 0.25);
        }

        #[test]
        fn test_parameter_response_serialise() {
            let response = ParameterResponse {
                curve_id: "abc-123".to_string(),
                parameter_type: ParameterType::DiscountFactor,
                data: vec![
                    ParameterPoint {
                        tenor: 0.25,
                        value: 0.9875,
                    },
                    ParameterPoint {
                        tenor: 0.5,
                        value: 0.975,
                    },
                ],
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"parameterType\""));
            assert!(json.contains("\"discount_factor\""));
        }
    }

    // =========================================================================
    // Builder List Tests (Task 4.1)
    // =========================================================================

    mod builder_list_tests {
        use super::*;

        #[test]
        fn test_builder_list_response_new() {
            let response = BuilderListResponse::new();

            assert_eq!(response.interpolation_methods.len(), 4);
            assert_eq!(response.bootstrap_methods.len(), 2);

            // Check that linear_on_log_df is recommended
            let linear_on_log_df = response
                .interpolation_methods
                .iter()
                .find(|m| m.id == "linear_on_log_df")
                .unwrap();
            assert!(linear_on_log_df.recommended);

            // Check that global is disabled
            let global = response
                .bootstrap_methods
                .iter()
                .find(|m| m.id == "global")
                .unwrap();
            assert!(!global.enabled);
        }

        #[test]
        fn test_builder_list_response_serialise() {
            let response = BuilderListResponse::new();
            let json = serde_json::to_string(&response).unwrap();

            assert!(json.contains("\"interpolationMethods\""));
            assert!(json.contains("\"bootstrapMethods\""));
            assert!(json.contains("\"recommended\""));
            assert!(json.contains("\"enabled\""));
        }
    }

    // =========================================================================
    // Problem Details Tests (Task 2.3)
    // =========================================================================

    mod problem_details_tests {
        use super::*;

        #[test]
        fn test_problem_details_validation() {
            let problem = ProblemDetails::validation("Rate must be positive");
            assert_eq!(problem.status, 400);
            assert_eq!(problem.title, "Validation Error");
            assert!(problem.detail.contains("Rate must be positive"));
        }

        #[test]
        fn test_problem_details_not_found() {
            let problem = ProblemDetails::not_found("Curve", "abc-123");
            assert_eq!(problem.status, 404);
            assert!(problem.detail.contains("Curve 'abc-123' not found"));
        }

        #[test]
        fn test_problem_details_calculation() {
            let problem = ProblemDetails::calculation("Bootstrap failed to converge");
            assert_eq!(problem.status, 422);
            assert_eq!(problem.title, "Calculation Error");
        }

        #[test]
        fn test_problem_details_with_instance() {
            let problem = ProblemDetails::validation("Invalid rate")
                .with_instance("/api/curves/build/request-123");

            assert!(problem.instance.is_some());
            assert_eq!(problem.instance.unwrap(), "/api/curves/build/request-123");
        }

        #[test]
        fn test_problem_details_serialise() {
            let problem = ProblemDetails::validation("Test error");
            let json = serde_json::to_string(&problem).unwrap();

            assert!(json.contains("\"type\""));
            assert!(json.contains("\"title\""));
            assert!(json.contains("\"status\""));
            assert!(json.contains("\"detail\""));
            // Instance should be skipped when None
            assert!(!json.contains("\"instance\""));
        }
    }
}
