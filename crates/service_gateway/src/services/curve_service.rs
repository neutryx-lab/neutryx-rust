//! Curve service wrapping `CurveBootstrapper` facade
//!
//! Provides high-level curve building operations using `pricer_models`.

use std::{sync::Arc, time::Instant};

use adapter_loader::{parse_instruments, validate_rates, InstrumentSpec};
use pricer_core::math::formulas::{simple_forward_rate, zero_rate_from_df};
use pricer_models::{
    builder::{BootstrapConfig, CurveBootstrapper, InterpolationMethod as BuilderInterpolation},
    market::YieldCurve,
};

#[cfg(test)]
use crate::rest::dto::CurveInstrumentInput;
use crate::{
    error::ServerError,
    rest::dto::{
        BootstrapMethod, CurveBuildRequest, CurveBuildResponse, CurvePillar, DiscountFactorRequest,
        DiscountFactorResponse, ForwardRatePoint, ForwardRateRequest, ForwardRateResponse,
        InterpolationMethod,
    },
    state::{AppState, InstrumentInput},
};

/// Service for building and querying yield curves
pub struct CurveService;

impl CurveService {
    /// Build a yield curve from market instruments
    pub fn build_curve(
        request: &CurveBuildRequest,
        state: &Arc<AppState>,
    ) -> Result<CurveBuildResponse, ServerError> {
        let start = Instant::now();

        // Parse reference date (default to today if not provided)
        let reference_date = if let Some(ref date_str) = request.reference_date {
            chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map_err(|e| ServerError::InvalidRequest(format!("Invalid reference_date: {e}")))?
        } else {
            chrono::Utc::now().date_naive()
        };

        // Separate regular instruments from events.
        // rate_shifts: individual forward-rate shifts at event times.
        //   - FOMC -25bp cut → delta_rate = -0.0025 (forward rate decreases)
        //   - TOY  +10bp     → delta_rate = +0.001  (forward rate increases)
        //   - TOY revert     → delta_rate = -0.001  (forward rate returns to base)
        let mut regular_specs: Vec<InstrumentSpec> = Vec::new();
        let mut rate_shifts: Vec<(f64, f64)> = Vec::new();

        for i in &request.instruments {
            if i.instrument_type.to_lowercase() == "event" {
                // Handle Event instruments (jumps and turns)
                if let Some(ref event_date_str) = i.event_date {
                    let event_date = chrono::NaiveDate::parse_from_str(event_date_str, "%Y-%m-%d")
                        .map_err(|e| {
                            ServerError::InvalidRequest(format!("Invalid event_date: {e}"))
                        })?;

                    let days = (event_date - reference_date).num_days();
                    if days > 0 {
                        let time_years = days as f64 / 365.0;
                        let expected_spike = i.expected_rate_spike.unwrap_or(i.rate);

                        // Forward rate shifts by expected_spike at the event date
                        rate_shifts.push((time_years, expected_spike));

                        // Turn events: forward rate reverts at end_date
                        if let Some(ref end_date_str) = i.end_date {
                            if let Ok(end_date) =
                                chrono::NaiveDate::parse_from_str(end_date_str, "%Y-%m-%d")
                            {
                                let end_days = (end_date - reference_date).num_days();
                                if end_days > 0 {
                                    let end_time = end_days as f64 / 365.0;
                                    rate_shifts.push((end_time, -expected_spike));
                                }
                            }
                        }
                    }
                    // Skip past events (days <= 0)
                }
            } else {
                // Regular instrument
                regular_specs.push(InstrumentSpec {
                    instrument_type: i.instrument_type.clone(),
                    tenor: i.tenor.clone(),
                    rate: i.rate,
                    event_date: None,
                    expected_rate_spike: None,
                });
            }
        }

        rate_shifts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Validate rates for regular instruments only
        validate_rates(&regular_specs, -0.10, 0.50)
            .map_err(|e| ServerError::InvalidRequest(format!("Rate validation failed: {e}")))?;

        // Parse regular instruments using adapter_loader
        let market_instruments = if regular_specs.is_empty() {
            return Err(ServerError::InvalidRequest(
                "At least one non-event instrument is required".to_string(),
            ));
        } else {
            parse_instruments(&regular_specs).map_err(|e| {
                ServerError::InvalidRequest(format!("Instrument parsing failed: {e}"))
            })?
        };

        // Convert interpolation method
        let interpolation = match request.interpolation {
            InterpolationMethod::Linear => BuilderInterpolation::Linear,
            InterpolationMethod::LogLinear => BuilderInterpolation::LogLinear,
            InterpolationMethod::CubicSpline => BuilderInterpolation::CubicSpline,
        };

        // Build curve using pricer_models CurveBootstrapper
        let base_curve = match request.bootstrap_method {
            BootstrapMethod::Sequential => {
                let config = BootstrapConfig::new(request.tolerance, request.max_iterations)
                    .with_interpolation(interpolation);

                CurveBootstrapper::with_config(config)
                    .bootstrap_to_curve(&market_instruments)
                    .map_err(|e| ServerError::Pricing(format!("Bootstrap failed: {e}")))?
            }
            BootstrapMethod::Global => {
                let config = BootstrapConfig::new(request.tolerance, request.max_iterations)
                    .with_interpolation(interpolation);

                CurveBootstrapper::with_config(config)
                    .bootstrap_to_curve(&market_instruments)
                    .map_err(|e| ServerError::Pricing(format!("Global bootstrap failed: {e}")))?
            }
        };

        // Generate time-dependent jump data for forward-rate shift model.
        //
        // The old model applied constant offsets to log(df), which created
        // delta-function spikes in daily forward rates (e.g., -25bp cut → 91% spike!).
        //
        // The correct model: a forward-rate shift s at time t_j changes the DF as:
        //   df(t) = base_df(t) * exp(-s * (t - t_j))   for t > t_j
        //
        // This gives: f(t) = f_base(t) + s  (smooth shift, no spike).
        //
        // We generate a dense daily grid of log-df offsets:
        //   offset(t) = -sum(s_i * (t - t_i))  for all shifts s_i at t_i <= t
        let jump_data: Vec<(f64, f64)> = if rate_shifts.is_empty() {
            Vec::new()
        } else {
            let max_time = base_curve.pillars().last().copied().unwrap_or(1.0);
            let dt = 1.0 / 365.0;
            let grid_count = (max_time / dt).ceil() as usize + 2;
            (0..grid_count)
                .map(|i| {
                    let t = i as f64 * dt;
                    let offset: f64 = rate_shifts
                        .iter()
                        .take_while(|(t_j, _)| *t_j <= t)
                        .map(|(t_j, shift)| -shift * (t - t_j))
                        .sum();
                    (t, offset)
                })
                .collect()
        };

        // Apply jump data to the curve
        let curve = if jump_data.is_empty() {
            base_curve
        } else {
            base_curve.with_jumps(jump_data)
        };

        // Extract pillar data with jump-adjusted discount factors
        let pillars: Vec<CurvePillar> = curve
            .pillars()
            .iter()
            .filter_map(|time| {
                let df = curve.discount_factor(*time).ok()?;
                Some(CurvePillar {
                    time: *time,
                    discount_factor: df,
                    zero_rate: zero_rate_from_df(df, *time),
                })
            })
            .collect();

        // Generate forward curve on daily grid
        let max_time = curve.pillars().last().copied().unwrap_or(1.0);
        let dt = 1.0 / 365.0;
        let num_points = (max_time / dt).floor() as usize;
        let forward_curve: Vec<ForwardRatePoint> = (0..num_points)
            .filter_map(|i| {
                let t = i as f64 * dt;
                let t_next = t + dt;
                let fwd = curve.forward_rate(t, t_next).ok()?;
                Some(ForwardRatePoint {
                    time: t,
                    forward_rate: fwd,
                })
            })
            .collect();

        // Cache the curve
        let instrument_inputs: Vec<InstrumentInput> = request
            .instruments
            .iter()
            .map(|i| InstrumentInput {
                instrument_type: i.instrument_type.clone(),
                tenor: i.tenor.clone(),
                rate: i.rate,
            })
            .collect();

        let curve_id = state.curve_cache.add(curve, instrument_inputs);

        let elapsed = start.elapsed();

        Ok(CurveBuildResponse {
            curve_id: curve_id.to_string(),
            index: request.index.clone(),
            currency: request.currency.clone(),
            pillars,
            forward_curve,
            instrument_count: request.instruments.len(),
            converged: true,
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Get discount factor from a cached curve
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

    /// Get forward rate from a cached curve
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

    fn create_test_state() -> Arc<AppState> { Arc::new(AppState::new()) }

    #[test]
    fn test_build_simple_curve() {
        let state = create_test_state();

        let request = CurveBuildRequest {
            index: "USD-SOFR".to_string(),
            currency: "USD".to_string(),
            reference_date: None,
            instruments: vec![
                CurveInstrumentInput {
                    instrument_type: "deposit".to_string(),
                    tenor: "1M".to_string(),
                    rate: 0.05,
                    event_date: None,
                    expected_rate_spike: None,
                    end_date: None,
                },
                CurveInstrumentInput {
                    instrument_type: "swap".to_string(),
                    tenor: "1Y".to_string(),
                    rate: 0.052,
                    event_date: None,
                    expected_rate_spike: None,
                    end_date: None,
                },
                CurveInstrumentInput {
                    instrument_type: "swap".to_string(),
                    tenor: "2Y".to_string(),
                    rate: 0.054,
                    event_date: None,
                    expected_rate_spike: None,
                    end_date: None,
                },
            ],
            interpolation: InterpolationMethod::Linear,
            bootstrap_method: BootstrapMethod::Sequential,
            tolerance: 1e-10,
            max_iterations: 100,
        };

        let response = CurveService::build_curve(&request, &state).unwrap();

        assert!(!response.curve_id.is_empty());
        assert_eq!(response.instrument_count, 3);
        assert!(response.converged);
        assert!(!response.pillars.is_empty());

        // Verify discount factors are decreasing
        for window in response.pillars.windows(2) {
            assert!(window[1].discount_factor <= window[0].discount_factor);
        }
    }

    #[test]
    fn test_build_curve_with_event() {
        let state = create_test_state();

        let request = CurveBuildRequest {
            index: "USD-SOFR".to_string(),
            currency: "USD".to_string(),
            reference_date: Some("2026-01-29".to_string()),
            instruments: vec![
                CurveInstrumentInput {
                    instrument_type: "deposit".to_string(),
                    tenor: "1M".to_string(),
                    rate: 0.05,
                    event_date: None,
                    expected_rate_spike: None,
                    end_date: None,
                },
                CurveInstrumentInput {
                    instrument_type: "event".to_string(),
                    tenor: String::new(),
                    rate: 0.0,
                    event_date: Some("2026-03-18".to_string()),
                    expected_rate_spike: Some(-0.0025),
                    end_date: None, // Permanent jump (CB meeting)
                },
                CurveInstrumentInput {
                    instrument_type: "swap".to_string(),
                    tenor: "1Y".to_string(),
                    rate: 0.052,
                    event_date: None,
                    expected_rate_spike: None,
                    end_date: None,
                },
            ],
            interpolation: InterpolationMethod::Linear,
            bootstrap_method: BootstrapMethod::Sequential,
            tolerance: 1e-10,
            max_iterations: 100,
        };

        let response = CurveService::build_curve(&request, &state).unwrap();

        assert!(!response.curve_id.is_empty());
        assert_eq!(response.instrument_count, 3);
        assert!(response.converged);
        assert!(!response.pillars.is_empty());
    }

    #[test]
    fn test_build_curve_with_turn_event() {
        let state = create_test_state();

        let request = CurveBuildRequest {
            index: "USD-SOFR".to_string(),
            currency: "USD".to_string(),
            reference_date: Some("2026-01-29".to_string()),
            instruments: vec![
                CurveInstrumentInput {
                    instrument_type: "deposit".to_string(),
                    tenor: "1M".to_string(),
                    rate: 0.05,
                    event_date: None,
                    expected_rate_spike: None,
                    end_date: None,
                },
                CurveInstrumentInput {
                    instrument_type: "event".to_string(),
                    tenor: String::new(),
                    rate: 0.0,
                    event_date: Some("2026-12-31".to_string()),
                    expected_rate_spike: Some(0.001), // 10bp turn spike
                    end_date: Some("2027-01-04".to_string()), // Reverts after Jan 4
                },
                CurveInstrumentInput {
                    instrument_type: "swap".to_string(),
                    tenor: "1Y".to_string(),
                    rate: 0.052,
                    event_date: None,
                    expected_rate_spike: None,
                    end_date: None,
                },
                CurveInstrumentInput {
                    instrument_type: "swap".to_string(),
                    tenor: "2Y".to_string(),
                    rate: 0.054,
                    event_date: None,
                    expected_rate_spike: None,
                    end_date: None,
                },
            ],
            interpolation: InterpolationMethod::Linear,
            bootstrap_method: BootstrapMethod::Sequential,
            tolerance: 1e-10,
            max_iterations: 100,
        };

        let response = CurveService::build_curve(&request, &state).unwrap();
        assert!(response.converged);

        // Verify no extreme spikes in the forward curve.
        // With the forward-rate-shift model, the 10bp turn should produce a smooth
        // ~10bp bump at year-end, not a delta-function spike of 10bp*365 = 36.5%.
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

        // Verify the turn creates a visible bump during the turn period
        let turn_time = 336.0 / 365.0; // ~Dec 31
        let revert_time = 340.0 / 365.0; // ~Jan 4

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
            // During turn should be higher than after revert by ~10bp
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
        // Verify that a -25bp CB cut produces a smooth ~25bp downward shift,
        // not a delta-function spike of 25bp * 365 = 91%.
        let state = create_test_state();

        let request = CurveBuildRequest {
            index: "USD-SOFR".to_string(),
            currency: "USD".to_string(),
            reference_date: Some("2026-01-29".to_string()),
            instruments: vec![
                CurveInstrumentInput {
                    instrument_type: "deposit".to_string(),
                    tenor: "1M".to_string(),
                    rate: 0.05,
                    event_date: None,
                    expected_rate_spike: None,
                    end_date: None,
                },
                CurveInstrumentInput {
                    instrument_type: "event".to_string(),
                    tenor: String::new(),
                    rate: 0.0,
                    event_date: Some("2026-06-01".to_string()),
                    expected_rate_spike: Some(-0.0025), // -25bp cut
                    end_date: None,
                },
                CurveInstrumentInput {
                    instrument_type: "swap".to_string(),
                    tenor: "1Y".to_string(),
                    rate: 0.05,
                    event_date: None,
                    expected_rate_spike: None,
                    end_date: None,
                },
            ],
            interpolation: InterpolationMethod::Linear,
            bootstrap_method: BootstrapMethod::Sequential,
            tolerance: 1e-10,
            max_iterations: 100,
        };

        let response = CurveService::build_curve(&request, &state).unwrap();
        assert!(response.converged);

        // All forward rates should be within a reasonable range (no 91% spikes).
        // The short end (t < 0.01) may be noisy due to interpolation, so skip it.
        for point in response.forward_curve.iter().filter(|p| p.time > 0.01) {
            assert!(
                point.forward_rate > -0.05 && point.forward_rate < 0.15,
                "Forward rate out of range at t={:.4}: {:.4}% (old model would spike to ~91%)",
                point.time,
                point.forward_rate * 100.0
            );
        }

        // Forward rate after the cut should be ~25bp lower than before
        let event_time = 123.0 / 365.0; // ~Jun 1
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
