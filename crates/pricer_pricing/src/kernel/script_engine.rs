//! Script engine for path-dependent exotic products.
//!
//! This module provides `ScriptEngine` which executes `ScriptKernel` IR
//! for exotic products such as barrier options, Asian options, and
//! autocallables.
//!
//! # Architecture
//!
//! Unlike `LinearEngine` which uses a unified branchless formula,
//! `ScriptEngine` executes a sequence of operations maintaining internal state:
//!
//! ```text
//! ScriptEngine {
//!     current_value: f64,      // Working register
//!     accumulated_sum: f64,    // For averaging (Asian)
//!     accumulate_count: u32,   // Number of observations
//!     barrier_active: bool,    // Barrier status (knock-in/out)
//!     registers: [f64; 8],     // General-purpose registers
//! }
//! ```
//!
//! # Design Principles
//!
//! - **Linear Execution**: Operations execute in sequence with no jumps
//! - **Enum Dispatch**: All operations are known at compile time
//! - **No Runtime Type Dispatch**: Uses match on `ScriptOp` variants
//! - **Enzyme AD Compatible**: Uses only primitive types

use pricer_core::ir::{BarrierType, ScriptKernel, ScriptOp};

use super::provider::CurveProvider;

/// Trait for providing spot prices for exotic products.
///
/// Extends `CurveProvider` with spot price access needed for
/// barrier monitoring and Asian averaging.
pub trait SpotProvider: CurveProvider {
    /// Returns the spot price at a given observation time.
    ///
    /// # Arguments
    ///
    /// * `index_id` - Underlying index ID (equity, FX, commodity)
    /// * `time_years` - Time in years from valuation date
    ///
    /// # Returns
    ///
    /// Spot price at the given time. For analytical pricing, this
    /// typically returns the forward price discounted appropriately.
    fn spot_price(&self, index_id: u16, time_years: f64) -> f64;

    /// Returns the current spot price (at valuation date).
    ///
    /// # Arguments
    ///
    /// * `index_id` - Underlying index ID
    fn current_spot(&self, index_id: u16) -> f64 { self.spot_price(index_id, 0.0) }
}

/// Extension to `FlatCurveProvider` for exotic pricing.
#[derive(Debug, Clone)]
pub struct FlatSpotProvider {
    /// Flat continuously compounded discount rate.
    discount_rate: f64,
    /// Flat forward rate.
    forward_rate_val: f64,
    /// Spot price.
    spot: f64,
    /// Valuation date as days from epoch.
    valuation_date_days: i32,
}

impl FlatSpotProvider {
    /// Creates a new flat spot provider.
    ///
    /// # Arguments
    ///
    /// * `discount_rate` - Continuous compound discount rate
    /// * `forward_rate` - Flat forward rate for floating indices
    /// * `spot` - Spot price for underlying
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
        // Forward price: S × exp(r × t)
        self.spot * (self.discount_rate * time_years).exp()
    }

    fn current_spot(&self, _index_id: u16) -> f64 { self.spot }
}

/// Execution state for script engine.
///
/// Maintains the working registers and status flags during
/// kernel execution.
#[derive(Debug, Clone)]
struct ExecutionState {
    /// Current working value.
    current_value: f64,
    /// Accumulated sum for averaging.
    accumulated_sum: f64,
    /// Number of accumulated observations.
    accumulate_count: u32,
    /// Whether the option is still active (not knocked out).
    is_alive: bool,
    /// Whether a knock-in barrier has been triggered.
    is_knocked_in: bool,
    /// Whether the kernel has knock-in barriers.
    has_knock_in: bool,
    /// General-purpose registers.
    registers: [f64; 8],
    /// Total PV accumulated from Pay operations.
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
    /// Creates a new execution state for a kernel.
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
            // For knock-in options, the option is NOT active until triggered
            is_knocked_in: false,
            ..Default::default()
        }
    }

    /// Returns true if the option should produce a payout.
    #[inline]
    fn should_pay(&self) -> bool {
        // If alive (not knocked out) AND (no knock-in requirement OR knocked-in)
        self.is_alive && (!self.has_knock_in || self.is_knocked_in)
    }
}

/// Script engine for exotic products.
///
/// `ScriptEngine` executes `ScriptKernel` IR by processing operations
/// in sequence, maintaining internal state for:
///
/// - **Barriers**: Tracking knock-in/knock-out status
/// - **Asian averaging**: Accumulating observations
/// - **Register operations**: Store/load intermediate values
///
/// # Example
///
/// ```ignore
/// use pricer_pricing::kernel::{ScriptEngine, FlatSpotProvider};
/// use pricer_core::ir::ScriptKernel;
///
/// let kernel = /* compiled script kernel */;
/// let provider = FlatSpotProvider::new(0.05, 0.03, 100.0);
///
/// let pv = ScriptEngine::price(&kernel, &provider);
/// println!("PV: {}", pv);
/// ```
pub struct ScriptEngine;

impl ScriptEngine {
    /// Prices a `ScriptKernel` and returns the present value.
    ///
    /// # Arguments
    ///
    /// * `kernel` - Compiled script kernel from `ExoticCompiler`
    /// * `provider` - Market data provider with spot prices
    ///
    /// # Returns
    ///
    /// Present value of the exotic product. Returns 0.0 if:
    /// - The option is knocked out
    /// - A knock-in barrier is never triggered
    ///
    /// # Algorithm
    ///
    /// Operations are executed in sequence:
    ///
    /// 1. **CheckBarrier**: Updates `is_alive` or `is_knocked_in`
    /// 2. **Accumulate**: Adds current spot to `accumulated_sum`
    /// 3. **CalcAverage**: Computes average from accumulated values
    /// 4. **ApplyPayoff**: Applies call/put payoff function
    /// 5. **Pay**: Discounts current value and adds to total PV
    pub fn price<P: SpotProvider>(kernel: &ScriptKernel, provider: &P) -> f64 {
        if kernel.is_empty() {
            return 0.0;
        }

        let mut state = ExecutionState::new(kernel);
        let mut obs_idx = 0_usize;

        for op in &kernel.ops {
            // If knocked out, skip all further operations except Pay
            // (Pay will check should_pay and add 0)
            if !state.is_alive && !matches!(op, ScriptOp::Pay { .. }) {
                continue;
            }

            // Get current observation time if needed
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
                    // For index_id, use 1 as default (could be extended)
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

                    // Move to next observation after barrier check
                    obs_idx += 1;
                }

                ScriptOp::Accumulate => {
                    // For index_id, use 1 as default
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
                        // Get maturity time for discounting
                        let maturity = kernel.maturity().unwrap_or(0.0);
                        let maturity_days =
                            (maturity * 365.0) as i32 + provider.valuation_date_days();
                        let df = provider.discount_factor(dc_id, maturity_days);
                        state.total_pv += state.current_value * df;
                    }
                }

                ScriptOp::EndIf => {
                    // No-op for linear execution
                }

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
    ///
    /// Useful for debugging and understanding the execution flow.
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

                _ => {
                    // Handle other ops minimally for trace
                    TraceStep::executed(op_idx, format!("{op:?}"), state.current_value, &state)
                }
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
    use pricer_core::ir::ScriptKernelBuilder;

    use super::*;

    fn create_flat_provider(spot: f64) -> FlatSpotProvider {
        FlatSpotProvider::new(0.05, 0.03, spot)
    }

    // =========================================================================
    // FlatSpotProvider Tests
    // =========================================================================

    #[test]
    fn test_flat_spot_provider_new() {
        let provider = FlatSpotProvider::new(0.05, 0.03, 100.0);
        assert!((provider.spot - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_flat_spot_provider_discount_factor() {
        let provider = FlatSpotProvider::new(0.05, 0.03, 100.0);

        // Today: DF = 1.0
        let df_today = provider.discount_factor(0, 0);
        assert!((df_today - 1.0).abs() < 1e-10);

        // 1 year: DF = exp(-0.05)
        let df_1y = provider.discount_factor(0, 365);
        let expected = (-0.05_f64).exp();
        assert!((df_1y - expected).abs() < 1e-6);
    }

    #[test]
    fn test_flat_spot_provider_spot_price() {
        let provider = FlatSpotProvider::new(0.05, 0.03, 100.0);

        // At t=0: spot = 100
        let spot_0 = provider.spot_price(1, 0.0);
        assert!((spot_0 - 100.0).abs() < 1e-10);

        // At t=1: forward = S * exp(r * t) = 100 * exp(0.05) ≈ 105.13
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

    // =========================================================================
    // Vanilla Option Tests
    // =========================================================================

    #[test]
    fn test_vanilla_call_itm() {
        // In-the-money call: spot=110, strike=100
        let provider = create_flat_provider(110.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let strike_idx = builder.add_constant(100.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::Accumulate) // Get spot at expiry
            .push_op(ScriptOp::CalcAverage) // current_value = spot
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

        // Forward at T=1: 110 * exp(0.05) ≈ 115.67
        // Payoff: max(115.67 - 100, 0) = 15.67
        // PV: 15.67 * exp(-0.05) ≈ 14.90
        assert!(pv > 14.0, "ITM call should have positive PV, got {pv}");
        assert!(pv < 17.0, "PV should be around 15, got {pv}");
    }

    #[test]
    fn test_vanilla_call_otm() {
        // Out-of-the-money call: spot=90, strike=100
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

        // Forward at T=1: 90 * exp(0.05) ≈ 94.62
        // Payoff: max(94.62 - 100, 0) = 0
        assert!(pv.abs() < 1e-10, "OTM call should have zero PV, got {pv}");
    }

    #[test]
    fn test_vanilla_put_itm() {
        // In-the-money put: spot=90, strike=100
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

        // Forward at T=1: 90 * exp(0.05) ≈ 94.62
        // Payoff: max(100 - 94.62, 0) = 5.38
        // PV: 5.38 * exp(-0.05) ≈ 5.12
        assert!(pv > 4.0, "ITM put should have positive PV, got {pv}");
        assert!(pv < 7.0, "PV should be around 5, got {pv}");
    }

    // =========================================================================
    // Barrier Option Tests
    // =========================================================================

    #[test]
    fn test_up_and_out_not_triggered() {
        // Up-and-out call: spot=100, barrier=120, strike=95
        // Spot stays below barrier, so option survives
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

        // Forward at T=1: 100 * exp(0.05) ≈ 105.13 (below barrier 120)
        // Payoff: max(105.13 - 95, 0) = 10.13
        // PV: ~9.64
        assert!(
            pv > 8.0,
            "Up-and-out not triggered should have positive PV, got {pv}"
        );
    }

    #[test]
    fn test_up_and_out_triggered() {
        // Up-and-out call: spot=100, barrier=102, strike=95
        // Forward at T=1 ≈ 105.13 > 102, barrier triggered
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let barrier_idx = builder.add_constant(102.0); // Low barrier
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

        // Barrier triggered at T=1: 105.13 > 102
        // Option is knocked out, PV = 0
        assert!(
            pv.abs() < 1e-10,
            "Up-and-out triggered should have zero PV, got {pv}"
        );
    }

    #[test]
    fn test_down_and_out_triggered() {
        // Down-and-out put: spot=100, barrier=95, strike=100
        let provider = create_flat_provider(90.0); // Spot below barrier

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

        // Forward at T=1: 90 * exp(0.05) ≈ 94.62 < 95
        // Barrier triggered, PV = 0
        assert!(
            pv.abs() < 1e-10,
            "Down-and-out triggered should have zero PV, got {pv}"
        );
    }

    #[test]
    fn test_up_and_in_triggered() {
        // Up-and-in call: spot=100, barrier=102, strike=95
        // Forward > barrier, knock-in triggered
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

        // Forward at T=1: 105.13 > 102, knock-in triggered
        // Payoff: max(105.13 - 95, 0) ≈ 10.13
        assert!(
            pv > 8.0,
            "Up-and-in triggered should have positive PV, got {pv}"
        );
    }

    #[test]
    fn test_up_and_in_not_triggered() {
        // Up-and-in call: spot=100, barrier=120, strike=95
        // Forward < barrier, knock-in NOT triggered
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let barrier_idx = builder.add_constant(120.0); // High barrier
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

        // Forward at T=1: 105.13 < 120, knock-in NOT triggered
        // Option never activates, PV = 0
        assert!(
            pv.abs() < 1e-10,
            "Up-and-in not triggered should have zero PV, got {pv}"
        );
    }

    // =========================================================================
    // Asian Option Tests
    // =========================================================================

    #[test]
    fn test_asian_call_averaging() {
        // Asian call with quarterly observations
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new()
            .add_observation_time(0.25)
            .add_observation_time(0.5)
            .add_observation_time(0.75)
            .add_observation_time(1.0);

        let strike_idx = builder.add_constant(100.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::Accumulate) // t=0.25
            .push_op(ScriptOp::Accumulate) // t=0.5
            .push_op(ScriptOp::Accumulate) // t=0.75
            .push_op(ScriptOp::Accumulate) // t=1.0
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

        // Forwards: 100*exp(0.05*0.25) ≈ 101.26
        //           100*exp(0.05*0.5)  ≈ 102.53
        //           100*exp(0.05*0.75) ≈ 103.82
        //           100*exp(0.05*1.0)  ≈ 105.13
        // Average: (101.26 + 102.53 + 103.82 + 105.13) / 4 ≈ 103.19
        // Payoff: max(103.19 - 100, 0) = 3.19
        // PV: 3.19 * exp(-0.05) ≈ 3.03

        assert!(pv > 2.5, "Asian call should have positive PV, got {pv}");
        assert!(pv < 4.0, "Asian PV should be around 3, got {pv}");
    }

    #[test]
    fn test_asian_put_averaging() {
        // Asian put with quarterly observations
        let provider = create_flat_provider(95.0); // Lower spot

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

        // Average will be less than 100, so put has value
        assert!(pv > 1.0, "Asian put should have positive PV, got {pv}");
    }

    // =========================================================================
    // Register Operation Tests
    // =========================================================================

    #[test]
    fn test_store_and_load() {
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let value_idx = builder.add_constant(42.0);
        let notional_idx = builder.add_constant(1.0);

        let kernel = builder
            .push_op(ScriptOp::CalcFixed { amount_idx: value_idx })
            .push_op(ScriptOp::Store { register: 0 })
            .push_op(ScriptOp::CalcFixed {
                amount_idx: notional_idx,
            }) // Change current value
            .push_op(ScriptOp::Load { register: 0 }) // Restore original
            .push_op(ScriptOp::ApplyNotional { notional_idx })
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        let pv = ScriptEngine::price(&kernel, &provider);

        // Should get 42 * 1 * DF ≈ 42 * 0.9512 ≈ 40
        assert!(pv > 38.0, "Store/Load should preserve value, got {pv}");
        assert!(pv < 42.0, "PV should be discounted, got {pv}");
    }

    // =========================================================================
    // Trace Tests
    // =========================================================================

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

        // Should have 6 steps
        assert_eq!(trace.steps.len(), 6);

        // Barrier not triggered
        assert!(trace.is_alive);
        assert!(!trace.is_knocked_in);

        // First step is barrier check
        assert!(trace.steps[0].op_name.contains("Up-and-Out"));
        assert_eq!(trace.steps[0].triggered, Some(false));
    }

    #[test]
    fn test_trace_knocked_out() {
        let provider = create_flat_provider(100.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let barrier_idx = builder.add_constant(102.0); // Low barrier
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

        // Barrier triggered
        assert!(!trace.is_alive);
        assert!(trace.steps[0].triggered == Some(true));

        // Remaining ops should be skipped
        for step in &trace.steps[1..5] {
            assert!(step.skipped, "Op should be skipped: {:?}", step.op_name);
        }

        // Final PV should be 0
        assert!(trace.final_pv.abs() < 1e-10);
    }

    // =========================================================================
    // Empty Kernel Tests
    // =========================================================================

    #[test]
    fn test_empty_kernel() {
        let provider = create_flat_provider(100.0);
        let kernel = ScriptKernel::empty();

        let pv = ScriptEngine::price(&kernel, &provider);
        assert!(pv.abs() < 1e-10, "Empty kernel should have zero PV");
    }

    // =========================================================================
    // Notional Tests
    // =========================================================================

    #[test]
    fn test_notional_scaling() {
        let provider = create_flat_provider(110.0);

        let mut builder = ScriptKernelBuilder::new().add_observation_time(1.0);
        let strike_idx = builder.add_constant(100.0);
        let notional_idx = builder.add_constant(1_000_000.0); // 1M notional

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

        // Forward at T=1: 110 * exp(0.05) ≈ 115.67
        // Payoff: max(115.67 - 100, 0) = 15.67
        // Scaled: 15.67 * 1M = 15.67M
        // PV: 15.67M * exp(-0.05) ≈ 14.9M

        assert!(
            pv > 14_000_000.0,
            "PV should be scaled by notional, got {pv}"
        );
        assert!(pv < 16_000_000.0, "PV should be around 15M, got {pv}");
    }
}
