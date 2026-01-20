//! Core numeric, time, and financial types.
//!
//! This module provides:
//! - `dual`: Dual number type integration with num-dual for automatic
//!   differentiation (when `num-dual-mode` feature is enabled)
//! - `time`: Time types (Date, DayCountConvention, BusinessDayConvention) for
//!   financial calculations
//! - `currency`: ISO 4217 currency codes with metadata
//! - `currency_pair`: Currency pair types for FX calculations
//! - `error`: Structured error types for pricing, date, currency,
//!   interpolation, solver, and calibration operations
//!
//! # Re-exports
//!
//! For convenience, commonly used types are re-exported at this module level:
//! - [`Date`], [`DayCountConvention`], [`BusinessDayConvention`],
//!   [`time_to_maturity`], [`time_to_maturity_dates`] from `time`
//! - [`Currency`] from `currency`
//! - [`CurrencyPair`] from `currency_pair`
//! - [`PricingError`], [`DateError`], [`CurrencyError`],
//!   [`InterpolationError`], [`SolverError`], [`CalibrationError`],
//!   [`CalibrationErrorKind`] from `error`
//!
//! # Migration Notice
//!
//! The following types are now re-exported from `infra_master`:
//! - `Currency` - Use `infra_master::Currency` directly
//! - `Date` - Use `infra_master::Date` directly
//! - `DayCountConvention` - Use `infra_master::DayCountConvention` directly
//! - `BusinessDayConvention` - Use `infra_master::BusinessDayConvention`
//!   directly
//! - `DateError` - Use `infra_master::DateError` directly
//! - `CurrencyError` - Use `infra_master::CurrencyError` for simple cases

pub mod currency_pair;
#[cfg(feature = "num-dual-mode")]
pub mod dual;
pub mod error;
pub mod time;

// Re-export from infra_master (authoritative source)
pub use currency_pair::CurrencyPair;
pub use error::{
    CalibrationError, CalibrationErrorKind, CurrencyError, DateError, InterpolationError,
    PricingError, SolverError,
};
#[deprecated(
    since = "0.2.0",
    note = "Use infra_master::BusinessDayConvention directly"
)]
pub use infra_master::BusinessDayConvention;
#[deprecated(since = "0.2.0", note = "Use infra_master::Currency directly")]
pub use infra_master::Currency;
// Re-export time types from infra_master
#[deprecated(since = "0.2.0", note = "Use infra_master::Date directly")]
pub use infra_master::Date;
#[deprecated(
    since = "0.2.0",
    note = "Use infra_master::DayCountConvention directly"
)]
pub use infra_master::DayCountConvention;
pub use time::{time_to_maturity, time_to_maturity_dates};
