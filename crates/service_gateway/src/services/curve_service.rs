//! Curve service wrapping `CurveBootstrapper` facade
//!
//! Provides high-level curve building operations using `pricer_models`.

use std::{sync::Arc, time::Instant};

use adapter_loader::{parse_instruments, validate_rates, InstrumentSpec};
use chrono::{Datelike, Months, NaiveDate};
use infra_domain::{
    market::{definition::JumpPillar, RateIndex},
    time::{parse_tenor_to_years, Date, DayCounter},
};
use pricer_core::math::formulas::{simple_forward_rate, zero_rate_from_df};
use pricer_models::{
    builder::{
        BootstrapConfig, CurveBootstrapper, GlobalBootstrapConfig, GlobalBootstrapper,
        InterpolationMethod as BuilderInterpolation, JacobianMatrix,
        JumpPillar as PricerJumpPillar,
    },
    market::{build_forward_rate_shift_grid, BootstrapInterpolation, YieldCurve},
};

#[cfg(test)]
use crate::rest::dto::CurveInstrumentInput;
use crate::{
    error::ServerError,
    rest::dto::{
        BootstrapMethod, ChartGridPoint, CurveBuildRequest, CurveBuildResponse, CurvePillar,
        DiscountFactorRequest, DiscountFactorResponse, ForwardRatePoint, ForwardRateRequest,
        ForwardRateResponse, ForwardSwapRateRequest, ForwardSwapRateResponse, InterpolationMethod,
        JacobianData,
    },
    state::{AppState, InstrumentInput},
};

// ---------------------------------------------------------------------------
// Chart grid generation helpers
// ---------------------------------------------------------------------------

const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Format date for short-term chart axis: "15-Jan"
fn format_short_term_label(date: NaiveDate) -> String {
    format!("{}-{}", date.day(), MONTH_ABBR[date.month0() as usize])
}

/// Format date for long-term chart axis: "Mar-26"
fn format_long_term_label(date: NaiveDate) -> String {
    format!(
        "{}-{:02}",
        MONTH_ABBR[date.month0() as usize],
        date.year() % 100
    )
}

/// Internal model time axis: ACT/365 Fixed.
///
/// All time conversions in this module use this day counter,
/// ensuring consistency with the pricer_models internal basis.
const MODEL_DAY_COUNTER: DayCounter = DayCounter::Actual365Fixed;

/// Generate short-term grid dates: daily up to 3M, weekly 3M→1Y.
fn generate_short_term_dates(ref_date: NaiveDate) -> Vec<NaiveDate> {
    let three_months = ref_date
        .checked_add_months(Months::new(3))
        .unwrap_or(ref_date);
    let one_year = ref_date
        .checked_add_months(Months::new(12))
        .unwrap_or(ref_date);

    let mut dates = Vec::new();

    // Daily up to 3M
    let mut d = ref_date + chrono::Duration::days(1);
    while d <= three_months {
        dates.push(d);
        d += chrono::Duration::days(1);
    }

    // Weekly from 3M to 1Y
    d = three_months + chrono::Duration::days(7);
    while d <= one_year {
        dates.push(d);
        d += chrono::Duration::days(7);
    }

    dates
}

/// Generate long-term grid dates: quarterly 3M→10Y, semi-annual 10.5Y→20Y,
/// annual 21Y→30Y.
fn generate_long_term_dates(ref_date: NaiveDate) -> Vec<NaiveDate> {
    let mut dates = Vec::new();

    // Quarterly from 3M (q=1) to 10Y (q=40)
    for q in 1..=40u32 {
        if let Some(d) = ref_date.checked_add_months(Months::new(q * 3)) {
            dates.push(d);
        }
    }

    // Semi-annual from 10.5Y (h=21) to 20Y (h=40)
    for h in 21..=40u32 {
        if let Some(d) = ref_date.checked_add_months(Months::new(h * 6)) {
            dates.push(d);
        }
    }

    // Annual from 21Y to 30Y
    for y in 21..=30u32 {
        if let Some(d) = ref_date.checked_add_months(Months::new(y * 12)) {
            dates.push(d);
        }
    }

    dates
}

/// Build `ChartGridPoint` vec from grid dates.
///
/// Time is derived from `(date - ref_date).num_days() / 365.0` at the point of
/// use, guaranteeing consistency with the internal model basis.
fn build_chart_grid<C: YieldCurve<f64>>(
    ref_date: NaiveDate,
    dates: &[NaiveDate],
    curve: &C,
    label_fn: fn(NaiveDate) -> String,
    day_counter: DayCounter,
) -> Vec<ChartGridPoint> {
    dates
        .iter()
        .filter_map(|date| {
            let time = MODEL_DAY_COUNTER.year_fraction_from_days((*date - ref_date).num_days());
            let df = curve.discount_factor(time).ok()?;
            let fwd = if time > 0.0 {
                overnight_forward_rate(curve, ref_date, *date, day_counter)?
            } else {
                0.0
            };
            Some(ChartGridPoint {
                date: date.format("%Y-%m-%d").to_string(),
                time,
                discount_factor: df,
                forward_rate: fwd,
                label: label_fn(*date),
            })
        })
        .collect()
}

/// Resolve the day count convention from the request index name.
///
/// Uses `RateIndex::from_index_name()` to parse compound index names
/// (e.g., "USD-SOFR", "EUR-EURIBOR-6M") and looks up the `DayCounter`
/// from the canonical definition in `infra_domain`.
/// Falls back to ACT/365 Fixed if the index is not recognised.
fn resolve_day_counter(index: &str) -> DayCounter {
    RateIndex::from_index_name(index)
        .map(|ri| ri.day_counter())
        .unwrap_or(DayCounter::Actual365Fixed)
}

/// Compute the overnight forward rate at a given date using the proper
/// day count convention from the index definition.
///
/// 1. Query DF at `date` and `date + 1 calendar day` on the curve's ACT/365
///    Fixed time axis.
/// 2. Compute accrual fraction δ = `DayCounter::year_fraction(date, date + 1)`.
/// 3. Forward rate F = (DF₁ / DF₂ − 1) / δ.
fn overnight_forward_rate<C: YieldCurve<f64>>(
    curve: &C,
    ref_date: NaiveDate,
    date: NaiveDate,
    day_counter: DayCounter,
) -> Option<f64> {
    let d = (date - ref_date).num_days();
    let next_date = date + chrono::Duration::days(1);
    let t1 = MODEL_DAY_COUNTER.year_fraction_from_days(d);
    let t2 = MODEL_DAY_COUNTER.year_fraction_from_days(d + 1);
    let df1 = if t1 <= 0.0 {
        1.0
    } else {
        curve.discount_factor(t1).ok()?
    };
    let df2 = curve.discount_factor(t2).ok()?;
    let delta = day_counter.year_fraction(Date::from(date), Date::from(next_date));
    if delta <= 0.0 {
        return None;
    }
    Some((df1 / df2 - 1.0) / delta)
}

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

        // Separate regular instruments from event instruments.
        // Events are converted to infra_domain::JumpPillar definitions,
        // which pricer_models converts to the internal jump grid.
        let mut regular_specs: Vec<InstrumentSpec> = Vec::new();
        let mut jump_pillars: Vec<JumpPillar> = Vec::new();

        for i in &request.instruments {
            if i.instrument_type.to_lowercase() == "event" {
                if let Some(ref event_date_str) = i.event_date {
                    let event_date = Date::parse(event_date_str).map_err(|e| {
                        ServerError::InvalidRequest(format!("Invalid event_date: {e}"))
                    })?;

                    // API sends decimal rates (e.g. -0.0025); JumpPillar takes bps (-25.0)
                    let expected_spike = i.expected_rate_spike.unwrap_or(i.rate);
                    let jump_bps = expected_spike * 10_000.0;

                    let mut pillar = JumpPillar::new(event_date, jump_bps, 1.0);

                    // Turn events: spike reverts at end_date
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
            InterpolationMethod::LinearDf => BuilderInterpolation::Linear,
            InterpolationMethod::LogLinearDf => BuilderInterpolation::LogLinear,
            InterpolationMethod::FlatForward => BuilderInterpolation::FlatForward,
        };

        // Build the forward-rate-shift grid from jump pillars.
        //
        // Jump pillars (FOMC meetings, turn-of-year events) are converted to
        // a dense daily grid of ramp offsets by pricer_models. This model
        // ensures that forward rates shift as a step function (not a delta),
        // producing smooth curves suitable for pricing.
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

        // Build curve using pricer_models.
        //
        // When jump data is present, we use jump-aware calibration: the
        // bootstrapper evaluates instrument pricing errors on the
        // jump-adjusted curve, so the resulting base DFs ensure the
        // combined (base + jumps) curve correctly reprices all inputs.
        let config = BootstrapConfig::new(request.tolerance, request.max_iterations)
            .with_interpolation(interpolation);
        let bootstrapper = CurveBootstrapper::with_config(config);

        let (curve, maybe_jacobian, actual_method) = match request.bootstrap_method {
            BootstrapMethod::Sequential => {
                let (curve, jac) = bootstrapper
                    .bootstrap_to_curve_with_jacobian(&market_instruments, &jump_data)
                    .map_err(|e| ServerError::Pricing(format!("Bootstrap failed: {e}")))?;
                (curve, Some(jac), "sequential")
            }
            BootstrapMethod::Global => {
                let bootstrap_interp = match interpolation {
                    BuilderInterpolation::Linear => BootstrapInterpolation::Linear,
                    BuilderInterpolation::LogLinear => BootstrapInterpolation::LogLinear,
                    BuilderInterpolation::FlatForward => BootstrapInterpolation::FlatForward,
                };
                let global_config = GlobalBootstrapConfig::default()
                    .with_tolerance(request.tolerance)
                    .with_max_iterations(request.max_iterations)
                    .with_interpolation(bootstrap_interp)
                    .with_jacobian_inverse(true);
                let global = GlobalBootstrapper::new(global_config);

                let n = market_instruments.len();

                if jump_pillars.is_empty() {
                    // No jumps: J⁻¹ is n x n. By IFT d(log DF)/dr = J_sys⁻¹
                    // because ∂F/∂r = -I (pricing_error = theoretical - market).
                    match global.calibrate(&market_instruments) {
                        Ok(result) => {
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
                            (result.curve, jacobian, "global")
                        }
                        Err(e) => {
                            // Global solver failed (e.g. singular Jacobian).
                            // Fall back to sequential bootstrap with FD Jacobian.
                            tracing::warn!(
                                "Global bootstrap failed ({e}), falling back to sequential"
                            );
                            let (curve, jac) = bootstrapper
                                .bootstrap_to_curve_with_jacobian(
                                    &market_instruments,
                                    &jump_data,
                                )
                                .map_err(|e2| {
                                    ServerError::Pricing(format!("Bootstrap failed: {e2}"))
                                })?;
                            (curve, Some(jac), "sequential (fallback)")
                        }
                    }
                } else {
                    // With jumps: global solver merges regular + jump unknowns,
                    // so J⁻¹ dimensions don't map cleanly to regular instruments.
                    let pricer_jumps: Vec<PricerJumpPillar<f64>> = jump_pillars
                        .iter()
                        .map(|jp| {
                            let time =
                                MODEL_DAY_COUNTER.year_fraction(valuation_date, jp.jump_date());
                            PricerJumpPillar::new(time, jp.expected_jump_bps())
                        })
                        .collect();
                    match global.calibrate_with_jumps(&market_instruments, pricer_jumps) {
                        Ok(result) => (result.curve, None, "global"),
                        Err(e) => {
                            tracing::warn!(
                                "Global bootstrap with jumps failed ({e}), \
                                 falling back to sequential"
                            );
                            let (curve, jac) = bootstrapper
                                .bootstrap_to_curve_with_jacobian(
                                    &market_instruments,
                                    &jump_data,
                                )
                                .map_err(|e2| {
                                    ServerError::Pricing(format!("Bootstrap failed: {e2}"))
                                })?;
                            (curve, Some(jac), "sequential (fallback)")
                        }
                    }
                }
            }
        };

        // Build Jacobian labels from regular instrument specs, sorted by
        // maturity to match the bootstrap order.
        let jacobian_data = maybe_jacobian.map(|jac| {
            let mut sorted_specs = regular_specs.clone();
            sorted_specs.sort_by(|a, b| {
                let ta = parse_tenor_to_years(&a.tenor).unwrap_or(0.0);
                let tb = parse_tenor_to_years(&b.tenor).unwrap_or(0.0);
                ta.partial_cmp(&tb).unwrap_or(std::cmp::Ordering::Equal)
            });

            let jacobian_labels: Vec<String> = sorted_specs
                .iter()
                .map(|spec| {
                    let lower = spec.instrument_type.to_lowercase();
                    let type_label = match lower.as_str() {
                        "deposit" | "depo" => "Depo",
                        "ois" => "OIS",
                        "fra" => "FRA",
                        "swap" | "irs" => "IRS",
                        "future" | "futures" => "Fut",
                        _ => &lower,
                    };
                    format!("{}-{}", type_label, spec.tenor)
                })
                .collect();

            JacobianData {
                row_labels: jacobian_labels.clone(),
                col_labels: jacobian_labels,
                matrix: jac.data,
                size: jac.size,
            }
        });

        // Resolve day count convention from the canonical RateIndex definition.
        // Used for all forward rate annualisation (date-based δ).
        let day_counter = resolve_day_counter(&request.index);

        // Extract pillar data with jump-adjusted discount factors
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

        // Generate forward curve on daily grid (date-based iteration)
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

        // Generate pre-computed chart display grids (date-based)
        let short_term_dates = generate_short_term_dates(reference_date);
        let long_term_dates = generate_long_term_dates(reference_date);
        let short_term_grid = build_chart_grid(
            reference_date,
            &short_term_dates,
            &curve,
            format_short_term_label,
            day_counter,
        );
        let long_term_grid = build_chart_grid(
            reference_date,
            &long_term_dates,
            &curve,
            format_long_term_label,
            day_counter,
        );

        let interpolation_str = match request.interpolation {
            InterpolationMethod::LinearDf => "linear_df",
            InterpolationMethod::LogLinearDf => "log_linear_df",
            InterpolationMethod::FlatForward => "flat_forward",
        }
        .to_string();

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

    /// Compute forward swap rate matrix from a cached curve
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

    /// Compute a single forward swap rate: (df_start - df_end) / annuity
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

        // Annuity: sum of DFs at annual payment dates
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
            interpolation: InterpolationMethod::LinearDf,
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
            interpolation: InterpolationMethod::LinearDf,
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
            interpolation: InterpolationMethod::LinearDf,
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
            interpolation: InterpolationMethod::LinearDf,
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
