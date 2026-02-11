//! Market data types and rate infrastructure.

/// Fundamental market types (Currency, CurrencyPair, RateType,.
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

mod event_instrument;
mod market_instrument;
mod registry;

pub use core::{CompoundingMethod, Currency, CurrencyPair, RateType};

pub use definition::{
    CalibrationMethod, CalibrationModel, CurveDefError, CurveDefinition, IndexConventions,
    InstrumentConventions, InstrumentDefError, InstrumentDefinition, InstrumentTemplate,
    InterpolationMethod, RateIndexDefError, RateIndexDefinition, StrikeAxisType,
};
pub use event_instrument::EventInstrument;
pub use index::{FxFixingSource, FxIndex, FxIndexMetadata, IndexMetadata, RateIndex};
pub use index::{SwapIndex, SwapIndexMetadata};
pub use instrument::convention;
pub use market_instrument::{MarketInstrument, MarketInstrumentError};
pub use quote::{
    MarketQuote, MarketQuoteError, MarketQuoteSet, QuoteId, QuoteType, QuoteValidator,
    StandardQuoteValidator, StrikeType, VolQuoteType,
};
#[cfg(feature = "serde")]
pub use registry::DefinitionBundle;
pub use registry::{DefinitionRegistry, RegistryError};
pub use source::TickerMapping;
pub use source::{DataSource, InstrumentMapper, SourcePriority, StandardInstrumentMapper};
