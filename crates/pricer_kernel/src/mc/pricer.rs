//! Monte Carlo pricing engine.
//!
//! This module provides the orchestration layer for Monte Carlo pricing
//! with optional automatic differentiation for Greeks computation.
//!
//! # Overview
//!
//! The [`MonteCarloPricer`] coordinates:
//! 1. Random number generation (via [`PricerRng`](crate::rng::PricerRng))
//! 2. Path generation (via [`generate_gbm_paths`](super::paths::generate_gbm_paths))
//! 3. Payoff computation (via [`compute_payoffs`](super::payoff::compute_payoffs))
//! 4. Discounting and aggregation
//! 5. Greeks via AD (or bump-and-revalue as placeholder)
//!
//! # Workspace Reuse
//!
//! The pricer maintains an internal [`PathWorkspace`](super::workspace::PathWorkspace)
//! that is reused across pricing calls, minimising memory allocations.

use super::config::MonteCarloConfig;
use super::error::ConfigError;
use super::paths::{generate_gbm_paths, generate_gbm_paths_tangent_spot, GbmParams};
use super::payoff::{compute_payoff, compute_payoffs, PayoffParams};
use super::workspace::PathWorkspace;
use crate::rng::PricerRng;

/// Greek type for selection.
///
/// Specifies which sensitivity to compute alongside the price.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Greek {
    /// Delta: dPrice/dSpot
    Delta,
    /// Gamma: d²Price/dSpot²
    Gamma,
    /// Vega: dPrice/dVolatility
    Vega,
    /// Theta: dPrice/dTime
    Theta,
    /// Rho: dPrice/dRate
    Rho,
}

/// Pricing result with optional Greeks.
///
/// Contains the Monte Carlo price estimate and optionally computed sensitivities.
///
/// # Examples
///
/// ```rust
/// use pricer_kernel::mc::PricingResult;
///
/// let result = PricingResult {
///     price: 10.5,
///     std_error: 0.05,
///     delta: Some(0.55),
///     gamma: None,
///     vega: Some(25.0),
///     theta: None,
///     rho: None,
/// };
///
/// println!("Price: {} +/- {}", result.price, result.std_error * 1.96);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PricingResult {
    /// Present value of the instrument.
    pub price: f64,
    /// Standard error of the price estimate.
    pub std_error: f64,
    /// Delta (dPrice/dSpot).
    pub delta: Option<f64>,
    /// Gamma (d²Price/dSpot²).
    pub gamma: Option<f64>,
    /// Vega (dPrice/dVolatility).
    pub vega: Option<f64>,
    /// Theta (dPrice/dTime).
    pub theta: Option<f64>,
    /// Rho (dPrice/dRate).
    pub rho: Option<f64>,
}

impl PricingResult {
    /// Returns the 95% confidence interval half-width.
    #[inline]
    pub fn confidence_95(&self) -> f64 {
        1.96 * self.std_error
    }

    /// Returns the 99% confidence interval half-width.
    #[inline]
    pub fn confidence_99(&self) -> f64 {
        2.576 * self.std_error
    }
}

/// Monte Carlo pricing engine.
///
/// Orchestrates path generation, payoff computation, and Greeks calculation
/// using optional automatic differentiation.
///
/// # Workspace Reuse
///
/// The pricer maintains an internal workspace that is reused across pricing calls.
/// This minimises memory allocations for repeated pricing operations.
///
/// # Examples
///
/// ```rust
/// use pricer_kernel::mc::{
///     MonteCarloPricer, MonteCarloConfig, GbmParams, PayoffParams, Greek,
/// };
///
/// let config = MonteCarloConfig::builder()
///     .n_paths(10_000)
///     .n_steps(252)
///     .seed(42)
///     .build()
///     .unwrap();
///
/// let mut pricer = MonteCarloPricer::new(config).unwrap();
///
/// let gbm = GbmParams::default();
/// let payoff = PayoffParams::call(100.0);
/// let discount_factor = 0.95;
///
/// let result = pricer.price_european(gbm, payoff, discount_factor);
/// println!("Price: {} +/- {}", result.price, result.std_error);
/// ```
pub struct MonteCarloPricer {
    config: MonteCarloConfig,
    workspace: PathWorkspace,
    rng: PricerRng,
}

impl MonteCarloPricer {
    /// Creates a new pricer with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Monte Carlo configuration
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if configuration is invalid.
    pub fn new(config: MonteCarloConfig) -> Result<Self, ConfigError> {
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

    /// Creates a new pricer with a specific seed.
    ///
    /// Convenience constructor that overrides the config seed.
    ///
    /// # Arguments
    ///
    /// * `config` - Monte Carlo configuration
    /// * `seed` - Seed for reproducibility
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if configuration is invalid.
    pub fn with_seed(config: MonteCarloConfig, seed: u64) -> Result<Self, ConfigError> {
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
    pub fn config(&self) -> &MonteCarloConfig {
        &self.config
    }

    /// Resets the pricer state for a new simulation.
    ///
    /// Resets the workspace and RNG (using original seed).
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
    ///
    /// # Arguments
    ///
    /// * `gbm` - GBM parameters (spot, rate, volatility, maturity)
    /// * `payoff` - Payoff parameters (strike, type, smoothing)
    /// * `discount_factor` - Present value discount factor (e.g., exp(-r*T))
    ///
    /// # Returns
    ///
    /// Price and standard error.
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

    /// Prices a European option with selected Greeks.
    ///
    /// # Arguments
    ///
    /// * `gbm` - GBM parameters
    /// * `payoff` - Payoff parameters
    /// * `discount_factor` - Discount factor
    /// * `greeks` - Which Greeks to compute
    ///
    /// # Returns
    ///
    /// Price and requested Greeks.
    ///
    /// # Implementation Note
    ///
    /// Phase 3.2 uses bump-and-revalue for Greeks. Phase 4 will integrate
    /// Enzyme AD for true automatic differentiation.
    pub fn price_with_greeks(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
        greeks: &[Greek],
    ) -> PricingResult {
        // Base price
        let mut result = self.price_european(gbm, payoff, discount_factor);

        // Compute requested Greeks
        for greek in greeks {
            match greek {
                Greek::Delta => {
                    result.delta = Some(self.compute_delta(gbm, payoff, discount_factor));
                }
                Greek::Gamma => {
                    result.gamma = Some(self.compute_gamma(gbm, payoff, discount_factor));
                }
                Greek::Vega => {
                    result.vega = Some(self.compute_vega(gbm, payoff, discount_factor));
                }
                Greek::Theta => {
                    result.theta = Some(self.compute_theta(gbm, payoff, discount_factor));
                }
                Greek::Rho => {
                    result.rho = Some(self.compute_rho(gbm, payoff, discount_factor));
                }
            }
        }

        result
    }

    /// Computes Delta using central differences (Phase 3.2 placeholder).
    ///
    /// In Phase 4, this will use Enzyme forward-mode AD.
    fn compute_delta(&mut self, gbm: GbmParams, payoff: PayoffParams, discount_factor: f64) -> f64 {
        // Bump size: 1% of spot or minimum 0.01
        let bump = (0.01 * gbm.spot).max(0.01);

        // Save RNG state for reproducibility
        let seed = self.rng.seed();

        // Price at S + bump
        self.reset_with_seed(seed);
        let gbm_up = GbmParams {
            spot: gbm.spot + bump,
            ..gbm
        };
        let price_up = self.price_european(gbm_up, payoff, discount_factor).price;

        // Price at S - bump
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            spot: gbm.spot - bump,
            ..gbm
        };
        let price_down = self.price_european(gbm_down, payoff, discount_factor).price;

        // Central difference
        (price_up - price_down) / (2.0 * bump)
    }

    /// Computes Gamma using central differences.
    fn compute_gamma(&mut self, gbm: GbmParams, payoff: PayoffParams, discount_factor: f64) -> f64 {
        let bump = (0.01 * gbm.spot).max(0.01);
        let seed = self.rng.seed();

        // Price at S
        self.reset_with_seed(seed);
        let price_mid = self.price_european(gbm, payoff, discount_factor).price;

        // Price at S + bump
        self.reset_with_seed(seed);
        let gbm_up = GbmParams {
            spot: gbm.spot + bump,
            ..gbm
        };
        let price_up = self.price_european(gbm_up, payoff, discount_factor).price;

        // Price at S - bump
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            spot: gbm.spot - bump,
            ..gbm
        };
        let price_down = self.price_european(gbm_down, payoff, discount_factor).price;

        // Second derivative via central differences
        (price_up - 2.0 * price_mid + price_down) / (bump * bump)
    }

    /// Computes Vega using central differences.
    ///
    /// In Phase 4, this will use Enzyme reverse-mode AD.
    fn compute_vega(&mut self, gbm: GbmParams, payoff: PayoffParams, discount_factor: f64) -> f64 {
        // Bump size: 1% of vol (absolute)
        let bump = 0.01;
        let seed = self.rng.seed();

        // Price at vol + bump
        self.reset_with_seed(seed);
        let gbm_up = GbmParams {
            volatility: gbm.volatility + bump,
            ..gbm
        };
        let price_up = self.price_european(gbm_up, payoff, discount_factor).price;

        // Price at vol - bump
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            volatility: (gbm.volatility - bump).max(0.001),
            ..gbm
        };
        let price_down = self.price_european(gbm_down, payoff, discount_factor).price;

        // Central difference (scaled to 1% vol move)
        (price_up - price_down) / (2.0 * bump)
    }

    /// Computes Theta using central differences.
    fn compute_theta(&mut self, gbm: GbmParams, payoff: PayoffParams, discount_factor: f64) -> f64 {
        // Bump size: 1 day = 1/252 years
        let bump = 1.0 / 252.0;
        let seed = self.rng.seed();

        // Price at T - bump (shorter maturity)
        self.reset_with_seed(seed);
        let gbm_short = GbmParams {
            maturity: (gbm.maturity - bump).max(0.001),
            ..gbm
        };
        let price_short = self
            .price_european(gbm_short, payoff, discount_factor)
            .price;

        // Price at T (original)
        self.reset_with_seed(seed);
        let price_orig = self.price_european(gbm, payoff, discount_factor).price;

        // Theta is typically negative (time decay)
        // dP/dT, scaled to daily
        -(price_orig - price_short) / bump
    }

    /// Computes Rho using central differences.
    fn compute_rho(&mut self, gbm: GbmParams, payoff: PayoffParams, _discount_factor: f64) -> f64 {
        // Bump size: 1% rate change (absolute)
        let bump = 0.01;
        let seed = self.rng.seed();

        // Price at r + bump
        self.reset_with_seed(seed);
        let gbm_up = GbmParams {
            rate: gbm.rate + bump,
            ..gbm
        };
        let df_up = (-(gbm.rate + bump) * gbm.maturity).exp();
        let price_up = self.price_european(gbm_up, payoff, df_up).price;

        // Price at r - bump
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            rate: gbm.rate - bump,
            ..gbm
        };
        let df_down = (-(gbm.rate - bump) * gbm.maturity).exp();
        let price_down = self.price_european(gbm_down, payoff, df_down).price;

        // Central difference (scaled to 1% rate move)
        (price_up - price_down) / (2.0 * bump)
    }

    /// Prices using forward-mode AD for spot sensitivity.
    ///
    /// This method uses tangent propagation to compute Delta efficiently
    /// in a single forward pass.
    ///
    /// # Arguments
    ///
    /// * `gbm` - GBM parameters
    /// * `payoff` - Payoff parameters
    /// * `discount_factor` - Discount factor
    ///
    /// # Returns
    ///
    /// (price, delta) tuple.
    ///
    /// # Note
    ///
    /// This is the Phase 3.2 implementation using manual tangent propagation.
    /// Phase 4 will use Enzyme's `#[autodiff_forward]` macro.
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

            // Tangent payoff: d(payoff)/d(spot) = d(payoff)/d(terminal) × d(terminal)/d(spot)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

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
        let delta_bump = pricer.compute_delta(gbm, payoff, df);

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
}
