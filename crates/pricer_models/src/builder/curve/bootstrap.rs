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
//! | Jacobian | FD Jacobian dDF/dr (lower-triangular) | Full J⁻¹ available |
//! | AAD | Per-pillar | Implicit function theorem |

use pricer_core::math::solvers::{NewtonRaphsonSolver, SolverConfig};

use crate::{
    builder::{BootstrapError, BootstrapResult, CalibrationInstrument},
    market::curves::{BootstrapInterpolation, BootstrappedCurve, MarketInstrument},
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
    pub interpolation: BootstrapInterpolation,
    /// Finite difference epsilon for numerical derivative.
    pub fd_epsilon: f64,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-10,
            interpolation: BootstrapInterpolation::LogLinear,
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
    pub fn with_interpolation(mut self, interpolation: BootstrapInterpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Sets the finite difference epsilon.
    pub fn with_fd_epsilon(mut self, epsilon: f64) -> Self {
        self.fd_epsilon = epsilon;
        self
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
        self.bootstrap_instruments_inner(instruments, &[])
    }

    /// Jump-aware bootstrap: solves for base DFs such that the combined
    /// (base + jumps) curve reprices all instruments.
    ///
    /// When `jumps` is non-empty, each Newton-Raphson evaluation constructs
    /// a temporary curve **with** jump offsets attached, so the pricing
    /// error reflects the full adjusted discount factors.
    fn bootstrap_instruments_inner<I>(
        &self,
        instruments: &[I],
        jumps: &[(f64, f64)],
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
                    self.config.interpolation,
                    true, // allow extrapolation
                ) {
                    Ok(c) => c,
                    Err(_) => return f64::MAX, // Invalid curve
                };

                // Attach jump data so pricing evaluates the full adjusted
                // curve (base + forward-rate shifts).
                let curve = if jumps.is_empty() {
                    curve
                } else {
                    curve.with_jumps(jumps.to_vec())
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
            self.config.interpolation,
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
            interpolation: self.config.interpolation,
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
// Finite-Difference Jacobian for Sequential Bootstrap
// =============================================================================

/// Result of a finite-difference Jacobian computation.
///
/// Contains the matrix d(log DF_i) / dr_j where:
/// - Row i corresponds to pillar i (log discount factor log DF_i)
/// - Column j corresponds to instrument j (market rate r_j)
///
/// Using log(DF) rather than DF directly because:
/// - The global solver parametrises unknowns as x = log(DF)
/// - Log-linear interpolation operates in log(DF) space
/// - log(DF) = −r·t gives uniform scale across maturities
///
/// Note: the service layer normalises each row by T_i to produce
/// `[d(log DF_i)/T_i] / dr_j ≈ −dz_i/dr_j` (zero-rate sensitivity).
///
/// For the sequential bootstrapper this matrix is lower-triangular
/// because DF_i depends only on rates r_1 .. r_i.
#[derive(Debug, Clone)]
pub struct JacobianMatrix {
    /// Row-major n x n matrix of d(log DF_i) / dr_j values.
    pub data: Vec<Vec<f64>>,
    /// Number of instruments / pillars (n).
    pub size: usize,
}

impl CurveBootstrapper {
    /// Compute the finite-difference Jacobian d(log DF_i) / dr_j for
    /// sequential bootstrap.
    ///
    /// For each instrument j, bumps its market rate by +/- epsilon,
    /// re-bootstraps the full curve, and computes the central-difference
    /// derivative of log(DF) at each pillar with respect to the bumped rate.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Sorted slice of market instruments (same order as
    ///   `bootstrap_instruments` output)
    ///
    /// # Returns
    ///
    /// A `JacobianMatrix` of size n x n.
    pub fn compute_fd_jacobian(
        &self,
        instruments: &[MarketInstrument<f64>],
    ) -> Result<JacobianMatrix, BootstrapError> {
        self.compute_fd_jacobian_inner(instruments, &[])
    }

    /// Compute the FD Jacobian with jump-aware bootstrap.
    ///
    /// Each bumped re-bootstrap uses `bootstrap_instruments_inner` with
    /// the same jump data, so the sensitivities reflect the combined
    /// (base + jumps) curve.
    fn compute_fd_jacobian_inner(
        &self,
        instruments: &[MarketInstrument<f64>],
        jumps: &[(f64, f64)],
    ) -> Result<JacobianMatrix, BootstrapError> {
        let n = instruments.len();
        let epsilon = self.config.fd_epsilon;

        let mut data = vec![vec![0.0; n]; n];

        for j in 0..n {
            // Bump instrument j up
            let mut bumped_up = instruments.to_vec();
            bumped_up[j] = bumped_up[j].with_bumped_rate(epsilon);

            // Bump instrument j down
            let mut bumped_down = instruments.to_vec();
            bumped_down[j] = bumped_down[j].with_bumped_rate(-epsilon);

            // Re-bootstrap with bumped instruments (jump-aware)
            let result_up = self.bootstrap_instruments_inner(&bumped_up, jumps)?;
            let result_down = self.bootstrap_instruments_inner(&bumped_down, jumps)?;

            // Central difference: d(log DF_i) / dr_j
            for i in 0..n {
                let log_df_up = result_up.discount_factors[i].ln();
                let log_df_down = result_down.discount_factors[i].ln();
                data[i][j] = (log_df_up - log_df_down) / (2.0 * epsilon);
            }
        }

        // Zero out upper triangle to enforce lower-triangular structure
        // (numerical noise may produce tiny values above the diagonal)
        for i in 0..n {
            for j in (i + 1)..n {
                data[i][j] = 0.0;
            }
        }

        Ok(JacobianMatrix { data, size: n })
    }

    /// Bootstrap a curve with jump data and compute the finite-difference
    /// Jacobian.
    ///
    /// This is a convenience method that:
    /// 1. Sorts instruments and bootstraps discount factors
    /// 2. Computes the FD Jacobian dDF/dr
    /// 3. Constructs the curve with optional jump data
    ///
    /// # Returns
    ///
    /// A tuple of `(BootstrappedCurve, JacobianMatrix)`.
    pub fn bootstrap_to_curve_with_jacobian(
        &self,
        instruments: &[MarketInstrument<f64>],
        jumps: &[(f64, f64)],
    ) -> Result<(BootstrappedCurve<f64>, JacobianMatrix), BootstrapError> {
        // Sort instruments by maturity (same ordering as bootstrap_instruments)
        let mut sorted: Vec<MarketInstrument<f64>> = instruments.to_vec();
        sorted.sort_by(|a, b| {
            CalibrationInstrument::<f64>::maturity(a)
                .partial_cmp(&CalibrationInstrument::<f64>::maturity(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Jump-aware calibration: base DFs are solved so that
        // (base + jumps) reprices all instruments.
        let base_result = self.bootstrap_instruments_inner(&sorted, jumps)?;
        let jacobian = self.compute_fd_jacobian_inner(&sorted, jumps)?;

        let curve = BootstrappedCurve::new(
            base_result.pillars,
            base_result.discount_factors,
            self.config.interpolation,
            true,
        )
        .map_err(BootstrapError::InvalidInput)?;

        let curve = if jumps.is_empty() {
            curve
        } else {
            curve.with_jumps(jumps.to_vec())
        };

        Ok((curve, jacobian))
    }
}

// =============================================================================
// Jump-Aware Bootstrap Extensions
// =============================================================================

impl CurveBootstrapper {
    /// Bootstrap a curve with jump data.
    ///
    /// This method bootstraps a yield curve and attaches jump information
    /// for modelling rate discontinuities at central bank meeting dates.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Slice of calibration instruments
    /// * `jumps` - Pre-computed jump data as `(time, cumulative_offset)` pairs.
    ///   Typically a dense daily grid produced by the forward-rate-shift model.
    ///
    /// # Returns
    ///
    /// A `BootstrappedCurve` with jump data attached.
    pub fn bootstrap_to_curve_with_jumps<I>(
        &self,
        instruments: &[I],
        jumps: &[(f64, f64)],
    ) -> Result<BootstrappedCurve<f64>, BootstrapError>
    where
        I: CalibrationInstrument<f64> + Clone,
    {
        // Jump-aware calibration: base DFs are solved so that
        // (base + jumps) reprices all instruments.
        let result = self.bootstrap_instruments_inner(instruments, jumps)?;

        let curve = BootstrappedCurve::new(
            result.pillars,
            result.discount_factors,
            self.config.interpolation,
            true,
        )
        .map_err(BootstrapError::InvalidInput)?;

        if jumps.is_empty() {
            Ok(curve)
        } else {
            Ok(curve.with_jumps(jumps.to_vec()))
        }
    }

    /// Bootstrap a curve with JumpPillar definitions.
    ///
    /// This method converts JumpPillars to curve-compatible format and
    /// bootstraps a yield curve with the jump information attached.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Slice of calibration instruments
    /// * `jump_pillars` - Slice of JumpPillar definitions
    /// * `valuation_date` - Valuation date for year fraction calculation
    /// * `day_counter` - Day count convention for year fraction calculation
    ///
    /// # Returns
    ///
    /// A `BootstrappedCurve` with jump data attached.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pricer_models::builder::{CurveBootstrapper, CalibrationInstrument};
    /// use pricer_models::market::curves::MarketInstrument;
    /// use infra_domain::market::definition::JumpPillar;
    /// use infra_domain::time::{Date, DayCounter};
    ///
    /// let valuation = Date::from_ymd(2024, 1, 1).unwrap();
    /// let jump = JumpPillar::new(
    ///     Date::from_ymd(2024, 6, 12).unwrap(),
    ///     25.0,  // 25 bps
    ///     0.8,   // 80% confidence
    /// );
    ///
    /// let instruments = vec![
    ///     MarketInstrument::ois(1.0, 0.03),
    ///     MarketInstrument::ois(2.0, 0.035),
    /// ];
    ///
    /// let bootstrapper = CurveBootstrapper::new();
    /// let curve = bootstrapper.bootstrap_to_curve_with_jump_pillars(
    ///     &instruments,
    ///     &[jump],
    ///     valuation,
    ///     DayCounter::Actual365Fixed,
    /// ).unwrap();
    ///
    /// assert!(curve.has_jumps());
    /// ```
    pub fn bootstrap_to_curve_with_jump_pillars<I>(
        &self,
        instruments: &[I],
        jump_pillars: &[infra_domain::market::definition::JumpPillar],
        valuation_date: infra_domain::time::Date,
        day_counter: infra_domain::time::DayCounter,
    ) -> Result<BootstrappedCurve<f64>, BootstrapError>
    where
        I: CalibrationInstrument<f64> + Clone,
    {
        use crate::market::jumps::convert_jump_pillars_to_tuples;

        // Convert JumpPillars to (time, cumulative_offset) tuples
        let jumps = convert_jump_pillars_to_tuples(jump_pillars, valuation_date, day_counter);

        // Debug logging (if enabled in the future)
        #[cfg(feature = "debug-logging")]
        {
            if !jumps.is_empty() {
                eprintln!(
                    "[CurveBootstrapper] Applying {} jumps to curve",
                    jumps.len()
                );
                for (t, offset) in &jumps {
                    eprintln!(
                        "  - Jump at t={:.4}: cumulative_offset={:.6} ({:.2} bps)",
                        t,
                        offset,
                        offset * 10000.0
                    );
                }
            }
        }

        self.bootstrap_to_curve_with_jumps(instruments, &jumps)
    }
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
            .with_interpolation(BootstrapInterpolation::LogLinear)
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

    // =========================================================================
    // Jump-Aware Bootstrap Tests
    // =========================================================================

    #[test]
    fn test_bootstrap_to_curve_with_jumps_no_jumps() {
        // When no jumps are provided, should behave same as regular bootstrap
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let curve = bootstrapper.bootstrap_to_curve_with_jumps(&instruments, &[]);

        assert!(curve.is_ok(), "Bootstrap failed: {:?}", curve.err());
        let curve = curve.unwrap();

        assert!(!curve.has_jumps());
        let df_1y = curve.discount_factor(1.0).unwrap();
        assert!(df_1y > 0.0 && df_1y < 1.0);
    }

    #[test]
    fn test_bootstrap_to_curve_with_jumps_single_jump() {
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
        ];

        // Single jump: 25 bps at t=0.5
        let jumps: Vec<(f64, f64)> = vec![(0.5, -0.0025)];

        let bootstrapper = CurveBootstrapper::new();
        let curve = bootstrapper.bootstrap_to_curve_with_jumps(&instruments, &jumps);

        assert!(curve.is_ok(), "Bootstrap failed: {:?}", curve.err());
        let curve = curve.unwrap();

        assert!(curve.has_jumps());
        assert_eq!(curve.jumps().len(), 1);

        // Left and right limits at jump should differ
        use pricer_core::types::Limit;
        let df_left = curve.discount_factor_with_limit(0.5, Limit::Left).unwrap();
        let df_right = curve.discount_factor_with_limit(0.5, Limit::Right).unwrap();
        assert!(df_right < df_left);
    }

    #[test]
    fn test_bootstrap_to_curve_with_jumps_multiple_jumps() {
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
            MarketInstrument::ois(5.0, 0.04),
        ];

        // Two jumps: 25 bps at t=0.25, another 25 bps at t=0.75
        let jumps: Vec<(f64, f64)> = vec![
            (0.25, -0.0025), // First jump cumulative
            (0.75, -0.005),  // Second jump cumulative (including first)
        ];

        let bootstrapper = CurveBootstrapper::new();
        let curve = bootstrapper.bootstrap_to_curve_with_jumps(&instruments, &jumps);

        assert!(curve.is_ok(), "Bootstrap failed: {:?}", curve.err());
        let curve = curve.unwrap();

        assert!(curve.has_jumps());
        assert_eq!(curve.jumps().len(), 2);

        // After all jumps, should have cumulative offset
        let df = curve.discount_factor(1.0).unwrap();
        assert!(df > 0.0 && df < 1.0);
    }

    #[test]
    fn test_bootstrap_to_curve_with_jumps_backward_compatible() {
        // Ensure instruments without jumps still price correctly
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
        ];

        let bootstrapper = CurveBootstrapper::new();

        // Regular bootstrap
        let curve_regular = bootstrapper.bootstrap_to_curve(&instruments).unwrap();

        // Jump-aware bootstrap with no jumps
        let curve_with_jumps = bootstrapper
            .bootstrap_to_curve_with_jumps(&instruments, &[])
            .unwrap();

        // Discount factors should match
        let df1_reg = curve_regular.discount_factor(1.0).unwrap();
        let df1_jump = curve_with_jumps.discount_factor(1.0).unwrap();
        assert_relative_eq!(df1_reg, df1_jump, epsilon = 1e-10);

        let df2_reg = curve_regular.discount_factor(2.0).unwrap();
        let df2_jump = curve_with_jumps.discount_factor(2.0).unwrap();
        assert_relative_eq!(df2_reg, df2_jump, epsilon = 1e-10);
    }

    #[test]
    fn test_bootstrap_to_curve_with_jumps_structure() {
        // Verify that jumps are correctly applied to the curve structure.
        // Note: With jumps applied AFTER bootstrap, the instruments will NOT
        // reprice exactly because the jumps modify the discount factors.
        // This is expected behavior - the curve captures the jump structure
        // while maintaining a calibrated base curve.
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
        ];

        let jumps: Vec<(f64, f64)> = vec![(0.5, -0.0025)];

        let bootstrapper = CurveBootstrapper::new();
        let curve = bootstrapper
            .bootstrap_to_curve_with_jumps(&instruments, &jumps)
            .unwrap();

        // Verify jump structure is in place
        assert!(curve.has_jumps());
        assert_eq!(curve.jumps().len(), 1);
        assert_eq!(curve.jumps()[0], (0.5, -0.0025));

        // Verify discount factors are affected by the jump
        use pricer_core::types::Limit;
        let df_at_jump_left = curve.discount_factor_with_limit(0.5, Limit::Left).unwrap();
        let df_at_jump_right = curve.discount_factor_with_limit(0.5, Limit::Right).unwrap();

        // Right limit should be lower due to negative offset
        assert!(df_at_jump_right < df_at_jump_left);
    }

    #[test]
    fn test_bootstrap_to_curve_with_jumps_from_definition() {
        use infra_domain::{
            market::definition::JumpPillar,
            time::{Date, DayCounter},
        };

        let valuation = Date::from_ymd(2024, 1, 1).unwrap();

        // Create a simple curve definition with a jump
        let jump = JumpPillar::new(
            Date::from_ymd(2024, 6, 12).unwrap(), // ~0.45 years
            25.0,                                 // 25 bps
            0.8,                                  // 80% confidence
        );

        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let curve = bootstrapper.bootstrap_to_curve_with_jump_pillars(
            &instruments,
            &[jump],
            valuation,
            DayCounter::Actual365Fixed,
        );

        assert!(curve.is_ok(), "Bootstrap failed: {:?}", curve.err());
        let curve = curve.unwrap();

        assert!(curve.has_jumps());
        assert_eq!(curve.jumps().len(), 1);
    }

    #[test]
    fn test_bootstrap_to_curve_with_jumps_empty_instruments() {
        let instruments: Vec<MarketInstrument<f64>> = vec![];
        let jumps: Vec<(f64, f64)> = vec![(0.5, -0.0025)];

        let bootstrapper = CurveBootstrapper::new();
        let result = bootstrapper.bootstrap_to_curve_with_jumps(&instruments, &jumps);

        assert!(result.is_err());
        match result.unwrap_err() {
            BootstrapError::InsufficientData { .. } => {}
            other => panic!("Expected InsufficientData error, got {:?}", other),
        }
    }

    // =========================================================================
    // Finite-Difference Jacobian Tests
    // =========================================================================

    #[test]
    fn test_fd_jacobian_dimensions() {
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
            MarketInstrument::ois(5.0, 0.04),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let jacobian = bootstrapper.compute_fd_jacobian(&instruments).unwrap();

        assert_eq!(jacobian.size, 3);
        assert_eq!(jacobian.data.len(), 3);
        for row in &jacobian.data {
            assert_eq!(row.len(), 3);
        }
    }

    #[test]
    fn test_fd_jacobian_lower_triangular() {
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
            MarketInstrument::ois(5.0, 0.04),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let jacobian = bootstrapper.compute_fd_jacobian(&instruments).unwrap();

        // Upper triangle should be exactly zero
        for i in 0..jacobian.size {
            for j in (i + 1)..jacobian.size {
                assert_eq!(
                    jacobian.data[i][j], 0.0,
                    "Upper triangle [{i}][{j}] should be zero, got {}",
                    jacobian.data[i][j]
                );
            }
        }
    }

    #[test]
    fn test_fd_jacobian_diagonal_nonzero() {
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
            MarketInstrument::ois(5.0, 0.04),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let jacobian = bootstrapper.compute_fd_jacobian(&instruments).unwrap();

        // Diagonal entries should be nonzero (log DF_i depends on r_i)
        for i in 0..jacobian.size {
            assert!(
                jacobian.data[i][i].abs() > 1e-10,
                "Diagonal [{i}][{i}] should be nonzero, got {}",
                jacobian.data[i][i]
            );
        }
    }

    #[test]
    fn test_fd_jacobian_diagonal_negative() {
        // When rates go up, log(DF) goes down: d(log DF)/dr < 0
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
            MarketInstrument::ois(5.0, 0.04),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let jacobian = bootstrapper.compute_fd_jacobian(&instruments).unwrap();

        for i in 0..jacobian.size {
            assert!(
                jacobian.data[i][i] < 0.0,
                "Diagonal [{i}][{i}] should be negative (d(log DF)/dr < 0), got {}",
                jacobian.data[i][i]
            );
        }
    }

    #[test]
    fn test_bootstrap_to_curve_with_jacobian() {
        let instruments = vec![
            MarketInstrument::ois(1.0, 0.03),
            MarketInstrument::ois(2.0, 0.035),
            MarketInstrument::ois(5.0, 0.04),
        ];

        let bootstrapper = CurveBootstrapper::new();
        let (curve, jacobian) = bootstrapper
            .bootstrap_to_curve_with_jacobian(&instruments, &[])
            .unwrap();

        // Curve should be valid
        let df_1y = curve.discount_factor(1.0).unwrap();
        assert!(df_1y > 0.0 && df_1y < 1.0);

        // Jacobian should match instrument count
        assert_eq!(jacobian.size, 3);
    }
}
