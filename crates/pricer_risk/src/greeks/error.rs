//! Unified error types for Greeks calculation.
//!
//! This module provides a consolidated error hierarchy for all Greeks-related
//! operations including configuration, calculation, and benchmarking.
//!
//! # Migration
//!
//! The following types are now aliases to variants of [`GreeksError`]:
//! - `GreeksConfigError` → [`GreeksError::Config`]
//! - `IrsGreeksError` → [`GreeksError::Calculation`] (with l1l2-integration)
//! - `BenchmarkError` → [`GreeksError::Benchmark`] (with l1l2-integration)

use thiserror::Error;

/// Unified error type for Greeks calculation operations.
///
/// Consolidates configuration, calculation, and benchmarking errors
/// into a single error hierarchy for consistent error handling.
///
/// # Variants
///
/// - Configuration errors: Invalid bump widths, tolerances, or modes
/// - Calculation errors: Swap validation, curve lookup, AAD failures
/// - Benchmark errors: Configuration and execution failures
///
/// # Examples
///
/// ```rust
/// use pricer_risk::greeks::GreeksError;
///
/// fn validate_spot_bump(bump: f64) -> Result<(), GreeksError> {
///     if bump <= 0.0 {
///         return Err(GreeksError::InvalidSpotBump(
///             "spot bump must be positive".to_string(),
///         ));
///     }
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Error)]
pub enum GreeksError {
    // =========================================================================
    // Configuration errors (from GreeksConfigError)
    // =========================================================================
    /// Invalid spot bump value.
    #[error("Invalid spot bump: {0}")]
    InvalidSpotBump(String),

    /// Invalid volatility bump value.
    #[error("Invalid vol bump: {0}")]
    InvalidVolBump(String),

    /// Invalid time bump value.
    #[error("Invalid time bump: {0}")]
    InvalidTimeBump(String),

    /// Invalid rate bump value.
    #[error("Invalid rate bump: {0}")]
    InvalidRateBump(String),

    /// Invalid verification tolerance.
    #[error("Invalid tolerance: {0}")]
    InvalidTolerance(String),

    // =========================================================================
    // Calculation errors (from IrsGreeksError)
    // =========================================================================
    /// Invalid swap parameters.
    #[error("Invalid swap parameters: {0}")]
    InvalidSwap(String),

    /// Curve not found in curve set.
    #[error("Curve not found: {0}")]
    CurveNotFound(String),

    /// AAD computation failed.
    #[error("AAD computation failed: {0}")]
    AadFailed(String),

    /// Accuracy check failed between AAD and bump-and-revalue.
    #[error("Accuracy check failed: max relative error {0} exceeds tolerance {1}")]
    AccuracyCheckFailed(f64, f64),

    /// Invalid configuration for calculation.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    // =========================================================================
    // Benchmark errors (from BenchmarkError)
    // =========================================================================
    /// Invalid benchmark configuration.
    #[error("Invalid benchmark configuration: {0}")]
    InvalidBenchmarkConfig(String),
}

impl GreeksError {
    /// Creates an invalid spot bump error.
    pub fn invalid_spot_bump(msg: impl Into<String>) -> Self {
        Self::InvalidSpotBump(msg.into())
    }

    /// Creates an invalid vol bump error.
    pub fn invalid_vol_bump(msg: impl Into<String>) -> Self {
        Self::InvalidVolBump(msg.into())
    }

    /// Creates an invalid time bump error.
    pub fn invalid_time_bump(msg: impl Into<String>) -> Self {
        Self::InvalidTimeBump(msg.into())
    }

    /// Creates an invalid rate bump error.
    pub fn invalid_rate_bump(msg: impl Into<String>) -> Self {
        Self::InvalidRateBump(msg.into())
    }

    /// Creates an invalid tolerance error.
    pub fn invalid_tolerance(msg: impl Into<String>) -> Self {
        Self::InvalidTolerance(msg.into())
    }

    /// Creates an invalid swap error.
    pub fn invalid_swap(msg: impl Into<String>) -> Self {
        Self::InvalidSwap(msg.into())
    }

    /// Creates a curve not found error.
    pub fn curve_not_found(name: impl Into<String>) -> Self {
        Self::CurveNotFound(name.into())
    }

    /// Creates an AAD failed error.
    pub fn aad_failed(msg: impl Into<String>) -> Self {
        Self::AadFailed(msg.into())
    }

    /// Creates an accuracy check failed error.
    pub fn accuracy_check_failed(max_error: f64, tolerance: f64) -> Self {
        Self::AccuracyCheckFailed(max_error, tolerance)
    }

    /// Creates an invalid config error.
    pub fn invalid_config(msg: impl Into<String>) -> Self {
        Self::InvalidConfig(msg.into())
    }

    /// Creates an invalid benchmark config error.
    pub fn invalid_benchmark_config(msg: impl Into<String>) -> Self {
        Self::InvalidBenchmarkConfig(msg.into())
    }

    /// Returns true if this is a configuration error.
    pub fn is_config_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidSpotBump(_)
                | Self::InvalidVolBump(_)
                | Self::InvalidTimeBump(_)
                | Self::InvalidRateBump(_)
                | Self::InvalidTolerance(_)
        )
    }

    /// Returns true if this is a calculation error.
    pub fn is_calculation_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidSwap(_)
                | Self::CurveNotFound(_)
                | Self::AadFailed(_)
                | Self::AccuracyCheckFailed(_, _)
                | Self::InvalidConfig(_)
        )
    }

    /// Returns true if this is a benchmark error.
    pub fn is_benchmark_error(&self) -> bool {
        matches!(self, Self::InvalidBenchmarkConfig(_))
    }
}
