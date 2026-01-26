//! Error types for Monte Carlo pricing kernel.
//!
//! This module defines structured error types for configuration validation
//! and runtime errors in the Monte Carlo simulation engine.

use thiserror::Error;

/// Configuration error for Monte Carlo pricer.
///
/// These errors occur during construction when invalid parameters are provided.
///
/// Note: Named `MonteCarloConfigError` to avoid collision with
/// `infra_config::ConfigError` which handles system-wide configuration errors.
#[derive(Error, Clone, Debug, PartialEq, Eq)]
pub enum MonteCarloConfigError {
    /// Path count outside valid range [1, 10_000_000].
    #[error("Invalid path count {0}: must be in range [1, 10_000_000]")]
    InvalidPathCount(usize),
    /// Step count outside valid range [1, 10_000].
    #[error("Invalid step count {0}: must be in range [1, 10_000]")]
    InvalidStepCount(usize),
    /// Invalid parameter value with name and description.
    #[error("Invalid parameter '{name}': {value}")]
    InvalidParameter {
        /// Parameter name.
        name: &'static str,
        /// Description of the invalid value.
        value: String,
    },
    /// Layout configuration error.
    #[error("Layout configuration error: {0}")]
    LayoutError(#[from] LayoutConfigError),
}

/// Error type for layout and streaming configuration.
///
/// These errors occur when invalid layout or streaming parameters are specified.
#[derive(Error, Clone, Debug, PartialEq, Eq)]
pub enum LayoutConfigError {
    /// Streaming mode requires TimeStepFirst layout.
    #[error("Streaming mode requires TimeStepFirst layout")]
    StreamingRequiresTimeStepFirst,

    /// Alignment must be a power of 2.
    #[error("Alignment must be a power of 2, got {0}")]
    InvalidAlignment(usize),

    /// Buffer steps must be at least 2.
    #[error("Buffer steps must be at least 2, got {0}")]
    InvalidBufferSteps(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_display() {
        let err = MonteCarloConfigError::InvalidPathCount(0);
        assert!(err.to_string().contains("Invalid path count 0"));

        let err = MonteCarloConfigError::InvalidStepCount(20_000);
        assert!(err.to_string().contains("Invalid step count 20000"));

        let err = MonteCarloConfigError::InvalidParameter {
            name: "volatility",
            value: "must be positive".to_string(),
        };
        assert!(err.to_string().contains("volatility"));
    }
}
