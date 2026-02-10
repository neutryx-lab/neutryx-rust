use num_traits::Float;
use pricer_core::{
    math::linalg::{lu_solve, DMatrix, LinearAlgebraError, RealField},
    types::SolverError,
};

use crate::{
    builder::{
        jump::JumpPillar,
        CalibrationInstrument, CalibrationProblem, CalibrationProblemConfig,
    },
    market::curves::BootstrappedCurve,
};

use super::{
    config::GlobalBootstrapConfig,
    result::GlobalBootstrapResult,
    vector_norm,
};

// =============================================================================
// Global Bootstrapper
// =============================================================================

/// Global curve bootstrapper using multi-dimensional Newton-Raphson.
///
/// Solves the system F(x) = 0 where:
/// - x = log(DF) at each pillar (ensures DF > 0)
/// - F_i = pricing_error(instrument_i, curve)
#[derive(Debug, Clone)]
pub struct GlobalBootstrapper<T: Float> {
    config: GlobalBootstrapConfig<T>,
}

impl<T: RealField + Float + Copy> GlobalBootstrapper<T> {
    /// Create a new global bootstrapper with the given configuration.
    pub fn new(config: GlobalBootstrapConfig<T>) -> Self { Self { config } }

    /// Calibrate using the unified CalibrationEngine.
    ///
    /// This method provides an alternative implementation using the
    /// `CalibrationEngine<LUStrategy>`, which shares the same linear
    /// algebra infrastructure with sequential bootstrap.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Market instruments to calibrate
    ///
    /// # Returns
    ///
    /// `GlobalBootstrapResult` compatible with existing code.
    pub fn calibrate_with_engine<I: CalibrationInstrument<T> + Clone>(
        &self,
        instruments: &[I],
    ) -> Result<GlobalBootstrapResult<T>, SolverError> {
        use super::super::super::engine::{CalibrationEngine, CalibrationEngineConfig};

        let engine_config = CalibrationEngineConfig {
            tolerance: self.config.tolerance,
            param_tolerance: self.config.param_tolerance,
            max_iterations: self.config.max_iterations,
            jacobian_epsilon: self.config.jacobian_epsilon,
            store_jacobian_inverse: self.config.store_jacobian_inverse,
            interpolation: self.config.interpolation,
            allow_extrapolation: self.config.allow_extrapolation,
            damping_factor: self.config.damping_factor,
            debug_logging: self.config.debug_logging,
        };

        let mut engine = CalibrationEngine::with_lu_strategy(engine_config);

        let result = engine.calibrate(instruments).map_err(|e| {
            SolverError::NumericalInstability(format!("CalibrationEngine failed: {e}"))
        })?;

        Ok(GlobalBootstrapResult::from_calibration_result(result))
    }

    /// Calibrate with jumps using the unified CalibrationEngine.
    ///
    /// This method provides an alternative implementation for jump calibration
    /// using `CalibrationEngine<LUStrategy>`.
    pub fn calibrate_with_jumps_engine<I: CalibrationInstrument<T> + Clone>(
        &self,
        instruments: &[I],
        jump_pillars: Vec<JumpPillar<T>>,
    ) -> Result<GlobalBootstrapResult<T>, SolverError> {
        use super::super::super::engine::{CalibrationEngine, CalibrationEngineConfig};

        let engine_config = CalibrationEngineConfig {
            tolerance: self.config.tolerance,
            param_tolerance: self.config.param_tolerance,
            max_iterations: self.config.max_iterations,
            jacobian_epsilon: self.config.jacobian_epsilon,
            store_jacobian_inverse: self.config.store_jacobian_inverse,
            interpolation: self.config.interpolation,
            allow_extrapolation: self.config.allow_extrapolation,
            damping_factor: self.config.damping_factor,
            debug_logging: self.config.debug_logging,
        };

        let mut engine = CalibrationEngine::with_lu_strategy(engine_config);

        let result = engine
            .calibrate_with_jumps(instruments, jump_pillars)
            .map_err(|e| {
                SolverError::NumericalInstability(format!(
                    "CalibrationEngine with jumps failed: {e}"
                ))
            })?;

        Ok(GlobalBootstrapResult::from_calibration_result(result))
    }

    /// Create a bootstrapper with default configuration.
    pub fn with_defaults() -> Self { Self::new(GlobalBootstrapConfig::default()) }

    /// Get the configuration.
    pub fn config(&self) -> &GlobalBootstrapConfig<T> { &self.config }

    /// Calibrate a yield curve from the given instruments.
    pub fn calibrate<I: CalibrationInstrument<T>>(
        &self,
        instruments: &[I],
    ) -> Result<GlobalBootstrapResult<T>, SolverError> {
        self.calibrate_impl(instruments, &[])
    }

    /// Calibrate a yield curve with a forward-rate-shift grid.
    ///
    /// This method uses the same forward-rate-shift model as the sequential
    /// bootstrapper: the shift grid is attached to the curve at each Newton
    /// step so that instruments are priced on the (base + shifts) curve.
    /// Only log(DF) parameters are calibrated; jump amplitudes are fixed
    /// by the shift grid.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Calibration instruments
    /// * `shift_grid` - Pre-computed `(time, cumulative_offset)` pairs from
    ///   `build_forward_rate_shift_grid`. Pass `&[]` for no shifts.
    pub fn calibrate_with_shift_grid<I: CalibrationInstrument<T>>(
        &self,
        instruments: &[I],
        shift_grid: &[(T, T)],
    ) -> Result<GlobalBootstrapResult<T>, SolverError> {
        self.calibrate_impl(instruments, shift_grid)
    }

    /// Internal calibration implementation.
    ///
    /// When `shift_grid` is non-empty, the forward-rate-shift data is
    /// attached to the curve at every Newton step so that instrument
    /// pricing accounts for jump discontinuities.
    fn calibrate_impl<I: CalibrationInstrument<T>>(
        &self,
        instruments: &[I],
        shift_grid: &[(T, T)],
    ) -> Result<GlobalBootstrapResult<T>, SolverError> {
        let n = instruments.len();
        if n == 0 {
            return Err(SolverError::NumericalInstability(
                "No instruments provided for calibration".to_string(),
            ));
        }

        // Extract pillars (maturities) from instruments
        let mut pillars: Vec<T> = instruments.iter().map(|i| i.maturity()).collect();
        pillars.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        pillars.dedup_by(|a, b| Float::abs(*a - *b) < pricer_core::math::numeric::from_f64::<T>(1e-10));

        let n_pillars = pillars.len();

        // Initial guess: log(DF) assuming flat 3% curve
        let mut x: Vec<T> = pillars
            .iter()
            .map(|&t| -(pricer_core::math::numeric::from_f64::<T>(0.03) * t))
            .collect();

        // Residual history for debugging
        let mut residual_history = if self.config.debug_logging {
            Some(Vec::with_capacity(self.config.max_iterations))
        } else {
            None
        };

        // Newton iteration
        for iter in 0..self.config.max_iterations {
            let discount_factors: Vec<T> = x.iter().map(|&xi| Float::exp(xi)).collect();
            let curve = self.build_curve_with_shifts(&pillars, &discount_factors, shift_grid)?;

            let residuals = self.compute_residuals(instruments, &curve)?;
            let residual_norm = vector_norm(&residuals);

            if let Some(ref mut history) = residual_history {
                history.push(residual_norm);
            }

            // Check convergence
            if residual_norm < self.config.tolerance {
                let j_vecs = self.compute_jacobian_impl(&x, &pillars, instruments, shift_grid)?;
                let j_matrix =
                    DMatrix::from_row_slice(n, n_pillars, &self.flatten_jacobian(&j_vecs));

                // For overdetermined systems (n > n_pillars), compute the
                // normal-equation matrix (J^T J)^{-1} which maps rate changes
                // to parameter changes. For square systems, compute J^{-1}.
                let (jacobian_inverse, condition_number) = if self.config.store_jacobian_inverse {
                    if n == n_pillars {
                        let inv = self.compute_inverse(&j_matrix)?;
                        let cond = self.estimate_condition_number(&j_matrix);
                        (Some(inv), cond)
                    } else {
                        let jtj = j_matrix.transpose() * &j_matrix;
                        let inv = self.compute_inverse(&jtj).ok();
                        let cond = inv
                            .as_ref()
                            .and_then(|_| self.estimate_condition_number(&jtj));
                        (inv, cond)
                    }
                } else {
                    (None, self.estimate_condition_number(&j_matrix))
                };

                // Return the **base** curve (without shifts).
                // The caller attaches the shift grid via
                // `BootstrappedCurve::with_jumps()` for display.
                let base_curve = self.build_curve(&pillars, &discount_factors)?;
                return Ok(GlobalBootstrapResult {
                    curve: base_curve,
                    pillars: pillars.clone(),
                    discount_factors,
                    residual_norm,
                    iterations: iter,
                    converged: true,
                    jacobian_inverse,
                    residual_history,
                    condition_number,
                    pricing_errors: Some(residuals),
                    realised_jumps: None,
                });
            }

            // Compute Jacobian
            let j = self.compute_jacobian_impl(&x, &pillars, instruments, shift_grid)?;

            // Solve J * delta = -F for delta.
            // For overdetermined systems (n > n_pillars), use least squares
            // via normal equations: (J^T J) delta = J^T (-F).
            let neg_residuals: Vec<T> = residuals.iter().map(|&r| -r).collect();
            let j_matrix = DMatrix::from_row_slice(n, n_pillars, &self.flatten_jacobian(&j));
            let delta = if n == n_pillars {
                self.solve_linear_system(&j_matrix, &neg_residuals)?
            } else {
                self.solve_least_squares(&j_matrix, &neg_residuals)?
            };

            // Check parameter convergence
            let param_change = vector_norm(&delta);
            if param_change < self.config.param_tolerance {
                let (jacobian_inverse, condition_number) = if self.config.store_jacobian_inverse {
                    if n == n_pillars {
                        let inv = self.compute_inverse(&j_matrix)?;
                        let cond = self.estimate_condition_number(&j_matrix);
                        (Some(inv), cond)
                    } else {
                        let jtj = j_matrix.transpose() * &j_matrix;
                        let inv = self.compute_inverse(&jtj).ok();
                        let cond = inv
                            .as_ref()
                            .and_then(|_| self.estimate_condition_number(&jtj));
                        (inv, cond)
                    }
                } else {
                    (None, self.estimate_condition_number(&j_matrix))
                };

                let base_curve = self.build_curve(&pillars, &discount_factors)?;
                return Ok(GlobalBootstrapResult {
                    curve: base_curve,
                    pillars: pillars.clone(),
                    discount_factors,
                    residual_norm,
                    iterations: iter,
                    converged: true,
                    jacobian_inverse,
                    residual_history,
                    condition_number,
                    pricing_errors: Some(residuals),
                    realised_jumps: None,
                });
            }

            // Update x
            for (i, d) in delta.iter().enumerate() {
                x[i] = x[i] + *d;
            }
        }

        Err(SolverError::MaxIterationsExceeded {
            iterations: self.config.max_iterations,
        })
    }

    /// Estimate the condition number of a matrix.
    fn estimate_condition_number(&self, j: &DMatrix<T>) -> Option<T> {
        let nrows = j.nrows();
        if nrows == 0 {
            return None;
        }

        let mut max_row_sum = T::zero();
        let mut min_row_sum = T::infinity();

        for i in 0..nrows {
            let row_sum = (0..j.ncols())
                .map(|k| Float::abs(j[(i, k)]))
                .fold(T::zero(), |acc, x| acc + x);
            if row_sum > max_row_sum {
                max_row_sum = row_sum;
            }
            if row_sum < min_row_sum && row_sum > T::zero() {
                min_row_sum = row_sum;
            }
        }

        if min_row_sum > T::zero() {
            Some(max_row_sum / min_row_sum)
        } else {
            None
        }
    }

    /// Build a curve from pillars and discount factors.
    fn build_curve(
        &self,
        pillars: &[T],
        discount_factors: &[T],
    ) -> Result<BootstrappedCurve<T>, SolverError> {
        BootstrappedCurve::new(
            pillars.to_vec(),
            discount_factors.to_vec(),
            self.config.interpolation,
            self.config.allow_extrapolation,
        )
        .map_err(SolverError::NumericalInstability)
    }

    /// Build a curve with optional forward-rate-shift data.
    ///
    /// When `shift_grid` is non-empty, the resulting curve includes jump
    /// adjustments so that instrument pricing accounts for rate
    /// discontinuities at central bank meeting dates.
    fn build_curve_with_shifts(
        &self,
        pillars: &[T],
        discount_factors: &[T],
        shift_grid: &[(T, T)],
    ) -> Result<BootstrappedCurve<T>, SolverError> {
        let curve = self.build_curve(pillars, discount_factors)?;
        if shift_grid.is_empty() {
            Ok(curve)
        } else {
            Ok(curve.with_jumps(shift_grid.to_vec()))
        }
    }

    /// Compute the residual vector (pricing errors).
    fn compute_residuals<I: CalibrationInstrument<T>>(
        &self,
        instruments: &[I],
        curve: &BootstrappedCurve<T>,
    ) -> Result<Vec<T>, SolverError> {
        instruments
            .iter()
            .map(|instr| {
                instr
                    .pricing_error(curve)
                    .map_err(|e| SolverError::NumericalInstability(format!("{e}")))
            })
            .collect()
    }

    /// Compute the Jacobian matrix via finite differences.
    /// Compute the Jacobian matrix with optional forward-rate-shift data.
    fn compute_jacobian_impl<I: CalibrationInstrument<T>>(
        &self,
        x: &[T],
        pillars: &[T],
        instruments: &[I],
        shift_grid: &[(T, T)],
    ) -> Result<Vec<Vec<T>>, SolverError> {
        let n = instruments.len();
        let m = pillars.len();
        let eps = self.config.jacobian_epsilon;

        let discount_factors: Vec<T> = x.iter().map(|&xi| Float::exp(xi)).collect();
        let curve = self.build_curve_with_shifts(pillars, &discount_factors, shift_grid)?;
        let f0 = self.compute_residuals(instruments, &curve)?;

        let mut jacobian = vec![vec![T::zero(); m]; n];

        for j in 0..m {
            let mut x_pert = x.to_vec();
            x_pert[j] = x_pert[j] + eps;

            let df_pert: Vec<T> = x_pert.iter().map(|&xi| Float::exp(xi)).collect();
            let curve_pert = self.build_curve_with_shifts(pillars, &df_pert, shift_grid)?;
            let f_pert = self.compute_residuals(instruments, &curve_pert)?;

            for i in 0..n {
                jacobian[i][j] = (f_pert[i] - f0[i]) / eps;
            }
        }

        Ok(jacobian)
    }

    /// Flatten the Jacobian for matrix construction.
    fn flatten_jacobian(&self, j: &[Vec<T>]) -> Vec<T> {
        j.iter().flat_map(|row| row.iter().copied()).collect()
    }

    /// Solve the linear system J * x = b.
    fn solve_linear_system(&self, j: &DMatrix<T>, b: &[T]) -> Result<Vec<T>, SolverError> {
        lu_solve(j, b).map_err(|e: LinearAlgebraError| e.into())
    }

    /// Compute the inverse of the Jacobian matrix.
    fn compute_inverse(&self, j: &DMatrix<T>) -> Result<DMatrix<T>, SolverError> {
        pricer_core::math::linalg::inverse(j).map_err(|e: LinearAlgebraError| e.into())
    }

    /// Calibrate using the CalibrationProblem approach.
    pub fn calibrate_with_problem<I>(
        &self,
        instruments: Vec<I>,
    ) -> Result<GlobalBootstrapResult<T>, SolverError>
    where
        I: CalibrationInstrument<T> + Clone,
    {
        use pricer_core::math::solvers::{MultidimNewtonConfig, MultidimensionalNewtonSolver};

        let problem_config = CalibrationProblemConfig::from(&self.config);
        let problem = CalibrationProblem::with_config(instruments.clone(), problem_config)
            .map_err(|e| {
                SolverError::NumericalInstability(format!("Problem creation failed: {e}"))
            })?;

        let solver_config: MultidimNewtonConfig<T> = MultidimNewtonConfig {
            tolerance: self.config.tolerance,
            param_tolerance: self.config.param_tolerance,
            max_iterations: self.config.max_iterations,
            jacobian_epsilon: self.config.jacobian_epsilon,
            store_jacobian_inverse: self.config.store_jacobian_inverse,
        };

        let solver = MultidimensionalNewtonSolver::new(solver_config);

        let initial_guess = problem.initial_guess_vector();
        let result = solver.solve(&problem, initial_guess)?;

        let pillars = problem.pillars().to_vec();
        let log_df: Vec<T> = result.solution.iter().copied().collect();
        let discount_factors: Vec<T> = log_df.iter().map(|&x| Float::exp(x)).collect();

        let curve = self.build_curve(&pillars, &discount_factors)?;

        let pricing_errors = problem.compute_residuals(&curve).ok();

        Ok(GlobalBootstrapResult {
            curve,
            pillars,
            discount_factors,
            residual_norm: result.residual_norm,
            iterations: result.iterations,
            converged: result.converged,
            jacobian_inverse: result.jacobian_inverse,
            residual_history: None,
            condition_number: None,
            pricing_errors,
            realised_jumps: None,
        })
    }

    // =========================================================================
    // Jump-aware calibration methods
    // =========================================================================

    /// Calibrate a yield curve with jump pillars at CB meeting dates.
    ///
    /// This method extends the standard calibration to include jump parameters
    /// at central bank meeting dates. The parameter vector becomes:
    /// `[log(DF_1), ..., log(DF_n), jump_1, ..., jump_m]`
    ///
    /// # Arguments
    ///
    /// * `instruments` - Calibration instruments
    /// * `jump_pillars` - Jump pillars for CB meeting dates
    ///
    /// # Returns
    ///
    /// * `Ok(GlobalBootstrapResult)` - Calibration result with realised jumps
    /// * `Err(SolverError)` - If calibration fails
    pub fn calibrate_with_jumps<I>(
        &self,
        instruments: &[I],
        jump_pillars: Vec<JumpPillar<T>>,
    ) -> Result<GlobalBootstrapResult<T>, SolverError>
    where
        I: CalibrationInstrument<T> + Clone,
    {
        if instruments.is_empty() {
            return Err(SolverError::NumericalInstability(
                "No instruments provided for calibration".to_string(),
            ));
        }

        if jump_pillars.is_empty() {
            // No jumps, fall back to regular calibration
            return self.calibrate(instruments);
        }

        // Create CalibrationProblem with jumps
        let problem_config = CalibrationProblemConfig::from(&self.config);
        let problem = CalibrationProblem::with_jumps(
            instruments.to_vec(),
            jump_pillars.clone(),
            problem_config,
        )
        .map_err(|e| {
            SolverError::NumericalInstability(format!(
                "Failed to create calibration problem with jumps: {e}"
            ))
        })?;

        // Initial guess including jump parameters
        let mut x = problem.initial_guess_with_jumps();
        let n_pillars = problem.pillars().len();
        let n_jumps = problem.num_jumps();

        // Residual history for debugging
        let mut residual_history = if self.config.debug_logging {
            Some(Vec::with_capacity(self.config.max_iterations))
        } else {
            None
        };

        // Newton iteration with extended parameter vector
        for iter in 0..self.config.max_iterations {
            // Compute residuals
            let residuals = problem.compute_residuals_with_jumps(&x).map_err(|e| {
                SolverError::NumericalInstability(format!("Residual computation failed: {e}"))
            })?;
            let residual_norm = vector_norm(&residuals);

            if let Some(ref mut history) = residual_history {
                history.push(residual_norm);
            }

            // Debug logging
            if self.config.debug_logging {
                let jump_vals = problem.extract_jumps(&x);
                let jump_bps: Vec<f64> = jump_vals
                    .iter()
                    .map(|&j| JumpPillar::rate_to_bps(j).to_f64().unwrap_or(0.0))
                    .collect();
                eprintln!(
                    "[Jump Calibration] Iter {}: residual={:.6e}, jumps(bps)={:?}",
                    iter,
                    residual_norm.to_f64().unwrap_or(0.0),
                    jump_bps
                );
            }

            // Check convergence
            if residual_norm < self.config.tolerance {
                return self.finalize_jump_result(
                    &problem,
                    &x,
                    residual_norm,
                    iter,
                    residual_history,
                );
            }

            // Compute Jacobian with jumps
            let jacobian = problem.compute_jacobian_with_jumps(&x).map_err(|e| {
                SolverError::NumericalInstability(format!("Jacobian computation failed: {e}"))
            })?;

            // Solve J * delta = -F
            let neg_residuals: Vec<T> = residuals.iter().map(|&r| -r).collect();

            // The Jacobian is (n+k) × (n+k) after jump regularisation makes
            // the system square.  For any remaining non-square cases, fall
            // back to least squares via normal equations.
            let n_rows = jacobian.nrows();
            let n_cols = jacobian.ncols();
            let delta = if n_rows == n_cols {
                self.solve_linear_system(&jacobian, &neg_residuals)?
            } else {
                self.solve_least_squares(&jacobian, &neg_residuals)?
            };

            // Check parameter convergence
            let param_change = vector_norm(&delta);
            if param_change < self.config.param_tolerance {
                return self.finalize_jump_result(
                    &problem,
                    &x,
                    residual_norm,
                    iter,
                    residual_history,
                );
            }

            // Update parameters with optional damping for jumps
            let jump_damping = self
                .config
                .jump_config
                .as_ref()
                .and_then(|jc| jc.jump_damping)
                .unwrap_or_else(T::one);

            for i in 0..n_pillars {
                x[i] = x[i] + delta[i];
            }
            for i in 0..n_jumps {
                x[n_pillars + i] = x[n_pillars + i] + delta[n_pillars + i] * jump_damping;
            }
        }

        Err(SolverError::MaxIterationsExceeded {
            iterations: self.config.max_iterations,
        })
    }

    /// Finalise the calibration result with jump information.
    ///
    /// Returns a **base** curve (pillar DFs only, no jump adjustment).
    /// The caller must attach jump data via `BootstrappedCurve::with_jumps()`
    /// using the same forward-rate-shift grid as the sequential bootstrap so
    /// that forward rate discontinuities are rendered correctly.
    fn finalize_jump_result<I>(
        &self,
        problem: &CalibrationProblem<T, I>,
        params: &[T],
        residual_norm: T,
        iterations: usize,
        residual_history: Option<Vec<T>>,
    ) -> Result<GlobalBootstrapResult<T>, SolverError>
    where
        I: CalibrationInstrument<T> + Clone,
    {
        let log_df = problem.extract_log_df(params);
        let jumps = problem.extract_jumps(params);

        let pillars = problem.pillars().to_vec();
        let n = pillars.len();
        let discount_factors: Vec<T> = log_df.iter().map(|&x| Float::exp(x)).collect();

        // Build base curve (pillar DFs only, no jump adjustment).
        // Jump data is attached by the service layer using the same
        // forward-rate-shift grid as the sequential bootstrap.
        let curve = BootstrappedCurve::new(
            pillars.clone(),
            discount_factors.clone(),
            self.config.interpolation,
            true,
        )
        .map_err(SolverError::NumericalInstability)?;

        // Get realised jumps
        let realised_jumps = Some(problem.get_realised_jumps(params));

        // Compute pricing errors on the jump-adjusted curve (for diagnostics)
        let adjusted_curve = problem.build_curve_with_jumps(log_df, jumps).map_err(|e| {
            SolverError::NumericalInstability(format!("Failed to build jump curve: {e}"))
        })?;
        let pricing_errors = problem.compute_residuals(&adjusted_curve).ok();

        // Compute Jacobian inverse for the instrument-pillar sub-block.
        //
        // The full Jacobian has structure:
        //   [ J_inst_df (n_inst × n_pillars)    J_inst_jump (n_inst × k)    ]
        //   [ 0         (k × n_pillars)          I_reg       (k × k)          ]
        //
        // For square systems (n_inst == n_pillars), compute J_inst_df^{-1}.
        // For overdetermined systems (n_inst > n_pillars), compute
        // (J^T J)^{-1} which maps rate perturbations to parameter changes.
        let n_inst = problem.instruments().len();
        let jacobian_inverse = if self.config.store_jacobian_inverse {
            let full_jac = problem.compute_jacobian_with_jumps(params).map_err(|e| {
                SolverError::NumericalInstability(format!("Jacobian computation failed: {e}"))
            })?;
            let inst_jac = full_jac.view((0, 0), (n_inst, n)).into_owned();
            if n_inst == n {
                self.compute_inverse(&inst_jac).ok()
            } else {
                let jtj = inst_jac.transpose() * &inst_jac;
                self.compute_inverse(&jtj).ok()
            }
        } else {
            None
        };

        let condition_number = jacobian_inverse.as_ref().and_then(|_| {
            let full_jac = problem.compute_jacobian_with_jumps(params).ok()?;
            let inst_jac = full_jac.view((0, 0), (n_inst, n)).into_owned();
            if n_inst == n {
                self.estimate_condition_number(&inst_jac)
            } else {
                let jtj = inst_jac.transpose() * &inst_jac;
                self.estimate_condition_number(&jtj)
            }
        });

        Ok(GlobalBootstrapResult {
            curve,
            pillars,
            discount_factors,
            residual_norm,
            iterations,
            converged: true,
            jacobian_inverse,
            residual_history,
            condition_number,
            pricing_errors,
            realised_jumps,
        })
    }

    /// Solve a least squares problem for overdetermined systems.
    ///
    /// Solves J^T J x = J^T b using normal equations.
    fn solve_least_squares(&self, j: &DMatrix<T>, b: &[T]) -> Result<Vec<T>, SolverError> {
        let b_vec = DMatrix::from_column_slice(b.len(), 1, b);
        let jt = j.transpose();
        let jtj = &jt * j;
        let jtb = &jt * &b_vec;

        let jtb_vec: Vec<T> = jtb.iter().copied().collect();
        self.solve_linear_system(&jtj, &jtb_vec)
    }

    /// Merge regular pillars with jump pillars, avoiding duplicates.
    ///
    /// # Arguments
    ///
    /// * `regular_pillars` - Standard pillar times from instruments
    /// * `jump_pillars` - Jump pillar times
    /// * `tolerance` - Time tolerance for duplicate detection
    ///
    /// # Returns
    ///
    /// Tuple of (merged_pillars, jump_indices) where jump_indices are
    /// the positions of jump pillars in the merged array.
    pub fn merge_pillars(
        &self,
        regular_pillars: &[T],
        jump_pillars: &[JumpPillar<T>],
        tolerance: T,
    ) -> (Vec<T>, Vec<usize>) {
        let mut merged: Vec<T> = regular_pillars.to_vec();
        let mut jump_indices = Vec::with_capacity(jump_pillars.len());

        for jp in jump_pillars {
            // Check if this jump time is already in the merged list
            let existing_idx = merged
                .iter()
                .position(|&t| Float::abs(t - jp.time) < tolerance);

            match existing_idx {
                Some(idx) => {
                    // Jump pillar coincides with existing pillar
                    jump_indices.push(idx);
                }
                None => {
                    // Find insertion position to maintain sorted order
                    let insert_pos = merged
                        .iter()
                        .position(|&t| t > jp.time)
                        .unwrap_or(merged.len());

                    merged.insert(insert_pos, jp.time);

                    // Adjust indices for the insertion
                    for idx in &mut jump_indices {
                        if *idx >= insert_pos {
                            *idx += 1;
                        }
                    }
                    jump_indices.push(insert_pos);
                }
            }
        }

        (merged, jump_indices)
    }
}
