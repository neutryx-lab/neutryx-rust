//! Market quote management.
//!
//! This module provides types for managing market quotes:
//!
//! - [`MarketRate`]: A single market quote with metadata
//! - [`RateId`]: Unique identifier for a market quote
//! - [`MarketRateSet`]: Collection of quotes with O(1) lookup
//! - [`QuoteType`]: Quote classification (Bid, Ask, Mid, Last)
//! - [`RateValidator`]: Validation trait for quotes
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::quote::{MarketRate, RateId, MarketRateSet, QuoteType};
//! use infra_domain::market::core::{Currency, RateType};
//! use infra_domain::market::source::DataSource;
//! use infra_domain::time::Tenor;
//!
//! let id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
//! let rate = MarketRate::new(
//!     id.clone(),
//!     QuoteType::Mid,
//!     0.05,
//!     1700000000000,
//!     DataSource::Bloomberg,
//! ).unwrap();
//!
//! let mut rate_set = MarketRateSet::new();
//! rate_set.insert(rate);
//! ```

mod error;
mod market_quote;
mod quote_id;
mod quote_set;
mod quote_type;
mod strike_type;
mod validation;
mod vol_quote_type;

// New types (preferred)
pub use error::MarketQuoteError;
// Deprecated aliases (for backward compatibility)
#[allow(deprecated)]
pub use error::MarketRateError;
pub use market_quote::MarketQuote;
#[allow(deprecated)]
pub use market_quote::MarketRate;
pub use quote_id::QuoteId;
#[allow(deprecated)]
pub use quote_id::RateId;
pub use quote_set::MarketQuoteSet;
#[allow(deprecated)]
pub use quote_set::MarketRateSet;
pub use quote_type::QuoteType;
pub use strike_type::StrikeType;
pub use validation::{QuoteValidator, StandardQuoteValidator};
#[allow(deprecated)]
pub use validation::{RateValidator, StandardRateValidator};
pub use vol_quote_type::VolQuoteType;
