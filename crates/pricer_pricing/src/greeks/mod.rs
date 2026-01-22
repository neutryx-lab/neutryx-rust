//! Greeks calculation types and configuration.
//!
//! This module provides:
//! - [`GreeksResult<T>`]: Generic result type for Greeks calculations
//!   (AD-compatible)
//! - [`GreeksConfig`]: Configuration for bump widths and calculation modes
//! - [`GreeksMode`]: Calculation mode selection (Bump-and-Revalue, AAD,
//!   num-dual)
//! - [`GreeksError`]: Unified error type for all Greeks operations

mod config;
mod error;
mod result;

pub use config::{GreeksConfig, GreeksConfigBuilder, GreeksConfigError, GreeksMode};
pub use error::GreeksError;
pub use result::GreeksResult;

#[cfg(test)]
mod tests;
