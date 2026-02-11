//! Demo handlers for the demo_gui frontend.

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
        MarketConfigResponse, MarketRateDetailResponse, MarketRatesResponse, RateCashflowsResponse,
        RateIndexDetailResponse, RateIndicesResponse, RateInstrumentResponse, SabrSmileRequest,
        SabrSmileResponse, TradeExpandRequest, VolcubeCalibrateRequest, VolcubeCalibrateResponse,
        VolcubeIndicesResponse, VolcubeInstrumentsResponse, VolcubeModelsResponse,
    },
    services::{DemoService, VolcubeService},
    state::AppState,
};

/// GET /api/config.
pub async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AppConfigResponse>, ServerError> {
    let response = DemoService::get_config(&state)?;
    Ok(Json(response))
}

/// GET /api/instruments.
pub async fn get_instruments(
    State(state): State<Arc<AppState>>,
) -> Result<Json<InstrumentsResponse>, ServerError> {
    let response = DemoService::get_instruments(&state)?;
    Ok(Json(response))
}

/// POST /api/trade/expand.
pub async fn expand_trade(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<TradeExpandRequest>,
) -> Result<Json<ExpandedTrade>, ServerError> {
    let response = DemoService::expand_trade(&request, &state)?;
    Ok(Json(response))
}

/// POST /api/pricer/price.
pub async fn price_trade(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<DemoPricingRequest>,
) -> Result<Json<DemoPricingResult>, ServerError> {
    let response = DemoService::price_trade(&request, &state)?;
    Ok(Json(response))
}

/// POST /api/pricer/greeks.
pub async fn calculate_greeks(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<DemoGreeksRequest>,
) -> Result<Json<DemoGreeksResult>, ServerError> {
    let response = DemoService::calculate_greeks(&request, &state)?;
    Ok(Json(response))
}

/// GET /api/market/rates.
pub async fn get_market_rates(
    State(state): State<Arc<AppState>>,
) -> Result<Json<MarketRatesResponse>, ServerError> {
    let response = DemoService::get_market_rates(&state)?;
    Ok(Json(response))
}

/// GET /api/market/config.
pub async fn get_market_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<MarketConfigResponse>, ServerError> {
    let response = DemoService::get_market_config(&state)?;
    Ok(Json(response))
}

/// GET /api/market/rates/:rate_id.
pub async fn get_rate_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<MarketRateDetailResponse>, ServerError> {
    let response = DemoService::get_rate_detail(&id, &state)?;
    Ok(Json(response))
}

/// POST /api/market/rates/refresh.
pub async fn refresh_market_rates(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, ServerError> {
    DemoService::refresh_market_rates(&state)?;
    Ok(StatusCode::OK)
}

/// GET /api/market/conventions.
pub async fn get_conventions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ConventionsResponse>, ServerError> {
    let response = DemoService::get_conventions(&state)?;
    Ok(Json(response))
}

/// GET /api/market/conventions/:id.
pub async fn get_convention_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Convention>, ServerError> {
    let response = DemoService::get_convention_detail(&id, &state)?;
    Ok(Json(response))
}

/// GET /api/irvol/currencies.
pub async fn get_ir_vol_currencies(
    State(state): State<Arc<AppState>>,
) -> Result<Json<IrVolCurrenciesResponse>, ServerError> {
    let response = VolcubeService::get_ir_vol_currencies(&state)?;
    Ok(Json(response))
}

/// GET /api/irvol/quotes/:currency.
pub async fn get_ir_vol_quotes(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<IrVolQuotesResponse>, ServerError> {
    let response = VolcubeService::get_ir_vol_quotes(&id, &state)?;
    Ok(Json(response))
}

/// GET /api/fxvol/pairs.
pub async fn get_fx_vol_pairs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<FxVolPairsResponse>, ServerError> {
    let response = VolcubeService::get_fx_vol_pairs(&state)?;
    Ok(Json(response))
}

/// GET /api/fxvol/quotes/:pair.
pub async fn get_fx_vol_quotes(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<FxVolQuotesResponse>, ServerError> {
    let response = VolcubeService::get_fx_vol_quotes(&id, &state)?;
    Ok(Json(response))
}

/// GET /api/market/events.
pub async fn get_events(
    State(state): State<Arc<AppState>>,
) -> Result<Json<EventsResponse>, ServerError> {
    let response = DemoService::get_events(&state)?;
    Ok(Json(response))
}

/// GET /api/market/events/types.
pub async fn get_event_types(
    State(state): State<Arc<AppState>>,
) -> Result<Json<EventTypesResponse>, ServerError> {
    let response = DemoService::get_event_types(&state)?;
    Ok(Json(response))
}

/// GET /api/market/holidays.
pub async fn get_holidays(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HolidaysResponse>, ServerError> {
    let response = DemoService::get_holidays(&state)?;
    Ok(Json(response))
}

/// GET /api/curves.
pub async fn get_available_curves(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AvailableCurvesResponse>, ServerError> {
    let response = DemoService::get_available_curves(&state)?;
    Ok(Json(response))
}

/// GET /api/curves/indices.
pub async fn get_curve_indices(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CurveIndicesResponse>, ServerError> {
    let response = DemoService::get_curve_indices(&state)?;
    Ok(Json(response))
}

/// GET /api/curves/instruments/:index.
pub async fn get_curve_instruments(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<CurveInstrumentsResponse>, ServerError> {
    let response = DemoService::get_curve_instruments(&id, &state)?;
    Ok(Json(response))
}

/// GET /api/volcube/indices.
pub async fn get_volcube_indices(
    State(state): State<Arc<AppState>>,
) -> Result<Json<VolcubeIndicesResponse>, ServerError> {
    let response = VolcubeService::get_volcube_indices(&state)?;
    Ok(Json(response))
}

/// GET /api/volcube/models.
pub async fn get_volcube_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<VolcubeModelsResponse>, ServerError> {
    let response = VolcubeService::get_volcube_models(&state)?;
    Ok(Json(response))
}

/// GET /api/volcube/instruments/:currency.
pub async fn get_volcube_instruments(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<VolcubeInstrumentsResponse>, ServerError> {
    let response = VolcubeService::get_volcube_instruments(&id, &state)?;
    Ok(Json(response))
}

/// POST /api/volcube/calibrate.
pub async fn calibrate_volcube(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<VolcubeCalibrateRequest>,
) -> Result<Json<VolcubeCalibrateResponse>, ServerError> {
    let response = VolcubeService::calibrate_volcube(&request, &state)?;
    Ok(Json(response))
}

/// POST /api/fxvol/calibrate.
pub async fn calibrate_fxvol(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<FxVolCalibrateRequest>,
) -> Result<Json<VolcubeCalibrateResponse>, ServerError> {
    let response = VolcubeService::calibrate_fxvol(&request, &state)?;
    Ok(Json(response))
}

/// GET /api/market/export/csv.
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

/// GET /api/market/export/json.
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

/// GET /api/market/rates/:rate_id/instrument.
pub async fn get_rate_instrument(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RateInstrumentResponse>, ServerError> {
    let response = DemoService::get_rate_instrument(&id, &state)?;
    Ok(Json(response))
}

/// GET /api/market/rates/:rate_id/cashflows.
pub async fn get_rate_cashflows(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RateCashflowsResponse>, ServerError> {
    let response = DemoService::get_rate_cashflows(&id, &state)?;
    Ok(Json(response))
}

/// GET /api/market/indices.
pub async fn get_rate_indices(
    State(state): State<Arc<AppState>>,
) -> Result<Json<RateIndicesResponse>, ServerError> {
    let response = DemoService::get_rate_indices(&state)?;
    Ok(Json(response))
}

/// GET /api/market/indices/:code.
pub async fn get_rate_index_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RateIndexDetailResponse>, ServerError> {
    let response = DemoService::get_rate_index_detail(&id, &state)?;
    Ok(Json(response))
}

/// GET /api/market/indices/:code/rates.
pub async fn get_index_rates(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<IndexRatesResponse>, ServerError> {
    let response = DemoService::get_index_rates(&id, &state)?;
    Ok(Json(response))
}

/// GET /api/market/indices/:code/conventions.
pub async fn get_index_conventions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<IndexConventionsResponse>, ServerError> {
    let response = DemoService::get_index_conventions(&id, &state)?;
    Ok(Json(response))
}

/// POST /api/volcube/implied-pdf.
pub async fn compute_implied_pdf(
    AppJson(request): AppJson<ImpliedPdfRequest>,
) -> Result<Json<ImpliedPdfResponse>, ServerError> {
    let response = VolcubeService::compute_implied_pdf(&request)?;
    Ok(Json(response))
}

/// POST /api/volcube/sabr-smile.
pub async fn compute_sabr_smile(
    AppJson(request): AppJson<SabrSmileRequest>,
) -> Result<Json<SabrSmileResponse>, ServerError> {
    let response = VolcubeService::compute_sabr_smile(&request)?;
    Ok(Json(response))
}
