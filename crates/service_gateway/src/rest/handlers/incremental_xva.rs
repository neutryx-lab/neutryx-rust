//! Incremental XVA engine handlers for the demo GUI.

use axum::Json;

use crate::{
    error::ServerError,
    rest::dto::incremental_xva::{
        IncrementalXvaDefaultConfig, IncrementalXvaRequest, IncrementalXvaResponse,
    },
    services::IncrementalXvaService,
};

/// GET /api/incremental-xva/config - Get default incremental XVA configuration.
pub async fn get_incremental_xva_config() -> Result<Json<IncrementalXvaDefaultConfig>, ServerError>
{
    Ok(Json(IncrementalXvaService::get_default_config()?))
}

/// POST /api/incremental-xva/run - Run incremental XVA computation.
pub async fn run_incremental_xva(
    Json(request): Json<IncrementalXvaRequest>,
) -> Result<Json<IncrementalXvaResponse>, ServerError> {
    Ok(Json(IncrementalXvaService::run(&request)?))
}
