//! Integration tests: Monte Carlo pricing with YieldCurve.

#[cfg(test)]
mod tests {
    use pricer_models::market::curves::{FlatCurve, YieldCurve};

    use crate::methods::mc::{GbmParams, MonteCarloConfig, MonteCarloPricer, PayoffParams};

    /// Test that FlatCurve is accessible and can be used for discounting.
    #[test]
    fn test_flat_curve_accessible() {
        let curve = FlatCurve::new(0.05_f64);

        let df = curve.discount_factor(1.0).unwrap();
        assert!((df - (-0.05_f64).exp()).abs() < 1e-10);
    }

    /// Test pricing European call with YieldCurve.
    #[test]
    fn test_price_with_yield_curve() {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
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
        let payoff = PayoffParams::call(100.0);
        let curve = FlatCurve::new(0.05_f64);

        let mut pricer1 = MonteCarloPricer::new(config.clone()).unwrap();
        let result = pricer1.price_european_with_curve(gbm, payoff, &curve);

        assert!(result.price > 0.0);

        let mut pricer2 = MonteCarloPricer::new(config).unwrap();
        let manual_df = curve.discount_factor(gbm.maturity).unwrap();
        let result_manual = pricer2.price_european(gbm, payoff, manual_df);

        assert!(
            (result.price - result_manual.price).abs() < 1e-10,
            "Prices differ: {} vs {}",
            result.price,
            result_manual.price
        );
    }

    /// Test that discount factor from YieldCurve matches manual calculation.
    #[test]
    fn test_yield_curve_discount_factor_consistency() {
        let rate = 0.05_f64;
        let maturity = 1.0_f64;
        let curve = FlatCurve::new(rate);

        let df_from_curve = curve.discount_factor(maturity).unwrap();
        let df_manual = (-rate * maturity).exp();

        assert!((df_from_curve - df_manual).abs() < 1e-10);
    }

    /// Test pricing with Greeks using YieldCurve.
    #[test]
    fn test_price_with_greeks_and_curve() {
        use crate::methods::mc::Greek;

        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let curve = FlatCurve::new(0.05_f64);

        let result = pricer.price_with_greeks_and_curve(gbm, payoff, &curve, &[Greek::Delta]);

        assert!(result.delta.is_some());
        let delta = result.delta.unwrap();
        assert!(delta > 0.3 && delta < 0.8, "Delta = {}", delta);
    }
}
