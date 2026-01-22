//! Numerical integration methods.
//!
//! This module provides various numerical integration (quadrature) methods
//! commonly used in quantitative finance for option pricing and risk calculations.
//!
//! ## Available Methods
//!
//! - **Gauss-Legendre**: High-accuracy fixed-point quadrature (7, 15, 21 points)
//! - **Gauss-Kronrod**: Quadrature with embedded error estimation (G7-K15, G10-K21)
//! - **Adaptive**: Interval bisection with tolerance-based convergence
//! - **Runge-Kutta**: ODE solvers (RK4, RK45 Dormand-Prince)
//!
//! ## Usage
//!
//! ```
//! use pricer_core::math::integrators::{integrate_gauss_legendre, GaussLegendreOrder};
//!
//! // Integrate x^2 from 0 to 1 (exact result = 1/3)
//! let result = integrate_gauss_legendre(|x: f64| x * x, 0.0_f64, 1.0_f64, GaussLegendreOrder::N7);
//! assert!((result.value - 1.0_f64 / 3.0_f64).abs() < 1e-10);
//! ```
//!
//! ## AD Compatibility
//!
//! All integration functions are generic over `T: Float` to support automatic
//! differentiation through dual numbers.

mod adaptive;
mod gauss_kronrod;
mod gauss_legendre;
mod runge_kutta;

pub use adaptive::{integrate_adaptive, integrate_tanh_sinh, TanhSinhOptions};
pub use gauss_kronrod::{integrate_gauss_kronrod, GaussKronrodRule};
pub use gauss_legendre::{integrate_gauss_legendre, GaussLegendreOrder};
pub use runge_kutta::{rk4_step, rk45_integrate, Rk45Options};

use thiserror::Error;

/// Result of a numerical integration.
#[derive(Debug, Clone, Copy)]
pub struct IntegrationResult<T> {
    /// The computed integral value.
    pub value: T,
    /// Estimated error (if available from the method).
    pub error_estimate: Option<T>,
    /// Number of function evaluations.
    pub num_evaluations: usize,
}

impl<T: Copy> IntegrationResult<T> {
    /// Creates a new integration result with a value only.
    #[must_use]
    pub fn new(value: T, num_evaluations: usize) -> Self {
        Self {
            value,
            error_estimate: None,
            num_evaluations,
        }
    }

    /// Creates a new integration result with value and error estimate.
    #[must_use]
    pub fn with_error(value: T, error: T, num_evaluations: usize) -> Self {
        Self {
            value,
            error_estimate: Some(error),
            num_evaluations,
        }
    }
}

/// Errors that can occur during numerical integration.
#[derive(Error, Debug, Clone)]
pub enum IntegrationError {
    /// The integration did not converge within the allowed iterations.
    #[error("Integration did not converge after {max_iterations} iterations")]
    NotConverged {
        /// Maximum iterations attempted.
        max_iterations: usize,
    },

    /// Invalid integration bounds.
    #[error("Invalid integration bounds: a={a}, b={b}")]
    InvalidBounds {
        /// Lower bound.
        a: f64,
        /// Upper bound.
        b: f64,
    },

    /// A numerical computation failed.
    #[error("Numerical error: {0}")]
    NumericalError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_result_new() {
        let result: IntegrationResult<f64> = IntegrationResult::new(1.5, 10);
        assert!((result.value - 1.5).abs() < 1e-15);
        assert!(result.error_estimate.is_none());
        assert_eq!(result.num_evaluations, 10);
    }

    #[test]
    fn test_integration_result_with_error() {
        let result: IntegrationResult<f64> = IntegrationResult::with_error(1.5, 0.001, 20);
        assert!((result.value - 1.5).abs() < 1e-15);
        assert!((result.error_estimate.unwrap() - 0.001).abs() < 1e-15);
        assert_eq!(result.num_evaluations, 20);
    }
}
