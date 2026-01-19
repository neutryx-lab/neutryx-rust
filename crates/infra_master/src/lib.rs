// Clippy configuration for infra_master
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]

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
//! ## Example
//!
//! ```rust
//! use infra_master::{Calendar, CalendarId};
//!
//! let calendar = Calendar::get(CalendarId::Target);
//! assert!(calendar.is_business_day(chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()));
//! ```

mod business_day;
mod calendar;
mod counterparty;
mod currency;
mod date;
mod day_count;
mod direction;
mod error;
mod frequency;
mod period;
mod rate_index;
mod tenor;

pub use business_day::BusinessDayConvention;
pub use calendar::{Calendar, CalendarId};
pub use counterparty::{CsaTerms, NettingSetConfig};
pub use currency::Currency;
pub use date::Date;
pub use day_count::DayCountConvention;
pub use direction::{SwapDirection, TradeDirection};
pub use error::{CurrencyError, DateError, MasterDataError};
pub use frequency::Frequency;
pub use period::Period;
pub use rate_index::RateIndex;
pub use tenor::{EndOfMonthRule, Tenor};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        BusinessDayConvention, Calendar, CalendarId, CsaTerms, Currency, CurrencyError, Date,
        DateError, DayCountConvention, EndOfMonthRule, Frequency, MasterDataError, NettingSetConfig,
        Period, RateIndex, SwapDirection, Tenor, TradeDirection,
    };
}
