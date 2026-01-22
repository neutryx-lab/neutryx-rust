//! Curve fitting and regression analysis.
//!
//! This module provides algorithms for fitting curves and distributions
//! to data, including:
//!
//! - **Polynomial fitting**: Linear, quadratic, and higher-order polynomial fits
//! - **Weighted fitting**: Least squares with observation weights
//! - **Gaussian fitting**: Estimate parameters of Gaussian distributions
//!
//! ## Example
//!
//! ```ignore
//! use pricer_core::math::fitting::{linear_regression, fit_gaussian};
//!
//! // Linear regression
//! let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
//! let y = vec![2.1, 4.0, 5.9, 8.1, 9.9];
//! let result = linear_regression(&x, &y).unwrap();
//! println!("y = {} + {}x, R² = {}", result.params[0], result.params[1], result.r_squared);
//!
//! // Gaussian parameter estimation
//! let samples = vec![0.1, -0.2, 0.05, 0.15, -0.1];
//! let gauss = fit_gaussian(&samples).unwrap();
//! println!("Mean: {}, Std: {}", gauss.mean, gauss.std_dev);
//! ```

mod error;
mod gaussian;
mod least_squares;
mod result;

pub use error::FittingError;
pub use gaussian::{fit_gaussian, fit_gaussian_curve};
pub use least_squares::{linear_regression, polynomial_fit, weighted_polynomial_fit};
pub use result::{FittingResult, GaussianFitResult};
