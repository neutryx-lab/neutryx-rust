//! Core numeric, time, and financial types.
//!
//! This module provides:
//! - `traced`: Execution trace types for computation graph extraction (when
//!   `execution-trace` feature is enabled)
//! - `time`: Time utilities (`DayCountConvention`, `time_to_maturity`) for
//!   financial calculations
//! - `currency_pair`: FX rate types for foreign exchange calculations
//! - `error`: Structured error types for pricing, interpolation, solver, and
//!   calibration operations
//! - `limit`: Limit enum for jump-aware curve interpolation
//!
//! # Note
//!
//! For core financial types (`Date`, `Currency`, `DayCounter`,
//! `BusinessDayConvention`), import directly from `infra_domain`.

pub mod currency_pair;
pub mod error;
pub mod limit;
pub mod time;
#[cfg(feature = "execution-trace")]
pub mod traced;
#[cfg(feature = "execution-trace")]
pub mod traced_export;
#[cfg(feature = "execution-trace")]
pub mod traced_float;

// Re-export pricer_core-specific types
#[allow(deprecated)]
pub use currency_pair::CurrencyPair;
pub use currency_pair::{FxPair, FxRate};
pub use error::{
    CalibrationError, CalibrationErrorKind, InterpolationError, PricingError, SolverError,
};
pub use limit::Limit;
pub use time::{time_to_maturity, time_to_maturity_dates};
// Re-export execution trace types when feature is enabled
#[cfg(feature = "execution-trace")]
pub use traced::{
    clear_trace_context, get_trace_context, set_trace_context, DetailLevel, ExecutionTrace, NodeId,
    Operation, Scope, ScopeGuard, ScopeId, SourceLocation, TraceEdge, TraceNode,
};
#[cfg(feature = "execution-trace")]
pub use traced_export::{
    export_graph, D3Edge, D3Graph, D3Metadata, D3Node, D3NodeGroup, D3NodeType,
};
#[cfg(feature = "execution-trace")]
pub use traced_float::TracedFloat;
