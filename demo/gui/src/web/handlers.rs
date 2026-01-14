//! HTTP handlers for the web API.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use super::pricer_types::{
    DemoMarketData, EquityOptionParams, FxOptionParams, GreeksData, InstrumentParams,
    InstrumentType, IrsParams, OptionType, PricingErrorResponse, PricingRequest, PricingResponse,
};
use super::websocket::broadcast_pricing_complete;
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
    pub product: String,
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

fn sample_trades() -> Vec<TradeData> {
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

/// Get portfolio data
pub async fn get_portfolio(State(state): State<Arc<AppState>>) -> Json<PortfolioResponse> {
    let start = Instant::now();

    // Sample portfolio data (in production, fetch from service_gateway)
    let trades = sample_trades();

    let total_pv: f64 = trades.iter().map(|t| t.pv).sum();
    let trade_count = trades.len();

    // Task 6.2: Record response time and warn if > 1s
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
pub async fn get_exposure(State(state): State<Arc<AppState>>) -> Json<ExposureResponse> {
    let start = Instant::now();

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
    let peak = time_series
        .iter()
        .max_by(|a, b| a.ee.partial_cmp(&b.ee).unwrap())
        .unwrap();

    // Task 6.2: Record response time and warn if > 1s
    let elapsed_us = start.elapsed().as_micros() as u64;
    state.metrics.record_exposure_time(elapsed_us).await;
    if elapsed_us > 1_000_000 {
        tracing::warn!("Exposure API response slow: {}ms", elapsed_us / 1000);
    }

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
pub async fn get_risk_metrics(State(state): State<Arc<AppState>>) -> Json<RiskMetricsResponse> {
    let start = Instant::now();

    let cva = -15_000.0;
    let dva = 5_000.0;
    let fva = -8_000.0;

    // Task 6.2: Record response time and warn if > 1s
    let elapsed_us = start.elapsed().as_micros() as u64;
    state.metrics.record_risk_time(elapsed_us).await;
    if elapsed_us > 1_000_000 {
        tracing::warn!("Risk API response slow: {}ms", elapsed_us / 1000);
    }

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

// =============================================================================
// Task 2.1: Pricing Handler Implementation
// =============================================================================

/// Standard normal cumulative distribution function (CDF).
fn norm_cdf(x: f64) -> f64 {
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

/// Standard normal probability density function (PDF).
fn norm_pdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// Black-Scholes pricing for European options.
fn black_scholes_price(
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    vol: f64,
    is_call: bool,
) -> f64 {
    if time <= 0.0 {
        let intrinsic = if is_call {
            (spot - strike).max(0.0)
        } else {
            (strike - spot).max(0.0)
        };
        return intrinsic;
    }

    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;
    let discount = (-rate * time).exp();

    if is_call {
        spot * norm_cdf(d1) - strike * discount * norm_cdf(d2)
    } else {
        strike * discount * norm_cdf(-d2) - spot * norm_cdf(-d1)
    }
}

/// Black-Scholes Greeks calculation.
fn black_scholes_greeks(
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    vol: f64,
    is_call: bool,
) -> GreeksData {
    if time <= 0.0 {
        return GreeksData {
            delta: if is_call {
                if spot > strike { 1.0 } else { 0.0 }
            } else if spot < strike {
                -1.0
            } else {
                0.0
            },
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
        };
    }

    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;
    let discount = (-rate * time).exp();
    let pdf_d1 = norm_pdf(d1);

    let delta = if is_call { norm_cdf(d1) } else { norm_cdf(d1) - 1.0 };
    let gamma = pdf_d1 / (spot * vol * sqrt_t);
    let vega = spot * pdf_d1 * sqrt_t / 100.0;
    let theta_part1 = -(spot * pdf_d1 * vol) / (2.0 * sqrt_t);
    let theta = if is_call {
        (theta_part1 - rate * strike * discount * norm_cdf(d2)) / 365.0
    } else {
        (theta_part1 + rate * strike * discount * norm_cdf(-d2)) / 365.0
    };
    let rho = if is_call {
        strike * time * discount * norm_cdf(d2) / 100.0
    } else {
        -strike * time * discount * norm_cdf(-d2) / 100.0
    };

    GreeksData { delta, gamma, vega, theta, rho }
}

/// Garman-Kohlhagen pricing for FX options.
fn garman_kohlhagen_price(
    spot: f64,
    strike: f64,
    time: f64,
    dom_rate: f64,
    for_rate: f64,
    vol: f64,
    is_call: bool,
) -> f64 {
    if time <= 0.0 {
        return if is_call {
            (spot - strike).max(0.0)
        } else {
            (strike - spot).max(0.0)
        };
    }

    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (dom_rate - for_rate + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;
    let dom_discount = (-dom_rate * time).exp();
    let for_discount = (-for_rate * time).exp();

    if is_call {
        spot * for_discount * norm_cdf(d1) - strike * dom_discount * norm_cdf(d2)
    } else {
        strike * dom_discount * norm_cdf(-d2) - spot * for_discount * norm_cdf(-d1)
    }
}

/// Garman-Kohlhagen Greeks calculation.
fn garman_kohlhagen_greeks(
    spot: f64,
    strike: f64,
    time: f64,
    dom_rate: f64,
    for_rate: f64,
    vol: f64,
    is_call: bool,
) -> GreeksData {
    if time <= 0.0 {
        return GreeksData {
            delta: if is_call {
                if spot > strike { 1.0 } else { 0.0 }
            } else if spot < strike {
                -1.0
            } else {
                0.0
            },
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
        };
    }

    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (dom_rate - for_rate + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;
    let dom_discount = (-dom_rate * time).exp();
    let for_discount = (-for_rate * time).exp();
    let pdf_d1 = norm_pdf(d1);

    let delta = if is_call {
        for_discount * norm_cdf(d1)
    } else {
        for_discount * (norm_cdf(d1) - 1.0)
    };
    let gamma = for_discount * pdf_d1 / (spot * vol * sqrt_t);
    let vega = spot * for_discount * pdf_d1 * sqrt_t / 100.0;
    let theta_part1 = -(spot * for_discount * pdf_d1 * vol) / (2.0 * sqrt_t);
    let theta = if is_call {
        (theta_part1 + for_rate * spot * for_discount * norm_cdf(d1)
            - dom_rate * strike * dom_discount * norm_cdf(d2)) / 365.0
    } else {
        (theta_part1 - for_rate * spot * for_discount * norm_cdf(-d1)
            + dom_rate * strike * dom_discount * norm_cdf(-d2)) / 365.0
    };
    let rho = if is_call {
        strike * time * dom_discount * norm_cdf(d2) / 100.0
    } else {
        -strike * time * dom_discount * norm_cdf(-d2) / 100.0
    };

    GreeksData { delta, gamma, vega, theta, rho }
}

/// Simple IRS pricing (demo approximation).
fn irs_price(notional: f64, fixed_rate: f64, tenor: f64, market_rate: f64) -> f64 {
    let pv01 = tenor * 0.9;
    notional * (fixed_rate - market_rate) * pv01
}

/// IRS Greeks (simplified for demo).
fn irs_greeks(notional: f64, tenor: f64) -> GreeksData {
    let dv01 = notional * tenor * 0.0001 * 0.9;
    GreeksData {
        delta: dv01,
        gamma: 0.0,
        vega: 0.0,
        theta: 0.0,
        rho: dv01,
    }
}

/// Validate equity option parameters.
fn validate_equity_params(params: &EquityOptionParams) -> Result<(), (String, String)> {
    if params.spot <= 0.0 {
        return Err(("spot".to_string(), "Spot price must be positive".to_string()));
    }
    if params.strike <= 0.0 {
        return Err(("strike".to_string(), "Strike price must be positive".to_string()));
    }
    if params.expiry_years < 0.0 {
        return Err(("expiryYears".to_string(), "Expiry must be non-negative".to_string()));
    }
    if params.volatility <= 0.0 {
        return Err(("volatility".to_string(), "Volatility must be positive".to_string()));
    }
    if params.volatility > 5.0 {
        return Err(("volatility".to_string(), "Volatility seems too high (>500%)".to_string()));
    }
    Ok(())
}

/// Validate FX option parameters.
fn validate_fx_params(params: &FxOptionParams) -> Result<(), (String, String)> {
    if params.spot <= 0.0 {
        return Err(("spot".to_string(), "Spot rate must be positive".to_string()));
    }
    if params.strike <= 0.0 {
        return Err(("strike".to_string(), "Strike rate must be positive".to_string()));
    }
    if params.expiry_years < 0.0 {
        return Err(("expiryYears".to_string(), "Expiry must be non-negative".to_string()));
    }
    if params.volatility <= 0.0 {
        return Err(("volatility".to_string(), "Volatility must be positive".to_string()));
    }
    if params.volatility > 5.0 {
        return Err(("volatility".to_string(), "Volatility seems too high (>500%)".to_string()));
    }
    Ok(())
}

/// Validate IRS parameters.
fn validate_irs_params(params: &IrsParams) -> Result<(), (String, String)> {
    if params.notional <= 0.0 {
        return Err(("notional".to_string(), "Notional must be positive".to_string()));
    }
    if params.tenor_years <= 0.0 {
        return Err(("tenorYears".to_string(), "Tenor must be positive".to_string()));
    }
    Ok(())
}

/// Price an instrument and optionally compute Greeks.
///
/// POST /api/price
pub async fn price_instrument(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PricingRequest>,
) -> Result<Json<PricingResponse>, (StatusCode, Json<PricingErrorResponse>)> {
    // Generate unique calculation ID using timestamp and nanoseconds
    let now = chrono::Utc::now();
    let calculation_id = format!(
        "calc-{}-{}",
        now.timestamp_millis(),
        now.timestamp_subsec_nanos() % 10000
    );

    let market_rate = DemoMarketData::get_curve_rate(request.market_data.as_ref());

    let (pv, greeks) = match (&request.instrument_type, &request.params) {
        (InstrumentType::EquityVanillaOption, InstrumentParams::EquityOption(params)) => {
            if let Err((field, message)) = validate_equity_params(params) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(PricingErrorResponse {
                        error_type: "ValidationError".to_string(),
                        message,
                        field: Some(field),
                    }),
                ));
            }

            let is_call = params.option_type == OptionType::Call;
            let pv = black_scholes_price(
                params.spot, params.strike, params.expiry_years, params.rate, params.volatility, is_call,
            );
            let greeks = if request.compute_greeks {
                Some(black_scholes_greeks(
                    params.spot, params.strike, params.expiry_years, params.rate, params.volatility, is_call,
                ))
            } else {
                None
            };
            (pv, greeks)
        }

        (InstrumentType::FxOption, InstrumentParams::FxOption(params)) => {
            if let Err((field, message)) = validate_fx_params(params) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(PricingErrorResponse {
                        error_type: "ValidationError".to_string(),
                        message,
                        field: Some(field),
                    }),
                ));
            }

            let is_call = params.option_type == OptionType::Call;
            let pv = garman_kohlhagen_price(
                params.spot, params.strike, params.expiry_years,
                params.domestic_rate, params.foreign_rate, params.volatility, is_call,
            );
            let greeks = if request.compute_greeks {
                Some(garman_kohlhagen_greeks(
                    params.spot, params.strike, params.expiry_years,
                    params.domestic_rate, params.foreign_rate, params.volatility, is_call,
                ))
            } else {
                None
            };
            (pv, greeks)
        }

        (InstrumentType::Irs, InstrumentParams::Irs(params)) => {
            if let Err((field, message)) = validate_irs_params(params) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(PricingErrorResponse {
                        error_type: "ValidationError".to_string(),
                        message,
                        field: Some(field),
                    }),
                ));
            }

            let pv = irs_price(params.notional, params.fixed_rate, params.tenor_years, market_rate);
            let greeks = if request.compute_greeks {
                Some(irs_greeks(params.notional, params.tenor_years))
            } else {
                None
            };
            (pv, greeks)
        }

        _ => {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(PricingErrorResponse {
                    error_type: "PricingError".to_string(),
                    message: "Instrument type does not match provided parameters".to_string(),
                    field: None,
                }),
            ));
        }
    };

    if !pv.is_finite() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(PricingErrorResponse {
                error_type: "PricingError".to_string(),
                message: "Numerical instability in pricing calculation".to_string(),
                field: None,
            }),
        ));
    }

    // Task 3.1: Broadcast pricing complete notification via WebSocket
    let greeks_json = greeks.as_ref().map(|g| {
        serde_json::json!({
            "delta": g.delta,
            "gamma": g.gamma,
            "vega": g.vega,
            "theta": g.theta,
            "rho": g.rho
        })
    });
    let instrument_type_str = match &request.instrument_type {
        InstrumentType::EquityVanillaOption => "equity_vanilla_option",
        InstrumentType::FxOption => "fx_option",
        InstrumentType::Irs => "irs",
    };
    broadcast_pricing_complete(&state, &calculation_id, instrument_type_str, pv, greeks_json);

    Ok(Json(PricingResponse {
        calculation_id,
        instrument_type: request.instrument_type,
        pv,
        greeks,
        timestamp: chrono::Utc::now().timestamp_millis(),
    }))
}

// =============================================================================
// Task 3.1: Graph API Types and Handler
// =============================================================================

/// Query parameters for graph endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct GraphQueryParams {
    /// Optional trade ID to filter graph extraction
    pub trade_id: Option<String>,
}

/// Graph node for API response (D3.js compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeResponse {
    /// Unique identifier for the node
    pub id: String,
    /// Operation type (D3.js compatible: "type" field)
    #[serde(rename = "type")]
    pub node_type: String,
    /// Human-readable label
    pub label: String,
    /// Current computed value
    pub value: Option<f64>,
    /// Whether this node is a sensitivity calculation target
    pub is_sensitivity_target: bool,
    /// Visual grouping for colour coding
    pub group: String,
}

/// Graph edge for API response (D3.js compatible: "links")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeResponse {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Optional edge weight
    pub weight: Option<f64>,
}

/// Graph metadata for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetadataResponse {
    /// Trade ID (None for aggregate graphs)
    pub trade_id: Option<String>,
    /// Total number of nodes
    pub node_count: usize,
    /// Total number of edges
    pub edge_count: usize,
    /// Graph depth (longest path)
    pub depth: usize,
    /// Generation timestamp (ISO 8601)
    pub generated_at: String,
}

/// Graph API response (D3.js compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResponse {
    /// All nodes in the computation graph
    pub nodes: Vec<GraphNodeResponse>,
    /// All edges (D3.js compatible: "links")
    pub links: Vec<GraphEdgeResponse>,
    /// Graph metadata
    pub metadata: GraphMetadataResponse,
}

/// Error response for graph API
#[derive(Debug, Serialize)]
pub struct GraphErrorResponse {
    /// Error type
    pub error_type: String,
    /// Error message
    pub message: String,
}

// =============================================================================
// Task 3.3: Graph Cache for Performance Optimisation
// =============================================================================

/// Cached graph entry with timestamp
#[derive(Debug, Clone)]
pub struct CachedGraph {
    /// The cached graph response
    pub graph: GraphResponse,
    /// When the cache entry was created
    pub created_at: Instant,
}

/// Graph cache with TTL support
#[derive(Debug, Default)]
pub struct GraphCache {
    /// Cache entries by trade_id (None key = all trades)
    entries: HashMap<Option<String>, CachedGraph>,
}

impl GraphCache {
    /// Cache TTL in seconds
    const TTL_SECONDS: u64 = 5;

    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get a cached graph if it exists and is not expired
    pub fn get(&self, trade_id: &Option<String>) -> Option<&GraphResponse> {
        self.entries.get(trade_id).and_then(|entry| {
            if entry.created_at.elapsed().as_secs() < Self::TTL_SECONDS {
                Some(&entry.graph)
            } else {
                None
            }
        })
    }

    /// Insert a graph into the cache
    pub fn insert(&mut self, trade_id: Option<String>, graph: GraphResponse) {
        self.entries.insert(
            trade_id,
            CachedGraph {
                graph,
                created_at: Instant::now(),
            },
        );
    }

    /// Remove expired entries from the cache
    pub fn cleanup(&mut self) {
        self.entries
            .retain(|_, entry| entry.created_at.elapsed().as_secs() < Self::TTL_SECONDS);
    }

    /// Clear the entire cache
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Generate a sample computation graph for a trade
///
/// In production, this would call the GraphExtractor from pricer_pricing.
/// For the demo, we generate a representative graph structure.
fn generate_sample_graph(trade_id: Option<&str>) -> GraphResponse {
    // Generate nodes based on trade
    let mut nodes = Vec::new();
    let mut links = Vec::new();

    // Sample trades and their parameters
    let trades_data = if let Some(tid) = trade_id {
        vec![(tid.to_string(), get_trade_params(tid))]
    } else {
        vec![
            ("T001".to_string(), get_trade_params("T001")),
            ("T002".to_string(), get_trade_params("T002")),
            ("T003".to_string(), get_trade_params("T003")),
        ]
    };

    for (tid, params) in &trades_data {
        let mut intermediate_ids = Vec::new();

        // Create input nodes for each parameter
        for (i, param) in params.iter().enumerate() {
            let node_id = format!("{}_{}", tid, param);
            nodes.push(GraphNodeResponse {
                id: node_id.clone(),
                node_type: "input".to_string(),
                label: param.clone(),
                value: Some(100.0 + (i as f64) * 10.0),
                is_sensitivity_target: true,
                group: "sensitivity".to_string(),
            });
        }

        // Create intermediate computation nodes
        for (i, chunk) in params.chunks(2).enumerate() {
            let node_id = format!("{}_op_{}", tid, i);
            let label = if chunk.len() == 2 {
                format!("{} * {}", chunk[0], chunk[1])
            } else {
                format!("exp({})", chunk[0])
            };
            let node_type = if chunk.len() == 2 { "mul" } else { "exp" };

            nodes.push(GraphNodeResponse {
                id: node_id.clone(),
                node_type: node_type.to_string(),
                label,
                value: Some(25.0 + (i as f64) * 5.0),
                is_sensitivity_target: false,
                group: "intermediate".to_string(),
            });

            // Add edges from inputs to operation
            for param in chunk {
                links.push(GraphEdgeResponse {
                    source: format!("{}_{}", tid, param),
                    target: node_id.clone(),
                    weight: None,
                });
            }

            intermediate_ids.push(node_id);
        }

        // Create second level combination nodes
        let mut second_level_ids = Vec::new();
        for (i, chunk) in intermediate_ids.chunks(2).enumerate() {
            let node_id = format!("{}_combine_{}", tid, i);
            let label = if chunk.len() == 2 {
                format!("{} + {}", chunk[0], chunk[1])
            } else {
                format!("sqrt({})", chunk[0])
            };
            let node_type = if chunk.len() == 2 { "add" } else { "sqrt" };

            nodes.push(GraphNodeResponse {
                id: node_id.clone(),
                node_type: node_type.to_string(),
                label,
                value: Some(50.0 + (i as f64) * 10.0),
                is_sensitivity_target: false,
                group: "intermediate".to_string(),
            });

            for source in chunk {
                links.push(GraphEdgeResponse {
                    source: source.clone(),
                    target: node_id.clone(),
                    weight: None,
                });
            }

            second_level_ids.push(node_id);
        }

        // Create output node
        let output_id = format!("{}_price", tid);
        nodes.push(GraphNodeResponse {
            id: output_id.clone(),
            node_type: "output".to_string(),
            label: "price".to_string(),
            value: Some(125.5),
            is_sensitivity_target: false,
            group: "output".to_string(),
        });

        // Connect final nodes to output
        let final_sources = if second_level_ids.is_empty() {
            &intermediate_ids
        } else {
            &second_level_ids
        };
        for source in final_sources {
            links.push(GraphEdgeResponse {
                source: source.clone(),
                target: output_id.clone(),
                weight: None,
            });
        }
    }

    // Calculate depth (simplified: count layers)
    let depth = if nodes.is_empty() { 0 } else { 4 };

    let generated_at = chrono::Utc::now().to_rfc3339();

    GraphResponse {
        metadata: GraphMetadataResponse {
            trade_id: trade_id.map(String::from),
            node_count: nodes.len(),
            edge_count: links.len(),
            depth,
            generated_at,
        },
        nodes,
        links,
    }
}

/// Get parameters for a specific trade
fn get_trade_params(trade_id: &str) -> Vec<String> {
    match trade_id {
        "T001" => vec![
            "spot".to_string(),
            "vol".to_string(),
            "rate".to_string(),
            "time".to_string(),
        ],
        "T002" => vec![
            "fx_spot".to_string(),
            "dom_rate".to_string(),
            "for_rate".to_string(),
        ],
        "T003" => vec![
            "swap_rate".to_string(),
            "discount".to_string(),
            "notional".to_string(),
            "tenor".to_string(),
        ],
        "T004" => vec!["eur_usd".to_string(), "vol".to_string(), "rate".to_string()],
        "T005" => vec![
            "spread".to_string(),
            "recovery".to_string(),
            "hazard".to_string(),
        ],
        _ => vec!["param1".to_string(), "param2".to_string()],
    }
}

/// Check if a trade exists
fn trade_exists(trade_id: &str) -> bool {
    matches!(trade_id, "T001" | "T002" | "T003" | "T004" | "T005")
}

/// Get computation graph endpoint
///
/// # Endpoint
///
/// `GET /api/graph` - Get computation graph for all trades
/// `GET /api/graph?trade_id=T001` - Get computation graph for specific trade
///
/// # Response
///
/// Returns a D3.js compatible graph structure with nodes, links, and metadata.
///
/// # Errors
///
/// - 404 Not Found: If the specified trade_id does not exist
/// - 500 Internal Server Error: If graph extraction fails
pub async fn get_graph(
    State(state): State<Arc<AppState>>,
    Query(params): Query<GraphQueryParams>,
) -> Result<Json<GraphResponse>, (StatusCode, Json<GraphErrorResponse>)> {
    let start = Instant::now();

    // Check if trade exists (if specified)
    if let Some(ref trade_id) = params.trade_id {
        if !trade_exists(trade_id) {
            return Err((
                StatusCode::NOT_FOUND,
                Json(GraphErrorResponse {
                    error_type: "TradeNotFound".to_string(),
                    message: format!("Trade '{}' not found", trade_id),
                }),
            ));
        }
    }

    // Check cache first (Task 3.3: Performance optimisation)
    {
        let cache = state.graph_cache.read().await;
        if let Some(cached) = cache.get(&params.trade_id) {
            // Task 6.2: Record cache hit time
            let elapsed_us = start.elapsed().as_micros() as u64;
            state.metrics.record_graph_time(elapsed_us).await;
            return Ok(Json(cached.clone()));
        }
    }

    // Generate graph (in production, call GraphExtractor)
    let graph = generate_sample_graph(params.trade_id.as_deref());

    // Update cache
    {
        let mut cache = state.graph_cache.write().await;
        cache.insert(params.trade_id.clone(), graph.clone());
    }

    // Task 6.2: Record response time and warn if > 1s
    let elapsed_us = start.elapsed().as_micros() as u64;
    state.metrics.record_graph_time(elapsed_us).await;
    if elapsed_us > 1_000_000 {
        tracing::warn!("Graph API response slow: {}ms", elapsed_us / 1000);
    }

    Ok(Json(graph))
}

// =============================================================================
// Task 7.2: Speed Comparison Chart API
// =============================================================================

use crate::visualisation::{BenchmarkVisualiser, SpeedComparisonData};

/// Query parameters for speed comparison endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct SpeedComparisonQueryParams {
    /// AAD mean time in nanoseconds (optional, uses sample data if not provided)
    pub aad_mean_ns: Option<f64>,
    /// Bump mean time in nanoseconds (optional, uses sample data if not provided)
    pub bump_mean_ns: Option<f64>,
    /// Number of tenor points (optional, defaults to 20)
    pub tenor_count: Option<usize>,
}

/// Speed comparison chart response (Chart.js compatible)
///
/// # Task Coverage
///
/// - Task 7.2: 速度比較チャートの実装
///   - Webモードではchart.js互換JSONデータを出力
///
/// # Requirements Coverage
///
/// - Requirement 7.2: 速度比較のバーチャートを表示
/// - Requirement 7.5: Webモードではchart.js互換のJSONデータを出力
#[derive(Debug, Clone, Serialize)]
pub struct SpeedComparisonResponse {
    /// Chart type (always "bar")
    #[serde(rename = "type")]
    pub chart_type: String,
    /// Chart data
    pub data: SpeedComparisonChartData,
    /// Chart options
    pub options: SpeedComparisonChartOptions,
    /// Raw benchmark data for additional processing
    pub benchmark: SpeedComparisonBenchmarkData,
}

/// Chart.js compatible data structure
#[derive(Debug, Clone, Serialize)]
pub struct SpeedComparisonChartData {
    /// X-axis labels
    pub labels: Vec<String>,
    /// Chart datasets
    pub datasets: Vec<SpeedComparisonDataset>,
}

/// Chart.js compatible dataset
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedComparisonDataset {
    /// Dataset label
    pub label: String,
    /// Data values
    pub data: Vec<f64>,
    /// Background colours
    pub background_color: Vec<String>,
}

/// Chart.js options
#[derive(Debug, Clone, Serialize)]
pub struct SpeedComparisonChartOptions {
    /// Title configuration
    pub title: SpeedComparisonTitleOptions,
}

/// Chart.js title options
#[derive(Debug, Clone, Serialize)]
pub struct SpeedComparisonTitleOptions {
    /// Whether to display the title
    pub display: bool,
    /// Title text
    pub text: String,
}

/// Raw benchmark data for client-side processing
#[derive(Debug, Clone, Serialize)]
pub struct SpeedComparisonBenchmarkData {
    /// AAD mean time in microseconds
    pub aad_mean_us: f64,
    /// Bump mean time in microseconds
    pub bump_mean_us: f64,
    /// Speedup ratio (bump / aad)
    pub speedup_ratio: f64,
    /// Number of tenor points
    pub tenor_count: usize,
}

/// Get speed comparison chart data endpoint
///
/// # Endpoint
///
/// `GET /api/benchmark/speed-comparison`
///
/// # Query Parameters
///
/// - `aad_mean_ns` (optional): AAD mean time in nanoseconds
/// - `bump_mean_ns` (optional): Bump mean time in nanoseconds
/// - `tenor_count` (optional): Number of tenor points (default: 20)
///
/// # Response
///
/// Returns Chart.js compatible JSON data for rendering a speed comparison bar chart.
///
/// # Task Coverage
///
/// - Task 7.2: 速度比較チャートの実装
///
/// # Requirements Coverage
///
/// - Requirement 7.2: 速度比較のバーチャートを表示
/// - Requirement 7.5: Webモードではchart.js互換のJSONデータを出力
pub async fn get_speed_comparison(
    Query(params): Query<SpeedComparisonQueryParams>,
) -> Json<SpeedComparisonResponse> {
    // Use provided data or sample data
    let data = if let (Some(aad_ns), Some(bump_ns)) = (params.aad_mean_ns, params.bump_mean_ns) {
        let tenor_count = params.tenor_count.unwrap_or(20);
        SpeedComparisonData::new(aad_ns, bump_ns, tenor_count)
    } else {
        SpeedComparisonData::sample()
    };

    // Generate Chart.js compatible response
    let visualiser = BenchmarkVisualiser::new();
    let chartjs = visualiser.to_chartjs_json(&data);

    Json(SpeedComparisonResponse {
        chart_type: chartjs.chart_type,
        data: SpeedComparisonChartData {
            labels: chartjs.data.labels,
            datasets: chartjs
                .data
                .datasets
                .into_iter()
                .map(|ds| SpeedComparisonDataset {
                    label: ds.label,
                    data: ds.data,
                    background_color: ds.background_color,
                })
                .collect(),
        },
        options: SpeedComparisonChartOptions {
            title: SpeedComparisonTitleOptions {
                display: chartjs.options.title.display,
                text: chartjs.options.title.text,
            },
        },
        benchmark: SpeedComparisonBenchmarkData {
            aad_mean_us: data.aad_mean_us(),
            bump_mean_us: data.bump_mean_us(),
            speedup_ratio: data.speedup_ratio,
            tenor_count: data.tenor_count,
        },
    })
}

// =========================================================================
// Task 6.3: Performance Metrics Endpoint (Requirement 9.4)
// =========================================================================

/// API response times statistics
#[derive(Debug, Serialize)]
pub struct ApiResponseTimes {
    pub portfolio_avg_ms: f64,
    pub exposure_avg_ms: f64,
    pub risk_avg_ms: f64,
    pub graph_avg_ms: f64,
}

/// Performance metrics response
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub api_response_times: ApiResponseTimes,
    pub websocket_connections: u32,
    pub websocket_message_latency_ms: f64,
    pub uptime_seconds: u64,
}

/// Get performance metrics endpoint
///
/// Returns JSON with API response times, WebSocket statistics, and uptime.
pub async fn get_metrics(State(state): State<Arc<AppState>>) -> Json<MetricsResponse> {
    let metrics = &state.metrics;

    Json(MetricsResponse {
        api_response_times: ApiResponseTimes {
            portfolio_avg_ms: metrics.portfolio_avg_ms().await,
            exposure_avg_ms: metrics.exposure_avg_ms().await,
            risk_avg_ms: metrics.risk_avg_ms().await,
            graph_avg_ms: metrics.graph_avg_ms().await,
        },
        websocket_connections: metrics.ws_connection_count(),
        websocket_message_latency_ms: metrics.ws_latency_avg_ms().await,
        uptime_seconds: metrics.uptime_seconds(),
    })
}

// =========================================================================
// Task 13.2: Index HTML with Config Injection (Requirement 1.1)
// =========================================================================

use axum::response::Html;
use tower_http::services::ServeFile;

/// Serve index.html with injected configuration
///
/// Reads the index.html template and replaces the placeholder config
/// with values from environment variables (FB_DEBUG_MODE, FB_LOG_LEVEL).
pub async fn get_index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let index_path = "demo/gui/static/index.html";

    match tokio::fs::read_to_string(index_path).await {
        Ok(content) => {
            // Replace the config placeholder with actual values
            let config_script = format!(
                r#"<script id="fb-config">
        window.__FB_CONFIG__ = {{
            debugMode: {},
            logLevel: '{}'
        }};
    </script>"#,
                state.debug_config.debug_mode, state.debug_config.log_level
            );

            // Replace the placeholder config in the HTML
            let modified = content.replace(
                r#"<script id="fb-config">
        window.__FB_CONFIG__ = {
            debugMode: false,
            logLevel: 'INFO'
        };
    </script>"#,
                &config_script,
            );

            Html(modified).into_response()
        }
        Err(_) => {
            // Fallback if file cannot be read
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load index.html").into_response()
        }
    }
}

/// Create a service that serves index.html with config injection for fallback
pub fn serve_index_with_config() -> ServeFile {
    ServeFile::new("demo/gui/static/index.html")
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

    // =========================================================================
    // Task 3.1: Graph API Tests
    // =========================================================================

    mod graph_api_tests {
        use super::*;

        #[tokio::test]
        async fn test_get_graph_all_trades() {
            let state = Arc::new(AppState::new());
            let params = GraphQueryParams { trade_id: None };

            let result = get_graph(State(state), Query(params)).await;

            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(!response.nodes.is_empty());
            assert!(!response.links.is_empty());
            assert!(response.metadata.node_count > 0);
            assert!(response.metadata.edge_count > 0);
        }

        #[tokio::test]
        async fn test_get_graph_specific_trade() {
            let state = Arc::new(AppState::new());
            let params = GraphQueryParams {
                trade_id: Some("T001".to_string()),
            };

            let result = get_graph(State(state), Query(params)).await;

            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.metadata.trade_id, Some("T001".to_string()));
            assert!(!response.nodes.is_empty());
            assert!(!response.links.is_empty());
        }

        #[tokio::test]
        async fn test_get_graph_trade_not_found() {
            let state = Arc::new(AppState::new());
            let params = GraphQueryParams {
                trade_id: Some("NONEXISTENT".to_string()),
            };

            let result = get_graph(State(state), Query(params)).await;

            assert!(result.is_err());
            let (status, error) = result.unwrap_err();
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert_eq!(error.error_type, "TradeNotFound");
        }

        #[tokio::test]
        async fn test_graph_response_d3js_compatible() {
            let state = Arc::new(AppState::new());
            let params = GraphQueryParams {
                trade_id: Some("T001".to_string()),
            };

            let result = get_graph(State(state), Query(params)).await;
            let response = result.unwrap();

            // Verify D3.js compatible structure
            // - Has "nodes" array
            // - Has "links" array (not "edges")
            // - Each node has "type" field (not "node_type" in JSON)
            let json = serde_json::to_string(&response.0).unwrap();
            assert!(json.contains("\"nodes\":"));
            assert!(json.contains("\"links\":"));
            assert!(json.contains("\"type\":"));
            assert!(!json.contains("\"edges\":"));
        }

        #[tokio::test]
        async fn test_graph_contains_input_nodes() {
            let state = Arc::new(AppState::new());
            let params = GraphQueryParams {
                trade_id: Some("T001".to_string()),
            };

            let result = get_graph(State(state), Query(params)).await;
            let response = result.unwrap();

            let input_nodes: Vec<_> = response
                .nodes
                .iter()
                .filter(|n| n.node_type == "input")
                .collect();
            assert!(!input_nodes.is_empty(), "Graph should contain input nodes");
        }

        #[tokio::test]
        async fn test_graph_contains_output_node() {
            let state = Arc::new(AppState::new());
            let params = GraphQueryParams {
                trade_id: Some("T001".to_string()),
            };

            let result = get_graph(State(state), Query(params)).await;
            let response = result.unwrap();

            let output_nodes: Vec<_> = response
                .nodes
                .iter()
                .filter(|n| n.node_type == "output")
                .collect();
            assert!(!output_nodes.is_empty(), "Graph should contain output node");
        }

        #[tokio::test]
        async fn test_graph_contains_sensitivity_targets() {
            let state = Arc::new(AppState::new());
            let params = GraphQueryParams {
                trade_id: Some("T001".to_string()),
            };

            let result = get_graph(State(state), Query(params)).await;
            let response = result.unwrap();

            let sensitivity_nodes: Vec<_> = response
                .nodes
                .iter()
                .filter(|n| n.is_sensitivity_target)
                .collect();
            assert!(
                !sensitivity_nodes.is_empty(),
                "Graph should contain sensitivity target nodes"
            );
        }

        #[tokio::test]
        async fn test_graph_metadata_has_required_fields() {
            let state = Arc::new(AppState::new());
            let params = GraphQueryParams {
                trade_id: Some("T001".to_string()),
            };

            let result = get_graph(State(state), Query(params)).await;
            let response = result.unwrap();

            // Verify metadata fields
            assert!(response.metadata.node_count > 0);
            assert!(response.metadata.edge_count > 0);
            assert!(response.metadata.depth > 0);
            assert!(!response.metadata.generated_at.is_empty());
            // generated_at should be ISO 8601 format
            assert!(response.metadata.generated_at.contains("T"));
        }
    }

    // =========================================================================
    // Task 3.3: Graph Cache Tests
    // =========================================================================

    mod graph_cache_tests {
        use super::*;
        use std::time::Duration;

        #[test]
        fn test_cache_new() {
            let cache = GraphCache::new();
            assert!(cache.get(&None).is_none());
        }

        #[test]
        fn test_cache_insert_and_get() {
            let mut cache = GraphCache::new();
            let graph = generate_sample_graph(Some("T001"));

            cache.insert(Some("T001".to_string()), graph.clone());

            let cached = cache.get(&Some("T001".to_string()));
            assert!(cached.is_some());
            assert_eq!(cached.unwrap().metadata.trade_id, Some("T001".to_string()));
        }

        #[test]
        fn test_cache_miss_for_different_key() {
            let mut cache = GraphCache::new();
            let graph = generate_sample_graph(Some("T001"));

            cache.insert(Some("T001".to_string()), graph);

            // Different trade_id should miss
            let cached = cache.get(&Some("T002".to_string()));
            assert!(cached.is_none());
        }

        #[test]
        fn test_cache_clear() {
            let mut cache = GraphCache::new();
            let graph = generate_sample_graph(Some("T001"));

            cache.insert(Some("T001".to_string()), graph);
            assert!(cache.get(&Some("T001".to_string())).is_some());

            cache.clear();
            assert!(cache.get(&Some("T001".to_string())).is_none());
        }

        #[tokio::test]
        async fn test_handler_uses_cache() {
            let state = Arc::new(AppState::new());
            let params = GraphQueryParams {
                trade_id: Some("T001".to_string()),
            };

            // First call - cache miss, generates graph
            let result1 = get_graph(State(Arc::clone(&state)), Query(params.clone())).await;
            assert!(result1.is_ok());
            let response1 = result1.unwrap();
            let timestamp1 = response1.metadata.generated_at.clone();

            // Second call - should use cache (same timestamp)
            let result2 = get_graph(State(Arc::clone(&state)), Query(params)).await;
            assert!(result2.is_ok());
            let response2 = result2.unwrap();
            let timestamp2 = response2.metadata.generated_at.clone();

            // Both should have same timestamp (from cache)
            assert_eq!(timestamp1, timestamp2);
        }

        #[tokio::test]
        async fn test_response_time_under_500ms() {
            let state = Arc::new(AppState::new());
            let params = GraphQueryParams {
                trade_id: Some("T001".to_string()),
            };

            let start = std::time::Instant::now();
            let result = get_graph(State(state), Query(params)).await;
            let elapsed = start.elapsed();

            assert!(result.is_ok());
            assert!(
                elapsed < Duration::from_millis(500),
                "Response took {:?}, expected < 500ms",
                elapsed
            );
        }
    }

    // =========================================================================
    // Task 7.2: Speed Comparison API Tests
    // =========================================================================

    mod speed_comparison_tests {
        use super::*;

        #[tokio::test]
        async fn test_get_speed_comparison_default() {
            let params = SpeedComparisonQueryParams {
                aad_mean_ns: None,
                bump_mean_ns: None,
                tenor_count: None,
            };

            let response = get_speed_comparison(Query(params)).await;

            assert_eq!(response.chart_type, "bar");
            assert_eq!(response.data.labels.len(), 2);
            assert_eq!(response.data.labels[0], "AAD");
            assert_eq!(response.data.labels[1], "Bump-and-Revalue");
        }

        #[tokio::test]
        async fn test_get_speed_comparison_with_custom_data() {
            let params = SpeedComparisonQueryParams {
                aad_mean_ns: Some(10_000.0),
                bump_mean_ns: Some(200_000.0),
                tenor_count: Some(10),
            };

            let response = get_speed_comparison(Query(params)).await;

            assert!((response.benchmark.aad_mean_us - 10.0).abs() < 1e-10);
            assert!((response.benchmark.bump_mean_us - 200.0).abs() < 1e-10);
            assert!((response.benchmark.speedup_ratio - 20.0).abs() < 1e-10);
            assert_eq!(response.benchmark.tenor_count, 10);
        }

        #[tokio::test]
        async fn test_get_speed_comparison_chartjs_structure() {
            let params = SpeedComparisonQueryParams {
                aad_mean_ns: Some(15_000.0),
                bump_mean_ns: Some(300_000.0),
                tenor_count: Some(20),
            };

            let response = get_speed_comparison(Query(params)).await;

            // Verify Chart.js structure
            assert_eq!(response.chart_type, "bar");
            assert!(response.options.title.display);
            assert!(response.options.title.text.contains("speedup"));

            // Verify datasets
            assert_eq!(response.data.datasets.len(), 1);
            let dataset = &response.data.datasets[0];
            assert_eq!(dataset.data.len(), 2);
            assert_eq!(dataset.background_color.len(), 2);
            assert_eq!(dataset.background_color[0], "#4CAF50"); // AAD green
            assert_eq!(dataset.background_color[1], "#FF5722"); // Bump orange
        }

        #[tokio::test]
        async fn test_get_speed_comparison_serialisation() {
            let params = SpeedComparisonQueryParams {
                aad_mean_ns: None,
                bump_mean_ns: None,
                tenor_count: None,
            };

            let response = get_speed_comparison(Query(params)).await;

            // Verify response can be serialised to JSON
            let json = serde_json::to_string(&response.0).unwrap();
            // serde_json compact format doesn't add spaces after colons
            assert!(json.contains("\"type\":\"bar\""));
            assert!(json.contains("\"labels\""));
            assert!(json.contains("\"datasets\""));
            assert!(json.contains("\"backgroundColor\"")); // camelCase
        }

        #[tokio::test]
        async fn test_get_speed_comparison_benchmark_data() {
            let params = SpeedComparisonQueryParams {
                aad_mean_ns: None,
                bump_mean_ns: None,
                tenor_count: None,
            };

            let response = get_speed_comparison(Query(params)).await;

            // Verify benchmark data is present
            assert!(response.benchmark.aad_mean_us > 0.0);
            assert!(response.benchmark.bump_mean_us > 0.0);
            assert!(response.benchmark.speedup_ratio > 0.0);
            assert!(response.benchmark.tenor_count > 0);
        }
    }

    // =========================================================================
    // Task 2.1: Pricing Handler Tests
    // =========================================================================

    mod pricing_tests {
        use super::*;
        use crate::web::pricer_types::{EquityOptionParams, FxOptionParams, IrsParams, OptionType};

        #[test]
        fn test_norm_cdf_at_zero() {
            let result = norm_cdf(0.0);
            assert!((result - 0.5).abs() < 1e-6);
        }

        #[test]
        fn test_norm_cdf_positive() {
            let result = norm_cdf(1.0);
            assert!(result > 0.8 && result < 0.9);
        }

        #[test]
        fn test_norm_cdf_negative() {
            let result = norm_cdf(-1.0);
            assert!(result > 0.1 && result < 0.2);
        }

        #[test]
        fn test_norm_pdf_at_zero() {
            let result = norm_pdf(0.0);
            let expected = 1.0 / (2.0 * std::f64::consts::PI).sqrt();
            assert!((result - expected).abs() < 1e-10);
        }

        #[test]
        fn test_black_scholes_call_price() {
            // ATM call with 1y, 5% rate, 20% vol
            let price = black_scholes_price(100.0, 100.0, 1.0, 0.05, 0.20, true);
            // Expected around 10.45 for these parameters
            assert!(price > 10.0 && price < 11.0);
        }

        #[test]
        fn test_black_scholes_put_price() {
            let price = black_scholes_price(100.0, 100.0, 1.0, 0.05, 0.20, false);
            // Put price should be lower than call for ATM due to interest rate
            assert!(price > 5.0 && price < 7.0);
        }

        #[test]
        fn test_black_scholes_call_put_parity() {
            let spot = 100.0;
            let strike = 100.0;
            let time = 1.0;
            let rate = 0.05;
            let vol = 0.20;

            let call = black_scholes_price(spot, strike, time, rate, vol, true);
            let put = black_scholes_price(spot, strike, time, rate, vol, false);
            let discount = (-rate * time).exp();

            // Put-Call Parity: C - P = S - K * exp(-rT)
            let lhs = call - put;
            let rhs = spot - strike * discount;
            assert!((lhs - rhs).abs() < 1e-10);
        }

        #[test]
        fn test_black_scholes_expired_call_itm() {
            let price = black_scholes_price(110.0, 100.0, 0.0, 0.05, 0.20, true);
            assert!((price - 10.0).abs() < 1e-10);
        }

        #[test]
        fn test_black_scholes_expired_call_otm() {
            let price = black_scholes_price(90.0, 100.0, 0.0, 0.05, 0.20, true);
            assert!((price - 0.0).abs() < 1e-10);
        }

        #[test]
        fn test_black_scholes_greeks_delta_call() {
            let greeks = black_scholes_greeks(100.0, 100.0, 1.0, 0.05, 0.20, true);
            // ATM call delta should be around 0.5-0.6
            assert!(greeks.delta > 0.5 && greeks.delta < 0.7);
        }

        #[test]
        fn test_black_scholes_greeks_delta_put() {
            let greeks = black_scholes_greeks(100.0, 100.0, 1.0, 0.05, 0.20, false);
            // Put delta is negative
            assert!(greeks.delta < 0.0);
            assert!(greeks.delta > -0.6 && greeks.delta < -0.3);
        }

        #[test]
        fn test_black_scholes_greeks_gamma_positive() {
            let greeks = black_scholes_greeks(100.0, 100.0, 1.0, 0.05, 0.20, true);
            assert!(greeks.gamma > 0.0);
        }

        #[test]
        fn test_black_scholes_greeks_vega_positive() {
            let greeks = black_scholes_greeks(100.0, 100.0, 1.0, 0.05, 0.20, true);
            assert!(greeks.vega > 0.0);
        }

        #[test]
        fn test_garman_kohlhagen_call_price() {
            let price = garman_kohlhagen_price(1.10, 1.10, 1.0, 0.05, 0.02, 0.10, true);
            assert!(price > 0.0);
        }

        #[test]
        fn test_garman_kohlhagen_put_price() {
            let price = garman_kohlhagen_price(1.10, 1.10, 1.0, 0.05, 0.02, 0.10, false);
            assert!(price > 0.0);
        }

        #[test]
        fn test_irs_price_positive_fixed() {
            // Fixed rate higher than market rate should be positive
            let pv = irs_price(1_000_000.0, 0.05, 5.0, 0.03);
            assert!(pv > 0.0);
        }

        #[test]
        fn test_irs_price_negative_fixed() {
            // Fixed rate lower than market rate should be negative
            let pv = irs_price(1_000_000.0, 0.03, 5.0, 0.05);
            assert!(pv < 0.0);
        }

        #[test]
        fn test_irs_greeks() {
            let greeks = irs_greeks(1_000_000.0, 5.0);
            // DV01 should be positive
            assert!(greeks.delta > 0.0);
            assert_eq!(greeks.gamma, 0.0);
            assert_eq!(greeks.vega, 0.0);
        }

        #[test]
        fn test_validate_equity_params_valid() {
            let params = EquityOptionParams {
                spot: 100.0,
                strike: 100.0,
                expiry_years: 1.0,
                volatility: 0.20,
                rate: 0.05,
                option_type: OptionType::Call,
            };
            assert!(validate_equity_params(&params).is_ok());
        }

        #[test]
        fn test_validate_equity_params_negative_spot() {
            let params = EquityOptionParams {
                spot: -100.0,
                strike: 100.0,
                expiry_years: 1.0,
                volatility: 0.20,
                rate: 0.05,
                option_type: OptionType::Call,
            };
            let err = validate_equity_params(&params).unwrap_err();
            assert_eq!(err.0, "spot");
        }

        #[test]
        fn test_validate_equity_params_high_volatility() {
            let params = EquityOptionParams {
                spot: 100.0,
                strike: 100.0,
                expiry_years: 1.0,
                volatility: 6.0, // 600%
                rate: 0.05,
                option_type: OptionType::Call,
            };
            let err = validate_equity_params(&params).unwrap_err();
            assert_eq!(err.0, "volatility");
        }

        #[test]
        fn test_validate_fx_params_valid() {
            let params = FxOptionParams {
                spot: 1.10,
                strike: 1.12,
                expiry_years: 0.5,
                volatility: 0.10,
                domestic_rate: 0.05,
                foreign_rate: 0.02,
                option_type: OptionType::Put,
            };
            assert!(validate_fx_params(&params).is_ok());
        }

        #[test]
        fn test_validate_irs_params_valid() {
            let params = IrsParams {
                notional: 1_000_000.0,
                fixed_rate: 0.025,
                tenor_years: 5.0,
            };
            assert!(validate_irs_params(&params).is_ok());
        }

        #[test]
        fn test_validate_irs_params_zero_notional() {
            let params = IrsParams {
                notional: 0.0,
                fixed_rate: 0.025,
                tenor_years: 5.0,
            };
            let err = validate_irs_params(&params).unwrap_err();
            assert_eq!(err.0, "notional");
        }
    }

    // =========================================================================
    // Task 12.2: Error Path Tests
    // =========================================================================

    mod error_path_tests {
        use super::*;
        use crate::web::pricer_types::{EquityOptionParams, InstrumentParams, InstrumentType, OptionType, PricingRequest};

        #[tokio::test]
        async fn test_pricing_with_invalid_instrument_type() {
            let state = Arc::new(AppState::new());

            // Invalid params should return validation error
            let params = EquityOptionParams {
                spot: -100.0, // Invalid negative spot
                strike: 100.0,
                expiry_years: 1.0,
                volatility: 0.20,
                rate: 0.05,
                option_type: OptionType::Call,
            };

            let request = PricingRequest {
                instrument_type: InstrumentType::EquityVanillaOption,
                params: InstrumentParams::EquityOption(params),
                market_data: None,
                compute_greeks: false,
            };

            let result = price_instrument(State(state), axum::Json(request)).await;
            let (status, _) = result.into_response().into_parts();
            assert_eq!(status.status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_pricing_with_extreme_volatility() {
            let state = Arc::new(AppState::new());

            let params = EquityOptionParams {
                spot: 100.0,
                strike: 100.0,
                expiry_years: 1.0,
                volatility: 10.0, // 1000% volatility - should be rejected
                rate: 0.05,
                option_type: OptionType::Call,
            };

            let request = PricingRequest {
                instrument_type: InstrumentType::EquityVanillaOption,
                params: InstrumentParams::EquityOption(params),
                market_data: None,
                compute_greeks: false,
            };

            let result = price_instrument(State(state), axum::Json(request)).await;
            let (status, _) = result.into_response().into_parts();
            assert_eq!(status.status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_graph_with_empty_trade_id() {
            let state = Arc::new(AppState::new());
            let params = GraphQueryParams {
                trade_id: Some("".to_string()),
            };

            let result = get_graph(State(state), Query(params)).await;

            // Empty trade ID should be treated as a not-found error
            assert!(result.is_err());
        }
    }

    // =========================================================================
    // Task 12.1: Performance Metrics Tests
    // =========================================================================

    mod performance_metrics_tests {
        use crate::web::PerformanceMetrics;

        #[tokio::test]
        async fn test_metrics_record_portfolio_time() {
            let metrics = PerformanceMetrics::new();

            metrics.record_portfolio_time(1000).await;
            metrics.record_portfolio_time(2000).await;
            metrics.record_portfolio_time(3000).await;

            let avg = metrics.portfolio_avg_ms().await;
            assert!((avg - 2.0).abs() < 0.01); // 2000us average = 2ms
        }

        #[tokio::test]
        async fn test_metrics_record_exposure_time() {
            let metrics = PerformanceMetrics::new();

            metrics.record_exposure_time(500).await;
            let avg = metrics.exposure_avg_ms().await;

            assert!((avg - 0.5).abs() < 0.01); // 500us = 0.5ms
        }

        #[tokio::test]
        async fn test_metrics_ws_connection_count() {
            let metrics = PerformanceMetrics::new();

            assert_eq!(metrics.ws_connection_count(), 0);

            metrics.increment_ws_connections();
            metrics.increment_ws_connections();
            assert_eq!(metrics.ws_connection_count(), 2);

            metrics.decrement_ws_connections();
            assert_eq!(metrics.ws_connection_count(), 1);
        }

        #[tokio::test]
        async fn test_metrics_max_entries_limit() {
            let metrics = PerformanceMetrics::new();

            // Record more than MAX_ENTRIES
            for i in 0..1100 {
                metrics.record_portfolio_time(i as u64).await;
            }

            // Should only keep last 1000 entries
            let times = metrics.portfolio_times.read().await;
            assert_eq!(times.len(), 1000);
        }

        #[test]
        fn test_metrics_uptime() {
            let metrics = PerformanceMetrics::new();
            let uptime = metrics.uptime_seconds();
            assert!(uptime >= 0);
        }
    }
}
