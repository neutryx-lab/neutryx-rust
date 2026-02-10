//! Demo handlers for the demo_gui frontend
//!
//! Thin handlers that delegate business logic to `DemoService` and
//! `VolcubeService`.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Response, StatusCode},
    Json,
};

use crate::{
    error::{AppJson, ServerError},
    rest::dto::demo::{
        AppConfigResponse, AvailableCurvesResponse, Convention, ConventionsResponse,
        CurveIndicesResponse, CurveInstrumentsResponse, DemoGreeksRequest, DemoGreeksResult,
        DemoPricingRequest, DemoPricingResult, EventTypesResponse, EventsResponse, ExpandedTrade,
        ExportFormat, FxVolCalibrateRequest, FxVolPairsResponse, FxVolQuotesResponse,
        HolidaysResponse, ImpliedPdfRequest, ImpliedPdfResponse, IndexConventionsResponse,
        IndexRatesResponse, InstrumentsResponse, IrVolCurrenciesResponse, IrVolQuotesResponse,
        MarketConfigResponse, MarketRateDetailResponse, MarketRatesResponse,
        RateCashflowsResponse, RateIndexDetailResponse, RateIndicesResponse,
        RateInstrumentResponse, SabrSmileRequest, SabrSmileResponse, TradeExpandRequest,
        VolcubeCalibrateRequest, VolcubeCalibrateResponse, VolcubeIndicesResponse,
        VolcubeInstrumentsResponse, VolcubeModelsResponse,
    },
    services::{DemoService, VolcubeService},
    state::AppState,
};

// =============================================================================
// Configuration & Instruments
// =============================================================================

get_handler! {
    /// GET /api/config
    pub async fn get_config(=> AppConfigResponse) = DemoService::get_config;
}

get_handler! {
    /// GET /api/instruments
    pub async fn get_instruments(=> InstrumentsResponse) = DemoService::get_instruments;
}

// =============================================================================
// Trade Expansion & Pricing
// =============================================================================

json_handler! {
    /// POST /api/trade/expand
    pub async fn expand_trade(TradeExpandRequest => ExpandedTrade) = DemoService::expand_trade;
}

json_handler! {
    /// POST /api/pricer/price
    pub async fn price_trade(DemoPricingRequest => DemoPricingResult) = DemoService::price_trade;
}

json_handler! {
    /// POST /api/pricer/greeks
    pub async fn calculate_greeks(DemoGreeksRequest => DemoGreeksResult) = DemoService::calculate_greeks;
}

// =============================================================================
// Market Data
// =============================================================================

get_handler! {
    /// GET /api/market/rates
    pub async fn get_market_rates(=> MarketRatesResponse) = DemoService::get_market_rates;
}

get_handler! {
    /// GET /api/market/config
    pub async fn get_market_config(=> MarketConfigResponse) = DemoService::get_market_config;
}

path_handler! {
    /// GET /api/market/rates/:rate_id
    pub async fn get_rate_detail(=> MarketRateDetailResponse) = DemoService::get_rate_detail;
}

/// POST /api/market/rates/refresh
pub async fn refresh_market_rates(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, ServerError> {
    DemoService::refresh_market_rates(&state)?;
    Ok(StatusCode::OK)
}

// =============================================================================
// Conventions
// =============================================================================

get_handler! {
    /// GET /api/market/conventions
    pub async fn get_conventions(=> ConventionsResponse) = DemoService::get_conventions;
}

path_handler! {
    /// GET /api/market/conventions/:id
    pub async fn get_convention_detail(=> Convention) = DemoService::get_convention_detail;
}

// =============================================================================
// IR & FX Volatility
// =============================================================================

get_handler! {
    /// GET /api/irvol/currencies
    pub async fn get_ir_vol_currencies(=> IrVolCurrenciesResponse) = VolcubeService::get_ir_vol_currencies;
}

path_handler! {
    /// GET /api/irvol/quotes/:currency
    pub async fn get_ir_vol_quotes(=> IrVolQuotesResponse) = VolcubeService::get_ir_vol_quotes;
}

get_handler! {
    /// GET /api/fxvol/pairs
    pub async fn get_fx_vol_pairs(=> FxVolPairsResponse) = VolcubeService::get_fx_vol_pairs;
}

path_handler! {
    /// GET /api/fxvol/quotes/:pair
    pub async fn get_fx_vol_quotes(=> FxVolQuotesResponse) = VolcubeService::get_fx_vol_quotes;
}

// =============================================================================
// Events & Holidays
// =============================================================================

get_handler! {
    /// GET /api/market/events
    pub async fn get_events(=> EventsResponse) = DemoService::get_events;
}

get_handler! {
    /// GET /api/market/events/types
    pub async fn get_event_types(=> EventTypesResponse) = DemoService::get_event_types;
}

get_handler! {
    /// GET /api/market/holidays
    pub async fn get_holidays(=> HolidaysResponse) = DemoService::get_holidays;
}

// =============================================================================
// Curves
// =============================================================================

get_handler! {
    /// GET /api/curves
    pub async fn get_available_curves(=> AvailableCurvesResponse) = DemoService::get_available_curves;
}

get_handler! {
    /// GET /api/curves/indices
    pub async fn get_curve_indices(=> CurveIndicesResponse) = DemoService::get_curve_indices;
}

path_handler! {
    /// GET /api/curves/instruments/:index
    pub async fn get_curve_instruments(=> CurveInstrumentsResponse) = DemoService::get_curve_instruments;
}

// =============================================================================
// Volcube
// =============================================================================

get_handler! {
    /// GET /api/volcube/indices
    pub async fn get_volcube_indices(=> VolcubeIndicesResponse) = VolcubeService::get_volcube_indices;
}

get_handler! {
    /// GET /api/volcube/models
    pub async fn get_volcube_models(=> VolcubeModelsResponse) = VolcubeService::get_volcube_models;
}

path_handler! {
    /// GET /api/volcube/instruments/:currency
    pub async fn get_volcube_instruments(=> VolcubeInstrumentsResponse) = VolcubeService::get_volcube_instruments;
}

json_handler! {
    /// POST /api/volcube/calibrate
    pub async fn calibrate_volcube(VolcubeCalibrateRequest => VolcubeCalibrateResponse) = VolcubeService::calibrate_volcube;
}

json_handler! {
    /// POST /api/fxvol/calibrate
    pub async fn calibrate_fxvol(FxVolCalibrateRequest => VolcubeCalibrateResponse) = VolcubeService::calibrate_fxvol;
}

// =============================================================================
// Export
// =============================================================================

/// GET /api/market/export/csv
pub async fn export_market_csv(
    State(state): State<Arc<AppState>>,
) -> Result<Response<Body>, ServerError> {
    let data = DemoService::export_market_data(ExportFormat::Csv, &state)?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"market_rates.csv\"",
        )
        .body(Body::from(data))
        .map_err(|e| ServerError::Internal(format!("Failed to build response: {e}")))
}

/// GET /api/market/export/json
pub async fn export_market_json(
    State(state): State<Arc<AppState>>,
) -> Result<Response<Body>, ServerError> {
    let data = DemoService::export_market_data(ExportFormat::Json, &state)?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"market_rates.json\"",
        )
        .body(Body::from(data))
        .map_err(|e| ServerError::Internal(format!("Failed to build response: {e}")))
}

// =============================================================================
// Rate Instrument & Cashflows
// =============================================================================

path_handler! {
    /// GET /api/market/rates/:rate_id/instrument
    pub async fn get_rate_instrument(=> RateInstrumentResponse) = DemoService::get_rate_instrument;
}

path_handler! {
    /// GET /api/market/rates/:rate_id/cashflows
    pub async fn get_rate_cashflows(=> RateCashflowsResponse) = DemoService::get_rate_cashflows;
}

// =============================================================================
// Rate Index
// =============================================================================

get_handler! {
    /// GET /api/market/indices
    pub async fn get_rate_indices(=> RateIndicesResponse) = DemoService::get_rate_indices;
}

path_handler! {
    /// GET /api/market/indices/:code
    pub async fn get_rate_index_detail(=> RateIndexDetailResponse) = DemoService::get_rate_index_detail;
}

path_handler! {
    /// GET /api/market/indices/:code/rates
    pub async fn get_index_rates(=> IndexRatesResponse) = DemoService::get_index_rates;
}

path_handler! {
    /// GET /api/market/indices/:code/conventions
    pub async fn get_index_conventions(=> IndexConventionsResponse) = DemoService::get_index_conventions;
}

// =============================================================================
// Implied PDF & SABR
// =============================================================================

stateless_json_handler! {
    /// POST /api/volcube/implied-pdf
    pub async fn compute_implied_pdf(ImpliedPdfRequest => ImpliedPdfResponse) = VolcubeService::compute_implied_pdf;
}

stateless_json_handler! {
    /// POST /api/volcube/sabr-smile
    pub async fn compute_sabr_smile(SabrSmileRequest => SabrSmileResponse) = VolcubeService::compute_sabr_smile;
}
