//! Core numeric, time, and financial types.
//!
//! This module provides:
//! - `time`: Time utilities (`DayCountConvention`, `time_to_maturity`) for
//!   financial calculations
//! - `currency_pair`: FX rate types for foreign exchange calculations
//! - `error`: Structured error types for pricing, solver, and
//!   calibration operations
//! - `limit`: Limit enum for jump-aware curve interpolation
//!
//! # Note
//!
//! For core financial types (`Date`, `Currency`, `DayCounter`,
//! `BusinessDayConvention`), import directly from `infra_domain`.

pub mod currency_pair;
pub mod error;
pub mod limit;
pub mod time;

// Re-export pricer_core-specific types
#[allow(deprecated)]
pub use currency_pair::CurrencyPair;
pub use currency_pair::{FxPair, FxRate};
pub use error::{CalibrationError, CalibrationErrorKind, PricingError, SolverError};
pub use limit::Limit;
pub use time::{time_to_maturity, time_to_maturity_dates};
