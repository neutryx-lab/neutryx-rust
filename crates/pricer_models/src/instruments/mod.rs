//! Lightweight instrument definitions for analytical pricing models.
//!
//! This module provides minimal type definitions required by the analytical
//! pricing models (Black-Scholes, Garman-Kohlhagen, etc.) without depending
//! on the full infra_master trade infrastructure.
//!
//! For full instrument definitions suitable for trade management, see
//! `infra_master::trade`.

mod fx;
mod vanilla;

pub use fx::FxOptionType;
pub use vanilla::{ExerciseStyle, InstrumentParams, PayoffType, VanillaOption};
