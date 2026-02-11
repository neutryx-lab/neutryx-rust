//! Curve handlers
//!
//! Thin handlers delegating to `CurveService`.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::{
    error::{AppJson, ServerError},
    rest::dto::{
        CurveBuildRequest, CurveBuildResponse, DiscountFactorRequest, DiscountFactorResponse,
        ForwardRateRequest, ForwardRateResponse, ForwardSwapRateRequest, ForwardSwapRateResponse,
    },
    services::CurveService,
    state::AppState,
};

json_handler! {
    /// POST /api/curves/build
    pub async fn build_curve(CurveBuildRequest => CurveBuildResponse) = CurveService::build_curve;
}

json_handler! {
    /// POST /api/curves/discount-factor
    pub async fn get_discount_factor(DiscountFactorRequest => DiscountFactorResponse) = CurveService::get_discount_factor;
}

json_handler! {
    /// POST /api/curves/forward-rate
    pub async fn get_forward_rate(ForwardRateRequest => ForwardRateResponse) = CurveService::get_forward_rate;
}

json_handler! {
    /// POST /api/curves/forward-swap-rates
    pub async fn get_forward_swap_rates(ForwardSwapRateRequest => ForwardSwapRateResponse) = CurveService::compute_forward_swap_rates;
}
