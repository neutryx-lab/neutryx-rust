//! Convention module for market conventions.
//!
//! This module provides types for representing market conventions
//! used in standardised financial instruments.
//!
//! # Module Organisation
//!
//! Conventions are organised by asset class:
//!
//! - **rates**: Interest rate conventions (deposits, FRAs, futures, swaps,
//!   bonds, caps/floors, swaptions, inflation, cross-currency)
//! - **fx**: Foreign exchange conventions (spot, options, swaps)
//! - **credit**: Credit conventions (CDS)
//! - **equity**: Equity conventions
//! - **commodity**: Commodity conventions
//!
//! # Example
//!
//! ```rust
//! use infra_domain::market::convention::{SwapConvention, FxConvention, ConventionSet};
//!
//! // Get USD SOFR swap convention
//! let usd_sofr = SwapConvention::usd_sofr();
//!
//! // Get USD standard convention set
//! let usd_conventions = ConventionSet::usd_standard();
//! ```

// =============================================================================
// Convention factory macro
// =============================================================================

/// Generates named factory methods for a convention type.
///
/// Each entry in the table produces a `#[must_use] pub fn name() -> Self`
/// that constructs `Self { field1: val1, field2: val2, ... }`.
///
/// # Syntax
///
/// ```ignore
/// define_convention_factories! {
///     for DepositConvention;
///     /// USD deposit convention (ACT/360, NY, ModFol, T+2).
///     usd => { day_count: DayCounter::Actual360, calendar: CalendarId::NewYork,
///              business_day_convention: BusinessDayConvention::ModifiedFollowing, spot_lag: 2 };
///     /// EUR deposit convention (ACT/360, TARGET, ModFol, T+2).
///     eur => { day_count: DayCounter::Actual360, calendar: CalendarId::Target,
///              business_day_convention: BusinessDayConvention::ModifiedFollowing, spot_lag: 2 };
/// }
/// ```
macro_rules! define_convention_factories {
    (
        for $Type:ident;
        $(
            $(#[$meta:meta])*
            $name:ident => { $($field:ident : $val:expr),* $(,)? };
        )*
    ) => {
        impl $Type {
            $(
                $(#[$meta])*
                #[must_use]
                pub fn $name() -> Self {
                    Self { $($field: $val),* }
                }
            )*
        }
    };
}

// Make macro available within the convention submodules
pub(crate) use define_convention_factories;

// Asset class modules
mod commodity;
mod credit;
mod equity;
mod fx;
mod rates;

// Infrastructure modules
mod convention_set;
mod market_convention;
mod registry;

pub use commodity::{CommodityConvention, DeliveryConvention, PriceQuotation};
pub use convention_set::ConventionSet;
pub use credit::CdsConvention;
// Re-export equity types
pub use equity::{DividendConvention, EquityConvention, EquitySettlementType};
// Re-export FX types
pub use fx::{
    // Option
    CutOffTime,
    DeltaConvention,
    // Spot
    FxConvention,
    FxOptionConvention,
    // Swap
    FxSettlementType,
    FxSwapConvention,
    NearLegType,
    PremiumCurrency,
};
pub use market_convention::MarketConvention;
pub use rates::{
    // Cross-currency
    BasisSpreadLeg,
    // Bond
    BondConvention,
    // Cap/Floor
    CapFloorConvention,
    // Deposit
    DepositConvention,
    // FRA
    FraConvention,
    // Futures
    FuturesConvention,
    // Inflation
    InflationIndex,
    InflationInterpolation,
    InflationSwapConvention,
    // Swaption
    SettlementConvention,
    // Swap
    SwapConvention,
    SwapLegConvention,
    SwaptionConvention,
    XCcyBasisConvention,
    XCcyLegConvention,
};
pub use registry::{ConventionKey, ConventionRegistry, RegistryError};
