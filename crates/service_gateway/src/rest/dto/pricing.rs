//! Pricing-related DTOs

use serde::{Deserialize, Serialize};

/// Instrument type for pricing
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentType {
    VanillaOption,
    EuropeanOption,
    Forward,
    Swap,
    Fra,
}

/// Pricing request for a single instrument
#[derive(Debug, Clone, Deserialize)]
pub struct PricingRequest {
    /// Type of instrument
    pub instrument_type: InstrumentType,
    /// Strike price
    pub strike: f64,
    /// Time to expiry in years
    pub expiry: f64,
    /// Whether the option is a call (true) or put (false)
    #[serde(default = "default_is_call")]
    pub is_call: bool,
    /// Spot price
    pub spot: f64,
    /// Volatility (annualised)
    pub volatility: f64,
    /// Risk-free rate (annualised)
    pub rate: f64,
    /// Dividend yield (optional)
    #[serde(default)]
    pub dividend_yield: f64,
    /// Whether to compute Greeks
    #[serde(default)]
    pub compute_greeks: bool,
}

fn default_is_call() -> bool {
    true
}

/// Greeks computed for an instrument
#[derive(Debug, Clone, Serialize)]
pub struct GreeksResponse {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
}

/// Pricing response for a single instrument
#[derive(Debug, Clone, Serialize)]
pub struct PricingResponse {
    /// Calculated price (present value)
    pub price: f64,
    /// Greeks if computed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeks: Option<GreeksResponse>,
    /// Calculation time in milliseconds
    pub calculation_time_ms: f64,
}

/// Portfolio pricing request
#[derive(Debug, Clone, Deserialize)]
pub struct PortfolioPricingRequest {
    /// List of instruments to price
    pub instruments: Vec<PricingRequest>,
    /// Whether to compute Greeks for all instruments
    #[serde(default)]
    pub compute_greeks: bool,
}

/// Single instrument result in portfolio
#[derive(Debug, Clone, Serialize)]
pub struct PortfolioInstrumentResult {
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeks: Option<GreeksResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Portfolio pricing response
#[derive(Debug, Clone, Serialize)]
pub struct PortfolioPricingResponse {
    /// Results for each instrument
    pub results: Vec<PortfolioInstrumentResult>,
    /// Total portfolio value
    pub total_value: f64,
    /// Number of successful pricings
    pub success_count: usize,
    /// Number of failed pricings
    pub failure_count: usize,
    /// Total calculation time in milliseconds
    pub calculation_time_ms: f64,
}

/// Health check response
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}
