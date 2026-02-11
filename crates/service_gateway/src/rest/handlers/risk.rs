//! Risk handlers
//!
//! Thin handlers delegating to `RiskService`.

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

#[cfg(feature = "risk")]
json_handler! {
    /// POST /api/v1/risk/greeks
    pub async fn compute_greeks(GreeksRequest => RiskGreeksResponse) = RiskService::compute_greeks;
}

#[cfg(feature = "risk")]
json_handler! {
    /// POST /api/v1/risk/scenarios
    pub async fn run_scenarios(ScenarioRequest => ScenarioResponse) = RiskService::run_scenarios;
}
