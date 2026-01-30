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
pub use infra_master::ids::{BookId, CounterpartyId, IssuerId, PortfolioId, TradeId};
// =============================================================================
// Market Types (always available)
// =============================================================================
pub use infra_master::market::{Currency, RateIndex};
pub use infra_master::time::{
    AccrualPeriod, BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar, Date, DayCounter,
    EndOfMonthRule, Frequency, JointCalendar, JointCalendarRule, Period, Tenor, TimeError,
    TimeUnit,
};
// =============================================================================
// Trade Types (always available)
// =============================================================================
pub use infra_master::trade::{SwapDirection, TradeDirection};
// =============================================================================
// Book & Portfolio Types (always available)
// =============================================================================
pub use infra_master::{
    Book, BookBuilder, BookMetadata, BookOwnership, BookType, PortfolioBuilder,
    PortfolioDefinition, PortfolioMetadata, PortfolioScope, RegulatoryBookType,
};
// =============================================================================
// Error Types (always available)
// =============================================================================
pub use infra_master::{
    BookError, CurrencyError, DateError, ExposureError, MasterDataError, NettingError,
    PortfolioError, ValidationError, ValidationResult,
};
// =============================================================================
// Counterparty Types (always available)
// =============================================================================
pub use infra_master::{CsaTerms, NettingSet};
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
