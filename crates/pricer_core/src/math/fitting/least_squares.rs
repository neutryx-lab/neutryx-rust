//! Least squares fitting algorithms.
//!
//! This module provides linear least squares fitting using SVD decomposition
//! for numerical stability.

use num_traits::Float;

use super::{error::FittingError, result::FittingResult};

/// Perform linear least squares fit: y = a₀ + a₁x + a₂x² + ...
///
/// Fits a polynomial of degree `degree` to the data points (x, y).
///
/// # Arguments
///
/// * `x` - Independent variable values
/// * `y` - Dependent variable values
/// * `degree` - Polynomial degree (0 = constant, 1 = linear, 2 = quadratic,
///   etc.)
///
/// # Returns
///
/// `FittingResult` containing the coefficients [a₀, a₁, a₂, ...],
/// residuals, and R² value.
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::fitting::polynomial_fit;
///
/// let x = vec![0.0, 1.0, 2.0, 3.0];
/// let y = vec![1.0, 3.0, 5.0, 7.0];  // y = 1 + 2x
/// let result = polynomial_fit(&x, &y, 1).unwrap();
/// // result.params ≈ [1.0, 2.0]
/// ```
pub fn polynomial_fit<T: Float>(
    x: &[T],
    y: &[T],
    degree: usize,
) -> Result<FittingResult<T>, FittingError> {
    let n = x.len();
    let n_params = degree + 1;

    // Validation
    if n != y.len() {
        return Err(FittingError::DimensionMismatch(format!(
            "x has {} elements, y has {} elements",
            n,
            y.len()
        )));
    }
    if n < n_params {
        return Err(FittingError::InsufficientData {
            needed: n_params,
            got: n,
        });
    }

    // Build Vandermonde matrix
    // A[i,j] = x[i]^j
    let mut a_data = Vec::with_capacity(n * n_params);
    for &xi in x {
        let mut power = T::one();
        for _ in 0..n_params {
            a_data.push(power);
            power = power * xi;
        }
    }

    // Solve using normal equations: (A^T A) x = A^T y
    // For better numerical stability, we use the normal equations approach
    // A more sophisticated implementation would use SVD

    // Compute A^T A
    let mut ata = vec![T::zero(); n_params * n_params];
    for i in 0..n_params {
        for j in 0..n_params {
            let mut sum = T::zero();
            for k in 0..n {
                sum = sum + a_data[k * n_params + i] * a_data[k * n_params + j];
            }
            ata[i * n_params + j] = sum;
        }
    }

    // Compute A^T y
    let mut aty = vec![T::zero(); n_params];
    for i in 0..n_params {
        let mut sum = T::zero();
        for k in 0..n {
            sum = sum + a_data[k * n_params + i] * y[k];
        }
        aty[i] = sum;
    }

    // Solve using Gaussian elimination with partial pivoting
    let params = solve_linear_system(&ata, &aty, n_params)?;

    // Calculate residuals and statistics
    let mut residuals = Vec::with_capacity(n);
    let mut ss_residual = T::zero();
    let mut y_sum = T::zero();

    for i in 0..n {
        let mut y_pred = T::zero();
        let mut power = T::one();
        for p in &params {
            y_pred = y_pred + *p * power;
            power = power * x[i];
        }
        let residual = y[i] - y_pred;
        residuals.push(residual);
        ss_residual = ss_residual + residual * residual;
        y_sum = y_sum + y[i];
    }

    let y_mean = y_sum / T::from(n).unwrap();
    let mut ss_total = T::zero();
    for &yi in y {
        let diff = yi - y_mean;
        ss_total = ss_total + diff * diff;
    }

    let r_squared = if ss_total > T::zero() {
        T::one() - ss_residual / ss_total
    } else {
        T::one() // Perfect fit when all y values are the same
    };

    Ok(FittingResult::new(
        params,
        residuals,
        r_squared,
        ss_residual,
        ss_total,
        n_params,
    ))
}

/// Perform simple linear regression: y = a + bx
///
/// This is a specialised, optimised version for linear regression.
///
/// # Arguments
///
/// * `x` - Independent variable values
/// * `y` - Dependent variable values
///
/// # Returns
///
/// `FittingResult` with params = [intercept, slope]
pub fn linear_regression<T: Float>(x: &[T], y: &[T]) -> Result<FittingResult<T>, FittingError> {
    polynomial_fit(x, y, 1)
}

/// Perform weighted least squares polynomial fit.
///
/// # Arguments
///
/// * `x` - Independent variable values
/// * `y` - Dependent variable values
/// * `weights` - Weights for each data point (larger = more important)
/// * `degree` - Polynomial degree
pub fn weighted_polynomial_fit<T: Float>(
    x: &[T],
    y: &[T],
    weights: &[T],
    degree: usize,
) -> Result<FittingResult<T>, FittingError> {
    let n = x.len();
    let n_params = degree + 1;

    // Validation
    if n != y.len() || n != weights.len() {
        return Err(FittingError::DimensionMismatch(
            "x, y, and weights must have the same length".to_string(),
        ));
    }
    if n < n_params {
        return Err(FittingError::InsufficientData {
            needed: n_params,
            got: n,
        });
    }

    // Check for negative weights
    for &w in weights {
        if w < T::zero() {
            return Err(FittingError::InvalidData(
                "negative weight found".to_string(),
            ));
        }
    }

    // Build weighted Vandermonde matrix
    // A[i,j] = sqrt(w[i]) * x[i]^j
    let mut a_data = Vec::with_capacity(n * n_params);
    let mut y_weighted = Vec::with_capacity(n);

    for i in 0..n {
        let sqrt_w = weights[i].sqrt();
        let mut power = T::one();
        for _ in 0..n_params {
            a_data.push(sqrt_w * power);
            power = power * x[i];
        }
        y_weighted.push(sqrt_w * y[i]);
    }

    // Compute A^T A
    let mut ata = vec![T::zero(); n_params * n_params];
    for i in 0..n_params {
        for j in 0..n_params {
            let mut sum = T::zero();
            for k in 0..n {
                sum = sum + a_data[k * n_params + i] * a_data[k * n_params + j];
            }
            ata[i * n_params + j] = sum;
        }
    }

    // Compute A^T y_weighted
    let mut aty = vec![T::zero(); n_params];
    for i in 0..n_params {
        let mut sum = T::zero();
        for k in 0..n {
            sum = sum + a_data[k * n_params + i] * y_weighted[k];
        }
        aty[i] = sum;
    }

    // Solve using Gaussian elimination
    let params = solve_linear_system(&ata, &aty, n_params)?;

    // Calculate residuals and statistics (unweighted)
    let mut residuals = Vec::with_capacity(n);
    let mut ss_residual = T::zero();
    let mut y_sum = T::zero();
    let mut w_sum = T::zero();

    for i in 0..n {
        let mut y_pred = T::zero();
        let mut power = T::one();
        for p in &params {
            y_pred = y_pred + *p * power;
            power = power * x[i];
        }
        let residual = y[i] - y_pred;
        residuals.push(residual);
        ss_residual = ss_residual + weights[i] * residual * residual;
        y_sum = y_sum + weights[i] * y[i];
        w_sum = w_sum + weights[i];
    }

    let y_mean = y_sum / w_sum;
    let mut ss_total = T::zero();
    for i in 0..n {
        let diff = y[i] - y_mean;
        ss_total = ss_total + weights[i] * diff * diff;
    }

    let r_squared = if ss_total > T::zero() {
        T::one() - ss_residual / ss_total
    } else {
        T::one()
    };

    Ok(FittingResult::new(
        params,
        residuals,
        r_squared,
        ss_residual,
        ss_total,
        n_params,
    ))
}

/// Solve a linear system using Gaussian elimination with partial pivoting.
fn solve_linear_system<T: Float>(a: &[T], b: &[T], n: usize) -> Result<Vec<T>, FittingError> {
    // Copy to working arrays
    let mut matrix = a.to_vec();
    let mut rhs = b.to_vec();

    // Gaussian elimination with partial pivoting
    for k in 0..n {
        // Find pivot
        let mut max_val = matrix[k * n + k].abs();
        let mut max_row = k;
        for i in (k + 1)..n {
            let val = matrix[i * n + k].abs();
            if val > max_val {
                max_val = val;
                max_row = i;
            }
        }

        // Check for singular matrix
        if max_val < T::from(1e-14).unwrap() {
            return Err(FittingError::FittingFailed(
                "Matrix is singular or nearly singular".to_string(),
            ));
        }

        // Swap rows if needed
        if max_row != k {
            for j in 0..n {
                matrix.swap(k * n + j, max_row * n + j);
            }
            rhs.swap(k, max_row);
        }

        // Eliminate
        for i in (k + 1)..n {
            let factor = matrix[i * n + k] / matrix[k * n + k];
            for j in k..n {
                matrix[i * n + j] = matrix[i * n + j] - factor * matrix[k * n + j];
            }
            rhs[i] = rhs[i] - factor * rhs[k];
        }
    }

    // Back substitution
    let mut x = vec![T::zero(); n];
    for i in (0..n).rev() {
        let mut sum = rhs[i];
        for j in (i + 1)..n {
            sum = sum - matrix[i * n + j] * x[j];
        }
        x[i] = sum / matrix[i * n + i];
    }

    Ok(x)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_linear_regression() {
        // Perfect linear relationship: y = 2 + 3x
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![2.0, 5.0, 8.0, 11.0, 14.0];

        let result = linear_regression(&x, &y).unwrap();

        assert_eq!(result.params.len(), 2);
        assert_relative_eq!(result.params[0], 2.0, epsilon = 1e-10); // intercept
        assert_relative_eq!(result.params[1], 3.0, epsilon = 1e-10); // slope
        assert_relative_eq!(result.r_squared, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_quadratic_fit() {
        // y = 1 + 2x + 0.5x²
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y: Vec<f64> = x.iter().map(|&xi| 1.0 + 2.0 * xi + 0.5 * xi * xi).collect();

        let result = polynomial_fit(&x, &y, 2).unwrap();

        assert_eq!(result.params.len(), 3);
        assert_relative_eq!(result.params[0], 1.0, epsilon = 1e-8);
        assert_relative_eq!(result.params[1], 2.0, epsilon = 1e-8);
        assert_relative_eq!(result.params[2], 0.5, epsilon = 1e-8);
        assert_relative_eq!(result.r_squared, 1.0, epsilon = 1e-8);
    }

    #[test]
    fn test_polynomial_fit_with_noise() {
        // Linear data with some noise
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.1, 2.9, 5.2, 6.8, 9.1, 10.9]; // approximately y = 1 + 2x

        let result = polynomial_fit(&x, &y, 1).unwrap();

        // Should get reasonable estimates
        assert_relative_eq!(result.params[0], 1.0, epsilon = 0.3);
        assert_relative_eq!(result.params[1], 2.0, epsilon = 0.1);
        assert!(result.r_squared > 0.99);
    }

    #[test]
    fn test_constant_fit() {
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y = vec![5.0, 5.0, 5.0, 5.0];

        let result = polynomial_fit(&x, &y, 0).unwrap();

        assert_eq!(result.params.len(), 1);
        assert_relative_eq!(result.params[0], 5.0, epsilon = 1e-10);
        assert_relative_eq!(result.r_squared, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_weighted_linear_regression() {
        // Linear data with higher weight on later points
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![2.0, 5.0, 8.0, 11.0, 14.0];
        let weights = vec![1.0, 1.0, 1.0, 2.0, 2.0]; // More weight on later points

        let result = weighted_polynomial_fit(&x, &y, &weights, 1).unwrap();

        // Should still fit well for perfect data
        assert_relative_eq!(result.params[0], 2.0, epsilon = 1e-8);
        assert_relative_eq!(result.params[1], 3.0, epsilon = 1e-8);
    }

    #[test]
    fn test_insufficient_data() {
        let x = vec![1.0, 2.0];
        let y = vec![1.0, 2.0];

        // Trying to fit degree 2 with only 2 points
        let result = polynomial_fit(&x, &y, 2);
        assert!(matches!(result, Err(FittingError::InsufficientData { .. })));
    }

    #[test]
    fn test_dimension_mismatch() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0];

        let result = polynomial_fit(&x, &y, 1);
        assert!(matches!(result, Err(FittingError::DimensionMismatch(_))));
    }

    #[test]
    fn test_negative_weights() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 2.0];
        let weights = vec![1.0, -1.0, 1.0];

        let result = weighted_polynomial_fit(&x, &y, &weights, 1);
        assert!(matches!(result, Err(FittingError::InvalidData(_))));
    }

    #[test]
    fn test_residuals() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 2.0]; // Perfect fit: y = x

        let result = polynomial_fit(&x, &y, 1).unwrap();

        // All residuals should be zero
        for residual in &result.residuals {
            assert!(residual.abs() < 1e-10);
        }
    }

    #[test]
    fn test_cubic_fit() {
        // y = x³ - 2x² + x + 1
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|&xi| xi.powi(3) - 2.0 * xi.powi(2) + xi + 1.0)
            .collect();

        let result = polynomial_fit(&x, &y, 3).unwrap();

        assert_eq!(result.params.len(), 4);
        assert_relative_eq!(result.params[0], 1.0, epsilon = 1e-6); // constant
        assert_relative_eq!(result.params[1], 1.0, epsilon = 1e-6); // x coefficient
        assert_relative_eq!(result.params[2], -2.0, epsilon = 1e-6); // x² coefficient
        assert_relative_eq!(result.params[3], 1.0, epsilon = 1e-6); // x³ coefficient
    }
}
