//! Pricer API types for the FrictionalBank WebApp.
//!
//! This module defines request/response types for the pricing API endpoint.
//! All types support JSON serialisation with camelCase field names for
//! JavaScript interoperability.
//!
//! # Task Coverage
//!
//! - Task 1.1: プライサーAPI用リクエスト/レスポンス型の定義
//!
//! # Requirements Coverage
//!
//! - Requirement 2.1, 2.2, 2.3: 商品固有パラメータ型
//! - Requirement 3.2, 3.5: 価格計算レスポンス型
//! - Requirement 4.1, 4.2: Greeks結果型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;
use uuid::Uuid;

// =============================================================================
// Instrument Type Enum
// =============================================================================

/// Instrument type for pricing requests.
///
/// Represents the type of derivative instrument to price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentType {
    /// Equity vanilla option (European style)
    EquityVanillaOption,
    /// FX option (Garman-Kohlhagen model)
    FxOption,
    /// Interest Rate Swap
    Irs,
}

// =============================================================================
// Option Type Enum
// =============================================================================

/// Option type (Call or Put).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OptionType {
    /// Call option (right to buy)
    #[default]
    Call,
    /// Put option (right to sell)
    Put,
}

// =============================================================================
// Instrument Parameters
// =============================================================================

/// Equity vanilla option parameters.
///
/// Parameters required to price a European-style equity option
/// using the Black-Scholes model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityOptionParams {
    /// Current spot price of the underlying
    pub spot: f64,
    /// Strike price of the option
    pub strike: f64,
    /// Time to expiry in years
    pub expiry_years: f64,
    /// Annualised volatility (e.g., 0.2 for 20%)
    pub volatility: f64,
    /// Risk-free interest rate (e.g., 0.05 for 5%)
    pub rate: f64,
    /// Option type (Call or Put)
    pub option_type: OptionType,
}

/// FX option parameters.
///
/// Parameters required to price an FX option using the
/// Garman-Kohlhagen model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxOptionParams {
    /// Current spot exchange rate
    pub spot: f64,
    /// Strike exchange rate
    pub strike: f64,
    /// Time to expiry in years
    pub expiry_years: f64,
    /// Domestic risk-free interest rate
    pub domestic_rate: f64,
    /// Foreign risk-free interest rate
    pub foreign_rate: f64,
    /// Annualised volatility
    pub volatility: f64,
    /// Option type (Call or Put)
    pub option_type: OptionType,
}

/// Interest Rate Swap parameters.
///
/// Parameters required to price an IRS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrsParams {
    /// Notional principal amount
    pub notional: f64,
    /// Fixed leg rate (e.g., 0.03 for 3%)
    pub fixed_rate: f64,
    /// Swap tenor in years
    pub tenor_years: f64,
}

/// Instrument parameters (discriminated union).
///
/// Uses `#[serde(untagged)]` for flexible JSON parsing where the
/// structure determines the variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InstrumentParams {
    /// Equity option parameters
    EquityOption(EquityOptionParams),
    /// FX option parameters
    FxOption(FxOptionParams),
    /// IRS parameters
    Irs(IrsParams),
}

// =============================================================================
// Market Data Configuration
// =============================================================================

/// Market data source configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketDataSource {
    /// Use demo/sample market data
    Demo,
    /// Use custom market data from request
    Custom,
}

/// Market data configuration for pricing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataConfig {
    /// Market data source
    pub source: MarketDataSource,
    /// Custom yield curve rate (for Custom source)
    pub curve_rate: Option<f64>,
    /// Custom volatility (for Custom source)
    pub volatility: Option<f64>,
}

// =============================================================================
// Demo Market Data (Task 1.2)
// =============================================================================

/// Default demo market data values.
///
/// Provides standard market data for demonstration purposes.
#[derive(Debug, Clone, PartialEq)]
pub struct DemoMarketData {
    /// Flat yield curve rate (5%)
    pub curve_rate: f64,
    /// Default volatility (20%)
    pub volatility: f64,
}

impl Default for DemoMarketData {
    fn default() -> Self {
        Self::new()
    }
}

impl DemoMarketData {
    /// Default flat curve rate (5% p.a.)
    pub const DEFAULT_CURVE_RATE: f64 = 0.05;
    /// Default volatility (20% annualised)
    pub const DEFAULT_VOLATILITY: f64 = 0.20;

    /// Create new demo market data with default values.
    pub fn new() -> Self {
        Self {
            curve_rate: Self::DEFAULT_CURVE_RATE,
            volatility: Self::DEFAULT_VOLATILITY,
        }
    }

    /// Create demo market data with custom curve rate.
    pub fn with_curve_rate(mut self, rate: f64) -> Self {
        self.curve_rate = rate;
        self
    }

    /// Create demo market data with custom volatility.
    pub fn with_volatility(mut self, vol: f64) -> Self {
        self.volatility = vol;
        self
    }

    /// Get the effective curve rate from market data config.
    ///
    /// Uses custom rate if provided and source is Custom,
    /// otherwise returns demo default.
    pub fn get_curve_rate(config: Option<&MarketDataConfig>) -> f64 {
        config
            .and_then(|c| {
                if c.source == MarketDataSource::Custom {
                    c.curve_rate
                } else {
                    None
                }
            })
            .unwrap_or(Self::DEFAULT_CURVE_RATE)
    }

    /// Get the effective volatility from market data config.
    ///
    /// Uses custom volatility if provided and source is Custom,
    /// otherwise returns demo default.
    pub fn get_volatility(config: Option<&MarketDataConfig>) -> f64 {
        config
            .and_then(|c| {
                if c.source == MarketDataSource::Custom {
                    c.volatility
                } else {
                    None
                }
            })
            .unwrap_or(Self::DEFAULT_VOLATILITY)
    }
}

// =============================================================================
// Pricing Request
// =============================================================================

/// Pricing calculation request.
///
/// Sent by the client to request a price calculation for an instrument.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingRequest {
    /// Type of instrument to price
    pub instrument_type: InstrumentType,
    /// Instrument-specific parameters
    pub params: InstrumentParams,
    /// Market data configuration (optional, defaults to Demo)
    pub market_data: Option<MarketDataConfig>,
    /// Whether to compute Greeks
    pub compute_greeks: bool,
}

// =============================================================================
// Greeks Data
// =============================================================================

/// Greeks calculation results.
///
/// Contains first-order sensitivities (Delta, Gamma, Vega, Theta, Rho).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GreeksData {
    /// Delta: sensitivity to underlying price
    pub delta: f64,
    /// Gamma: sensitivity of delta to underlying price
    pub gamma: f64,
    /// Vega: sensitivity to volatility (per 1% move)
    pub vega: f64,
    /// Theta: time decay (per day)
    pub theta: f64,
    /// Rho: sensitivity to interest rate (per 1% move)
    pub rho: f64,
}

// =============================================================================
// Pricing Response
// =============================================================================

/// Pricing calculation response.
///
/// Returned by the server after a successful price calculation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingResponse {
    /// Unique calculation ID (for history tracking)
    pub calculation_id: String,
    /// Type of instrument that was priced
    pub instrument_type: InstrumentType,
    /// Present value / price
    pub pv: f64,
    /// Greeks (if compute_greeks was true)
    pub greeks: Option<GreeksData>,
    /// Calculation timestamp (Unix epoch milliseconds)
    pub timestamp: i64,
}

// =============================================================================
// Error Response
// =============================================================================

/// Pricing error response.
///
/// Returned when a pricing request fails.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingErrorResponse {
    /// Error type identifier
    pub error_type: String,
    /// Human-readable error message
    pub message: String,
    /// Field that caused the error (for validation errors)
    pub field: Option<String>,
}

// =============================================================================
// Bootstrap Types (Task 1.1: IRS Bootstrap & Risk)
// =============================================================================

/// Interpolation method for yield curve construction.
///
/// Determines how discount factors are interpolated between pillar points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationMethod {
    /// Linear interpolation of discount factors
    Linear,
    /// Log-linear interpolation (linear in log-space)
    #[default]
    LogLinear,
}

/// Par rate input for a single tenor point.
///
/// Represents a market-observed IRS par rate at a specific tenor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParRateInput {
    /// Tenor string (e.g., "1Y", "2Y", "5Y", "10Y", "30Y")
    pub tenor: String,
    /// Par rate as decimal (e.g., 0.025 for 2.5%)
    pub rate: f64,
}

/// Bootstrap request for yield curve construction.
///
/// Sent by the client to construct a yield curve from par rates.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapRequest {
    /// List of par rate inputs for each tenor
    pub par_rates: Vec<ParRateInput>,
    /// Interpolation method (default: log_linear)
    #[serde(default)]
    pub interpolation: InterpolationMethod,
}

/// Bootstrap response with constructed curve data.
///
/// Returned by the server after successful curve construction.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapResponse {
    /// Unique curve identifier (UUID)
    pub curve_id: String,
    /// Pillar points in years
    pub pillars: Vec<f64>,
    /// Discount factors at each pillar
    pub discount_factors: Vec<f64>,
    /// Zero rates at each pillar
    pub zero_rates: Vec<f64>,
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
}

// =============================================================================
// IRS Pricing Types (Task 1.2: IRS Bootstrap & Risk)
// =============================================================================

/// Payment frequency for IRS legs.
///
/// Determines how often payments are made on the swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum PaymentFrequency {
    /// Annual payments (once per year)
    #[default]
    Annual,
    /// Semi-annual payments (twice per year)
    SemiAnnual,
    /// Quarterly payments (four times per year)
    Quarterly,
    /// Monthly payments (twelve times per year)
    Monthly,
}

/// IRS pricing request using a bootstrapped curve.
///
/// Sent by the client to price an IRS using a previously constructed curve.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrsPricingRequest {
    /// Curve identifier from bootstrap response
    pub curve_id: String,
    /// Notional principal amount
    pub notional: f64,
    /// Fixed leg rate (e.g., 0.03 for 3%)
    pub fixed_rate: f64,
    /// Swap tenor in years
    pub tenor_years: f64,
    /// Payment frequency for both legs
    pub payment_frequency: PaymentFrequency,
}

/// IRS pricing response with valuation results.
///
/// Returned by the server after successful IRS pricing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IrsPricingResponse {
    /// Net present value of the swap
    pub npv: f64,
    /// Present value of the fixed leg
    pub fixed_leg_pv: f64,
    /// Present value of the floating leg
    pub float_leg_pv: f64,
    /// Processing time in microseconds
    pub processing_time_us: f64,
}

// =============================================================================
// Risk Types (Task 1.3: IRS Bootstrap & Risk)
// =============================================================================

/// Risk calculation request for delta sensitivities.
///
/// Sent by the client to calculate risk using Bump or AAD methods.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskRequest {
    /// Curve identifier from bootstrap response
    pub curve_id: String,
    /// Notional principal amount
    pub notional: f64,
    /// Fixed leg rate (e.g., 0.03 for 3%)
    pub fixed_rate: f64,
    /// Swap tenor in years
    pub tenor_years: f64,
    /// Payment frequency for both legs
    pub payment_frequency: PaymentFrequency,
    /// Bump size in basis points (default: 1)
    #[serde(default = "default_bump_size_bps")]
    pub bump_size_bps: f64,
}

/// Default bump size of 1 basis point.
fn default_bump_size_bps() -> f64 {
    1.0
}

/// Delta result for a single tenor point.
///
/// Represents the sensitivity of NPV to a 1bp change in the par rate at this tenor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaResult {
    /// Tenor string (e.g., "1Y", "5Y", "10Y")
    pub tenor: String,
    /// Delta value (NPV sensitivity per bp)
    pub delta: f64,
    /// Processing time for this tenor in microseconds
    pub processing_time_us: f64,
}

/// Timing statistics for risk calculations.
///
/// Aggregated timing information across all tenor calculations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimingStats {
    /// Mean calculation time in microseconds
    pub mean_us: f64,
    /// Standard deviation of calculation times in microseconds
    pub std_dev_us: f64,
    /// Minimum calculation time in microseconds
    pub min_us: f64,
    /// Maximum calculation time in microseconds
    pub max_us: f64,
    /// Total calculation time in milliseconds
    pub total_ms: f64,
}

/// Risk calculation result for a single method (Bump or AAD).
///
/// Contains all delta values and timing information for one calculation method.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskMethodResult {
    /// Delta results for each tenor
    pub deltas: Vec<DeltaResult>,
    /// DV01 (sum of all deltas)
    pub dv01: f64,
    /// Timing statistics
    pub timing: TimingStats,
}

/// Timing comparison between Bump and AAD methods.
///
/// Used to compare performance of the two risk calculation approaches.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimingComparison {
    /// Total Bump method time in milliseconds
    pub bump_total_ms: f64,
    /// Total AAD method time in milliseconds (null if AAD unavailable)
    pub aad_total_ms: Option<f64>,
    /// Speedup ratio (Bump time / AAD time, null if AAD unavailable)
    pub speedup_ratio: Option<f64>,
}

/// Risk comparison response with both methods' results.
///
/// Returned by the server after executing both Bump and AAD risk calculations.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskCompareResponse {
    /// Bump method results
    pub bump: RiskMethodResult,
    /// AAD method results (null if AAD unavailable)
    pub aad: Option<RiskMethodResult>,
    /// Whether AAD is available
    pub aad_available: bool,
    /// Speedup ratio (Bump time / AAD time, null if AAD unavailable)
    pub speedup_ratio: Option<f64>,
    /// Timing comparison
    pub comparison: TimingComparison,
}

/// Risk response for Bump method only.
///
/// Returned by the /api/risk/bump endpoint.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskBumpResponse {
    /// Delta results for each tenor
    pub deltas: Vec<DeltaResult>,
    /// DV01 (sum of all deltas)
    pub dv01: f64,
    /// Timing statistics
    pub timing: TimingStats,
}

/// Risk response for AAD method only.
///
/// Returned by the /api/risk/aad endpoint.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskAadResponse {
    /// Delta results for each tenor
    pub deltas: Vec<DeltaResult>,
    /// DV01 (sum of all deltas)
    pub dv01: f64,
    /// Timing statistics
    pub timing: TimingStats,
    /// Whether AAD is available (always true for this response)
    pub aad_available: bool,
}

// =============================================================================
// Error Response Types (Task 1.4: IRS Bootstrap & Risk)
// =============================================================================

/// Error details for validation and calculation errors.
///
/// Provides additional context about the error, including the failing field,
/// tenor (for bootstrap failures), and remediation suggestions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetails {
    /// Field that caused the validation error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Tenor that failed during bootstrap
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenor: Option<String>,
    /// Suggestion for how to fix the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl ErrorDetails {
    /// Create new empty error details.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create error details with a field.
    pub fn with_field(field: impl Into<String>) -> Self {
        Self {
            field: Some(field.into()),
            ..Default::default()
        }
    }

    /// Create error details with a tenor.
    pub fn with_tenor(tenor: impl Into<String>) -> Self {
        Self {
            tenor: Some(tenor.into()),
            ..Default::default()
        }
    }

    /// Create error details with a suggestion.
    pub fn with_suggestion(suggestion: impl Into<String>) -> Self {
        Self {
            suggestion: Some(suggestion.into()),
            ..Default::default()
        }
    }

    /// Add a field to the error details.
    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// Add a tenor to the error details.
    pub fn tenor(mut self, tenor: impl Into<String>) -> Self {
        self.tenor = Some(tenor.into());
        self
    }

    /// Add a suggestion to the error details.
    pub fn suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Check if the error details are empty.
    pub fn is_empty(&self) -> bool {
        self.field.is_none() && self.tenor.is_none() && self.suggestion.is_none()
    }
}

/// Error response for IRS Bootstrap & Risk API.
///
/// Follows the design document schema for error responses with HTTP status mapping:
/// - 400 Bad Request: Validation errors (negative values, invalid fields)
/// - 404 Not Found: Curve ID not found
/// - 422 Unprocessable Entity: Bootstrap convergence failure, calculation errors
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IrsBootstrapErrorResponse {
    /// Error type identifier (e.g., "ValidationError", "NotFoundError", "CalculationError")
    pub error: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details (only serialised if non-empty)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ErrorDetails>,
}

impl IrsBootstrapErrorResponse {
    /// Create a validation error (HTTP 400).
    pub fn validation_error(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            error: "ValidationError".to_string(),
            message: message.into(),
            details: Some(ErrorDetails::with_field(field)),
        }
    }

    /// Create a not found error (HTTP 404).
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            error: "NotFoundError".to_string(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a curve not found error (HTTP 404).
    pub fn curve_not_found(curve_id: impl Into<String>) -> Self {
        let curve_id = curve_id.into();
        Self {
            error: "NotFoundError".to_string(),
            message: format!("Curve with ID '{}' not found", curve_id),
            details: Some(ErrorDetails::with_field("curveId")),
        }
    }

    /// Create a bootstrap convergence failure error (HTTP 422).
    pub fn bootstrap_convergence_failure(
        tenor: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let tenor = tenor.into();
        Self {
            error: "CalculationError".to_string(),
            message: format!("Bootstrap failed to converge at tenor {}", tenor),
            details: Some(ErrorDetails::new().tenor(tenor).suggestion(suggestion)),
        }
    }

    /// Create a calculation error (HTTP 422).
    pub fn calculation_error(message: impl Into<String>) -> Self {
        Self {
            error: "CalculationError".to_string(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a calculation error with details (HTTP 422).
    pub fn calculation_error_with_details(
        message: impl Into<String>,
        details: ErrorDetails,
    ) -> Self {
        Self {
            error: "CalculationError".to_string(),
            message: message.into(),
            details: if details.is_empty() {
                None
            } else {
                Some(details)
            },
        }
    }
}

// =============================================================================
// Validation Types (Task 1.4: IRS Bootstrap & Risk)
// =============================================================================

/// Validation error for Par Rate and IRS parameter validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Par rate is negative
    NegativeParRate { tenor: String, rate: f64 },
    /// Par rate is not a valid number (NaN or infinite)
    InvalidParRateValue { tenor: String, rate: f64 },
    /// Empty par rates list
    EmptyParRates,
    /// Notional is not positive
    NotionalNotPositive { notional: f64 },
    /// Fixed rate is out of valid range
    FixedRateOutOfRange { fixed_rate: f64 },
    /// Tenor years is not positive
    TenorYearsNotPositive { tenor_years: f64 },
    /// Tenor years exceeds maximum
    TenorYearsExceedsMax { tenor_years: f64, max: f64 },
    /// Invalid tenor string format
    InvalidTenorFormat { tenor: String },
    /// Bump size is not positive
    BumpSizeNotPositive { bump_size_bps: f64 },
}

impl ValidationError {
    /// Convert validation error to IrsBootstrapErrorResponse.
    pub fn to_error_response(&self) -> IrsBootstrapErrorResponse {
        match self {
            ValidationError::NegativeParRate { tenor, rate } => {
                IrsBootstrapErrorResponse::validation_error(
                    format!(
                        "Par rate at tenor {} must be non-negative, got {}",
                        tenor, rate
                    ),
                    format!("parRates[{}].rate", tenor),
                )
            }
            ValidationError::InvalidParRateValue { tenor, rate } => {
                IrsBootstrapErrorResponse::validation_error(
                    format!(
                        "Par rate at tenor {} must be a valid number, got {}",
                        tenor, rate
                    ),
                    format!("parRates[{}].rate", tenor),
                )
            }
            ValidationError::EmptyParRates => IrsBootstrapErrorResponse::validation_error(
                "Par rates list cannot be empty",
                "parRates",
            ),
            ValidationError::NotionalNotPositive { notional } => {
                IrsBootstrapErrorResponse::validation_error(
                    format!("Notional must be positive, got {}", notional),
                    "notional",
                )
            }
            ValidationError::FixedRateOutOfRange { fixed_rate } => {
                IrsBootstrapErrorResponse::validation_error(
                    format!(
                        "Fixed rate must be between -1.0 and 1.0, got {}",
                        fixed_rate
                    ),
                    "fixedRate",
                )
            }
            ValidationError::TenorYearsNotPositive { tenor_years } => {
                IrsBootstrapErrorResponse::validation_error(
                    format!("Tenor years must be positive, got {}", tenor_years),
                    "tenorYears",
                )
            }
            ValidationError::TenorYearsExceedsMax { tenor_years, max } => {
                IrsBootstrapErrorResponse::validation_error(
                    format!(
                        "Tenor years {} exceeds maximum of {} years",
                        tenor_years, max
                    ),
                    "tenorYears",
                )
            }
            ValidationError::InvalidTenorFormat { tenor } => {
                IrsBootstrapErrorResponse::validation_error(
                    format!(
                        "Invalid tenor format '{}'. Expected format like '1Y', '5Y', '10Y'",
                        tenor
                    ),
                    "tenor",
                )
            }
            ValidationError::BumpSizeNotPositive { bump_size_bps } => {
                IrsBootstrapErrorResponse::validation_error(
                    format!("Bump size must be positive, got {} bps", bump_size_bps),
                    "bumpSizeBps",
                )
            }
        }
    }
}

// =============================================================================
// Validation Functions (Task 1.4: IRS Bootstrap & Risk)
// =============================================================================

/// Valid tenor strings for IRS Bootstrap.
pub const VALID_TENORS: [&str; 9] = ["1Y", "2Y", "3Y", "5Y", "7Y", "10Y", "15Y", "20Y", "30Y"];

/// Maximum tenor years allowed.
pub const MAX_TENOR_YEARS: f64 = 50.0;

/// Validate a single par rate input.
///
/// Returns an error if the rate is negative or not a valid number.
pub fn validate_par_rate(input: &ParRateInput) -> Result<(), ValidationError> {
    // Check for NaN or infinite values
    if input.rate.is_nan() || input.rate.is_infinite() {
        return Err(ValidationError::InvalidParRateValue {
            tenor: input.tenor.clone(),
            rate: input.rate,
        });
    }

    // Check for negative values
    if input.rate < 0.0 {
        return Err(ValidationError::NegativeParRate {
            tenor: input.tenor.clone(),
            rate: input.rate,
        });
    }

    Ok(())
}

/// Validate a list of par rate inputs for bootstrap.
///
/// Returns an error if any rate is invalid or the list is empty.
pub fn validate_par_rates(inputs: &[ParRateInput]) -> Result<(), ValidationError> {
    if inputs.is_empty() {
        return Err(ValidationError::EmptyParRates);
    }

    for input in inputs {
        validate_par_rate(input)?;
    }

    Ok(())
}

/// Validate IRS pricing request parameters.
///
/// Returns an error if any parameter is out of valid range.
pub fn validate_irs_pricing_request(request: &IrsPricingRequest) -> Result<(), ValidationError> {
    // Validate notional
    if request.notional <= 0.0 || request.notional.is_nan() || request.notional.is_infinite() {
        return Err(ValidationError::NotionalNotPositive {
            notional: request.notional,
        });
    }

    // Validate fixed rate (must be between -100% and +100%)
    if request.fixed_rate.is_nan()
        || request.fixed_rate.is_infinite()
        || request.fixed_rate < -1.0
        || request.fixed_rate > 1.0
    {
        return Err(ValidationError::FixedRateOutOfRange {
            fixed_rate: request.fixed_rate,
        });
    }

    // Validate tenor years
    if request.tenor_years <= 0.0
        || request.tenor_years.is_nan()
        || request.tenor_years.is_infinite()
    {
        return Err(ValidationError::TenorYearsNotPositive {
            tenor_years: request.tenor_years,
        });
    }

    if request.tenor_years > MAX_TENOR_YEARS {
        return Err(ValidationError::TenorYearsExceedsMax {
            tenor_years: request.tenor_years,
            max: MAX_TENOR_YEARS,
        });
    }

    Ok(())
}

/// Validate risk request parameters.
///
/// Returns an error if any parameter is out of valid range.
pub fn validate_risk_request(request: &RiskRequest) -> Result<(), ValidationError> {
    // Validate notional
    if request.notional <= 0.0 || request.notional.is_nan() || request.notional.is_infinite() {
        return Err(ValidationError::NotionalNotPositive {
            notional: request.notional,
        });
    }

    // Validate fixed rate
    if request.fixed_rate.is_nan()
        || request.fixed_rate.is_infinite()
        || request.fixed_rate < -1.0
        || request.fixed_rate > 1.0
    {
        return Err(ValidationError::FixedRateOutOfRange {
            fixed_rate: request.fixed_rate,
        });
    }

    // Validate tenor years
    if request.tenor_years <= 0.0
        || request.tenor_years.is_nan()
        || request.tenor_years.is_infinite()
    {
        return Err(ValidationError::TenorYearsNotPositive {
            tenor_years: request.tenor_years,
        });
    }

    if request.tenor_years > MAX_TENOR_YEARS {
        return Err(ValidationError::TenorYearsExceedsMax {
            tenor_years: request.tenor_years,
            max: MAX_TENOR_YEARS,
        });
    }

    // Validate bump size
    if request.bump_size_bps <= 0.0
        || request.bump_size_bps.is_nan()
        || request.bump_size_bps.is_infinite()
    {
        return Err(ValidationError::BumpSizeNotPositive {
            bump_size_bps: request.bump_size_bps,
        });
    }

    Ok(())
}

/// Parse a tenor string (e.g., "5Y") into years.
///
/// Returns the number of years, or an error if the format is invalid.
pub fn parse_tenor_to_years(tenor: &str) -> Result<f64, ValidationError> {
    // Tenor format: digits followed by Y (case-insensitive)
    let tenor_upper = tenor.to_uppercase();
    if !tenor_upper.ends_with('Y') {
        return Err(ValidationError::InvalidTenorFormat {
            tenor: tenor.to_string(),
        });
    }

    let years_str = &tenor_upper[..tenor_upper.len() - 1];
    years_str
        .parse::<f64>()
        .map_err(|_| ValidationError::InvalidTenorFormat {
            tenor: tenor.to_string(),
        })
}

// =============================================================================
// CurveCache Types (Task 1.5: IRS Bootstrap & Risk)
// =============================================================================

/// Cached curve entry with metadata.
///
/// Stores a bootstrapped curve along with its data for quick access.
/// Used by the CurveCache to store curves created by the bootstrap API.
#[derive(Debug, Clone)]
pub struct CachedCurve {
    /// Pillar maturities in years
    pub pillars: Vec<f64>,
    /// Discount factors at each pillar
    pub discount_factors: Vec<f64>,
    /// Zero rates at each pillar
    pub zero_rates: Vec<f64>,
    /// Original par rates used to bootstrap this curve (Task 4.1: Required for bump-and-revalue)
    pub par_rates: Vec<ParRateInput>,
    /// Creation timestamp
    pub created_at: Instant,
}

impl CachedCurve {
    /// Create a new cached curve entry.
    ///
    /// # Arguments
    ///
    /// * `pillars` - Pillar maturities in years
    /// * `discount_factors` - Discount factors at each pillar
    /// * `zero_rates` - Zero rates at each pillar
    /// * `par_rates` - Original par rates used to bootstrap this curve
    pub fn new(
        pillars: Vec<f64>,
        discount_factors: Vec<f64>,
        zero_rates: Vec<f64>,
        par_rates: Vec<ParRateInput>,
    ) -> Self {
        Self {
            pillars,
            discount_factors,
            zero_rates,
            par_rates,
            created_at: Instant::now(),
        }
    }

    /// Calculate zero rates from discount factors.
    ///
    /// Zero rate = -ln(DF) / T
    pub fn calculate_zero_rates(pillars: &[f64], discount_factors: &[f64]) -> Vec<f64> {
        pillars
            .iter()
            .zip(discount_factors.iter())
            .map(|(t, df)| {
                if *t > 0.0 && *df > 0.0 {
                    -df.ln() / t
                } else {
                    0.0
                }
            })
            .collect()
    }

    /// Get the number of pillars.
    pub fn pillar_count(&self) -> usize {
        self.pillars.len()
    }

    /// Get the age of this cache entry in seconds.
    pub fn age_seconds(&self) -> u64 {
        self.created_at.elapsed().as_secs()
    }
}

/// In-memory cache for bootstrapped curves.
///
/// Provides thread-safe storage and retrieval of bootstrapped curves
/// keyed by UUID. Used by the IRS Bootstrap & Risk API handlers.
///
/// # Thread Safety
///
/// Uses `RwLock` for thread-safe access with read-write separation.
/// Multiple readers can access the cache simultaneously, while writes
/// are exclusive.
///
/// # Requirements Coverage
///
/// - Requirement 2.3: Store constructed curves for subsequent pricing
/// - Requirement 3.2: Enable curve retrieval by curve_id for pricing
#[derive(Debug, Default)]
pub struct BootstrapCurveCache {
    /// Map of curve_id to cached curve data
    curves: RwLock<HashMap<Uuid, CachedCurve>>,
}

impl BootstrapCurveCache {
    /// Create a new empty curve cache.
    pub fn new() -> Self {
        Self {
            curves: RwLock::new(HashMap::new()),
        }
    }

    /// Add a curve to the cache.
    ///
    /// # Arguments
    ///
    /// * `curve_id` - Unique identifier for the curve
    /// * `curve` - The cached curve data
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned.
    pub fn add(&self, curve_id: Uuid, curve: CachedCurve) {
        let mut curves = self.curves.write().unwrap();
        curves.insert(curve_id, curve);
    }

    /// Get a curve from the cache.
    ///
    /// # Arguments
    ///
    /// * `curve_id` - The curve identifier to look up
    ///
    /// # Returns
    ///
    /// `Some(curve)` if found, `None` if not found.
    pub fn get(&self, curve_id: &Uuid) -> Option<CachedCurve> {
        let curves = self.curves.read().unwrap();
        curves.get(curve_id).cloned()
    }

    /// Check if a curve exists in the cache.
    ///
    /// # Arguments
    ///
    /// * `curve_id` - The curve identifier to check
    ///
    /// # Returns
    ///
    /// `true` if the curve exists, `false` otherwise.
    pub fn exists(&self, curve_id: &Uuid) -> bool {
        let curves = self.curves.read().unwrap();
        curves.contains_key(curve_id)
    }

    /// Remove a curve from the cache.
    ///
    /// # Arguments
    ///
    /// * `curve_id` - The curve identifier to remove
    ///
    /// # Returns
    ///
    /// The removed curve if it existed, `None` otherwise.
    pub fn remove(&self, curve_id: &Uuid) -> Option<CachedCurve> {
        let mut curves = self.curves.write().unwrap();
        curves.remove(curve_id)
    }

    /// Get the number of curves in the cache.
    pub fn len(&self) -> usize {
        let curves = self.curves.read().unwrap();
        curves.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        let curves = self.curves.read().unwrap();
        curves.is_empty()
    }

    /// Clear all curves from the cache.
    pub fn clear(&self) {
        let mut curves = self.curves.write().unwrap();
        curves.clear();
    }

    /// Remove curves older than the specified age in seconds.
    ///
    /// # Arguments
    ///
    /// * `max_age_seconds` - Maximum age in seconds for curves to keep
    ///
    /// # Returns
    ///
    /// Number of curves removed.
    pub fn cleanup(&self, max_age_seconds: u64) -> usize {
        let mut curves = self.curves.write().unwrap();
        let initial_len = curves.len();
        curves.retain(|_, curve| curve.age_seconds() < max_age_seconds);
        initial_len - curves.len()
    }
}

// =============================================================================
// Greeks Compare Types (Task 4.1: IRS Greeks WebApp Integration)
// =============================================================================

/// Greeks calculation mode for API requests.
///
/// Specifies whether to use Bump-and-Revalue or AAD for Greeks calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GreeksCalculationMode {
    /// Bump-and-Revalue method (finite differences)
    #[default]
    Bump,
    /// Adjoint Algorithmic Differentiation
    Aad,
    /// Both methods for comparison
    Compare,
}

/// Greeks compare request for Bump vs AAD comparison.
///
/// Sent by the client to calculate Greeks using both methods and compare results.
///
/// # Requirements Coverage
///
/// - Requirement 4.2: Bump 法と AAD 法の両方で計算を実行
/// - Requirement 4.6: `/api/greeks/compare` エンドポイント
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GreeksCompareRequest {
    /// Curve identifier from bootstrap response
    pub curve_id: String,
    /// Notional principal amount
    pub notional: f64,
    /// Fixed leg rate (e.g., 0.03 for 3%)
    pub fixed_rate: f64,
    /// Swap tenor in years
    pub tenor_years: f64,
    /// Payment frequency for both legs
    pub payment_frequency: PaymentFrequency,
    /// Bump size in basis points (default: 1)
    #[serde(default = "default_bump_size_bps")]
    pub bump_size_bps: f64,
    /// Include second-order Greeks (Gamma, Vanna, Volga)
    #[serde(default)]
    pub include_second_order: bool,
}

/// Individual Greek value with optional components.
///
/// Represents a single Greek sensitivity value for API responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GreekValue {
    /// Delta (first-order sensitivity to underlying)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<f64>,
    /// Gamma (second-order sensitivity to underlying)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamma: Option<f64>,
    /// Vega (sensitivity to volatility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vega: Option<f64>,
    /// Theta (sensitivity to time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta: Option<f64>,
    /// Rho (sensitivity to interest rate)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rho: Option<f64>,
    /// Vanna (cross-gamma: delta-vol sensitivity)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vanna: Option<f64>,
    /// Volga (second-order vol sensitivity)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volga: Option<f64>,
}

impl GreekValue {
    /// Create a new GreekValue with delta only.
    pub fn with_delta(delta: f64) -> Self {
        Self {
            delta: Some(delta),
            ..Default::default()
        }
    }

    /// Create a new GreekValue with rho (for IRS).
    pub fn with_rho(rho: f64) -> Self {
        Self {
            rho: Some(rho),
            ..Default::default()
        }
    }

    /// Add gamma to this GreekValue.
    pub fn and_gamma(mut self, gamma: f64) -> Self {
        self.gamma = Some(gamma);
        self
    }

    /// Add vega to this GreekValue.
    pub fn and_vega(mut self, vega: f64) -> Self {
        self.vega = Some(vega);
        self
    }
}

/// Greeks result for a single calculation method.
///
/// Contains NPV, DV01, tenor deltas, and timing information.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GreeksMethodResult {
    /// Net present value
    pub npv: f64,
    /// DV01 (total delta per basis point)
    pub dv01: f64,
    /// Delta results for each tenor
    pub tenor_deltas: Vec<DeltaResult>,
    /// Aggregated Greek values
    pub greeks: GreekValue,
    /// Calculation mode used
    pub mode: String,
    /// Timing statistics
    pub timing: TimingStats,
}

/// Difference between Bump and AAD results.
///
/// Contains absolute and relative errors for each Greek.
///
/// # Requirements Coverage
///
/// - Requirement 4.5: 相対誤差・絶対誤差を表形式で表示
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GreeksDiff {
    /// Absolute error in NPV
    pub npv_abs_error: f64,
    /// Relative error in NPV (as percentage)
    pub npv_rel_error_pct: f64,
    /// Absolute error in DV01
    pub dv01_abs_error: f64,
    /// Relative error in DV01 (as percentage)
    pub dv01_rel_error_pct: f64,
    /// Per-tenor delta differences
    pub tenor_diffs: Vec<TenorDiff>,
    /// Maximum absolute error across all tenors
    pub max_abs_error: f64,
    /// Maximum relative error across all tenors (as percentage)
    pub max_rel_error_pct: f64,
}

/// Difference for a single tenor point.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TenorDiff {
    /// Tenor string (e.g., "1Y", "5Y")
    pub tenor: String,
    /// Bump method delta
    pub bump_delta: f64,
    /// AAD method delta
    pub aad_delta: f64,
    /// Absolute difference
    pub abs_diff: f64,
    /// Relative difference (as percentage)
    pub rel_diff_pct: f64,
}

/// Greeks comparison response with both methods' results.
///
/// Returned by the `/api/greeks/compare` endpoint.
///
/// # Requirements Coverage
///
/// - Requirement 4.3: Bump vs AAD の計算結果の差分を並列表示
/// - Requirement 4.4: パフォーマンス比較をチャートで可視化
/// - Requirement 4.6: `/api/greeks/compare` エンドポイント
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GreeksCompareResponse {
    /// Bump method results
    pub bump: GreeksMethodResult,
    /// AAD method results
    pub aad: GreeksMethodResult,
    /// Differences between methods
    pub diff: GreeksDiff,
    /// Timing comparison
    pub timing_comparison: TimingComparison,
    /// Whether results are within acceptable tolerance
    pub within_tolerance: bool,
    /// Tolerance threshold used (relative error percentage)
    pub tolerance_pct: f64,
}

// =============================================================================
// First/Second Order Greeks Types (Task 4.2: IRS Greeks WebApp Integration)
// =============================================================================

/// First-order Greeks request for IRS.
///
/// Sent by the client to calculate first-order Greeks (Delta, Vega, Rho, Theta).
///
/// # Requirements Coverage
///
/// - Requirement 4.1: 一次 Greeks の計算
/// - Requirement 7.1: `/api/greeks/first-order` エンドポイント
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstOrderGreeksRequest {
    /// Curve identifier from bootstrap response
    pub curve_id: String,
    /// Notional principal amount
    pub notional: f64,
    /// Fixed leg rate (e.g., 0.03 for 3%)
    pub fixed_rate: f64,
    /// Swap tenor in years
    pub tenor_years: f64,
    /// Payment frequency for both legs
    pub payment_frequency: PaymentFrequency,
    /// Calculation mode (bump or aad)
    #[serde(default)]
    pub mode: GreeksCalculationMode,
}

/// First-order Greeks response.
///
/// Returns Delta, Vega, Rho, and Theta for the IRS.
///
/// # Requirements Coverage
///
/// - Requirement 4.1: 一次 Greeks の計算
/// - Requirement 7.1: `/api/greeks/first-order` エンドポイント
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstOrderGreeksResponse {
    /// Net present value
    pub npv: f64,
    /// DV01 (delta per basis point)
    pub dv01: f64,
    /// Delta (rate sensitivity)
    pub delta: f64,
    /// Vega (volatility sensitivity, may be 0 for IRS)
    pub vega: f64,
    /// Rho (interest rate sensitivity)
    pub rho: f64,
    /// Theta (time decay)
    pub theta: f64,
    /// Tenor deltas for each curve point
    pub tenor_deltas: Vec<DeltaResult>,
    /// Calculation mode used
    pub mode: String,
    /// Timing statistics
    pub timing: TimingStats,
}

/// Second-order Greeks request for IRS.
///
/// Sent by the client to calculate second-order Greeks (Gamma, Vanna, Volga).
///
/// # Requirements Coverage
///
/// - Requirement 4.1: 二次 Greeks の計算
/// - Requirement 7.2: `/api/greeks/second-order` エンドポイント
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondOrderGreeksRequest {
    /// Curve identifier from bootstrap response
    pub curve_id: String,
    /// Notional principal amount
    pub notional: f64,
    /// Fixed leg rate (e.g., 0.03 for 3%)
    pub fixed_rate: f64,
    /// Swap tenor in years
    pub tenor_years: f64,
    /// Payment frequency for both legs
    pub payment_frequency: PaymentFrequency,
    /// Calculation mode (bump or aad)
    #[serde(default)]
    pub mode: GreeksCalculationMode,
}

/// Second-order Greeks response.
///
/// Returns Gamma, Vanna, and Volga for the IRS.
///
/// # Requirements Coverage
///
/// - Requirement 4.1: 二次 Greeks の計算
/// - Requirement 7.2: `/api/greeks/second-order` エンドポイント
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondOrderGreeksResponse {
    /// Net present value
    pub npv: f64,
    /// Gamma (second derivative to rate)
    pub gamma: f64,
    /// Vanna (cross derivative: delta-vol)
    pub vanna: f64,
    /// Volga (second derivative to volatility)
    pub volga: f64,
    /// Convexity (second derivative of price to yield)
    pub convexity: f64,
    /// Calculation mode used
    pub mode: String,
    /// Timing statistics
    pub timing: TimingStats,
}

/// Validate first-order Greeks request parameters.
pub fn validate_first_order_greeks_request(
    request: &FirstOrderGreeksRequest,
) -> Result<(), ValidationError> {
    if request.notional <= 0.0 || request.notional.is_nan() || request.notional.is_infinite() {
        return Err(ValidationError::NotionalNotPositive {
            notional: request.notional,
        });
    }

    if request.fixed_rate.is_nan()
        || request.fixed_rate.is_infinite()
        || request.fixed_rate < -1.0
        || request.fixed_rate > 1.0
    {
        return Err(ValidationError::FixedRateOutOfRange {
            fixed_rate: request.fixed_rate,
        });
    }

    if request.tenor_years <= 0.0
        || request.tenor_years.is_nan()
        || request.tenor_years.is_infinite()
    {
        return Err(ValidationError::TenorYearsNotPositive {
            tenor_years: request.tenor_years,
        });
    }

    if request.tenor_years > MAX_TENOR_YEARS {
        return Err(ValidationError::TenorYearsExceedsMax {
            tenor_years: request.tenor_years,
            max: MAX_TENOR_YEARS,
        });
    }

    Ok(())
}

/// Validate second-order Greeks request parameters.
pub fn validate_second_order_greeks_request(
    request: &SecondOrderGreeksRequest,
) -> Result<(), ValidationError> {
    if request.notional <= 0.0 || request.notional.is_nan() || request.notional.is_infinite() {
        return Err(ValidationError::NotionalNotPositive {
            notional: request.notional,
        });
    }

    if request.fixed_rate.is_nan()
        || request.fixed_rate.is_infinite()
        || request.fixed_rate < -1.0
        || request.fixed_rate > 1.0
    {
        return Err(ValidationError::FixedRateOutOfRange {
            fixed_rate: request.fixed_rate,
        });
    }

    if request.tenor_years <= 0.0
        || request.tenor_years.is_nan()
        || request.tenor_years.is_infinite()
    {
        return Err(ValidationError::TenorYearsNotPositive {
            tenor_years: request.tenor_years,
        });
    }

    if request.tenor_years > MAX_TENOR_YEARS {
        return Err(ValidationError::TenorYearsExceedsMax {
            tenor_years: request.tenor_years,
            max: MAX_TENOR_YEARS,
        });
    }

    Ok(())
}

// =============================================================================
// Bucket DV01 Types (Task 4.3: IRS Greeks WebApp Integration)
// =============================================================================

/// Standard tenor buckets for Bucket DV01 calculation.
pub const BUCKET_TENORS: [&str; 9] = ["1M", "3M", "6M", "1Y", "2Y", "5Y", "10Y", "20Y", "30Y"];

/// Bucket DV01 request for IRS.
///
/// Sent by the client to calculate tenor-specific DV01 sensitivities.
///
/// # Requirements Coverage
///
/// - Requirement 7.3: `/api/greeks/bucket-dv01` エンドポイント
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketDv01Request {
    /// Curve identifier from bootstrap response
    pub curve_id: String,
    /// Notional principal amount
    pub notional: f64,
    /// Fixed leg rate (e.g., 0.03 for 3%)
    pub fixed_rate: f64,
    /// Swap tenor in years
    pub tenor_years: f64,
    /// Payment frequency for both legs
    pub payment_frequency: PaymentFrequency,
    /// Custom tenor buckets (optional, defaults to BUCKET_TENORS)
    #[serde(default)]
    pub custom_tenors: Option<Vec<String>>,
    /// Include key rate duration calculation
    #[serde(default)]
    pub include_key_rate_duration: bool,
}

/// Individual bucket DV01 result.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketDv01Result {
    /// Tenor string (e.g., "1Y", "5Y")
    pub tenor: String,
    /// DV01 for this bucket
    pub dv01: f64,
    /// Key rate duration (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_rate_duration: Option<f64>,
    /// Percentage of total DV01
    pub pct_of_total: f64,
}

/// Bucket DV01 response.
///
/// Returns tenor-specific DV01 sensitivities for the IRS.
///
/// # Requirements Coverage
///
/// - Requirement 7.3: `/api/greeks/bucket-dv01` エンドポイント
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketDv01Response {
    /// NPV
    pub npv: f64,
    /// Total DV01 (sum of all buckets)
    pub total_dv01: f64,
    /// Per-bucket DV01 results
    pub buckets: Vec<BucketDv01Result>,
    /// Whether buckets sum correctly (consistency check)
    pub buckets_consistent: bool,
    /// Timing statistics
    pub timing: TimingStats,
}

/// Validate Bucket DV01 request parameters.
pub fn validate_bucket_dv01_request(request: &BucketDv01Request) -> Result<(), ValidationError> {
    if request.notional <= 0.0 || request.notional.is_nan() || request.notional.is_infinite() {
        return Err(ValidationError::NotionalNotPositive {
            notional: request.notional,
        });
    }

    if request.fixed_rate.is_nan()
        || request.fixed_rate.is_infinite()
        || request.fixed_rate < -1.0
        || request.fixed_rate > 1.0
    {
        return Err(ValidationError::FixedRateOutOfRange {
            fixed_rate: request.fixed_rate,
        });
    }

    if request.tenor_years <= 0.0
        || request.tenor_years.is_nan()
        || request.tenor_years.is_infinite()
    {
        return Err(ValidationError::TenorYearsNotPositive {
            tenor_years: request.tenor_years,
        });
    }

    if request.tenor_years > MAX_TENOR_YEARS {
        return Err(ValidationError::TenorYearsExceedsMax {
            tenor_years: request.tenor_years,
            max: MAX_TENOR_YEARS,
        });
    }

    Ok(())
}

/// Validate Greeks compare request parameters.
///
/// Returns an error if any parameter is out of valid range.
pub fn validate_greeks_compare_request(
    request: &GreeksCompareRequest,
) -> Result<(), ValidationError> {
    // Validate notional
    if request.notional <= 0.0 || request.notional.is_nan() || request.notional.is_infinite() {
        return Err(ValidationError::NotionalNotPositive {
            notional: request.notional,
        });
    }

    // Validate fixed rate
    if request.fixed_rate.is_nan()
        || request.fixed_rate.is_infinite()
        || request.fixed_rate < -1.0
        || request.fixed_rate > 1.0
    {
        return Err(ValidationError::FixedRateOutOfRange {
            fixed_rate: request.fixed_rate,
        });
    }

    // Validate tenor years
    if request.tenor_years <= 0.0
        || request.tenor_years.is_nan()
        || request.tenor_years.is_infinite()
    {
        return Err(ValidationError::TenorYearsNotPositive {
            tenor_years: request.tenor_years,
        });
    }

    if request.tenor_years > MAX_TENOR_YEARS {
        return Err(ValidationError::TenorYearsExceedsMax {
            tenor_years: request.tenor_years,
            max: MAX_TENOR_YEARS,
        });
    }

    // Validate bump size
    if request.bump_size_bps <= 0.0
        || request.bump_size_bps.is_nan()
        || request.bump_size_bps.is_infinite()
    {
        return Err(ValidationError::BumpSizeNotPositive {
            bump_size_bps: request.bump_size_bps,
        });
    }

    Ok(())
}

// =============================================================================
// Greeks Visualisation Types (Task 5.1, 5.2: Greeks Heatmap & Timeseries)
// =============================================================================

/// Greek type selector for visualisation APIs.
///
/// Specifies which Greek to visualise in heatmaps or timeseries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GreekType {
    /// Delta: first-order sensitivity to underlying
    #[default]
    Delta,
    /// Gamma: second-order sensitivity to underlying
    Gamma,
    /// Vega: sensitivity to volatility
    Vega,
    /// Theta: time decay
    Theta,
    /// Rho: sensitivity to interest rate
    Rho,
    /// Vanna: cross-gamma (delta-vol sensitivity)
    Vanna,
    /// Volga: second-order vol sensitivity
    Volga,
}

impl std::fmt::Display for GreekType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GreekType::Delta => write!(f, "delta"),
            GreekType::Gamma => write!(f, "gamma"),
            GreekType::Vega => write!(f, "vega"),
            GreekType::Theta => write!(f, "theta"),
            GreekType::Rho => write!(f, "rho"),
            GreekType::Vanna => write!(f, "vanna"),
            GreekType::Volga => write!(f, "volga"),
        }
    }
}

/// Greeks heatmap request parameters.
///
/// Query parameters for the `/api/greeks/heatmap` endpoint.
///
/// # Requirements Coverage
///
/// - Requirement 5.1: テナー × ストライクの二次元ヒートマップ
/// - Requirement 5.3: `/api/greeks/heatmap` エンドポイント
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GreeksHeatmapRequest {
    /// Greek type to visualise (default: delta)
    #[serde(default)]
    pub greek_type: GreekType,
    /// Spot price (default: 100.0)
    #[serde(default = "default_spot")]
    pub spot: f64,
    /// Risk-free rate (default: 0.05)
    #[serde(default = "default_rate")]
    pub rate: f64,
    /// Volatility (default: 0.20)
    #[serde(default = "default_volatility")]
    pub volatility: f64,
    /// Option type: "call" or "put" (default: "call")
    #[serde(default)]
    pub option_type: OptionType,
}

fn default_spot() -> f64 {
    100.0
}

fn default_rate() -> f64 {
    0.05
}

fn default_volatility() -> f64 {
    0.20
}

impl Default for GreeksHeatmapRequest {
    fn default() -> Self {
        Self {
            greek_type: GreekType::default(),
            spot: default_spot(),
            rate: default_rate(),
            volatility: default_volatility(),
            option_type: OptionType::Call,
        }
    }
}

/// Greeks heatmap response (D3.js compatible).
///
/// Returns a 2D matrix of Greek values across tenor and strike dimensions.
///
/// # Requirements Coverage
///
/// - Requirement 5.1: D3.js 互換 JSON フォーマット
/// - Requirement 5.3: `/api/greeks/heatmap` エンドポイント
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GreeksHeatmapResponse {
    /// X-axis labels (tenors in years, e.g., ["0.25", "0.5", "1.0", "2.0"])
    pub x_axis: Vec<String>,
    /// Y-axis labels (strikes as percentages of spot, e.g., ["80%", "90%", "100%", "110%", "120%"])
    pub y_axis: Vec<String>,
    /// 2D matrix of values: values[y][x] corresponds to y_axis[y] and x_axis[x]
    pub values: Vec<Vec<f64>>,
    /// Greek type being visualised
    pub greek_type: String,
    /// Spot price used
    pub spot: f64,
    /// Risk-free rate used
    pub rate: f64,
    /// Volatility used
    pub volatility: f64,
    /// Option type used
    pub option_type: String,
    /// Minimum value in the matrix (for colour scale)
    pub min_value: f64,
    /// Maximum value in the matrix (for colour scale)
    pub max_value: f64,
}

/// Greeks timeseries request parameters.
///
/// Query parameters for the `/api/greeks/timeseries` endpoint.
///
/// # Requirements Coverage
///
/// - Requirement 5.2: Greeks の時間推移を折れ線グラフで表示
/// - Requirement 5.3: `/api/greeks/timeseries` エンドポイント
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GreeksTimeseriesRequest {
    /// Greek types to include in the timeseries
    #[serde(default = "default_greek_types")]
    pub greek_types: Vec<GreekType>,
    /// Spot price (default: 100.0)
    #[serde(default = "default_spot")]
    pub spot: f64,
    /// Strike price (default: 100.0)
    #[serde(default = "default_strike")]
    pub strike: f64,
    /// Risk-free rate (default: 0.05)
    #[serde(default = "default_rate")]
    pub rate: f64,
    /// Volatility (default: 0.20)
    #[serde(default = "default_volatility")]
    pub volatility: f64,
    /// Option type: "call" or "put" (default: "call")
    #[serde(default)]
    pub option_type: OptionType,
    /// Time horizon in years (default: 1.0)
    #[serde(default = "default_time_horizon")]
    pub time_horizon: f64,
    /// Number of time points (default: 50)
    #[serde(default = "default_num_points")]
    pub num_points: usize,
}

fn default_greek_types() -> Vec<GreekType> {
    vec![GreekType::Delta, GreekType::Gamma, GreekType::Theta]
}

fn default_strike() -> f64 {
    100.0
}

fn default_time_horizon() -> f64 {
    1.0
}

fn default_num_points() -> usize {
    50
}

impl Default for GreeksTimeseriesRequest {
    fn default() -> Self {
        Self {
            greek_types: default_greek_types(),
            spot: default_spot(),
            strike: default_strike(),
            rate: default_rate(),
            volatility: default_volatility(),
            option_type: OptionType::Call,
            time_horizon: default_time_horizon(),
            num_points: default_num_points(),
        }
    }
}

/// Single timeseries data series.
///
/// Contains timestamps and values for one Greek.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeseriesSeries {
    /// Greek type for this series
    pub greek_type: String,
    /// Values at each timestamp
    pub values: Vec<f64>,
}

/// Greeks timeseries response.
///
/// Returns time-series data for multiple Greeks.
///
/// # Requirements Coverage
///
/// - Requirement 5.2: 複数 Greeks の重ね合わせ表示
/// - Requirement 5.3: `/api/greeks/timeseries` エンドポイント
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GreeksTimeseriesResponse {
    /// Timestamps (time to expiry in days)
    pub timestamps: Vec<f64>,
    /// Series data for each Greek
    pub series: Vec<TimeseriesSeries>,
    /// Spot price used
    pub spot: f64,
    /// Strike price used
    pub strike: f64,
    /// Risk-free rate used
    pub rate: f64,
    /// Volatility used
    pub volatility: f64,
    /// Option type used
    pub option_type: String,
}

// =============================================================================
// Scenario Analysis Types (Task 6: Scenario Analysis UI)
// =============================================================================

/// Preset scenario type for stress testing.
///
/// Represents predefined market scenarios for risk analysis.
///
/// # Requirements Coverage
///
/// - Requirement 6.2: プリセットシナリオ（Rate Shock, Vol Spike, FX Crisis 等）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetScenarioTypeApi {
    /// Interest rate +1bp parallel shift
    RateUp1bp,
    /// Interest rate +10bp parallel shift
    RateUp10bp,
    /// Interest rate +100bp parallel shift
    RateUp100bp,
    /// Interest rate -1bp parallel shift
    RateDown1bp,
    /// Interest rate -10bp parallel shift
    RateDown10bp,
    /// Interest rate -100bp parallel shift
    RateDown100bp,
    /// Curve steepening (short rates down, long rates up)
    CurveSteepen,
    /// Curve flattening (short rates up, long rates down)
    CurveFlatten,
    /// Butterfly (middle rates spike)
    Butterfly,
    /// Credit spread +50bp
    CreditWiden50bp,
    /// Credit spread +100bp
    CreditWiden100bp,
    /// Equity -10%
    EquityDown10Pct,
    /// Equity -20%
    EquityDown20Pct,
    /// FX -10% (base currency weakens)
    FxDown10Pct,
    /// Volatility +5 vega points
    VolUp5Pts,
}

impl PresetScenarioTypeApi {
    /// Get human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::RateUp1bp => "IR +1bp",
            Self::RateUp10bp => "IR +10bp",
            Self::RateUp100bp => "IR +100bp",
            Self::RateDown1bp => "IR -1bp",
            Self::RateDown10bp => "IR -10bp",
            Self::RateDown100bp => "IR -100bp",
            Self::CurveSteepen => "Curve Steepen",
            Self::CurveFlatten => "Curve Flatten",
            Self::Butterfly => "Butterfly",
            Self::CreditWiden50bp => "Credit +50bp",
            Self::CreditWiden100bp => "Credit +100bp",
            Self::EquityDown10Pct => "Equity -10%",
            Self::EquityDown20Pct => "Equity -20%",
            Self::FxDown10Pct => "FX -10%",
            Self::VolUp5Pts => "Vol +5pts",
        }
    }

    /// Get description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::RateUp1bp => "Parallel interest rate shift +1 basis point",
            Self::RateUp10bp => "Parallel interest rate shift +10 basis points",
            Self::RateUp100bp => "Parallel interest rate shift +100 basis points",
            Self::RateDown1bp => "Parallel interest rate shift -1 basis point",
            Self::RateDown10bp => "Parallel interest rate shift -10 basis points",
            Self::RateDown100bp => "Parallel interest rate shift -100 basis points",
            Self::CurveSteepen => "Short rates -25bp, long rates +25bp",
            Self::CurveFlatten => "Short rates +25bp, long rates -25bp",
            Self::Butterfly => "Wings -10bp, belly +20bp",
            Self::CreditWiden50bp => "Credit spread widening +50 basis points",
            Self::CreditWiden100bp => "Credit spread widening +100 basis points",
            Self::EquityDown10Pct => "Equity prices decline 10%",
            Self::EquityDown20Pct => "Equity prices decline 20%",
            Self::FxDown10Pct => "Base currency weakens 10%",
            Self::VolUp5Pts => "Implied volatility increases 5 percentage points",
        }
    }

    /// Get category.
    pub fn category(&self) -> &'static str {
        match self {
            Self::RateUp1bp
            | Self::RateUp10bp
            | Self::RateUp100bp
            | Self::RateDown1bp
            | Self::RateDown10bp
            | Self::RateDown100bp => "rate",
            Self::CurveSteepen | Self::CurveFlatten | Self::Butterfly => "curve",
            Self::CreditWiden50bp | Self::CreditWiden100bp => "credit",
            Self::EquityDown10Pct | Self::EquityDown20Pct => "equity",
            Self::FxDown10Pct => "fx",
            Self::VolUp5Pts => "volatility",
        }
    }

    /// Get all available preset scenario types.
    pub fn all() -> Vec<Self> {
        vec![
            Self::RateUp1bp,
            Self::RateUp10bp,
            Self::RateUp100bp,
            Self::RateDown1bp,
            Self::RateDown10bp,
            Self::RateDown100bp,
            Self::CurveSteepen,
            Self::CurveFlatten,
            Self::Butterfly,
            Self::CreditWiden50bp,
            Self::CreditWiden100bp,
            Self::EquityDown10Pct,
            Self::EquityDown20Pct,
            Self::FxDown10Pct,
            Self::VolUp5Pts,
        ]
    }
}

/// Preset scenario information for API response.
///
/// Contains all information about a preset scenario including its
/// identifier, name, description, and parameters.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetScenarioInfo {
    /// Scenario type identifier
    pub scenario_type: PresetScenarioTypeApi,
    /// Human-readable name
    pub name: String,
    /// Description of what the scenario does
    pub description: String,
    /// Category (rate, curve, credit, equity, fx, volatility)
    pub category: String,
    /// Shift amount in basis points (for rate scenarios) or percentage (for others)
    pub shift_amount: f64,
    /// Unit of the shift amount
    pub shift_unit: String,
}

impl PresetScenarioInfo {
    /// Create a new preset scenario info from a scenario type.
    pub fn from_type(scenario_type: PresetScenarioTypeApi) -> Self {
        let (shift_amount, shift_unit) = match scenario_type {
            PresetScenarioTypeApi::RateUp1bp => (1.0, "bp"),
            PresetScenarioTypeApi::RateUp10bp => (10.0, "bp"),
            PresetScenarioTypeApi::RateUp100bp => (100.0, "bp"),
            PresetScenarioTypeApi::RateDown1bp => (-1.0, "bp"),
            PresetScenarioTypeApi::RateDown10bp => (-10.0, "bp"),
            PresetScenarioTypeApi::RateDown100bp => (-100.0, "bp"),
            PresetScenarioTypeApi::CurveSteepen => (25.0, "bp"),
            PresetScenarioTypeApi::CurveFlatten => (25.0, "bp"),
            PresetScenarioTypeApi::Butterfly => (20.0, "bp"),
            PresetScenarioTypeApi::CreditWiden50bp => (50.0, "bp"),
            PresetScenarioTypeApi::CreditWiden100bp => (100.0, "bp"),
            PresetScenarioTypeApi::EquityDown10Pct => (-10.0, "%"),
            PresetScenarioTypeApi::EquityDown20Pct => (-20.0, "%"),
            PresetScenarioTypeApi::FxDown10Pct => (-10.0, "%"),
            PresetScenarioTypeApi::VolUp5Pts => (5.0, "pts"),
        };

        Self {
            scenario_type,
            name: scenario_type.name().to_string(),
            description: scenario_type.description().to_string(),
            category: scenario_type.category().to_string(),
            shift_amount,
            shift_unit: shift_unit.to_string(),
        }
    }
}

/// Response for GET /api/scenarios/presets endpoint.
///
/// Returns all available preset scenarios.
///
/// # Requirements Coverage
///
/// - Requirement 6.2: プリセットシナリオの選択機能
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioPresetsResponse {
    /// List of all available preset scenarios
    pub presets: Vec<PresetScenarioInfo>,
    /// Total number of presets
    pub count: usize,
    /// Grouped presets by category
    pub by_category: HashMap<String, Vec<PresetScenarioInfo>>,
}

impl ScenarioPresetsResponse {
    /// Create response with all preset scenarios.
    pub fn new() -> Self {
        let all_types = PresetScenarioTypeApi::all();
        let presets: Vec<PresetScenarioInfo> = all_types
            .into_iter()
            .map(PresetScenarioInfo::from_type)
            .collect();

        let mut by_category: HashMap<String, Vec<PresetScenarioInfo>> = HashMap::new();
        for preset in &presets {
            by_category
                .entry(preset.category.clone())
                .or_default()
                .push(preset.clone());
        }

        let count = presets.len();

        Self {
            presets,
            count,
            by_category,
        }
    }
}

impl Default for ScenarioPresetsResponse {
    fn default() -> Self {
        Self::new()
    }
}

/// Risk factor shift specification for custom scenarios.
///
/// Allows users to define custom shifts for scenario analysis.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskFactorShiftApi {
    /// Type of risk factor (rate, credit, equity, fx, volatility)
    pub factor_type: String,
    /// Identifier pattern (e.g., "*" for all, "USD.*" for USD factors)
    #[serde(default = "default_pattern")]
    pub pattern: String,
    /// Shift amount
    pub shift_amount: f64,
    /// Shift type (parallel, relative, absolute)
    #[serde(default = "default_shift_type")]
    pub shift_type: String,
}

fn default_pattern() -> String {
    "*".to_string()
}

fn default_shift_type() -> String {
    "parallel".to_string()
}

/// Request for POST /api/scenarios/run endpoint.
///
/// Execute a scenario and calculate P&L impact.
///
/// # Requirements Coverage
///
/// - Requirement 6.1: リスクファクターシフト量を変更して再計算
/// - Requirement 6.3: カスタムシナリオを定義
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioRunRequest {
    /// Curve ID for pricing (from bootstrap)
    pub curve_id: String,
    /// Notional amount
    pub notional: f64,
    /// Fixed rate for IRS
    pub fixed_rate: f64,
    /// Tenor in years
    pub tenor_years: f64,
    /// Payment frequency
    pub payment_frequency: PaymentFrequency,
    /// Preset scenario type (if using preset)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_scenario: Option<PresetScenarioTypeApi>,
    /// Custom risk factor shifts (if using custom scenario)
    #[serde(default)]
    pub custom_shifts: Vec<RiskFactorShiftApi>,
    /// Scenario name (for custom scenarios)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_name: Option<String>,
}

/// P&L result for a single scenario.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioPnlResult {
    /// Scenario name
    pub scenario_name: String,
    /// Base value before scenario
    pub base_value: f64,
    /// Stressed value after scenario
    pub stressed_value: f64,
    /// P&L (stressed - base)
    pub pnl: f64,
    /// P&L as percentage of base
    pub pnl_pct: f64,
    /// Whether this is a loss
    pub is_loss: bool,
}

impl ScenarioPnlResult {
    /// Create a new P&L result.
    pub fn new(scenario_name: impl Into<String>, base_value: f64, stressed_value: f64) -> Self {
        let pnl = stressed_value - base_value;
        let pnl_pct = if base_value.abs() > 1e-10 {
            (pnl / base_value.abs()) * 100.0
        } else {
            0.0
        };
        Self {
            scenario_name: scenario_name.into(),
            base_value,
            stressed_value,
            pnl,
            pnl_pct,
            is_loss: pnl < 0.0,
        }
    }
}

/// Response for POST /api/scenarios/run endpoint.
///
/// Contains the P&L result and timing information.
///
/// # Requirements Coverage
///
/// - Requirement 6.4: シナリオ間の PnL 比較
/// - Requirement 6.5: プログレスインジケータ（非同期ジョブ用）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioRunResponse {
    /// Scenario result
    pub result: ScenarioPnlResult,
    /// Applied shifts summary
    pub applied_shifts: Vec<AppliedShiftInfo>,
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
    /// Job ID (for long-running calculations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

/// Information about an applied shift.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedShiftInfo {
    /// Factor type
    pub factor_type: String,
    /// Pattern
    pub pattern: String,
    /// Shift amount
    pub shift_amount: f64,
    /// Shift type
    pub shift_type: String,
}

/// Request for scenario comparison.
///
/// Run multiple scenarios and compare P&L results.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioCompareRequest {
    /// Curve ID for pricing
    pub curve_id: String,
    /// Notional amount
    pub notional: f64,
    /// Fixed rate for IRS
    pub fixed_rate: f64,
    /// Tenor in years
    pub tenor_years: f64,
    /// Payment frequency
    pub payment_frequency: PaymentFrequency,
    /// List of preset scenarios to compare
    pub scenarios: Vec<PresetScenarioTypeApi>,
}

/// Response for scenario comparison.
///
/// Contains P&L results for all compared scenarios.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioCompareResponse {
    /// Results for each scenario
    pub results: Vec<ScenarioPnlResult>,
    /// Worst case scenario
    pub worst_case: Option<ScenarioPnlResult>,
    /// Best case scenario
    pub best_case: Option<ScenarioPnlResult>,
    /// Total processing time in milliseconds
    pub total_processing_time_ms: f64,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // InstrumentType Serialisation Tests
    // =========================================================================

    mod instrument_type_tests {
        use super::*;

        #[test]
        fn test_serialize_equity_vanilla_option() {
            let t = InstrumentType::EquityVanillaOption;
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, "\"equity_vanilla_option\"");
        }

        #[test]
        fn test_serialize_fx_option() {
            let t = InstrumentType::FxOption;
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, "\"fx_option\"");
        }

        #[test]
        fn test_serialize_irs() {
            let t = InstrumentType::Irs;
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, "\"irs\"");
        }

        #[test]
        fn test_deserialize_instrument_types() {
            let eq: InstrumentType = serde_json::from_str("\"equity_vanilla_option\"").unwrap();
            assert_eq!(eq, InstrumentType::EquityVanillaOption);

            let fx: InstrumentType = serde_json::from_str("\"fx_option\"").unwrap();
            assert_eq!(fx, InstrumentType::FxOption);

            let irs: InstrumentType = serde_json::from_str("\"irs\"").unwrap();
            assert_eq!(irs, InstrumentType::Irs);
        }
    }

    // =========================================================================
    // OptionType Serialisation Tests
    // =========================================================================

    mod option_type_tests {
        use super::*;

        #[test]
        fn test_serialize_call() {
            let t = OptionType::Call;
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, "\"call\"");
        }

        #[test]
        fn test_serialize_put() {
            let t = OptionType::Put;
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, "\"put\"");
        }

        #[test]
        fn test_deserialize_option_types() {
            let call: OptionType = serde_json::from_str("\"call\"").unwrap();
            assert_eq!(call, OptionType::Call);

            let put: OptionType = serde_json::from_str("\"put\"").unwrap();
            assert_eq!(put, OptionType::Put);
        }
    }

    // =========================================================================
    // EquityOptionParams Tests
    // =========================================================================

    mod equity_option_params_tests {
        use super::*;

        fn sample_equity_params() -> EquityOptionParams {
            EquityOptionParams {
                spot: 100.0,
                strike: 105.0,
                expiry_years: 1.0,
                volatility: 0.2,
                rate: 0.05,
                option_type: OptionType::Call,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let params = sample_equity_params();
            let json = serde_json::to_string(&params).unwrap();

            assert!(json.contains("\"spot\":100"));
            assert!(json.contains("\"strike\":105"));
            assert!(json.contains("\"expiryYears\":1"));
            assert!(json.contains("\"volatility\":0.2"));
            assert!(json.contains("\"rate\":0.05"));
            assert!(json.contains("\"optionType\":\"call\""));
        }

        #[test]
        fn test_deserialize_camel_case() {
            let json = r#"{
                "spot": 100.0,
                "strike": 105.0,
                "expiryYears": 1.0,
                "volatility": 0.2,
                "rate": 0.05,
                "optionType": "call"
            }"#;

            let params: EquityOptionParams = serde_json::from_str(json).unwrap();
            assert!((params.spot - 100.0).abs() < 1e-10);
            assert!((params.strike - 105.0).abs() < 1e-10);
            assert!((params.expiry_years - 1.0).abs() < 1e-10);
            assert!((params.volatility - 0.2).abs() < 1e-10);
            assert!((params.rate - 0.05).abs() < 1e-10);
            assert_eq!(params.option_type, OptionType::Call);
        }
    }

    // =========================================================================
    // FxOptionParams Tests
    // =========================================================================

    mod fx_option_params_tests {
        use super::*;

        fn sample_fx_params() -> FxOptionParams {
            FxOptionParams {
                spot: 1.10,
                strike: 1.15,
                expiry_years: 0.5,
                domestic_rate: 0.05,
                foreign_rate: 0.02,
                volatility: 0.15,
                option_type: OptionType::Put,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let params = sample_fx_params();
            let json = serde_json::to_string(&params).unwrap();

            assert!(json.contains("\"spot\":1.1"));
            assert!(json.contains("\"strike\":1.15"));
            assert!(json.contains("\"expiryYears\":0.5"));
            assert!(json.contains("\"domesticRate\":0.05"));
            assert!(json.contains("\"foreignRate\":0.02"));
            assert!(json.contains("\"volatility\":0.15"));
            assert!(json.contains("\"optionType\":\"put\""));
        }

        #[test]
        fn test_deserialize_camel_case() {
            let json = r#"{
                "spot": 1.10,
                "strike": 1.15,
                "expiryYears": 0.5,
                "domesticRate": 0.05,
                "foreignRate": 0.02,
                "volatility": 0.15,
                "optionType": "put"
            }"#;

            let params: FxOptionParams = serde_json::from_str(json).unwrap();
            assert!((params.spot - 1.10).abs() < 1e-10);
            assert!((params.domestic_rate - 0.05).abs() < 1e-10);
            assert!((params.foreign_rate - 0.02).abs() < 1e-10);
            assert_eq!(params.option_type, OptionType::Put);
        }
    }

    // =========================================================================
    // IrsParams Tests
    // =========================================================================

    mod irs_params_tests {
        use super::*;

        fn sample_irs_params() -> IrsParams {
            IrsParams {
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let params = sample_irs_params();
            let json = serde_json::to_string(&params).unwrap();

            assert!(json.contains("\"notional\":10000000"));
            assert!(json.contains("\"fixedRate\":0.03"));
            assert!(json.contains("\"tenorYears\":5"));
        }

        #[test]
        fn test_deserialize_camel_case() {
            let json = r#"{
                "notional": 10000000.0,
                "fixedRate": 0.03,
                "tenorYears": 5.0
            }"#;

            let params: IrsParams = serde_json::from_str(json).unwrap();
            assert!((params.notional - 10_000_000.0).abs() < 1e-10);
            assert!((params.fixed_rate - 0.03).abs() < 1e-10);
            assert!((params.tenor_years - 5.0).abs() < 1e-10);
        }
    }

    // =========================================================================
    // InstrumentParams (Untagged Union) Tests
    // =========================================================================

    mod instrument_params_tests {
        use super::*;

        #[test]
        fn test_deserialize_equity_option() {
            let json = r#"{
                "spot": 100.0,
                "strike": 105.0,
                "expiryYears": 1.0,
                "volatility": 0.2,
                "rate": 0.05,
                "optionType": "call"
            }"#;

            let params: InstrumentParams = serde_json::from_str(json).unwrap();
            match params {
                InstrumentParams::EquityOption(p) => {
                    assert!((p.spot - 100.0).abs() < 1e-10);
                }
                _ => panic!("Expected EquityOption variant"),
            }
        }

        #[test]
        fn test_deserialize_fx_option() {
            let json = r#"{
                "spot": 1.10,
                "strike": 1.15,
                "expiryYears": 0.5,
                "domesticRate": 0.05,
                "foreignRate": 0.02,
                "volatility": 0.15,
                "optionType": "put"
            }"#;

            let params: InstrumentParams = serde_json::from_str(json).unwrap();
            match params {
                InstrumentParams::FxOption(p) => {
                    assert!((p.domestic_rate - 0.05).abs() < 1e-10);
                    assert!((p.foreign_rate - 0.02).abs() < 1e-10);
                }
                _ => panic!("Expected FxOption variant"),
            }
        }

        #[test]
        fn test_deserialize_irs() {
            let json = r#"{
                "notional": 10000000.0,
                "fixedRate": 0.03,
                "tenorYears": 5.0
            }"#;

            let params: InstrumentParams = serde_json::from_str(json).unwrap();
            match params {
                InstrumentParams::Irs(p) => {
                    assert!((p.notional - 10_000_000.0).abs() < 1e-10);
                }
                _ => panic!("Expected Irs variant"),
            }
        }
    }

    // =========================================================================
    // PricingRequest Tests
    // =========================================================================

    mod pricing_request_tests {
        use super::*;

        #[test]
        fn test_deserialize_full_request() {
            let json = r#"{
                "instrumentType": "equity_vanilla_option",
                "params": {
                    "spot": 100.0,
                    "strike": 105.0,
                    "expiryYears": 1.0,
                    "volatility": 0.2,
                    "rate": 0.05,
                    "optionType": "call"
                },
                "computeGreeks": true
            }"#;

            let request: PricingRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.instrument_type, InstrumentType::EquityVanillaOption);
            assert!(request.compute_greeks);
            assert!(request.market_data.is_none());
        }

        #[test]
        fn test_deserialize_with_market_data() {
            let json = r#"{
                "instrumentType": "fx_option",
                "params": {
                    "spot": 1.10,
                    "strike": 1.15,
                    "expiryYears": 0.5,
                    "domesticRate": 0.05,
                    "foreignRate": 0.02,
                    "volatility": 0.15,
                    "optionType": "put"
                },
                "marketData": {
                    "source": "demo"
                },
                "computeGreeks": false
            }"#;

            let request: PricingRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.instrument_type, InstrumentType::FxOption);
            assert!(!request.compute_greeks);
            assert!(request.market_data.is_some());
            assert_eq!(request.market_data.unwrap().source, MarketDataSource::Demo);
        }

        #[test]
        fn test_deserialize_irs_request() {
            let json = r#"{
                "instrumentType": "irs",
                "params": {
                    "notional": 10000000.0,
                    "fixedRate": 0.03,
                    "tenorYears": 5.0
                },
                "computeGreeks": true
            }"#;

            let request: PricingRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.instrument_type, InstrumentType::Irs);
            assert!(request.compute_greeks);
        }
    }

    // =========================================================================
    // GreeksData Tests
    // =========================================================================

    mod greeks_data_tests {
        use super::*;

        fn sample_greeks() -> GreeksData {
            GreeksData {
                delta: 0.55,
                gamma: 0.02,
                vega: 0.35,
                theta: -0.05,
                rho: 0.12,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let greeks = sample_greeks();
            let json = serde_json::to_string(&greeks).unwrap();

            assert!(json.contains("\"delta\":0.55"));
            assert!(json.contains("\"gamma\":0.02"));
            assert!(json.contains("\"vega\":0.35"));
            assert!(json.contains("\"theta\":-0.05"));
            assert!(json.contains("\"rho\":0.12"));
        }

        #[test]
        fn test_deserialize_greeks() {
            let json = r#"{
                "delta": 0.55,
                "gamma": 0.02,
                "vega": 0.35,
                "theta": -0.05,
                "rho": 0.12
            }"#;

            let greeks: GreeksData = serde_json::from_str(json).unwrap();
            assert!((greeks.delta - 0.55).abs() < 1e-10);
            assert!((greeks.theta - (-0.05)).abs() < 1e-10);
        }
    }

    // =========================================================================
    // PricingResponse Tests
    // =========================================================================

    mod pricing_response_tests {
        use super::*;

        #[test]
        fn test_serialize_without_greeks() {
            let response = PricingResponse {
                calculation_id: "calc-001".to_string(),
                instrument_type: InstrumentType::Irs,
                pv: 125_000.5,
                greeks: None,
                timestamp: 1700000000000,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"calculationId\":\"calc-001\""));
            assert!(json.contains("\"instrumentType\":\"irs\""));
            assert!(json.contains("\"pv\":125000.5"));
            assert!(json.contains("\"greeks\":null"));
            assert!(json.contains("\"timestamp\":1700000000000"));
        }

        #[test]
        fn test_serialize_with_greeks() {
            let greeks = GreeksData {
                delta: 0.55,
                gamma: 0.02,
                vega: 0.35,
                theta: -0.05,
                rho: 0.12,
            };

            let response = PricingResponse {
                calculation_id: "calc-002".to_string(),
                instrument_type: InstrumentType::EquityVanillaOption,
                pv: 10.25,
                greeks: Some(greeks),
                timestamp: 1700000000000,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"calculationId\":\"calc-002\""));
            assert!(json.contains("\"instrumentType\":\"equity_vanilla_option\""));
            assert!(json.contains("\"delta\":0.55"));
        }
    }

    // =========================================================================
    // PricingErrorResponse Tests
    // =========================================================================

    mod pricing_error_response_tests {
        use super::*;

        #[test]
        fn test_serialize_validation_error() {
            let error = PricingErrorResponse {
                error_type: "ValidationError".to_string(),
                message: "Strike must be positive".to_string(),
                field: Some("strike".to_string()),
            };

            let json = serde_json::to_string(&error).unwrap();
            assert!(json.contains("\"errorType\":\"ValidationError\""));
            assert!(json.contains("\"message\":\"Strike must be positive\""));
            assert!(json.contains("\"field\":\"strike\""));
        }

        #[test]
        fn test_serialize_pricing_error() {
            let error = PricingErrorResponse {
                error_type: "PricingError".to_string(),
                message: "Numerical instability in pricing".to_string(),
                field: None,
            };

            let json = serde_json::to_string(&error).unwrap();
            assert!(json.contains("\"errorType\":\"PricingError\""));
            assert!(json.contains("\"field\":null"));
        }
    }

    // =========================================================================
    // MarketDataConfig Tests
    // =========================================================================

    mod market_data_config_tests {
        use super::*;

        #[test]
        fn test_serialize_demo_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Demo,
                curve_rate: None,
                volatility: None,
            };

            let json = serde_json::to_string(&config).unwrap();
            assert!(json.contains("\"source\":\"demo\""));
        }

        #[test]
        fn test_serialize_custom_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Custom,
                curve_rate: Some(0.05),
                volatility: Some(0.2),
            };

            let json = serde_json::to_string(&config).unwrap();
            assert!(json.contains("\"source\":\"custom\""));
            assert!(json.contains("\"curveRate\":0.05"));
            assert!(json.contains("\"volatility\":0.2"));
        }

        #[test]
        fn test_deserialize_market_data_config() {
            let json = r#"{
                "source": "custom",
                "curveRate": 0.04,
                "volatility": 0.25
            }"#;

            let config: MarketDataConfig = serde_json::from_str(json).unwrap();
            assert_eq!(config.source, MarketDataSource::Custom);
            assert!((config.curve_rate.unwrap() - 0.04).abs() < 1e-10);
            assert!((config.volatility.unwrap() - 0.25).abs() < 1e-10);
        }
    }

    // =========================================================================
    // DemoMarketData Tests (Task 1.2)
    // =========================================================================

    mod demo_market_data_tests {
        use super::*;

        #[test]
        fn test_demo_market_data_new() {
            let data = DemoMarketData::new();
            assert!((data.curve_rate - 0.05).abs() < 1e-10);
            assert!((data.volatility - 0.20).abs() < 1e-10);
        }

        #[test]
        fn test_demo_market_data_default() {
            let data = DemoMarketData::default();
            assert_eq!(data, DemoMarketData::new());
        }

        #[test]
        fn test_demo_market_data_with_curve_rate() {
            let data = DemoMarketData::new().with_curve_rate(0.03);
            assert!((data.curve_rate - 0.03).abs() < 1e-10);
            assert!((data.volatility - 0.20).abs() < 1e-10);
        }

        #[test]
        fn test_demo_market_data_with_volatility() {
            let data = DemoMarketData::new().with_volatility(0.30);
            assert!((data.curve_rate - 0.05).abs() < 1e-10);
            assert!((data.volatility - 0.30).abs() < 1e-10);
        }

        #[test]
        fn test_demo_market_data_builder_chain() {
            let data = DemoMarketData::new()
                .with_curve_rate(0.04)
                .with_volatility(0.25);
            assert!((data.curve_rate - 0.04).abs() < 1e-10);
            assert!((data.volatility - 0.25).abs() < 1e-10);
        }

        #[test]
        fn test_get_curve_rate_none_config() {
            let rate = DemoMarketData::get_curve_rate(None);
            assert!((rate - 0.05).abs() < 1e-10);
        }

        #[test]
        fn test_get_curve_rate_demo_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Demo,
                curve_rate: Some(0.10),
                volatility: Some(0.40),
            };
            let rate = DemoMarketData::get_curve_rate(Some(&config));
            // Demo source should use default, ignoring custom values
            assert!((rate - 0.05).abs() < 1e-10);
        }

        #[test]
        fn test_get_curve_rate_custom_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Custom,
                curve_rate: Some(0.08),
                volatility: Some(0.30),
            };
            let rate = DemoMarketData::get_curve_rate(Some(&config));
            assert!((rate - 0.08).abs() < 1e-10);
        }

        #[test]
        fn test_get_curve_rate_custom_source_no_value() {
            let config = MarketDataConfig {
                source: MarketDataSource::Custom,
                curve_rate: None,
                volatility: Some(0.30),
            };
            let rate = DemoMarketData::get_curve_rate(Some(&config));
            // Falls back to default when Custom but no value provided
            assert!((rate - 0.05).abs() < 1e-10);
        }

        #[test]
        fn test_get_volatility_none_config() {
            let vol = DemoMarketData::get_volatility(None);
            assert!((vol - 0.20).abs() < 1e-10);
        }

        #[test]
        fn test_get_volatility_demo_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Demo,
                curve_rate: Some(0.10),
                volatility: Some(0.40),
            };
            let vol = DemoMarketData::get_volatility(Some(&config));
            // Demo source should use default, ignoring custom values
            assert!((vol - 0.20).abs() < 1e-10);
        }

        #[test]
        fn test_get_volatility_custom_source() {
            let config = MarketDataConfig {
                source: MarketDataSource::Custom,
                curve_rate: Some(0.08),
                volatility: Some(0.35),
            };
            let vol = DemoMarketData::get_volatility(Some(&config));
            assert!((vol - 0.35).abs() < 1e-10);
        }

        #[test]
        fn test_get_volatility_custom_source_no_value() {
            let config = MarketDataConfig {
                source: MarketDataSource::Custom,
                curve_rate: Some(0.08),
                volatility: None,
            };
            let vol = DemoMarketData::get_volatility(Some(&config));
            // Falls back to default when Custom but no value provided
            assert!((vol - 0.20).abs() < 1e-10);
        }

        #[test]
        fn test_constants() {
            assert!((DemoMarketData::DEFAULT_CURVE_RATE - 0.05).abs() < 1e-10);
            assert!((DemoMarketData::DEFAULT_VOLATILITY - 0.20).abs() < 1e-10);
        }
    }

    // =========================================================================
    // InterpolationMethod Tests (Task 1.1)
    // =========================================================================

    mod interpolation_method_tests {
        use super::*;

        #[test]
        fn test_serialize_linear() {
            let method = InterpolationMethod::Linear;
            let json = serde_json::to_string(&method).unwrap();
            assert_eq!(json, "\"linear\"");
        }

        #[test]
        fn test_serialize_log_linear() {
            let method = InterpolationMethod::LogLinear;
            let json = serde_json::to_string(&method).unwrap();
            assert_eq!(json, "\"log_linear\"");
        }

        #[test]
        fn test_deserialize_interpolation_methods() {
            let linear: InterpolationMethod = serde_json::from_str("\"linear\"").unwrap();
            assert_eq!(linear, InterpolationMethod::Linear);

            let log_linear: InterpolationMethod = serde_json::from_str("\"log_linear\"").unwrap();
            assert_eq!(log_linear, InterpolationMethod::LogLinear);
        }

        #[test]
        fn test_default_is_log_linear() {
            let default_method = InterpolationMethod::default();
            assert_eq!(default_method, InterpolationMethod::LogLinear);
        }
    }

    // =========================================================================
    // ParRateInput Tests (Task 1.1)
    // =========================================================================

    mod par_rate_input_tests {
        use super::*;

        fn sample_par_rate() -> ParRateInput {
            ParRateInput {
                tenor: "5Y".to_string(),
                rate: 0.025,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let input = sample_par_rate();
            let json = serde_json::to_string(&input).unwrap();

            assert!(json.contains("\"tenor\":\"5Y\""));
            assert!(json.contains("\"rate\":0.025"));
        }

        #[test]
        fn test_deserialize_camel_case() {
            let json = r#"{
                "tenor": "10Y",
                "rate": 0.035
            }"#;

            let input: ParRateInput = serde_json::from_str(json).unwrap();
            assert_eq!(input.tenor, "10Y");
            assert!((input.rate - 0.035).abs() < 1e-10);
        }

        #[test]
        fn test_deserialize_all_tenors() {
            let tenors = ["1Y", "2Y", "3Y", "5Y", "7Y", "10Y", "15Y", "20Y", "30Y"];
            for tenor in tenors {
                let json = format!(r#"{{"tenor": "{}", "rate": 0.02}}"#, tenor);
                let input: ParRateInput = serde_json::from_str(&json).unwrap();
                assert_eq!(input.tenor, tenor);
            }
        }
    }

    // =========================================================================
    // BootstrapRequest Tests (Task 1.1)
    // =========================================================================

    mod bootstrap_request_tests {
        use super::*;

        #[test]
        fn test_deserialize_with_interpolation() {
            let json = r#"{
                "parRates": [
                    {"tenor": "1Y", "rate": 0.02},
                    {"tenor": "5Y", "rate": 0.025},
                    {"tenor": "10Y", "rate": 0.03}
                ],
                "interpolation": "linear"
            }"#;

            let request: BootstrapRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.par_rates.len(), 3);
            assert_eq!(request.par_rates[0].tenor, "1Y");
            assert!((request.par_rates[0].rate - 0.02).abs() < 1e-10);
            assert_eq!(request.interpolation, InterpolationMethod::Linear);
        }

        #[test]
        fn test_deserialize_without_interpolation_defaults_to_log_linear() {
            let json = r#"{
                "parRates": [
                    {"tenor": "1Y", "rate": 0.02},
                    {"tenor": "5Y", "rate": 0.025}
                ]
            }"#;

            let request: BootstrapRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.par_rates.len(), 2);
            assert_eq!(request.interpolation, InterpolationMethod::LogLinear);
        }

        #[test]
        fn test_deserialize_full_curve_9_tenors() {
            let json = r#"{
                "parRates": [
                    {"tenor": "1Y", "rate": 0.020},
                    {"tenor": "2Y", "rate": 0.022},
                    {"tenor": "3Y", "rate": 0.024},
                    {"tenor": "5Y", "rate": 0.026},
                    {"tenor": "7Y", "rate": 0.028},
                    {"tenor": "10Y", "rate": 0.030},
                    {"tenor": "15Y", "rate": 0.032},
                    {"tenor": "20Y", "rate": 0.034},
                    {"tenor": "30Y", "rate": 0.035}
                ],
                "interpolation": "log_linear"
            }"#;

            let request: BootstrapRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.par_rates.len(), 9);
            assert_eq!(request.par_rates[8].tenor, "30Y");
            assert!((request.par_rates[8].rate - 0.035).abs() < 1e-10);
        }
    }

    // =========================================================================
    // BootstrapResponse Tests (Task 1.1)
    // =========================================================================

    mod bootstrap_response_tests {
        use super::*;

        fn sample_bootstrap_response() -> BootstrapResponse {
            BootstrapResponse {
                curve_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
                pillars: vec![1.0, 2.0, 5.0, 10.0],
                discount_factors: vec![0.98, 0.95, 0.88, 0.74],
                zero_rates: vec![0.020, 0.025, 0.026, 0.030],
                processing_time_ms: 15.5,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let response = sample_bootstrap_response();
            let json = serde_json::to_string(&response).unwrap();

            assert!(json.contains("\"curveId\":\"550e8400-e29b-41d4-a716-446655440000\""));
            assert!(json.contains("\"pillars\":[1.0,2.0,5.0,10.0]"));
            assert!(json.contains("\"discountFactors\":[0.98,0.95,0.88,0.74]"));
            assert!(json.contains("\"zeroRates\":[0.02,0.025,0.026,0.03]"));
            assert!(json.contains("\"processingTimeMs\":15.5"));
        }

        #[test]
        fn test_serialize_empty_arrays() {
            let response = BootstrapResponse {
                curve_id: "test-id".to_string(),
                pillars: vec![],
                discount_factors: vec![],
                zero_rates: vec![],
                processing_time_ms: 0.0,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"pillars\":[]"));
            assert!(json.contains("\"discountFactors\":[]"));
            assert!(json.contains("\"zeroRates\":[]"));
        }

        #[test]
        fn test_serialize_full_9_tenor_response() {
            let response = BootstrapResponse {
                curve_id: "curve-001".to_string(),
                pillars: vec![1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
                discount_factors: vec![
                    0.9804, 0.9560, 0.9310, 0.8798, 0.8319, 0.7536, 0.6349, 0.5349, 0.3769,
                ],
                zero_rates: vec![
                    0.0200, 0.0225, 0.0237, 0.0256, 0.0264, 0.0283, 0.0302, 0.0317, 0.0325,
                ],
                processing_time_ms: 42.7,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"curveId\":\"curve-001\""));
            assert!(json.contains("\"processingTimeMs\":42.7"));
            // Verify array lengths are preserved
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["pillars"].as_array().unwrap().len(), 9);
            assert_eq!(parsed["discountFactors"].as_array().unwrap().len(), 9);
            assert_eq!(parsed["zeroRates"].as_array().unwrap().len(), 9);
        }
    }

    // =========================================================================
    // PaymentFrequency Tests (Task 1.2)
    // =========================================================================

    mod payment_frequency_tests {
        use super::*;

        #[test]
        fn test_serialize_annual() {
            let freq = PaymentFrequency::Annual;
            let json = serde_json::to_string(&freq).unwrap();
            assert_eq!(json, "\"annual\"");
        }

        #[test]
        fn test_serialize_semi_annual() {
            let freq = PaymentFrequency::SemiAnnual;
            let json = serde_json::to_string(&freq).unwrap();
            assert_eq!(json, "\"semi_annual\"");
        }

        #[test]
        fn test_serialize_quarterly() {
            let freq = PaymentFrequency::Quarterly;
            let json = serde_json::to_string(&freq).unwrap();
            assert_eq!(json, "\"quarterly\"");
        }

        #[test]
        fn test_deserialize_payment_frequencies() {
            let annual: PaymentFrequency = serde_json::from_str("\"annual\"").unwrap();
            assert_eq!(annual, PaymentFrequency::Annual);

            let semi: PaymentFrequency = serde_json::from_str("\"semi_annual\"").unwrap();
            assert_eq!(semi, PaymentFrequency::SemiAnnual);

            let quarterly: PaymentFrequency = serde_json::from_str("\"quarterly\"").unwrap();
            assert_eq!(quarterly, PaymentFrequency::Quarterly);
        }

        #[test]
        fn test_default_is_annual() {
            let default_freq = PaymentFrequency::default();
            assert_eq!(default_freq, PaymentFrequency::Annual);
        }
    }

    // =========================================================================
    // IrsPricingRequest Tests (Task 1.2)
    // =========================================================================

    mod irs_pricing_request_tests {
        use super::*;

        #[test]
        fn test_deserialize_full_request() {
            let json = r#"{
                "curveId": "550e8400-e29b-41d4-a716-446655440000",
                "notional": 10000000.0,
                "fixedRate": 0.03,
                "tenorYears": 5.0,
                "paymentFrequency": "annual"
            }"#;

            let request: IrsPricingRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.curve_id, "550e8400-e29b-41d4-a716-446655440000");
            assert!((request.notional - 10_000_000.0).abs() < 1e-10);
            assert!((request.fixed_rate - 0.03).abs() < 1e-10);
            assert!((request.tenor_years - 5.0).abs() < 1e-10);
            assert_eq!(request.payment_frequency, PaymentFrequency::Annual);
        }

        #[test]
        fn test_deserialize_with_semi_annual() {
            let json = r#"{
                "curveId": "curve-001",
                "notional": 50000000.0,
                "fixedRate": 0.025,
                "tenorYears": 10.0,
                "paymentFrequency": "semi_annual"
            }"#;

            let request: IrsPricingRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.payment_frequency, PaymentFrequency::SemiAnnual);
            assert!((request.notional - 50_000_000.0).abs() < 1e-10);
        }

        #[test]
        fn test_deserialize_with_quarterly() {
            let json = r#"{
                "curveId": "curve-002",
                "notional": 100000000.0,
                "fixedRate": 0.035,
                "tenorYears": 7.0,
                "paymentFrequency": "quarterly"
            }"#;

            let request: IrsPricingRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.payment_frequency, PaymentFrequency::Quarterly);
            assert!((request.tenor_years - 7.0).abs() < 1e-10);
        }
    }

    // =========================================================================
    // IrsPricingResponse Tests (Task 1.2)
    // =========================================================================

    mod irs_pricing_response_tests {
        use super::*;

        fn sample_pricing_response() -> IrsPricingResponse {
            IrsPricingResponse {
                npv: 125_000.50,
                fixed_leg_pv: 4_500_000.0,
                float_leg_pv: 4_375_000.50,
                processing_time_us: 850.5,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let response = sample_pricing_response();
            let json = serde_json::to_string(&response).unwrap();

            assert!(json.contains("\"npv\":125000.5"));
            assert!(json.contains("\"fixedLegPv\":4500000"));
            assert!(json.contains("\"floatLegPv\":4375000.5"));
            assert!(json.contains("\"processingTimeUs\":850.5"));
        }

        #[test]
        fn test_serialize_negative_npv() {
            let response = IrsPricingResponse {
                npv: -75_000.25,
                fixed_leg_pv: 4_200_000.0,
                float_leg_pv: 4_275_000.25,
                processing_time_us: 920.0,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"npv\":-75000.25"));
        }

        #[test]
        fn test_serialize_zero_npv() {
            let response = IrsPricingResponse {
                npv: 0.0,
                fixed_leg_pv: 5_000_000.0,
                float_leg_pv: 5_000_000.0,
                processing_time_us: 500.0,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"npv\":0"));
            // Fixed and float leg PV should be equal for zero NPV
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["fixedLegPv"], parsed["floatLegPv"]);
        }

        #[test]
        fn test_serialize_large_notional() {
            let response = IrsPricingResponse {
                npv: 1_250_000.0,
                fixed_leg_pv: 450_000_000.0,
                float_leg_pv: 448_750_000.0,
                processing_time_us: 1200.0,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"npv\":1250000"));
            assert!(json.contains("\"fixedLegPv\":450000000"));
        }
    }

    // =========================================================================
    // RiskRequest Tests (Task 1.3)
    // =========================================================================

    mod risk_request_tests {
        use super::*;

        #[test]
        fn test_deserialize_full_request() {
            let json = r#"{
                "curveId": "curve-001",
                "notional": 10000000.0,
                "fixedRate": 0.03,
                "tenorYears": 5.0,
                "paymentFrequency": "annual",
                "bumpSizeBps": 1.0
            }"#;

            let request: RiskRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.curve_id, "curve-001");
            assert!((request.notional - 10_000_000.0).abs() < 1e-10);
            assert!((request.fixed_rate - 0.03).abs() < 1e-10);
            assert!((request.tenor_years - 5.0).abs() < 1e-10);
            assert_eq!(request.payment_frequency, PaymentFrequency::Annual);
            assert!((request.bump_size_bps - 1.0).abs() < 1e-10);
        }

        #[test]
        fn test_deserialize_without_bump_size_defaults_to_1() {
            let json = r#"{
                "curveId": "curve-002",
                "notional": 50000000.0,
                "fixedRate": 0.025,
                "tenorYears": 10.0,
                "paymentFrequency": "semi_annual"
            }"#;

            let request: RiskRequest = serde_json::from_str(json).unwrap();
            assert!((request.bump_size_bps - 1.0).abs() < 1e-10);
        }

        #[test]
        fn test_deserialize_custom_bump_size() {
            let json = r#"{
                "curveId": "curve-003",
                "notional": 100000000.0,
                "fixedRate": 0.035,
                "tenorYears": 7.0,
                "paymentFrequency": "quarterly",
                "bumpSizeBps": 0.5
            }"#;

            let request: RiskRequest = serde_json::from_str(json).unwrap();
            assert!((request.bump_size_bps - 0.5).abs() < 1e-10);
        }
    }

    // =========================================================================
    // DeltaResult Tests (Task 1.3)
    // =========================================================================

    mod delta_result_tests {
        use super::*;

        fn sample_delta_result() -> DeltaResult {
            DeltaResult {
                tenor: "5Y".to_string(),
                delta: -4500.25,
                processing_time_us: 125.5,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let result = sample_delta_result();
            let json = serde_json::to_string(&result).unwrap();

            assert!(json.contains("\"tenor\":\"5Y\""));
            assert!(json.contains("\"delta\":-4500.25"));
            assert!(json.contains("\"processingTimeUs\":125.5"));
        }

        #[test]
        fn test_deserialize_delta_result() {
            let json = r#"{
                "tenor": "10Y",
                "delta": -8750.50,
                "processingTimeUs": 200.0
            }"#;

            let result: DeltaResult = serde_json::from_str(json).unwrap();
            assert_eq!(result.tenor, "10Y");
            assert!((result.delta - (-8750.50)).abs() < 1e-10);
            assert!((result.processing_time_us - 200.0).abs() < 1e-10);
        }
    }

    // =========================================================================
    // TimingStats Tests (Task 1.3)
    // =========================================================================

    mod timing_stats_tests {
        use super::*;

        fn sample_timing_stats() -> TimingStats {
            TimingStats {
                mean_us: 150.5,
                std_dev_us: 25.3,
                min_us: 100.0,
                max_us: 250.0,
                total_ms: 1.355,
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let stats = sample_timing_stats();
            let json = serde_json::to_string(&stats).unwrap();

            assert!(json.contains("\"meanUs\":150.5"));
            assert!(json.contains("\"stdDevUs\":25.3"));
            assert!(json.contains("\"minUs\":100"));
            assert!(json.contains("\"maxUs\":250"));
            assert!(json.contains("\"totalMs\":1.355"));
        }

        #[test]
        fn test_deserialize_timing_stats() {
            let json = r#"{
                "meanUs": 200.0,
                "stdDevUs": 30.5,
                "minUs": 150.0,
                "maxUs": 300.0,
                "totalMs": 1.8
            }"#;

            let stats: TimingStats = serde_json::from_str(json).unwrap();
            assert!((stats.mean_us - 200.0).abs() < 1e-10);
            assert!((stats.std_dev_us - 30.5).abs() < 1e-10);
            assert!((stats.min_us - 150.0).abs() < 1e-10);
            assert!((stats.max_us - 300.0).abs() < 1e-10);
            assert!((stats.total_ms - 1.8).abs() < 1e-10);
        }
    }

    // =========================================================================
    // RiskMethodResult Tests (Task 1.3)
    // =========================================================================

    mod risk_method_result_tests {
        use super::*;

        fn sample_risk_method_result() -> RiskMethodResult {
            RiskMethodResult {
                deltas: vec![
                    DeltaResult {
                        tenor: "1Y".to_string(),
                        delta: -1000.0,
                        processing_time_us: 100.0,
                    },
                    DeltaResult {
                        tenor: "5Y".to_string(),
                        delta: -4500.0,
                        processing_time_us: 120.0,
                    },
                    DeltaResult {
                        tenor: "10Y".to_string(),
                        delta: -8000.0,
                        processing_time_us: 130.0,
                    },
                ],
                dv01: -13500.0,
                timing: TimingStats {
                    mean_us: 116.67,
                    std_dev_us: 12.47,
                    min_us: 100.0,
                    max_us: 130.0,
                    total_ms: 0.35,
                },
            }
        }

        #[test]
        fn test_serialize_camel_case() {
            let result = sample_risk_method_result();
            let json = serde_json::to_string(&result).unwrap();

            assert!(json.contains("\"deltas\":["));
            assert!(json.contains("\"dv01\":-13500"));
            assert!(json.contains("\"timing\":{"));
        }

        #[test]
        fn test_serialize_preserves_delta_count() {
            let result = sample_risk_method_result();
            let json = serde_json::to_string(&result).unwrap();

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["deltas"].as_array().unwrap().len(), 3);
        }
    }

    // =========================================================================
    // TimingComparison Tests (Task 1.3)
    // =========================================================================

    mod timing_comparison_tests {
        use super::*;

        #[test]
        fn test_serialize_with_aad() {
            let comparison = TimingComparison {
                bump_total_ms: 5.0,
                aad_total_ms: Some(0.5),
                speedup_ratio: Some(10.0),
            };

            let json = serde_json::to_string(&comparison).unwrap();
            assert!(json.contains("\"bumpTotalMs\":5"));
            assert!(json.contains("\"aadTotalMs\":0.5"));
            assert!(json.contains("\"speedupRatio\":10"));
        }

        #[test]
        fn test_serialize_without_aad() {
            let comparison = TimingComparison {
                bump_total_ms: 5.0,
                aad_total_ms: None,
                speedup_ratio: None,
            };

            let json = serde_json::to_string(&comparison).unwrap();
            assert!(json.contains("\"bumpTotalMs\":5"));
            assert!(json.contains("\"aadTotalMs\":null"));
            assert!(json.contains("\"speedupRatio\":null"));
        }
    }

    // =========================================================================
    // RiskCompareResponse Tests (Task 1.3)
    // =========================================================================

    mod risk_compare_response_tests {
        use super::*;

        fn sample_bump_result() -> RiskMethodResult {
            RiskMethodResult {
                deltas: vec![DeltaResult {
                    tenor: "5Y".to_string(),
                    delta: -4500.0,
                    processing_time_us: 1000.0,
                }],
                dv01: -4500.0,
                timing: TimingStats {
                    mean_us: 1000.0,
                    std_dev_us: 0.0,
                    min_us: 1000.0,
                    max_us: 1000.0,
                    total_ms: 1.0,
                },
            }
        }

        fn sample_aad_result() -> RiskMethodResult {
            RiskMethodResult {
                deltas: vec![DeltaResult {
                    tenor: "5Y".to_string(),
                    delta: -4500.0,
                    processing_time_us: 100.0,
                }],
                dv01: -4500.0,
                timing: TimingStats {
                    mean_us: 100.0,
                    std_dev_us: 0.0,
                    min_us: 100.0,
                    max_us: 100.0,
                    total_ms: 0.1,
                },
            }
        }

        #[test]
        fn test_serialize_with_aad_available() {
            let response = RiskCompareResponse {
                bump: sample_bump_result(),
                aad: Some(sample_aad_result()),
                aad_available: true,
                speedup_ratio: Some(10.0),
                comparison: TimingComparison {
                    bump_total_ms: 1.0,
                    aad_total_ms: Some(0.1),
                    speedup_ratio: Some(10.0),
                },
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"bump\":{"));
            assert!(json.contains("\"aad\":{"));
            assert!(json.contains("\"aadAvailable\":true"));
            assert!(json.contains("\"speedupRatio\":10"));
            assert!(json.contains("\"comparison\":{"));
        }

        #[test]
        fn test_serialize_without_aad() {
            let response = RiskCompareResponse {
                bump: sample_bump_result(),
                aad: None,
                aad_available: false,
                speedup_ratio: None,
                comparison: TimingComparison {
                    bump_total_ms: 1.0,
                    aad_total_ms: None,
                    speedup_ratio: None,
                },
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"bump\":{"));
            assert!(json.contains("\"aad\":null"));
            assert!(json.contains("\"aadAvailable\":false"));
            assert!(json.contains("\"speedupRatio\":null"));
        }
    }

    // =========================================================================
    // RiskBumpResponse Tests (Task 1.3)
    // =========================================================================

    mod risk_bump_response_tests {
        use super::*;

        #[test]
        fn test_serialize_camel_case() {
            let response = RiskBumpResponse {
                deltas: vec![
                    DeltaResult {
                        tenor: "1Y".to_string(),
                        delta: -1000.0,
                        processing_time_us: 500.0,
                    },
                    DeltaResult {
                        tenor: "5Y".to_string(),
                        delta: -4500.0,
                        processing_time_us: 600.0,
                    },
                ],
                dv01: -5500.0,
                timing: TimingStats {
                    mean_us: 550.0,
                    std_dev_us: 50.0,
                    min_us: 500.0,
                    max_us: 600.0,
                    total_ms: 1.1,
                },
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"deltas\":["));
            assert!(json.contains("\"dv01\":-5500"));
            assert!(json.contains("\"timing\":{"));

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["deltas"].as_array().unwrap().len(), 2);
        }
    }

    // =========================================================================
    // RiskAadResponse Tests (Task 1.3)
    // =========================================================================

    mod risk_aad_response_tests {
        use super::*;

        #[test]
        fn test_serialize_camel_case() {
            let response = RiskAadResponse {
                deltas: vec![
                    DeltaResult {
                        tenor: "1Y".to_string(),
                        delta: -1000.0,
                        processing_time_us: 50.0,
                    },
                    DeltaResult {
                        tenor: "5Y".to_string(),
                        delta: -4500.0,
                        processing_time_us: 50.0,
                    },
                ],
                dv01: -5500.0,
                timing: TimingStats {
                    mean_us: 50.0,
                    std_dev_us: 0.0,
                    min_us: 50.0,
                    max_us: 50.0,
                    total_ms: 0.1,
                },
                aad_available: true,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"deltas\":["));
            assert!(json.contains("\"dv01\":-5500"));
            assert!(json.contains("\"timing\":{"));
            assert!(json.contains("\"aadAvailable\":true"));

            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["deltas"].as_array().unwrap().len(), 2);
        }
    }

    // =========================================================================
    // ErrorDetails Tests (Task 1.4)
    // =========================================================================

    mod error_details_tests {
        use super::*;

        #[test]
        fn test_new_is_empty() {
            let details = ErrorDetails::new();
            assert!(details.is_empty());
            assert!(details.field.is_none());
            assert!(details.tenor.is_none());
            assert!(details.suggestion.is_none());
        }

        #[test]
        fn test_default_is_empty() {
            let details = ErrorDetails::default();
            assert!(details.is_empty());
        }

        #[test]
        fn test_with_field() {
            let details = ErrorDetails::with_field("notional");
            assert!(!details.is_empty());
            assert_eq!(details.field, Some("notional".to_string()));
            assert!(details.tenor.is_none());
            assert!(details.suggestion.is_none());
        }

        #[test]
        fn test_with_tenor() {
            let details = ErrorDetails::with_tenor("5Y");
            assert!(!details.is_empty());
            assert!(details.field.is_none());
            assert_eq!(details.tenor, Some("5Y".to_string()));
            assert!(details.suggestion.is_none());
        }

        #[test]
        fn test_with_suggestion() {
            let details = ErrorDetails::with_suggestion("Try a smaller value");
            assert!(!details.is_empty());
            assert!(details.field.is_none());
            assert!(details.tenor.is_none());
            assert_eq!(details.suggestion, Some("Try a smaller value".to_string()));
        }

        #[test]
        fn test_builder_chain() {
            let details = ErrorDetails::new()
                .field("parRates[5Y].rate")
                .tenor("5Y")
                .suggestion("Ensure rate is positive");

            assert!(!details.is_empty());
            assert_eq!(details.field, Some("parRates[5Y].rate".to_string()));
            assert_eq!(details.tenor, Some("5Y".to_string()));
            assert_eq!(
                details.suggestion,
                Some("Ensure rate is positive".to_string())
            );
        }

        #[test]
        fn test_serialize_skips_none_fields() {
            let details = ErrorDetails::with_field("notional");
            let json = serde_json::to_string(&details).unwrap();

            assert!(json.contains("\"field\":\"notional\""));
            assert!(!json.contains("\"tenor\""));
            assert!(!json.contains("\"suggestion\""));
        }

        #[test]
        fn test_serialize_all_fields() {
            let details = ErrorDetails::new()
                .field("rate")
                .tenor("10Y")
                .suggestion("Check input");

            let json = serde_json::to_string(&details).unwrap();
            assert!(json.contains("\"field\":\"rate\""));
            assert!(json.contains("\"tenor\":\"10Y\""));
            assert!(json.contains("\"suggestion\":\"Check input\""));
        }

        #[test]
        fn test_deserialize() {
            let json = r#"{
                "field": "notional",
                "tenor": "5Y",
                "suggestion": "Use positive value"
            }"#;

            let details: ErrorDetails = serde_json::from_str(json).unwrap();
            assert_eq!(details.field, Some("notional".to_string()));
            assert_eq!(details.tenor, Some("5Y".to_string()));
            assert_eq!(details.suggestion, Some("Use positive value".to_string()));
        }
    }

    // =========================================================================
    // IrsBootstrapErrorResponse Tests (Task 1.4)
    // =========================================================================

    mod irs_bootstrap_error_response_tests {
        use super::*;

        #[test]
        fn test_validation_error() {
            let error = IrsBootstrapErrorResponse::validation_error(
                "Notional must be positive",
                "notional",
            );

            assert_eq!(error.error, "ValidationError");
            assert_eq!(error.message, "Notional must be positive");
            assert!(error.details.is_some());
            assert_eq!(
                error.details.as_ref().unwrap().field,
                Some("notional".to_string())
            );
        }

        #[test]
        fn test_not_found() {
            let error = IrsBootstrapErrorResponse::not_found("Resource not found");

            assert_eq!(error.error, "NotFoundError");
            assert_eq!(error.message, "Resource not found");
            assert!(error.details.is_none());
        }

        #[test]
        fn test_curve_not_found() {
            let error =
                IrsBootstrapErrorResponse::curve_not_found("550e8400-e29b-41d4-a716-446655440000");

            assert_eq!(error.error, "NotFoundError");
            assert!(error
                .message
                .contains("550e8400-e29b-41d4-a716-446655440000"));
            assert!(error.details.is_some());
            assert_eq!(
                error.details.as_ref().unwrap().field,
                Some("curveId".to_string())
            );
        }

        #[test]
        fn test_bootstrap_convergence_failure() {
            let error = IrsBootstrapErrorResponse::bootstrap_convergence_failure(
                "10Y",
                "Try adjusting nearby tenor rates",
            );

            assert_eq!(error.error, "CalculationError");
            assert!(error.message.contains("10Y"));
            assert!(error.message.contains("converge"));
            assert!(error.details.is_some());
            let details = error.details.as_ref().unwrap();
            assert_eq!(details.tenor, Some("10Y".to_string()));
            assert_eq!(
                details.suggestion,
                Some("Try adjusting nearby tenor rates".to_string())
            );
        }

        #[test]
        fn test_calculation_error() {
            let error = IrsBootstrapErrorResponse::calculation_error("Numerical instability");

            assert_eq!(error.error, "CalculationError");
            assert_eq!(error.message, "Numerical instability");
            assert!(error.details.is_none());
        }

        #[test]
        fn test_calculation_error_with_details() {
            let details = ErrorDetails::with_field("fixedRate").suggestion("Check rate range");
            let error =
                IrsBootstrapErrorResponse::calculation_error_with_details("Rate error", details);

            assert_eq!(error.error, "CalculationError");
            assert_eq!(error.message, "Rate error");
            assert!(error.details.is_some());
        }

        #[test]
        fn test_calculation_error_with_empty_details() {
            let details = ErrorDetails::new();
            let error =
                IrsBootstrapErrorResponse::calculation_error_with_details("Rate error", details);

            assert!(error.details.is_none());
        }

        #[test]
        fn test_serialize_validation_error_camel_case() {
            let error = IrsBootstrapErrorResponse::validation_error(
                "Notional must be positive",
                "notional",
            );
            let json = serde_json::to_string(&error).unwrap();

            assert!(json.contains("\"error\":\"ValidationError\""));
            assert!(json.contains("\"message\":\"Notional must be positive\""));
            assert!(json.contains("\"details\":{"));
            assert!(json.contains("\"field\":\"notional\""));
        }

        #[test]
        fn test_serialize_not_found_skips_details() {
            let error = IrsBootstrapErrorResponse::not_found("Not found");
            let json = serde_json::to_string(&error).unwrap();

            assert!(!json.contains("\"details\""));
        }
    }

    // =========================================================================
    // ValidationError Tests (Task 1.4)
    // =========================================================================

    mod validation_error_tests {
        use super::*;

        #[test]
        fn test_negative_par_rate_to_error_response() {
            let error = ValidationError::NegativeParRate {
                tenor: "5Y".to_string(),
                rate: -0.01,
            };
            let response = error.to_error_response();

            assert_eq!(response.error, "ValidationError");
            assert!(response.message.contains("5Y"));
            assert!(response.message.contains("-0.01"));
            assert!(response.details.is_some());
        }

        #[test]
        fn test_invalid_par_rate_value_to_error_response() {
            let error = ValidationError::InvalidParRateValue {
                tenor: "10Y".to_string(),
                rate: f64::NAN,
            };
            let response = error.to_error_response();

            assert_eq!(response.error, "ValidationError");
            assert!(response.message.contains("10Y"));
        }

        #[test]
        fn test_empty_par_rates_to_error_response() {
            let error = ValidationError::EmptyParRates;
            let response = error.to_error_response();

            assert_eq!(response.error, "ValidationError");
            assert!(response.message.contains("empty"));
        }

        #[test]
        fn test_notional_not_positive_to_error_response() {
            let error = ValidationError::NotionalNotPositive { notional: -1000.0 };
            let response = error.to_error_response();

            assert_eq!(response.error, "ValidationError");
            assert!(response.message.contains("Notional"));
            assert!(response.message.contains("-1000"));
        }

        #[test]
        fn test_fixed_rate_out_of_range_to_error_response() {
            let error = ValidationError::FixedRateOutOfRange { fixed_rate: 1.5 };
            let response = error.to_error_response();

            assert_eq!(response.error, "ValidationError");
            assert!(response.message.contains("1.5"));
        }

        #[test]
        fn test_tenor_years_not_positive_to_error_response() {
            let error = ValidationError::TenorYearsNotPositive { tenor_years: 0.0 };
            let response = error.to_error_response();

            assert_eq!(response.error, "ValidationError");
            assert!(response.message.contains("Tenor"));
        }

        #[test]
        fn test_tenor_years_exceeds_max_to_error_response() {
            let error = ValidationError::TenorYearsExceedsMax {
                tenor_years: 60.0,
                max: 50.0,
            };
            let response = error.to_error_response();

            assert_eq!(response.error, "ValidationError");
            assert!(response.message.contains("60"));
            assert!(response.message.contains("50"));
        }

        #[test]
        fn test_invalid_tenor_format_to_error_response() {
            let error = ValidationError::InvalidTenorFormat {
                tenor: "invalid".to_string(),
            };
            let response = error.to_error_response();

            assert_eq!(response.error, "ValidationError");
            assert!(response.message.contains("invalid"));
        }

        #[test]
        fn test_bump_size_not_positive_to_error_response() {
            let error = ValidationError::BumpSizeNotPositive { bump_size_bps: 0.0 };
            let response = error.to_error_response();

            assert_eq!(response.error, "ValidationError");
            assert!(response.message.contains("Bump size"));
        }
    }

    // =========================================================================
    // Par Rate Validation Tests (Task 1.4)
    // =========================================================================

    mod par_rate_validation_tests {
        use super::*;

        #[test]
        fn test_validate_par_rate_valid() {
            let input = ParRateInput {
                tenor: "5Y".to_string(),
                rate: 0.025,
            };
            assert!(validate_par_rate(&input).is_ok());
        }

        #[test]
        fn test_validate_par_rate_zero_is_valid() {
            let input = ParRateInput {
                tenor: "1Y".to_string(),
                rate: 0.0,
            };
            assert!(validate_par_rate(&input).is_ok());
        }

        #[test]
        fn test_validate_par_rate_negative() {
            let input = ParRateInput {
                tenor: "5Y".to_string(),
                rate: -0.01,
            };
            let result = validate_par_rate(&input);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::NegativeParRate { tenor, rate } => {
                    assert_eq!(tenor, "5Y");
                    assert!((rate - (-0.01)).abs() < 1e-10);
                }
                _ => panic!("Expected NegativeParRate error"),
            }
        }

        #[test]
        fn test_validate_par_rate_nan() {
            let input = ParRateInput {
                tenor: "10Y".to_string(),
                rate: f64::NAN,
            };
            let result = validate_par_rate(&input);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::InvalidParRateValue { tenor, .. } => {
                    assert_eq!(tenor, "10Y");
                }
                _ => panic!("Expected InvalidParRateValue error"),
            }
        }

        #[test]
        fn test_validate_par_rate_infinity() {
            let input = ParRateInput {
                tenor: "30Y".to_string(),
                rate: f64::INFINITY,
            };
            let result = validate_par_rate(&input);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::InvalidParRateValue { tenor, .. } => {
                    assert_eq!(tenor, "30Y");
                }
                _ => panic!("Expected InvalidParRateValue error"),
            }
        }

        #[test]
        fn test_validate_par_rates_valid() {
            let inputs = vec![
                ParRateInput {
                    tenor: "1Y".to_string(),
                    rate: 0.02,
                },
                ParRateInput {
                    tenor: "5Y".to_string(),
                    rate: 0.025,
                },
                ParRateInput {
                    tenor: "10Y".to_string(),
                    rate: 0.03,
                },
            ];
            assert!(validate_par_rates(&inputs).is_ok());
        }

        #[test]
        fn test_validate_par_rates_empty() {
            let inputs: Vec<ParRateInput> = vec![];
            let result = validate_par_rates(&inputs);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::EmptyParRates => {}
                _ => panic!("Expected EmptyParRates error"),
            }
        }

        #[test]
        fn test_validate_par_rates_one_invalid() {
            let inputs = vec![
                ParRateInput {
                    tenor: "1Y".to_string(),
                    rate: 0.02,
                },
                ParRateInput {
                    tenor: "5Y".to_string(),
                    rate: -0.01,
                }, // Invalid
                ParRateInput {
                    tenor: "10Y".to_string(),
                    rate: 0.03,
                },
            ];
            let result = validate_par_rates(&inputs);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::NegativeParRate { tenor, .. } => {
                    assert_eq!(tenor, "5Y");
                }
                _ => panic!("Expected NegativeParRate error"),
            }
        }
    }

    // =========================================================================
    // IRS Parameter Validation Tests (Task 1.4)
    // =========================================================================

    mod irs_parameter_validation_tests {
        use super::*;

        fn valid_irs_pricing_request() -> IrsPricingRequest {
            IrsPricingRequest {
                curve_id: "curve-001".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
            }
        }

        fn valid_risk_request() -> RiskRequest {
            RiskRequest {
                curve_id: "curve-001".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            }
        }

        #[test]
        fn test_validate_irs_pricing_request_valid() {
            let request = valid_irs_pricing_request();
            assert!(validate_irs_pricing_request(&request).is_ok());
        }

        #[test]
        fn test_validate_irs_pricing_request_negative_notional() {
            let mut request = valid_irs_pricing_request();
            request.notional = -1000.0;
            let result = validate_irs_pricing_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::NotionalNotPositive { .. } => {}
                _ => panic!("Expected NotionalNotPositive error"),
            }
        }

        #[test]
        fn test_validate_irs_pricing_request_zero_notional() {
            let mut request = valid_irs_pricing_request();
            request.notional = 0.0;
            let result = validate_irs_pricing_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::NotionalNotPositive { .. } => {}
                _ => panic!("Expected NotionalNotPositive error"),
            }
        }

        #[test]
        fn test_validate_irs_pricing_request_nan_notional() {
            let mut request = valid_irs_pricing_request();
            request.notional = f64::NAN;
            let result = validate_irs_pricing_request(&request);
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_irs_pricing_request_fixed_rate_out_of_range_high() {
            let mut request = valid_irs_pricing_request();
            request.fixed_rate = 1.5;
            let result = validate_irs_pricing_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::FixedRateOutOfRange { .. } => {}
                _ => panic!("Expected FixedRateOutOfRange error"),
            }
        }

        #[test]
        fn test_validate_irs_pricing_request_fixed_rate_out_of_range_low() {
            let mut request = valid_irs_pricing_request();
            request.fixed_rate = -1.5;
            let result = validate_irs_pricing_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::FixedRateOutOfRange { .. } => {}
                _ => panic!("Expected FixedRateOutOfRange error"),
            }
        }

        #[test]
        fn test_validate_irs_pricing_request_negative_fixed_rate_valid() {
            // Negative rates within -1.0 to 1.0 are valid
            let mut request = valid_irs_pricing_request();
            request.fixed_rate = -0.005; // -0.5% is valid
            assert!(validate_irs_pricing_request(&request).is_ok());
        }

        #[test]
        fn test_validate_irs_pricing_request_zero_tenor_years() {
            let mut request = valid_irs_pricing_request();
            request.tenor_years = 0.0;
            let result = validate_irs_pricing_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TenorYearsNotPositive { .. } => {}
                _ => panic!("Expected TenorYearsNotPositive error"),
            }
        }

        #[test]
        fn test_validate_irs_pricing_request_negative_tenor_years() {
            let mut request = valid_irs_pricing_request();
            request.tenor_years = -1.0;
            let result = validate_irs_pricing_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TenorYearsNotPositive { .. } => {}
                _ => panic!("Expected TenorYearsNotPositive error"),
            }
        }

        #[test]
        fn test_validate_irs_pricing_request_tenor_years_exceeds_max() {
            let mut request = valid_irs_pricing_request();
            request.tenor_years = 60.0; // Exceeds MAX_TENOR_YEARS (50)
            let result = validate_irs_pricing_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TenorYearsExceedsMax { tenor_years, max } => {
                    assert!((tenor_years - 60.0).abs() < 1e-10);
                    assert!((max - 50.0).abs() < 1e-10);
                }
                _ => panic!("Expected TenorYearsExceedsMax error"),
            }
        }

        #[test]
        fn test_validate_risk_request_valid() {
            let request = valid_risk_request();
            assert!(validate_risk_request(&request).is_ok());
        }

        #[test]
        fn test_validate_risk_request_zero_bump_size() {
            let mut request = valid_risk_request();
            request.bump_size_bps = 0.0;
            let result = validate_risk_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::BumpSizeNotPositive { .. } => {}
                _ => panic!("Expected BumpSizeNotPositive error"),
            }
        }

        #[test]
        fn test_validate_risk_request_negative_bump_size() {
            let mut request = valid_risk_request();
            request.bump_size_bps = -0.5;
            let result = validate_risk_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::BumpSizeNotPositive { .. } => {}
                _ => panic!("Expected BumpSizeNotPositive error"),
            }
        }

        #[test]
        fn test_validate_risk_request_nan_bump_size() {
            let mut request = valid_risk_request();
            request.bump_size_bps = f64::NAN;
            let result = validate_risk_request(&request);
            assert!(result.is_err());
        }
    }

    // =========================================================================
    // Tenor Parsing Tests (Task 1.4)
    // =========================================================================

    mod tenor_parsing_tests {
        use super::*;

        #[test]
        fn test_parse_tenor_to_years_valid() {
            assert!((parse_tenor_to_years("1Y").unwrap() - 1.0).abs() < 1e-10);
            assert!((parse_tenor_to_years("2Y").unwrap() - 2.0).abs() < 1e-10);
            assert!((parse_tenor_to_years("5Y").unwrap() - 5.0).abs() < 1e-10);
            assert!((parse_tenor_to_years("10Y").unwrap() - 10.0).abs() < 1e-10);
            assert!((parse_tenor_to_years("30Y").unwrap() - 30.0).abs() < 1e-10);
        }

        #[test]
        fn test_parse_tenor_to_years_case_insensitive() {
            assert!((parse_tenor_to_years("5y").unwrap() - 5.0).abs() < 1e-10);
            assert!((parse_tenor_to_years("10Y").unwrap() - 10.0).abs() < 1e-10);
        }

        #[test]
        fn test_parse_tenor_to_years_invalid_no_suffix() {
            let result = parse_tenor_to_years("5");
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::InvalidTenorFormat { tenor } => {
                    assert_eq!(tenor, "5");
                }
                _ => panic!("Expected InvalidTenorFormat error"),
            }
        }

        #[test]
        fn test_parse_tenor_to_years_invalid_wrong_suffix() {
            let result = parse_tenor_to_years("5M");
            assert!(result.is_err());
        }

        #[test]
        fn test_parse_tenor_to_years_invalid_non_numeric() {
            let result = parse_tenor_to_years("XY");
            assert!(result.is_err());
        }

        #[test]
        fn test_parse_tenor_to_years_invalid_empty() {
            let result = parse_tenor_to_years("");
            assert!(result.is_err());
        }

        #[test]
        fn test_parse_tenor_to_years_fractional() {
            assert!((parse_tenor_to_years("0.5Y").unwrap() - 0.5).abs() < 1e-10);
            assert!((parse_tenor_to_years("1.5Y").unwrap() - 1.5).abs() < 1e-10);
        }

        #[test]
        fn test_valid_tenors_constant() {
            assert_eq!(VALID_TENORS.len(), 9);
            assert!(VALID_TENORS.contains(&"1Y"));
            assert!(VALID_TENORS.contains(&"30Y"));
        }

        #[test]
        fn test_max_tenor_years_constant() {
            assert!((MAX_TENOR_YEARS - 50.0).abs() < 1e-10);
        }
    }

    // =========================================================================
    // CurveCache Tests (Task 1.5)
    // =========================================================================

    mod curve_cache_tests {
        use super::*;

        fn sample_cached_curve() -> CachedCurve {
            let par_rates = vec![
                ParRateInput {
                    tenor: "1Y".to_string(),
                    rate: 0.025,
                },
                ParRateInput {
                    tenor: "2Y".to_string(),
                    rate: 0.028,
                },
                ParRateInput {
                    tenor: "3Y".to_string(),
                    rate: 0.030,
                },
                ParRateInput {
                    tenor: "5Y".to_string(),
                    rate: 0.033,
                },
                ParRateInput {
                    tenor: "10Y".to_string(),
                    rate: 0.038,
                },
            ];
            CachedCurve::new(
                vec![1.0, 2.0, 3.0, 5.0, 10.0],
                vec![0.97, 0.94, 0.91, 0.85, 0.72],
                vec![0.0304, 0.0309, 0.0315, 0.0325, 0.0329],
                par_rates,
            )
        }

        #[test]
        fn test_cached_curve_new() {
            let curve = sample_cached_curve();
            assert_eq!(curve.pillars.len(), 5);
            assert_eq!(curve.discount_factors.len(), 5);
            assert_eq!(curve.zero_rates.len(), 5);
        }

        #[test]
        fn test_cached_curve_pillar_count() {
            let curve = sample_cached_curve();
            assert_eq!(curve.pillar_count(), 5);
        }

        #[test]
        fn test_cached_curve_age_seconds() {
            let curve = sample_cached_curve();
            // Age should be very small immediately after creation
            assert!(curve.age_seconds() < 2);
        }

        #[test]
        fn test_calculate_zero_rates() {
            let pillars = vec![1.0, 2.0, 5.0];
            let dfs = vec![0.97, 0.94, 0.85];
            let zero_rates = CachedCurve::calculate_zero_rates(&pillars, &dfs);

            // Zero rate = -ln(DF) / T
            // For T=1, DF=0.97: r = -ln(0.97) / 1 ≈ 0.0305
            assert!(zero_rates.len() == 3);
            assert!((zero_rates[0] - (-0.97_f64.ln())).abs() < 1e-10);
            assert!((zero_rates[1] - (-0.94_f64.ln() / 2.0)).abs() < 1e-10);
            assert!((zero_rates[2] - (-0.85_f64.ln() / 5.0)).abs() < 1e-10);
        }

        #[test]
        fn test_calculate_zero_rates_edge_cases() {
            // Zero time should return 0 rate
            let pillars = vec![0.0, 1.0];
            let dfs = vec![1.0, 0.97];
            let zero_rates = CachedCurve::calculate_zero_rates(&pillars, &dfs);
            assert_eq!(zero_rates[0], 0.0);
        }

        #[test]
        fn test_cache_new_is_empty() {
            let cache = BootstrapCurveCache::new();
            assert!(cache.is_empty());
            assert_eq!(cache.len(), 0);
        }

        #[test]
        fn test_cache_add_and_get() {
            let cache = BootstrapCurveCache::new();
            let curve = sample_cached_curve();
            let curve_id = Uuid::new_v4();

            cache.add(curve_id, curve.clone());

            assert!(!cache.is_empty());
            assert_eq!(cache.len(), 1);

            let retrieved = cache.get(&curve_id);
            assert!(retrieved.is_some());
            let retrieved_curve = retrieved.unwrap();
            assert_eq!(retrieved_curve.pillars.len(), curve.pillars.len());
            assert_eq!(
                retrieved_curve.discount_factors.len(),
                curve.discount_factors.len()
            );
        }

        #[test]
        fn test_cache_exists() {
            let cache = BootstrapCurveCache::new();
            let curve = sample_cached_curve();
            let curve_id = Uuid::new_v4();
            let non_existent_id = Uuid::new_v4();

            cache.add(curve_id, curve);

            assert!(cache.exists(&curve_id));
            assert!(!cache.exists(&non_existent_id));
        }

        #[test]
        fn test_cache_get_nonexistent() {
            let cache = BootstrapCurveCache::new();
            let curve_id = Uuid::new_v4();

            let result = cache.get(&curve_id);
            assert!(result.is_none());
        }

        #[test]
        fn test_cache_remove() {
            let cache = BootstrapCurveCache::new();
            let curve = sample_cached_curve();
            let curve_id = Uuid::new_v4();

            cache.add(curve_id, curve);
            assert!(cache.exists(&curve_id));

            let removed = cache.remove(&curve_id);
            assert!(removed.is_some());
            assert!(!cache.exists(&curve_id));
            assert!(cache.is_empty());
        }

        #[test]
        fn test_cache_remove_nonexistent() {
            let cache = BootstrapCurveCache::new();
            let curve_id = Uuid::new_v4();

            let removed = cache.remove(&curve_id);
            assert!(removed.is_none());
        }

        #[test]
        fn test_cache_clear() {
            let cache = BootstrapCurveCache::new();

            // Add multiple curves
            for _ in 0..5 {
                cache.add(Uuid::new_v4(), sample_cached_curve());
            }

            assert_eq!(cache.len(), 5);

            cache.clear();

            assert!(cache.is_empty());
            assert_eq!(cache.len(), 0);
        }

        #[test]
        fn test_cache_multiple_curves() {
            let cache = BootstrapCurveCache::new();
            let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();

            for id in &ids {
                cache.add(*id, sample_cached_curve());
            }

            assert_eq!(cache.len(), 3);

            for id in &ids {
                assert!(cache.exists(id));
            }
        }

        #[test]
        fn test_cache_overwrite_existing() {
            let cache = BootstrapCurveCache::new();
            let curve_id = Uuid::new_v4();

            // Add initial curve
            let par_rates1 = vec![ParRateInput {
                tenor: "1Y".to_string(),
                rate: 0.03,
            }];
            let curve1 = CachedCurve::new(vec![1.0], vec![0.97], vec![0.0304], par_rates1);
            cache.add(curve_id, curve1);
            assert_eq!(cache.len(), 1);

            // Overwrite with new curve
            let par_rates2 = vec![
                ParRateInput {
                    tenor: "1Y".to_string(),
                    rate: 0.03,
                },
                ParRateInput {
                    tenor: "2Y".to_string(),
                    rate: 0.031,
                },
            ];
            let curve2 = CachedCurve::new(
                vec![1.0, 2.0],
                vec![0.97, 0.94],
                vec![0.0304, 0.0309],
                par_rates2,
            );
            cache.add(curve_id, curve2);

            assert_eq!(cache.len(), 1);
            let retrieved = cache.get(&curve_id).unwrap();
            assert_eq!(retrieved.pillars.len(), 2);
        }

        #[test]
        fn test_cache_default() {
            let cache = BootstrapCurveCache::default();
            assert!(cache.is_empty());
        }
    }

    // =========================================================================
    // Greeks Compare Types Tests (Task 4.1)
    // =========================================================================

    mod greeks_compare_tests {
        use super::*;

        fn valid_greeks_compare_request() -> GreeksCompareRequest {
            GreeksCompareRequest {
                curve_id: "curve-001".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
                include_second_order: false,
            }
        }

        #[test]
        fn test_greeks_calculation_mode_serialization() {
            let bump_mode = GreeksCalculationMode::Bump;
            let json = serde_json::to_string(&bump_mode).unwrap();
            assert_eq!(json, "\"bump\"");

            let aad_mode = GreeksCalculationMode::Aad;
            let json = serde_json::to_string(&aad_mode).unwrap();
            assert_eq!(json, "\"aad\"");

            let compare_mode = GreeksCalculationMode::Compare;
            let json = serde_json::to_string(&compare_mode).unwrap();
            assert_eq!(json, "\"compare\"");
        }

        #[test]
        fn test_greeks_calculation_mode_default() {
            let mode = GreeksCalculationMode::default();
            assert_eq!(mode, GreeksCalculationMode::Bump);
        }

        #[test]
        fn test_greeks_compare_request_deserialize() {
            let json = r#"{
                "curveId": "curve-001",
                "notional": 10000000,
                "fixedRate": 0.03,
                "tenorYears": 5.0,
                "paymentFrequency": "annual",
                "bumpSizeBps": 1.0,
                "includeSecondOrder": true
            }"#;

            let request: GreeksCompareRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.curve_id, "curve-001");
            assert!((request.notional - 10_000_000.0).abs() < 1e-10);
            assert!((request.fixed_rate - 0.03).abs() < 1e-10);
            assert!((request.tenor_years - 5.0).abs() < 1e-10);
            assert_eq!(request.payment_frequency, PaymentFrequency::Annual);
            assert!((request.bump_size_bps - 1.0).abs() < 1e-10);
            assert!(request.include_second_order);
        }

        #[test]
        fn test_greeks_compare_request_default_bump_size() {
            let json = r#"{
                "curveId": "curve-001",
                "notional": 10000000,
                "fixedRate": 0.03,
                "tenorYears": 5.0,
                "paymentFrequency": "annual"
            }"#;

            let request: GreeksCompareRequest = serde_json::from_str(json).unwrap();
            assert!((request.bump_size_bps - 1.0).abs() < 1e-10);
            assert!(!request.include_second_order);
        }

        #[test]
        fn test_greek_value_with_delta() {
            let greeks = GreekValue::with_delta(0.5);
            assert!((greeks.delta.unwrap() - 0.5).abs() < 1e-10);
            assert!(greeks.gamma.is_none());
            assert!(greeks.vega.is_none());
            assert!(greeks.theta.is_none());
            assert!(greeks.rho.is_none());
        }

        #[test]
        fn test_greek_value_with_rho() {
            let greeks = GreekValue::with_rho(15.0);
            assert!(greeks.delta.is_none());
            assert!((greeks.rho.unwrap() - 15.0).abs() < 1e-10);
        }

        #[test]
        fn test_greek_value_builder_chain() {
            let greeks = GreekValue::with_delta(0.5)
                .and_gamma(0.02)
                .and_vega(25.0);

            assert!((greeks.delta.unwrap() - 0.5).abs() < 1e-10);
            assert!((greeks.gamma.unwrap() - 0.02).abs() < 1e-10);
            assert!((greeks.vega.unwrap() - 25.0).abs() < 1e-10);
        }

        #[test]
        fn test_greeks_method_result_serialize() {
            let result = GreeksMethodResult {
                npv: 125000.0,
                dv01: 4500.0,
                tenor_deltas: vec![
                    DeltaResult {
                        tenor: "1Y".to_string(),
                        delta: -1000.0,
                        processing_time_us: 100.0,
                    },
                    DeltaResult {
                        tenor: "5Y".to_string(),
                        delta: -3500.0,
                        processing_time_us: 120.0,
                    },
                ],
                greeks: GreekValue::with_rho(4500.0),
                mode: "bump".to_string(),
                timing: TimingStats {
                    mean_us: 110.0,
                    std_dev_us: 10.0,
                    min_us: 100.0,
                    max_us: 120.0,
                    total_ms: 0.22,
                },
            };

            let json = serde_json::to_string(&result).unwrap();
            assert!(json.contains("\"npv\":125000"));
            assert!(json.contains("\"dv01\":4500"));
            assert!(json.contains("\"mode\":\"bump\""));
        }

        #[test]
        fn test_greeks_diff_serialize() {
            let diff = GreeksDiff {
                npv_abs_error: 0.01,
                npv_rel_error_pct: 0.00008,
                dv01_abs_error: 0.001,
                dv01_rel_error_pct: 0.00002,
                tenor_diffs: vec![TenorDiff {
                    tenor: "5Y".to_string(),
                    bump_delta: -3500.0,
                    aad_delta: -3500.01,
                    abs_diff: 0.01,
                    rel_diff_pct: 0.00029,
                }],
                max_abs_error: 0.01,
                max_rel_error_pct: 0.00029,
            };

            let json = serde_json::to_string(&diff).unwrap();
            assert!(json.contains("\"npvAbsError\":"));
            assert!(json.contains("\"npvRelErrorPct\":"));
            assert!(json.contains("\"tenorDiffs\":"));
        }

        #[test]
        fn test_greeks_compare_response_serialize() {
            let bump_result = GreeksMethodResult {
                npv: 125000.0,
                dv01: 4500.0,
                tenor_deltas: vec![],
                greeks: GreekValue::default(),
                mode: "bump".to_string(),
                timing: TimingStats {
                    mean_us: 1000.0,
                    std_dev_us: 50.0,
                    min_us: 950.0,
                    max_us: 1050.0,
                    total_ms: 5.0,
                },
            };

            let aad_result = GreeksMethodResult {
                npv: 125000.0,
                dv01: 4500.0,
                tenor_deltas: vec![],
                greeks: GreekValue::default(),
                mode: "aad".to_string(),
                timing: TimingStats {
                    mean_us: 100.0,
                    std_dev_us: 5.0,
                    min_us: 95.0,
                    max_us: 105.0,
                    total_ms: 0.5,
                },
            };

            let response = GreeksCompareResponse {
                bump: bump_result,
                aad: aad_result,
                diff: GreeksDiff {
                    npv_abs_error: 0.0,
                    npv_rel_error_pct: 0.0,
                    dv01_abs_error: 0.0,
                    dv01_rel_error_pct: 0.0,
                    tenor_diffs: vec![],
                    max_abs_error: 0.0,
                    max_rel_error_pct: 0.0,
                },
                timing_comparison: TimingComparison {
                    bump_total_ms: 5.0,
                    aad_total_ms: Some(0.5),
                    speedup_ratio: Some(10.0),
                },
                within_tolerance: true,
                tolerance_pct: 0.01,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"bump\":{"));
            assert!(json.contains("\"aad\":{"));
            assert!(json.contains("\"withinTolerance\":true"));
            assert!(json.contains("\"speedupRatio\":10"));
        }

        #[test]
        fn test_validate_greeks_compare_request_valid() {
            let request = valid_greeks_compare_request();
            assert!(validate_greeks_compare_request(&request).is_ok());
        }

        #[test]
        fn test_validate_greeks_compare_request_invalid_notional() {
            let mut request = valid_greeks_compare_request();
            request.notional = -10_000_000.0;

            let result = validate_greeks_compare_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::NotionalNotPositive { .. } => {}
                _ => panic!("Expected NotionalNotPositive error"),
            }
        }

        #[test]
        fn test_validate_greeks_compare_request_invalid_fixed_rate() {
            let mut request = valid_greeks_compare_request();
            request.fixed_rate = 1.5;

            let result = validate_greeks_compare_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::FixedRateOutOfRange { .. } => {}
                _ => panic!("Expected FixedRateOutOfRange error"),
            }
        }

        #[test]
        fn test_validate_greeks_compare_request_invalid_tenor() {
            let mut request = valid_greeks_compare_request();
            request.tenor_years = -5.0;

            let result = validate_greeks_compare_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TenorYearsNotPositive { .. } => {}
                _ => panic!("Expected TenorYearsNotPositive error"),
            }
        }

        #[test]
        fn test_validate_greeks_compare_request_invalid_bump_size() {
            let mut request = valid_greeks_compare_request();
            request.bump_size_bps = -1.0;

            let result = validate_greeks_compare_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::BumpSizeNotPositive { .. } => {}
                _ => panic!("Expected BumpSizeNotPositive error"),
            }
        }
    }

    // =========================================================================
    // Task 4.2: First/Second Order Greeks Types Tests
    // =========================================================================

    mod first_second_order_greeks_tests {
        use super::*;

        fn valid_first_order_request() -> FirstOrderGreeksRequest {
            FirstOrderGreeksRequest {
                curve_id: "curve-001".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                mode: GreeksCalculationMode::Bump,
            }
        }

        fn valid_second_order_request() -> SecondOrderGreeksRequest {
            SecondOrderGreeksRequest {
                curve_id: "curve-001".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                mode: GreeksCalculationMode::Bump,
            }
        }

        #[test]
        fn test_first_order_request_deserialize() {
            let json = r#"{
                "curveId": "curve-001",
                "notional": 10000000,
                "fixedRate": 0.03,
                "tenorYears": 5.0,
                "paymentFrequency": "annual",
                "mode": "bump"
            }"#;

            let request: FirstOrderGreeksRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.curve_id, "curve-001");
            assert!((request.notional - 10_000_000.0).abs() < 1e-10);
            assert_eq!(request.mode, GreeksCalculationMode::Bump);
        }

        #[test]
        fn test_first_order_request_default_mode() {
            let json = r#"{
                "curveId": "curve-001",
                "notional": 10000000,
                "fixedRate": 0.03,
                "tenorYears": 5.0,
                "paymentFrequency": "annual"
            }"#;

            let request: FirstOrderGreeksRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.mode, GreeksCalculationMode::Bump);
        }

        #[test]
        fn test_first_order_response_serialize() {
            let response = FirstOrderGreeksResponse {
                npv: 125000.0,
                dv01: 4500.0,
                delta: 4500.0,
                vega: 0.0,
                rho: 4500.0,
                theta: -10.0,
                tenor_deltas: vec![DeltaResult {
                    tenor: "1Y".to_string(),
                    delta: -1000.0,
                    processing_time_us: 100.0,
                }],
                mode: "bump".to_string(),
                timing: TimingStats {
                    mean_us: 100.0,
                    std_dev_us: 10.0,
                    min_us: 90.0,
                    max_us: 110.0,
                    total_ms: 0.1,
                },
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"npv\":125000"));
            assert!(json.contains("\"dv01\":4500"));
            assert!(json.contains("\"delta\":4500"));
            assert!(json.contains("\"rho\":4500"));
            assert!(json.contains("\"theta\":-10"));
        }

        #[test]
        fn test_second_order_request_deserialize() {
            let json = r#"{
                "curveId": "curve-001",
                "notional": 10000000,
                "fixedRate": 0.03,
                "tenorYears": 5.0,
                "paymentFrequency": "annual",
                "mode": "aad"
            }"#;

            let request: SecondOrderGreeksRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.curve_id, "curve-001");
            assert_eq!(request.mode, GreeksCalculationMode::Aad);
        }

        #[test]
        fn test_second_order_response_serialize() {
            let response = SecondOrderGreeksResponse {
                npv: 125000.0,
                gamma: 50.0,
                vanna: 0.0,
                volga: 0.0,
                convexity: 0.0004,
                mode: "bump".to_string(),
                timing: TimingStats {
                    mean_us: 200.0,
                    std_dev_us: 20.0,
                    min_us: 180.0,
                    max_us: 220.0,
                    total_ms: 0.2,
                },
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"npv\":125000"));
            assert!(json.contains("\"gamma\":50"));
            assert!(json.contains("\"convexity\":0.0004"));
        }

        #[test]
        fn test_validate_first_order_request_valid() {
            let request = valid_first_order_request();
            assert!(validate_first_order_greeks_request(&request).is_ok());
        }

        #[test]
        fn test_validate_first_order_request_invalid_notional() {
            let mut request = valid_first_order_request();
            request.notional = -10_000_000.0;

            let result = validate_first_order_greeks_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::NotionalNotPositive { .. } => {}
                _ => panic!("Expected NotionalNotPositive error"),
            }
        }

        #[test]
        fn test_validate_first_order_request_invalid_tenor() {
            let mut request = valid_first_order_request();
            request.tenor_years = -5.0;

            let result = validate_first_order_greeks_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TenorYearsNotPositive { .. } => {}
                _ => panic!("Expected TenorYearsNotPositive error"),
            }
        }

        #[test]
        fn test_validate_second_order_request_valid() {
            let request = valid_second_order_request();
            assert!(validate_second_order_greeks_request(&request).is_ok());
        }

        #[test]
        fn test_validate_second_order_request_invalid_fixed_rate() {
            let mut request = valid_second_order_request();
            request.fixed_rate = 1.5;

            let result = validate_second_order_greeks_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::FixedRateOutOfRange { .. } => {}
                _ => panic!("Expected FixedRateOutOfRange error"),
            }
        }
    }

    // =========================================================================
    // Task 4.3: Bucket DV01 Types Tests
    // =========================================================================

    mod bucket_dv01_tests {
        use super::*;

        fn valid_bucket_dv01_request() -> BucketDv01Request {
            BucketDv01Request {
                curve_id: "curve-001".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                custom_tenors: None,
                include_key_rate_duration: false,
            }
        }

        #[test]
        fn test_bucket_dv01_request_deserialize() {
            let json = r#"{
                "curveId": "curve-001",
                "notional": 10000000,
                "fixedRate": 0.03,
                "tenorYears": 5.0,
                "paymentFrequency": "annual"
            }"#;

            let request: BucketDv01Request = serde_json::from_str(json).unwrap();
            assert_eq!(request.curve_id, "curve-001");
            assert!((request.notional - 10_000_000.0).abs() < 1e-10);
            assert!(request.custom_tenors.is_none());
            assert!(!request.include_key_rate_duration);
        }

        #[test]
        fn test_bucket_dv01_request_with_custom_tenors() {
            let json = r#"{
                "curveId": "curve-001",
                "notional": 10000000,
                "fixedRate": 0.03,
                "tenorYears": 5.0,
                "paymentFrequency": "annual",
                "customTenors": ["1Y", "2Y", "5Y"],
                "includeKeyRateDuration": true
            }"#;

            let request: BucketDv01Request = serde_json::from_str(json).unwrap();
            assert!(request.custom_tenors.is_some());
            let tenors = request.custom_tenors.unwrap();
            assert_eq!(tenors.len(), 3);
            assert!(request.include_key_rate_duration);
        }

        #[test]
        fn test_bucket_dv01_result_serialize() {
            let result = BucketDv01Result {
                tenor: "5Y".to_string(),
                dv01: 450.0,
                key_rate_duration: Some(0.9),
                pct_of_total: 50.0,
            };

            let json = serde_json::to_string(&result).unwrap();
            assert!(json.contains("\"tenor\":\"5Y\""));
            assert!(json.contains("\"dv01\":450"));
            assert!(json.contains("\"keyRateDuration\":0.9"));
            assert!(json.contains("\"pctOfTotal\":50"));
        }

        #[test]
        fn test_bucket_dv01_result_serialize_without_krd() {
            let result = BucketDv01Result {
                tenor: "5Y".to_string(),
                dv01: 450.0,
                key_rate_duration: None,
                pct_of_total: 50.0,
            };

            let json = serde_json::to_string(&result).unwrap();
            assert!(json.contains("\"dv01\":450"));
            assert!(!json.contains("keyRateDuration"));
        }

        #[test]
        fn test_bucket_dv01_response_serialize() {
            let response = BucketDv01Response {
                npv: 125000.0,
                total_dv01: 900.0,
                buckets: vec![
                    BucketDv01Result {
                        tenor: "1Y".to_string(),
                        dv01: 200.0,
                        key_rate_duration: None,
                        pct_of_total: 22.22,
                    },
                    BucketDv01Result {
                        tenor: "5Y".to_string(),
                        dv01: 700.0,
                        key_rate_duration: None,
                        pct_of_total: 77.78,
                    },
                ],
                buckets_consistent: true,
                timing: TimingStats {
                    mean_us: 50.0,
                    std_dev_us: 5.0,
                    min_us: 0.0,
                    max_us: 100.0,
                    total_ms: 0.1,
                },
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"npv\":125000"));
            assert!(json.contains("\"totalDv01\":900"));
            assert!(json.contains("\"bucketsConsistent\":true"));
        }

        #[test]
        fn test_bucket_tenors_constant() {
            assert_eq!(BUCKET_TENORS.len(), 9);
            assert_eq!(BUCKET_TENORS[0], "1M");
            assert_eq!(BUCKET_TENORS[8], "30Y");
        }

        #[test]
        fn test_validate_bucket_dv01_request_valid() {
            let request = valid_bucket_dv01_request();
            assert!(validate_bucket_dv01_request(&request).is_ok());
        }

        #[test]
        fn test_validate_bucket_dv01_request_invalid_notional() {
            let mut request = valid_bucket_dv01_request();
            request.notional = -10_000_000.0;

            let result = validate_bucket_dv01_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::NotionalNotPositive { .. } => {}
                _ => panic!("Expected NotionalNotPositive error"),
            }
        }

        #[test]
        fn test_validate_bucket_dv01_request_invalid_tenor() {
            let mut request = valid_bucket_dv01_request();
            request.tenor_years = 100.0;

            let result = validate_bucket_dv01_request(&request);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TenorYearsExceedsMax { .. } => {}
                _ => panic!("Expected TenorYearsExceedsMax error"),
            }
        }
    }

    // =========================================================================
    // Task 5.1: Greeks Heatmap Types Tests
    // =========================================================================

    mod greeks_heatmap_tests {
        use super::*;

        #[test]
        fn test_greek_type_default_is_delta() {
            let greek_type = GreekType::default();
            assert_eq!(greek_type, GreekType::Delta);
        }

        #[test]
        fn test_greek_type_display() {
            assert_eq!(GreekType::Delta.to_string(), "delta");
            assert_eq!(GreekType::Gamma.to_string(), "gamma");
            assert_eq!(GreekType::Vega.to_string(), "vega");
            assert_eq!(GreekType::Theta.to_string(), "theta");
            assert_eq!(GreekType::Rho.to_string(), "rho");
            assert_eq!(GreekType::Vanna.to_string(), "vanna");
            assert_eq!(GreekType::Volga.to_string(), "volga");
        }

        #[test]
        fn test_greek_type_serialisation() {
            let json = serde_json::to_string(&GreekType::Delta).unwrap();
            assert_eq!(json, "\"delta\"");

            let json = serde_json::to_string(&GreekType::Gamma).unwrap();
            assert_eq!(json, "\"gamma\"");
        }

        #[test]
        fn test_greek_type_deserialisation() {
            let greek: GreekType = serde_json::from_str("\"delta\"").unwrap();
            assert_eq!(greek, GreekType::Delta);

            let greek: GreekType = serde_json::from_str("\"vega\"").unwrap();
            assert_eq!(greek, GreekType::Vega);
        }

        #[test]
        fn test_heatmap_request_default() {
            let request = GreeksHeatmapRequest::default();
            assert_eq!(request.greek_type, GreekType::Delta);
            assert!((request.spot - 100.0).abs() < 1e-10);
            assert!((request.rate - 0.05).abs() < 1e-10);
            assert!((request.volatility - 0.20).abs() < 1e-10);
            assert_eq!(request.option_type, OptionType::Call);
        }

        #[test]
        fn test_heatmap_request_deserialisation() {
            let json = r#"{
                "greekType": "gamma",
                "spot": 110.0,
                "rate": 0.03,
                "volatility": 0.25,
                "optionType": "put"
            }"#;

            let request: GreeksHeatmapRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.greek_type, GreekType::Gamma);
            assert!((request.spot - 110.0).abs() < 1e-10);
            assert!((request.rate - 0.03).abs() < 1e-10);
            assert!((request.volatility - 0.25).abs() < 1e-10);
            assert_eq!(request.option_type, OptionType::Put);
        }

        #[test]
        fn test_heatmap_request_deserialisation_with_defaults() {
            let json = r#"{}"#;

            let request: GreeksHeatmapRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.greek_type, GreekType::Delta);
            assert!((request.spot - 100.0).abs() < 1e-10);
        }

        #[test]
        fn test_heatmap_response_serialisation() {
            let response = GreeksHeatmapResponse {
                x_axis: vec!["0.25".to_string(), "0.5".to_string(), "1.0".to_string()],
                y_axis: vec!["80%".to_string(), "100%".to_string(), "120%".to_string()],
                values: vec![
                    vec![0.3, 0.4, 0.5],
                    vec![0.5, 0.55, 0.6],
                    vec![0.7, 0.75, 0.8],
                ],
                greek_type: "delta".to_string(),
                spot: 100.0,
                rate: 0.05,
                volatility: 0.20,
                option_type: "call".to_string(),
                min_value: 0.3,
                max_value: 0.8,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"xAxis\""));
            assert!(json.contains("\"yAxis\""));
            assert!(json.contains("\"values\""));
            assert!(json.contains("\"greekType\":\"delta\""));
            assert!(json.contains("\"minValue\":0.3"));
            assert!(json.contains("\"maxValue\":0.8"));
        }

        #[test]
        fn test_heatmap_response_d3_compatible_structure() {
            // D3.js heatmap expects: x_axis (columns), y_axis (rows), values[row][col]
            let response = GreeksHeatmapResponse {
                x_axis: vec!["0.25".to_string(), "0.5".to_string()],
                y_axis: vec!["80%".to_string(), "100%".to_string()],
                values: vec![
                    vec![0.3, 0.4],  // row 0 (80%)
                    vec![0.5, 0.55], // row 1 (100%)
                ],
                greek_type: "delta".to_string(),
                spot: 100.0,
                rate: 0.05,
                volatility: 0.20,
                option_type: "call".to_string(),
                min_value: 0.3,
                max_value: 0.55,
            };

            // Verify structure matches D3.js expectations
            assert_eq!(response.x_axis.len(), 2); // columns
            assert_eq!(response.y_axis.len(), 2); // rows
            assert_eq!(response.values.len(), response.y_axis.len());
            for row in &response.values {
                assert_eq!(row.len(), response.x_axis.len());
            }
        }
    }

    // =========================================================================
    // Task 5.2: Greeks Timeseries Types Tests
    // =========================================================================

    mod greeks_timeseries_tests {
        use super::*;

        #[test]
        fn test_timeseries_request_default() {
            let request = GreeksTimeseriesRequest::default();
            assert_eq!(
                request.greek_types,
                vec![GreekType::Delta, GreekType::Gamma, GreekType::Theta]
            );
            assert!((request.spot - 100.0).abs() < 1e-10);
            assert!((request.strike - 100.0).abs() < 1e-10);
            assert!((request.rate - 0.05).abs() < 1e-10);
            assert!((request.volatility - 0.20).abs() < 1e-10);
            assert_eq!(request.option_type, OptionType::Call);
            assert!((request.time_horizon - 1.0).abs() < 1e-10);
            assert_eq!(request.num_points, 50);
        }

        #[test]
        fn test_timeseries_request_deserialisation() {
            let json = r#"{
                "greekTypes": ["delta", "vega"],
                "spot": 105.0,
                "strike": 100.0,
                "rate": 0.04,
                "volatility": 0.30,
                "optionType": "call",
                "timeHorizon": 2.0,
                "numPoints": 100
            }"#;

            let request: GreeksTimeseriesRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.greek_types, vec![GreekType::Delta, GreekType::Vega]);
            assert!((request.spot - 105.0).abs() < 1e-10);
            assert!((request.strike - 100.0).abs() < 1e-10);
            assert!((request.time_horizon - 2.0).abs() < 1e-10);
            assert_eq!(request.num_points, 100);
        }

        #[test]
        fn test_timeseries_request_deserialisation_with_defaults() {
            let json = r#"{}"#;

            let request: GreeksTimeseriesRequest = serde_json::from_str(json).unwrap();
            assert_eq!(
                request.greek_types,
                vec![GreekType::Delta, GreekType::Gamma, GreekType::Theta]
            );
            assert_eq!(request.num_points, 50);
        }

        #[test]
        fn test_timeseries_series_serialisation() {
            let series = TimeseriesSeries {
                greek_type: "delta".to_string(),
                values: vec![0.5, 0.52, 0.54, 0.56],
            };

            let json = serde_json::to_string(&series).unwrap();
            assert!(json.contains("\"greekType\":\"delta\""));
            assert!(json.contains("\"values\":[0.5,0.52,0.54,0.56]"));
        }

        #[test]
        fn test_timeseries_response_serialisation() {
            let response = GreeksTimeseriesResponse {
                timestamps: vec![365.0, 180.0, 90.0, 30.0, 1.0],
                series: vec![
                    TimeseriesSeries {
                        greek_type: "delta".to_string(),
                        values: vec![0.5, 0.52, 0.55, 0.6, 0.9],
                    },
                    TimeseriesSeries {
                        greek_type: "gamma".to_string(),
                        values: vec![0.02, 0.025, 0.03, 0.04, 0.1],
                    },
                ],
                spot: 100.0,
                strike: 100.0,
                rate: 0.05,
                volatility: 0.20,
                option_type: "call".to_string(),
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"timestamps\""));
            assert!(json.contains("\"series\""));
            assert!(json.contains("\"greekType\":\"delta\""));
            assert!(json.contains("\"greekType\":\"gamma\""));
        }

        #[test]
        fn test_timeseries_response_structure() {
            let response = GreeksTimeseriesResponse {
                timestamps: vec![365.0, 180.0, 90.0],
                series: vec![TimeseriesSeries {
                    greek_type: "delta".to_string(),
                    values: vec![0.5, 0.52, 0.55],
                }],
                spot: 100.0,
                strike: 100.0,
                rate: 0.05,
                volatility: 0.20,
                option_type: "call".to_string(),
            };

            // Each series should have same length as timestamps
            for series in &response.series {
                assert_eq!(series.values.len(), response.timestamps.len());
            }
        }
    }
}
