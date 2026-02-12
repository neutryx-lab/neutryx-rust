//! Static dispatch enum for stochastic models.
//!
//! This module provides `StochasticModelEnum` for zero-cost abstraction over
//! different stochastic models (GBM, Heston, Hull-White, CIR). Using an enum
//! instead of trait objects ensures Enzyme LLVM compatibility and optimal
//! performance.
//!
//! ## Design Philosophy
//!
//! - **Static dispatch**: All model dispatch via `match` expressions
//! - **Zero-cost abstraction**: No vtable overhead
//! - **Enzyme-friendly**: Concrete types allow LLVM-level AD optimisation
//! - **All models included**: Interest rate, equity, and hybrid models
//!
//! ## Example
//!
//! ```
//! use pricer_models::stochastic::model_enum::StochasticModelEnum;
//! use pricer_models::stochastic::{GBMModel, GBMParams};
//!
//! // Create a GBM model wrapped in the enum
//! let model = StochasticModelEnum::<f64>::gbm();
//!
//! // Get model properties
//! assert_eq!(model.model_name(), "GBM");
//! assert_eq!(model.brownian_dim(), 1);
//! assert!(!model.is_two_factor());
//! ```

use pricer_core::traits::Float;

use super::{
    cir::{CIRModel, CIRParams},
    gbm::{GBMModel, GBMParams},
    heston::{HestonModel, HestonParams},
    hull_white::{HullWhiteModel, HullWhiteParams},
    stochastic::{SingleState, StochasticState, TwoFactorState},
};

/// Unified state type for all models.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModelState<T: Float> {
    /// Single-factor state (GBM, etc.)
    Single(SingleState<T>),
    /// Two-factor state (Heston, etc.)
    TwoFactor(TwoFactorState<T>),
}

impl<T: Float + Default> Default for ModelState<T> {
    fn default() -> Self { ModelState::Single(SingleState::default()) }
}

impl<T: Float + Default> ModelState<T> {
    /// Get state dimension.
    pub fn dimension(&self) -> usize {
        match self {
            Self::Single(_) => 1,
            Self::TwoFactor(_) => 2,
        }
    }

    /// Get state component by index.
    pub fn get(&self, index: usize) -> Option<T> {
        match self {
            Self::Single(s) => s.get(index),
            Self::TwoFactor(s) => s.get(index),
        }
    }

    /// Convert to vector representation.
    pub fn to_vec(&self) -> Vec<T> {
        match self {
            Self::Single(s) => s.to_array(),
            Self::TwoFactor(s) => s.to_array(),
        }
    }

    /// Get the price component (always first element).
    pub fn price(&self) -> T { self.get(0).unwrap_or(T::zero()) }

    /// Get variance component if available (second element for two-factor).
    pub fn variance(&self) -> Option<T> {
        match self {
            Self::Single(_) => None,
            Self::TwoFactor(s) => Some(s.second),
        }
    }
}

/// Unified parameter type for all stochastic models.
#[derive(Clone, Debug)]
pub enum ModelParams<T: Float> {
    /// GBM model parameters
    GBM(GBMParams<T>),
    /// Heston stochastic volatility model parameters
    Heston(HestonParams<T>),
    /// Hull-White model parameters
    HullWhite(HullWhiteParams<T>),
    /// CIR model parameters
    CIR(CIRParams<T>),
}

impl<T: Float> ModelParams<T> {
    /// Get the spot/initial price from parameters.
    ///
    /// For interest rate models, this returns the initial short rate.
    pub fn spot(&self) -> T {
        match self {
            ModelParams::GBM(p) => p.spot,
            ModelParams::Heston(p) => p.spot,
            ModelParams::HullWhite(p) => p.initial_short_rate,
            ModelParams::CIR(p) => p.initial_rate,
        }
    }

    /// Get the rate parameter (mean reversion speed for rate models).
    pub fn rate(&self) -> T {
        match self {
            ModelParams::GBM(p) => p.rate,
            ModelParams::Heston(p) => p.rate,
            ModelParams::HullWhite(p) => p.mean_reversion,
            ModelParams::CIR(p) => p.mean_reversion,
        }
    }

    /// Get the volatility parameter (sqrt(v0) for Heston).
    pub fn volatility(&self) -> T {
        match self {
            ModelParams::GBM(p) => p.volatility,
            ModelParams::Heston(p) => p.v0.sqrt(),
            ModelParams::HullWhite(p) => p.volatility,
            ModelParams::CIR(p) => p.volatility,
        }
    }

    /// Get initial variance for Heston model (returns None for other models).
    pub fn initial_variance(&self) -> Option<T> {
        match self {
            ModelParams::Heston(p) => Some(p.v0),
            _ => None,
        }
    }
}

/// Stochastic models ordered by increasing complexity (static dispatch for
/// Enzyme LLVM compatibility).
///
/// | Model | Factors | Type |
/// |-------|---------|------|
/// | `GBM` | 1 | Equity |
/// | `Heston` | 2 | Equity (SV) |
/// | `HullWhite` | 1 | Rates |
/// | `CIR` | 1 | Rates |
#[derive(Clone, Debug)]
pub enum StochasticModelEnum<T: Float> {
    /// Geometric Brownian Motion (1-factor, constant volatility).
    GBM(GBMModel<T>),
    /// Heston stochastic volatility model (2-factor, mean-reverting variance).
    Heston(HestonModel<T>),
    /// Hull-White one-factor model (rates, mean reversion to forward curve).
    HullWhite(HullWhiteModel<T>),
    /// Cox-Ingersoll-Ross model (rates, positive rates guaranteed).
    CIR(CIRModel<T>),
}

impl<T: Float + Default> Default for StochasticModelEnum<T> {
    fn default() -> Self { StochasticModelEnum::GBM(GBMModel::new()) }
}

/// Dispatches a `StochasticModel` associated function across all enum variants.
macro_rules! dispatch_assoc_fn {
    ($self:expr, $method:ident) => {
        match $self {
            Self::GBM(_) => GBMModel::<T>::$method(),
            Self::Heston(_) => HestonModel::<T>::$method(),
            Self::HullWhite(_) => HullWhiteModel::<T>::$method(),
            Self::CIR(_) => CIRModel::<T>::$method(),
        }
    };
}

impl<T: Float + Default> StochasticModelEnum<T> {
    /// Create a new GBM model.
    pub fn gbm() -> Self { Self::GBM(GBMModel::new()) }

    /// Create a new Heston model with given parameters.
    pub fn heston(params: HestonParams<T>) -> Option<Self> {
        HestonModel::new(params).ok().map(Self::Heston)
    }

    /// Create a new Hull-White model.
    pub fn hull_white() -> Self { Self::HullWhite(HullWhiteModel::new()) }

    /// Create a new CIR model.
    pub fn cir() -> Self { Self::CIR(CIRModel::new()) }

    /// Get the model name.
    pub fn model_name(&self) -> &'static str { dispatch_assoc_fn!(self, model_name) }

    /// Get the number of Brownian motion dimensions required.
    pub fn brownian_dim(&self) -> usize { dispatch_assoc_fn!(self, brownian_dim) }

    /// Check if this is a two-factor model.
    pub fn is_two_factor(&self) -> bool { matches!(self, Self::Heston(_)) }

    /// Check if this is an interest rate model.
    pub fn is_rate_model(&self) -> bool { matches!(self, Self::HullWhite(_) | Self::CIR(_)) }

    /// Get the number of stochastic factors in the model.
    pub fn num_factors(&self) -> usize { dispatch_assoc_fn!(self, num_factors) }

    /// Get initial state for the model.
    pub fn initial_state(&self, params: &ModelParams<T>) -> ModelState<T> {
        match (self, params) {
            (Self::GBM(_), ModelParams::GBM(p)) => ModelState::Single(GBMModel::initial_state(p)),
            (Self::Heston(_), ModelParams::Heston(p)) => {
                ModelState::TwoFactor(HestonModel::initial_state(p))
            }
            (Self::HullWhite(_), ModelParams::HullWhite(p)) => {
                ModelState::Single(HullWhiteModel::initial_state(p))
            }
            (Self::CIR(_), ModelParams::CIR(p)) => ModelState::Single(CIRModel::initial_state(p)),
            #[allow(unreachable_patterns)]
            _ => ModelState::default(),
        }
    }

    /// Evolve state by one time step.
    pub fn evolve_step(
        &self,
        state: ModelState<T>,
        dt: T,
        dw: &[T],
        params: &ModelParams<T>,
    ) -> ModelState<T> {
        match (self, &state, params) {
            (Self::GBM(_), ModelState::Single(s), ModelParams::GBM(p)) => {
                ModelState::Single(GBMModel::evolve_step(*s, dt, dw, p))
            }
            (Self::Heston(_), ModelState::TwoFactor(s), ModelParams::Heston(p)) => {
                ModelState::TwoFactor(HestonModel::evolve_step(*s, dt, dw, p))
            }
            (Self::HullWhite(_), ModelState::Single(s), ModelParams::HullWhite(p)) => {
                ModelState::Single(HullWhiteModel::evolve_step(*s, dt, dw, p))
            }
            (Self::CIR(_), ModelState::Single(s), ModelParams::CIR(p)) => {
                ModelState::Single(CIRModel::evolve_step(*s, dt, dw, p))
            }
            #[allow(unreachable_patterns)]
            _ => state,
        }
    }

    /// Generate a full path for Monte Carlo simulation.
    pub fn generate_path(
        &self,
        params: &ModelParams<T>,
        n_steps: usize,
        dt: T,
        randoms: &[T],
    ) -> Vec<ModelState<T>> {
        let brownian_dim = self.brownian_dim();
        assert!(
            randoms.len() >= n_steps * brownian_dim,
            "Insufficient random numbers for path generation"
        );

        let mut path = Vec::with_capacity(n_steps + 1);
        let mut state = self.initial_state(params);
        path.push(state);

        for step in 0..n_steps {
            let dw_start = step * brownian_dim;
            let dw = &randoms[dw_start..dw_start + brownian_dim];
            state = self.evolve_step(state, dt, dw, params);
            path.push(state);
        }

        path
    }
}

use super::stochastic::StochasticModel;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_enum_gbm_creation() {
        let model = StochasticModelEnum::<f64>::gbm();
        assert_eq!(model.model_name(), "GBM");
    }

    #[test]
    fn test_model_enum_default() {
        let model: StochasticModelEnum<f64> = Default::default();
        assert_eq!(model.model_name(), "GBM");
    }

    #[test]
    fn test_model_enum_brownian_dim() {
        let model = StochasticModelEnum::<f64>::gbm();
        assert_eq!(model.brownian_dim(), 1);
    }

    #[test]
    fn test_model_enum_is_two_factor() {
        let model = StochasticModelEnum::<f64>::gbm();
        assert!(!model.is_two_factor());
    }

    #[test]
    fn test_model_enum_num_factors() {
        let model = StochasticModelEnum::<f64>::gbm();
        assert_eq!(model.num_factors(), 1);
    }

    #[test]
    fn test_model_enum_initial_state() {
        let model = StochasticModelEnum::<f64>::gbm();
        let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());

        let state = model.initial_state(&params);
        assert_eq!(state.price(), 100.0);
        assert_eq!(state.variance(), None);
    }

    #[test]
    fn test_model_enum_evolve_step() {
        let model = StochasticModelEnum::<f64>::gbm();
        let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());

        let state = model.initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [0.0];

        let next_state = model.evolve_step(state, dt, &dw, &params);

        assert!((next_state.price() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_model_enum_generate_path() {
        let model = StochasticModelEnum::<f64>::gbm();
        let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());

        let n_steps = 10;
        let dt = 1.0 / 252.0;
        let randoms = vec![0.0; n_steps];

        let path = model.generate_path(&params, n_steps, dt, &randoms);

        assert_eq!(path.len(), n_steps + 1);
        assert_eq!(path[0].price(), 100.0);

        for state in &path {
            assert!(state.price() > 0.0);
            assert!(state.price().is_finite());
        }
    }

    #[test]
    fn test_model_state_dimension() {
        let single: ModelState<f64> = ModelState::Single(SingleState(100.0));
        assert_eq!(single.dimension(), 1);

        let two_factor: ModelState<f64> = ModelState::TwoFactor(TwoFactorState {
            first: 100.0,
            second: 0.04,
        });
        assert_eq!(two_factor.dimension(), 2);
    }

    #[test]
    fn test_model_state_get() {
        let single: ModelState<f64> = ModelState::Single(SingleState(100.0));
        assert_eq!(single.get(0), Some(100.0));
        assert_eq!(single.get(1), None);

        let two_factor: ModelState<f64> = ModelState::TwoFactor(TwoFactorState {
            first: 100.0,
            second: 0.04,
        });
        assert_eq!(two_factor.get(0), Some(100.0));
        assert_eq!(two_factor.get(1), Some(0.04));
        assert_eq!(two_factor.get(2), None);
    }

    #[test]
    fn test_model_state_to_vec() {
        let single: ModelState<f64> = ModelState::Single(SingleState(100.0));
        assert_eq!(single.to_vec(), vec![100.0]);

        let two_factor: ModelState<f64> = ModelState::TwoFactor(TwoFactorState {
            first: 100.0,
            second: 0.04,
        });
        assert_eq!(two_factor.to_vec(), vec![100.0, 0.04]);
    }

    #[test]
    fn test_model_state_price() {
        let single: ModelState<f64> = ModelState::Single(SingleState(100.0));
        assert_eq!(single.price(), 100.0);

        let two_factor: ModelState<f64> = ModelState::TwoFactor(TwoFactorState {
            first: 105.0,
            second: 0.04,
        });
        assert_eq!(two_factor.price(), 105.0);
    }

    #[test]
    fn test_model_state_variance() {
        let single: ModelState<f64> = ModelState::Single(SingleState(100.0));
        assert_eq!(single.variance(), None);

        let two_factor: ModelState<f64> = ModelState::TwoFactor(TwoFactorState {
            first: 100.0,
            second: 0.04,
        });
        assert_eq!(two_factor.variance(), Some(0.04));
    }

    #[test]
    fn test_model_params_accessors() {
        let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());
        assert_eq!(params.spot(), 100.0);
        assert_eq!(params.rate(), 0.05);
        assert_eq!(params.volatility(), 0.2);
    }

    #[test]
    fn test_model_enum_clone() {
        let model1 = StochasticModelEnum::<f64>::gbm();
        let model2 = model1.clone();
        assert_eq!(model1.model_name(), model2.model_name());
    }

    #[test]
    fn test_model_enum_pattern_matching() {
        let model = StochasticModelEnum::<f64>::gbm();

        match model {
            StochasticModelEnum::GBM(_gbm) => {
                assert!(true);
            }
            _ => {
                panic!("Expected GBM variant");
            }
        }
    }

    #[test]
    fn test_model_enum_heston_creation() {
        let heston_params =
            HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = StochasticModelEnum::<f64>::heston(heston_params);
        assert!(model.is_some());

        let model = model.unwrap();
        assert_eq!(model.model_name(), "Heston");
        assert_eq!(model.brownian_dim(), 2);
        assert_eq!(model.num_factors(), 2);
        assert!(model.is_two_factor());
        assert!(!model.is_rate_model());
    }

    mod rates_tests {
        use super::*;

        #[test]
        fn test_model_enum_hull_white_creation() {
            let model = StochasticModelEnum::<f64>::hull_white();
            assert_eq!(model.model_name(), "HullWhite1F");
            assert_eq!(model.brownian_dim(), 1);
            assert_eq!(model.num_factors(), 1);
            assert!(!model.is_two_factor());
            assert!(model.is_rate_model());
        }

        #[test]
        fn test_model_enum_cir_creation() {
            let model = StochasticModelEnum::<f64>::cir();
            assert_eq!(model.model_name(), "CIR");
            assert_eq!(model.brownian_dim(), 1);
            assert_eq!(model.num_factors(), 1);
            assert!(!model.is_two_factor());
            assert!(model.is_rate_model());
        }
    }
}
