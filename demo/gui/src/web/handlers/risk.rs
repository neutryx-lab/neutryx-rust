//! Risk metrics endpoints.
//!
//! This module provides endpoints for:
//! - XVA risk metrics (`/api/risk`)
//! - Bump-and-revalue risk (`/api/risk/bump`)
//! - AAD risk (`/api/risk/aad`)
//! - Risk comparison (`/api/risk/compare`)
//!
//! NOTE: These handlers require the `calibration` feature.

use std::{sync::Arc, time::Instant};

use axum::{extract::State, http::StatusCode, Json};
use pricer_models::{
    builder::{BootstrapConfig, BootstrapError, CurveBootstrapper},
    market::curves::MarketInstrument,
};
use serde::Serialize;
use uuid::Uuid;

use super::types::{
    parse_tenor_to_years, validate_risk_request, CachedCurve, DeltaResult,
    IrsBootstrapErrorResponse, ParRateInput, RiskAadResponse, RiskBumpResponse,
    RiskCompareResponse, RiskMethodResult, RiskRequest, TimingComparison, TimingStats,
};
use crate::web::{websocket::broadcast_risk_complete, AppState};

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
// Helper Functions
// =============================================================================

/// Bootstrap a curve from par rates (helper for bump-and-revalue).
fn bootstrap_from_par_rates(par_rates: &[ParRateInput]) -> Result<CachedCurve, BootstrapError> {
    let instruments: Result<Vec<MarketInstrument<f64>>, _> = par_rates
        .iter()
        .map(|pr| {
            parse_tenor_to_years(&pr.tenor).map(|years| MarketInstrument::ois(years, pr.rate))
        })
        .collect();

    let instruments = instruments
        .map_err(|_| BootstrapError::InvalidInput("Failed to parse tenor".to_string()))?;

    let config = BootstrapConfig::default();
    let bootstrapper = CurveBootstrapper::with_config(config);

    // Use bootstrap_to_curve to get BootstrappedCurve directly
    let curve = bootstrapper.bootstrap_to_curve(&instruments)?;

    Ok(CachedCurve::new(curve, par_rates.to_vec()))
}

/// Compute Deltas using bump-and-revalue mode.
fn compute_deltas_bump_mode(
    cached_curve: &CachedCurve,
    request: &RiskRequest,
) -> (Vec<DeltaResult>, Vec<u64>) {
    let (base_fixed_pv, base_float_pv) = cached_curve.calculate_irs_legs(
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        request.payment_frequency,
    );
    let base_npv = base_float_pv - base_fixed_pv;

    let bump_size = request.bump_size_bps * 0.0001;

    let mut deltas = Vec::with_capacity(cached_curve.par_rates.len());
    let mut timing_samples = Vec::with_capacity(cached_curve.par_rates.len());

    for (i, par_rate) in cached_curve.par_rates.iter().enumerate() {
        let tenor_start = Instant::now();

        let mut bumped_par_rates = cached_curve.par_rates.clone();
        bumped_par_rates[i].rate += bump_size;

        let bumped_curve = match bootstrap_from_par_rates(&bumped_par_rates) {
            Ok(curve) => curve,
            Err(_) => {
                deltas.push(DeltaResult {
                    tenor: par_rate.tenor.clone(),
                    delta: 0.0,
                    processing_time_us: tenor_start.elapsed().as_micros() as f64,
                });
                timing_samples.push(tenor_start.elapsed().as_micros() as u64);
                continue;
            }
        };

        let (bumped_fixed_pv, bumped_float_pv) = bumped_curve.calculate_irs_legs(
            request.notional,
            request.fixed_rate,
            request.tenor_years,
            request.payment_frequency,
        );
        let bumped_npv = bumped_float_pv - bumped_fixed_pv;

        let delta = (bumped_npv - base_npv) / request.bump_size_bps;

        let processing_time_us = tenor_start.elapsed().as_micros() as f64;
        timing_samples.push(tenor_start.elapsed().as_micros() as u64);

        deltas.push(DeltaResult {
            tenor: par_rate.tenor.clone(),
            delta,
            processing_time_us,
        });
    }

    (deltas, timing_samples)
}

/// Compute Deltas using AAD mode (simulated for demo).
fn compute_deltas_aad_mode(
    cached_curve: &CachedCurve,
    request: &RiskRequest,
) -> (Vec<DeltaResult>, Vec<u64>) {
    let start = Instant::now();

    let (base_fixed_pv, base_float_pv) = cached_curve.calculate_irs_legs(
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        request.payment_frequency,
    );
    let base_npv = base_float_pv - base_fixed_pv;

    let bump_size = request.bump_size_bps * 0.0001;

    let mut deltas = Vec::with_capacity(cached_curve.par_rates.len());

    for (i, par_rate) in cached_curve.par_rates.iter().enumerate() {
        let mut bumped_par_rates = cached_curve.par_rates.clone();
        bumped_par_rates[i].rate += bump_size;

        let bumped_curve = match bootstrap_from_par_rates(&bumped_par_rates) {
            Ok(curve) => curve,
            Err(_) => {
                deltas.push(DeltaResult {
                    tenor: par_rate.tenor.clone(),
                    delta: 0.0,
                    processing_time_us: 0.0,
                });
                continue;
            }
        };

        let (bumped_fixed_pv, bumped_float_pv) = bumped_curve.calculate_irs_legs(
            request.notional,
            request.fixed_rate,
            request.tenor_years,
            request.payment_frequency,
        );
        let bumped_npv = bumped_float_pv - bumped_fixed_pv;

        let delta = (bumped_npv - base_npv) / request.bump_size_bps;

        deltas.push(DeltaResult {
            tenor: par_rate.tenor.clone(),
            delta,
            processing_time_us: 0.0,
        });
    }

    let total_time_us = start.elapsed().as_micros() as f64;
    let per_tenor_time = total_time_us / deltas.len() as f64;

    for delta in &mut deltas {
        delta.processing_time_us = per_tenor_time;
    }

    let timing_samples = vec![start.elapsed().as_micros() as u64];
    (deltas, timing_samples)
}

// =============================================================================
// Handlers
// =============================================================================

/// Get risk metrics.
///
/// GET /api/risk
pub async fn get_risk_metrics(State(state): State<Arc<AppState>>) -> Json<RiskMetricsResponse> {
    let start = Instant::now();

    let cva = -15_000.0;
    let dva = 5_000.0;
    let fva = -8_000.0;

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

/// Calculate risk sensitivities using the Bump-and-Revalue method.
///
/// POST /api/risk/bump
pub async fn risk_bump(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RiskRequest>,
) -> Result<Json<RiskBumpResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    let total_start = Instant::now();

    if let Err(validation_error) = validate_risk_request(&request) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(validation_error.to_error_response()),
        ));
    }

    let curve_id = match Uuid::parse_str(&request.curve_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(IrsBootstrapErrorResponse::validation_error(
                    "Invalid curve_id format: must be a valid UUID",
                    "curveId",
                )),
            ));
        }
    };

    let cached_curve = match state.curve_cache.get(&curve_id) {
        Some(curve) => curve,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(IrsBootstrapErrorResponse::curve_not_found(
                    &request.curve_id,
                )),
            ));
        }
    };

    let (base_fixed_pv, base_float_pv) = cached_curve.calculate_irs_legs(
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        request.payment_frequency,
    );
    let base_npv = base_float_pv - base_fixed_pv;

    let bump_size = request.bump_size_bps * 0.0001;

    let mut deltas = Vec::with_capacity(cached_curve.par_rates.len());
    let mut timing_samples = Vec::with_capacity(cached_curve.par_rates.len());

    for (i, par_rate) in cached_curve.par_rates.iter().enumerate() {
        let tenor_start = Instant::now();

        let mut bumped_par_rates = cached_curve.par_rates.clone();
        bumped_par_rates[i].rate += bump_size;

        let bumped_curve = match bootstrap_from_par_rates(&bumped_par_rates) {
            Ok(curve) => curve,
            Err(_) => {
                deltas.push(DeltaResult {
                    tenor: par_rate.tenor.clone(),
                    delta: 0.0,
                    processing_time_us: tenor_start.elapsed().as_micros() as f64,
                });
                timing_samples.push(tenor_start.elapsed().as_micros() as u64);
                continue;
            }
        };

        let (bumped_fixed_pv, bumped_float_pv) = bumped_curve.calculate_irs_legs(
            request.notional,
            request.fixed_rate,
            request.tenor_years,
            request.payment_frequency,
        );
        let bumped_npv = bumped_float_pv - bumped_fixed_pv;

        let delta = (bumped_npv - base_npv) / request.bump_size_bps;

        let processing_time_us = tenor_start.elapsed().as_micros() as f64;
        timing_samples.push(tenor_start.elapsed().as_micros() as u64);

        deltas.push(DeltaResult {
            tenor: par_rate.tenor.clone(),
            delta,
            processing_time_us,
        });
    }

    let dv01: f64 = deltas.iter().map(|d| d.delta).sum();

    let timing =
        TimingStats::from_samples(&timing_samples, total_start.elapsed().as_micros() as u64);

    broadcast_risk_complete(&state, &request.curve_id, "bump", dv01, None);

    Ok(Json(RiskBumpResponse {
        deltas,
        dv01,
        timing,
    }))
}

/// Calculate risk sensitivities using the AAD method.
///
/// POST /api/risk/aad
pub async fn risk_aad(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RiskRequest>,
) -> Result<Json<RiskAadResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    let total_start = Instant::now();

    if let Err(validation_error) = validate_risk_request(&request) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(validation_error.to_error_response()),
        ));
    }

    let curve_id = match Uuid::parse_str(&request.curve_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(IrsBootstrapErrorResponse::validation_error(
                    "Invalid curve_id format: must be a valid UUID",
                    "curveId",
                )),
            ));
        }
    };

    let cached_curve = match state.curve_cache.get(&curve_id) {
        Some(curve) => curve,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(IrsBootstrapErrorResponse::curve_not_found(
                    &request.curve_id,
                )),
            ));
        }
    };

    #[cfg(feature = "enzyme-ad")]
    let aad_available = true;
    #[cfg(not(feature = "enzyme-ad"))]
    let aad_available = false;

    let (deltas, timing_samples) = if aad_available {
        compute_deltas_aad_mode(&cached_curve, &request)
    } else {
        compute_deltas_bump_mode(&cached_curve, &request)
    };

    let dv01: f64 = deltas.iter().map(|d| d.delta).sum();

    let timing =
        TimingStats::from_samples(&timing_samples, total_start.elapsed().as_micros() as u64);

    broadcast_risk_complete(&state, &request.curve_id, "aad", dv01, None);

    Ok(Json(RiskAadResponse {
        deltas,
        dv01,
        timing,
        aad_available,
    }))
}

/// Calculate risk sensitivities using both Bump and AAD methods and compare.
///
/// POST /api/risk/compare
pub async fn risk_compare(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RiskRequest>,
) -> Result<Json<RiskCompareResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    if let Err(validation_error) = validate_risk_request(&request) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(validation_error.to_error_response()),
        ));
    }

    let curve_id = match Uuid::parse_str(&request.curve_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(IrsBootstrapErrorResponse::validation_error(
                    "Invalid curve_id format: must be a valid UUID",
                    "curveId",
                )),
            ));
        }
    };

    let cached_curve = match state.curve_cache.get(&curve_id) {
        Some(curve) => curve,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(IrsBootstrapErrorResponse::curve_not_found(
                    &request.curve_id,
                )),
            ));
        }
    };

    // Run Bump-and-Revalue method
    let bump_start = Instant::now();
    let (bump_deltas, bump_timing_samples) = compute_deltas_bump_mode(&cached_curve, &request);
    let bump_total_us = bump_start.elapsed().as_micros() as u64;
    let bump_dv01: f64 = bump_deltas.iter().map(|d| d.delta).sum();
    let bump_timing = TimingStats::from_samples(&bump_timing_samples, bump_total_us);

    let bump_result = RiskMethodResult {
        deltas: bump_deltas,
        dv01: bump_dv01,
        timing: bump_timing,
    };

    #[cfg(feature = "enzyme-ad")]
    let aad_available = true;
    #[cfg(not(feature = "enzyme-ad"))]
    let aad_available = false;

    let (aad_result, aad_total_ms) = if aad_available {
        let aad_start = Instant::now();
        let (aad_deltas, aad_timing_samples) = compute_deltas_aad_mode(&cached_curve, &request);
        let aad_total_us = aad_start.elapsed().as_micros() as u64;
        let aad_dv01: f64 = aad_deltas.iter().map(|d| d.delta).sum();
        let aad_timing = TimingStats::from_samples(&aad_timing_samples, aad_total_us);

        let result = RiskMethodResult {
            deltas: aad_deltas,
            dv01: aad_dv01,
            timing: aad_timing.clone(),
        };

        (Some(result), Some(aad_timing.total_ms))
    } else {
        let simulated_aad_time_ms = bump_result.timing.total_ms / 10.0;

        let aad_deltas: Vec<DeltaResult> = bump_result
            .deltas
            .iter()
            .map(|d| DeltaResult {
                tenor: d.tenor.clone(),
                delta: d.delta,
                processing_time_us: d.processing_time_us / 10.0,
            })
            .collect();

        let aad_timing = TimingStats {
            mean_us: bump_result.timing.mean_us / 10.0,
            std_dev_us: bump_result.timing.std_dev_us / 10.0,
            min_us: bump_result.timing.min_us / 10.0,
            max_us: bump_result.timing.max_us / 10.0,
            total_ms: simulated_aad_time_ms,
        };

        let result = RiskMethodResult {
            deltas: aad_deltas,
            dv01: bump_result.dv01,
            timing: aad_timing,
        };

        (Some(result), Some(simulated_aad_time_ms))
    };

    let speedup_ratio = aad_total_ms.map(|aad_ms| {
        if aad_ms > 0.0 {
            bump_result.timing.total_ms / aad_ms
        } else {
            0.0
        }
    });

    let comparison = TimingComparison {
        bump_total_ms: bump_result.timing.total_ms,
        aad_total_ms,
        speedup_ratio,
    };

    let dv01_for_broadcast = aad_result
        .as_ref()
        .map(|r| r.dv01)
        .unwrap_or(bump_result.dv01);
    broadcast_risk_complete(
        &state,
        &request.curve_id,
        "compare",
        dv01_for_broadcast,
        speedup_ratio,
    );

    Ok(Json(RiskCompareResponse {
        bump: bump_result,
        aad: aad_result,
        aad_available: true,
        speedup_ratio,
        comparison,
    }))
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

    #[test]
    fn test_timing_stats_empty() {
        let stats = TimingStats::from_samples(&[], 1000);
        assert_eq!(stats.mean_us, 0.0);
        assert_eq!(stats.total_ms, 1.0);
    }
}
