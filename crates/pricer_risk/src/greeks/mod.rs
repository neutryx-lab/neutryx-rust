//! Greeks calculation types and configuration.
//!
//! This module provides:
//! - [`GreeksResult<T>`]: Generic result type for Greeks calculations
//!   (AD-compatible)
//! - [`GreeksConfig`]: Configuration for bump widths and calculation modes
//! - [`GreeksMode`]: Calculation mode selection (Bump-and-Revalue, AAD,
//!   num-dual)
//! - [`GreeksError`]: Unified error type for all Greeks operations
//!
//! # Migration from pricer_pricing
//!
//! This module was migrated from `pricer_pricing::greeks` to
//! `pricer_risk::greeks` in version 0.8.0. The `pricer_pricing::greeks`
//! module is deprecated and re-exports from this module.
//!
//! # Example
//!
//! ```rust
//! use pricer_risk::greeks::{GreeksConfig, GreeksMode, GreeksResult};
//!
//! // Create configuration
//! let config = GreeksConfig::builder()
//!     .mode(GreeksMode::BumpRevalue)
//!     .spot_bump_relative(0.01)
//!     .build()
//!     .unwrap();
//!
//! // Create result
//! let result = GreeksResult::<f64>::new(10.5, 0.05)
//!     .with_delta(0.55)
//!     .with_gamma(0.02);
//! ```

mod config;
mod error;
mod result;

/// Automatic differentiation for Greeks calculation.
///
/// This module provides Enzyme LLVM-level automatic differentiation
/// for high-performance gradient computation.
pub mod ad;

pub use config::{GreeksConfig, GreeksConfigBuilder, GreeksConfigError, GreeksMode};
pub use error::GreeksError;
pub use result::GreeksResult;

#[cfg(test)]
mod tests;
