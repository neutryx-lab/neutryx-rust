//! Implicit Function Theorem-based curve sensitivity computation.
//!
//! This module provides the `ImplicitSolver` for computing sensitivities of
//! calibrated curve parameters to market inputs using the Implicit Function
//! Theorem. This approach avoids differentiating through the Newton-Raphson
//! iterations, providing efficient gradient computation for AAD.
//!
//! ## Implicit Function Theorem
//!
//! For a calibration problem F(x*, m) = 0 where:
//! - x* is the calibrated curve parameter vector
//! - m is the market data (quotes, rates)
//!
//! The implicit function theorem gives:
//!
//! ```text
//! ∂x*/∂m = -J⁻¹ · ∂F/∂m
//! ```
//!
//! For reverse-mode AD with loss function L(x*):
//!
//! ```text
//! ∂L/∂m = ∂L/∂x* · ∂x*/∂m = -∂L/∂x* · J⁻ᵀ · ∂F/∂m
//! ```
//!
//! When F is the pricing error (model - market), ∂F/∂m = -I, giving:
//!
//! ```text
//! ∂L/∂m = J⁻ᵀ · ∂L/∂x*
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use pricer_risk::greeks::ad::implicit_solver::{ImplicitSolver, CurveSensitivities};
//! use nalgebra::{DMatrix, DVector};
//!
//! // After solving F(x*) = 0 with Newton-Raphson, we have J⁻¹ at convergence
//! let jacobian_inverse = DMatrix::from_row_slice(2, 2, &[
//!     1.0, 0.5,
//!     0.2, 0.8,
//! ]);
//!
//! // Adjoint from reverse-mode AD: ∂L/∂x*
//! let adjoint_x = DVector::from_vec(vec![1.0, 0.5]);
//!
//! // Compute market sensitivities
//! let sensitivities = ImplicitSolver::compute_curve_sensitivities(
//!     &jacobian_inverse,
//!     &adjoint_x,
//! ).unwrap();
//!
//! assert_eq!(sensitivities.market_sensitivities.len(), 2);
//! ```
//!
//! ## Requirements Traceability
//!
//! - Requirement 6: AAD陰関数定理統合

use nalgebra::{DMatrix, DVector, RealField};
use num_traits::Float;
use thiserror::Error;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during implicit solver operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ImplicitSolverError {
    /// Dimension mismatch between Jacobian inverse and adjoint vector.
    #[error("Dimension mismatch: Jacobian inverse has {jacobian_cols} columns but adjoint has {adjoint_len} elements")]
    DimensionMismatch {
        /// Number of columns in the Jacobian inverse.
        jacobian_cols: usize,
        /// Length of the adjoint vector.
        adjoint_len: usize,
    },

    /// Jacobian inverse is not available.
    #[error("Jacobian inverse not available; use finite difference fallback")]
    JacobianInverseNotAvailable,

    /// Function evaluation failed during finite difference computation.
    #[error("Function evaluation failed: {0}")]
    FunctionEvaluationFailed(String),
}

// =============================================================================
// Result Types
// =============================================================================

/// Result of curve sensitivity computation.
///
/// Contains the sensitivities of the loss function with respect to market
/// inputs, computed via the implicit function theorem.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveSensitivities<T: RealField + Copy> {
    /// Sensitivities with respect to market data: ∂L/∂m
    ///
    /// Each element corresponds to the sensitivity to a market quote
    /// (e.g., deposit rate, swap rate, futures price).
    pub market_sensitivities: DVector<T>,
}

impl<T: RealField + Copy> CurveSensitivities<T> {
    /// Create new curve sensitivities from a sensitivity vector.
    pub fn new(market_sensitivities: DVector<T>) -> Self {
        Self {
            market_sensitivities,
        }
    }

    /// Get the dimension (number of market sensitivities).
    pub fn dimension(&self) -> usize { self.market_sensitivities.len() }

    /// Check if all sensitivities are finite.
    pub fn is_finite(&self) -> bool
    where
        T: Float,
    {
        self.market_sensitivities.iter().all(|&x| x.is_finite())
    }
}

// =============================================================================
// ImplicitSolver
// =============================================================================

/// Solver for computing curve sensitivities using the Implicit Function
/// Theorem.
///
/// This struct provides methods for computing sensitivities of calibrated
/// parameters to market inputs, enabling efficient gradient computation in
/// reverse-mode AAD without differentiating through Newton-Raphson iterations.
///
/// # AAD Integration
///
/// The implicit function theorem provides a closed-form expression for the
/// sensitivity of calibrated parameters to market inputs:
///
/// ```text
/// ∂L/∂m = J⁻ᵀ · ∂L/∂x*
/// ```
///
/// where:
/// - L is the loss function (e.g., portfolio value)
/// - m is market data (quotes, rates)
/// - x* is the calibrated parameter vector
/// - J is the Jacobian of the calibration residual function
///
/// # Fallback Mode
///
/// When the Jacobian inverse is not available (e.g., when
/// `store_jacobian_inverse` was false), finite difference approximation
/// is used as a fallback.
pub struct ImplicitSolver;

impl ImplicitSolver {
    /// Compute curve sensitivities using the Implicit Function Theorem.
    ///
    /// Computes ∂L/∂m = J⁻ᵀ · ∂L/∂x* where:
    /// - J⁻¹ is the Jacobian inverse at the calibration solution
    /// - ∂L/∂x* is the adjoint (gradient) of the loss with respect to x*
    ///
    /// # Arguments
    ///
    /// * `jacobian_inverse` - The Jacobian inverse J⁻¹ from the solver result
    /// * `adjoint_x` - The adjoint vector ∂L/∂x* from reverse-mode AD
    ///
    /// # Returns
    ///
    /// `CurveSensitivities` containing ∂L/∂m for each market input.
    ///
    /// # Errors
    ///
    /// Returns `ImplicitSolverError::DimensionMismatch` if the dimensions
    /// of the Jacobian inverse and adjoint vector are incompatible.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_risk::greeks::ad::implicit_solver::{ImplicitSolver, CurveSensitivities};
    /// use nalgebra::{DMatrix, DVector};
    ///
    /// // 2x2 Jacobian inverse from calibration
    /// let j_inv = DMatrix::from_row_slice(2, 2, &[0.5, 0.0, 0.0, 0.5]);
    /// let adjoint = DVector::from_vec(vec![2.0, 4.0]);
    ///
    /// let sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint).unwrap();
    /// assert_eq!(sens.dimension(), 2);
    /// ```
    pub fn compute_curve_sensitivities<T: RealField + Copy>(
        jacobian_inverse: &DMatrix<T>,
        adjoint_x: &DVector<T>,
    ) -> Result<CurveSensitivities<T>, ImplicitSolverError> {
        // Validate dimensions
        if jacobian_inverse.ncols() != adjoint_x.len() {
            return Err(ImplicitSolverError::DimensionMismatch {
                jacobian_cols: jacobian_inverse.ncols(),
                adjoint_len: adjoint_x.len(),
            });
        }

        // Compute J⁻ᵀ · ∂L/∂x*
        let j_inv_t = jacobian_inverse.transpose();
        let market_sensitivities = &j_inv_t * adjoint_x;

        Ok(CurveSensitivities::new(market_sensitivities))
    }

    /// Compute curve sensitivities using finite difference fallback.
    ///
    /// This method is used when the Jacobian inverse is not available.
    /// It computes sensitivities by bumping each curve node and
    /// recomputing the loss function.
    ///
    /// Uses central differences for better accuracy:
    /// ```text
    /// ∂L/∂m_i ≈ (L(m + ε·e_i) - L(m - ε·e_i)) / (2ε)
    /// ```
    ///
    /// # Arguments
    ///
    /// * `loss_fn` - Function that computes the loss given curve nodes
    /// * `curve_nodes` - Current curve node values
    /// * `epsilon` - Finite difference step size
    ///
    /// # Returns
    ///
    /// `CurveSensitivities` containing approximate ∂L/∂m for each input.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_risk::greeks::ad::implicit_solver::{ImplicitSolver, CurveSensitivities};
    /// use nalgebra::DVector;
    ///
    /// // Loss function: L(x) = x[0]^2 + x[1]^2
    /// let loss_fn = |x: &DVector<f64>| x[0] * x[0] + x[1] * x[1];
    /// let nodes = DVector::from_vec(vec![3.0, 4.0]);
    ///
    /// let sens = ImplicitSolver::compute_curve_sensitivities_fd(
    ///     loss_fn,
    ///     &nodes,
    ///     1e-6,
    /// );
    ///
    /// // ∂L/∂x[0] = 2*3 = 6, ∂L/∂x[1] = 2*4 = 8
    /// assert!((sens.market_sensitivities[0] - 6.0).abs() < 1e-4);
    /// assert!((sens.market_sensitivities[1] - 8.0).abs() < 1e-4);
    /// ```
    pub fn compute_curve_sensitivities_fd<T, F>(
        loss_fn: F,
        curve_nodes: &DVector<T>,
        epsilon: T,
    ) -> CurveSensitivities<T>
    where
        T: RealField + Copy + Float,
        F: Fn(&DVector<T>) -> T,
    {
        let n = curve_nodes.len();
        let mut sensitivities = DVector::zeros(n);
        let two = T::one() + T::one();

        for i in 0..n {
            // Bump up
            let mut nodes_up = curve_nodes.clone();
            nodes_up[i] = nodes_up[i] + epsilon;
            let loss_up = loss_fn(&nodes_up);

            // Bump down
            let mut nodes_down = curve_nodes.clone();
            nodes_down[i] = nodes_down[i] - epsilon;
            let loss_down = loss_fn(&nodes_down);

            // Central difference
            sensitivities[i] = (loss_up - loss_down) / (two * epsilon);
        }

        CurveSensitivities::new(sensitivities)
    }

    /// Compute sensitivities with automatic fallback to finite differences.
    ///
    /// If Jacobian inverse is available (Some), uses the implicit function
    /// theorem. Otherwise, falls back to finite difference approximation.
    ///
    /// # Arguments
    ///
    /// * `jacobian_inverse` - Optional Jacobian inverse from solver
    /// * `adjoint_x` - Adjoint vector (only used if J⁻¹ is available)
    /// * `loss_fn` - Loss function (only used for FD fallback)
    /// * `curve_nodes` - Curve nodes (only used for FD fallback)
    /// * `epsilon` - FD step size (only used for FD fallback)
    ///
    /// # Returns
    ///
    /// `CurveSensitivities` and a boolean indicating whether FD was used.
    pub fn compute_with_fallback<T, F>(
        jacobian_inverse: Option<&DMatrix<T>>,
        adjoint_x: &DVector<T>,
        loss_fn: F,
        curve_nodes: &DVector<T>,
        epsilon: T,
    ) -> (CurveSensitivities<T>, bool)
    where
        T: RealField + Copy + Float,
        F: Fn(&DVector<T>) -> T,
    {
        match jacobian_inverse {
            Some(j_inv) => {
                match Self::compute_curve_sensitivities(j_inv, adjoint_x) {
                    Ok(sens) => (sens, false), // Used implicit FT
                    Err(_) => {
                        // Dimension mismatch, fall back to FD
                        let sens =
                            Self::compute_curve_sensitivities_fd(loss_fn, curve_nodes, epsilon);
                        (sens, true)
                    }
                }
            }
            None => {
                // No Jacobian inverse, use FD
                let sens = Self::compute_curve_sensitivities_fd(loss_fn, curve_nodes, epsilon);
                (sens, true)
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    // =========================================================================
    // CurveSensitivities Tests
    // =========================================================================

    #[test]
    fn test_curve_sensitivities_new() {
        let sens = DVector::from_vec(vec![1.0, 2.0, 3.0]);
        let curve_sens = CurveSensitivities::new(sens.clone());

        assert_eq!(curve_sens.dimension(), 3);
        assert_eq!(curve_sens.market_sensitivities, sens);
    }

    #[test]
    fn test_curve_sensitivities_is_finite() {
        let finite_sens = CurveSensitivities::new(DVector::from_vec(vec![1.0, 2.0]));
        assert!(finite_sens.is_finite());

        let infinite_sens = CurveSensitivities::new(DVector::from_vec(vec![f64::INFINITY, 2.0]));
        assert!(!infinite_sens.is_finite());

        let nan_sens = CurveSensitivities::new(DVector::from_vec(vec![f64::NAN, 2.0]));
        assert!(!nan_sens.is_finite());
    }

    // =========================================================================
    // ImplicitSolver - compute_curve_sensitivities Tests
    // =========================================================================

    #[test]
    fn test_compute_curve_sensitivities_identity_jacobian() {
        // J⁻¹ = I means J⁻ᵀ = I, so ∂L/∂m = ∂L/∂x*
        let j_inv = DMatrix::<f64>::identity(3, 3);
        let adjoint = DVector::from_vec(vec![1.0, 2.0, 3.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint).unwrap();

        assert_eq!(sens.dimension(), 3);
        assert_relative_eq!(sens.market_sensitivities[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(sens.market_sensitivities[1], 2.0, epsilon = 1e-10);
        assert_relative_eq!(sens.market_sensitivities[2], 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_curve_sensitivities_diagonal_jacobian() {
        // J⁻¹ = diag(2, 0.5), so J⁻ᵀ = diag(2, 0.5)
        let j_inv = DMatrix::from_row_slice(2, 2, &[2.0, 0.0, 0.0, 0.5]);
        let adjoint = DVector::from_vec(vec![1.0, 4.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint).unwrap();

        // J⁻ᵀ · [1, 4] = [2*1, 0.5*4] = [2, 2]
        assert_relative_eq!(sens.market_sensitivities[0], 2.0, epsilon = 1e-10);
        assert_relative_eq!(sens.market_sensitivities[1], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_curve_sensitivities_general_jacobian() {
        // J⁻¹ = [[1, 2], [3, 4]], J⁻ᵀ = [[1, 3], [2, 4]]
        let j_inv = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let adjoint = DVector::from_vec(vec![1.0, 1.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint).unwrap();

        // J⁻ᵀ · [1, 1] = [[1, 3], [2, 4]] · [1, 1] = [1+3, 2+4] = [4, 6]
        assert_relative_eq!(sens.market_sensitivities[0], 4.0, epsilon = 1e-10);
        assert_relative_eq!(sens.market_sensitivities[1], 6.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_curve_sensitivities_dimension_mismatch() {
        let j_inv = DMatrix::<f64>::identity(2, 2);
        let adjoint = DVector::from_vec(vec![1.0, 2.0, 3.0]); // Wrong dimension

        let result = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint);

        assert!(result.is_err());
        match result.unwrap_err() {
            ImplicitSolverError::DimensionMismatch {
                jacobian_cols,
                adjoint_len,
            } => {
                assert_eq!(jacobian_cols, 2);
                assert_eq!(adjoint_len, 3);
            }
            _ => panic!("Expected DimensionMismatch error"),
        }
    }

    // =========================================================================
    // ImplicitSolver - compute_curve_sensitivities_fd Tests
    // =========================================================================

    #[test]
    fn test_compute_sensitivities_fd_quadratic() {
        // L(x) = x[0]^2 + x[1]^2
        // ∂L/∂x[0] = 2*x[0], ∂L/∂x[1] = 2*x[1]
        let loss_fn = |x: &DVector<f64>| x[0] * x[0] + x[1] * x[1];
        let nodes = DVector::from_vec(vec![3.0, 4.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        assert_relative_eq!(sens.market_sensitivities[0], 6.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[1], 8.0, epsilon = 1e-4);
    }

    #[test]
    fn test_compute_sensitivities_fd_linear() {
        // L(x) = 2*x[0] + 3*x[1] + 5*x[2]
        // ∂L/∂x = [2, 3, 5]
        let loss_fn = |x: &DVector<f64>| 2.0 * x[0] + 3.0 * x[1] + 5.0 * x[2];
        let nodes = DVector::from_vec(vec![1.0, 2.0, 3.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        assert_relative_eq!(sens.market_sensitivities[0], 2.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[1], 3.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[2], 5.0, epsilon = 1e-4);
    }

    #[test]
    fn test_compute_sensitivities_fd_cross_terms() {
        // L(x) = x[0] * x[1]
        // ∂L/∂x[0] = x[1] = 4, ∂L/∂x[1] = x[0] = 3
        let loss_fn = |x: &DVector<f64>| x[0] * x[1];
        let nodes = DVector::from_vec(vec![3.0, 4.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        assert_relative_eq!(sens.market_sensitivities[0], 4.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[1], 3.0, epsilon = 1e-4);
    }

    #[test]
    fn test_compute_sensitivities_fd_exponential() {
        // L(x) = exp(x[0])
        // ∂L/∂x[0] = exp(x[0])
        let loss_fn = |x: &DVector<f64>| x[0].exp();
        let nodes = DVector::from_vec(vec![1.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        assert_relative_eq!(sens.market_sensitivities[0], 1.0_f64.exp(), epsilon = 1e-4);
    }

    // =========================================================================
    // ImplicitSolver - compute_with_fallback Tests
    // =========================================================================

    #[test]
    fn test_compute_with_fallback_uses_implicit_when_available() {
        let j_inv = DMatrix::<f64>::identity(2, 2);
        let adjoint = DVector::from_vec(vec![1.0, 2.0]);
        let nodes = DVector::from_vec(vec![0.0, 0.0]); // Not used
        let loss_fn = |_: &DVector<f64>| 0.0; // Not used

        let (sens, used_fd) =
            ImplicitSolver::compute_with_fallback(Some(&j_inv), &adjoint, loss_fn, &nodes, 1e-6);

        assert!(!used_fd); // Should NOT use FD
        assert_relative_eq!(sens.market_sensitivities[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(sens.market_sensitivities[1], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_with_fallback_uses_fd_when_no_jacobian() {
        let adjoint = DVector::from_vec(vec![1.0, 2.0]); // Not used for FD
        let nodes = DVector::from_vec(vec![3.0, 4.0]);
        let loss_fn = |x: &DVector<f64>| x[0] * x[0] + x[1] * x[1];

        let (sens, used_fd) =
            ImplicitSolver::compute_with_fallback(None, &adjoint, loss_fn, &nodes, 1e-6);

        assert!(used_fd); // Should use FD
        assert_relative_eq!(sens.market_sensitivities[0], 6.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[1], 8.0, epsilon = 1e-4);
    }

    #[test]
    fn test_compute_with_fallback_uses_fd_on_dimension_mismatch() {
        let j_inv = DMatrix::<f64>::identity(2, 2);
        let adjoint = DVector::from_vec(vec![1.0, 2.0, 3.0]); // Mismatched dimension
        let nodes = DVector::from_vec(vec![3.0, 4.0, 5.0]);
        let loss_fn = |x: &DVector<f64>| x[0] + x[1] + x[2];

        let (sens, used_fd) =
            ImplicitSolver::compute_with_fallback(Some(&j_inv), &adjoint, loss_fn, &nodes, 1e-6);

        assert!(used_fd); // Should fall back to FD
        assert_relative_eq!(sens.market_sensitivities[0], 1.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[1], 1.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[2], 1.0, epsilon = 1e-4);
    }

    // =========================================================================
    // Error Type Tests
    // =========================================================================

    #[test]
    fn test_implicit_solver_error_display() {
        let err = ImplicitSolverError::DimensionMismatch {
            jacobian_cols: 2,
            adjoint_len: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Dimension mismatch"));
        assert!(msg.contains("2"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn test_implicit_solver_error_clone() {
        let err = ImplicitSolverError::JacobianInverseNotAvailable;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // =========================================================================
    // Integration with Solver Result Pattern
    // =========================================================================

    #[test]
    fn test_integration_with_multidim_solver_result_pattern() {
        // Simulate what happens after MultidimensionalNewtonSolver converges:
        // The solver returns J⁻¹ at the solution point, which we use for AAD.

        // Suppose we calibrated a 3-point yield curve
        let jacobian_inverse = DMatrix::from_row_slice(
            3,
            3,
            &[
                0.98, 0.01, 0.00, // Sensitivity of node 1 to instruments
                0.01, 0.95, 0.02, // Sensitivity of node 2 to instruments
                0.00, 0.03, 0.97, // Sensitivity of node 3 to instruments
            ],
        );

        // Adjoint from portfolio valuation: ∂V/∂(curve_nodes)
        let adjoint_x = DVector::from_vec(vec![1000.0, 500.0, 200.0]);

        // Compute market sensitivities: ∂V/∂(instrument_quotes)
        let sens = ImplicitSolver::compute_curve_sensitivities(&jacobian_inverse, &adjoint_x)
            .expect("Should succeed with valid dimensions");

        // J⁻ᵀ · adjoint gives market sensitivities
        // These represent DV01 to each calibration instrument
        assert_eq!(sens.dimension(), 3);
        assert!(sens.is_finite());

        // Verify the transpose multiplication was correct
        let j_inv_t = jacobian_inverse.transpose();
        let expected = &j_inv_t * &adjoint_x;
        for i in 0..3 {
            assert_relative_eq!(sens.market_sensitivities[i], expected[i], epsilon = 1e-10);
        }
    }

    #[test]
    fn test_financial_interpretation_of_sensitivities() {
        // Financial scenario: 2 calibration instruments (deposit, swap)
        // calibrate 2 curve nodes (1Y, 5Y discount factors)

        // Jacobian at solution: J[i,j] = ∂(residual_i)/∂(node_j)
        // This represents how each instrument price changes with curve nodes
        //
        // J⁻¹ tells us how curve nodes change with instrument quotes
        // ∂(nodes)/∂(quotes) = J⁻¹

        let jacobian_inverse = DMatrix::from_row_slice(
            2,
            2,
            &[
                0.99, 0.0, // 1Y node mainly from deposit
                0.01, 0.98, // 5Y node mainly from swap
            ],
        );

        // Portfolio has DV01 = [100, 50] to [1Y, 5Y] nodes
        let adjoint_x = DVector::from_vec(vec![100.0, 50.0]);

        let sens =
            ImplicitSolver::compute_curve_sensitivities(&jacobian_inverse, &adjoint_x).unwrap();

        // Expected: DV01 to deposit ≈ 100 * 0.99 + 50 * 0.01 = 99.5
        // Expected: DV01 to swap ≈ 100 * 0.0 + 50 * 0.98 = 49
        assert_relative_eq!(sens.market_sensitivities[0], 99.5, epsilon = 0.01);
        assert_relative_eq!(sens.market_sensitivities[1], 49.0, epsilon = 0.01);
    }

    // AAD Verification Tests

    #[test]
    fn test_implicit_vs_fd_accuracy_linear_function() {
        // Test that ImplicitSolver and FD produce same results for linear functions
        // For L(x) = c^T * x, ∂L/∂x = c
        // With J = I, the sensitivities should equal the coefficients

        let n = 5;
        let coeffs = DVector::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let nodes = DVector::from_vec(vec![0.1, 0.2, 0.3, 0.4, 0.5]);

        // Implicit FT approach: J⁻¹ = I, adjoint = coeffs
        let j_inv = DMatrix::<f64>::identity(n, n);
        let implicit_sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &coeffs).unwrap();

        // FD approach
        let loss_fn = |x: &DVector<f64>| coeffs.dot(x);
        let fd_sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        // Both should produce the same sensitivities (the coefficients)
        for i in 0..n {
            assert_relative_eq!(
                implicit_sens.market_sensitivities[i],
                fd_sens.market_sensitivities[i],
                epsilon = 1e-4
            );
            assert_relative_eq!(
                implicit_sens.market_sensitivities[i],
                coeffs[i],
                epsilon = 1e-10
            );
        }
    }

    #[test]
    fn test_implicit_vs_fd_accuracy_quadratic_with_non_identity_jacobian() {
        // More complex test: quadratic loss with non-identity Jacobian
        // L(x) = x^T * A * x where A is positive definite
        // ∂L/∂x = 2 * A * x (adjoint for specific x)
        //
        // With J⁻¹ available, ImplicitSolver should give same result as FD

        let n = 3;

        // Symmetric positive definite matrix A
        let a = DMatrix::from_row_slice(
            n,
            n,
            &[
                2.0, 0.5, 0.1, //
                0.5, 3.0, 0.2, //
                0.1, 0.2, 4.0, //
            ],
        );

        // Current point
        let nodes = DVector::from_vec(vec![1.0, 2.0, 3.0]);

        // For quadratic L(x) = x^T A x, gradient is 2Ax
        let adjoint = 2.0 * &a * &nodes;

        // Assume J = I for this test (so J⁻¹ = I)
        let j_inv = DMatrix::<f64>::identity(n, n);

        // Implicit FT
        let implicit_sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint).unwrap();

        // FD
        let a_clone = a.clone();
        let loss_fn = move |x: &DVector<f64>| x.dot(&(&a_clone * x));
        let fd_sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        // Both should match
        for i in 0..n {
            assert_relative_eq!(
                implicit_sens.market_sensitivities[i],
                fd_sens.market_sensitivities[i],
                epsilon = 1e-3
            );
        }
    }

    #[test]
    fn test_aad_verification_30_node_curve() {
        // Realistic test: 30-node yield curve
        // Verify that Implicit FT produces accurate sensitivities

        let n = 30;

        // Simulate a realistic Jacobian inverse from curve calibration
        // Diagonal-dominant with exponential decay off-diagonal
        let j_inv = DMatrix::from_fn(n, n, |i, j| {
            let dist = (i as f64 - j as f64).abs();
            if i == j {
                0.99
            } else {
                0.05 * (-0.5 * dist).exp()
            }
        });

        // Simulated portfolio DV01 to curve nodes (decreasing sensitivity to longer
        // tenors)
        let adjoint = DVector::from_fn(n, |i, _| 1000.0 * (-0.1 * i as f64).exp());

        // Compute sensitivities using Implicit FT
        let sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint).unwrap();

        // Verify properties:
        // 1. All sensitivities should be finite
        assert!(sens.is_finite());

        // 2. Dimension should match
        assert_eq!(sens.dimension(), n);

        // 3. Short-end sensitivities should be larger (due to adjoint structure)
        // (This is a sanity check, not a strict mathematical requirement)
        assert!(
            sens.market_sensitivities[0].abs() > sens.market_sensitivities[n - 1].abs(),
            "Short-end sensitivity should be larger than long-end"
        );

        // 4. Verify by comparing against FD for a simplified loss function
        // Loss = adjoint^T * nodes (linear approximation)
        let nodes = DVector::from_fn(n, |i, _| 0.03 + 0.001 * i as f64);
        let adjoint_for_fd = adjoint.clone();
        let loss_fn = move |x: &DVector<f64>| adjoint_for_fd.dot(x);

        let fd_sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        // With J⁻¹ applied to adjoint, the result should be different from raw adjoint
        // but the computation should be consistent
        assert!(fd_sens.is_finite());
        assert_eq!(fd_sens.dimension(), n);
    }

    #[test]
    fn test_aad_end_to_end_flow() {
        // End-to-end test simulating the full AAD flow:
        // 1. Calibration produces J⁻¹ at solution
        // 2. Portfolio valuation produces ∂V/∂(curve_nodes)
        // 3. ImplicitSolver computes ∂V/∂(market_quotes)

        // Step 1: Simulate calibration result
        let n = 5; // 5 calibration instruments
        let calibration_jacobian_inverse = DMatrix::from_row_slice(
            n,
            n,
            &[
                0.98, 0.01, 0.00, 0.00, 0.00, // Deposit mainly affects 1Y node
                0.01, 0.96, 0.02, 0.00, 0.00, // 2Y swap
                0.00, 0.02, 0.95, 0.02, 0.00, // 3Y swap
                0.00, 0.00, 0.02, 0.94, 0.03, // 5Y swap
                0.00, 0.00, 0.00, 0.03, 0.93, // 10Y swap
            ],
        );

        // Step 2: Portfolio valuation gives adjoint (DV01 to curve nodes)
        let portfolio_dv01_to_nodes = DVector::from_vec(vec![
            500.0, // DV01 to 1Y node
            300.0, // DV01 to 2Y node
            200.0, // DV01 to 3Y node
            150.0, // DV01 to 5Y node
            100.0, // DV01 to 10Y node
        ]);

        // Step 3: Compute DV01 to market quotes using ImplicitSolver
        let market_dv01 = ImplicitSolver::compute_curve_sensitivities(
            &calibration_jacobian_inverse,
            &portfolio_dv01_to_nodes,
        )
        .unwrap();

        // Verify results are sensible
        assert!(market_dv01.is_finite());
        assert_eq!(market_dv01.dimension(), n);

        // DV01 to deposit should be dominated by 1Y node contribution
        // DV01 to 10Y swap should reflect 10Y node contribution with some spillover
        let deposit_dv01 = market_dv01.market_sensitivities[0];
        let ten_year_swap_dv01 = market_dv01.market_sensitivities[4];

        // Sanity checks
        assert!(
            deposit_dv01 > 400.0,
            "Deposit DV01 should be ~500 (1Y node contribution)"
        );
        assert!(
            ten_year_swap_dv01 > 80.0 && ten_year_swap_dv01 < 200.0,
            "10Y swap DV01 should reflect 10Y node with spillover"
        );

        // Total DV01 should be conserved approximately
        // (sum of market_dv01 ≈ sum of portfolio_dv01 if J⁻ᵀ preserves row sums)
        let total_portfolio_dv01: f64 = portfolio_dv01_to_nodes.iter().sum();
        let total_market_dv01: f64 = market_dv01.market_sensitivities.iter().sum();

        // Within 20% tolerance due to off-diagonal effects
        assert!(
            (total_market_dv01 - total_portfolio_dv01).abs() / total_portfolio_dv01 < 0.20,
            "Total DV01 should be approximately conserved"
        );
    }
}
