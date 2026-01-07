//! Core numeric, time, and financial types.
//!
//! This module provides:
//! - `dual`: Dual number type integration with num-dual for automatic differentiation (when `num-dual-mode` feature is enabled)
//! - `time`: Time types (Date, DayCountConvention, BusinessDayConvention) for financial calculations
//! - `currency`: ISO 4217 currency codes with metadata
//! - `error`: Structured error types for pricing, date, currency, interpolation, and solver operations
//!
//! # Re-exports
//!
//! For convenience, commonly used types are re-exported at this module level:
//! - [`Date`], [`DayCountConvention`], [`BusinessDayConvention`], [`time_to_maturity`], [`time_to_maturity_dates`] from `time`
//! - [`Currency`] from `currency`
//! - [`PricingError`], [`DateError`], [`CurrencyError`], [`InterpolationError`], [`SolverError`] from `error`

pub mod currency;
#[cfg(feature = "num-dual-mode")]
pub mod dual;
pub mod error;
pub mod time;

// Re-export commonly used types at module level
pub use currency::Currency;
pub use error::{CurrencyError, DateError, InterpolationError, PricingError, SolverError};
pub use time::{
    time_to_maturity, time_to_maturity_dates, BusinessDayConvention, Date, DayCountConvention,
};
