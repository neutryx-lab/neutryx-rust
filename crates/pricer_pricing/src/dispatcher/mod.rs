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

use std::time::Instant;

use infra_config::PricingMethod;

use crate::generic_pricer::PricingError;
use crate::result::{PricingMetadata, TreeTypeMetadata, UnifiedGreeks, UnifiedPricingResult};
use crate::tree::BinomialTree;

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
    fn default() -> Self {
        Self::new()
    }
}

impl PricingMethodDispatcher {
    /// Creates a new dispatcher with default configuration.
    pub fn new() -> Self {
        Self {
            config: DispatcherConfig::default(),
        }
    }

    /// Creates a new dispatcher with the specified configuration.
    pub fn with_config(config: DispatcherConfig) -> Self {
        Self { config }
    }

    /// Returns the dispatcher configuration.
    pub fn config(&self) -> &DispatcherConfig {
        &self.config
    }

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
            PricingMethod::Tree => {
                self.price_with_tree(spot, strike, expiry, rate, volatility, is_call, is_american, tree_steps)
            }
            PricingMethod::MonteCarlo => {
                self.price_with_monte_carlo(
                    spot,
                    strike,
                    expiry,
                    rate,
                    volatility,
                    is_call,
                    is_american,
                    mc_paths,
                )
            }
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

        let mut result = UnifiedPricingResult::new(pv, PricingMethod::Tree, elapsed.as_nanos() as u64);

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
        use crate::mc::{Greek, MonteCarloConfig, MonteCarloPricer, GbmParams, PayoffParams, PayoffType};

        let start = Instant::now();
        let num_paths = mc_paths.unwrap_or(self.config.default_mc_paths);

        let gbm_params = GbmParams {
            spot,
            rate,
            volatility,
            maturity: expiry,
        };

        let payoff_type = if is_call { PayoffType::Call } else { PayoffType::Put };
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
                &[Greek::Delta, Greek::Gamma, Greek::Vega, Greek::Theta, Greek::Rho],
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
            let y = 1.0 - (((((A5 * t + A4) * t) + A3) * t + A2) * t + A1) * t * (-x_abs * x_abs).exp();

            0.5 * (1.0 + sign * y)
        }

        // Normal PDF for Greeks
        fn norm_pdf(x: f64) -> f64 {
            (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
        }

        let pv = if is_call {
            spot * norm_cdf(d1) - strike * (-rate * expiry).exp() * norm_cdf(d2)
        } else {
            strike * (-rate * expiry).exp() * norm_cdf(-d2) - spot * norm_cdf(-d1)
        };

        let greeks = if self.config.compute_greeks {
            let delta = if is_call { norm_cdf(d1) } else { norm_cdf(d1) - 1.0 };
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
