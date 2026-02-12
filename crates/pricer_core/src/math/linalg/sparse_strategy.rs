//! Sparse matrix solve strategies for calibration problems.
//!
//! This module provides `SparseLUStrategy` which implements
//! `LinearSolveStrategy` for sparse matrices. It stores matrices in CSR format
//! for memory efficiency and automatically determines when sparse
//! representation is beneficial.
//!
//! ## Sparsity Detection
//!
//! The strategy automatically detects matrix sparsity. When sparsity exceeds
//! the configured threshold (default 70%), sparse storage is used. This is
//! particularly beneficial for:
//!
//! - Large-scale curve calibration problems (100+ pillars)
//! - Jacobian matrices with local sensitivity patterns
//! - Storing J⁻¹ for IFT-based Greeks computation
//!
//! ## Implementation Notes
//!
//! The current implementation uses dense LU for the actual solve operation,
//! with CSR storage for memory efficiency.

use nalgebra::{DMatrix, RealField};
use num_traits::Float;

use super::{
    error::LinearAlgebraError,
    lu_solve,
    sparse::{is_sparse_beneficial, sparsity_ratio, to_csr, to_dense, CsrMatrix},
    wrappers::{require_dims, require_square},
    LinearSolveStrategy,
};
use crate::math::numeric::from_f64;

/// LU decomposition-based strategy with sparse storage optimisation.
#[derive(Debug, Clone)]
pub struct SparseLUStrategy<T: RealField + Copy> {
    /// Stored dense matrix (used when not sparse enough).
    dense_matrix: Option<DMatrix<T>>,
    /// Stored sparse matrix in CSR format.
    sparse_matrix: Option<CsrMatrix<T>>,
    /// Zero threshold for sparsity detection.
    zero_threshold: T,
    /// Minimum sparsity ratio to use sparse storage (default: 0.7 = 70%).
    sparsity_threshold: f64,
    /// Whether the stored matrix is in sparse format.
    is_sparse: bool,
    /// Cached sparsity ratio of the decomposed matrix.
    cached_sparsity: Option<f64>,
}

impl<T: RealField + Copy + Float> Default for SparseLUStrategy<T> {
    fn default() -> Self {
        Self {
            dense_matrix: None,
            sparse_matrix: None,
            zero_threshold: from_f64(1e-15),
            sparsity_threshold: 0.7, // 70% threshold per design
            is_sparse: false,
            cached_sparsity: None,
        }
    }
}

impl<T: RealField + Copy + Float> SparseLUStrategy<T> {
    /// Create a new strategy with custom thresholds.
    pub fn with_thresholds(zero_threshold: T, sparsity_threshold: f64) -> Self {
        Self {
            dense_matrix: None,
            sparse_matrix: None,
            zero_threshold,
            sparsity_threshold,
            is_sparse: false,
            cached_sparsity: None,
        }
    }

    /// Create a strategy with a specific sparsity threshold.
    pub fn with_sparsity_threshold(threshold: f64) -> Self {
        Self {
            sparsity_threshold: threshold,
            ..Self::default()
        }
    }

    /// Get the sparsity ratio of the decomposed matrix, or `None` if not yet
    /// decomposed.
    pub fn sparsity(&self) -> Option<f64> { self.cached_sparsity }

    /// Check if the strategy is currently using sparse storage.
    pub fn is_using_sparse(&self) -> bool { self.is_sparse }

    /// Check if a matrix would benefit from sparse storage.
    pub fn would_use_sparse(&self, matrix: &DMatrix<T>) -> bool {
        is_sparse_beneficial(matrix, self.zero_threshold, self.sparsity_threshold)
    }

    /// Get the zero threshold used for sparsity detection.
    pub fn zero_threshold(&self) -> T { self.zero_threshold }

    /// Get the sparsity threshold.
    pub fn sparsity_threshold(&self) -> f64 { self.sparsity_threshold }

    /// Decompose a sparse matrix directly from CSR input.
    pub fn decompose_sparse(&mut self, csr: CsrMatrix<T>) -> Result<(), LinearAlgebraError> {
        if csr.nrows() != csr.ncols() {
            return Err(LinearAlgebraError::NotSquare {
                rows: csr.nrows(),
                cols: csr.ncols(),
            });
        }

        let total = csr.nrows() * csr.ncols();
        let nnz = csr.nnz();
        let sparsity = if total > 0 {
            (total - nnz) as f64 / total as f64
        } else {
            0.0
        };

        self.sparse_matrix = Some(csr);
        self.dense_matrix = None;
        self.is_sparse = true;
        self.cached_sparsity = Some(sparsity);

        Ok(())
    }

    /// Get the stored matrix as dense format, converting from sparse if
    /// necessary.
    fn get_dense(&self) -> Result<DMatrix<T>, LinearAlgebraError> {
        if let Some(ref dense) = self.dense_matrix {
            Ok(dense.clone())
        } else if let Some(ref sparse) = self.sparse_matrix {
            Ok(to_dense(sparse))
        } else {
            Err(LinearAlgebraError::InvalidInput(
                "Matrix not decomposed".to_string(),
            ))
        }
    }
}

impl<T: RealField + Copy + Float> LinearSolveStrategy<T> for SparseLUStrategy<T> {
    fn decompose(&mut self, matrix: &DMatrix<T>) -> Result<(), LinearAlgebraError> {
        require_square(matrix)?;

        // Calculate sparsity and decide storage format
        let sparsity = sparsity_ratio(matrix, self.zero_threshold);
        self.cached_sparsity = Some(sparsity);

        if sparsity >= self.sparsity_threshold {
            // Use sparse storage
            let csr = to_csr(matrix, self.zero_threshold);
            self.sparse_matrix = Some(csr);
            self.dense_matrix = None;
            self.is_sparse = true;
        } else {
            // Use dense storage
            self.dense_matrix = Some(matrix.clone());
            self.sparse_matrix = None;
            self.is_sparse = false;
        }

        Ok(())
    }

    fn solve(&self, b: &[T]) -> Result<Vec<T>, LinearAlgebraError> {
        let matrix = self.get_dense()?;
        require_dims(matrix.nrows(), b.len())?;
        lu_solve(&matrix, b)
    }

    fn inverse(&self) -> Result<DMatrix<T>, LinearAlgebraError> {
        let matrix = self.get_dense()?;

        matrix
            .clone()
            .try_inverse()
            .ok_or(LinearAlgebraError::SingularMatrix)
    }

    fn name(&self) -> &'static str {
        if self.is_sparse {
            "Sparse LU Decomposition (CSR storage)"
        } else {
            "Sparse LU Decomposition (dense storage)"
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_sparse_lu_strategy_default() {
        let strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

        assert_relative_eq!(strategy.zero_threshold(), 1e-15, epsilon = 1e-20);
        assert_relative_eq!(strategy.sparsity_threshold(), 0.7, epsilon = 1e-10);
        assert!(!strategy.is_using_sparse());
        assert!(strategy.sparsity().is_none());
    }

    #[test]
    fn test_sparse_lu_strategy_with_thresholds() {
        let strategy: SparseLUStrategy<f64> = SparseLUStrategy::with_thresholds(1e-10, 0.5);

        assert_relative_eq!(strategy.zero_threshold(), 1e-10, epsilon = 1e-15);
        assert_relative_eq!(strategy.sparsity_threshold(), 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_decompose_sparse_matrix() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::with_sparsity_threshold(0.5);

        // 5x5 diagonal matrix: 20/25 = 80% sparsity
        let mut matrix = DMatrix::zeros(5, 5);
        for i in 0..5 {
            matrix[(i, i)] = (i + 1) as f64;
        }

        strategy.decompose(&matrix).unwrap();

        assert!(strategy.is_using_sparse());
        assert_relative_eq!(strategy.sparsity().unwrap(), 0.8, epsilon = 1e-10);
    }

    #[test]
    fn test_decompose_dense_matrix() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

        // Dense matrix (all non-zero)
        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);

        strategy.decompose(&matrix).unwrap();

        assert!(!strategy.is_using_sparse());
        assert_relative_eq!(strategy.sparsity().unwrap(), 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_decompose_not_square_error() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

        let matrix = DMatrix::from_row_slice(2, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        let result = strategy.decompose(&matrix);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LinearAlgebraError::NotSquare { rows: 2, cols: 3 }
        ));
    }

    #[test]
    fn test_solve_diagonal() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

        // Diagonal matrix
        let matrix = DMatrix::from_row_slice(3, 3, &[2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0]);

        strategy.decompose(&matrix).unwrap();
        let x = strategy.solve(&[4.0, 9.0, 16.0]).unwrap();

        // x = [2, 3, 4]
        assert_relative_eq!(x[0], 2.0, epsilon = 1e-10);
        assert_relative_eq!(x[1], 3.0, epsilon = 1e-10);
        assert_relative_eq!(x[2], 4.0, epsilon = 1e-10);
    }

    #[test]
    fn test_solve_lower_triangular() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

        // Lower triangular matrix
        let matrix = DMatrix::from_row_slice(3, 3, &[1.0, 0.0, 0.0, 2.0, 3.0, 0.0, 4.0, 5.0, 6.0]);

        strategy.decompose(&matrix).unwrap();
        let b = vec![1.0, 8.0, 32.0];
        let x = strategy.solve(&b).unwrap();

        // Verify A * x = b
        for i in 0..3 {
            let mut sum = 0.0;
            for j in 0..3 {
                sum += matrix[(i, j)] * x[j];
            }
            assert_relative_eq!(sum, b[i], epsilon = 1e-10);
        }
    }

    #[test]
    fn test_solve_general() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);

        strategy.decompose(&matrix).unwrap();
        let b = vec![5.0, 11.0];
        let x = strategy.solve(&b).unwrap();

        // Verify A * x = b
        let ax0 = matrix[(0, 0)] * x[0] + matrix[(0, 1)] * x[1];
        let ax1 = matrix[(1, 0)] * x[0] + matrix[(1, 1)] * x[1];
        assert_relative_eq!(ax0, b[0], epsilon = 1e-10);
        assert_relative_eq!(ax1, b[1], epsilon = 1e-10);
    }

    #[test]
    fn test_solve_dimension_mismatch() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);

        strategy.decompose(&matrix).unwrap();
        let result = strategy.solve(&[1.0, 2.0, 3.0]); // Wrong size

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LinearAlgebraError::DimensionMismatch { .. }
        ));
    }

    #[test]
    fn test_inverse() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

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
    fn test_inverse_sparse() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::with_sparsity_threshold(0.5);

        // 5x5 diagonal matrix
        let mut matrix = DMatrix::zeros(5, 5);
        for i in 0..5 {
            matrix[(i, i)] = (i + 1) as f64;
        }

        strategy.decompose(&matrix).unwrap();
        assert!(strategy.is_using_sparse());

        let inv = strategy.inverse().unwrap();

        // Verify A * A^{-1} = I
        let product = &matrix * &inv;
        for i in 0..5 {
            for j in 0..5 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert_relative_eq!(product[(i, j)], expected, epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_inverse_singular() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

        // Singular matrix (second row is twice the first)
        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 2.0, 4.0]);

        strategy.decompose(&matrix).unwrap();
        let result = strategy.inverse();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LinearAlgebraError::SingularMatrix
        ));
    }

    #[test]
    fn test_solve_before_decompose() {
        let strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

        let result = strategy.solve(&[1.0, 2.0]);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LinearAlgebraError::InvalidInput(_)
        ));
    }

    #[test]
    fn test_name_sparse() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::with_sparsity_threshold(0.5);

        // Sparse matrix
        let mut matrix = DMatrix::zeros(5, 5);
        for i in 0..5 {
            matrix[(i, i)] = 1.0;
        }

        strategy.decompose(&matrix).unwrap();

        assert_eq!(strategy.name(), "Sparse LU Decomposition (CSR storage)");
    }

    #[test]
    fn test_name_dense() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

        // Dense matrix
        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);

        strategy.decompose(&matrix).unwrap();

        assert_eq!(strategy.name(), "Sparse LU Decomposition (dense storage)");
    }

    #[test]
    fn test_would_use_sparse() {
        let strategy: SparseLUStrategy<f64> = SparseLUStrategy::with_sparsity_threshold(0.7);

        // Sparse matrix (80% zeros)
        let mut sparse = DMatrix::zeros(5, 5);
        for i in 0..5 {
            sparse[(i, i)] = 1.0;
        }
        assert!(strategy.would_use_sparse(&sparse));

        // Dense matrix (0% zeros)
        let dense = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        assert!(!strategy.would_use_sparse(&dense));
    }

    #[test]
    fn test_decompose_sparse_csr() {
        let mut strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();

        // Create CSR matrix directly
        let csr = CsrMatrix::try_from_csr_data(
            3,
            3,
            vec![0, 1, 2, 3],
            vec![0, 1, 2],
            vec![1.0, 2.0, 3.0],
        )
        .unwrap();

        strategy.decompose_sparse(csr).unwrap();

        assert!(strategy.is_using_sparse());
        // 3x3 with 3 non-zeros = 6/9 = 66.7% sparsity
        assert_relative_eq!(strategy.sparsity().unwrap(), 6.0 / 9.0, epsilon = 1e-10);

        // Solve diagonal system
        let x = strategy.solve(&[1.0, 4.0, 9.0]).unwrap();
        assert_relative_eq!(x[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(x[1], 2.0, epsilon = 1e-10);
        assert_relative_eq!(x[2], 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_sparse_vs_lu_strategy_equivalence() {
        use super::super::LUStrategy;

        let matrix = DMatrix::from_row_slice(3, 3, &[1.0, 2.0, 0.0, 3.0, 4.0, 0.0, 0.0, 5.0, 6.0]);

        let b = vec![5.0, 11.0, 17.0];

        // Solve with LU strategy
        let mut lu_strategy: LUStrategy<f64> = LUStrategy::default();
        lu_strategy.decompose(&matrix).unwrap();
        let lu_x = lu_strategy.solve(&b).unwrap();

        // Solve with Sparse LU strategy
        let mut sparse_strategy: SparseLUStrategy<f64> = SparseLUStrategy::default();
        sparse_strategy.decompose(&matrix).unwrap();
        let sparse_x = sparse_strategy.solve(&b).unwrap();

        // Results should be equivalent
        for i in 0..3 {
            assert_relative_eq!(lu_x[i], sparse_x[i], epsilon = 1e-10);
        }

        // Inverses should also be equivalent
        let lu_inv = lu_strategy.inverse().unwrap();
        let sparse_inv = sparse_strategy.inverse().unwrap();

        for i in 0..3 {
            for j in 0..3 {
                assert_relative_eq!(lu_inv[(i, j)], sparse_inv[(i, j)], epsilon = 1e-10);
            }
        }
    }
}
