//! Monte Carlo pricing engine.
//!
//! This module provides the orchestration layer for Monte Carlo pricing
//! with optional automatic differentiation for Greeks computation.
//!
//! # Overview
//!
//! The [`MonteCarloPricer`] coordinates:
//! 1. Random number generation (via
//!    [`PricerRng`](pricer_core::math::rng::PricerRng))
//! 2. Path generation (via
//!    [`generate_gbm_paths`](super::paths::generate_gbm_paths))
//! 3. Payoff computation (via
//!    [`compute_payoffs`](super::payoff::compute_payoffs))
//! 4. Discounting and aggregation
//! 5. Greeks via AD (or bump-and-revalue as placeholder)
//!
//! # Workspace Reuse
//!
//! The pricer maintains an internal
//! [`PathWorkspace`](super::workspace::PathWorkspace) that is reused across
//! pricing calls, minimising memory allocations.

use pricer_core::math::rng::PricerRng;

use super::{
    config::MonteCarloConfig,
    error::MonteCarloConfigError,
    paths::{generate_gbm_paths, generate_gbm_paths_tangent_spot, GbmParams},
    payoff::{compute_payoff, compute_payoffs, PayoffParams},
    streaming::{
        ArithmeticAverageObserver, BarrierObserver, EuropeanObserver, LookbackObserver,
        StreamingEngine, StreamingObserver,
    },
    workspace::PathWorkspace,
};
use crate::methods::path_dependent::{PathDependentPayoff, PathObserver, PathPayoffType};

/// Greek type for selection.
///
/// First-order: Delta (∂V/∂S), Vega (∂V/∂σ), Theta (∂V/∂τ), Rho (∂V/∂r).
/// Second-order: Gamma (∂²V/∂S²), Vanna (∂²V/∂S∂σ), Volga (∂²V/∂σ²).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Greek {
    /// ∂V/∂S
    Delta,
    /// ∂V/∂σ
    Vega,
    /// ∂V/∂τ
    Theta,
    /// ∂V/∂r
    Rho,
    /// ∂²V/∂S²
    Gamma,
    /// ∂²V/∂S∂σ
    Vanna,
    /// ∂²V/∂σ²
    Volga,
}

/// Pricing result with optional Greeks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PricingResult {
    /// Present value.
    pub price: f64,
    /// Standard error.
    pub std_error: f64,
    /// ∂V/∂S
    pub delta: Option<f64>,
    /// ∂V/∂σ
    pub vega: Option<f64>,
    /// ∂V/∂τ
    pub theta: Option<f64>,
    /// ∂V/∂r
    pub rho: Option<f64>,
    /// ∂²V/∂S²
    pub gamma: Option<f64>,
    /// ∂²V/∂S∂σ
    pub vanna: Option<f64>,
    /// ∂²V/∂σ²
    pub volga: Option<f64>,
}

impl PricingResult {
    /// Returns the 95% confidence interval half-width.
    #[inline]
    pub fn confidence_95(&self) -> f64 { 1.96 * self.std_error }

    /// Returns the 99% confidence interval half-width.
    #[inline]
    pub fn confidence_99(&self) -> f64 { 2.576 * self.std_error }
}

/// Monte Carlo pricing engine with workspace reuse and bump-and-revalue Greeks.
pub struct MonteCarloPricer {
    config: MonteCarloConfig,
    workspace: PathWorkspace,
    /// Random number generator (pub(crate) for Enzyme AD access).
    pub(crate) rng: PricerRng,
}

/// Pricing mode for unified bump-and-revalue Greeks.
#[derive(Clone, Copy)]
enum PricingMode {
    European(PayoffParams),
    PathDependent(PathPayoffType<f64>),
}

impl MonteCarloPricer {
    /// Creates a new pricer. Returns error if `config` is invalid.
    pub fn new(config: MonteCarloConfig) -> Result<Self, MonteCarloConfigError> {
        config.validate()?;

        let seed = config.seed().unwrap_or(0);
        let workspace = PathWorkspace::new(config.n_paths(), config.n_steps());
        let rng = PricerRng::from_seed(seed);

        Ok(Self {
            config,
            workspace,
            rng,
        })
    }

    /// Creates a new pricer with a specific seed (overrides config seed).
    pub fn with_seed(config: MonteCarloConfig, seed: u64) -> Result<Self, MonteCarloConfigError> {
        config.validate()?;

        let workspace = PathWorkspace::new(config.n_paths(), config.n_steps());
        let rng = PricerRng::from_seed(seed);

        Ok(Self {
            config,
            workspace,
            rng,
        })
    }

    /// Returns a reference to the configuration.
    #[inline]
    pub fn config(&self) -> &MonteCarloConfig { &self.config }

    /// Returns the current RNG seed for reproducible finite difference
    /// calculations.
    #[inline]
    pub fn current_seed(&self) -> u64 { self.rng.seed() }

    /// Resets the pricer state (workspace + RNG with original seed).
    pub fn reset(&mut self) {
        self.workspace.reset();
        self.rng = PricerRng::from_seed(self.config.seed().unwrap_or(0));
    }

    /// Resets the pricer with a new seed.
    pub fn reset_with_seed(&mut self, seed: u64) {
        self.workspace.reset();
        self.rng = PricerRng::from_seed(seed);
    }

    /// Prices a European option using Monte Carlo simulation.
    pub fn price_european(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> PricingResult {
        let n_paths = self.config.n_paths();
        let n_steps = self.config.n_steps();

        // Ensure workspace capacity
        self.workspace.ensure_capacity(n_paths, n_steps);

        // Generate random samples
        self.rng.fill_normal(self.workspace.randoms_mut());

        // Generate paths
        generate_gbm_paths(&mut self.workspace, gbm, n_paths, n_steps);

        // Compute payoffs
        compute_payoffs(&mut self.workspace, payoff, n_paths, n_steps);

        // Aggregate: discounted mean and standard error
        let payoffs = self.workspace.payoffs();
        let sum: f64 = payoffs.iter().sum();
        let mean = sum / n_paths as f64;

        let variance: f64 =
            payoffs.iter().map(|&p| (p - mean).powi(2)).sum::<f64>() / (n_paths - 1) as f64;
        let std_dev = variance.sqrt();
        let std_error = std_dev / (n_paths as f64).sqrt();

        PricingResult {
            price: mean * discount_factor,
            std_error: std_error * discount_factor,
            ..Default::default()
        }
    }

    /// Prices a European option with selected Greeks via bump-and-revalue.
    pub fn price_with_greeks(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
        greeks: &[Greek],
    ) -> PricingResult {
        let mut result = self.price_european(gbm, payoff, discount_factor);
        let mode = PricingMode::European(payoff);
        for greek in greeks {
            match greek {
                Greek::Delta => result.delta = Some(self.fd_delta(gbm, mode, discount_factor)),
                Greek::Gamma => result.gamma = Some(self.fd_gamma(gbm, mode, discount_factor)),
                Greek::Vega => result.vega = Some(self.fd_vega(gbm, mode, discount_factor)),
                Greek::Theta => result.theta = Some(self.fd_theta(gbm, mode, discount_factor)),
                Greek::Rho => result.rho = Some(self.fd_rho(gbm, mode)),
                Greek::Vanna => result.vanna = Some(self.fd_vanna(gbm, mode, discount_factor)),
                Greek::Volga => result.volga = Some(self.fd_volga(gbm, mode, discount_factor)),
            }
        }
        result
    }

    // ========================================================================
    // Bump-and-Revalue Greeks (unified for European and path-dependent)
    // ========================================================================

    /// Reprices with the given pricing mode.
    fn reprice(&mut self, gbm: GbmParams, mode: PricingMode, df: f64) -> f64 {
        match mode {
            PricingMode::European(p) => self.price_european(gbm, p, df).price,
            PricingMode::PathDependent(p) => self.price_path_dependent(gbm, p, df).price,
        }
    }

    /// Delta: (V(S+h) - V(S-h)) / 2h.
    fn fd_delta(&mut self, gbm: GbmParams, mode: PricingMode, df: f64) -> f64 {
        let h = (0.01 * gbm.spot).max(0.01);
        let seed = self.rng.seed();
        self.reset_with_seed(seed);
        let up = self.reprice(
            GbmParams {
                spot: gbm.spot + h,
                ..gbm
            },
            mode,
            df,
        );
        self.reset_with_seed(seed);
        let dn = self.reprice(
            GbmParams {
                spot: gbm.spot - h,
                ..gbm
            },
            mode,
            df,
        );
        (up - dn) / (2.0 * h)
    }

    /// Gamma: (V(S+h) - 2V(S) + V(S-h)) / h².
    fn fd_gamma(&mut self, gbm: GbmParams, mode: PricingMode, df: f64) -> f64 {
        let h = (0.01 * gbm.spot).max(0.01);
        let seed = self.rng.seed();
        self.reset_with_seed(seed);
        let mid = self.reprice(gbm, mode, df);
        self.reset_with_seed(seed);
        let up = self.reprice(
            GbmParams {
                spot: gbm.spot + h,
                ..gbm
            },
            mode,
            df,
        );
        self.reset_with_seed(seed);
        let dn = self.reprice(
            GbmParams {
                spot: gbm.spot - h,
                ..gbm
            },
            mode,
            df,
        );
        (up - 2.0 * mid + dn) / (h * h)
    }

    /// Vega: (V(σ+h) - V(σ-h)) / 2h.
    fn fd_vega(&mut self, gbm: GbmParams, mode: PricingMode, df: f64) -> f64 {
        let h = 0.01;
        let seed = self.rng.seed();
        self.reset_with_seed(seed);
        let up = self.reprice(
            GbmParams {
                volatility: gbm.volatility + h,
                ..gbm
            },
            mode,
            df,
        );
        self.reset_with_seed(seed);
        let dn = self.reprice(
            GbmParams {
                volatility: (gbm.volatility - h).max(0.001),
                ..gbm
            },
            mode,
            df,
        );
        (up - dn) / (2.0 * h)
    }

    /// Theta: -(V(T) - V(T-h)) / h, h = 1/252 (one day).
    fn fd_theta(&mut self, gbm: GbmParams, mode: PricingMode, df: f64) -> f64 {
        let h = 1.0 / 252.0;
        let seed = self.rng.seed();
        self.reset_with_seed(seed);
        let short = self.reprice(
            GbmParams {
                maturity: (gbm.maturity - h).max(0.001),
                ..gbm
            },
            mode,
            df,
        );
        self.reset_with_seed(seed);
        let orig = self.reprice(gbm, mode, df);
        -(orig - short) / h
    }

    /// Rho: (V(r+h) - V(r-h)) / 2h with recalculated discount factors.
    fn fd_rho(&mut self, gbm: GbmParams, mode: PricingMode) -> f64 {
        let h = 0.01;
        let seed = self.rng.seed();
        self.reset_with_seed(seed);
        let df_up = (-(gbm.rate + h) * gbm.maturity).exp();
        let up = self.reprice(
            GbmParams {
                rate: gbm.rate + h,
                ..gbm
            },
            mode,
            df_up,
        );
        self.reset_with_seed(seed);
        let df_dn = (-(gbm.rate - h) * gbm.maturity).exp();
        let dn = self.reprice(
            GbmParams {
                rate: gbm.rate - h,
                ..gbm
            },
            mode,
            df_dn,
        );
        (up - dn) / (2.0 * h)
    }

    /// Vanna: (V(S+h,σ+k) - V(S+h,σ-k) - V(S-h,σ+k) + V(S-h,σ-k)) / 4hk.
    fn fd_vanna(&mut self, gbm: GbmParams, mode: PricingMode, df: f64) -> f64 {
        let h = (0.01 * gbm.spot).max(0.01);
        let k = 0.01;
        let vol_dn = (gbm.volatility - k).max(0.001);
        let seed = self.rng.seed();
        self.reset_with_seed(seed);
        let uu = self.reprice(
            GbmParams {
                spot: gbm.spot + h,
                volatility: gbm.volatility + k,
                ..gbm
            },
            mode,
            df,
        );
        self.reset_with_seed(seed);
        let ud = self.reprice(
            GbmParams {
                spot: gbm.spot + h,
                volatility: vol_dn,
                ..gbm
            },
            mode,
            df,
        );
        self.reset_with_seed(seed);
        let du = self.reprice(
            GbmParams {
                spot: gbm.spot - h,
                volatility: gbm.volatility + k,
                ..gbm
            },
            mode,
            df,
        );
        self.reset_with_seed(seed);
        let dd = self.reprice(
            GbmParams {
                spot: gbm.spot - h,
                volatility: vol_dn,
                ..gbm
            },
            mode,
            df,
        );
        (uu - ud - du + dd) / (4.0 * h * k)
    }

    /// Volga: (V(σ+h) - 2V(σ) + V(σ-h)) / h².
    fn fd_volga(&mut self, gbm: GbmParams, mode: PricingMode, df: f64) -> f64 {
        let h = 0.01;
        let seed = self.rng.seed();
        self.reset_with_seed(seed);
        let mid = self.reprice(gbm, mode, df);
        self.reset_with_seed(seed);
        let up = self.reprice(
            GbmParams {
                volatility: gbm.volatility + h,
                ..gbm
            },
            mode,
            df,
        );
        self.reset_with_seed(seed);
        let dn = self.reprice(
            GbmParams {
                volatility: (gbm.volatility - h).max(0.001),
                ..gbm
            },
            mode,
            df,
        );
        (up - 2.0 * mid + dn) / (h * h)
    }

    // ========================================================================
    // Phase 4: L1/L2 Integration - YieldCurve methods
    // ========================================================================

    /// Prices a European option with discount factor from a `YieldCurve`.
    ///
    /// # Panics
    ///
    /// Panics if the curve returns an error for the given maturity.
    #[cfg(feature = "l1l2-integration")]
    pub fn price_european_with_curve<C>(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        curve: &C,
    ) -> PricingResult
    where
        C: pricer_models::market::curves::YieldCurve<f64>,
    {
        let discount_factor = curve
            .discount_factor(gbm.maturity)
            .expect("YieldCurve::discount_factor failed for given maturity");
        self.price_european(gbm, payoff, discount_factor)
    }

    /// Prices a European option with Greeks, discount factor from a
    /// `YieldCurve`.
    #[cfg(feature = "l1l2-integration")]
    pub fn price_with_greeks_and_curve<C>(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        curve: &C,
        greeks: &[Greek],
    ) -> PricingResult
    where
        C: pricer_models::market::curves::YieldCurve<f64>,
    {
        let discount_factor = curve
            .discount_factor(gbm.maturity)
            .expect("YieldCurve::discount_factor failed for given maturity");
        self.price_with_greeks(gbm, payoff, discount_factor, greeks)
    }

    /// Prices with forward-mode AD for Delta via tangent propagation.
    /// Returns (price, delta).
    pub fn price_with_delta_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> (f64, f64) {
        let n_paths = self.config.n_paths();
        let n_steps = self.config.n_steps();

        // Ensure workspace capacity
        self.workspace.ensure_capacity(n_paths, n_steps);

        // Generate random samples
        self.rng.fill_normal(self.workspace.randoms_mut());

        // Generate paths with tangent (d/dS₀)
        let tangent_paths = generate_gbm_paths_tangent_spot(
            &mut self.workspace,
            gbm,
            1.0, // d_spot = 1.0 (seed tangent)
            n_paths,
            n_steps,
        );

        // Compute payoffs and their tangents
        let paths = self.workspace.paths();
        let n_steps_plus_1 = n_steps + 1;

        let mut price_sum = 0.0;
        let mut delta_sum = 0.0;

        for path_idx in 0..n_paths {
            let terminal_price = paths[path_idx * n_steps_plus_1 + n_steps];
            let terminal_tangent = tangent_paths[path_idx * n_steps_plus_1 + n_steps];

            // Primal payoff
            let payoff_value = compute_payoff(terminal_price, payoff);
            price_sum += payoff_value;

            // Tangent payoff: d(payoff)/d(spot) = d(payoff)/d(terminal) ×
            // d(terminal)/d(spot)
            let payoff_deriv = super::payoff::soft_plus_derivative(
                match payoff.payoff_type {
                    super::payoff::PayoffType::Call => terminal_price - payoff.strike,
                    super::payoff::PayoffType::Put => payoff.strike - terminal_price,
                },
                payoff.smoothing_epsilon,
            );

            // Sign adjustment for put
            let sign = match payoff.payoff_type {
                super::payoff::PayoffType::Call => 1.0,
                super::payoff::PayoffType::Put => -1.0,
            };

            delta_sum += payoff_deriv * sign * terminal_tangent;
        }

        let price = (price_sum / n_paths as f64) * discount_factor;
        let delta = (delta_sum / n_paths as f64) * discount_factor;

        (price, delta)
    }

    // ========================================================================
    // Phase 4: Path-Dependent Options Integration
    // ========================================================================

    /// Prices a path-dependent option (Asian, Barrier, Lookback) using Monte
    /// Carlo.
    pub fn price_path_dependent(
        &mut self,
        gbm: GbmParams,
        payoff: PathPayoffType<f64>,
        discount_factor: f64,
    ) -> PricingResult {
        let n_paths = self.config.n_paths();
        let n_steps = self.config.n_steps();
        let n_steps_plus_1 = n_steps + 1;

        // Ensure workspace capacity
        self.workspace.ensure_capacity(n_paths, n_steps);

        // Generate random samples
        self.rng.fill_normal(self.workspace.randoms_mut());

        // Generate GBM paths
        generate_gbm_paths(&mut self.workspace, gbm, n_paths, n_steps);

        let paths = self.workspace.paths();

        // Compute path-dependent payoffs
        let mut payoff_sum = 0.0;
        let mut payoff_sum_sq = 0.0;

        for path_idx in 0..n_paths {
            let mut observer: PathObserver<f64> = PathObserver::new();

            // Observe each price in the path
            for step_idx in 0..n_steps_plus_1 {
                let price = paths[path_idx * n_steps_plus_1 + step_idx];
                observer.observe(price);
            }

            // Set terminal price
            let terminal = paths[path_idx * n_steps_plus_1 + n_steps];
            observer.set_terminal(terminal);

            // Compute payoff
            let payoff_value = payoff.compute(&[], &observer);
            payoff_sum += payoff_value;
            payoff_sum_sq += payoff_value * payoff_value;
        }

        // Aggregate: discounted mean and standard error
        let mean = payoff_sum / n_paths as f64;
        let variance = (payoff_sum_sq / n_paths as f64) - mean * mean;
        let std_dev = variance.max(0.0).sqrt();
        let std_error = std_dev / (n_paths as f64).sqrt();

        PricingResult {
            price: mean * discount_factor,
            std_error: std_error * discount_factor,
            ..Default::default()
        }
    }

    /// Prices a path-dependent option with selected Greeks via
    /// bump-and-revalue.
    pub fn price_path_dependent_with_greeks(
        &mut self,
        gbm: GbmParams,
        payoff: PathPayoffType<f64>,
        discount_factor: f64,
        greeks: &[Greek],
    ) -> PricingResult {
        let mut result = self.price_path_dependent(gbm, payoff, discount_factor);
        let mode = PricingMode::PathDependent(payoff);
        for greek in greeks {
            match greek {
                Greek::Delta => result.delta = Some(self.fd_delta(gbm, mode, discount_factor)),
                Greek::Gamma => result.gamma = Some(self.fd_gamma(gbm, mode, discount_factor)),
                Greek::Vega => result.vega = Some(self.fd_vega(gbm, mode, discount_factor)),
                Greek::Theta => result.theta = Some(self.fd_theta(gbm, mode, discount_factor)),
                Greek::Rho => result.rho = Some(self.fd_rho(gbm, mode)),
                Greek::Vanna | Greek::Volga => {} // Not yet implemented for path-dependent
            }
        }
        result
    }

    // ========================================================================
    // Streaming Mode Methods
    // ========================================================================

    /// Prices a European option using streaming mode (O(paths) memory).
    pub fn price_streaming(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> PricingResult {
        let n_paths = self.config.n_paths();
        let n_steps = self.config.n_steps();
        let seed = self.config.seed().unwrap_or(0);

        // Create streaming engine
        let streaming_config = *self.config.streaming();
        let mut engine = StreamingEngine::new(n_paths, n_steps, streaming_config, seed);

        // Create observer based on payoff type
        let is_call = matches!(payoff.payoff_type, super::payoff::PayoffType::Call);
        let mut observer =
            EuropeanObserver::new(n_paths, payoff.strike, payoff.smoothing_epsilon, is_call);

        // Run streaming simulation
        let result = engine.run(gbm, &mut observer);

        PricingResult {
            price: result.mean * discount_factor,
            std_error: result.std_error * discount_factor,
            ..Default::default()
        }
    }

    /// Prices an Asian option (arithmetic average) using streaming mode.
    pub fn price_asian_streaming(
        &mut self,
        gbm: GbmParams,
        strike: f64,
        is_call: bool,
        discount_factor: f64,
    ) -> PricingResult {
        let n_paths = self.config.n_paths();
        let n_steps = self.config.n_steps();
        let seed = self.config.seed().unwrap_or(0);
        let epsilon = 1e-6;

        let streaming_config = *self.config.streaming();
        let mut engine = StreamingEngine::new(n_paths, n_steps, streaming_config, seed);
        let mut observer = ArithmeticAverageObserver::new(n_paths, strike, epsilon, is_call);

        let result = engine.run(gbm, &mut observer);

        PricingResult {
            price: result.mean * discount_factor,
            std_error: result.std_error * discount_factor,
            ..Default::default()
        }
    }

    /// Prices a barrier option using streaming mode.
    pub fn price_barrier_streaming(
        &mut self,
        gbm: GbmParams,
        strike: f64,
        barrier: f64,
        is_up: bool,
        is_out: bool,
        is_call: bool,
        discount_factor: f64,
    ) -> PricingResult {
        let n_paths = self.config.n_paths();
        let n_steps = self.config.n_steps();
        let seed = self.config.seed().unwrap_or(0);
        let epsilon = 1e-6;

        let streaming_config = *self.config.streaming();
        let mut engine = StreamingEngine::new(n_paths, n_steps, streaming_config, seed);
        let mut observer =
            BarrierObserver::new(n_paths, strike, barrier, epsilon, is_up, is_out, is_call);

        let result = engine.run(gbm, &mut observer);

        PricingResult {
            price: result.mean * discount_factor,
            std_error: result.std_error * discount_factor,
            ..Default::default()
        }
    }

    /// Prices a lookback option using streaming mode.
    pub fn price_lookback_streaming(
        &mut self,
        gbm: GbmParams,
        strike: Option<f64>,
        is_call: bool,
        is_floating: bool,
        discount_factor: f64,
    ) -> PricingResult {
        let n_paths = self.config.n_paths();
        let n_steps = self.config.n_steps();
        let seed = self.config.seed().unwrap_or(0);
        let epsilon = 1e-6;

        let streaming_config = *self.config.streaming();
        let mut engine = StreamingEngine::new(n_paths, n_steps, streaming_config, seed);
        let mut observer = LookbackObserver::new(n_paths, strike, epsilon, is_call, is_floating);

        let result = engine.run(gbm, &mut observer);

        PricingResult {
            price: result.mean * discount_factor,
            std_error: result.std_error * discount_factor,
            ..Default::default()
        }
    }

    /// Prices using streaming mode with a custom [`StreamingObserver`].
    pub fn price_streaming_with_observer<O>(
        &mut self,
        gbm: GbmParams,
        observer: &mut O,
        discount_factor: f64,
    ) -> PricingResult
    where
        O: StreamingObserver,
    {
        let n_paths = self.config.n_paths();
        let n_steps = self.config.n_steps();
        let seed = self.config.seed().unwrap_or(0);

        let streaming_config = *self.config.streaming();
        let mut engine = StreamingEngine::new(n_paths, n_steps, streaming_config, seed);

        let result = engine.run(gbm, observer);

        PricingResult {
            price: result.mean * discount_factor,
            std_error: result.std_error * discount_factor,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn create_test_pricer() -> MonteCarloPricer {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();
        MonteCarloPricer::new(config).unwrap()
    }

    #[test]
    fn test_pricer_creation() {
        let pricer = create_test_pricer();
        assert_eq!(pricer.config().n_paths(), 10_000);
        assert_eq!(pricer.config().n_steps(), 50);
    }

    #[test]
    fn test_pricer_with_seed() {
        let config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(10)
            .build()
            .unwrap();
        let pricer = MonteCarloPricer::with_seed(config, 12345).unwrap();
        assert!(pricer.config().seed().is_none()); // Config seed not set
    }

    #[test]
    fn test_price_european_call() {
        let mut pricer = create_test_pricer();
        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let df = (-0.05_f64).exp();

        let result = pricer.price_european(gbm, payoff, df);

        assert!(result.price > 0.0);
        assert!(result.std_error > 0.0);
        assert!(result.std_error < result.price * 0.1); // Reasonable std error
    }

    #[test]
    fn test_price_european_put() {
        let mut pricer = create_test_pricer();
        let gbm = GbmParams::default();
        let payoff = PayoffParams::put(100.0);
        let df = (-0.05_f64).exp();

        let result = pricer.price_european(gbm, payoff, df);

        assert!(result.price > 0.0);
    }

    #[test]
    fn test_pricer_reproducibility() {
        let config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(10)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer1 = MonteCarloPricer::new(config.clone()).unwrap();
        let mut pricer2 = MonteCarloPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let df = 0.95;

        let result1 = pricer1.price_european(gbm, payoff, df);
        let result2 = pricer2.price_european(gbm, payoff, df);

        assert_eq!(result1.price, result2.price);
        assert_eq!(result1.std_error, result2.std_error);
    }

    #[test]
    fn test_pricer_reset() {
        let config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(10)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let df = 0.95;

        let result1 = pricer.price_european(gbm, payoff, df);

        pricer.reset();
        let result2 = pricer.price_european(gbm, payoff, df);

        assert_eq!(result1.price, result2.price);
    }

    #[test]
    fn test_price_with_delta() {
        let mut pricer = create_test_pricer();
        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let df = (-0.05_f64).exp();

        let result = pricer.price_with_greeks(gbm, payoff, df, &[Greek::Delta]);

        assert!(result.delta.is_some());
        let delta = result.delta.unwrap();

        // Delta of ATM call should be around 0.5-0.6
        assert!(delta > 0.3 && delta < 0.8, "Delta = {}", delta);
    }

    #[test]
    fn test_price_with_vega() {
        let mut pricer = create_test_pricer();
        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let df = (-0.05_f64).exp();

        let result = pricer.price_with_greeks(gbm, payoff, df, &[Greek::Vega]);

        assert!(result.vega.is_some());
        let vega = result.vega.unwrap();

        // Vega should be positive for options
        assert!(vega > 0.0, "Vega = {}", vega);
    }

    #[test]
    fn test_price_with_multiple_greeks() {
        let mut pricer = create_test_pricer();
        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let df = (-0.05_f64).exp();

        let result =
            pricer.price_with_greeks(gbm, payoff, df, &[Greek::Delta, Greek::Gamma, Greek::Vega]);

        assert!(result.delta.is_some());
        assert!(result.gamma.is_some());
        assert!(result.vega.is_some());
        assert!(result.theta.is_none());
        assert!(result.rho.is_none());
    }

    #[test]
    fn test_price_with_delta_ad() {
        let mut pricer = create_test_pricer();
        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let df = (-0.05_f64).exp();

        let (price, delta) = pricer.price_with_delta_ad(gbm, payoff, df);

        assert!(price > 0.0);
        assert!(delta > 0.3 && delta < 0.8, "Delta = {}", delta);
    }

    #[test]
    fn test_delta_ad_vs_bump() {
        let config = MonteCarloConfig::builder()
            .n_paths(50_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let df = (-0.05_f64).exp();

        // AD Delta
        pricer.reset_with_seed(42);
        let (_, delta_ad) = pricer.price_with_delta_ad(gbm, payoff, df);

        // Bump-and-revalue Delta
        pricer.reset_with_seed(42);
        let delta_bump = pricer.fd_delta(gbm, PricingMode::European(payoff), df);

        // Should be within 10% of each other
        assert_relative_eq!(delta_ad, delta_bump, max_relative = 0.1);
    }

    #[test]
    fn test_pricing_result_confidence() {
        let result = PricingResult {
            price: 10.0,
            std_error: 0.1,
            ..Default::default()
        };

        assert_relative_eq!(result.confidence_95(), 1.96 * 0.1, epsilon = 1e-10);
        assert_relative_eq!(result.confidence_99(), 2.576 * 0.1, epsilon = 1e-10);
    }

    #[test]
    fn test_call_put_parity_mc() {
        // Put-call parity: C - P = S - K * exp(-rT)
        let config = MonteCarloConfig::builder()
            .n_paths(50_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let gbm = GbmParams {
            spot: 100.0,
            rate: 0.05,
            volatility: 0.2,
            maturity: 1.0,
        };
        let strike = 100.0;
        let df = (-gbm.rate * gbm.maturity).exp();

        // Call price
        let mut pricer = MonteCarloPricer::new(config.clone()).unwrap();
        let call_price = pricer
            .price_european(gbm, PayoffParams::call(strike), df)
            .price;

        // Put price
        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let put_price = pricer
            .price_european(gbm, PayoffParams::put(strike), df)
            .price;

        // Expected: S - K * exp(-rT) = 100 - 100 * exp(-0.05) ≈ 4.88
        let expected_diff = gbm.spot - strike * df;
        let actual_diff = call_price - put_price;

        assert_relative_eq!(actual_diff, expected_diff, max_relative = 0.05);
    }

    // ========================================================================
    // Path-Dependent Options Tests
    // ========================================================================

    #[test]
    fn test_price_asian_arithmetic_call() {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = (-0.05_f64).exp();

        let result = pricer.price_path_dependent(gbm, payoff, df);

        assert!(result.price > 0.0);
        assert!(result.std_error > 0.0);
        // Asian call should be cheaper than European call due to averaging
        assert!(result.std_error < result.price * 0.1);
    }

    #[test]
    fn test_price_asian_geometric_call() {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_geometric_call(100.0, 1e-6);
        let df = (-0.05_f64).exp();

        let result = pricer.price_path_dependent(gbm, payoff, df);

        assert!(result.price > 0.0);
        // Geometric average should be lower than arithmetic average
    }

    #[test]
    fn test_asian_arithmetic_vs_geometric() {
        let config = MonteCarloConfig::builder()
            .n_paths(20_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let gbm = GbmParams::default();
        let df = (-0.05_f64).exp();

        // Arithmetic Asian call
        let mut pricer1 = MonteCarloPricer::new(config.clone()).unwrap();
        let arith_price = pricer1
            .price_path_dependent(gbm, PathPayoffType::asian_arithmetic_call(100.0, 1e-6), df)
            .price;

        // Geometric Asian call
        let mut pricer2 = MonteCarloPricer::new(config).unwrap();
        let geom_price = pricer2
            .price_path_dependent(gbm, PathPayoffType::asian_geometric_call(100.0, 1e-6), df)
            .price;

        // Geometric average is always <= arithmetic average (AM-GM inequality)
        // So geometric Asian call should be <= arithmetic Asian call
        assert!(
            geom_price <= arith_price * 1.05, // Allow 5% tolerance for MC noise
            "Geometric price ({}) should be <= Arithmetic price ({})",
            geom_price,
            arith_price
        );
    }

    #[test]
    fn test_price_barrier_up_out_call() {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        // Up-and-Out barrier significantly above spot
        let payoff = PathPayoffType::barrier_up_out_call(100.0, 150.0, 1e-6);
        let df = (-0.05_f64).exp();

        let result = pricer.price_path_dependent(gbm, payoff, df);

        assert!(result.price > 0.0);
        // Barrier option should be cheaper than vanilla option
    }

    #[test]
    fn test_price_lookback_fixed_call() {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let payoff = PathPayoffType::lookback_fixed_call(100.0, 1e-6);
        let df = (-0.05_f64).exp();

        let result = pricer.price_path_dependent(gbm, payoff, df);

        assert!(result.price > 0.0);
        // Lookback should be more expensive than vanilla due to path maximum
    }

    #[test]
    fn test_price_lookback_floating_call() {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let payoff = PathPayoffType::lookback_floating_call(1e-6);
        let df = (-0.05_f64).exp();

        let result = pricer.price_path_dependent(gbm, payoff, df);

        // Floating lookback call should always have positive payoff
        assert!(result.price > 0.0);
    }

    #[test]
    fn test_path_dependent_reproducibility() {
        let config = MonteCarloConfig::builder()
            .n_paths(5_000)
            .n_steps(20)
            .seed(42)
            .build()
            .unwrap();

        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = 0.95;

        let mut pricer1 = MonteCarloPricer::new(config.clone()).unwrap();
        let result1 = pricer1.price_path_dependent(gbm, payoff, df);

        let mut pricer2 = MonteCarloPricer::new(config).unwrap();
        let result2 = pricer2.price_path_dependent(gbm, payoff, df);

        assert_eq!(result1.price, result2.price);
        assert_eq!(result1.std_error, result2.std_error);
    }

    #[test]
    fn test_path_dependent_with_delta() {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(30)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
        let df = (-0.05_f64).exp();

        let result = pricer.price_path_dependent_with_greeks(gbm, payoff, df, &[Greek::Delta]);

        assert!(result.delta.is_some());
        let delta = result.delta.unwrap();

        // Delta of Asian call should be lower than European call (0.3-0.6)
        assert!(
            delta > 0.2 && delta < 0.7,
            "Asian Delta = {} (expected 0.2-0.7)",
            delta
        );
    }

    #[test]
    fn test_path_dependent_with_vega() {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(30)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let payoff = PathPayoffType::lookback_fixed_call(100.0, 1e-6);
        let df = (-0.05_f64).exp();

        let result = pricer.price_path_dependent_with_greeks(gbm, payoff, df, &[Greek::Vega]);

        assert!(result.vega.is_some());
        let vega = result.vega.unwrap();

        // Vega should be positive for options
        assert!(vega > 0.0, "Vega = {}", vega);
    }

    // ========================================================================
    // Streaming Mode Tests
    // ========================================================================

    #[test]
    fn test_price_streaming_european_call() {
        use super::super::layout_config::{PathLayout, PathLayoutConfig, StreamingConfig};

        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
            .streaming(StreamingConfig::enabled())
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let df = (-0.05_f64).exp();

        let result = pricer.price_streaming(gbm, payoff, df);

        assert!(result.price > 0.0);
        assert!(result.std_error > 0.0);
    }

    #[test]
    fn test_price_asian_streaming() {
        use super::super::layout_config::{PathLayout, PathLayoutConfig, StreamingConfig};

        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
            .streaming(StreamingConfig::enabled())
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let df = (-0.05_f64).exp();

        let result = pricer.price_asian_streaming(gbm, 100.0, true, df);

        assert!(result.price > 0.0);
        assert!(result.std_error > 0.0);
    }

    #[test]
    fn test_price_barrier_streaming() {
        use super::super::layout_config::{PathLayout, PathLayoutConfig, StreamingConfig};

        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
            .streaming(StreamingConfig::enabled())
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let df = (-0.05_f64).exp();

        // Up-and-out call with barrier at 150
        let result = pricer.price_barrier_streaming(gbm, 100.0, 150.0, true, true, true, df);

        assert!(result.price >= 0.0);
        assert!(result.std_error >= 0.0);
    }

    #[test]
    fn test_price_lookback_streaming() {
        use super::super::layout_config::{PathLayout, PathLayoutConfig, StreamingConfig};

        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
            .streaming(StreamingConfig::enabled())
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let df = (-0.05_f64).exp();

        // Floating strike lookback call
        let result = pricer.price_lookback_streaming(gbm, None, true, true, df);

        assert!(result.price > 0.0);
        assert!(result.std_error > 0.0);
    }

    #[test]
    fn test_streaming_vs_batch_similar_results() {
        use super::super::layout_config::{PathLayout, PathLayoutConfig, StreamingConfig};

        // Compare streaming vs batch European call
        let n_paths = 50_000;
        let n_steps = 50;
        let seed = 42;
        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let df = (-0.05_f64).exp();

        // Batch mode
        let batch_config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .seed(seed)
            .build()
            .unwrap();
        let mut batch_pricer = MonteCarloPricer::new(batch_config).unwrap();
        let batch_result = batch_pricer.price_european(gbm, payoff, df);

        // Streaming mode
        let streaming_config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
            .streaming(StreamingConfig::enabled())
            .seed(seed)
            .build()
            .unwrap();
        let mut streaming_pricer = MonteCarloPricer::new(streaming_config).unwrap();
        let streaming_result = streaming_pricer.price_streaming(gbm, payoff, df);

        // Results should be similar (within 10% for this seed)
        let diff_ratio = (streaming_result.price - batch_result.price).abs() / batch_result.price;
        assert!(
            diff_ratio < 0.10,
            "Streaming ({:.4}) vs Batch ({:.4}) diff ratio: {:.4}",
            streaming_result.price,
            batch_result.price,
            diff_ratio
        );
    }
}
