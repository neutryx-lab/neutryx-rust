//! Pricing configuration structures.
//!
//! Provides [`PricingConfig`] for configuration-driven pricing calculations.

use std::path::PathBuf;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::ConfigError;

/// Pricing calculation method selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PricingMethod {
    /// Closed-form analytical solutions (Black-Scholes, etc.)
    #[default]
    Analytical,
    /// Monte Carlo simulation
    MonteCarlo,
}

/// Monte Carlo simulation parameters.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MonteCarloParams {
    /// Number of simulation paths.
    pub num_paths: usize,
    /// Number of time steps per path.
    pub num_steps: usize,
    /// Optional seed for reproducibility.
    #[serde(default)]
    pub seed: Option<u64>,
}

impl Default for MonteCarloParams {
    fn default() -> Self {
        Self {
            num_paths: 10_000,
            num_steps: 252,
            seed: None,
        }
    }
}

/// Configuration for pricing calculations.
///
/// This structure defines all parameters needed for pricing operations,
/// supporting both TOML and JSON configuration formats.
///
/// # Example
///
/// ```rust
/// use infra_config::PricingConfig;
/// use chrono::NaiveDate;
/// use std::path::PathBuf;
///
/// let config = PricingConfig {
///     valuation_date: NaiveDate::from_ymd_opt(2026, 1, 25).unwrap(),
///     reporting_currency: "USD".to_string(),
///     ..Default::default()
/// };
///
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PricingConfig {
    /// Valuation date for pricing (YYYY-MM-DD format).
    pub valuation_date: NaiveDate,
    /// Reporting currency (ISO 4217 code).
    pub reporting_currency: String,
    /// Pricing method selection.
    #[serde(default)]
    pub pricing_method: PricingMethod,
    /// Monte Carlo parameters (used when pricing_method is MonteCarlo).
    #[serde(default)]
    pub monte_carlo: Option<MonteCarloParams>,
    /// Path to market data file(s).
    pub market_data_path: PathBuf,
    /// Path to trade data file(s).
    pub trade_data_path: PathBuf,
    /// Path to CSA data file (optional).
    #[serde(default)]
    pub csa_data_path: Option<PathBuf>,
    /// Enable parallel processing.
    #[serde(default = "default_true")]
    pub parallel_enabled: bool,
}

fn default_true() -> bool { true }

impl Default for PricingConfig {
    fn default() -> Self {
        Self {
            valuation_date: chrono::Local::now().date_naive(),
            reporting_currency: "USD".to_string(),
            pricing_method: PricingMethod::default(),
            monte_carlo: None,
            market_data_path: PathBuf::from("data/market.json"),
            trade_data_path: PathBuf::from("data/trades.json"),
            csa_data_path: None,
            parallel_enabled: true,
        }
    }
}

impl PricingConfig {
    /// Validates the configuration and returns errors for invalid values.
    ///
    /// # Validation Rules
    ///
    /// - `reporting_currency` must not be empty
    /// - If `pricing_method` is `MonteCarlo`, `monte_carlo` must be provided
    /// - `monte_carlo.num_paths` must be > 0
    /// - `monte_carlo.num_steps` must be > 0
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` with specific validation failure details.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate reporting currency
        if self.reporting_currency.is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "reporting_currency".to_string(),
                message: "reporting currency must not be empty".to_string(),
            });
        }

        // Validate currency format (ISO 4217: 3 uppercase letters)
        if self.reporting_currency.len() != 3
            || !self
                .reporting_currency
                .chars()
                .all(|c| c.is_ascii_uppercase())
        {
            return Err(ConfigError::InvalidValue {
                key: "reporting_currency".to_string(),
                message: format!(
                    "invalid currency code '{}': must be 3 uppercase letters (ISO 4217)",
                    self.reporting_currency
                ),
            });
        }

        // Validate Monte Carlo params when method is MonteCarlo
        if self.pricing_method == PricingMethod::MonteCarlo {
            match &self.monte_carlo {
                Some(mc) => {
                    if mc.num_paths == 0 {
                        return Err(ConfigError::InvalidValue {
                            key: "monte_carlo.num_paths".to_string(),
                            message: "num_paths must be greater than 0".to_string(),
                        });
                    }
                    if mc.num_steps == 0 {
                        return Err(ConfigError::InvalidValue {
                            key: "monte_carlo.num_steps".to_string(),
                            message: "num_steps must be greater than 0".to_string(),
                        });
                    }
                }
                None => {
                    return Err(ConfigError::missing_required(
                        "monte_carlo parameters required when pricing_method is 'monte_carlo'",
                    ));
                }
            }
        }

        Ok(())
    }

    /// Loads configuration from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if parsing fails.
    pub fn from_json(json: &str) -> Result<Self, ConfigError> {
        serde_json::from_str(json).map_err(|e| ConfigError::InvalidValue {
            key: "json".to_string(),
            message: e.to_string(),
        })
    }

    /// Loads configuration from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if parsing fails.
    pub fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        toml::from_str(toml_str).map_err(|e| ConfigError::InvalidValue {
            key: "toml".to_string(),
            message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // RED Phase Tests - These define expected behavior
    // =========================================================================

    #[test]
    fn test_pricing_config_default_creates_valid_config() {
        let config = PricingConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.reporting_currency, "USD");
        assert_eq!(config.pricing_method, PricingMethod::Analytical);
        assert!(config.parallel_enabled);
    }

    #[test]
    fn test_pricing_config_validates_empty_currency() {
        let config = PricingConfig {
            reporting_currency: String::new(),
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("reporting_currency"));
    }

    #[test]
    fn test_pricing_config_validates_invalid_currency_format() {
        let config = PricingConfig {
            reporting_currency: "usd".to_string(), // lowercase
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ISO 4217"));
    }

    #[test]
    fn test_pricing_config_monte_carlo_requires_params() {
        let config = PricingConfig {
            pricing_method: PricingMethod::MonteCarlo,
            monte_carlo: None,
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("monte_carlo parameters required"));
    }

    #[test]
    fn test_pricing_config_monte_carlo_validates_num_paths() {
        let config = PricingConfig {
            pricing_method: PricingMethod::MonteCarlo,
            monte_carlo: Some(MonteCarloParams {
                num_paths: 0,
                num_steps: 100,
                seed: None,
            }),
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("num_paths"));
    }

    #[test]
    fn test_pricing_config_monte_carlo_validates_num_steps() {
        let config = PricingConfig {
            pricing_method: PricingMethod::MonteCarlo,
            monte_carlo: Some(MonteCarloParams {
                num_paths: 1000,
                num_steps: 0,
                seed: None,
            }),
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("num_steps"));
    }

    #[test]
    fn test_pricing_config_valid_monte_carlo() {
        let config = PricingConfig {
            pricing_method: PricingMethod::MonteCarlo,
            monte_carlo: Some(MonteCarloParams {
                num_paths: 10_000,
                num_steps: 252,
                seed: Some(42),
            }),
            ..Default::default()
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_pricing_config_from_json() {
        let json = r#"{
            "valuation_date": "2026-01-25",
            "reporting_currency": "EUR",
            "pricing_method": "analytical",
            "market_data_path": "data/market.json",
            "trade_data_path": "data/trades.json",
            "parallel_enabled": false
        }"#;

        let config = PricingConfig::from_json(json).unwrap();
        assert_eq!(
            config.valuation_date,
            NaiveDate::from_ymd_opt(2026, 1, 25).unwrap()
        );
        assert_eq!(config.reporting_currency, "EUR");
        assert_eq!(config.pricing_method, PricingMethod::Analytical);
        assert!(!config.parallel_enabled);
    }

    #[test]
    fn test_pricing_config_from_json_with_monte_carlo() {
        let json = r#"{
            "valuation_date": "2026-01-25",
            "reporting_currency": "JPY",
            "pricing_method": "monte_carlo",
            "monte_carlo": {
                "num_paths": 100000,
                "num_steps": 365,
                "seed": 12345
            },
            "market_data_path": "data/market.json",
            "trade_data_path": "data/trades.json"
        }"#;

        let config = PricingConfig::from_json(json).unwrap();
        assert_eq!(config.pricing_method, PricingMethod::MonteCarlo);
        let mc = config.monte_carlo.unwrap();
        assert_eq!(mc.num_paths, 100_000);
        assert_eq!(mc.num_steps, 365);
        assert_eq!(mc.seed, Some(12345));
    }

    #[test]
    fn test_pricing_config_from_toml() {
        let toml_str = r#"
            valuation_date = "2026-01-25"
            reporting_currency = "GBP"
            pricing_method = "analytical"
            market_data_path = "data/market.json"
            trade_data_path = "data/trades.json"
            parallel_enabled = true
        "#;

        let config = PricingConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.reporting_currency, "GBP");
        assert!(config.parallel_enabled);
    }

    #[test]
    fn test_pricing_config_from_toml_with_monte_carlo() {
        let toml_str = r#"
            valuation_date = "2026-06-30"
            reporting_currency = "USD"
            pricing_method = "monte_carlo"
            market_data_path = "data/market.json"
            trade_data_path = "data/trades.json"

            [monte_carlo]
            num_paths = 50000
            num_steps = 126
        "#;

        let config = PricingConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.pricing_method, PricingMethod::MonteCarlo);
        let mc = config.monte_carlo.unwrap();
        assert_eq!(mc.num_paths, 50_000);
        assert_eq!(mc.num_steps, 126);
        assert!(mc.seed.is_none());
    }

    #[test]
    fn test_pricing_config_serializes_to_json() {
        let config = PricingConfig {
            valuation_date: NaiveDate::from_ymd_opt(2026, 1, 25).unwrap(),
            reporting_currency: "USD".to_string(),
            pricing_method: PricingMethod::Analytical,
            monte_carlo: None,
            market_data_path: PathBuf::from("data/market.json"),
            trade_data_path: PathBuf::from("data/trades.json"),
            csa_data_path: None,
            parallel_enabled: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"valuation_date\":\"2026-01-25\""));
        assert!(json.contains("\"reporting_currency\":\"USD\""));
    }

    #[test]
    fn test_monte_carlo_params_default() {
        let params = MonteCarloParams::default();
        assert_eq!(params.num_paths, 10_000);
        assert_eq!(params.num_steps, 252);
        assert!(params.seed.is_none());
    }

    #[test]
    fn test_pricing_method_serde() {
        // Test serialization
        assert_eq!(
            serde_json::to_string(&PricingMethod::Analytical).unwrap(),
            "\"analytical\""
        );
        assert_eq!(
            serde_json::to_string(&PricingMethod::MonteCarlo).unwrap(),
            "\"monte_carlo\""
        );

        // Test deserialization
        let analytical: PricingMethod = serde_json::from_str("\"analytical\"").unwrap();
        assert_eq!(analytical, PricingMethod::Analytical);

        let mc: PricingMethod = serde_json::from_str("\"monte_carlo\"").unwrap();
        assert_eq!(mc, PricingMethod::MonteCarlo);
    }
}
