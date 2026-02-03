// Clippy configuration for adapter_fpml
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]

//! # adapter_fpml
//!
//! FpML 5.x XML parser for Neutryx.
//!
//! This crate parses FpML trade definitions and converts them directly to
//! `infra_master::trade::Trade` objects.
//!
//! ## Architecture Position
//!
//! Part of the **A**dapter layer in the A-I-P-S architecture.
//! Depends on `infra_master` for trade types and identifiers.
//!
//! ## Supported Products
//!
//! | Asset Class | FpML Element | Description |
//! |-------------|--------------|-------------|
//! | Rates | `<swap>` | Interest Rate Swap (IRS, OIS) |
//! | Rates | `<swaption>` | Swaption |
//! | Rates | `<capFloor>` | Cap/Floor |
//! | FX | `<fxSingleLeg>` | FX Spot/Forward |
//! | FX | `<fxSwap>` | FX Swap |
//! | FX | `<fxOption>` | FX Option |
//! | Equity | `<equityOption>` | Equity Option |
//! | Credit | `<creditDefaultSwap>` | CDS (single name & index) |
//! | Commodity | `<commoditySwap>` | Commodity Swap |
//!
//! ## Example
//!
//! ```rust,ignore
//! use adapter_fpml::FpmlParser;
//!
//! let xml = std::fs::read_to_string("irs_usd_001.xml")?;
//! let trade = FpmlParser::parse(&xml)?;
//!
//! println!("Trade ID: {}", trade.id);
//! println!("Trade Type: {:?}", trade.trade_type);
//! println!("Number of legs: {}", trade.num_legs());
//! ```
//!
//! ## Parsing Multiple Trades
//!
//! ```rust,ignore
//! use adapter_fpml::FpmlParser;
//!
//! let xml = std::fs::read_to_string("portfolio.xml")?;
//! let trades = FpmlParser::parse_multiple(&xml)?;
//!
//! for trade in trades {
//!     println!("Parsed: {}", trade.id);
//! }
//! ```

mod common;
mod error;
mod parser;
mod products;

pub use error::FpmlError;
pub use parser::FpmlParser;

// Re-export common utilities for advanced usage
pub use common::{parse_date, parse_decimal, parse_parties, parse_trade_header, Party, TradeHeader};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::{FpmlError, FpmlParser};
}
