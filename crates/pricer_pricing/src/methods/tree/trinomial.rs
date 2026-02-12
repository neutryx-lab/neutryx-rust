//! Trinomial tree implementation for option pricing.

use super::common::TreeBase;
use crate::generic_pricer::ConfigError;

/// Trinomial tree parameters.
#[derive(Debug, Clone, Copy)]
pub struct TrinomialParams {
    /// Up factor (u = exp(sigma * sqrt(2 * dt)))
    pub u: f64,
    /// Down factor (d = 1/u)
    pub d: f64,
    /// Risk-neutral probability of up move
    pub p_u: f64,
    /// Risk-neutral probability of middle move
    pub p_m: f64,
    /// Risk-neutral probability of down move
    pub p_d: f64,
    /// Time step size
    pub dt: f64,
}

impl TrinomialParams {
    /// Computes trinomial parameters from volatility, rate, and time step.
    pub fn compute(volatility: f64, rate: f64, dt: f64) -> Self {
        let lambda = 3.0_f64.sqrt();

        let u = (lambda * volatility * dt.sqrt()).exp();
        let d = 1.0 / u;

        let nu = rate - 0.5 * volatility * volatility;

        let sqrt_dt = dt.sqrt();
        let lambda_sq = lambda * lambda;

        let drift_term = nu * sqrt_dt / (2.0 * lambda * volatility);

        let p_u = 1.0 / (2.0 * lambda_sq) + drift_term;
        let p_d = 1.0 / (2.0 * lambda_sq) - drift_term;
        let p_m = 1.0 - 1.0 / lambda_sq;

        let p_u = p_u.clamp(0.0, 1.0);
        let p_d = p_d.clamp(0.0, 1.0);
        let p_m = p_m.clamp(0.0, 1.0);

        let total = p_u + p_m + p_d;
        let p_u = p_u / total;
        let p_m = p_m / total;
        let p_d = p_d / total;

        Self {
            u,
            d,
            p_u,
            p_m,
            p_d,
            dt,
        }
    }
}

/// Trinomial tree for option pricing.
#[derive(Debug, Clone)]
pub struct TrinomialTree {
    base: TreeBase,
    params: TrinomialParams,
}

impl TrinomialTree {
    /// Creates a new trinomial tree for option pricing.
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
        let params = TrinomialParams::compute(volatility, rate, base.dt());
        Ok(Self { base, params })
    }

    /// Returns the trinomial parameters.
    pub fn params(&self) -> TrinomialParams { self.params }

    /// Computes the spot price at node (i, j) where:
    fn spot_at_node(&self, j: i32) -> f64 { self.base.spot * self.params.u.powi(j) }

    /// Prices the option using backward induction.
    pub fn price(&self) -> f64 {
        let n = self.base.num_steps;
        let p_u = self.params.p_u;
        let p_m = self.params.p_m;
        let p_d = self.params.p_d;
        let discount = self.base.discount();

        let size = 2 * n + 1;
        let mut values: Vec<f64> = (0..size)
            .map(|idx| {
                let j = idx as i32 - n as i32;
                let spot_t = self.spot_at_node(j);
                self.base.payoff(spot_t)
            })
            .collect();

        for i in (0..n).rev() {
            let new_size = 2 * i + 1;
            let mut new_values = vec![0.0; new_size];

            for idx in 0..new_size {
                let j = idx as i32 - i as i32;

                let up_idx = ((j + 1) + (i + 1) as i32) as usize;
                let mid_idx = (j + (i + 1) as i32) as usize;
                let down_idx = ((j - 1) + (i + 1) as i32) as usize;

                let continuation = discount
                    * (p_u * values[up_idx] + p_m * values[mid_idx] + p_d * values[down_idx]);

                if self.base.is_american {
                    let spot_ij = self.spot_at_node(j);
                    let intrinsic = self.base.payoff(spot_ij);
                    new_values[idx] = continuation.max(intrinsic);
                } else {
                    new_values[idx] = continuation;
                }
            }
            values = new_values;
        }

        values[0]
    }

    /// Computes Delta from the tree.
    pub fn delta(&self) -> f64 {
        if self.base.num_steps < 1 {
            return 0.0;
        }

        let n = self.base.num_steps;
        let u = self.params.u;
        let p_u = self.params.p_u;
        let p_m = self.params.p_m;
        let p_d = self.params.p_d;
        let discount = self.base.discount();

        let size = 2 * n + 1;
        let mut values: Vec<f64> = (0..size)
            .map(|idx| {
                let j = idx as i32 - n as i32;
                let spot_t = self.spot_at_node(j);
                self.base.payoff(spot_t)
            })
            .collect();

        for i in (1..n).rev() {
            let new_size = 2 * i + 1;
            let mut new_values = vec![0.0; new_size];

            for idx in 0..new_size {
                let j = idx as i32 - i as i32;
                let up_idx = ((j + 1) + (i + 1) as i32) as usize;
                let mid_idx = (j + (i + 1) as i32) as usize;
                let down_idx = ((j - 1) + (i + 1) as i32) as usize;

                let continuation = discount
                    * (p_u * values[up_idx] + p_m * values[mid_idx] + p_d * values[down_idx]);

                if self.base.is_american {
                    let spot_ij = self.spot_at_node(j);
                    let intrinsic = self.base.payoff(spot_ij);
                    new_values[idx] = continuation.max(intrinsic);
                } else {
                    new_values[idx] = continuation;
                }
            }
            values = new_values;
        }

        let v_u = values[2];
        let v_d = values[0];
        let s_u = self.base.spot * u;
        let s_d = self.base.spot / u;

        (v_u - v_d) / (s_u - s_d)
    }

    /// Computes Gamma from the tree.
    pub fn gamma(&self) -> f64 {
        if self.base.num_steps < 1 {
            return 0.0;
        }

        let n = self.base.num_steps;
        let u = self.params.u;
        let p_u = self.params.p_u;
        let p_m = self.params.p_m;
        let p_d = self.params.p_d;
        let discount = self.base.discount();

        let size = 2 * n + 1;
        let mut values: Vec<f64> = (0..size)
            .map(|idx| {
                let j = idx as i32 - n as i32;
                let spot_t = self.spot_at_node(j);
                self.base.payoff(spot_t)
            })
            .collect();

        for i in (1..n).rev() {
            let new_size = 2 * i + 1;
            let mut new_values = vec![0.0; new_size];

            for idx in 0..new_size {
                let j = idx as i32 - i as i32;
                let up_idx = ((j + 1) + (i + 1) as i32) as usize;
                let mid_idx = (j + (i + 1) as i32) as usize;
                let down_idx = ((j - 1) + (i + 1) as i32) as usize;

                let continuation = discount
                    * (p_u * values[up_idx] + p_m * values[mid_idx] + p_d * values[down_idx]);

                if self.base.is_american {
                    let spot_ij = self.spot_at_node(j);
                    let intrinsic = self.base.payoff(spot_ij);
                    new_values[idx] = continuation.max(intrinsic);
                } else {
                    new_values[idx] = continuation;
                }
            }
            values = new_values;
        }

        let v_u = values[2];
        let v_m = values[1];
        let v_d = values[0];

        let s_u = self.base.spot * u;
        let s_m = self.base.spot;
        let s_d = self.base.spot / u;

        let delta_up = (v_u - v_m) / (s_u - s_m);
        let delta_down = (v_m - v_d) / (s_m - s_d);
        let h = (s_u - s_d) / 2.0;

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
    fn test_trinomial_params_compute() {
        let params = TrinomialParams::compute(0.2, 0.05, 0.01);

        assert!(params.u > 1.0, "u should be > 1");
        assert!(
            (params.u * params.d - 1.0).abs() < 1e-10,
            "u * d should be 1"
        );
        assert!(
            (params.p_u + params.p_m + params.p_d - 1.0).abs() < 1e-10,
            "Probabilities should sum to 1"
        );
        assert!(
            params.p_u > 0.0 && params.p_u < 1.0,
            "p_u should be in (0,1)"
        );
        assert!(
            params.p_m > 0.0 && params.p_m < 1.0,
            "p_m should be in (0,1)"
        );
        assert!(
            params.p_d > 0.0 && params.p_d < 1.0,
            "p_d should be in (0,1)"
        );
    }

    #[test]
    fn test_trinomial_tree_new_success() {
        let tree = TrinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 100, true, false);
        assert!(tree.is_ok());

        let tree = tree.unwrap();
        assert_eq!(tree.spot(), 100.0);
        assert_eq!(tree.strike(), 100.0);
        assert!(tree.is_call());
        assert!(!tree.is_american());
    }

    #[test]
    fn test_trinomial_tree_invalid_spot() {
        let result = TrinomialTree::new(-100.0, 100.0, 1.0, 0.05, 0.2, 100, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("spot"));
    }

    #[test]
    fn test_trinomial_tree_invalid_strike() {
        let result = TrinomialTree::new(100.0, 0.0, 1.0, 0.05, 0.2, 100, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("strike"));
    }

    #[test]
    fn test_trinomial_tree_invalid_expiry() {
        let result = TrinomialTree::new(100.0, 100.0, 0.0, 0.05, 0.2, 100, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expiry"));
    }

    #[test]
    fn test_trinomial_tree_invalid_volatility() {
        let result = TrinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.0, 100, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("volatility"));
    }

    #[test]
    fn test_trinomial_tree_invalid_steps() {
        let result = TrinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 0, true, false);
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

        for num_steps in [50, 100, 200] {
            let tree = TrinomialTree::new(
                spot, strike, expiry, rate, volatility, num_steps, true, false,
            )
            .unwrap();
            let tree_price = tree.price();

            let tolerance = match num_steps {
                50 => 0.15,
                100 => 0.08,
                _ => 0.05,
            };

            assert!(
                (tree_price - bs_price).abs() < tolerance,
                "Trinomial tree price {} should be close to BS price {} with {} steps (diff: {})",
                tree_price,
                bs_price,
                num_steps,
                (tree_price - bs_price).abs()
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
            TrinomialTree::new(spot, strike, expiry, rate, volatility, 200, false, false).unwrap();
        let tree_price = tree.price();

        assert!(
            (tree_price - bs_price).abs() < 0.1,
            "Trinomial tree put price {} should be close to BS price {} (diff: {})",
            tree_price,
            bs_price,
            (tree_price - bs_price).abs()
        );
    }

    #[test]
    fn test_american_put_greater_than_european() {
        let spot = 100.0;
        let strike = 100.0;
        let rate = 0.05;
        let volatility = 0.2;
        let expiry = 1.0;
        let num_steps = 200;

        let european_tree = TrinomialTree::new(
            spot, strike, expiry, rate, volatility, num_steps, false, false,
        )
        .unwrap();
        let american_tree = TrinomialTree::new(
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
        let num_steps = 200;

        let european_tree = TrinomialTree::new(
            spot, strike, expiry, rate, volatility, num_steps, true, false,
        )
        .unwrap();
        let american_tree = TrinomialTree::new(
            spot, strike, expiry, rate, volatility, num_steps, true, true,
        )
        .unwrap();

        let european_price = european_tree.price();
        let american_price = american_tree.price();

        assert!(
            (american_price - european_price).abs() < 1e-4,
            "American call {} should equal European call {} (no dividends)",
            american_price,
            european_price
        );
    }

    #[test]
    fn test_delta_reasonable_range() {
        let tree = TrinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 200, true, false).unwrap();
        let delta = tree.delta();

        assert!(
            delta > 0.4 && delta < 0.8,
            "Call delta {} should be reasonable",
            delta
        );
    }

    #[test]
    fn test_delta_put_negative() {
        let tree = TrinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 200, false, false).unwrap();
        let delta = tree.delta();

        assert!(delta < 0.0, "Put delta {} should be negative", delta);
    }

    #[test]
    fn test_gamma_positive() {
        let tree = TrinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 200, true, false).unwrap();
        let gamma = tree.gamma();

        assert!(gamma > 0.0, "Gamma {} should be positive", gamma);
    }

    #[test]
    fn test_deep_itm_call_delta_near_one() {
        let tree = TrinomialTree::new(150.0, 100.0, 1.0, 0.05, 0.2, 200, true, false).unwrap();
        let delta = tree.delta();

        assert!(
            delta > 0.85,
            "Deep ITM call delta {} should be close to 1",
            delta
        );
    }

    #[test]
    fn test_deep_otm_call_delta_near_zero() {
        let tree = TrinomialTree::new(50.0, 100.0, 1.0, 0.05, 0.2, 200, true, false).unwrap();
        let delta = tree.delta();

        assert!(
            delta < 0.15,
            "Deep OTM call delta {} should be close to 0",
            delta
        );
    }

    #[test]
    fn test_trinomial_params_accessible() {
        let tree = TrinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 100, true, false).unwrap();
        let params = tree.params();

        assert!((params.u * params.d - 1.0).abs() < 1e-10);
        assert!((params.p_u + params.p_m + params.p_d - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_trinomial_vs_binomial_consistency() {
        use crate::methods::tree::BinomialTree;

        let spot = 100.0;
        let strike = 100.0;
        let rate = 0.05;
        let volatility = 0.2;
        let expiry = 1.0;

        let binomial =
            BinomialTree::new(spot, strike, expiry, rate, volatility, 500, true, false).unwrap();
        let trinomial =
            TrinomialTree::new(spot, strike, expiry, rate, volatility, 200, true, false).unwrap();

        let bi_price = binomial.price();
        let tri_price = trinomial.price();

        assert!(
            (bi_price - tri_price).abs() < 0.2,
            "Binomial {} and Trinomial {} prices should be close",
            bi_price,
            tri_price
        );
    }
}
