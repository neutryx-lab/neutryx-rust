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
    market::{
        build_forward_rate_shift_grid,
        curves::CurveEnum,
        fx::{FxCurve, FxCurveEnum},
        BootstrapInterpolation, MarketInstrument, YieldCurve,
    },
};

use super::chart_grid::{
    generate_chart_grids, generate_fx_chart_grids, overnight_forward_rate, resolve_day_counter,
    MODEL_DAY_COUNTER,
};
#[cfg(test)]
use crate::rest::dto::CurveInstrumentInput;
use crate::{
    error::ServerError,
    rest::dto::{
        BootstrapMethod, CurveBuildRequest, CurveBuildResponse, CurvePillar, CurveType,
        DiscountFactorRequest, DiscountFactorResponse, FxCurveMethod, ForwardRatePoint,
        ForwardRateRequest, ForwardRateResponse, ForwardSwapRateRequest, ForwardSwapRateResponse,
        JacobianData,
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

        // Branch: credit curve bootstrap from CDS spreads.
        if request.curve_type == CurveType::Credit {
            return Self::build_credit_curve(request, state, reference_date, start);
        }

        // Branch: FX forward curve construction.
        if request.curve_type == CurveType::Fx {
            return Self::build_fx_curve(request, state, reference_date, start);
        }

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
                    coupon_rate: i.coupon_rate,
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

        let config = BootstrapConfig {
            interpolation,
            ..BootstrapConfig::new(request.tolerance, request.max_iterations)
        };
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
            BootstrapMethod::Global
            | BootstrapMethod::LevenbergMarquardt
            | BootstrapMethod::Penalised
            | BootstrapMethod::BestFit => {
                let damping = match request.bootstrap_method {
                    BootstrapMethod::LevenbergMarquardt => Some(1e-3),
                    BootstrapMethod::Penalised => {
                        Some(request.penalty_weight.unwrap_or(1e-4))
                    }
                    _ => None,
                };
                let global_config = GlobalBootstrapConfig {
                    tolerance: request.tolerance,
                    param_tolerance: request.tolerance,
                    max_iterations: request.max_iterations,
                    interpolation,
                    damping_factor: damping,
                    ..Default::default()
                };
                let global = GlobalBootstrapper::new(global_config);

                let n = market_instruments.len();
                let method_label = match request.bootstrap_method {
                    BootstrapMethod::LevenbergMarquardt => "levenberg_marquardt",
                    BootstrapMethod::Penalised => "penalised",
                    BootstrapMethod::BestFit => "best_fit",
                    _ => "global",
                };

                let result = global
                    .calibrate_with_shift_grid(&market_instruments, &jump_data)
                    .map_err(|e| {
                        ServerError::Pricing(format!("{} calibration failed: {e}", method_label))
                    })?;
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
                (curve, jacobian, method_label)
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
                        "bond" => "Bond",
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
                    survival_probability: None,
                    hazard_rate: None,
                    fx_forward: None,
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
            BootstrapInterpolation::CubicSplineFwd => "cubic_spline_fwd",
            BootstrapInterpolation::MonotoneConvex => "monotone_convex",
            BootstrapInterpolation::LogCubicDF => "log_cubic_df",
            BootstrapInterpolation::TensionSpline => "tension_spline",
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
            curve_type: "rate".to_string(),
            spot: None,
            currency_pair: None,
        })
    }

    /// Build a credit (survival probability) curve from CDS spreads.
    fn build_credit_curve(
        request: &CurveBuildRequest,
        state: &Arc<AppState>,
        reference_date: chrono::NaiveDate,
        start: Instant,
    ) -> Result<CurveBuildResponse, ServerError> {
        // Retrieve the risk-free discount curve from cache.
        let discount_curve_id = request
            .discount_curve_id
            .as_ref()
            .ok_or_else(|| {
                ServerError::InvalidRequest(
                    "discount_curve_id is required for credit curves".to_string(),
                )
            })?
            .parse()
            .map_err(|_| {
                ServerError::InvalidRequest("Invalid discount_curve_id format".to_string())
            })?;

        let rf_entry = state.curve_cache.get(&discount_curve_id).ok_or_else(|| {
            ServerError::NotFound(format!(
                "Discount curve {} not found — build the rate curve first",
                request.discount_curve_id.as_deref().unwrap_or("?")
            ))
        })?;

        // Determine max maturity from CDS instruments.
        let cds_specs: Vec<(String, f64, f64)> = request
            .instruments
            .iter()
            .filter(|i| {
                let t = i.instrument_type.to_lowercase();
                t == "cds" || t == "credit"
            })
            .map(|i| {
                let tenor_years = parse_tenor_to_years(&i.tenor).unwrap_or(1.0);
                (i.tenor.clone(), tenor_years, i.rate)
            })
            .collect();

        if cds_specs.is_empty() {
            return Err(ServerError::InvalidRequest(
                "At least one CDS instrument is required for credit curves".to_string(),
            ));
        }

        let max_maturity = cds_specs.iter().map(|(_, t, _)| *t).fold(1.0_f64, f64::max);

        // Pre-sample risk-free DFs at quarterly intervals.
        let rf_dfs = Self::sample_discount_factors(&rf_entry.curve, max_maturity);

        // Convert to MarketInstrument::Cds.
        let recovery = request.recovery_rate;
        let mut market_instruments: Vec<MarketInstrument<f64>> = cds_specs
            .iter()
            .map(|(_, tenor_years, spread)| {
                MarketInstrument::cds(*tenor_years, *spread, recovery, rf_dfs.clone())
            })
            .collect();
        market_instruments.sort_by(|a, b| {
            a.maturity()
                .partial_cmp(&b.maturity())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        market_instruments.dedup_by(|a, b| (a.maturity() - b.maturity()).abs() < 1e-10);

        // Bootstrap using the standard bootstrapper.
        let interpolation = request.interpolation;
        let config = BootstrapConfig {
            interpolation,
            ..BootstrapConfig::new(request.tolerance, request.max_iterations)
        };
        let bootstrapper = CurveBootstrapper::with_config(config);

        let (curve, jacobian) = bootstrapper
            .bootstrap_to_curve_with_jacobian(&market_instruments, &[])
            .map_err(|e| ServerError::Pricing(format!("Credit curve bootstrap failed: {e}")))?;

        // Build Jacobian data with CDS labels.
        let jacobian_data = {
            let labels: Vec<String> = cds_specs
                .iter()
                .map(|(tenor, _, _)| format!("CDS-{}", tenor))
                .collect();
            let size = jacobian.size;
            let matrix = jacobian.data;
            Some(JacobianData {
                row_labels: labels.clone(),
                col_labels: labels,
                matrix,
                size,
            })
        };

        let day_counter = resolve_day_counter(&request.index);

        // Build pillars with survival probability and hazard rate.
        let pillars: Vec<CurvePillar> = curve
            .pillars()
            .iter()
            .filter_map(|time| {
                let sp = curve.discount_factor(*time).ok()?;
                let days = (*time * 365.0).round() as i64;
                let date = reference_date + chrono::Duration::days(days);
                let hazard = if *time > 0.0 { -sp.ln() / *time } else { 0.0 };
                Some(CurvePillar {
                    date: date.format("%Y-%m-%d").to_string(),
                    time: *time,
                    discount_factor: sp,
                    zero_rate: zero_rate_from_df(sp, *time),
                    forward_rate: hazard,
                    survival_probability: Some(sp),
                    hazard_rate: Some(hazard),
                    fx_forward: None,
                })
            })
            .collect();

        // Forward curve: hazard rate on daily grid.
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
            BootstrapInterpolation::CubicSplineFwd => "cubic_spline_fwd",
            BootstrapInterpolation::MonotoneConvex => "monotone_convex",
            BootstrapInterpolation::LogCubicDF => "log_cubic_df",
            BootstrapInterpolation::TensionSpline => "tension_spline",
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
            instrument_count: cds_specs.len(),
            interpolation: interpolation_str,
            converged: true,
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
            bootstrap_method: "bootstrapping".to_string(),
            jacobian: jacobian_data,
            curve_type: "credit".to_string(),
            spot: None,
            currency_pair: None,
        })
    }

    /// Build an FX forward curve.
    fn build_fx_curve(
        request: &CurveBuildRequest,
        state: &Arc<AppState>,
        reference_date: chrono::NaiveDate,
        start: Instant,
    ) -> Result<CurveBuildResponse, ServerError> {
        let spot = request.spot.ok_or_else(|| {
            ServerError::InvalidRequest("spot is required for FX curves".to_string())
        })?;
        let pair_str = request.currency_pair.as_deref().ok_or_else(|| {
            ServerError::InvalidRequest("currency_pair is required for FX curves".to_string())
        })?;

        let currency_pair = Self::parse_currency_pair(pair_str)?;

        // Collect pillar times from forward instruments.
        let pillar_specs: Vec<(f64, String, f64)> = request
            .instruments
            .iter()
            .filter(|i| i.instrument_type.to_lowercase() == "fx_forward")
            .filter_map(|i| {
                let tenor_years = parse_tenor_to_years(&i.tenor).ok()?;
                Some((tenor_years, i.tenor.clone(), i.rate))
            })
            .collect();

        let fx_curve: FxCurveEnum<f64> = match request.fx_curve_method {
            FxCurveMethod::Flat => {
                // Compute annualised forward points from pips.
                // Determine pip scaling from pair (JPY pairs use 100, others use 10000).
                let is_jpy_pair = pair_str.contains("JPY");
                let pip_scale = if is_jpy_pair { 100.0 } else { 10_000.0 };

                // Fit a flat forward-points-per-year by using the 1Y point if available,
                // otherwise average the annualised rates.
                let fwd_pts_per_year = if let Some((t, _, pts)) =
                    pillar_specs.iter().find(|(t, _, _)| (*t - 1.0).abs() < 0.01)
                {
                    // Use 1Y forward point directly: fwd_pts = ln(F/S) where F = S + pts/scale.
                    let fwd = spot + pts / pip_scale;
                    (fwd / spot).ln() / t
                } else if !pillar_specs.is_empty() {
                    // Average annualised log forward points.
                    let sum: f64 = pillar_specs
                        .iter()
                        .map(|(t, _, pts)| {
                            let fwd = spot + pts / pip_scale;
                            if *t > 0.0 {
                                (fwd / spot).ln() / t
                            } else {
                                0.0
                            }
                        })
                        .sum();
                    sum / pillar_specs.len() as f64
                } else {
                    0.0
                };

                FxCurveEnum::flat(spot, fwd_pts_per_year, currency_pair)
            }
            FxCurveMethod::IrpGeneric => {
                let dom_entry = Self::get_cached_curve(state, &request.domestic_curve_id, "domestic")?;
                let for_entry = Self::get_cached_curve(state, &request.foreign_curve_id, "foreign")?;
                FxCurveEnum::irp_generic(
                    spot,
                    CurveEnum::bootstrapped(dom_entry.curve.clone()),
                    CurveEnum::bootstrapped(for_entry.curve.clone()),
                    currency_pair,
                )
            }
        };

        // Determine max time for chart grids.
        let max_time = pillar_specs
            .iter()
            .map(|(t, _, _)| *t)
            .fold(2.0_f64, f64::max);

        // Build pillars at input tenors.
        let pillars: Vec<CurvePillar> = pillar_specs
            .iter()
            .filter_map(|(tenor_years, _tenor_str, _pips)| {
                let fwd = fx_curve.forward_rate(*tenor_years).ok()?;
                let days = (*tenor_years * 365.0).round() as i64;
                let date = reference_date + chrono::Duration::days(days);
                Some(CurvePillar {
                    date: date.format("%Y-%m-%d").to_string(),
                    time: *tenor_years,
                    discount_factor: 0.0,
                    zero_rate: 0.0,
                    forward_rate: fwd,
                    survival_probability: None,
                    hazard_rate: None,
                    fx_forward: Some(fwd),
                })
            })
            .collect();

        // Daily forward curve grid.
        let max_days = (max_time * 365.0).round() as i64;
        let forward_curve: Vec<ForwardRatePoint> = (0..=max_days)
            .filter_map(|day| {
                let time = MODEL_DAY_COUNTER.year_fraction_from_days(day);
                let fwd = fx_curve.forward_rate(time).ok()?;
                let date = reference_date + chrono::Duration::days(day);
                Some(ForwardRatePoint {
                    date: date.format("%Y-%m-%d").to_string(),
                    time,
                    forward_rate: fwd,
                })
            })
            .collect();

        let (short_term_grid, long_term_grid) =
            generate_fx_chart_grids(reference_date, &fx_curve, max_time);

        let elapsed = start.elapsed();

        Ok(CurveBuildResponse {
            curve_id: uuid::Uuid::new_v4().to_string(),
            index: request.index.clone(),
            currency: request.currency.clone(),
            pillars,
            forward_curve,
            short_term_grid,
            long_term_grid,
            instrument_count: pillar_specs.len(),
            interpolation: "fx_forward".to_string(),
            converged: true,
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
            bootstrap_method: match request.fx_curve_method {
                FxCurveMethod::Flat => "flat",
                FxCurveMethod::IrpGeneric => "irp_generic",
            }
            .to_string(),
            jacobian: None,
            curve_type: "fx".to_string(),
            spot: Some(spot),
            currency_pair: Some(pair_str.to_string()),
        })
    }

    /// Parse a 6-character currency pair string (e.g. "EURUSD") into a
    /// `CurrencyPair`.
    fn parse_currency_pair(
        pair_str: &str,
    ) -> Result<infra_domain::trade::instrument_def::CurrencyPair, ServerError> {
        use infra_domain::market::Currency;
        use infra_domain::trade::instrument_def::CurrencyPair;
        if pair_str.len() < 6 {
            return Err(ServerError::InvalidRequest(format!(
                "Currency pair must be 6 characters (e.g. EURUSD), got '{pair_str}'"
            )));
        }
        let base: Currency = pair_str[..3].parse().map_err(|_| {
            ServerError::InvalidRequest(format!(
                "Unknown base currency: {}",
                &pair_str[..3]
            ))
        })?;
        let quote: Currency = pair_str[3..6].parse().map_err(|_| {
            ServerError::InvalidRequest(format!(
                "Unknown quote currency: {}",
                &pair_str[3..6]
            ))
        })?;
        Ok(CurrencyPair::new(base, quote))
    }

    /// Retrieve a cached yield curve by ID field.
    fn get_cached_curve(
        state: &Arc<AppState>,
        curve_id_opt: &Option<String>,
        label: &str,
    ) -> Result<CurveEntry, ServerError> {
        let id_str = curve_id_opt.as_ref().ok_or_else(|| {
            ServerError::InvalidRequest(format!("{label}_curve_id is required for IRP FX curves"))
        })?;
        let id: uuid::Uuid = id_str.parse().map_err(|_| {
            ServerError::InvalidRequest(format!("Invalid {label}_curve_id format"))
        })?;
        state.curve_cache.get(&id).ok_or_else(|| {
            ServerError::NotFound(format!(
                "{} curve {} not found — build the rate curve first",
                label, id_str
            ))
        })
    }

    /// Pre-sample risk-free discount factors at quarterly intervals.
    fn sample_discount_factors(curve: &dyn YieldCurve<f64>, max_maturity: f64) -> Vec<(f64, f64)> {
        let step = 0.25_f64;
        let n = ((max_maturity / step).ceil() as usize).max(1) + 1;
        (0..=n)
            .filter_map(|i| {
                let t = (i as f64) * step;
                let df = curve.discount_factor(t).ok()?;
                Some((t, df))
            })
            .collect()
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
            coupon_rate: None,
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
            coupon_rate: None,
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
            curve_type: CurveType::Rate,
            discount_curve_id: None,
            recovery_rate: 0.40,
            tension: None,
            penalty_weight: None,
            fx_curve_method: FxCurveMethod::Flat,
            currency_pair: None,
            spot: None,
            domestic_curve_id: None,
            foreign_curve_id: None,
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
