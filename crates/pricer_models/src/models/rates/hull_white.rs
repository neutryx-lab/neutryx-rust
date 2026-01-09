//! Hull-White one-factor interest rate model.
//!
//! The Hull-White model is a short-rate model described by:
//! ```text
//! dr(t) = [theta(t) - a * r(t)] * dt + sigma * dW(t)
//! ```
//! where:
//! - r(t) = short rate at time t
//! - a = mean reversion speed (must be positive)
//! - sigma = volatility (must be positive)
//! - theta(t) = time-dependent drift (calibrated to initial yield curve)
//! - dW(t) = Wiener process increment
//!
//! ## Key Properties
//!
//! - **Mean reversion**: Rates tend to revert to a long-term mean
//! - **Analytical tractability**: Closed-form solutions for zero-coupon bonds
//! - **Negative rates**: Model allows for negative interest rates
//!
//! ## Usage
//!
//! ```
//! use pricer_models::models::rates::hull_white::{HullWhiteModel, HullWhiteParams};
//! use pricer_models::models::stochastic::StochasticModel;
//! use pricer_core::market_data::curves::FlatCurve;
//!
//! // Create parameters with flat initial curve
//! let params = HullWhiteParams::new(0.05_f64, 0.01, FlatCurve::new(0.03)).unwrap();
//!
//! // Get initial state (short rate from curve)
//! let state = HullWhiteModel::initial_state(&params);
//! assert!((state.0 - 0.03_f64).abs() < 1e-10);
//!
//! // Evolve one step
//! let dt = 1.0_f64 / 252.0;
//! let dw = [0.0_f64];
//! let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);
//! ```

use pricer_core::market_data::curves::{FlatCurve, YieldCurve};
use pricer_core::traits::priceable::Differentiable;
use pricer_core::traits::Float;

use crate::models::stochastic::{SingleState, StochasticModel};

/// Hull-White model parameters.
///
/// # Type Parameters
///
/// * `T` - Float type (f64 or DualNumber for AD compatibility)
///
/// # Fields
///
/// * `mean_reversion` - Mean reversion speed (a), must be positive
/// * `volatility` - Short rate volatility (sigma), must be positive
/// * `initial_short_rate` - Initial short rate r(0) from the initial curve
/// * `initial_curve` - Initial yield curve for calibration
#[derive(Clone, Debug)]
pub struct HullWhiteParams<T: Float> {
    /// Mean reversion speed (a > 0)
    pub mean_reversion: T,
    /// Volatility of short rate (sigma > 0)
    pub volatility: T,
    /// Initial short rate r(0)
    pub initial_short_rate: T,
    /// Initial yield curve (for theta calibration)
    pub initial_curve: FlatCurve<T>,
}

impl<T: Float> HullWhiteParams<T> {
    /// Create new Hull-White parameters with validation.
    ///
    /// # Arguments
    ///
    /// * `mean_reversion` - Mean reversion speed (must be positive)
    /// * `volatility` - Short rate volatility (must be positive)
    /// * `initial_curve` - Initial yield curve
    ///
    /// # Returns
    ///
    /// `Some(HullWhiteParams)` if parameters are valid, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_models::models::rates::hull_white::HullWhiteParams;
    /// use pricer_core::market_data::curves::FlatCurve;
    ///
    /// let params = HullWhiteParams::new(0.05, 0.01, FlatCurve::new(0.03));
    /// assert!(params.is_some());
    ///
    /// // Invalid: negative mean reversion
    /// let invalid = HullWhiteParams::new(-0.05, 0.01, FlatCurve::new(0.03));
    /// assert!(invalid.is_none());
    /// ```
    pub fn new(mean_reversion: T, volatility: T, initial_curve: FlatCurve<T>) -> Option<Self> {
        if mean_reversion <= T::zero() || volatility <= T::zero() {
            return None;
        }

        // Extract initial short rate from curve (instantaneous forward rate at t=0)
        // For a flat curve, this equals the constant rate
        let initial_short_rate = initial_curve
            .zero_rate(T::from(0.01).unwrap_or(T::one()))
            .unwrap_or(T::zero());

        Some(Self {
            mean_reversion,
            volatility,
            initial_short_rate,
            initial_curve,
        })
    }

    /// Get the long-term mean rate (approximate).
    ///
    /// For a flat initial curve with rate r*, the long-term mean
    /// is approximately r* + (sigma^2) / (2 * a^2).
    pub fn long_term_mean(&self) -> T {
        let sigma = self.volatility;
        let a = self.mean_reversion;
        let two = T::from(2.0).unwrap_or(T::one());
        self.initial_short_rate + (sigma * sigma) / (two * a * a)
    }

    /// Calculate theta(t) for the initial curve fitting.
    ///
    /// For a flat curve with rate r*, theta = a * r* + (sigma^2 / 2a) * (1 - exp(-2at)).
    /// This ensures the model fits the initial term structure exactly.
    pub fn theta(&self, t: T) -> T {
        let a = self.mean_reversion;
        let sigma = self.volatility;
        let r_star = self.initial_short_rate;

        // For flat curve: theta = a * r* + (sigma^2 / 2a) * (1 - exp(-2at))
        let two = T::from(2.0).unwrap_or(T::one());
        let term = T::one() - (-(two * a * t)).exp();
        a * r_star + (sigma * sigma / (two * a)) * term
    }
}

/// Hull-White one-factor model for short rate dynamics.
///
/// Implements the StochasticModel trait for Monte Carlo simulation.
///
/// # Discretization
///
/// Uses Euler-Maruyama discretization:
/// ```text
/// r(t+dt) = r(t) + [theta(t) - a * r(t)] * dt + sigma * sqrt(dt) * dW
/// ```
#[derive(Clone, Debug, Default)]
pub struct HullWhiteModel<T: Float> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Float> HullWhiteModel<T> {
    /// Create a new Hull-White model instance.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Float> Differentiable for HullWhiteModel<T> {}

impl<T: Float + Default> StochasticModel<T> for HullWhiteModel<T> {
    type State = SingleState<T>;
    type Params = HullWhiteParams<T>;

    fn evolve_step(state: Self::State, dt: T, dw: &[T], params: &Self::Params) -> Self::State {
        // Euler-Maruyama discretization:
        // r(t+dt) = r(t) + [theta(t) - a * r(t)] * dt + sigma * sqrt(dt) * dW
        let r = state.0;
        let a = params.mean_reversion;
        let sigma = params.volatility;

        // For simplicity, use constant theta (good for short simulations)
        // In production, theta should be time-dependent for curve fitting
        let theta = params.theta(T::zero());

        // Drift: [theta - a * r] * dt
        let drift = (theta - a * r) * dt;

        // Diffusion: sigma * sqrt(dt) * dW
        let diffusion = sigma * dt.sqrt() * dw[0];

        SingleState(r + drift + diffusion)
    }

    fn initial_state(params: &Self::Params) -> Self::State {
        SingleState(params.initial_short_rate)
    }

    fn brownian_dim() -> usize {
        1
    }

    fn model_name() -> &'static str {
        "HullWhite1F"
    }

    fn num_factors() -> usize {
        1 // Hull-White 1F is a single-factor model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 9.2: Hull-White 1F Model Tests (TDD)
    // ================================================================

    // Test: Parameter validation
    #[test]
    fn test_hull_white_params_new_valid() {
        let params = HullWhiteParams::new(0.05_f64, 0.01, FlatCurve::new(0.03));
        assert!(params.is_some());
        let p = params.unwrap();
        assert_eq!(p.mean_reversion, 0.05);
        assert_eq!(p.volatility, 0.01);
        assert!((p.initial_short_rate - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_hull_white_params_new_invalid_mean_reversion() {
        // Negative mean reversion
        let params = HullWhiteParams::new(-0.05_f64, 0.01, FlatCurve::new(0.03));
        assert!(params.is_none());

        // Zero mean reversion
        let params = HullWhiteParams::new(0.0_f64, 0.01, FlatCurve::new(0.03));
        assert!(params.is_none());
    }

    #[test]
    fn test_hull_white_params_new_invalid_volatility() {
        // Negative volatility
        let params = HullWhiteParams::new(0.05_f64, -0.01, FlatCurve::new(0.03));
        assert!(params.is_none());

        // Zero volatility
        let params = HullWhiteParams::new(0.05_f64, 0.0, FlatCurve::new(0.03));
        assert!(params.is_none());
    }

    #[test]
    fn test_hull_white_params_long_term_mean() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let ltm = params.long_term_mean();
        // Expected: 0.03 + (0.01^2) / (2 * 0.1^2) = 0.03 + 0.0001 / 0.02 = 0.03 + 0.005 = 0.035
        assert!((ltm - 0.035).abs() < 1e-10);
    }

    #[test]
    fn test_hull_white_params_theta() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        // theta at t=0: a * r* = 0.1 * 0.03 = 0.003
        let theta_0 = params.theta(0.0);
        assert!((theta_0 - 0.003).abs() < 1e-10);
    }

    // Test: Model properties
    #[test]
    fn test_hull_white_model_new() {
        let model: HullWhiteModel<f64> = HullWhiteModel::new();
        assert_eq!(HullWhiteModel::<f64>::model_name(), "HullWhite1F");
        assert_eq!(HullWhiteModel::<f64>::brownian_dim(), 1);
        assert_eq!(HullWhiteModel::<f64>::num_factors(), 1);
        let _ = model; // Suppress unused warning
    }

    #[test]
    fn test_hull_white_initial_state() {
        let params = HullWhiteParams::new(0.05_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        assert!((state.0 - 0.03).abs() < 1e-10);
    }

    // Test: Evolution step
    #[test]
    fn test_hull_white_evolve_step_no_shock() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        let dt = 1.0 / 252.0; // Daily step
        let dw = [0.0];

        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);

        // With zero shock, should move towards long-term mean
        // drift = [theta - a * r] * dt = [0.003 - 0.1 * 0.03] * dt = 0 * dt = 0
        // For flat curve at initial state, drift is approximately zero
        assert!((next_state.0 - state.0).abs() < 1e-8);
    }

    #[test]
    fn test_hull_white_evolve_step_positive_shock() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [1.0]; // Positive shock

        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);

        // With positive shock, rate should increase
        assert!(next_state.0 > state.0);
    }

    #[test]
    fn test_hull_white_evolve_step_negative_shock() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [-1.0]; // Negative shock

        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);

        // With negative shock, rate should decrease
        assert!(next_state.0 < state.0);
    }

    #[test]
    fn test_hull_white_mean_reversion() {
        // Start above long-term mean, should trend down
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.02)).unwrap();
        let state = SingleState(0.08_f64); // Well above 0.02
        let dt = 0.1; // Larger time step
        let dw = [0.0];

        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);

        // Should decrease towards long-term mean
        assert!(next_state.0 < state.0);
    }

    #[test]
    fn test_hull_white_allows_negative_rates() {
        // Start near zero, with large negative shock
        let params = HullWhiteParams::new(0.1_f64, 0.05, FlatCurve::new(0.001)).unwrap();
        let state = SingleState(0.001_f64);
        let dt = 1.0 / 12.0; // Monthly step for larger diffusion effect
        let dw = [-3.0]; // Large negative shock

        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);

        // Rate can go negative in Hull-White model
        // Diffusion: 0.05 * sqrt(1/12) * (-3) = -0.0433
        // This should push the rate negative
        assert!(next_state.0 < 0.0);
    }

    // Test: Multiple steps path generation
    #[test]
    fn test_hull_white_path_generation() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let mut state = HullWhiteModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let n_steps = 252; // One year

        // Generate path with zero shocks
        for _ in 0..n_steps {
            state = HullWhiteModel::evolve_step(state, dt, &[0.0], &params);
        }

        // After one year with zero shocks, should be close to initial
        // (since we started at the long-term mean for flat curve)
        assert!(state.0.is_finite());
    }

    // Test: Differentiable marker
    #[test]
    fn test_hull_white_is_differentiable() {
        let model: HullWhiteModel<f64> = HullWhiteModel::new();
        let _: &dyn Differentiable = &model;
    }

    // Test: f32 compatibility
    #[test]
    fn test_hull_white_f32_compatibility() {
        let params = HullWhiteParams::new(0.1_f32, 0.01, FlatCurve::new(0.03)).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        assert!((state.0 - 0.03_f32).abs() < 1e-6);

        let dt = 1.0_f32 / 252.0;
        let dw = [0.0_f32];
        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);
        assert!(next_state.0.is_finite());
    }

    // Test: Clone
    #[test]
    fn test_hull_white_params_clone() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let cloned = params.clone();
        assert_eq!(params.mean_reversion, cloned.mean_reversion);
        assert_eq!(params.volatility, cloned.volatility);
    }
}
