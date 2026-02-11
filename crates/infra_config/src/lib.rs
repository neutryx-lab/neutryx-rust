#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]

//! System configuration and environment management for Neutryx.

mod app_config;
mod error;
mod pricing_config;
mod risk_config;
mod settings;

pub use app_config::{AppConfig, CurrencyRateIndexMap, DefaultsRegistry, EnumRegistry};
pub use error::ConfigError;
pub use pricing_config::{MonteCarloParams, PricingConfig, PricingMethod, TreeParams, TreeType};
pub use risk_config::{
    BumpSizes, GreekType, GreeksMethod, MarketShift, RiskConfig, ScenarioConfig, SecondOrderMode,
    ShiftType,
};
pub use settings::{
    DatabaseConfig, EngineConfig, GrpcConfig, LoggingConfig, RestConfig, ServiceConfig, Settings,
};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::{
        AppConfig, BumpSizes, ConfigError, CurrencyRateIndexMap, DatabaseConfig, DefaultsRegistry,
        EngineConfig, EnumRegistry, GreekType, GreeksMethod, GrpcConfig, LoggingConfig,
        MarketShift, MonteCarloParams, PricingConfig, PricingMethod, RestConfig, RiskConfig,
        ScenarioConfig, SecondOrderMode, ServiceConfig, Settings, ShiftType, TreeParams, TreeType,
    };
}
