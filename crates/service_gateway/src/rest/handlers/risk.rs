//! Risk handlers.

#[cfg(feature = "risk")]
use std::sync::Arc;

#[cfg(feature = "risk")]
use axum::{extract::State, Json};

#[cfg(feature = "risk")]
use crate::{
    error::{AppJson, ServerError},
    rest::dto::{GreeksRequest, RiskGreeksResponse, ScenarioRequest, ScenarioResponse},
    services::RiskService,
    state::AppState,
};

/// POST /api/v1/risk/greeks.
#[cfg(feature = "risk")]
pub async fn compute_greeks(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<GreeksRequest>,
) -> Result<Json<RiskGreeksResponse>, ServerError> {
    let response = RiskService::compute_greeks(&request, &state)?;
    Ok(Json(response))
}

/// POST /api/v1/risk/scenarios.
#[cfg(feature = "risk")]
pub async fn run_scenarios(
    State(state): State<Arc<AppState>>,
    AppJson(request): AppJson<ScenarioRequest>,
) -> Result<Json<ScenarioResponse>, ServerError> {
    let response = RiskService::run_scenarios(&request, &state)?;
    Ok(Json(response))
}
