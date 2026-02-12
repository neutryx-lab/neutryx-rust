//! Convenient imports for common Neutryx types.
//!
//! This prelude re-exports the most frequently used types, allowing users to
//! get started quickly with a single import:
//!
//! ```rust,ignore
//! use neutryx::prelude::*;
//!
//! let date = Date::from_ymd(2024, 1, 15).unwrap();
//! let calendar = ConcreteCalendar::get(CalendarId::Target);
//! let usd = Currency::USD;
//! ```

#[cfg(feature = "full")]
pub use infra_config::{
    BumpSizes, GreekType, GreeksMethod, MonteCarloParams, PricingConfig, PricingMethod, RiskConfig,
    ScenarioConfig, ShiftType,
};
pub use infra_domain::{
    book::{Book, BookBuilder, BookMetadata, BookOwnership, BookType, RegulatoryBookType},
    counterparty::{CsaTerms, NettingSet},
    error::{
        BookError, CurrencyError, DateError, ExposureError, MasterDataError, NettingError,
        PortfolioError, ValidationError, ValidationResult,
    },
    ids::{BookId, CounterpartyId, IssuerId, PortfolioId, TradeId},
    market::{Currency, RateIndex},
    portfolio::{
        PortfolioBookMapping, PortfolioDefinition, PortfolioDefinitionBuilder, PortfolioMetadata,
        PortfolioScope,
    },
    time::{
        AccrualPeriod, BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar, Date,
        DayCounter, EndOfMonthRule, Frequency, JointCalendar, JointCalendarRule, Period, Tenor,
        TimeError, TimeUnit,
    },
    trade::{SwapDirection, TradeDirection},
};
#[cfg(feature = "full")]
pub use pricer_pricing::{MonteCarloConfig, MonteCarloPricer, PricingResult, UnifiedPricingResult};
#[cfg(feature = "full")]
pub use pricer_risk::{
    AggregatedGreeks, GreeksConfig, GreeksResult, PortfolioRiskResult, RiskEngine,
    RiskEngineConfig, RiskError, RiskResult, XvaCalculator, XvaConfig,
};
