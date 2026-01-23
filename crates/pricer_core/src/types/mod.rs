//! Core numeric, time, and financial types.
//!
//! This module provides:
//! - `dual`: Dual number type integration with num-dual for automatic
//!   differentiation (when `num-dual-mode` feature is enabled)
//! - `time`: Time utilities (`DayCountConvention`, `time_to_maturity`) for
//!   financial calculations
//! - `currency_pair`: Currency pair types for FX calculations
//! - `error`: Structured error types for pricing, interpolation, solver, and
//!   calibration operations
//!
//! # Note
//!
//! For core financial types (`Date`, `Currency`, `DayCounter`,
//! `BusinessDayConvention`), import directly from `infra_master`.

pub mod currency_pair;
#[cfg(feature = "num-dual-mode")]
pub mod dual;
pub mod error;
pub mod time;

// Re-export pricer_core-specific types
pub use currency_pair::CurrencyPair;
pub use error::{
    CalibrationError, CalibrationErrorKind, InterpolationError, PricingError, SolverError,
};
pub use time::{time_to_maturity, time_to_maturity_dates};
