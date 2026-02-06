//! Core market data types.
//!
//! This module provides fundamental types used throughout the market data
//! infrastructure:
//!
//! - [`Currency`]: ISO 4217 currency codes with decimal precision
//! - [`CurrencyPair`]: FX currency pair representation
//! - [`CompoundingMethod`]: Interest rate compounding conventions
//! - [`RateType`]: Classification of market rate types (Deposit, Swap, FX,
//!   etc.)
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::core::{Currency, CurrencyPair, CompoundingMethod, RateType};
//!
//! let usd = Currency::USD;
//! assert_eq!(usd.code(), "USD");
//!
//! let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
//! assert_eq!(pair.to_string(), "EUR/USD");
//!
//! let method = CompoundingMethod::Compounded;
//! assert_eq!(method.name(), "Compounded");
//! ```

mod compounding;
mod currency;
mod currency_pair;
mod rate_type;

pub use compounding::CompoundingMethod;
pub use currency::Currency;
pub use currency_pair::CurrencyPair;
pub use rate_type::RateType;
