//! Trade module for financial instrument representation.
//!
//! This module provides types for representing financial trades as
//! cashflow-expanded structures suitable for pricing.
//!
//! # Architecture
//!
//! ```text
//! Trade (共通フォーマット - Pricing への入力)
//! ├── id: TradeId
//! ├── legs: Vec<Leg>
//! │   └── Leg
//! │       ├── cashflows: Vec<Cashflow>
//! │       │   └── Cashflow
//! │       │       ├── payoff: Payoff
//! │       │       ├── payment_date: Date
//! │       │       └── ...
//! │       ├── direction: Direction
//! │       └── leg_type: LegType
//! ├── trade_type: TradeType
//! └── metadata: TradeMetadata
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use infra_domain::trade::{Trade, Direction, LegType, TradeType};
//!
//! let trade = Trade::builder()
//!     .id("SWAP001")
//!     .legs(vec![fixed_leg, float_leg])
//!     .trade_type(TradeType::Swap)
//!     .build();
//! ```

mod book_assignment;
mod builder;
mod cashflow;
mod direction;
mod error;
mod index;
mod index_requirement;
mod leg;
mod payoff;
mod pricing_instrument;
mod trade;

// Existing calibration instruments (for backward compatibility)
mod instrument;

/// Standard instrument definitions for all asset classes.
///
/// **DEPRECATED**: Use `infra_domain::market::instrument` instead.
/// This module re-exports from `market::instrument` for backward compatibility.
#[deprecated(since = "0.2.0", note = "Use infra_domain::market::instrument instead")]
pub mod instrument_def {
    //! Re-exports from `market::instrument` for backward compatibility.
    pub use crate::market::instrument::*;
}

pub use book_assignment::{BookTransferReason, TradeBookAssignment, TradeBookHistory};
#[allow(deprecated)]
pub use builder::LegBuilder;
pub use builder::{LegConfig, LegConfigBuilder};
pub use cashflow::{Cashflow, CashflowType, DailyAccrual};
pub use direction::{SwapDirection, TradeDirection};
pub use error::TradeError;
pub use index::{IndexObservation, IndexType};
pub use index_requirement::IndexRequirement;
pub use instrument::Instrument;
// Re-export common types from market::instrument
pub use crate::market::instrument::AssetClass;
pub use leg::{Direction, Leg, LegType};
pub use payoff::{OptionType, Payoff};
pub use pricing_instrument::{
    ExerciseStyle, Forward, ForwardDirection, FxOptionType, InstrumentParams, PayoffType,
    PricingInstrument, VanillaOption,
};
pub use trade::{
    BarrierType, ExerciseType, ProtectionSide, SettlementType, Trade, TradeBuilder, TradeMetadata,
    TradeType,
};

pub use crate::ids::TradeId;
