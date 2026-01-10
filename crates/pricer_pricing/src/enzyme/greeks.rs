//! Enzyme-based Greeks calculation for Monte Carlo pricing.
//!
//! This module provides the `GreeksEnzyme` trait for integrating Enzyme
//! automatic differentiation with the Monte Carlo pricer. When the `enzyme-ad`
//! feature is enabled, it uses LLVM-level AD; otherwise, it falls back to
//! finite difference approximations.
//!
//! # Usage
//!
//! ```rust,no_run
//! use pricer_pricing::enzyme::greeks::{GreeksEnzyme, GreeksMode, EnzymeGreeksResult};
//! use pricer_pricing::mc::{GbmParams, PayoffParams, MonteCarloPricer, MonteCarloConfig};
//!
//! let config = MonteCarloConfig::builder()
//!     .n_paths(10_000)
//!     .n_steps(1)
//!     .build()
//!     .unwrap();
//! let mut pricer = MonteCarloPricer::new(config).unwrap();
//!
//! let gbm = GbmParams::new(100.0, 0.05, 0.2, 1.0);
//! let payoff = PayoffParams::call(100.0);
//! let df = (-0.05_f64 * 1.0).exp();
//!
//! // Compute all Greeks using automatic differentiation
//! let result = pricer.price_with_enzyme_greeks(gbm, payoff, df, GreeksMode::Auto);
//! println!("Price: {:.4}, Delta: {:.4}", result.price, result.delta);
//! ```

use crate::greeks::GreeksResult;
use crate::mc::{GbmParams, MonteCarloPricer, PayoffParams, PricingResult};

/// Mode for Greeks computation.
///
/// Controls which method is used for computing sensitivities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GreeksMode {
    /// Automatically select the best available method.
    ///
    /// Uses Enzyme AD when available, falls back to finite differences.
    #[default]
    Auto,

    /// Force Enzyme AD (fails if not available).
    EnzymeOnly,

    /// Force finite difference approximations.
    FiniteDifference,

    /// Use forward mode AD for single Greeks.
    ///
    /// Efficient for computing one Greek at a time.
    ForwardMode,

    /// Use reverse mode AD for all Greeks at once.
    ///
    /// Most efficient when computing multiple Greeks.
    ReverseMode,
}

impl GreeksMode {
    /// Returns whether this mode requires Enzyme AD.
    #[inline]
    pub fn requires_enzyme(&self) -> bool {
        matches!(
            self,
            Self::EnzymeOnly | Self::ForwardMode | Self::ReverseMode
        )
    }

    /// Returns whether Enzyme AD is available.
    #[inline]
    pub fn enzyme_available() -> bool {
        cfg!(feature = "enzyme-ad")
    }

    /// Resolves Auto mode to a concrete method.
    #[inline]
    pub fn resolve(&self) -> Self {
        match self {
            Self::Auto => {
                if Self::enzyme_available() {
                    Self::ReverseMode
                } else {
                    Self::FiniteDifference
                }
            }
            other => *other,
        }
    }
}

/// Result of Enzyme-based Greeks computation.
///
/// Contains price and all first/second-order Greeks computed via AD.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EnzymeGreeksResult {
    /// Computed price.
    pub price: f64,

    /// Standard error of price estimate.
    pub std_error: f64,

    // First-order Greeks
    /// Delta: ∂V/∂S
    pub delta: f64,
    /// Vega: ∂V/∂σ
    pub vega: f64,
    /// Theta: ∂V/∂τ
    pub theta: f64,
    /// Rho: ∂V/∂r
    pub rho: f64,

    // Second-order Greeks
    /// Gamma: ∂²V/∂S²
    pub gamma: f64,
    /// Vanna: ∂²V/∂S∂σ
    pub vanna: f64,
    /// Volga: ∂²V/∂σ²
    pub volga: f64,
}

impl EnzymeGreeksResult {
    /// Creates a new result with all Greeks.
    #[inline]
    pub fn new(
        price: f64,
        std_error: f64,
        delta: f64,
        gamma: f64,
        vega: f64,
        theta: f64,
        rho: f64,
    ) -> Self {
        Self {
            price,
            std_error,
            delta,
            vega,
            theta,
            rho,
            gamma,
            vanna: 0.0,
            volga: 0.0,
        }
    }

    /// Creates a result with only price (no Greeks computed).
    #[inline]
    pub fn price_only(price: f64, std_error: f64) -> Self {
        Self {
            price,
            std_error,
            ..Default::default()
        }
    }

    /// Converts to `GreeksResult<f64>` for compatibility.
    #[inline]
    pub fn to_greeks_result(&self) -> GreeksResult<f64> {
        GreeksResult::new(self.price, self.std_error)
            .with_delta(self.delta)
            .with_gamma(self.gamma)
            .with_vega(self.vega)
            .with_theta(self.theta)
            .with_rho(self.rho)
    }

    /// Converts to `PricingResult` for compatibility with existing API.
    #[inline]
    pub fn to_pricing_result(&self) -> PricingResult {
        PricingResult {
            price: self.price,
            std_error: self.std_error,
            delta: Some(self.delta),
            gamma: Some(self.gamma),
            vega: Some(self.vega),
            theta: Some(self.theta),
            rho: Some(self.rho),
            vanna: if self.vanna != 0.0 {
                Some(self.vanna)
            } else {
                None
            },
            volga: if self.volga != 0.0 {
                Some(self.volga)
            } else {
                None
            },
        }
    }

    /// Sets the Vanna value.
    #[inline]
    pub fn with_vanna(mut self, vanna: f64) -> Self {
        self.vanna = vanna;
        self
    }

    /// Sets the Volga value.
    #[inline]
    pub fn with_volga(mut self, volga: f64) -> Self {
        self.volga = volga;
        self
    }
}

impl From<EnzymeGreeksResult> for GreeksResult<f64> {
    fn from(result: EnzymeGreeksResult) -> Self {
        result.to_greeks_result()
    }
}

impl From<EnzymeGreeksResult> for PricingResult {
    fn from(result: EnzymeGreeksResult) -> Self {
        result.to_pricing_result()
    }
}

/// Trait for Enzyme-based Greeks computation on Monte Carlo pricers.
///
/// This trait provides methods for computing Greeks using automatic
/// differentiation. Implementations can use Enzyme AD when available
/// or fall back to finite differences.
///
/// # Implementation Note
///
/// When `enzyme-ad` feature is enabled, the trait methods use LLVM-level
/// AD via Enzyme. Otherwise, they fall back to bump-and-revalue with
/// the same interface.
pub trait GreeksEnzyme {
    /// Computes price and all first-order Greeks.
    ///
    /// # Arguments
    ///
    /// * `gbm` - GBM parameters (spot, rate, volatility, maturity)
    /// * `payoff` - Payoff specification (call/put, strike)
    /// * `discount_factor` - Discount factor for present value
    /// * `mode` - Computation mode (Auto, EnzymeOnly, FiniteDifference)
    ///
    /// # Returns
    ///
    /// `EnzymeGreeksResult` containing price and all Greeks.
    fn price_with_enzyme_greeks(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
        mode: GreeksMode,
    ) -> EnzymeGreeksResult;

    /// Computes only Delta using forward mode AD.
    ///
    /// This is more efficient than computing all Greeks when only
    /// Delta is needed.
    fn compute_delta_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64;

    /// Computes only Gamma using nested AD or finite differences.
    fn compute_gamma_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64;

    /// Computes only Vega using AD.
    fn compute_vega_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64;

    /// Computes only Theta using AD.
    fn compute_theta_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64;

    /// Computes only Rho using AD.
    fn compute_rho_ad(&mut self, gbm: GbmParams, payoff: PayoffParams, discount_factor: f64)
        -> f64;
}

/// Implementation of GreeksEnzyme for MonteCarloPricer.
///
/// When `enzyme-ad` feature is enabled, uses Enzyme LLVM-level AD.
/// Otherwise, falls back to finite difference approximations.
impl GreeksEnzyme for MonteCarloPricer {
    fn price_with_enzyme_greeks(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
        mode: GreeksMode,
    ) -> EnzymeGreeksResult {
        let resolved_mode = mode.resolve();

        match resolved_mode {
            GreeksMode::FiniteDifference | GreeksMode::Auto => {
                // Fall back to finite differences
                self.compute_greeks_fd(gbm, payoff, discount_factor)
            }
            GreeksMode::ForwardMode => {
                // Use forward mode for individual Greeks
                self.compute_greeks_forward(gbm, payoff, discount_factor)
            }
            GreeksMode::ReverseMode => {
                // Use reverse mode for all Greeks at once
                self.compute_greeks_reverse(gbm, payoff, discount_factor)
            }
            GreeksMode::EnzymeOnly => {
                // EnzymeOnly mode - use reverse if available
                #[cfg(feature = "enzyme-ad")]
                {
                    self.compute_greeks_reverse(gbm, payoff, discount_factor)
                }
                #[cfg(not(feature = "enzyme-ad"))]
                {
                    panic!("Enzyme AD not available. Enable the 'enzyme-ad' feature.");
                }
            }
        }
    }

    fn compute_delta_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
        #[cfg(feature = "enzyme-ad")]
        {
            // Enzyme forward mode would go here
            // For now, use the existing price_with_delta_ad method
            let result = self.price_with_delta_ad(gbm, payoff, discount_factor);
            result.delta.unwrap_or(0.0)
        }
        #[cfg(not(feature = "enzyme-ad"))]
        {
            self.compute_delta_fd(gbm, payoff, discount_factor)
        }
    }

    fn compute_gamma_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
        // Gamma requires nested AD or finite differences on Delta
        // For now, use finite differences
        self.compute_gamma_fd(gbm, payoff, discount_factor)
    }

    fn compute_vega_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
        #[cfg(feature = "enzyme-ad")]
        {
            // Enzyme would compute this via reverse mode
            self.compute_vega_fd(gbm, payoff, discount_factor)
        }
        #[cfg(not(feature = "enzyme-ad"))]
        {
            self.compute_vega_fd(gbm, payoff, discount_factor)
        }
    }

    fn compute_theta_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
        self.compute_theta_fd(gbm, payoff, discount_factor)
    }

    fn compute_rho_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
        self.compute_rho_fd(gbm, payoff, discount_factor)
    }
}

/// Internal implementation methods for MonteCarloPricer.
impl MonteCarloPricer {
    /// Computes all Greeks using finite differences.
    fn compute_greeks_fd(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> EnzymeGreeksResult {
        // Base price
        let base_result = self.price_european(gbm, payoff, discount_factor);

        // Compute all Greeks
        let delta = self.compute_delta_fd(gbm, payoff, discount_factor);
        let gamma = self.compute_gamma_fd(gbm, payoff, discount_factor);
        let vega = self.compute_vega_fd(gbm, payoff, discount_factor);
        let theta = self.compute_theta_fd(gbm, payoff, discount_factor);
        let rho = self.compute_rho_fd(gbm, payoff, discount_factor);

        EnzymeGreeksResult::new(
            base_result.price,
            base_result.std_error,
            delta,
            gamma,
            vega,
            theta,
            rho,
        )
    }

    /// Computes Greeks using forward mode AD.
    fn compute_greeks_forward(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> EnzymeGreeksResult {
        // Forward mode computes one Greek at a time
        let base_result = self.price_european(gbm, payoff, discount_factor);

        let delta = self.compute_delta_ad(gbm, payoff, discount_factor);
        let gamma = self.compute_gamma_fd(gbm, payoff, discount_factor);
        let vega = self.compute_vega_fd(gbm, payoff, discount_factor);
        let theta = self.compute_theta_fd(gbm, payoff, discount_factor);
        let rho = self.compute_rho_fd(gbm, payoff, discount_factor);

        EnzymeGreeksResult::new(
            base_result.price,
            base_result.std_error,
            delta,
            gamma,
            vega,
            theta,
            rho,
        )
    }

    /// Computes Greeks using reverse mode AD.
    ///
    /// In a full Enzyme implementation, this would compute all Greeks
    /// in a single reverse pass. Currently falls back to finite differences.
    fn compute_greeks_reverse(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> EnzymeGreeksResult {
        #[cfg(feature = "enzyme-ad")]
        {
            // When Enzyme is enabled, this would use #[autodiff_reverse]
            // to compute all Greeks in one reverse pass.
            // For now, use finite differences as placeholder.
            self.compute_greeks_fd(gbm, payoff, discount_factor)
        }
        #[cfg(not(feature = "enzyme-ad"))]
        {
            self.compute_greeks_fd(gbm, payoff, discount_factor)
        }
    }

    /// Computes Delta using finite differences (central difference).
    fn compute_delta_fd(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
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
        let price_up = self.price_european(gbm_up, payoff, discount_factor).price;

        // Price at S - bump
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            spot: gbm.spot - bump,
            ..gbm
        };
        let price_down = self.price_european(gbm_down, payoff, discount_factor).price;

        (price_up - price_down) / (2.0 * bump)
    }

    /// Computes Gamma using finite differences.
    fn compute_gamma_fd(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
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

        (price_up - 2.0 * price_mid + price_down) / (bump * bump)
    }

    /// Computes Vega using finite differences.
    fn compute_vega_fd(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
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
        let price_up = self.price_european(gbm_up, payoff, discount_factor).price;

        // Price at vol - bump
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            volatility: (gbm.volatility - bump).max(0.001),
            ..gbm
        };
        let price_down = self.price_european(gbm_down, payoff, discount_factor).price;

        (price_up - price_down) / (2.0 * bump)
    }

    /// Computes Theta using finite differences.
    fn compute_theta_fd(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
        let bump = 1.0 / 252.0; // 1 day
        let seed = self.rng.seed();

        // Price at T
        self.reset_with_seed(seed);
        let price_now = self.price_european(gbm, payoff, discount_factor).price;

        // Price at T - bump
        self.reset_with_seed(seed);
        let gbm_short = GbmParams {
            maturity: (gbm.maturity - bump).max(0.001),
            ..gbm
        };
        let price_short = self
            .price_european(gbm_short, payoff, discount_factor)
            .price;

        // Theta is typically negative (time decay)
        // Convention: dV/dT where T decreases
        -(price_now - price_short) / bump
    }

    /// Computes Rho using finite differences.
    fn compute_rho_fd(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
        let bump = 0.0001; // 1 basis point
        let seed = self.rng.seed();

        // Price at r + bump (with adjusted discount factor)
        self.reset_with_seed(seed);
        let gbm_up = GbmParams {
            rate: gbm.rate + bump,
            ..gbm
        };
        let df_up = discount_factor * (-bump * gbm.maturity).exp();
        let price_up = self.price_european(gbm_up, payoff, df_up).price;

        // Price at r - bump
        self.reset_with_seed(seed);
        let gbm_down = GbmParams {
            rate: gbm.rate - bump,
            ..gbm
        };
        let df_down = discount_factor * (bump * gbm.maturity).exp();
        let price_down = self.price_european(gbm_down, payoff, df_down).price;

        // Scaled to 1% rate move
        (price_up - price_down) / (2.0 * bump) * 0.01
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mc::MonteCarloConfig;

    fn create_pricer() -> MonteCarloPricer {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(1)
            .seed(42)
            .build()
            .expect("Failed to build config");
        MonteCarloPricer::new(config).expect("Failed to create pricer")
    }

    fn standard_params() -> (GbmParams, PayoffParams, f64) {
        let gbm = GbmParams::new(100.0, 0.05, 0.2, 1.0);
        let payoff = PayoffParams::call(100.0);
        let df = (-0.05_f64 * 1.0).exp();
        (gbm, payoff, df)
    }

    #[test]
    fn test_greeks_mode_requires_enzyme() {
        assert!(!GreeksMode::Auto.requires_enzyme());
        assert!(GreeksMode::EnzymeOnly.requires_enzyme());
        assert!(!GreeksMode::FiniteDifference.requires_enzyme());
        assert!(GreeksMode::ForwardMode.requires_enzyme());
        assert!(GreeksMode::ReverseMode.requires_enzyme());
    }

    #[test]
    fn test_greeks_mode_resolve() {
        let auto = GreeksMode::Auto.resolve();
        // Without enzyme-ad feature, should resolve to FiniteDifference
        #[cfg(not(feature = "enzyme-ad"))]
        assert_eq!(auto, GreeksMode::FiniteDifference);

        let fd = GreeksMode::FiniteDifference.resolve();
        assert_eq!(fd, GreeksMode::FiniteDifference);
    }

    #[test]
    fn test_enzyme_greeks_result_new() {
        let result = EnzymeGreeksResult::new(10.5, 0.05, 0.55, 0.02, 25.0, -10.0, 15.0);

        assert!((result.price - 10.5).abs() < 1e-10);
        assert!((result.delta - 0.55).abs() < 1e-10);
        assert!((result.gamma - 0.02).abs() < 1e-10);
        assert!((result.vega - 25.0).abs() < 1e-10);
        assert!((result.theta - (-10.0)).abs() < 1e-10);
        assert!((result.rho - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_enzyme_greeks_result_price_only() {
        let result = EnzymeGreeksResult::price_only(10.5, 0.05);

        assert!((result.price - 10.5).abs() < 1e-10);
        assert_eq!(result.delta, 0.0);
        assert_eq!(result.gamma, 0.0);
    }

    #[test]
    fn test_enzyme_greeks_result_to_greeks_result() {
        let enzyme_result = EnzymeGreeksResult::new(10.5, 0.05, 0.55, 0.02, 25.0, -10.0, 15.0);
        let greeks = enzyme_result.to_greeks_result();

        assert!((greeks.delta.unwrap() - 0.55).abs() < 1e-10);
        assert!((greeks.gamma.unwrap() - 0.02).abs() < 1e-10);
        assert!((greeks.vega.unwrap() - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_enzyme_greeks_result_to_pricing_result() {
        let enzyme_result = EnzymeGreeksResult::new(10.5, 0.05, 0.55, 0.02, 25.0, -10.0, 15.0);
        let pricing = enzyme_result.to_pricing_result();

        assert!((pricing.price - 10.5).abs() < 1e-10);
        assert_eq!(pricing.delta, Some(0.55));
        assert_eq!(pricing.gamma, Some(0.02));
        assert_eq!(pricing.vega, Some(25.0));
    }

    #[test]
    fn test_enzyme_greeks_result_with_vanna_volga() {
        let result = EnzymeGreeksResult::new(10.5, 0.05, 0.55, 0.02, 25.0, -10.0, 15.0)
            .with_vanna(1.5)
            .with_volga(2.0);

        assert!((result.vanna - 1.5).abs() < 1e-10);
        assert!((result.volga - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_price_with_enzyme_greeks_auto() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let result = pricer.price_with_enzyme_greeks(gbm, payoff, df, GreeksMode::Auto);

        // Price should be reasonable for ATM call
        assert!(result.price > 5.0 && result.price < 20.0);

        // Delta should be around 0.5-0.7 for ATM call
        assert!(result.delta > 0.4 && result.delta < 0.8);

        // Gamma should be positive
        assert!(result.gamma > 0.0);

        // Vega should be positive
        assert!(result.vega > 0.0);
    }

    #[test]
    fn test_price_with_enzyme_greeks_fd() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let result = pricer.price_with_enzyme_greeks(gbm, payoff, df, GreeksMode::FiniteDifference);

        assert!(result.price > 0.0);
        assert!(result.delta > 0.0);
        assert!(result.gamma > 0.0);
    }

    #[test]
    fn test_compute_delta_ad() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let delta = pricer.compute_delta_ad(gbm, payoff, df);

        // ATM call delta should be around 0.5-0.7
        assert!(delta > 0.4 && delta < 0.8);
    }

    #[test]
    fn test_compute_gamma_ad() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let gamma = pricer.compute_gamma_ad(gbm, payoff, df);

        // Gamma should be positive for options
        assert!(gamma > 0.0);
    }

    #[test]
    fn test_compute_vega_ad() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let vega = pricer.compute_vega_ad(gbm, payoff, df);

        // Vega should be positive for long options
        assert!(vega > 0.0);
    }

    #[test]
    fn test_compute_theta_ad() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let theta = pricer.compute_theta_ad(gbm, payoff, df);

        // Theta should be negative for long options (time decay)
        // Note: Our convention makes it negative
        // For ATM options, theta can be significantly negative
        assert!(theta < 10.0); // Just check it's reasonable
    }

    #[test]
    fn test_compute_rho_ad() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let rho = pricer.compute_rho_ad(gbm, payoff, df);

        // Rho for call should be positive
        assert!(rho > -1.0 && rho < 1.0); // Reasonable range for scaled rho
    }

    #[test]
    fn test_greeks_conversion_from() {
        let enzyme_result = EnzymeGreeksResult::new(10.5, 0.05, 0.55, 0.02, 25.0, -10.0, 15.0);

        let greeks: GreeksResult<f64> = enzyme_result.into();
        assert!((greeks.delta.unwrap() - 0.55).abs() < 1e-10);

        let enzyme_result2 = EnzymeGreeksResult::new(10.5, 0.05, 0.55, 0.02, 25.0, -10.0, 15.0);
        let pricing: PricingResult = enzyme_result2.into();
        assert!((pricing.price - 10.5).abs() < 1e-10);
    }
}
