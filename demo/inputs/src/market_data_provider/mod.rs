//! Market data provider simulations.
//!
//! This module provides mock implementations of market data feeds
//! that generate realistic quote data for the Neutryx adapter layer.

mod bloomberg_sim;
mod reuters_sim;
mod synthetic;

pub use bloomberg_sim::BloombergSim;
pub use reuters_sim::ReutersSim;
pub use synthetic::SyntheticGenerator;

use adapter_feeds::MarketQuote;
use async_channel::Receiver;

/// Trait for market data providers
#[async_trait::async_trait]
pub trait MarketDataProvider: Send + Sync {
    /// Start streaming market data
    async fn start(&self) -> Receiver<MarketQuote>;

    /// Stop streaming
    async fn stop(&self);

    /// Get a snapshot of current quotes
    async fn snapshot(&self) -> Vec<MarketQuote>;
}
