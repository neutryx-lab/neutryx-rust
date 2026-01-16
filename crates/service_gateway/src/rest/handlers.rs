//! REST API handlers

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ServerError;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Pricing request
#[derive(Deserialize)]
pub struct PriceRequest {
    pub instrument_type: String,
    pub strike: f64,
    pub expiry: f64,
    pub is_call: Option<bool>,
    pub spot: f64,
    pub volatility: f64,
    pub rate: f64,
}

/// Pricing response
#[derive(Serialize)]
pub struct PriceResponse {
    pub price: f64,
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub theta: Option<f64>,
}

/// Portfolio pricing request
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct PortfolioRequest {
    pub instruments: Vec<PriceRequest>,
    pub compute_greeks: Option<bool>,
}

/// Portfolio pricing response
#[derive(Serialize)]
pub struct PortfolioResponse {
    pub results: Vec<PriceResponse>,
    pub total_value: f64,
}

/// Calibration request
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct CalibrateRequest {
    pub model_type: String,
    pub market_data: serde_json::Value,
}

/// Calibration response
#[derive(Serialize)]
pub struct CalibrateResponse {
    pub model_type: String,
    pub parameters: serde_json::Value,
    pub error: f64,
}

/// Exposure request
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ExposureRequest {
    pub portfolio: Vec<PriceRequest>,
    pub time_grid: Vec<f64>,
    pub num_paths: Option<usize>,
}

/// Exposure response
#[derive(Serialize)]
pub struct ExposureResponse {
    pub ee: Vec<f64>,
    pub epe: f64,
    pub ene: f64,
    pub pfe_95: Vec<f64>,
}

// ============================================================================
// Handlers
// ============================================================================

/// Health check endpoint
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Price a single instrument
pub async fn price_instrument(
    Json(request): Json<PriceRequest>,
) -> Result<Json<PriceResponse>, ServerError> {
    // Placeholder implementation using inline Black-Scholes.
    // Production: integrate pricer_pricing for full model support.

    let price = match request.instrument_type.as_str() {
        "vanilla_option" | "european_option" => {
            // Black-Scholes placeholder
            let d1 = ((request.spot / request.strike).ln()
                + (request.rate + 0.5 * request.volatility.powi(2)) * request.expiry)
                / (request.volatility * request.expiry.sqrt());
            let d2 = d1 - request.volatility * request.expiry.sqrt();

            let nd1 = normal_cdf(d1);
            let nd2 = normal_cdf(d2);

            let is_call = request.is_call.unwrap_or(true);
            if is_call {
                request.spot * nd1 - request.strike * (-request.rate * request.expiry).exp() * nd2
            } else {
                request.strike * (-request.rate * request.expiry).exp() * normal_cdf(-d2)
                    - request.spot * normal_cdf(-d1)
            }
        }
        "forward" => request.spot * (request.rate * request.expiry).exp() - request.strike,
        other => {
            return Err(ServerError::InvalidRequest(format!(
                "Unknown instrument type: {}",
                other
            )));
        }
    };

    Ok(Json(PriceResponse {
        price,
        delta: None,
        gamma: None,
        vega: None,
        theta: None,
    }))
}

/// Price a portfolio of instruments
pub async fn price_portfolio(
    Json(request): Json<PortfolioRequest>,
) -> Result<Json<PortfolioResponse>, ServerError> {
    let mut results = Vec::with_capacity(request.instruments.len());
    let mut total_value = 0.0;

    for instrument in request.instruments {
        let response = price_instrument(Json(instrument)).await?;
        total_value += response.price;
        results.push(response.0);
    }

    Ok(Json(PortfolioResponse {
        results,
        total_value,
    }))
}

/// Calibrate model parameters
pub async fn calibrate(
    Json(request): Json<CalibrateRequest>,
) -> Result<Json<CalibrateResponse>, ServerError> {
    // Placeholder returning hardcoded parameters.
    // Production: integrate pricer_optimiser for market-data-driven calibration.

    match request.model_type.as_str() {
        "hull-white" => Ok(Json(CalibrateResponse {
            model_type: "hull-white".to_string(),
            parameters: serde_json::json!({
                "alpha": 0.05,
                "sigma": 0.01
            }),
            error: 0.0001,
        })),
        "cir" => Ok(Json(CalibrateResponse {
            model_type: "cir".to_string(),
            parameters: serde_json::json!({
                "kappa": 0.1,
                "theta": 0.05,
                "sigma": 0.02
            }),
            error: 0.0002,
        })),
        other => Err(ServerError::InvalidRequest(format!(
            "Unknown model type: {}",
            other
        ))),
    }
}

/// Calculate exposure metrics
pub async fn calculate_exposure(
    Json(request): Json<ExposureRequest>,
) -> Result<Json<ExposureResponse>, ServerError> {
    // Placeholder returning zero exposure profiles.
    // Production: integrate pricer_risk for Monte Carlo simulation.

    let num_times = request.time_grid.len();

    Ok(Json(ExposureResponse {
        ee: vec![0.0; num_times],
        epe: 0.0,
        ene: 0.0,
        pfe_95: vec![0.0; num_times],
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Standard normal CDF approximation
fn normal_cdf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs() / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    0.5 * (1.0 + sign * y)
}
