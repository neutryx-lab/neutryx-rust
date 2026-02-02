//! Market data types and rate infrastructure.
//!
//! This module provides comprehensive market data handling including:
//!
//! - Currency and rate index definitions
//! - Market rate quotes with metadata
//! - Rate validation and bounds checking
//! - Ticker mapping for external data sources
//! - Instrument mapping for curve calibration
//!
//! # Overview
//!
//! ## Core Types
//!
//! - [`Currency`]: ISO 4217 currency codes with decimal precision
//! - [`RateIndex`]: Benchmark interest rate indices (SOFR, EURIBOR, etc.)
//! - [`RateType`]: Categories of market rates (Deposit, Swap, FX, etc.)
//! - [`QuoteType`]: Quote types (Bid, Ask, Mid, Last)
//! - [`DataSource`]: Data provider identification
//!
//! ## Rate Management
//!
//! - [`RateId`]: Unique identifier for a market rate
//! - [`MarketRate`]: A single market rate quote with metadata
//! - [`MarketRateSet`]: Collection of rates with O(1) lookup
//! - [`TickerMapping`]: External ticker to internal ID mapping
//!
//! ## Validation and Mapping
//!
//! - [`RateValidator`]: Trait for rate validation
//! - [`StandardRateValidator`]: Default validation with market bounds
//! - [`InstrumentMapper`]: Trait for rate-to-instrument conversion
//! - [`StandardInstrumentMapper`]: Default instrument mapping
//!
//! # Examples
//!
//! ## Basic Usage
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
//!
//! ## Creating Market Rates
//!
//! ```
//! use infra_master::market::{
//!     MarketRate, RateId, RateType, QuoteType, DataSource, Currency
//! };
//! use infra_master::time::Tenor;
//!
//! let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
//! let rate = MarketRate::new(
//!     rate_id,
//!     QuoteType::Mid,
//!     0.05,
//!     1700000000000,
//!     DataSource::Bloomberg,
//! ).unwrap();
//!
//! assert_eq!(rate.value, 0.05);
//! ```
//!
//! ## Managing Rate Collections
//!
//! ```
//! use infra_master::market::{
//!     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
//! };
//! use infra_master::time::Tenor;
//!
//! let mut rate_set = MarketRateSet::new();
//!
//! let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
//! let rate = MarketRate::new(rate_id.clone(), QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
//!
//! rate_set.insert(rate);
//! assert!(rate_set.get_rate(&rate_id, QuoteType::Mid).is_some());
//! ```
//!
//! ## Converting to Instruments
//!
//! ```
//! use infra_master::market::{
//!     MarketRateSet, MarketRate, RateId, RateType, QuoteType,
//!     DataSource, Currency, StandardInstrumentMapper
//! };
//! use infra_master::time::{Date, Tenor};
//!
//! let mut rate_set = MarketRateSet::new();
//! let rate_id = RateId::new(Currency::USD, Tenor::FiveYears, RateType::Swap);
//! let rate = MarketRate::new(rate_id, QuoteType::Mid, 0.045, 1700000000000, DataSource::Bloomberg).unwrap();
//! rate_set.insert(rate);
//!
//! let mapper = StandardInstrumentMapper::new();
//! let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
//!
//! let instruments = rate_set.to_instruments_lossy(&mapper, valuation_date);
//! assert_eq!(instruments.len(), 1);
//! ```

mod compounding;
mod currency;
mod data_source;
mod error;
mod event_instrument;
mod mapper;
mod quote_type;
mod rate;
mod rate_id;
mod rate_index;
mod rate_set;
mod rate_type;
mod ticker;
mod validation;

// Submodules for extended market data types
pub mod convention;
pub mod events;
pub mod instrument;
pub mod volatility;

// Compounding methods
pub use compounding::CompoundingMethod;
// Core types
pub use currency::Currency;
// Quote and rate types
pub use data_source::{DataSource, SourcePriority};
// Error types
pub use error::MarketRateError;
// Instrument mapping
pub use mapper::{InstrumentMapper, StandardInstrumentMapper};
pub use quote_type::QuoteType;
// Market rate and collections
pub use rate::MarketRate;
// Rate identification and mapping
pub use rate_id::RateId;
pub use rate_index::{IndexMetadata, RateIndex};
pub use rate_set::MarketRateSet;
pub use rate_type::RateType;
pub use ticker::TickerMapping;
// Validation
pub use validation::{RateValidator, StandardRateValidator};
// Event instruments
pub use event_instrument::EventInstrument;
