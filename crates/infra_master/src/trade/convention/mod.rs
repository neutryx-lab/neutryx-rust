//! Convention module for market conventions.
//!
//! This module provides types for representing market conventions
//! used in standardised financial instruments.
//!
//! # Example
//!
//! ```rust
//! use infra_master::trade::convention::{SwapConvention, FxConvention, ConventionSet};
//!
//! // Get USD SOFR swap convention
//! let usd_sofr = SwapConvention::usd_sofr();
//!
//! // Get USD standard convention set
//! let usd_conventions = ConventionSet::usd_standard();
//! ```

mod bond;
mod capfloor;
mod cds;
mod commodity;
mod convention_set;
mod equity;
mod fra;
mod futures;
mod fx;
mod fx_option;
mod inflation;
mod swap;
mod swaption;

pub use bond::BondConvention;
pub use capfloor::CapFloorConvention;
pub use cds::CdsConvention;
pub use commodity::{CommodityConvention, DeliveryConvention, PriceQuotation};
pub use convention_set::ConventionSet;
pub use equity::{DividendConvention, EquityConvention, EquitySettlementType};
pub use fra::FraConvention;
pub use futures::FuturesConvention;
pub use fx::FxConvention;
pub use fx_option::{CutOffTime, DeltaConvention, FxOptionConvention, PremiumCurrency};
pub use inflation::{InflationIndex, InflationInterpolation, InflationSwapConvention};
pub use swap::{SwapConvention, SwapLegConvention};
pub use swaption::{SettlementConvention, SwaptionConvention};
