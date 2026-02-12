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
//! - `enzyme-ad` - Enzyme automatic differentiation (requires nightly + LLVM
//!   18)
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

/// System configuration: pricing parameters, risk settings, service config.
#[cfg(feature = "full")]
pub use infra_config as config;
/// Static master data: dates, currencies, calendars, trade definitions.
pub use infra_domain as master;
/// Mathematical foundation: smoothing functions, kernels, numerical traits.
#[cfg(feature = "analytics")]
pub use pricer_core as core;
/// Financial models: yield curves, volatility surfaces, stochastic processes.
#[cfg(feature = "analytics")]
pub use pricer_models as models;
/// Pricing engines: Monte Carlo, binomial trees, analytical formulas.
#[cfg(feature = "full")]
pub use pricer_pricing as pricing;
/// Risk analytics: Greeks, XVA, scenario analysis.
#[cfg(feature = "full")]
pub use pricer_risk as risk;

pub mod prelude;
