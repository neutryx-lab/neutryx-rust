//! Time management: dates, calendars, day count conventions, and period
//! calculations.

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
