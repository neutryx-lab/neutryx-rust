// Clippy configuration for infra_master
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::if_not_else)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::unnecessary_map_or)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

//! # `infra_master`
//!
//! Static master data (Calendars, Currencies, ISINs) for Neutryx.
//!
//! This crate is the "Source of Truth" for static finance data including:
//! - Holiday calendars (TARGET, NY, JP)
//! - Day Count Convention lookups
//! - Counterparty and CSA (Credit Support Annex) master data
//! - Netting set configurations
//!
//! ## Architecture Position
//!
//! Part of the **I**nfra layer in the A-I-P-S architecture.
//! Must not depend on **P**ricer or **S**ervice crates.
//!
//! ## Module Structure
//!
//! - [`time`]: Date handling, calendars, day count conventions, periods, frequency
//! - [`market`]: Currency definitions, rate indices
//! - [`trade`]: Trade representation, legs, cashflows, directions
//! - [`convention`]: Market conventions for various instruments
//! - [`counterparty`]: Counterparty, CSA, netting set management
//!
//! ## Example
//!
//! ```rust
//! use infra_master::time::{ConcreteCalendar, CalendarId, Calendar, Date, Frequency};
//! use infra_master::market::{Currency, RateIndex};
//!
//! let calendar = ConcreteCalendar::get(CalendarId::Target);
//! let date = Date::from_ymd(2026, 1, 5).unwrap();
//! assert!(calendar.is_business_day(date));
//!
//! let freq = Frequency::Quarterly;
//! assert_eq!(freq.periods_per_year(), 4);
//!
//! let usd = Currency::USD;
//! assert_eq!(usd.code(), "USD");
//! ```

// Core modules
pub mod convention;
pub mod counterparty;
pub mod market;
pub mod time;
pub mod trade;

// Error types
mod error;
pub use error::{CurrencyError, DateError, MasterDataError};

// Counterparty module types (re-exported for convenience)
pub use counterparty::{CsaTerms, NettingSet};

// Re-export commonly used types at crate root for convenience
// Time module types
pub use time::{
    AccrualPeriod, BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar, Date,
    DayCounter, EndOfMonthRule, Frequency, JointCalendar, JointCalendarRule, Period, Tenor,
    TimeError, TimeUnit,
};

// Market module types
pub use market::{Currency, RateIndex};

// Trade module types
pub use trade::{SwapDirection, TradeDirection};

// Backward compatibility aliases
#[allow(deprecated)]
#[deprecated(since = "0.4.0", note = "Use time::DayCounter instead")]
pub type DayCountConvention = time::DayCounter;

/// Prelude module for convenient imports
pub mod prelude {
    // Time types
    pub use crate::time::{
        AccrualPeriod, BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar, Date,
        DayCounter, EndOfMonthRule, Frequency, JointCalendar, JointCalendarRule, Period, Tenor,
        TimeError, TimeUnit,
    };

    // Market types
    pub use crate::market::{Currency, RateIndex};

    // Trade types
    pub use crate::trade::{SwapDirection, TradeDirection};

    // Error types
    pub use crate::error::{CurrencyError, DateError, MasterDataError};
}
