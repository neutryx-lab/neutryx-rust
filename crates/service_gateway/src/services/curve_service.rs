//! Curve service wrapping `CurveBootstrapper` facade.

use std::{sync::Arc, time::Instant};

use adapter_loader::{parse_instruments, validate_rates, InstrumentSpec};
use infra_domain::{
    market::definition::JumpPillar,
    time::{parse_tenor_to_years, Date},
};
use pricer_core::math::formulas::{simple_forward_rate, zero_rate_from_df};
use pricer_models::{
    builder::{
        BootstrapConfig, CurveBootstrapper, GlobalBootstrapConfig, GlobalBootstrapper,
        JacobianMatrix,
    },
    market::{build_forward_rate_shift_grid, BootstrapInterpolation, YieldCurve},
};

use super::chart_grid::{
    generate_chart_grids, overnight_forward_rate, resolve_day_counter, MODEL_DAY_COUNTER,
};
#[cfg(test)]
use crate::rest::dto::CurveInstrumentInput;
use crate::{
    error::ServerError,
    rest::dto::{
        BootstrapMethod, CurveBuildRequest, CurveBuildResponse, CurvePillar, DiscountFactorRequest,
        DiscountFactorResponse, ForwardRatePoint, ForwardRateRequest, ForwardRateResponse,
        ForwardSwapRateRequest, ForwardSwapRateResponse, JacobianData,
    },
    state::{AppState, CurveEntry, InstrumentInput},
};

/// Service for building and querying yield curves.
pub struct CurveService;

impl CurveService {
    /// Build a yield curve from market instruments.
    pub fn build_curve(
        request: &CurveBuildRequest,
        state: &Arc<AppState>,
    ) -> Result<CurveBuildResponse, ServerError> {
        let start = Instant::now();

        let reference_date = if let Some(ref date_str) = request.reference_date {
            chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map_err(|e| ServerError::InvalidRequest(format!("Invalid reference_date: {e}")))?
        } else {
            chrono::Utc::now().date_naive()
        };

        let mut regular_specs: Vec<InstrumentSpec> = Vec::new();
        let mut jump_pillars: Vec<JumpPillar> = Vec::new();

        for i in &request.instruments {
            if i.instrument_type.to_lowercase() == "event" {
                if let Some(ref event_date_str) = i.event_date {
                    let event_date = Date::parse(event_date_str).map_err(|e| {
                        ServerError::InvalidRequest(format!("Invalid event_date: {e}"))
                    })?;

                    let expected_spike = i.expected_rate_spike.unwrap_or(i.rate);
                    let jump_bps = expected_spike * 10_000.0;

                    let mut pillar = JumpPillar::new(event_date, jump_bps, 1.0);

                    if let Some(ref end_date_str) = i.end_date {
                        let end_date = Date::parse(end_date_str).map_err(|e| {
                            ServerError::InvalidRequest(format!("Invalid end_date: {e}"))
                        })?;
                        pillar = pillar.with_end_date(end_date);
                    }

                    jump_pillars.push(pillar);
                }
            } else {
                regular_specs.push(InstrumentSpec {
                    instrument_type: i.instrument_type.clone(),
                    tenor: i.tenor.clone(),
                    rate: i.rate,
                    event_date: None,
                    expected_rate_spike: None,
                });
            }
        }

        validate_rates(&regular_specs, -0.10, 0.50)
            .map_err(|e| ServerError::InvalidRequest(format!("Rate validation failed: {e}")))?;

        let market_instruments = if regular_specs.is_empty() {
            return Err(ServerError::InvalidRequest(
                "At least one non-event instrument is required".to_string(),
            ));
        } else {
            parse_instruments(&regular_specs).map_err(|e| {
                ServerError::InvalidRequest(format!("Instrument parsing failed: {e}"))
            })?
        };

        let interpolation = request.interpolation;

        let valuation_date = Date::from_naive(reference_date);
        let max_time = market_instruments
            .iter()
            .map(|i| i.maturity())
            .fold(1.0_f64, f64::max);
        let jump_data = build_forward_rate_shift_grid(
            &jump_pillars,
            valuation_date,
            MODEL_DAY_COUNTER,
            max_time,
        );

        let config = BootstrapConfig::new(request.tolerance, request.max_iterations)
            .with_interpolation(interpolation);
        let bootstrapper = CurveBootstrapper::with_config(config);

        let (curve, maybe_jacobian, actual_method) = match request.bootstrap_method {
            BootstrapMethod::Bootstrapping => {
                let mut deduped = market_instruments.clone();
                deduped.dedup_by(|a, b| (a.maturity() - b.maturity()).abs() < 1e-10);

                let (curve, jac) = bootstrapper
                    .bootstrap_to_curve_with_jacobian(&deduped, &jump_data)
                    .map_err(|e| ServerError::Pricing(format!("Bootstrap failed: {e}")))?;
                (curve, Some(jac), "bootstrapping")
            }
            BootstrapMethod::Global => {
                let global_config = GlobalBootstrapConfig::default()
                    .with_tolerance(request.tolerance)
                    .with_max_iterations(request.max_iterations)
                    .with_interpolation(interpolation)
                    .with_jacobian_inverse(true);
                let global = GlobalBootstrapper::new(global_config);

                let n = market_instruments.len();

                let result = global
                    .calibrate_with_shift_grid(&market_instruments, &jump_data)
                    .map_err(|e| ServerError::Pricing(format!("Global bootstrap failed: {e}")))?;
                let jacobian = result.jacobian_inverse.as_ref().map(|j_inv| {
                    let size = n.min(j_inv.nrows());
                    let mut data = vec![vec![0.0; size]; size];
                    for i in 0..size {
                        for j in 0..size {
                            data[i][j] = j_inv[(i, j)];
                        }
                    }
                    JacobianMatrix { data, size }
                });
                let curve = if jump_data.is_empty() {
                    result.curve
                } else {
                    result.curve.with_jumps(jump_data.clone())
                };
                (curve, jacobian, "global")
            }
        };

        let jacobian_data = maybe_jacobian.map(|jac| {
            let mut sorted_specs: Vec<(f64, String)> = regular_specs
                .iter()
                .map(|spec| {
                    let t = spec.tenor_years().unwrap_or(0.0);
                    let lower = spec.instrument_type.to_lowercase();
                    let type_label = match lower.as_str() {
                        "deposit" | "depo" => "Depo",
                        "ois" => "OIS",
                        "fra" => "FRA",
                        "swap" | "irs" => "IRS",
                        "future" | "futures" => "Fut",
                        _ => &lower,
                    };
                    (t, format!("{}-{}", type_label, spec.tenor))
                })
                .collect();
            sorted_specs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            sorted_specs.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-10);

            let pillar_times: Vec<f64> = sorted_specs.iter().map(|(t, _)| *t).collect();
            let jacobian_labels: Vec<String> =
                sorted_specs.into_iter().map(|(_, label)| label).collect();

            let matrix: Vec<Vec<f64>> = jac
                .data
                .into_iter()
                .enumerate()
                .map(|(i, row)| {
                    let t_i = pillar_times.get(i).copied().unwrap_or(1.0);
                    if t_i.abs() < 1e-12 {
                        row
                    } else {
                        row.into_iter().map(|v| v / t_i).collect()
                    }
                })
                .collect();

            JacobianData {
                row_labels: jacobian_labels.clone(),
                col_labels: jacobian_labels,
                matrix,
                size: jac.size,
            }
        });

        let day_counter = resolve_day_counter(&request.index);

        let pillars: Vec<CurvePillar> = curve
            .pillars()
            .iter()
            .filter_map(|time| {
                let df = curve.discount_factor(*time).ok()?;
                let days = (*time * 365.0).round() as i64;
                let date = reference_date + chrono::Duration::days(days);
                let fwd = if *time > 0.0 {
                    overnight_forward_rate(&curve, reference_date, date, day_counter).unwrap_or(0.0)
                } else {
                    0.0
                };
                Some(CurvePillar {
                    date: date.format("%Y-%m-%d").to_string(),
                    time: *time,
                    discount_factor: df,
                    zero_rate: zero_rate_from_df(df, *time),
                    forward_rate: fwd,
                })
            })
            .collect();

        let max_days = curve
            .pillars()
            .last()
            .map(|t| (t * 365.0).round() as i64)
            .unwrap_or(365);
        let forward_curve: Vec<ForwardRatePoint> = (0..max_days)
            .filter_map(|day| {
                let date = reference_date + chrono::Duration::days(day);
                let time = MODEL_DAY_COUNTER.year_fraction_from_days(day);
                let fwd = overnight_forward_rate(&curve, reference_date, date, day_counter)?;
                Some(ForwardRatePoint {
                    date: date.format("%Y-%m-%d").to_string(),
                    time,
                    forward_rate: fwd,
                })
            })
            .collect();

        let (short_term_grid, long_term_grid) =
            generate_chart_grids(reference_date, &curve, day_counter);

        let interpolation_str = match interpolation {
            BootstrapInterpolation::Linear => "linear_df",
            BootstrapInterpolation::LogLinear => "log_linear_df",
            BootstrapInterpolation::FlatForward => "flat_forward",
        }
        .to_string();

        let instrument_inputs: Vec<InstrumentInput> = request
            .instruments
            .iter()
            .map(|i| InstrumentInput {
                instrument_type: i.instrument_type.clone(),
                tenor: i.tenor.clone(),
                rate: i.rate,
            })
            .collect();

        let curve_id = state.curve_cache.add(CurveEntry {
            curve,
            instruments: instrument_inputs,
        });

        let elapsed = start.elapsed();

        Ok(CurveBuildResponse {
            curve_id: curve_id.to_string(),
            index: request.index.clone(),
            currency: request.currency.clone(),
            pillars,
            forward_curve,
            short_term_grid,
            long_term_grid,
            instrument_count: request.instruments.len(),
            interpolation: interpolation_str,
            converged: true,
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
            bootstrap_method: actual_method.to_string(),
            jacobian: jacobian_data,
        })
    }

    /// Get discount factor from a cached curve.
    pub fn get_discount_factor(
        request: &DiscountFactorRequest,
        state: &Arc<AppState>,
    ) -> Result<DiscountFactorResponse, ServerError> {
        let curve_id = request
            .curve_id
            .parse()
            .map_err(|_| ServerError::InvalidRequest("Invalid curve_id format".to_string()))?;

        let entry = state.curve_cache.get(&curve_id).ok_or_else(|| {
            ServerError::NotFound(format!("Curve {} not found", request.curve_id))
        })?;

        let df = entry
            .curve
            .discount_factor(request.time)
            .map_err(|e| ServerError::Pricing(format!("Failed to compute discount factor: {e}")))?;
        let zero_rate = zero_rate_from_df(df, request.time);

        Ok(DiscountFactorResponse {
            curve_id: request.curve_id.clone(),
            time: request.time,
            discount_factor: df,
            zero_rate,
        })
    }

    /// Compute forward swap rate matrix from a cached curve.
    pub fn compute_forward_swap_rates(
        request: &ForwardSwapRateRequest,
        state: &Arc<AppState>,
    ) -> Result<ForwardSwapRateResponse, ServerError> {
        let start = Instant::now();

        let curve_id = request
            .curve_id
            .parse()
            .map_err(|_| ServerError::InvalidRequest("Invalid curve_id format".to_string()))?;

        let entry = state.curve_cache.get(&curve_id).ok_or_else(|| {
            ServerError::NotFound(format!("Curve {} not found", request.curve_id))
        })?;

        let mut rates = std::collections::HashMap::new();

        for expiry_str in &request.expiries {
            let expiry_years = parse_tenor_to_years(expiry_str).map_err(|e| {
                ServerError::InvalidRequest(format!("Invalid expiry tenor '{expiry_str}': {e}"))
            })?;

            for tenor_str in &request.tenors {
                let tenor_years = parse_tenor_to_years(tenor_str).map_err(|e| {
                    ServerError::InvalidRequest(format!("Invalid swap tenor '{tenor_str}': {e}"))
                })?;

                let rate = Self::forward_swap_rate(&entry.curve, expiry_years, tenor_years)?;
                let key = format!("{expiry_str}|{tenor_str}");
                rates.insert(key, rate);
            }
        }

        let elapsed = start.elapsed();

        Ok(ForwardSwapRateResponse {
            curve_id: request.curve_id.clone(),
            rates,
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Compute a single forward swap rate: (`df_start` - `df_end`) / annuity.
    pub(crate) fn forward_swap_rate(
        curve: &dyn YieldCurve<f64>,
        expiry_years: f64,
        tenor_years: f64,
    ) -> Result<f64, ServerError> {
        let df_start = curve
            .discount_factor(expiry_years)
            .map_err(|e| ServerError::Pricing(format!("Failed to compute DF at expiry: {e}")))?;
        let df_end = curve
            .discount_factor(expiry_years + tenor_years)
            .map_err(|e| ServerError::Pricing(format!("Failed to compute DF at maturity: {e}")))?;

        let n_payments = tenor_years.ceil() as usize;
        if n_payments == 0 {
            return Err(ServerError::InvalidRequest(
                "Swap tenor must be at least 1 year".to_string(),
            ));
        }

        let annuity: f64 = (1..=n_payments)
            .map(|i| {
                let t = expiry_years + i as f64;
                curve.discount_factor(t).unwrap_or(0.0)
            })
            .sum();

        if annuity.abs() < 1e-15 {
            return Err(ServerError::Pricing("Annuity is zero".to_string()));
        }

        Ok((df_start - df_end) / annuity)
    }

    /// Get forward rate from a cached curve.
    pub fn get_forward_rate(
        request: &ForwardRateRequest,
        state: &Arc<AppState>,
    ) -> Result<ForwardRateResponse, ServerError> {
        let curve_id = request
            .curve_id
            .parse()
            .map_err(|_| ServerError::InvalidRequest("Invalid curve_id format".to_string()))?;

        let entry = state.curve_cache.get(&curve_id).ok_or_else(|| {
            ServerError::NotFound(format!("Curve {} not found", request.curve_id))
        })?;

        if request.end_time <= request.start_time {
            return Err(ServerError::InvalidRequest(
                "end_time must be greater than start_time".to_string(),
            ));
        }

        let df_start = entry
            .curve
            .discount_factor(request.start_time)
            .map_err(|e| ServerError::Pricing(format!("Failed to compute start DF: {e}")))?;
        let df_end = entry
            .curve
            .discount_factor(request.end_time)
            .map_err(|e| ServerError::Pricing(format!("Failed to compute end DF: {e}")))?;
        let tau = request.end_time - request.start_time;
        let forward_rate = simple_forward_rate(df_start, df_end, tau);

        Ok(ForwardRateResponse {
            curve_id: request.curve_id.clone(),
            start_time: request.start_time,
            end_time: request.end_time,
            forward_rate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst(itype: &str, tenor: &str, rate: f64) -> CurveInstrumentInput {
        CurveInstrumentInput {
            instrument_type: itype.to_string(),
            tenor: tenor.to_string(),
            rate,
            event_date: None,
            expected_rate_spike: None,
            end_date: None,
        }
    }

    fn event(date: &str, spike: f64, end: Option<&str>) -> CurveInstrumentInput {
        CurveInstrumentInput {
            instrument_type: "event".to_string(),
            tenor: String::new(),
            rate: 0.0,
            event_date: Some(date.to_string()),
            expected_rate_spike: Some(spike),
            end_date: end.map(String::from),
        }
    }

    fn build_req(
        ref_date: Option<&str>,
        instruments: Vec<CurveInstrumentInput>,
    ) -> CurveBuildRequest {
        CurveBuildRequest {
            index: "USD-SOFR".to_string(),
            currency: "USD".to_string(),
            reference_date: ref_date.map(String::from),
            instruments,
            interpolation: BootstrapInterpolation::Linear,
            bootstrap_method: BootstrapMethod::Bootstrapping,
            tolerance: 1e-10,
            max_iterations: 100,
        }
    }

    #[test]
    fn test_build_simple_curve() {
        let state = AppState::test_state();
        let request = build_req(
            None,
            vec![
                inst("deposit", "1M", 0.05),
                inst("swap", "1Y", 0.052),
                inst("swap", "2Y", 0.054),
            ],
        );

        let response = CurveService::build_curve(&request, &state).unwrap();

        assert!(!response.curve_id.is_empty());
        assert_eq!(response.instrument_count, 3);
        assert!(response.converged);
        assert!(!response.pillars.is_empty());

        for window in response.pillars.windows(2) {
            assert!(window[1].discount_factor <= window[0].discount_factor);
        }
    }

    #[test]
    fn test_build_curve_with_event() {
        let state = AppState::test_state();
        let request = build_req(
            Some("2026-01-29"),
            vec![
                inst("deposit", "1M", 0.05),
                event("2026-03-18", -0.0025, None),
                inst("swap", "1Y", 0.052),
            ],
        );

        let response = CurveService::build_curve(&request, &state).unwrap();

        assert!(!response.curve_id.is_empty());
        assert_eq!(response.instrument_count, 3);
        assert!(response.converged);
        assert!(!response.pillars.is_empty());
    }

    #[test]
    fn test_build_curve_with_turn_event() {
        let state = AppState::test_state();
        let request = build_req(
            Some("2026-01-29"),
            vec![
                inst("deposit", "1M", 0.05),
                event("2026-12-31", 0.001, Some("2027-01-04")),
                inst("swap", "1Y", 0.052),
                inst("swap", "2Y", 0.054),
            ],
        );

        let response = CurveService::build_curve(&request, &state).unwrap();
        assert!(response.converged);

        let max_fwd = response
            .forward_curve
            .iter()
            .map(|p| p.forward_rate)
            .fold(0.0_f64, f64::max);
        assert!(
            max_fwd < 0.10,
            "Forward rate should stay below 10%, got {:.4}%",
            max_fwd * 100.0
        );

        let turn_time = 336.0 / 365.0;
        let revert_time = 340.0 / 365.0;

        let fwd_during_turn: Vec<_> = response
            .forward_curve
            .iter()
            .filter(|p| p.time > turn_time && p.time < revert_time)
            .collect();
        let fwd_after_revert: Vec<_> = response
            .forward_curve
            .iter()
            .filter(|p| p.time > revert_time + 0.01 && p.time < revert_time + 0.05)
            .collect();

        if let (Some(during), Some(after)) = (fwd_during_turn.first(), fwd_after_revert.first()) {
            let diff = during.forward_rate - after.forward_rate;
            assert!(
                diff > 0.0005 && diff < 0.002,
                "Turn should raise fwd by ~10bp: during={:.6}, after={:.6}, diff={:.4}bp",
                during.forward_rate,
                after.forward_rate,
                diff * 10_000.0
            );
        }
    }

    #[test]
    fn test_forward_rate_shift_no_spike() {
        let state = AppState::test_state();
        let request = build_req(
            Some("2026-01-29"),
            vec![
                inst("deposit", "1M", 0.05),
                event("2026-06-01", -0.0025, None),
                inst("swap", "1Y", 0.05),
            ],
        );

        let response = CurveService::build_curve(&request, &state).unwrap();
        assert!(response.converged);

        for point in response.forward_curve.iter().filter(|p| p.time > 0.01) {
            assert!(
                point.forward_rate > -0.05 && point.forward_rate < 0.15,
                "Forward rate out of range at t={:.4}: {:.4}% (old model would spike to ~91%)",
                point.time,
                point.forward_rate * 100.0
            );
        }

        let event_time = 123.0 / 365.0;
        let fwd_before: Vec<_> = response
            .forward_curve
            .iter()
            .filter(|p| p.time > event_time - 0.05 && p.time < event_time - 0.01)
            .collect();
        let fwd_after: Vec<_> = response
            .forward_curve
            .iter()
            .filter(|p| p.time > event_time + 0.01 && p.time < event_time + 0.05)
            .collect();

        if let (Some(before), Some(after)) = (fwd_before.first(), fwd_after.first()) {
            let diff = before.forward_rate - after.forward_rate;
            assert!(
                diff > 0.001 && diff < 0.005,
                "Cut should shift fwd down ~25bp: before={:.6}, after={:.6}, diff={:.4}bp",
                before.forward_rate,
                after.forward_rate,
                diff * 10_000.0
            );
        }
    }
}
