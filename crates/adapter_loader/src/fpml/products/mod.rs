//! FpML product-specific parsers.
//!
//! Each sub-module handles parsing for a specific asset class.

pub mod commodity;
pub mod credit;
pub mod equity;
pub mod fx;
pub mod rates;

pub use commodity::parse_commodity_swap;
pub use credit::{parse_credit_default_swap, parse_credit_default_swap_index};
pub use equity::parse_equity_option;
pub use fx::{parse_fx_forward, parse_fx_option, parse_fx_swap};
pub use rates::{parse_cap_floor, parse_swap, parse_swaption};
