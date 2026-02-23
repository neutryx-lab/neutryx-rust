//! XVA engine handlers for the demo GUI.

use axum::Json;

use crate::{
    error::{ServerError, ValidatedJson},
    rest::dto::xva::{
        XvaBilateralRequest, XvaBilateralResponse, XvaDefaultConfigResponse, XvaSimulationRequest,
        XvaSimulationResponse,
    },
    services::XvaService,
};

/// GET /api/xva/config - Get default XVA configuration and demo counterparties.
pub async fn get_xva_config() -> Result<Json<XvaDefaultConfigResponse>, ServerError> {
    Ok(Json(XvaService::get_default_config()?))
}

/// POST /api/xva/simulate - Run XVA Monte Carlo simulation.
pub async fn run_xva_simulation(
    ValidatedJson(request): ValidatedJson<XvaSimulationRequest>,
) -> Result<Json<XvaSimulationResponse>, ServerError> {
    Ok(Json(XvaService::run_simulation(&request)?))
}

/// POST /api/xva/bilateral - Compute bilateral CVA/DVA/FVA from exposure
/// profiles.
pub async fn compute_xva_bilateral(
    ValidatedJson(request): ValidatedJson<XvaBilateralRequest>,
) -> Result<Json<XvaBilateralResponse>, ServerError> {
    Ok(Json(XvaService::compute_bilateral(&request)?))
}

/// GET /api/xva/export/csv - Export risk indicators as CSV.
pub async fn export_xva_csv(
) -> Result<Json<crate::rest::dto::xva::XvaCsvExportResponse>, ServerError> {
    // Export the first netting set from the last simulation
    Ok(Json(XvaService::export_csv("NS_BANK_A")?))
}
