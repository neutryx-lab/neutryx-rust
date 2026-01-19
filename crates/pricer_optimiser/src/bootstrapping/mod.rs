//! Yield curve bootstrapping from OIS/Swap rates.
//!
//! This module re-exports types from `pricer_core::market_data::bootstrapping`.
//! The bootstrapping functionality has been moved to pricer_core as part of
//! the P0 market data layer.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pricer_optimiser::bootstrapping::{
//!     SequentialBootstrapper, BootstrapInstrument, GenericBootstrapConfig,
//! };
//!
//! let instruments = vec![
//!     BootstrapInstrument::ois(1.0, 0.03),
//!     BootstrapInstrument::ois(2.0, 0.032),
//! ];
//!
//! let config = GenericBootstrapConfig::default();
//! let bootstrapper = SequentialBootstrapper::new(config);
//! let result = bootstrapper.bootstrap(&instruments).unwrap();
//! ```

// Re-export everything from pricer_core::market_data::bootstrapping
pub use pricer_core::market_data::bootstrapping::*;
