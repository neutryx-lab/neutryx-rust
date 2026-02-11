//! Pricing-related DTOs.

use serde::{Deserialize, Serialize};
use validator::Validate;

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Instrument type for pricing.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum InstrumentType {
    /// Vanilla option (American or European).
    VanillaOption,
    /// European-style option.
    EuropeanOption,
    /// Forward contract.
    Forward,
    /// Interest rate swap.
    Swap,
    /// Forward rate agreement.
    Fra,
}

/// Pricing request for a single instrument.
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PricingRequest {
    /// Type of instrument.
    pub instrument_type: InstrumentType,
    /// Strike price.
    #[validate(range(exclusive_min = 0.0))]
    pub strike: f64,
    /// Time to expiry in years.
    #[validate(range(exclusive_min = 0.0))]
    pub expiry: f64,
    /// Whether the option is a call (true) or put (false).
    #[serde(default = "default_is_call")]
    pub is_call: bool,
    /// Spot price.
    #[validate(range(exclusive_min = 0.0))]
    pub spot: f64,
    /// Volatility (annualised).
    #[validate(range(min = 0.0))]
    pub volatility: f64,
    /// Risk-free rate (annualised).
    pub rate: f64,
    /// Dividend yield (optional).
    #[serde(default)]
    pub dividend_yield: f64,
    /// Whether to compute Greeks.
    #[serde(default)]
    pub compute_greeks: bool,
}

fn default_is_call() -> bool { true }

/// Greeks computed for an instrument.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct GreeksResponse {
    /// Delta (dV/dS).
    pub delta: f64,
    /// Gamma (d²V/dS²).
    pub gamma: f64,
    /// Vega (dV/dσ).
    pub vega: f64,
    /// Theta (dV/dt).
    pub theta: f64,
    /// Rho (dV/dr).
    pub rho: f64,
}

/// Pricing response for a single instrument.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PricingResponse {
    /// Calculated price (present value).
    pub price: f64,
    /// Greeks if computed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeks: Option<GreeksResponse>,
    /// Calculation time in milliseconds.
    pub calculation_time_ms: f64,
}

/// Portfolio pricing request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PortfolioPricingRequest {
    /// List of instruments to price.
    #[validate(length(min = 1))]
    #[validate(nested)]
    pub instruments: Vec<PricingRequest>,
    /// Whether to compute Greeks for all instruments.
    #[serde(default)]
    pub compute_greeks: bool,
}

/// Single instrument result in portfolio.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PortfolioInstrumentResult {
    /// Calculated price (present value).
    pub price: f64,
    /// Greeks if computed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeks: Option<GreeksResponse>,
    /// Error message if pricing failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Portfolio pricing response.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PortfolioPricingResponse {
    /// Results for each instrument.
    pub results: Vec<PortfolioInstrumentResult>,
    /// Total portfolio value.
    pub total_value: f64,
    /// Number of successful pricings.
    pub success_count: usize,
    /// Number of failed pricings.
    pub failure_count: usize,
    /// Total calculation time in milliseconds.
    pub calculation_time_ms: f64,
}

/// Health check response.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct HealthResponse {
    /// Service status (e.g.
    pub status: String,
    /// Service version.
    pub version: String,
}
