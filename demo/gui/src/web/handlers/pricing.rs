//! Pricing-related handlers.
//!
//! This module provides pricing handlers:
//! - `/api/price` - Price an instrument
//! - `/api/bootstrap` - Bootstrap a yield curve
//! - `/api/price-irs` - Price an IRS using a bootstrapped curve

use std::{sync::Arc, time::Instant};

use axum::{extract::State, http::StatusCode, Json};
use pricer_models::market::calibration::bootstrapping::{
    BootstrapError, BootstrapInstrument, GenericBootstrapConfig, SequentialBootstrapper,
};
use uuid::Uuid;

use crate::web::{
    pricing_service,
    websocket::{broadcast_bootstrap_complete, broadcast_pricing_complete},
    AppState,
};

use super::types::{
    parse_tenor_to_years, validate_irs_pricing_request, validate_par_rates, BootstrapRequest,
    BootstrapResponse, CachedCurve, DemoMarketData, EquityOptionParams, FxOptionParams,
    GreeksData, InstrumentParams, InstrumentType, IrsBootstrapErrorResponse, IrsParams,
    IrsPricingRequest, IrsPricingResponse, OptionType, PaymentFrequency, PricingErrorResponse,
    PricingRequest, PricingResponse,
};

// =============================================================================
// Helper Functions
// =============================================================================

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
        return Err((
            "spot".to_string(),
            "Spot price must be positive".to_string(),
        ));
    }
    if params.strike <= 0.0 {
        return Err((
            "strike".to_string(),
            "Strike price must be positive".to_string(),
        ));
    }
    if params.expiry_years < 0.0 {
        return Err((
            "expiryYears".to_string(),
            "Expiry must be non-negative".to_string(),
        ));
    }
    if params.volatility <= 0.0 {
        return Err((
            "volatility".to_string(),
            "Volatility must be positive".to_string(),
        ));
    }
    if params.volatility > 5.0 {
        return Err((
            "volatility".to_string(),
            "Volatility seems too high (>500%)".to_string(),
        ));
    }
    Ok(())
}

/// Validate FX option parameters.
fn validate_fx_params(params: &FxOptionParams) -> Result<(), (String, String)> {
    if params.spot <= 0.0 {
        return Err(("spot".to_string(), "Spot rate must be positive".to_string()));
    }
    if params.strike <= 0.0 {
        return Err((
            "strike".to_string(),
            "Strike rate must be positive".to_string(),
        ));
    }
    if params.expiry_years < 0.0 {
        return Err((
            "expiryYears".to_string(),
            "Expiry must be non-negative".to_string(),
        ));
    }
    if params.volatility <= 0.0 {
        return Err((
            "volatility".to_string(),
            "Volatility must be positive".to_string(),
        ));
    }
    if params.volatility > 5.0 {
        return Err((
            "volatility".to_string(),
            "Volatility seems too high (>500%)".to_string(),
        ));
    }
    Ok(())
}

/// Validate IRS parameters.
fn validate_irs_params(params: &IrsParams) -> Result<(), (String, String)> {
    if params.notional <= 0.0 {
        return Err((
            "notional".to_string(),
            "Notional must be positive".to_string(),
        ));
    }
    if params.tenor_years <= 0.0 {
        return Err((
            "tenorYears".to_string(),
            "Tenor must be positive".to_string(),
        ));
    }
    Ok(())
}

/// Convert BootstrapError to HTTP error response.
fn convert_bootstrap_error(error: BootstrapError) -> (StatusCode, Json<IrsBootstrapErrorResponse>) {
    match error {
        BootstrapError::ConvergenceFailure {
            maturity,
            residual: _,
            iterations: _,
        } => {
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
                Json(IrsBootstrapErrorResponse::validation_error(
                    format!("Duplicate tenor: {}", tenor),
                    format!("parRates[{}]", tenor),
                )),
            )
        }
        BootstrapError::InsufficientData { required, provided } => (
            StatusCode::BAD_REQUEST,
            Json(IrsBootstrapErrorResponse::validation_error(
                format!(
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
                Json(IrsBootstrapErrorResponse::calculation_error(format!(
                    "Negative rate {} at tenor {} is not allowed",
                    rate, tenor
                ))),
            )
        }
        BootstrapError::ArbitrageDetected { maturity } => {
            let tenor = format!("{}Y", maturity as i32);
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(IrsBootstrapErrorResponse::calculation_error(format!(
                    "Arbitrage detected at tenor {}: discount factors must be monotonically decreasing",
                    tenor
                ))),
            )
        }
        BootstrapError::InvalidInput(msg) => (
            StatusCode::BAD_REQUEST,
            Json(IrsBootstrapErrorResponse::validation_error(msg, "parRates")),
        ),
        BootstrapError::InvalidMaturity {
            maturity,
            max_maturity,
        } => {
            let tenor = format!("{}Y", maturity as i32);
            (
                StatusCode::BAD_REQUEST,
                Json(IrsBootstrapErrorResponse::validation_error(
                    format!(
                        "Invalid maturity {}: must be between 0 and {} years",
                        tenor, max_maturity
                    ),
                    format!("parRates[{}].tenor", tenor),
                )),
            )
        }
        BootstrapError::Solver(solver_err) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(IrsBootstrapErrorResponse::calculation_error(format!(
                "Solver error: {}",
                solver_err
            ))),
        ),
        BootstrapError::MarketData(mkt_err) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(IrsBootstrapErrorResponse::calculation_error(format!(
                "Market data error: {}",
                mkt_err
            ))),
        ),
    }
}

/// Calculate IRS leg present values using a cached curve.
fn calculate_irs_legs(
    curve: &CachedCurve,
    notional: f64,
    fixed_rate: f64,
    tenor_years: f64,
    frequency: PaymentFrequency,
) -> (f64, f64) {
    let payments_per_year = frequency.periods_per_year() as f64;
    let period_years = 1.0 / payments_per_year;
    let num_periods = (tenor_years * payments_per_year).ceil() as usize;

    let mut fixed_leg_pv = 0.0;
    let mut float_leg_pv = 0.0;

    for i in 1..=num_periods {
        let payment_time = i as f64 * period_years;

        if payment_time > tenor_years + 0.001 {
            break;
        }

        let df = interpolate_discount_factor(curve, payment_time);

        let fixed_cashflow = notional * fixed_rate * period_years;
        fixed_leg_pv += fixed_cashflow * df;

        let prev_time = (i - 1) as f64 * period_years;
        let forward_rate = calculate_forward_rate(curve, prev_time, payment_time);
        let float_cashflow = notional * forward_rate * period_years;
        float_leg_pv += float_cashflow * df;
    }

    (fixed_leg_pv, float_leg_pv)
}

/// Interpolate discount factor from cached curve (log-linear interpolation).
pub fn interpolate_discount_factor(curve: &CachedCurve, t: f64) -> f64 {
    if t <= 0.0 {
        return 1.0;
    }

    let pillars = &curve.pillars;
    let dfs = &curve.discount_factors;

    if pillars.is_empty() {
        return 1.0;
    }

    if t <= pillars[0] {
        let r = -dfs[0].ln() / pillars[0];
        return (-r * t).exp();
    }

    if t >= *pillars.last().unwrap() {
        let n = pillars.len();
        let r = -dfs[n - 1].ln() / pillars[n - 1];
        return (-r * t).exp();
    }

    let mut lo = 0;
    for (i, &p) in pillars.iter().enumerate() {
        if p <= t {
            lo = i;
        }
    }

    let t1 = pillars[lo];
    let t2 = pillars[lo + 1];
    let df1 = dfs[lo];
    let df2 = dfs[lo + 1];

    let w = (t - t1) / (t2 - t1);
    let log_df = df1.ln() * (1.0 - w) + df2.ln() * w;
    log_df.exp()
}

/// Calculate forward rate between two times.
pub fn calculate_forward_rate(curve: &CachedCurve, t1: f64, t2: f64) -> f64 {
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
// Handlers
// =============================================================================

/// Price an instrument and optionally compute Greeks.
///
/// POST /api/price
pub async fn price_instrument(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PricingRequest>,
) -> Result<Json<PricingResponse>, (StatusCode, Json<PricingErrorResponse>)> {
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

            let result = pricing_service::price_equity_option(
                params.spot,
                params.strike,
                params.expiry_years,
                params.rate,
                params.volatility,
                is_call,
                request.compute_greeks,
            )
            .map_err(|e| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(PricingErrorResponse {
                        error_type: "PricingError".to_string(),
                        message: e.to_string(),
                        field: None,
                    }),
                )
            })?;

            let greeks = result.greeks.map(|g| GreeksData {
                delta: g.delta,
                gamma: g.gamma,
                vega: g.vega,
                theta: g.theta,
                rho: g.rho,
            });
            (result.price, greeks)
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

            let result = pricing_service::price_fx_option(
                params.spot,
                params.strike,
                params.expiry_years,
                params.domestic_rate,
                params.foreign_rate,
                params.volatility,
                is_call,
                request.compute_greeks,
            )
            .map_err(|e| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(PricingErrorResponse {
                        error_type: "PricingError".to_string(),
                        message: e.to_string(),
                        field: None,
                    }),
                )
            })?;

            let greeks = result.greeks.map(|g| GreeksData {
                delta: g.delta,
                gamma: g.gamma,
                vega: g.vega,
                theta: g.theta,
                rho: g.rho_domestic,
            });
            (result.price, greeks)
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

            let pv = irs_price(
                params.notional,
                params.fixed_rate,
                params.tenor_years,
                market_rate,
            );
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
    broadcast_pricing_complete(
        &state,
        &calculation_id,
        instrument_type_str,
        pv,
        greeks_json,
    );

    Ok(Json(PricingResponse {
        calculation_id,
        instrument_type: request.instrument_type,
        pv,
        greeks,
        timestamp: chrono::Utc::now().timestamp_millis(),
    }))
}

/// Bootstrap a yield curve from par rates.
///
/// POST /api/bootstrap
pub async fn bootstrap_curve(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BootstrapRequest>,
) -> Result<Json<BootstrapResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    let start = Instant::now();

    if let Err(validation_error) = validate_par_rates(&request.par_rates) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(validation_error.to_error_response()),
        ));
    }

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

    let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::default();
    let bootstrapper = SequentialBootstrapper::new(config);

    let result = match bootstrapper.bootstrap(&instruments) {
        Ok(r) => r,
        Err(bootstrap_error) => {
            return Err(convert_bootstrap_error(bootstrap_error));
        }
    };

    let zero_rates = CachedCurve::calculate_zero_rates(&result.pillars, &result.discount_factors);

    let cached_curve = CachedCurve::new(
        result.pillars.clone(),
        result.discount_factors.clone(),
        zero_rates.clone(),
        request.par_rates.clone(),
    );
    let curve_id = Uuid::new_v4();
    state.curve_cache.add(curve_id, cached_curve);

    let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let tenor_count = result.pillars.len();
    let curve_id_str = curve_id.to_string();

    broadcast_bootstrap_complete(&state, &curve_id_str, tenor_count, processing_time_ms);

    Ok(Json(BootstrapResponse {
        curve_id: curve_id_str,
        pillars: result.pillars,
        discount_factors: result.discount_factors,
        zero_rates,
        processing_time_ms,
    }))
}

/// Price an IRS using a previously bootstrapped curve.
///
/// POST /api/price-irs
pub async fn price_irs(
    State(state): State<Arc<AppState>>,
    Json(request): Json<IrsPricingRequest>,
) -> Result<Json<IrsPricingResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    let start = Instant::now();

    if let Err(validation_error) = validate_irs_pricing_request(&request) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(validation_error.to_error_response()),
        ));
    }

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

    let cached_curve = match state.curve_cache.get(&curve_id) {
        Some(curve) => curve,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(IrsBootstrapErrorResponse::curve_not_found(
                    &request.curve_id,
                )),
            ));
        }
    };

    let (fixed_leg_pv, float_leg_pv) = calculate_irs_legs(
        &cached_curve,
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        request.payment_frequency,
    );

    let npv = float_leg_pv - fixed_leg_pv;
    let processing_time_us = start.elapsed().as_micros() as f64;

    Ok(Json(IrsPricingResponse {
        npv,
        fixed_leg_pv,
        float_leg_pv,
        processing_time_us,
    }))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_equity_params_valid() {
        let params = EquityOptionParams {
            spot: 100.0,
            strike: 105.0,
            expiry_years: 1.0,
            rate: 0.05,
            volatility: 0.2,
            option_type: OptionType::Call,
        };
        assert!(validate_equity_params(&params).is_ok());
    }

    #[test]
    fn test_validate_equity_params_negative_spot() {
        let params = EquityOptionParams {
            spot: -100.0,
            strike: 105.0,
            expiry_years: 1.0,
            rate: 0.05,
            volatility: 0.2,
            option_type: OptionType::Call,
        };
        assert!(validate_equity_params(&params).is_err());
    }

    #[test]
    fn test_irs_price() {
        let pv = irs_price(10_000_000.0, 0.05, 5.0, 0.04);
        assert!(pv > 0.0);
    }
}
