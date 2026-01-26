//! Exposure metrics endpoint.
//!
//! This module provides endpoints for:
//! - Exposure profile calculation (`/api/exposure`)

use std::{sync::Arc, time::Instant};

use axum::{extract::State, Json};
use serde::Serialize;

use crate::web::AppState;

// =============================================================================
// Types
// =============================================================================

/// Exposure metrics response.
#[derive(Debug, Serialize)]
pub struct ExposureResponse {
    /// Expected Exposure.
    pub ee: f64,
    /// Expected Positive Exposure.
    pub epe: f64,
    /// Expected Negative Exposure.
    pub ene: f64,
    /// Potential Future Exposure.
    pub pfe: f64,
    /// Effective Expected Positive Exposure.
    pub eepe: f64,
    /// Time series of exposure points.
    pub time_series: Vec<ExposurePoint>,
}

/// Single exposure data point.
#[derive(Debug, Serialize)]
pub struct ExposurePoint {
    /// Time in years.
    pub time: f64,
    /// Expected Exposure at this time.
    pub ee: f64,
    /// Expected Positive Exposure at this time.
    pub epe: f64,
    /// Potential Future Exposure at this time.
    pub pfe: f64,
    /// Expected Negative Exposure at this time.
    pub ene: f64,
}

// =============================================================================
// Handlers
// =============================================================================

/// Get exposure metrics.
///
/// GET /api/exposure
///
/// Returns the exposure profile including EE, EPE, ENE, PFE, and EEPE metrics,
/// along with a time series of exposure points.
pub async fn get_exposure(State(state): State<Arc<AppState>>) -> Json<ExposureResponse> {
    let start = Instant::now();

    // Generate sample exposure profile
    let time_series: Vec<ExposurePoint> = (0..=40)
        .map(|i| {
            let t = i as f64 * 0.25;
            let decay = (-0.15 * t).exp();
            let growth = 1.0 - (-0.8 * t).exp();
            let profile = growth * decay;

            ExposurePoint {
                time: t,
                ee: 500_000.0 * profile + 100_000.0,
                epe: 450_000.0 * profile + 80_000.0,
                pfe: 900_000.0 * profile + 150_000.0,
                ene: -200_000.0 * profile - 50_000.0,
            }
        })
        .collect();

    // Summary metrics at peak
    let peak = time_series
        .iter()
        .max_by(|a, b| a.ee.partial_cmp(&b.ee).unwrap())
        .unwrap();

    // Record response time and warn if > 1s
    let elapsed_us = start.elapsed().as_micros() as u64;
    state.metrics.record_exposure_time(elapsed_us).await;
    if elapsed_us > 1_000_000 {
        tracing::warn!("Exposure API response slow: {}ms", elapsed_us / 1000);
    }

    Json(ExposureResponse {
        ee: peak.ee,
        epe: peak.epe,
        ene: peak.ene,
        pfe: peak.pfe,
        eepe: 350_000.0,
        time_series,
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exposure_point_serialisation() {
        let point = ExposurePoint {
            time: 1.0,
            ee: 100_000.0,
            epe: 90_000.0,
            pfe: 150_000.0,
            ene: -50_000.0,
        };
        let json = serde_json::to_string(&point).unwrap();
        assert!(json.contains("\"time\":1.0"));
        assert!(json.contains("\"ee\":100000.0"));
    }

    #[test]
    fn test_exposure_response_serialisation() {
        let response = ExposureResponse {
            ee: 500_000.0,
            epe: 450_000.0,
            ene: -200_000.0,
            pfe: 900_000.0,
            eepe: 350_000.0,
            time_series: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"ee\":500000.0"));
        assert!(json.contains("\"time_series\":[]"));
    }
}
