//! Curve service wrapping CurveBootstrapper facade
//!
//! Provides high-level curve building operations using pricer_models.

use std::{sync::Arc, time::Instant};

use adapter_loader::{parse_instruments, validate_rates, InstrumentSpec};
use pricer_core::math::formulas::{simple_forward_rate, zero_rate_from_df};
use pricer_models::{
    builder::{BootstrapConfig, CurveBootstrapper, InterpolationMethod as BuilderInterpolation},
    market::YieldCurve,
};

use crate::{
    error::ServerError,
    rest::dto::{
        BootstrapMethod, CurveBuildRequest, CurveBuildResponse, CurvePillar, DiscountFactorRequest,
        DiscountFactorResponse, ForwardRateRequest, ForwardRateResponse, InterpolationMethod,
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

        // Convert DTO to adapter_loader specs
        let specs: Vec<InstrumentSpec> = request
            .instruments
            .iter()
            .map(|i| InstrumentSpec {
                instrument_type: i.instrument_type.clone(),
                tenor: i.tenor.clone(),
                rate: i.rate,
            })
            .collect();

        // Validate rates
        validate_rates(&specs, -0.10, 0.50)
            .map_err(|e| ServerError::InvalidRequest(format!("Rate validation failed: {e}")))?;

        // Parse instruments using adapter_loader
        let market_instruments = parse_instruments(&specs)
            .map_err(|e| ServerError::InvalidRequest(format!("Instrument parsing failed: {e}")))?;

        // Convert interpolation method
        let interpolation = match request.interpolation {
            InterpolationMethod::Linear => BuilderInterpolation::Linear,
            InterpolationMethod::LogLinear => BuilderInterpolation::LogLinear,
            InterpolationMethod::CubicSpline => BuilderInterpolation::CubicSpline,
        };

        // Build curve using pricer_models CurveBootstrapper
        let curve = match request.bootstrap_method {
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

        // Extract pillar data
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
            instruments: vec![
                CurveInstrumentInput {
                    instrument_type: "deposit".to_string(),
                    tenor: "1M".to_string(),
                    rate: 0.05,
                },
                CurveInstrumentInput {
                    instrument_type: "swap".to_string(),
                    tenor: "1Y".to_string(),
                    rate: 0.052,
                },
                CurveInstrumentInput {
                    instrument_type: "swap".to_string(),
                    tenor: "2Y".to_string(),
                    rate: 0.054,
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
}
