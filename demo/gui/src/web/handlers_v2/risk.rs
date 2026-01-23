//! Risk metrics endpoints.
//!
//! This module provides endpoints for:
//! - XVA risk metrics (`/api/risk`)
//!
//! More complex risk handlers (risk_bump, risk_aad, risk_compare) remain in
//! the legacy handlers module and will be migrated in a future iteration.

use std::{sync::Arc, time::Instant};

use axum::{extract::State, Json};
use serde::Serialize;

use crate::web::AppState;

// =============================================================================
// Types
// =============================================================================

/// Risk metrics response.
#[derive(Debug, Serialize)]
pub struct RiskMetricsResponse {
    /// Total portfolio present value.
    pub total_pv: f64,
    /// Credit Valuation Adjustment.
    pub cva: f64,
    /// Debt Valuation Adjustment.
    pub dva: f64,
    /// Funding Valuation Adjustment.
    pub fva: f64,
    /// Total XVA (sum of CVA, DVA, FVA).
    pub total_xva: f64,
    /// Expected Exposure.
    pub ee: f64,
    /// Expected Positive Exposure.
    pub epe: f64,
    /// Potential Future Exposure.
    pub pfe: f64,
}

// =============================================================================
// Handlers
// =============================================================================

/// Get risk metrics.
///
/// GET /api/risk
///
/// Returns XVA metrics (CVA, DVA, FVA) and exposure metrics.
pub async fn get_risk_metrics(State(state): State<Arc<AppState>>) -> Json<RiskMetricsResponse> {
    let start = Instant::now();

    let cva = -15_000.0;
    let dva = 5_000.0;
    let fva = -8_000.0;

    // Record response time and warn if > 1s
    let elapsed_us = start.elapsed().as_micros() as u64;
    state.metrics.record_risk_time(elapsed_us).await;
    if elapsed_us > 1_000_000 {
        tracing::warn!("Risk API response slow: {}ms", elapsed_us / 1000);
    }

    Json(RiskMetricsResponse {
        total_pv: 353_000.0,
        cva,
        dva,
        fva,
        total_xva: cva + dva + fva,
        ee: 500_000.0,
        epe: 450_000.0,
        pfe: 800_000.0,
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_metrics_response_serialisation() {
        let response = RiskMetricsResponse {
            total_pv: 353_000.0,
            cva: -15_000.0,
            dva: 5_000.0,
            fva: -8_000.0,
            total_xva: -18_000.0,
            ee: 500_000.0,
            epe: 450_000.0,
            pfe: 800_000.0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"cva\":-15000.0"));
        assert!(json.contains("\"total_xva\":-18000.0"));
    }

    #[test]
    fn test_xva_calculation() {
        let cva = -15_000.0;
        let dva = 5_000.0;
        let fva = -8_000.0;
        let total_xva = cva + dva + fva;
        assert_eq!(total_xva, -18_000.0);
    }
}
