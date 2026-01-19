// Clippy configuration for adapter_feeds
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

//! # adapter_feeds
//!
//! Real-time and snapshot market data parsers for Neutryx.
//!
//! This crate handles connectivity to market data providers (Reuters,
//! Bloomberg, internal lakes) and normalises raw quotes (Bid/Ask, Last) into
//! standardised `MarketQuote` structs.
//!
//! ## Architecture Position
//!
//! Part of the **A**dapter layer in the A-I-P-S architecture.
//! Depends only on `pricer_core` (for types) and `infra_master` (for
//! identifiers).
//!
//! ## Example
//!
//! ```rust,ignore
//! use adapter_feeds::MarketQuote;
//!
//! let quote = MarketQuote::new("AAPL", 150.25, 150.30);
//! ```

mod quote;

pub use quote::{MarketQuote, QuoteType};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{MarketQuote, QuoteType};
}
