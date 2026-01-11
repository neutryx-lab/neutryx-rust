//! HTTP handlers for the web API.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::AppState;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Health check endpoint
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Trade data for portfolio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeData {
    pub id: String,
    pub instrument: String,
    pub notional: f64,
    pub pv: f64,
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
}

/// Portfolio response
#[derive(Debug, Serialize)]
pub struct PortfolioResponse {
    pub trades: Vec<TradeData>,
    pub total_pv: f64,
    pub trade_count: usize,
}

/// Get portfolio data
pub async fn get_portfolio(
    State(_state): State<Arc<AppState>>,
) -> Json<PortfolioResponse> {
    // Sample portfolio data (in production, fetch from service_gateway)
    let trades = vec![
        TradeData {
            id: "T001".to_string(),
            instrument: "AAPL Call 200".to_string(),
            notional: 1_000_000.0,
            pv: 125_000.0,
            delta: 0.65,
            gamma: 0.02,
            vega: 0.15,
        },
        TradeData {
            id: "T002".to_string(),
            instrument: "USD/JPY Forward".to_string(),
            notional: 5_000_000.0,
            pv: -45_000.0,
            delta: 0.98,
            gamma: 0.0,
            vega: 0.0,
        },
        TradeData {
            id: "T003".to_string(),
            instrument: "5Y IRS Pay".to_string(),
            notional: 10_000_000.0,
            pv: 250_000.0,
            delta: 4.5,
            gamma: 0.0,
            vega: 0.0,
        },
        TradeData {
            id: "T004".to_string(),
            instrument: "EUR/USD Option".to_string(),
            notional: 2_000_000.0,
            pv: 35_000.0,
            delta: 0.45,
            gamma: 0.01,
            vega: 0.08,
        },
        TradeData {
            id: "T005".to_string(),
            instrument: "CDS Protection".to_string(),
            notional: 3_000_000.0,
            pv: -12_000.0,
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
        },
    ];

    let total_pv: f64 = trades.iter().map(|t| t.pv).sum();
    let trade_count = trades.len();

    Json(PortfolioResponse {
        trades,
        total_pv,
        trade_count,
    })
}

/// Price request for portfolio
#[derive(Debug, Deserialize)]
pub struct PriceRequest {
    pub instruments: Vec<PriceRequestItem>,
    pub compute_greeks: Option<bool>,
}

/// Single instrument price request
#[derive(Debug, Deserialize)]
pub struct PriceRequestItem {
    pub instrument_id: String,
    pub spot: f64,
    pub rate: f64,
    pub vol: f64,
}

/// Price portfolio (POST)
pub async fn price_portfolio(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<PriceRequest>,
) -> impl IntoResponse {
    // In production, forward to service_gateway
    let response = PortfolioResponse {
        trades: vec![],
        total_pv: 0.0,
        trade_count: request.instruments.len(),
    };

    (StatusCode::OK, Json(response))
}

/// Exposure metrics response
#[derive(Debug, Serialize)]
pub struct ExposureResponse {
    pub ee: f64,
    pub epe: f64,
    pub ene: f64,
    pub pfe: f64,
    pub eepe: f64,
    pub time_series: Vec<ExposurePoint>,
}

/// Single exposure data point
#[derive(Debug, Serialize)]
pub struct ExposurePoint {
    pub time: f64,
    pub ee: f64,
    pub epe: f64,
    pub pfe: f64,
    pub ene: f64,
}

/// Get exposure metrics
pub async fn get_exposure(
    State(_state): State<Arc<AppState>>,
) -> Json<ExposureResponse> {
    // Generate sample exposure profile
    let time_series: Vec<ExposurePoint> = (0..=40)
        .map(|i| {
            let t = i as f64 * 0.25;
            let decay = (-0.15 * t).exp();
            let growth = 1.0 - (-0.8 * t).exp();
            let profile = growth * decay;

            ExposurePoint {
                time: t,
                ee: 500_000.0 * profile + 100_000.0,
                epe: 450_000.0 * profile + 80_000.0,
                pfe: 900_000.0 * profile + 150_000.0,
                ene: -200_000.0 * profile - 50_000.0,
            }
        })
        .collect();

    // Summary metrics at peak
    let peak = time_series.iter()
        .max_by(|a, b| a.ee.partial_cmp(&b.ee).unwrap())
        .unwrap();

    Json(ExposureResponse {
        ee: peak.ee,
        epe: peak.epe,
        ene: peak.ene,
        pfe: peak.pfe,
        eepe: 350_000.0,
        time_series,
    })
}

/// Risk metrics response
#[derive(Debug, Serialize)]
pub struct RiskMetricsResponse {
    pub total_pv: f64,
    pub cva: f64,
    pub dva: f64,
    pub fva: f64,
    pub total_xva: f64,
    pub ee: f64,
    pub epe: f64,
    pub pfe: f64,
}

/// Get risk metrics
pub async fn get_risk_metrics(
    State(_state): State<Arc<AppState>>,
) -> Json<RiskMetricsResponse> {
    let cva = -15_000.0;
    let dva = 5_000.0;
    let fva = -8_000.0;

    Json(RiskMetricsResponse {
        total_pv: 353_000.0,
        cva,
        dva,
        fva,
        total_xva: cva + dva + fva,
        ee: 500_000.0,
        epe: 450_000.0,
        pfe: 800_000.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        let response = health().await;
        assert_eq!(response.status, "ok");
    }

    #[tokio::test]
    async fn test_get_portfolio() {
        let state = Arc::new(AppState::new());
        let response = get_portfolio(State(state)).await;
        assert!(!response.trades.is_empty());
        assert!(response.trade_count > 0);
    }

    #[tokio::test]
    async fn test_get_exposure() {
        let state = Arc::new(AppState::new());
        let response = get_exposure(State(state)).await;
        assert!(!response.time_series.is_empty());
        assert!(response.ee > 0.0);
    }

    #[tokio::test]
    async fn test_get_risk_metrics() {
        let state = Arc::new(AppState::new());
        let response = get_risk_metrics(State(state)).await;
        assert!(response.total_pv != 0.0);
    }
}
