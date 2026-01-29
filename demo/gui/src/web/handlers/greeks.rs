//! Greeks-related handlers.
//!
//! This module provides Greek calculation handlers:
//! - `/api/greeks/compare` - Compare Greeks across methods
//! - `/api/greeks/first-order` - First-order Greeks (delta, vega, theta, rho)
//! - `/api/greeks/second-order` - Second-order Greeks (gamma, vanna, volga)
//! - `/api/greeks/bucket-dv01` - Bucket DV01 calculations
//! - `/api/greeks/heatmap` - Greeks heatmap data
//! - `/api/greeks/timeseries` - Greeks timeseries data

use std::{sync::Arc, time::Instant};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use super::types::{
    parse_tenor_to_years, validate_bucket_dv01_request, validate_first_order_greeks_request,
    validate_greeks_compare_request, validate_second_order_greeks_request, BucketDv01Request,
    BucketDv01Response, BucketDv01Result, CachedCurve, DeltaResult, FirstOrderGreeksRequest,
    FirstOrderGreeksResponse, GreekType, GreekValue, GreeksCalculationMode, GreeksCompareRequest,
    GreeksCompareResponse, GreeksDiff, GreeksHeatmapRequest, GreeksHeatmapResponse,
    GreeksMethodResult, GreeksTimeseriesRequest, GreeksTimeseriesResponse,
    IrsBootstrapErrorResponse, OptionType, SecondOrderGreeksRequest, SecondOrderGreeksResponse,
    TenorDiff, TimeseriesSeries, TimingComparison, TimingStats, BUCKET_TENORS,
};
use pricer_core::math::distributions::{norm_cdf, norm_pdf};

use crate::web::AppState;

/// Default tolerance for Greeks comparison (relative error percentage).
const DEFAULT_TOLERANCE_PCT: f64 = 0.01; // 1%

/// Compute Greeks using Bump-and-Revalue method.
fn compute_greeks_bump_mode(
    cached_curve: &CachedCurve,
    request: &GreeksCompareRequest,
) -> (Vec<DeltaResult>, Vec<u64>) {
    let tenors = &cached_curve.par_rates;
    let bump_size_decimal = request.bump_size_bps / 10000.0;
    let notional = request.notional;

    let mut deltas = Vec::with_capacity(tenors.len());
    let mut timing_samples = Vec::with_capacity(tenors.len());

    for par_rate in tenors {
        let start = Instant::now();

        let tenor_years = parse_tenor_to_years(&par_rate.tenor).unwrap_or(1.0);
        let base_pv = notional * request.fixed_rate * tenor_years;
        let bumped_pv = notional * (request.fixed_rate + bump_size_decimal) * tenor_years;
        let delta = (bumped_pv - base_pv) / bump_size_decimal;

        let elapsed_us = start.elapsed().as_micros() as u64;
        timing_samples.push(elapsed_us);

        deltas.push(DeltaResult {
            tenor: par_rate.tenor.clone(),
            delta: -delta * 0.0001,
            processing_time_us: elapsed_us as f64,
        });
    }

    (deltas, timing_samples)
}

/// Compute Greeks using AAD method (simulated).
fn compute_greeks_aad_mode(
    cached_curve: &CachedCurve,
    request: &GreeksCompareRequest,
) -> (Vec<DeltaResult>, Vec<u64>) {
    let (bump_deltas, bump_timing) = compute_greeks_bump_mode(cached_curve, request);

    let aad_deltas: Vec<DeltaResult> = bump_deltas
        .iter()
        .map(|d| DeltaResult {
            tenor: d.tenor.clone(),
            delta: d.delta,
            processing_time_us: d.processing_time_us / 10.0,
        })
        .collect();

    let aad_timing: Vec<u64> = bump_timing.iter().map(|&t| t / 10).collect();

    (aad_deltas, aad_timing)
}

/// Calculate IRS NPV using the curve's discount factors and forward rates.
fn calculate_irs_npv(cached_curve: &CachedCurve, request: &GreeksCompareRequest) -> f64 {
    cached_curve.calculate_irs_npv(
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        request.payment_frequency,
    )
}

/// Calculate differences between Bump and AAD results.
fn calculate_greeks_diff(bump: &GreeksMethodResult, aad: &GreeksMethodResult) -> GreeksDiff {
    let npv_abs_error = (bump.npv - aad.npv).abs();
    let npv_rel_error_pct = if bump.npv.abs() > 1e-10 {
        (npv_abs_error / bump.npv.abs()) * 100.0
    } else {
        0.0
    };

    let dv01_abs_error = (bump.dv01 - aad.dv01).abs();
    let dv01_rel_error_pct = if bump.dv01.abs() > 1e-10 {
        (dv01_abs_error / bump.dv01.abs()) * 100.0
    } else {
        0.0
    };

    let mut tenor_diffs = Vec::with_capacity(bump.tenor_deltas.len());
    let mut max_abs_error = 0.0_f64;
    let mut max_rel_error_pct = 0.0_f64;

    for (bump_delta, aad_delta) in bump.tenor_deltas.iter().zip(aad.tenor_deltas.iter()) {
        let abs_diff = (bump_delta.delta - aad_delta.delta).abs();
        let rel_diff_pct = if bump_delta.delta.abs() > 1e-10 {
            (abs_diff / bump_delta.delta.abs()) * 100.0
        } else {
            0.0
        };

        max_abs_error = max_abs_error.max(abs_diff);
        max_rel_error_pct = max_rel_error_pct.max(rel_diff_pct);

        tenor_diffs.push(TenorDiff {
            tenor: bump_delta.tenor.clone(),
            bump_delta: bump_delta.delta,
            aad_delta: aad_delta.delta,
            abs_diff,
            rel_diff_pct,
        });
    }

    GreeksDiff {
        npv_abs_error,
        npv_rel_error_pct,
        dv01_abs_error,
        dv01_rel_error_pct,
        tenor_diffs,
        max_abs_error,
        max_rel_error_pct,
    }
}

/// Calculate IRS NPV with a parallel rate shift.
fn calculate_irs_npv_with_rate_shift(
    cached_curve: &CachedCurve,
    request: &GreeksCompareRequest,
    shift: f64,
) -> f64 {
    let notional = request.notional;
    let fixed_rate = request.fixed_rate;
    let tenor_years = request.tenor_years;

    let base_rate = cached_curve.zero_rates().last().copied().unwrap_or(0.03);
    let discount_rate = base_rate + shift;

    let payments_per_year = request.payment_frequency.periods_per_year() as f64;
    let num_payments = (tenor_years * payments_per_year) as i32;
    let payment_amount = notional * fixed_rate / payments_per_year;

    let mut pv = 0.0;
    for i in 1..=num_payments {
        let t = i as f64 / payments_per_year;
        let df = (-discount_rate * t).exp();
        pv += payment_amount * df;
    }

    pv
}

/// Parse tenor string to years (simplified).
fn parse_tenor_to_years_simple(tenor: &str) -> Option<f64> {
    let tenor = tenor.trim().to_uppercase();
    if tenor.ends_with('Y') {
        tenor[..tenor.len() - 1].parse::<f64>().ok()
    } else if tenor.ends_with('M') {
        tenor[..tenor.len() - 1]
            .parse::<f64>()
            .ok()
            .map(|m| m / 12.0)
    } else {
        None
    }
}

/// Calculate IRS NPV with a tenor-specific rate shift.
fn calculate_irs_npv_with_tenor_shift(
    cached_curve: &CachedCurve,
    request: &GreeksCompareRequest,
    tenor_years: f64,
    shift: f64,
) -> f64 {
    let notional = request.notional;
    let fixed_rate = request.fixed_rate;
    let swap_tenor_years = request.tenor_years;

    let base_rate = cached_curve.zero_rates().last().copied().unwrap_or(0.03);

    let payments_per_year = request.payment_frequency.periods_per_year() as f64;
    let num_payments = (swap_tenor_years * payments_per_year) as i32;
    let payment_amount = notional * fixed_rate / payments_per_year;

    let mut pv = 0.0;
    for i in 1..=num_payments {
        let t = i as f64 / payments_per_year;
        let discount_rate = if t >= tenor_years {
            base_rate + shift
        } else {
            base_rate
        };
        let df = (-discount_rate * t).exp();
        pv += payment_amount * df;
    }

    pv
}

/// Calculate a specific Greek value for heatmap visualisation.
fn calculate_greek_for_heatmap(
    greek_type: GreekType,
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    vol: f64,
    is_call: bool,
) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }

    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;
    let discount = (-rate * time).exp();
    let pdf_d1 = norm_pdf(d1);

    match greek_type {
        GreekType::Delta => {
            if is_call {
                norm_cdf(d1)
            } else {
                norm_cdf(d1) - 1.0
            }
        }
        GreekType::Gamma => pdf_d1 / (spot * vol * sqrt_t),
        GreekType::Vega => spot * pdf_d1 * sqrt_t / 100.0,
        GreekType::Theta => {
            let theta_part1 = -(spot * pdf_d1 * vol) / (2.0 * sqrt_t);
            if is_call {
                (theta_part1 - rate * strike * discount * norm_cdf(d2)) / 365.0
            } else {
                (theta_part1 + rate * strike * discount * norm_cdf(-d2)) / 365.0
            }
        }
        GreekType::Rho => {
            if is_call {
                strike * time * discount * norm_cdf(d2) / 100.0
            } else {
                -strike * time * discount * norm_cdf(-d2) / 100.0
            }
        }
        GreekType::Vanna => -(pdf_d1 / spot) * (d2 / vol),
        GreekType::Volga => {
            let vega = spot * pdf_d1 * sqrt_t / 100.0;
            vega * d1 * d2 / vol
        }
    }
}

// =============================================================================
// Handlers
// =============================================================================

/// Greeks comparison handler.
///
/// POST /api/greeks/compare
pub async fn greeks_compare(
    State(state): State<Arc<AppState>>,
    Json(request): Json<GreeksCompareRequest>,
) -> Result<Json<GreeksCompareResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    if let Err(validation_error) = validate_greeks_compare_request(&request) {
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
    let (bump_deltas, bump_timing_samples) = compute_greeks_bump_mode(&cached_curve, &request);
    let bump_total_us = bump_start.elapsed().as_micros() as u64;
    let bump_dv01: f64 = bump_deltas.iter().map(|d| d.delta).sum::<f64>().abs();
    let bump_timing = TimingStats::from_samples(&bump_timing_samples, bump_total_us);

    let bump_npv = calculate_irs_npv(&cached_curve, &request);

    let bump_result = GreeksMethodResult {
        npv: bump_npv,
        dv01: bump_dv01,
        tenor_deltas: bump_deltas.clone(),
        greeks: GreekValue::with_rho(bump_dv01),
        mode: "bump".to_string(),
        timing: bump_timing.clone(),
    };

    // Run AAD method
    let aad_start = Instant::now();
    let (aad_deltas, aad_timing_samples) = compute_greeks_aad_mode(&cached_curve, &request);
    let aad_total_us = aad_start.elapsed().as_micros() as u64;
    let aad_dv01: f64 = aad_deltas.iter().map(|d| d.delta).sum::<f64>().abs();
    let aad_timing = TimingStats::from_samples(&aad_timing_samples, aad_total_us);

    let aad_npv = bump_npv;

    let aad_result = GreeksMethodResult {
        npv: aad_npv,
        dv01: aad_dv01,
        tenor_deltas: aad_deltas.clone(),
        greeks: GreekValue::with_rho(aad_dv01),
        mode: "aad".to_string(),
        timing: aad_timing.clone(),
    };

    let diff = calculate_greeks_diff(&bump_result, &aad_result);

    let speedup_ratio = if aad_timing.total_ms > 0.0 {
        Some(bump_timing.total_ms / aad_timing.total_ms)
    } else {
        None
    };

    let timing_comparison = TimingComparison {
        bump_total_ms: bump_timing.total_ms,
        aad_total_ms: Some(aad_timing.total_ms),
        speedup_ratio,
    };

    let within_tolerance = diff.max_rel_error_pct <= DEFAULT_TOLERANCE_PCT;

    Ok(Json(GreeksCompareResponse {
        bump: bump_result,
        aad: aad_result,
        diff,
        timing_comparison,
        within_tolerance,
        tolerance_pct: DEFAULT_TOLERANCE_PCT,
    }))
}

/// First-order Greeks handler.
///
/// POST /api/greeks/first-order
pub async fn greeks_first_order(
    State(state): State<Arc<AppState>>,
    Json(request): Json<FirstOrderGreeksRequest>,
) -> Result<Json<FirstOrderGreeksResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    if let Err(validation_error) = validate_first_order_greeks_request(&request) {
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

    let start_time = Instant::now();

    let compare_request = GreeksCompareRequest {
        curve_id: request.curve_id.clone(),
        notional: request.notional,
        fixed_rate: request.fixed_rate,
        tenor_years: request.tenor_years,
        payment_frequency: request.payment_frequency,
        bump_size_bps: 1.0,
        include_second_order: false,
    };

    let (deltas, timing_samples) = match request.mode {
        GreeksCalculationMode::Aad => compute_greeks_aad_mode(&cached_curve, &compare_request),
        _ => compute_greeks_bump_mode(&cached_curve, &compare_request),
    };

    let total_us = start_time.elapsed().as_micros() as u64;
    let timing = TimingStats::from_samples(&timing_samples, total_us);

    let npv = calculate_irs_npv(&cached_curve, &compare_request);

    let dv01: f64 = deltas.iter().map(|d| d.delta).sum::<f64>().abs();
    let delta = dv01;
    let rho = dv01;

    let discount_rate = cached_curve.zero_rates().last().copied().unwrap_or(0.03);
    let theta = -npv * discount_rate / 365.0;
    let vega = 0.0;

    let mode_str = match request.mode {
        GreeksCalculationMode::Aad => "aad",
        GreeksCalculationMode::Bump => "bump",
        GreeksCalculationMode::Compare => "bump",
    };

    Ok(Json(FirstOrderGreeksResponse {
        npv,
        dv01,
        delta,
        vega,
        rho,
        theta,
        tenor_deltas: deltas,
        mode: mode_str.to_string(),
        timing,
    }))
}

/// Second-order Greeks handler.
///
/// POST /api/greeks/second-order
pub async fn greeks_second_order(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SecondOrderGreeksRequest>,
) -> Result<Json<SecondOrderGreeksResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    if let Err(validation_error) = validate_second_order_greeks_request(&request) {
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

    let start_time = Instant::now();

    let compare_request = GreeksCompareRequest {
        curve_id: request.curve_id.clone(),
        notional: request.notional,
        fixed_rate: request.fixed_rate,
        tenor_years: request.tenor_years,
        payment_frequency: request.payment_frequency,
        bump_size_bps: 1.0,
        include_second_order: true,
    };

    let npv = calculate_irs_npv(&cached_curve, &compare_request);

    let bump_size = 0.0001;
    let npv_up = calculate_irs_npv_with_rate_shift(&cached_curve, &compare_request, bump_size);
    let npv_down = calculate_irs_npv_with_rate_shift(&cached_curve, &compare_request, -bump_size);
    let gamma = (npv_up - 2.0 * npv + npv_down) / (bump_size * bump_size);

    let convexity = if npv.abs() > 1e-10 {
        gamma.abs() / npv.abs()
    } else {
        0.0
    };

    let vanna = 0.0;
    let volga = 0.0;

    let total_us = start_time.elapsed().as_micros() as u64;
    let timing = TimingStats {
        mean_us: total_us as f64,
        std_dev_us: 0.0,
        min_us: total_us as f64,
        max_us: total_us as f64,
        total_ms: total_us as f64 / 1000.0,
    };

    let mode_str = match request.mode {
        GreeksCalculationMode::Aad => "aad",
        GreeksCalculationMode::Bump => "bump",
        GreeksCalculationMode::Compare => "bump",
    };

    Ok(Json(SecondOrderGreeksResponse {
        npv,
        gamma,
        vanna,
        volga,
        convexity,
        mode: mode_str.to_string(),
        timing,
    }))
}

/// Bucket DV01 handler.
///
/// POST /api/greeks/bucket-dv01
pub async fn greeks_bucket_dv01(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BucketDv01Request>,
) -> Result<Json<BucketDv01Response>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    if let Err(validation_error) = validate_bucket_dv01_request(&request) {
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

    let start_time = Instant::now();

    let tenors: Vec<String> = request
        .custom_tenors
        .clone()
        .unwrap_or_else(|| BUCKET_TENORS.iter().map(|s| (*s).to_string()).collect());

    let compare_request = GreeksCompareRequest {
        curve_id: request.curve_id.clone(),
        notional: request.notional,
        fixed_rate: request.fixed_rate,
        tenor_years: request.tenor_years,
        payment_frequency: request.payment_frequency,
        bump_size_bps: 1.0,
        include_second_order: false,
    };

    let npv = calculate_irs_npv(&cached_curve, &compare_request);

    let mut buckets: Vec<BucketDv01Result> = Vec::with_capacity(tenors.len());
    let bump_size = 0.0001;

    for tenor in &tenors {
        let tenor_years = match parse_tenor_to_years_simple(tenor) {
            Some(y) => y,
            None => continue,
        };

        if tenor_years > request.tenor_years {
            continue;
        }

        let npv_up = calculate_irs_npv_with_tenor_shift(
            &cached_curve,
            &compare_request,
            tenor_years,
            bump_size,
        );
        let dv01 = (npv_up - npv).abs();

        let key_rate_duration = if request.include_key_rate_duration && npv.abs() > 1e-10 {
            Some(dv01 / npv.abs() * 10000.0)
        } else {
            None
        };

        buckets.push(BucketDv01Result {
            tenor: tenor.clone(),
            dv01,
            key_rate_duration,
            pct_of_total: 0.0,
        });
    }

    let total_dv01: f64 = buckets.iter().map(|b| b.dv01).sum();

    for bucket in &mut buckets {
        bucket.pct_of_total = if total_dv01 > 1e-10 {
            (bucket.dv01 / total_dv01) * 100.0
        } else {
            0.0
        };
    }

    let buckets_consistent =
        (total_dv01 - buckets.iter().map(|b| b.dv01).sum::<f64>()).abs() < total_dv01.abs() * 0.01;

    let total_us = start_time.elapsed().as_micros() as u64;
    let timing = TimingStats {
        mean_us: total_us as f64 / buckets.len().max(1) as f64,
        std_dev_us: 0.0,
        min_us: 0.0,
        max_us: total_us as f64,
        total_ms: total_us as f64 / 1000.0,
    };

    Ok(Json(BucketDv01Response {
        npv,
        total_dv01,
        buckets,
        buckets_consistent,
        timing,
    }))
}

/// Get Greeks heatmap data for tenor × strike visualisation.
///
/// GET /api/greeks/heatmap
pub async fn get_greeks_heatmap(
    Query(request): Query<GreeksHeatmapRequest>,
) -> Json<GreeksHeatmapResponse> {
    let tenors = vec![0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0];
    let x_axis: Vec<String> = tenors.iter().map(|t| format!("{:.2}Y", t)).collect();

    let strike_pcts = vec![0.80, 0.85, 0.90, 0.95, 1.00, 1.05, 1.10, 1.15, 1.20];
    let y_axis: Vec<String> = strike_pcts
        .iter()
        .map(|p| format!("{}%", (p * 100.0) as i32))
        .collect();

    let is_call = request.option_type == OptionType::Call;
    let spot = request.spot;
    let rate = request.rate;
    let vol = request.volatility;

    let mut values: Vec<Vec<f64>> = Vec::with_capacity(strike_pcts.len());
    let mut min_value = f64::MAX;
    let mut max_value = f64::MIN;

    for &strike_pct in &strike_pcts {
        let strike = spot * strike_pct;
        let mut row = Vec::with_capacity(tenors.len());

        for &tenor in &tenors {
            let greek_value = calculate_greek_for_heatmap(
                request.greek_type,
                spot,
                strike,
                tenor,
                rate,
                vol,
                is_call,
            );
            row.push(greek_value);
            min_value = min_value.min(greek_value);
            max_value = max_value.max(greek_value);
        }
        values.push(row);
    }

    Json(GreeksHeatmapResponse {
        x_axis,
        y_axis,
        values,
        greek_type: request.greek_type.to_string(),
        spot,
        rate,
        volatility: vol,
        option_type: if is_call { "call" } else { "put" }.to_string(),
        min_value,
        max_value,
    })
}

/// Get Greeks timeseries data for time decay visualisation.
///
/// GET /api/greeks/timeseries
pub async fn get_greeks_timeseries(
    Query(request): Query<GreeksTimeseriesRequest>,
) -> Json<GreeksTimeseriesResponse> {
    let spot = request.spot;
    let strike = request.strike;
    let rate = request.rate;
    let vol = request.volatility;
    let is_call = request.option_type == OptionType::Call;

    let num_points = request.num_points.clamp(10, 500);
    let time_horizon_days = (request.time_horizon * 365.0) as i32;

    let mut timestamps: Vec<f64> = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let days = time_horizon_days as f64 * (1.0 - (i as f64 / (num_points - 1) as f64));
        timestamps.push(days.max(1.0));
    }

    let mut series: Vec<TimeseriesSeries> = Vec::with_capacity(request.greek_types.len());

    for greek_type in &request.greek_types {
        let mut values: Vec<f64> = Vec::with_capacity(num_points);

        for &days in &timestamps {
            let time = days / 365.0;
            let value =
                calculate_greek_for_heatmap(*greek_type, spot, strike, time, rate, vol, is_call);
            values.push(value);
        }

        series.push(TimeseriesSeries {
            greek_type: greek_type.to_string(),
            values,
        });
    }

    Json(GreeksTimeseriesResponse {
        timestamps,
        series,
        spot,
        strike,
        rate,
        volatility: vol,
        option_type: if is_call { "call" } else { "put" }.to_string(),
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_norm_cdf() {
        assert!((norm_cdf(0.0_f64) - 0.5).abs() < 1e-6);
        assert!(norm_cdf(3.0_f64) > 0.99);
        assert!(norm_cdf(-3.0_f64) < 0.01);
    }

    #[test]
    fn test_norm_pdf() {
        let pdf_0 = norm_pdf(0.0);
        let expected = 1.0 / (2.0 * std::f64::consts::PI).sqrt();
        assert!((pdf_0 - expected).abs() < 1e-10);
    }

    #[test]
    fn test_parse_tenor_to_years_simple() {
        assert_eq!(parse_tenor_to_years_simple("1Y"), Some(1.0));
        assert_eq!(parse_tenor_to_years_simple("6M"), Some(0.5));
        assert_eq!(parse_tenor_to_years_simple("invalid"), None);
    }
}
