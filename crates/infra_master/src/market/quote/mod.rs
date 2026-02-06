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
//! use infra_master::market::quote::{MarketRate, RateId, MarketRateSet, QuoteType};
//! use infra_master::market::core::{Currency, RateType};
//! use infra_master::market::source::DataSource;
//! use infra_master::time::Tenor;
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
mod quote_type;
mod rate;
mod rate_id;
mod rate_set;
mod strike_type;
mod validation;
mod vol_quote_type;

pub use error::MarketRateError;
pub use quote_type::QuoteType;
pub use rate::MarketRate;
pub use rate_id::RateId;
pub use rate_set::MarketRateSet;
pub use strike_type::StrikeType;
pub use validation::{RateValidator, StandardRateValidator};
pub use vol_quote_type::VolQuoteType;
