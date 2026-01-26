//! Risk Engine API handlers.
//!
//! Provides REST API handlers for Greeks and risk calculations using
//! the unified RiskEngine facade from pricer_risk.
//!
//! # Endpoints
//!
//! - `POST /api/risk-engine/greeks` - Compute Greeks for a single trade
//! - `POST /api/risk-engine/portfolio-greeks` - Compute Greeks for a portfolio
//! - `POST /api/risk-engine/scenario-greeks` - Compute scenario-based Greeks
//!
//! # Requirements Coverage
//!
//! - Requirement 9.1: async-compatible interface
//! - Requirement 9.4: job-based execution
//! - Requirement 9.5: handler pattern conformance

// Allow large error types in closures - boxing RiskError would require changes to pricer_risk
#![allow(clippy::result_large_err)]

use std::{sync::Arc, time::Instant};

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use infra_config::{BumpSizes, GreekType, GreeksMethod, RiskConfig, SecondOrderMode};
use pricer_models::{
    analytical::garman_kohlhagen::{GarmanKohlhagen, GarmanKohlhagenParams},
    instruments::FxOptionType,
};
use pricer_risk::{greeks::GreeksResult, RiskEngine, RiskEngineConfig, RiskError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};

use crate::web::{jobs::JobCreatedResponse, AppState};

// =============================================================================
// Risk Engine API Types
// =============================================================================

/// Request to compute Greeks for a single trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreeksRequest {
    pub trade_id: String,
    pub instrument_type: String,
    pub spot: f64,
    pub strike: f64,
    pub expiry: f64,
    pub volatility: f64,
    pub rate: f64,
    #[serde(default)]
    pub dividend_yield: f64,
    #[serde(default)]
    pub risk_config: Option<RiskConfigOverride>,
}

/// Request to compute Greeks for a portfolio of trades.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioGreeksRequest {
    pub trades: Vec<GreeksRequest>,
    #[serde(default)]
    pub risk_config: Option<RiskConfigOverride>,
    #[serde(default)]
    pub async_mode: bool,
}

/// Risk configuration overrides for API requests.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskConfigOverride {
    #[serde(default)]
    pub greeks_method: Option<String>,
    #[serde(default)]
    pub target_greeks: Option<Vec<String>>,
    #[serde(default)]
    pub bump_sizes: Option<BumpSizesOverride>,
    #[serde(default)]
    pub second_order_mode: Option<String>,
}

/// Bump sizes override for API requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BumpSizesOverride {
    #[serde(default)]
    pub rate: Option<f64>,
    #[serde(default)]
    pub vol: Option<f64>,
    #[serde(default)]
    pub spot: Option<f64>,
}

/// Request for scenario-based Greeks calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioGreeksRequest {
    #[serde(flatten)]
    pub base: GreeksRequest,
    pub scenario_name: String,
    pub shifts: MarketShifts,
}

/// Market shifts for scenario analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketShifts {
    #[serde(default)]
    pub spot_shift: f64,
    #[serde(default)]
    pub spot_shift_relative: f64,
    #[serde(default)]
    pub vol_shift: f64,
    #[serde(default)]
    pub rate_shift: f64,
}

/// Response containing computed Greeks for a single trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreeksResponse {
    pub trade_id: String,
    pub pv: f64,
    pub greeks: ComputedGreeksDto,
    pub method: String,
    pub metrics: ExecutionMetricsDto,
}

/// Computed Greeks DTO.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputedGreeksDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamma: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vega: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rho: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vanna: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volga: Option<f64>,
}

/// Execution metrics DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetricsDto {
    pub computation_time_ms: f64,
}

/// Response for portfolio Greeks calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioGreeksResponse {
    pub results: Vec<GreeksResponse>,
    pub failures: Vec<FailedCalculationDto>,
    pub aggregations: AggregatedGreeksDto,
    pub stats: ExecutionStatsDto,
}

/// Failed calculation DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedCalculationDto {
    pub trade_id: String,
    pub error: String,
}

/// Aggregated Greeks DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedGreeksDto {
    pub total_pv: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_delta: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_gamma: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_vega: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_theta: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_rho: Option<f64>,
}

/// Execution statistics DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatsDto {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub elapsed_ms: f64,
    pub used_parallel: bool,
}

/// Response for scenario-based Greeks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioGreeksResponse {
    pub scenario_name: String,
    pub result: GreeksResponse,
}

/// Error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskErrorResponse {
    pub error: String,
    pub message: String,
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
// Task 8.3: Handlers
// =============================================================================

/// POST /api/risk-engine/greeks
///
/// Computes Greeks for a single trade.
///
/// Uses `spawn_blocking` to offload CPU-bound Greeks calculation
/// to the Tokio blocking threadpool.
pub async fn compute_greeks(
    State(state): State<Arc<AppState>>,
    Json(request): Json<GreeksRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    // Build RiskConfig from request
    let risk_config = build_risk_config(request.risk_config.as_ref());

    // Clone request data for the blocking task
    let trade_id = request.trade_id.clone();
    let spot = request.spot;
    let strike = request.strike;
    let expiry = request.expiry;
    let volatility = request.volatility;
    let rate = request.rate;
    let dividend_yield = request.dividend_yield;
    let is_call = request.instrument_type.to_lowercase().contains("call");

    // Spawn blocking task for CPU-bound computation
    let result = tokio::task::spawn_blocking(move || {
        let engine = RiskEngine::new(RiskEngineConfig::new(risk_config));

        engine.compute_greeks(&trade_id, || {
            // Use Garman-Kohlhagen for pricing
            let price = gk_price(
                spot,
                strike,
                expiry,
                volatility,
                rate,
                dividend_yield,
                is_call,
            );

            // Compute Greeks via bump-and-revalue
            let delta = compute_delta_bump(
                spot,
                strike,
                expiry,
                volatility,
                rate,
                dividend_yield,
                is_call,
            );
            let gamma = compute_gamma_bump(
                spot,
                strike,
                expiry,
                volatility,
                rate,
                dividend_yield,
                is_call,
            );
            let vega = compute_vega_bump(
                spot,
                strike,
                expiry,
                volatility,
                rate,
                dividend_yield,
                is_call,
            );
            let theta = compute_theta_bump(
                spot,
                strike,
                expiry,
                volatility,
                rate,
                dividend_yield,
                is_call,
            );
            let rho = compute_rho_bump(
                spot,
                strike,
                expiry,
                volatility,
                rate,
                dividend_yield,
                is_call,
            );

            Ok(GreeksResult::new(price, 0.0)
                .with_delta(delta)
                .with_gamma(gamma)
                .with_vega(vega)
                .with_theta(theta)
                .with_rho(rho))
        })
    })
    .await;

    // Record metrics
    let elapsed_us = start.elapsed().as_micros() as u64;
    state.metrics.record_risk_time(elapsed_us).await;

    match result {
        Ok(Ok(risk_result)) => {
            let response: GreeksResponse = risk_result.into();
            (StatusCode::OK, Json(json!(response)))
        }
        Ok(Err(e)) => {
            warn!("Greeks calculation failed: {}", e);
            let error_response = RiskErrorResponse {
                error: "calculation_failed".to_string(),
                message: e.to_string(),
                details: None,
            };
            (StatusCode::BAD_REQUEST, Json(json!(error_response)))
        }
        Err(e) => {
            warn!("Task join error: {}", e);
            let error_response = RiskErrorResponse {
                error: "internal_error".to_string(),
                message: "Internal computation error".to_string(),
                details: Some(e.to_string()),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(error_response)),
            )
        }
    }
}

/// POST /api/risk-engine/portfolio-greeks
///
/// Computes Greeks for a portfolio of trades.
///
/// For large portfolios (>100 trades) with async_mode=true,
/// creates an async job and returns immediately.
pub async fn compute_portfolio_greeks(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PortfolioGreeksRequest>,
) -> impl IntoResponse {
    let start = Instant::now();
    let trade_count = request.trades.len();

    // Check if we should run as async job
    if request.async_mode && trade_count > 100 {
        return run_portfolio_greeks_async(state, request).await;
    }

    // Build RiskConfig from request
    let risk_config = build_risk_config(request.risk_config.as_ref());

    // Clone trades for the blocking task
    let trades = request.trades.clone();

    // Spawn blocking task for CPU-bound computation
    let result = tokio::task::spawn_blocking(move || {
        let engine = RiskEngine::new(RiskEngineConfig::new(risk_config));

        // Build pricing functions for each trade
        let trade_fns: Vec<(
            &str,
            Box<dyn Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync>,
        )> = trades
            .iter()
            .map(|t| {
                let spot = t.spot;
                let strike = t.strike;
                let expiry = t.expiry;
                let volatility = t.volatility;
                let rate = t.rate;
                let dividend_yield = t.dividend_yield;
                let is_call = t.instrument_type.to_lowercase().contains("call");
                let trade_id = t.trade_id.as_str();

                let pricing_fn: Box<
                    dyn Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync,
                > = Box::new(move || {
                    let price = gk_price(
                        spot,
                        strike,
                        expiry,
                        volatility,
                        rate,
                        dividend_yield,
                        is_call,
                    );

                    let delta = compute_delta_bump(
                        spot,
                        strike,
                        expiry,
                        volatility,
                        rate,
                        dividend_yield,
                        is_call,
                    );
                    let gamma = compute_gamma_bump(
                        spot,
                        strike,
                        expiry,
                        volatility,
                        rate,
                        dividend_yield,
                        is_call,
                    );
                    let vega = compute_vega_bump(
                        spot,
                        strike,
                        expiry,
                        volatility,
                        rate,
                        dividend_yield,
                        is_call,
                    );

                    Ok(GreeksResult::new(price, 0.0)
                        .with_delta(delta)
                        .with_gamma(gamma)
                        .with_vega(vega))
                });

                (trade_id, pricing_fn)
            })
            .collect();

        engine.compute_portfolio_greeks(trade_fns)
    })
    .await;

    // Record metrics
    let elapsed_us = start.elapsed().as_micros() as u64;
    state.metrics.record_risk_time(elapsed_us).await;

    match result {
        Ok(Ok(portfolio_result)) => {
            info!(
                "Portfolio Greeks computed: {} trades, {}ms",
                trade_count,
                elapsed_us / 1000
            );
            let response: PortfolioGreeksResponse = portfolio_result.into();
            (StatusCode::OK, Json(json!(response)))
        }
        Ok(Err(e)) => {
            warn!("Portfolio Greeks calculation failed: {}", e);
            let error_response = RiskErrorResponse {
                error: "calculation_failed".to_string(),
                message: e.to_string(),
                details: None,
            };
            (StatusCode::BAD_REQUEST, Json(json!(error_response)))
        }
        Err(e) => {
            warn!("Task join error: {}", e);
            let error_response = RiskErrorResponse {
                error: "internal_error".to_string(),
                message: "Internal computation error".to_string(),
                details: Some(e.to_string()),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(error_response)),
            )
        }
    }
}

/// POST /api/risk-engine/scenario-greeks
///
/// Computes Greeks under a specified market scenario.
pub async fn compute_scenario_greeks(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ScenarioGreeksRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    // Apply market shifts to base request
    let shifted_spot = apply_spot_shift(request.base.spot, &request.shifts);
    let shifted_vol = request.base.volatility + request.shifts.vol_shift;
    let shifted_rate = request.base.rate + request.shifts.rate_shift;

    // Build RiskConfig
    let risk_config = build_risk_config(request.base.risk_config.as_ref());

    // Clone data for blocking task
    let trade_id = request.base.trade_id.clone();
    let scenario_name = request.scenario_name.clone();
    let strike = request.base.strike;
    let expiry = request.base.expiry;
    let dividend_yield = request.base.dividend_yield;
    let is_call = request.base.instrument_type.to_lowercase().contains("call");

    // Spawn blocking task
    let result = tokio::task::spawn_blocking(move || {
        let engine = RiskEngine::new(RiskEngineConfig::new(risk_config));

        engine.compute_greeks_with_scenario(&trade_id, &scenario_name, || {
            let price = gk_price(
                shifted_spot,
                strike,
                expiry,
                shifted_vol,
                shifted_rate,
                dividend_yield,
                is_call,
            );

            let delta = compute_delta_bump(
                shifted_spot,
                strike,
                expiry,
                shifted_vol,
                shifted_rate,
                dividend_yield,
                is_call,
            );

            Ok(GreeksResult::new(price, 0.0).with_delta(delta))
        })
    })
    .await;

    // Record metrics
    let elapsed_us = start.elapsed().as_micros() as u64;
    state.metrics.record_risk_time(elapsed_us).await;

    match result {
        Ok(Ok(scenario_result)) => {
            let response: ScenarioGreeksResponse = scenario_result.into();
            (StatusCode::OK, Json(json!(response)))
        }
        Ok(Err(e)) => {
            warn!("Scenario Greeks calculation failed: {}", e);
            let error_response = RiskErrorResponse {
                error: "calculation_failed".to_string(),
                message: e.to_string(),
                details: None,
            };
            (StatusCode::BAD_REQUEST, Json(json!(error_response)))
        }
        Err(e) => {
            warn!("Task join error: {}", e);
            let error_response = RiskErrorResponse {
                error: "internal_error".to_string(),
                message: "Internal computation error".to_string(),
                details: Some(e.to_string()),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(error_response)),
            )
        }
    }
}

// =============================================================================
// Task 8.4: Job-Based Execution
// =============================================================================

/// Runs portfolio Greeks calculation as an async job.
///
/// Creates a job, spawns the computation in the background,
/// and returns immediately with the job ID.
async fn run_portfolio_greeks_async(
    state: Arc<AppState>,
    request: PortfolioGreeksRequest,
) -> (StatusCode, Json<serde_json::Value>) {
    // Create job
    let job_id = state
        .job_manager
        .create_job(Some("Portfolio Greeks Calculation"))
        .await;

    info!(
        "Created async job {} for {} trades",
        job_id,
        request.trades.len()
    );

    // Spawn background task
    let job_manager = state.job_manager.clone();
    let trades = request.trades;
    let risk_config = build_risk_config(request.risk_config.as_ref());

    tokio::spawn(async move {
        // Update progress: starting
        job_manager.update_progress(job_id, 10).await;

        // Run computation in blocking task
        let result = tokio::task::spawn_blocking(move || {
            let engine = RiskEngine::new(RiskEngineConfig::new(risk_config));

            let trade_fns: Vec<(
                &str,
                Box<dyn Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync>,
            )> = trades
                .iter()
                .map(|t| {
                    let spot = t.spot;
                    let strike = t.strike;
                    let expiry = t.expiry;
                    let volatility = t.volatility;
                    let rate = t.rate;
                    let dividend_yield = t.dividend_yield;
                    let is_call = t.instrument_type.to_lowercase().contains("call");
                    let trade_id = t.trade_id.as_str();

                    let pricing_fn: Box<
                        dyn Fn() -> Result<GreeksResult<f64>, RiskError> + Send + Sync,
                    > = Box::new(move || {
                        let price = gk_price(
                            spot,
                            strike,
                            expiry,
                            volatility,
                            rate,
                            dividend_yield,
                            is_call,
                        );
                        let delta = compute_delta_bump(
                            spot,
                            strike,
                            expiry,
                            volatility,
                            rate,
                            dividend_yield,
                            is_call,
                        );
                        Ok(GreeksResult::new(price, 0.0).with_delta(delta))
                    });

                    (trade_id, pricing_fn)
                })
                .collect();

            engine.compute_portfolio_greeks(trade_fns)
        })
        .await;

        // Update job status based on result
        match result {
            Ok(Ok(portfolio_result)) => {
                let response: PortfolioGreeksResponse = portfolio_result.into();
                let result_json = serde_json::to_value(response).unwrap_or_default();
                job_manager.complete_job(job_id, result_json).await;
            }
            Ok(Err(e)) => {
                job_manager.fail_job(job_id, e.to_string()).await;
            }
            Err(e) => {
                job_manager
                    .fail_job(job_id, format!("Task error: {}", e))
                    .await;
            }
        }
    });

    // Return job ID immediately
    let response = JobCreatedResponse::new(job_id);
    (StatusCode::ACCEPTED, Json(json!(response)))
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Build RiskConfig from optional override.
fn build_risk_config(override_config: Option<&RiskConfigOverride>) -> RiskConfig {
    let mut config = RiskConfig::default();

    if let Some(overrides) = override_config {
        // Apply method override
        if let Some(method) = &overrides.greeks_method {
            config.greeks_method = match method.to_lowercase().as_str() {
                "aad" => GreeksMethod::Aad,
                _ => GreeksMethod::Bump,
            };
        }

        // Apply target Greeks override
        if let Some(targets) = &overrides.target_greeks {
            config.target_greeks = targets
                .iter()
                .filter_map(|g| match g.to_lowercase().as_str() {
                    "delta" => Some(GreekType::Delta),
                    "gamma" => Some(GreekType::Gamma),
                    "vega" => Some(GreekType::Vega),
                    "theta" => Some(GreekType::Theta),
                    "rho" => Some(GreekType::Rho),
                    "vanna" => Some(GreekType::Vanna),
                    "volga" => Some(GreekType::Volga),
                    _ => None,
                })
                .collect();
        }

        // Apply bump sizes override
        if let Some(bumps) = &overrides.bump_sizes {
            config.bump_sizes = BumpSizes {
                rate: bumps.rate.unwrap_or(0.0001),
                vol: bumps.vol.unwrap_or(0.01),
                spot: bumps.spot.unwrap_or(0.01),
            };
        }

        // Apply second-order mode override
        if let Some(mode) = &overrides.second_order_mode {
            config.second_order_mode = match mode.to_lowercase().as_str() {
                "serial" => SecondOrderMode::Serial,
                _ => SecondOrderMode::Parallel,
            };
        }
    }

    config
}

/// Apply spot shift from scenario.
fn apply_spot_shift(spot: f64, shifts: &MarketShifts) -> f64 {
    let mut shifted = spot + shifts.spot_shift;
    shifted *= 1.0 + shifts.spot_shift_relative;
    shifted
}

// =============================================================================
// Greeks Bump-and-Revalue Helpers
// =============================================================================

const SPOT_BUMP: f64 = 0.01; // 1%
const VOL_BUMP: f64 = 0.01; // 1 vol point
const TIME_BUMP: f64 = 1.0 / 365.0; // 1 day
const RATE_BUMP: f64 = 0.0001; // 1bp

/// Helper function to price using Garman-Kohlhagen model.
fn gk_price(
    spot: f64,
    strike: f64,
    expiry: f64,
    vol: f64,
    rate: f64,
    div: f64,
    is_call: bool,
) -> f64 {
    let params = GarmanKohlhagenParams::new(spot, strike, rate, div, vol, expiry);
    match params {
        Ok(p) => {
            let model = GarmanKohlhagen::new(p);
            let opt_type = if is_call {
                FxOptionType::Call
            } else {
                FxOptionType::Put
            };
            model.price(opt_type)
        }
        Err(_) => 0.0, // Return 0 on invalid params
    }
}

fn compute_delta_bump(
    spot: f64,
    strike: f64,
    expiry: f64,
    vol: f64,
    rate: f64,
    div: f64,
    is_call: bool,
) -> f64 {
    let bump = spot * SPOT_BUMP;
    let up = gk_price(spot + bump, strike, expiry, vol, rate, div, is_call);
    let down = gk_price(spot - bump, strike, expiry, vol, rate, div, is_call);
    (up - down) / (2.0 * bump)
}

fn compute_gamma_bump(
    spot: f64,
    strike: f64,
    expiry: f64,
    vol: f64,
    rate: f64,
    div: f64,
    is_call: bool,
) -> f64 {
    let bump = spot * SPOT_BUMP;
    let base = gk_price(spot, strike, expiry, vol, rate, div, is_call);
    let up = gk_price(spot + bump, strike, expiry, vol, rate, div, is_call);
    let down = gk_price(spot - bump, strike, expiry, vol, rate, div, is_call);
    (up - 2.0 * base + down) / (bump * bump)
}

fn compute_vega_bump(
    spot: f64,
    strike: f64,
    expiry: f64,
    vol: f64,
    rate: f64,
    div: f64,
    is_call: bool,
) -> f64 {
    let up = gk_price(spot, strike, expiry, vol + VOL_BUMP, rate, div, is_call);
    let down = gk_price(spot, strike, expiry, vol - VOL_BUMP, rate, div, is_call);
    (up - down) / (2.0 * VOL_BUMP)
}

fn compute_theta_bump(
    spot: f64,
    strike: f64,
    expiry: f64,
    vol: f64,
    rate: f64,
    div: f64,
    is_call: bool,
) -> f64 {
    if expiry <= TIME_BUMP {
        return 0.0;
    }
    let base = gk_price(spot, strike, expiry, vol, rate, div, is_call);
    let later = gk_price(spot, strike, expiry - TIME_BUMP, vol, rate, div, is_call);
    (later - base) / TIME_BUMP
}

fn compute_rho_bump(
    spot: f64,
    strike: f64,
    expiry: f64,
    vol: f64,
    rate: f64,
    div: f64,
    is_call: bool,
) -> f64 {
    let up = gk_price(spot, strike, expiry, vol, rate + RATE_BUMP, div, is_call);
    let down = gk_price(spot, strike, expiry, vol, rate - RATE_BUMP, div, is_call);
    (up - down) / (2.0 * RATE_BUMP)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_risk_config_default() {
        let config = build_risk_config(None);
        assert_eq!(config.greeks_method, GreeksMethod::Bump);
    }

    #[test]
    fn test_build_risk_config_with_overrides() {
        let overrides = RiskConfigOverride {
            greeks_method: Some("bump".to_string()),
            target_greeks: Some(vec!["delta".to_string(), "vega".to_string()]),
            bump_sizes: Some(BumpSizesOverride {
                rate: Some(0.0002),
                vol: Some(0.02),
                spot: None,
            }),
            second_order_mode: Some("serial".to_string()),
        };

        let config = build_risk_config(Some(&overrides));

        assert_eq!(config.greeks_method, GreeksMethod::Bump);
        assert_eq!(config.target_greeks.len(), 2);
        assert!(config.target_greeks.contains(&GreekType::Delta));
        assert!(config.target_greeks.contains(&GreekType::Vega));
        assert_eq!(config.bump_sizes.rate, 0.0002);
        assert_eq!(config.bump_sizes.vol, 0.02);
        assert_eq!(config.second_order_mode, SecondOrderMode::Serial);
    }

    #[test]
    fn test_apply_spot_shift_absolute() {
        let shifts = MarketShifts {
            spot_shift: 5.0,
            spot_shift_relative: 0.0,
            vol_shift: 0.0,
            rate_shift: 0.0,
        };
        let result = apply_spot_shift(100.0, &shifts);
        assert!((result - 105.0).abs() < 1e-10);
    }

    #[test]
    fn test_apply_spot_shift_relative() {
        let shifts = MarketShifts {
            spot_shift: 0.0,
            spot_shift_relative: 0.10, // +10%
            vol_shift: 0.0,
            rate_shift: 0.0,
        };
        let result = apply_spot_shift(100.0, &shifts);
        assert!((result - 110.0).abs() < 1e-10);
    }

    #[test]
    fn test_apply_spot_shift_combined() {
        let shifts = MarketShifts {
            spot_shift: 5.0,           // +5 absolute
            spot_shift_relative: 0.10, // then +10%
            vol_shift: 0.0,
            rate_shift: 0.0,
        };
        let result = apply_spot_shift(100.0, &shifts);
        // (100 + 5) * 1.10 = 115.5
        assert!((result - 115.5).abs() < 1e-10);
    }

    #[test]
    fn test_compute_delta_bump() {
        let delta = compute_delta_bump(100.0, 100.0, 1.0, 0.20, 0.05, 0.0, true);
        // Delta for ATM call should be around 0.5-0.6
        assert!(delta > 0.4 && delta < 0.8);
    }

    #[test]
    fn test_compute_gamma_bump() {
        let gamma = compute_gamma_bump(100.0, 100.0, 1.0, 0.20, 0.05, 0.0, true);
        // Gamma should be positive
        assert!(gamma > 0.0);
    }

    #[test]
    fn test_compute_vega_bump() {
        let vega = compute_vega_bump(100.0, 100.0, 1.0, 0.20, 0.05, 0.0, true);
        // Vega should be positive
        assert!(vega > 0.0);
    }

    #[test]
    fn test_compute_theta_bump() {
        let theta = compute_theta_bump(100.0, 100.0, 1.0, 0.20, 0.05, 0.0, true);
        // Theta should be negative for long options (time decay)
        assert!(theta < 0.0);
    }

    #[test]
    fn test_compute_rho_bump() {
        let rho = compute_rho_bump(100.0, 100.0, 1.0, 0.20, 0.05, 0.0, true);
        // Rho for call should be positive
        assert!(rho > 0.0);
    }
}
