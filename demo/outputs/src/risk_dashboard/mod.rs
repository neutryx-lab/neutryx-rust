//! Risk dashboard systems.
//!
//! This module provides mock implementations of risk monitoring
//! dashboards that receive real-time updates.

mod metrics_store;
mod websocket_sink;

pub use metrics_store::MetricsStore;
pub use websocket_sink::WebSocketSink;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Risk metric types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    /// Expected Exposure
    EE,
    /// Expected Positive Exposure
    EPE,
    /// Potential Future Exposure
    PFE,
    /// Credit Value Adjustment
    CVA,
    /// Debit Value Adjustment
    DVA,
    /// Funding Value Adjustment
    FVA,
    /// VaR
    VaR,
    /// Greeks - Delta
    Delta,
    /// Greeks - Gamma
    Gamma,
    /// Greeks - Vega
    Vega,
    /// Greeks - Theta
    Theta,
    /// Greeks - Rho
    Rho,
    /// PnL
    PnL,
}

/// Risk metric update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricUpdate {
    /// Metric type
    pub metric_type: MetricType,
    /// Entity (portfolio, netting set, trade)
    pub entity_id: String,
    /// Value
    pub value: f64,
    /// Currency
    pub currency: String,
    /// Confidence level (for VaR, PFE)
    pub confidence: Option<f64>,
    /// Time horizon (for VaR, PFE)
    pub horizon_days: Option<u32>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Aggregated risk summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSummary {
    /// Total portfolio value
    pub portfolio_value: f64,
    /// Total CVA
    pub total_cva: f64,
    /// Total DVA
    pub total_dva: f64,
    /// Total FVA
    pub total_fva: f64,
    /// VaR (99%, 1-day)
    pub var_99_1d: f64,
    /// Maximum EPE
    pub max_epe: f64,
    /// Summary by counterparty
    pub by_counterparty: Vec<CounterpartyRisk>,
    /// Timestamp
    pub as_of: DateTime<Utc>,
}

/// Counterparty risk summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterpartyRisk {
    /// Counterparty ID
    pub counterparty_id: String,
    /// Exposure
    pub exposure: f64,
    /// CVA
    pub cva: f64,
    /// PFE (95%)
    pub pfe_95: f64,
}
