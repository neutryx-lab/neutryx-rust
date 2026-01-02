//! REST API server for Neutryx XVA pricing library
//!
//! This crate provides an HTTP REST API for the Neutryx pricing library,
//! exposing option pricing, Greeks calculation, and XVA computation endpoints.

pub mod config;

// Re-export pricer dependencies for integration
pub use pricer_core;
pub use pricer_models;
pub use pricer_xva;

#[cfg(feature = "kernel-integration")]
pub use pricer_kernel;

/// Server version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
