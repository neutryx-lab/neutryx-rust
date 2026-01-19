//! Trade definitions for financial instruments.
//!
//! This module provides instrument and schedule definitions for pricing
//! with enum dispatch architecture for Enzyme AD compatibility.
//!
//! # Architecture
//!
//! The trades module is part of L1 (pricer_core) and provides:
//! - `instruments`: Financial instrument definitions (options, forwards, swaps)
//! - `schedules`: Payment schedule generation for scheduled instruments
//!
//! # Design Philosophy
//!
//! Uses enum dispatch (NOT trait objects) for static dispatch optimisation:
//! - `Instrument<T>` enum wraps all instrument types
//! - All types generic over `T: Float` for AD compatibility
//! - Smooth payoff approximations via `crate::math::smoothing`
//!
//! # Asset Class Modules
//!
//! Instruments are organized by asset class (enabled via feature flags):
//! - `equity`: Equity derivatives (VanillaOption, Forward) - default
//! - `rates`: Interest rate derivatives (IRS, Swaption, Cap/Floor)
//! - `credit`: Credit derivatives (CDS)
//! - `fx`: FX derivatives (FxOption, FxForward)
//! - `commodity`: Commodity derivatives
//! - `exotic`: Exotic derivatives (VarianceSwap, Cliquet, etc.)
//!
//! # Examples
//!
//! ```
//! use pricer_core::trades::instruments::{
//!     Instrument, VanillaOption, Forward, Swap,
//!     InstrumentParams, PayoffType, ExerciseStyle, Direction, PaymentFrequency,
//! };
//! use pricer_core::types::Currency;
//!
//! // Create a vanilla call option
//! let params = InstrumentParams::new(100.0_f64, 1.0, 1_000_000.0).unwrap();
//! let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
//! let instrument = Instrument::Vanilla(call);
//!
//! // Compute payoff
//! let payoff = instrument.payoff(110.0);
//! assert!(payoff > 0.0);
//! ```

pub mod instruments;
pub mod schedules;

// Re-export commonly used types at trades level
pub use instruments::{
    Direction, ExerciseStyle, Forward, Instrument, InstrumentError, InstrumentParams,
    PaymentFrequency, PayoffType, Swap, VanillaOption,
};

// Asset class sub-enums (feature-gated)
#[cfg(feature = "equity")]
pub use instruments::EquityInstrument;

#[cfg(feature = "rates")]
pub use instruments::RatesInstrument;

#[cfg(feature = "credit")]
pub use instruments::CreditInstrument;

#[cfg(feature = "fx")]
pub use instruments::FxInstrument;

// Hierarchical enum
pub use instruments::{AssetClass, InstrumentEnum};

// Traits
pub use instruments::{Cashflow, CashflowInstrument, InstrumentTrait};

// Schedules (for scheduled instruments like IRS)
pub use schedules::{Frequency, Period, Schedule, ScheduleBuilder, ScheduleError};
