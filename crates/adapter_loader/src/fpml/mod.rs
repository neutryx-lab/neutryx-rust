//! FpML 5.x XML parser for Neutryx.
//!
//! Parses FpML trade definitions and converts them directly to
//! `infra_domain::trade::Trade` objects.
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

mod common;
mod error;
mod parser;
mod products;

// Re-export common utilities for advanced usage
pub use common::{
    parse_currency, parse_date, parse_decimal, parse_parties, parse_trade_header, Party,
    TradeHeader,
};
pub use error::FpmlError;
pub use parser::FpmlParser;
