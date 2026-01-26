//! Pricing kernel module for linear products.
//!
//! This module provides the runtime pricing engine for `PricingKernel` IR:
//!
//! - [`CurveProvider`]: Trait for market data access (discount factors, forward
//!   rates)
//! - [`KernelContext`]: Runtime context holding curve references
//! - [`LinearEngine`]: SIMD-friendly pricing engine for linear products
//!
//! # Design Principles
//!
//! - **Trait-based abstraction**: `CurveProvider` abstracts market data access
//! - **Branchless pricing**: Unified formula works for fixed and floating
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
mod context;
#[cfg(feature = "l1l2-integration")]
mod engine;
#[cfg(feature = "l1l2-integration")]
mod provider;

#[cfg(feature = "l1l2-integration")]
pub use context::KernelContext;
#[cfg(feature = "l1l2-integration")]
pub use engine::{days_to_years, years_to_days, LinearEngine};
#[cfg(feature = "l1l2-integration")]
pub use provider::{CurveProvider, FlatCurveProvider};

// Integration tests: E2E Trade → PricingKernel → Price
#[cfg(all(test, feature = "l1l2-integration"))]
mod integration;
