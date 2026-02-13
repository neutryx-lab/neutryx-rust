//! Unified calculation settings for the pricing engine.
//!
//! [`CalcSetting`] controls **how** a trade is priced: which numerical method
//! to use, whether to compute Greeks, what reporting currency to apply, and
//! optional method-specific configuration (Monte Carlo paths, tree steps,
//! etc.).

use bon::Builder;
use infra_domain::market::Currency;

// GreeksMode is re-exported via generic_pricer.
pub use crate::generic_pricer::GreeksMode;
// Re-exported at the crate root so we use short paths.
use crate::methods::tree::TreeType;

// ---------------------------------------------------------------------------
// Pricing method hint
// ---------------------------------------------------------------------------

/// Hint to the engine about which numerical method to use.
///
/// `Auto` lets the pricer choose the most appropriate method based on the
/// trade type and market data available.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PricingMethodHint {
    /// Pricer decides based on the trade type.
    #[default]
    Auto,
    /// Closed-form or cashflow discounting.
    Analytical,
    /// Path simulation (Monte Carlo).
    MonteCarlo,
    /// Binomial / trinomial tree.
    Tree,
}

// ---------------------------------------------------------------------------
// Method-specific settings
// ---------------------------------------------------------------------------

/// Lightweight Monte Carlo configuration passed by the caller.
///
/// This is intentionally simpler than the engine-internal [`MonteCarloConfig`]
/// which carries layout and streaming details the caller need not know about.
///
/// [`MonteCarloConfig`]: crate::methods::mc::MonteCarloConfig
#[derive(Clone, Debug, PartialEq, Eq, Builder)]
pub struct MonteCarloSetting {
    /// Number of simulation paths (default: 10 000).
    #[builder(default = 10_000)]
    pub num_paths: usize,
    /// Number of time steps per path (default: 100).
    #[builder(default = 100)]
    pub num_steps: usize,
    /// Optional seed for reproducibility.
    pub seed: Option<u64>,
}

impl Default for MonteCarloSetting {
    fn default() -> Self {
        Self {
            num_paths: 10_000,
            num_steps: 100,
            seed: None,
        }
    }
}

/// Lightweight tree configuration passed by the caller.
#[derive(Clone, Debug, PartialEq, Eq, Builder)]
pub struct TreeSetting {
    /// Number of time steps in the tree (default: 100).
    #[builder(default = 100)]
    pub num_steps: usize,
    /// Tree type: Binomial or Trinomial (default: Binomial).
    #[builder(default)]
    pub tree_type: TreeType,
}

impl Default for TreeSetting {
    fn default() -> Self {
        Self {
            num_steps: 100,
            tree_type: TreeType::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// CalcSetting
// ---------------------------------------------------------------------------

/// Unified calculation settings that accompany every pricing request.
///
/// Use [`CalcSetting::builder()`] for ergonomic construction or
/// [`CalcSetting::default()`] for sensible defaults (Auto method, no Greeks,
/// USD reporting currency).
#[derive(Clone, Debug, Builder)]
pub struct CalcSetting {
    /// Hint for the numerical method to use.
    #[builder(default)]
    pub method: PricingMethodHint,

    /// Whether the engine should compute Greeks alongside the PV.
    #[builder(default)]
    pub compute_greeks: bool,

    /// Reporting (output) currency for the result.
    #[builder(default = Currency::USD)]
    pub reporting_currency: Currency,

    /// Optional Monte Carlo configuration (used when `method` is
    /// [`PricingMethodHint::MonteCarlo`] or `Auto` resolves to MC).
    pub mc_config: Option<MonteCarloSetting>,

    /// Optional tree configuration (used when `method` is
    /// [`PricingMethodHint::Tree`] or `Auto` resolves to a tree method).
    pub tree_config: Option<TreeSetting>,

    /// How Greeks are calculated (bump-and-revalue vs Enzyme AAD).
    #[builder(default)]
    pub greeks_mode: GreeksMode,
}

impl Default for CalcSetting {
    fn default() -> Self {
        Self {
            method: PricingMethodHint::Auto,
            compute_greeks: false,
            reporting_currency: Currency::USD,
            mc_config: None,
            tree_config: None,
            greeks_mode: GreeksMode::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_has_sensible_values() {
        let cs = CalcSetting::default();
        assert_eq!(cs.method, PricingMethodHint::Auto);
        assert!(!cs.compute_greeks);
        assert_eq!(cs.reporting_currency, Currency::USD);
        assert!(cs.mc_config.is_none());
        assert!(cs.tree_config.is_none());
        assert_eq!(cs.greeks_mode, GreeksMode::BumpRevalue);
    }

    #[test]
    fn test_builder_defaults_match_default_impl() {
        let from_builder = CalcSetting::builder().build();
        let from_default = CalcSetting::default();

        assert_eq!(from_builder.method, from_default.method);
        assert_eq!(from_builder.compute_greeks, from_default.compute_greeks);
        assert_eq!(
            from_builder.reporting_currency,
            from_default.reporting_currency
        );
        assert_eq!(from_builder.mc_config, from_default.mc_config);
        assert_eq!(from_builder.tree_config, from_default.tree_config);
        assert_eq!(from_builder.greeks_mode, from_default.greeks_mode);
    }

    #[test]
    fn test_builder_custom_values() {
        let cs = CalcSetting::builder()
            .reporting_currency(Currency::EUR)
            .compute_greeks(true)
            .method(PricingMethodHint::MonteCarlo)
            .mc_config(MonteCarloSetting {
                num_paths: 50_000,
                num_steps: 252,
                seed: Some(42),
            })
            .build();

        assert_eq!(cs.method, PricingMethodHint::MonteCarlo);
        assert!(cs.compute_greeks);
        assert_eq!(cs.reporting_currency, Currency::EUR);
        let mc = cs.mc_config.unwrap();
        assert_eq!(mc.num_paths, 50_000);
        assert_eq!(mc.num_steps, 252);
        assert_eq!(mc.seed, Some(42));
    }

    #[test]
    fn test_builder_tree_config() {
        let cs = CalcSetting::builder()
            .method(PricingMethodHint::Tree)
            .tree_config(TreeSetting {
                num_steps: 500,
                tree_type: TreeType::Trinomial,
            })
            .build();

        assert_eq!(cs.method, PricingMethodHint::Tree);
        let tree = cs.tree_config.unwrap();
        assert_eq!(tree.num_steps, 500);
        assert_eq!(tree.tree_type, TreeType::Trinomial);
    }

    #[test]
    fn test_builder_analytical() {
        let cs = CalcSetting::builder()
            .method(PricingMethodHint::Analytical)
            .compute_greeks(true)
            .reporting_currency(Currency::GBP)
            .build();

        assert_eq!(cs.method, PricingMethodHint::Analytical);
        assert!(cs.compute_greeks);
        assert_eq!(cs.reporting_currency, Currency::GBP);
        assert!(cs.mc_config.is_none());
        assert!(cs.tree_config.is_none());
    }

    #[test]
    fn test_monte_carlo_setting_default() {
        let mc = MonteCarloSetting::default();
        assert_eq!(mc.num_paths, 10_000);
        assert_eq!(mc.num_steps, 100);
        assert!(mc.seed.is_none());
    }

    #[test]
    fn test_monte_carlo_setting_builder() {
        let mc = MonteCarloSetting::builder()
            .num_paths(100_000)
            .num_steps(365)
            .seed(123)
            .build();
        assert_eq!(mc.num_paths, 100_000);
        assert_eq!(mc.num_steps, 365);
        assert_eq!(mc.seed, Some(123));
    }

    #[test]
    fn test_tree_setting_default() {
        let ts = TreeSetting::default();
        assert_eq!(ts.num_steps, 100);
        assert_eq!(ts.tree_type, TreeType::Binomial);
    }

    #[test]
    fn test_tree_setting_builder() {
        let ts = TreeSetting::builder()
            .num_steps(1000)
            .tree_type(TreeType::Trinomial)
            .build();
        assert_eq!(ts.num_steps, 1000);
        assert_eq!(ts.tree_type, TreeType::Trinomial);
    }

    #[test]
    fn test_pricing_method_hint_default() {
        assert_eq!(PricingMethodHint::default(), PricingMethodHint::Auto);
    }

    #[test]
    fn test_calc_setting_clone() {
        let original = CalcSetting::builder()
            .method(PricingMethodHint::MonteCarlo)
            .compute_greeks(true)
            .mc_config(MonteCarloSetting::default())
            .build();
        let cloned = original.clone();
        assert_eq!(cloned.method, original.method);
        assert_eq!(cloned.compute_greeks, original.compute_greeks);
        assert_eq!(cloned.mc_config, original.mc_config);
    }
}
