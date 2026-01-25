//! Greeks calculation types and configuration.
//!
//! **Deprecation Notice**: This module will be deprecated in a future release.
//! Please use [`pricer_risk::greeks`] for new code, which provides the same
//! types with better integration into the risk management layer.
//!
//! This module provides:
//! - [`GreeksResult<T>`]: Generic result type for Greeks calculations
//!   (AD-compatible)
//! - [`GreeksConfig`]: Configuration for bump widths and calculation modes
//! - [`GreeksMode`]: Calculation mode selection (Bump-and-Revalue, AAD,
//!   num-dual)
//! - [`GreeksError`]: Unified error type for all Greeks operations
//!
//! # Migration Guide
//!
//! Replace imports from `pricer_pricing::greeks` with `pricer_risk::greeks`:
//!
//! ```rust,ignore
//! // Old (deprecated)
//! use pricer_pricing::greeks::{GreeksConfig, GreeksResult};
//!
//! // New (recommended)
//! use pricer_risk::greeks::{GreeksConfig, GreeksResult};
//! ```

mod config;
mod error;
mod result;

pub use config::{GreeksConfig, GreeksConfigBuilder, GreeksConfigError, GreeksMode};
pub use error::GreeksError;
pub use result::GreeksResult;

#[cfg(test)]
mod tests;
