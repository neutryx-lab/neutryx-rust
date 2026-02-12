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

/// Generates a PhantomData-based stochastic model struct with `StochasticModel` trait
/// implementation and a marker trait. Eliminates boilerplate for single-state models
/// (GBM, CIR, Hull-White) that share the same structural pattern.
macro_rules! define_phantom_model {
    (
        $(#[$model_meta:meta])*
        model $model_name:ident,
        params: $params_type:ty,
        state: $state_type:ty,
        marker: $marker_trait:ident,
        brownian_dim: $bdim:expr,
        num_factors: $nf:expr,
        name: $name_str:expr,
        evolve_step($st:ident, $dt:ident, $dw:ident, $p:ident) $evolve_body:block,
        initial_state($ip:ident) $init_body:block $(,)?
    ) => {
        $(#[$model_meta])*
        #[derive(Clone, Debug, Default)]
        pub struct $model_name<T: Float> {
            _phantom: std::marker::PhantomData<T>,
        }

        impl<T: Float> $model_name<T> {
            /// Create a new model instance.
            pub fn new() -> Self { Self { _phantom: std::marker::PhantomData } }
        }

        impl<T: Float> Differentiable for $model_name<T> {}

        impl<T: Float + Default> StochasticModel<T> for $model_name<T> {
            type State = $state_type;
            type Params = $params_type;

            fn evolve_step($st: Self::State, $dt: T, $dw: &[T], $p: &Self::Params) -> Self::State
                $evolve_body

            fn initial_state($ip: &Self::Params) -> Self::State
                $init_body

            fn brownian_dim() -> usize { $bdim }
            fn model_name() -> &'static str { $name_str }
            fn num_factors() -> usize { $nf }
        }

        impl<T: Float + Default> $marker_trait<T> for $model_name<T> {}
    };
}

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
