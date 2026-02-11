//! Script engine for path-dependent exotic products.

use pricer_core::kernel::{BarrierType, ScriptKernel, ScriptOp};

use super::provider::CurveProvider;

/// Trait for providing spot prices for exotic products.
pub trait SpotProvider: CurveProvider {
    /// Returns the spot price at a given observation time.
    fn spot_price(&self, index_id: u16, time_years: f64) -> f64;

    /// Returns the current spot price (at valuation date).
    fn current_spot(&self, index_id: u16) -> f64 { self.spot_price(index_id, 0.0) }
}

/// Extension to `FlatCurveProvider` for exotic pricing.
#[derive(Debug, Clone)]
pub struct FlatSpotProvider {
    discount_rate: f64,
    forward_rate_val: f64,
    spot: f64,
    valuation_date_days: i32,
}

impl FlatSpotProvider {
    /// Creates a new flat spot provider.
    #[must_use]
    pub fn new(discount_rate: f64, forward_rate: f64, spot: f64) -> Self {
        Self {
            discount_rate,
            forward_rate_val: forward_rate,
            spot,
            valuation_date_days: 0,
        }
    }

    /// Creates a provider with a specific valuation date.
    #[must_use]
    pub fn with_valuation_date(mut self, valuation_date_days: i32) -> Self {
        self.valuation_date_days = valuation_date_days;
        self
    }
}

impl CurveProvider for FlatSpotProvider {
    fn discount_factor(&self, _curve_id: u8, days_from_epoch: i32) -> f64 {
        let days_to_payment = days_from_epoch - self.valuation_date_days;
        if days_to_payment <= 0 {
            return 1.0;
        }
        let t = days_to_payment as f64 / 365.0;
        (-self.discount_rate * t).exp()
    }

    fn forward_rate(&self, fwd_index_id: u16, _fixing_days: i32, _tenor_days: i32) -> f64 {
        if fwd_index_id == 0 {
            0.0
        } else {
            self.forward_rate_val
        }
    }

    fn fx_rate(&self, _fx_id: u16) -> f64 { 1.0 }

    fn valuation_date_days(&self) -> i32 { self.valuation_date_days }
}

impl SpotProvider for FlatSpotProvider {
    fn spot_price(&self, _index_id: u16, time_years: f64) -> f64 {
        self.spot * (self.discount_rate * time_years).exp()
    }

    fn current_spot(&self, _index_id: u16) -> f64 { self.spot }
}

#[derive(Debug, Clone)]
struct ExecutionState {
    current_value: f64,
    accumulated_sum: f64,
    accumulate_count: u32,
    is_alive: bool,
    is_knocked_in: bool,
    has_knock_in: bool,
    registers: [f64; 8],
    total_pv: f64,
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self {
            current_value: 0.0,
            accumulated_sum: 0.0,
            accumulate_count: 0,
            is_alive: true,
            is_knocked_in: false,
            has_knock_in: false,
            registers: [0.0; 8],
            total_pv: 0.0,
        }
    }
}

impl ExecutionState {
    fn new(kernel: &ScriptKernel) -> Self {
        let has_knock_in = kernel.ops.iter().any(|op| {
            matches!(
                op,
                ScriptOp::CheckBarrier {
                    barrier_type: BarrierType::UpIn | BarrierType::DownIn,
                    ..
                }
            )
        });

        Self {
            has_knock_in,
            is_knocked_in: false,
            ..Default::default()
        }
    }

    #[inline]
    fn should_pay(&self) -> bool { self.is_alive && (!self.has_knock_in || self.is_knocked_in) }
}

/// Script engine for exotic products.
pub struct ScriptEngine;

impl ScriptEngine {
    /// Prices a `ScriptKernel` and returns the present value.
    pub fn price<P: SpotProvider>(kernel: &ScriptKernel, provider: &P) -> f64 {
        if kernel.is_empty() {
            return 0.0;
        }

        let mut state = ExecutionState::new(kernel);
        let mut obs_idx = 0_usize;

        for op in &kernel.ops {
            if !state.is_alive && !matches!(op, ScriptOp::Pay { .. }) {
                continue;
            }

            let current_time = if obs_idx < kernel.observation_count {
                kernel.observation_times[obs_idx]
            } else {
                kernel.observation_times.last().copied().unwrap_or(0.0)
            };

            match *op {
                ScriptOp::CalcFixed { amount_idx } => {
                    state.current_value = kernel.constant(amount_idx);
                }

                ScriptOp::CalcFloat {
                    index_id,
                    gearing_idx,
                    spread_idx,
                } => {
                    let fwd = provider.forward_rate(index_id, 0, 90);
                    let gearing = kernel.constant(gearing_idx);
                    let spread = kernel.constant(spread_idx);
                    state.current_value = fwd * gearing + spread;
                }

                ScriptOp::CheckBarrier {
                    barrier_idx,
                    barrier_type,
                } => {
                    let barrier = kernel.constant(barrier_idx);
                    let spot = provider.spot_price(1, current_time);

                    match barrier_type {
                        BarrierType::UpOut => {
                            if spot >= barrier {
                                state.is_alive = false;
                            }
                        }
                        BarrierType::DownOut => {
                            if spot <= barrier {
                                state.is_alive = false;
                            }
                        }
                        BarrierType::UpIn => {
                            if spot >= barrier {
                                state.is_knocked_in = true;
                            }
                        }
                        BarrierType::DownIn => {
                            if spot <= barrier {
                                state.is_knocked_in = true;
                            }
                        }
                    }

                    obs_idx += 1;
                }

                ScriptOp::Accumulate => {
                    let spot = provider.spot_price(1, current_time);
                    state.accumulated_sum += spot;
                    state.accumulate_count += 1;
                    obs_idx += 1;
                }

                ScriptOp::CalcAverage => {
                    if state.accumulate_count > 0 {
                        state.current_value =
                            state.accumulated_sum / f64::from(state.accumulate_count);
                    } else {
                        state.current_value = 0.0;
                    }
                }

                ScriptOp::ApplyPayoff {
                    strike_idx,
                    is_call,
                } => {
                    let strike = kernel.constant(strike_idx);
                    if is_call {
                        state.current_value = (state.current_value - strike).max(0.0);
                    } else {
                        state.current_value = (strike - state.current_value).max(0.0);
                    }
                }

                ScriptOp::ApplyNotional { notional_idx } => {
                    let notional = kernel.constant(notional_idx);
                    state.current_value *= notional;
                }

                ScriptOp::Pay { ccy_id: _, dc_id } => {
                    if state.should_pay() {
                        let maturity = kernel.maturity().unwrap_or(0.0);
                        let maturity_days =
                            (maturity * 365.0) as i32 + provider.valuation_date_days();
                        let df = provider.discount_factor(dc_id, maturity_days);
                        state.total_pv += state.current_value * df;
                    }
                }

                ScriptOp::EndIf => {}

                ScriptOp::Store { register } => {
                    if (register as usize) < state.registers.len() {
                        state.registers[register as usize] = state.current_value;
                    }
                }

                ScriptOp::Load { register } => {
                    if (register as usize) < state.registers.len() {
                        state.current_value = state.registers[register as usize];
                    }
                }
            }
        }

        state.total_pv
    }

    /// Executes the kernel and returns detailed execution trace.
    pub fn trace<P: SpotProvider>(kernel: &ScriptKernel, provider: &P) -> ExecutionTrace {
        let mut trace = ExecutionTrace::new();

        if kernel.is_empty() {
            return trace;
        }

        let mut state = ExecutionState::new(kernel);
        let mut obs_idx = 0_usize;

        for (op_idx, op) in kernel.ops.iter().enumerate() {
            if !state.is_alive && !matches!(op, ScriptOp::Pay { .. }) {
                trace
                    .steps
                    .push(TraceStep::skipped(op_idx, format!("{op:?}")));
                continue;
            }

            let current_time = if obs_idx < kernel.observation_count {
                kernel.observation_times[obs_idx]
            } else {
                kernel.observation_times.last().copied().unwrap_or(0.0)
            };

            let step = match *op {
                ScriptOp::CheckBarrier {
                    barrier_idx,
                    barrier_type,
                } => {
                    let barrier = kernel.constant(barrier_idx);
                    let spot = provider.spot_price(1, current_time);

                    let triggered = match barrier_type {
                        BarrierType::UpOut | BarrierType::UpIn => spot >= barrier,
                        BarrierType::DownOut | BarrierType::DownIn => spot <= barrier,
                    };

                    match barrier_type {
                        BarrierType::UpOut | BarrierType::DownOut => {
                            if triggered {
                                state.is_alive = false;
                            }
                        }
                        BarrierType::UpIn | BarrierType::DownIn => {
                            if triggered {
                                state.is_knocked_in = true;
                            }
                        }
                    }

                    obs_idx += 1;

                    TraceStep {
                        op_idx,
                        op_name: format!("{barrier_type}"),
                        time: Some(current_time),
                        spot: Some(spot),
                        barrier: Some(barrier),
                        triggered: Some(triggered),
                        current_value: state.current_value,
                        is_alive: state.is_alive,
                        is_knocked_in: state.is_knocked_in,
                        skipped: false,
                    }
                }

                ScriptOp::Accumulate => {
                    let spot = provider.spot_price(1, current_time);
                    state.accumulated_sum += spot;
                    state.accumulate_count += 1;
                    obs_idx += 1;

                    TraceStep {
                        op_idx,
                        op_name: "Accumulate".to_string(),
                        time: Some(current_time),
                        spot: Some(spot),
                        barrier: None,
                        triggered: None,
                        current_value: state.accumulated_sum,
                        is_alive: state.is_alive,
                        is_knocked_in: state.is_knocked_in,
                        skipped: false,
                    }
                }

                ScriptOp::CalcAverage => {
                    if state.accumulate_count > 0 {
                        state.current_value =
                            state.accumulated_sum / f64::from(state.accumulate_count);
                    }

                    TraceStep::executed(
                        op_idx,
                        "CalcAverage".to_string(),
                        state.current_value,
                        &state,
                    )
                }

                ScriptOp::ApplyPayoff {
                    strike_idx,
                    is_call,
                } => {
                    let strike = kernel.constant(strike_idx);
                    if is_call {
                        state.current_value = (state.current_value - strike).max(0.0);
                    } else {
                        state.current_value = (strike - state.current_value).max(0.0);
                    }

                    let name = if is_call {
                        format!("Call(K={strike})")
                    } else {
                        format!("Put(K={strike})")
                    };

                    TraceStep::executed(op_idx, name, state.current_value, &state)
                }

                ScriptOp::Pay { .. } => {
                    let maturity = kernel.maturity().unwrap_or(0.0);
                    let maturity_days = (maturity * 365.0) as i32 + provider.valuation_date_days();
                    let df = provider.discount_factor(0, maturity_days);
                    let pv = if state.should_pay() {
                        state.current_value * df
                    } else {
                        0.0
                    };
                    state.total_pv += pv;

                    TraceStep {
                        op_idx,
                        op_name: format!("Pay(DF={df:.4})"),
                        time: Some(maturity),
                        spot: None,
                        barrier: None,
                        triggered: None,
                        current_value: pv,
                        is_alive: state.is_alive,
                        is_knocked_in: state.is_knocked_in,
                        skipped: false,
                    }
                }

                _ => TraceStep::executed(op_idx, format!("{op:?}"), state.current_value, &state),
            };

            trace.steps.push(step);
        }

        trace.final_pv = state.total_pv;
        trace.is_alive = state.is_alive;
        trace.is_knocked_in = state.is_knocked_in;

        trace
    }
}

/// Execution trace for debugging.
#[derive(Debug, Clone)]
pub struct ExecutionTrace {
    /// Individual execution steps.
    pub steps: Vec<TraceStep>,
    /// Final PV.
    pub final_pv: f64,
    /// Whether option is still alive.
    pub is_alive: bool,
    /// Whether knock-in triggered.
    pub is_knocked_in: bool,
}

impl ExecutionTrace {
    fn new() -> Self {
        Self {
            steps: Vec::new(),
            final_pv: 0.0,
            is_alive: true,
            is_knocked_in: false,
        }
    }
}

/// Single step in execution trace.
#[derive(Debug, Clone)]
pub struct TraceStep {
    /// Operation index.
    pub op_idx: usize,
    /// Operation name.
    pub op_name: String,
    /// Observation time (if applicable).
    pub time: Option<f64>,
    /// Spot price (if applicable).
    pub spot: Option<f64>,
    /// Barrier level (if applicable).
    pub barrier: Option<f64>,
    /// Whether barrier was triggered.
    pub triggered: Option<bool>,
    /// Current value after operation.
    pub current_value: f64,
    /// Is option still alive.
    pub is_alive: bool,
    /// Is knock-in triggered.
    pub is_knocked_in: bool,
    /// Was operation skipped.
    pub skipped: bool,
}

impl TraceStep {
    fn skipped(op_idx: usize, op_name: String) -> Self {
        Self {
            op_idx,
            op_name,
            time: None,
            spot: None,
            barrier: None,
            triggered: None,
            current_value: 0.0,
            is_alive: false,
            is_knocked_in: false,
            skipped: true,
        }
    }

    fn executed(op_idx: usize, op_name: String, value: f64, state: &ExecutionState) -> Self {
        Self {
            op_idx,
            op_name,
            time: None,
            spot: None,
            barrier: None,
            triggered: None,
            current_value: value,
            is_alive: state.is_alive,
            is_knocked_in: state.is_knocked_in,
            skipped: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use pricer_core::kernel::ScriptKernelBuilder;

    use super::*;

    fn create_flat_provider(spot: f64) -> FlatSpotProvider {
        FlatSpotProvider::new(0.05, 0.03, spot)
    }

    #[test]
    fn test_flat_spot_provider_new() {
        let provider = FlatSpotProvider::new(0.05, 0.03, 100.0);
        assert!((provider.spot - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_flat_spot_provider_discount_factor() {
        let provider = FlatSpotProvider::new(0.05, 0.03, 100.0);

        let df_today = provider.discount_factor(0, 0);
        assert!((df_today - 1.0).abs() < 1e-10);

        let df_1y = provider.discount_factor(0, 365);
        let expected = (-0.05_f64).exp();
        assert!((df_1y - expected).abs() < 1e-6);
    }

    #[test]
    fn test_flat_spot_provider_spot_price() {
        let provider = FlatSpotProvider::new(0.05, 0.03, 100.0);

        let spot_0 = provider.spot_price(1, 0.0);
        assert!((spot_0 - 100.0).abs() < 1e-10);

        let spot_1y = provider.spot_price(1, 1.0);
        let expected = 100.0 * (0.05_f64).exp();
        assert!(
            (spot_1y - expected).abs() < 1e-6,
            "Expected {expected}, got {spot_1y}"
        );
    }

    #[test]
    fn test_flat_spot_provider_current_spot() {
        let provider = FlatSpotProvider::new(0.05, 0.03, 100.0);
        let spot = provider.current_spot(1);
        assert!((spot - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_vanilla_call_itm() {
        let provider = create_flat_provider(110.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let strike_idx = builder.add_constant(100.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: true,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(pv > 14.0, "ITM call should have positive PV, got {pv}");
        assert!(pv < 17.0, "PV should be around 15, got {pv}");
    }

    #[test]
    fn test_vanilla_call_otm() {
        let provider = create_flat_provider(90.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let strike_idx = builder.add_constant(100.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: true,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(pv.abs() < 1e-10, "OTM call should have zero PV, got {pv}");
    }

    #[test]
    fn test_vanilla_put_itm() {
        let provider = create_flat_provider(90.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let strike_idx = builder.add_constant(100.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: false,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(pv > 4.0, "ITM put should have positive PV, got {pv}");
        assert!(pv < 7.0, "PV should be around 5, got {pv}");
    }

    #[test]
    fn test_up_and_out_not_triggered() {
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let barrier_idx = builder.add_constant(120.0);
        let strike_idx = builder.add_constant(95.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::CheckBarrier {
                barrier_idx,
                barrier_type: BarrierType::UpOut,
            })
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: true,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(
            pv > 8.0,
            "Up-and-out not triggered should have positive PV, got {pv}"
        );
    }

    #[test]
    fn test_up_and_out_triggered() {
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let barrier_idx = builder.add_constant(102.0);
        let strike_idx = builder.add_constant(95.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::CheckBarrier {
                barrier_idx,
                barrier_type: BarrierType::UpOut,
            })
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: true,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(
            pv.abs() < 1e-10,
            "Up-and-out triggered should have zero PV, got {pv}"
        );
    }

    #[test]
    fn test_down_and_out_triggered() {
        let provider = create_flat_provider(90.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let barrier_idx = builder.add_constant(95.0);
        let strike_idx = builder.add_constant(100.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::CheckBarrier {
                barrier_idx,
                barrier_type: BarrierType::DownOut,
            })
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: false,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(
            pv.abs() < 1e-10,
            "Down-and-out triggered should have zero PV, got {pv}"
        );
    }

    #[test]
    fn test_up_and_in_triggered() {
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let barrier_idx = builder.add_constant(102.0);
        let strike_idx = builder.add_constant(95.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::CheckBarrier {
                barrier_idx,
                barrier_type: BarrierType::UpIn,
            })
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: true,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(
            pv > 8.0,
            "Up-and-in triggered should have positive PV, got {pv}"
        );
    }

    #[test]
    fn test_up_and_in_not_triggered() {
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let barrier_idx = builder.add_constant(120.0);
        let strike_idx = builder.add_constant(95.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::CheckBarrier {
                barrier_idx,
                barrier_type: BarrierType::UpIn,
            })
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: true,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(
            pv.abs() < 1e-10,
            "Up-and-in not triggered should have zero PV, got {pv}"
        );
    }

    #[test]
    fn test_asian_call_averaging() {
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new()
            .add_observation_time(0.25)
            .add_observation_time(0.5)
            .add_observation_time(0.75)
            .add_observation_time(1.0);

        let strike_idx = builder.add_constant(100.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: true,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(pv > 2.5, "Asian call should have positive PV, got {pv}");
        assert!(pv < 4.0, "Asian PV should be around 3, got {pv}");
    }

    #[test]
    fn test_asian_put_averaging() {
        let provider = create_flat_provider(95.0);

        let mut builder = ScriptKernelBuilder::new()
            .add_observation_time(0.25)
            .add_observation_time(0.5)
            .add_observation_time(0.75)
            .add_observation_time(1.0);

        let strike_idx = builder.add_constant(100.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: false,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(pv > 1.0, "Asian put should have positive PV, got {pv}");
    }

    #[test]
    fn test_store_and_load() {
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let value_idx = builder.add_constant(42.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::CalcFixed {
                amount_idx: value_idx,
            })
            .push_op(ScriptOp::Store { register: 0 })
            .push_op(ScriptOp::CalcFixed {
                amount_idx: notional_idx,
            })
            .push_op(ScriptOp::Load { register: 0 })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(pv > 38.0, "Store/Load should preserve value, got {pv}");
        assert!(pv < 42.0, "PV should be discounted, got {pv}");
    }

    #[test]
    fn test_execution_trace() {
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let barrier_idx = builder.add_constant(120.0);
        let strike_idx = builder.add_constant(95.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::CheckBarrier {
                barrier_idx,
                barrier_type: BarrierType::UpOut,
            })
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: true,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let trace = ScriptEngine::trace(&kernel, &provider);

        assert_eq!(trace.steps.len(), 6);
        assert!(trace.is_alive);
        assert!(!trace.is_knocked_in);
        assert!(trace.steps[0].op_name.contains("Up-and-Out"));
        assert_eq!(trace.steps[0].triggered, Some(false));
    }

    #[test]
    fn test_trace_knocked_out() {
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let barrier_idx = builder.add_constant(102.0);
        let strike_idx = builder.add_constant(95.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::CheckBarrier {
                barrier_idx,
                barrier_type: BarrierType::UpOut,
            })
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: true,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let trace = ScriptEngine::trace(&kernel, &provider);

        assert!(!trace.is_alive);
        assert!(trace.steps[0].triggered == Some(true));

        for step in &trace.steps[1..5] {
            assert!(step.skipped, "Op should be skipped: {:?}", step.op_name);
        }

        assert!(trace.final_pv.abs() < 1e-10);
    }

    #[test]
    fn test_empty_kernel() {
        let provider = create_flat_provider(100.0);
        let kernel = ScriptKernel::empty();

        let pv = ScriptEngine::price(&kernel, &provider);
        assert!(pv.abs() < 1e-10, "Empty kernel should have zero PV");
    }

    #[test]
    fn test_notional_scaling() {
        let provider = create_flat_provider(110.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let strike_idx = builder.add_constant(100.0);
        let notional_idx = builder.add_constant(1_000_000.0);

        let kernel = builder
            .push_op(ScriptOp::Accumulate)
            .push_op(ScriptOp::CalcAverage)
            .push_op(ScriptOp::ApplyPayoff {
                strike_idx,
                is_call: true,
            })
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        assert!(
            pv > 14_000_000.0,
            "PV should be scaled by notional, got {pv}"
        );
        assert!(pv < 16_000_000.0, "PV should be around 15M, got {pv}");
    }
}
