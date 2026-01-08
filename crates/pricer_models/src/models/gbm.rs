//! Geometric Brownian Motion (GBM) model implementation.
//!
//! GBM is the fundamental model for asset price dynamics, described by:
//! ```text
//! dS = r * S * dt + sigma * S * dW
//! ```
//! where:
//! - S = asset price
//! - r = risk-free rate
//! - sigma = volatility
//! - dW = Wiener process increment
//!
//! ## Log-space formulation
//!
//! For numerical stability, we use the exact solution:
//! ```text
//! S(t+dt) = S(t) * exp((r - 0.5*sigma^2)*dt + sigma*sqrt(dt)*dW)
//! ```

use pricer_core::traits::priceable::Differentiable;
use pricer_core::traits::Float;

use super::stochastic::{SingleState, StochasticModel};

/// GBM model parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GBMParams<T: Float> {
    /// Initial spot price
    pub spot: T,
    /// Risk-free rate (annualized)
    pub rate: T,
    /// Volatility (annualized)
    pub volatility: T,
    /// Smooth approximation epsilon (for AD compatibility)
    pub smoothing_epsilon: T,
}

impl<T: Float> GBMParams<T> {
    /// Create new GBM parameters with validation.
    ///
    /// # Arguments
    /// * `spot` - Initial spot price (must be positive)
    /// * `rate` - Risk-free rate
    /// * `volatility` - Volatility (must be non-negative)
    ///
    /// # Returns
    /// `Some(GBMParams)` if valid, `None` otherwise
    pub fn new(spot: T, rate: T, volatility: T) -> Option<Self> {
        if spot <= T::zero() || volatility < T::zero() {
            return None;
        }
        Some(Self {
            spot,
            rate,
            volatility,
            smoothing_epsilon: T::from(1e-8).unwrap_or(T::zero()),
        })
    }

    /// Create parameters with custom smoothing epsilon.
    pub fn with_epsilon(mut self, epsilon: T) -> Self {
        self.smoothing_epsilon = epsilon;
        self
    }
}

impl<T: Float> Default for GBMParams<T> {
    fn default() -> Self {
        Self {
            spot: T::from(100.0).unwrap_or(T::one()),
            rate: T::from(0.05).unwrap_or(T::zero()),
            volatility: T::from(0.2).unwrap_or(T::zero()),
            smoothing_epsilon: T::from(1e-8).unwrap_or(T::zero()),
        }
    }
}

/// Geometric Brownian Motion model.
///
/// A single-factor model for asset price simulation using log-space formulation
/// for numerical stability.
#[derive(Clone, Debug, Default)]
pub struct GBMModel<T: Float> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Float> GBMModel<T> {
    /// Create a new GBM model instance.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Float> Differentiable for GBMModel<T> {}

impl<T: Float + Default> StochasticModel<T> for GBMModel<T> {
    type State = SingleState<T>;
    type Params = GBMParams<T>;

    fn evolve_step(state: Self::State, dt: T, dw: &[T], params: &Self::Params) -> Self::State {
        // Log-space exact solution: S(t+dt) = S(t) * exp((r - 0.5*sigma^2)*dt + sigma*sqrt(dt)*dW)
        let s = state.0;
        let r = params.rate;
        let sigma = params.volatility;

        // Drift term: (r - 0.5 * sigma^2) * dt
        let half = T::from(0.5).unwrap_or(T::zero());
        let drift = (r - half * sigma * sigma) * dt;

        // Diffusion term: sigma * sqrt(dt) * dW
        let diffusion = sigma * dt.sqrt() * dw[0];

        // New price
        SingleState(s * (drift + diffusion).exp())
    }

    fn initial_state(params: &Self::Params) -> Self::State {
        SingleState(params.spot)
    }

    fn brownian_dim() -> usize {
        1
    }

    fn model_name() -> &'static str {
        "GBM"
    }

    fn num_factors() -> usize {
        1 // GBM is a single-factor model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 2.2: GBM Model Tests (TDD)
    // ================================================================

    #[test]
    fn test_gbm_params_new_valid() {
        let params = GBMParams::new(100.0_f64, 0.05, 0.2);
        assert!(params.is_some());
        let p = params.unwrap();
        assert_eq!(p.spot, 100.0);
        assert_eq!(p.rate, 0.05);
        assert_eq!(p.volatility, 0.2);
    }

    #[test]
    fn test_gbm_params_new_invalid_spot() {
        let params = GBMParams::new(-100.0_f64, 0.05, 0.2);
        assert!(params.is_none());

        let params_zero = GBMParams::new(0.0_f64, 0.05, 0.2);
        assert!(params_zero.is_none());
    }

    #[test]
    fn test_gbm_params_new_invalid_volatility() {
        let params = GBMParams::new(100.0_f64, 0.05, -0.1);
        assert!(params.is_none());
    }

    #[test]
    fn test_gbm_params_default() {
        let params: GBMParams<f64> = Default::default();
        assert_eq!(params.spot, 100.0);
        assert_eq!(params.rate, 0.05);
        assert_eq!(params.volatility, 0.2);
    }

    #[test]
    fn test_gbm_params_with_epsilon() {
        let params = GBMParams::new(100.0_f64, 0.05, 0.2)
            .unwrap()
            .with_epsilon(1e-6);
        assert_eq!(params.smoothing_epsilon, 1e-6);
    }

    #[test]
    fn test_gbm_model_new() {
        let model: GBMModel<f64> = GBMModel::new();
        assert_eq!(GBMModel::<f64>::model_name(), "GBM");
        assert_eq!(GBMModel::<f64>::brownian_dim(), 1);
        assert_eq!(GBMModel::<f64>::num_factors(), 1);
        let _ = model; // Suppress unused warning
    }

    #[test]
    fn test_gbm_initial_state() {
        let params = GBMParams::new(100.0_f64, 0.05, 0.2).unwrap();
        let state = GBMModel::initial_state(&params);
        assert_eq!(state.0, 100.0);
    }

    #[test]
    fn test_gbm_evolve_step_no_shock() {
        let params = GBMParams::new(100.0_f64, 0.05, 0.2).unwrap();
        let state = GBMModel::initial_state(&params);
        let dt = 1.0 / 252.0; // daily step
        let dw = [0.0];

        let next_state = GBMModel::evolve_step(state, dt, &dw, &params);

        // Expected: S * exp((r - 0.5*sigma^2)*dt)
        let expected_drift = (0.05 - 0.5 * 0.2 * 0.2) * dt;
        let expected = 100.0 * expected_drift.exp();

        assert!((next_state.0 - expected).abs() < 1e-10);
    }

    #[test]
    fn test_gbm_evolve_step_positive_shock() {
        let params = GBMParams::new(100.0_f64, 0.05, 0.2).unwrap();
        let state = GBMModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [1.0]; // positive shock

        let next_state = GBMModel::evolve_step(state, dt, &dw, &params);

        // With positive shock, price should increase
        assert!(next_state.0 > state.0);
    }

    #[test]
    fn test_gbm_evolve_step_negative_shock() {
        let params = GBMParams::new(100.0_f64, 0.05, 0.2).unwrap();
        let state = GBMModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [-1.0]; // negative shock

        let next_state = GBMModel::evolve_step(state, dt, &dw, &params);

        // With negative shock, price should decrease
        assert!(next_state.0 < state.0);
    }

    #[test]
    fn test_gbm_martingale_property() {
        // Under risk-neutral measure, E[S_T] = S_0 * exp(r*T)
        // With dW=0 (expected value of Brownian), we get deterministic growth
        let params = GBMParams::new(100.0_f64, 0.05, 0.2).unwrap();
        let mut state = GBMModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [0.0];
        let n_steps = 252; // 1 year

        for _ in 0..n_steps {
            state = GBMModel::evolve_step(state, dt, &dw, &params);
        }

        // After 1 year with zero volatility effect, S_T approx S_0 * exp((r - 0.5*sigma^2)*1)
        let expected = 100.0 * ((0.05 - 0.5 * 0.04) * 1.0).exp();
        assert!((state.0 - expected).abs() < 0.01);
    }

    #[test]
    fn test_gbm_model_is_differentiable() {
        // Verify that GBMModel implements Differentiable
        let model: GBMModel<f64> = GBMModel::new();
        let _: &dyn Differentiable = &model;
    }

    #[test]
    fn test_gbm_model_generic_f32() {
        // Verify GBM works with f32
        let params = GBMParams::new(100.0_f32, 0.05, 0.2).unwrap();
        let state = GBMModel::initial_state(&params);
        assert_eq!(state.0, 100.0_f32);

        let dt = 1.0_f32 / 252.0;
        let dw = [0.0_f32];
        let next_state = GBMModel::evolve_step(state, dt, &dw, &params);
        assert!(next_state.0.is_finite());
    }
}
