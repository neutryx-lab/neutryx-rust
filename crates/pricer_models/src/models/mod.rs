//! Stochastic process models (GBM, Heston, etc.).
//!
//! This module provides stochastic models for Monte Carlo simulation:
//! - `StochasticModel` trait: Unified interface for all models
//! - `StochasticModelEnum`: Static dispatch enum for Enzyme compatibility
//! - `GBMModel`: Geometric Brownian Motion model
//!
//! ## Design Philosophy
//!
//! All models use:
//! - Static dispatch via enum (not `Box<dyn Trait>`)
//! - Generic `Float` type for AD compatibility
//! - Smooth approximations for differentiability
//!
//! ## Example
//!
//! ```
//! use pricer_models::models::{StochasticModelEnum, ModelParams, GBMParams};
//!
//! // Create a GBM model via the enum
//! let model = StochasticModelEnum::<f64>::gbm();
//!
//! // Create parameters
//! let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());
//!
//! // Generate a path
//! let n_steps = 10;
//! let dt = 1.0 / 252.0;
//! let randoms = vec![0.0; n_steps];
//! let path = model.generate_path(&params, n_steps, dt, &randoms);
//! ```

pub mod gbm;
pub mod model_enum;
pub mod stochastic;

// Re-export core trait types
pub use stochastic::{SingleState, StochasticModel, StochasticState, TwoFactorState};

// Re-export GBM model
pub use gbm::{GBMModel, GBMParams};

// Re-export enum types for static dispatch
pub use model_enum::{ModelParams, ModelState, StochasticModelEnum};
