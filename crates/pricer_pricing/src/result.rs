//! Unified pricing result types.
//!
//! This module provides the unified pricing result structure that all
//! pricing methods (Discount, Monte Carlo, Tree) return.
//!
//! # Design
//!
//! - [`UnifiedPricingResult`]: Unified result structure for all methods
//! - [`UnifiedGreeks`]: Greeks computed from pricing
//! - [`PricingMetadata`]: Method-specific metadata
//!
//! The naming "Unified" distinguishes from the existing
//! `generic_pricer::PricingResult` which is Trade-centric with legs and
//! cashflows.

use infra_config::PricingMethod;

/// Method-specific pricing metadata.
#[derive(Debug, Clone)]
pub enum PricingMetadata {
    /// Monte Carlo method metadata.
    MonteCarlo {
        /// Number of paths used.
        num_paths: usize,
        /// Standard error of the estimate.
        standard_error: f64,
    },
    /// Tree method metadata.
    Tree {
        /// Number of time steps.
        num_steps: usize,
        /// Tree type used.
        tree_type: TreeTypeMetadata,
    },
    /// Analytical (Discount) method metadata.
    Discount {
        /// Model used (e.g., "Black-Scholes", "Garman-Kohlhagen").
        model: String,
    },
}

/// Tree type for metadata (mirrors infra_config::TreeType).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeTypeMetadata {
    /// Binomial tree.
    Binomial,
    /// Trinomial tree.
    Trinomial,
}

/// Greeks computed from pricing.
///
/// All fields are optional since not all methods compute all Greeks.
#[derive(Debug, Clone, Default)]
pub struct UnifiedGreeks {
    /// Delta: first derivative with respect to spot.
    pub delta: Option<f64>,
    /// Gamma: second derivative with respect to spot.
    pub gamma: Option<f64>,
    /// Vega: derivative with respect to volatility.
    pub vega: Option<f64>,
    /// Theta: derivative with respect to time.
    pub theta: Option<f64>,
    /// Rho: derivative with respect to interest rate.
    pub rho: Option<f64>,
}

impl UnifiedGreeks {
    /// Creates new Greeks with all values specified.
    pub fn new(
        delta: Option<f64>,
        gamma: Option<f64>,
        vega: Option<f64>,
        theta: Option<f64>,
        rho: Option<f64>,
    ) -> Self {
        Self {
            delta,
            gamma,
            vega,
            theta,
            rho,
        }
    }

    /// Creates Greeks with only delta and gamma (common for Tree methods).
    pub fn from_delta_gamma(delta: f64, gamma: f64) -> Self {
        Self {
            delta: Some(delta),
            gamma: Some(gamma),
            ..Default::default()
        }
    }

    /// Returns true if any Greeks are present.
    pub fn has_any(&self) -> bool {
        self.delta.is_some()
            || self.gamma.is_some()
            || self.vega.is_some()
            || self.theta.is_some()
            || self.rho.is_some()
    }
}

/// Unified pricing result structure.
///
/// This is the standard result type returned by all pricing methods.
/// It provides a consistent interface regardless of whether the pricing
/// was done analytically, via Monte Carlo, or via Tree methods.
#[derive(Debug, Clone)]
pub struct UnifiedPricingResult {
    /// Present value (option price or PV).
    pub pv: f64,
    /// Pricing method used.
    pub method: PricingMethod,
    /// Computation time in nanoseconds.
    pub computation_time_ns: u64,
    /// Greeks (optional).
    pub greeks: Option<UnifiedGreeks>,
    /// Method-specific metadata (optional).
    pub metadata: Option<PricingMetadata>,
}

impl UnifiedPricingResult {
    /// Creates a new pricing result.
    pub fn new(pv: f64, method: PricingMethod, computation_time_ns: u64) -> Self {
        Self {
            pv,
            method,
            computation_time_ns,
            greeks: None,
            metadata: None,
        }
    }

    /// Creates a pricing result with Greeks.
    pub fn with_greeks(mut self, greeks: UnifiedGreeks) -> Self {
        self.greeks = Some(greeks);
        self
    }

    /// Creates a pricing result with metadata.
    pub fn with_metadata(mut self, metadata: PricingMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Returns the delta if computed.
    pub fn delta(&self) -> Option<f64> {
        self.greeks.as_ref().and_then(|g| g.delta)
    }

    /// Returns the gamma if computed.
    pub fn gamma(&self) -> Option<f64> {
        self.greeks.as_ref().and_then(|g| g.gamma)
    }

    /// Returns the vega if computed.
    pub fn vega(&self) -> Option<f64> {
        self.greeks.as_ref().and_then(|g| g.vega)
    }

    /// Returns the theta if computed.
    pub fn theta(&self) -> Option<f64> {
        self.greeks.as_ref().and_then(|g| g.theta)
    }

    /// Returns the rho if computed.
    pub fn rho(&self) -> Option<f64> {
        self.greeks.as_ref().and_then(|g| g.rho)
    }

    /// Returns true if this result includes Greeks.
    pub fn has_greeks(&self) -> bool {
        self.greeks.as_ref().is_some_and(|g| g.has_any())
    }

    /// Returns the standard error (Monte Carlo only).
    pub fn standard_error(&self) -> Option<f64> {
        match &self.metadata {
            Some(PricingMetadata::MonteCarlo { standard_error, .. }) => Some(*standard_error),
            _ => None,
        }
    }

    /// Returns the number of paths (Monte Carlo only).
    pub fn num_paths(&self) -> Option<usize> {
        match &self.metadata {
            Some(PricingMetadata::MonteCarlo { num_paths, .. }) => Some(*num_paths),
            _ => None,
        }
    }

    /// Returns the number of steps (Tree only).
    pub fn num_steps(&self) -> Option<usize> {
        match &self.metadata {
            Some(PricingMetadata::Tree { num_steps, .. }) => Some(*num_steps),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_pricing_result_new() {
        let result = UnifiedPricingResult::new(10.5, PricingMethod::Analytical, 1000);

        assert!((result.pv - 10.5).abs() < 1e-10);
        assert_eq!(result.method, PricingMethod::Analytical);
        assert_eq!(result.computation_time_ns, 1000);
        assert!(result.greeks.is_none());
        assert!(result.metadata.is_none());
    }

    #[test]
    fn test_unified_pricing_result_with_greeks() {
        let greeks = UnifiedGreeks::from_delta_gamma(0.55, 0.02);
        let result = UnifiedPricingResult::new(10.5, PricingMethod::Tree, 1000).with_greeks(greeks);

        assert!(result.has_greeks());
        assert!((result.delta().unwrap() - 0.55).abs() < 1e-10);
        assert!((result.gamma().unwrap() - 0.02).abs() < 1e-10);
        assert!(result.vega().is_none());
    }

    #[test]
    fn test_unified_pricing_result_with_mc_metadata() {
        let metadata = PricingMetadata::MonteCarlo {
            num_paths: 100_000,
            standard_error: 0.05,
        };
        let result = UnifiedPricingResult::new(10.5, PricingMethod::MonteCarlo, 5000)
            .with_metadata(metadata);

        assert_eq!(result.num_paths(), Some(100_000));
        assert!((result.standard_error().unwrap() - 0.05).abs() < 1e-10);
        assert!(result.num_steps().is_none());
    }

    #[test]
    fn test_unified_pricing_result_with_tree_metadata() {
        let metadata = PricingMetadata::Tree {
            num_steps: 500,
            tree_type: TreeTypeMetadata::Binomial,
        };
        let result =
            UnifiedPricingResult::new(10.5, PricingMethod::Tree, 2000).with_metadata(metadata);

        assert_eq!(result.num_steps(), Some(500));
        assert!(result.standard_error().is_none());
    }

    #[test]
    fn test_unified_greeks_default() {
        let greeks = UnifiedGreeks::default();
        assert!(greeks.delta.is_none());
        assert!(greeks.gamma.is_none());
        assert!(greeks.vega.is_none());
        assert!(greeks.theta.is_none());
        assert!(greeks.rho.is_none());
        assert!(!greeks.has_any());
    }

    #[test]
    fn test_unified_greeks_from_delta_gamma() {
        let greeks = UnifiedGreeks::from_delta_gamma(0.6, 0.01);
        assert!(greeks.delta.is_some());
        assert!(greeks.gamma.is_some());
        assert!(greeks.vega.is_none());
        assert!(greeks.has_any());
    }

    #[test]
    fn test_unified_greeks_new() {
        let greeks = UnifiedGreeks::new(Some(0.55), Some(0.02), Some(15.0), Some(-5.0), Some(10.0));

        assert!((greeks.delta.unwrap() - 0.55).abs() < 1e-10);
        assert!((greeks.gamma.unwrap() - 0.02).abs() < 1e-10);
        assert!((greeks.vega.unwrap() - 15.0).abs() < 1e-10);
        assert!((greeks.theta.unwrap() - (-5.0)).abs() < 1e-10);
        assert!((greeks.rho.unwrap() - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_pricing_metadata_discount() {
        let metadata = PricingMetadata::Discount {
            model: "Black-Scholes".to_string(),
        };

        match metadata {
            PricingMetadata::Discount { model } => {
                assert_eq!(model, "Black-Scholes");
            }
            _ => panic!("Expected Discount metadata"),
        }
    }

    #[test]
    fn test_tree_type_metadata() {
        let binomial = TreeTypeMetadata::Binomial;
        let trinomial = TreeTypeMetadata::Trinomial;

        assert_ne!(binomial, trinomial);
        assert_eq!(binomial, TreeTypeMetadata::Binomial);
    }
}
