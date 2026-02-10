//! Greeks calculation types, configuration, and AD integration.

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
