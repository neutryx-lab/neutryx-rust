// Clippy configuration for infra_master
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]

//! # `infra_master`
//!
//! Static master data (Calendars, Currencies, ISINs) for Neutryx.
//!
//! This crate is the "Source of Truth" for static finance data including:
//! - Holiday calendars (TARGET, NY, JP)
//! - Currency definitions (ISO 4217)
//! - Day Count Convention lookups
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

mod calendar;
mod day_count;
mod error;

pub use calendar::{Calendar, CalendarId};
pub use day_count::DayCountConvention;
pub use error::MasterDataError;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{Calendar, CalendarId, DayCountConvention, MasterDataError};
}
