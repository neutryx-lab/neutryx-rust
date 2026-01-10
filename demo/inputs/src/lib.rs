//! # Upstream Systems
//!
//! Mock upstream systems that provide input data to the Neutryx Adapter layer.
//!
//! This crate simulates external market data providers, trade booking systems,
//! and file generators that would typically feed into the A-I-P-S architecture.
//!
//! ## Modules
//!
//! - [`market_data_provider`]: Simulates market data feeds (Reuters, Bloomberg style)
//! - [`trade_source`]: Simulates front office trade booking
//! - [`file_source`]: Generates CSV/Parquet files for batch processing

pub mod file_source;
pub mod market_data_provider;
pub mod trade_source;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::file_source::{CsvGenerator, FileGenerator};
    pub use crate::market_data_provider::{BloombergSim, MarketDataProvider, ReutersSim, SyntheticGenerator};
    pub use crate::trade_source::{FpmlGenerator, FrontOffice, TradeSource};
}
