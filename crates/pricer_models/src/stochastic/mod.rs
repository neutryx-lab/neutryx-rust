//! Stochastic process models for Monte Carlo simulation.
//!
//! This module provides stochastic models with a flat structure and
//! trait markers for asset class categorisation.
//!
//! ## Model Categories (via trait markers)
//!
//! - `EquityModel`: GBM, Heston
//! - `RatesModel`: Hull-White, CIR
//! - `HybridModel`: Correlated multi-factor
//!
//! ## Design Philosophy
//!
//! - Static dispatch via enum (not `Box<dyn Trait>`)
//! - Generic `Float` type for AD compatibility
//! - Smooth approximations for differentiability
//! - Trait markers allow categorisation by asset class
//!
//! ## Example
//!
//! ```
//! use pricer_models::stochastic::{StochasticModelEnum, ModelParams, GBMParams};
//!
//! let model = StochasticModelEnum::<f64>::gbm();
//! let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());
//!
//! let n_steps = 10;
//! let dt = 1.0 / 252.0;
//! let randoms = vec![0.0; n_steps];
//! let path = model.generate_path(&params, n_steps, dt, &randoms);
//! ```

// Core infrastructure
pub mod error;
pub mod model_enum;
pub mod stochastic;
pub mod validation;

// === Individual Models ===

pub mod gbm;

pub mod heston;

pub mod hull_white;

pub mod cir;

pub mod correlated;

// === Re-exports ===

pub use cir::{CIRModel, CIRParams};
pub use correlated::{CholeskyFactor, CorrelatedModels, CorrelationError, CorrelationMatrix};
pub use error::ModelError;
pub use gbm::{GBMModel, GBMParams};
pub use heston::{HestonError, HestonModel, HestonParams};
pub use hull_white::{HullWhiteModel, HullWhiteParams, ThetaFunction};
pub use model_enum::{ModelParams, ModelState, StochasticModelEnum};
pub use stochastic::{
    EquityModel, FxModel, HybridModel, RatesModel, SingleState, StochasticModel, StochasticState,
    TwoFactorState,
};
