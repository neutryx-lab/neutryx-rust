//! Generic Pricer Engine.

mod batch;
mod config;
mod error;
mod kernel;
mod payoff_evaluator;
mod pricer;
mod result;

pub use batch::{
    BatchPricer, BatchPricingResult, BatchStats, ExecutionStats, PortfolioAggregations,
    PortfolioPricer, PortfolioPricingResult, TradeId,
};
pub use config::{
    DefaultCurrency, ModelConfig, ModelConfigBuilder, PricerConfig, PricerConfigBuilder,
};
pub use error::{ConfigError, PricingError};
pub use kernel::{
    price_cashflow, price_cashflow_stream, BusinessDayConvention, DayCountConvention,
    DiscountCalculator, Frequency,
};
pub use payoff_evaluator::PayoffEvaluator;
pub use pricer::{
    GenericPricer, SimpleCashflow, SimpleLeg, StandaloneCashflowResult, StandaloneLegResult,
    StandalonePricingResult,
};
pub use result::{
    CashflowPricingResult, LegPricingResult, PathDistribution, PricingResult, SimpleDate,
    SimpleDirection,
};
