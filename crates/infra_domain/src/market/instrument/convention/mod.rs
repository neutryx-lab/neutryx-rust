//! Convention module for market conventions.
//!
//! This module provides types for representing market conventions
//! used in standardised financial instruments.
//!
//! # Module Organisation
//!
//! Conventions are organised by asset class:
//!
//! - [`rates`]: Interest rate conventions (deposits, FRAs, futures, swaps, bonds, caps/floors, swaptions, inflation, cross-currency)
//! - [`fx`]: Foreign exchange conventions (spot, options, swaps)
//! - [`credit`]: Credit conventions (CDS)
//! - [`equity`]: Equity conventions
//! - [`commodity`]: Commodity conventions
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
mod convention_template;
mod market_convention;
mod registry;

// Re-export rates types
pub use rates::{
    // Deposit
    DepositConvention,
    // FRA
    FraConvention,
    // Futures
    FuturesConvention,
    // Swap
    SwapConvention, SwapLegConvention,
    // Bond
    BondConvention,
    // Cap/Floor
    CapFloorConvention,
    // Swaption
    SettlementConvention, SwaptionConvention,
    // Inflation
    InflationIndex, InflationInterpolation, InflationSwapConvention,
    // Cross-currency
    BasisSpreadLeg, XCcyBasisConvention, XCcyLegConvention,
};

// Re-export FX types
pub use fx::{
    // Spot
    FxConvention,
    // Option
    CutOffTime, DeltaConvention, FxOptionConvention, PremiumCurrency,
    // Swap
    FxSettlementType, FxSwapConvention, NearLegType,
};

// Re-export credit types
pub use credit::CdsConvention;

// Re-export equity types
pub use equity::{DividendConvention, EquityConvention, EquitySettlementType};

// Re-export commodity types
pub use commodity::{CommodityConvention, DeliveryConvention, PriceQuotation};

// Re-export infrastructure types
pub use convention_set::ConventionSet;
pub use market_convention::MarketConvention;
pub use registry::{ConventionKey, ConventionRegistry, RegistryError};

// Template support for bulk convention generation
pub use convention_template::{ConventionBundle, ConventionTemplate, CurrencyDefaults};
