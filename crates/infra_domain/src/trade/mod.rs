//! Trade module for financial instrument representation.

mod book_assignment;
mod builder;
mod cashflow;
mod direction;
mod error;
mod event_leg;
mod fixing;
mod index;
mod index_requirement;
mod leg;
mod payoff;
mod pricing_instrument;
mod sub_schedule;
mod trade;

mod instrument;

/// Standard instrument definitions for all asset classes.
#[deprecated(since = "0.2.0", note = "Use infra_domain::market::instrument instead")]
pub mod instrument_def {
    pub use crate::market::instrument::*;
}

pub use book_assignment::{BookTransferReason, TradeBookAssignment};
pub use builder::{LegConfig, LegConfigBuilder};
pub use cashflow::{Cashflow, CashflowType, CompoundType};
pub use direction::{SwapDirection, TradeDirection};
pub use error::TradeError;
pub use event_leg::{
    AccumSide, BarrierEvent, BarrierEventType, BarrierSpec, EventKind, EventLeg, ExerciseEvent,
    MonitoringType,
};
pub use fixing::{Fixing, FixingView};
pub use index::{BondIndexSubtype, IndexObservation, IndexType};
pub use index_requirement::IndexRequirement;
pub use instrument::Instrument;
pub use leg::{Direction, Leg, LegType};
pub use payoff::{OptionType, Payoff};
pub use pricing_instrument::{
    ExerciseStyle, Forward, ForwardDirection, FxOptionType, InstrumentParams, PayoffType,
    PricingInstrument, VanillaOption,
};
pub use sub_schedule::SubSchedule;
pub use trade::{
    ExerciseType, ProtectionSide, SettlementType, Trade, TradeBuilder, TradeMetadata, TradeType,
};

pub use crate::{ids::TradeId, market::instrument::AssetClass};
