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

// Asset class modules
mod commodity;
mod credit;
mod equity;
mod fx;
mod rates;

// Infrastructure modules
mod convention_set;
#[cfg(feature = "serde")]
mod convention_template;
mod market_convention;
mod registry;

// Re-export rates types
// Re-export commodity types
pub use commodity::{CommodityConvention, DeliveryConvention, PriceQuotation};
// Re-export infrastructure types
pub use convention_set::ConventionSet;
// Template support for bulk convention generation
#[cfg(feature = "serde")]
pub use convention_template::{ConventionBundle, ConventionTemplate, CurrencyDefaults};
// Re-export credit types
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
