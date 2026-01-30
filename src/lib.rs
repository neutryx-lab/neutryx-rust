//! # Neutryx - Derivatives Pricing Library
//!
//! Unified facade crate providing access to all Neutryx functionality through
//! a single dependency.
//!
//! ## Quick Start
//!
//! ```toml
//! [dependencies]
//! neutryx = { path = "." }
//! ```
//!
//! ```rust,ignore
//! use neutryx::prelude::*;
//! use neutryx::models::market::YieldCurve;
//!
//! let date = Date::from_ymd(2024, 1, 15).unwrap();
//! let usd = Currency::USD;
//! ```
//!
//! ## Feature Flags
//!
//! - `minimal` - Master data only (dates, currencies, trade definitions)
//! - `analytics` - Curve building, models, analytical pricing
//! - `full` (default) - Complete pricing and risk functionality
//! - `serde` - Serialisation support across all crates
//! - `global-bootstrap` - Multi-dimensional Newton solver for curve calibration
//! - `enzyme-ad` - Enzyme automatic differentiation (requires nightly + LLVM 18)
//!
//! ## Module Structure
//!
//! The facade re-exports underlying crates with intuitive aliases:
//!
//! - [`master`] - Static master data (dates, currencies, calendars, trades)
//! - [`config`] - System configuration (pricing params, risk settings)
//! - [`core`] - Mathematical foundation (smoothing, kernels, traits)
//! - [`models`] - Financial models (curves, volatility, stochastic processes)
//! - [`pricing`] - Pricing engines (Monte Carlo, trees, analytical)
//! - [`risk`] - Risk analytics (Greeks, XVA, scenarios)

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

// =============================================================================
// Infra Layer (I) - Always available
// =============================================================================

/// Static master data: dates, currencies, calendars, trade definitions.
///
/// This module provides the foundational types used throughout Neutryx:
/// - [`Date`](master::Date), [`Calendar`](master::Calendar), [`DayCounter`](master::DayCounter)
/// - [`Currency`](master::Currency), [`RateIndex`](master::RateIndex)
/// - Trade builders and ID types
pub use infra_master as master;

/// System configuration: pricing parameters, risk settings, service config.
#[cfg(feature = "full")]
pub use infra_config as config;

/// Persistence layer: database backends, storage traits.
#[cfg(feature = "storage")]
pub use infra_store as store;

// =============================================================================
// Pricer Layer (P) - Feature-gated
// =============================================================================

/// Mathematical foundation: smoothing functions, kernels, numerical traits.
///
/// Layer 1 of the pricing stack, providing:
/// - Differentiable smoothing functions
/// - Pricing kernels with SIMD support
/// - Numerical traits for generic computation
#[cfg(feature = "analytics")]
pub use pricer_core as core;

/// Financial models: yield curves, volatility surfaces, stochastic processes.
///
/// Layer 2 of the pricing stack, providing:
/// - Yield curve bootstrapping and interpolation
/// - Volatility surface calibration
/// - Stochastic models (GBM, Heston, Hull-White)
#[cfg(feature = "analytics")]
pub use pricer_models as models;

/// Pricing engines: Monte Carlo, binomial trees, analytical formulas.
///
/// Layer 3 of the pricing stack, providing:
/// - Monte Carlo simulation with variance reduction
/// - Tree-based pricing (CRR, Jarrow-Rudd)
/// - Computation graph for automatic differentiation
#[cfg(feature = "full")]
pub use pricer_pricing as pricing;

/// Risk analytics: Greeks, XVA, scenario analysis.
///
/// Layer 4 (application layer) of the pricing stack, providing:
/// - Greeks calculation (Delta, Gamma, Vega, etc.)
/// - XVA metrics (CVA, DVA, FVA)
/// - Scenario engine for stress testing
#[cfg(feature = "full")]
pub use pricer_risk as risk;

// =============================================================================
// Prelude - Convenient imports
// =============================================================================

pub mod prelude;
