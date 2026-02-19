//! Demo handlers for the demo_gui frontend.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Response, StatusCode},
    Json,
};

use crate::{
    error::{ServerError, ValidatedJson},
    rest::dto::{
        demo::{
            AppConfigResponse, AvailableCurvesResponse, BondQuotesResponse, Convention,
            ConventionsResponse, CreditQuotesResponse, CurveIndicesResponse,
            CurveInstrumentsResponse, DemoAdvancedGreeksRequest, DemoAdvancedGreeksResult,
            DemoGreeksRequest, DemoGreeksResult, DemoPricingRequest, DemoPricingResult,
            EventTypesResponse, EventsResponse, ExpandedTrade, ExportFormat, FxVolCalibrateRequest,
            FxVolPairsResponse, FxVolQuotesResponse, HolidaysResponse, ImpliedPdfRequest,
            ImpliedPdfResponse, IndexConventionsResponse, IndexRatesResponse, InstrumentsResponse,
            IrVolCurrenciesResponse, IrVolQuotesResponse, MarketConfigResponse,
            MarketRateDetailResponse, MarketRatesResponse, PricerGraphRequest, PricerGraphResponse,
            RateCashflowsResponse, RateIndexDetailResponse, RateIndicesResponse,
            RateInstrumentResponse, ResolveTenorRequest, ResolveTenorResponse, SabrSmileRequest,
            SmileResponse, TradeExpandRequest, VolSmileRequest, VolcubeCalibrateRequest,
            VolcubeCalibrateResponse, VolcubeIndicesResponse, VolcubeInstrumentsResponse,
            VolcubeModelsResponse,
        },
        exotic::{ExoticPricingResponse, ExoticProductDef, ExoticProductRequest},
    },
    services::{DemoService, ExoticService, VolcubeService},
    state::AppState,
};

/// Handler: State -> Json<Response>.
macro_rules! state_handler {
    ($(#[$doc:meta])* $fn:ident, $svc:ident :: $method:ident -> $res:ty) => {
        $(#[$doc])*
        pub async fn $fn(
            State(state): State<Arc<AppState>>,
        ) -> Result<Json<$res>, ServerError> {
            Ok(Json($svc::$method(&state)?))
        }
    };
}

/// Handler: State + Path<String> -> Json<Response>.
macro_rules! state_path_handler {
    ($(#[$doc:meta])* $fn:ident, $svc:ident :: $method:ident -> $res:ty) => {
        $(#[$doc])*
        pub async fn $fn(
            State(state): State<Arc<AppState>>,
            Path(id): Path<String>,
        ) -> Result<Json<$res>, ServerError> {
            Ok(Json($svc::$method(&id, &state)?))
        }
    };
}

/// Handler: State + `ValidatedJson<Request>` -> `Json<Response>`.
macro_rules! state_body_handler {
    ($(#[$doc:meta])* $fn:ident, $svc:ident :: $method:ident($req:ty) -> $res:ty) => {
        $(#[$doc])*
        pub async fn $fn(
            State(state): State<Arc<AppState>>,
            ValidatedJson(request): ValidatedJson<$req>,
        ) -> Result<Json<$res>, ServerError> {
            Ok(Json($svc::$method(&request, &state)?))
        }
    };
}

/// Handler: `ValidatedJson<Request>` -> `Json<Response>` (no state).
macro_rules! body_handler {
    ($(#[$doc:meta])* $fn:ident, $svc:ident :: $method:ident($req:ty) -> $res:ty) => {
        $(#[$doc])*
        pub async fn $fn(
            ValidatedJson(request): ValidatedJson<$req>,
        ) -> Result<Json<$res>, ServerError> {
            Ok(Json($svc::$method(&request)?))
        }
    };
}

state_handler!(/// GET /api/config.
    get_config, DemoService::get_config -> AppConfigResponse);
state_handler!(/// GET /api/instruments.
    get_instruments, DemoService::get_instruments -> InstrumentsResponse);
state_handler!(/// GET /api/market/rates.
    get_market_rates, DemoService::get_market_rates -> MarketRatesResponse);
state_handler!(/// GET /api/market/config.
    get_market_config, DemoService::get_market_config -> MarketConfigResponse);
state_handler!(/// GET /api/market/conventions.
    get_conventions, DemoService::get_conventions -> ConventionsResponse);
state_handler!(/// GET /api/market/events.
    get_events, DemoService::get_events -> EventsResponse);
state_handler!(/// GET /api/market/events/types.
    get_event_types, DemoService::get_event_types -> EventTypesResponse);
state_handler!(/// GET /api/market/holidays.
    get_holidays, DemoService::get_holidays -> HolidaysResponse);
state_handler!(/// GET /api/market/bonds.
    get_bond_quotes, DemoService::get_bond_quotes -> BondQuotesResponse);
state_handler!(/// GET /api/market/credit.
    get_credit_quotes, DemoService::get_credit_quotes -> CreditQuotesResponse);
state_handler!(/// GET /api/curves.
    get_available_curves, DemoService::get_available_curves -> AvailableCurvesResponse);
state_handler!(/// GET /api/curves/indices.
    get_curve_indices, DemoService::get_curve_indices -> CurveIndicesResponse);
state_handler!(/// GET /api/market/indices.
    get_rate_indices, DemoService::get_rate_indices -> RateIndicesResponse);
state_handler!(/// GET /api/irvol/currencies.
    get_ir_vol_currencies, VolcubeService::get_ir_vol_currencies -> IrVolCurrenciesResponse);
state_handler!(/// GET /api/fxvol/pairs.
    get_fx_vol_pairs, VolcubeService::get_fx_vol_pairs -> FxVolPairsResponse);
state_handler!(/// GET /api/volcube/indices.
    get_volcube_indices, VolcubeService::get_volcube_indices -> VolcubeIndicesResponse);
state_handler!(/// GET /api/volcube/models.
    get_volcube_models, VolcubeService::get_volcube_models -> VolcubeModelsResponse);

state_path_handler!(/// GET /api/market/rates/:rate_id.
    get_rate_detail, DemoService::get_rate_detail -> MarketRateDetailResponse);
state_path_handler!(/// GET /api/market/conventions/:id.
    get_convention_detail, DemoService::get_convention_detail -> Convention);
state_path_handler!(/// GET /api/curves/instruments/:index.
    get_curve_instruments, DemoService::get_curve_instruments -> CurveInstrumentsResponse);
state_path_handler!(/// GET /api/market/rates/:rate_id/instrument.
    get_rate_instrument, DemoService::get_rate_instrument -> RateInstrumentResponse);
state_path_handler!(/// GET /api/market/rates/:rate_id/cashflows.
    get_rate_cashflows, DemoService::get_rate_cashflows -> RateCashflowsResponse);
state_path_handler!(/// GET /api/market/indices/:code.
    get_rate_index_detail, DemoService::get_rate_index_detail -> RateIndexDetailResponse);
state_path_handler!(/// GET /api/market/indices/:code/rates.
    get_index_rates, DemoService::get_index_rates -> IndexRatesResponse);
state_path_handler!(/// GET /api/market/indices/:code/conventions.
    get_index_conventions, DemoService::get_index_conventions -> IndexConventionsResponse);
state_path_handler!(/// GET /api/irvol/quotes/:currency.
    get_ir_vol_quotes, VolcubeService::get_ir_vol_quotes -> IrVolQuotesResponse);
state_path_handler!(/// GET /api/fxvol/quotes/:pair.
    get_fx_vol_quotes, VolcubeService::get_fx_vol_quotes -> FxVolQuotesResponse);
state_path_handler!(/// GET /api/volcube/instruments/:currency.
    get_volcube_instruments, VolcubeService::get_volcube_instruments -> VolcubeInstrumentsResponse);

state_body_handler!(/// POST /api/trade/expand.
    expand_trade, DemoService::expand_trade(TradeExpandRequest) -> ExpandedTrade);
state_body_handler!(/// POST /api/pricer/price.
    price_trade, DemoService::price_trade(DemoPricingRequest) -> DemoPricingResult);
state_body_handler!(/// POST /api/pricer/greeks.
    calculate_greeks, DemoService::calculate_greeks(DemoGreeksRequest) -> DemoGreeksResult);
state_body_handler!(/// POST /api/pricer/advanced-greeks.
    calculate_advanced_greeks, DemoService::calculate_advanced_greeks(DemoAdvancedGreeksRequest) -> DemoAdvancedGreeksResult);
state_body_handler!(/// POST /api/volcube/calibrate.
    calibrate_volcube, VolcubeService::calibrate_volcube(VolcubeCalibrateRequest) -> VolcubeCalibrateResponse);
state_body_handler!(/// POST /api/fxvol/calibrate.
    calibrate_fxvol, VolcubeService::calibrate_fxvol(FxVolCalibrateRequest) -> VolcubeCalibrateResponse);

body_handler!(/// POST /api/volcube/implied-pdf.
    compute_implied_pdf, VolcubeService::compute_implied_pdf(ImpliedPdfRequest) -> ImpliedPdfResponse);
body_handler!(/// POST /api/volcube/sabr-smile.
    compute_sabr_smile, VolcubeService::compute_sabr_smile(SabrSmileRequest) -> SmileResponse);
body_handler!(/// POST /api/volcube/model-smile.
    compute_model_smile, VolcubeService::compute_model_smile(VolSmileRequest) -> SmileResponse);
body_handler!(/// POST /api/utils/resolve-tenor.
    resolve_tenor, DemoService::resolve_tenor(ResolveTenorRequest) -> ResolveTenorResponse);
body_handler!(/// POST /api/pricer/graph.
    get_pricer_graph, DemoService::get_pricer_graph(PricerGraphRequest) -> PricerGraphResponse);

/// POST /api/market/rates/refresh.
pub async fn refresh_market_rates(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, ServerError> {
    DemoService::refresh_market_rates(&state)?;
    Ok(StatusCode::OK)
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

/// GET /api/pricer/exotic-products.
pub async fn get_exotic_products() -> Json<Vec<ExoticProductDef>> {
    Json(ExoticService::get_exotic_products())
}

/// POST /api/pricer/price-exotic.
pub async fn price_exotic(
    Json(request): Json<ExoticProductRequest>,
) -> Result<Json<ExoticPricingResponse>, ServerError> {
    let response = ExoticService::price_exotic(&request).map_err(|e| ServerError::Pricing(e))?;
    Ok(Json(response))
}
