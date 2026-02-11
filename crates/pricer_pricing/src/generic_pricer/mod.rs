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
pub use config::DefaultCurrency;
pub use config::{ModelConfig, ModelConfigBuilder, PricerConfig, PricerConfigBuilder};
pub use error::{ConfigError, PricingError};
pub use kernel::{
    price_cashflow, price_cashflow_stream, BusinessDayConvention, DayCountConvention,
    DiscountCalculator, Frequency,
};
pub use payoff_evaluator::PayoffEvaluator;
pub use pricer::GenericPricer;
pub use pricer::{
    SimpleCashflow, SimpleLeg, StandaloneCashflowResult, StandaloneLegResult,
    StandalonePricingResult,
};
pub use result::{CashflowPricingResult, LegPricingResult, PathDistribution, PricingResult};
pub use result::{SimpleDate, SimpleDirection};
