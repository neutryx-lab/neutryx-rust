//! Demo configuration management.
//!
//! Handles loading and management of demo configuration from TOML files
//! with environment variable override support.

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Demo operation mode
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DemoMode {
    /// Full demo with all features
    Full,
    /// Quick demo with limited trades
    Quick,
    /// Custom configuration
    Custom,
}

impl Default for DemoMode {
    fn default() -> Self {
        Self::Quick
    }
}

/// Demo configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DemoConfig {
    /// Demo mode
    #[serde(default)]
    pub mode: DemoMode,

    /// Data directory path
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// Maximum number of trades (for Quick mode)
    pub max_trades: Option<usize>,

    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Service gateway URL
    #[serde(default = "default_gateway_url")]
    pub gateway_url: String,
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("demo/data")
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_gateway_url() -> String {
    "http://localhost:8080".to_string()
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            mode: DemoMode::default(),
            data_dir: default_data_dir(),
            max_trades: Some(10),
            log_level: default_log_level(),
            gateway_url: default_gateway_url(),
        }
    }
}

impl DemoConfig {
    /// Load configuration from a TOML file
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(e.to_string()))?;

        toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Load configuration from default path or return default config
    pub fn load_or_default() -> Self {
        let config_path = PathBuf::from("demo/data/config/demo_config.toml");
        Self::load(&config_path).unwrap_or_default()
    }

    /// Apply environment variable overrides
    pub fn with_env_override(mut self) -> Self {
        if let Ok(mode) = std::env::var("DEMO_MODE") {
            self.mode = match mode.to_lowercase().as_str() {
                "full" => DemoMode::Full,
                "quick" => DemoMode::Quick,
                "custom" => DemoMode::Custom,
                _ => self.mode,
            };
        }

        if let Ok(data_dir) = std::env::var("DEMO_DATA_DIR") {
            self.data_dir = PathBuf::from(data_dir);
        }

        if let Ok(max_trades) = std::env::var("DEMO_MAX_TRADES") {
            self.max_trades = max_trades.parse().ok();
        }

        if let Ok(log_level) = std::env::var("DEMO_LOG_LEVEL") {
            self.log_level = log_level;
        }

        if let Ok(gateway_url) = std::env::var("DEMO_GATEWAY_URL") {
            self.gateway_url = gateway_url;
        }

        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut errors = Vec::new();

        // Validate log level
        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&self.log_level.to_lowercase().as_str()) {
            errors.push(format!(
                "Invalid log_level '{}'. Valid values: {:?}",
                self.log_level, valid_log_levels
            ));
        }

        // Validate gateway URL format
        if !self.gateway_url.starts_with("http://") && !self.gateway_url.starts_with("https://") {
            errors.push(format!(
                "Invalid gateway_url '{}'. Must start with http:// or https://",
                self.gateway_url
            ));
        }

        // Validate max_trades range
        if let Some(max_trades) = self.max_trades {
            if max_trades == 0 {
                errors.push("max_trades must be greater than 0".to_string());
            }
            if max_trades > 1_000_000 {
                errors.push(format!(
                    "max_trades {} exceeds maximum allowed (1,000,000)",
                    max_trades
                ));
            }
        }

        // Validate data_dir is not empty
        if self.data_dir.as_os_str().is_empty() {
            errors.push("data_dir cannot be empty".to_string());
        }

        // Mode-specific validation
        if self.mode == DemoMode::Quick && self.max_trades.is_none() {
            errors.push("Quick mode requires max_trades to be set".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::Validation(errors))
        }
    }

    /// Load configuration from file and validate
    pub fn load_and_validate(path: &Path) -> Result<Self, ConfigError> {
        let config = Self::load(path)?;
        config.validate()?;
        Ok(config)
    }

    /// Load from file with environment overrides and validate
    pub fn load_with_env_and_validate(path: &Path) -> Result<Self, ConfigError> {
        let config = Self::load(path)?.with_env_override();
        config.validate()?;
        Ok(config)
    }
}

/// Configuration error type
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// IO error reading config file
    Io(String),
    /// Parse error in config file
    Parse(String),
    /// Validation error
    Validation(Vec<String>),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "IO error: {}", msg),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
            Self::Validation(errors) => write!(f, "Validation errors: {}", errors.join("; ")),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DemoConfig::default();
        assert_eq!(config.mode, DemoMode::Quick);
        assert_eq!(config.max_trades, Some(10));
    }

    #[test]
    fn test_env_override() {
        std::env::set_var("DEMO_MODE", "full");
        let config = DemoConfig::default().with_env_override();
        assert_eq!(config.mode, DemoMode::Full);
        std::env::remove_var("DEMO_MODE");
    }

    #[test]
    fn test_default_config_validates() {
        let config = DemoConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_log_level() {
        let mut config = DemoConfig::default();
        config.log_level = "invalid".to_string();

        let result = config.validate();
        assert!(result.is_err());

        if let Err(ConfigError::Validation(errors)) = result {
            assert!(errors.iter().any(|e| e.contains("log_level")));
        } else {
            panic!("Expected validation error");
        }
    }

    #[test]
    fn test_validate_valid_log_levels() {
        for level in &["trace", "debug", "info", "warn", "error", "INFO", "DEBUG"] {
            let mut config = DemoConfig::default();
            config.log_level = level.to_string();
            assert!(config.validate().is_ok(), "Log level '{}' should be valid", level);
        }
    }

    #[test]
    fn test_validate_invalid_gateway_url() {
        let mut config = DemoConfig::default();
        config.gateway_url = "not-a-valid-url".to_string();

        let result = config.validate();
        assert!(result.is_err());

        if let Err(ConfigError::Validation(errors)) = result {
            assert!(errors.iter().any(|e| e.contains("gateway_url")));
        } else {
            panic!("Expected validation error");
        }
    }

    #[test]
    fn test_validate_https_gateway_url() {
        let mut config = DemoConfig::default();
        config.gateway_url = "https://secure.example.com:8443".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_max_trades_zero() {
        let mut config = DemoConfig::default();
        config.max_trades = Some(0);

        let result = config.validate();
        assert!(result.is_err());

        if let Err(ConfigError::Validation(errors)) = result {
            assert!(errors.iter().any(|e| e.contains("max_trades")));
        } else {
            panic!("Expected validation error");
        }
    }

    #[test]
    fn test_validate_max_trades_too_large() {
        let mut config = DemoConfig::default();
        config.max_trades = Some(2_000_000);

        let result = config.validate();
        assert!(result.is_err());

        if let Err(ConfigError::Validation(errors)) = result {
            assert!(errors.iter().any(|e| e.contains("exceeds maximum")));
        } else {
            panic!("Expected validation error");
        }
    }

    #[test]
    fn test_validate_empty_data_dir() {
        let mut config = DemoConfig::default();
        config.data_dir = PathBuf::from("");

        let result = config.validate();
        assert!(result.is_err());

        if let Err(ConfigError::Validation(errors)) = result {
            assert!(errors.iter().any(|e| e.contains("data_dir")));
        } else {
            panic!("Expected validation error");
        }
    }

    #[test]
    fn test_validate_quick_mode_requires_max_trades() {
        let mut config = DemoConfig::default();
        config.mode = DemoMode::Quick;
        config.max_trades = None;

        let result = config.validate();
        assert!(result.is_err());

        if let Err(ConfigError::Validation(errors)) = result {
            assert!(errors.iter().any(|e| e.contains("Quick mode")));
        } else {
            panic!("Expected validation error");
        }
    }

    #[test]
    fn test_validate_full_mode_no_max_trades_required() {
        let mut config = DemoConfig::default();
        config.mode = DemoMode::Full;
        config.max_trades = None;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_multiple_errors() {
        let mut config = DemoConfig::default();
        config.log_level = "invalid".to_string();
        config.gateway_url = "bad-url".to_string();
        config.max_trades = Some(0);

        let result = config.validate();
        assert!(result.is_err());

        if let Err(ConfigError::Validation(errors)) = result {
            assert!(errors.len() >= 3, "Expected at least 3 validation errors");
        } else {
            panic!("Expected validation error");
        }
    }

    #[test]
    fn test_config_error_display() {
        let error = ConfigError::Validation(vec![
            "Error 1".to_string(),
            "Error 2".to_string(),
        ]);
        let display = format!("{}", error);
        assert!(display.contains("Error 1"));
        assert!(display.contains("Error 2"));
    }
}
