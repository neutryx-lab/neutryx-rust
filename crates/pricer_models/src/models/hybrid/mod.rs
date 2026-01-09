//! Hybrid and correlated stochastic models.
//!
//! This module provides models for multi-factor processes:
//! - [`CorrelatedModels`]: Correlate multiple stochastic models via Cholesky decomposition
//! - [`CorrelationMatrix`]: Validated correlation matrix with Cholesky factorization
//!
//! # Feature Flag
//!
//! This module is available when the `exotic` feature is enabled
//! (hybrid models are typically used for exotic derivatives).
//!
//! # Correlated Brownian Motions
//!
//! Given independent standard normals Z = [Z1, Z2, ..., Zn], generate
//! correlated normals W = L * Z using Cholesky decomposition where C = L * L^T.
//!
//! ## Example
//!
//! ```
//! use pricer_models::models::hybrid::{CorrelatedModels, CorrelationMatrix};
//!
//! // Create 2-factor correlation (rho = 0.5)
//! let matrix = CorrelationMatrix::new(&[
//!     1.0_f64, 0.5,
//!     0.5, 1.0,
//! ], 2).unwrap();
//!
//! // Create correlated models container
//! let models = CorrelatedModels::new(matrix).unwrap();
//!
//! // Transform independent normals to correlated
//! let independent = [0.5_f64, 0.8];
//! let correlated = models.correlate(&independent);
//! ```

pub mod correlated;

// Re-export main types
pub use correlated::{CholeskyFactor, CorrelatedModels, CorrelationError, CorrelationMatrix};
