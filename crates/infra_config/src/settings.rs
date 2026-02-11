//! Settings and configuration structures.

use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};

use crate::{error::ConfigError, pricing_config::PricingConfig, risk_config::RiskConfig};

/// Main application settings.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Settings {
    /// Engine configuration (thread pools, memory limits, Monte Carlo).
    #[serde(default)]
    pub engine: EngineConfig,
    /// Database configuration.
    #[serde(default)]
    pub database: DatabaseConfig,
    /// Service-specific configuration (CLI, Gateway).
    #[serde(default)]
    pub service: ServiceConfig,
    /// Logging and output configuration.
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Pricing configuration (optional).
    #[serde(default)]
    pub pricing: Option<PricingConfig>,
    /// Risk/Greeks configuration (optional).
    #[serde(default)]
    pub risk: Option<RiskConfig>,
}

impl Settings {
    /// Load settings from configuration files and environment variables.
    pub fn load() -> Result<Self, ConfigError> {
        let env = std::env::var("NEUTRYX_ENV").unwrap_or_else(|_| "development".into());

        let config = Config::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{env}")).required(false))
            .add_source(Environment::with_prefix("NEUTRYX").separator("__"))
            .build()?;

        let settings: Settings = config.try_deserialize()?;
        Ok(settings)
    }

    /// Load settings from a TOML string.
    pub fn from_toml_str(toml_str: &str) -> Result<Self, ConfigError> {
        toml::from_str(toml_str).map_err(|e| ConfigError::InvalidValue {
            key: "toml".to_string(),
            message: e.to_string(),
        })
    }

    /// Load settings from a JSON string.
    pub fn from_json_str(json: &str) -> Result<Self, ConfigError> {
        serde_json::from_str(json).map_err(|e| ConfigError::InvalidValue {
            key: "json".to_string(),
            message: e.to_string(),
        })
    }

    /// Validates all nested configurations and collects all errors.
    pub fn validate(&self) -> Result<(), Vec<ConfigError>> {
        let errors: Vec<ConfigError> = [
            self.pricing.as_ref().and_then(|p| p.validate().err()),
            self.risk.as_ref().and_then(|r| r.validate().err()),
        ]
        .into_iter()
        .flatten()
        .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Engine configuration.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct EngineConfig {
    /// Thread pool size for parallel computation.
    pub thread_pool_size: usize,
    /// Memory limit for AD engine (in MB).
    pub memory_limit_mb: usize,
    /// Monte Carlo simulation paths (default batch size).
    pub mc_paths: usize,
    /// Checkpointing enabled.
    pub checkpointing_enabled: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            thread_pool_size: num_cpus::get(),
            memory_limit_mb: 1024,
            mc_paths: 10_000,
            checkpointing_enabled: false,
        }
    }
}

/// Database configuration.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Database connection URL.
    pub url: String,
    /// Maximum connection pool size.
    pub max_connections: u32,
    /// Connection timeout (in seconds).
    pub connection_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: 10,
            connection_timeout_secs: 30,
        }
    }
}

/// Service-specific configuration.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct ServiceConfig {
    /// REST API configuration.
    pub rest: RestConfig,
    /// gRPC API configuration.
    pub grpc: GrpcConfig,
}

/// REST API configuration.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct RestConfig {
    /// Enable REST API.
    pub enabled: bool,
    /// REST API bind address.
    pub addr: String,
}

impl Default for RestConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            addr: "0.0.0.0:8080".to_string(),
        }
    }
}

/// gRPC API configuration.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct GrpcConfig {
    /// Enable gRPC API.
    pub enabled: bool,
    /// gRPC bind address.
    pub addr: String,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            addr: "0.0.0.0:50051".to_string(),
        }
    }
}

/// Logging and output configuration.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error).
    pub level: String,
    /// Output directory for reports and results.
    pub output_dir: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            output_dir: "./output".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        pricing_config::PricingMethod,
        risk_config::{GreekType, GreeksMethod},
    };

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert!(settings.engine.thread_pool_size > 0);
        assert_eq!(settings.engine.memory_limit_mb, 1024);
    }

    #[test]
    fn test_settings_from_toml_str() {
        let toml = r#"
            [engine]
            thread_pool_size = 4
            memory_limit_mb = 2048

            [pricing]
            valuation_date = "2026-01-25"
            reporting_currency = "EUR"
            pricing_method = "analytical"
            market_data_path = "data/market.json"
            trade_data_path = "data/trades.json"

            [risk]
            greeks_method = "bump"
            target_greeks = ["delta", "vega", "gamma"]
        "#;

        let settings = Settings::from_toml_str(toml).unwrap();
        assert_eq!(settings.engine.thread_pool_size, 4);
        assert_eq!(settings.engine.memory_limit_mb, 2048);

        let pricing = settings.pricing.unwrap();
        assert_eq!(pricing.reporting_currency, "EUR");
        assert_eq!(pricing.pricing_method, PricingMethod::Analytical);

        let risk = settings.risk.unwrap();
        assert_eq!(risk.greeks_method, GreeksMethod::Bump);
        assert_eq!(risk.target_greeks.len(), 3);
    }

    #[test]
    fn test_settings_from_json_str() {
        let json = r#"{
            "engine": {
                "thread_pool_size": 8
            },
            "pricing": {
                "valuation_date": "2026-06-30",
                "reporting_currency": "JPY",
                "pricing_method": "monte_carlo",
                "monte_carlo": {
                    "num_paths": 50000,
                    "num_steps": 365
                },
                "market_data_path": "data/market.json",
                "trade_data_path": "data/trades.json"
            }
        }"#;

        let settings = Settings::from_json_str(json).unwrap();
        assert_eq!(settings.engine.thread_pool_size, 8);

        let pricing = settings.pricing.unwrap();
        assert_eq!(pricing.reporting_currency, "JPY");
        assert_eq!(pricing.pricing_method, PricingMethod::MonteCarlo);
    }

    #[test]
    fn test_settings_validate_collects_all_errors() {
        let toml = r#"
            [pricing]
            valuation_date = "2026-01-25"
            reporting_currency = "invalid"
            pricing_method = "analytical"
            market_data_path = "data/market.json"
            trade_data_path = "data/trades.json"

            [risk]
            target_greeks = []
        "#;

        let settings = Settings::from_toml_str(toml).unwrap();
        let result = settings.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_settings_without_pricing_and_risk() {
        let toml = r#"
            [engine]
            thread_pool_size = 2
        "#;

        let settings = Settings::from_toml_str(toml).unwrap();
        assert!(settings.pricing.is_none());
        assert!(settings.risk.is_none());
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_settings_valid_full_config() {
        let toml = r#"
            [engine]
            thread_pool_size = 4

            [pricing]
            valuation_date = "2026-01-25"
            reporting_currency = "USD"
            market_data_path = "data/market.json"
            trade_data_path = "data/trades.json"

            [risk]
            greeks_method = "aad"
            target_greeks = ["delta", "gamma"]
        "#;

        let settings = Settings::from_toml_str(toml).unwrap();
        assert!(settings.validate().is_ok());

        let risk = settings.risk.unwrap();
        assert_eq!(risk.greeks_method, GreeksMethod::Aad);
        assert!(risk.target_greeks.contains(&GreekType::Delta));
        assert!(risk.target_greeks.contains(&GreekType::Gamma));
    }
}
