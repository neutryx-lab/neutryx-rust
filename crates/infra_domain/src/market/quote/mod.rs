//! Market quote management.
//!
//! This module provides types for managing market quotes:
//!
//! - [`MarketQuote`]: A single market quote with metadata
//! - [`QuoteId`]: Unique identifier for a market quote
//! - [`MarketQuoteSet`]: Collection of quotes with O(1) lookup
//! - [`QuoteType`]: Quote classification (Bid, Ask, Mid, Last)
//! - [`QuoteValidator`]: Validation trait for quotes

mod error;
mod market_quote;
mod quote_id;
mod quote_set;
mod quote_type;
mod strike_type;
mod validation;
mod vol_quote_type;

pub use error::MarketQuoteError;
pub use market_quote::MarketQuote;
pub use quote_id::QuoteId;
pub use quote_set::MarketQuoteSet;
pub use quote_type::QuoteType;
pub use strike_type::StrikeType;
pub use validation::{QuoteValidator, StandardQuoteValidator};
pub use vol_quote_type::VolQuoteType;
