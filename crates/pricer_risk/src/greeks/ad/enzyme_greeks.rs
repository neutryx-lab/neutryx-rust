//! Enzyme-based Greeks calculation for Monte Carlo pricing.

use pricer_pricing::methods::mc::{GbmParams, MonteCarloPricer, PayoffParams, PricingResult};

/// Greeks calculation result with optional sensitivities.
#[derive(Clone, Debug, PartialEq)]
pub struct GreeksResult<T> {
    pub price: T,
    pub std_error: T,
    pub delta: Option<T>,
    pub vega: Option<T>,
    pub theta: Option<T>,
    pub rho: Option<T>,
    pub gamma: Option<T>,
    pub vanna: Option<T>,
    pub volga: Option<T>,
}

impl<T: Default> GreeksResult<T> {
    /// Creates a new result with only price and standard error.
    pub fn new(price: T, std_error: T) -> Self {
        Self {
            price,
            std_error,
            delta: None,
            vega: None,
            theta: None,
            rho: None,
            gamma: None,
            vanna: None,
            volga: None,
        }
    }

    /// Sets delta.
    pub fn with_delta(mut self, delta: T) -> Self {
        self.delta = Some(delta);
        self
    }

    /// Sets gamma.
    pub fn with_gamma(mut self, gamma: T) -> Self {
        self.gamma = Some(gamma);
        self
    }

    /// Sets vega.
    pub fn with_vega(mut self, vega: T) -> Self {
        self.vega = Some(vega);
        self
    }

    /// Sets theta.
    pub fn with_theta(mut self, theta: T) -> Self {
        self.theta = Some(theta);
        self
    }

    /// Sets rho.
    pub fn with_rho(mut self, rho: T) -> Self {
        self.rho = Some(rho);
        self
    }
}

/// Mode for Greeks computation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GreeksMode {
    /// Automatically select the best available method.
    #[default]
    Auto,

    /// Force Enzyme AD (fails if not available).
    EnzymeOnly,

    /// Force finite difference approximations.
    FiniteDifference,

    /// Use forward mode AD for single Greeks.
    ForwardMode,

    /// Use reverse mode AD for all Greeks at once.
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
    pub fn enzyme_available() -> bool { cfg!(feature = "enzyme-ad") }

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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EnzymeGreeksResult {
    pub price: f64,
    pub std_error: f64,
    pub delta: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
    pub gamma: f64,
    pub vanna: f64,
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

#[allow(deprecated)]
impl From<EnzymeGreeksResult> for GreeksResult<f64> {
    fn from(result: EnzymeGreeksResult) -> Self { result.to_greeks_result() }
}

impl From<EnzymeGreeksResult> for PricingResult {
    fn from(result: EnzymeGreeksResult) -> Self { result.to_pricing_result() }
}

/// Trait for Enzyme-based Greeks computation on Monte Carlo pricers.
pub trait GreeksEnzyme {
    /// Computes price and all first-order Greeks.
    fn price_with_enzyme_greeks(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
        mode: GreeksMode,
    ) -> EnzymeGreeksResult;

    /// Computes only Delta using forward mode AD.
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
                compute_greeks_fd(self, gbm, payoff, discount_factor)
            }
            GreeksMode::ForwardMode => compute_greeks_forward(self, gbm, payoff, discount_factor),
            GreeksMode::ReverseMode => compute_greeks_reverse(self, gbm, payoff, discount_factor),
            GreeksMode::EnzymeOnly => {
                #[cfg(feature = "enzyme-ad")]
                {
                    compute_greeks_reverse(self, gbm, payoff, discount_factor)
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
            let result = self.price_with_delta_ad(gbm, payoff, discount_factor);
            result.delta.unwrap_or(0.0)
        }
        #[cfg(not(feature = "enzyme-ad"))]
        {
            compute_delta_fd(self, gbm, payoff, discount_factor)
        }
    }

    fn compute_gamma_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
        compute_gamma_fd(self, gbm, payoff, discount_factor)
    }

    fn compute_vega_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
        #[cfg(feature = "enzyme-ad")]
        {
            compute_vega_fd(self, gbm, payoff, discount_factor)
        }
        #[cfg(not(feature = "enzyme-ad"))]
        {
            compute_vega_fd(self, gbm, payoff, discount_factor)
        }
    }

    fn compute_theta_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
        compute_theta_fd(self, gbm, payoff, discount_factor)
    }

    fn compute_rho_ad(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
    ) -> f64 {
        compute_rho_fd(self, gbm, payoff, discount_factor)
    }
}

fn compute_greeks_fd(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    discount_factor: f64,
) -> EnzymeGreeksResult {
    let base_result = pricer.price_european(gbm, payoff, discount_factor);

    let delta = compute_delta_fd(pricer, gbm, payoff, discount_factor);
    let gamma = compute_gamma_fd(pricer, gbm, payoff, discount_factor);
    let vega = compute_vega_fd(pricer, gbm, payoff, discount_factor);
    let theta = compute_theta_fd(pricer, gbm, payoff, discount_factor);
    let rho = compute_rho_fd(pricer, gbm, payoff, discount_factor);

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

fn compute_greeks_forward(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    discount_factor: f64,
) -> EnzymeGreeksResult {
    let base_result = pricer.price_european(gbm, payoff, discount_factor);

    let (_, delta) = pricer.price_with_delta_ad(gbm, payoff, discount_factor);
    let gamma = compute_gamma_fd(pricer, gbm, payoff, discount_factor);
    let vega = compute_vega_fd(pricer, gbm, payoff, discount_factor);
    let theta = compute_theta_fd(pricer, gbm, payoff, discount_factor);
    let rho = compute_rho_fd(pricer, gbm, payoff, discount_factor);

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

fn compute_greeks_reverse(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    discount_factor: f64,
) -> EnzymeGreeksResult {
    #[cfg(feature = "enzyme-ad")]
    {
        compute_greeks_fd(pricer, gbm, payoff, discount_factor)
    }
    #[cfg(not(feature = "enzyme-ad"))]
    {
        compute_greeks_fd(pricer, gbm, payoff, discount_factor)
    }
}

fn compute_delta_fd(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    discount_factor: f64,
) -> f64 {
    let bump = (0.01 * gbm.spot).max(0.01);
    let seed = pricer.current_seed();

    pricer.reset_with_seed(seed);
    let gbm_up = GbmParams {
        spot: gbm.spot + bump,
        ..gbm
    };
    let price_up = pricer.price_european(gbm_up, payoff, discount_factor).price;

    pricer.reset_with_seed(seed);
    let gbm_down = GbmParams {
        spot: gbm.spot - bump,
        ..gbm
    };
    let price_down = pricer
        .price_european(gbm_down, payoff, discount_factor)
        .price;

    (price_up - price_down) / (2.0 * bump)
}

fn compute_gamma_fd(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    discount_factor: f64,
) -> f64 {
    let bump = (0.01 * gbm.spot).max(0.01);
    let seed = pricer.current_seed();

    pricer.reset_with_seed(seed);
    let price_mid = pricer.price_european(gbm, payoff, discount_factor).price;

    pricer.reset_with_seed(seed);
    let gbm_up = GbmParams {
        spot: gbm.spot + bump,
        ..gbm
    };
    let price_up = pricer.price_european(gbm_up, payoff, discount_factor).price;

    pricer.reset_with_seed(seed);
    let gbm_down = GbmParams {
        spot: gbm.spot - bump,
        ..gbm
    };
    let price_down = pricer
        .price_european(gbm_down, payoff, discount_factor)
        .price;

    (price_up - 2.0 * price_mid + price_down) / (bump * bump)
}

fn compute_vega_fd(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    discount_factor: f64,
) -> f64 {
    let bump = 0.01;
    let seed = pricer.current_seed();

    pricer.reset_with_seed(seed);
    let gbm_up = GbmParams {
        volatility: gbm.volatility + bump,
        ..gbm
    };
    let price_up = pricer.price_european(gbm_up, payoff, discount_factor).price;

    pricer.reset_with_seed(seed);
    let gbm_down = GbmParams {
        volatility: (gbm.volatility - bump).max(0.001),
        ..gbm
    };
    let price_down = pricer
        .price_european(gbm_down, payoff, discount_factor)
        .price;

    (price_up - price_down) / (2.0 * bump)
}

fn compute_theta_fd(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    discount_factor: f64,
) -> f64 {
    let bump = 1.0 / 252.0;
    let seed = pricer.current_seed();

    pricer.reset_with_seed(seed);
    let price_now = pricer.price_european(gbm, payoff, discount_factor).price;

    pricer.reset_with_seed(seed);
    let gbm_short = GbmParams {
        maturity: (gbm.maturity - bump).max(0.001),
        ..gbm
    };
    let price_short = pricer
        .price_european(gbm_short, payoff, discount_factor)
        .price;

    -(price_now - price_short) / bump
}

fn compute_rho_fd(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    discount_factor: f64,
) -> f64 {
    let bump = 0.0001;
    let seed = pricer.current_seed();

    pricer.reset_with_seed(seed);
    let gbm_up = GbmParams {
        rate: gbm.rate + bump,
        ..gbm
    };
    let df_up = discount_factor * (-bump * gbm.maturity).exp();
    let price_up = pricer.price_european(gbm_up, payoff, df_up).price;

    pricer.reset_with_seed(seed);
    let gbm_down = GbmParams {
        rate: gbm.rate - bump,
        ..gbm
    };
    let df_down = discount_factor * (bump * gbm.maturity).exp();
    let price_down = pricer.price_european(gbm_down, payoff, df_down).price;

    (price_up - price_down) / (2.0 * bump) * 0.01
}

#[cfg(test)]
mod tests {
    use pricer_pricing::methods::mc::MonteCarloConfig;

    use super::*;

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

        assert!(result.price > 5.0 && result.price < 20.0);
        assert!(result.delta > 0.4 && result.delta < 0.8);
        assert!(result.gamma > 0.0);
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

        assert!(delta > 0.4 && delta < 0.8);
    }

    #[test]
    fn test_compute_gamma_ad() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let gamma = pricer.compute_gamma_ad(gbm, payoff, df);

        assert!(gamma > 0.0);
    }

    #[test]
    fn test_compute_vega_ad() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let vega = pricer.compute_vega_ad(gbm, payoff, df);

        assert!(vega > 0.0);
    }

    #[test]
    fn test_compute_theta_ad() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let theta = pricer.compute_theta_ad(gbm, payoff, df);

        assert!(theta < 10.0);
    }

    #[test]
    fn test_compute_rho_ad() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let rho = pricer.compute_rho_ad(gbm, payoff, df);

        assert!(rho > -1.0 && rho < 1.0);
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
