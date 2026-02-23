//! Gibson-Schwartz 2-factor commodity model.
//!
//! Two correlated SDEs modelling commodity spot price and convenience yield:
//!
//! ```text
//! d(S + shift) / (S + shift) = (r - δ) dt + σ_S dW_S    (shifted lognormal spot)
//! dδ = κ(θ_δ - δ) dt + σ_δ dW_δ                         (OU convenience yield)
//! ```
//!
//! with correlation structure:
//! ```text
//! E[dW_S · dW_δ] = ρ dt
//! ```
//!
//! ## Key Features
//!
//! - **Shifted lognormal** spot dynamics for negative price support
//! - **Mean-reverting convenience yield** (Ornstein-Uhlenbeck / Hull-White)
//! - **Inline 2×2 Cholesky** for AD-compatible correlated Brownian motions
//! - **Analytical pricer** for commodity forwards and vanilla options (Black-76
//!   with CY-adjusted forward)
//! - **Closed-form log-spot variance** for term-structure consistent volatility
//!   scaling
//!
//! ## Design
//!
//! The model follows the Jarrow-Yildirim pattern: manual `StochasticModel<T>`
//! impl with inline Cholesky, custom error type, and `validate_params!` macro.
//! The shift parameter (default 0) extends standard lognormal to accommodate
//! negative commodity prices observed in recent energy markets.

use pricer_core::{
    math::smoothing::{smooth_max, smooth_sqrt},
    traits::{priceable::Differentiable, Float},
};
use thiserror::Error;

use super::{
    stochastic::{CommodityModel, StochasticModel, TwoFactorState},
    validation::{ComputationError, ParamValidationError, DEFAULT_SMOOTHING_EPSILON},
};
use crate::validate_params;

// ─── Error ───────────────────────────────────────────────────────────────────

/// Gibson-Schwartz model error type.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum GibsonSchwartzError {
    /// Parameter validation error.
    #[error("Parameter error: {0}")]
    Param(#[from] ParamValidationError),

    /// Numerical computation error.
    #[error("Computation error: {0}")]
    Computation(#[from] ComputationError),
}

impl From<GibsonSchwartzError> for pricer_core::types::PricingError {
    fn from(err: GibsonSchwartzError) -> Self {
        match err {
            GibsonSchwartzError::Param(e) => {
                pricer_core::types::PricingError::InvalidInput(e.to_string())
            }
            GibsonSchwartzError::Computation(e) => {
                pricer_core::types::PricingError::NumericalInstability(e.to_string())
            }
        }
    }
}

// ─── Parameters ──────────────────────────────────────────────────────────────

/// Gibson-Schwartz 2-factor commodity model parameters.
///
/// ## Factor 1: Spot price (shifted lognormal)
///
/// ```text
/// d(S + shift) / (S + shift) = (r - δ) dt + σ_S dW_S
/// ```
///
/// ## Factor 2: Convenience yield (OU/HW mean-reverting)
///
/// ```text
/// dδ = κ (θ_δ - δ) dt + σ_δ dW_δ
/// ```
///
/// ## Correlation
///
/// ```text
/// E[dW_S · dW_δ] = ρ dt
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GibsonSchwartzParams<T: Float> {
    /// Spot price (S > 0).
    pub spot: T,
    /// Risk-free rate (r).
    pub rate: T,
    /// Initial convenience yield (δ₀).
    pub initial_convenience_yield: T,
    /// Mean reversion speed of convenience yield (κ > 0).
    pub kappa: T,
    /// Long-term mean of convenience yield (θ_δ).
    pub theta_delta: T,
    /// Spot volatility (σ_S > 0).
    pub sigma_spot: T,
    /// Convenience yield volatility (σ_δ > 0).
    pub sigma_cy: T,
    /// Spot-CY correlation (ρ ∈ [-1, 1]).
    pub rho: T,
    /// Shift parameter for negative price support (≥ 0; 0 = standard
    /// lognormal).
    pub shift: T,
    /// Smoothing epsilon for AD-compatible approximations.
    pub smoothing_epsilon: T,
}

#[allow(clippy::too_many_arguments)]
impl<T: Float> GibsonSchwartzParams<T> {
    /// Create new Gibson-Schwartz parameters with validation.
    pub fn new(
        spot: T,
        rate: T,
        initial_convenience_yield: T,
        kappa: T,
        theta_delta: T,
        sigma_spot: T,
        sigma_cy: T,
        rho: T,
    ) -> Result<Self, GibsonSchwartzError> {
        let params = Self {
            spot,
            rate,
            initial_convenience_yield,
            kappa,
            theta_delta,
            sigma_spot,
            sigma_cy,
            rho,
            shift: T::zero(),
            smoothing_epsilon: T::from(DEFAULT_SMOOTHING_EPSILON).unwrap_or(T::zero()),
        };
        params.validate()?;
        Ok(params)
    }

    /// Set custom shift parameter for negative price support.
    pub fn with_shift(mut self, shift: T) -> Result<Self, GibsonSchwartzError> {
        self.shift = shift;
        self.validate()?;
        Ok(self)
    }

    /// Set custom smoothing epsilon.
    pub fn with_epsilon(mut self, epsilon: T) -> Self {
        self.smoothing_epsilon = epsilon;
        self
    }

    /// Validate all parameters.
    pub fn validate(&self) -> Result<(), GibsonSchwartzError> {
        validate_params! {
            self_val = self, f64_conv = |v: T| v.to_f64().unwrap_or(f64::NAN),
            positive: [spot, kappa, sigma_spot, sigma_cy],
            non_negative: [shift],
            correlation: [rho],
        }
    }

    /// Compute commodity forward price: `F(τ) = S × exp((r − δ) × τ)`.
    pub fn forward_price(&self, tau: T) -> T {
        self.spot * ((self.rate - self.initial_convenience_yield) * tau).exp()
    }
}

impl Default for GibsonSchwartzParams<f64> {
    /// Default parameters for WTI crude oil market.
    fn default() -> Self {
        Self::new(
            75.0, // spot (USD/bbl)
            0.05, // risk-free rate
            0.03, // initial convenience yield
            1.2,  // CY mean reversion
            0.03, // CY long-term mean
            0.35, // spot vol (35%)
            0.15, // CY vol
            0.3,  // spot-CY correlation
        )
        .expect("Default Gibson-Schwartz params must be valid")
    }
}

// ─── Model ───────────────────────────────────────────────────────────────────

/// Gibson-Schwartz 2-factor commodity model.
///
/// Uses Euler-Maruyama discretisation with inline 2×2 Cholesky decomposition
/// for AD-compatible correlated Brownian motions. The model receives 2
/// independent standard normals per step and correlates them internally.
#[derive(Clone, Debug, Default)]
pub struct GibsonSchwartzModel<T: Float> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Float> GibsonSchwartzModel<T> {
    /// Create a new Gibson-Schwartz model instance.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Float> Differentiable for GibsonSchwartzModel<T> {}

impl<T: Float + Default> StochasticModel<T> for GibsonSchwartzModel<T> {
    type State = TwoFactorState<T>;
    type Params = GibsonSchwartzParams<T>;

    /// Evolve state by one time step using Euler-Maruyama.
    ///
    /// `dw` must contain 2 independent standard normal random variates:
    /// `[z_spot, z_cy]`. Correlation is applied internally via inline 2×2
    /// Cholesky decomposition.
    ///
    /// State: `first` = spot price S, `second` = convenience yield δ.
    fn evolve_step(state: Self::State, dt: T, dw: &[T], params: &Self::Params) -> Self::State {
        let s = state.first; // spot price
        let delta = state.second; // convenience yield

        let z1 = dw.first().copied().unwrap_or(T::zero());
        let z2 = dw.get(1).copied().unwrap_or(T::zero());

        let eps = params.smoothing_epsilon;
        let rho = params.rho;

        // ── Inline 2×2 Cholesky: L where C = L Lᵀ ──
        // C = [[1, ρ], [ρ, 1]]
        // L = [[1, 0], [ρ, √(1 − ρ²)]]
        let one_minus_rho_sq = T::one() - rho * rho;
        let l_11 = smooth_sqrt(smooth_max(one_minus_rho_sq, eps, eps), eps);

        // Correlated Brownians: W = L · Z
        let dw_s = z1;
        let dw_delta = rho * z1 + l_11 * z2;

        let sqrt_dt = dt.sqrt();

        // ── Spot price: shifted lognormal (log-space for positivity) ──
        // d(S + shift) / (S + shift) = (r − δ) dt + σ_S dW_S
        // Exact: (S_new + shift) = (S + shift) × exp((r − δ − ½σ²) dt + σ √dt dW_S)
        let shifted = s + params.shift;
        let shifted_safe = smooth_max(shifted, eps, eps);
        let half = T::from(0.5).unwrap_or(T::zero());
        let spot_drift = (params.rate - delta - half * params.sigma_spot * params.sigma_spot) * dt;
        let spot_diff = params.sigma_spot * sqrt_dt * dw_s;
        let shifted_next = shifted_safe * (spot_drift + spot_diff).exp();
        let s_next = shifted_next - params.shift;

        // ── Convenience yield: OU process (Euler-Maruyama) ──
        // dδ = κ(θ_δ − δ) dt + σ_δ dW_δ
        let cy_drift = params.kappa * (params.theta_delta - delta) * dt;
        let cy_diff = params.sigma_cy * sqrt_dt * dw_delta;
        let delta_next = delta + cy_drift + cy_diff;

        TwoFactorState {
            first: s_next,
            second: delta_next,
        }
    }

    fn initial_state(params: &Self::Params) -> Self::State {
        TwoFactorState {
            first: params.spot,
            second: params.initial_convenience_yield,
        }
    }

    fn brownian_dim() -> usize { 2 }

    fn model_name() -> &'static str { "GibsonSchwartz" }

    fn num_factors() -> usize { 2 }
}

impl<T: Float + Default> CommodityModel<T> for GibsonSchwartzModel<T> {}

// ─── Analytical Helpers ──────────────────────────────────────────────────────

/// Compute the Gibson-Schwartz commodity forward curve.
///
/// `F(t, T) = S(t) × exp((r − δ(t)) × (T − t))`
///
/// Returns a vector of `(tenor, forward_price)` pairs for the given tenors.
pub fn commodity_forward_curve<T: Float>(
    spot: T,
    convenience_yield: T,
    rate: T,
    tenors: &[T],
) -> Vec<(T, T)> {
    tenors
        .iter()
        .map(|&tau| {
            let forward = spot * ((rate - convenience_yield) * tau).exp();
            (tau, forward)
        })
        .collect()
}

/// Compute the Gibson-Schwartz analytical variance of `ln(S)` at time `T`.
///
/// ```text
/// Var[ln S(T)] = σ_S² T
///     + σ_δ²/κ² [T − 2/κ (1−e^{−κT}) + 1/(2κ)(1−e^{−2κT})]
///     − 2ρ σ_S σ_δ/κ [T − 1/κ (1−e^{−κT})]
/// ```
///
/// Used for term-structure consistent volatility scaling and implied vol
/// computation from the Gibson-Schwartz model.
pub fn analytical_log_spot_variance<T: Float>(params: &GibsonSchwartzParams<T>, t: T) -> T {
    let k = params.kappa;
    let s_s = params.sigma_spot;
    let s_d = params.sigma_cy;
    let rho = params.rho;
    let two = T::from(2.0).unwrap_or(T::one() + T::one());

    let exp_kt = (-k * t).exp();
    let exp_2kt = (-two * k * t).exp();

    // Spot variance contribution
    let var_spot = s_s * s_s * t;

    // CY variance contribution to spot
    let var_cy = s_d * s_d / (k * k)
        * (t - two / k * (T::one() - exp_kt) + T::one() / (two * k) * (T::one() - exp_2kt));

    // Cross-term
    let covar = two * rho * s_s * s_d / k * (t - T::one() / k * (T::one() - exp_kt));

    var_spot + var_cy - covar
}

// ─── Analytical Commodity Pricer ─────────────────────────────────────────────

/// Analytical pricer for commodity forwards and vanilla options under
/// Gibson-Schwartz.
///
/// Uses Black-76 with convenience-yield-adjusted forward for vanilla options.
/// This serves as the **deterministic fallback** when Monte Carlo is not
/// needed, providing the "hybrid design" described in the model architecture:
/// complex path-dependent exotics use the full MC process, while vanilla
/// products are priced analytically for speed.
pub struct CommodityAnalyticalPricer;

impl CommodityAnalyticalPricer {
    /// Compute commodity forward price.
    ///
    /// `F(t, T) = S(t) × exp((r − δ(t)) × (T − t))`
    pub fn forward_price<T: Float>(spot: T, rate: T, convenience_yield: T, tau: T) -> T {
        spot * ((rate - convenience_yield) * tau).exp()
    }

    /// Price a commodity forward contract (PV of long position).
    ///
    /// `PV = (F − K) × exp(−r × τ)`
    pub fn forward_pv<T: Float>(spot: T, strike: T, rate: T, convenience_yield: T, tau: T) -> T {
        let forward = Self::forward_price(spot, rate, convenience_yield, tau);
        let df = (-rate * tau).exp();
        (forward - strike) * df
    }

    /// Price a European commodity option using Black-76 with CY-adjusted
    /// forward.
    ///
    /// This implements the deterministic fallback for vanilla commodity
    /// options. The forward is adjusted for convenience yield, then priced
    /// using the standard Black-76 formula.
    pub fn option_price<T: Float>(
        spot: T,
        strike: T,
        rate: T,
        convenience_yield: T,
        sigma: T,
        tau: T,
        is_call: bool,
    ) -> T {
        let forward = Self::forward_price(spot, rate, convenience_yield, tau);
        let df = (-rate * tau).exp();
        let half = T::from(0.5).unwrap_or(T::zero());
        let sqrt_tau = tau.sqrt();
        let vol_sqrt_t = sigma * sqrt_tau;

        // Degenerate case: zero vol or zero time
        if vol_sqrt_t <= T::zero() {
            let payoff = if is_call {
                smooth_max(
                    forward - strike,
                    T::zero(),
                    T::from(DEFAULT_SMOOTHING_EPSILON).unwrap_or(T::zero()),
                )
            } else {
                smooth_max(
                    strike - forward,
                    T::zero(),
                    T::from(DEFAULT_SMOOTHING_EPSILON).unwrap_or(T::zero()),
                )
            };
            return df * payoff;
        }

        let d1 = ((forward / strike).ln() + half * sigma * sigma * tau) / vol_sqrt_t;
        let d2 = d1 - vol_sqrt_t;

        // Standard normal CDF via erfc: Φ(x) = 0.5 × erfc(−x / √2)
        let inv_sqrt2 = T::from(std::f64::consts::FRAC_1_SQRT_2).unwrap_or(T::zero());
        let n_d1 = T::from(0.5).unwrap_or(T::zero()) * (T::one() + erf_approx(d1 * inv_sqrt2));
        let n_d2 = T::from(0.5).unwrap_or(T::zero()) * (T::one() + erf_approx(d2 * inv_sqrt2));

        if is_call {
            df * (forward * n_d1 - strike * n_d2)
        } else {
            let n_neg_d1 = T::one() - n_d1;
            let n_neg_d2 = T::one() - n_d2;
            df * (strike * n_neg_d2 - forward * n_neg_d1)
        }
    }

    /// Compute the average expected convenience yield over `[0, T]`.
    ///
    /// For the OU process `dδ = κ(θ − δ)dt + σ_δ dW_δ`:
    ///
    /// ```text
    /// E[δ(t)] = θ + (δ₀ − θ) e^{−κt}
    /// δ̄(T) = (1/T) ∫₀ᵀ E[δ(t)] dt = θ + (δ₀ − θ)(1 − e^{−κT}) / (κT)
    /// ```
    pub fn average_expected_cy<T: Float>(initial_cy: T, kappa: T, theta: T, tau: T) -> T {
        let eps = T::from(1e-10).unwrap_or(T::zero());
        if kappa.abs() < eps || tau.abs() < eps {
            return initial_cy;
        }
        theta + (initial_cy - theta) * (T::one() - (-kappa * tau).exp()) / (kappa * tau)
    }
}

/// Abramowitz & Stegun approximation for `erf(x)`, AD-compatible.
fn erf_approx<T: Float>(x: T) -> T {
    // Constants from A&S 7.1.26 (max error ~1.5e-7)
    let a1 = T::from(0.254829592).unwrap_or(T::zero());
    let a2 = T::from(-0.284496736).unwrap_or(T::zero());
    let a3 = T::from(1.421413741).unwrap_or(T::zero());
    let a4 = T::from(-1.453152027).unwrap_or(T::zero());
    let a5 = T::from(1.061405429).unwrap_or(T::zero());
    let p = T::from(0.3275911).unwrap_or(T::zero());

    let sign = if x >= T::zero() { T::one() } else { -T::one() };
    let x_abs = x.abs();
    let t = T::one() / (T::one() + p * x_abs);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    let y = T::one() - (a1 * t + a2 * t2 + a3 * t3 + a4 * t4 + a5 * t5) * (-x_abs * x_abs).exp();
    sign * y
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::stochastic::{StochasticModel, StochasticState};

    fn default_params() -> GibsonSchwartzParams<f64> { GibsonSchwartzParams::default() }

    // ── Parameter Tests ──

    #[test]
    fn test_params_new_valid() {
        let p = default_params();
        assert_eq!(p.spot, 75.0);
        assert_eq!(p.rate, 0.05);
        assert_eq!(p.initial_convenience_yield, 0.03);
        assert_eq!(p.kappa, 1.2);
        assert_eq!(p.theta_delta, 0.03);
        assert_eq!(p.sigma_spot, 0.35);
        assert_eq!(p.sigma_cy, 0.15);
        assert_eq!(p.rho, 0.3);
        assert_eq!(p.shift, 0.0);
    }

    #[test]
    fn test_params_rejects_negative_spot() {
        assert!(GibsonSchwartzParams::new(-75.0, 0.05, 0.03, 1.2, 0.03, 0.35, 0.15, 0.3).is_err());
    }

    #[test]
    fn test_params_rejects_negative_kappa() {
        assert!(GibsonSchwartzParams::new(75.0, 0.05, 0.03, -1.2, 0.03, 0.35, 0.15, 0.3).is_err());
    }

    #[test]
    fn test_params_rejects_negative_spot_vol() {
        assert!(GibsonSchwartzParams::new(75.0, 0.05, 0.03, 1.2, 0.03, -0.35, 0.15, 0.3).is_err());
    }

    #[test]
    fn test_params_rejects_negative_cy_vol() {
        assert!(GibsonSchwartzParams::new(75.0, 0.05, 0.03, 1.2, 0.03, 0.35, -0.15, 0.3).is_err());
    }

    #[test]
    fn test_params_rejects_rho_out_of_range() {
        assert!(GibsonSchwartzParams::new(75.0, 0.05, 0.03, 1.2, 0.03, 0.35, 0.15, 1.5).is_err());
        assert!(GibsonSchwartzParams::new(75.0, 0.05, 0.03, 1.2, 0.03, 0.35, 0.15, -1.5).is_err());
    }

    #[test]
    fn test_params_boundary_correlations() {
        // Exact ±1 should be accepted (closed interval)
        assert!(GibsonSchwartzParams::new(75.0, 0.05, 0.03, 1.2, 0.03, 0.35, 0.15, 1.0).is_ok());
        assert!(GibsonSchwartzParams::new(75.0, 0.05, 0.03, 1.2, 0.03, 0.35, 0.15, -1.0).is_ok());
    }

    #[test]
    fn test_params_with_shift() {
        let p = default_params().with_shift(10.0).unwrap();
        assert_eq!(p.shift, 10.0);
    }

    #[test]
    fn test_params_rejects_negative_shift() {
        assert!(default_params().with_shift(-1.0).is_err());
    }

    #[test]
    fn test_forward_price() {
        let p = default_params();
        let tau = 1.0;
        let expected = 75.0 * ((0.05 - 0.03) * 1.0_f64).exp();
        let fwd = p.forward_price(tau);
        assert!((fwd - expected).abs() < 1e-10);
    }

    // ── Model Tests ──

    #[test]
    fn test_model_basics() {
        assert_eq!(GibsonSchwartzModel::<f64>::brownian_dim(), 2);
        assert_eq!(GibsonSchwartzModel::<f64>::model_name(), "GibsonSchwartz");
        assert_eq!(GibsonSchwartzModel::<f64>::num_factors(), 2);
    }

    #[test]
    fn test_initial_state() {
        let p = default_params();
        let state = GibsonSchwartzModel::initial_state(&p);
        assert_eq!(state.first, 75.0); // spot
        assert_eq!(state.second, 0.03); // CY
    }

    #[test]
    fn test_evolve_step_no_shock() {
        let p = default_params();
        let state = GibsonSchwartzModel::initial_state(&p);
        let dt = 1.0 / 252.0;
        let dw = [0.0, 0.0];

        let next = GibsonSchwartzModel::evolve_step(state, dt, &dw, &p);

        // Spot should barely change with zero shock (drift only)
        assert!((next.first - state.first).abs() < 0.5);
        // CY should barely change
        assert!((next.second - state.second).abs() < 0.01);
    }

    #[test]
    fn test_evolve_step_positive_spot_shock() {
        let p = default_params();
        let state = GibsonSchwartzModel::initial_state(&p);
        let dt = 1.0 / 252.0;
        let dw = [3.0, 0.0]; // strong positive shock to spot

        let next = GibsonSchwartzModel::evolve_step(state, dt, &dw, &p);
        assert!(
            next.first > state.first,
            "Positive shock should raise spot price"
        );
    }

    #[test]
    fn test_evolve_step_negative_spot_shock() {
        let p = default_params();
        let state = GibsonSchwartzModel::initial_state(&p);
        let dt = 1.0 / 252.0;
        let dw = [-3.0, 0.0]; // strong negative shock to spot

        let next = GibsonSchwartzModel::evolve_step(state, dt, &dw, &p);
        assert!(
            next.first < state.first,
            "Negative shock should lower spot price"
        );
    }

    #[test]
    fn test_cy_mean_reversion_up() {
        // When delta < theta, drift should push CY up
        let mut p = default_params();
        p.initial_convenience_yield = 0.0; // below theta=0.03
        let state = GibsonSchwartzModel::initial_state(&p);
        let dt = 1.0 / 252.0;
        let dw = [0.0, 0.0];

        let next = GibsonSchwartzModel::evolve_step(state, dt, &dw, &p);
        assert!(
            next.second > state.second,
            "CY should revert upward toward theta"
        );
    }

    #[test]
    fn test_cy_mean_reversion_down() {
        // When delta > theta, drift should push CY down
        let mut p = default_params();
        p.initial_convenience_yield = 0.10; // above theta=0.03
        let state = GibsonSchwartzModel::initial_state(&p);
        let dt = 1.0 / 252.0;
        let dw = [0.0, 0.0];

        let next = GibsonSchwartzModel::evolve_step(state, dt, &dw, &p);
        assert!(
            next.second < state.second,
            "CY should revert downward toward theta"
        );
    }

    #[test]
    fn test_multi_step_stability() {
        let p = default_params();
        let mut state = GibsonSchwartzModel::initial_state(&p);
        let dt = 1.0 / 252.0;

        for i in 0..252 {
            let phase = i as f64 * 0.1;
            let dw = [0.1 * phase.sin(), 0.1 * phase.cos()];
            state = GibsonSchwartzModel::evolve_step(state, dt, &dw, &p);

            assert!(state.first.is_finite(), "Spot non-finite at step {i}");
            assert!(state.second.is_finite(), "CY non-finite at step {i}");
        }
    }

    #[test]
    fn test_long_simulation_stability() {
        let p = default_params();
        let mut state = GibsonSchwartzModel::initial_state(&p);
        let dt = 1.0 / 252.0;

        // 5-year simulation
        for i in 0..(252 * 5) {
            let phase = i as f64 * 0.1;
            let dw = [0.1 * phase.sin(), 0.1 * (phase * 0.7).cos()];
            state = GibsonSchwartzModel::evolve_step(state, dt, &dw, &p);
        }

        assert!(state.first.is_finite(), "Spot should be finite after 5Y");
        assert!(state.second.is_finite(), "CY should be finite after 5Y");
    }

    #[test]
    fn test_extreme_rho_stability() {
        for &rho_val in &[-0.99, -0.5, 0.0, 0.5, 0.99] {
            let p = GibsonSchwartzParams::new(75.0, 0.05, 0.03, 1.2, 0.03, 0.35, 0.15, rho_val)
                .unwrap();
            let mut state = GibsonSchwartzModel::initial_state(&p);
            let dt = 1.0 / 252.0;

            for i in 0..252 {
                let phase = i as f64 * 0.3;
                let dw = [0.5 * phase.sin(), 0.5 * phase.cos()];
                state = GibsonSchwartzModel::evolve_step(state, dt, &dw, &p);
            }

            assert!(state.first.is_finite(), "Spot non-finite for rho={rho_val}");
            assert!(state.second.is_finite(), "CY non-finite for rho={rho_val}");
        }
    }

    #[test]
    fn test_shifted_lognormal() {
        let p = GibsonSchwartzParams::new(75.0, 0.05, 0.03, 1.2, 0.03, 0.35, 0.15, 0.3)
            .unwrap()
            .with_shift(100.0)
            .unwrap();

        let mut state = GibsonSchwartzModel::initial_state(&p);
        let dt = 1.0 / 252.0;

        // Large negative shocks should not crash with shift
        for _ in 0..50 {
            let dw = [-2.0, 0.0];
            state = GibsonSchwartzModel::evolve_step(state, dt, &dw, &p);
        }

        assert!(
            state.first.is_finite(),
            "Shifted model should remain finite"
        );
        // With shift=100, S+shift should stay positive even if S goes negative
    }

    #[test]
    fn test_correlation_effect() {
        // With high positive rho, spot and CY shocks should be correlated
        let p = GibsonSchwartzParams::new(75.0, 0.05, 0.03, 1.2, 0.03, 0.35, 0.15, 0.99).unwrap();
        let state = GibsonSchwartzModel::initial_state(&p);
        let dt = 1.0 / 252.0;

        // Shock only the first independent normal
        let dw = [3.0, 0.0];
        let next = GibsonSchwartzModel::evolve_step(state, dt, &dw, &p);

        assert!(next.first > state.first, "Spot should increase");
        // CY should also increase due to correlation
        assert!(
            next.second > state.second,
            "CY should increase (rho=0.99, positive z1 spills through Cholesky)"
        );
    }

    #[test]
    fn test_stochastic_state_trait() {
        let state = TwoFactorState::<f64> {
            first: 75.0,
            second: 0.03,
        };
        assert_eq!(TwoFactorState::<f64>::dimension(), 2);
        assert_eq!(state.get(0), Some(75.0));
        assert_eq!(state.get(1), Some(0.03));
        assert_eq!(state.to_array(), vec![75.0, 0.03]);
    }

    // ── Analytical Helper Tests ──

    #[test]
    fn test_commodity_forward_curve() {
        let tenors = vec![0.25, 0.5, 1.0, 2.0, 5.0];
        let curve = commodity_forward_curve(75.0, 0.03, 0.05, &tenors);

        assert_eq!(curve.len(), 5);

        // Forward should increase with tenor (r > delta → contango)
        for i in 1..curve.len() {
            assert!(
                curve[i].1 > curve[i - 1].1,
                "Forward should increase in contango"
            );
        }

        // Check 1Y forward
        let expected_1y = 75.0 * (0.02_f64 * 1.0).exp();
        assert!((curve[2].1 - expected_1y).abs() < 1e-10);
    }

    #[test]
    fn test_commodity_forward_curve_backwardation() {
        // When delta > r, curve should be in backwardation
        let tenors = vec![0.25, 0.5, 1.0, 2.0];
        let curve = commodity_forward_curve(75.0, 0.08, 0.05, &tenors);

        for i in 1..curve.len() {
            assert!(
                curve[i].1 < curve[i - 1].1,
                "Forward should decrease in backwardation"
            );
        }
    }

    #[test]
    fn test_analytical_log_spot_variance() {
        let p = default_params();
        let var_1y = analytical_log_spot_variance(&p, 1.0);

        // Should be positive
        assert!(var_1y > 0.0, "Variance must be positive");
        assert!(var_1y.is_finite(), "Variance must be finite");

        // Should be approximately sigma_S^2 * T for small CY vol
        let mut p_low_cy_vol = p;
        p_low_cy_vol.sigma_cy = 0.001;
        p_low_cy_vol.rho = 0.0;
        let var_approx = analytical_log_spot_variance(&p_low_cy_vol, 1.0);
        let expected_approx = 0.35 * 0.35 * 1.0;
        assert!(
            (var_approx - expected_approx).abs() < 0.01,
            "With tiny CY vol, variance should be ~sigma_S^2*T: got {var_approx}, expected {expected_approx}"
        );
    }

    #[test]
    fn test_analytical_variance_monotone_time() {
        let p = default_params();
        let var_1y = analytical_log_spot_variance(&p, 1.0);
        let var_5y = analytical_log_spot_variance(&p, 5.0);
        assert!(var_5y > var_1y, "Variance should increase with time");
    }

    // ── Analytical Pricer Tests ──

    #[test]
    fn test_analytical_forward_price() {
        let fwd = CommodityAnalyticalPricer::forward_price(75.0, 0.05, 0.03, 1.0);
        let expected = 75.0 * (0.02_f64).exp();
        assert!((fwd - expected).abs() < 1e-10);
    }

    #[test]
    fn test_analytical_forward_pv() {
        let pv = CommodityAnalyticalPricer::forward_pv(75.0, 75.0, 0.05, 0.03, 1.0);
        // Forward > strike (contango) → positive PV for long
        let fwd = 75.0 * (0.02_f64).exp();
        let df = (-0.05_f64).exp();
        let expected = (fwd - 75.0) * df;
        assert!((pv - expected).abs() < 1e-10);
    }

    #[test]
    fn test_analytical_option_price_call() {
        let pv = CommodityAnalyticalPricer::option_price(75.0, 75.0, 0.05, 0.03, 0.30, 1.0, true);
        assert!(pv > 0.0, "ATM call should have positive value");
        assert!(pv < 75.0, "Call value should be less than spot");
    }

    #[test]
    fn test_analytical_option_price_put() {
        let pv = CommodityAnalyticalPricer::option_price(75.0, 75.0, 0.05, 0.03, 0.30, 1.0, false);
        assert!(pv > 0.0, "ATM put should have positive value");
    }

    #[test]
    fn test_analytical_option_put_call_parity() {
        let spot = 75.0;
        let strike = 80.0;
        let r = 0.05;
        let cy = 0.03;
        let sigma = 0.30;
        let tau = 1.0;

        let call = CommodityAnalyticalPricer::option_price(spot, strike, r, cy, sigma, tau, true);
        let put = CommodityAnalyticalPricer::option_price(spot, strike, r, cy, sigma, tau, false);

        // Put-call parity: C - P = df * (F - K)
        let forward = CommodityAnalyticalPricer::forward_price(spot, r, cy, tau);
        let df = (-r * tau).exp();
        let parity_diff = call - put;
        let expected = df * (forward - strike);

        assert!(
            (parity_diff - expected).abs() < 1e-6,
            "Put-call parity violated: C-P={parity_diff}, df*(F-K)={expected}"
        );
    }

    #[test]
    fn test_analytical_option_zero_vol() {
        // With zero vol, option value should be intrinsic value discounted
        let forward = CommodityAnalyticalPricer::forward_price(75.0, 0.05, 0.03, 1.0);
        let df = (-0.05_f64).exp();

        let itm_call =
            CommodityAnalyticalPricer::option_price(75.0, 70.0, 0.05, 0.03, 0.0001, 1.0, true);
        let expected_itm = df * (forward - 70.0);
        assert!(
            (itm_call - expected_itm).abs() < 0.1,
            "Near-zero vol ITM call ≈ discounted intrinsic"
        );
    }

    #[test]
    fn test_average_expected_cy() {
        let avg = CommodityAnalyticalPricer::average_expected_cy(0.03, 1.2, 0.03, 1.0);
        // When delta0 = theta, average should equal theta
        assert!(
            (avg - 0.03).abs() < 1e-10,
            "Average CY should equal theta when delta0=theta"
        );

        // When delta0 > theta, average should be between theta and delta0
        let avg2 = CommodityAnalyticalPricer::average_expected_cy(0.10, 1.2, 0.03, 1.0);
        assert!(
            avg2 > 0.03 && avg2 < 0.10,
            "Average CY should be between theta and delta0"
        );
    }

    // ── Error Type Tests ──

    #[test]
    fn test_error_display() {
        let param_err =
            GibsonSchwartzError::Param(ParamValidationError::must_be_positive("spot", -75.0));
        assert!(param_err.to_string().contains("spot"));

        let comp_err = GibsonSchwartzError::Computation(ComputationError::non_finite("spot price"));
        assert!(comp_err.to_string().contains("spot price"));
    }

    #[test]
    fn test_error_to_pricing_error() {
        let param_err: pricer_core::types::PricingError =
            GibsonSchwartzError::Param(ParamValidationError::must_be_positive("s", -1.0)).into();
        assert!(matches!(
            param_err,
            pricer_core::types::PricingError::InvalidInput(_)
        ));

        let comp_err: pricer_core::types::PricingError =
            GibsonSchwartzError::Computation(ComputationError::non_finite("x")).into();
        assert!(matches!(
            comp_err,
            pricer_core::types::PricingError::NumericalInstability(_)
        ));
    }

    // ── Generic Float Tests ──

    #[test]
    fn test_model_f32() {
        let p =
            GibsonSchwartzParams::new(75.0_f32, 0.05, 0.03, 1.2, 0.03, 0.35, 0.15, 0.3).unwrap();
        let state = GibsonSchwartzModel::initial_state(&p);
        let next = GibsonSchwartzModel::evolve_step(state, 1.0_f32 / 252.0, &[0.5, 0.3], &p);
        assert!(next.first.is_finite() && next.second.is_finite());
    }
}
