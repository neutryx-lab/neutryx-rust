//! Stochastic process models (GBM, Heston, etc.).
//!
//! This module provides stochastic models for Monte Carlo simulation:
//! - `StochasticModel` trait: Unified interface for all models
//! - `StochasticModelEnum`: Static dispatch enum for Enzyme compatibility
//! - `GBMModel`: Geometric Brownian Motion model
//! - `HestonModel`: Heston stochastic volatility model
//!
//! ## Model Categories
//!
//! Models are organised by category (enabled via feature flags):
//! - `equity`: Equity models (GBM, Heston, SABR) - default
//! - `rates`: Interest rate models (Hull-White, CIR)
//! - `exotic`: Advanced models including hybrid multi-factor models
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

// Core model infrastructure (always available)
pub mod model_enum;
pub mod stochastic;

// Model category submodules (feature-gated)
#[cfg(feature = "equity")]
pub mod equity;

#[cfg(feature = "rates")]
pub mod rates;

#[cfg(feature = "exotic")]
pub mod hybrid;

// Re-export core trait types
// Re-export equity models from equity/ submodule for backward compatibility
#[cfg(feature = "equity")]
pub use equity::{
    gbm::{GBMModel, GBMParams},
    heston::{HestonError, HestonModel, HestonParams},
    sabr::{SABRError, SABRModel, SABRParams},
};
pub use model_enum::{ModelParams, ModelState, StochasticModelEnum};
pub use stochastic::{SingleState, StochasticModel, StochasticState, TwoFactorState};
