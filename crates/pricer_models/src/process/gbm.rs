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

use pricer_core::traits::{priceable::Differentiable, Float};

use crate::{
    process::{
        stochastic::{EquityModel, SingleState, StochasticModel},
        validation::{ParamValidationError, DEFAULT_SMOOTHING_EPSILON},
    },
    validate_params,
};

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
    pub fn new(spot: T, rate: T, volatility: T) -> Option<Self> {
        let params = Self {
            spot,
            rate,
            volatility,
            smoothing_epsilon: T::from(DEFAULT_SMOOTHING_EPSILON).unwrap_or(T::zero()),
        };
        params.validate().ok()?;
        Some(params)
    }

    /// Validate GBM parameters.
    pub fn validate(&self) -> Result<(), ParamValidationError> {
        validate_params! {
            self_val = self, f64_conv = |v: T| v.to_f64().unwrap_or(f64::NAN),
            positive: [spot],
            non_negative: [volatility],
        }
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
            smoothing_epsilon: T::from(DEFAULT_SMOOTHING_EPSILON).unwrap_or(T::zero()),
        }
    }
}

define_phantom_model! {
    /// Geometric Brownian Motion model (log-space formulation for numerical stability).
    model GBMModel,
    params: GBMParams<T>,
    state: SingleState<T>,
    marker: EquityModel,
    brownian_dim: 1,
    num_factors: 1,
    name: "GBM",
    evolve_step(state, dt, dw, params) {
        // Log-space exact solution: S(t+dt) = S(t) * exp((r - 0.5*sigma^2)*dt + sigma*sqrt(dt)*dW)
        let s = state.0;
        let r = params.rate;
        let sigma = params.volatility;
        let half = T::from(0.5).unwrap_or(T::zero());
        let drift = (r - half * sigma * sigma) * dt;
        let diffusion = sigma * dt.sqrt() * dw[0];
        SingleState(s * (drift + diffusion).exp())
    },
    initial_state(params) { SingleState(params.spot) },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::{
        stochastic::StochasticModel, test_macros::generate_stochastic_model_tests,
    };

    generate_stochastic_model_tests! {
        model: GBMModel<f64>,
        model_f32: GBMModel<f32>,
        default_f64_params: GBMParams::new(100.0_f64, 0.05, 0.2).unwrap(),
        default_f32_params: GBMParams::new(100.0_f32, 0.05, 0.2).unwrap(),
        model_name: "GBM",
        brownian_dim: 1,
        num_factors: 1,
        zero_shock: [0.0],
        positive_shock: [1.0],
        negative_shock: [-1.0],
        price_increased: |next: &SingleState<f64>, prev: &SingleState<f64>| next.0 > prev.0,
        price_decreased: |next: &SingleState<f64>, prev: &SingleState<f64>| next.0 < prev.0,
        state_finite_check: |s: &SingleState<f64>| s.0.is_finite(),
    }

    // --- GBM-specific tests ---

    #[test]
    fn test_gbm_params_new_valid() {
        let p = GBMParams::new(100.0_f64, 0.05, 0.2).unwrap();
        assert_eq!(p.spot, 100.0);
        assert_eq!(p.rate, 0.05);
        assert_eq!(p.volatility, 0.2);
    }

    #[test]
    fn test_gbm_params_new_invalid() {
        assert!(GBMParams::new(-100.0_f64, 0.05, 0.2).is_none());
        assert!(GBMParams::new(0.0_f64, 0.05, 0.2).is_none());
        assert!(GBMParams::new(100.0_f64, 0.05, -0.1).is_none());
    }

    #[test]
    fn test_gbm_params_default() {
        let p: GBMParams<f64> = Default::default();
        assert_eq!(p.spot, 100.0);
        assert_eq!(p.rate, 0.05);
        assert_eq!(p.volatility, 0.2);
    }

    #[test]
    fn test_gbm_params_with_epsilon() {
        let p = GBMParams::new(100.0_f64, 0.05, 0.2)
            .unwrap()
            .with_epsilon(1e-6);
        assert_eq!(p.smoothing_epsilon, 1e-6);
    }

    #[test]
    fn test_gbm_martingale_property() {
        let params = GBMParams::new(100.0_f64, 0.05, 0.2).unwrap();
        let mut state = GBMModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        for _ in 0..252 {
            state = GBMModel::evolve_step(state, dt, &[0.0], &params);
        }
        let expected = 100.0 * ((0.05 - 0.5 * 0.04) * 1.0).exp();
        assert!((state.0 - expected).abs() < 0.01);
    }
}
