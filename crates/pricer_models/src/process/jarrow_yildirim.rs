//! Jarrow-Yildirim 3-factor inflation model.
//!
//! Three correlated SDEs modelling nominal rates, real rates, and inflation index:
//!
//! ```text
//! dn = [θ_n(t) - a_n * n] dt + σ_n * dW_n    (HW1F nominal)
//! dr = [θ_r(t) - a_r * r] dt + σ_r * dW_r    (HW1F real)
//! dI/I = (n - r) dt + σ_I * dW_I              (inflation index)
//! ```
//!
//! with correlation structure:
//! ```text
//! E[dW_n · dW_r] = ρ_nr dt
//! E[dW_n · dW_I] = ρ_nI dt
//! E[dW_r · dW_I] = ρ_rI dt
//! ```
//!
//! The inflation index drift `(n - r)` is the no-arbitrage (Fisher) condition
//! from the FX analogy: nominal = domestic, real = foreign, I = FX rate.
//!
//! ## Key Features
//!
//! - Analytical ZCIS pricing via `ZcisAnalyticalPricer`
//! - HW1F affine bond price formulas for nominal and real curves
//! - Inline 3×3 Cholesky for AD-compatible correlated Brownian motions
//! - Euler-Maruyama discretisation with log-space inflation index evolution

use pricer_core::{
    math::smoothing::{smooth_max, smooth_sqrt},
    traits::{priceable::Differentiable, Float},
};
use thiserror::Error;

use super::{
    correlated::{CorrelationError, CorrelationMatrix},
    hull_white::ThetaFunction,
    stochastic::{HybridModel, StochasticModel, ThreeFactorState},
    validation::{ComputationError, ParamValidationError, DEFAULT_SMOOTHING_EPSILON},
};
use crate::validate_params;

// ─── Error ───────────────────────────────────────────────────────────────────

/// Jarrow-Yildirim model error type.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum JarrowYildirimError {
    /// Parameter validation error.
    #[error("Parameter error: {0}")]
    Param(#[from] ParamValidationError),

    /// Numerical computation error.
    #[error("Computation error: {0}")]
    Computation(#[from] ComputationError),

    /// Correlation matrix error.
    #[error("Correlation error: {0}")]
    Correlation(#[from] CorrelationError),
}

impl From<JarrowYildirimError> for pricer_core::types::PricingError {
    fn from(err: JarrowYildirimError) -> Self {
        match err {
            JarrowYildirimError::Param(e) => {
                pricer_core::types::PricingError::InvalidInput(e.to_string())
            }
            JarrowYildirimError::Computation(e) => {
                pricer_core::types::PricingError::NumericalInstability(e.to_string())
            }
            JarrowYildirimError::Correlation(e) => {
                pricer_core::types::PricingError::ModelFailure(e.to_string())
            }
        }
    }
}

// ─── HW1F Bond Price Helpers ─────────────────────────────────────────────────

/// HW1F B-factor: `B(t,T) = (1 - exp(-a·τ)) / a` where `τ = T - t`.
///
/// For `a ≈ 0`, falls back to `τ` (the limit as a → 0).
#[inline]
pub fn hw_b_factor<T: Float>(a: T, tau: T) -> T {
    let eps = T::from(1e-10).unwrap_or(T::zero());
    if a.abs() < eps {
        tau
    } else {
        (T::one() - (-a * tau).exp()) / a
    }
}

/// HW1F log bond price for flat initial curve at rate `r*`.
///
/// `ln P(t,T) = -r*·τ + B·r* - σ²/(4a)·B²·(1-exp(-2at)) - B·r_t`
///
/// where `τ = T - t`, `B = B(a, τ)`.
#[inline]
pub fn hw_log_bond_price<T: Float>(a: T, sigma: T, r_star: T, t: T, tau: T, r_t: T) -> T {
    let b = hw_b_factor(a, tau);
    let two = T::from(2.0).unwrap_or(T::one() + T::one());
    let four = two + two;
    let eps = T::from(1e-10).unwrap_or(T::zero());

    let convexity = if a.abs() > eps {
        sigma * sigma / (four * a) * b * b * (T::one() - (-two * a * t).exp())
    } else {
        T::zero()
    };

    -r_star * tau + b * r_star + convexity - b * r_t
}

/// HW1F zero-coupon bond price `P(t,T)` given current short rate `r_t`.
#[inline]
pub fn hw_bond_price<T: Float>(a: T, sigma: T, r_star: T, t: T, maturity: T, r_t: T) -> T {
    let tau = maturity - t;
    hw_log_bond_price(a, sigma, r_star, t, tau, r_t).exp()
}

// ─── Parameters ──────────────────────────────────────────────────────────────

/// Jarrow-Yildirim model parameters.
#[derive(Clone, Debug)]
pub struct JarrowYildirimParams<T: Float> {
    // ── Nominal rate (HW1F) ──
    /// Mean reversion speed for nominal short rate (a_n > 0).
    pub nominal_mean_reversion: T,
    /// Volatility of nominal short rate (σ_n > 0).
    pub nominal_volatility: T,
    /// Initial nominal short rate r_n(0).
    pub initial_nominal_rate: T,
    /// Theta function for nominal rate drift calibration.
    pub nominal_theta: ThetaFunction<T>,

    // ── Real rate (HW1F) ──
    /// Mean reversion speed for real short rate (a_r > 0).
    pub real_mean_reversion: T,
    /// Volatility of real short rate (σ_r > 0).
    pub real_volatility: T,
    /// Initial real short rate r_r(0).
    pub initial_real_rate: T,
    /// Theta function for real rate drift calibration.
    pub real_theta: ThetaFunction<T>,

    // ── Inflation index ──
    /// Volatility of inflation index (σ_I > 0).
    pub inflation_volatility: T,
    /// Initial inflation index level I(0) > 0.
    pub initial_index: T,

    // ── Correlation ──
    /// Correlation between nominal and real rates (ρ_nr ∈ [-1,1]).
    pub rho_nominal_real: T,
    /// Correlation between nominal rate and inflation (ρ_nI ∈ [-1,1]).
    pub rho_nominal_inflation: T,
    /// Correlation between real rate and inflation (ρ_rI ∈ [-1,1]).
    pub rho_real_inflation: T,

    // ── Simulation state ──
    /// Current simulation time (advanced by `advance_time`).
    pub current_time: T,
    /// Smoothing epsilon for AD-compatible approximations.
    pub smoothing_epsilon: T,
}

#[allow(clippy::too_many_arguments)]
impl<T: Float> JarrowYildirimParams<T> {
    /// Create new JY parameters with full specification.
    pub fn new(
        nominal_mean_reversion: T,
        nominal_volatility: T,
        initial_nominal_rate: T,
        real_mean_reversion: T,
        real_volatility: T,
        initial_real_rate: T,
        inflation_volatility: T,
        initial_index: T,
        rho_nominal_real: T,
        rho_nominal_inflation: T,
        rho_real_inflation: T,
    ) -> Result<Self, JarrowYildirimError> {
        let nominal_theta = ThetaFunction::from_flat_curve(
            nominal_mean_reversion,
            nominal_volatility,
            initial_nominal_rate,
        );
        let real_theta = ThetaFunction::from_flat_curve(
            real_mean_reversion,
            real_volatility,
            initial_real_rate,
        );

        let params = Self {
            nominal_mean_reversion,
            nominal_volatility,
            initial_nominal_rate,
            nominal_theta,
            real_mean_reversion,
            real_volatility,
            initial_real_rate,
            real_theta,
            inflation_volatility,
            initial_index,
            rho_nominal_real,
            rho_nominal_inflation,
            rho_real_inflation,
            current_time: T::zero(),
            smoothing_epsilon: T::from(DEFAULT_SMOOTHING_EPSILON).unwrap_or(T::zero()),
        };
        params.validate()?;
        Ok(params)
    }

    /// Create parameters with custom theta functions.
    pub fn with_theta_functions(
        nominal_mean_reversion: T,
        nominal_volatility: T,
        initial_nominal_rate: T,
        nominal_theta: ThetaFunction<T>,
        real_mean_reversion: T,
        real_volatility: T,
        initial_real_rate: T,
        real_theta: ThetaFunction<T>,
        inflation_volatility: T,
        initial_index: T,
        rho_nominal_real: T,
        rho_nominal_inflation: T,
        rho_real_inflation: T,
    ) -> Result<Self, JarrowYildirimError> {
        let params = Self {
            nominal_mean_reversion,
            nominal_volatility,
            initial_nominal_rate,
            nominal_theta,
            real_mean_reversion,
            real_volatility,
            initial_real_rate,
            real_theta,
            inflation_volatility,
            initial_index,
            rho_nominal_real,
            rho_nominal_inflation,
            rho_real_inflation,
            current_time: T::zero(),
            smoothing_epsilon: T::from(DEFAULT_SMOOTHING_EPSILON).unwrap_or(T::zero()),
        };
        params.validate()?;
        Ok(params)
    }

    /// Validate all parameters.
    pub fn validate(&self) -> Result<(), JarrowYildirimError> {
        validate_params! {
            self_val = self, f64_conv = |v: T| v.to_f64().unwrap_or(f64::NAN),
            positive: [
                nominal_mean_reversion, nominal_volatility,
                real_mean_reversion, real_volatility,
                inflation_volatility, initial_index
            ],
            correlation: [rho_nominal_real, rho_nominal_inflation, rho_real_inflation],
        }
    }

    /// Build the 3×3 correlation matrix [nominal, real, inflation].
    pub fn correlation_matrix(&self) -> Result<CorrelationMatrix<T>, CorrelationError> {
        #[rustfmt::skip]
        let data = [
            T::one(),                  self.rho_nominal_real,       self.rho_nominal_inflation,
            self.rho_nominal_real,     T::one(),                    self.rho_real_inflation,
            self.rho_nominal_inflation, self.rho_real_inflation,    T::one(),
        ];
        CorrelationMatrix::new(&data, 3)
    }

    /// Compute nominal zero-coupon bond price P_n(t, T | n_t).
    pub fn nominal_bond_price(&self, t: T, maturity: T, n_t: T) -> T {
        hw_bond_price(
            self.nominal_mean_reversion,
            self.nominal_volatility,
            self.initial_nominal_rate,
            t,
            maturity,
            n_t,
        )
    }

    /// Compute real zero-coupon bond price P_r(t, T | r_t).
    pub fn real_bond_price(&self, t: T, maturity: T, r_t: T) -> T {
        hw_bond_price(
            self.real_mean_reversion,
            self.real_volatility,
            self.initial_real_rate,
            t,
            maturity,
            r_t,
        )
    }

    /// Advance simulation time by `dt`.
    pub fn advance_time(&mut self, dt: T) {
        self.current_time = self.current_time + dt;
    }

    /// Reset simulation time to zero.
    pub fn reset_time(&mut self) {
        self.current_time = T::zero();
    }
}

impl Default for JarrowYildirimParams<f64> {
    fn default() -> Self {
        Self::new(
            0.03,  // nominal mean reversion
            0.01,  // nominal volatility
            0.03,  // initial nominal rate
            0.02,  // real mean reversion
            0.008, // real volatility
            0.01,  // initial real rate
            0.02,  // inflation volatility
            100.0, // initial index
            0.5,   // ρ_nr
            -0.2,  // ρ_nI
            -0.3,  // ρ_rI
        )
        .expect("Default JY params must be valid")
    }
}

// ─── Model ───────────────────────────────────────────────────────────────────

/// Jarrow-Yildirim 3-factor inflation model.
///
/// Uses Euler-Maruyama discretisation with inline 3×3 Cholesky decomposition.
/// The model receives 3 independent standard normals per step and correlates
/// them internally.
#[derive(Clone, Debug, Default)]
pub struct JarrowYildirimModel<T: Float> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Float> JarrowYildirimModel<T> {
    /// Create a new JY model instance.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Float> Differentiable for JarrowYildirimModel<T> {}

impl<T: Float + Default> StochasticModel<T> for JarrowYildirimModel<T> {
    type State = ThreeFactorState<T>;
    type Params = JarrowYildirimParams<T>;

    /// Evolve state by one time step using Euler-Maruyama.
    ///
    /// `dw` must contain 3 independent standard normal random variates:
    /// `[z_nominal, z_real, z_inflation]`. Correlation is applied internally
    /// via inline 3×3 Cholesky decomposition.
    fn evolve_step(state: Self::State, dt: T, dw: &[T], params: &Self::Params) -> Self::State {
        let n = state.first; // nominal short rate
        let r = state.second; // real short rate
        let idx = state.third; // inflation index

        let z1 = dw.first().copied().unwrap_or(T::zero());
        let z2 = dw.get(1).copied().unwrap_or(T::zero());
        let z3 = dw.get(2).copied().unwrap_or(T::zero());

        let eps = params.smoothing_epsilon;

        // ── Inline 3×3 Cholesky: L where C = L·Lᵀ ──
        // C = [[1, ρ_nr, ρ_ni], [ρ_nr, 1, ρ_ri], [ρ_ni, ρ_ri, 1]]
        let rho_nr = params.rho_nominal_real;
        let rho_ni = params.rho_nominal_inflation;
        let rho_ri = params.rho_real_inflation;

        // L[0,0] = 1
        // L[1,0] = ρ_nr, L[1,1] = √(1 - ρ_nr²)
        let l_11 = smooth_sqrt(smooth_max(T::one() - rho_nr * rho_nr, eps, eps), eps);

        // L[2,0] = ρ_ni
        // L[2,1] = (ρ_ri - ρ_nr · ρ_ni) / L[1,1]
        let l_21 = if l_11 > eps {
            (rho_ri - rho_nr * rho_ni) / l_11
        } else {
            T::zero()
        };
        // L[2,2] = √(1 - L[2,0]² - L[2,1]²)
        let l_22_sq = T::one() - rho_ni * rho_ni - l_21 * l_21;
        let l_22 = smooth_sqrt(smooth_max(l_22_sq, eps, eps), eps);

        // Correlated Brownians: W = L · Z
        let dw_n = z1;
        let dw_r = rho_nr * z1 + l_11 * z2;
        let dw_i = rho_ni * z1 + l_21 * z2 + l_22 * z3;

        let sqrt_dt = dt.sqrt();

        // ── Nominal rate (HW1F) ──
        let theta_n = params.nominal_theta.evaluate(params.current_time);
        let drift_n = (theta_n - params.nominal_mean_reversion * n) * dt;
        let diff_n = params.nominal_volatility * sqrt_dt * dw_n;
        let n_next = n + drift_n + diff_n;

        // ── Real rate (HW1F) ──
        let theta_r = params.real_theta.evaluate(params.current_time);
        let drift_r = (theta_r - params.real_mean_reversion * r) * dt;
        let diff_r = params.real_volatility * sqrt_dt * dw_r;
        let r_next = r + drift_r + diff_r;

        // ── Inflation index (log-space for positivity) ──
        let sigma_i = params.inflation_volatility;
        let half = T::from(0.5).unwrap_or(T::zero());
        let log_drift = (n - r - half * sigma_i * sigma_i) * dt;
        let log_diff = sigma_i * sqrt_dt * dw_i;
        let i_next = idx * (log_drift + log_diff).exp();
        let i_next = smooth_max(i_next, eps, eps);

        ThreeFactorState {
            first: n_next,
            second: r_next,
            third: i_next,
        }
    }

    fn initial_state(params: &Self::Params) -> Self::State {
        ThreeFactorState {
            first: params.initial_nominal_rate,
            second: params.initial_real_rate,
            third: params.initial_index,
        }
    }

    fn brownian_dim() -> usize {
        3
    }

    fn model_name() -> &'static str {
        "JarrowYildirim"
    }

    fn num_factors() -> usize {
        3
    }
}

impl<T: Float + Default> HybridModel for JarrowYildirimModel<T> {}

// ─── ZCIS Analytical Pricer ──────────────────────────────────────────────────

/// Analytical pricer for Zero-Coupon Inflation Swaps under the JY model.
///
/// ## ZCIS MtM Formula
///
/// ```text
/// MtM(t) = N × [ I_t / I_0 × P_r(t,T) − (1+K)^T_mat × P_n(t,T) ]
/// ```
///
/// where:
/// - `N` = notional
/// - `I_t` = inflation index at time `t`, `I_0` = base index
/// - `P_r(t,T)` = real zero-coupon bond price
/// - `P_n(t,T)` = nominal zero-coupon bond price
/// - `K` = fixed rate (annual)
/// - `T_mat` = total maturity in years (from inception)
pub struct ZcisAnalyticalPricer;

impl ZcisAnalyticalPricer {
    /// Price a ZCIS at time `t` given current model state.
    ///
    /// # Arguments
    /// - `params`: JY model parameters
    /// - `n_t`: current nominal short rate
    /// - `r_t`: current real short rate
    /// - `i_t`: current inflation index level
    /// - `t`: current time (years from inception)
    /// - `maturity`: swap maturity (years from inception)
    /// - `notional`: swap notional amount
    /// - `fixed_rate`: annual fixed rate K
    /// - `base_index`: base inflation index I_0
    pub fn price<T: Float>(
        params: &JarrowYildirimParams<T>,
        n_t: T,
        r_t: T,
        i_t: T,
        t: T,
        maturity: T,
        notional: T,
        fixed_rate: T,
        base_index: T,
    ) -> T {
        let p_nominal = params.nominal_bond_price(t, maturity, n_t);
        let p_real = params.real_bond_price(t, maturity, r_t);

        // Inflation leg: N × (I_t / I_0) × P_r(t,T)
        let inflation_leg = notional * (i_t / base_index) * p_real;

        // Fixed leg: N × (1+K)^T_mat × P_n(t,T)
        let one_plus_k = T::one() + fixed_rate;
        let fixed_leg = notional * one_plus_k.powf(maturity) * p_nominal;

        inflation_leg - fixed_leg
    }

    /// Decompose ZCIS into leg PVs.
    pub fn leg_pvs<T: Float>(
        params: &JarrowYildirimParams<T>,
        n_t: T,
        r_t: T,
        i_t: T,
        t: T,
        maturity: T,
        notional: T,
        fixed_rate: T,
        base_index: T,
    ) -> (T, T) {
        let p_nominal = params.nominal_bond_price(t, maturity, n_t);
        let p_real = params.real_bond_price(t, maturity, r_t);

        let inflation_leg = notional * (i_t / base_index) * p_real;
        let one_plus_k = T::one() + fixed_rate;
        let fixed_leg = notional * one_plus_k.powf(maturity) * p_nominal;

        (inflation_leg, fixed_leg)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::stochastic::{StochasticModel, StochasticState};

    fn default_params() -> JarrowYildirimParams<f64> {
        JarrowYildirimParams::default()
    }

    // ── Parameter Tests ──

    #[test]
    fn test_params_new_valid() {
        let p = default_params();
        assert_eq!(p.nominal_mean_reversion, 0.03);
        assert_eq!(p.nominal_volatility, 0.01);
        assert_eq!(p.initial_nominal_rate, 0.03);
        assert_eq!(p.real_mean_reversion, 0.02);
        assert_eq!(p.real_volatility, 0.008);
        assert_eq!(p.initial_real_rate, 0.01);
        assert_eq!(p.inflation_volatility, 0.02);
        assert_eq!(p.initial_index, 100.0);
        assert_eq!(p.rho_nominal_real, 0.5);
        assert_eq!(p.rho_nominal_inflation, -0.2);
        assert_eq!(p.rho_real_inflation, -0.3);
    }

    #[test]
    fn test_params_rejects_invalid() {
        // Negative mean reversion
        assert!(JarrowYildirimParams::new(
            -0.03, 0.01, 0.03, 0.02, 0.008, 0.01, 0.02, 100.0, 0.5, -0.2, -0.3
        )
        .is_err());

        // Zero volatility
        assert!(JarrowYildirimParams::new(
            0.03, 0.0, 0.03, 0.02, 0.008, 0.01, 0.02, 100.0, 0.5, -0.2, -0.3
        )
        .is_err());

        // Negative initial index
        assert!(JarrowYildirimParams::new(
            0.03, 0.01, 0.03, 0.02, 0.008, 0.01, 0.02, -100.0, 0.5, -0.2, -0.3
        )
        .is_err());

        // Correlation out of range
        assert!(JarrowYildirimParams::new(
            0.03, 0.01, 0.03, 0.02, 0.008, 0.01, 0.02, 100.0, 1.5, -0.2, -0.3
        )
        .is_err());
    }

    #[test]
    fn test_params_boundary_correlations() {
        // Exact ±1 should be accepted (closed interval)
        assert!(JarrowYildirimParams::new(
            0.03, 0.01, 0.03, 0.02, 0.008, 0.01, 0.02, 100.0, 1.0, -1.0, 0.0
        )
        .is_ok());
    }

    #[test]
    fn test_params_correlation_matrix() {
        let p = default_params();
        let corr = p.correlation_matrix();
        assert!(corr.is_ok());
        let m = corr.unwrap();
        assert_eq!(m.dim(), 3);
        assert!((m.get(0, 1) - 0.5).abs() < 1e-10);
        assert!((m.get(0, 2) - (-0.2)).abs() < 1e-10);
        assert!((m.get(1, 2) - (-0.3)).abs() < 1e-10);
    }

    #[test]
    fn test_params_advance_time() {
        let mut p = default_params();
        assert!((p.current_time - 0.0).abs() < 1e-10);
        p.advance_time(0.1);
        assert!((p.current_time - 0.1).abs() < 1e-10);
        p.advance_time(0.2);
        assert!((p.current_time - 0.3).abs() < 1e-10);
        p.reset_time();
        assert!((p.current_time - 0.0).abs() < 1e-10);
    }

    // ── Bond Price Tests ──

    #[test]
    fn test_hw_b_factor() {
        // B(a, τ) = (1 - exp(-a·τ)) / a
        let b = hw_b_factor(0.1_f64, 1.0);
        let expected = (1.0 - (-0.1_f64).exp()) / 0.1;
        assert!((b - expected).abs() < 1e-10);

        // Near-zero mean reversion → B ≈ τ
        let b_small = hw_b_factor(1e-12_f64, 5.0);
        assert!((b_small - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_bond_price_at_zero() {
        // At t=0 with r(0) = r*, P(0,T) should equal exp(-r*·T)
        let r_star = 0.03_f64;
        let price = hw_bond_price(0.05, 0.01, r_star, 0.0, 5.0, r_star);
        let expected = (-r_star * 5.0_f64).exp();
        assert!(
            (price - expected).abs() < 1e-6,
            "P(0,5) = {price}, expected {expected}"
        );
    }

    #[test]
    fn test_bond_price_positive() {
        let p = default_params();
        let price_n = p.nominal_bond_price(0.0, 5.0, 0.03);
        let price_r = p.real_bond_price(0.0, 5.0, 0.01);
        assert!(price_n > 0.0 && price_n < 1.5);
        assert!(price_r > 0.0 && price_r < 1.5);
    }

    #[test]
    fn test_bond_price_monotone_rate() {
        let p = default_params();
        // Higher rate → lower bond price
        let p_low = p.nominal_bond_price(0.0, 5.0, 0.01);
        let p_high = p.nominal_bond_price(0.0, 5.0, 0.10);
        assert!(p_low > p_high);
    }

    #[test]
    fn test_bond_price_monotone_maturity() {
        let p = default_params();
        // Longer maturity → lower bond price (for positive rates)
        let p_short = p.nominal_bond_price(0.0, 1.0, 0.03);
        let p_long = p.nominal_bond_price(0.0, 10.0, 0.03);
        assert!(p_short > p_long);
    }

    // ── Model Tests ──

    #[test]
    fn test_model_basics() {
        assert_eq!(JarrowYildirimModel::<f64>::brownian_dim(), 3);
        assert_eq!(JarrowYildirimModel::<f64>::model_name(), "JarrowYildirim");
        assert_eq!(JarrowYildirimModel::<f64>::num_factors(), 3);
    }

    #[test]
    fn test_initial_state() {
        let p = default_params();
        let state = JarrowYildirimModel::initial_state(&p);
        assert_eq!(state.first, 0.03); // nominal rate
        assert_eq!(state.second, 0.01); // real rate
        assert_eq!(state.third, 100.0); // inflation index
    }

    #[test]
    fn test_evolve_step_no_shock() {
        let p = default_params();
        let state = JarrowYildirimModel::initial_state(&p);
        let dt = 1.0 / 252.0;
        let dw = [0.0, 0.0, 0.0];

        let next = JarrowYildirimModel::evolve_step(state, dt, &dw, &p);

        // Rates should barely change with zero shock
        assert!((next.first - state.first).abs() < 1e-6);
        assert!((next.second - state.second).abs() < 1e-6);
        // Index drift = (n - r) * dt = (0.03 - 0.01) * dt ≈ small positive
        assert!(next.third > 0.0);
    }

    #[test]
    fn test_evolve_step_positive_shock() {
        let p = default_params();
        let state = JarrowYildirimModel::initial_state(&p);
        let dt = 1.0 / 252.0;
        let dw = [2.0, 0.0, 0.0]; // strong positive shock to nominal rate

        let next = JarrowYildirimModel::evolve_step(state, dt, &dw, &p);
        assert!(next.first > state.first, "Positive shock should raise nominal rate");
    }

    #[test]
    fn test_evolve_step_inflation_positivity() {
        let p = default_params();
        let state = JarrowYildirimModel::initial_state(&p);
        let dt = 1.0 / 252.0;

        // Even with strong negative shock, index must stay positive
        let dw = [-3.0, -3.0, -3.0];
        let next = JarrowYildirimModel::evolve_step(state, dt, &dw, &p);
        assert!(next.third > 0.0, "Inflation index must remain positive");
    }

    #[test]
    fn test_multi_step_stability() {
        let p = default_params();
        let mut state = JarrowYildirimModel::initial_state(&p);
        let dt = 1.0 / 252.0;

        for i in 0..252 {
            let phase = i as f64 * 0.1;
            let dw = [
                0.1 * phase.sin(),
                0.1 * phase.cos(),
                0.1 * (phase * 0.7).sin(),
            ];
            state = JarrowYildirimModel::evolve_step(state, dt, &dw, &p);

            assert!(state.first.is_finite(), "Nominal rate non-finite at step {i}");
            assert!(state.second.is_finite(), "Real rate non-finite at step {i}");
            assert!(
                state.third > 0.0 && state.third.is_finite(),
                "Index invalid at step {i}: {}",
                state.third
            );
        }
    }

    #[test]
    fn test_correlation_effect() {
        // With high positive ρ_nr, nominal and real shocks should be correlated
        let p = JarrowYildirimParams::new(
            0.03, 0.01, 0.03, 0.02, 0.008, 0.01, 0.02, 100.0,
            0.99, 0.0, 0.0, // high ρ_nr
        )
        .unwrap();
        let state = JarrowYildirimModel::initial_state(&p);
        let dt = 1.0 / 252.0;

        // Shock only the first independent normal
        let dw = [2.0, 0.0, 0.0];
        let next = JarrowYildirimModel::evolve_step(state, dt, &dw, &p);

        // Both rates should move up (correlated via Cholesky)
        assert!(next.first > state.first, "Nominal should increase");
        assert!(next.second > state.second, "Real should increase (ρ=0.99)");
    }

    #[test]
    fn test_long_simulation_stability() {
        let p = default_params();
        let mut state = JarrowYildirimModel::initial_state(&p);
        let dt = 1.0 / 252.0;

        // 5-year simulation
        for i in 0..(252 * 5) {
            let phase = i as f64 * 0.1;
            let dw = [
                0.1 * phase.sin(),
                0.1 * phase.cos(),
                0.1 * (phase * 0.3).sin(),
            ];
            state = JarrowYildirimModel::evolve_step(state, dt, &dw, &p);
        }

        assert!(state.first.is_finite());
        assert!(state.second.is_finite());
        assert!(state.third > 0.0 && state.third.is_finite());
    }

    #[test]
    fn test_stochastic_state_trait() {
        let state = ThreeFactorState::<f64> {
            first: 0.03,
            second: 0.01,
            third: 100.0,
        };
        assert_eq!(ThreeFactorState::<f64>::dimension(), 3);
        assert_eq!(state.get(0), Some(0.03));
        assert_eq!(state.get(1), Some(0.01));
        assert_eq!(state.get(2), Some(100.0));
        assert_eq!(state.to_array(), vec![0.03, 0.01, 100.0]);
    }

    #[test]
    fn test_model_f32() {
        let p = JarrowYildirimParams::new(
            0.03_f32, 0.01, 0.03, 0.02, 0.008, 0.01, 0.02, 100.0, 0.5, -0.2, -0.3,
        )
        .unwrap();
        let state = JarrowYildirimModel::initial_state(&p);
        let next =
            JarrowYildirimModel::evolve_step(state, 1.0_f32 / 252.0, &[0.5, 0.0, 0.0], &p);
        assert!(next.first.is_finite() && next.second.is_finite() && next.third > 0.0);
    }

    // ── ZCIS Pricer Tests ──

    #[test]
    fn test_zcis_at_inception() {
        let p = default_params();
        // At t=0, a fairly-priced ZCIS should have MtM ≈ 0
        // The "fair" fixed rate K satisfies:
        //   I_0/I_0 * P_r(0,T) = (1+K)^T * P_n(0,T)
        //   P_r(0,T) / P_n(0,T) = (1+K)^T
        // For T=5: (1+K)^5 = P_r/P_n

        let maturity = 5.0;
        let p_r = p.real_bond_price(0.0, maturity, 0.01);
        let p_n = p.nominal_bond_price(0.0, maturity, 0.03);
        let fair_k = (p_r / p_n).powf(1.0 / maturity) - 1.0;

        let mtm = ZcisAnalyticalPricer::price(
            &p,
            0.03,
            0.01,
            100.0,
            0.0,
            maturity,
            1_000_000.0,
            fair_k,
            100.0,
        );

        assert!(
            mtm.abs() < 1.0,
            "Fair-rate ZCIS should have MtM ≈ 0, got {mtm}"
        );
    }

    #[test]
    fn test_zcis_leg_decomposition() {
        let p = default_params();
        let (infl, fixed) = ZcisAnalyticalPricer::leg_pvs(
            &p,
            0.03,
            0.01,
            100.0,
            0.0,
            5.0,
            1_000_000.0,
            0.02,
            100.0,
        );
        assert!(infl > 0.0, "Inflation leg PV should be positive");
        assert!(fixed > 0.0, "Fixed leg PV should be positive");

        let mtm = ZcisAnalyticalPricer::price(
            &p,
            0.03,
            0.01,
            100.0,
            0.0,
            5.0,
            1_000_000.0,
            0.02,
            100.0,
        );
        assert!((mtm - (infl - fixed)).abs() < 1e-6, "MtM = Infl - Fixed");
    }

    #[test]
    fn test_zcis_higher_index_positive_mtm() {
        let p = default_params();
        // If inflation index rose from 100 to 110, inflation receiver benefits
        let mtm = ZcisAnalyticalPricer::price(
            &p,
            0.03,
            0.01,
            110.0, // index appreciated
            1.0,
            5.0,
            1_000_000.0,
            0.02,
            100.0,
        );
        // MtM should be more positive than at-par
        let mtm_par = ZcisAnalyticalPricer::price(
            &p,
            0.03,
            0.01,
            100.0,
            1.0,
            5.0,
            1_000_000.0,
            0.02,
            100.0,
        );
        assert!(mtm > mtm_par, "Higher index → higher MtM for inflation receiver");
    }

    // ── Error Type Tests ──

    #[test]
    fn test_error_display() {
        let param_err =
            JarrowYildirimError::Param(ParamValidationError::must_be_positive("sigma_n", -0.01));
        assert!(param_err.to_string().contains("sigma_n"));

        let comp_err =
            JarrowYildirimError::Computation(ComputationError::non_finite("bond price"));
        assert!(comp_err.to_string().contains("bond price"));

        let corr_err = JarrowYildirimError::Correlation(CorrelationError::NotPositiveDefinite);
        assert!(corr_err.to_string().contains("positive definite"));
    }

    #[test]
    fn test_error_to_pricing_error() {
        let param_err: pricer_core::types::PricingError =
            JarrowYildirimError::Param(ParamValidationError::must_be_positive("v", -1.0)).into();
        assert!(matches!(
            param_err,
            pricer_core::types::PricingError::InvalidInput(_)
        ));

        let comp_err: pricer_core::types::PricingError =
            JarrowYildirimError::Computation(ComputationError::non_finite("x")).into();
        assert!(matches!(
            comp_err,
            pricer_core::types::PricingError::NumericalInstability(_)
        ));

        let corr_err: pricer_core::types::PricingError =
            JarrowYildirimError::Correlation(CorrelationError::NotPositiveDefinite).into();
        assert!(matches!(
            corr_err,
            pricer_core::types::PricingError::ModelFailure(_)
        ));
    }
}
