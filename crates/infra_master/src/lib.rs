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
//! ## Time Module
//!
//! The new [`time`] module provides a reorganised structure for time-related
//! types with additional features:
//!
//! - Excel serial date conversion
//! - Calendar trait with `JointCalendar` support
//! - Generic `Period` with `TimeUnit`
//! - Unified `TimeError` type
//!
//! Legacy types are re-exported at crate root for backward compatibility.
//!
//! ## Example (New API)
//!
//! ```rust
//! use infra_master::time::{ConcreteCalendar, CalendarId, Calendar, Date};
//!
//! let calendar = ConcreteCalendar::get(CalendarId::Target);
//! let date = Date::from_ymd(2026, 1, 5).unwrap();
//! assert!(calendar.is_business_day(date));
//! ```
//!
//! ## Example (Legacy API)
//!
//! The legacy API remains fully supported:
//!
//! ```rust
//! use infra_master::{Calendar, CalendarId};
//!
//! let calendar = Calendar::get(CalendarId::Target);
//! assert!(calendar.is_business_day(chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()));
//! ```

// New time module
pub mod time;

// Trade module (new)
pub mod trade;

// Convention module (new)
pub mod convention;

// CounterParty module (new comprehensive module)
pub mod counterparty;

// Legacy modules (kept for backward compatibility)
mod business_day;
mod calendar;
mod counterparty_legacy;
mod currency;
mod date;
mod day_count;
mod direction;
mod error;
mod frequency;
mod period;
mod rate_index;
mod tenor;

// Legacy re-exports for backward compatibility
pub use business_day::BusinessDayConvention;
pub use calendar::{Calendar, CalendarId};
// Legacy CsaTerms and NettingSetConfig - kept for backward compatibility
pub use counterparty_legacy::{CsaTerms as LegacyCsaTerms, NettingSetConfig};
// Alias the legacy CsaTerms at crate root for compatibility
pub use counterparty_legacy::CsaTerms;
pub use currency::Currency;
pub use date::Date;
pub use day_count::DayCountConvention;
pub use direction::{SwapDirection, TradeDirection};
pub use error::{CurrencyError, DateError, MasterDataError};
pub use frequency::Frequency;
pub use period::Period;
pub use rate_index::RateIndex;
pub use tenor::{EndOfMonthRule, Tenor};

// Re-export new time module types at crate root for convenience
pub use time::{
    AccrualPeriod, ConcreteCalendar, DayCounter, JointCalendar, JointCalendarRule, TimeError,
    TimeUnit,
};
// Note: time::Period is a generic period (length + TimeUnit)
// The legacy Period (period.rs) is an AccrualPeriod (start, end, payment dates)
// Both are available: use time::Period for generic periods, crate::Period for accrual periods

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        BusinessDayConvention, Calendar, CalendarId, CsaTerms, Currency, CurrencyError, Date,
        DateError, DayCountConvention, EndOfMonthRule, Frequency, MasterDataError,
        NettingSetConfig, Period, RateIndex, SwapDirection, Tenor, TradeDirection,
    };

    // Also include new time module types
    pub use crate::time::{
        AccrualPeriod, ConcreteCalendar, DayCounter, JointCalendar, JointCalendarRule, TimeError,
        TimeUnit,
    };
}
