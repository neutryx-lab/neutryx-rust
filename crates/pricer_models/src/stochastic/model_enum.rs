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
//! - **Feature-gated models**: Interest rate models require `rates` feature
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

#[cfg(feature = "rates")]
use super::cir::{CIRModel, CIRParams};
#[cfg(feature = "equity")]
use super::gbm::{GBMModel, GBMParams};
#[cfg(feature = "equity")]
use super::heston::{HestonModel, HestonParams};
#[cfg(feature = "rates")]
use super::hull_white::{HullWhiteModel, HullWhiteParams};
use super::stochastic::{SingleState, StochasticState, TwoFactorState};

/// Unified state type for all models.
///
/// This enum wraps model-specific state types, allowing uniform handling
/// of simulation results across different models.
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
            ModelState::Single(_) => SingleState::<T>::dimension(),
            ModelState::TwoFactor(_) => TwoFactorState::<T>::dimension(),
        }
    }

    /// Get state component by index.
    pub fn get(&self, index: usize) -> Option<T> {
        match self {
            ModelState::Single(s) => s.get(index),
            ModelState::TwoFactor(s) => s.get(index),
        }
    }

    /// Convert to vector representation.
    pub fn to_vec(&self) -> Vec<T> {
        match self {
            ModelState::Single(s) => s.to_array(),
            ModelState::TwoFactor(s) => s.to_array(),
        }
    }

    /// Get the price component (always first element).
    pub fn price(&self) -> T { self.get(0).unwrap_or(T::zero()) }

    /// Get variance component if available (second element for two-factor).
    pub fn variance(&self) -> Option<T> {
        match self {
            ModelState::Single(_) => None,
            ModelState::TwoFactor(s) => Some(s.second),
        }
    }
}

/// Unified parameter type for all stochastic models.
///
/// This enum wraps model-specific parameter types.
#[derive(Clone, Debug)]
pub enum ModelParams<T: Float> {
    /// GBM model parameters
    #[cfg(feature = "equity")]
    GBM(GBMParams<T>),
    /// Heston stochastic volatility model parameters
    #[cfg(feature = "equity")]
    Heston(HestonParams<T>),
    /// Hull-White model parameters (requires `rates` feature)
    #[cfg(feature = "rates")]
    HullWhite(HullWhiteParams<T>),
    /// CIR model parameters (requires `rates` feature)
    #[cfg(feature = "rates")]
    CIR(CIRParams<T>),
}

impl<T: Float> ModelParams<T> {
    /// Get the spot/initial price from parameters.
    ///
    /// For interest rate models, this returns the initial short rate.
    #[cfg(feature = "equity")]
    pub fn spot(&self) -> T {
        match self {
            ModelParams::GBM(p) => p.spot,
            ModelParams::Heston(p) => p.spot,
            #[cfg(feature = "rates")]
            ModelParams::HullWhite(p) => p.initial_short_rate,
            #[cfg(feature = "rates")]
            ModelParams::CIR(p) => p.initial_rate,
        }
    }

    /// Get the rate parameter.
    ///
    /// For interest rate models, this returns the mean reversion speed.
    /// For Heston, this returns the risk-free rate.
    #[cfg(feature = "equity")]
    pub fn rate(&self) -> T {
        match self {
            ModelParams::GBM(p) => p.rate,
            ModelParams::Heston(p) => p.rate,
            #[cfg(feature = "rates")]
            ModelParams::HullWhite(p) => p.mean_reversion,
            #[cfg(feature = "rates")]
            ModelParams::CIR(p) => p.mean_reversion,
        }
    }

    /// Get the volatility parameter (primary volatility for all models).
    ///
    /// For Heston, this returns the initial volatility (sqrt of v0).
    #[cfg(feature = "equity")]
    pub fn volatility(&self) -> T {
        match self {
            ModelParams::GBM(p) => p.volatility,
            ModelParams::Heston(p) => p.v0.sqrt(),
            #[cfg(feature = "rates")]
            ModelParams::HullWhite(p) => p.volatility,
            #[cfg(feature = "rates")]
            ModelParams::CIR(p) => p.volatility,
        }
    }

    /// Get initial variance for Heston model (returns None for other models).
    #[cfg(feature = "equity")]
    pub fn initial_variance(&self) -> Option<T> {
        match self {
            ModelParams::Heston(p) => Some(p.v0),
            _ => None,
        }
    }
}

/// Stochastic models ordered by increasing complexity.
///
/// This enum enables zero-cost abstraction over different stochastic models.
/// Use this instead of `Box<dyn StochasticModel>` for Enzyme LLVM
/// compatibility.
///
/// # Ordering Rationale
///
/// Variants are ordered by model complexity (simplest to most specialised):
///
/// - **Level 1 (Basic)**: `GBM` - 1-factor, constant volatility
/// - **Level 2 (Intermediate)**: `Heston` - 2-factor, stochastic vol
/// - **Level 3 (Specialised)**: `HullWhite`, `CIR` - Rate models with mean
///   reversion
///
/// This ordering helps users understand model sophistication and choose
/// appropriately for their use case.
///
/// # Supported Models
///
/// | Model | Factors | Type | Feature |
/// |-------|---------|------|---------|
/// | `GBM` | 1 | Equity | default |
/// | `Heston` | 2 | Equity (SV) | default |
/// | `HullWhite` | 1 | Rates | `rates` |
/// | `CIR` | 1 | Rates | `rates` |
///
/// # Adding New Models
///
/// When adding new models, place them according to complexity:
/// - Simple 1-factor models near `GBM`
/// - Stochastic volatility models near `Heston`
/// - Rate models near `HullWhite`/`CIR`
///
/// # Example
///
/// ```
/// use pricer_models::stochastic::model_enum::StochasticModelEnum;
///
/// let model = StochasticModelEnum::<f64>::gbm();
///
/// match &model {
///     StochasticModelEnum::GBM(_) => println!("Using GBM model"),
///     StochasticModelEnum::Heston(_) => println!("Using Heston model"),
///     #[cfg(feature = "rates")]
///     _ => println!("Using rate model"),
/// }
/// ```
#[derive(Clone, Debug)]
pub enum StochasticModelEnum<T: Float> {
    // === Level 1: Basic (1-factor, constant parameters) ===
    /// Geometric Brownian Motion (simplest, 1-factor, constant volatility).
    /// Complexity level: 1 (baseline).
    #[cfg(feature = "equity")]
    GBM(GBMModel<T>),

    // === Level 2: Intermediate (2-factor, stochastic volatility) ===
    /// Heston stochastic volatility model (2-factor, mean-reverting variance).
    /// Complexity level: 2 (intermediate).
    #[cfg(feature = "equity")]
    Heston(HestonModel<T>),

    // === Level 3: Specialised (rate models with mean reversion) ===
    /// Hull-White one-factor model (rates, mean reversion to forward curve).
    /// Complexity level: 3 (specialised). Requires `rates` feature.
    #[cfg(feature = "rates")]
    HullWhite(HullWhiteModel<T>),
    /// Cox-Ingersoll-Ross model (rates, positive rates guaranteed).
    /// Complexity level: 3 (specialised). Requires `rates` feature.
    #[cfg(feature = "rates")]
    CIR(CIRModel<T>),
}

#[cfg(feature = "equity")]
impl<T: Float + Default> Default for StochasticModelEnum<T> {
    fn default() -> Self { StochasticModelEnum::GBM(GBMModel::new()) }
}

impl<T: Float + Default> StochasticModelEnum<T> {
    /// Create a new GBM model.
    #[cfg(feature = "equity")]
    pub fn gbm() -> Self { StochasticModelEnum::GBM(GBMModel::new()) }

    /// Create a new Heston model with given parameters.
    ///
    /// # Arguments
    /// * `params` - Heston model parameters
    ///
    /// # Returns
    /// `Some(StochasticModelEnum::Heston)` if parameters are valid, `None`
    /// otherwise
    #[cfg(feature = "equity")]
    pub fn heston(params: HestonParams<T>) -> Option<Self> {
        HestonModel::new(params)
            .ok()
            .map(StochasticModelEnum::Heston)
    }

    /// Create a new Hull-White model (requires `rates` feature).
    #[cfg(feature = "rates")]
    pub fn hull_white() -> Self { StochasticModelEnum::HullWhite(HullWhiteModel::new()) }

    /// Create a new CIR model (requires `rates` feature).
    #[cfg(feature = "rates")]
    pub fn cir() -> Self { StochasticModelEnum::CIR(CIRModel::new()) }

    /// Get the model name.
    pub fn model_name(&self) -> &'static str {
        match self {
            #[cfg(feature = "equity")]
            StochasticModelEnum::GBM(_) => GBMModel::<T>::model_name(),
            #[cfg(feature = "equity")]
            StochasticModelEnum::Heston(_) => HestonModel::<T>::model_name(),
            #[cfg(feature = "rates")]
            StochasticModelEnum::HullWhite(_) => HullWhiteModel::<T>::model_name(),
            #[cfg(feature = "rates")]
            StochasticModelEnum::CIR(_) => CIRModel::<T>::model_name(),
        }
    }

    /// Get the number of Brownian motion dimensions required.
    pub fn brownian_dim(&self) -> usize {
        match self {
            #[cfg(feature = "equity")]
            StochasticModelEnum::GBM(_) => GBMModel::<T>::brownian_dim(),
            #[cfg(feature = "equity")]
            StochasticModelEnum::Heston(_) => HestonModel::<T>::brownian_dim(),
            #[cfg(feature = "rates")]
            StochasticModelEnum::HullWhite(_) => HullWhiteModel::<T>::brownian_dim(),
            #[cfg(feature = "rates")]
            StochasticModelEnum::CIR(_) => CIRModel::<T>::brownian_dim(),
        }
    }

    /// Check if this is a two-factor model.
    pub fn is_two_factor(&self) -> bool {
        match self {
            #[cfg(feature = "equity")]
            StochasticModelEnum::GBM(_) => false,
            #[cfg(feature = "equity")]
            StochasticModelEnum::Heston(_) => true,
            #[cfg(feature = "rates")]
            StochasticModelEnum::HullWhite(_) => false,
            #[cfg(feature = "rates")]
            StochasticModelEnum::CIR(_) => false,
        }
    }

    /// Check if this is an interest rate model.
    pub fn is_rate_model(&self) -> bool {
        match self {
            #[cfg(feature = "equity")]
            StochasticModelEnum::GBM(_) => false,
            #[cfg(feature = "equity")]
            StochasticModelEnum::Heston(_) => false,
            #[cfg(feature = "rates")]
            StochasticModelEnum::HullWhite(_) => true,
            #[cfg(feature = "rates")]
            StochasticModelEnum::CIR(_) => true,
        }
    }

    /// Get the number of stochastic factors in the model.
    pub fn num_factors(&self) -> usize {
        match self {
            #[cfg(feature = "equity")]
            StochasticModelEnum::GBM(_) => GBMModel::<T>::num_factors(),
            #[cfg(feature = "equity")]
            StochasticModelEnum::Heston(_) => HestonModel::<T>::num_factors(),
            #[cfg(feature = "rates")]
            StochasticModelEnum::HullWhite(_) => HullWhiteModel::<T>::num_factors(),
            #[cfg(feature = "rates")]
            StochasticModelEnum::CIR(_) => CIRModel::<T>::num_factors(),
        }
    }

    /// Get initial state for the model.
    #[cfg(feature = "equity")]
    pub fn initial_state(&self, params: &ModelParams<T>) -> ModelState<T> {
        match (self, params) {
            (StochasticModelEnum::GBM(_), ModelParams::GBM(p)) => {
                ModelState::Single(GBMModel::initial_state(p))
            }
            (StochasticModelEnum::Heston(_), ModelParams::Heston(p)) => {
                ModelState::TwoFactor(HestonModel::initial_state(p))
            }
            #[cfg(feature = "rates")]
            (StochasticModelEnum::HullWhite(_), ModelParams::HullWhite(p)) => {
                ModelState::Single(HullWhiteModel::initial_state(p))
            }
            #[cfg(feature = "rates")]
            (StochasticModelEnum::CIR(_), ModelParams::CIR(p)) => {
                ModelState::Single(CIRModel::initial_state(p))
            }
            #[allow(unreachable_patterns)]
            _ => ModelState::default(),
        }
    }

    /// Evolve state by one time step.
    #[cfg(feature = "equity")]
    pub fn evolve_step(
        &self,
        state: ModelState<T>,
        dt: T,
        dw: &[T],
        params: &ModelParams<T>,
    ) -> ModelState<T> {
        match (self, &state, params) {
            (StochasticModelEnum::GBM(_), ModelState::Single(s), ModelParams::GBM(p)) => {
                ModelState::Single(GBMModel::evolve_step(*s, dt, dw, p))
            }
            (StochasticModelEnum::Heston(_), ModelState::TwoFactor(s), ModelParams::Heston(p)) => {
                ModelState::TwoFactor(HestonModel::evolve_step(*s, dt, dw, p))
            }
            #[cfg(feature = "rates")]
            (
                StochasticModelEnum::HullWhite(_),
                ModelState::Single(s),
                ModelParams::HullWhite(p),
            ) => ModelState::Single(HullWhiteModel::evolve_step(*s, dt, dw, p)),
            #[cfg(feature = "rates")]
            (StochasticModelEnum::CIR(_), ModelState::Single(s), ModelParams::CIR(p)) => {
                ModelState::Single(CIRModel::evolve_step(*s, dt, dw, p))
            }
            #[allow(unreachable_patterns)]
            _ => state,
        }
    }

    /// Generate a full path for Monte Carlo simulation.
    #[cfg(feature = "equity")]
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

// Import StochasticModel trait for use in implementations
use super::stochastic::StochasticModel;

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // StochasticModelEnum Tests
    // ================================================================

    #[cfg(feature = "equity")]
    #[test]
    fn test_model_enum_gbm_creation() {
        let model = StochasticModelEnum::<f64>::gbm();
        assert_eq!(model.model_name(), "GBM");
    }

    #[cfg(feature = "equity")]
    #[test]
    fn test_model_enum_default() {
        let model: StochasticModelEnum<f64> = Default::default();
        assert_eq!(model.model_name(), "GBM");
    }

    #[cfg(feature = "equity")]
    #[test]
    fn test_model_enum_brownian_dim() {
        let model = StochasticModelEnum::<f64>::gbm();
        assert_eq!(model.brownian_dim(), 1);
    }

    #[cfg(feature = "equity")]
    #[test]
    fn test_model_enum_is_two_factor() {
        let model = StochasticModelEnum::<f64>::gbm();
        assert!(!model.is_two_factor());
    }

    #[cfg(feature = "equity")]
    #[test]
    fn test_model_enum_num_factors() {
        let model = StochasticModelEnum::<f64>::gbm();
        assert_eq!(model.num_factors(), 1);
    }

    #[cfg(feature = "equity")]
    #[test]
    fn test_model_enum_initial_state() {
        let model = StochasticModelEnum::<f64>::gbm();
        let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());

        let state = model.initial_state(&params);
        assert_eq!(state.price(), 100.0);
        assert_eq!(state.variance(), None);
    }

    #[cfg(feature = "equity")]
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

    #[cfg(feature = "equity")]
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

    #[cfg(feature = "equity")]
    #[test]
    fn test_model_params_accessors() {
        let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());
        assert_eq!(params.spot(), 100.0);
        assert_eq!(params.rate(), 0.05);
        assert_eq!(params.volatility(), 0.2);
    }

    #[cfg(feature = "equity")]
    #[test]
    fn test_model_enum_clone() {
        let model1 = StochasticModelEnum::<f64>::gbm();
        let model2 = model1.clone();
        assert_eq!(model1.model_name(), model2.model_name());
    }

    #[cfg(feature = "equity")]
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

    #[cfg(feature = "equity")]
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

    #[cfg(feature = "rates")]
    mod rates_tests {
        use super::*;
        use crate::market::curves::FlatCurve;

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
