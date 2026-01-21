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
//! use infra_master::trade::{TradeBuilder, Direction, LegType, TradeType};
//!
//! let trade = TradeBuilder::new("SWAP001")
//!     .add_leg(fixed_leg)
//!     .add_leg(float_leg)
//!     .trade_type(TradeType::Swap)
//!     .build();
//! ```

mod builder;
mod cashflow;
mod direction;
mod error;
mod index;
mod instrument;
mod leg;
mod payoff;
mod pricing_instrument;
mod trade;

/// Market conventions for standardised financial instruments.
pub mod convention;

pub use builder::{LegBuilder, TradeBuilder};
pub use cashflow::{Cashflow, CashflowType};
pub use direction::{SwapDirection, TradeDirection};
pub use error::TradeError;
pub use index::{IndexObservation, IndexType};
pub use instrument::Instrument;
pub use leg::{Direction, Leg, LegType};
pub use payoff::{OptionType, Payoff};
pub use pricing_instrument::{
    ExerciseStyle, Forward, ForwardDirection, InstrumentParams, PayoffType, PricingInstrument,
    VanillaOption,
};
pub use trade::{ExerciseType, SettlementType, Trade, TradeId, TradeMetadata, TradeType};
