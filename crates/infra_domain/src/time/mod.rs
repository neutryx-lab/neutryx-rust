//! Time management module for financial calculations.
//!
//! This module provides date handling, calendar operations,
//! day count conventions, and period calculations for financial instruments.
//!
//! # Overview
//!
//! The time module is organised into the following submodules:
//!
//! - `error`: Error types for time-related operations
//! - `types`: Date wrapper with Excel serial conversion
//! - `calendars`: Calendar trait and implementations
//! - `day_counters`: Day count conventions
//! - `period`: Period, tenor, and accrual period types
//! - `frequency`: Payment frequency definitions
//!
//! # Examples
//!
//! ```
//! use infra_domain::time::{Date, DayCounter, Period, TimeUnit};
//!
//! // Create a date
//! let date = Date::from_ymd(2024, 6, 15).unwrap();
//!
//! // Calculate year fraction
//! let start = Date::from_ymd(2024, 1, 1).unwrap();
//! let end = Date::from_ymd(2024, 7, 1).unwrap();
//! let yf = DayCounter::Actual365Fixed.year_fraction(start, end);
//! assert!((yf - 0.4986).abs() < 0.001);
//!
//! // Add a period to a date
//! let future = date + Period::months(3);
//! assert_eq!(future, Date::from_ymd(2024, 9, 15).unwrap());
//! ```
//!
//! # Calendar Operations
//!
//! ```
//! use infra_domain::time::{
//!     Calendar, ConcreteCalendar, CalendarId, Date, BusinessDayConvention
//! };
//!
//! let calendar = ConcreteCalendar::get(CalendarId::Target);
//! let date = Date::from_ymd(2026, 1, 5).unwrap(); // Monday
//! assert!(calendar.is_business_day(date));
//!
//! // Adjust for weekends
//! let saturday = Date::from_ymd(2026, 1, 10).unwrap();
//! let adjusted = calendar.adjust(saturday, BusinessDayConvention::Following);
//! assert_eq!(adjusted, Date::from_ymd(2026, 1, 12).unwrap()); // Monday
//! ```
//!
//! # Excel Serial Conversion
//!
//! ```
//! use infra_domain::time::Date;
//!
//! let date = Date::from_ymd(2024, 1, 1).unwrap();
//! let serial = date.to_serial();
//! assert_eq!(serial, 45292);
//!
//! let restored = Date::from_serial(serial).unwrap();
//! assert_eq!(date, restored);
//! ```

mod calendars;
mod day_counters;
mod error;
mod frequency;
mod period;
mod types;

// Primary exports
pub use calendars::{
    BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar, JointCalendar, JointCalendarRule,
};
pub use day_counters::DayCounter;
pub use error::TimeError;
pub use frequency::Frequency;
pub use period::{
    parse_expiry_to_date, parse_fra_tenor, parse_tenor_to_years, AccrualPeriod, EndOfMonthRule,
    Period, Tenor, TimeUnit,
};
pub use types::Date;
