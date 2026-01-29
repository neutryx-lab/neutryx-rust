//! Linear solve strategies for calibration problems.
//!
//! This module provides abstracted linear algebra strategies that allow
//! the same calibration algorithm to use different matrix structures:
//!
//! - **LU Strategy**: Full dense matrix with O(n^3) LU decomposition
//! - **Lower Triangular Strategy**: Exploits lower triangular structure for O(n^2)
//!   forward substitution (used by sequential bootstrap)
//!
//! ## Mathematical Background
//!
//! In curve calibration, we solve F(x) = 0 using Newton-Raphson:
//!
//! x_{k+1} = x_k - J(x_k)^{-1} * F(x_k)
//!
//! The Jacobian structure depends on the calibration approach:
//!
//! - **Global Bootstrap**: Dense Jacobian, requires full LU decomposition
//! - **Sequential Bootstrap**: Lower triangular Jacobian (when instruments
//!   are sorted by maturity), can use fast forward substitution
//!
//! ## AAD Integration
//!
//! Both strategies support storing the Jacobian inverse for use with the
//! implicit function theorem in AAD:
//!
//! dx*/dm = -J^{-1} * dF/dm
//!
//! where x* is the calibrated solution and m are market parameters.

use nalgebra::{DMatrix, RealField};
use num_traits::Float;

use super::{error::LinearAlgebraError, lu_solve};
use crate::math::numeric::from_f64;

// =============================================================================
// LinearSolveStrategy Trait
// =============================================================================

/// Strategy trait for linear system solvers in calibration problems.
///
/// Implementations handle different Jacobian matrix structures:
/// - Full dense matrices (LU decomposition)
/// - Lower triangular matrices (forward substitution)
/// - Block diagonal matrices (future extension)
///
/// # Type Parameters
///
/// * `T` - Floating-point type satisfying `RealField + Copy + Float`
pub trait LinearSolveStrategy<T: RealField + Copy + Float>: Clone + Default {
    /// Decompose the matrix and store internal state for later use.
    ///
    /// This method prepares the matrix for efficient solving and inverse
    /// computation. The exact operation depends on the strategy:
    /// - LU: Computes LU factorisation
    /// - Lower Triangular: Validates structure and stores matrix
    ///
    /// # Arguments
    ///
    /// * `matrix` - The Jacobian matrix to decompose
    ///
    /// # Errors
    ///
    /// Returns error if the matrix is singular or has invalid structure.
    fn decompose(&mut self, matrix: &DMatrix<T>) -> Result<(), LinearAlgebraError>;

    /// Solve the linear system M * x = b using the stored decomposition.
    ///
    /// # Arguments
    ///
    /// * `b` - Right-hand side vector
    ///
    /// # Returns
    ///
    /// Solution vector x
    ///
    /// # Errors
    ///
    /// Returns error if decompose() hasn't been called or system is singular.
    fn solve(&self, b: &[T]) -> Result<Vec<T>, LinearAlgebraError>;

    /// Compute the inverse of the stored matrix.
    ///
    /// Used for AAD sensitivity computation via implicit function theorem.
    ///
    /// # Returns
    ///
    /// The inverse matrix M^{-1}
    ///
    /// # Errors
    ///
    /// Returns error if decompose() hasn't been called or matrix is singular.
    fn inverse(&self) -> Result<DMatrix<T>, LinearAlgebraError>;

    /// Validate that the matrix has the expected structure.
    ///
    /// Override this for strategies that require specific matrix structures
    /// (e.g., lower triangular).
    ///
    /// # Arguments
    ///
    /// * `matrix` - The matrix to validate
    ///
    /// # Errors
    ///
    /// Returns error if matrix doesn't satisfy structural requirements.
    fn validate_structure(&self, _matrix: &DMatrix<T>) -> Result<(), LinearAlgebraError> {
        Ok(()) // Default: no validation
    }

    /// Returns a human-readable name for this strategy.
    fn name(&self) -> &'static str;
}

// =============================================================================
// LU Strategy (Full Dense Matrix)
// =============================================================================

/// LU decomposition-based strategy for full dense matrices.
///
/// Used by Global Bootstrap where the Jacobian is a general dense matrix.
/// Computational complexity: O(n^3) for decomposition, O(n^2) for solve.
#[derive(Debug, Clone)]
pub struct LUStrategy<T: RealField + Copy> {
    /// Stored matrix for solving and inverse computation.
    matrix: Option<DMatrix<T>>,
}

impl<T: RealField + Copy> Default for LUStrategy<T> {
    fn default() -> Self {
        Self { matrix: None }
    }
}

impl<T: RealField + Copy + Float> LinearSolveStrategy<T> for LUStrategy<T> {
    fn decompose(&mut self, matrix: &DMatrix<T>) -> Result<(), LinearAlgebraError> {
        if matrix.nrows() != matrix.ncols() {
            return Err(LinearAlgebraError::NotSquare {
                rows: matrix.nrows(),
                cols: matrix.ncols(),
            });
        }
        self.matrix = Some(matrix.clone());
        Ok(())
    }

    fn solve(&self, b: &[T]) -> Result<Vec<T>, LinearAlgebraError> {
        let matrix = self
            .matrix
            .as_ref()
            .ok_or_else(|| LinearAlgebraError::InvalidInput("Matrix not decomposed".to_string()))?;
        lu_solve(matrix, b)
    }

    fn inverse(&self) -> Result<DMatrix<T>, LinearAlgebraError> {
        let matrix = self
            .matrix
            .as_ref()
            .ok_or_else(|| LinearAlgebraError::InvalidInput("Matrix not decomposed".to_string()))?;
        matrix
            .clone()
            .try_inverse()
            .ok_or(LinearAlgebraError::SingularMatrix)
    }

    fn name(&self) -> &'static str {
        "LU Decomposition"
    }
}

// =============================================================================
// Lower Triangular Strategy (Sequential Bootstrap)
// =============================================================================

/// Forward substitution strategy for lower triangular matrices.
///
/// Used by Sequential Bootstrap where instruments are sorted by maturity,
/// making the Jacobian lower triangular.
///
/// Computational complexity: O(n^2) for both solve and inverse.
#[derive(Debug, Clone)]
pub struct LowerTriangularStrategy<T: RealField + Copy> {
    /// Stored lower triangular matrix.
    matrix: Option<DMatrix<T>>,
    /// Tolerance for triangularity check.
    tolerance: T,
}

impl<T: RealField + Copy + Float> Default for LowerTriangularStrategy<T> {
    fn default() -> Self {
        Self {
            matrix: None,
            tolerance: from_f64(1e-10),
        }
    }
}

impl<T: RealField + Copy + Float> LowerTriangularStrategy<T> {
    /// Create a new strategy with custom tolerance.
    pub fn with_tolerance(tolerance: T) -> Self {
        Self {
            matrix: None,
            tolerance,
        }
    }
}

impl<T: RealField + Copy + Float> LinearSolveStrategy<T> for LowerTriangularStrategy<T> {
    fn decompose(&mut self, matrix: &DMatrix<T>) -> Result<(), LinearAlgebraError> {
        self.validate_structure(matrix)?;
        self.matrix = Some(matrix.clone());
        Ok(())
    }

    fn solve(&self, b: &[T]) -> Result<Vec<T>, LinearAlgebraError> {
        let matrix = self
            .matrix
            .as_ref()
            .ok_or_else(|| LinearAlgebraError::InvalidInput("Matrix not decomposed".to_string()))?;
        forward_substitution(matrix, b)
    }

    fn inverse(&self) -> Result<DMatrix<T>, LinearAlgebraError> {
        let matrix = self
            .matrix
            .as_ref()
            .ok_or_else(|| LinearAlgebraError::InvalidInput("Matrix not decomposed".to_string()))?;
        lower_triangular_inverse(matrix)
    }

    fn validate_structure(&self, matrix: &DMatrix<T>) -> Result<(), LinearAlgebraError> {
        if matrix.nrows() != matrix.ncols() {
            return Err(LinearAlgebraError::NotSquare {
                rows: matrix.nrows(),
                cols: matrix.ncols(),
            });
        }

        // Check upper triangle is zero (within tolerance)
        let n = matrix.nrows();
        for i in 0..n {
            for j in (i + 1)..n {
                if matrix[(i, j)].abs() > self.tolerance {
                    return Err(LinearAlgebraError::InvalidInput(format!(
                        "Matrix is not lower triangular: element ({}, {}) = {:?}",
                        i,
                        j,
                        matrix[(i, j)]
                    )));
                }
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "Forward Substitution (Lower Triangular)"
    }
}

// =============================================================================
// Forward Substitution Algorithm
// =============================================================================

/// Solve a lower triangular system L * x = b using forward substitution.
///
/// # Algorithm
///
/// For a lower triangular matrix L:
/// ```text
/// x[0] = b[0] / L[0,0]
/// x[i] = (b[i] - sum_{j=0}^{i-1} L[i,j] * x[j]) / L[i,i]
/// ```
///
/// # Complexity
///
/// O(n^2) operations.
///
/// # Arguments
///
/// * `l` - Lower triangular matrix (n x n)
/// * `b` - Right-hand side vector (length n)
///
/// # Returns
///
/// Solution vector x (length n)
///
/// # Errors
///
/// Returns error if matrix has zero diagonal (singular).
pub fn forward_substitution<T: RealField + Copy + Float>(
    l: &DMatrix<T>,
    b: &[T],
) -> Result<Vec<T>, LinearAlgebraError> {
    let n = l.nrows();

    if n != l.ncols() {
        return Err(LinearAlgebraError::NotSquare {
            rows: n,
            cols: l.ncols(),
        });
    }

    if n != b.len() {
        return Err(LinearAlgebraError::DimensionMismatch {
            expected: format!("{}", n),
            got: format!("{}", b.len()),
        });
    }

    let mut x = vec![T::zero(); n];
    let epsilon: T = from_f64(1e-15);

    for i in 0..n {
        let mut sum = b[i];
        for j in 0..i {
            sum = sum - l[(i, j)] * x[j];
        }

        if Float::abs(l[(i, i)]) < epsilon {
            return Err(LinearAlgebraError::SingularMatrix);
        }

        x[i] = sum / l[(i, i)];
    }

    Ok(x)
}

/// Compute the inverse of a lower triangular matrix.
///
/// # Algorithm
///
/// Solves L * L^{-1} = I by computing each column of L^{-1}
/// via forward substitution with e_j (unit vectors).
///
/// # Complexity
///
/// O(n^3) operations total (n forward substitutions of O(n^2) each).
/// However, exploiting triangular structure, this is more efficient
/// than general matrix inversion.
///
/// # Arguments
///
/// * `l` - Lower triangular matrix (n x n)
///
/// # Returns
///
/// Inverse matrix L^{-1} (also lower triangular)
///
/// # Errors
///
/// Returns error if matrix is singular.
pub fn lower_triangular_inverse<T: RealField + Copy + Float>(
    l: &DMatrix<T>,
) -> Result<DMatrix<T>, LinearAlgebraError> {
    let n = l.nrows();

    if n != l.ncols() {
        return Err(LinearAlgebraError::NotSquare {
            rows: n,
            cols: l.ncols(),
        });
    }

    let mut inv = DMatrix::zeros(n, n);

    for j in 0..n {
        // Create unit vector e_j
        let mut e_j = vec![T::zero(); n];
        e_j[j] = T::one();

        // Solve L * col_j = e_j
        let col = forward_substitution(l, &e_j)?;

        for i in 0..n {
            inv[(i, j)] = col[i];
        }
    }

    Ok(inv)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_forward_substitution_2x2() {
        // L = [2, 0; 3, 4], b = [2, 11]
        // x = [1, 2]
        let l = DMatrix::from_row_slice(2, 2, &[2.0, 0.0, 3.0, 4.0]);
        let b = vec![2.0, 11.0];

        let x = forward_substitution(&l, &b).unwrap();

        assert_relative_eq!(x[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(x[1], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_forward_substitution_3x3() {
        // L = [1, 0, 0; 2, 3, 0; 4, 5, 6], b = [1, 8, 32]
        // x = [1, 2, 3]
        let l = DMatrix::from_row_slice(3, 3, &[1.0, 0.0, 0.0, 2.0, 3.0, 0.0, 4.0, 5.0, 6.0]);
        let b = vec![1.0, 8.0, 32.0];

        let x = forward_substitution(&l, &b).unwrap();

        assert_relative_eq!(x[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(x[1], 2.0, epsilon = 1e-10);
        assert_relative_eq!(x[2], 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_forward_substitution_singular() {
        // L has zero diagonal
        let l = DMatrix::from_row_slice(2, 2, &[0.0, 0.0, 1.0, 2.0]);
        let b = vec![1.0, 2.0];

        let result = forward_substitution(&l, &b);
        assert!(result.is_err());
    }

    #[test]
    fn test_lower_triangular_inverse_2x2() {
        // L = [2, 0; 3, 4]
        // L^{-1} = [0.5, 0; -0.375, 0.25]
        let l = DMatrix::from_row_slice(2, 2, &[2.0, 0.0, 3.0, 4.0]);

        let inv = lower_triangular_inverse(&l).unwrap();

        // Verify L * L^{-1} = I
        let product = &l * &inv;
        assert_relative_eq!(product[(0, 0)], 1.0, epsilon = 1e-10);
        assert_relative_eq!(product[(0, 1)], 0.0, epsilon = 1e-10);
        assert_relative_eq!(product[(1, 0)], 0.0, epsilon = 1e-10);
        assert_relative_eq!(product[(1, 1)], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_lower_triangular_inverse_3x3() {
        let l = DMatrix::from_row_slice(3, 3, &[1.0, 0.0, 0.0, 2.0, 3.0, 0.0, 4.0, 5.0, 6.0]);

        let inv = lower_triangular_inverse(&l).unwrap();

        // Verify L * L^{-1} = I
        let product = &l * &inv;
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert_relative_eq!(product[(i, j)], expected, epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_lu_strategy_solve() {
        let mut strategy = LUStrategy::default();
        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let b = vec![5.0, 11.0];

        strategy.decompose(&matrix).unwrap();
        let x = strategy.solve(&b).unwrap();

        // Verify A * x = b
        let ax0 = matrix[(0, 0)] * x[0] + matrix[(0, 1)] * x[1];
        let ax1 = matrix[(1, 0)] * x[0] + matrix[(1, 1)] * x[1];
        assert_relative_eq!(ax0, b[0], epsilon = 1e-10);
        assert_relative_eq!(ax1, b[1], epsilon = 1e-10);
    }

    #[test]
    fn test_lu_strategy_inverse() {
        let mut strategy = LUStrategy::default();
        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);

        strategy.decompose(&matrix).unwrap();
        let inv = strategy.inverse().unwrap();

        // Verify A * A^{-1} = I
        let product = &matrix * &inv;
        assert_relative_eq!(product[(0, 0)], 1.0, epsilon = 1e-10);
        assert_relative_eq!(product[(0, 1)], 0.0, epsilon = 1e-10);
        assert_relative_eq!(product[(1, 0)], 0.0, epsilon = 1e-10);
        assert_relative_eq!(product[(1, 1)], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_lower_triangular_strategy_solve() {
        let mut strategy = LowerTriangularStrategy::default();
        let l = DMatrix::from_row_slice(2, 2, &[2.0, 0.0, 3.0, 4.0]);
        let b = vec![2.0, 11.0];

        strategy.decompose(&l).unwrap();
        let x = strategy.solve(&b).unwrap();

        assert_relative_eq!(x[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(x[1], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_lower_triangular_strategy_inverse() {
        let mut strategy = LowerTriangularStrategy::default();
        let l = DMatrix::from_row_slice(2, 2, &[2.0, 0.0, 3.0, 4.0]);

        strategy.decompose(&l).unwrap();
        let inv = strategy.inverse().unwrap();

        // Verify L * L^{-1} = I
        let product = &l * &inv;
        assert_relative_eq!(product[(0, 0)], 1.0, epsilon = 1e-10);
        assert_relative_eq!(product[(0, 1)], 0.0, epsilon = 1e-10);
        assert_relative_eq!(product[(1, 0)], 0.0, epsilon = 1e-10);
        assert_relative_eq!(product[(1, 1)], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_lower_triangular_strategy_rejects_non_triangular() {
        let strategy = LowerTriangularStrategy::<f64>::default();
        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]); // Not lower triangular

        let result = strategy.validate_structure(&matrix);
        assert!(result.is_err());
    }

    #[test]
    fn test_strategy_names() {
        let lu: LUStrategy<f64> = LUStrategy::default();
        let lt: LowerTriangularStrategy<f64> = LowerTriangularStrategy::default();

        assert_eq!(lu.name(), "LU Decomposition");
        assert_eq!(lt.name(), "Forward Substitution (Lower Triangular)");
    }
}
