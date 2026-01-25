//! Risk Engine API types.
//!
//! Provides request and response types for the RiskEngine API endpoints.
//!
//! # Requirements Coverage
//!
//! - Requirement 9.2: serde-compatible configuration types
//! - Requirement 9.3: JSON-serializable response types

use serde::{Deserialize, Serialize};

// =============================================================================
// Task 8.2: Request Types
// =============================================================================

/// Request to compute Greeks for a single trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreeksRequest {
    /// Trade identifier.
    pub trade_id: String,

    /// Instrument type (e.g., "vanilla_call", "vanilla_put").
    pub instrument_type: String,

    /// Spot price.
    pub spot: f64,

    /// Strike price.
    pub strike: f64,

    /// Time to expiry in years.
    pub expiry: f64,

    /// Volatility (as decimal, e.g., 0.20 for 20%).
    pub volatility: f64,

    /// Risk-free rate (as decimal).
    pub rate: f64,

    /// Optional dividend yield (as decimal).
    #[serde(default)]
    pub dividend_yield: f64,

    /// Optional risk configuration overrides.
    #[serde(default)]
    pub risk_config: Option<RiskConfigOverride>,
}

/// Request to compute Greeks for a portfolio of trades.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioGreeksRequest {
    /// List of trades in the portfolio.
    pub trades: Vec<GreeksRequest>,

    /// Optional risk configuration overrides.
    #[serde(default)]
    pub risk_config: Option<RiskConfigOverride>,

    /// Whether to run as async job (for large portfolios).
    #[serde(default)]
    pub async_mode: bool,
}

/// Risk configuration overrides for API requests.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskConfigOverride {
    /// Greeks calculation method ("aad" or "bump").
    #[serde(default)]
    pub greeks_method: Option<String>,

    /// Target Greeks to compute.
    #[serde(default)]
    pub target_greeks: Option<Vec<String>>,

    /// Custom bump sizes.
    #[serde(default)]
    pub bump_sizes: Option<BumpSizesOverride>,

    /// Second-order calculation mode ("parallel" or "serial").
    #[serde(default)]
    pub second_order_mode: Option<String>,
}

/// Bump sizes override for API requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BumpSizesOverride {
    /// Rate bump (default: 0.0001 = 1bp).
    #[serde(default)]
    pub rate: Option<f64>,

    /// Volatility bump (default: 0.01 = 1 vol point).
    #[serde(default)]
    pub vol: Option<f64>,

    /// Spot bump (default: 0.01 = 1%).
    #[serde(default)]
    pub spot: Option<f64>,
}

/// Request for scenario-based Greeks calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioGreeksRequest {
    /// Base Greeks request.
    #[serde(flatten)]
    pub base: GreeksRequest,

    /// Scenario name.
    pub scenario_name: String,

    /// Market shifts to apply.
    pub shifts: MarketShifts,
}

/// Market shifts for scenario analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketShifts {
    /// Spot price shift (absolute).
    #[serde(default)]
    pub spot_shift: f64,

    /// Spot price shift (relative, e.g., 0.10 for +10%).
    #[serde(default)]
    pub spot_shift_relative: f64,

    /// Volatility shift (absolute, e.g., 0.05 for +5 vol points).
    #[serde(default)]
    pub vol_shift: f64,

    /// Rate shift (absolute, e.g., 0.01 for +100bp).
    #[serde(default)]
    pub rate_shift: f64,
}

// =============================================================================
// Task 8.2: Response Types
// =============================================================================

/// Response containing computed Greeks for a single trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreeksResponse {
    /// Trade identifier.
    pub trade_id: String,

    /// Present value of the trade.
    pub pv: f64,

    /// Computed Greeks.
    pub greeks: ComputedGreeksDto,

    /// Calculation method used.
    pub method: String,

    /// Execution metrics.
    pub metrics: ExecutionMetricsDto,
}

/// Computed Greeks DTO.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputedGreeksDto {
    /// Delta (dV/dS).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<f64>,

    /// Gamma (d2V/dS2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamma: Option<f64>,

    /// Vega (dV/dσ).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vega: Option<f64>,

    /// Theta (dV/dt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta: Option<f64>,

    /// Rho (dV/dr).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rho: Option<f64>,

    /// Vanna (d2V/dSdσ).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vanna: Option<f64>,

    /// Volga (d2V/dσ2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volga: Option<f64>,
}

/// Execution metrics DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetricsDto {
    /// Computation time in milliseconds.
    pub computation_time_ms: f64,
}

/// Response for portfolio Greeks calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioGreeksResponse {
    /// Individual trade results.
    pub results: Vec<GreeksResponse>,

    /// Failed calculations.
    pub failures: Vec<FailedCalculationDto>,

    /// Aggregated Greeks across the portfolio.
    pub aggregations: AggregatedGreeksDto,

    /// Execution statistics.
    pub stats: ExecutionStatsDto,
}

/// Failed calculation DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedCalculationDto {
    /// Trade identifier.
    pub trade_id: String,

    /// Error message.
    pub error: String,
}

/// Aggregated Greeks DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedGreeksDto {
    /// Total PV across all trades.
    pub total_pv: f64,

    /// Total delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_delta: Option<f64>,

    /// Total gamma.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_gamma: Option<f64>,

    /// Total vega.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_vega: Option<f64>,

    /// Total theta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_theta: Option<f64>,

    /// Total rho.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_rho: Option<f64>,
}

/// Execution statistics DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatsDto {
    /// Total number of trades processed.
    pub total: usize,

    /// Number of successful calculations.
    pub successful: usize,

    /// Number of failed calculations.
    pub failed: usize,

    /// Total execution time in milliseconds.
    pub elapsed_ms: f64,

    /// Whether parallel processing was used.
    pub used_parallel: bool,
}

/// Response for scenario-based Greeks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioGreeksResponse {
    /// Scenario name.
    pub scenario_name: String,

    /// Greeks result under this scenario.
    pub result: GreeksResponse,
}

/// Error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskErrorResponse {
    /// Error code.
    pub error: String,

    /// Error message.
    pub message: String,

    /// Additional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

// =============================================================================
// Conversions from pricer_risk types
// =============================================================================

impl From<pricer_risk::ComputedGreeks> for ComputedGreeksDto {
    fn from(g: pricer_risk::ComputedGreeks) -> Self {
        Self {
            delta: g.delta,
            gamma: g.gamma,
            vega: g.vega,
            theta: g.theta,
            rho: g.rho,
            vanna: g.vanna,
            volga: g.volga,
        }
    }
}

impl From<pricer_risk::PerformanceMetrics> for ExecutionMetricsDto {
    fn from(m: pricer_risk::PerformanceMetrics) -> Self {
        Self {
            computation_time_ms: m.computation_time_ms,
        }
    }
}

impl From<pricer_risk::RiskResult> for GreeksResponse {
    fn from(r: pricer_risk::RiskResult) -> Self {
        Self {
            trade_id: r.trade_id,
            pv: r.pv,
            greeks: r.greeks.into(),
            method: format!("{:?}", r.method),
            metrics: r.metrics.into(),
        }
    }
}

impl From<pricer_risk::FailedCalculation> for FailedCalculationDto {
    fn from(f: pricer_risk::FailedCalculation) -> Self {
        Self {
            trade_id: f.trade_id,
            error: f.error_message,
        }
    }
}

impl From<pricer_risk::ExecutionStats> for ExecutionStatsDto {
    fn from(s: pricer_risk::ExecutionStats) -> Self {
        Self {
            total: s.total_trades,
            successful: s.successful,
            failed: s.failed,
            elapsed_ms: s.total_time_ms,
            used_parallel: s.used_parallel,
        }
    }
}

impl From<pricer_risk::PortfolioRiskResult> for PortfolioGreeksResponse {
    fn from(r: pricer_risk::PortfolioRiskResult) -> Self {
        // Calculate total_pv from results
        let total_pv: f64 = r.results.iter().map(|result| result.pv).sum();

        Self {
            results: r.results.clone().into_iter().map(Into::into).collect(),
            failures: r.failures.into_iter().map(Into::into).collect(),
            aggregations: AggregatedGreeksDto {
                total_pv,
                total_delta: r.aggregations.total.delta,
                total_gamma: r.aggregations.total.gamma,
                total_vega: r.aggregations.total.vega,
                total_theta: r.aggregations.total.theta,
                total_rho: r.aggregations.total.rho,
            },
            stats: r.stats.into(),
        }
    }
}

impl From<pricer_risk::ScenarioGreeksResult> for ScenarioGreeksResponse {
    fn from(r: pricer_risk::ScenarioGreeksResult) -> Self {
        Self {
            scenario_name: r.scenario_name,
            result: r.result.into(),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greeks_request_serialization() {
        let request = GreeksRequest {
            trade_id: "T001".to_string(),
            instrument_type: "vanilla_call".to_string(),
            spot: 100.0,
            strike: 100.0,
            expiry: 1.0,
            volatility: 0.20,
            rate: 0.05,
            dividend_yield: 0.0,
            risk_config: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"trade_id\":\"T001\""));
        assert!(json.contains("\"spot\":100.0"));

        let deserialized: GreeksRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.trade_id, "T001");
    }

    #[test]
    fn test_greeks_response_serialization() {
        let response = GreeksResponse {
            trade_id: "T001".to_string(),
            pv: 10.5,
            greeks: ComputedGreeksDto {
                delta: Some(0.5),
                gamma: Some(0.02),
                vega: Some(0.25),
                theta: Some(-0.01),
                rho: Some(0.15),
                vanna: None,
                volga: None,
            },
            method: "Bump".to_string(),
            metrics: ExecutionMetricsDto {
                computation_time_ms: 1.5,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"delta\":0.5"));
        assert!(json.contains("\"method\":\"Bump\""));
        // vanna should not be present (skip_serializing_if = None)
        assert!(!json.contains("\"vanna\""));
    }

    #[test]
    fn test_portfolio_greeks_response_serialization() {
        let response = PortfolioGreeksResponse {
            results: vec![],
            failures: vec![FailedCalculationDto {
                trade_id: "T002".to_string(),
                error: "Market data missing".to_string(),
            }],
            aggregations: AggregatedGreeksDto {
                total_pv: 1000.0,
                total_delta: Some(0.75),
                total_gamma: None,
                total_vega: Some(5.0),
                total_theta: None,
                total_rho: None,
            },
            stats: ExecutionStatsDto {
                total: 10,
                successful: 9,
                failed: 1,
                elapsed_ms: 50.0,
                used_parallel: true,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total_pv\":1000.0"));
        assert!(json.contains("\"used_parallel\":true"));
    }

    #[test]
    fn test_risk_config_override_defaults() {
        let json = r#"{}"#;
        let config: RiskConfigOverride = serde_json::from_str(json).unwrap();
        assert!(config.greeks_method.is_none());
        assert!(config.target_greeks.is_none());
        assert!(config.bump_sizes.is_none());
    }

    #[test]
    fn test_market_shifts_defaults() {
        let shifts = MarketShifts::default();
        assert_eq!(shifts.spot_shift, 0.0);
        assert_eq!(shifts.vol_shift, 0.0);
        assert_eq!(shifts.rate_shift, 0.0);
    }
}
