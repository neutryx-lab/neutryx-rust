//! FpML 5.x XML parser for Neutryx.

mod common;
mod error;
mod parser;
mod products;

pub use common::{
    build_metadata, extract_nested_amount, extract_nested_date, parse_currency, parse_date,
    parse_decimal, parse_parties, parse_trade_header, Party, TradeHeader,
};
pub use error::FpmlError;
pub use parser::FpmlParser;
