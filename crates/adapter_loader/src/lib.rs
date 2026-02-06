// Clippy configuration for adapter_loader
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::redundant_closure_for_method_calls)]

//! # adapter_loader
//!
//! Flat file loaders (CSV/JSON/Parquet) for Neutryx.
//!
//! This crate handles bulk loading of CSV, JSON, or Parquet files.
//! CSA and netting set types are re-exported from `infra_domain` for
//! backward compatibility.
//!
//! ## Architecture Position
//!
//! Part of the **A**dapter layer in the A-I-P-S architecture.
//! Depends only on `infra_domain` (for master data types).
//!
//! ## Example
//!
//! ```rust,ignore
//! use adapter_loader::{CsvLoader, JsonLoader, TradeLoader, MarketLoader};
//!
//! // CSV loading
//! let records = CsvLoader::load("trades.csv")?;
//!
//! // JSON loading (generic)
//! let config: MyConfig = JsonLoader::load("config.json")?;
//!
//! // Trade data loading
//! let trades = TradeLoader::load_portfolio("trades.json")?;
//!
//! // Market data loading
//! let market = MarketLoader::load("market.json")?;
//! ```

mod csa;
mod csv_loader;
mod error;
#[cfg(feature = "curve-builder")]
mod instrument_parser;
mod json_loader;
mod vol_surface_loader;

pub use csa::{CsaTerms, NettingSet};
pub use csv_loader::CsvLoader;
pub use error::LoaderError;
// Curve builder support (requires curve-builder feature)
#[cfg(feature = "curve-builder")]
pub use instrument_parser::{
    parse_instruments, validate_rate, validate_rates, InstrumentParseError, InstrumentSpec,
};
pub use json_loader::{
    CsaLoader, CurveData, CurvePoint, FxSpotData, JsonLoader, MarketData, MarketLoader,
    TradeLoader, VolPoint, VolSurfaceData,
};
pub use vol_surface_loader::{
    parse_expiry_string, parse_fra_tenor, parse_tenor_string, CapFloorVolCsvRow, QuoteTypeJson,
    StrikeValue, SwaptionVolCsvRow, TenorValue, VolQuoteJson, VolQuoteSetJson, VolSurfaceLoader,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        CsaLoader, CsaTerms, CsvLoader, JsonLoader, LoaderError, MarketData, MarketLoader,
        NettingSet, TradeLoader,
    };
}
