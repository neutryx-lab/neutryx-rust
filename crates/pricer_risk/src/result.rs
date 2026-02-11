//! Risk engine result types.

use std::collections::HashMap;

use infra_config::GreeksMethod;
use serde::{Deserialize, Serialize};

use crate::{greeks::GreeksResult, scenarios::RiskFactorId};

/// Computed Greeks for a single trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedGreeks {
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub theta: Option<f64>,
    pub rho: Option<f64>,
    pub vanna: Option<f64>,
    pub volga: Option<f64>,
}

impl ComputedGreeks {
    /// Creates an empty ComputedGreeks.
    pub fn empty() -> Self {
        Self {
            delta: None,
            gamma: None,
            vega: None,
            theta: None,
            rho: None,
            vanna: None,
            volga: None,
        }
    }

    /// Creates ComputedGreeks from a GreeksResult.
    pub fn from_greeks_result(result: &GreeksResult<f64>) -> Self {
        Self {
            delta: result.delta,
            gamma: result.gamma,
            vega: result.vega,
            theta: result.theta,
            rho: result.rho,
            vanna: result.vanna,
            volga: result.volga,
        }
    }

    /// Returns true if any Greek is computed.
    pub fn has_any(&self) -> bool {
        self.delta.is_some()
            || self.gamma.is_some()
            || self.vega.is_some()
            || self.theta.is_some()
            || self.rho.is_some()
            || self.vanna.is_some()
            || self.volga.is_some()
    }

    /// Returns the number of computed Greeks.
    pub fn count(&self) -> usize {
        [
            self.delta.is_some(),
            self.gamma.is_some(),
            self.vega.is_some(),
            self.theta.is_some(),
            self.rho.is_some(),
            self.vanna.is_some(),
            self.volga.is_some(),
        ]
        .iter()
        .filter(|&&v| v)
        .count()
    }
}

impl Default for ComputedGreeks {
    fn default() -> Self { Self::empty() }
}

/// Performance metrics for a calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub computation_time_ms: f64,
    pub memory_usage_bytes: Option<usize>,
}

impl PerformanceMetrics {
    /// Creates new performance metrics.
    pub fn new(computation_time_ms: f64) -> Self {
        Self {
            computation_time_ms,
            memory_usage_bytes: None,
        }
    }

    /// Creates metrics with memory usage.
    pub fn with_memory(computation_time_ms: f64, memory_usage_bytes: usize) -> Self {
        Self {
            computation_time_ms,
            memory_usage_bytes: Some(memory_usage_bytes),
        }
    }
}

/// Result of a single trade risk calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskResult {
    pub trade_id: String,
    pub pv: f64,
    pub greeks: ComputedGreeks,
    pub method: GreeksMethod,
    pub metrics: PerformanceMetrics,
}

impl RiskResult {
    /// Creates a new RiskResult.
    pub fn new(
        trade_id: impl Into<String>,
        pv: f64,
        greeks: ComputedGreeks,
        method: GreeksMethod,
        computation_time_ms: f64,
    ) -> Self {
        Self {
            trade_id: trade_id.into(),
            pv,
            greeks,
            method,
            metrics: PerformanceMetrics::new(computation_time_ms),
        }
    }
}

/// Aggregated Greeks across a portfolio.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregatedGreeks {
    pub by_risk_factor: HashMap<RiskFactorId, ComputedGreeks>,
    pub by_currency: HashMap<String, ComputedGreeks>,
    pub by_tenor_bucket: HashMap<String, ComputedGreeks>,
    pub total: ComputedGreeks,
}

impl AggregatedGreeks {
    /// Creates empty aggregated Greeks.
    pub fn empty() -> Self { Self::default() }

    /// Creates aggregated Greeks from individual results.
    pub fn from_results(results: &[RiskResult]) -> Self {
        let mut total_delta = 0.0;
        let mut total_gamma = 0.0;
        let mut total_vega = 0.0;
        let mut total_theta = 0.0;
        let mut total_rho = 0.0;
        let mut total_vanna = 0.0;
        let mut total_volga = 0.0;

        let mut has_delta = false;
        let mut has_gamma = false;
        let mut has_vega = false;
        let mut has_theta = false;
        let mut has_rho = false;
        let mut has_vanna = false;
        let mut has_volga = false;

        for result in results {
            if let Some(d) = result.greeks.delta {
                total_delta += d;
                has_delta = true;
            }
            if let Some(g) = result.greeks.gamma {
                total_gamma += g;
                has_gamma = true;
            }
            if let Some(v) = result.greeks.vega {
                total_vega += v;
                has_vega = true;
            }
            if let Some(t) = result.greeks.theta {
                total_theta += t;
                has_theta = true;
            }
            if let Some(r) = result.greeks.rho {
                total_rho += r;
                has_rho = true;
            }
            if let Some(va) = result.greeks.vanna {
                total_vanna += va;
                has_vanna = true;
            }
            if let Some(vo) = result.greeks.volga {
                total_volga += vo;
                has_volga = true;
            }
        }

        Self {
            by_risk_factor: HashMap::new(),
            by_currency: HashMap::new(),
            by_tenor_bucket: HashMap::new(),
            total: ComputedGreeks {
                delta: if has_delta { Some(total_delta) } else { None },
                gamma: if has_gamma { Some(total_gamma) } else { None },
                vega: if has_vega { Some(total_vega) } else { None },
                theta: if has_theta { Some(total_theta) } else { None },
                rho: if has_rho { Some(total_rho) } else { None },
                vanna: if has_vanna { Some(total_vanna) } else { None },
                volga: if has_volga { Some(total_volga) } else { None },
            },
        }
    }
}

/// Execution statistics for portfolio risk calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_trades: usize,
    pub successful: usize,
    pub failed: usize,
    pub total_time_ms: f64,
    pub avg_time_per_trade_ms: f64,
    pub used_parallel: bool,
}

impl ExecutionStats {
    /// Creates new execution statistics.
    pub fn new(
        total_trades: usize,
        successful: usize,
        failed: usize,
        total_time_ms: f64,
        used_parallel: bool,
    ) -> Self {
        let avg_time_per_trade_ms = if total_trades > 0 {
            total_time_ms / total_trades as f64
        } else {
            0.0
        };

        Self {
            total_trades,
            successful,
            failed,
            total_time_ms,
            avg_time_per_trade_ms,
            used_parallel,
        }
    }

    /// Returns the success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        if self.total_trades == 0 {
            0.0
        } else {
            (self.successful as f64 / self.total_trades as f64) * 100.0
        }
    }
}

/// A failed calculation entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedCalculation {
    pub trade_id: String,
    pub error_message: String,
}

impl FailedCalculation {
    /// Creates a new FailedCalculation.
    pub fn new(trade_id: impl Into<String>, error_message: impl Into<String>) -> Self {
        Self {
            trade_id: trade_id.into(),
            error_message: error_message.into(),
        }
    }
}

/// Result of portfolio risk calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRiskResult {
    pub results: Vec<RiskResult>,
    pub failures: Vec<FailedCalculation>,
    pub aggregations: AggregatedGreeks,
    pub stats: ExecutionStats,
}

impl PortfolioRiskResult {
    /// Creates a new PortfolioRiskResult.
    pub fn new(
        results: Vec<RiskResult>,
        failures: Vec<FailedCalculation>,
        total_time_ms: f64,
        used_parallel: bool,
    ) -> Self {
        let total_trades = results.len() + failures.len();
        let successful = results.len();
        let failed = failures.len();

        let aggregations = AggregatedGreeks::from_results(&results);
        let stats = ExecutionStats::new(
            total_trades,
            successful,
            failed,
            total_time_ms,
            used_parallel,
        );

        Self {
            results,
            failures,
            aggregations,
            stats,
        }
    }

    /// Returns the total PV across all trades.
    pub fn total_pv(&self) -> f64 { self.results.iter().map(|r| r.pv).sum() }

    /// Returns true if all calculations succeeded.
    pub fn all_succeeded(&self) -> bool { self.failures.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_computed_greeks_empty() {
        let greeks = ComputedGreeks::empty();
        assert!(!greeks.has_any());
        assert_eq!(greeks.count(), 0);
    }

    #[test]
    fn test_computed_greeks_has_any() {
        let greeks = ComputedGreeks {
            delta: Some(0.5),
            ..Default::default()
        };
        assert!(greeks.has_any());
        assert_eq!(greeks.count(), 1);
    }

    #[test]
    fn test_computed_greeks_count() {
        let greeks = ComputedGreeks {
            delta: Some(0.5),
            gamma: Some(0.02),
            vega: Some(0.1),
            theta: None,
            rho: None,
            vanna: None,
            volga: None,
        };
        assert_eq!(greeks.count(), 3);
    }

    #[test]
    fn test_performance_metrics_new() {
        let metrics = PerformanceMetrics::new(1.5);
        assert!((metrics.computation_time_ms - 1.5).abs() < f64::EPSILON);
        assert!(metrics.memory_usage_bytes.is_none());
    }

    #[test]
    fn test_performance_metrics_with_memory() {
        let metrics = PerformanceMetrics::with_memory(2.5, 1024);
        assert!((metrics.computation_time_ms - 2.5).abs() < f64::EPSILON);
        assert_eq!(metrics.memory_usage_bytes, Some(1024));
    }

    #[test]
    fn test_risk_result_new() {
        let greeks = ComputedGreeks {
            delta: Some(0.5),
            gamma: Some(0.02),
            ..Default::default()
        };
        let result = RiskResult::new("T001", 100.0, greeks, GreeksMethod::Bump, 1.5);

        assert_eq!(result.trade_id, "T001");
        assert!((result.pv - 100.0).abs() < f64::EPSILON);
        assert_eq!(result.method, GreeksMethod::Bump);
        assert!((result.metrics.computation_time_ms - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_aggregated_greeks_from_results() {
        let results = vec![
            RiskResult::new(
                "T001",
                100.0,
                ComputedGreeks {
                    delta: Some(0.5),
                    gamma: Some(0.02),
                    ..Default::default()
                },
                GreeksMethod::Bump,
                1.0,
            ),
            RiskResult::new(
                "T002",
                50.0,
                ComputedGreeks {
                    delta: Some(0.3),
                    gamma: Some(0.01),
                    vega: Some(0.1),
                    ..Default::default()
                },
                GreeksMethod::Bump,
                1.0,
            ),
        ];

        let aggregated = AggregatedGreeks::from_results(&results);

        assert!((aggregated.total.delta.unwrap() - 0.8).abs() < 1e-10);
        assert!((aggregated.total.gamma.unwrap() - 0.03).abs() < 1e-10);
        assert!((aggregated.total.vega.unwrap() - 0.1).abs() < 1e-10);
        assert!(aggregated.total.theta.is_none());
    }

    #[test]
    fn test_execution_stats_new() {
        let stats = ExecutionStats::new(100, 95, 5, 150.0, true);

        assert_eq!(stats.total_trades, 100);
        assert_eq!(stats.successful, 95);
        assert_eq!(stats.failed, 5);
        assert!((stats.total_time_ms - 150.0).abs() < f64::EPSILON);
        assert!((stats.avg_time_per_trade_ms - 1.5).abs() < f64::EPSILON);
        assert!(stats.used_parallel);
    }

    #[test]
    fn test_execution_stats_success_rate() {
        let stats = ExecutionStats::new(100, 95, 5, 150.0, true);
        assert!((stats.success_rate() - 95.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_execution_stats_success_rate_empty() {
        let stats = ExecutionStats::new(0, 0, 0, 0.0, false);
        assert_eq!(stats.success_rate(), 0.0);
    }

    #[test]
    fn test_portfolio_risk_result_new() {
        let results = vec![RiskResult::new(
            "T001",
            100.0,
            ComputedGreeks {
                delta: Some(0.5),
                ..Default::default()
            },
            GreeksMethod::Bump,
            1.0,
        )];
        let failures: Vec<FailedCalculation> = vec![];

        let portfolio_result = PortfolioRiskResult::new(results, failures, 2.0, false);

        assert_eq!(portfolio_result.results.len(), 1);
        assert!(portfolio_result.all_succeeded());
        assert!((portfolio_result.total_pv() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_portfolio_risk_result_with_failures() {
        let results = vec![RiskResult::new(
            "T001",
            100.0,
            ComputedGreeks::default(),
            GreeksMethod::Bump,
            1.0,
        )];
        let failures = vec![FailedCalculation::new("T002", "Curve not found")];

        let portfolio_result = PortfolioRiskResult::new(results, failures, 3.0, true);

        assert!(!portfolio_result.all_succeeded());
        assert_eq!(portfolio_result.stats.total_trades, 2);
        assert_eq!(portfolio_result.stats.successful, 1);
        assert_eq!(portfolio_result.stats.failed, 1);
    }
}
