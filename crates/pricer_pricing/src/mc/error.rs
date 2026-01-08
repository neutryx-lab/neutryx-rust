//! Error types for Monte Carlo pricing kernel.
//!
//! This module defines structured error types for configuration validation
//! and runtime errors in the Monte Carlo simulation engine.

use std::fmt;

/// Configuration error for Monte Carlo pricer.
///
/// These errors occur during construction when invalid parameters are provided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// Path count outside valid range [1, 10_000_000].
    InvalidPathCount(usize),
    /// Step count outside valid range [1, 10_000].
    InvalidStepCount(usize),
    /// Invalid parameter value with name and description.
    InvalidParameter {
        /// Parameter name.
        name: &'static str,
        /// Description of the invalid value.
        value: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPathCount(count) => {
                write!(
                    f,
                    "Invalid path count {}: must be in range [1, 10_000_000]",
                    count
                )
            }
            Self::InvalidStepCount(count) => {
                write!(
                    f,
                    "Invalid step count {}: must be in range [1, 10_000]",
                    count
                )
            }
            Self::InvalidParameter { name, value } => {
                write!(f, "Invalid parameter '{}': {}", name, value)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::InvalidPathCount(0);
        assert!(err.to_string().contains("Invalid path count 0"));

        let err = ConfigError::InvalidStepCount(20_000);
        assert!(err.to_string().contains("Invalid step count 20000"));

        let err = ConfigError::InvalidParameter {
            name: "volatility",
            value: "must be positive".to_string(),
        };
        assert!(err.to_string().contains("volatility"));
    }
}
