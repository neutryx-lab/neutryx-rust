//! Linear algebra operations (nalgebra wrapper).
//!
//! This module provides a thin wrapper around the `nalgebra` crate,
//! offering a unified interface consistent with `pricer_core` conventions.
//!
//! ## Features
//!
//! - **Matrix types**: `DMatrix<T>`, `DVector<T>` (dynamic size)
//! - **Decompositions**: Cholesky, LU, QR
//! - **Operations**: Matrix multiplication, transpose, inverse, determinant
//!
//! ## AD Compatibility
//!
//! All functions are generic over `T: RealField`, which includes `Dual64`
//! from `num-dual`. This ensures automatic differentiation compatibility.
//!
//! ## Example
//!
//! ```ignore
//! use pricer_core::math::linalg::{Matrix, cholesky_solve};
//!
//! // Create a positive definite matrix
//! let a = Matrix::from_row_slice(2, 2, &[4.0, 2.0, 2.0, 3.0]);
//! let b = vec![1.0, 2.0];
//!
//! // Solve A * x = b using Cholesky decomposition
//! let x = cholesky_solve(&a, &b).unwrap();
//! ```

// Re-export nalgebra's main types
pub use nalgebra::{Cholesky, DMatrix, DVector, RealField, LU, QR, SVD};

mod error;
#[cfg(feature = "sparse")]
pub mod sparse;
#[cfg(feature = "sparse")]
mod sparse_strategy;
mod strategy;
mod wrappers;

pub use error::LinearAlgebraError;
pub use strategy::{
    forward_substitution, lower_triangular_inverse, LUStrategy, LinearSolveStrategy,
    LowerTriangularStrategy,
};
#[cfg(feature = "sparse")]
pub use sparse_strategy::SparseLUStrategy;
pub use wrappers::{
    cholesky, cholesky_solve, determinant, frobenius_norm, inverse, lu_decompose, lu_solve,
    mat_mat_mul, mat_vec_mul, qr_decompose, qr_solve, svd_solve, trace,
};

/// Dynamic-size matrix type alias.
///
/// This is an alias for `nalgebra::DMatrix<T>` for convenience.
pub type Matrix<T> = DMatrix<T>;

/// Dynamic-size vector type alias.
///
/// This is an alias for `nalgebra::DVector<T>` for convenience.
pub type Vector<T> = DVector<T>;

/// Create a matrix from a row-major slice.
///
/// # Arguments
///
/// * `nrows` - Number of rows
/// * `ncols` - Number of columns
/// * `data` - Row-major data slice (length = nrows * ncols)
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::linalg::matrix_from_rows;
///
/// let m = matrix_from_rows(2, 2, &[1.0, 2.0, 3.0, 4.0]);
/// // Creates:
/// // | 1  2 |
/// // | 3  4 |
/// ```
#[must_use]
pub fn matrix_from_rows<T: RealField + Copy>(nrows: usize, ncols: usize, data: &[T]) -> Matrix<T> {
    DMatrix::from_row_slice(nrows, ncols, data)
}

/// Create a column vector from a slice.
///
/// # Arguments
///
/// * `data` - Data slice
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::linalg::vector_from_slice;
///
/// let v = vector_from_slice(&[1.0, 2.0, 3.0]);
/// ```
#[must_use]
pub fn vector_from_slice<T: RealField + Copy>(data: &[T]) -> Vector<T> {
    DVector::from_column_slice(data)
}

/// Create an identity matrix.
///
/// # Arguments
///
/// * `n` - Size of the matrix (n x n)
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::linalg::identity;
///
/// let i3: Matrix<f64> = identity(3);
/// ```
#[must_use]
pub fn identity<T: RealField + Copy>(n: usize) -> Matrix<T> { DMatrix::identity(n, n) }

/// Create a zero matrix.
///
/// # Arguments
///
/// * `nrows` - Number of rows
/// * `ncols` - Number of columns
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::linalg::zeros;
///
/// let z: Matrix<f64> = zeros(2, 3);
/// ```
#[must_use]
pub fn zeros<T: RealField + Copy>(nrows: usize, ncols: usize) -> Matrix<T> {
    DMatrix::zeros(nrows, ncols)
}

/// Create a matrix filled with ones.
///
/// # Arguments
///
/// * `nrows` - Number of rows
/// * `ncols` - Number of columns
#[must_use]
pub fn ones<T: RealField + Copy>(nrows: usize, ncols: usize) -> Matrix<T> {
    DMatrix::from_element(nrows, ncols, T::one())
}

/// Create a diagonal matrix from a vector.
///
/// # Arguments
///
/// * `diag` - Diagonal elements
#[must_use]
pub fn diagonal<T: RealField + Copy>(diag: &[T]) -> Matrix<T> {
    let v = DVector::from_column_slice(diag);
    DMatrix::from_diagonal(&v)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_matrix_from_rows() {
        let m: Matrix<f64> = matrix_from_rows(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(m[(0, 0)], 1.0);
        assert_eq!(m[(0, 1)], 2.0);
        assert_eq!(m[(1, 0)], 3.0);
        assert_eq!(m[(1, 1)], 4.0);
    }

    #[test]
    fn test_vector_from_slice() {
        let v: Vector<f64> = vector_from_slice(&[1.0, 2.0, 3.0]);
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);
        assert_eq!(v[2], 3.0);
    }

    #[test]
    fn test_identity() {
        let i: Matrix<f64> = identity(3);
        assert_eq!(i[(0, 0)], 1.0);
        assert_eq!(i[(1, 1)], 1.0);
        assert_eq!(i[(2, 2)], 1.0);
        assert_eq!(i[(0, 1)], 0.0);
        assert_eq!(i[(1, 0)], 0.0);
    }

    #[test]
    fn test_zeros() {
        let z: Matrix<f64> = zeros(2, 3);
        assert_eq!(z.nrows(), 2);
        assert_eq!(z.ncols(), 3);
        for i in 0..2 {
            for j in 0..3 {
                assert_eq!(z[(i, j)], 0.0);
            }
        }
    }

    #[test]
    fn test_ones() {
        let o: Matrix<f64> = ones(2, 2);
        for i in 0..2 {
            for j in 0..2 {
                assert_eq!(o[(i, j)], 1.0);
            }
        }
    }

    #[test]
    fn test_diagonal() {
        let d: Matrix<f64> = diagonal(&[1.0, 2.0, 3.0]);
        assert_eq!(d[(0, 0)], 1.0);
        assert_eq!(d[(1, 1)], 2.0);
        assert_eq!(d[(2, 2)], 3.0);
        assert_eq!(d[(0, 1)], 0.0);
        assert_eq!(d[(1, 0)], 0.0);
    }

    #[test]
    fn test_type_aliases_work() {
        let m: Matrix<f64> = Matrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let v: Vector<f64> = Vector::from_column_slice(&[1.0, 2.0]);
        let result = m * v;
        assert_relative_eq!(result[0], 5.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 11.0, epsilon = 1e-10);
    }

    #[test]
    fn test_matrix_transpose() {
        let m: Matrix<f64> = matrix_from_rows(2, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let t = m.transpose();
        assert_eq!(t.nrows(), 3);
        assert_eq!(t.ncols(), 2);
        assert_eq!(t[(0, 0)], 1.0);
        assert_eq!(t[(0, 1)], 4.0);
        assert_eq!(t[(1, 0)], 2.0);
    }

    #[test]
    fn test_cholesky_solve_integration() {
        // Test the full workflow
        let a: Matrix<f64> = matrix_from_rows(2, 2, &[4.0, 2.0, 2.0, 3.0]);
        let b = vec![8.0, 7.0];
        let x = cholesky_solve(&a, &b).unwrap();

        // Verify solution
        let v = vector_from_slice(&x);
        let result = &a * v;
        assert_relative_eq!(result[0], b[0], epsilon = 1e-10);
        assert_relative_eq!(result[1], b[1], epsilon = 1e-10);
    }
}
