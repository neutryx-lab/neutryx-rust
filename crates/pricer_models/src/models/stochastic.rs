//! StochasticModel trait for unified stochastic process interface.
//!
//! This module defines the core trait abstraction for stochastic process models
//! (GBM, Heston, SABR), enabling a consistent interface for path generation
//! and Monte Carlo simulations.
//!
//! ## Design Philosophy
//!
//! - **Static dispatch only**: Use enum-based dispatch, not `Box<dyn Trait>`
//! - **Generic Float type**: Supports f64 and DualNumber for AD compatibility
//! - **Differentiable marker**: All models must implement `Differentiable` trait
//!
//! ## Example
//!
//! ```
//! use pricer_models::models::stochastic::{StochasticModel, StochasticState};
//! use pricer_core::traits::Float;
//! use pricer_core::traits::priceable::Differentiable;
//!
//! // Implement for a simple GBM model
//! struct SimpleGBM<T: Float> {
//!     spot: T,
//!     rate: T,
//!     volatility: T,
//! }
//!
//! impl<T: Float> Differentiable for SimpleGBM<T> {}
//!
//! // Model implementation would follow...
//! ```

use pricer_core::traits::priceable::Differentiable;
use pricer_core::traits::Float;

/// State representation for stochastic models.
///
/// Different models have different state dimensions:
/// - GBM: Single value (price)
/// - Heston: Two values (price, variance)
/// - Multi-factor models: Multiple values
///
/// This trait provides a unified interface for state manipulation.
pub trait StochasticState<T: Float>: Clone + Copy + Default {
    /// Number of state variables
    fn dimension() -> usize;

    /// Get state component by index
    fn get(&self, index: usize) -> Option<T>;

    /// Set state component by index
    fn set(&mut self, index: usize, value: T) -> bool;

    /// Create state from slice
    fn from_slice(values: &[T]) -> Option<Self>;

    /// Convert state to array representation
    fn to_array(&self) -> Vec<T>;
}

/// Single-factor state (e.g., GBM price)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SingleState<T: Float>(pub T);

impl<T: Float + Default> StochasticState<T> for SingleState<T> {
    fn dimension() -> usize {
        1
    }

    fn get(&self, index: usize) -> Option<T> {
        if index == 0 {
            Some(self.0)
        } else {
            None
        }
    }

    fn set(&mut self, index: usize, value: T) -> bool {
        if index == 0 {
            self.0 = value;
            true
        } else {
            false
        }
    }

    fn from_slice(values: &[T]) -> Option<Self> {
        if !values.is_empty() {
            Some(SingleState(values[0]))
        } else {
            None
        }
    }

    fn to_array(&self) -> Vec<T> {
        vec![self.0]
    }
}

/// Two-factor state (e.g., Heston price and variance)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TwoFactorState<T: Float> {
    /// First state variable (typically price)
    pub first: T,
    /// Second state variable (typically variance)
    pub second: T,
}

impl<T: Float + Default> StochasticState<T> for TwoFactorState<T> {
    fn dimension() -> usize {
        2
    }

    fn get(&self, index: usize) -> Option<T> {
        match index {
            0 => Some(self.first),
            1 => Some(self.second),
            _ => None,
        }
    }

    fn set(&mut self, index: usize, value: T) -> bool {
        match index {
            0 => {
                self.first = value;
                true
            }
            1 => {
                self.second = value;
                true
            }
            _ => false,
        }
    }

    fn from_slice(values: &[T]) -> Option<Self> {
        if values.len() >= 2 {
            Some(TwoFactorState {
                first: values[0],
                second: values[1],
            })
        } else {
            None
        }
    }

    fn to_array(&self) -> Vec<T> {
        vec![self.first, self.second]
    }
}

/// Unified trait interface for stochastic process models.
///
/// This trait defines the core methods required for Monte Carlo simulation:
/// - `evolve_step`: Advance state by one time step
/// - `initial_state`: Get starting state from parameters
/// - `brownian_dim`: Number of Brownian motion increments needed
///
/// # Type Parameters
/// * `T` - Float type (f64 or DualNumber)
///
/// # Type Safety
/// - T: Float bound ensures numeric operations available
/// - State type is model-specific (e.g., f64 for GBM, (f64, f64) for Heston)
///
/// # Static Dispatch Only
/// **IMPORTANT**: Do NOT use `Box<dyn StochasticModel>`. Use enum-based dispatch
/// via `StochasticModelEnum` for Enzyme LLVM compatibility.
///
/// # Example
/// ```ignore
/// use pricer_models::models::stochastic::StochasticModel;
///
/// // Correct: enum-based dispatch
/// match model {
///     StochasticModelEnum::GBM(gbm) => gbm.evolve_step(state, dt, &dw),
///     StochasticModelEnum::Heston(heston) => heston.evolve_step(state, dt, &dw),
/// }
///
/// // WRONG: dynamic dispatch - incompatible with Enzyme
/// // let model: Box<dyn StochasticModel<f64>> = ...
/// ```
pub trait StochasticModel<T: Float>: Differentiable {
    /// Model-specific state type (GBM: `SingleState<T>`, Heston: `TwoFactorState<T>`)
    type State: StochasticState<T>;

    /// Model parameters type
    type Params: Clone;

    /// Evolve state by one time step.
    ///
    /// # Arguments
    /// * `state` - Current state
    /// * `dt` - Time step size (must be positive)
    /// * `dw` - Brownian motion increments (length must equal `brownian_dim()`)
    /// * `params` - Model parameters
    ///
    /// # Returns
    /// Next state after one time step
    ///
    /// # Preconditions
    /// - `dt > 0`
    /// - `dw.len() == Self::brownian_dim()`
    ///
    /// # Postconditions
    /// - Returned state is finite (no NaN/Inf)
    fn evolve_step(state: Self::State, dt: T, dw: &[T], params: &Self::Params) -> Self::State;

    /// Get initial state from model parameters.
    ///
    /// # Arguments
    /// * `params` - Model parameters containing initial values
    ///
    /// # Returns
    /// Initial state for simulation
    fn initial_state(params: &Self::Params) -> Self::State;

    /// Number of Brownian motion increments required per step.
    ///
    /// # Returns
    /// - 1 for single-factor models (GBM)
    /// - 2 for two-factor models (Heston with correlated Brownian)
    fn brownian_dim() -> usize;

    /// Model name for logging and debugging.
    fn model_name() -> &'static str;

    /// Number of stochastic factors in the model.
    ///
    /// This method returns the number of independent risk factors in the model:
    /// - 1 for single-factor models (GBM, Hull-White 1F, CIR)
    /// - 2 for two-factor models (G2++, Heston with separate volatility)
    /// - n for multi-factor models (LMM, hybrid models)
    ///
    /// # Returns
    ///
    /// Number of stochastic factors (must be >= 1)
    ///
    /// # Note
    ///
    /// This is distinct from `brownian_dim()` which returns the number of
    /// Brownian motion increments required. For correlated models, these
    /// may differ (e.g., Heston has 2 factors but may use 2 correlated Brownians).
    fn num_factors() -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 2.1: StochasticModel Trait Tests (TDD - RED phase)
    // ================================================================

    // Test: SingleState basic operations
    #[test]
    fn test_single_state_dimension() {
        assert_eq!(SingleState::<f64>::dimension(), 1);
    }

    #[test]
    fn test_single_state_get_set() {
        let mut state = SingleState(100.0_f64);
        assert_eq!(state.get(0), Some(100.0));
        assert_eq!(state.get(1), None);

        assert!(state.set(0, 105.0));
        assert_eq!(state.get(0), Some(105.0));
        assert!(!state.set(1, 0.0));
    }

    #[test]
    fn test_single_state_from_slice() {
        let state = SingleState::<f64>::from_slice(&[100.0]);
        assert!(state.is_some());
        assert_eq!(state.unwrap().0, 100.0);

        let empty = SingleState::<f64>::from_slice(&[]);
        assert!(empty.is_none());
    }

    #[test]
    fn test_single_state_to_array() {
        let state = SingleState(100.0_f64);
        let arr = state.to_array();
        assert_eq!(arr, vec![100.0]);
    }

    // Test: TwoFactorState basic operations
    #[test]
    fn test_two_factor_state_dimension() {
        assert_eq!(TwoFactorState::<f64>::dimension(), 2);
    }

    #[test]
    fn test_two_factor_state_get_set() {
        let mut state = TwoFactorState {
            first: 100.0_f64,
            second: 0.04_f64,
        };
        assert_eq!(state.get(0), Some(100.0));
        assert_eq!(state.get(1), Some(0.04));
        assert_eq!(state.get(2), None);

        assert!(state.set(0, 105.0));
        assert!(state.set(1, 0.05));
        assert_eq!(state.get(0), Some(105.0));
        assert_eq!(state.get(1), Some(0.05));
        assert!(!state.set(2, 0.0));
    }

    #[test]
    fn test_two_factor_state_from_slice() {
        let state = TwoFactorState::<f64>::from_slice(&[100.0, 0.04]);
        assert!(state.is_some());
        let s = state.unwrap();
        assert_eq!(s.first, 100.0);
        assert_eq!(s.second, 0.04);

        let short = TwoFactorState::<f64>::from_slice(&[100.0]);
        assert!(short.is_none());
    }

    #[test]
    fn test_two_factor_state_to_array() {
        let state = TwoFactorState {
            first: 100.0_f64,
            second: 0.04_f64,
        };
        let arr = state.to_array();
        assert_eq!(arr, vec![100.0, 0.04]);
    }

    // Test: StochasticModel trait implementation with mock model
    struct MockGBM {
        _smoothing_epsilon: f64,
    }

    impl Differentiable for MockGBM {}

    #[derive(Clone)]
    struct MockGBMParams {
        spot: f64,
        rate: f64,
        volatility: f64,
    }

    impl StochasticModel<f64> for MockGBM {
        type State = SingleState<f64>;
        type Params = MockGBMParams;

        fn evolve_step(
            state: Self::State,
            dt: f64,
            dw: &[f64],
            params: &Self::Params,
        ) -> Self::State {
            // Simple Euler discretization: S_t+dt = S_t * exp((r - 0.5*vol^2)*dt + vol*sqrt(dt)*dW)
            let s = state.0;
            let drift = (params.rate - 0.5 * params.volatility * params.volatility) * dt;
            let diffusion = params.volatility * dt.sqrt() * dw[0];
            SingleState(s * (drift + diffusion).exp())
        }

        fn initial_state(params: &Self::Params) -> Self::State {
            SingleState(params.spot)
        }

        fn brownian_dim() -> usize {
            1
        }

        fn model_name() -> &'static str {
            "MockGBM"
        }

        fn num_factors() -> usize {
            1
        }
    }

    #[test]
    fn test_stochastic_model_trait_initial_state() {
        let params = MockGBMParams {
            spot: 100.0,
            rate: 0.05,
            volatility: 0.2,
        };

        let state = MockGBM::initial_state(&params);
        assert_eq!(state.0, 100.0);
    }

    #[test]
    fn test_stochastic_model_trait_brownian_dim() {
        assert_eq!(MockGBM::brownian_dim(), 1);
    }

    #[test]
    fn test_stochastic_model_trait_model_name() {
        assert_eq!(MockGBM::model_name(), "MockGBM");
    }

    #[test]
    fn test_stochastic_model_trait_num_factors() {
        // MockGBM is a single-factor model
        assert_eq!(MockGBM::num_factors(), 1);
    }

    #[test]
    fn test_stochastic_model_trait_evolve_step() {
        let params = MockGBMParams {
            spot: 100.0,
            rate: 0.05,
            volatility: 0.2,
        };

        let state = MockGBM::initial_state(&params);
        let dt = 1.0 / 252.0; // daily step
        let dw = [0.0]; // no random shock

        let next_state = MockGBM::evolve_step(state, dt, &dw, &params);

        // With dW=0, price should increase by drift
        assert!(next_state.0 > state.0 * 0.99); // price should be close to original
        assert!(next_state.0 < state.0 * 1.01);
    }

    #[test]
    fn test_stochastic_model_trait_evolve_positive_shock() {
        let params = MockGBMParams {
            spot: 100.0,
            rate: 0.05,
            volatility: 0.2,
        };

        let state = MockGBM::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [1.0]; // positive shock

        let next_state = MockGBM::evolve_step(state, dt, &dw, &params);

        // With positive shock, price should increase
        assert!(next_state.0 > state.0);
    }

    #[test]
    fn test_stochastic_model_trait_evolve_negative_shock() {
        let params = MockGBMParams {
            spot: 100.0,
            rate: 0.05,
            volatility: 0.2,
        };

        let state = MockGBM::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [-1.0]; // negative shock

        let next_state = MockGBM::evolve_step(state, dt, &dw, &params);

        // With negative shock, price should decrease
        assert!(next_state.0 < state.0);
    }

    // Test: Differentiable marker trait requirement
    #[test]
    fn test_stochastic_model_requires_differentiable() {
        // This test verifies at compile time that StochasticModel requires Differentiable
        let _mock = MockGBM {
            _smoothing_epsilon: 1e-6,
        };
        // If this compiles, the trait bound is satisfied
    }

    // Test: Generic Float type support (compile-time verification)
    #[test]
    fn test_stochastic_model_generic_float() {
        // Verify that SingleState works with f32 as well
        let state_f32 = SingleState(100.0_f32);
        assert_eq!(state_f32.get(0), Some(100.0_f32));

        let state_f64 = SingleState(100.0_f64);
        assert_eq!(state_f64.get(0), Some(100.0_f64));
    }

    // Test: State is Clone + Copy (required for efficient simulation)
    #[test]
    fn test_state_clone_copy() {
        let state1 = SingleState(100.0_f64);
        let state2 = state1; // Copy
        let state3 = state1.clone(); // Clone

        assert_eq!(state1.0, state2.0);
        assert_eq!(state1.0, state3.0);

        let two_factor1 = TwoFactorState {
            first: 100.0_f64,
            second: 0.04,
        };
        let two_factor2 = two_factor1; // Copy
        let two_factor3 = two_factor1.clone(); // Clone

        assert_eq!(two_factor1.first, two_factor2.first);
        assert_eq!(two_factor1.first, two_factor3.first);
    }

    // Test: Default implementation for state types
    #[test]
    fn test_state_default() {
        let single: SingleState<f64> = Default::default();
        assert_eq!(single.0, 0.0);

        let two_factor: TwoFactorState<f64> = Default::default();
        assert_eq!(two_factor.first, 0.0);
        assert_eq!(two_factor.second, 0.0);
    }
}
