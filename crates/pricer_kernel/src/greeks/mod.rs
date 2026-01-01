//! Greeks calculation types and configuration.
//!
//! This module provides:
//! - [`GreeksResult<T>`]: Generic result type for Greeks calculations (AD-compatible)
//! - [`GreeksConfig`]: Configuration for bump widths and calculation modes
//! - [`GreeksMode`]: Calculation mode selection (Bump-and-Revalue, AAD, num-dual)

mod config;
mod result;

pub use config::{GreeksConfig, GreeksConfigBuilder, GreeksMode};
pub use result::GreeksResult;

#[cfg(test)]
mod tests;
