//! Sequential curve bootstrapping implementation.
//!
//! This module provides a sequential (pillar-by-pillar) approach to curve
//! calibration, where each discount factor is solved iteratively using the
//! previous pillars.
//!
//! ## Algorithm
//!
//! For each instrument in maturity order:
//! 1. Fix all previous discount factors
//! 2. Use Newton-Raphson to find the discount factor that prices this
//!    instrument correctly
//! 3. Build an intermediate curve with all solved discount factors
//! 4. Proceed to the next instrument
//!
//! ## Comparison with Global Solver
//!
//! | Aspect | Sequential | Global |
//! |--------|------------|--------|
//! | Complexity | O(n) solves | O(n²) per iteration |
//! | Robustness | May fail if instruments overlap | Handles overlapping |
//! | Jacobian | Not computed | Full J⁻¹ available |
//! | AAD | Per-pillar | Implicit function theorem |

use pricer_core::math::solvers::{NewtonRaphsonSolver, SolverConfig};

use crate::{
    builder::{BootstrapError, BootstrapResult, CalibrationInstrument},
    market::curves::{BootstrapInterpolation, BootstrappedCurve},
};

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for curve bootstrapping.
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    /// Maximum iterations per pillar.
    pub max_iterations: usize,
    /// Convergence tolerance for pricing error.
    pub tolerance: f64,
    /// Interpolation method for the resulting curve.
    pub interpolation: InterpolationMethod,
    /// Finite difference epsilon for numerical derivative.
    pub fd_epsilon: f64,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-10,
            interpolation: InterpolationMethod::LogLinear,
            fd_epsilon: 1e-6,
        }
    }
}

impl BootstrapConfig {
    /// Creates a new configuration with specified tolerance and max iterations.
    pub fn new(tolerance: f64, max_iterations: usize) -> Self {
        Self {
            max_iterations,
            tolerance,
            ..Default::default()
        }
    }

    /// Sets the interpolation method.
    pub fn with_interpolation(mut self, interpolation: InterpolationMethod) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Sets the finite difference epsilon.
    pub fn with_fd_epsilon(mut self, epsilon: f64) -> Self {
        self.fd_epsilon = epsilon;
        self
    }
}

/// Interpolation method for discount factors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterpolationMethod {
    /// Linear interpolation on discount factors.
    Linear,
    /// Log-linear interpolation (linear on log of discount factors).
    #[default]
    LogLinear,
    /// Cubic spline interpolation (not yet implemented).
    CubicSpline,
}

impl InterpolationMethod {
    /// Converts to the curve module's interpolation enum.
    fn to_bootstrap_interpolation(self) -> BootstrapInterpolation {
        match self {
            Self::Linear => BootstrapInterpolation::Linear,
            Self::LogLinear => BootstrapInterpolation::LogLinear,
            Self::CubicSpline => BootstrapInterpolation::LogLinear, // Fallback
        }
    }
}

// =============================================================================
// Bootstrapper
// =============================================================================

/// Sequential curve bootstrapper for yield curve construction.
///
/// This bootstrapper solves for discount factors one at a time, in order of
/// increasing maturity. It uses the [`CalibrationInstrument`] trait to support
/// various instrument types (OIS, IRS, FRA, Futures).
///
/// # Example
///
/// ```ignore
/// use pricer_models::builder::{CurveBootstrapper, BootstrapConfig, CalibrationInstrument};
/// use pricer_models::market::curves::MarketInstrument;
///
/// let instruments = vec![
///     MarketInstrument::ois(1.0, 0.03),
///     MarketInstrument::ois(2.0, 0.035),
///     MarketInstrument::ois(5.0, 0.04),
/// ];
///
/// let bootstrapper = CurveBootstrapper::new();
/// let result = bootstrapper.bootstrap_instruments(&instruments)?;
/// ```
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

    /// Bootstrap a curve from calibration instruments.
    ///
    /// This method sorts instruments by maturity and solves for each discount
    /// factor sequentially using Newton-Raphson iteration.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Slice of calibration instruments
    ///
    /// # Returns
    ///
    /// A `BootstrapResult` containing:
    /// - `discount_factors`: Solved discount factors at each pillar
    /// - `pillars`: Maturities in years
    /// - `residual`: Sum of squared pricing errors
    ///
    /// # Errors
    ///
    /// - `BootstrapError::InsufficientData` if no instruments provided
    /// - `BootstrapError::ConvergenceFailure` if Newton-Raphson fails to
    ///   converge
    pub fn bootstrap_instruments<I>(
        &self,
        instruments: &[I],
    ) -> Result<BootstrapResult, BootstrapError>
    where
        I: CalibrationInstrument<f64> + Clone,
    {
        if instruments.is_empty() {
            return Err(BootstrapError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }

        // Sort instruments by maturity
        let mut sorted_instruments: Vec<I> = instruments.to_vec();
        sorted_instruments.sort_by(|a, b| {
            a.maturity()
                .partial_cmp(&b.maturity())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Check for duplicate maturities
        for i in 1..sorted_instruments.len() {
            let prev_mat = sorted_instruments[i - 1].maturity();
            let curr_mat = sorted_instruments[i].maturity();
            if (prev_mat - curr_mat).abs() < 1e-10 {
                return Err(BootstrapError::DuplicateMaturity { maturity: curr_mat });
            }
        }

        let mut pillars: Vec<f64> = Vec::with_capacity(sorted_instruments.len());
        let mut discount_factors: Vec<f64> = Vec::with_capacity(sorted_instruments.len());
        let mut total_residual = 0.0;

        // Configure Newton-Raphson solver
        let solver_config = SolverConfig::new(self.config.tolerance, self.config.max_iterations);
        let solver = NewtonRaphsonSolver::new(solver_config);

        for instrument in sorted_instruments.iter() {
            let maturity = instrument.maturity();
            let market_rate = instrument.market_rate();

            // Initial guess for discount factor: exp(-r * t)
            let initial_df = (-market_rate * maturity).exp();

            // Build objective function: pricing_error(df) = 0
            let objective = |df: f64| -> f64 {
                // Build temporary curve with current pillars + new df
                let mut temp_pillars = pillars.clone();
                let mut temp_dfs = discount_factors.clone();
                temp_pillars.push(maturity);
                temp_dfs.push(df);

                let curve = match BootstrappedCurve::new(
                    temp_pillars,
                    temp_dfs,
                    self.config.interpolation.to_bootstrap_interpolation(),
                    true, // allow extrapolation
                ) {
                    Ok(c) => c,
                    Err(_) => return f64::MAX, // Invalid curve
                };

                // Compute pricing error
                instrument.pricing_error(&curve).unwrap_or(f64::MAX)
            };

            // Numerical derivative using finite difference
            let epsilon = self.config.fd_epsilon;
            let objective_prime = |df: f64| -> f64 {
                let f_plus = objective(df + epsilon);
                let f_minus = objective(df - epsilon);
                (f_plus - f_minus) / (2.0 * epsilon)
            };

            // Solve for discount factor
            let solved_df = solver
                .find_root(&objective, &objective_prime, initial_df)
                .map_err(|_e| BootstrapError::ConvergenceFailure {
                    maturity,
                    residual: objective(initial_df),
                    iterations: self.config.max_iterations,
                })?;

            // Validate solved discount factor
            if solved_df <= 0.0 || solved_df > 1.5 {
                return Err(BootstrapError::NegativeRate {
                    maturity,
                    rate: -solved_df.ln() / maturity,
                });
            }

            // Compute final residual for this pillar
            let final_error = objective(solved_df);
            total_residual += final_error * final_error;

            pillars.push(maturity);
            discount_factors.push(solved_df);
        }

        Ok(BootstrapResult {
            discount_factors,
            pillars,
            residual: total_residual.sqrt(),
        })
    }

    /// Bootstrap a curve and return the curve object directly.
    ///
    /// This is a convenience method that wraps `bootstrap_instruments` and
    /// constructs a `BootstrappedCurve` from the result.
    pub fn bootstrap_to_curve<I>(
        &self,
        instruments: &[I],
    ) -> Result<BootstrappedCurve<f64>, BootstrapError>
    where
        I: CalibrationInstrument<f64> + Clone,
    {
        let result = self.bootstrap_instruments(instruments)?;

        BootstrappedCurve::new(
            result.pillars,
            result.discount_factors,
            self.config.interpolation.to_bootstrap_interpolation(),
            true,
        )
        .map_err(BootstrapError::InvalidInput)
    }

    /// Legacy bootstrap method using simple swap rate stripping.
    ///
    /// This method is kept for backward compatibility. For new code, prefer
    /// `bootstrap_instruments` which supports all instrument types.
    ///
    /// # Arguments
    ///
    /// * `pillars` - Pillar dates in years from today
    /// * `swap_rates` - Market-observed swap rates (par rates)
    pub fn bootstrap(
        &self,
        pillars: &[f64],
        swap_rates: &[f64],
    ) -> Result<BootstrapResult, BootstrapError> {
        if pillars.len() != swap_rates.len() {
            return Err(BootstrapError::InvalidInput(
                "Pillars and swap rates must have the same length".to_string(),
            ));
        }

        if pillars.is_empty() {
            return Err(BootstrapError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }

        // Use CalibrationInstrument-based approach
        use crate::market::curves::MarketInstrument;

        let instruments: Vec<MarketInstrument<f64>> = pillars
            .iter()
            .zip(swap_rates.iter())
            .map(|(&t, &r)| MarketInstrument::ois(t, r))
            .collect();

        self.bootstrap_instruments(&instruments)
    }

    /// Bootstrap with Jacobian inverse computation using
    /// GlobalCalibrationEngine.
    ///
    /// This method uses the unified `CalibrationEngine<LUStrategy>` internally
    /// to compute the Jacobian inverse, which is useful for AAD-based
    /// sensitivity calculation via implicit function theorem.
    ///
    /// Note: This method is only available when the `global-bootstrap` feature
    /// is enabled, as it uses the same infrastructure as GlobalBootstrapper.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Slice of calibration instruments
    ///
    /// # Returns
    ///
    /// A tuple of (curve, jacobian_inverse) where jacobian_inverse is
    /// Some(DMatrix) if computation succeeded, None otherwise.
    #[cfg(feature = "global-bootstrap")]
    pub fn bootstrap_with_jacobian<I>(
        &self,
        instruments: &[I],
    ) -> Result<
        (
            BootstrappedCurve<f64>,
            Option<pricer_core::math::linalg::DMatrix<f64>>,
        ),
        BootstrapError,
    >
    where
        I: CalibrationInstrument<f64> + Clone,
    {
        use crate::builder::engine::{CalibrationEngine, CalibrationEngineConfig};

        let engine_config = CalibrationEngineConfig {
            tolerance: self.config.tolerance,
            param_tolerance: self.config.tolerance,
            max_iterations: self.config.max_iterations,
            jacobian_epsilon: self.config.fd_epsilon,
            store_jacobian_inverse: true,
            interpolation: self.config.interpolation.to_bootstrap_interpolation(),
            allow_extrapolation: true,
            damping_factor: None,
            debug_logging: false,
        };

        let mut engine = CalibrationEngine::with_lu_strategy(engine_config);

        let result = engine
            .calibrate(instruments)
            .map_err(|e| BootstrapError::InvalidInput(format!("CalibrationEngine failed: {e}")))?;

        if !result.converged {
            return Err(BootstrapError::ConvergenceFailure {
                maturity: result.pillars.last().copied().unwrap_or(0.0),
                residual: result.residual_norm,
                iterations: result.iterations,
            });
        }

        Ok((result.curve, result.jacobian_inverse))
    }
}

impl Default for CurveBootstrapper {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::market::curves::{MarketInstrument, YieldCurve};

    #[test]
    fn test_simple_bootstrap() {
        let bootstrapper = CurveBootstrapper::new();
        let pillars = vec![0.5, 1.0, 2.0];
        let swap_rates = vec![0.02, 0.025, 0.03];

        let result = bootstrapper.bootstrap(&pillars, &swap_rates);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.discount_factors.len(), 3);
        assert!(result.discount_factors[0] > result.discount_factors[1]);
        assert!(result.discount_factors[1] > result.discount_factors[2]);
    }

    #[test]
    fn test_bootstrap_instruments_ois() {
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
            MarketInstrument::ois(5.0, 0.04),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let result = bootstrapper.bootstrap_instruments(&instruments);
        assert!(result.is_ok(), "Bootstrap failed: {:?}", result.err());

        let result = result.unwrap();
        assert_eq!(result.pillars.len(), 3);
        assert_eq!(result.discount_factors.len(), 3);

        assert_relative_eq!(result.pillars[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result.pillars[1], 2.0, epsilon = 1e-10);
        assert_relative_eq!(result.pillars[2], 5.0, epsilon = 1e-10);

        for df in &result.discount_factors {
            assert!(*df > 0.0, "Discount factor should be positive");
            assert!(*df < 1.0, "Discount factor should be less than 1");
        }
        assert!(result.discount_factors[0] > result.discount_factors[1]);
        assert!(result.discount_factors[1] > result.discount_factors[2]);

        assert!(
            result.residual < 1e-8,
            "Residual {} is too large",
            result.residual
        );
    }

    #[test]
    fn test_bootstrap_to_curve() {
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
            MarketInstrument::ois(5.0, 0.04),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let curve = bootstrapper.bootstrap_to_curve(&instruments);
        assert!(curve.is_ok(), "Bootstrap failed: {:?}", curve.err());

        let curve = curve.unwrap();

        let df_1y = curve.discount_factor(1.0).unwrap();
        let df_2y = curve.discount_factor(2.0).unwrap();
        let df_5y = curve.discount_factor(5.0).unwrap();

        assert!(df_1y > 0.0 && df_1y < 1.0);
        assert!(df_2y > 0.0 && df_2y < 1.0);
        assert!(df_5y > 0.0 && df_5y < 1.0);
        assert!(df_1y > df_2y);
        assert!(df_2y > df_5y);
    }

    #[test]
    fn test_bootstrap_unsorted_instruments() {
        let instruments = vec![
            MarketInstrument::ois(5.0, 0.04),
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let result = bootstrapper.bootstrap_instruments(&instruments);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_relative_eq!(result.pillars[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result.pillars[1], 2.0, epsilon = 1e-10);
        assert_relative_eq!(result.pillars[2], 5.0, epsilon = 1e-10);
    }

    #[test]
    fn test_bootstrap_empty_instruments() {
        let instruments: Vec<MarketInstrument<f64>> = vec![];
        let bootstrapper = CurveBootstrapper::new();
        let result = bootstrapper.bootstrap_instruments(&instruments);
        assert!(result.is_err());

        match result.unwrap_err() {
            BootstrapError::InsufficientData { required, provided } => {
                assert_eq!(required, 1);
                assert_eq!(provided, 0);
            }
            other => panic!("Expected InsufficientData error, got {:?}", other),
        }
    }

    #[test]
    fn test_bootstrap_duplicate_maturity() {
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(1.0, 0.032),
            MarketInstrument::ois(2.0, 0.035),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let result = bootstrapper.bootstrap_instruments(&instruments);
        assert!(result.is_err());

        match result.unwrap_err() {
            BootstrapError::DuplicateMaturity { maturity } => {
                assert_relative_eq!(maturity, 1.0, epsilon = 1e-10);
            }
            other => panic!("Expected DuplicateMaturity error, got {:?}", other),
        }
    }

    #[test]
    fn test_bootstrap_with_custom_config() {
        let config = BootstrapConfig::new(1e-12, 200)
            .with_interpolation(InterpolationMethod::LogLinear)
            .with_fd_epsilon(1e-8);

        let bootstrapper = CurveBootstrapper::with_config(config);

        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
        ];

        let result = bootstrapper.bootstrap_instruments(&instruments);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bootstrap_fra_instruments() {
        let instruments = vec![
            MarketInstrument::fra(0.0, 0.5, 0.025),
            MarketInstrument::fra(0.5, 1.0, 0.028),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let result = bootstrapper.bootstrap_instruments(&instruments);
        assert!(result.is_ok(), "FRA bootstrap failed: {:?}", result.err());

        let result = result.unwrap();
        assert_eq!(result.pillars.len(), 2);
    }

    #[test]
    fn test_bootstrap_mixed_instruments() {
        let instruments = vec![
            MarketInstrument::fra(0.0, 0.5, 0.025),
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let result = bootstrapper.bootstrap_instruments(&instruments);
        assert!(result.is_ok(), "Mixed bootstrap failed: {:?}", result.err());

        let result = result.unwrap();
        assert_eq!(result.pillars.len(), 3);
    }

    #[test]
    fn test_repricing_after_bootstrap() {
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
            MarketInstrument::ois(5.0, 0.04),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let curve = bootstrapper.bootstrap_to_curve(&instruments).unwrap();

        for inst in &instruments {
            let error = inst.pricing_error(&curve).unwrap();
            assert!(
                error.abs() < 1e-8,
                "Instrument {:?} has pricing error {}",
                inst.instrument_type(),
                error
            );
        }
    }
}
