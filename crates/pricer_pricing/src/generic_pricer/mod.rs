//! Pricing support types: errors, result structures, and payoff evaluation.

mod config;
mod error;
mod payoff_evaluator;
mod result;

pub use config::GreeksMode;
pub use error::{ConfigError, PricingError};
pub use payoff_evaluator::PayoffEvaluator;
pub use result::{
    CashflowPricingResult, LegPricingResult, PathDistribution, PricingResult, SimpleDate,
    SimpleDirection,
};
