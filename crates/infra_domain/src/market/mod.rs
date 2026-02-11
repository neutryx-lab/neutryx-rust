//! Market data types and rate infrastructure.

// ============================================================================
// Organized Submodules
// ============================================================================

/// Fundamental market types (Currency, CurrencyPair, RateType,
/// CompoundingMethod).
pub mod core;
/// Index definitions (RateIndex, FxIndex, SwapIndex).
pub mod index;
/// Market quote management (MarketQuote, QuoteId, MarketQuoteSet).
pub mod quote;
/// Data sources and mapping (DataSource, TickerMapping, InstrumentMapper).
pub mod source;

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

// Definition types (re-exported from definition module)
pub use definition::{
    CalibrationMethod, CalibrationModel, CurveDefError, CurveDefinition, IndexConventions,
    InstrumentConventions, InstrumentDefError, InstrumentDefinition, InstrumentTemplate,
    InterpolationMethod, RateIndexDefError, RateIndexDefinition, StrikeAxisType,
};
// Event instruments
pub use event_instrument::EventInstrument;
// Index types
pub use index::{FxFixingSource, FxIndex, FxIndexMetadata, IndexMetadata, RateIndex};
pub use index::{SwapIndex, SwapIndexMetadata};
// Convention module (re-exported from instrument module for backward compatibility)
pub use instrument::convention;
// Market instrument (for curve calibration)
pub use market_instrument::{MarketInstrument, MarketInstrumentError};
// Quote types
pub use quote::{
    MarketQuote, MarketQuoteError, MarketQuoteSet, QuoteId, QuoteType, QuoteValidator,
    StandardQuoteValidator, StrikeType, VolQuoteType,
};
#[cfg(feature = "serde")]
pub use registry::DefinitionBundle;
// Registry
pub use registry::{DefinitionRegistry, RegistryError};
pub use source::TickerMapping;
// Data sources
pub use source::{DataSource, InstrumentMapper, SourcePriority, StandardInstrumentMapper};
