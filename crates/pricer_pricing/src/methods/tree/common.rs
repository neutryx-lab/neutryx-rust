//! Common tree pricing infrastructure shared between binomial and trinomial
//! trees.

use crate::pricer::ConfigError;

/// Shared parameters for all tree pricing methods.
#[derive(Debug, Clone)]
pub struct TreeBase {
    pub spot: f64,
    pub strike: f64,
    pub expiry: f64,
    pub rate: f64,
    pub volatility: f64,
    pub num_steps: usize,
    pub is_call: bool,
    pub is_american: bool,
}

impl TreeBase {
    /// Validates and creates common tree parameters.
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
        for (name, value) in [
            ("spot", spot),
            ("strike", strike),
            ("expiry", expiry),
            ("volatility", volatility),
        ] {
            if value <= 0.0 {
                return Err(ConfigError::InvalidModelParameter {
                    name,
                    reason: format!("{name} must be positive"),
                });
            }
        }
        if num_steps == 0 {
            return Err(ConfigError::InvalidModelParameter {
                name: "num_steps",
                reason: "num_steps must be greater than 0".to_string(),
            });
        }

        Ok(Self {
            spot,
            strike,
            expiry,
            rate,
            volatility,
            num_steps,
            is_call,
            is_american,
        })
    }

    /// Time step size.
    #[inline]
    pub fn dt(&self) -> f64 { self.expiry / self.num_steps as f64 }

    /// Computes the payoff at a given spot level.
    #[inline]
    pub fn payoff(&self, spot: f64) -> f64 {
        if self.is_call {
            (spot - self.strike).max(0.0)
        } else {
            (self.strike - spot).max(0.0)
        }
    }

    /// Discount factor per time step.
    #[inline]
    pub fn discount(&self) -> f64 { (-self.rate * self.dt()).exp() }
}
