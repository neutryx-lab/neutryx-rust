//! Probability distributions for financial calculations.
//!
//! This module provides probability distribution functions commonly used
//! in quantitative finance, including normal distributions for option pricing,
//! bivariate normal for correlation-dependent products, and non-central
//! chi-squared for interest rate models.
//!
//! All functions are generic over `T: Float` to support both `f64` and
//! dual number types for automatic differentiation compatibility.
//!
//! ## Available Distributions
//!
//! - **Normal**: Standard normal CDF, PDF, and inverse CDF (quantile)
//! - **Bivariate Normal**: CDF for two correlated normal variables
//! - **Non-central Chi-Squared**: CDF for CIR model bond pricing
//! - **Gaussian Copula**: Joint probability from marginals and correlation
//!
//! ## Precision Guarantees
//!
//! - Normal CDF: relative error < 1e-15 (Hart approximation)
//! - Normal inverse CDF: relative error < 1e-9 (Acklam approximation)
//! - Bivariate Normal CDF: relative error < 1e-10 (Drezner-Wesolowsky)
//! - Non-central Chi-Squared CDF: relative error < 1e-8
//!
//! ## Example
//!
//! ```
//! use pricer_core::math::distributions::{norm_cdf, norm_pdf, norm_inv_cdf};
//!
//! let x = 0.5_f64;
//! let cdf = norm_cdf(x);
//! let pdf = norm_pdf(x);
//! let p = 0.5_f64;
//! let quantile = norm_inv_cdf(p).unwrap();
//!
//! assert!((cdf - 0.6914624612740131).abs() < 1e-10);
//! assert!((quantile - 0.0).abs() < 1e-10);
//! ```

mod error;
mod normal;

pub use error::DistributionError;
pub use normal::{norm_cdf, norm_inv_cdf, norm_pdf};
