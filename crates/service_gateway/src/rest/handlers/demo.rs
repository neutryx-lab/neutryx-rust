//! Demo handlers for the demo_gui frontend
//!
//! Thin handlers that delegate business logic to `DemoService`.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Response, StatusCode},
    Json,
};

use crate::{
    error::ServerError,
    rest::dto::demo::{
        AppConfigResponse, AvailableCurvesResponse, Convention, ConventionsResponse,
        CurveIndicesResponse, CurveInstrumentsResponse, DemoGreeksRequest, DemoGreeksResult,
        DemoPricingRequest, DemoPricingResult, EventTypesResponse, EventsResponse, ExpandedTrade,
        ExportFormat, FxVolCalibrateRequest, FxVolPairsResponse, FxVolQuotesResponse,
        HolidaysResponse, IndexConventionsResponse, IndexRatesResponse, InstrumentsResponse,
        IrVolCurrenciesResponse, IrVolQuotesResponse, MarketConfigResponse,
        MarketRateDetailResponse, MarketRatesResponse, RateCashflowsResponse,
        RateIndexDetailResponse, RateIndicesResponse, RateInstrumentResponse, TradeExpandRequest,
        VolcubeCalibrateRequest, VolcubeCalibrateResponse, VolcubeIndicesResponse,
        VolcubeInstrumentsResponse, VolcubeModelsResponse,
    },
    services::{DemoService, VolcubeService},
    state::AppState,
};

// =============================================================================
// Configuration API
// =============================================================================

/// Get application configuration
///
/// GET /api/config
pub async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AppConfigResponse>, ServerError> {
    let response = DemoService::get_config(&state)?;
    Ok(Json(response))
}

// =============================================================================
// Instruments API
// =============================================================================

/// Get available instruments
///
/// GET /api/instruments
pub async fn get_instruments(
    State(state): State<Arc<AppState>>,
) -> Result<Json<InstrumentsResponse>, ServerError> {
    let response = DemoService::get_instruments(&state)?;
    Ok(Json(response))
}

// =============================================================================
// Trade Expansion API
// =============================================================================

/// Expand a trade into cashflows
///
/// POST /api/trade/expand
pub async fn expand_trade(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TradeExpandRequest>,
) -> Result<Json<ExpandedTrade>, ServerError> {
    let response = DemoService::expand_trade(&request, &state)?;
    Ok(Json(response))
}

// =============================================================================
// Pricing API
// =============================================================================

/// Price a trade
///
/// POST /api/pricer/price
pub async fn price_trade(
    State(state): State<Arc<AppState>>,
    Json(request): Json<DemoPricingRequest>,
) -> Result<Json<DemoPricingResult>, ServerError> {
    let response = DemoService::price_trade(&request, &state)?;
    Ok(Json(response))
}

/// Calculate Greeks
///
/// POST /api/pricer/greeks
pub async fn calculate_greeks(
    State(state): State<Arc<AppState>>,
    Json(request): Json<DemoGreeksRequest>,
) -> Result<Json<DemoGreeksResult>, ServerError> {
    let response = DemoService::calculate_greeks(&request, &state)?;
    Ok(Json(response))
}

// =============================================================================
// Market Data API
// =============================================================================

/// Get market rates
///
/// GET /api/market/rates
pub async fn get_market_rates(
    State(state): State<Arc<AppState>>,
) -> Result<Json<MarketRatesResponse>, ServerError> {
    let response = DemoService::get_market_rates(&state)?;
    Ok(Json(response))
}

/// Get market config
///
/// GET /api/market/config
pub async fn get_market_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<MarketConfigResponse>, ServerError> {
    let response = DemoService::get_market_config(&state)?;
    Ok(Json(response))
}

/// Get rate detail
///
/// GET /api/market/rates/:rate_id
pub async fn get_rate_detail(
    State(state): State<Arc<AppState>>,
    Path(rate_id): Path<String>,
) -> Result<Json<MarketRateDetailResponse>, ServerError> {
    let response = DemoService::get_rate_detail(&rate_id, &state)?;
    Ok(Json(response))
}

/// Refresh market rates
///
/// POST /api/market/rates/refresh
pub async fn refresh_market_rates(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, ServerError> {
    DemoService::refresh_market_rates(&state)?;
    Ok(StatusCode::OK)
}

// =============================================================================
// Conventions API
// =============================================================================

/// Get conventions
///
/// GET /api/market/conventions
pub async fn get_conventions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ConventionsResponse>, ServerError> {
    let response = DemoService::get_conventions(&state)?;
    Ok(Json(response))
}

/// Get convention detail
///
/// GET /api/market/conventions/:id
pub async fn get_convention_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Convention>, ServerError> {
    let response = DemoService::get_convention_detail(&id, &state)?;
    Ok(Json(response))
}

// =============================================================================
// IR Volatility API
// =============================================================================

/// Get IR vol currencies
///
/// GET /api/irvol/currencies
pub async fn get_ir_vol_currencies(
    State(state): State<Arc<AppState>>,
) -> Result<Json<IrVolCurrenciesResponse>, ServerError> {
    let response = VolcubeService::get_ir_vol_currencies(&state)?;
    Ok(Json(response))
}

/// Get IR vol quotes for a currency
///
/// GET /api/irvol/quotes/:currency
pub async fn get_ir_vol_quotes(
    State(state): State<Arc<AppState>>,
    Path(currency): Path<String>,
) -> Result<Json<IrVolQuotesResponse>, ServerError> {
    let response = VolcubeService::get_ir_vol_quotes(&currency, &state)?;
    Ok(Json(response))
}

// =============================================================================
// FX Volatility API
// =============================================================================

/// Get FX vol pairs
///
/// GET /api/fxvol/pairs
pub async fn get_fx_vol_pairs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<FxVolPairsResponse>, ServerError> {
    let response = VolcubeService::get_fx_vol_pairs(&state)?;
    Ok(Json(response))
}

/// Get FX vol quotes for a pair
///
/// GET /api/fxvol/quotes/:pair
pub async fn get_fx_vol_quotes(
    State(state): State<Arc<AppState>>,
    Path(pair): Path<String>,
) -> Result<Json<FxVolQuotesResponse>, ServerError> {
    let response = VolcubeService::get_fx_vol_quotes(&pair, &state)?;
    Ok(Json(response))
}

// =============================================================================
// Events API
// =============================================================================

/// Get market events
///
/// GET /api/market/events
pub async fn get_events(
    State(state): State<Arc<AppState>>,
) -> Result<Json<EventsResponse>, ServerError> {
    let response = DemoService::get_events(&state)?;
    Ok(Json(response))
}

/// Get event types
///
/// GET /api/market/events/types
pub async fn get_event_types(
    State(state): State<Arc<AppState>>,
) -> Result<Json<EventTypesResponse>, ServerError> {
    let response = DemoService::get_event_types(&state)?;
    Ok(Json(response))
}

/// Get market holidays
///
/// GET /api/market/holidays
pub async fn get_holidays(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HolidaysResponse>, ServerError> {
    let response = DemoService::get_holidays(&state)?;
    Ok(Json(response))
}

// =============================================================================
// Curves API
// =============================================================================

/// Get available curves
///
/// GET /api/curves
pub async fn get_available_curves(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AvailableCurvesResponse>, ServerError> {
    let response = DemoService::get_available_curves(&state)?;
    Ok(Json(response))
}

/// Get curve indices for bootstrapping
///
/// GET /api/curves/indices
pub async fn get_curve_indices(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CurveIndicesResponse>, ServerError> {
    let response = DemoService::get_curve_indices(&state)?;
    Ok(Json(response))
}

/// Get instruments for a specific curve index
///
/// GET /api/curves/instruments/:index
pub async fn get_curve_instruments(
    State(state): State<Arc<AppState>>,
    Path(index): Path<String>,
) -> Result<Json<CurveInstrumentsResponse>, ServerError> {
    let response = DemoService::get_curve_instruments(&index, &state)?;
    Ok(Json(response))
}

// =============================================================================
// Volcube API
// =============================================================================

/// Get volcube indices (swaption currencies)
///
/// GET /api/volcube/indices
pub async fn get_volcube_indices(
    State(state): State<Arc<AppState>>,
) -> Result<Json<VolcubeIndicesResponse>, ServerError> {
    let response = VolcubeService::get_volcube_indices(&state)?;
    Ok(Json(response))
}

/// Get available calibration models
///
/// GET /api/volcube/models
pub async fn get_volcube_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<VolcubeModelsResponse>, ServerError> {
    let response = VolcubeService::get_volcube_models(&state)?;
    Ok(Json(response))
}

/// Get swaption instruments for volcube calibration
///
/// GET /api/volcube/instruments/:currency
pub async fn get_volcube_instruments(
    State(state): State<Arc<AppState>>,
    Path(currency): Path<String>,
) -> Result<Json<VolcubeInstrumentsResponse>, ServerError> {
    let response = VolcubeService::get_volcube_instruments(&currency, &state)?;
    Ok(Json(response))
}

/// Calibrate volcube (swaption vol surface)
///
/// POST /api/volcube/calibrate
pub async fn calibrate_volcube(
    State(state): State<Arc<AppState>>,
    Json(request): Json<VolcubeCalibrateRequest>,
) -> Result<Json<VolcubeCalibrateResponse>, ServerError> {
    let response = VolcubeService::calibrate_volcube(&request, &state)?;
    Ok(Json(response))
}

/// Calibrate FX vol surface
///
/// POST /api/fxvol/calibrate
pub async fn calibrate_fxvol(
    State(state): State<Arc<AppState>>,
    Json(request): Json<FxVolCalibrateRequest>,
) -> Result<Json<VolcubeCalibrateResponse>, ServerError> {
    let response = VolcubeService::calibrate_fxvol(&request, &state)?;
    Ok(Json(response))
}

// =============================================================================
// Export API
// =============================================================================

/// Export market data as CSV
///
/// GET /api/market/export/csv
pub async fn export_market_csv(
    State(state): State<Arc<AppState>>,
) -> Result<Response<Body>, ServerError> {
    let data = DemoService::export_market_data(ExportFormat::Csv, &state)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"market_rates.csv\"",
        )
        .body(Body::from(data))
        .map_err(|e| ServerError::Internal(format!("Failed to build response: {e}")))?;

    Ok(response)
}

/// Export market data as JSON
///
/// GET /api/market/export/json
pub async fn export_market_json(
    State(state): State<Arc<AppState>>,
) -> Result<Response<Body>, ServerError> {
    let data = DemoService::export_market_data(ExportFormat::Json, &state)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"market_rates.json\"",
        )
        .body(Body::from(data))
        .map_err(|e| ServerError::Internal(format!("Failed to build response: {e}")))?;

    Ok(response)
}

// =============================================================================
// Rate Instrument API (market-convention-instrument)
// =============================================================================

/// Get instrument details for a rate
///
/// GET /api/market/rates/:rate_id/instrument
pub async fn get_rate_instrument(
    State(state): State<Arc<AppState>>,
    Path(rate_id): Path<String>,
) -> Result<Json<RateInstrumentResponse>, ServerError> {
    let response = DemoService::get_rate_instrument(&rate_id, &state)?;
    Ok(Json(response))
}

/// Get cashflows for a rate instrument
///
/// GET /api/market/rates/:rate_id/cashflows
pub async fn get_rate_cashflows(
    State(state): State<Arc<AppState>>,
    Path(rate_id): Path<String>,
) -> Result<Json<RateCashflowsResponse>, ServerError> {
    let response = DemoService::get_rate_cashflows(&rate_id, &state)?;
    Ok(Json(response))
}

// =============================================================================
// Rate Index API (market-convention-instrument)
// =============================================================================

/// Get all rate indices
///
/// GET /api/market/indices
pub async fn get_rate_indices(
    State(state): State<Arc<AppState>>,
) -> Result<Json<RateIndicesResponse>, ServerError> {
    let response = DemoService::get_rate_indices(&state)?;
    Ok(Json(response))
}

/// Get rate index detail
///
/// GET /api/market/indices/:code
pub async fn get_rate_index_detail(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> Result<Json<RateIndexDetailResponse>, ServerError> {
    let response = DemoService::get_rate_index_detail(&code, &state)?;
    Ok(Json(response))
}

/// Get rates for a rate index
///
/// GET /api/market/indices/:code/rates
pub async fn get_index_rates(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> Result<Json<IndexRatesResponse>, ServerError> {
    let response = DemoService::get_index_rates(&code, &state)?;
    Ok(Json(response))
}

/// Get conventions for a rate index
///
/// GET /api/market/indices/:code/conventions
pub async fn get_index_conventions(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> Result<Json<IndexConventionsResponse>, ServerError> {
    let response = DemoService::get_index_conventions(&code, &state)?;
    Ok(Json(response))
}

// =============================================================================
// Implied PDF API
// =============================================================================

/// Compute implied probability density via Breeden-Litzenberger
///
/// POST /api/volcube/implied-pdf
pub async fn compute_implied_pdf(
    Json(request): Json<crate::rest::dto::demo::ImpliedPdfRequest>,
) -> Result<Json<crate::rest::dto::demo::ImpliedPdfResponse>, ServerError> {
    let response = VolcubeService::compute_implied_pdf(&request)?;
    Ok(Json(response))
}

/// Compute SABR smile and density from calibrated parameters
///
/// POST /api/volcube/sabr-smile
pub async fn compute_sabr_smile(
    Json(request): Json<crate::rest::dto::demo::SabrSmileRequest>,
) -> Result<Json<crate::rest::dto::demo::SabrSmileResponse>, ServerError> {
    let response = VolcubeService::compute_sabr_smile(&request)?;
    Ok(Json(response))
}
