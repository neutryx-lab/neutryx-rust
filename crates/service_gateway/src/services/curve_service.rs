//! Curve service wrapping `CurveBootstrapper` facade
//!
//! Provides high-level curve building operations using `pricer_models`.

use std::{sync::Arc, time::Instant};

use adapter_loader::{parse_instruments, validate_rates, InstrumentSpec};
use pricer_core::math::formulas::{simple_forward_rate, zero_rate_from_df};
use pricer_models::{
    builder::{BootstrapConfig, CurveBootstrapper, InterpolationMethod as BuilderInterpolation},
    market::{curves::MarketInstrument, YieldCurve},
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

        // Separate regular instruments from events
        let mut regular_specs: Vec<InstrumentSpec> = Vec::new();
        let mut event_instruments: Vec<MarketInstrument<f64>> = Vec::new();

        for i in &request.instruments {
            if i.instrument_type.to_lowercase() == "event" {
                // Handle Event instruments
                if let Some(ref event_date_str) = i.event_date {
                    let event_date = chrono::NaiveDate::parse_from_str(event_date_str, "%Y-%m-%d")
                        .map_err(|e| {
                            ServerError::InvalidRequest(format!("Invalid event_date: {e}"))
                        })?;

                    // Calculate time to event in years
                    let days = (event_date - reference_date).num_days();
                    if days > 0 {
                        let time_years = days as f64 / 365.0;
                        let expected_spike = i.expected_rate_spike.unwrap_or(i.rate);
                        event_instruments.push(MarketInstrument::event_with_rate(
                            time_years,
                            expected_spike,
                        ));
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

        // Convert event instruments to jump data for curve
        // Jump format: (time, cumulative_offset) where offset is in log(df) space
        // A rate hike (positive spike) decreases df, so offset = -spike
        let mut jumps: Vec<(f64, f64)> = event_instruments
            .iter()
            .filter_map(|inst| inst.expected_jump().map(|jump| (inst.maturity(), jump)))
            .collect();

        // Sort jumps by time and calculate cumulative offsets
        jumps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut cumulative = 0.0;
        let jump_data: Vec<(f64, f64)> = jumps
            .into_iter()
            .map(|(time, jump)| {
                // Convert rate jump to log-space offset: offset = -jump (rate hike decreases
                // df)
                cumulative -= jump;
                (time, cumulative)
            })
            .collect();

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
                // Global bootstrap uses the same sequential method for now
                // (GlobalBootstrapper requires additional feature flag)
                let config = BootstrapConfig::new(request.tolerance, request.max_iterations)
                    .with_interpolation(interpolation);

                CurveBootstrapper::with_config(config)
                    .bootstrap_to_curve(&market_instruments)
                    .map_err(|e| ServerError::Pricing(format!("Global bootstrap failed: {e}")))?
            }
        };

        // Apply jump data to the curve if there are events
        let curve = if jump_data.is_empty() {
            base_curve
        } else {
            base_curve.with_jumps(jump_data)
        };

        // Extract pillar data (bootstrap nodes)
        let pillars: Vec<CurvePillar> = curve
            .pillars()
            .iter()
            .zip(curve.discount_factors().iter())
            .map(|(time, df)| CurvePillar {
                time: *time,
                discount_factor: *df,
                zero_rate: zero_rate_from_df(*df, *time),
            })
            .collect();

        // Generate forward curve on daily grid using DF ratios
        let max_time = curve.pillars().last().copied().unwrap_or(1.0);
        let dt = 1.0 / 365.0; // Daily grid

        // Generate grid from t=0 to max_time, stepping by dt
        // Use YieldCurve::forward_rate which handles t=0 correctly
        let num_points = (max_time / dt).floor() as usize;
        let forward_curve: Vec<ForwardRatePoint> = (0..num_points)
            .filter_map(|i| {
                let t = i as f64 * dt;
                let t_next = t + dt;
                // Use YieldCurve::forward_rate method directly
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
                },
                CurveInstrumentInput {
                    instrument_type: "swap".to_string(),
                    tenor: "1Y".to_string(),
                    rate: 0.052,
                    event_date: None,
                    expected_rate_spike: None,
                },
                CurveInstrumentInput {
                    instrument_type: "swap".to_string(),
                    tenor: "2Y".to_string(),
                    rate: 0.054,
                    event_date: None,
                    expected_rate_spike: None,
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
                },
                CurveInstrumentInput {
                    instrument_type: "event".to_string(),
                    tenor: String::new(),
                    rate: 0.0,
                    event_date: Some("2026-03-18".to_string()),
                    expected_rate_spike: Some(-0.0025),
                },
                CurveInstrumentInput {
                    instrument_type: "swap".to_string(),
                    tenor: "1Y".to_string(),
                    rate: 0.052,
                    event_date: None,
                    expected_rate_spike: None,
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
}
