//! Tree pricing method interface.

use std::time::Instant;

use super::{
    binomial::BinomialTree,
    config::{TreeConfig, TreeType},
    trinomial::TrinomialTree,
};
use crate::generic_pricer::PricingError;

/// Greeks computed from tree-based pricing.
#[derive(Debug, Clone, Default)]
pub struct TreeGreeks {
    /// Delta: first derivative with respect to spot
    pub delta: Option<f64>,
    /// Gamma: second derivative with respect to spot
    pub gamma: Option<f64>,
}

/// Result from tree-based pricing.
#[derive(Debug, Clone)]
pub struct TreePricingResult {
    /// Present value (option price)
    pub pv: f64,
    /// Greeks computed from the tree
    pub greeks: Option<TreeGreeks>,
    /// Number of steps used in the tree
    pub num_steps: usize,
    /// Tree type used
    pub tree_type: TreeType,
    /// Computation time in nanoseconds
    pub computation_time_ns: u64,
}

/// Tree-based pricing method.
#[derive(Debug, Clone)]
pub struct TreeMethod {
    config: TreeConfig,
}

impl TreeMethod {
    /// Creates a new TreeMethod with the given configuration.
    pub fn new(config: TreeConfig) -> Self { Self { config } }

    /// Creates a TreeMethod with default configuration.
    pub fn with_defaults() -> Self { Self::new(TreeConfig::default()) }

    /// Returns the configuration.
    pub fn config(&self) -> &TreeConfig { &self.config }

    /// Prices a vanilla option.
    pub fn price(
        &self,
        spot: f64,
        strike: f64,
        expiry: f64,
        rate: f64,
        volatility: f64,
        is_call: bool,
        is_american: bool,
    ) -> Result<TreePricingResult, PricingError> {
        let start = Instant::now();

        match self.config.tree_type {
            TreeType::Binomial => {
                let tree = BinomialTree::new(
                    spot,
                    strike,
                    expiry,
                    rate,
                    volatility,
                    self.config.num_steps,
                    is_call,
                    is_american,
                )
                .map_err(|e| PricingError::InvalidInput {
                    reason: e.to_string(),
                })?;

                let pv = tree.price();

                let greeks = if self.config.compute_greeks {
                    Some(TreeGreeks {
                        delta: Some(tree.delta()),
                        gamma: Some(tree.gamma()),
                    })
                } else {
                    None
                };

                let elapsed = start.elapsed();

                Ok(TreePricingResult {
                    pv,
                    greeks,
                    num_steps: self.config.num_steps,
                    tree_type: TreeType::Binomial,
                    computation_time_ns: elapsed.as_nanos() as u64,
                })
            }
            TreeType::Trinomial => {
                let tree = TrinomialTree::new(
                    spot,
                    strike,
                    expiry,
                    rate,
                    volatility,
                    self.config.num_steps,
                    is_call,
                    is_american,
                )
                .map_err(|e| PricingError::InvalidInput {
                    reason: e.to_string(),
                })?;

                let pv = tree.price();

                let greeks = if self.config.compute_greeks {
                    Some(TreeGreeks {
                        delta: Some(tree.delta()),
                        gamma: Some(tree.gamma()),
                    })
                } else {
                    None
                };

                let elapsed = start.elapsed();

                Ok(TreePricingResult {
                    pv,
                    greeks,
                    num_steps: self.config.num_steps,
                    tree_type: TreeType::Trinomial,
                    computation_time_ns: elapsed.as_nanos() as u64,
                })
            }
        }
    }

    /// Computes only Greeks (without full pricing result).
    pub fn compute_greeks(
        &self,
        spot: f64,
        strike: f64,
        expiry: f64,
        rate: f64,
        volatility: f64,
        is_call: bool,
        is_american: bool,
    ) -> Result<TreeGreeks, PricingError> {
        match self.config.tree_type {
            TreeType::Binomial => {
                let tree = BinomialTree::new(
                    spot,
                    strike,
                    expiry,
                    rate,
                    volatility,
                    self.config.num_steps,
                    is_call,
                    is_american,
                )
                .map_err(|e| PricingError::InvalidInput {
                    reason: e.to_string(),
                })?;

                Ok(TreeGreeks {
                    delta: Some(tree.delta()),
                    gamma: Some(tree.gamma()),
                })
            }
            TreeType::Trinomial => {
                let tree = TrinomialTree::new(
                    spot,
                    strike,
                    expiry,
                    rate,
                    volatility,
                    self.config.num_steps,
                    is_call,
                    is_american,
                )
                .map_err(|e| PricingError::InvalidInput {
                    reason: e.to_string(),
                })?;

                Ok(TreeGreeks {
                    delta: Some(tree.delta()),
                    gamma: Some(tree.gamma()),
                })
            }
        }
    }

    /// Checks if this method supports the given parameters.
    pub fn supports_vanilla(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_method_new() {
        let config = TreeConfig::new(200);
        let method = TreeMethod::new(config);
        assert_eq!(method.config().num_steps, 200);
    }

    #[test]
    fn test_tree_method_with_defaults() {
        let method = TreeMethod::with_defaults();
        assert_eq!(method.config().num_steps, 100);
        assert_eq!(method.config().tree_type, TreeType::Binomial);
    }

    #[test]
    fn test_tree_method_price_european_call() {
        let method = TreeMethod::with_defaults();
        let result = method.price(100.0, 100.0, 1.0, 0.05, 0.2, true, false);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.pv > 8.0 && result.pv < 15.0);
        assert!(result.greeks.is_some());
        assert!(result.computation_time_ns > 0);
    }

    #[test]
    fn test_tree_method_price_american_put() {
        let method = TreeMethod::with_defaults();
        let result = method.price(100.0, 100.0, 1.0, 0.05, 0.2, false, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.pv > 0.0);
        assert!(result.greeks.is_some());
        let greeks = result.greeks.unwrap();
        assert!(greeks.delta.is_some());
        assert!(greeks.gamma.is_some());
        assert!(greeks.delta.unwrap() < 0.0);
    }

    #[test]
    fn test_tree_method_price_invalid_params() {
        let method = TreeMethod::with_defaults();
        let result = method.price(-100.0, 100.0, 1.0, 0.05, 0.2, true, false);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("spot"));
    }

    #[test]
    fn test_tree_method_no_greeks() {
        let config = TreeConfig::builder()
            .num_steps(100)
            .compute_greeks(false)
            .build()
            .unwrap();
        let method = TreeMethod::new(config);

        let result = method
            .price(100.0, 100.0, 1.0, 0.05, 0.2, true, false)
            .unwrap();
        assert!(result.greeks.is_none());
    }

    #[test]
    fn test_tree_method_compute_greeks_only() {
        let method = TreeMethod::with_defaults();
        let greeks = method
            .compute_greeks(100.0, 100.0, 1.0, 0.05, 0.2, true, false)
            .unwrap();

        assert!(greeks.delta.is_some());
        assert!(greeks.gamma.is_some());
        assert!(greeks.delta.unwrap() > 0.0);
        assert!(greeks.gamma.unwrap() > 0.0);
    }

    #[test]
    fn test_tree_method_trinomial_pricing() {
        let config = TreeConfig::builder()
            .tree_type(TreeType::Trinomial)
            .num_steps(100)
            .build()
            .unwrap();
        let method = TreeMethod::new(config);

        let result = method.price(100.0, 100.0, 1.0, 0.05, 0.2, true, false);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.pv > 8.0 && result.pv < 15.0);
        assert_eq!(result.tree_type, TreeType::Trinomial);
        assert!(result.greeks.is_some());
    }

    #[test]
    fn test_tree_method_trinomial_greeks() {
        let config = TreeConfig::builder()
            .tree_type(TreeType::Trinomial)
            .num_steps(100)
            .compute_greeks(true)
            .build()
            .unwrap();
        let method = TreeMethod::new(config);

        let greeks = method
            .compute_greeks(100.0, 100.0, 1.0, 0.05, 0.2, true, false)
            .unwrap();

        assert!(greeks.delta.is_some());
        assert!(greeks.gamma.is_some());
        assert!(greeks.delta.unwrap() > 0.0);
        assert!(greeks.gamma.unwrap() > 0.0);
    }

    #[test]
    fn test_tree_method_supports_vanilla() {
        let method = TreeMethod::with_defaults();
        assert!(method.supports_vanilla());
    }

    #[test]
    fn test_tree_pricing_result_metadata() {
        let method = TreeMethod::with_defaults();
        let result = method
            .price(100.0, 100.0, 1.0, 0.05, 0.2, true, false)
            .unwrap();

        assert_eq!(result.num_steps, 100);
        assert_eq!(result.tree_type, TreeType::Binomial);
    }
}
