#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::if_not_else)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::unnecessary_map_or)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

//! Static master data (Calendars, Currencies, ISINs) for Neutryx.

#[macro_use]
mod macros;

pub mod book;
pub mod counterparty;
pub mod ids;
pub mod market;
pub mod portfolio;
pub mod time;
pub mod trade;

pub mod error;

/// Prelude module for convenient imports.
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
