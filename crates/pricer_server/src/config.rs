//! Server configuration management
//!
//! Handles loading configuration from environment variables, TOML files, and CLI arguments.

use serde::Deserialize;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

/// Configuration error types
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid port number: {0}. Must be between 1 and 65535")]
    InvalidPort(u16),

    #[error("Invalid log level: {0}. Must be one of: trace, debug, info, warn, error")]
    InvalidLogLevel(String),

    #[error("Invalid environment: {0}. Must be one of: development, staging, production")]
    InvalidEnvironment(String),

    #[error("Configuration file error: {0}")]
    FileError(String),

    #[error("Environment variable error: {0}")]
    EnvError(String),
}

/// Log levels supported by the server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl std::str::FromStr for LogLevel {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(ConfigError::InvalidLogLevel(s.to_string())),
        }
    }
}

impl LogLevel {
    /// Convert log level to tracing filter string
    pub fn as_filter_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_filter_str())
    }
}

/// Environment types for configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Development,
    Staging,
    Production,
}

impl std::str::FromStr for Environment {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Environment::Development),
            "staging" | "stage" => Ok(Environment::Staging),
            "production" | "prod" => Ok(Environment::Production),
            _ => Err(ConfigError::InvalidEnvironment(s.to_string())),
        }
    }
}

impl Environment {
    /// Check if this is a production environment
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Staging => write!(f, "staging"),
            Environment::Production => write!(f, "production"),
        }
    }
}

/// Server configuration structure
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Host address to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Log level
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: LogLevel,
    /// Whether API key authentication is required
    pub api_key_required: bool,
    /// List of valid API keys
    #[serde(default)]
    pub api_keys: Vec<String>,
    /// Rate limit (requests per minute)
    pub rate_limit_rpm: u32,
    /// Shutdown timeout in seconds
    pub shutdown_timeout_secs: u64,
    /// Whether pricer_kernel integration is enabled
    #[serde(skip)]
    pub kernel_enabled: bool,
    /// Environment (development, staging, production)
    #[serde(deserialize_with = "deserialize_environment")]
    pub environment: Environment,
}

fn deserialize_log_level<'de, D>(deserializer: D) -> Result<LogLevel, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    LogLevel::from_str(&s).map_err(serde::de::Error::custom)
}

fn deserialize_environment<'de, D>(deserializer: D) -> Result<Environment, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Environment::from_str(&s).map_err(serde::de::Error::custom)
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            log_level: LogLevel::Info,
            api_key_required: false,
            api_keys: Vec::new(),
            rate_limit_rpm: 60,
            shutdown_timeout_secs: 30,
            kernel_enabled: cfg!(feature = "kernel-integration"),
            environment: Environment::Development,
        }
    }
}

impl ServerConfig {
    /// Create a new ServerConfig with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();

        // Host
        if let Ok(host) = std::env::var("PRICER_SERVER_HOST") {
            config.host = host;
        }

        // Port
        if let Ok(port_str) = std::env::var("PRICER_SERVER_PORT") {
            config.port = port_str.parse().map_err(|_| ConfigError::InvalidPort(0))?;
        }

        // Log level
        if let Ok(log_level) = std::env::var("PRICER_LOG_LEVEL") {
            config.log_level = LogLevel::from_str(&log_level)?;
        }

        // API key required
        if let Ok(required) = std::env::var("PRICER_API_KEY_REQUIRED") {
            config.api_key_required = required.to_lowercase() == "true";
        }

        // API keys (comma-separated)
        if let Ok(keys) = std::env::var("PRICER_API_KEYS") {
            config.api_keys = keys.split(',').map(|s| s.trim().to_string()).collect();
        }

        // Rate limit
        if let Ok(rpm_str) = std::env::var("PRICER_RATE_LIMIT_RPM") {
            config.rate_limit_rpm = rpm_str.parse().unwrap_or(60);
        }

        // Shutdown timeout
        if let Ok(timeout_str) = std::env::var("PRICER_SHUTDOWN_TIMEOUT_SECS") {
            config.shutdown_timeout_secs = timeout_str.parse().unwrap_or(30);
        }

        // Environment
        if let Ok(env) = std::env::var("PRICER_ENV") {
            config.environment = Environment::from_str(&env)?;
        }

        // Set kernel_enabled based on feature flag
        config.kernel_enabled = cfg!(feature = "kernel-integration");

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::FileError(format!("Failed to read config file: {}", e)))?;

        let mut config: ServerConfig = toml::from_str(&content)
            .map_err(|e| ConfigError::FileError(format!("Failed to parse TOML: {}", e)))?;

        // Set kernel_enabled based on feature flag
        config.kernel_enabled = cfg!(feature = "kernel-integration");

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate port range
        if self.port == 0 {
            return Err(ConfigError::InvalidPort(self.port));
        }

        Ok(())
    }

    /// Get the socket address string
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Merge with CLI arguments (CLI takes precedence)
    pub fn merge_with_cli(&mut self, cli: &CliArgs) {
        if let Some(host) = &cli.host {
            self.host = host.clone();
        }
        if let Some(port) = cli.port {
            self.port = port;
        }
        if let Some(log_level) = &cli.log_level {
            if let Ok(level) = LogLevel::from_str(log_level) {
                self.log_level = level;
            }
        }
    }
}

/// CLI arguments structure
#[derive(Debug, Clone, Default)]
pub struct CliArgs {
    /// Config file path
    pub config_file: Option<PathBuf>,
    /// Host address override
    pub host: Option<String>,
    /// Port override
    pub port: Option<u16>,
    /// Log level override
    pub log_level: Option<String>,
}

/// Build configuration from all sources
///
/// Priority (highest to lowest):
/// 1. CLI arguments
/// 2. Environment variables
/// 3. Config file
/// 4. Default values
pub fn build_config(cli: &CliArgs) -> Result<ServerConfig, ConfigError> {
    // Start with defaults or file config
    let mut config = if let Some(config_path) = &cli.config_file {
        ServerConfig::from_file(config_path)?
    } else {
        ServerConfig::default()
    };

    // Override with environment variables
    if let Ok(env_config) = ServerConfig::from_env() {
        // Only override non-default values from env
        if std::env::var("PRICER_SERVER_HOST").is_ok() {
            config.host = env_config.host;
        }
        if std::env::var("PRICER_SERVER_PORT").is_ok() {
            config.port = env_config.port;
        }
        if std::env::var("PRICER_LOG_LEVEL").is_ok() {
            config.log_level = env_config.log_level;
        }
        if std::env::var("PRICER_API_KEY_REQUIRED").is_ok() {
            config.api_key_required = env_config.api_key_required;
        }
        if std::env::var("PRICER_API_KEYS").is_ok() {
            config.api_keys = env_config.api_keys;
        }
        if std::env::var("PRICER_RATE_LIMIT_RPM").is_ok() {
            config.rate_limit_rpm = env_config.rate_limit_rpm;
        }
        if std::env::var("PRICER_SHUTDOWN_TIMEOUT_SECS").is_ok() {
            config.shutdown_timeout_secs = env_config.shutdown_timeout_secs;
        }
        if std::env::var("PRICER_ENV").is_ok() {
            config.environment = env_config.environment;
        }
    }

    // Override with CLI arguments
    config.merge_with_cli(cli);

    // Final validation
    config.validate()?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.log_level, LogLevel::Info);
        assert!(!config.api_key_required);
        assert!(config.api_keys.is_empty());
        assert_eq!(config.rate_limit_rpm, 60);
        assert_eq!(config.shutdown_timeout_secs, 30);
        assert_eq!(config.environment, Environment::Development);
    }

    #[test]
    fn test_log_level_parsing() {
        assert_eq!(LogLevel::from_str("trace").unwrap(), LogLevel::Trace);
        assert_eq!(LogLevel::from_str("DEBUG").unwrap(), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("Info").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("WARN").unwrap(), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("error").unwrap(), LogLevel::Error);

        assert!(LogLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_environment_parsing() {
        assert_eq!(
            Environment::from_str("development").unwrap(),
            Environment::Development
        );
        assert_eq!(
            Environment::from_str("dev").unwrap(),
            Environment::Development
        );
        assert_eq!(
            Environment::from_str("staging").unwrap(),
            Environment::Staging
        );
        assert_eq!(
            Environment::from_str("stage").unwrap(),
            Environment::Staging
        );
        assert_eq!(
            Environment::from_str("production").unwrap(),
            Environment::Production
        );
        assert_eq!(
            Environment::from_str("prod").unwrap(),
            Environment::Production
        );

        assert!(Environment::from_str("invalid").is_err());
    }

    #[test]
    fn test_environment_is_production() {
        assert!(!Environment::Development.is_production());
        assert!(!Environment::Staging.is_production());
        assert!(Environment::Production.is_production());
    }

    #[test]
    fn test_socket_addr() {
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            ..Default::default()
        };
        assert_eq!(config.socket_addr(), "127.0.0.1:3000");
    }

    #[test]
    fn test_validate_port() {
        let mut config = ServerConfig::default();
        config.port = 0;
        assert!(config.validate().is_err());

        config.port = 8080;
        assert!(config.validate().is_ok());

        config.port = 65535;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(format!("{}", LogLevel::Trace), "trace");
        assert_eq!(format!("{}", LogLevel::Debug), "debug");
        assert_eq!(format!("{}", LogLevel::Info), "info");
        assert_eq!(format!("{}", LogLevel::Warn), "warn");
        assert_eq!(format!("{}", LogLevel::Error), "error");
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(format!("{}", Environment::Development), "development");
        assert_eq!(format!("{}", Environment::Staging), "staging");
        assert_eq!(format!("{}", Environment::Production), "production");
    }

    #[test]
    fn test_cli_args_merge() {
        let mut config = ServerConfig::default();
        let cli = CliArgs {
            host: Some("192.168.1.1".to_string()),
            port: Some(9000),
            log_level: Some("debug".to_string()),
            config_file: None,
        };

        config.merge_with_cli(&cli);

        assert_eq!(config.host, "192.168.1.1");
        assert_eq!(config.port, 9000);
        assert_eq!(config.log_level, LogLevel::Debug);
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_str = r#"
            host = "127.0.0.1"
            port = 3000
            log_level = "debug"
            api_key_required = true
            api_keys = ["key1", "key2"]
            rate_limit_rpm = 120
            shutdown_timeout_secs = 60
            environment = "production"
        "#;

        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
        assert_eq!(config.log_level, LogLevel::Debug);
        assert!(config.api_key_required);
        assert_eq!(config.api_keys, vec!["key1", "key2"]);
        assert_eq!(config.rate_limit_rpm, 120);
        assert_eq!(config.shutdown_timeout_secs, 60);
        assert_eq!(config.environment, Environment::Production);
    }

    #[test]
    fn test_partial_toml_deserialization() {
        let toml_str = r#"
            port = 9000
        "#;

        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        // Should use defaults for unspecified fields
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 9000);
        assert_eq!(config.log_level, LogLevel::Info);
    }

    #[test]
    fn test_log_level_as_filter_str() {
        assert_eq!(LogLevel::Trace.as_filter_str(), "trace");
        assert_eq!(LogLevel::Debug.as_filter_str(), "debug");
        assert_eq!(LogLevel::Info.as_filter_str(), "info");
        assert_eq!(LogLevel::Warn.as_filter_str(), "warn");
        assert_eq!(LogLevel::Error.as_filter_str(), "error");
    }

    #[test]
    fn test_build_config_with_defaults() {
        // Clear any environment variables that might interfere
        std::env::remove_var("PRICER_SERVER_HOST");
        std::env::remove_var("PRICER_SERVER_PORT");
        std::env::remove_var("PRICER_LOG_LEVEL");
        std::env::remove_var("PRICER_ENV");

        let cli = CliArgs::default();
        let config = build_config(&cli).unwrap();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.log_level, LogLevel::Info);
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::InvalidPort(0);
        assert!(err.to_string().contains("Invalid port"));

        let err = ConfigError::InvalidLogLevel("bad".to_string());
        assert!(err.to_string().contains("Invalid log level"));

        let err = ConfigError::InvalidEnvironment("bad".to_string());
        assert!(err.to_string().contains("Invalid environment"));
    }
}
