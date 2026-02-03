//! Convention module for market conventions.
//!
//! This module provides types for representing market conventions
//! used in standardised financial instruments.
//!
//! # Migration Note
//!
//! This module was moved from `trade/convention/` to `market/convention/`
//! to better reflect the conceptual relationship between market rates and
//! their associated conventions.
//!
//! # Example
//!
//! ```rust
//! use infra_master::market::convention::{SwapConvention, FxConvention, ConventionSet};
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
mod convention_template;
mod deposit;
mod equity;
mod fra;
mod futures;
mod fx;
mod fx_option;
mod fx_swap;
mod inflation;
mod market_convention;
mod registry;
mod swap;
mod swaption;
mod xccy_basis;

pub use bond::BondConvention;
pub use capfloor::CapFloorConvention;
pub use cds::CdsConvention;
pub use commodity::{CommodityConvention, DeliveryConvention, PriceQuotation};
pub use convention_set::ConventionSet;
pub use deposit::DepositConvention;
pub use equity::{DividendConvention, EquityConvention, EquitySettlementType};
pub use fra::FraConvention;
pub use futures::FuturesConvention;
pub use fx::FxConvention;
pub use fx_option::{CutOffTime, DeltaConvention, FxOptionConvention, PremiumCurrency};
pub use fx_swap::{FxSettlementType, FxSwapConvention, NearLegType};
pub use inflation::{InflationIndex, InflationInterpolation, InflationSwapConvention};
pub use market_convention::MarketConvention;
pub use registry::{ConventionKey, ConventionRegistry, RegistryError};
pub use swap::{SwapConvention, SwapLegConvention};
pub use swaption::{SettlementConvention, SwaptionConvention};
pub use xccy_basis::{BasisSpreadLeg, XCcyBasisConvention, XCcyLegConvention};

// Template support for bulk convention generation
pub use convention_template::{ConventionBundle, ConventionTemplate, CurrencyDefaults};
