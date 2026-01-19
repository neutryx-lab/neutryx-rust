//! CLI error types

use thiserror::Error;

/// Result type for CLI operations
pub type Result<T> = std::result::Result<T, CliError>;

/// CLI-specific errors
#[derive(Error, Debug)]
pub enum CliError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] infra_config::ConfigError),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Pricing error
    #[error("Pricing error: {0}")]
    Pricing(String),

    /// Calibration error
    #[error("Calibration error: {0}")]
    Calibration(String),

    /// Invalid argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),
}
