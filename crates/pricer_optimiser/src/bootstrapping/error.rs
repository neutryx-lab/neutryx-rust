//! Bootstrap-specific error types.
//!
//! This module provides structured error handling for yield curve bootstrapping
//! operations with detailed diagnostic information for each failure mode.

use pricer_core::market_data::MarketDataError;
use pricer_core::types::SolverError;
use thiserror::Error;

/// Errors that can occur during yield curve bootstrapping.
///
/// Provides structured error handling with diagnostic information
/// including maturity points, residuals, and iteration counts.
///
/// # Variants
///
/// - `ConvergenceFailure`: Solver failed to converge at a specific pillar
/// - `DuplicateMaturity`: Multiple instruments with same maturity
/// - `InsufficientData`: Not enough instruments for bootstrapping
/// - `NegativeRate`: Negative rate detected (warning or error)
/// - `ArbitrageDetected`: Discount factors not monotonically decreasing
/// - `SolverError`: Wrapped solver error from root-finding
/// - `MarketDataError`: Wrapped market data error
/// - `InvalidInput`: General invalid input error
///
/// # Examples
///
/// ```
/// use pricer_optimiser::bootstrapping::BootstrapError;
///
/// let err = BootstrapError::ConvergenceFailure {
///     maturity: 5.0,
///     residual: 0.001,
///     iterations: 100,
/// };
/// assert!(format!("{}", err).contains("5"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum BootstrapError {
    /// Solver failed to converge at a specific maturity point.
    #[error("Failed to converge at maturity {maturity}: residual = {residual} after {iterations} iterations")]
    ConvergenceFailure {
        /// Maturity (in years) where convergence failed
        maturity: f64,
        /// Final residual value
        residual: f64,
        /// Number of iterations attempted
        iterations: usize,
    },

    /// Duplicate maturity detected in input instruments.
    #[error("Duplicate maturity detected: {maturity}")]
    DuplicateMaturity {
        /// The duplicated maturity value
        maturity: f64,
    },

    /// Insufficient instruments provided for bootstrapping.
    #[error("Insufficient instruments: need at least {required}, got {provided}")]
    InsufficientData {
        /// Minimum number of instruments required
        required: usize,
        /// Number of instruments provided
        provided: usize,
    },

    /// Negative rate detected at a maturity point.
    #[error("Negative rate detected at maturity {maturity}: rate = {rate}")]
    NegativeRate {
        /// Maturity where negative rate was detected
        maturity: f64,
        /// The negative rate value
        rate: f64,
    },

    /// Arbitrage detected (discount factors not monotonically decreasing).
    #[error(
        "Arbitrage detected: discount factor not monotonically decreasing at maturity {maturity}"
    )]
    ArbitrageDetected {
        /// Maturity where arbitrage was detected
        maturity: f64,
    },

    /// Wrapped solver error from root-finding operations.
    #[error("Solver error: {0}")]
    Solver(#[from] SolverError),

    /// Wrapped market data error.
    #[error("Market data error: {0}")]
    MarketData(#[from] MarketDataError),

    /// General invalid input error.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Invalid maturity range.
    #[error("Invalid maturity: {maturity} (must be > 0 and <= {max_maturity})")]
    InvalidMaturity {
        /// The invalid maturity value
        maturity: f64,
        /// Maximum allowed maturity
        max_maturity: f64,
    },
}

impl BootstrapError {
    /// Create a convergence failure error.
    pub fn convergence_failure(maturity: f64, residual: f64, iterations: usize) -> Self {
        Self::ConvergenceFailure {
            maturity,
            residual,
            iterations,
        }
    }

    /// Create a duplicate maturity error.
    pub fn duplicate_maturity(maturity: f64) -> Self {
        Self::DuplicateMaturity { maturity }
    }

    /// Create an insufficient data error.
    pub fn insufficient_data(required: usize, provided: usize) -> Self {
        Self::InsufficientData { required, provided }
    }

    /// Create a negative rate error.
    pub fn negative_rate(maturity: f64, rate: f64) -> Self {
        Self::NegativeRate { maturity, rate }
    }

    /// Create an arbitrage detected error.
    pub fn arbitrage_detected(maturity: f64) -> Self {
        Self::ArbitrageDetected { maturity }
    }

    /// Create an invalid input error.
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    /// Create an invalid maturity error.
    pub fn invalid_maturity(maturity: f64, max_maturity: f64) -> Self {
        Self::InvalidMaturity {
            maturity,
            max_maturity,
        }
    }

    /// Check if this is a convergence failure.
    pub fn is_convergence_failure(&self) -> bool {
        matches!(self, Self::ConvergenceFailure { .. })
    }

    /// Check if this is a duplicate maturity error.
    pub fn is_duplicate_maturity(&self) -> bool {
        matches!(self, Self::DuplicateMaturity { .. })
    }

    /// Check if this is an insufficient data error.
    pub fn is_insufficient_data(&self) -> bool {
        matches!(self, Self::InsufficientData { .. })
    }

    /// Check if this is a negative rate error.
    pub fn is_negative_rate(&self) -> bool {
        matches!(self, Self::NegativeRate { .. })
    }

    /// Check if this is an arbitrage detected error.
    pub fn is_arbitrage_detected(&self) -> bool {
        matches!(self, Self::ArbitrageDetected { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Convergence Failure Tests
    // ========================================

    #[test]
    fn test_convergence_failure_display() {
        let err = BootstrapError::convergence_failure(5.0, 0.001, 100);
        let display = format!("{}", err);
        assert!(display.contains("5"));
        assert!(display.contains("0.001"));
        assert!(display.contains("100"));
    }

    #[test]
    fn test_convergence_failure_is_check() {
        let err = BootstrapError::convergence_failure(5.0, 0.001, 100);
        assert!(err.is_convergence_failure());
        assert!(!err.is_duplicate_maturity());
    }

    // ========================================
    // Duplicate Maturity Tests
    // ========================================

    #[test]
    fn test_duplicate_maturity_display() {
        let err = BootstrapError::duplicate_maturity(2.5);
        let display = format!("{}", err);
        assert!(display.contains("Duplicate maturity"));
        assert!(display.contains("2.5"));
    }

    #[test]
    fn test_duplicate_maturity_is_check() {
        let err = BootstrapError::duplicate_maturity(2.5);
        assert!(err.is_duplicate_maturity());
        assert!(!err.is_convergence_failure());
    }

    // ========================================
    // Insufficient Data Tests
    // ========================================

    #[test]
    fn test_insufficient_data_display() {
        let err = BootstrapError::insufficient_data(5, 3);
        let display = format!("{}", err);
        assert!(display.contains("Insufficient"));
        assert!(display.contains("5"));
        assert!(display.contains("3"));
    }

    #[test]
    fn test_insufficient_data_is_check() {
        let err = BootstrapError::insufficient_data(5, 3);
        assert!(err.is_insufficient_data());
    }

    // ========================================
    // Negative Rate Tests
    // ========================================

    #[test]
    fn test_negative_rate_display() {
        let err = BootstrapError::negative_rate(1.0, -0.005);
        let display = format!("{}", err);
        assert!(display.contains("Negative rate"));
        assert!(display.contains("-0.005"));
    }

    #[test]
    fn test_negative_rate_is_check() {
        let err = BootstrapError::negative_rate(1.0, -0.005);
        assert!(err.is_negative_rate());
    }

    // ========================================
    // Arbitrage Detected Tests
    // ========================================

    #[test]
    fn test_arbitrage_detected_display() {
        let err = BootstrapError::arbitrage_detected(3.0);
        let display = format!("{}", err);
        assert!(display.contains("Arbitrage"));
        assert!(display.contains("3"));
    }

    #[test]
    fn test_arbitrage_detected_is_check() {
        let err = BootstrapError::arbitrage_detected(3.0);
        assert!(err.is_arbitrage_detected());
    }

    // ========================================
    // From Trait Tests
    // ========================================

    #[test]
    fn test_from_solver_error() {
        let solver_err = SolverError::MaxIterationsExceeded { iterations: 100 };
        let bootstrap_err: BootstrapError = solver_err.into();
        match bootstrap_err {
            BootstrapError::Solver(SolverError::MaxIterationsExceeded { iterations }) => {
                assert_eq!(iterations, 100);
            }
            _ => panic!("Expected Solver variant"),
        }
    }

    #[test]
    fn test_from_market_data_error() {
        let mkt_err = MarketDataError::InvalidMaturity { t: -1.0 };
        let bootstrap_err: BootstrapError = mkt_err.into();
        match bootstrap_err {
            BootstrapError::MarketData(MarketDataError::InvalidMaturity { t }) => {
                assert!((t - (-1.0)).abs() < 1e-10);
            }
            _ => panic!("Expected MarketData variant"),
        }
    }

    // ========================================
    // Invalid Input Tests
    // ========================================

    #[test]
    fn test_invalid_input_display() {
        let err = BootstrapError::invalid_input("empty instrument list");
        let display = format!("{}", err);
        assert!(display.contains("Invalid input"));
        assert!(display.contains("empty instrument list"));
    }

    // ========================================
    // Invalid Maturity Tests
    // ========================================

    #[test]
    fn test_invalid_maturity_display() {
        let err = BootstrapError::invalid_maturity(60.0, 50.0);
        let display = format!("{}", err);
        assert!(display.contains("Invalid maturity"));
        assert!(display.contains("60"));
        assert!(display.contains("50"));
    }

    // ========================================
    // Clone and Equality Tests
    // ========================================

    #[test]
    fn test_clone_and_equality() {
        let err1 = BootstrapError::convergence_failure(5.0, 0.001, 100);
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = BootstrapError::convergence_failure(5.0, 0.001, 100);
        let _: &dyn std::error::Error = &err;
    }

    // ========================================
    // Direct Variant Construction Tests
    // ========================================

    #[test]
    fn test_direct_convergence_failure() {
        let err = BootstrapError::ConvergenceFailure {
            maturity: 5.0,
            residual: 0.001,
            iterations: 100,
        };
        assert!(err.is_convergence_failure());
    }

    #[test]
    fn test_direct_duplicate_maturity() {
        let err = BootstrapError::DuplicateMaturity { maturity: 2.5 };
        assert!(err.is_duplicate_maturity());
    }
}
