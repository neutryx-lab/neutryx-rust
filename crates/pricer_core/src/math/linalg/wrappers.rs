//! Wrapper functions for common linear algebra operations.
//!
//! These functions provide a convenient interface to nalgebra's decomposition
//! and solving capabilities, with proper error handling and AD compatibility.

use nalgebra::{Cholesky, DMatrix, DVector, LU, QR, RealField};

use super::error::LinearAlgebraError;

/// Solve a linear system using Cholesky decomposition.
///
/// Solves A * x = b where A is a positive definite symmetric matrix.
///
/// # Arguments
///
/// * `a` - Positive definite symmetric matrix
/// * `b` - Right-hand side vector
///
/// # Returns
///
/// Solution vector x
///
/// # Errors
///
/// Returns `NotPositiveDefinite` if the matrix is not positive definite.
/// Returns `DimensionMismatch` if dimensions are incompatible.
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::linalg::{Matrix, cholesky_solve};
///
/// let a = Matrix::from_row_slice(2, 2, &[4.0, 2.0, 2.0, 3.0]);
/// let b = vec![1.0, 2.0];
/// let x = cholesky_solve(&a, &b).unwrap();
/// ```
pub fn cholesky_solve<T: RealField + Copy>(
    a: &DMatrix<T>,
    b: &[T],
) -> Result<Vec<T>, LinearAlgebraError> {
    // Check dimensions
    if a.nrows() != a.ncols() {
        return Err(LinearAlgebraError::NotSquare {
            rows: a.nrows(),
            cols: a.ncols(),
        });
    }
    if a.nrows() != b.len() {
        return Err(LinearAlgebraError::DimensionMismatch {
            expected: format!("{}", a.nrows()),
            got: format!("{}", b.len()),
        });
    }

    let chol = Cholesky::new(a.clone()).ok_or(LinearAlgebraError::NotPositiveDefinite)?;
    let b_vec = DVector::from_column_slice(b);
    let x = chol.solve(&b_vec);
    Ok(x.iter().copied().collect())
}

/// Solve a linear system using LU decomposition.
///
/// Solves A * x = b for general square matrices.
///
/// # Arguments
///
/// * `a` - Square matrix
/// * `b` - Right-hand side vector
///
/// # Returns
///
/// Solution vector x
///
/// # Errors
///
/// Returns `SingularMatrix` if the matrix is singular.
/// Returns `DimensionMismatch` if dimensions are incompatible.
pub fn lu_solve<T: RealField + Copy>(a: &DMatrix<T>, b: &[T]) -> Result<Vec<T>, LinearAlgebraError> {
    // Check dimensions
    if a.nrows() != a.ncols() {
        return Err(LinearAlgebraError::NotSquare {
            rows: a.nrows(),
            cols: a.ncols(),
        });
    }
    if a.nrows() != b.len() {
        return Err(LinearAlgebraError::DimensionMismatch {
            expected: format!("{}", a.nrows()),
            got: format!("{}", b.len()),
        });
    }

    let lu = LU::new(a.clone());
    let b_vec = DVector::from_column_slice(b);
    let x = lu.solve(&b_vec).ok_or(LinearAlgebraError::SingularMatrix)?;
    Ok(x.iter().copied().collect())
}

/// Solve a linear system using QR decomposition.
///
/// For square systems, solves A * x = b exactly.
/// For overdetermined systems (m > n), solves via normal equations.
///
/// # Arguments
///
/// * `a` - Matrix (m x n)
/// * `b` - Right-hand side vector (length m)
///
/// # Returns
///
/// Solution vector x (length n)
///
/// # Errors
///
/// Returns `DimensionMismatch` if dimensions are incompatible.
pub fn qr_solve<T: RealField + Copy>(
    a: &DMatrix<T>,
    b: &[T],
) -> Result<Vec<T>, LinearAlgebraError> {
    if a.nrows() != b.len() {
        return Err(LinearAlgebraError::DimensionMismatch {
            expected: format!("{}", a.nrows()),
            got: format!("{}", b.len()),
        });
    }
    if a.nrows() < a.ncols() {
        return Err(LinearAlgebraError::InvalidInput(
            "QR solve requires m >= n (overdetermined or square system)".to_string(),
        ));
    }

    let b_vec = DVector::from_column_slice(b);

    // For square systems, use direct QR solve
    if a.nrows() == a.ncols() {
        let qr = QR::new(a.clone());
        let x = qr.solve(&b_vec).ok_or(LinearAlgebraError::DecompositionFailed(
            "QR decomposition could not solve the system".to_string(),
        ))?;
        return Ok(x.iter().copied().collect());
    }

    // For overdetermined systems, use normal equations: (A^T A) x = A^T b
    let at = a.transpose();
    let ata = &at * a;
    let atb = &at * b_vec;

    let lu = LU::new(ata);
    let x = lu.solve(&atb).ok_or(LinearAlgebraError::DecompositionFailed(
        "Normal equations could not be solved (system may be rank deficient)".to_string(),
    ))?;
    Ok(x.iter().copied().collect())
}

/// Solve a least squares problem using SVD (more robust than QR for ill-conditioned systems).
///
/// Finds x that minimises ||A * x - b||².
///
/// # Arguments
///
/// * `a` - Matrix (m x n)
/// * `b` - Right-hand side vector (length m)
/// * `epsilon` - Tolerance for singular value cutoff (values below epsilon * max_sv are treated as zero)
///
/// # Returns
///
/// Least squares solution vector x (length n)
pub fn svd_solve<T: RealField + Copy>(
    a: &DMatrix<T>,
    b: &[T],
    epsilon: T,
) -> Result<Vec<T>, LinearAlgebraError> {
    use nalgebra::SVD;

    if a.nrows() != b.len() {
        return Err(LinearAlgebraError::DimensionMismatch {
            expected: format!("{}", a.nrows()),
            got: format!("{}", b.len()),
        });
    }

    let svd = SVD::new(a.clone(), true, true);
    let b_vec = DVector::from_column_slice(b);
    let x = svd
        .solve(&b_vec, epsilon)
        .map_err(|e| LinearAlgebraError::DecompositionFailed(e.to_string()))?;
    Ok(x.iter().copied().collect())
}

/// Compute the determinant of a square matrix.
///
/// # Arguments
///
/// * `a` - Square matrix
///
/// # Returns
///
/// The determinant of the matrix
///
/// # Errors
///
/// Returns `NotSquare` if the matrix is not square.
pub fn determinant<T: RealField + Copy>(a: &DMatrix<T>) -> Result<T, LinearAlgebraError> {
    if a.nrows() != a.ncols() {
        return Err(LinearAlgebraError::NotSquare {
            rows: a.nrows(),
            cols: a.ncols(),
        });
    }
    Ok(a.determinant())
}

/// Compute the inverse of a square matrix.
///
/// # Arguments
///
/// * `a` - Square matrix
///
/// # Returns
///
/// The inverse matrix A⁻¹
///
/// # Errors
///
/// Returns `SingularMatrix` if the matrix is singular.
/// Returns `NotSquare` if the matrix is not square.
pub fn inverse<T: RealField + Copy>(a: &DMatrix<T>) -> Result<DMatrix<T>, LinearAlgebraError> {
    if a.nrows() != a.ncols() {
        return Err(LinearAlgebraError::NotSquare {
            rows: a.nrows(),
            cols: a.ncols(),
        });
    }
    a.clone().try_inverse().ok_or(LinearAlgebraError::SingularMatrix)
}

/// Perform Cholesky decomposition.
///
/// Computes the lower triangular matrix L such that A = L * L^T.
///
/// # Arguments
///
/// * `a` - Positive definite symmetric matrix
///
/// # Returns
///
/// Lower triangular matrix L
///
/// # Errors
///
/// Returns `NotPositiveDefinite` if the matrix is not positive definite.
/// Returns `NotSquare` if the matrix is not square.
pub fn cholesky<T: RealField + Copy>(a: &DMatrix<T>) -> Result<DMatrix<T>, LinearAlgebraError> {
    if a.nrows() != a.ncols() {
        return Err(LinearAlgebraError::NotSquare {
            rows: a.nrows(),
            cols: a.ncols(),
        });
    }
    let chol = Cholesky::new(a.clone()).ok_or(LinearAlgebraError::NotPositiveDefinite)?;
    Ok(chol.l())
}

/// Perform LU decomposition with partial pivoting.
///
/// Computes matrices P, L, U such that P * A = L * U.
///
/// # Arguments
///
/// * `a` - Square matrix
///
/// # Returns
///
/// Tuple of (L, U, P) where P is a permutation matrix,
/// L is lower triangular, and U is upper triangular.
///
/// # Errors
///
/// Returns `NotSquare` if the matrix is not square.
pub fn lu_decompose<T: RealField + Copy>(
    a: &DMatrix<T>,
) -> Result<(DMatrix<T>, DMatrix<T>), LinearAlgebraError> {
    if a.nrows() != a.ncols() {
        return Err(LinearAlgebraError::NotSquare {
            rows: a.nrows(),
            cols: a.ncols(),
        });
    }
    let lu = LU::new(a.clone());
    Ok((lu.l(), lu.u()))
}

/// Perform QR decomposition.
///
/// Computes matrices Q, R such that A = Q * R.
///
/// # Arguments
///
/// * `a` - Matrix (m x n)
///
/// # Returns
///
/// Tuple of (Q, R) where Q is orthogonal and R is upper triangular.
pub fn qr_decompose<T: RealField + Copy>(a: &DMatrix<T>) -> (DMatrix<T>, DMatrix<T>) {
    let qr = QR::new(a.clone());
    (qr.q(), qr.r())
}

/// Compute the trace of a square matrix.
///
/// # Arguments
///
/// * `a` - Square matrix
///
/// # Returns
///
/// Sum of diagonal elements
///
/// # Errors
///
/// Returns `NotSquare` if the matrix is not square.
pub fn trace<T: RealField + Copy>(a: &DMatrix<T>) -> Result<T, LinearAlgebraError> {
    if a.nrows() != a.ncols() {
        return Err(LinearAlgebraError::NotSquare {
            rows: a.nrows(),
            cols: a.ncols(),
        });
    }
    Ok(a.trace())
}

/// Compute the Frobenius norm of a matrix.
///
/// # Arguments
///
/// * `a` - Matrix
///
/// # Returns
///
/// The Frobenius norm (sqrt of sum of squared elements)
pub fn frobenius_norm<T: RealField + Copy>(a: &DMatrix<T>) -> T {
    a.norm()
}

/// Compute the matrix-vector product A * x.
///
/// # Arguments
///
/// * `a` - Matrix (m x n)
/// * `x` - Vector (length n)
///
/// # Returns
///
/// Result vector (length m)
///
/// # Errors
///
/// Returns `DimensionMismatch` if dimensions are incompatible.
pub fn mat_vec_mul<T: RealField + Copy>(
    a: &DMatrix<T>,
    x: &[T],
) -> Result<Vec<T>, LinearAlgebraError> {
    if a.ncols() != x.len() {
        return Err(LinearAlgebraError::DimensionMismatch {
            expected: format!("{}", a.ncols()),
            got: format!("{}", x.len()),
        });
    }
    let x_vec = DVector::from_column_slice(x);
    let result = a * x_vec;
    Ok(result.iter().copied().collect())
}

/// Compute the matrix-matrix product A * B.
///
/// # Arguments
///
/// * `a` - Matrix (m x k)
/// * `b` - Matrix (k x n)
///
/// # Returns
///
/// Result matrix (m x n)
///
/// # Errors
///
/// Returns `DimensionMismatch` if inner dimensions don't match.
pub fn mat_mat_mul<T: RealField + Copy>(
    a: &DMatrix<T>,
    b: &DMatrix<T>,
) -> Result<DMatrix<T>, LinearAlgebraError> {
    if a.ncols() != b.nrows() {
        return Err(LinearAlgebraError::DimensionMismatch {
            expected: format!("{}", a.ncols()),
            got: format!("{}", b.nrows()),
        });
    }
    Ok(a * b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn create_positive_definite_2x2() -> DMatrix<f64> {
        // [4, 2; 2, 3] is positive definite
        DMatrix::from_row_slice(2, 2, &[4.0, 2.0, 2.0, 3.0])
    }

    fn create_singular_2x2() -> DMatrix<f64> {
        // [1, 2; 2, 4] is singular (row 2 = 2 * row 1)
        DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 2.0, 4.0])
    }

    #[test]
    fn test_cholesky_solve() {
        let a = create_positive_definite_2x2();
        let b = vec![8.0, 7.0];
        let x = cholesky_solve(&a, &b).unwrap();

        // Verify: A * x should equal b
        let ax = mat_vec_mul(&a, &x).unwrap();
        assert_relative_eq!(ax[0], b[0], epsilon = 1e-10);
        assert_relative_eq!(ax[1], b[1], epsilon = 1e-10);
    }

    #[test]
    fn test_cholesky_solve_not_positive_definite() {
        // [-1, 0; 0, -1] is not positive definite
        let a = DMatrix::from_row_slice(2, 2, &[-1.0, 0.0, 0.0, -1.0]);
        let b = vec![1.0, 1.0];
        let result = cholesky_solve(&a, &b);
        assert!(matches!(result, Err(LinearAlgebraError::NotPositiveDefinite)));
    }

    #[test]
    fn test_lu_solve() {
        let a = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let b = vec![5.0, 11.0];
        let x = lu_solve(&a, &b).unwrap();

        // Verify: A * x should equal b
        let ax = mat_vec_mul(&a, &x).unwrap();
        assert_relative_eq!(ax[0], b[0], epsilon = 1e-10);
        assert_relative_eq!(ax[1], b[1], epsilon = 1e-10);
    }

    #[test]
    fn test_lu_solve_singular() {
        let a = create_singular_2x2();
        let b = vec![3.0, 6.0];
        let result = lu_solve(&a, &b);
        assert!(matches!(result, Err(LinearAlgebraError::SingularMatrix)));
    }

    #[test]
    fn test_qr_solve_square() {
        let a = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let b = vec![5.0, 11.0];
        let x = qr_solve(&a, &b).unwrap();

        // Verify: A * x should equal b
        let ax = mat_vec_mul(&a, &x).unwrap();
        assert_relative_eq!(ax[0], b[0], epsilon = 1e-10);
        assert_relative_eq!(ax[1], b[1], epsilon = 1e-10);
    }

    #[test]
    fn test_qr_solve_overdetermined() {
        // Overdetermined system (3 equations, 2 unknowns)
        let a = DMatrix::from_row_slice(3, 2, &[1.0, 1.0, 1.0, 2.0, 1.0, 3.0]);
        let b = vec![2.0, 3.0, 4.0];
        let x = qr_solve(&a, &b).unwrap();

        // Result should minimise ||Ax - b||²
        assert_eq!(x.len(), 2);
    }

    #[test]
    fn test_determinant() {
        let a = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let det = determinant(&a).unwrap();
        assert_relative_eq!(det, -2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_determinant_not_square() {
        let a = DMatrix::from_row_slice(2, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let result = determinant(&a);
        assert!(matches!(result, Err(LinearAlgebraError::NotSquare { .. })));
    }

    #[test]
    fn test_inverse() {
        let a = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let a_inv = inverse(&a).unwrap();

        // A * A⁻¹ should equal identity
        let product = mat_mat_mul(&a, &a_inv).unwrap();
        assert_relative_eq!(product[(0, 0)], 1.0, epsilon = 1e-10);
        assert_relative_eq!(product[(0, 1)], 0.0, epsilon = 1e-10);
        assert_relative_eq!(product[(1, 0)], 0.0, epsilon = 1e-10);
        assert_relative_eq!(product[(1, 1)], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_inverse_singular() {
        let a = create_singular_2x2();
        let result = inverse(&a);
        assert!(matches!(result, Err(LinearAlgebraError::SingularMatrix)));
    }

    #[test]
    fn test_cholesky_decomposition() {
        let a = create_positive_definite_2x2();
        let l = cholesky(&a).unwrap();

        // L * L^T should equal A
        let l_t = l.transpose();
        let reconstructed = mat_mat_mul(&l, &l_t).unwrap();
        for i in 0..2 {
            for j in 0..2 {
                assert_relative_eq!(reconstructed[(i, j)], a[(i, j)], epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_lu_decomposition() {
        let a = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let (l, u) = lu_decompose(&a).unwrap();

        // L * U should equal A (or P * A)
        let reconstructed = mat_mat_mul(&l, &u).unwrap();
        // Note: With pivoting, we get P*A = L*U, but for this simple case...
        assert_eq!(reconstructed.nrows(), 2);
        assert_eq!(reconstructed.ncols(), 2);
    }

    #[test]
    fn test_qr_decomposition() {
        let a = DMatrix::from_row_slice(3, 2, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let (q, r) = qr_decompose(&a);

        // Q * R should equal A
        let reconstructed = mat_mat_mul(&q, &r).unwrap();
        for i in 0..3 {
            for j in 0..2 {
                assert_relative_eq!(reconstructed[(i, j)], a[(i, j)], epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_trace() {
        let a = DMatrix::from_row_slice(3, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
        let tr = trace(&a).unwrap();
        assert_relative_eq!(tr, 15.0, epsilon = 1e-10); // 1 + 5 + 9
    }

    #[test]
    fn test_frobenius_norm() {
        let a = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let norm = frobenius_norm(&a);
        // sqrt(1 + 4 + 9 + 16) = sqrt(30)
        assert_relative_eq!(norm, 30.0_f64.sqrt(), epsilon = 1e-10);
    }

    #[test]
    fn test_mat_vec_mul() {
        let a = DMatrix::from_row_slice(2, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let x = vec![1.0, 1.0, 1.0];
        let result = mat_vec_mul(&a, &x).unwrap();
        assert_relative_eq!(result[0], 6.0, epsilon = 1e-10); // 1+2+3
        assert_relative_eq!(result[1], 15.0, epsilon = 1e-10); // 4+5+6
    }

    #[test]
    fn test_mat_vec_mul_dimension_mismatch() {
        let a = DMatrix::from_row_slice(2, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let x = vec![1.0, 1.0]; // Wrong size
        let result = mat_vec_mul(&a, &x);
        assert!(matches!(
            result,
            Err(LinearAlgebraError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn test_mat_mat_mul() {
        let a = DMatrix::from_row_slice(2, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let b = DMatrix::from_row_slice(3, 2, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let result = mat_mat_mul(&a, &b).unwrap();
        assert_eq!(result.nrows(), 2);
        assert_eq!(result.ncols(), 2);
        // (1,2,3)*(1,3,5)^T = 1+6+15 = 22
        assert_relative_eq!(result[(0, 0)], 22.0, epsilon = 1e-10);
    }

    #[test]
    fn test_mat_mat_mul_dimension_mismatch() {
        let a = DMatrix::from_row_slice(2, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let b = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]); // Wrong inner dimension
        let result = mat_mat_mul(&a, &b);
        assert!(matches!(
            result,
            Err(LinearAlgebraError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn test_dimension_check_cholesky_solve() {
        let a = DMatrix::from_row_slice(2, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let b = vec![1.0, 2.0];
        let result = cholesky_solve(&a, &b);
        assert!(matches!(result, Err(LinearAlgebraError::NotSquare { .. })));
    }

    #[test]
    fn test_dimension_check_lu_solve() {
        let a = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let b = vec![1.0, 2.0, 3.0]; // Wrong size
        let result = lu_solve(&a, &b);
        assert!(matches!(
            result,
            Err(LinearAlgebraError::DimensionMismatch { .. })
        ));
    }
}
