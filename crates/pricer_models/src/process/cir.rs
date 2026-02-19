//! Cox-Ingersoll-Ross (CIR) interest rate model.
//!
//! The CIR model is a short-rate model described by:
//! ```text
//! dr(t) = a * (b - r(t)) * dt + sigma * sqrt(r(t)) * dW(t)
//! ```
//! where:
//! - r(t) = short rate at time t
//! - a = mean reversion speed (must be positive)
//! - b = long-term mean rate (must be positive)
//! - sigma = volatility (must be positive)
//! - dW(t) = Wiener process increment
//!
//! ## Feller Condition
//!
//! For the CIR process to remain strictly positive, the Feller condition must
//! hold:
//! ```text
//! 2 * a * b >= sigma^2
//! ```
//!
//! If this condition is violated, the process can touch zero.
//!
//! ## Key Properties
//!
//! - **Mean reversion**: Rates tend to revert to the long-term mean b
//! - **Non-negative rates**: When Feller condition holds, rates remain positive
//! - **Square-root diffusion**: Volatility is proportional to sqrt(r)
//!
//! ## Usage
//! ```
//! use pricer_models::process::{CIRModel, CIRParams};
//! use pricer_models::process::stochastic::StochasticModel;
//!
//! // Create parameters satisfying Feller condition
//! let params = CIRParams::new(0.1_f64, 0.05, 0.05, 0.03).unwrap();
//! assert!(params.satisfies_feller());
//!
//! // Get initial state
//! let state = CIRModel::initial_state(&params);
//! assert_eq!(state.0, 0.03);
//!
//! // Evolve one step
//! let dt = 1.0_f64 / 252.0;
//! let dw = [0.0_f64];
//! let next_state = CIRModel::evolve_step(state, dt, &dw, &params);
//! assert!(next_state.0 >= 0.0); // Non-negative with Feller condition
//! ```

use pricer_core::traits::{priceable::Differentiable, Float};

use crate::{
    process::{
        stochastic::{RatesModel, SingleState, StochasticModel},
        validation::ParamValidationError,
    },
    validate_params,
};

/// CIR model parameters.
#[derive(Clone, Debug)]
pub struct CIRParams<T: Float> {
    /// Mean reversion speed (a > 0)
    pub mean_reversion: T,
    /// Long-term mean rate (b > 0)
    pub long_term_mean: T,
    /// Volatility of short rate (sigma > 0)
    pub volatility: T,
    /// Initial short rate r(0)
    pub initial_rate: T,
}

impl<T: Float> CIRParams<T> {
    /// Creates new CIR parameters with validation. Returns `None` if any
    /// parameter is non-positive.
    pub fn new(
        mean_reversion: T,
        long_term_mean: T,
        volatility: T,
        initial_rate: T,
    ) -> Option<Self> {
        let params = Self {
            mean_reversion,
            long_term_mean,
            volatility,
            initial_rate,
        };
        params.validate().ok()?;
        Some(params)
    }

    /// Validate CIR parameters.
    pub fn validate(&self) -> Result<(), ParamValidationError> {
        validate_params! {
            self_val = self, f64_conv = |v: T| v.to_f64().unwrap_or(f64::NAN),
            positive: [mean_reversion, long_term_mean, volatility, initial_rate],
        }
    }

    /// Returns true if the Feller condition holds: `2ab >= sigma^2`.
    pub fn satisfies_feller(&self) -> bool {
        let two = T::from(2.0).unwrap_or(T::one());
        let lhs = two * self.mean_reversion * self.long_term_mean;
        let rhs = self.volatility * self.volatility;
        lhs >= rhs
    }

    /// Returns the Feller ratio `2ab / sigma^2` (>= 1.0 means condition is
    /// satisfied).
    pub fn feller_ratio(&self) -> T {
        let two = T::from(2.0).unwrap_or(T::one());
        let numerator = two * self.mean_reversion * self.long_term_mean;
        let denominator = self.volatility * self.volatility;
        if denominator > T::zero() {
            numerator / denominator
        } else {
            T::infinity()
        }
    }
}

define_phantom_model! {
    /// Cox-Ingersoll-Ross model for short rate dynamics.
    ///
    /// Uses Euler-Maruyama discretisation with truncation:
    /// `r(t+dt) = r(t) + a(b - r(t))dt + sigma * sqrt(max(r(t),0)) * sqrt(dt) * dW`
    model CIRModel,
    params: CIRParams<T>,
    state: SingleState<T>,
    marker: RatesModel,
    brownian_dim: 1,
    num_factors: 1,
    name: "CIR",
    evolve_step(state, dt, dw, params) {
        let r = state.0;
        let a = params.mean_reversion;
        let b = params.long_term_mean;
        let sigma = params.volatility;
        let drift = a * (b - r) * dt;
        let r_pos = if r > T::zero() { r } else { T::zero() };
        let diffusion = sigma * r_pos.sqrt() * dt.sqrt() * dw[0];
        let new_r = r + drift + diffusion;
        let epsilon = T::from(1e-10).unwrap_or(T::zero());
        SingleState(if new_r < epsilon { epsilon } else { new_r })
    },
    initial_state(params) { SingleState(params.initial_rate) },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cir_params_new_valid() {
        let params = CIRParams::new(0.1_f64, 0.05, 0.05, 0.03);
        assert!(params.is_some());
        let p = params.unwrap();
        assert_eq!(p.mean_reversion, 0.1);
        assert_eq!(p.long_term_mean, 0.05);
        assert_eq!(p.volatility, 0.05);
        assert_eq!(p.initial_rate, 0.03);
    }

    #[test]
    fn test_cir_params_new_invalid_mean_reversion() {
        // Negative mean reversion
        let params = CIRParams::new(-0.1_f64, 0.05, 0.05, 0.03);
        assert!(params.is_none());

        // Zero mean reversion
        let params = CIRParams::new(0.0_f64, 0.05, 0.05, 0.03);
        assert!(params.is_none());
    }

    #[test]
    fn test_cir_params_new_invalid_long_term_mean() {
        // Negative long-term mean
        let params = CIRParams::new(0.1_f64, -0.05, 0.05, 0.03);
        assert!(params.is_none());

        // Zero long-term mean
        let params = CIRParams::new(0.1_f64, 0.0, 0.05, 0.03);
        assert!(params.is_none());
    }

    #[test]
    fn test_cir_params_new_invalid_volatility() {
        // Negative volatility
        let params = CIRParams::new(0.1_f64, 0.05, -0.05, 0.03);
        assert!(params.is_none());

        // Zero volatility
        let params = CIRParams::new(0.1_f64, 0.05, 0.0, 0.03);
        assert!(params.is_none());
    }

    #[test]
    fn test_cir_params_new_invalid_initial_rate() {
        // Negative initial rate
        let params = CIRParams::new(0.1_f64, 0.05, 0.05, -0.03);
        assert!(params.is_none());

        // Zero initial rate
        let params = CIRParams::new(0.1_f64, 0.05, 0.05, 0.0);
        assert!(params.is_none());
    }

    #[test]
    fn test_cir_params_satisfies_feller() {
        // 2 * 0.1 * 0.05 = 0.01 >= 0.05^2 = 0.0025
        let params = CIRParams::new(0.1_f64, 0.05, 0.05, 0.03).unwrap();
        assert!(params.satisfies_feller());
    }

    #[test]
    fn test_cir_params_violates_feller() {
        // 2 * 0.01 * 0.02 = 0.0004 < 0.1^2 = 0.01
        let params = CIRParams::new(0.01_f64, 0.02, 0.1, 0.03).unwrap();
        assert!(!params.satisfies_feller());
    }

    #[test]
    fn test_cir_params_feller_ratio() {
        // Feller ratio = 2 * 0.1 * 0.05 / 0.05^2 = 0.01 / 0.0025 = 4.0
        let params = CIRParams::new(0.1_f64, 0.05, 0.05, 0.03).unwrap();
        let ratio = params.feller_ratio();
        assert!((ratio - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_cir_model_new() {
        let model: CIRModel<f64> = CIRModel::new();
        assert_eq!(CIRModel::<f64>::model_name(), "CIR");
        assert_eq!(CIRModel::<f64>::brownian_dim(), 1);
        assert_eq!(CIRModel::<f64>::num_factors(), 1);
        let _ = model; // Suppress unused warning
    }

    #[test]
    fn test_cir_initial_state() {
        let params = CIRParams::new(0.1_f64, 0.05, 0.05, 0.03).unwrap();
        let state = CIRModel::initial_state(&params);
        assert_eq!(state.0, 0.03);
    }

    #[test]
    fn test_cir_evolve_step_no_shock() {
        let params = CIRParams::new(0.1_f64, 0.05, 0.05, 0.03).unwrap();
        let state = CIRModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [0.0];

        let next_state = CIRModel::evolve_step(state, dt, &dw, &params);

        // Drift: 0.1 * (0.05 - 0.03) * dt = 0.1 * 0.02 * dt = 0.002 * dt
        // Rate should increase slightly towards long-term mean
        assert!(next_state.0 > state.0);
    }

    #[test]
    fn test_cir_evolve_step_positive_shock() {
        let params = CIRParams::new(0.1_f64, 0.05, 0.05, 0.03).unwrap();
        let state = CIRModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [1.0]; // Positive shock

        let next_state = CIRModel::evolve_step(state, dt, &dw, &params);

        // With positive shock, rate should increase
        assert!(next_state.0 > state.0);
    }

    #[test]
    fn test_cir_evolve_step_negative_shock() {
        let params = CIRParams::new(0.1_f64, 0.05, 0.05, 0.03).unwrap();
        let state = CIRModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [-1.0]; // Negative shock

        let next_state = CIRModel::evolve_step(state, dt, &dw, &params);

        // With negative shock, rate should decrease (but still positive due to Feller)
        assert!(next_state.0 < state.0);
        assert!(next_state.0 > 0.0);
    }

    #[test]
    fn test_cir_mean_reversion() {
        // Start above long-term mean, should trend down
        let params = CIRParams::new(0.5_f64, 0.03, 0.05, 0.08).unwrap();
        let state = SingleState(0.08_f64); // Above long-term mean 0.03
        let dt = 0.1; // Larger time step
        let dw = [0.0];

        let next_state = CIRModel::evolve_step(state, dt, &dw, &params);

        // Should decrease towards long-term mean
        assert!(next_state.0 < state.0);
    }

    #[test]
    fn test_cir_stays_positive_with_feller() {
        // With Feller condition satisfied, rate should stay positive
        let params = CIRParams::new(0.2_f64, 0.03, 0.03, 0.01).unwrap();
        assert!(params.satisfies_feller());

        let state = SingleState(0.01_f64);
        let dt = 1.0 / 252.0;
        let dw = [-2.0]; // Negative shock

        let next_state = CIRModel::evolve_step(state, dt, &dw, &params);

        // Should remain positive (or at least floored at epsilon)
        assert!(next_state.0 > 0.0);
    }

    #[test]
    fn test_cir_path_generation() {
        let params = CIRParams::new(0.1_f64, 0.05, 0.03, 0.04).unwrap();
        let mut state = CIRModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let n_steps = 252; // One year

        // Generate path with zero shocks
        for _ in 0..n_steps {
            state = CIRModel::evolve_step(state, dt, &[0.0], &params);
        }

        // After one year, should be close to long-term mean
        assert!(state.0.is_finite());
        assert!(state.0 > 0.0);
    }

    #[test]
    fn test_cir_is_differentiable() {
        let model: CIRModel<f64> = CIRModel::new();
        let _: &dyn Differentiable = &model;
    }

    #[test]
    fn test_cir_f32_compatibility() {
        let params = CIRParams::new(0.1_f32, 0.05, 0.05, 0.03).unwrap();
        let state = CIRModel::initial_state(&params);
        assert_eq!(state.0, 0.03_f32);

        let dt = 1.0_f32 / 252.0;
        let dw = [0.0_f32];
        let next_state = CIRModel::evolve_step(state, dt, &dw, &params);
        assert!(next_state.0.is_finite());
        assert!(next_state.0 > 0.0_f32);
    }

    #[test]
    fn test_cir_params_clone() {
        let params = CIRParams::new(0.1_f64, 0.05, 0.05, 0.03).unwrap();
        let cloned = params.clone();
        assert_eq!(params.mean_reversion, cloned.mean_reversion);
        assert_eq!(params.long_term_mean, cloned.long_term_mean);
        assert_eq!(params.volatility, cloned.volatility);
        assert_eq!(params.initial_rate, cloned.initial_rate);
    }
}
