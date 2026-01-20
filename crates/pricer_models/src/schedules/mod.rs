//! Schedule generation for interest rate derivatives.
//!
//! This module re-exports schedules from `pricer_core::trades::schedules`
//! for backward compatibility.
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

// Re-export from pricer_core for backward compatibility
pub use pricer_core::trades::schedules::{
    Frequency, Period, Schedule, ScheduleBuilder, ScheduleError,
};
