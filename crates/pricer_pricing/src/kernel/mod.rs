//! Pricing kernel module for linear and exotic products.
//!
//! This module provides runtime pricing engines for `PricingKernel` and
//! `ScriptKernel` IRs:
//!
//! - [`CurveProvider`]: Trait for market data access (discount factors, forward
//!   rates)
//! - [`SpotProvider`]: Extended trait for exotic products (spot prices)
//! - [`KernelContext`]: Runtime context holding curve references
//! - [`LinearEngine`]: SIMD-friendly pricing engine for linear products
//! - [`ScriptEngine`]: Sequential execution engine for exotic products
//!
//! # Design Principles
//!
//! - **Trait-based abstraction**: `CurveProvider`/`SpotProvider` abstract
//!   market data
//! - **Branchless pricing**: Linear products use unified formula
//! - **Sequential execution**: Exotic products use opcode dispatch
//! - **SIMD-friendly**: Sequential array access, no data-dependent branching
//!
//! # Example
//!
//! ```ignore
//! use pricer_pricing::kernel::{CurveProvider, KernelContext, LinearEngine};
//! use pricer_core::ir::PricingKernel;
//!
//! // Create market data provider
//! let curves = FlatCurveProvider::new(0.05, 0.03);
//!
//! // Create pricing context
//! let context = KernelContext::new(&curves);
//!
//! // Price the kernel
//! let npv = LinearEngine::price(&kernel, &context);
//! ```

#[cfg(feature = "l1l2-integration")]
mod callable_engine;
#[cfg(feature = "l1l2-integration")]
mod context;
#[cfg(feature = "l1l2-integration")]
mod engine;
#[cfg(feature = "l1l2-integration")]
mod lsmc;
#[cfg(feature = "l1l2-integration")]
mod provider;
#[cfg(feature = "l1l2-integration")]
mod script_engine;

#[cfg(feature = "l1l2-integration")]
pub use callable_engine::{
    BackwardPassResult, CallableEngine, ExerciseDecision, ExerciseState, SimulatedPaths,
};
#[cfg(feature = "l1l2-integration")]
pub use context::KernelContext;
#[cfg(feature = "l1l2-integration")]
pub use engine::{days_to_years, years_to_days, LinearEngine};
#[cfg(feature = "l1l2-integration")]
pub use lsmc::{BasisFunction, LSMCRegressor, RegressionResult};
#[cfg(feature = "l1l2-integration")]
pub use provider::{
    CurveProvider, FlatCurveProvider, IndexedMarketAdapter, IndexedMarketAdapterBuilder,
};
#[cfg(feature = "l1l2-integration")]
pub use script_engine::{ExecutionTrace, FlatSpotProvider, ScriptEngine, SpotProvider, TraceStep};

// Integration tests: E2E Trade → PricingKernel → Price
#[cfg(all(test, feature = "l1l2-integration"))]
mod integration;
