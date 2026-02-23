//! MFM (Markov Functional Model) handlers for the demo GUI.

use axum::Json;

use crate::{
    error::ServerError,
    rest::dto::mfm::{
        BermudanPriceRequest, BermudanPriceResponse, CifEvaluateRequest, CifEvaluateResponse,
        GaussianTreeRequest, GaussianTreeResponse, MfmCalibrateRequest, MfmCalibrateResponse,
        MfmProductDef, TarnPriceRequest, TarnPriceResponse,
    },
    services::MfmService,
};

/// GET /api/mfm/products - List available MFM products.
pub async fn get_mfm_products() -> Json<Vec<MfmProductDef>> { Json(MfmService::get_products()) }

/// POST /api/mfm/calibrate - Calibrate the MFM model.
pub async fn calibrate_mfm(
    Json(request): Json<MfmCalibrateRequest>,
) -> Result<Json<MfmCalibrateResponse>, ServerError> {
    let response = MfmService::calibrate(&request).map_err(ServerError::Pricing)?;
    Ok(Json(response))
}

/// POST /api/mfm/gaussian-tree - Build a Gaussian recombining tree.
pub async fn build_gaussian_tree(
    Json(request): Json<GaussianTreeRequest>,
) -> Result<Json<GaussianTreeResponse>, ServerError> {
    let response = MfmService::build_gaussian_tree(&request).map_err(ServerError::Pricing)?;
    Ok(Json(response))
}

/// POST /api/mfm/cif-evaluate - Evaluate CIF coupon components.
pub async fn evaluate_cif(
    Json(request): Json<CifEvaluateRequest>,
) -> Result<Json<CifEvaluateResponse>, ServerError> {
    let response = MfmService::evaluate_cif(&request).map_err(ServerError::Pricing)?;
    Ok(Json(response))
}

/// POST /api/mfm/bermudan - Price a Bermudan swaption.
pub async fn price_bermudan(
    Json(request): Json<BermudanPriceRequest>,
) -> Result<Json<BermudanPriceResponse>, ServerError> {
    let response = MfmService::price_bermudan(&request).map_err(ServerError::Pricing)?;
    Ok(Json(response))
}

/// POST /api/mfm/tarn - Price a TARN.
pub async fn price_tarn(
    Json(request): Json<TarnPriceRequest>,
) -> Result<Json<TarnPriceResponse>, ServerError> {
    let response = MfmService::price_tarn(&request).map_err(ServerError::Pricing)?;
    Ok(Json(response))
}
