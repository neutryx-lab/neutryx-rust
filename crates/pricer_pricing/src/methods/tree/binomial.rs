//! Binomial tree implementation using the Cox-Ross-Rubinstein (CRR) algorithm.

use super::common::TreeBase;
use crate::generic_pricer::ConfigError;

/// CRR (Cox-Ross-Rubinstein) parameters for binomial tree.
#[derive(Debug, Clone, Copy)]
pub struct CrrParams {
    /// Up factor (u = exp(sigma * sqrt(dt)))
    pub u: f64,
    /// Down factor (d = 1/u)
    pub d: f64,
    /// Risk-neutral probability of up move
    pub p: f64,
    /// Time step size
    pub dt: f64,
}

impl CrrParams {
    /// Computes CRR parameters from volatility, rate, and time step.
    pub fn compute(volatility: f64, rate: f64, dt: f64) -> Self {
        let u = (volatility * dt.sqrt()).exp();
        let d = 1.0 / u;
        let p = ((rate * dt).exp() - d) / (u - d);

        Self { u, d, p, dt }
    }
}

/// Binomial tree for option pricing using the CRR algorithm.
#[derive(Debug, Clone)]
pub struct BinomialTree {
    base: TreeBase,
    params: CrrParams,
}

impl BinomialTree {
    /// Creates a new binomial tree for option pricing.
    pub fn new(
        spot: f64,
        strike: f64,
        expiry: f64,
        rate: f64,
        volatility: f64,
        num_steps: usize,
        is_call: bool,
        is_american: bool,
    ) -> Result<Self, ConfigError> {
        let base = TreeBase::new(
            spot,
            strike,
            expiry,
            rate,
            volatility,
            num_steps,
            is_call,
            is_american,
        )?;
        let params = CrrParams::compute(volatility, rate, base.dt());
        Ok(Self { base, params })
    }

    /// Returns the CRR parameters.
    pub fn params(&self) -> CrrParams { self.params }

    /// Prices the option using backward induction.
    pub fn price(&self) -> f64 {
        let n = self.base.num_steps;
        let u = self.params.u;
        let d = self.params.d;
        let p = self.params.p;
        let discount = self.base.discount();

        let mut values: Vec<f64> = (0..=n)
            .map(|j| {
                let spot_t = self.base.spot * u.powi(j as i32) * d.powi((n - j) as i32);
                self.base.payoff(spot_t)
            })
            .collect();

        for i in (0..n).rev() {
            for j in 0..=i {
                let continuation = discount * (p * values[j + 1] + (1.0 - p) * values[j]);

                if self.base.is_american {
                    let spot_ij = self.base.spot * u.powi(j as i32) * d.powi((i - j) as i32);
                    let intrinsic = self.base.payoff(spot_ij);
                    values[j] = continuation.max(intrinsic);
                } else {
                    values[j] = continuation;
                }
            }
        }

        values[0]
    }

    /// Computes Delta from the tree.
    pub fn delta(&self) -> f64 {
        if self.base.num_steps < 1 {
            return 0.0;
        }

        let u = self.params.u;
        let d = self.params.d;
        let p = self.params.p;
        let discount = self.base.discount();
        let n = self.base.num_steps;

        let mut values: Vec<f64> = (0..=n)
            .map(|j| {
                let spot_t = self.base.spot * u.powi(j as i32) * d.powi((n - j) as i32);
                self.base.payoff(spot_t)
            })
            .collect();

        for i in (1..n).rev() {
            for j in 0..=i {
                let continuation = discount * (p * values[j + 1] + (1.0 - p) * values[j]);

                if self.base.is_american {
                    let spot_ij = self.base.spot * u.powi(j as i32) * d.powi((i - j) as i32);
                    let intrinsic = self.base.payoff(spot_ij);
                    values[j] = continuation.max(intrinsic);
                } else {
                    values[j] = continuation;
                }
            }
        }

        let v_u = values[1];
        let v_d = values[0];
        let s_u = self.base.spot * u;
        let s_d = self.base.spot * d;

        (v_u - v_d) / (s_u - s_d)
    }

    /// Computes Gamma from the tree.
    pub fn gamma(&self) -> f64 {
        if self.base.num_steps < 2 {
            return 0.0;
        }

        let u = self.params.u;
        let d = self.params.d;
        let p = self.params.p;
        let discount = self.base.discount();
        let n = self.base.num_steps;

        let mut values: Vec<f64> = (0..=n)
            .map(|j| {
                let spot_t = self.base.spot * u.powi(j as i32) * d.powi((n - j) as i32);
                self.base.payoff(spot_t)
            })
            .collect();

        for i in (2..n).rev() {
            for j in 0..=i {
                let continuation = discount * (p * values[j + 1] + (1.0 - p) * values[j]);

                if self.base.is_american {
                    let spot_ij = self.base.spot * u.powi(j as i32) * d.powi((i - j) as i32);
                    let intrinsic = self.base.payoff(spot_ij);
                    values[j] = continuation.max(intrinsic);
                } else {
                    values[j] = continuation;
                }
            }
        }

        let v_uu = values[2];
        let v_ud = values[1];
        let v_dd = values[0];

        let s_uu = self.base.spot * u * u;
        let s_ud = self.base.spot;
        let s_dd = self.base.spot * d * d;

        let delta_up = (v_uu - v_ud) / (s_uu - s_ud);
        let delta_down = (v_ud - v_dd) / (s_ud - s_dd);
        let h = (s_uu - s_dd) / 2.0;

        (delta_up - delta_down) / h
    }

    /// Returns whether this is a call option.
    pub fn is_call(&self) -> bool { self.base.is_call }

    /// Returns whether this is an American option.
    pub fn is_american(&self) -> bool { self.base.is_american }

    /// Returns the spot price.
    pub fn spot(&self) -> f64 { self.base.spot }

    /// Returns the strike price.
    pub fn strike(&self) -> f64 { self.base.strike }

    /// Returns the time to expiry.
    pub fn expiry(&self) -> f64 { self.base.expiry }

    /// Returns the number of steps.
    pub fn num_steps(&self) -> usize { self.base.num_steps }

    /// Returns the volatility.
    pub fn volatility(&self) -> f64 { self.base.volatility }

    /// Returns the risk-free rate.
    pub fn rate(&self) -> f64 { self.base.rate }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn black_scholes_call(spot: f64, strike: f64, rate: f64, volatility: f64, expiry: f64) -> f64 {
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

        let d1 = ((spot / strike).ln() + (rate + 0.5 * volatility * volatility) * expiry)
            / (volatility * expiry.sqrt());
        let d2 = d1 - volatility * expiry.sqrt();

        spot * norm_cdf(d1) - strike * (-rate * expiry).exp() * norm_cdf(d2)
    }

    fn black_scholes_put(spot: f64, strike: f64, rate: f64, volatility: f64, expiry: f64) -> f64 {
        let call = black_scholes_call(spot, strike, rate, volatility, expiry);
        call - spot + strike * (-rate * expiry).exp()
    }

    #[test]
    fn test_crr_params_compute() {
        let params = CrrParams::compute(0.2, 0.05, 0.01);

        assert!((params.u - 1.0202).abs() < 0.001);
        assert!((params.d - 0.9802).abs() < 0.001);
        assert!((params.u * params.d - 1.0).abs() < 1e-10);
        assert_eq!(params.dt, 0.01);
    }

    #[test]
    fn test_binomial_tree_new_success() {
        let tree = BinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 100, true, false);
        assert!(tree.is_ok());

        let tree = tree.unwrap();
        assert_eq!(tree.spot(), 100.0);
        assert_eq!(tree.strike(), 100.0);
        assert!(tree.is_call());
        assert!(!tree.is_american());
    }

    #[test]
    fn test_binomial_tree_invalid_spot() {
        let result = BinomialTree::new(-100.0, 100.0, 1.0, 0.05, 0.2, 100, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("spot"));
    }

    #[test]
    fn test_binomial_tree_invalid_strike() {
        let result = BinomialTree::new(100.0, 0.0, 1.0, 0.05, 0.2, 100, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("strike"));
    }

    #[test]
    fn test_binomial_tree_invalid_expiry() {
        let result = BinomialTree::new(100.0, 100.0, 0.0, 0.05, 0.2, 100, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expiry"));
    }

    #[test]
    fn test_binomial_tree_invalid_volatility() {
        let result = BinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.0, 100, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("volatility"));
    }

    #[test]
    fn test_binomial_tree_invalid_steps() {
        let result = BinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 0, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("num_steps"));
    }

    #[test]
    fn test_european_call_converges_to_black_scholes() {
        let spot = 100.0;
        let strike = 100.0;
        let rate = 0.05;
        let volatility = 0.2;
        let expiry = 1.0;

        let bs_price = black_scholes_call(spot, strike, rate, volatility, expiry);

        for num_steps in [100, 500, 1000] {
            let tree = BinomialTree::new(
                spot, strike, expiry, rate, volatility, num_steps, true, false,
            )
            .unwrap();
            let tree_price = tree.price();

            let tolerance = match num_steps {
                100 => 0.1,
                500 => 0.02,
                _ => 0.01,
            };

            assert!(
                (tree_price - bs_price).abs() < tolerance,
                "Tree price {} should be close to BS price {} with {} steps",
                tree_price,
                bs_price,
                num_steps
            );
        }
    }

    #[test]
    fn test_european_put_converges_to_black_scholes() {
        let spot = 100.0;
        let strike = 100.0;
        let rate = 0.05;
        let volatility = 0.2;
        let expiry = 1.0;

        let bs_price = black_scholes_put(spot, strike, rate, volatility, expiry);

        let tree =
            BinomialTree::new(spot, strike, expiry, rate, volatility, 500, false, false).unwrap();
        let tree_price = tree.price();

        assert!(
            (tree_price - bs_price).abs() < 0.02,
            "Tree put price {} should be close to BS price {}",
            tree_price,
            bs_price
        );
    }

    #[test]
    fn test_american_put_greater_than_european() {
        let spot = 100.0;
        let strike = 100.0;
        let rate = 0.05;
        let volatility = 0.2;
        let expiry = 1.0;
        let num_steps = 500;

        let european_tree = BinomialTree::new(
            spot, strike, expiry, rate, volatility, num_steps, false, false,
        )
        .unwrap();
        let american_tree = BinomialTree::new(
            spot, strike, expiry, rate, volatility, num_steps, false, true,
        )
        .unwrap();

        let european_price = european_tree.price();
        let american_price = american_tree.price();

        assert!(
            american_price >= european_price - 1e-10,
            "American put {} should be >= European put {}",
            american_price,
            european_price
        );
    }

    #[test]
    fn test_american_call_equals_european_no_dividend() {
        let spot = 100.0;
        let strike = 100.0;
        let rate = 0.05;
        let volatility = 0.2;
        let expiry = 1.0;
        let num_steps = 500;

        let european_tree = BinomialTree::new(
            spot, strike, expiry, rate, volatility, num_steps, true, false,
        )
        .unwrap();
        let american_tree = BinomialTree::new(
            spot, strike, expiry, rate, volatility, num_steps, true, true,
        )
        .unwrap();

        let european_price = european_tree.price();
        let american_price = american_tree.price();

        assert!(
            (american_price - european_price).abs() < 1e-6,
            "American call {} should equal European call {} (no dividends)",
            american_price,
            european_price
        );
    }

    #[test]
    fn test_delta_reasonable_range() {
        let tree = BinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 500, true, false).unwrap();
        let delta = tree.delta();

        assert!(
            delta > 0.4 && delta < 0.8,
            "Call delta {} should be reasonable",
            delta
        );
    }

    #[test]
    fn test_delta_put_negative() {
        let tree = BinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 500, false, false).unwrap();
        let delta = tree.delta();

        assert!(delta < 0.0, "Put delta {} should be negative", delta);
        assert!(
            delta > -0.7 && delta < -0.3,
            "Put delta {} should be reasonable",
            delta
        );
    }

    #[test]
    fn test_gamma_positive() {
        let tree = BinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 500, true, false).unwrap();
        let gamma = tree.gamma();

        assert!(gamma > 0.0, "Gamma {} should be positive", gamma);
    }

    #[test]
    fn test_deep_itm_call_delta_near_one() {
        let tree = BinomialTree::new(150.0, 100.0, 1.0, 0.05, 0.2, 500, true, false).unwrap();
        let delta = tree.delta();

        assert!(
            delta > 0.9,
            "Deep ITM call delta {} should be close to 1",
            delta
        );
    }

    #[test]
    fn test_deep_otm_call_delta_near_zero() {
        let tree = BinomialTree::new(50.0, 100.0, 1.0, 0.05, 0.2, 500, true, false).unwrap();
        let delta = tree.delta();

        assert!(
            delta < 0.1,
            "Deep OTM call delta {} should be close to 0",
            delta
        );
    }

    #[test]
    fn test_crr_params_accessible() {
        let tree = BinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 100, true, false).unwrap();
        let params = tree.params();

        assert!((params.u * params.d - 1.0).abs() < 1e-10);
        assert!(params.p > 0.0 && params.p < 1.0);
    }
}
