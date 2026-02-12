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

pub use infra_domain::book::{
    Book, BookBuilder, BookMetadata, BookOwnership, BookType, RegulatoryBookType,
};

pub use infra_domain::counterparty::{CsaTerms, NettingSet};

pub use infra_domain::error::{
    BookError, CurrencyError, DateError, ExposureError, MasterDataError, NettingError,
    PortfolioError, ValidationError, ValidationResult,
};

pub use infra_domain::ids::{BookId, CounterpartyId, IssuerId, PortfolioId, TradeId};

pub use infra_domain::market::{Currency, RateIndex};

pub use infra_domain::trade::{SwapDirection, TradeDirection};
pub use infra_domain::{
    portfolio::{
        PortfolioBookMapping, PortfolioDefinition, PortfolioDefinitionBuilder, PortfolioMetadata,
        PortfolioScope,
    },
    time::{
        AccrualPeriod, BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar, Date,
        DayCounter, EndOfMonthRule, Frequency, JointCalendar, JointCalendarRule, Period, Tenor,
        TimeError, TimeUnit,
    },
};

#[cfg(feature = "full")]
pub use pricer_pricing::{MonteCarloConfig, MonteCarloPricer, PricingResult, UnifiedPricingResult};

#[cfg(feature = "full")]
pub use pricer_risk::{
    AggregatedGreeks, GreeksConfig, GreeksResult, PortfolioRiskResult, RiskEngine,
    RiskEngineConfig, RiskError, RiskResult, XvaCalculator, XvaConfig,
};
