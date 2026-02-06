//! Portfolio-related DTOs for CRUD and aggregation operations
//!
//! Request/Response types for `PortfolioService` endpoints.

use serde::{Deserialize, Serialize};

// ============================================================================
// Common Types
// ============================================================================

/// Counterparty information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CounterpartyDto {
    /// Unique counterparty identifier
    pub id: String,
    /// Counterparty name
    pub name: String,
    /// Credit rating (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_rating: Option<String>,
}

/// Netting set information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NettingSetDto {
    /// Unique netting set identifier
    pub id: String,
    /// Associated counterparty ID
    pub counterparty_id: String,
    /// Netting set name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Trade representation for API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradeDto {
    /// Unique trade identifier
    pub trade_id: String,
    /// Trade type (e.g., "irs", "fx\_forward", "equity\_option")
    pub trade_type: String,
    /// Counterparty ID
    pub counterparty_id: String,
    /// Netting set ID
    pub netting_set_id: String,
    /// Notional amount
    pub notional: f64,
    /// Currency
    pub currency: String,
    /// Maturity date (ISO 8601 format)
    pub maturity_date: String,
    /// Additional trade parameters (JSON object)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

// ============================================================================
// Portfolio CRUD DTOs
// ============================================================================

/// Request to create a new portfolio
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields accessed via serde deserialization
pub struct CreatePortfolioRequest {
    /// Portfolio name (optional)
    #[serde(default)]
    pub name: Option<String>,
    /// Counterparties in the portfolio
    #[serde(default)]
    pub counterparties: Vec<CounterpartyDto>,
    /// Netting sets in the portfolio
    #[serde(default)]
    pub netting_sets: Vec<NettingSetDto>,
    /// Initial trades to add
    #[serde(default)]
    pub trades: Vec<TradeDto>,
}

/// Response for portfolio creation
#[derive(Debug, Clone, Serialize)]
pub struct CreatePortfolioResponse {
    /// Generated portfolio ID
    pub portfolio_id: String,
    /// Number of trades added
    pub trade_count: usize,
    /// Number of counterparties
    pub counterparty_count: usize,
    /// Number of netting sets
    pub netting_set_count: usize,
}

/// Response for portfolio retrieval
#[derive(Debug, Clone, Serialize)]
pub struct GetPortfolioResponse {
    /// Portfolio ID
    pub portfolio_id: String,
    /// Portfolio name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Number of trades
    pub trade_count: usize,
    /// Number of counterparties
    pub counterparty_count: usize,
    /// Number of netting sets
    pub netting_set_count: usize,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// List of trades (optional, for detailed view)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trades: Option<Vec<TradeDto>>,
}

/// Request to add trades to a portfolio
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields accessed via serde deserialization
pub struct AddTradesRequest {
    /// Trades to add
    pub trades: Vec<TradeDto>,
}

/// Response for adding trades
#[derive(Debug, Clone, Serialize)]
pub struct AddTradesResponse {
    /// Portfolio ID
    pub portfolio_id: String,
    /// Number of trades added
    pub trades_added: usize,
    /// Total trade count after addition
    pub total_trade_count: usize,
}

// ============================================================================
// Portfolio Aggregation DTOs
// ============================================================================

/// Request for portfolio Greeks calculation
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields accessed via serde deserialization
pub struct PortfolioGreeksRequest {
    /// Greek types to calculate (defaults to all first-order)
    #[serde(default = "default_greek_types")]
    pub greek_types: Vec<String>,
    /// Group by counterparty
    #[serde(default)]
    pub group_by_counterparty: bool,
    /// Group by netting set
    #[serde(default)]
    pub group_by_netting_set: bool,
}

fn default_greek_types() -> Vec<String> {
    vec![
        "delta".to_string(),
        "gamma".to_string(),
        "vega".to_string(),
        "theta".to_string(),
        "rho".to_string(),
    ]
}

/// Greeks for a group (counterparty or netting set)
#[derive(Debug, Clone, Serialize)]
pub struct GroupGreeksDto {
    /// Group identifier
    pub group_id: String,
    /// Group name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
    /// Number of trades in group
    pub trade_count: usize,
    /// Total delta
    pub delta: f64,
    /// Total gamma
    pub gamma: f64,
    /// Total vega
    pub vega: f64,
    /// Total theta
    pub theta: f64,
    /// Total rho
    pub rho: f64,
}

/// Trade calculation error
#[derive(Debug, Clone, Serialize)]
pub struct TradeErrorDto {
    /// Trade ID that failed
    pub trade_id: String,
    /// Error message
    pub error: String,
}

/// Response for portfolio Greeks calculation
#[derive(Debug, Clone, Serialize)]
pub struct PortfolioGreeksResponse {
    /// Portfolio ID
    pub portfolio_id: String,
    /// Total portfolio delta
    pub total_delta: f64,
    /// Total portfolio gamma
    pub total_gamma: f64,
    /// Total portfolio vega
    pub total_vega: f64,
    /// Total portfolio theta
    pub total_theta: f64,
    /// Total portfolio rho
    pub total_rho: f64,
    /// Greeks grouped by counterparty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_counterparty: Option<Vec<GroupGreeksDto>>,
    /// Greeks grouped by netting set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_netting_set: Option<Vec<GroupGreeksDto>>,
    /// Number of trades successfully processed
    pub success_count: usize,
    /// Number of trades that failed
    pub failure_count: usize,
    /// Details of failed trades
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<TradeErrorDto>,
    /// Calculation time in milliseconds
    pub calculation_time_ms: f64,
}

/// Response for portfolio pricing
#[derive(Debug, Clone, Serialize)]
pub struct PortfolioPriceResponse {
    /// Portfolio ID
    pub portfolio_id: String,
    /// Total portfolio present value
    pub total_pv: f64,
    /// Currency of the total PV
    pub currency: String,
    /// PV breakdown by trade
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_pvs: Option<Vec<TradePvDto>>,
    /// Number of trades successfully priced
    pub success_count: usize,
    /// Number of trades that failed
    pub failure_count: usize,
    /// Details of failed trades
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<TradeErrorDto>,
    /// Calculation time in milliseconds
    pub calculation_time_ms: f64,
}

/// Individual trade PV
#[derive(Debug, Clone, Serialize)]
pub struct TradePvDto {
    /// Trade ID
    pub trade_id: String,
    /// Trade type
    pub trade_type: String,
    /// Present value
    pub pv: f64,
    /// Currency
    pub currency: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_portfolio_request_defaults() {
        let json = r#"{}"#;
        let request: CreatePortfolioRequest = serde_json::from_str(json).unwrap();
        assert!(request.name.is_none());
        assert!(request.trades.is_empty());
        assert!(request.counterparties.is_empty());
    }

    #[test]
    fn test_create_portfolio_request_with_trades() {
        let json = r#"{
            "name": "Test Portfolio",
            "trades": [
                {
                    "trade_id": "T001",
                    "trade_type": "irs",
                    "counterparty_id": "CP001",
                    "netting_set_id": "NS001",
                    "notional": 1000000,
                    "currency": "USD",
                    "maturity_date": "2025-12-31"
                }
            ]
        }"#;
        let request: CreatePortfolioRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Test Portfolio".to_string()));
        assert_eq!(request.trades.len(), 1);
        assert_eq!(request.trades[0].trade_id, "T001");
    }

    #[test]
    fn test_portfolio_greeks_response_serialisation() {
        let response = PortfolioGreeksResponse {
            portfolio_id: "P123".to_string(),
            total_delta: 1000.0,
            total_gamma: 50.0,
            total_vega: 200.0,
            total_theta: -10.0,
            total_rho: 15.0,
            by_counterparty: None,
            by_netting_set: None,
            success_count: 10,
            failure_count: 0,
            errors: vec![],
            calculation_time_ms: 100.0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("total_delta"));
        assert!(json.contains("1000"));
        // Empty errors array should not be serialised
        assert!(!json.contains("errors"));
    }
}
