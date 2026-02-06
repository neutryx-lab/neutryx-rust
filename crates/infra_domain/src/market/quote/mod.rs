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
pub use market_quote::MarketQuote;
pub use quote_id::QuoteId;
pub use quote_set::MarketQuoteSet;
pub use quote_type::QuoteType;
pub use strike_type::StrikeType;
pub use validation::{QuoteValidator, StandardQuoteValidator};
pub use vol_quote_type::VolQuoteType;

// Deprecated aliases (for backward compatibility)
#[allow(deprecated)]
pub use error::MarketRateError;
#[allow(deprecated)]
pub use quote_id::RateId;
#[allow(deprecated)]
pub use market_quote::MarketRate;
#[allow(deprecated)]
pub use quote_set::MarketRateSet;
#[allow(deprecated)]
pub use validation::{RateValidator, StandardRateValidator};
