// Clippy configuration for infra_domain
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::if_not_else)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::unnecessary_map_or)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

//! # `infra_domain`
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
//! - [`time`]: Date handling, calendars, day count conventions, periods,
//!   frequency
//! - [`market`]: Currency definitions, rate indices
//! - [`trade`]: Trade representation, legs, cashflows, directions, conventions
//! - [`counterparty`]: Counterparty, CSA, netting set management
//!
//! ## Example
//!
//! ```rust
//! use infra_domain::time::{ConcreteCalendar, CalendarId, Calendar, Date, Frequency};
//! use infra_domain::market::{Currency, RateIndex};
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
pub mod book;
pub mod counterparty;
pub mod ids;
pub mod market;
pub mod portfolio;
pub mod time;
pub mod trade;

// Error types
pub mod error;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        book::{Book, BookBuilder, BookMetadata, BookOwnership, BookType, RegulatoryBookType},
        error::{
            BookError, CurrencyError, DateError, ExposureError, MasterDataError, NettingError,
            PortfolioError, ValidationError, ValidationResult,
        },
        ids::{BookId, CounterpartyId, IssuerId, PortfolioId, TradeId},
        market::{Currency, RateIndex},
        portfolio::{
            PortfolioBookMapping, PortfolioDefinition, PortfolioDefinitionBuilder,
            PortfolioMetadata, PortfolioScope,
        },
        time::{
            AccrualPeriod, BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar, Date,
            DayCounter, EndOfMonthRule, Frequency, JointCalendar, JointCalendarRule, Period, Tenor,
            TimeError, TimeUnit,
        },
        trade::{SwapDirection, TradeDirection},
    };
}
