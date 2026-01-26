//! Scenario-related handlers.
//!
//! This module provides scenario analysis handlers:
//! - `/api/scenario` - Run a scenario analysis

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::PricingErrorResponse;
use crate::web::AppState;

// =============================================================================
// Types
// =============================================================================

/// Request for scenario analysis
#[derive(Debug, Deserialize)]
pub struct ScenarioRequest {
    /// Rate shock in basis points.
    pub rate_shock: f64,
    /// Volatility shift in percentage.
    pub vol_shift: f64,
    /// Spread shock in basis points.
    pub spread_shock: f64,
    /// Correlation shift in percentage.
    pub corr_shift: f64,
}

/// Response for scenario analysis
#[derive(Debug, Serialize)]
pub struct ScenarioResponse {
    /// Stressed present value.
    pub stressed_pv: f64,
    /// Change in present value.
    pub pv_change: f64,
    /// Stressed CVA.
    pub stressed_cva: f64,
    /// Stressed DVA.
    pub stressed_dva: f64,
    /// Stressed FVA.
    pub stressed_fva: f64,
    /// Unique scenario identifier.
    pub scenario_id: String,
}

// =============================================================================
// Handlers
// =============================================================================

/// Run scenario analysis.
///
/// POST /api/scenario
///
/// Performs scenario analysis with given market shocks.
/// Returns stressed portfolio values and XVA impacts.
pub async fn run_scenario(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<ScenarioRequest>,
) -> Result<Json<ScenarioResponse>, (StatusCode, Json<PricingErrorResponse>)> {
    // Base portfolio values (from sample data)
    let base_pv = 353_000.0;
    let base_cva = -45_000.0;
    let base_dva = 12_000.0;
    let base_fva = -8_000.0;

    // Apply shocks to calculate stressed values
    let rate_impact = base_pv * (request.rate_shock / 10000.0) * 4.5;
    let vol_impact = base_pv * (request.vol_shift / 100.0) * 0.15;
    let spread_impact_cva = base_cva * (request.spread_shock / 100.0) * 0.8;
    let spread_impact_dva = base_dva * (request.spread_shock / 100.0) * 0.6;
    let corr_impact = base_pv * (request.corr_shift / 100.0) * 0.05;

    // Calculate stressed values
    let stressed_pv = base_pv - rate_impact - vol_impact - corr_impact;
    let pv_change = stressed_pv - base_pv;
    let stressed_cva = base_cva + spread_impact_cva;
    let stressed_dva = base_dva + spread_impact_dva;
    let stressed_fva = base_fva * (1.0 + request.rate_shock / 200.0);

    let scenario_id = Uuid::new_v4().to_string();

    Ok(Json(ScenarioResponse {
        stressed_pv,
        pv_change,
        stressed_cva,
        stressed_dva,
        stressed_fva,
        scenario_id,
    }))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_response_serialisation() {
        let response = ScenarioResponse {
            stressed_pv: 350_000.0,
            pv_change: -3_000.0,
            stressed_cva: -46_000.0,
            stressed_dva: 12_500.0,
            stressed_fva: -8_100.0,
            scenario_id: "test-id".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"stressed_pv\":350000.0"));
        assert!(json.contains("\"pv_change\":-3000.0"));
    }
}
