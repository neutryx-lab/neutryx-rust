//! Unified error types for Greeks calculation.

use thiserror::Error;

/// Unified error type for Greeks calculation operations.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum GreeksError {
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

    /// Invalid benchmark configuration.
    #[error("Invalid benchmark configuration: {0}")]
    InvalidBenchmarkConfig(String),
}

/// Generate `snake_case` constructor helpers for `GreeksError(String)`
/// variants.
macro_rules! greeks_error_ctor {
    ($($method:ident => $variant:ident),* $(,)?) => {
        $(
            pub fn $method(msg: impl Into<String>) -> Self { Self::$variant(msg.into()) }
        )*
    };
}

impl GreeksError {
    greeks_error_ctor!(
        invalid_spot_bump => InvalidSpotBump,
        invalid_vol_bump => InvalidVolBump,
        invalid_time_bump => InvalidTimeBump,
        invalid_rate_bump => InvalidRateBump,
        invalid_tolerance => InvalidTolerance,
        invalid_swap => InvalidSwap,
        curve_not_found => CurveNotFound,
        aad_failed => AadFailed,
        invalid_config => InvalidConfig,
        invalid_benchmark_config => InvalidBenchmarkConfig,
    );

    /// Creates an accuracy check failed error.
    pub fn accuracy_check_failed(max_error: f64, tolerance: f64) -> Self {
        Self::AccuracyCheckFailed(max_error, tolerance)
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
    pub fn is_benchmark_error(&self) -> bool { matches!(self, Self::InvalidBenchmarkConfig(_)) }
}
