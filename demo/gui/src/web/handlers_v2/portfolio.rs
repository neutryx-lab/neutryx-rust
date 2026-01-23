//! Portfolio management endpoints.
//!
//! This module provides endpoints for:
//! - Portfolio data retrieval (`/api/portfolio`)
//! - Portfolio pricing (`/api/portfolio` POST)
//! - Portfolio graph visualisation (`/api/v1/portfolio/graph`)
//! - Portfolio trades list (`/api/v1/portfolio/trades`)

use std::{collections::HashMap, sync::Arc, time::Instant};

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::web::AppState;

// =============================================================================
// Trade Types
// =============================================================================

/// Trade data for portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeData {
    /// Unique trade identifier.
    pub id: String,
    /// Instrument identifier.
    pub instrument: String,
    /// Product type (e.g. "IRS", "FRA").
    pub product: String,
    /// Trade notional amount.
    pub notional: f64,
    /// Present value.
    pub pv: f64,
    /// Delta Greek.
    pub delta: f64,
    /// Gamma Greek.
    pub gamma: f64,
    /// Vega Greek.
    pub vega: f64,
}

/// Portfolio response.
#[derive(Debug, Serialize)]
pub struct PortfolioResponse {
    /// List of trades in the portfolio.
    pub trades: Vec<TradeData>,
    /// Sum of all trade PVs.
    pub total_pv: f64,
    /// Number of trades.
    pub trade_count: usize,
}

// =============================================================================
// Sample Data
// =============================================================================

/// Generate sample trades for demonstration.
pub fn sample_trades() -> Vec<TradeData> {
    vec![
        TradeData {
            id: "T001".to_string(),
            instrument: "5Y IRS Pay Fixed".to_string(),
            product: "swap".to_string(),
            notional: 10_000_000.0,
            pv: 125_000.0,
            delta: 4.5,
            gamma: 0.0,
            vega: 0.0,
        },
        TradeData {
            id: "T002".to_string(),
            instrument: "10Y IRS Receive Fixed".to_string(),
            product: "swap".to_string(),
            notional: 25_000_000.0,
            pv: -180_000.0,
            delta: 8.2,
            gamma: 0.0,
            vega: 0.0,
        },
        TradeData {
            id: "T003".to_string(),
            instrument: "7Y IRS Pay Fixed".to_string(),
            product: "swap".to_string(),
            notional: 15_000_000.0,
            pv: 95_000.0,
            delta: 6.1,
            gamma: 0.0,
            vega: 0.0,
        },
        TradeData {
            id: "T004".to_string(),
            instrument: "3Y IRS Receive Fixed".to_string(),
            product: "swap".to_string(),
            notional: 5_000_000.0,
            pv: -32_000.0,
            delta: 2.8,
            gamma: 0.0,
            vega: 0.0,
        },
        TradeData {
            id: "T005".to_string(),
            instrument: "5Y Payer Swaption".to_string(),
            product: "swaption".to_string(),
            notional: 20_000_000.0,
            pv: 450_000.0,
            delta: 0.45,
            gamma: 0.02,
            vega: 0.85,
        },
        TradeData {
            id: "T006".to_string(),
            instrument: "10Y Receiver Swaption".to_string(),
            product: "swaption".to_string(),
            notional: 30_000_000.0,
            pv: 720_000.0,
            delta: 0.38,
            gamma: 0.015,
            vega: 1.2,
        },
        TradeData {
            id: "T007".to_string(),
            instrument: "3Y Payer Swaption".to_string(),
            product: "swaption".to_string(),
            notional: 8_000_000.0,
            pv: 180_000.0,
            delta: 0.52,
            gamma: 0.025,
            vega: 0.55,
        },
        TradeData {
            id: "T008".to_string(),
            instrument: "5Y Cap 3%".to_string(),
            product: "cap".to_string(),
            notional: 12_000_000.0,
            pv: 85_000.0,
            delta: 0.28,
            gamma: 0.01,
            vega: 0.35,
        },
        TradeData {
            id: "T009".to_string(),
            instrument: "10Y Cap 4%".to_string(),
            product: "cap".to_string(),
            notional: 18_000_000.0,
            pv: 210_000.0,
            delta: 0.32,
            gamma: 0.008,
            vega: 0.65,
        },
        TradeData {
            id: "T010".to_string(),
            instrument: "3Y Floor 1%".to_string(),
            product: "cap".to_string(),
            notional: 7_000_000.0,
            pv: 42_000.0,
            delta: -0.15,
            gamma: 0.005,
            vega: 0.22,
        },
        TradeData {
            id: "T011".to_string(),
            instrument: "15Y IRS Pay Fixed".to_string(),
            product: "swap".to_string(),
            notional: 50_000_000.0,
            pv: 380_000.0,
            delta: 12.5,
            gamma: 0.0,
            vega: 0.0,
        },
        TradeData {
            id: "T012".to_string(),
            instrument: "7Y Receiver Swaption".to_string(),
            product: "swaption".to_string(),
            notional: 15_000_000.0,
            pv: 320_000.0,
            delta: 0.42,
            gamma: 0.018,
            vega: 0.75,
        },
    ]
}

// =============================================================================
// Portfolio Handlers
// =============================================================================

/// Get portfolio data.
///
/// GET /api/portfolio
///
/// Returns the current portfolio with all trades and summary metrics.
pub async fn get_portfolio(State(state): State<Arc<AppState>>) -> Json<PortfolioResponse> {
    let start = Instant::now();

    // Sample portfolio data (in production, fetch from service_gateway)
    let trades = sample_trades();

    let total_pv: f64 = trades.iter().map(|t| t.pv).sum();
    let trade_count = trades.len();

    // Record response time and warn if > 1s
    let elapsed_us = start.elapsed().as_micros() as u64;
    state.metrics.record_portfolio_time(elapsed_us).await;
    if elapsed_us > 1_000_000 {
        tracing::warn!("Portfolio API response slow: {}ms", elapsed_us / 1000);
    }

    Json(PortfolioResponse {
        trades,
        total_pv,
        trade_count,
    })
}

// =============================================================================
// Price Portfolio
// =============================================================================

/// Price request for portfolio.
#[derive(Debug, Deserialize)]
pub struct PriceRequest {
    /// List of instruments to price.
    pub instruments: Vec<PriceRequestItem>,
    /// Whether to compute Greeks.
    pub compute_greeks: Option<bool>,
}

/// Single instrument price request.
#[derive(Debug, Deserialize)]
pub struct PriceRequestItem {
    /// Instrument identifier.
    pub instrument_id: String,
    /// Spot price.
    pub spot: f64,
    /// Interest rate.
    pub rate: f64,
    /// Volatility.
    pub vol: f64,
}

/// Price portfolio (POST).
///
/// POST /api/portfolio
///
/// Prices the portfolio with the given market data inputs.
pub async fn price_portfolio(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<PriceRequest>,
) -> impl IntoResponse {
    // In production, forward to service_gateway
    let mut trades: Vec<TradeData> = request
        .instruments
        .iter()
        .map(|item| TradeData {
            id: item.instrument_id.clone(),
            instrument: item.instrument_id.clone(),
            product: "swap".to_string(),
            notional: 10_000_000.0,
            pv: (item.spot - item.rate) * 1_000_000.0,
            delta: item.rate * 0.1,
            gamma: 0.0,
            vega: item.vol,
        })
        .collect();

    if trades.is_empty() {
        trades = sample_trades();
    }

    let total_pv: f64 = trades.iter().map(|t| t.pv).sum();
    let trade_count = trades.len();
    let response = PortfolioResponse {
        trades,
        total_pv,
        trade_count,
    };

    (StatusCode::OK, Json(response))
}

// =============================================================================
// Portfolio Trades List
// =============================================================================

/// Trade summary for portfolio trades list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioTradeSummary {
    /// Trade identifier.
    pub trade_id: String,
    /// Instrument type (e.g. "VanillaOption", "IRS", "FxOption").
    pub instrument_type: String,
    /// Currency (e.g. "USD", "JPY").
    pub currency: String,
    /// Notional amount.
    pub notional: f64,
    /// Maturity date (ISO 8601).
    pub maturity: String,
}

/// Portfolio trades statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioTradesStats {
    /// Total number of trades.
    pub total_count: usize,
    /// Breakdown by instrument type.
    pub by_instrument_type: HashMap<String, usize>,
}

/// Portfolio trades list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioTradesResponse {
    /// List of trades in the portfolio.
    pub trades: Vec<PortfolioTradeSummary>,
    /// Portfolio statistics.
    pub stats: PortfolioTradesStats,
}

/// Get portfolio trades list endpoint.
///
/// GET /api/v1/portfolio/trades
///
/// Returns a list of trades with summary information and statistics.
pub async fn get_portfolio_trades(
    State(_state): State<Arc<AppState>>,
) -> Json<PortfolioTradesResponse> {
    // Sample portfolio trades
    let trades = vec![
        PortfolioTradeSummary {
            trade_id: "T001".to_string(),
            instrument_type: "VanillaOption".to_string(),
            currency: "USD".to_string(),
            notional: 10_000_000.0,
            maturity: "2027-03-15".to_string(),
        },
        PortfolioTradeSummary {
            trade_id: "T002".to_string(),
            instrument_type: "FxOption".to_string(),
            currency: "USD/JPY".to_string(),
            notional: 50_000_000.0,
            maturity: "2026-09-20".to_string(),
        },
        PortfolioTradeSummary {
            trade_id: "T003".to_string(),
            instrument_type: "IRS".to_string(),
            currency: "USD".to_string(),
            notional: 100_000_000.0,
            maturity: "2031-06-01".to_string(),
        },
        PortfolioTradeSummary {
            trade_id: "T004".to_string(),
            instrument_type: "FxOption".to_string(),
            currency: "USD/JPY".to_string(),
            notional: 25_000_000.0,
            maturity: "2026-12-15".to_string(),
        },
        PortfolioTradeSummary {
            trade_id: "T005".to_string(),
            instrument_type: "IRS".to_string(),
            currency: "USD".to_string(),
            notional: 75_000_000.0,
            maturity: "2028-03-01".to_string(),
        },
    ];

    // Calculate statistics
    let mut by_instrument_type: HashMap<String, usize> = HashMap::new();
    for trade in &trades {
        *by_instrument_type
            .entry(trade.instrument_type.clone())
            .or_insert(0) += 1;
    }

    let stats = PortfolioTradesStats {
        total_count: trades.len(),
        by_instrument_type,
    };

    Json(PortfolioTradesResponse { trades, stats })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_trades_not_empty() {
        let trades = sample_trades();
        assert!(!trades.is_empty());
        assert_eq!(trades.len(), 12);
    }

    #[test]
    fn test_trade_data_serialisation() {
        let trade = TradeData {
            id: "T001".to_string(),
            instrument: "5Y IRS".to_string(),
            product: "swap".to_string(),
            notional: 10_000_000.0,
            pv: 125_000.0,
            delta: 4.5,
            gamma: 0.0,
            vega: 0.0,
        };
        let json = serde_json::to_string(&trade).unwrap();
        assert!(json.contains("\"id\":\"T001\""));
        assert!(json.contains("\"pv\":125000.0"));
    }

    #[test]
    fn test_portfolio_response_serialisation() {
        let response = PortfolioResponse {
            trades: vec![],
            total_pv: 1_000_000.0,
            trade_count: 0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total_pv\":1000000.0"));
        assert!(json.contains("\"trade_count\":0"));
    }
}
