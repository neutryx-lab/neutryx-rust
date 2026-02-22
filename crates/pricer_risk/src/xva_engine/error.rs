//! XVA engine error types.

use thiserror::Error;

/// Errors that can occur during XVA engine operations.
#[derive(Debug, Error)]
pub enum XvaEngineError {
    /// Simulation error during Monte Carlo path generation.
    #[error("Simulation error: {0}")]
    SimulationError(String),

    /// Calibration error during model parameter fitting.
    #[error("Calibration error: {0}")]
    CalibrationError(String),

    /// Hierarchy error in portfolio structure.
    #[error("Hierarchy error: {0}")]
    HierarchyError(String),

    /// Pricing error during trade valuation.
    #[error("Pricing error: {0}")]
    PricingError(String),

    /// Aggregation error during exposure computation.
    #[error("Aggregation error: {0}")]
    AggregationError(String),

    /// Configuration error in engine settings.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Invalid time grid specification.
    #[error("Invalid time grid: {0}")]
    InvalidTimeGrid(String),

    /// Missing market data required for computation.
    #[error("Missing market data: {0}")]
    MissingMarketData(String),

    /// Dimension mismatch between arrays or matrices.
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimension.
        expected: usize,
        /// Actual dimension.
        actual: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_error_display() {
        let err = XvaEngineError::SimulationError("path generation failed".to_string());
        assert_eq!(
            format!("{}", err),
            "Simulation error: path generation failed"
        );
    }

    #[test]
    fn test_calibration_error_display() {
        let err = XvaEngineError::CalibrationError("convergence failure".to_string());
        assert_eq!(format!("{}", err), "Calibration error: convergence failure");
    }

    #[test]
    fn test_hierarchy_error_display() {
        let err = XvaEngineError::HierarchyError("missing counterparty".to_string());
        assert_eq!(format!("{}", err), "Hierarchy error: missing counterparty");
    }

    #[test]
    fn test_pricing_error_display() {
        let err = XvaEngineError::PricingError("model not calibrated".to_string());
        assert_eq!(format!("{}", err), "Pricing error: model not calibrated");
    }

    #[test]
    fn test_aggregation_error_display() {
        let err = XvaEngineError::AggregationError("empty netting set".to_string());
        assert_eq!(format!("{}", err), "Aggregation error: empty netting set");
    }

    #[test]
    fn test_config_error_display() {
        let err = XvaEngineError::ConfigError("invalid n_paths".to_string());
        assert_eq!(format!("{}", err), "Configuration error: invalid n_paths");
    }

    #[test]
    fn test_invalid_time_grid_display() {
        let err = XvaEngineError::InvalidTimeGrid("non-monotonic".to_string());
        assert_eq!(format!("{}", err), "Invalid time grid: non-monotonic");
    }

    #[test]
    fn test_missing_market_data_display() {
        let err = XvaEngineError::MissingMarketData("USD yield curve".to_string());
        assert_eq!(format!("{}", err), "Missing market data: USD yield curve");
    }

    #[test]
    fn test_dimension_mismatch_display() {
        let err = XvaEngineError::DimensionMismatch {
            expected: 100,
            actual: 50,
        };
        assert_eq!(
            format!("{}", err),
            "Dimension mismatch: expected 100, got 50"
        );
    }

    #[test]
    fn test_error_is_error_trait() {
        let err: Box<dyn std::error::Error> =
            Box::new(XvaEngineError::SimulationError("test".to_string()));
        assert!(err.to_string().contains("Simulation"));
    }

    #[test]
    fn test_error_debug() {
        let err = XvaEngineError::ConfigError("bad config".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("ConfigError"));
        assert!(debug.contains("bad config"));
    }
}
