//! Event-driven IR for path-dependent products.
//!
//! This module provides `ScriptKernel`, an intermediate representation for
//! exotic products with path-dependency such as:
//! - Barrier options (knock-in/knock-out)
//! - Asian options (averaging)
//! - Autocallables
//!
//! # Architecture
//!
//! Unlike `PricingKernel` which uses a unified cashflow formula, `ScriptKernel`
//! represents computation as a sequence of operations (opcodes) that are
//! executed in order at observation times.
//!
//! ```text
//! ScriptKernel {
//!     observation_times: [0.25, 0.5, 0.75, 1.0],
//!     ops: [CheckBarrier, Accumulate, Accumulate, Accumulate, CalcFloat, Pay],
//!     constants: [barrier_level, strike, ...]
//! }
//! ```
//!
//! # Design Principles
//!
//! - **No Runtime Type Dispatch**: All operations are known at compile time
//! - **Linear Execution**: Operations execute in sequence with no jumps
//! - **Enzyme AD Compatible**: Uses only primitive types and arrays
//!
//! # Example
//!
//! ```
//! use pricer_core::kernel::{ScriptKernel, ScriptOp, BarrierType, ScriptKernelBuilder};
//!
//! // Create a simple barrier option script
//! let mut builder = ScriptKernelBuilder::new();
//! let barrier_idx = builder.add_constant(1.05);  // Barrier level (105%)
//! let strike_idx = builder.add_constant(1.0);    // Strike (100%)
//!
//! let kernel = builder
//!     .add_observation_time(1.0)
//!     .push_op(ScriptOp::CheckBarrier {
//!         barrier_idx,
//!         barrier_type: BarrierType::UpOut,
//!     })
//!     .push_op(ScriptOp::CalcFloat {
//!         index_id: 1,
//!         gearing_idx: barrier_idx,
//!         spread_idx: strike_idx,
//!     })
//!     .push_op(ScriptOp::Pay { ccy_id: 0, dc_id: 0 })
//!     .build()
//!     .expect("Valid kernel");
//! ```

/// Event-driven IR for path-dependent products.
///
/// `ScriptKernel` represents exotic products as a sequence of operations
/// executed at observation times. This representation supports:
///
/// - **Barrier Options**: Using `CheckBarrier` ops for knock-in/knock-out
/// - **Asian Options**: Using `Accumulate` ops for averaging
/// - **Generic Exotics**: Combining ops for complex payoffs
///
/// # Structure
///
/// - `observation_times`: Time points (in years from valuation date) where
///   market observations occur
/// - `ops`: Sequence of operations to execute
/// - `constants`: Constant values used by operations (barriers, strikes, etc.)
///
/// # Execution Model
///
/// The `ScriptEngine` executes operations in sequence, maintaining internal
/// state (accumulated value, barrier status) until a `Pay` operation
/// triggers the final cashflow calculation.
#[derive(Clone, Debug)]
pub struct ScriptKernel {
    /// Observation times (years from valuation date).
    ///
    /// These define when market observations occur (e.g., for averaging
    /// or barrier monitoring). Sorted in ascending order.
    pub observation_times: Vec<f64>,

    /// Operation codes to execute.
    ///
    /// Operations reference constants via indices into the `constants` array.
    pub ops: Vec<ScriptOp>,

    /// Constant operands (barrier levels, strikes, notionals, etc.).
    ///
    /// Indexed by the `*_idx` fields in `ScriptOp` variants.
    pub constants: Vec<f64>,

    /// Number of observation points.
    pub observation_count: usize,

    /// Trade ID for error reporting.
    pub trade_id: String,
}

impl ScriptKernel {
    /// Creates a new script kernel.
    ///
    /// # Arguments
    ///
    /// * `observation_times` - Observation times in years from valuation date
    /// * `ops` - Operations to execute
    /// * `constants` - Constant values for operations
    ///
    /// # Errors
    ///
    /// Returns error if observation_times is empty or ops is empty.
    pub fn new(
        observation_times: Vec<f64>,
        ops: Vec<ScriptOp>,
        constants: Vec<f64>,
    ) -> Result<Self, super::CompileError> {
        if observation_times.is_empty() {
            return Err(super::CompileError::EmptyTrade(
                "ScriptKernel requires at least one observation time".to_string(),
            ));
        }
        if ops.is_empty() {
            return Err(super::CompileError::EmptyTrade(
                "ScriptKernel requires at least one operation".to_string(),
            ));
        }

        Ok(Self {
            observation_count: observation_times.len(),
            observation_times,
            ops,
            constants,
            trade_id: String::new(),
        })
    }

    /// Creates an empty script kernel (for testing).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            observation_times: Vec::new(),
            ops: Vec::new(),
            constants: Vec::new(),
            observation_count: 0,
            trade_id: String::new(),
        }
    }

    /// Sets the trade ID for error reporting.
    #[must_use]
    pub fn with_trade_id(mut self, trade_id: impl Into<String>) -> Self {
        self.trade_id = trade_id.into();
        self
    }

    /// Returns the number of observation times.
    #[must_use]
    pub fn observation_count(&self) -> usize { self.observation_count }

    /// Returns the number of operations.
    #[must_use]
    pub fn op_count(&self) -> usize { self.ops.len() }

    /// Returns true if the kernel is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.ops.is_empty() }

    /// Returns the constant at the given index.
    ///
    /// # Panics
    ///
    /// Panics if index is out of bounds.
    #[inline]
    #[must_use]
    pub fn constant(&self, idx: u16) -> f64 { self.constants[idx as usize] }

    /// Returns the observation time at the given index.
    ///
    /// # Panics
    ///
    /// Panics if index is out of bounds.
    #[inline]
    #[must_use]
    pub fn observation_time(&self, idx: usize) -> f64 { self.observation_times[idx] }

    /// Returns the final observation time (maturity).
    #[must_use]
    pub fn maturity(&self) -> Option<f64> { self.observation_times.last().copied() }

    /// Checks if the kernel contains any barrier operations.
    #[must_use]
    pub fn has_barriers(&self) -> bool {
        self.ops
            .iter()
            .any(|op| matches!(op, ScriptOp::CheckBarrier { .. }))
    }

    /// Checks if the kernel contains accumulation operations (Asian-style).
    #[must_use]
    pub fn has_accumulation(&self) -> bool {
        self.ops.iter().any(|op| matches!(op, ScriptOp::Accumulate))
    }

    /// Checks if the kernel contains target accrual operations.
    #[must_use]
    pub fn has_target_accrual(&self) -> bool {
        self.ops.iter().any(|op| matches!(op, ScriptOp::CheckTarget { .. }))
    }

    /// Checks if the kernel contains early termination operations.
    #[must_use]
    pub fn has_early_termination(&self) -> bool {
        self.ops.iter().any(|op| matches!(op, ScriptOp::EarlyTerminate))
    }

    /// Checks if the kernel contains memory coupon operations.
    #[must_use]
    pub fn has_coupon_memory(&self) -> bool {
        self.ops.iter().any(|op| matches!(op, ScriptOp::CouponMemory { .. }))
    }

    /// Checks if the kernel contains quantity accumulation operations.
    #[must_use]
    pub fn has_quantity_accumulation(&self) -> bool {
        self.ops.iter().any(|op| matches!(op, ScriptOp::AccumulateQuantity { .. }))
    }
}

/// Script operation codes.
///
/// Each operation performs a specific computation during kernel execution.
/// Operations use indices into the `constants` array for their parameters.
///
/// # Design Notes
///
/// - All variants are `Copy` for efficient passing
/// - Index fields are `u16` to keep the enum small (fits in cache line)
/// - No heap allocations in any variant
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptOp {
    /// Fixed amount cashflow.
    ///
    /// Sets the current value to a fixed amount from constants.
    ///
    /// `value = constants[amount_idx]`
    CalcFixed {
        /// Index into constants for the fixed amount.
        amount_idx: u16,
    },

    /// Floating rate cashflow calculation.
    ///
    /// Calculates: `value = (forward_rate × gearing) + spread`
    ///
    /// Uses market data to get the forward rate for `index_id`.
    CalcFloat {
        /// Forward index ID (from IndexMapper).
        index_id: u16,
        /// Index into constants for gearing multiplier.
        gearing_idx: u16,
        /// Index into constants for spread.
        spread_idx: u16,
    },

    /// Barrier level check.
    ///
    /// Checks if the underlying spot crosses the barrier level.
    /// For knock-out: terminates the option if breached.
    /// For knock-in: activates the option if breached.
    CheckBarrier {
        /// Index into constants for barrier level.
        barrier_idx: u16,
        /// Type of barrier (up/down, in/out).
        barrier_type: BarrierType,
    },

    /// Accumulate current observation for averaging.
    ///
    /// Used for Asian options to accumulate spot observations
    /// for later averaging.
    Accumulate,

    /// Calculate average from accumulated observations.
    ///
    /// Finalises the averaging calculation after all `Accumulate` ops.
    CalcAverage,

    /// Apply payoff function to current value.
    ///
    /// Applies the payoff: `max(value - strike, 0)` for call
    /// or `max(strike - value, 0)` for put.
    ApplyPayoff {
        /// Index into constants for strike price.
        strike_idx: u16,
        /// True for call, false for put.
        is_call: bool,
    },

    /// Multiply current value by notional.
    ///
    /// `value = value × constants[notional_idx]`
    ApplyNotional {
        /// Index into constants for notional amount.
        notional_idx: u16,
    },

    /// Emit payment cashflow.
    ///
    /// Discounts the current value to present value and
    /// applies FX conversion if needed.
    Pay {
        /// Currency ID for the payment.
        ccy_id: u8,
        /// Discount curve ID.
        dc_id: u8,
    },

    /// Conditional block end marker.
    ///
    /// Marks the end of a conditional section (e.g., after barrier check).
    EndIf,

    /// Store current value to register.
    ///
    /// Useful for complex payoffs that need intermediate values.
    Store {
        /// Register index (0-7).
        register: u8,
    },

    /// Load value from register.
    ///
    /// Retrieves a previously stored value.
    Load {
        /// Register index (0-7).
        register: u8,
    },

    /// Compare accumulated value against target for TARF-style early termination.
    ///
    /// Checks if the running accumulated sum exceeds (or falls below) a target level.
    /// If the condition is met, the option is terminated (`is_alive = false`).
    CheckTarget {
        /// Index into constants for the target level.
        target_idx: u16,
        /// If true, terminate when accumulated >= target.
        /// If false, terminate when accumulated <= target.
        terminate_above: bool,
    },

    /// Emit an intermediate payment at the current observation time.
    ///
    /// Unlike `Pay` which discounts at maturity, this discounts at the
    /// current observation time. Used for periodic coupon payments and
    /// TARF settlement fixings.
    PayIntermediate {
        /// Currency ID for the payment.
        ccy_id: u8,
        /// Discount curve ID.
        dc_id: u8,
    },

    /// Force early termination of the product.
    ///
    /// Sets `is_alive = false`. Used after an autocall barrier triggers
    /// and the coupon/principal has been paid via `PayIntermediate`.
    EarlyTerminate,

    /// Accumulate a memory coupon for Snowball-style products.
    ///
    /// Adds the coupon amount to a running memory sum. When a barrier
    /// is subsequently triggered, all accumulated memory coupons can
    /// be paid out.
    CouponMemory {
        /// Index into constants for the per-period coupon amount.
        coupon_idx: u16,
    },

    /// Accumulate notional quantity for Accumulator Forward products.
    ///
    /// Tracks accumulated quantity separately from P&L accumulation.
    /// Used for products where quantity (not value) determines settlement.
    AccumulateQuantity {
        /// Index into constants for the quantity per fixing.
        quantity_idx: u16,
    },
}

/// Barrier type for exotic options.
///
/// Defines the direction and effect of barrier breach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, strum::Display)]
pub enum BarrierType {
    /// Up-and-In: Option activates when spot rises above barrier.
    #[strum(serialize = "Up-and-In")]
    UpIn,
    /// Up-and-Out: Option terminates when spot rises above barrier.
    #[strum(serialize = "Up-and-Out")]
    UpOut,
    /// Down-and-In: Option activates when spot falls below barrier.
    #[strum(serialize = "Down-and-In")]
    DownIn,
    /// Down-and-Out: Option terminates when spot falls below barrier.
    #[strum(serialize = "Down-and-Out")]
    DownOut,
}

impl BarrierType {
    /// Returns true if this is an "In" barrier (knock-in).
    #[must_use]
    pub fn is_knock_in(&self) -> bool { matches!(self, BarrierType::UpIn | BarrierType::DownIn) }

    /// Returns true if this is an "Out" barrier (knock-out).
    #[must_use]
    pub fn is_knock_out(&self) -> bool { matches!(self, BarrierType::UpOut | BarrierType::DownOut) }

    /// Returns true if this is an "Up" barrier.
    #[must_use]
    pub fn is_up(&self) -> bool { matches!(self, BarrierType::UpIn | BarrierType::UpOut) }

    /// Returns true if this is a "Down" barrier.
    #[must_use]
    pub fn is_down(&self) -> bool { matches!(self, BarrierType::DownIn | BarrierType::DownOut) }
}

/// Builder for constructing `ScriptKernel` instances.
///
/// # Example
///
/// ```
/// use pricer_core::kernel::{ScriptKernelBuilder, ScriptOp, BarrierType};
///
/// let mut builder = ScriptKernelBuilder::new();
/// let barrier_idx = builder.add_constant(105.0);  // Barrier at 105
///
/// let kernel = builder
///     .trade_id("BARRIER001")
///     .add_observation_time(0.5)
///     .add_observation_time(1.0)
///     .push_op(ScriptOp::CheckBarrier {
///         barrier_idx,
///         barrier_type: BarrierType::UpOut,
///     })
///     .push_op(ScriptOp::Pay { ccy_id: 0, dc_id: 0 })
///     .build()
///     .expect("Valid kernel");
/// ```
#[derive(Debug, Default)]
pub struct ScriptKernelBuilder {
    observation_times: Vec<f64>,
    ops: Vec<ScriptOp>,
    constants: Vec<f64>,
    trade_id: String,
}

impl ScriptKernelBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Sets the trade ID.
    #[must_use]
    pub fn trade_id(mut self, id: impl Into<String>) -> Self {
        self.trade_id = id.into();
        self
    }

    /// Adds an observation time.
    ///
    /// Times should be added in ascending order.
    #[must_use]
    pub fn add_observation_time(mut self, time: f64) -> Self {
        self.observation_times.push(time);
        self
    }

    /// Adds multiple observation times.
    #[must_use]
    pub fn add_observation_times(mut self, times: impl IntoIterator<Item = f64>) -> Self {
        self.observation_times.extend(times);
        self
    }

    /// Adds a constant and returns its index.
    pub fn add_constant(&mut self, value: f64) -> u16 {
        let idx = self.constants.len() as u16;
        self.constants.push(value);
        idx
    }

    /// Pushes an operation.
    #[must_use]
    pub fn push_op(mut self, op: ScriptOp) -> Self {
        self.ops.push(op);
        self
    }

    /// Pushes multiple operations.
    #[must_use]
    pub fn push_ops(mut self, ops: impl IntoIterator<Item = ScriptOp>) -> Self {
        self.ops.extend(ops);
        self
    }

    /// Builds the ScriptKernel.
    ///
    /// # Errors
    ///
    /// Returns error if observation_times or ops is empty.
    pub fn build(mut self) -> Result<ScriptKernel, super::CompileError> {
        // Sort observation times
        self.observation_times
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mut kernel = ScriptKernel::new(self.observation_times, self.ops, self.constants)?;
        kernel.trade_id = self.trade_id;
        Ok(kernel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_kernel_new() {
        let kernel = ScriptKernel::new(
            vec![0.25, 0.5, 0.75, 1.0],
            vec![
                ScriptOp::Accumulate,
                ScriptOp::Pay {
                    ccy_id: 0,
                    dc_id: 0,
                },
            ],
            vec![100.0, 1.05],
        )
        .expect("Valid kernel");

        assert_eq!(kernel.observation_count(), 4);
        assert_eq!(kernel.op_count(), 2);
        assert!(!kernel.is_empty());
    }

    #[test]
    fn test_script_kernel_empty() {
        let kernel = ScriptKernel::empty();
        assert!(kernel.is_empty());
        assert_eq!(kernel.observation_count(), 0);
    }

    #[test]
    fn test_script_kernel_error_no_observations() {
        let result = ScriptKernel::new(
            vec![],
            vec![ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            }],
            vec![],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_script_kernel_error_no_ops() {
        let result = ScriptKernel::new(vec![1.0], vec![], vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_script_kernel_with_trade_id() {
        let kernel = ScriptKernel::new(
            vec![1.0],
            vec![ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            }],
            vec![],
        )
        .expect("Valid kernel")
        .with_trade_id("TEST001");

        assert_eq!(kernel.trade_id, "TEST001");
    }

    #[test]
    fn test_script_kernel_constant_access() {
        let kernel = ScriptKernel::new(
            vec![1.0],
            vec![ScriptOp::CalcFixed { amount_idx: 0 }],
            vec![100.0, 200.0, 300.0],
        )
        .expect("Valid kernel");

        assert!((kernel.constant(0) - 100.0).abs() < 1e-10);
        assert!((kernel.constant(1) - 200.0).abs() < 1e-10);
        assert!((kernel.constant(2) - 300.0).abs() < 1e-10);
    }

    #[test]
    fn test_script_kernel_maturity() {
        let kernel = ScriptKernel::new(
            vec![0.25, 0.5, 0.75, 1.0],
            vec![ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            }],
            vec![],
        )
        .expect("Valid kernel");

        assert!((kernel.maturity().unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_script_kernel_has_barriers() {
        let kernel_with_barrier = ScriptKernel::new(
            vec![1.0],
            vec![
                ScriptOp::CheckBarrier {
                    barrier_idx: 0,
                    barrier_type: BarrierType::UpOut,
                },
                ScriptOp::Pay {
                    ccy_id: 0,
                    dc_id: 0,
                },
            ],
            vec![105.0],
        )
        .expect("Valid kernel");

        let kernel_no_barrier = ScriptKernel::new(
            vec![1.0],
            vec![ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            }],
            vec![],
        )
        .expect("Valid kernel");

        assert!(kernel_with_barrier.has_barriers());
        assert!(!kernel_no_barrier.has_barriers());
    }

    #[test]
    fn test_script_kernel_has_accumulation() {
        let kernel_with_accum = ScriptKernel::new(
            vec![0.25, 0.5, 0.75, 1.0],
            vec![
                ScriptOp::Accumulate,
                ScriptOp::Accumulate,
                ScriptOp::CalcAverage,
                ScriptOp::Pay {
                    ccy_id: 0,
                    dc_id: 0,
                },
            ],
            vec![],
        )
        .expect("Valid kernel");

        assert!(kernel_with_accum.has_accumulation());
    }

    #[test]
    fn test_barrier_type_properties() {
        assert!(BarrierType::UpIn.is_knock_in());
        assert!(BarrierType::DownIn.is_knock_in());
        assert!(BarrierType::UpOut.is_knock_out());
        assert!(BarrierType::DownOut.is_knock_out());

        assert!(BarrierType::UpIn.is_up());
        assert!(BarrierType::UpOut.is_up());
        assert!(BarrierType::DownIn.is_down());
        assert!(BarrierType::DownOut.is_down());
    }

    #[test]
    fn test_barrier_type_display() {
        assert_eq!(format!("{}", BarrierType::UpIn), "Up-and-In");
        assert_eq!(format!("{}", BarrierType::UpOut), "Up-and-Out");
        assert_eq!(format!("{}", BarrierType::DownIn), "Down-and-In");
        assert_eq!(format!("{}", BarrierType::DownOut), "Down-and-Out");
    }

    #[test]
    fn test_script_op_size() {
        // Ensure ScriptOp is reasonably sized for cache efficiency
        assert!(
            std::mem::size_of::<ScriptOp>() <= 8,
            "ScriptOp should fit in 8 bytes or less"
        );
    }

    #[test]
    fn test_script_kernel_builder() {
        let mut builder = ScriptKernelBuilder::new().trade_id("ASIAN001");

        // Add observation times for quarterly averaging
        builder = builder.add_observation_times([0.25, 0.5, 0.75, 1.0]);

        // Add constants
        let strike_idx = builder.add_constant(100.0);
        let notional_idx = builder.add_constant(1_000_000.0);

        // Add operations
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

        assert_eq!(kernel.trade_id, "ASIAN001");
        assert_eq!(kernel.observation_count(), 4);
        assert_eq!(kernel.op_count(), 8);
        assert!(kernel.has_accumulation());
        assert!(!kernel.has_barriers());
    }

    #[test]
    fn test_script_kernel_builder_barrier_option() {
        let mut builder = ScriptKernelBuilder::new().trade_id("BARRIER001");

        // Add single observation at expiry
        builder = builder.add_observation_time(1.0);

        // Add constants
        let barrier_idx = builder.add_constant(1.10); // 110% barrier
        let strike_idx = builder.add_constant(1.0); // ATM strike
        let notional_idx = builder.add_constant(100_000.0);

        let kernel = builder
            .push_op(ScriptOp::CheckBarrier {
                barrier_idx,
                barrier_type: BarrierType::UpOut,
            })
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

        assert!(kernel.has_barriers());
        assert!(!kernel.has_accumulation());
    }

    #[test]
    fn test_script_kernel_clone() {
        let kernel = ScriptKernel::new(
            vec![1.0],
            vec![ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            }],
            vec![100.0],
        )
        .expect("Valid kernel");

        let cloned = kernel.clone();
        assert_eq!(cloned.observation_count(), kernel.observation_count());
        assert_eq!(cloned.op_count(), kernel.op_count());
    }

    #[test]
    fn test_script_kernel_debug() {
        let kernel = ScriptKernel::new(
            vec![1.0],
            vec![ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            }],
            vec![],
        )
        .expect("Valid kernel");

        let debug_str = format!("{:?}", kernel);
        assert!(debug_str.contains("ScriptKernel"));
        assert!(debug_str.contains("observation_times"));
    }

    #[test]
    fn test_script_op_calc_fixed() {
        let op = ScriptOp::CalcFixed { amount_idx: 5 };
        if let ScriptOp::CalcFixed { amount_idx } = op {
            assert_eq!(amount_idx, 5);
        } else {
            panic!("Expected CalcFixed");
        }
    }

    #[test]
    fn test_script_op_calc_float() {
        let op = ScriptOp::CalcFloat {
            index_id: 1,
            gearing_idx: 0,
            spread_idx: 1,
        };
        if let ScriptOp::CalcFloat {
            index_id,
            gearing_idx,
            spread_idx,
        } = op
        {
            assert_eq!(index_id, 1);
            assert_eq!(gearing_idx, 0);
            assert_eq!(spread_idx, 1);
        } else {
            panic!("Expected CalcFloat");
        }
    }

    #[test]
    fn test_script_op_store_load() {
        let store_op = ScriptOp::Store { register: 3 };
        let load_op = ScriptOp::Load { register: 3 };

        if let ScriptOp::Store { register } = store_op {
            assert_eq!(register, 3);
        }
        if let ScriptOp::Load { register } = load_op {
            assert_eq!(register, 3);
        }
    }

    #[test]
    fn test_builder_sorts_observation_times() {
        let kernel = ScriptKernelBuilder::new()
            .add_observation_time(1.0)
            .add_observation_time(0.25)
            .add_observation_time(0.75)
            .add_observation_time(0.5)
            .push_op(ScriptOp::Pay {
                ccy_id: 0,
                dc_id: 0,
            })
            .build()
            .expect("Valid kernel");

        // Should be sorted
        assert!((kernel.observation_time(0) - 0.25).abs() < 1e-10);
        assert!((kernel.observation_time(1) - 0.5).abs() < 1e-10);
        assert!((kernel.observation_time(2) - 0.75).abs() < 1e-10);
        assert!((kernel.observation_time(3) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_has_target_accrual() {
        let kernel = ScriptKernel::new(
            vec![1.0],
            vec![
                ScriptOp::CheckTarget { target_idx: 0, terminate_above: true },
                ScriptOp::Pay { ccy_id: 0, dc_id: 0 },
            ],
            vec![50.0],
        ).expect("Valid kernel");
        assert!(kernel.has_target_accrual());
        assert!(!kernel.has_early_termination());
    }

    #[test]
    fn test_has_early_termination() {
        let kernel = ScriptKernel::new(
            vec![1.0],
            vec![
                ScriptOp::EarlyTerminate,
                ScriptOp::Pay { ccy_id: 0, dc_id: 0 },
            ],
            vec![],
        ).expect("Valid kernel");
        assert!(kernel.has_early_termination());
    }

    #[test]
    fn test_has_coupon_memory() {
        let kernel = ScriptKernel::new(
            vec![1.0],
            vec![
                ScriptOp::CouponMemory { coupon_idx: 0 },
                ScriptOp::Pay { ccy_id: 0, dc_id: 0 },
            ],
            vec![100.0],
        ).expect("Valid kernel");
        assert!(kernel.has_coupon_memory());
    }

    #[test]
    fn test_has_quantity_accumulation() {
        let kernel = ScriptKernel::new(
            vec![1.0],
            vec![
                ScriptOp::AccumulateQuantity { quantity_idx: 0 },
                ScriptOp::Pay { ccy_id: 0, dc_id: 0 },
            ],
            vec![1000.0],
        ).expect("Valid kernel");
        assert!(kernel.has_quantity_accumulation());
    }
}
