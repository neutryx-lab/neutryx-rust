//! Market data types and definitions.
//!
//! This module provides market-related types including currencies
//! and benchmark rate indices.
//!
//! # Overview
//!
//! - [`Currency`]: ISO 4217 currency codes with decimal precision
//! - [`RateIndex`]: Benchmark interest rate indices (SOFR, EURIBOR, etc.)
//!
//! # Examples
//!
//! ```
//! use infra_master::market::{Currency, RateIndex};
//!
//! let usd = Currency::USD;
//! assert_eq!(usd.code(), "USD");
//!
//! let sofr = RateIndex::Sofr;
//! assert_eq!(sofr.currency(), Currency::USD);
//! ```

mod currency;
mod rate_index;

pub use currency::Currency;
pub use rate_index::RateIndex;
