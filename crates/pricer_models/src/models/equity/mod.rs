//! Equity stochastic models.
//!
//! This module provides stochastic models for equity price processes:
//! - Geometric Brownian Motion (GBM)
//!
//! # Feature Flag
//!
//! This module is available when the `equity` feature is enabled (default).
//!
//! # Examples
//!
//! ```
//! use pricer_models::models::equity::GBMModel;
//! use pricer_models::models::{StochasticModel, GBMParams, ModelParams};
//!
//! // Create a GBM model
//! let model = GBMModel::<f64>::new();
//! let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());
//! ```

// Re-export GBM from parent module
pub use super::gbm::{GBMModel, GBMParams};
