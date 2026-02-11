//! Greeks calculation types, configuration, and AD integration.

mod config;
mod error;
mod result;

/// Automatic differentiation for Greeks calculation via Enzyme LLVM-level AD.
#[allow(missing_docs)]
pub mod ad;

pub use config::{GreeksConfig, GreeksConfigBuilder, GreeksConfigError, GreeksMode};
pub use error::GreeksError;
pub use result::GreeksResult;

#[cfg(test)]
mod tests;
