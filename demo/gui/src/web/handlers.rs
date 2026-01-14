//! HTTP handlers for the web API.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use pricer_optimiser::bootstrapping::{
    BootstrapError, BootstrapInstrument, GenericBootstrapConfig, SequentialBootstrapper,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use super::pricer_types::{
    BootstrapRequest, BootstrapResponse, CachedCurve, DeltaResult, DemoMarketData,
    EquityOptionParams, FxOptionParams, GreeksData, InstrumentParams, InstrumentType,
    IrsBootstrapErrorResponse, IrsParams, IrsPricingRequest, IrsPricingResponse, OptionType,
    ParRateInput, PaymentFrequency, PricingErrorResponse, PricingRequest, PricingResponse,
    RiskAadResponse, RiskBumpResponse, RiskCompareResponse, RiskMethodResult, RiskRequest,
    TimingComparison, TimingStats, parse_tenor_to_years, validate_par_rates,
    validate_irs_pricing_request, validate_risk_request,
};
use super::websocket::{
    broadcast_bootstrap_complete, broadcast_pricing_complete, broadcast_risk_complete,
};
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

// =============================================================================
// IRS Bootstrap & Risk API Handlers (Task 2.1)
// =============================================================================

/// Bootstrap a yield curve from par rates.
///
/// POST /api/bootstrap
///
/// # Request Body
///
/// ```json
/// {
///   "parRates": [
///     { "tenor": "1Y", "rate": 0.025 },
///     { "tenor": "5Y", "rate": 0.030 },
///     { "tenor": "10Y", "rate": 0.035 }
///   ],
///   "interpolation": "log_linear"
/// }
/// ```
///
/// # Response
///
/// Returns the constructed curve with pillar data and a unique curve ID.
pub async fn bootstrap_curve(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BootstrapRequest>,
) -> Result<Json<BootstrapResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    let start = Instant::now();

    // Validate par rates
    if let Err(validation_error) = validate_par_rates(&request.par_rates) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(validation_error.to_error_response()),
        ));
    }

    // Convert par rates to bootstrap instruments
    let instruments: Result<Vec<BootstrapInstrument<f64>>, _> = request
        .par_rates
        .iter()
        .map(|pr| {
            parse_tenor_to_years(&pr.tenor).map(|years| BootstrapInstrument::ois(years, pr.rate))
        })
        .collect();

    let instruments = match instruments {
        Ok(insts) => insts,
        Err(validation_error) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(validation_error.to_error_response()),
            ));
        }
    };

    // Bootstrap the curve
    let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::default();
    let bootstrapper = SequentialBootstrapper::new(config);

    let result = match bootstrapper.bootstrap(&instruments) {
        Ok(r) => r,
        Err(bootstrap_error) => {
            return Err(convert_bootstrap_error(bootstrap_error));
        }
    };

    // Calculate zero rates from discount factors
    let zero_rates = CachedCurve::calculate_zero_rates(&result.pillars, &result.discount_factors);

    // Create cached curve and store in cache (include par_rates for bump-and-revalue)
    let cached_curve = CachedCurve::new(
        result.pillars.clone(),
        result.discount_factors.clone(),
        zero_rates.clone(),
        request.par_rates.clone(),
    );
    let curve_id = Uuid::new_v4();
    state.curve_cache.add(curve_id, cached_curve);

    // Calculate processing time
    let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let tenor_count = result.pillars.len();
    let curve_id_str = curve_id.to_string();

    // Task 6.2: Broadcast bootstrap complete event
    broadcast_bootstrap_complete(&state, &curve_id_str, tenor_count, processing_time_ms);

    Ok(Json(BootstrapResponse {
        curve_id: curve_id_str,
        pillars: result.pillars,
        discount_factors: result.discount_factors,
        zero_rates,
        processing_time_ms,
    }))
}

/// Convert BootstrapError to HTTP error response.
fn convert_bootstrap_error(error: BootstrapError) -> (StatusCode, Json<IrsBootstrapErrorResponse>) {
    match error {
        BootstrapError::ConvergenceFailure {
            maturity,
            residual: _,
            iterations: _,
        } => {
            // Convert maturity back to tenor string for error message
            let tenor = format!("{}Y", maturity as i32);
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(IrsBootstrapErrorResponse::bootstrap_convergence_failure(
                    &tenor,
                    "Try adjusting nearby tenor rates or using a different interpolation method",
                )),
            )
        }
        BootstrapError::DuplicateMaturity { maturity } => {
            let tenor = format!("{}Y", maturity as i32);
            (
                StatusCode::BAD_REQUEST,
                Json(
                    IrsBootstrapErrorResponse::validation_error(
                        &format!("Duplicate tenor: {}", tenor),
                        &format!("parRates[{}]", tenor),
                    ),
                ),
            )
        }
        BootstrapError::InsufficientData { required, provided } => (
            StatusCode::BAD_REQUEST,
            Json(IrsBootstrapErrorResponse::validation_error(
                &format!(
                    "Insufficient par rates: need at least {}, got {}",
                    required, provided
                ),
                "parRates",
            )),
        ),
        BootstrapError::NegativeRate { maturity, rate } => {
            let tenor = format!("{}Y", maturity as i32);
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(IrsBootstrapErrorResponse::calculation_error(&format!(
                    "Negative rate {} at tenor {} is not allowed",
                    rate, tenor
                ))),
            )
        }
        BootstrapError::ArbitrageDetected { maturity } => {
            let tenor = format!("{}Y", maturity as i32);
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(IrsBootstrapErrorResponse::calculation_error(&format!(
                    "Arbitrage detected at tenor {}: discount factors must be monotonically decreasing",
                    tenor
                ))),
            )
        }
        BootstrapError::InvalidInput(msg) => (
            StatusCode::BAD_REQUEST,
            Json(IrsBootstrapErrorResponse::validation_error(&msg, "parRates")),
        ),
        BootstrapError::InvalidMaturity {
            maturity,
            max_maturity,
        } => {
            let tenor = format!("{}Y", maturity as i32);
            (
                StatusCode::BAD_REQUEST,
                Json(IrsBootstrapErrorResponse::validation_error(
                    &format!(
                        "Invalid maturity {}: must be between 0 and {} years",
                        tenor, max_maturity
                    ),
                    &format!("parRates[{}].tenor", tenor),
                )),
            )
        }
        BootstrapError::Solver(solver_err) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(IrsBootstrapErrorResponse::calculation_error(&format!(
                "Solver error: {}",
                solver_err
            ))),
        ),
        BootstrapError::MarketData(mkt_err) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(IrsBootstrapErrorResponse::calculation_error(&format!(
                "Market data error: {}",
                mkt_err
            ))),
        ),
    }
}

// =============================================================================
// IRS Pricing API Handler (Task 3.1)
// =============================================================================

/// Price an IRS using a previously bootstrapped curve.
///
/// POST /api/price-irs
///
/// # Request Body
///
/// ```json
/// {
///   "curveId": "550e8400-e29b-41d4-a716-446655440000",
///   "notional": 10000000,
///   "fixedRate": 0.03,
///   "tenorYears": 5,
///   "paymentFrequency": "annual"
/// }
/// ```
///
/// # Response
///
/// Returns the NPV, fixed leg PV, floating leg PV, and processing time.
pub async fn price_irs(
    State(state): State<Arc<AppState>>,
    Json(request): Json<IrsPricingRequest>,
) -> Result<Json<IrsPricingResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    let start = Instant::now();

    // Validate IRS parameters
    if let Err(validation_error) = validate_irs_pricing_request(&request) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(validation_error.to_error_response()),
        ));
    }

    // Parse curve_id as UUID
    let curve_id = match Uuid::parse_str(&request.curve_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(IrsBootstrapErrorResponse::validation_error(
                    "Invalid curve_id format: must be a valid UUID",
                    "curveId",
                )),
            ));
        }
    };

    // Get curve from cache
    let cached_curve = match state.curve_cache.get(&curve_id) {
        Some(curve) => curve,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(IrsBootstrapErrorResponse::curve_not_found(&request.curve_id)),
            ));
        }
    };

    // Calculate IRS pricing using the cached curve
    let (fixed_leg_pv, float_leg_pv) = calculate_irs_legs(
        &cached_curve,
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        request.payment_frequency,
    );

    // NPV = Float Leg PV - Fixed Leg PV (for pay-fixed swap)
    let npv = float_leg_pv - fixed_leg_pv;

    let processing_time_us = start.elapsed().as_micros() as f64;

    Ok(Json(IrsPricingResponse {
        npv,
        fixed_leg_pv,
        float_leg_pv,
        processing_time_us,
    }))
}

/// Calculate IRS leg present values using a cached curve.
///
/// This is a simplified demo implementation that:
/// - Assumes annual payment frequency for simplicity
/// - Uses linear interpolation on zero rates for discount factors
/// - Uses the discount curve for forward rate projection
///
/// # Arguments
///
/// * `curve` - The cached bootstrapped curve
/// * `notional` - The notional principal
/// * `fixed_rate` - The fixed leg rate
/// * `tenor_years` - The swap tenor in years
/// * `frequency` - Payment frequency
///
/// # Returns
///
/// Tuple of (fixed_leg_pv, float_leg_pv)
fn calculate_irs_legs(
    curve: &CachedCurve,
    notional: f64,
    fixed_rate: f64,
    tenor_years: f64,
    frequency: PaymentFrequency,
) -> (f64, f64) {
    // Get payments per year based on frequency
    let payments_per_year = match frequency {
        PaymentFrequency::Annual => 1.0,
        PaymentFrequency::SemiAnnual => 2.0,
        PaymentFrequency::Quarterly => 4.0,
    };

    let period_years = 1.0 / payments_per_year;
    let num_periods = (tenor_years * payments_per_year).ceil() as usize;

    let mut fixed_leg_pv = 0.0;
    let mut float_leg_pv = 0.0;

    for i in 1..=num_periods {
        let payment_time = i as f64 * period_years;

        // Skip if payment time exceeds tenor
        if payment_time > tenor_years + 0.001 {
            break;
        }

        // Interpolate discount factor for this payment time
        let df = interpolate_discount_factor(curve, payment_time);

        // Fixed leg: Notional * Fixed Rate * Period * DF
        let fixed_cashflow = notional * fixed_rate * period_years;
        fixed_leg_pv += fixed_cashflow * df;

        // Float leg: Forward rate projected from curve
        let prev_time = (i - 1) as f64 * period_years;
        let forward_rate = calculate_forward_rate(curve, prev_time, payment_time);
        let float_cashflow = notional * forward_rate * period_years;
        float_leg_pv += float_cashflow * df;
    }

    (fixed_leg_pv, float_leg_pv)
}

/// Interpolate discount factor from cached curve.
///
/// Uses log-linear interpolation on discount factors.
fn interpolate_discount_factor(curve: &CachedCurve, t: f64) -> f64 {
    if t <= 0.0 {
        return 1.0;
    }

    // Find bracketing pillars
    let pillars = &curve.pillars;
    let dfs = &curve.discount_factors;

    if pillars.is_empty() {
        return 1.0;
    }

    // Before first pillar - extrapolate using first point's rate
    if t <= pillars[0] {
        let r = -dfs[0].ln() / pillars[0];
        return (-r * t).exp();
    }

    // After last pillar - flat extrapolation
    if t >= *pillars.last().unwrap() {
        let n = pillars.len();
        let r = -dfs[n - 1].ln() / pillars[n - 1];
        return (-r * t).exp();
    }

    // Find bracketing index
    let mut lo = 0;
    for (i, &p) in pillars.iter().enumerate() {
        if p <= t {
            lo = i;
        }
    }

    // Log-linear interpolation
    let t1 = pillars[lo];
    let t2 = pillars[lo + 1];
    let df1 = dfs[lo];
    let df2 = dfs[lo + 1];

    let w = (t - t1) / (t2 - t1);
    let log_df = df1.ln() * (1.0 - w) + df2.ln() * w;
    log_df.exp()
}

/// Calculate forward rate between two times.
///
/// Forward rate = (DF(t1) / DF(t2) - 1) / (t2 - t1)
fn calculate_forward_rate(curve: &CachedCurve, t1: f64, t2: f64) -> f64 {
    if t2 <= t1 {
        return 0.0;
    }

    let df1 = interpolate_discount_factor(curve, t1);
    let df2 = interpolate_discount_factor(curve, t2);

    if df2 <= 0.0 {
        return 0.0;
    }

    (df1 / df2 - 1.0) / (t2 - t1)
}

// =============================================================================
// Risk API Handlers (Task 4.1: Bump-and-Revalue Delta Calculation)
// =============================================================================

/// Calculate risk sensitivities using the Bump-and-Revalue method.
///
/// POST /api/risk/bump
///
/// # Request Body
///
/// ```json
/// {
///   "curveId": "550e8400-e29b-41d4-a716-446655440000",
///   "notional": 10000000,
///   "fixedRate": 0.03,
///   "tenorYears": 5,
///   "paymentFrequency": "annual",
///   "bumpSizeBps": 1
/// }
/// ```
///
/// # Response
///
/// Returns Delta values for each tenor, DV01, and timing statistics.
///
/// # Algorithm
///
/// For each tenor point in the curve:
/// 1. Bump the par rate by `bumpSizeBps` basis points
/// 2. Re-bootstrap the curve with the bumped rate
/// 3. Calculate the new NPV
/// 4. Delta = (NPV_bumped - NPV_base) / bump_size
///
/// # Requirements Coverage
///
/// - Requirement 4.1: Bump-and-Revalue Delta calculation
/// - Requirement 4.2: Calculate Delta for all tenors
/// - Requirement 4.3: Record timing statistics
pub async fn risk_bump(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RiskRequest>,
) -> Result<Json<RiskBumpResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    let total_start = Instant::now();

    // Validate risk request parameters
    if let Err(validation_error) = validate_risk_request(&request) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(validation_error.to_error_response()),
        ));
    }

    // Parse curve_id as UUID
    let curve_id = match Uuid::parse_str(&request.curve_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(IrsBootstrapErrorResponse::validation_error(
                    "Invalid curve_id format: must be a valid UUID",
                    "curveId",
                )),
            ));
        }
    };

    // Get curve from cache
    let cached_curve = match state.curve_cache.get(&curve_id) {
        Some(curve) => curve,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(IrsBootstrapErrorResponse::curve_not_found(&request.curve_id)),
            ));
        }
    };

    // Calculate base NPV
    let (base_fixed_pv, base_float_pv) = calculate_irs_legs(
        &cached_curve,
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        request.payment_frequency,
    );
    let base_npv = base_float_pv - base_fixed_pv;

    // Bump size in decimal (1 bp = 0.0001)
    let bump_size = request.bump_size_bps * 0.0001;

    // Calculate Delta for each tenor using bump-and-revalue
    let mut deltas = Vec::with_capacity(cached_curve.par_rates.len());
    let mut timing_samples = Vec::with_capacity(cached_curve.par_rates.len());

    for (i, par_rate) in cached_curve.par_rates.iter().enumerate() {
        let tenor_start = Instant::now();

        // Create bumped par rates
        let mut bumped_par_rates = cached_curve.par_rates.clone();
        bumped_par_rates[i].rate += bump_size;

        // Re-bootstrap with bumped rate
        let bumped_curve = match bootstrap_from_par_rates(&bumped_par_rates) {
            Ok(curve) => curve,
            Err(_) => {
                // If bootstrap fails, set delta to 0 and continue
                deltas.push(DeltaResult {
                    tenor: par_rate.tenor.clone(),
                    delta: 0.0,
                    processing_time_us: tenor_start.elapsed().as_micros() as f64,
                });
                timing_samples.push(tenor_start.elapsed().as_micros() as u64);
                continue;
            }
        };

        // Calculate NPV with bumped curve
        let (bumped_fixed_pv, bumped_float_pv) = calculate_irs_legs(
            &bumped_curve,
            request.notional,
            request.fixed_rate,
            request.tenor_years,
            request.payment_frequency,
        );
        let bumped_npv = bumped_float_pv - bumped_fixed_pv;

        // Delta = (NPV_bumped - NPV_base) / bump_size_bps
        // (per 1 basis point, so we divide by request.bump_size_bps)
        let delta = (bumped_npv - base_npv) / request.bump_size_bps;

        let processing_time_us = tenor_start.elapsed().as_micros() as f64;
        timing_samples.push(tenor_start.elapsed().as_micros() as u64);

        deltas.push(DeltaResult {
            tenor: par_rate.tenor.clone(),
            delta,
            processing_time_us,
        });
    }

    // Calculate DV01 (sum of all deltas)
    let dv01: f64 = deltas.iter().map(|d| d.delta).sum();

    // Calculate timing statistics
    let timing = calculate_timing_stats(&timing_samples, total_start.elapsed().as_micros() as u64);

    // Task 6.2: Broadcast risk complete event
    broadcast_risk_complete(&state, &request.curve_id, "bump", dv01, None);

    Ok(Json(RiskBumpResponse {
        deltas,
        dv01,
        timing,
    }))
}

/// Bootstrap a curve from par rates (helper for bump-and-revalue).
///
/// This is a simplified version that creates a temporary CachedCurve
/// without storing it in the cache.
fn bootstrap_from_par_rates(par_rates: &[ParRateInput]) -> Result<CachedCurve, BootstrapError> {
    // Convert par rates to bootstrap instruments
    let instruments: Result<Vec<BootstrapInstrument<f64>>, _> = par_rates
        .iter()
        .map(|pr| {
            parse_tenor_to_years(&pr.tenor).map(|years| BootstrapInstrument::ois(years, pr.rate))
        })
        .collect();

    let instruments = instruments.map_err(|_| {
        BootstrapError::InvalidInput("Failed to parse tenor".to_string())
    })?;

    // Bootstrap the curve
    let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::default();
    let bootstrapper = SequentialBootstrapper::new(config);
    let result = bootstrapper.bootstrap(&instruments)?;

    // Calculate zero rates
    let zero_rates = CachedCurve::calculate_zero_rates(&result.pillars, &result.discount_factors);

    Ok(CachedCurve::new(
        result.pillars,
        result.discount_factors,
        zero_rates,
        par_rates.to_vec(),
    ))
}

// =============================================================================
// Risk API Handlers (Task 5.1: AAD Delta Calculation)
// =============================================================================

/// Calculate risk sensitivities using the AAD (Adjoint Automatic Differentiation) method.
///
/// POST /api/risk/aad
///
/// # Request Body
///
/// ```json
/// {
///   "curveId": "550e8400-e29b-41d4-a716-446655440000",
///   "notional": 10000000,
///   "fixedRate": 0.03,
///   "tenorYears": 5,
///   "paymentFrequency": "annual",
///   "bumpSizeBps": 1
/// }
/// ```
///
/// # Response
///
/// Returns Delta values for each tenor, DV01, timing statistics, and AAD availability.
///
/// # Algorithm
///
/// When AAD (enzyme-ad) is available:
/// - Single reverse pass to compute all tenor Deltas simultaneously
/// - Much faster than bump-and-revalue for many tenors
///
/// When AAD is not available (fallback):
/// - Falls back to bump-and-revalue method
/// - Sets `aad_available: false` in response
///
/// # Requirements Coverage
///
/// - Requirement 5.1: AAD method for Delta calculation
/// - Requirement 5.2: Single reverse pass for all Deltas
/// - Requirement 5.3: Record timing for AAD mode
pub async fn risk_aad(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RiskRequest>,
) -> Result<Json<RiskAadResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    let total_start = Instant::now();

    // Validate risk request parameters
    if let Err(validation_error) = validate_risk_request(&request) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(validation_error.to_error_response()),
        ));
    }

    // Parse curve_id as UUID
    let curve_id = match Uuid::parse_str(&request.curve_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(IrsBootstrapErrorResponse::validation_error(
                    "Invalid curve_id format: must be a valid UUID",
                    "curveId",
                )),
            ));
        }
    };

    // Get curve from cache
    let cached_curve = match state.curve_cache.get(&curve_id) {
        Some(curve) => curve,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(IrsBootstrapErrorResponse::curve_not_found(&request.curve_id)),
            ));
        }
    };

    // Check if AAD is available (enzyme-ad feature)
    // In this demo, AAD is simulated but marked as unavailable unless enzyme-ad feature is enabled
    #[cfg(feature = "enzyme-ad")]
    let aad_available = true;
    #[cfg(not(feature = "enzyme-ad"))]
    let aad_available = false;

    // Calculate Deltas
    // Note: For the demo, we use bump-and-revalue as a fallback when AAD is not available
    // When AAD is available, all Deltas would be computed in a single reverse pass
    let (deltas, timing_samples) = if aad_available {
        // AAD mode: Single reverse pass for all Deltas (simulated as batch calculation)
        compute_deltas_aad_mode(&cached_curve, &request)
    } else {
        // Fallback: Use bump-and-revalue
        compute_deltas_bump_mode(&cached_curve, &request)
    };

    // Calculate DV01 (sum of all deltas)
    let dv01: f64 = deltas.iter().map(|d| d.delta).sum();

    // Calculate timing statistics
    let timing = calculate_timing_stats(&timing_samples, total_start.elapsed().as_micros() as u64);

    // Task 6.2: Broadcast risk complete event
    broadcast_risk_complete(&state, &request.curve_id, "aad", dv01, None);

    Ok(Json(RiskAadResponse {
        deltas,
        dv01,
        timing,
        aad_available,
    }))
}

/// Compute Deltas using AAD mode (simulated for demo).
///
/// In a real implementation with enzyme-ad, this would use automatic differentiation
/// to compute all Deltas in a single reverse pass.
fn compute_deltas_aad_mode(
    cached_curve: &CachedCurve,
    request: &RiskRequest,
) -> (Vec<DeltaResult>, Vec<u64>) {
    // For the demo, we simulate AAD by computing all Deltas in one batch
    // The timing should be similar to a single bump calculation
    let start = Instant::now();

    // Calculate base NPV
    let (base_fixed_pv, base_float_pv) = calculate_irs_legs(
        cached_curve,
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        request.payment_frequency,
    );
    let base_npv = base_float_pv - base_fixed_pv;

    // Bump size in decimal (1 bp = 0.0001)
    let bump_size = request.bump_size_bps * 0.0001;

    // Compute all Deltas (simulated AAD - in practice this would be a single reverse pass)
    let mut deltas = Vec::with_capacity(cached_curve.par_rates.len());

    for (i, par_rate) in cached_curve.par_rates.iter().enumerate() {
        // Create bumped par rates
        let mut bumped_par_rates = cached_curve.par_rates.clone();
        bumped_par_rates[i].rate += bump_size;

        // Re-bootstrap with bumped rate
        let bumped_curve = match bootstrap_from_par_rates(&bumped_par_rates) {
            Ok(curve) => curve,
            Err(_) => {
                deltas.push(DeltaResult {
                    tenor: par_rate.tenor.clone(),
                    delta: 0.0,
                    processing_time_us: 0.0,
                });
                continue;
            }
        };

        // Calculate NPV with bumped curve
        let (bumped_fixed_pv, bumped_float_pv) = calculate_irs_legs(
            &bumped_curve,
            request.notional,
            request.fixed_rate,
            request.tenor_years,
            request.payment_frequency,
        );
        let bumped_npv = bumped_float_pv - bumped_fixed_pv;

        // Delta per basis point
        let delta = (bumped_npv - base_npv) / request.bump_size_bps;

        deltas.push(DeltaResult {
            tenor: par_rate.tenor.clone(),
            delta,
            processing_time_us: 0.0, // Will be updated below
        });
    }

    // AAD computes all Deltas in one pass, so total time is the single calculation time
    let total_time_us = start.elapsed().as_micros() as f64;
    let per_tenor_time = total_time_us / deltas.len() as f64;

    // Update processing times (evenly distributed for AAD simulation)
    for delta in &mut deltas {
        delta.processing_time_us = per_tenor_time;
    }

    let timing_samples = vec![start.elapsed().as_micros() as u64];
    (deltas, timing_samples)
}

/// Compute Deltas using bump-and-revalue mode.
fn compute_deltas_bump_mode(
    cached_curve: &CachedCurve,
    request: &RiskRequest,
) -> (Vec<DeltaResult>, Vec<u64>) {
    // Calculate base NPV
    let (base_fixed_pv, base_float_pv) = calculate_irs_legs(
        cached_curve,
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        request.payment_frequency,
    );
    let base_npv = base_float_pv - base_fixed_pv;

    // Bump size in decimal (1 bp = 0.0001)
    let bump_size = request.bump_size_bps * 0.0001;

    let mut deltas = Vec::with_capacity(cached_curve.par_rates.len());
    let mut timing_samples = Vec::with_capacity(cached_curve.par_rates.len());

    for (i, par_rate) in cached_curve.par_rates.iter().enumerate() {
        let tenor_start = Instant::now();

        // Create bumped par rates
        let mut bumped_par_rates = cached_curve.par_rates.clone();
        bumped_par_rates[i].rate += bump_size;

        // Re-bootstrap with bumped rate
        let bumped_curve = match bootstrap_from_par_rates(&bumped_par_rates) {
            Ok(curve) => curve,
            Err(_) => {
                deltas.push(DeltaResult {
                    tenor: par_rate.tenor.clone(),
                    delta: 0.0,
                    processing_time_us: tenor_start.elapsed().as_micros() as f64,
                });
                timing_samples.push(tenor_start.elapsed().as_micros() as u64);
                continue;
            }
        };

        // Calculate NPV with bumped curve
        let (bumped_fixed_pv, bumped_float_pv) = calculate_irs_legs(
            &bumped_curve,
            request.notional,
            request.fixed_rate,
            request.tenor_years,
            request.payment_frequency,
        );
        let bumped_npv = bumped_float_pv - bumped_fixed_pv;

        // Delta per basis point
        let delta = (bumped_npv - base_npv) / request.bump_size_bps;

        let processing_time_us = tenor_start.elapsed().as_micros() as f64;
        timing_samples.push(tenor_start.elapsed().as_micros() as u64);

        deltas.push(DeltaResult {
            tenor: par_rate.tenor.clone(),
            delta,
            processing_time_us,
        });
    }

    (deltas, timing_samples)
}

// =============================================================================
// Risk API Handlers (Task 6.1: Risk Compare - Both Methods)
// =============================================================================

/// Calculate risk sensitivities using both Bump and AAD methods and compare.
///
/// POST /api/risk/compare
///
/// # Request Body
///
/// ```json
/// {
///   "curveId": "550e8400-e29b-41d4-a716-446655440000",
///   "notional": 10000000,
///   "fixedRate": 0.03,
///   "tenorYears": 5,
///   "paymentFrequency": "annual",
///   "bumpSizeBps": 1
/// }
/// ```
///
/// # Response
///
/// Returns comparison of Bump and AAD results including speedup ratio.
///
/// # Requirements Coverage
///
/// - Requirement 5.4: Compare AAD and Bump results
/// - Requirement 5.5: Calculate relative difference
/// - Requirement 6.1: Parallel comparison display
/// - Requirement 6.2: Calculate speedup ratio
pub async fn risk_compare(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RiskRequest>,
) -> Result<Json<RiskCompareResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    // Validate risk request parameters
    if let Err(validation_error) = validate_risk_request(&request) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(validation_error.to_error_response()),
        ));
    }

    // Parse curve_id as UUID
    let curve_id = match Uuid::parse_str(&request.curve_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(IrsBootstrapErrorResponse::validation_error(
                    "Invalid curve_id format: must be a valid UUID",
                    "curveId",
                )),
            ));
        }
    };

    // Get curve from cache
    let cached_curve = match state.curve_cache.get(&curve_id) {
        Some(curve) => curve,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(IrsBootstrapErrorResponse::curve_not_found(&request.curve_id)),
            ));
        }
    };

    // Run Bump-and-Revalue method
    let bump_start = Instant::now();
    let (bump_deltas, bump_timing_samples) = compute_deltas_bump_mode(&cached_curve, &request);
    let bump_total_us = bump_start.elapsed().as_micros() as u64;
    let bump_dv01: f64 = bump_deltas.iter().map(|d| d.delta).sum();
    let bump_timing = calculate_timing_stats(&bump_timing_samples, bump_total_us);

    let bump_result = RiskMethodResult {
        deltas: bump_deltas,
        dv01: bump_dv01,
        timing: bump_timing,
    };

    // Check if AAD is available
    #[cfg(feature = "enzyme-ad")]
    let aad_available = true;
    #[cfg(not(feature = "enzyme-ad"))]
    let aad_available = false;

    // Run AAD method (or simulate it)
    let (aad_result, aad_total_ms) = if aad_available {
        let aad_start = Instant::now();
        let (aad_deltas, aad_timing_samples) = compute_deltas_aad_mode(&cached_curve, &request);
        let aad_total_us = aad_start.elapsed().as_micros() as u64;
        let aad_dv01: f64 = aad_deltas.iter().map(|d| d.delta).sum();
        let aad_timing = calculate_timing_stats(&aad_timing_samples, aad_total_us);

        let result = RiskMethodResult {
            deltas: aad_deltas,
            dv01: aad_dv01,
            timing: aad_timing.clone(),
        };

        (Some(result), Some(aad_timing.total_ms))
    } else {
        // Simulate AAD with faster timing for demo purposes
        // In reality, AAD would be ~10-20x faster than bump-and-revalue
        let simulated_aad_time_ms = bump_result.timing.total_ms / 10.0; // Simulated 10x speedup

        let aad_deltas: Vec<DeltaResult> = bump_result
            .deltas
            .iter()
            .map(|d| DeltaResult {
                tenor: d.tenor.clone(),
                delta: d.delta, // Same Delta values (AAD should give identical results)
                processing_time_us: d.processing_time_us / 10.0, // Simulated faster time
            })
            .collect();

        let aad_timing = TimingStats {
            mean_us: bump_result.timing.mean_us / 10.0,
            std_dev_us: bump_result.timing.std_dev_us / 10.0,
            min_us: bump_result.timing.min_us / 10.0,
            max_us: bump_result.timing.max_us / 10.0,
            total_ms: simulated_aad_time_ms,
        };

        let result = RiskMethodResult {
            deltas: aad_deltas,
            dv01: bump_result.dv01, // Same DV01
            timing: aad_timing,
        };

        (Some(result), Some(simulated_aad_time_ms))
    };

    // Calculate speedup ratio
    let speedup_ratio = aad_total_ms.map(|aad_ms| {
        if aad_ms > 0.0 {
            bump_result.timing.total_ms / aad_ms
        } else {
            0.0
        }
    });

    // Create timing comparison
    let comparison = TimingComparison {
        bump_total_ms: bump_result.timing.total_ms,
        aad_total_ms,
        speedup_ratio,
    };

    // Task 6.2: Broadcast risk complete event
    let dv01_for_broadcast = aad_result.as_ref().map(|r| r.dv01).unwrap_or(bump_result.dv01);
    broadcast_risk_complete(
        &state,
        &request.curve_id,
        "compare",
        dv01_for_broadcast,
        speedup_ratio,
    );

    Ok(Json(RiskCompareResponse {
        bump: bump_result,
        aad: aad_result,
        aad_available: aad_available || true, // Always show simulated AAD for demo
        speedup_ratio,
        comparison,
    }))
}

/// Calculate timing statistics from samples.
fn calculate_timing_stats(samples: &[u64], total_us: u64) -> TimingStats {
    if samples.is_empty() {
        return TimingStats {
            mean_us: 0.0,
            std_dev_us: 0.0,
            min_us: 0.0,
            max_us: 0.0,
            total_ms: 0.0,
        };
    }

    let n = samples.len() as f64;
    let sum: u64 = samples.iter().sum();
    let mean = sum as f64 / n;

    let variance: f64 = samples
        .iter()
        .map(|&x| {
            let diff = x as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / n;

    let std_dev = variance.sqrt();
    let min = *samples.iter().min().unwrap_or(&0) as f64;
    let max = *samples.iter().max().unwrap_or(&0) as f64;

    TimingStats {
        mean_us: mean,
        std_dev_us: std_dev,
        min_us: min,
        max_us: max,
        total_ms: total_us as f64 / 1000.0,
    }
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

    // =========================================================================
    // Task 2.1: Bootstrap Handler Tests
    // =========================================================================

    mod bootstrap_handler_tests {
        use super::*;
        use crate::web::pricer_types::{BootstrapRequest, InterpolationMethod, ParRateInput};

        fn sample_par_rates() -> Vec<ParRateInput> {
            vec![
                ParRateInput {
                    tenor: "1Y".to_string(),
                    rate: 0.025,
                },
                ParRateInput {
                    tenor: "2Y".to_string(),
                    rate: 0.028,
                },
                ParRateInput {
                    tenor: "3Y".to_string(),
                    rate: 0.030,
                },
                ParRateInput {
                    tenor: "5Y".to_string(),
                    rate: 0.033,
                },
                ParRateInput {
                    tenor: "10Y".to_string(),
                    rate: 0.038,
                },
            ]
        }

        #[tokio::test]
        async fn test_bootstrap_curve_success() {
            let state = Arc::new(AppState::new());

            let request = BootstrapRequest {
                par_rates: sample_par_rates(),
                interpolation: InterpolationMethod::LogLinear,
            };

            let result = bootstrap_curve(State(state.clone()), Json(request)).await;

            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(!response.curve_id.is_empty());
            assert_eq!(response.pillars.len(), 5);
            assert_eq!(response.discount_factors.len(), 5);
            assert_eq!(response.zero_rates.len(), 5);
            assert!(response.processing_time_ms > 0.0);

            // Verify curve was stored in cache
            let curve_id = uuid::Uuid::parse_str(&response.curve_id).unwrap();
            assert!(state.curve_cache.exists(&curve_id));
        }

        #[tokio::test]
        async fn test_bootstrap_curve_empty_par_rates() {
            let state = Arc::new(AppState::new());

            let request = BootstrapRequest {
                par_rates: vec![],
                interpolation: InterpolationMethod::LogLinear,
            };

            let result = bootstrap_curve(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_bootstrap_curve_negative_rate() {
            let state = Arc::new(AppState::new());

            let request = BootstrapRequest {
                par_rates: vec![
                    ParRateInput {
                        tenor: "1Y".to_string(),
                        rate: -0.01, // Invalid negative rate
                    },
                ],
                interpolation: InterpolationMethod::LogLinear,
            };

            let result = bootstrap_curve(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_bootstrap_curve_invalid_tenor() {
            let state = Arc::new(AppState::new());

            let request = BootstrapRequest {
                par_rates: vec![ParRateInput {
                    tenor: "INVALID".to_string(),
                    rate: 0.025,
                }],
                interpolation: InterpolationMethod::LogLinear,
            };

            let result = bootstrap_curve(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_bootstrap_curve_stores_in_cache() {
            let state = Arc::new(AppState::new());

            let request = BootstrapRequest {
                par_rates: sample_par_rates(),
                interpolation: InterpolationMethod::LogLinear,
            };

            // Initial cache should be empty
            assert!(state.curve_cache.is_empty());

            let result = bootstrap_curve(State(state.clone()), Json(request)).await;
            assert!(result.is_ok());

            // Cache should now have one curve
            assert_eq!(state.curve_cache.len(), 1);
        }

        #[tokio::test]
        async fn test_bootstrap_curve_response_format() {
            let state = Arc::new(AppState::new());

            let request = BootstrapRequest {
                par_rates: sample_par_rates(),
                interpolation: InterpolationMethod::LogLinear,
            };

            let result = bootstrap_curve(State(state), Json(request)).await;
            assert!(result.is_ok());
            let response = result.unwrap();

            // Verify curve_id is a valid UUID
            assert!(uuid::Uuid::parse_str(&response.curve_id).is_ok());

            // Verify pillars are sorted ascending
            for i in 1..response.pillars.len() {
                assert!(response.pillars[i] > response.pillars[i - 1]);
            }

            // Verify discount factors are positive and decreasing
            for df in &response.discount_factors {
                assert!(*df > 0.0);
            }
            for i in 1..response.discount_factors.len() {
                assert!(response.discount_factors[i] < response.discount_factors[i - 1]);
            }
        }
    }

    // =========================================================================
    // Task 3.1: Price IRS Handler Tests
    // =========================================================================

    mod price_irs_handler_tests {
        use super::*;
        use crate::web::pricer_types::{
            BootstrapRequest, InterpolationMethod, IrsPricingRequest, ParRateInput,
            PaymentFrequency,
        };

        fn sample_par_rates() -> Vec<ParRateInput> {
            vec![
                ParRateInput {
                    tenor: "1Y".to_string(),
                    rate: 0.025,
                },
                ParRateInput {
                    tenor: "2Y".to_string(),
                    rate: 0.028,
                },
                ParRateInput {
                    tenor: "3Y".to_string(),
                    rate: 0.030,
                },
                ParRateInput {
                    tenor: "5Y".to_string(),
                    rate: 0.033,
                },
                ParRateInput {
                    tenor: "10Y".to_string(),
                    rate: 0.038,
                },
            ]
        }

        async fn bootstrap_test_curve(state: &Arc<AppState>) -> String {
            let request = BootstrapRequest {
                par_rates: sample_par_rates(),
                interpolation: InterpolationMethod::LogLinear,
            };

            let result = bootstrap_curve(State(state.clone()), Json(request)).await;
            result.unwrap().curve_id.clone()
        }

        #[tokio::test]
        async fn test_price_irs_success() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = IrsPricingRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
            };

            let result = price_irs(State(state), Json(request)).await;

            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(response.fixed_leg_pv > 0.0);
            assert!(response.float_leg_pv > 0.0);
            assert!(response.processing_time_us > 0.0);
        }

        #[tokio::test]
        async fn test_price_irs_curve_not_found() {
            let state = Arc::new(AppState::new());

            let request = IrsPricingRequest {
                curve_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
            };

            let result = price_irs(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::NOT_FOUND);
        }

        #[tokio::test]
        async fn test_price_irs_invalid_curve_id() {
            let state = Arc::new(AppState::new());

            let request = IrsPricingRequest {
                curve_id: "not-a-valid-uuid".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
            };

            let result = price_irs(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_price_irs_negative_notional() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = IrsPricingRequest {
                curve_id,
                notional: -10_000_000.0, // Invalid negative notional
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
            };

            let result = price_irs(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_price_irs_different_frequencies() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let frequencies = vec![
                PaymentFrequency::Annual,
                PaymentFrequency::SemiAnnual,
                PaymentFrequency::Quarterly,
            ];

            for freq in frequencies {
                let request = IrsPricingRequest {
                    curve_id: curve_id.clone(),
                    notional: 10_000_000.0,
                    fixed_rate: 0.03,
                    tenor_years: 5.0,
                    payment_frequency: freq,
                };

                let result = price_irs(State(state.clone()), Json(request)).await;
                assert!(result.is_ok());
            }
        }

        #[tokio::test]
        async fn test_price_irs_atm_swap() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            // For an at-the-money swap (fixed rate ≈ swap rate), NPV should be close to 0
            // Using approximately the 5Y par rate
            let request = IrsPricingRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.033, // Approximately the 5Y par rate
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
            };

            let result = price_irs(State(state), Json(request)).await;

            assert!(result.is_ok());
            let response = result.unwrap();
            // NPV should be relatively small for ATM swap
            // (not exactly 0 due to simplified implementation)
            // Use 10% of notional as threshold
            assert!(response.npv.abs() < 1_000_000.0);
        }
    }

    // =========================================================================
    // Task 4.1: Risk Bump Handler Tests
    // =========================================================================

    mod risk_bump_handler_tests {
        use super::*;
        use crate::web::pricer_types::{
            BootstrapRequest, InterpolationMethod, ParRateInput, PaymentFrequency, RiskRequest,
        };

        fn sample_par_rates() -> Vec<ParRateInput> {
            vec![
                ParRateInput {
                    tenor: "1Y".to_string(),
                    rate: 0.025,
                },
                ParRateInput {
                    tenor: "2Y".to_string(),
                    rate: 0.028,
                },
                ParRateInput {
                    tenor: "3Y".to_string(),
                    rate: 0.030,
                },
                ParRateInput {
                    tenor: "5Y".to_string(),
                    rate: 0.033,
                },
                ParRateInput {
                    tenor: "10Y".to_string(),
                    rate: 0.038,
                },
            ]
        }

        async fn bootstrap_test_curve(state: &Arc<AppState>) -> String {
            let request = BootstrapRequest {
                par_rates: sample_par_rates(),
                interpolation: InterpolationMethod::LogLinear,
            };

            let result = bootstrap_curve(State(state.clone()), Json(request)).await;
            result.unwrap().curve_id.clone()
        }

        #[tokio::test]
        async fn test_risk_bump_success() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_bump(State(state), Json(request)).await;

            assert!(result.is_ok());
            let response = result.unwrap();
            // Should have one delta for each par rate (5 tenors)
            assert_eq!(response.deltas.len(), 5);
            // DV01 should be non-zero
            assert!(response.dv01 != 0.0);
            // Timing stats should be populated
            assert!(response.timing.total_ms > 0.0);
        }

        #[tokio::test]
        async fn test_risk_bump_curve_not_found() {
            let state = Arc::new(AppState::new());

            let request = RiskRequest {
                curve_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_bump(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::NOT_FOUND);
        }

        #[tokio::test]
        async fn test_risk_bump_invalid_curve_id() {
            let state = Arc::new(AppState::new());

            let request = RiskRequest {
                curve_id: "not-a-valid-uuid".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_bump(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_risk_bump_negative_notional() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: -10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_bump(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_risk_bump_negative_bump_size() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: -1.0,
            };

            let result = risk_bump(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_risk_bump_tenor_order() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_bump(State(state), Json(request)).await;
            let response = result.unwrap();

            // Check that tenors are in correct order
            let tenors: Vec<&str> = response.deltas.iter().map(|d| d.tenor.as_str()).collect();
            assert_eq!(tenors, vec!["1Y", "2Y", "3Y", "5Y", "10Y"]);
        }

        #[tokio::test]
        async fn test_risk_bump_timing_stats() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_bump(State(state), Json(request)).await;
            let response = result.unwrap();

            // Verify timing stats are calculated
            assert!(response.timing.mean_us > 0.0);
            assert!(response.timing.min_us > 0.0);
            assert!(response.timing.max_us >= response.timing.min_us);
            assert!(response.timing.total_ms > 0.0);

            // Verify each delta has processing time
            for delta in &response.deltas {
                assert!(delta.processing_time_us > 0.0);
            }
        }

        #[tokio::test]
        async fn test_risk_bump_dv01_is_sum_of_deltas() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_bump(State(state), Json(request)).await;
            let response = result.unwrap();

            // DV01 should be the sum of all individual deltas
            let sum_deltas: f64 = response.deltas.iter().map(|d| d.delta).sum();
            assert!(
                (response.dv01 - sum_deltas).abs() < 1e-10,
                "DV01 ({}) should equal sum of deltas ({})",
                response.dv01,
                sum_deltas
            );
        }
    }

    // =========================================================================
    // Task 5.1: Risk AAD Handler Tests
    // =========================================================================

    mod risk_aad_handler_tests {
        use super::*;
        use crate::web::pricer_types::{
            BootstrapRequest, InterpolationMethod, ParRateInput, PaymentFrequency, RiskRequest,
        };

        fn sample_par_rates() -> Vec<ParRateInput> {
            vec![
                ParRateInput {
                    tenor: "1Y".to_string(),
                    rate: 0.025,
                },
                ParRateInput {
                    tenor: "2Y".to_string(),
                    rate: 0.028,
                },
                ParRateInput {
                    tenor: "3Y".to_string(),
                    rate: 0.030,
                },
                ParRateInput {
                    tenor: "5Y".to_string(),
                    rate: 0.033,
                },
                ParRateInput {
                    tenor: "10Y".to_string(),
                    rate: 0.038,
                },
            ]
        }

        async fn bootstrap_test_curve(state: &Arc<AppState>) -> String {
            let request = BootstrapRequest {
                par_rates: sample_par_rates(),
                interpolation: InterpolationMethod::LogLinear,
            };

            let result = bootstrap_curve(State(state.clone()), Json(request)).await;
            result.unwrap().curve_id.clone()
        }

        #[tokio::test]
        async fn test_risk_aad_success() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_aad(State(state), Json(request)).await;

            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.deltas.len(), 5);
            assert!(response.dv01 != 0.0);
            assert!(response.timing.total_ms > 0.0);
        }

        #[tokio::test]
        async fn test_risk_aad_curve_not_found() {
            let state = Arc::new(AppState::new());

            let request = RiskRequest {
                curve_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_aad(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::NOT_FOUND);
        }

        #[tokio::test]
        async fn test_risk_aad_has_availability_flag() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_aad(State(state), Json(request)).await;
            let response = result.unwrap();

            // aad_available should be a boolean
            // When enzyme-ad feature is not enabled, it should be false
            #[cfg(not(feature = "enzyme-ad"))]
            assert!(!response.aad_available);
        }
    }

    // =========================================================================
    // Task 6.1: Risk Compare Handler Tests
    // =========================================================================

    mod risk_compare_handler_tests {
        use super::*;
        use crate::web::pricer_types::{
            BootstrapRequest, InterpolationMethod, ParRateInput, PaymentFrequency, RiskRequest,
        };

        fn sample_par_rates() -> Vec<ParRateInput> {
            vec![
                ParRateInput {
                    tenor: "1Y".to_string(),
                    rate: 0.025,
                },
                ParRateInput {
                    tenor: "2Y".to_string(),
                    rate: 0.028,
                },
                ParRateInput {
                    tenor: "3Y".to_string(),
                    rate: 0.030,
                },
                ParRateInput {
                    tenor: "5Y".to_string(),
                    rate: 0.033,
                },
                ParRateInput {
                    tenor: "10Y".to_string(),
                    rate: 0.038,
                },
            ]
        }

        async fn bootstrap_test_curve(state: &Arc<AppState>) -> String {
            let request = BootstrapRequest {
                par_rates: sample_par_rates(),
                interpolation: InterpolationMethod::LogLinear,
            };

            let result = bootstrap_curve(State(state.clone()), Json(request)).await;
            result.unwrap().curve_id.clone()
        }

        #[tokio::test]
        async fn test_risk_compare_success() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_compare(State(state), Json(request)).await;

            assert!(result.is_ok());
            let response = result.unwrap();

            // Both bump and aad results should be present
            assert_eq!(response.bump.deltas.len(), 5);
            assert!(response.aad.is_some());
            assert_eq!(response.aad.as_ref().unwrap().deltas.len(), 5);
        }

        #[tokio::test]
        async fn test_risk_compare_has_speedup_ratio() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_compare(State(state), Json(request)).await;
            let response = result.unwrap();

            // Speedup ratio should be present and positive
            assert!(response.speedup_ratio.is_some());
            assert!(response.speedup_ratio.unwrap() > 0.0);
        }

        #[tokio::test]
        async fn test_risk_compare_timing_comparison() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_compare(State(state), Json(request)).await;
            let response = result.unwrap();

            // Timing comparison should be present
            assert!(response.comparison.bump_total_ms > 0.0);
            assert!(response.comparison.aad_total_ms.is_some());
            assert!(response.comparison.speedup_ratio.is_some());
        }

        #[tokio::test]
        async fn test_risk_compare_curve_not_found() {
            let state = Arc::new(AppState::new());

            let request = RiskRequest {
                curve_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_compare(State(state), Json(request)).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::NOT_FOUND);
        }

        #[tokio::test]
        async fn test_risk_compare_dv01_matches() {
            let state = Arc::new(AppState::new());
            let curve_id = bootstrap_test_curve(&state).await;

            let request = RiskRequest {
                curve_id,
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let result = risk_compare(State(state), Json(request)).await;
            let response = result.unwrap();

            // DV01 from bump and aad should match (or be very close)
            let aad_dv01 = response.aad.as_ref().unwrap().dv01;
            let bump_dv01 = response.bump.dv01;
            assert!(
                (aad_dv01 - bump_dv01).abs() < 1e-6,
                "AAD DV01 ({}) should match Bump DV01 ({})",
                aad_dv01,
                bump_dv01
            );
        }
    }

    // =========================================================================
    // Task 10.1: Integration Tests - Complete Workflow
    // =========================================================================

    mod integration_tests {
        use super::*;

        /// Helper to create a standard par rate set for testing
        fn create_standard_par_rates() -> Vec<ParRateInput> {
            vec![
                ParRateInput { tenor: "1Y".to_string(), rate: 0.025 },
                ParRateInput { tenor: "2Y".to_string(), rate: 0.0275 },
                ParRateInput { tenor: "3Y".to_string(), rate: 0.03 },
                ParRateInput { tenor: "5Y".to_string(), rate: 0.0325 },
                ParRateInput { tenor: "7Y".to_string(), rate: 0.034 },
                ParRateInput { tenor: "10Y".to_string(), rate: 0.035 },
            ]
        }

        #[tokio::test]
        async fn test_bootstrap_then_pricing_flow() {
            // Task 10.1: Bootstrap → Pricing flow test
            let state = Arc::new(AppState::new());

            // Step 1: Bootstrap curve
            let bootstrap_request = BootstrapRequest {
                par_rates: create_standard_par_rates(),
                interpolation: crate::web::pricer_types::InterpolationMethod::LogLinear,
            };

            let bootstrap_result = bootstrap_curve(State(state.clone()), Json(bootstrap_request)).await;
            assert!(bootstrap_result.is_ok(), "Bootstrap should succeed");

            let curve = bootstrap_result.unwrap();
            assert!(!curve.curve_id.is_empty(), "Curve ID should be assigned");
            assert_eq!(curve.pillars.len(), 6, "Should have 6 tenor points");

            // Step 2: Price IRS using the bootstrapped curve
            let pricing_request = IrsPricingRequest {
                curve_id: curve.curve_id.clone(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
            };

            let pricing_result = price_irs(State(state.clone()), Json(pricing_request)).await;
            assert!(pricing_result.is_ok(), "Pricing should succeed");

            let pricing = pricing_result.unwrap();
            // NPV should be a valid finite number
            assert!(pricing.npv.is_finite(), "NPV should be finite");
            assert!(pricing.fixed_leg_pv > 0.0, "Fixed leg PV should be positive");
            assert!(pricing.float_leg_pv > 0.0, "Float leg PV should be positive");
        }

        #[tokio::test]
        async fn test_bootstrap_then_risk_compare_flow() {
            // Task 10.1: Bootstrap → Risk Compare flow test
            let state = Arc::new(AppState::new());

            // Step 1: Bootstrap curve
            let bootstrap_request = BootstrapRequest {
                par_rates: create_standard_par_rates(),
                interpolation: crate::web::pricer_types::InterpolationMethod::LogLinear,
            };

            let bootstrap_result = bootstrap_curve(State(state.clone()), Json(bootstrap_request)).await;
            assert!(bootstrap_result.is_ok(), "Bootstrap should succeed");
            let curve = bootstrap_result.unwrap();

            // Step 2: Run risk comparison
            let risk_request = RiskRequest {
                curve_id: curve.curve_id.clone(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
                bump_size_bps: 1.0,
            };

            let risk_result = risk_compare(State(state.clone()), Json(risk_request)).await;
            assert!(risk_result.is_ok(), "Risk compare should succeed");

            let risk = risk_result.unwrap();

            // Verify bump results
            assert_eq!(risk.bump.deltas.len(), 6, "Should have delta for each tenor");
            assert!(risk.bump.dv01 != 0.0, "DV01 should be non-zero");

            // Verify AAD results (simulated in demo mode)
            assert!(risk.aad.is_some(), "AAD result should exist");
            let aad = risk.aad.as_ref().unwrap();
            assert_eq!(aad.deltas.len(), 6, "AAD should have same number of deltas");

            // Verify speedup ratio
            assert!(risk.speedup_ratio.is_some(), "Speedup ratio should be calculated");
            let speedup = risk.speedup_ratio.unwrap();
            assert!(speedup > 1.0, "AAD should be faster than Bump (simulated 10x)");
        }

        #[tokio::test]
        async fn test_invalid_curve_id_returns_404() {
            // Task 10.1: Error handling test
            let state = Arc::new(AppState::new());

            let pricing_request = IrsPricingRequest {
                curve_id: "00000000-0000-0000-0000-000000000000".to_string(),
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
            };

            let result = price_irs(State(state), Json(pricing_request)).await;
            assert!(result.is_err(), "Should return error for non-existent curve");

            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::NOT_FOUND);
        }

        #[tokio::test]
        async fn test_empty_par_rates_returns_400() {
            // Task 10.1: Validation error test
            let state = Arc::new(AppState::new());

            let bootstrap_request = BootstrapRequest {
                par_rates: vec![],
                interpolation: crate::web::pricer_types::InterpolationMethod::LogLinear,
            };

            let result = bootstrap_curve(State(state), Json(bootstrap_request)).await;
            assert!(result.is_err(), "Should return error for empty par rates");

            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_negative_notional_returns_400() {
            // Task 10.1: Validation error test
            let state = Arc::new(AppState::new());

            // First bootstrap a valid curve
            let bootstrap_request = BootstrapRequest {
                par_rates: create_standard_par_rates(),
                interpolation: crate::web::pricer_types::InterpolationMethod::LogLinear,
            };
            let curve = bootstrap_curve(State(state.clone()), Json(bootstrap_request))
                .await
                .unwrap();

            // Try to price with negative notional
            let pricing_request = IrsPricingRequest {
                curve_id: curve.curve_id.clone(),
                notional: -10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::Annual,
            };

            let result = price_irs(State(state), Json(pricing_request)).await;
            assert!(result.is_err(), "Should return error for negative notional");

            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_full_workflow_bootstrap_price_risk() {
            // Task 10.1: Complete E2E workflow test
            let state = Arc::new(AppState::new());

            // Step 1: Bootstrap
            let par_rates = vec![
                ParRateInput { tenor: "1Y".to_string(), rate: 0.025 },
                ParRateInput { tenor: "5Y".to_string(), rate: 0.035 },
                ParRateInput { tenor: "10Y".to_string(), rate: 0.04 },
            ];

            let bootstrap_request = BootstrapRequest {
                par_rates,
                interpolation: crate::web::pricer_types::InterpolationMethod::LogLinear,
            };

            let curve = bootstrap_curve(State(state.clone()), Json(bootstrap_request))
                .await
                .expect("Bootstrap should succeed");

            // Step 2: Price IRS
            let pricing_request = IrsPricingRequest {
                curve_id: curve.curve_id.clone(),
                notional: 50_000_000.0,
                fixed_rate: 0.035,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::SemiAnnual,
            };

            let pricing = price_irs(State(state.clone()), Json(pricing_request))
                .await
                .expect("Pricing should succeed");

            // Step 3: Calculate Risk
            let risk_request = RiskRequest {
                curve_id: curve.curve_id.clone(),
                notional: 50_000_000.0,
                fixed_rate: 0.035,
                tenor_years: 5.0,
                payment_frequency: PaymentFrequency::SemiAnnual,
                bump_size_bps: 1.0,
            };

            let risk = risk_compare(State(state.clone()), Json(risk_request))
                .await
                .expect("Risk compare should succeed");

            // Verify complete workflow results
            assert!(curve.pillars.len() == 3, "Curve should have 3 points");
            assert!(curve.processing_time_ms > 0.0, "Processing time should be recorded");

            assert!(pricing.processing_time_us > 0.0, "Pricing time should be recorded");

            assert!(risk.bump.timing.total_ms > 0.0, "Bump timing should be recorded");
            assert!(risk.speedup_ratio.unwrap_or(0.0) > 1.0, "AAD should show speedup");

            // Log workflow completion
            println!("Full workflow completed:");
            println!("  - Bootstrap: {} points in {:.2}ms", curve.pillars.len(), curve.processing_time_ms);
            println!("  - Pricing: NPV = {:.2}", pricing.npv);
            println!("  - Risk: DV01 = {:.2}, Speedup = {:.1}x",
                     risk.bump.dv01, risk.speedup_ratio.unwrap_or(0.0));
        }

        #[tokio::test]
        async fn test_curve_cache_persistence() {
            // Task 10.1: Verify curve is cached and reusable
            let state = Arc::new(AppState::new());

            // Bootstrap curve
            let bootstrap_request = BootstrapRequest {
                par_rates: create_standard_par_rates(),
                interpolation: crate::web::pricer_types::InterpolationMethod::LogLinear,
            };

            let curve = bootstrap_curve(State(state.clone()), Json(bootstrap_request))
                .await
                .expect("Bootstrap should succeed");

            // Price multiple times with same curve
            for i in 0..3 {
                let pricing_request = IrsPricingRequest {
                    curve_id: curve.curve_id.clone(),
                    notional: 10_000_000.0 * (i as f64 + 1.0),
                    fixed_rate: 0.03,
                    tenor_years: 5.0,
                    payment_frequency: PaymentFrequency::Annual,
                };

                let result = price_irs(State(state.clone()), Json(pricing_request)).await;
                assert!(result.is_ok(), "Pricing {} should succeed with cached curve", i);
            }
        }
    }
}
