//! Equity stochastic models.
//!
//! This module provides stochastic models for equity price processes:
//! - Geometric Brownian Motion (GBM)
//! - Heston stochastic volatility model
//! - SABR stochastic volatility model
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

// Equity model implementations
pub mod gbm;
pub mod heston;
pub mod sabr;

// Re-exports for convenience
pub use gbm::{GBMModel, GBMParams};
pub use heston::{HestonError, HestonModel, HestonParams};
pub use sabr::{SABRError, SABRModel, SABRParams};
