//! Stochastic process models for Monte Carlo simulation.
//!
//! This module provides stochastic models with a flat structure and
//! trait markers for asset class categorisation.
//!
//! ## Model Categories (via trait markers)
//!
//! - `EquityModel`: GBM, Heston, SABR
//! - `RatesModel`: Hull-White, CIR, SABR
//! - `FxModel`: SABR (+ future FX-specific models)
//! - `HybridModel`: Correlated multi-factor
//!
//! ## Design Philosophy
//!
//! - Static dispatch via enum (not `Box<dyn Trait>`)
//! - Generic `Float` type for AD compatibility
//! - Smooth approximations for differentiability
//! - Trait markers allow a single model (e.g., SABR) to serve multiple asset classes
//!
//! ## Example
//!
//! ```
//! use pricer_models::models::{StochasticModelEnum, ModelParams, GBMParams};
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
pub mod model_enum;
pub mod stochastic;

// === Individual Models ===

#[cfg(feature = "equity")]
pub mod gbm;

#[cfg(feature = "equity")]
pub mod heston;

#[cfg(feature = "equity")]
pub mod sabr;

#[cfg(feature = "rates")]
pub mod hull_white;

#[cfg(feature = "rates")]
pub mod cir;

#[cfg(feature = "exotic")]
pub mod correlated;

// === Re-exports ===

pub use model_enum::{ModelParams, ModelState, StochasticModelEnum};
pub use stochastic::{
    EquityModel, FxModel, HybridModel, RatesModel, SingleState, StochasticModel, StochasticState,
    TwoFactorState,
};

#[cfg(feature = "equity")]
pub use gbm::{GBMModel, GBMParams};

#[cfg(feature = "equity")]
pub use heston::{HestonError, HestonModel, HestonParams};

#[cfg(feature = "equity")]
pub use sabr::{SABRError, SABRModel, SABRParams};

#[cfg(feature = "rates")]
pub use hull_white::{HullWhiteModel, HullWhiteParams, ThetaFunction};

#[cfg(feature = "rates")]
pub use cir::{CIRModel, CIRParams};

#[cfg(feature = "exotic")]
pub use correlated::{CholeskyFactor, CorrelatedModels, CorrelationError, CorrelationMatrix};
