//! Convention module for market conventions.
//!
//! # Deprecation Notice
//!
//! This module has been moved to `infra_master::market::convention`.
//! Please update your imports to use the new location.
//!
//! # Migration
//!
//! Replace:
//! ```ignore
//! use infra_master::trade::convention::*;
//! ```
//!
//! With:
//! ```ignore
//! use infra_master::market::convention::*;
//! ```
//!
//! # Example
//!
//! ```rust
//! // New recommended import path:
//! use infra_master::market::convention::{SwapConvention, FxConvention, ConventionSet};
//!
//! // Get USD SOFR swap convention
//! let usd_sofr = SwapConvention::usd_sofr();
//!
//! // Get USD standard convention set
//! let usd_conventions = ConventionSet::usd_standard();
//! ```

// Re-export all types from market::convention with deprecation warnings.
// This maintains backward compatibility while encouraging migration to the new location.

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::BondConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::CapFloorConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::CdsConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::CommodityConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::ConventionSet;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::CutOffTime;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::DepositConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::DeltaConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::DeliveryConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::DividendConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::EquityConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::EquitySettlementType;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::FraConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::FuturesConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::FxConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::FxOptionConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::InflationIndex;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::InflationInterpolation;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::InflationSwapConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::PremiumCurrency;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::PriceQuotation;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::SettlementConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::SwapConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::SwapLegConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::SwaptionConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::BasisSpreadLeg;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::XCcyBasisConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::XCcyLegConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::FxSettlementType;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::FxSwapConvention;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::NearLegType;

#[deprecated(
    since = "0.8.0",
    note = "Moved to infra_master::market::convention. Please update your imports."
)]
pub use crate::market::convention::MarketConvention;
