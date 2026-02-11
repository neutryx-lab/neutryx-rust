#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::redundant_closure_for_method_calls)]

//! Flat file loaders (CSV/JSON/Parquet) for Neutryx.

mod csa;
mod csv_loader;
mod error;
#[cfg(feature = "fpml")]
pub mod fpml;
#[cfg(feature = "curve-builder")]
mod instrument_parser;
mod json_loader;
mod quote;
mod ticker_loader;
mod vol_surface_loader;

pub use csa::{CsaTerms, NettingSet};
pub use csv_loader::CsvLoader;
pub use error::LoaderError;
#[cfg(feature = "curve-builder")]
pub use instrument_parser::{
    parse_instruments, validate_rate, validate_rates, InstrumentParseError, InstrumentSpec,
};
pub use json_loader::{
    CsaLoader, CurveData, CurvePoint, FxSpotData, JsonLoader, MarketData, MarketLoader,
    TradeLoader, VolPoint, VolSurfaceData,
};
pub use quote::{MarketQuote, QuoteType};
pub use ticker_loader::{TickerMappingEntry, TickerMappingLoader};
pub use vol_surface_loader::{
    parse_expiry_string, parse_fra_tenor, parse_tenor_string, CapFloorVolCsvRow, QuoteTypeJson,
    StrikeValue, SwaptionVolCsvRow, TenorValue, VolQuoteJson, VolQuoteSetJson, VolSurfaceLoader,
};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::{
        CsaLoader, CsaTerms, CsvLoader, JsonLoader, LoaderError, MarketData, MarketLoader,
        MarketQuote, NettingSet, QuoteType, TickerMappingLoader, TradeLoader,
    };
}
