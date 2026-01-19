//! Configuration errors.

use thiserror::Error;

/// Errors that can occur during configuration loading.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Configuration file not found
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    /// Invalid configuration value
    #[error("Invalid configuration value for '{key}': {message}")]
    InvalidValue {
        /// Configuration key that has invalid value
        key: String,
        /// Description of validation failure
        message: String,
    },

    /// Missing required configuration
    #[error("Missing required configuration: {0}")]
    MissingRequired(String),

    /// Environment variable error
    #[error("Environment variable error: {0}")]
    EnvError(String),

    /// Underlying config crate error
    #[error("Configuration error: {0}")]
    ConfigCrateError(#[from] config::ConfigError),
}
