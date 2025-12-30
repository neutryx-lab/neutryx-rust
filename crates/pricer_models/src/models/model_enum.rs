//! Static dispatch enum for stochastic models.
//!
//! This module provides `StochasticModelEnum` for zero-cost abstraction over
//! different stochastic models (GBM, Heston, SABR). Using an enum instead of
//! trait objects ensures Enzyme LLVM compatibility and optimal performance.
//!
//! ## Design Philosophy
//!
//! - **Static dispatch**: All model dispatch via `match` expressions
//! - **Zero-cost abstraction**: No vtable overhead
//! - **Enzyme-friendly**: Concrete types allow LLVM-level AD optimization
//!
//! ## Example
//!
//! ```
//! use pricer_models::models::model_enum::StochasticModelEnum;
//! use pricer_models::models::gbm::{GBMModel, GBMParams};
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

use super::gbm::{GBMModel, GBMParams};
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
    fn default() -> Self {
        ModelState::Single(SingleState::default())
    }
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
    pub fn price(&self) -> T {
        self.get(0).unwrap_or(T::zero())
    }

    /// Get variance component if available (second element for two-factor).
    pub fn variance(&self) -> Option<T> {
        match self {
            ModelState::Single(_) => None,
            ModelState::TwoFactor(s) => Some(s.second),
        }
    }
}

/// Unified parameter type for all models.
///
/// This enum wraps model-specific parameter types.
#[derive(Clone, Debug)]
pub enum ModelParams<T: Float> {
    /// GBM model parameters
    GBM(GBMParams<T>),
    // Placeholder for future models:
    // Heston(HestonParams<T>),
    // SABR(SABRParams<T>),
}

impl<T: Float> ModelParams<T> {
    /// Get the spot/initial price from parameters.
    pub fn spot(&self) -> T {
        match self {
            ModelParams::GBM(p) => p.spot,
        }
    }

    /// Get the rate parameter.
    pub fn rate(&self) -> T {
        match self {
            ModelParams::GBM(p) => p.rate,
        }
    }

    /// Get the volatility parameter (primary volatility for all models).
    pub fn volatility(&self) -> T {
        match self {
            ModelParams::GBM(p) => p.volatility,
        }
    }
}

/// Static dispatch enum for stochastic models.
///
/// This enum enables zero-cost abstraction over different stochastic models.
/// Use this instead of `Box<dyn StochasticModel>` for Enzyme LLVM compatibility.
///
/// # Supported Models
///
/// - `GBM`: Geometric Brownian Motion (1-factor)
/// - `Heston`: Stochastic volatility model (2-factor) - placeholder
/// - `SABR`: SABR volatility model - placeholder
///
/// # Example
///
/// ```
/// use pricer_models::models::model_enum::StochasticModelEnum;
///
/// let model = StochasticModelEnum::<f64>::gbm();
///
/// match &model {
///     StochasticModelEnum::GBM(_) => println!("Using GBM model"),
/// }
/// ```
#[derive(Clone, Debug)]
pub enum StochasticModelEnum<T: Float> {
    /// Geometric Brownian Motion model
    GBM(GBMModel<T>),
    // Placeholder for future models (Task 3 and 4):
    // Heston(HestonModel<T>),
    // SABR(SABRModel<T>),
}

impl<T: Float + Default> Default for StochasticModelEnum<T> {
    fn default() -> Self {
        StochasticModelEnum::GBM(GBMModel::new())
    }
}

impl<T: Float + Default> StochasticModelEnum<T> {
    /// Create a new GBM model.
    pub fn gbm() -> Self {
        StochasticModelEnum::GBM(GBMModel::new())
    }

    // Placeholder constructors for future models:
    // pub fn heston(params: HestonParams<T>) -> Result<Self, HestonError> { ... }
    // pub fn sabr(params: SABRParams<T>) -> Result<Self, SABRError> { ... }

    /// Get the model name.
    pub fn model_name(&self) -> &'static str {
        match self {
            StochasticModelEnum::GBM(_) => GBMModel::<T>::model_name(),
        }
    }

    /// Get the number of Brownian motion dimensions required.
    pub fn brownian_dim(&self) -> usize {
        match self {
            StochasticModelEnum::GBM(_) => GBMModel::<T>::brownian_dim(),
        }
    }

    /// Check if this is a two-factor model.
    pub fn is_two_factor(&self) -> bool {
        match self {
            StochasticModelEnum::GBM(_) => false,
        }
    }

    /// Get initial state for the model.
    pub fn initial_state(&self, params: &ModelParams<T>) -> ModelState<T> {
        match (self, params) {
            (StochasticModelEnum::GBM(_), ModelParams::GBM(p)) => {
                ModelState::Single(GBMModel::initial_state(p))
            }
            #[allow(unreachable_patterns)]
            _ => ModelState::default(),
        }
    }

    /// Evolve state by one time step.
    ///
    /// # Arguments
    /// * `state` - Current model state
    /// * `dt` - Time step size
    /// * `dw` - Brownian motion increments
    /// * `params` - Model parameters
    ///
    /// # Returns
    /// Next state after one time step
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
            #[allow(unreachable_patterns)]
            _ => state, // Return unchanged state for mismatched types
        }
    }

    /// Generate a full path for Monte Carlo simulation.
    ///
    /// # Arguments
    /// * `params` - Model parameters
    /// * `n_steps` - Number of time steps
    /// * `dt` - Time step size
    /// * `randoms` - Random numbers (length must be n_steps * brownian_dim)
    ///
    /// # Returns
    /// Vector of states at each time step (including initial state)
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
    // Task 2.2: StochasticModelEnum Tests (TDD)
    // ================================================================

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
    fn test_model_enum_initial_state() {
        let model = StochasticModelEnum::<f64>::gbm();
        let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());

        let state = model.initial_state(&params);
        assert_eq!(state.price(), 100.0);
        assert_eq!(state.variance(), None); // GBM has no variance state
    }

    #[test]
    fn test_model_enum_evolve_step() {
        let model = StochasticModelEnum::<f64>::gbm();
        let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());

        let state = model.initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [0.0];

        let next_state = model.evolve_step(state, dt, &dw, &params);

        // Should be close to initial (small drift)
        assert!((next_state.price() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_model_enum_generate_path() {
        let model = StochasticModelEnum::<f64>::gbm();
        let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());

        let n_steps = 10;
        let dt = 1.0 / 252.0;
        let randoms = vec![0.0; n_steps]; // Zero shocks

        let path = model.generate_path(&params, n_steps, dt, &randoms);

        // Path should have n_steps + 1 elements (including initial state)
        assert_eq!(path.len(), n_steps + 1);

        // First state should be initial
        assert_eq!(path[0].price(), 100.0);

        // All prices should be positive and finite
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

        // Verify pattern matching works
        match model {
            StochasticModelEnum::GBM(_gbm) => {
                // GBM variant matched
                assert!(true);
            }
        }
    }

    #[test]
    fn test_model_enum_path_with_shocks() {
        let model = StochasticModelEnum::<f64>::gbm();
        let params = ModelParams::GBM(GBMParams::new(100.0, 0.05, 0.2).unwrap());

        let n_steps = 5;
        let dt = 1.0 / 252.0;

        // Positive shocks
        let randoms_up = vec![1.0; n_steps];
        let path_up = model.generate_path(&params, n_steps, dt, &randoms_up);

        // Negative shocks
        let randoms_down = vec![-1.0; n_steps];
        let path_down = model.generate_path(&params, n_steps, dt, &randoms_down);

        // With positive shocks, final price should be higher than with negative shocks
        assert!(path_up[n_steps].price() > path_down[n_steps].price());
    }

    #[test]
    fn test_model_enum_f32_support() {
        let model = StochasticModelEnum::<f32>::gbm();
        assert_eq!(model.model_name(), "GBM");

        let params = ModelParams::GBM(GBMParams::new(100.0_f32, 0.05, 0.2).unwrap());
        let state = model.initial_state(&params);
        assert_eq!(state.price(), 100.0_f32);
    }
}
