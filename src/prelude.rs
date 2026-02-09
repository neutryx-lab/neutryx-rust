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

// =============================================================================
// Time & Calendar Types (always available)
// =============================================================================

// =============================================================================
// Configuration Types (full feature)
// =============================================================================
#[cfg(feature = "full")]
pub use infra_config::{
    BumpSizes, GreekType, GreeksMethod, MonteCarloParams, PricingConfig, PricingMethod, RiskConfig,
    ScenarioConfig, ShiftType,
};
// =============================================================================
// ID Types (always available)
// =============================================================================
pub use infra_domain::ids::{BookId, CounterpartyId, IssuerId, PortfolioId, TradeId};
// =============================================================================
// Market Types (always available)
// =============================================================================
pub use infra_domain::market::{Currency, RateIndex};
pub use infra_domain::time::{
    AccrualPeriod, BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar, Date, DayCounter,
    EndOfMonthRule, Frequency, JointCalendar, JointCalendarRule, Period, Tenor, TimeError,
    TimeUnit,
};
// =============================================================================
// Trade Types (always available)
// =============================================================================
pub use infra_domain::trade::{SwapDirection, TradeDirection};
// =============================================================================
// Book & Portfolio Types (always available)
// =============================================================================
pub use infra_domain::book::{
    Book, BookBuilder, BookMetadata, BookOwnership, BookType, RegulatoryBookType,
};
pub use infra_domain::portfolio::{
    PortfolioBookMapping, PortfolioDefinition, PortfolioDefinitionBuilder, PortfolioMetadata,
    PortfolioScope,
};
// =============================================================================
// Error Types (always available)
// =============================================================================
pub use infra_domain::error::{
    BookError, CurrencyError, DateError, ExposureError, MasterDataError, NettingError,
    PortfolioError, ValidationError, ValidationResult,
};
// =============================================================================
// Counterparty Types (always available)
// =============================================================================
pub use infra_domain::counterparty::{CsaTerms, NettingSet};
// =============================================================================
// Pricing Result Types (full feature)
// =============================================================================
#[cfg(feature = "full")]
pub use pricer_pricing::{MonteCarloConfig, MonteCarloPricer, PricingResult, UnifiedPricingResult};
// =============================================================================
// Risk Types (full feature)
// =============================================================================
#[cfg(feature = "full")]
pub use pricer_risk::{
    AggregatedGreeks, GreeksConfig, GreeksResult, PortfolioRiskResult, RiskEngine,
    RiskEngineConfig, RiskError, RiskResult, XvaCalculator, XvaConfig,
};
