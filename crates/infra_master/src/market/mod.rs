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
//! # Module Structure
//!
//! - [`core`]: Fundamental types (Currency, CurrencyPair, RateType, CompoundingMethod)
//! - [`quote`]: Market quote management (MarketRate, RateId, MarketRateSet)
//! - [`index`]: Rate index definitions (RateIndex, IndexMetadata)
//! - [`source`]: Data sources and mapping (DataSource, TickerMapping, InstrumentMapper)
//! - [`definition`]: Curve/surface definitions (CurveDefinition, CalibrationModel)
//! - [`convention`]: Market conventions
//! - [`events`]: Economic calendar events
//! - [`instrument`]: Financial instrument definitions
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

// ============================================================================
// Organized Submodules
// ============================================================================

/// Fundamental market types (Currency, CurrencyPair, RateType, CompoundingMethod).
pub mod core;
/// Rate index definitions (RateIndex, IndexMetadata).
pub mod index;
/// Market quote management (MarketRate, RateId, MarketRateSet).
pub mod quote;
/// Data sources and mapping (DataSource, TickerMapping, InstrumentMapper).
pub mod source;

/// Market conventions.
pub mod convention;
/// Market object definitions (curves, vol surfaces, instruments, rate indices).
pub mod definition;
/// Economic calendar and market events.
pub mod events;
/// Standard instrument definitions for all asset classes.
pub mod instrument;

// ============================================================================
// Private modules (not yet migrated)
// ============================================================================

mod event_instrument;
mod market_instrument;
mod registry;

// ============================================================================
// Re-exports for backward compatibility
// ============================================================================

// Core types
pub use core::{CompoundingMethod, Currency, CurrencyPair, RateType};

// Rate index
pub use index::{IndexMetadata, RateIndex};

// Quote types
pub use quote::{MarketRate, MarketRateError, MarketRateSet, QuoteType, RateId};
pub use quote::{RateValidator, StandardRateValidator};
pub use quote::{StrikeType, VolQuoteType};

// Data sources
pub use source::{DataSource, InstrumentMapper, SourcePriority, StandardInstrumentMapper};
pub use source::TickerMapping;

// Event instruments
pub use event_instrument::EventInstrument;

// Market instrument (for curve calibration)
pub use market_instrument::{MarketInstrument, MarketInstrumentError};

// Registry
pub use registry::{DefinitionRegistry, RegistryError};
#[cfg(feature = "serde")]
pub use registry::DefinitionBundle;

// Definition types (re-exported from definition module)
pub use definition::{
    CalibrationMethod, CalibrationModel, CurveDefError, CurveDefinition,
    IndexConventions, InstrumentConventions, InstrumentDefError, InstrumentDefinition,
    InstrumentTemplate, InterpolationMethod, RateIndexDefError, RateIndexDefinition,
    StrikeAxisType,
};
