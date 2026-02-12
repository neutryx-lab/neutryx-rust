//! Enzyme-based Greeks calculation for Monte Carlo pricing.

use pricer_pricing::methods::mc::{GbmParams, MonteCarloPricer, PayoffParams};

use crate::greeks::GreeksResult;

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

/// Trait for Enzyme-based Greeks computation on Monte Carlo pricers.
pub trait GreeksEnzyme {
    /// Computes price and all first-order Greeks.
    fn price_with_enzyme_greeks(
        &mut self,
        gbm: GbmParams,
        payoff: PayoffParams,
        discount_factor: f64,
        mode: GreeksMode,
    ) -> GreeksResult<f64>;

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
    ) -> GreeksResult<f64> {
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
            let (_, delta) = self.price_with_delta_ad(gbm, payoff, discount_factor);
            delta
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
) -> GreeksResult<f64> {
    let base_result = pricer.price_european(gbm, payoff, discount_factor);

    let delta = compute_delta_fd(pricer, gbm, payoff, discount_factor);
    let gamma = compute_gamma_fd(pricer, gbm, payoff, discount_factor);
    let vega = compute_vega_fd(pricer, gbm, payoff, discount_factor);
    let theta = compute_theta_fd(pricer, gbm, payoff, discount_factor);
    let rho = compute_rho_fd(pricer, gbm, payoff, discount_factor);

    GreeksResult::new(base_result.price, base_result.std_error)
        .with_delta(delta)
        .with_gamma(gamma)
        .with_vega(vega)
        .with_theta(theta)
        .with_rho(rho)
}

fn compute_greeks_forward(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    discount_factor: f64,
) -> GreeksResult<f64> {
    let base_result = pricer.price_european(gbm, payoff, discount_factor);

    let (_, delta) = pricer.price_with_delta_ad(gbm, payoff, discount_factor);
    let gamma = compute_gamma_fd(pricer, gbm, payoff, discount_factor);
    let vega = compute_vega_fd(pricer, gbm, payoff, discount_factor);
    let theta = compute_theta_fd(pricer, gbm, payoff, discount_factor);
    let rho = compute_rho_fd(pricer, gbm, payoff, discount_factor);

    GreeksResult::new(base_result.price, base_result.std_error)
        .with_delta(delta)
        .with_gamma(gamma)
        .with_vega(vega)
        .with_theta(theta)
        .with_rho(rho)
}

fn compute_greeks_reverse(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    discount_factor: f64,
) -> GreeksResult<f64> {
    #[cfg(feature = "enzyme-ad")]
    {
        compute_greeks_fd(pricer, gbm, payoff, discount_factor)
    }
    #[cfg(not(feature = "enzyme-ad"))]
    {
        compute_greeks_fd(pricer, gbm, payoff, discount_factor)
    }
}

/// Central difference: `(price(up) - price(down)) / divisor`.
/// Automatically saves and restores the pricer seed for each leg.
fn fd_central(
    pricer: &mut MonteCarloPricer,
    payoff: PayoffParams,
    up: (GbmParams, f64),
    down: (GbmParams, f64),
    divisor: f64,
) -> f64 {
    let seed = pricer.current_seed();
    pricer.reset_with_seed(seed);
    let price_up = pricer.price_european(up.0, payoff, up.1).price;
    pricer.reset_with_seed(seed);
    let price_down = pricer.price_european(down.0, payoff, down.1).price;
    (price_up - price_down) / divisor
}

fn compute_delta_fd(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    df: f64,
) -> f64 {
    let h = (0.01 * gbm.spot).max(0.01);
    fd_central(
        pricer,
        payoff,
        (GbmParams { spot: gbm.spot + h, ..gbm }, df),
        (GbmParams { spot: gbm.spot - h, ..gbm }, df),
        2.0 * h,
    )
}

fn compute_gamma_fd(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    df: f64,
) -> f64 {
    let h = (0.01 * gbm.spot).max(0.01);
    let seed = pricer.current_seed();

    pricer.reset_with_seed(seed);
    let price_mid = pricer.price_european(gbm, payoff, df).price;
    pricer.reset_with_seed(seed);
    let price_up = pricer
        .price_european(GbmParams { spot: gbm.spot + h, ..gbm }, payoff, df)
        .price;
    pricer.reset_with_seed(seed);
    let price_down = pricer
        .price_european(GbmParams { spot: gbm.spot - h, ..gbm }, payoff, df)
        .price;

    (price_up - 2.0 * price_mid + price_down) / (h * h)
}

fn compute_vega_fd(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    df: f64,
) -> f64 {
    let h = 0.01;
    fd_central(
        pricer,
        payoff,
        (GbmParams { volatility: gbm.volatility + h, ..gbm }, df),
        (GbmParams { volatility: (gbm.volatility - h).max(0.001), ..gbm }, df),
        2.0 * h,
    )
}

fn compute_theta_fd(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    df: f64,
) -> f64 {
    let h = 1.0 / 252.0;
    let seed = pricer.current_seed();

    pricer.reset_with_seed(seed);
    let price_now = pricer.price_european(gbm, payoff, df).price;
    pricer.reset_with_seed(seed);
    let price_short = pricer
        .price_european(GbmParams { maturity: (gbm.maturity - h).max(0.001), ..gbm }, payoff, df)
        .price;

    -(price_now - price_short) / h
}

fn compute_rho_fd(
    pricer: &mut MonteCarloPricer,
    gbm: GbmParams,
    payoff: PayoffParams,
    df: f64,
) -> f64 {
    let h = 0.0001;
    fd_central(
        pricer,
        payoff,
        (GbmParams { rate: gbm.rate + h, ..gbm }, df * (-h * gbm.maturity).exp()),
        (GbmParams { rate: gbm.rate - h, ..gbm }, df * (h * gbm.maturity).exp()),
        2.0 * h,
    ) * 0.01
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
    fn test_greeks_result_builder() {
        let result = GreeksResult::new(10.5, 0.05)
            .with_delta(0.55)
            .with_gamma(0.02)
            .with_vega(25.0)
            .with_theta(-10.0)
            .with_rho(15.0)
            .with_vanna(1.5)
            .with_volga(2.0);

        assert!((result.price - 10.5).abs() < 1e-10);
        assert!((result.delta.unwrap() - 0.55).abs() < 1e-10);
        assert!((result.gamma.unwrap() - 0.02).abs() < 1e-10);
        assert!((result.vega.unwrap() - 25.0).abs() < 1e-10);
        assert!((result.theta.unwrap() - (-10.0)).abs() < 1e-10);
        assert!((result.rho.unwrap() - 15.0).abs() < 1e-10);
        assert!((result.vanna.unwrap() - 1.5).abs() < 1e-10);
        assert!((result.volga.unwrap() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_price_with_enzyme_greeks_auto() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let result = pricer.price_with_enzyme_greeks(gbm, payoff, df, GreeksMode::Auto);

        assert!(result.price > 5.0 && result.price < 20.0);
        assert!(result.delta.unwrap() > 0.4 && result.delta.unwrap() < 0.8);
        assert!(result.gamma.unwrap() > 0.0);
        assert!(result.vega.unwrap() > 0.0);
    }

    #[test]
    fn test_price_with_enzyme_greeks_fd() {
        let mut pricer = create_pricer();
        let (gbm, payoff, df) = standard_params();

        let result = pricer.price_with_enzyme_greeks(gbm, payoff, df, GreeksMode::FiniteDifference);

        assert!(result.price > 0.0);
        assert!(result.delta.unwrap() > 0.0);
        assert!(result.gamma.unwrap() > 0.0);
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

}
