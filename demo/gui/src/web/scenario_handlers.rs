//! Scenario analysis handlers for the FrictionalBank WebApp.
//!
//! This module provides HTTP handlers for scenario analysis:
//! - GET /api/scenarios/presets - Get all preset scenarios
//! - POST /api/scenarios/run - Execute a scenario
//! - POST /api/scenarios/compare - Compare multiple scenarios
//!
//! # Task Coverage
//!
//! - Task 6.1: プリセットシナリオ一覧 API の実装
//! - Task 6.2: シナリオ実行 API の実装
//! - Task 6.3: シナリオパラメータ調整 UI の実装
//! - Task 6.4: シナリオ比較結果 UI の実装

use axum::{extract::State, http::StatusCode, Json};
use pricer_optimiser::bootstrapping::{
    BootstrapInstrument, GenericBootstrapConfig, SequentialBootstrapper,
};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use super::pricer_types::{
    parse_tenor_to_years, AppliedShiftInfo, CachedCurve, IrsBootstrapErrorResponse, ParRateInput,
    PaymentFrequency, PresetScenarioTypeApi, ScenarioCompareRequest, ScenarioCompareResponse,
    ScenarioPnlResult, ScenarioPresetsResponse, ScenarioRunRequest, ScenarioRunResponse,
};
use super::AppState;

// =============================================================================
// Scenario Presets Handler (Task 6.1)
// =============================================================================

/// Get all available preset scenarios.
///
/// # Endpoint
///
/// `GET /api/scenarios/presets`
///
/// # Requirements Coverage
///
/// - Task 6.1: プリセットシナリオ一覧 API の実装
/// - Requirement 6.2: プリセットシナリオの選択機能
pub async fn get_scenario_presets(
    State(_state): State<Arc<AppState>>,
) -> Json<ScenarioPresetsResponse> {
    Json(ScenarioPresetsResponse::new())
}

// =============================================================================
// Scenario Run Handler (Task 6.2)
// =============================================================================

/// Execute a scenario and calculate P&L impact.
///
/// Supports both preset scenarios and custom risk factor shifts.
///
/// # Endpoint
///
/// `POST /api/scenarios/run`
///
/// # Requirements Coverage
///
/// - Task 6.2: シナリオ実行 API の実装
/// - Requirement 6.1: リスクファクターシフト量を変更して再計算
/// - Requirement 6.3: カスタムシナリオを定義
pub async fn run_scenario(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ScenarioRunRequest>,
) -> Result<Json<ScenarioRunResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    let start = Instant::now();

    // Parse curve_id as UUID
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

    // Get curve from cache
    let cached_curve = match state.curve_cache.get(&curve_id) {
        Some(curve) => curve,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(IrsBootstrapErrorResponse::curve_not_found(&request.curve_id)),
            ));
        }
    };

    // Calculate base NPV (no shift)
    let base_npv = calculate_irs_npv(
        &cached_curve,
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        &request.payment_frequency,
    );

    // Determine scenario name and shift amount
    let (scenario_name, shift_bps, applied_shifts) = if let Some(preset) = &request.preset_scenario
    {
        let shift_bps = get_preset_shift_bps(preset);
        let applied = vec![AppliedShiftInfo {
            factor_type: "rate".to_string(),
            pattern: "*".to_string(),
            shift_amount: shift_bps,
            shift_type: "parallel".to_string(),
        }];
        (preset.name().to_string(), shift_bps, applied)
    } else if !request.custom_shifts.is_empty() {
        // Custom scenario
        let total_shift: f64 = request
            .custom_shifts
            .iter()
            .filter(|s| s.factor_type == "rate")
            .map(|s| s.shift_amount)
            .sum();

        let applied: Vec<AppliedShiftInfo> = request
            .custom_shifts
            .iter()
            .map(|s| AppliedShiftInfo {
                factor_type: s.factor_type.clone(),
                pattern: s.pattern.clone(),
                shift_amount: s.shift_amount,
                shift_type: s.shift_type.clone(),
            })
            .collect();

        let name = request
            .scenario_name
            .clone()
            .unwrap_or_else(|| "Custom Scenario".to_string());

        (name, total_shift, applied)
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(IrsBootstrapErrorResponse::validation_error(
                "Either preset_scenario or custom_shifts must be provided",
                "presetScenario",
            )),
        ));
    };

    // Calculate stressed NPV with shifted curve
    let stressed_npv = if shift_bps.abs() > 1e-10 {
        calculate_stressed_npv(&cached_curve, &request, shift_bps)
    } else {
        base_npv // No shift for non-rate scenarios
    };

    let processing_time_ms = start.elapsed().as_micros() as f64 / 1000.0;
    let result = ScenarioPnlResult::new(scenario_name, base_npv, stressed_npv);

    Ok(Json(ScenarioRunResponse {
        result,
        applied_shifts,
        processing_time_ms,
        job_id: None, // Synchronous execution for now
    }))
}

// =============================================================================
// Scenario Compare Handler (Task 6.4)
// =============================================================================

/// Compare multiple scenarios and return P&L results.
///
/// # Endpoint
///
/// `POST /api/scenarios/compare`
///
/// # Requirements Coverage
///
/// - Task 6.4: シナリオ比較結果 UI の実装
/// - Requirement 6.4: シナリオ間の PnL 比較
pub async fn compare_scenarios(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ScenarioCompareRequest>,
) -> Result<Json<ScenarioCompareResponse>, (StatusCode, Json<IrsBootstrapErrorResponse>)> {
    let start = Instant::now();

    // Parse curve_id as UUID
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

    // Get curve from cache
    let cached_curve = match state.curve_cache.get(&curve_id) {
        Some(curve) => curve,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(IrsBootstrapErrorResponse::curve_not_found(&request.curve_id)),
            ));
        }
    };

    // Calculate base NPV
    let base_npv = calculate_irs_npv(
        &cached_curve,
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        &request.payment_frequency,
    );

    // Run each scenario and collect results
    let mut results: Vec<ScenarioPnlResult> = Vec::new();

    for scenario_type in &request.scenarios {
        let shift_bps = get_preset_shift_bps(scenario_type);

        let stressed_npv = if shift_bps.abs() > 1e-10 {
            let shift_decimal = shift_bps / 10000.0;
            let shifted_par_rates: Vec<ParRateInput> = cached_curve
                .par_rates
                .iter()
                .map(|pr| ParRateInput {
                    tenor: pr.tenor.clone(),
                    rate: pr.rate + shift_decimal,
                })
                .collect();

            calculate_scenario_stressed_npv(
                &shifted_par_rates,
                request.notional,
                request.fixed_rate,
                request.tenor_years,
                &request.payment_frequency,
            )
        } else {
            base_npv
        };

        results.push(ScenarioPnlResult::new(
            scenario_type.name(),
            base_npv,
            stressed_npv,
        ));
    }

    // Find worst and best cases
    let worst_case = results
        .iter()
        .min_by(|a, b| a.pnl.partial_cmp(&b.pnl).unwrap_or(std::cmp::Ordering::Equal))
        .cloned();

    let best_case = results
        .iter()
        .max_by(|a, b| a.pnl.partial_cmp(&b.pnl).unwrap_or(std::cmp::Ordering::Equal))
        .cloned();

    let total_processing_time_ms = start.elapsed().as_micros() as f64 / 1000.0;

    Ok(Json(ScenarioCompareResponse {
        results,
        worst_case,
        best_case,
        total_processing_time_ms,
    }))
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get shift amount in basis points for a preset scenario type.
fn get_preset_shift_bps(preset: &PresetScenarioTypeApi) -> f64 {
    match preset {
        PresetScenarioTypeApi::RateUp1bp => 1.0,
        PresetScenarioTypeApi::RateUp10bp => 10.0,
        PresetScenarioTypeApi::RateUp100bp => 100.0,
        PresetScenarioTypeApi::RateDown1bp => -1.0,
        PresetScenarioTypeApi::RateDown10bp => -10.0,
        PresetScenarioTypeApi::RateDown100bp => -100.0,
        PresetScenarioTypeApi::CurveSteepen => 25.0,
        PresetScenarioTypeApi::CurveFlatten => -25.0,
        PresetScenarioTypeApi::Butterfly => 20.0,
        PresetScenarioTypeApi::CreditWiden50bp => 50.0,
        PresetScenarioTypeApi::CreditWiden100bp => 100.0,
        PresetScenarioTypeApi::EquityDown10Pct => 0.0,
        PresetScenarioTypeApi::EquityDown20Pct => 0.0,
        PresetScenarioTypeApi::FxDown10Pct => 0.0,
        PresetScenarioTypeApi::VolUp5Pts => 0.0,
    }
}

/// Calculate stressed NPV for scenario run request.
fn calculate_stressed_npv(
    cached_curve: &CachedCurve,
    request: &ScenarioRunRequest,
    shift_bps: f64,
) -> f64 {
    let shift_decimal = shift_bps / 10000.0;
    let shifted_par_rates: Vec<ParRateInput> = cached_curve
        .par_rates
        .iter()
        .map(|pr| ParRateInput {
            tenor: pr.tenor.clone(),
            rate: pr.rate + shift_decimal,
        })
        .collect();

    calculate_scenario_stressed_npv(
        &shifted_par_rates,
        request.notional,
        request.fixed_rate,
        request.tenor_years,
        &request.payment_frequency,
    )
}

/// Calculate IRS NPV with the given curve.
fn calculate_irs_npv(
    curve: &CachedCurve,
    notional: f64,
    fixed_rate: f64,
    tenor_years: f64,
    payment_frequency: &PaymentFrequency,
) -> f64 {
    // Get payment frequency as number of payments per year
    let payments_per_year = match payment_frequency {
        PaymentFrequency::Annual => 1,
        PaymentFrequency::SemiAnnual => 2,
        PaymentFrequency::Quarterly => 4,
        PaymentFrequency::Monthly => 12,
    };

    let period = 1.0 / payments_per_year as f64;
    let num_periods = (tenor_years * payments_per_year as f64).round() as usize;

    // Calculate fixed leg PV
    let mut fixed_leg_pv = 0.0;
    let fixed_coupon = notional * fixed_rate * period;

    for i in 1..=num_periods {
        let t = i as f64 * period;
        let df = interpolate_df(curve, t);
        fixed_leg_pv += fixed_coupon * df;
    }

    // Add notional at maturity
    let maturity_df = interpolate_df(curve, tenor_years);
    fixed_leg_pv += notional * maturity_df;

    // Calculate floating leg PV (par swap assumption: floating leg PV = notional)
    let float_leg_pv = notional * 1.0; // At par, floating leg starts at notional

    // NPV = Fixed Leg PV - Float Leg PV (from receiver perspective)
    fixed_leg_pv - float_leg_pv
}

/// Calculate IRS NPV with stressed par rates.
///
/// Re-bootstraps the curve with shifted rates and prices the IRS.
fn calculate_scenario_stressed_npv(
    shifted_par_rates: &[ParRateInput],
    notional: f64,
    fixed_rate: f64,
    tenor_years: f64,
    payment_frequency: &PaymentFrequency,
) -> f64 {
    // Build instruments from shifted par rates
    let instruments: Vec<BootstrapInstrument<f64>> = shifted_par_rates
        .iter()
        .filter_map(|pr| {
            parse_tenor_to_years(&pr.tenor)
                .ok()
                .map(|years| BootstrapInstrument::ois(years, pr.rate))
        })
        .collect();

    if instruments.is_empty() {
        return 0.0;
    }

    // Bootstrap with shifted rates
    let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::default();
    let bootstrapper = SequentialBootstrapper::new(config);

    match bootstrapper.bootstrap(&instruments) {
        Ok(curve_data) => {
            // Calculate NPV with the shifted curve
            let stressed_curve = CachedCurve::new(
                curve_data.pillars.clone(),
                curve_data.discount_factors.clone(),
                CachedCurve::calculate_zero_rates(
                    &curve_data.pillars,
                    &curve_data.discount_factors,
                ),
                shifted_par_rates.to_vec(),
            );

            calculate_irs_npv(
                &stressed_curve,
                notional,
                fixed_rate,
                tenor_years,
                payment_frequency,
            )
        }
        Err(_) => 0.0,
    }
}

/// Interpolate discount factor from curve at time t.
fn interpolate_df(curve: &CachedCurve, t: f64) -> f64 {
    if curve.pillars.is_empty() {
        return 1.0;
    }

    // Find bracketing pillars
    let mut lower_idx = 0;
    let mut upper_idx = curve.pillars.len() - 1;

    for (i, &pillar) in curve.pillars.iter().enumerate() {
        if pillar <= t {
            lower_idx = i;
        }
        if pillar >= t && i < upper_idx {
            upper_idx = i;
            break;
        }
    }

    if lower_idx == upper_idx {
        return curve.discount_factors[lower_idx];
    }

    // Linear interpolation in log space
    let t_lower = curve.pillars[lower_idx];
    let t_upper = curve.pillars[upper_idx];
    let df_lower = curve.discount_factors[lower_idx];
    let df_upper = curve.discount_factors[upper_idx];

    if (t_upper - t_lower).abs() < 1e-10 {
        return df_lower;
    }

    let weight = (t - t_lower) / (t_upper - t_lower);
    let log_df = (1.0 - weight) * df_lower.ln() + weight * df_upper.ln();
    log_df.exp()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 6.1: Preset Scenarios API Tests (TDD)
    // =========================================================================

    #[tokio::test]
    async fn test_get_scenario_presets_returns_all_presets() {
        let state = Arc::new(AppState::new());
        let response = get_scenario_presets(State(state)).await;

        assert_eq!(response.count, 15);
        assert!(!response.presets.is_empty());
    }

    #[tokio::test]
    async fn test_get_scenario_presets_has_rate_scenarios() {
        let state = Arc::new(AppState::new());
        let response = get_scenario_presets(State(state)).await;

        let rate_presets: Vec<_> = response
            .presets
            .iter()
            .filter(|p| p.category == "rate")
            .collect();

        assert_eq!(rate_presets.len(), 6);
    }

    #[tokio::test]
    async fn test_get_scenario_presets_has_categories() {
        let state = Arc::new(AppState::new());
        let response = get_scenario_presets(State(state)).await;

        assert!(response.by_category.contains_key("rate"));
        assert!(response.by_category.contains_key("curve"));
        assert!(response.by_category.contains_key("credit"));
        assert!(response.by_category.contains_key("equity"));
    }

    #[tokio::test]
    async fn test_preset_scenario_info_has_correct_fields() {
        let state = Arc::new(AppState::new());
        let response = get_scenario_presets(State(state)).await;

        let rate_up_1bp = response
            .presets
            .iter()
            .find(|p| matches!(p.scenario_type, PresetScenarioTypeApi::RateUp1bp))
            .unwrap();

        assert_eq!(rate_up_1bp.name, "IR +1bp");
        assert_eq!(rate_up_1bp.category, "rate");
        assert!((rate_up_1bp.shift_amount - 1.0).abs() < 1e-10);
        assert_eq!(rate_up_1bp.shift_unit, "bp");
    }

    // =========================================================================
    // Task 6.2: Scenario Run API Tests (TDD)
    // =========================================================================

    #[test]
    fn test_get_preset_shift_bps_rate_up_1bp() {
        let shift = get_preset_shift_bps(&PresetScenarioTypeApi::RateUp1bp);
        assert!((shift - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_get_preset_shift_bps_rate_down_100bp() {
        let shift = get_preset_shift_bps(&PresetScenarioTypeApi::RateDown100bp);
        assert!((shift - (-100.0)).abs() < 1e-10);
    }

    #[test]
    fn test_get_preset_shift_bps_equity_down() {
        let shift = get_preset_shift_bps(&PresetScenarioTypeApi::EquityDown10Pct);
        assert!((shift - 0.0).abs() < 1e-10); // No rate impact
    }

    // =========================================================================
    // Task 6.4: Scenario PnL Result Tests (TDD)
    // =========================================================================

    #[test]
    fn test_scenario_pnl_result_new_loss() {
        let result = ScenarioPnlResult::new("Test", 1_000_000.0, 990_000.0);

        assert_eq!(result.scenario_name, "Test");
        assert!((result.base_value - 1_000_000.0).abs() < 1e-10);
        assert!((result.stressed_value - 990_000.0).abs() < 1e-10);
        assert!((result.pnl - (-10_000.0)).abs() < 1e-10);
        assert!(result.is_loss);
    }

    #[test]
    fn test_scenario_pnl_result_new_gain() {
        let result = ScenarioPnlResult::new("Test", 1_000_000.0, 1_010_000.0);

        assert!((result.pnl - 10_000.0).abs() < 1e-10);
        assert!(!result.is_loss);
    }

    #[test]
    fn test_scenario_pnl_result_pnl_pct() {
        let result = ScenarioPnlResult::new("Test", 1_000_000.0, 990_000.0);

        // PnL% = -10,000 / 1,000,000 * 100 = -1%
        assert!((result.pnl_pct - (-1.0)).abs() < 1e-10);
    }

    // =========================================================================
    // Helper Function Tests
    // =========================================================================

    #[test]
    fn test_interpolate_df_empty_curve() {
        let curve = CachedCurve::new(vec![], vec![], vec![], vec![]);
        let df = interpolate_df(&curve, 1.0);
        assert!((df - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_df_single_pillar() {
        let curve = CachedCurve::new(vec![1.0], vec![0.97], vec![0.03], vec![]);
        let df = interpolate_df(&curve, 1.0);
        assert!((df - 0.97).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_df_interpolation() {
        let curve = CachedCurve::new(vec![1.0, 2.0], vec![0.97, 0.94], vec![0.03, 0.03], vec![]);
        let df = interpolate_df(&curve, 1.5);
        // Should be between 0.94 and 0.97
        assert!(df > 0.94 && df < 0.97);
    }
}
