//! Schedule generation error types.

use pricer_core::types::time::Date;
use thiserror::Error;

/// Errors that can occur during schedule generation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    /// Start date must be before end date.
    #[error("Start date {start} must be before end date {end}")]
    InvalidDateRange {
        /// The start date.
        start: Date,
        /// The end date.
        end: Date,
    },

    /// Missing required field in builder.
    #[error("Missing required field: {field}")]
    MissingField {
        /// The name of the missing field.
        field: &'static str,
    },

    /// Schedule would generate no periods.
    #[error("Schedule would generate no periods between {start} and {end}")]
    NoPeriods {
        /// The start date.
        start: Date,
        /// The end date.
        end: Date,
    },

    /// Invalid frequency for the given date range.
    #[error("Frequency {frequency} is invalid for date range {start} to {end}")]
    InvalidFrequency {
        /// The frequency description.
        frequency: String,
        /// The start date.
        start: Date,
        /// The end date.
        end: Date,
    },

    /// Date arithmetic overflow.
    #[error("Date arithmetic overflow: {reason}")]
    DateOverflow {
        /// Reason for the overflow.
        reason: String,
    },
}
