//! Pricing method dispatcher.
//!
//! This module provides a unified dispatcher that routes pricing requests
//! to the appropriate pricing method (Discount, Monte Carlo, Tree).
//!
//! # Architecture
//!
//! The dispatcher acts as a facade that:
//! 1. Accepts pricing requests with a [`PricingMethod`]
//! 2. Routes to the appropriate pricer
//! 3. Returns a unified [`UnifiedPricingResult`]
//!
//! # Example
//!
//! ```rust
//! use pricer_pricing::dispatcher::PricingMethodDispatcher;
//! use infra_config::PricingMethod;
//!
//! let dispatcher = PricingMethodDispatcher::new();
//!
//! // Price a European call using Tree method
//! let result = dispatcher.price_vanilla(
//!     PricingMethod::Tree,
//!     100.0,  // spot
//!     100.0,  // strike
//!     1.0,    // expiry
//!     0.05,   // rate
//!     0.2,    // volatility
//!     true,   // is_call
//!     false,  // is_american
//!     None,   // tree steps (use default)
//!     None,   // mc paths (use default)
//! ).unwrap();
//!
//! println!("PV: {}", result.pv);
//! ```
//!
//! # Instrument Integration (requires `l1l2-integration` feature)
//!
//! When the `l1l2-integration` feature is enabled, the dispatcher can accept
//! `VanillaOption` and `Forward` instruments directly:
//!
//! ```rust,ignore
//! use pricer_pricing::dispatcher::PricingMethodDispatcher;
//! use infra_config::PricingMethod;
//! use infra_master::trade::{VanillaOption, ExerciseStyle};
//!
//! let dispatcher = PricingMethodDispatcher::new();
//! let option = VanillaOption::new(100.0, 1.0, 1.0, true, ExerciseStyle::European);
//!
//! let result = dispatcher.price_instrument(
//!     PricingMethod::Tree,
//!     &option,
//!     100.0,  // spot
//!     0.05,   // rate
//!     0.2,    // volatility
//!     None,   // tree steps
//!     None,   // mc paths
//! ).unwrap();
//! ```

use std::time::Instant;

use infra_config::PricingMethod;
// Instrument types (conditional on l1l2-integration feature)
#[cfg(feature = "l1l2-integration")]
use infra_master::trade::{ExerciseStyle, Forward, OptionType, VanillaOption};

use crate::{
    generic_pricer::PricingError,
    result::{PricingMetadata, TreeTypeMetadata, UnifiedGreeks, UnifiedPricingResult},
    tree::BinomialTree,
};

/// Configuration for the pricing method dispatcher.
#[derive(Debug, Clone)]
pub struct DispatcherConfig {
    /// Default number of tree steps.
    pub default_tree_steps: usize,
    /// Default number of Monte Carlo paths.
    pub default_mc_paths: usize,
    /// Default number of Monte Carlo time steps.
    pub default_mc_steps: usize,
    /// Whether to compute Greeks by default.
    pub compute_greeks: bool,
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            default_tree_steps: 100,
            default_mc_paths: 10_000,
            default_mc_steps: 252,
            compute_greeks: true,
        }
    }
}

/// Pricing method dispatcher.
///
/// Routes pricing requests to the appropriate pricing engine based on the
/// specified [`PricingMethod`].
#[derive(Debug, Clone)]
pub struct PricingMethodDispatcher {
    config: DispatcherConfig,
}

impl Default for PricingMethodDispatcher {
    fn default() -> Self { Self::new() }
}

impl PricingMethodDispatcher {
    /// Creates a new dispatcher with default configuration.
    pub fn new() -> Self {
        Self {
            config: DispatcherConfig::default(),
        }
    }

    /// Creates a new dispatcher with the specified configuration.
    pub fn with_config(config: DispatcherConfig) -> Self { Self { config } }

    /// Returns the dispatcher configuration.
    pub fn config(&self) -> &DispatcherConfig { &self.config }

    /// Dispatches a vanilla option pricing request.
    ///
    /// # Arguments
    ///
    /// * `method` - Pricing method to use
    /// * `spot` - Current spot price
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    /// * `rate` - Risk-free rate (annualized)
    /// * `volatility` - Volatility (annualized)
    /// * `is_call` - True for call, false for put
    /// * `is_american` - True for American, false for European
    /// * `tree_steps` - Optional number of tree steps (for Tree method)
    /// * `mc_paths` - Optional number of MC paths (for MonteCarlo method)
    ///
    /// # Errors
    ///
    /// Returns `PricingError` if:
    /// - Input parameters are invalid
    /// - Pricing method fails
    /// - Method not yet implemented (Analytical)
    pub fn price_vanilla(
        &self,
        method: PricingMethod,
        spot: f64,
        strike: f64,
        expiry: f64,
        rate: f64,
        volatility: f64,
        is_call: bool,
        is_american: bool,
        tree_steps: Option<usize>,
        mc_paths: Option<usize>,
    ) -> Result<UnifiedPricingResult, PricingError> {
        match method {
            PricingMethod::Tree => self.price_with_tree(
                spot,
                strike,
                expiry,
                rate,
                volatility,
                is_call,
                is_american,
                tree_steps,
            ),
            PricingMethod::MonteCarlo => self.price_with_monte_carlo(
                spot,
                strike,
                expiry,
                rate,
                volatility,
                is_call,
                is_american,
                mc_paths,
            ),
            PricingMethod::Analytical => {
                self.price_with_analytical(spot, strike, expiry, rate, volatility, is_call)
            }
        }
    }

    /// Prices using the Tree method.
    fn price_with_tree(
        &self,
        spot: f64,
        strike: f64,
        expiry: f64,
        rate: f64,
        volatility: f64,
        is_call: bool,
        is_american: bool,
        tree_steps: Option<usize>,
    ) -> Result<UnifiedPricingResult, PricingError> {
        let start = Instant::now();
        let num_steps = tree_steps.unwrap_or(self.config.default_tree_steps);

        let tree = BinomialTree::new(
            spot,
            strike,
            expiry,
            rate,
            volatility,
            num_steps,
            is_call,
            is_american,
        )
        .map_err(|e| PricingError::InvalidInput {
            reason: e.to_string(),
        })?;

        let pv = tree.price();

        let greeks = if self.config.compute_greeks {
            Some(UnifiedGreeks::from_delta_gamma(tree.delta(), tree.gamma()))
        } else {
            None
        };

        let elapsed = start.elapsed();

        let mut result =
            UnifiedPricingResult::new(pv, PricingMethod::Tree, elapsed.as_nanos() as u64);

        if let Some(g) = greeks {
            result = result.with_greeks(g);
        }

        result = result.with_metadata(PricingMetadata::Tree {
            num_steps,
            tree_type: TreeTypeMetadata::Binomial,
        });

        Ok(result)
    }

    /// Prices using the Monte Carlo method.
    fn price_with_monte_carlo(
        &self,
        spot: f64,
        strike: f64,
        expiry: f64,
        rate: f64,
        volatility: f64,
        is_call: bool,
        _is_american: bool,
        mc_paths: Option<usize>,
    ) -> Result<UnifiedPricingResult, PricingError> {
        use crate::mc::{
            GbmParams, Greek, MonteCarloConfig, MonteCarloPricer, PayoffParams, PayoffType,
        };

        let start = Instant::now();
        let num_paths = mc_paths.unwrap_or(self.config.default_mc_paths);

        let gbm_params = GbmParams {
            spot,
            rate,
            volatility,
            maturity: expiry,
        };

        let payoff_type = if is_call {
            PayoffType::Call
        } else {
            PayoffType::Put
        };
        let payoff_params = PayoffParams {
            strike,
            payoff_type,
            smoothing_epsilon: 1e-4,
        };

        let config = MonteCarloConfig::builder()
            .n_paths(num_paths)
            .n_steps(self.config.default_mc_steps)
            .seed(42) // Fixed seed for reproducibility
            .build()
            .map_err(|e| PricingError::InvalidInput {
                reason: format!("Invalid MC config: {}", e),
            })?;

        let mut pricer = MonteCarloPricer::new(config).map_err(|e| PricingError::InvalidInput {
            reason: format!("Failed to create MC pricer: {}", e),
        })?;

        // Compute discount factor
        let discount_factor = (-rate * expiry).exp();

        // Price with Greeks if requested
        let mc_result = if self.config.compute_greeks {
            pricer.price_with_greeks(
                gbm_params,
                payoff_params,
                discount_factor,
                &[
                    Greek::Delta,
                    Greek::Gamma,
                    Greek::Vega,
                    Greek::Theta,
                    Greek::Rho,
                ],
            )
        } else {
            pricer.price_european(gbm_params, payoff_params, discount_factor)
        };

        let elapsed = start.elapsed();

        let greeks = if self.config.compute_greeks {
            Some(UnifiedGreeks::new(
                mc_result.delta,
                mc_result.gamma,
                mc_result.vega,
                mc_result.theta,
                mc_result.rho,
            ))
        } else {
            None
        };

        let mut result = UnifiedPricingResult::new(
            mc_result.price,
            PricingMethod::MonteCarlo,
            elapsed.as_nanos() as u64,
        );

        if let Some(g) = greeks {
            result = result.with_greeks(g);
        }

        result = result.with_metadata(PricingMetadata::MonteCarlo {
            num_paths,
            standard_error: mc_result.std_error,
        });

        Ok(result)
    }

    /// Prices using the Analytical method (Black-Scholes).
    fn price_with_analytical(
        &self,
        spot: f64,
        strike: f64,
        expiry: f64,
        rate: f64,
        volatility: f64,
        is_call: bool,
    ) -> Result<UnifiedPricingResult, PricingError> {
        let start = Instant::now();

        // Validate inputs
        if spot <= 0.0 {
            return Err(PricingError::InvalidInput {
                reason: "spot must be positive".to_string(),
            });
        }
        if strike <= 0.0 {
            return Err(PricingError::InvalidInput {
                reason: "strike must be positive".to_string(),
            });
        }
        if expiry <= 0.0 {
            return Err(PricingError::InvalidInput {
                reason: "expiry must be positive".to_string(),
            });
        }
        if volatility <= 0.0 {
            return Err(PricingError::InvalidInput {
                reason: "volatility must be positive".to_string(),
            });
        }

        // Black-Scholes formula
        let d1 = ((spot / strike).ln() + (rate + 0.5 * volatility * volatility) * expiry)
            / (volatility * expiry.sqrt());
        let d2 = d1 - volatility * expiry.sqrt();

        // Abramowitz & Stegun approximation for normal CDF
        fn norm_cdf(x: f64) -> f64 {
            const A1: f64 = 0.254829592;
            const A2: f64 = -0.284496736;
            const A3: f64 = 1.421413741;
            const A4: f64 = -1.453152027;
            const A5: f64 = 1.061405429;
            const P: f64 = 0.3275911;

            let sign = if x < 0.0 { -1.0 } else { 1.0 };
            let x_abs = x.abs() / std::f64::consts::SQRT_2;

            let t = 1.0 / (1.0 + P * x_abs);
            let y =
                1.0 - (((((A5 * t + A4) * t) + A3) * t + A2) * t + A1) * t * (-x_abs * x_abs).exp();

            0.5 * (1.0 + sign * y)
        }

        // Normal PDF for Greeks
        fn norm_pdf(x: f64) -> f64 { (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt() }

        let pv = if is_call {
            spot * norm_cdf(d1) - strike * (-rate * expiry).exp() * norm_cdf(d2)
        } else {
            strike * (-rate * expiry).exp() * norm_cdf(-d2) - spot * norm_cdf(-d1)
        };

        let greeks = if self.config.compute_greeks {
            let delta = if is_call {
                norm_cdf(d1)
            } else {
                norm_cdf(d1) - 1.0
            };
            let gamma = norm_pdf(d1) / (spot * volatility * expiry.sqrt());
            let vega = spot * norm_pdf(d1) * expiry.sqrt() / 100.0; // Per 1% vol move
            let theta = {
                let term1 = -spot * norm_pdf(d1) * volatility / (2.0 * expiry.sqrt());
                if is_call {
                    (term1 - rate * strike * (-rate * expiry).exp() * norm_cdf(d2)) / 365.0
                } else {
                    (term1 + rate * strike * (-rate * expiry).exp() * norm_cdf(-d2)) / 365.0
                }
            };
            let rho = if is_call {
                strike * expiry * (-rate * expiry).exp() * norm_cdf(d2) / 100.0
            } else {
                -strike * expiry * (-rate * expiry).exp() * norm_cdf(-d2) / 100.0
            };

            Some(UnifiedGreeks::new(
                Some(delta),
                Some(gamma),
                Some(vega),
                Some(theta),
                Some(rho),
            ))
        } else {
            None
        };

        let elapsed = start.elapsed();

        let mut result =
            UnifiedPricingResult::new(pv, PricingMethod::Analytical, elapsed.as_nanos() as u64);

        if let Some(g) = greeks {
            result = result.with_greeks(g);
        }

        result = result.with_metadata(PricingMetadata::Discount {
            model: "Black-Scholes".to_string(),
        });

        Ok(result)
    }

    /// Returns whether the dispatcher supports the given method.
    pub fn supports_method(&self, method: PricingMethod) -> bool {
        matches!(
            method,
            PricingMethod::Tree | PricingMethod::MonteCarlo | PricingMethod::Analytical
        )
    }

    // =========================================================================
    // Instrument Integration (requires l1l2-integration feature)
    // =========================================================================

    /// Checks if the given method supports the instrument's exercise style.
    ///
    /// # Arguments
    ///
    /// * `method` - Pricing method to check
    /// * `is_american` - Whether the instrument has American exercise style
    ///
    /// # Returns
    ///
    /// `true` if the method supports the exercise style, `false` otherwise.
    /// For American options, only Tree method is recommended.
    #[cfg(feature = "l1l2-integration")]
    pub fn supports_exercise_style(&self, method: PricingMethod, is_american: bool) -> bool {
        match method {
            PricingMethod::Tree => true, // Tree supports both European and American
            PricingMethod::MonteCarlo => !is_american, // MC only European
            PricingMethod::Analytical => !is_american, // Analytical only European
        }
    }

    /// Returns recommended pricing method for the given exercise style.
    ///
    /// For American options, Tree method is recommended.
    /// For European options, Analytical is fastest but all methods work.
    #[cfg(feature = "l1l2-integration")]
    pub fn recommended_method(&self, is_american: bool) -> PricingMethod {
        if is_american {
            PricingMethod::Tree
        } else {
            PricingMethod::Analytical
        }
    }

    /// Prices a `VanillaOption` instrument.
    ///
    /// # Arguments
    ///
    /// * `method` - Pricing method to use
    /// * `option` - The vanilla option instrument
    /// * `spot` - Current spot price
    /// * `rate` - Risk-free rate (annualized)
    /// * `volatility` - Volatility (annualized)
    /// * `tree_steps` - Optional number of tree steps (for Tree method)
    /// * `mc_paths` - Optional number of MC paths (for MonteCarlo method)
    ///
    /// # Errors
    ///
    /// Returns `PricingError` if:
    /// - Input parameters are invalid
    /// - American option used with incompatible method (warning logged,
    ///   proceeds anyway)
    /// - Pricing method fails
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pricer_pricing::dispatcher::PricingMethodDispatcher;
    /// use infra_config::PricingMethod;
    /// use infra_master::trade::{VanillaOption, ExerciseStyle};
    ///
    /// let dispatcher = PricingMethodDispatcher::new();
    /// let option = VanillaOption::new(100.0, 1.0, 1.0, true, ExerciseStyle::European);
    ///
    /// let result = dispatcher.price_vanilla_option(
    ///     PricingMethod::Tree,
    ///     &option,
    ///     100.0, 0.05, 0.2, None, None,
    /// ).unwrap();
    /// ```
    #[cfg(feature = "l1l2-integration")]
    pub fn price_vanilla_option<T: num_traits::Float + Into<f64> + Copy>(
        &self,
        method: PricingMethod,
        option: &VanillaOption<T>,
        spot: f64,
        rate: f64,
        volatility: f64,
        tree_steps: Option<usize>,
        mc_paths: Option<usize>,
    ) -> Result<UnifiedPricingResult, PricingError> {
        let strike: f64 = option.strike().into();
        let expiry: f64 = option.expiry().into();
        let is_call = option.payoff_type() == OptionType::Call;
        let is_american = option.exercise_style() == ExerciseStyle::American;

        // Warn if using incompatible method for American options
        if is_american && !self.supports_exercise_style(method, is_american) {
            // Log warning (in production, use proper logging crate)
            #[cfg(debug_assertions)]
            eprintln!(
                "Warning: {:?} method may not accurately price American options. Consider using Tree method.",
                method
            );
        }

        self.price_vanilla(
            method,
            spot,
            strike,
            expiry,
            rate,
            volatility,
            is_call,
            is_american,
            tree_steps,
            mc_paths,
        )
    }

    /// Prices a `Forward` contract.
    ///
    /// Forwards are best priced using the Discount (Analytical) method
    /// as they have a closed-form solution: PV = (S - K * exp(-rT)) * direction
    ///
    /// # Arguments
    ///
    /// * `method` - Pricing method to use (Analytical recommended)
    /// * `forward` - The forward contract
    /// * `spot` - Current spot price
    /// * `rate` - Risk-free rate (annualized)
    ///
    /// # Returns
    ///
    /// Returns `UnifiedPricingResult` with the forward price.
    /// For forwards, Greeks are simpler: Delta = direction, Gamma = 0.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pricer_pricing::dispatcher::PricingMethodDispatcher;
    /// use infra_config::PricingMethod;
    /// use infra_master::trade::{Forward, ForwardDirection};
    ///
    /// let dispatcher = PricingMethodDispatcher::new();
    /// let forward = Forward::new(100.0, 1.0, 1.0, ForwardDirection::Long);
    ///
    /// let result = dispatcher.price_forward(
    ///     &forward,
    ///     100.0, // spot
    ///     0.05,  // rate
    /// ).unwrap();
    /// ```
    #[cfg(feature = "l1l2-integration")]
    pub fn price_forward<T: num_traits::Float + Into<f64> + Copy>(
        &self,
        forward: &Forward<T>,
        spot: f64,
        rate: f64,
    ) -> Result<UnifiedPricingResult, PricingError> {
        let start = Instant::now();

        let strike: f64 = forward.strike().into();
        let expiry: f64 = forward.expiry().into();
        let notional: f64 = forward.notional().into();
        let direction_sign = forward.direction().sign();

        // Validate inputs
        if spot <= 0.0 {
            return Err(PricingError::InvalidInput {
                reason: "spot must be positive".to_string(),
            });
        }
        if expiry <= 0.0 {
            return Err(PricingError::InvalidInput {
                reason: "expiry must be positive".to_string(),
            });
        }

        // Forward pricing: PV = (S - K * exp(-rT)) * direction * notional
        let discount_factor = (-rate * expiry).exp();
        let pv = (spot - strike * discount_factor) * direction_sign * notional;

        // Greeks for a forward contract
        let greeks = if self.config.compute_greeks {
            Some(UnifiedGreeks::new(
                Some(direction_sign * notional), // Delta = direction * notional
                Some(0.0),                       // Gamma = 0 (linear payoff)
                Some(0.0),                       // Vega = 0 (no vol dependency)
                Some(rate * strike * discount_factor * direction_sign * notional / 365.0), // Theta
                Some(-strike * expiry * discount_factor * direction_sign * notional / 100.0), // Rho
            ))
        } else {
            None
        };

        let elapsed = start.elapsed();

        let mut result =
            UnifiedPricingResult::new(pv, PricingMethod::Analytical, elapsed.as_nanos() as u64);

        if let Some(g) = greeks {
            result = result.with_greeks(g);
        }

        result = result.with_metadata(PricingMetadata::Discount {
            model: "Forward".to_string(),
        });

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatcher_new() {
        let dispatcher = PricingMethodDispatcher::new();
        assert_eq!(dispatcher.config().default_tree_steps, 100);
        assert!(dispatcher.config().compute_greeks);
    }

    #[test]
    fn test_dispatcher_with_config() {
        let config = DispatcherConfig {
            default_tree_steps: 500,
            default_mc_paths: 50_000,
            default_mc_steps: 365,
            compute_greeks: false,
        };
        let dispatcher = PricingMethodDispatcher::with_config(config);

        assert_eq!(dispatcher.config().default_tree_steps, 500);
        assert!(!dispatcher.config().compute_greeks);
    }

    #[test]
    fn test_dispatcher_price_with_tree() {
        let dispatcher = PricingMethodDispatcher::new();
        let result = dispatcher
            .price_vanilla(
                PricingMethod::Tree,
                100.0,
                100.0,
                1.0,
                0.05,
                0.2,
                true,
                false,
                Some(200),
                None,
            )
            .unwrap();

        assert!(result.pv > 0.0);
        assert_eq!(result.method, PricingMethod::Tree);
        assert!(result.has_greeks());
        assert_eq!(result.num_steps(), Some(200));
    }

    #[test]
    fn test_dispatcher_price_with_monte_carlo() {
        let dispatcher = PricingMethodDispatcher::new();
        let result = dispatcher
            .price_vanilla(
                PricingMethod::MonteCarlo,
                100.0,
                100.0,
                1.0,
                0.05,
                0.2,
                true,
                false,
                None,
                Some(1000),
            )
            .unwrap();

        assert!(result.pv > 0.0);
        assert_eq!(result.method, PricingMethod::MonteCarlo);
        assert_eq!(result.num_paths(), Some(1000));
    }

    #[test]
    fn test_dispatcher_price_with_analytical() {
        let dispatcher = PricingMethodDispatcher::new();
        let result = dispatcher
            .price_vanilla(
                PricingMethod::Analytical,
                100.0,
                100.0,
                1.0,
                0.05,
                0.2,
                true,
                false,
                None,
                None,
            )
            .unwrap();

        assert!(result.pv > 0.0);
        assert_eq!(result.method, PricingMethod::Analytical);
        assert!(result.has_greeks());
        // BS ATM call around 10.5
        assert!(result.pv > 8.0 && result.pv < 15.0);
    }

    #[test]
    fn test_dispatcher_analytical_vs_tree_convergence() {
        let dispatcher = PricingMethodDispatcher::new();

        // Analytical price
        let analytical = dispatcher
            .price_vanilla(
                PricingMethod::Analytical,
                100.0,
                100.0,
                1.0,
                0.05,
                0.2,
                true,
                false,
                None,
                None,
            )
            .unwrap();

        // Tree price with many steps
        let tree = dispatcher
            .price_vanilla(
                PricingMethod::Tree,
                100.0,
                100.0,
                1.0,
                0.05,
                0.2,
                true,
                false,
                Some(500),
                None,
            )
            .unwrap();

        // Should converge within 0.05
        assert!(
            (analytical.pv - tree.pv).abs() < 0.05,
            "Analytical {} vs Tree {}",
            analytical.pv,
            tree.pv
        );
    }

    #[test]
    fn test_dispatcher_put_prices() {
        let dispatcher = PricingMethodDispatcher::new();

        let call = dispatcher
            .price_vanilla(
                PricingMethod::Analytical,
                100.0,
                100.0,
                1.0,
                0.05,
                0.2,
                true,
                false,
                None,
                None,
            )
            .unwrap();

        let put = dispatcher
            .price_vanilla(
                PricingMethod::Analytical,
                100.0,
                100.0,
                1.0,
                0.05,
                0.2,
                false,
                false,
                None,
                None,
            )
            .unwrap();

        // Put-call parity: C - P = S - K * exp(-r * T)
        let spot: f64 = 100.0;
        let strike: f64 = 100.0;
        let rate: f64 = 0.05;
        let expiry: f64 = 1.0;
        let parity_diff = spot - strike * (-rate * expiry).exp();

        assert!(
            ((call.pv - put.pv) - parity_diff).abs() < 0.01,
            "Put-call parity failed: C-P={}, expected {}",
            call.pv - put.pv,
            parity_diff
        );
    }

    #[test]
    fn test_dispatcher_invalid_input() {
        let dispatcher = PricingMethodDispatcher::new();

        let result = dispatcher.price_vanilla(
            PricingMethod::Tree,
            -100.0, // Invalid spot
            100.0,
            1.0,
            0.05,
            0.2,
            true,
            false,
            None,
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_dispatcher_supports_method() {
        let dispatcher = PricingMethodDispatcher::new();

        assert!(dispatcher.supports_method(PricingMethod::Tree));
        assert!(dispatcher.supports_method(PricingMethod::MonteCarlo));
        assert!(dispatcher.supports_method(PricingMethod::Analytical));
    }

    #[test]
    fn test_dispatcher_no_greeks() {
        let config = DispatcherConfig {
            compute_greeks: false,
            ..Default::default()
        };
        let dispatcher = PricingMethodDispatcher::with_config(config);

        let result = dispatcher
            .price_vanilla(
                PricingMethod::Tree,
                100.0,
                100.0,
                1.0,
                0.05,
                0.2,
                true,
                false,
                None,
                None,
            )
            .unwrap();

        assert!(!result.has_greeks());
    }

    #[test]
    fn test_dispatcher_american_put() {
        let dispatcher = PricingMethodDispatcher::new();

        let european = dispatcher
            .price_vanilla(
                PricingMethod::Tree,
                100.0,
                100.0,
                1.0,
                0.05,
                0.2,
                false, // put
                false, // European
                Some(500),
                None,
            )
            .unwrap();

        let american = dispatcher
            .price_vanilla(
                PricingMethod::Tree,
                100.0,
                100.0,
                1.0,
                0.05,
                0.2,
                false, // put
                true,  // American
                Some(500),
                None,
            )
            .unwrap();

        // American put >= European put
        assert!(
            american.pv >= european.pv - 1e-6,
            "American {} should be >= European {}",
            american.pv,
            european.pv
        );
    }
}

// =============================================================================
// Instrument Integration Tests (requires l1l2-integration feature)
// =============================================================================

#[cfg(all(test, feature = "l1l2-integration"))]
mod instrument_tests {
    use infra_master::trade::{
        ExerciseStyle, Forward, ForwardDirection, InstrumentParams, OptionType, VanillaOption,
    };

    use super::*;

    /// Helper to create VanillaOption with simpler signature.
    fn make_option(
        strike: f64,
        expiry: f64,
        is_call: bool,
        is_american: bool,
    ) -> VanillaOption<f64> {
        let params = InstrumentParams::new(strike, expiry, 1.0).unwrap();
        let payoff_type = if is_call {
            OptionType::Call
        } else {
            OptionType::Put
        };
        let exercise_style = if is_american {
            ExerciseStyle::American
        } else {
            ExerciseStyle::European
        };
        VanillaOption::new(params, payoff_type, exercise_style, 1e-6)
    }

    #[test]
    fn test_supports_exercise_style() {
        let dispatcher = PricingMethodDispatcher::new();

        // European options supported by all methods
        assert!(dispatcher.supports_exercise_style(PricingMethod::Tree, false));
        assert!(dispatcher.supports_exercise_style(PricingMethod::MonteCarlo, false));
        assert!(dispatcher.supports_exercise_style(PricingMethod::Analytical, false));

        // American options only supported by Tree
        assert!(dispatcher.supports_exercise_style(PricingMethod::Tree, true));
        assert!(!dispatcher.supports_exercise_style(PricingMethod::MonteCarlo, true));
        assert!(!dispatcher.supports_exercise_style(PricingMethod::Analytical, true));
    }

    #[test]
    fn test_recommended_method() {
        let dispatcher = PricingMethodDispatcher::new();

        // European → Analytical (fastest)
        assert_eq!(
            dispatcher.recommended_method(false),
            PricingMethod::Analytical
        );

        // American → Tree (required for early exercise)
        assert_eq!(dispatcher.recommended_method(true), PricingMethod::Tree);
    }

    #[test]
    fn test_price_vanilla_option_european_call() {
        let dispatcher = PricingMethodDispatcher::new();

        let option = make_option(100.0, 1.0, true, false);

        let result = dispatcher
            .price_vanilla_option(
                PricingMethod::Tree,
                &option,
                100.0, // spot
                0.05,  // rate
                0.2,   // volatility
                Some(200),
                None,
            )
            .unwrap();

        assert!(result.pv > 0.0);
        assert_eq!(result.method, PricingMethod::Tree);
    }

    #[test]
    fn test_price_vanilla_option_american_put() {
        let dispatcher = PricingMethodDispatcher::new();

        let european = make_option(100.0, 1.0, false, false);
        let american = make_option(100.0, 1.0, false, true);

        let european_result = dispatcher
            .price_vanilla_option(
                PricingMethod::Tree,
                &european,
                100.0,
                0.05,
                0.2,
                Some(300),
                None,
            )
            .unwrap();

        let american_result = dispatcher
            .price_vanilla_option(
                PricingMethod::Tree,
                &american,
                100.0,
                0.05,
                0.2,
                Some(300),
                None,
            )
            .unwrap();

        // American put >= European put
        assert!(
            american_result.pv >= european_result.pv - 1e-6,
            "American {} should be >= European {}",
            american_result.pv,
            european_result.pv
        );
    }

    #[test]
    fn test_price_vanilla_option_consistency_with_raw() {
        let dispatcher = PricingMethodDispatcher::new();

        let option = make_option(100.0, 1.0, true, false);

        // Price using instrument
        let instrument_result = dispatcher
            .price_vanilla_option(
                PricingMethod::Analytical,
                &option,
                100.0,
                0.05,
                0.2,
                None,
                None,
            )
            .unwrap();

        // Price using raw parameters
        let raw_result = dispatcher
            .price_vanilla(
                PricingMethod::Analytical,
                100.0,
                100.0,
                1.0,
                0.05,
                0.2,
                true,
                false,
                None,
                None,
            )
            .unwrap();

        // Should be identical
        assert!(
            (instrument_result.pv - raw_result.pv).abs() < 1e-10,
            "Instrument {} vs Raw {}",
            instrument_result.pv,
            raw_result.pv
        );
    }

    #[test]
    fn test_price_forward_long() {
        let dispatcher = PricingMethodDispatcher::new();

        let forward = Forward::new(100.0, 1.0, 1.0, ForwardDirection::Long).unwrap();

        let result = dispatcher.price_forward(&forward, 100.0, 0.05).unwrap();

        // PV = (S - K * exp(-rT)) * direction * notional
        // PV = (100 - 100 * exp(-0.05)) * 1 * 1 ≈ 4.88
        let expected = 100.0 - 100.0 * (-0.05f64).exp();
        assert!(
            (result.pv - expected).abs() < 1e-6,
            "Forward PV {} vs expected {}",
            result.pv,
            expected
        );

        // Check Greeks
        assert!(result.has_greeks());
        let delta = result.delta().unwrap();
        assert!(
            (delta - 1.0).abs() < 1e-6,
            "Delta should be 1 for long forward"
        );

        let gamma = result.gamma().unwrap();
        assert!((gamma - 0.0).abs() < 1e-6, "Gamma should be 0 for forward");
    }

    #[test]
    fn test_price_forward_short() {
        let dispatcher = PricingMethodDispatcher::new();

        let forward = Forward::new(100.0, 1.0, 1.0, ForwardDirection::Short).unwrap();

        let result = dispatcher.price_forward(&forward, 100.0, 0.05).unwrap();

        // PV = (S - K * exp(-rT)) * direction * notional
        // PV = (100 - 100 * exp(-0.05)) * -1 * 1 ≈ -4.88
        let expected = -(100.0 - 100.0 * (-0.05f64).exp());
        assert!(
            (result.pv - expected).abs() < 1e-6,
            "Short Forward PV {} vs expected {}",
            result.pv,
            expected
        );

        // Delta should be -1 for short forward
        let delta = result.delta().unwrap();
        assert!(
            (delta - (-1.0)).abs() < 1e-6,
            "Delta should be -1 for short forward"
        );
    }

    #[test]
    fn test_price_forward_with_notional() {
        let dispatcher = PricingMethodDispatcher::new();

        let forward = Forward::new(100.0, 1.0, 1_000_000.0, ForwardDirection::Long).unwrap();

        let result = dispatcher.price_forward(&forward, 105.0, 0.05).unwrap();

        // PV = (105 - 100 * exp(-0.05)) * 1 * 1_000_000
        let expected = (105.0 - 100.0 * (-0.05f64).exp()) * 1_000_000.0;
        assert!(
            (result.pv - expected).abs() < 1.0, // Allow small tolerance for large notional
            "Forward with notional PV {} vs expected {}",
            result.pv,
            expected
        );
    }

    #[test]
    fn test_price_forward_invalid_spot() {
        let dispatcher = PricingMethodDispatcher::new();

        let forward = Forward::new(100.0, 1.0, 1.0, ForwardDirection::Long).unwrap();

        let result = dispatcher.price_forward(&forward, -100.0, 0.05);

        assert!(result.is_err());
    }

    #[test]
    fn test_price_forward_invalid_expiry() {
        let dispatcher = PricingMethodDispatcher::new();

        // Create forward with negative expiry
        let forward = Forward::new(100.0, -1.0, 1.0, ForwardDirection::Long).unwrap();

        let result = dispatcher.price_forward(&forward, 100.0, 0.05);

        assert!(result.is_err());
    }
}
