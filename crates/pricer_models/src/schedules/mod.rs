//! Schedule generation for interest rate derivatives.
//!
//! This module provides:
//! - [`Schedule`]: A collection of payment periods for financial instruments
//! - [`Period`]: A single accrual period with start, end, and payment dates
//! - [`Frequency`]: Payment frequency enumeration (Annual, SemiAnnual, etc.)
//! - [`ScheduleBuilder`]: Builder pattern for constructing schedules
//!
//! # Examples
//!
//! ```
//! use pricer_models::schedules::{Schedule, Period, Frequency, ScheduleBuilder};
//! use pricer_core::types::time::{Date, DayCountConvention};
//!
//! // Create a simple schedule using the builder
//! let schedule = ScheduleBuilder::new()
//!     .start(Date::from_ymd(2024, 1, 15).unwrap())
//!     .end(Date::from_ymd(2026, 1, 15).unwrap())
//!     .frequency(Frequency::SemiAnnual)
//!     .day_count(DayCountConvention::ActualActual360)
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(schedule.periods().len(), 4); // 4 semi-annual periods over 2 years
//! ```

mod error;
mod frequency;
mod period;
mod schedule;

pub use error::ScheduleError;
pub use frequency::Frequency;
pub use period::Period;
pub use schedule::{Schedule, ScheduleBuilder};
