//! Correlated stochastic models with Cholesky decomposition.
//!
//! This module provides infrastructure for correlating multiple stochastic models
//! using Cholesky decomposition to generate correlated Brownian motions.
//!
//! ## Mathematical Background
//!
//! Given `n` independent standard normal random variables `Z = [Z1, Z2, ..., Zn]`,
//! we can generate correlated normals `W = [W1, W2, ..., Wn]` using:
//!
//! ```text
//! W = L * Z
//! ```
//!
//! where `L` is the lower triangular Cholesky factor of the correlation matrix `C`:
//! ```text
//! C = L * L^T
//! ```
//!
//! ## Usage
//!
//! ```
//! use pricer_models::models::hybrid::correlated::{CorrelatedModels, CorrelationMatrix};
//!
//! // Create a 2x2 correlation matrix with rho = 0.5
//! let corr = CorrelationMatrix::new(&[
//!     1.0_f64, 0.5,
//!     0.5, 1.0,
//! ], 2).unwrap();
//!
//! // Get Cholesky factor
//! let cholesky = corr.cholesky();
//!
//! // Transform independent normals to correlated
//! let z = [0.5_f64, 0.8];
//! let w = cholesky.transform(&z);
//! assert_eq!(w.len(), 2);
//! ```

use pricer_core::traits::Float;

/// Error types for correlation operations.
#[derive(Debug, Clone, PartialEq)]
pub enum CorrelationError {
    /// Matrix is not positive definite
    NotPositiveDefinite,
    /// Matrix dimensions are invalid
    InvalidDimensions { expected: usize, got: usize },
    /// Diagonal elements are not 1.0
    InvalidDiagonal { index: usize, value: f64 },
    /// Matrix is not symmetric
    NotSymmetric { i: usize, j: usize },
    /// Correlation value out of range [-1, 1]
    OutOfRange { i: usize, j: usize, value: f64 },
}

impl std::fmt::Display for CorrelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorrelationError::NotPositiveDefinite => {
                write!(f, "Correlation matrix is not positive definite")
            }
            CorrelationError::InvalidDimensions { expected, got } => {
                write!(
                    f,
                    "Invalid matrix dimensions: expected {} elements, got {}",
                    expected, got
                )
            }
            CorrelationError::InvalidDiagonal { index, value } => {
                write!(
                    f,
                    "Diagonal element at index {} is {}, expected 1.0",
                    index, value
                )
            }
            CorrelationError::NotSymmetric { i, j } => {
                write!(f, "Matrix is not symmetric at ({}, {})", i, j)
            }
            CorrelationError::OutOfRange { i, j, value } => {
                write!(
                    f,
                    "Correlation at ({}, {}) is {}, must be in [-1, 1]",
                    i, j, value
                )
            }
        }
    }
}

impl std::error::Error for CorrelationError {}

/// Correlation matrix with validation and Cholesky decomposition.
///
/// A correlation matrix must satisfy:
/// - Square and symmetric
/// - Diagonal elements equal to 1.0
/// - Off-diagonal elements in [-1, 1]
/// - Positive semi-definite (for Cholesky: positive definite)
#[derive(Clone, Debug)]
pub struct CorrelationMatrix<T: Float> {
    /// Matrix elements in row-major order
    data: Vec<T>,
    /// Matrix dimension (n x n)
    dim: usize,
}

impl<T: Float> CorrelationMatrix<T> {
    /// Create a new correlation matrix from flat array (row-major).
    ///
    /// # Arguments
    ///
    /// * `data` - Matrix elements in row-major order (n*n elements)
    /// * `dim` - Matrix dimension (n)
    ///
    /// # Returns
    ///
    /// `Ok(CorrelationMatrix)` if valid, `Err(CorrelationError)` otherwise.
    ///
    /// # Validation
    ///
    /// - Must have exactly dim*dim elements
    /// - Diagonal elements must be 1.0
    /// - Must be symmetric
    /// - Off-diagonal elements must be in [-1, 1]
    pub fn new(data: &[T], dim: usize) -> Result<Self, CorrelationError> {
        let expected = dim * dim;
        if data.len() != expected {
            return Err(CorrelationError::InvalidDimensions {
                expected,
                got: data.len(),
            });
        }

        let one = T::one();
        let neg_one = -T::one();
        let epsilon = T::from(1e-10).unwrap_or(T::zero());

        // Validate diagonal (must be 1.0)
        for i in 0..dim {
            let diag = data[i * dim + i];
            if (diag - one).abs() > epsilon {
                return Err(CorrelationError::InvalidDiagonal {
                    index: i,
                    value: diag.to_f64().unwrap_or(0.0),
                });
            }
        }

        // Validate symmetry and range
        for i in 0..dim {
            for j in (i + 1)..dim {
                let val_ij = data[i * dim + j];
                let val_ji = data[j * dim + i];

                // Check symmetry
                if (val_ij - val_ji).abs() > epsilon {
                    return Err(CorrelationError::NotSymmetric { i, j });
                }

                // Check range [-1, 1]
                if val_ij < neg_one || val_ij > one {
                    return Err(CorrelationError::OutOfRange {
                        i,
                        j,
                        value: val_ij.to_f64().unwrap_or(0.0),
                    });
                }
            }
        }

        Ok(Self {
            data: data.to_vec(),
            dim,
        })
    }

    /// Create an identity correlation matrix (no correlation).
    pub fn identity(dim: usize) -> Self {
        let mut data = vec![T::zero(); dim * dim];
        for i in 0..dim {
            data[i * dim + i] = T::one();
        }
        Self { data, dim }
    }

    /// Get matrix dimension.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Get element at (i, j).
    pub fn get(&self, i: usize, j: usize) -> T {
        self.data[i * self.dim + j]
    }

    /// Compute Cholesky decomposition (lower triangular L where C = L * L^T).
    ///
    /// # Returns
    ///
    /// `Ok(CholeskyFactor)` if decomposition succeeds (matrix is positive definite),
    /// `Err(CorrelationError::NotPositiveDefinite)` otherwise.
    pub fn cholesky(&self) -> Result<CholeskyFactor<T>, CorrelationError> {
        let n = self.dim;
        let mut lower = vec![T::zero(); n * n];

        for i in 0..n {
            for j in 0..=i {
                let mut sum = T::zero();

                if j == i {
                    // Diagonal element
                    for k in 0..j {
                        let l_jk = lower[j * n + k];
                        sum = sum + l_jk * l_jk;
                    }
                    let diag = self.get(j, j) - sum;
                    if diag <= T::zero() {
                        return Err(CorrelationError::NotPositiveDefinite);
                    }
                    lower[j * n + j] = diag.sqrt();
                } else {
                    // Off-diagonal element
                    for k in 0..j {
                        sum = sum + lower[i * n + k] * lower[j * n + k];
                    }
                    let l_jj = lower[j * n + j];
                    if l_jj <= T::zero() {
                        return Err(CorrelationError::NotPositiveDefinite);
                    }
                    lower[i * n + j] = (self.get(i, j) - sum) / l_jj;
                }
            }
        }

        Ok(CholeskyFactor { data: lower, dim: n })
    }
}

/// Lower triangular Cholesky factor of a correlation matrix.
///
/// Used to transform independent standard normals into correlated normals.
#[derive(Clone, Debug)]
pub struct CholeskyFactor<T: Float> {
    /// Lower triangular matrix elements (row-major)
    data: Vec<T>,
    /// Matrix dimension
    dim: usize,
}

impl<T: Float> CholeskyFactor<T> {
    /// Get matrix dimension.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Get element at (i, j).
    ///
    /// Returns zero for upper triangular elements (j > i).
    pub fn get(&self, i: usize, j: usize) -> T {
        if j > i {
            T::zero()
        } else {
            self.data[i * self.dim + j]
        }
    }

    /// Transform independent standard normals to correlated normals.
    ///
    /// Given independent Z ~ N(0,1), computes W = L * Z where L is the Cholesky factor.
    /// The resulting W has correlation structure matching the original matrix.
    ///
    /// # Arguments
    ///
    /// * `z` - Slice of independent standard normal random variables
    ///
    /// # Returns
    ///
    /// Vector of correlated standard normal random variables.
    ///
    /// # Panics
    ///
    /// Panics if `z.len() < self.dim()`.
    pub fn transform(&self, z: &[T]) -> Vec<T> {
        assert!(
            z.len() >= self.dim,
            "Input vector length {} is less than matrix dimension {}",
            z.len(),
            self.dim
        );

        let n = self.dim;
        let mut w = Vec::with_capacity(n);

        for i in 0..n {
            let mut sum = T::zero();
            for j in 0..=i {
                sum = sum + self.get(i, j) * z[j];
            }
            w.push(sum);
        }

        w
    }

    /// Transform independent normals in place.
    ///
    /// More efficient when you can reuse the input buffer.
    ///
    /// # Arguments
    ///
    /// * `z` - Mutable slice of independent standard normals (transformed in place)
    pub fn transform_inplace(&self, z: &mut [T]) {
        assert!(
            z.len() >= self.dim,
            "Input vector length {} is less than matrix dimension {}",
            z.len(),
            self.dim
        );

        let n = self.dim;
        let mut temp = vec![T::zero(); n];

        // Compute W = L * Z
        for i in 0..n {
            let mut sum = T::zero();
            for j in 0..=i {
                sum = sum + self.get(i, j) * z[j];
            }
            temp[i] = sum;
        }

        // Copy back
        z[..n].copy_from_slice(&temp);
    }
}

/// Container for multiple correlated stochastic models.
///
/// This struct manages multiple stochastic models that share correlated
/// Brownian motion drivers. The correlation is implemented via Cholesky
/// decomposition of the correlation matrix.
///
/// # Design
///
/// Rather than coupling specific model types, this struct provides:
/// - Storage for the Cholesky factor
/// - Methods to transform independent randoms to correlated randoms
/// - Model-agnostic correlation handling
///
/// The actual model evolution is handled by the caller using the
/// transformed random numbers.
#[derive(Clone, Debug)]
pub struct CorrelatedModels<T: Float> {
    /// Number of factors/models
    num_factors: usize,
    /// Cholesky factor for correlation
    cholesky: CholeskyFactor<T>,
}

impl<T: Float> CorrelatedModels<T> {
    /// Create a new correlated models container.
    ///
    /// # Arguments
    ///
    /// * `correlation_matrix` - Correlation matrix for the factors
    ///
    /// # Returns
    ///
    /// `Ok(CorrelatedModels)` if Cholesky decomposition succeeds,
    /// `Err(CorrelationError)` otherwise.
    pub fn new(correlation_matrix: CorrelationMatrix<T>) -> Result<Self, CorrelationError> {
        let num_factors = correlation_matrix.dim();
        let cholesky = correlation_matrix.cholesky()?;

        Ok(Self {
            num_factors,
            cholesky,
        })
    }

    /// Create uncorrelated models (identity correlation matrix).
    pub fn uncorrelated(num_factors: usize) -> Self {
        let identity = CorrelationMatrix::identity(num_factors);
        // Identity matrix always has valid Cholesky
        let cholesky = identity.cholesky().unwrap();

        Self {
            num_factors,
            cholesky,
        }
    }

    /// Get the number of factors.
    pub fn num_factors(&self) -> usize {
        self.num_factors
    }

    /// Get reference to the Cholesky factor.
    pub fn cholesky(&self) -> &CholeskyFactor<T> {
        &self.cholesky
    }

    /// Generate correlated Brownian increments from independent increments.
    ///
    /// # Arguments
    ///
    /// * `independent_dw` - Independent standard normal increments
    ///
    /// # Returns
    ///
    /// Correlated standard normal increments.
    pub fn correlate(&self, independent_dw: &[T]) -> Vec<T> {
        self.cholesky.transform(independent_dw)
    }

    /// Generate correlated Brownian increments in place.
    pub fn correlate_inplace(&self, dw: &mut [T]) {
        self.cholesky.transform_inplace(dw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 9.5: CorrelatedModels Tests (TDD)
    // ================================================================

    // Test: CorrelationMatrix validation
    #[test]
    fn test_correlation_matrix_valid() {
        let data = [1.0_f64, 0.5, 0.5, 1.0];
        let matrix = CorrelationMatrix::new(&data, 2);
        assert!(matrix.is_ok());

        let m = matrix.unwrap();
        assert_eq!(m.dim(), 2);
        assert_eq!(m.get(0, 0), 1.0);
        assert_eq!(m.get(0, 1), 0.5);
        assert_eq!(m.get(1, 0), 0.5);
        assert_eq!(m.get(1, 1), 1.0);
    }

    #[test]
    fn test_correlation_matrix_invalid_dimensions() {
        let data = [1.0_f64, 0.5, 0.5]; // Only 3 elements for 2x2
        let matrix = CorrelationMatrix::new(&data, 2);
        assert!(matches!(
            matrix,
            Err(CorrelationError::InvalidDimensions { .. })
        ));
    }

    #[test]
    fn test_correlation_matrix_invalid_diagonal() {
        let data = [0.9_f64, 0.5, 0.5, 1.0]; // First diagonal is 0.9
        let matrix = CorrelationMatrix::new(&data, 2);
        assert!(matches!(
            matrix,
            Err(CorrelationError::InvalidDiagonal { .. })
        ));
    }

    #[test]
    fn test_correlation_matrix_not_symmetric() {
        let data = [1.0_f64, 0.5, 0.3, 1.0]; // 0.5 != 0.3
        let matrix = CorrelationMatrix::new(&data, 2);
        assert!(matches!(matrix, Err(CorrelationError::NotSymmetric { .. })));
    }

    #[test]
    fn test_correlation_matrix_out_of_range() {
        let data = [1.0_f64, 1.5, 1.5, 1.0]; // Correlation > 1
        let matrix = CorrelationMatrix::new(&data, 2);
        assert!(matches!(matrix, Err(CorrelationError::OutOfRange { .. })));
    }

    #[test]
    fn test_correlation_matrix_identity() {
        let identity: CorrelationMatrix<f64> = CorrelationMatrix::identity(3);
        assert_eq!(identity.dim(), 3);
        assert_eq!(identity.get(0, 0), 1.0);
        assert_eq!(identity.get(0, 1), 0.0);
        assert_eq!(identity.get(1, 1), 1.0);
        assert_eq!(identity.get(2, 2), 1.0);
    }

    // Test: Cholesky decomposition
    #[test]
    fn test_cholesky_identity() {
        let identity: CorrelationMatrix<f64> = CorrelationMatrix::identity(2);
        let cholesky = identity.cholesky();
        assert!(cholesky.is_ok());

        let l = cholesky.unwrap();
        // Identity Cholesky is identity
        assert!((l.get(0, 0) - 1.0).abs() < 1e-10);
        assert_eq!(l.get(0, 1), 0.0);
        assert_eq!(l.get(1, 0), 0.0);
        assert!((l.get(1, 1) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cholesky_2x2() {
        let data = [1.0_f64, 0.5, 0.5, 1.0];
        let matrix = CorrelationMatrix::new(&data, 2).unwrap();
        let cholesky = matrix.cholesky();
        assert!(cholesky.is_ok());

        let l = cholesky.unwrap();
        // L = [[1, 0], [0.5, sqrt(0.75)]]
        assert!((l.get(0, 0) - 1.0).abs() < 1e-10);
        assert!((l.get(1, 0) - 0.5).abs() < 1e-10);
        assert!((l.get(1, 1) - (0.75_f64).sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_cholesky_reconstruction() {
        // Verify L * L^T = C
        let data = [1.0_f64, 0.5, 0.5, 1.0];
        let matrix = CorrelationMatrix::new(&data, 2).unwrap();
        let l = matrix.cholesky().unwrap();

        // Reconstruct: C[i,j] = sum_k L[i,k] * L[j,k]
        for i in 0..2 {
            for j in 0..2 {
                let mut sum = 0.0_f64;
                for k in 0..2 {
                    sum += l.get(i, k) * l.get(j, k);
                }
                assert!(
                    (sum - matrix.get(i, j)).abs() < 1e-10,
                    "Reconstruction failed at ({}, {})",
                    i,
                    j
                );
            }
        }
    }

    #[test]
    fn test_cholesky_not_positive_definite() {
        // This matrix is not positive definite
        // Using correlation = 1.0 exactly makes the matrix singular
        let data = [1.0_f64, 1.0, 1.0, 1.0];
        let matrix = CorrelationMatrix::new(&data, 2).unwrap();
        let cholesky = matrix.cholesky();
        assert!(matches!(
            cholesky,
            Err(CorrelationError::NotPositiveDefinite)
        ));
    }

    // Test: CholeskyFactor transform
    #[test]
    fn test_cholesky_transform_identity() {
        let identity: CorrelationMatrix<f64> = CorrelationMatrix::identity(2);
        let l = identity.cholesky().unwrap();

        let z = [0.5_f64, 0.8];
        let w = l.transform(&z);

        // For identity, W = Z
        assert!((w[0] - 0.5).abs() < 1e-10);
        assert!((w[1] - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_cholesky_transform_correlated() {
        let data = [1.0_f64, 0.5, 0.5, 1.0];
        let matrix = CorrelationMatrix::new(&data, 2).unwrap();
        let l = matrix.cholesky().unwrap();

        let z = [1.0_f64, 0.0]; // Z1 = 1, Z2 = 0
        let w = l.transform(&z);

        // W1 = L[0,0] * Z1 = 1.0
        // W2 = L[1,0] * Z1 + L[1,1] * Z2 = 0.5
        assert!((w[0] - 1.0).abs() < 1e-10);
        assert!((w[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_cholesky_transform_inplace() {
        let data = [1.0_f64, 0.5, 0.5, 1.0];
        let matrix = CorrelationMatrix::new(&data, 2).unwrap();
        let l = matrix.cholesky().unwrap();

        let mut z = [1.0_f64, 0.0];
        l.transform_inplace(&mut z);

        assert!((z[0] - 1.0).abs() < 1e-10);
        assert!((z[1] - 0.5).abs() < 1e-10);
    }

    // Test: CorrelatedModels
    #[test]
    fn test_correlated_models_new() {
        let data = [1.0_f64, 0.5, 0.5, 1.0];
        let matrix = CorrelationMatrix::new(&data, 2).unwrap();
        let models = CorrelatedModels::new(matrix);

        assert!(models.is_ok());
        let m = models.unwrap();
        assert_eq!(m.num_factors(), 2);
    }

    #[test]
    fn test_correlated_models_uncorrelated() {
        let models: CorrelatedModels<f64> = CorrelatedModels::uncorrelated(3);
        assert_eq!(models.num_factors(), 3);
    }

    #[test]
    fn test_correlated_models_correlate() {
        let data = [1.0_f64, 0.5, 0.5, 1.0];
        let matrix = CorrelationMatrix::new(&data, 2).unwrap();
        let models = CorrelatedModels::new(matrix).unwrap();

        let independent = [1.0_f64, 0.0];
        let correlated = models.correlate(&independent);

        assert!((correlated[0] - 1.0).abs() < 1e-10);
        assert!((correlated[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_correlated_models_correlate_inplace() {
        let data = [1.0_f64, 0.5, 0.5, 1.0];
        let matrix = CorrelationMatrix::new(&data, 2).unwrap();
        let models = CorrelatedModels::new(matrix).unwrap();

        let mut dw = [1.0_f64, 0.0];
        models.correlate_inplace(&mut dw);

        assert!((dw[0] - 1.0).abs() < 1e-10);
        assert!((dw[1] - 0.5).abs() < 1e-10);
    }

    // Test: 3x3 correlation matrix
    #[test]
    fn test_correlation_matrix_3x3() {
        #[rustfmt::skip]
        let data = [
            1.0_f64, 0.3, 0.2,
            0.3, 1.0, 0.4,
            0.2, 0.4, 1.0,
        ];
        let matrix = CorrelationMatrix::new(&data, 3);
        assert!(matrix.is_ok());

        let m = matrix.unwrap();
        let cholesky = m.cholesky();
        assert!(cholesky.is_ok());
    }

    // Test: f32 compatibility
    #[test]
    fn test_correlation_matrix_f32() {
        let data = [1.0_f32, 0.5, 0.5, 1.0];
        let matrix = CorrelationMatrix::new(&data, 2);
        assert!(matrix.is_ok());

        let m = matrix.unwrap();
        let cholesky = m.cholesky();
        assert!(cholesky.is_ok());

        let l = cholesky.unwrap();
        let z = [1.0_f32, 0.0];
        let w = l.transform(&z);
        assert!((w[0] - 1.0_f32).abs() < 1e-6);
    }

    // Test: Error display
    #[test]
    fn test_correlation_error_display() {
        let err = CorrelationError::NotPositiveDefinite;
        assert!(err.to_string().contains("positive definite"));

        let err = CorrelationError::InvalidDimensions {
            expected: 4,
            got: 3,
        };
        assert!(err.to_string().contains("4"));
        assert!(err.to_string().contains("3"));
    }
}
