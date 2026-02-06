//! Sparse matrix types and utilities for large-scale calibration problems.
//!
//! This module provides CSR (Compressed Sparse Row) matrix support via
//! `nalgebra-sparse`, along with conversion utilities and sparsity analysis.
//!
//! ## Features
//!
//! - CSR/CSC type aliases compatible with `nalgebra`
//! - Dense-to-sparse conversion with configurable threshold
//! - Sparsity ratio calculation for automatic strategy selection
//!
//! ## Usage
//!
//! ```ignore
//! use pricer_core::math::linalg::sparse::{to_csr, sparsity_ratio};
//!
//! let dense = DMatrix::from_row_slice(3, 3, &[
//!     1.0, 0.0, 0.0,
//!     2.0, 3.0, 0.0,
//!     0.0, 0.0, 4.0,
//! ]);
//!
//! // Check if sparse representation is beneficial
//! if sparsity_ratio(&dense, 1e-10) > 0.5 {
//!     let sparse = to_csr(&dense, 1e-10);
//!     // Use sparse solver
//! }
//! ```

use nalgebra::{DMatrix, RealField};
use num_traits::Float;

/// CSR (Compressed Sparse Row) matrix type alias.
///
/// CSR is efficient for row slicing and matrix-vector products.
pub type CsrMatrix<T> = nalgebra_sparse::CsrMatrix<T>;

/// CSC (Compressed Sparse Column) matrix type alias.
///
/// CSC is efficient for column slicing and transpose operations.
pub type CscMatrix<T> = nalgebra_sparse::CscMatrix<T>;

/// Convert a dense matrix to CSR (Compressed Sparse Row) format.
///
/// Elements with absolute value below the threshold are treated as zero.
///
/// # Arguments
///
/// * `dense` - The dense matrix to convert
/// * `threshold` - Absolute value threshold for zero detection
///
/// # Returns
///
/// A CSR matrix containing only non-zero elements.
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::linalg::sparse::to_csr;
///
/// let dense = DMatrix::from_row_slice(2, 2, &[1.0, 0.0, 0.0, 2.0]);
/// let sparse = to_csr(&dense, 1e-10);
/// assert_eq!(sparse.nnz(), 2); // 2 non-zero elements
/// ```
pub fn to_csr<T: RealField + Copy + Float>(dense: &DMatrix<T>, threshold: T) -> CsrMatrix<T> {
    let nrows = dense.nrows();
    let ncols = dense.ncols();

    let mut row_offsets = Vec::with_capacity(nrows + 1);
    let mut col_indices = Vec::new();
    let mut values = Vec::new();

    row_offsets.push(0);

    for i in 0..nrows {
        for j in 0..ncols {
            let val = dense[(i, j)];
            if Float::abs(val) > threshold {
                col_indices.push(j);
                values.push(val);
            }
        }
        row_offsets.push(col_indices.len());
    }

    // Create CSR matrix from raw data
    CsrMatrix::try_from_csr_data(nrows, ncols, row_offsets, col_indices, values)
        .expect("Valid CSR data from dense matrix conversion")
}

/// Convert a CSR matrix back to dense format.
///
/// # Arguments
///
/// * `sparse` - The CSR matrix to convert
///
/// # Returns
///
/// A dense matrix with all elements (including zeros).
pub fn to_dense<T: RealField + Copy>(sparse: &CsrMatrix<T>) -> DMatrix<T> {
    let nrows = sparse.nrows();
    let ncols = sparse.ncols();
    let mut dense = DMatrix::zeros(nrows, ncols);

    for (i, row) in sparse.row_iter().enumerate() {
        for (&j, &val) in row.col_indices().iter().zip(row.values().iter()) {
            dense[(i, j)] = val;
        }
    }

    dense
}

/// Calculate the sparsity ratio of a matrix.
///
/// The sparsity ratio is the proportion of zero (or near-zero) elements.
/// A ratio of 0.7 means 70% of elements are zero.
///
/// # Arguments
///
/// * `matrix` - The matrix to analyse
/// * `threshold` - Absolute value threshold for zero detection
///
/// # Returns
///
/// Sparsity ratio in range [0, 1].
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::linalg::sparse::sparsity_ratio;
///
/// let diagonal = DMatrix::from_diagonal(&DVector::from_vec(vec![1.0, 2.0, 3.0]));
/// let ratio = sparsity_ratio(&diagonal, 1e-10);
/// // 6 zeros out of 9 elements = 0.6667
/// ```
pub fn sparsity_ratio<T: RealField + Copy + Float>(matrix: &DMatrix<T>, threshold: T) -> f64 {
    let total = matrix.nrows() * matrix.ncols();
    if total == 0 {
        return 0.0;
    }

    let non_zero_count = matrix
        .iter()
        .filter(|&&x| Float::abs(x) > threshold)
        .count();

    let zero_count = total - non_zero_count;
    (zero_count as f64) / (total as f64)
}

/// Check if a matrix is sufficiently sparse to benefit from sparse algorithms.
///
/// Generally, sparse algorithms are beneficial when sparsity exceeds 70%.
///
/// # Arguments
///
/// * `matrix` - The matrix to analyse
/// * `threshold` - Absolute value threshold for zero detection
/// * `sparsity_threshold` - Minimum sparsity ratio for sparse algorithms
///   (default: 0.7)
///
/// # Returns
///
/// `true` if the matrix is sparse enough to benefit from sparse algorithms.
pub fn is_sparse_beneficial<T: RealField + Copy + Float>(
    matrix: &DMatrix<T>,
    threshold: T,
    sparsity_threshold: f64,
) -> bool {
    sparsity_ratio(matrix, threshold) >= sparsity_threshold
}

/// Count non-zero elements in a matrix.
///
/// # Arguments
///
/// * `matrix` - The matrix to analyse
/// * `threshold` - Absolute value threshold for zero detection
///
/// # Returns
///
/// Number of elements with absolute value above threshold.
pub fn count_nonzeros<T: RealField + Copy + Float>(matrix: &DMatrix<T>, threshold: T) -> usize {
    matrix
        .iter()
        .filter(|&&x| Float::abs(x) > threshold)
        .count()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_to_csr_diagonal() {
        // Diagonal matrix: only diagonal elements are non-zero
        let dense = DMatrix::from_row_slice(3, 3, &[1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0]);

        let csr = to_csr(&dense, 1e-10);

        assert_eq!(csr.nrows(), 3);
        assert_eq!(csr.ncols(), 3);
        assert_eq!(csr.nnz(), 3); // 3 non-zero elements
    }

    #[test]
    fn test_to_csr_lower_triangular() {
        // Lower triangular matrix
        let dense = DMatrix::from_row_slice(3, 3, &[1.0, 0.0, 0.0, 2.0, 3.0, 0.0, 4.0, 5.0, 6.0]);

        let csr = to_csr(&dense, 1e-10);

        assert_eq!(csr.nnz(), 6); // 6 non-zero elements
    }

    #[test]
    fn test_to_csr_dense() {
        // Dense matrix (all non-zero)
        let dense = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);

        let csr = to_csr(&dense, 1e-10);

        assert_eq!(csr.nnz(), 4); // All 4 elements are non-zero
    }

    #[test]
    fn test_to_csr_empty() {
        // Empty matrix (all zeros)
        let dense = DMatrix::from_row_slice(2, 2, &[0.0, 0.0, 0.0, 0.0]);

        let csr = to_csr(&dense, 1e-10);

        assert_eq!(csr.nnz(), 0);
    }

    #[test]
    fn test_to_dense_roundtrip() {
        // Convert dense -> CSR -> dense should be identity
        let original =
            DMatrix::from_row_slice(3, 3, &[1.0, 0.0, 2.0, 0.0, 3.0, 0.0, 4.0, 0.0, 5.0]);

        let csr = to_csr(&original, 1e-10);
        let recovered = to_dense(&csr);

        assert_eq!(original.nrows(), recovered.nrows());
        assert_eq!(original.ncols(), recovered.ncols());

        for i in 0..original.nrows() {
            for j in 0..original.ncols() {
                assert_relative_eq!(original[(i, j)], recovered[(i, j)], epsilon = 1e-15);
            }
        }
    }

    #[test]
    fn test_sparsity_ratio_diagonal() {
        // 3x3 diagonal: 3 non-zero, 6 zero -> 6/9 = 0.6667
        let diagonal =
            DMatrix::from_row_slice(3, 3, &[1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0]);

        let ratio = sparsity_ratio(&diagonal, 1e-10);

        assert_relative_eq!(ratio, 6.0 / 9.0, epsilon = 1e-10);
    }

    #[test]
    fn test_sparsity_ratio_dense() {
        // All non-zero
        let dense = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);

        let ratio = sparsity_ratio(&dense, 1e-10);

        assert_relative_eq!(ratio, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_sparsity_ratio_empty() {
        // All zero
        let empty = DMatrix::from_row_slice(2, 2, &[0.0, 0.0, 0.0, 0.0]);

        let ratio = sparsity_ratio(&empty, 1e-10);

        assert_relative_eq!(ratio, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_sparsity_ratio_threshold() {
        // Values below threshold treated as zero
        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 1e-12, 1e-12, 1.0]);

        let ratio = sparsity_ratio(&matrix, 1e-10);

        // Only 2 elements (1.0 values) are above threshold
        assert_relative_eq!(ratio, 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_is_sparse_beneficial() {
        // 3x3 diagonal: 66.7% sparsity - below 70% threshold
        let diagonal =
            DMatrix::from_row_slice(3, 3, &[1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0]);

        assert!(!is_sparse_beneficial(&diagonal, 1e-10, 0.7));
        assert!(is_sparse_beneficial(&diagonal, 1e-10, 0.6)); // Lower threshold
    }

    #[test]
    fn test_is_sparse_beneficial_large_sparse() {
        // 5x5 diagonal: 20/25 = 80% sparsity
        let mut diagonal = DMatrix::zeros(5, 5);
        for i in 0..5 {
            diagonal[(i, i)] = (i + 1) as f64;
        }

        assert!(is_sparse_beneficial(&diagonal, 1e-10, 0.7));
    }

    #[test]
    fn test_count_nonzeros() {
        let matrix = DMatrix::from_row_slice(2, 3, &[1.0, 0.0, 2.0, 0.0, 3.0, 0.0]);

        assert_eq!(count_nonzeros(&matrix, 1e-10), 3);
    }

    #[test]
    fn test_count_nonzeros_with_threshold() {
        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 1e-12, 1e-8, 2.0]);

        // With threshold 1e-10: 1.0, 1e-8, 2.0 are non-zero
        assert_eq!(count_nonzeros(&matrix, 1e-10), 3);

        // With threshold 1e-6: only 1.0 and 2.0 are non-zero
        assert_eq!(count_nonzeros(&matrix, 1e-6), 2);
    }

    #[test]
    fn test_csr_row_access() {
        let dense = DMatrix::from_row_slice(3, 3, &[1.0, 0.0, 2.0, 0.0, 3.0, 0.0, 4.0, 0.0, 5.0]);

        let csr = to_csr(&dense, 1e-10);

        // Check row 0: values at columns 0 and 2
        let row0 = csr.row(0);
        assert_eq!(row0.nnz(), 2);

        // Check row 1: value at column 1
        let row1 = csr.row(1);
        assert_eq!(row1.nnz(), 1);

        // Check row 2: values at columns 0 and 2
        let row2 = csr.row(2);
        assert_eq!(row2.nnz(), 2);
    }

    #[test]
    fn test_non_square_matrix() {
        // 2x4 matrix
        let dense = DMatrix::from_row_slice(2, 4, &[1.0, 0.0, 2.0, 0.0, 0.0, 3.0, 0.0, 4.0]);

        let csr = to_csr(&dense, 1e-10);

        assert_eq!(csr.nrows(), 2);
        assert_eq!(csr.ncols(), 4);
        assert_eq!(csr.nnz(), 4);

        // Roundtrip
        let recovered = to_dense(&csr);
        assert_eq!(recovered.nrows(), 2);
        assert_eq!(recovered.ncols(), 4);
    }
}
