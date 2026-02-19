//! Unified pricing module: the [`Pricer`] entry point, calculation settings,
//! error types, result structures, and payoff evaluation.

pub mod calc_setting;
mod config;
mod error;
mod payoff_evaluator;
mod pricer;
mod result;

pub use calc_setting::{CalcSetting, MonteCarloSetting, PricingMethodHint, TreeSetting};
pub use config::GreeksMode;
pub use error::{ConfigError, PricingError};
pub use payoff_evaluator::PayoffEvaluator;
pub use pricer::Pricer;
pub use result::{
    CashflowPricingResult, LegPricingResult, PathDistribution, PricingResult, SimpleDate,
    SimpleDirection,
};
