//! Convention module for market conventions.

/// Generates named factory methods for a convention type.
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

pub(crate) use define_convention_factories;

mod commodity;
mod credit;
mod equity;
mod fx;
mod rates;

mod convention_set;
mod market_convention;
mod registry;

pub use commodity::{CommodityConvention, DeliveryConvention, PriceQuotation};
pub use convention_set::ConventionSet;
pub use credit::CdsConvention;
pub use equity::{DividendConvention, EquityConvention, EquitySettlementType};
pub use fx::{
    CutOffTime,
    DeltaConvention,
    FxConvention,
    FxOptionConvention,
    FxSettlementType,
    FxSwapConvention,
    NearLegType,
    PremiumCurrency,
};
pub use market_convention::MarketConvention;
pub use rates::{
    BasisSpreadLeg,
    BondConvention,
    CapFloorConvention,
    DepositConvention,
    FraConvention,
    FuturesConvention,
    InflationIndex,
    InflationInterpolation,
    InflationSwapConvention,
    SettlementConvention,
    SwapConvention,
    SwapLegConvention,
    SwaptionConvention,
    XCcyBasisConvention,
    XCcyLegConvention,
};
pub use registry::{ConventionKey, ConventionRegistry, RegistryError};
