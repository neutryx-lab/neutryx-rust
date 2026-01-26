//! Trinomial tree implementation for option pricing.
//!
//! This module provides a trinomial tree alternative to the binomial tree,
//! offering faster convergence with the same number of steps.

use crate::generic_pricer::ConfigError;

/// Trinomial tree parameters.
///
/// The trinomial tree uses three possible movements at each step:
/// - Up (u): with probability p_u
/// - Middle (m = 1): with probability p_m
/// - Down (d): with probability p_d
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
    ///
    /// Uses the Kamrad-Ritchken parameterisation where:
    /// - u = exp(λ * sigma * sqrt(dt)), with λ = sqrt(3)
    /// - d = 1/u
    /// - m = 1 (middle node stays at same level)
    ///
    /// # Arguments
    ///
    /// * `volatility` - Annualized volatility (sigma)
    /// * `rate` - Risk-free rate
    /// * `dt` - Time step size
    ///
    /// # Returns
    ///
    /// Trinomial parameters (u, d, p_u, p_m, p_d, dt)
    pub fn compute(volatility: f64, rate: f64, dt: f64) -> Self {
        // Kamrad-Ritchken trinomial tree parameters
        // λ = sqrt(3) for optimal stability
        let lambda = 3.0_f64.sqrt();

        // u = exp(lambda * sigma * sqrt(dt))
        let u = (lambda * volatility * dt.sqrt()).exp();
        let d = 1.0 / u;

        // Drift term: nu = r - 0.5 * sigma^2
        let nu = rate - 0.5 * volatility * volatility;

        // Probabilities using Kamrad-Ritchken formulation:
        // p_u = 1/(2*lambda^2) + nu*sqrt(dt)/(2*lambda*sigma)
        // p_d = 1/(2*lambda^2) - nu*sqrt(dt)/(2*lambda*sigma)
        // p_m = 1 - 1/lambda^2
        let sqrt_dt = dt.sqrt();
        let lambda_sq = lambda * lambda; // = 3

        let drift_term = nu * sqrt_dt / (2.0 * lambda * volatility);

        let p_u = 1.0 / (2.0 * lambda_sq) + drift_term;
        let p_d = 1.0 / (2.0 * lambda_sq) - drift_term;
        let p_m = 1.0 - 1.0 / lambda_sq;

        // Ensure probabilities are valid (clamp to [0, 1])
        let p_u = p_u.max(0.0).min(1.0);
        let p_d = p_d.max(0.0).min(1.0);
        let p_m = p_m.max(0.0).min(1.0);

        // Renormalise if needed
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
///
/// Implements a trinomial tree model for pricing European and American options.
/// The trinomial tree converges faster than the binomial tree for the same
/// number of steps.
#[derive(Debug, Clone)]
pub struct TrinomialTree {
    spot: f64,
    strike: f64,
    expiry: f64,
    rate: f64,
    volatility: f64,
    num_steps: usize,
    is_call: bool,
    is_american: bool,
    // Cached trinomial parameters
    params: TrinomialParams,
}

impl TrinomialTree {
    /// Creates a new trinomial tree for option pricing.
    ///
    /// # Arguments
    ///
    /// * `spot` - Current spot price
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    /// * `rate` - Risk-free rate (annualized)
    /// * `volatility` - Volatility (annualized)
    /// * `num_steps` - Number of time steps
    /// * `is_call` - True for call, false for put
    /// * `is_american` - True for American, false for European
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if parameters are invalid.
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
        // Validate parameters
        if spot <= 0.0 {
            return Err(ConfigError::InvalidModelParameter {
                name: "spot",
                reason: "spot must be positive".to_string(),
            });
        }
        if strike <= 0.0 {
            return Err(ConfigError::InvalidModelParameter {
                name: "strike",
                reason: "strike must be positive".to_string(),
            });
        }
        if expiry <= 0.0 {
            return Err(ConfigError::InvalidModelParameter {
                name: "expiry",
                reason: "expiry must be positive".to_string(),
            });
        }
        if volatility <= 0.0 {
            return Err(ConfigError::InvalidModelParameter {
                name: "volatility",
                reason: "volatility must be positive".to_string(),
            });
        }
        if num_steps == 0 {
            return Err(ConfigError::InvalidModelParameter {
                name: "num_steps",
                reason: "num_steps must be greater than 0".to_string(),
            });
        }

        let dt = expiry / num_steps as f64;
        let params = TrinomialParams::compute(volatility, rate, dt);

        Ok(Self {
            spot,
            strike,
            expiry,
            rate,
            volatility,
            num_steps,
            is_call,
            is_american,
            params,
        })
    }

    /// Returns the trinomial parameters.
    pub fn params(&self) -> TrinomialParams { self.params }

    /// Computes the payoff at a given spot level.
    fn payoff(&self, spot: f64) -> f64 {
        if self.is_call {
            (spot - self.strike).max(0.0)
        } else {
            (self.strike - spot).max(0.0)
        }
    }

    /// Computes the spot price at node (i, j) where:
    /// - i is the time step (0 to num_steps)
    /// - j is the price level relative to center (can be negative)
    ///
    /// At time step i, j ranges from -i to +i
    /// spot_ij = spot * u^j (where j can be negative, so u^(-j) = d^j)
    fn spot_at_node(&self, j: i32) -> f64 { self.spot * self.params.u.powi(j) }

    /// Prices the option using backward induction.
    ///
    /// # Returns
    ///
    /// The option price at time 0.
    pub fn price(&self) -> f64 {
        let n = self.num_steps;
        let p_u = self.params.p_u;
        let p_m = self.params.p_m;
        let p_d = self.params.p_d;
        let discount = (-self.rate * self.params.dt).exp();

        // At step i, we have 2*i + 1 nodes
        // j ranges from -i to +i
        // We store values indexed by j + n (to handle negative indices)

        // Terminal values (at maturity, step n)
        // j ranges from -n to +n, so 2n+1 values
        let size = 2 * n + 1;
        let mut values: Vec<f64> = (0..size)
            .map(|idx| {
                let j = idx as i32 - n as i32; // j ranges from -n to n
                let spot_t = self.spot_at_node(j);
                self.payoff(spot_t)
            })
            .collect();

        // Backward induction
        for i in (0..n).rev() {
            // At step i, j ranges from -i to +i
            let new_size = 2 * i + 1;
            let mut new_values = vec![0.0; new_size];

            for idx in 0..new_size {
                let j = idx as i32 - i as i32; // j ranges from -i to i

                // Current indices in the values array (from step i+1)
                // At step i+1, j ranges from -(i+1) to +(i+1)
                // So we need values at j-1, j, j+1 relative to step i+1
                let up_idx = ((j + 1) + (i + 1) as i32) as usize; // j+1 at step i+1
                let mid_idx = (j + (i + 1) as i32) as usize; // j at step i+1
                let down_idx = ((j - 1) + (i + 1) as i32) as usize; // j-1 at step i+1

                let continuation = discount
                    * (p_u * values[up_idx] + p_m * values[mid_idx] + p_d * values[down_idx]);

                if self.is_american {
                    // Early exercise check
                    let spot_ij = self.spot_at_node(j);
                    let intrinsic = self.payoff(spot_ij);
                    new_values[idx] = continuation.max(intrinsic);
                } else {
                    new_values[idx] = continuation;
                }
            }
            values = new_values;
        }

        // At step 0, there's only one node (j=0)
        values[0]
    }

    /// Computes Delta from the tree.
    ///
    /// Delta is computed from the first step of the tree using the
    /// up and down values.
    pub fn delta(&self) -> f64 {
        if self.num_steps < 1 {
            return 0.0;
        }

        let n = self.num_steps;
        let u = self.params.u;
        let p_u = self.params.p_u;
        let p_m = self.params.p_m;
        let p_d = self.params.p_d;
        let discount = (-self.rate * self.params.dt).exp();

        // Terminal values
        let size = 2 * n + 1;
        let mut values: Vec<f64> = (0..size)
            .map(|idx| {
                let j = idx as i32 - n as i32;
                let spot_t = self.spot_at_node(j);
                self.payoff(spot_t)
            })
            .collect();

        // Backward induction to step 1
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

                if self.is_american {
                    let spot_ij = self.spot_at_node(j);
                    let intrinsic = self.payoff(spot_ij);
                    new_values[idx] = continuation.max(intrinsic);
                } else {
                    new_values[idx] = continuation;
                }
            }
            values = new_values;
        }

        // At step 1: values[0] = V_d (j=-1), values[1] = V_m (j=0), values[2] = V_u
        // (j=1)
        let v_u = values[2]; // j = +1
        let v_d = values[0]; // j = -1
        let s_u = self.spot * u;
        let s_d = self.spot / u;

        (v_u - v_d) / (s_u - s_d)
    }

    /// Computes Gamma from the tree.
    ///
    /// Gamma is computed from the second derivative approximation using
    /// the values at step 1 of the tree.
    pub fn gamma(&self) -> f64 {
        if self.num_steps < 1 {
            return 0.0;
        }

        let n = self.num_steps;
        let u = self.params.u;
        let p_u = self.params.p_u;
        let p_m = self.params.p_m;
        let p_d = self.params.p_d;
        let discount = (-self.rate * self.params.dt).exp();

        // Terminal values
        let size = 2 * n + 1;
        let mut values: Vec<f64> = (0..size)
            .map(|idx| {
                let j = idx as i32 - n as i32;
                let spot_t = self.spot_at_node(j);
                self.payoff(spot_t)
            })
            .collect();

        // Backward induction to step 1
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

                if self.is_american {
                    let spot_ij = self.spot_at_node(j);
                    let intrinsic = self.payoff(spot_ij);
                    new_values[idx] = continuation.max(intrinsic);
                } else {
                    new_values[idx] = continuation;
                }
            }
            values = new_values;
        }

        // At step 1: values[0] = V_d (j=-1), values[1] = V_m (j=0), values[2] = V_u
        // (j=1)
        let v_u = values[2]; // j = +1
        let v_m = values[1]; // j = 0
        let v_d = values[0]; // j = -1

        let s_u = self.spot * u;
        let s_m = self.spot;
        let s_d = self.spot / u;

        // Gamma = (delta_up - delta_down) / h
        let delta_up = (v_u - v_m) / (s_u - s_m);
        let delta_down = (v_m - v_d) / (s_m - s_d);
        let h = (s_u - s_d) / 2.0;

        (delta_up - delta_down) / h
    }

    /// Returns whether this is a call option.
    pub fn is_call(&self) -> bool { self.is_call }

    /// Returns whether this is an American option.
    pub fn is_american(&self) -> bool { self.is_american }

    /// Returns the spot price.
    pub fn spot(&self) -> f64 { self.spot }

    /// Returns the strike price.
    pub fn strike(&self) -> f64 { self.strike }

    /// Returns the time to expiry.
    pub fn expiry(&self) -> f64 { self.expiry }

    /// Returns the number of steps.
    pub fn num_steps(&self) -> usize { self.num_steps }

    /// Returns the volatility.
    pub fn volatility(&self) -> f64 { self.volatility }

    /// Returns the risk-free rate.
    pub fn rate(&self) -> f64 { self.rate }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Black-Scholes reference for European option verification
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

        // u = exp(0.2 * sqrt(0.02)) ≈ exp(0.0283) ≈ 1.0287
        assert!(params.u > 1.0, "u should be > 1");
        // d = 1/u
        assert!(
            (params.u * params.d - 1.0).abs() < 1e-10,
            "u * d should be 1"
        );
        // Probabilities should sum to 1
        assert!(
            (params.p_u + params.p_m + params.p_d - 1.0).abs() < 1e-10,
            "Probabilities should sum to 1"
        );
        // All probabilities should be positive
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

        // Test convergence with increasing steps
        for num_steps in [50, 100, 200] {
            let tree = TrinomialTree::new(
                spot, strike, expiry, rate, volatility, num_steps, true, false,
            )
            .unwrap();
            let tree_price = tree.price();

            // Allow tolerance based on steps
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

        // Delta for ATM call should be around 0.5-0.7
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

        // Put delta should be negative
        assert!(delta < 0.0, "Put delta {} should be negative", delta);
    }

    #[test]
    fn test_gamma_positive() {
        let tree = TrinomialTree::new(100.0, 100.0, 1.0, 0.05, 0.2, 200, true, false).unwrap();
        let gamma = tree.gamma();

        // Gamma should always be positive for vanilla options
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

        // Verify trinomial relationship: u * d = 1
        assert!((params.u * params.d - 1.0).abs() < 1e-10);
        // Verify probabilities sum to 1
        assert!((params.p_u + params.p_m + params.p_d - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_trinomial_vs_binomial_consistency() {
        use crate::tree::BinomialTree;

        let spot = 100.0;
        let strike = 100.0;
        let rate = 0.05;
        let volatility = 0.2;
        let expiry = 1.0;

        // Both should converge to similar values
        let binomial =
            BinomialTree::new(spot, strike, expiry, rate, volatility, 500, true, false).unwrap();
        let trinomial =
            TrinomialTree::new(spot, strike, expiry, rate, volatility, 200, true, false).unwrap();

        let bi_price = binomial.price();
        let tri_price = trinomial.price();

        // Should be within reasonable tolerance of each other
        assert!(
            (bi_price - tri_price).abs() < 0.2,
            "Binomial {} and Trinomial {} prices should be close",
            bi_price,
            tri_price
        );
    }
}
