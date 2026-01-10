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
use crate::path_dependent::{PathObserver, PathPayoffType};
use crate::rng::PricerRng;

/// Greek type for selection.
///
/// Specifies which sensitivity to compute alongside the price.
///
/// # First-Order Greeks
///
/// - `Delta`: ∂V/∂S - Sensitivity to spot price
/// - `Vega`: ∂V/∂σ - Sensitivity to volatility
/// - `Theta`: ∂V/∂τ - Sensitivity to time (time decay)
/// - `Rho`: ∂V/∂r - Sensitivity to interest rate
///
/// # Second-Order Greeks
///
/// - `Gamma`: ∂²V/∂S² - Convexity with respect to spot
/// - `Vanna`: ∂²V/∂S∂σ - Cross sensitivity (delta-vol)
/// - `Volga`: ∂²V/∂σ² - Volatility convexity (also known as vomma)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Greek {
    // First-order Greeks
    /// Delta: ∂V/∂S (sensitivity to spot price)
    Delta,
    /// Vega: ∂V/∂σ (sensitivity to volatility)
    Vega,
    /// Theta: ∂V/∂τ (sensitivity to time, time decay)
    Theta,
    /// Rho: ∂V/∂r (sensitivity to interest rate)
    Rho,

    // Second-order Greeks
    /// Gamma: ∂²V/∂S² (convexity with respect to spot)
    Gamma,
    /// Vanna: ∂²V/∂S∂σ (cross sensitivity between spot and volatility)
    Vanna,
    /// Volga: ∂²V/∂σ² (volatility convexity, also known as vomma)
    Volga,
}

/// Pricing result with optional Greeks.
///
/// Contains the Monte Carlo price estimate and optionally computed sensitivities.
///
/// # First-Order Greeks
///
/// - `delta`: ∂V/∂S - Sensitivity to spot price
/// - `vega`: ∂V/∂σ - Sensitivity to volatility
/// - `theta`: ∂V/∂τ - Sensitivity to time (time decay)
/// - `rho`: ∂V/∂r - Sensitivity to interest rate
///
/// # Second-Order Greeks
///
/// - `gamma`: ∂²V/∂S² - Convexity with respect to spot
/// - `vanna`: ∂²V/∂S∂σ - Cross sensitivity (delta-vol)
/// - `volga`: ∂²V/∂σ² - Volatility convexity
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::mc::PricingResult;
///
/// let result = PricingResult {
///     price: 10.5,
///     std_error: 0.05,
///     delta: Some(0.55),
///     gamma: None,
///     vega: Some(25.0),
///     theta: None,
///     rho: None,
///     vanna: None,
///     volga: None,
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

    // First-order Greeks
    /// Delta: ∂V/∂S (sensitivity to spot price).
    pub delta: Option<f64>,
    /// Vega: ∂V/∂σ (sensitivity to volatility).
    pub vega: Option<f64>,
    /// Theta: ∂V/∂τ (sensitivity to time, time decay).
    pub theta: Option<f64>,
    /// Rho: ∂V/∂r (sensitivity to interest rate).
    pub rho: Option<f64>,

    // Second-order Greeks
    /// Gamma: ∂²V/∂S² (convexity with respect to spot).
    pub gamma: Option<f64>,
    /// Vanna: ∂²V/∂S∂σ (cross sensitivity between spot and volatility).
    pub vanna: Option<f64>,
    /// Volga: ∂²V/∂σ² (volatility convexity, also known as vomma).
    pub volga: Option<f64>,
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
/// use pricer_pricing::mc::{
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
    /// Random number generator (pub(crate) for Enzyme AD access).
    pub(crate) rng: PricerRng,
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
                // First-order Greeks
                Greek::Delta => {
                    result.delta = Some(self.compute_delta(gbm, payoff, discount_factor));
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
                // Second-order Greeks
                Greek::Gamma => {
                    result.gamma = Some(self.compute_gamma(gbm, payoff, discount_factor));
                }
                Greek::Vanna => {
                    result.vanna = Some(self.compute_vanna(gbm, payoff, discount_factor));
                }
                Greek::Volga => {
                    result.volga = Some(self.compute_volga(gbm, payoff, discount_factor));
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

    /// Computes Vanna (∂²V/∂S∂σ) using cross-differences.
    ///
    /// Vanna is the cross partial derivative of option value with respect to
    /// spot price and volatility. It measures how delta changes with volatility.
    ///
    /// Uses the formula: (V(S+h,σ+k) - V(S+h,σ-k) - V(S-h,σ+k) + V(S-h,σ-k)) / (4hk)
    fn compute_vanna(&mut self, gbm: GbmParams, payoff: PayoffParams, discount_factor: f64) -> f64 {
        let spot_bump = (0.01 * gbm.spot).max(0.01);
        let vol_bump = 0.01;
        let seed = self.rng.seed();

        // V(S+h, σ+k)
        self.reset_with_seed(seed);
        let gbm_up_up = GbmParams {
            spot: gbm.spot + spot_bump,
            volatility: gbm.volatility + vol_bump,
            ..gbm
        };
        let price_up_up = self
            .price_european(gbm_up_up, payoff, discount_factor)
            .price;

        // V(S+h, σ-k)
        self.reset_with_seed(seed);
        let gbm_up_down = GbmParams {
            spot: gbm.spot + spot_bump,
            volatility: (gbm.volatility - vol_bump).max(0.001),
            ..gbm
        };
        let price_up_down = self
            .price_european(gbm_up_down, payoff, discount_factor)
            .price;

        // V(S-h, σ+k)
        self.reset_with_seed(seed);
        let gbm_down_up = GbmParams {
            spot: gbm.spot - spot_bump,
            volatility: gbm.volatility + vol_bump,
            ..gbm
        };
        let price_down_up = self
            .price_european(gbm_down_up, payoff, discount_factor)
            .price;

        // V(S-h, σ-k)
        self.reset_with_seed(seed);
        let gbm_down_down = GbmParams {
            spot: gbm.spot - spot_bump,
            volatility: (gbm.volatility - vol_bump).max(0.001),
            ..gbm
        };
        let price_down_down = self
            .price_european(gbm_down_down, payoff, discount_factor)
            .price;

        // Cross-difference formula
        (price_up_up - price_up_down - price_down_up + price_down_down)
            / (4.0 * spot_bump * vol_bump)
    }

    /// Computes Volga (∂²V/∂σ²) using three-point formula.
    ///
    /// Volga (also known as vomma) is the second derivative of option value
    /// with respect to volatility. It measures the convexity of vega.
    ///
    /// Uses the formula: (V(σ+h) - 2V(σ) + V(σ-h)) / h²
    fn compute_volga(&mut self, gbm: GbmParams, payoff: PayoffParams, discount_factor: f64) -> f64 {
        let bump = 0.01;
        let seed = self.rng.seed();

        // V(σ)
        self.reset_with_seed(seed);
        let price_mid = self.price_european(gbm, payoff, discount_factor).price;

        // V(σ+h)
        self.reset_with_seed(seed);
        let gbm_up = GbmParams {
            volatility: gbm.volatility + bump,
            ..gbm
        };
        let price_up = self.price_european(gbm_up, payoff, discount_factor).price;

        // V(σ-h)
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            volatility: (gbm.volatility - bump).max(0.001),
            ..gbm
        };
        let price_down = self.price_european(gbm_down, payoff, discount_factor).price;

        // Three-point formula for second derivative
        (price_up - 2.0 * price_mid + price_down) / (bump * bump)
    }

    // ========================================================================
    // Phase 4: L1/L2 Integration - YieldCurve methods
    // ========================================================================

    /// Prices a European option using Monte Carlo simulation with a YieldCurve.
    ///
    /// This method uses the YieldCurve trait from pricer_core to compute
    /// the discount factor, providing a cleaner integration with the L1 layer.
    ///
    /// # Arguments
    ///
    /// * `gbm` - GBM parameters (spot, rate, volatility, maturity)
    /// * `payoff` - Payoff parameters (strike, type, smoothing)
    /// * `curve` - Yield curve implementing the YieldCurve trait
    ///
    /// # Returns
    ///
    /// Price and standard error.
    ///
    /// # Panics
    ///
    /// Panics if the curve returns an error for the given maturity.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pricer_pricing::mc::{MonteCarloPricer, MonteCarloConfig, GbmParams, PayoffParams};
    /// use pricer_core::market_data::curves::{FlatCurve, YieldCurve};
    ///
    /// let config = MonteCarloConfig::builder()
    ///     .n_paths(10_000)
    ///     .n_steps(50)
    ///     .seed(42)
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut pricer = MonteCarloPricer::new(config).unwrap();
    /// let gbm = GbmParams::default();
    /// let payoff = PayoffParams::call(100.0);
    /// let curve = FlatCurve::new(0.05);
    ///
    /// let result = pricer.price_european_with_curve(gbm, payoff, &curve);
    /// ```
    #[cfg(feature = "l1l2-integration")]
    pub fn price_european_with_curve<C>(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        curve: &C,
    ) -> PricingResult
    where
        C: pricer_core::market_data::curves::YieldCurve<f64>,
    {
        let discount_factor = curve
            .discount_factor(gbm.maturity)
            .expect("YieldCurve::discount_factor failed for given maturity");
        self.price_european(gbm, payoff, discount_factor)
    }

    /// Prices a European option with selected Greeks using a YieldCurve.
    ///
    /// This method uses the YieldCurve trait from pricer_core to compute
    /// the discount factor for both the base price and Greek calculations.
    ///
    /// # Arguments
    ///
    /// * `gbm` - GBM parameters
    /// * `payoff` - Payoff parameters
    /// * `curve` - Yield curve implementing the YieldCurve trait
    /// * `greeks` - Which Greeks to compute
    ///
    /// # Returns
    ///
    /// Price and requested Greeks.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pricer_pricing::mc::{MonteCarloPricer, MonteCarloConfig, GbmParams, PayoffParams, Greek};
    /// use pricer_core::market_data::curves::{FlatCurve, YieldCurve};
    ///
    /// let config = MonteCarloConfig::builder()
    ///     .n_paths(10_000)
    ///     .n_steps(50)
    ///     .seed(42)
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut pricer = MonteCarloPricer::new(config).unwrap();
    /// let gbm = GbmParams::default();
    /// let payoff = PayoffParams::call(100.0);
    /// let curve = FlatCurve::new(0.05);
    ///
    /// let result = pricer.price_with_greeks_and_curve(gbm, payoff, &curve, &[Greek::Delta]);
    /// ```
    #[cfg(feature = "l1l2-integration")]
    pub fn price_with_greeks_and_curve<C>(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        curve: &C,
        greeks: &[Greek],
    ) -> PricingResult
    where
        C: pricer_core::market_data::curves::YieldCurve<f64>,
    {
        let discount_factor = curve
            .discount_factor(gbm.maturity)
            .expect("YieldCurve::discount_factor failed for given maturity");
        self.price_with_greeks(gbm, payoff, discount_factor, greeks)
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

    // ========================================================================
    // Phase 4: Path-Dependent Options Integration
    // ========================================================================

    /// Prices a path-dependent option using Monte Carlo simulation.
    ///
    /// This method generates GBM paths and uses PathObserver to track
    /// path statistics (average, max, min, terminal) at each step.
    /// The payoff is computed using PathPayoffType for static dispatch.
    ///
    /// # Arguments
    ///
    /// * `gbm` - GBM parameters (spot, rate, volatility, maturity)
    /// * `payoff` - Path-dependent payoff type (Asian, Barrier, Lookback)
    /// * `discount_factor` - Present value discount factor
    ///
    /// # Returns
    ///
    /// Price and standard error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_pricing::mc::{MonteCarloPricer, MonteCarloConfig, GbmParams};
    /// use pricer_pricing::path_dependent::PathPayoffType;
    ///
    /// let config = MonteCarloConfig::builder()
    ///     .n_paths(10_000)
    ///     .n_steps(50)
    ///     .seed(42)
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut pricer = MonteCarloPricer::new(config).unwrap();
    /// let gbm = GbmParams::default();
    /// let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
    /// let df = (-0.05_f64).exp();
    ///
    /// let result = pricer.price_path_dependent(gbm, payoff, df);
    /// println!("Asian Call Price: {:.4}", result.price);
    /// ```
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

    /// Prices a path-dependent option with selected Greeks.
    ///
    /// # Arguments
    ///
    /// * `gbm` - GBM parameters
    /// * `payoff` - Path-dependent payoff type
    /// * `discount_factor` - Discount factor
    /// * `greeks` - Which Greeks to compute
    ///
    /// # Returns
    ///
    /// Price and requested Greeks.
    ///
    /// # Implementation Note
    ///
    /// Phase 4 uses bump-and-revalue for Greeks. Future phases will integrate
    /// Enzyme AD with checkpointing for path-dependent derivatives.
    pub fn price_path_dependent_with_greeks(
        &mut self,
        gbm: GbmParams,
        payoff: PathPayoffType<f64>,
        discount_factor: f64,
        greeks: &[Greek],
    ) -> PricingResult {
        // Base price
        let mut result = self.price_path_dependent(gbm, payoff, discount_factor);

        // Compute requested Greeks via bump-and-revalue
        for greek in greeks {
            match greek {
                Greek::Delta => {
                    result.delta =
                        Some(self.compute_delta_path_dependent(gbm, payoff, discount_factor));
                }
                Greek::Gamma => {
                    result.gamma =
                        Some(self.compute_gamma_path_dependent(gbm, payoff, discount_factor));
                }
                Greek::Vega => {
                    result.vega =
                        Some(self.compute_vega_path_dependent(gbm, payoff, discount_factor));
                }
                Greek::Theta => {
                    result.theta =
                        Some(self.compute_theta_path_dependent(gbm, payoff, discount_factor));
                }
                Greek::Rho => {
                    result.rho =
                        Some(self.compute_rho_path_dependent(gbm, payoff, discount_factor));
                }
                Greek::Vanna | Greek::Volga => {
                    // Second-order cross Greeks for path-dependent not yet implemented
                    // Will be added with Enzyme AD + checkpointing integration
                }
            }
        }

        result
    }

    /// Computes Delta for path-dependent options using central differences.
    fn compute_delta_path_dependent(
        &mut self,
        gbm: GbmParams,
        payoff: PathPayoffType<f64>,
        discount_factor: f64,
    ) -> f64 {
        let bump = (0.01 * gbm.spot).max(0.01);
        let seed = self.rng.seed();

        // Price at S + bump
        self.reset_with_seed(seed);
        let gbm_up = GbmParams {
            spot: gbm.spot + bump,
            ..gbm
        };
        let price_up = self
            .price_path_dependent(gbm_up, payoff, discount_factor)
            .price;

        // Price at S - bump
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            spot: gbm.spot - bump,
            ..gbm
        };
        let price_down = self
            .price_path_dependent(gbm_down, payoff, discount_factor)
            .price;

        (price_up - price_down) / (2.0 * bump)
    }

    /// Computes Gamma for path-dependent options using central differences.
    fn compute_gamma_path_dependent(
        &mut self,
        gbm: GbmParams,
        payoff: PathPayoffType<f64>,
        discount_factor: f64,
    ) -> f64 {
        let bump = (0.01 * gbm.spot).max(0.01);
        let seed = self.rng.seed();

        // Price at S
        self.reset_with_seed(seed);
        let price_mid = self
            .price_path_dependent(gbm, payoff, discount_factor)
            .price;

        // Price at S + bump
        self.reset_with_seed(seed);
        let gbm_up = GbmParams {
            spot: gbm.spot + bump,
            ..gbm
        };
        let price_up = self
            .price_path_dependent(gbm_up, payoff, discount_factor)
            .price;

        // Price at S - bump
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            spot: gbm.spot - bump,
            ..gbm
        };
        let price_down = self
            .price_path_dependent(gbm_down, payoff, discount_factor)
            .price;

        (price_up - 2.0 * price_mid + price_down) / (bump * bump)
    }

    /// Computes Vega for path-dependent options using central differences.
    fn compute_vega_path_dependent(
        &mut self,
        gbm: GbmParams,
        payoff: PathPayoffType<f64>,
        discount_factor: f64,
    ) -> f64 {
        let bump = 0.01;
        let seed = self.rng.seed();

        // Price at vol + bump
        self.reset_with_seed(seed);
        let gbm_up = GbmParams {
            volatility: gbm.volatility + bump,
            ..gbm
        };
        let price_up = self
            .price_path_dependent(gbm_up, payoff, discount_factor)
            .price;

        // Price at vol - bump
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            volatility: (gbm.volatility - bump).max(0.001),
            ..gbm
        };
        let price_down = self
            .price_path_dependent(gbm_down, payoff, discount_factor)
            .price;

        (price_up - price_down) / (2.0 * bump)
    }

    /// Computes Theta for path-dependent options using forward difference.
    fn compute_theta_path_dependent(
        &mut self,
        gbm: GbmParams,
        payoff: PathPayoffType<f64>,
        discount_factor: f64,
    ) -> f64 {
        let bump = 1.0 / 252.0; // 1 day
        let seed = self.rng.seed();

        // Price at T - bump
        self.reset_with_seed(seed);
        let gbm_short = GbmParams {
            maturity: (gbm.maturity - bump).max(0.001),
            ..gbm
        };
        let price_short = self
            .price_path_dependent(gbm_short, payoff, discount_factor)
            .price;

        // Price at T
        self.reset_with_seed(seed);
        let price_orig = self
            .price_path_dependent(gbm, payoff, discount_factor)
            .price;

        -(price_orig - price_short) / bump
    }

    /// Computes Rho for path-dependent options using central differences.
    fn compute_rho_path_dependent(
        &mut self,
        gbm: GbmParams,
        payoff: PathPayoffType<f64>,
        _discount_factor: f64,
    ) -> f64 {
        let bump = 0.01;
        let seed = self.rng.seed();

        // Price at r + bump
        self.reset_with_seed(seed);
        let gbm_up = GbmParams {
            rate: gbm.rate + bump,
            ..gbm
        };
        let df_up = (-(gbm.rate + bump) * gbm.maturity).exp();
        let price_up = self.price_path_dependent(gbm_up, payoff, df_up).price;

        // Price at r - bump
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            rate: gbm.rate - bump,
            ..gbm
        };
        let df_down = (-(gbm.rate - bump) * gbm.maturity).exp();
        let price_down = self.price_path_dependent(gbm_down, payoff, df_down).price;

        (price_up - price_down) / (2.0 * bump)
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
}
