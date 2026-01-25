//! Settings and configuration structures.

use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};

use crate::{error::ConfigError, pricing_config::PricingConfig, risk_config::RiskConfig};

/// Main application settings.
///
/// This is the single source of truth for all Neutryx configuration.
/// Services should use this struct instead of defining their own config types.
///
/// # Example
///
/// ```rust,ignore
/// use infra_config::Settings;
///
/// // Load from config files
/// let settings = Settings::load()?;
///
/// // Or load from TOML string
/// let toml = r#"
///     [pricing]
///     valuation_date = "2026-01-25"
///     reporting_currency = "USD"
///
///     [risk]
///     greeks_method = "bump"
/// "#;
/// let settings = Settings::from_toml_str(toml)?;
/// ```
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Settings {
    /// Engine configuration (thread pools, memory limits, Monte Carlo)
    #[serde(default)]
    pub engine: EngineConfig,
    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,
    /// Service-specific configuration (CLI, Gateway)
    #[serde(default)]
    pub service: ServiceConfig,
    /// Logging and output configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Pricing configuration (optional)
    #[serde(default)]
    pub pricing: Option<PricingConfig>,
    /// Risk/Greeks configuration (optional)
    #[serde(default)]
    pub risk: Option<RiskConfig>,
}

impl Settings {
    /// Load settings from configuration files and environment variables.
    ///
    /// Configuration is loaded in the following order (later sources override
    /// earlier):
    /// 1. `config/default.toml`
    /// 2. `config/{environment}.toml` (based on `NEUTRYX_ENV`)
    /// 3. Environment variables prefixed with `NEUTRYX_`
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
    ///
    /// Returns a list of all validation errors found, rather than stopping
    /// at the first error. This helps users fix all issues at once.
    pub fn validate(&self) -> Result<(), Vec<ConfigError>> {
        let mut errors = Vec::new();

        // Validate pricing config if present
        if let Some(ref pricing) = self.pricing {
            if let Err(e) = pricing.validate() {
                errors.push(e);
            }
        }

        // Validate risk config if present
        if let Some(ref risk) = self.risk {
            if let Err(e) = risk.validate() {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validates and returns the first error if any.
    ///
    /// For cases where you only need to know if the config is valid.
    pub fn validate_first(&self) -> Result<(), ConfigError> {
        if let Some(ref pricing) = self.pricing {
            pricing.validate()?;
        }
        if let Some(ref risk) = self.risk {
            risk.validate()?;
        }
        Ok(())
    }
}

/// Engine configuration.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EngineConfig {
    /// Thread pool size for parallel computation
    #[serde(default = "default_thread_pool_size")]
    pub thread_pool_size: usize,
    /// Memory limit for AD engine (in MB)
    #[serde(default = "default_memory_limit_mb")]
    pub memory_limit_mb: usize,
    /// Monte Carlo simulation paths (default batch size)
    #[serde(default = "default_mc_paths")]
    pub mc_paths: usize,
    /// Checkpointing enabled
    #[serde(default)]
    pub checkpointing_enabled: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            thread_pool_size: default_thread_pool_size(),
            memory_limit_mb: default_memory_limit_mb(),
            mc_paths: default_mc_paths(),
            checkpointing_enabled: false,
        }
    }
}

fn default_thread_pool_size() -> usize { num_cpus::get() }

fn default_memory_limit_mb() -> usize {
    1024 // 1 GB
}

fn default_mc_paths() -> usize { 10_000 }

/// Database configuration.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    /// Database connection URL
    #[serde(default)]
    pub url: String,
    /// Maximum connection pool size
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    /// Connection timeout (in seconds)
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: default_max_connections(),
            connection_timeout_secs: default_connection_timeout(),
        }
    }
}

fn default_max_connections() -> u32 { 10 }

fn default_connection_timeout() -> u64 { 30 }

/// Service-specific configuration.
///
/// Consolidates settings for CLI, Gateway, and other services.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ServiceConfig {
    /// REST API configuration
    #[serde(default)]
    pub rest: RestConfig,
    /// gRPC API configuration
    #[serde(default)]
    pub grpc: GrpcConfig,
}

/// REST API configuration.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RestConfig {
    /// Enable REST API
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// REST API bind address
    #[serde(default = "default_rest_addr")]
    pub addr: String,
}

impl Default for RestConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            addr: default_rest_addr(),
        }
    }
}

fn default_true() -> bool { true }

fn default_rest_addr() -> String { "0.0.0.0:8080".to_string() }

/// gRPC API configuration.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GrpcConfig {
    /// Enable gRPC API
    #[serde(default)]
    pub enabled: bool,
    /// gRPC bind address
    #[serde(default = "default_grpc_addr")]
    pub addr: String,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            addr: default_grpc_addr(),
        }
    }
}

fn default_grpc_addr() -> String { "0.0.0.0:50051".to_string() }

/// Logging and output configuration.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Output directory for reports and results
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            output_dir: default_output_dir(),
        }
    }
}

fn default_log_level() -> String { "info".to_string() }

fn default_output_dir() -> String { "./output".to_string() }

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
        // Should have 2 errors: invalid currency + empty target_greeks
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_settings_validate_first_stops_early() {
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
        let result = settings.validate_first();
        assert!(result.is_err());
        // Should return only the first error
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
