//! Risk engine result types.

use std::collections::HashMap;

use infra_config::GreeksMethod;
use serde::{Deserialize, Serialize};

use crate::{greeks::GreeksResult, scenarios::RiskFactorId};

/// Computed Greeks for a single trade.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComputedGreeks {
    /// Delta sensitivity.
    pub delta: Option<f64>,
    /// Gamma sensitivity.
    pub gamma: Option<f64>,
    /// Vega sensitivity.
    pub vega: Option<f64>,
    /// Theta sensitivity.
    pub theta: Option<f64>,
    /// Rho sensitivity.
    pub rho: Option<f64>,
    /// Vanna (cross-gamma) sensitivity.
    pub vanna: Option<f64>,
    /// Volga (vol-of-vol) sensitivity.
    pub volga: Option<f64>,
}

/// Helper to aggregate an `Option<f64>` field across results.
macro_rules! aggregate_greek {
    ($results:expr, $field:ident) => {{
        let mut total = 0.0;
        let mut has = false;
        for r in $results {
            if let Some(v) = r.greeks.$field {
                total += v;
                has = true;
            }
        }
        if has {
            Some(total)
        } else {
            None
        }
    }};
}

impl ComputedGreeks {
    /// Creates an empty ComputedGreeks.
    #[inline]
    pub fn empty() -> Self { Self::default() }

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
            self.delta, self.gamma, self.vega, self.theta, self.rho, self.vanna, self.volga,
        ]
        .iter()
        .filter(|v| v.is_some())
        .count()
    }
}

/// Performance metrics for a calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Computation time in milliseconds.
    pub computation_time_ms: f64,
    /// Memory usage in bytes, if tracked.
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
    /// Trade identifier.
    pub trade_id: String,
    /// Present value.
    pub pv: f64,
    /// Computed Greeks.
    pub greeks: ComputedGreeks,
    /// Calculation method used.
    pub method: GreeksMethod,
    /// Performance metrics.
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
    /// Greeks aggregated by risk factor.
    pub by_risk_factor: HashMap<RiskFactorId, ComputedGreeks>,
    /// Greeks aggregated by currency.
    pub by_currency: HashMap<String, ComputedGreeks>,
    /// Greeks aggregated by tenor bucket.
    pub by_tenor_bucket: HashMap<String, ComputedGreeks>,
    /// Total aggregated Greeks.
    pub total: ComputedGreeks,
}

impl AggregatedGreeks {
    /// Creates empty aggregated Greeks.
    pub fn empty() -> Self { Self::default() }

    /// Creates aggregated Greeks from individual results.
    pub fn from_results(results: &[RiskResult]) -> Self {
        Self {
            by_risk_factor: HashMap::new(),
            by_currency: HashMap::new(),
            by_tenor_bucket: HashMap::new(),
            total: ComputedGreeks {
                delta: aggregate_greek!(results, delta),
                gamma: aggregate_greek!(results, gamma),
                vega: aggregate_greek!(results, vega),
                theta: aggregate_greek!(results, theta),
                rho: aggregate_greek!(results, rho),
                vanna: aggregate_greek!(results, vanna),
                volga: aggregate_greek!(results, volga),
            },
        }
    }
}

/// Execution statistics for portfolio risk calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Total number of trades processed.
    pub total_trades: usize,
    /// Number of successful calculations.
    pub successful: usize,
    /// Number of failed calculations.
    pub failed: usize,
    /// Total computation time in milliseconds.
    pub total_time_ms: f64,
    /// Average time per trade in milliseconds.
    pub avg_time_per_trade_ms: f64,
    /// Whether parallel execution was used.
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
    /// Trade identifier.
    pub trade_id: String,
    /// Error message.
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
    /// Individual trade results.
    pub results: Vec<RiskResult>,
    /// Failed calculation entries.
    pub failures: Vec<FailedCalculation>,
    /// Aggregated Greeks across the portfolio.
    pub aggregations: AggregatedGreeks,
    /// Execution statistics.
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
