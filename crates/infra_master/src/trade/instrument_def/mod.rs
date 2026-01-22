//! Standard instrument definitions for financial products.
//!
//! This module provides comprehensive definitions for standard financial instruments
//! across all asset classes (Rates, FX, Equity, Credit, Commodity) as used by
//! Tier-1 bank trading desks.
//!
//! # Architecture
//!
//! The instrument definitions follow a hierarchical structure:
//!
//! ```text
//! InstrumentDefinition (enum)
//! ├── Rates: Swaption, CapFloor, Frn, CmsSwap, InflationSwap
//! ├── FX: FxSpot, FxForward, FxVanillaOption, FxBarrierOption, FxSwap
//! ├── Equity: EquityForward, EquityVanillaOption, AsianOption, ...
//! ├── Credit: Cds, CdsIndex, CdsOption, NtdBasket
//! └── Commodity: CommodityForward, CommoditySwap, ...
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use infra_master::trade::instrument::{InstrumentDefinition, AssetClass, Swaption};
//!
//! let swaption = InstrumentDefinition::Swaption(Swaption { ... });
//! assert_eq!(swaption.asset_class(), AssetClass::Rates);
//! assert!(swaption.is_option());
//! ```

mod common;
mod error;

// Asset class specific modules
mod commodity;
mod credit;
mod equity;
mod fx;
mod rates;

// Re-exports
pub use common::{
    AssetClass, BarrierDirection, BarrierType, ExerciseStyle, NotionalSchedule, PayerReceiver,
};
pub use error::InstrumentError;

// Rates instruments
pub use rates::{CapFloor, CapFloorType, CmsSwap, Frn, InflationSwap, SwapType, Swaption};

// FX instruments
pub use fx::{CurrencyPair, FxBarrierOption, FxForward, FxSpot, FxSwap, FxVanillaOption};

// Equity instruments
pub use equity::{
    AsianOption, AveragingType, BasketComponent, BasketOption, EquityBarrierOption,
    EquityForward, EquityReturnType, EquitySwap, EquityUnderlying, EquityVanillaOption,
    LookbackOption, LookbackType, MonitoringFrequency,
};

// Credit instruments
pub use credit::{Cds, CdsIndex, CdsOption, CreditEvent, NtdBasket};

// Commodity instruments
pub use commodity::{
    AgricultureType, CommodityAsianOption, CommodityForward, CommoditySwap, CommodityType,
    CommodityVanillaOption, EnergyType, MetalType, QuantityUnit, SpreadOption,
};
