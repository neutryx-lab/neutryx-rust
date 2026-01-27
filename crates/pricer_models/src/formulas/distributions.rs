//! Standard normal distribution functions.
//!
//! This module re-exports the high-precision implementations from
//! `pricer_core::math::distributions`.
//!
//! Available functions:
//! - [`norm_cdf`]: Cumulative distribution function (CDF) - Φ(x)
//! - [`norm_pdf`]: Probability density function (PDF) - φ(x)
//! - [`norm_inv_cdf`]: Inverse CDF (quantile function) - Φ⁻¹(p)
//!
//! All functions are generic over `T: Float` to support both `f64` and `Dual64`
//! for automatic differentiation.
//!
//! # Example
//!
//! ```
//! use pricer_models::formulas::distributions::{norm_cdf, norm_pdf};
//!
//! let cdf_0 = norm_cdf(0.0_f64);
//! assert!((cdf_0 - 0.5).abs() < 1e-7);
//!
//! let pdf_0 = norm_pdf(0.0_f64);
//! // φ(0) = 1 / sqrt(2π) ≈ 0.3989
//! assert!((pdf_0 - 0.3989422804).abs() < 1e-7);
//! ```

// Re-export from pricer_core for centralised implementation
pub use pricer_core::math::distributions::{norm_cdf, norm_inv_cdf, norm_pdf};
