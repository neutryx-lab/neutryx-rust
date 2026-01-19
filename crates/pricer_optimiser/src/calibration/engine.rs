//! Calibration engine implementation.

use crate::error::OptimiserError;

/// Configuration for model calibration.
#[derive(Debug, Clone)]
pub struct CalibrationConfig {
    /// Maximum iterations
    pub max_iterations: usize,
    /// Convergence tolerance
    pub tolerance: f64,
    /// Use finite differences for gradients (vs AD)
    pub use_finite_diff: bool,
    /// Finite difference step size
    pub fd_step: f64,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            tolerance: 1e-8,
            use_finite_diff: true,
            fd_step: 1e-6,
        }
    }
}

/// Result of model calibration.
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    /// Calibrated parameters
    pub parameters: Vec<f64>,
    /// Final residual (sum of squared errors)
    pub residual: f64,
    /// Number of iterations
    pub iterations: usize,
    /// Convergence status
    pub converged: bool,
}

/// Calibration engine for stochastic models.
pub struct CalibrationEngine {
    config: CalibrationConfig,
}

impl CalibrationEngine {
    /// Create a new calibration engine with default configuration.
    pub fn new() -> Self {
        Self {
            config: CalibrationConfig::default(),
        }
    }

    /// Create a new calibration engine with custom configuration.
    pub fn with_config(config: CalibrationConfig) -> Self { Self { config } }

    /// Calibrate a model to market data.
    ///
    /// # Arguments
    ///
    /// * `initial_params` - Initial parameter guess
    /// * `market_prices` - Market prices to match
    /// * `pricer` - Function that computes model prices given parameters
    ///
    /// # Returns
    ///
    /// A `CalibrationResult` containing the calibrated parameters.
    #[allow(clippy::needless_range_loop)]
    pub fn calibrate<F>(
        &self,
        initial_params: &[f64],
        market_prices: &[f64],
        pricer: F,
    ) -> Result<CalibrationResult, OptimiserError>
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        let mut params = initial_params.to_vec();
        let n_params = params.len();
        let n_prices = market_prices.len();

        if n_prices == 0 {
            return Err(OptimiserError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }

        let mut iteration = 0;
        let mut residual = f64::MAX;

        while iteration < self.config.max_iterations {
            // Compute model prices
            let model_prices = pricer(&params);

            // Compute residual
            let mut new_residual = 0.0;
            for (mp, tp) in market_prices.iter().zip(model_prices.iter()) {
                new_residual += (mp - tp).powi(2);
            }

            // Check convergence
            if new_residual < self.config.tolerance {
                return Ok(CalibrationResult {
                    parameters: params,
                    residual: new_residual,
                    iterations: iteration,
                    converged: true,
                });
            }

            // Compute Jacobian via finite differences
            let mut jacobian = vec![vec![0.0; n_params]; n_prices];
            for j in 0..n_params {
                let mut params_plus = params.clone();
                params_plus[j] += self.config.fd_step;
                let prices_plus = pricer(&params_plus);

                for i in 0..n_prices {
                    jacobian[i][j] = (prices_plus[i] - model_prices[i]) / self.config.fd_step;
                }
            }

            // Simple gradient descent step (simplified Levenberg-Marquardt)
            let mut gradient = vec![0.0; n_params];
            for j in 0..n_params {
                for i in 0..n_prices {
                    let error = market_prices[i] - model_prices[i];
                    gradient[j] += jacobian[i][j] * error;
                }
            }

            // Update parameters
            let step_size = 0.1 / (1.0 + iteration as f64);
            for (p, g) in params.iter_mut().zip(gradient.iter()) {
                *p += step_size * g;
            }

            residual = new_residual;
            iteration += 1;
        }

        Err(OptimiserError::ConvergenceFailure {
            iterations: iteration,
            residual,
        })
    }
}

impl Default for CalibrationEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_calibration() {
        let engine = CalibrationEngine::new();

        // Simple test: calibrate a = 2 such that a * x = y
        let market_prices = vec![2.0, 4.0, 6.0]; // y = 2x for x = 1, 2, 3
        let initial = vec![1.0]; // Start with a = 1

        let pricer = |params: &[f64]| {
            let a = params[0];
            vec![a * 1.0, a * 2.0, a * 3.0]
        };

        let result = engine.calibrate(&initial, &market_prices, pricer);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!((result.parameters[0] - 2.0).abs() < 0.1);
    }
}
