//! Curve bootstrapping implementation.

use crate::{bootstrapping::BootstrapResult, error::OptimiserError};

/// Configuration for curve bootstrapping.
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    /// Maximum iterations
    pub max_iterations: usize,
    /// Convergence tolerance
    pub tolerance: f64,
    /// Interpolation method
    pub interpolation: InterpolationMethod,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-10,
            interpolation: InterpolationMethod::LogLinear,
        }
    }
}

/// Interpolation method for discount factors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationMethod {
    /// Linear interpolation on discount factors
    Linear,
    /// Log-linear interpolation (linear on log of discount factors)
    LogLinear,
    /// Cubic spline interpolation
    CubicSpline,
}

/// Curve bootstrapper for yield curve construction.
pub struct CurveBootstrapper {
    config: BootstrapConfig,
}

impl CurveBootstrapper {
    /// Create a new curve bootstrapper with default configuration.
    pub fn new() -> Self {
        Self {
            config: BootstrapConfig::default(),
        }
    }

    /// Create a new curve bootstrapper with custom configuration.
    pub fn with_config(config: BootstrapConfig) -> Self { Self { config } }

    /// Returns the configuration.
    pub fn config(&self) -> &BootstrapConfig { &self.config }

    /// Bootstrap a discount curve from swap rates.
    ///
    /// # Arguments
    ///
    /// * `pillars` - Pillar dates in years from today
    /// * `swap_rates` - Market-observed swap rates
    ///
    /// # Returns
    ///
    /// A `BootstrapResult` containing the bootstrapped discount factors.
    pub fn bootstrap(
        &self,
        pillars: &[f64],
        swap_rates: &[f64],
    ) -> Result<BootstrapResult, OptimiserError> {
        if pillars.len() != swap_rates.len() {
            return Err(OptimiserError::InvalidMarketData(
                "Pillars and swap rates must have the same length".to_string(),
            ));
        }

        if pillars.is_empty() {
            return Err(OptimiserError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }

        // Simple bootstrapping (iterative stripping)
        let mut discount_factors = Vec::with_capacity(pillars.len());
        let mut iterations = 0;

        for (i, (&t, &rate)) in pillars.iter().zip(swap_rates.iter()).enumerate() {
            let df = if i == 0 {
                // First pillar: df = 1 / (1 + r * t)
                1.0 / (1.0 + rate * t)
            } else {
                // Subsequent pillars: solve for df using previous discount factors
                let mut sum = 0.0;
                for j in 0..i {
                    let dt = if j == 0 {
                        pillars[j]
                    } else {
                        pillars[j] - pillars[j - 1]
                    };
                    sum += rate * dt * discount_factors[j];
                }
                let dt_last = t - pillars[i - 1];
                (1.0 - sum) / (1.0 + rate * dt_last)
            };

            discount_factors.push(df);
            iterations += 1;

            if iterations >= self.config.max_iterations {
                break;
            }
        }

        Ok(BootstrapResult {
            discount_factors,
            pillars: pillars.to_vec(),
            residual: 0.0,
        })
    }
}

impl Default for CurveBootstrapper {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_bootstrap() {
        let bootstrapper = CurveBootstrapper::new();
        let pillars = vec![0.5, 1.0, 2.0];
        let swap_rates = vec![0.02, 0.025, 0.03];

        let result = bootstrapper.bootstrap(&pillars, &swap_rates);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.discount_factors.len(), 3);
        // Discount factors should be decreasing
        assert!(result.discount_factors[0] > result.discount_factors[1]);
        assert!(result.discount_factors[1] > result.discount_factors[2]);
    }
}
