//! Gaussian distribution fitting.
//!
//! This module provides functions to fit Gaussian distributions to data.

use num_traits::Float;

use super::error::FittingError;
use super::result::GaussianFitResult;

/// Fit a Gaussian distribution to sample data.
///
/// Estimates the mean and standard deviation from sample data
/// using maximum likelihood estimation.
///
/// # Arguments
///
/// * `data` - Sample data points
///
/// # Returns
///
/// `GaussianFitResult` containing the estimated mean, standard deviation,
/// and amplitude (normalised to integrate to 1).
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::fitting::fit_gaussian;
///
/// let samples = vec![0.1, -0.2, 0.05, 0.15, -0.1];
/// let result = fit_gaussian(&samples).unwrap();
/// println!("Mean: {}, Std: {}", result.mean, result.std_dev);
/// ```
pub fn fit_gaussian<T: Float>(data: &[T]) -> Result<GaussianFitResult<T>, FittingError> {
    let n = data.len();
    if n < 2 {
        return Err(FittingError::InsufficientData { needed: 2, got: n });
    }

    // Calculate sample mean
    let mut sum = T::zero();
    for &x in data {
        sum = sum + x;
    }
    let mean = sum / T::from(n).unwrap();

    // Calculate sample variance (unbiased estimator with n-1)
    let mut sq_sum = T::zero();
    for &x in data {
        let diff = x - mean;
        sq_sum = sq_sum + diff * diff;
    }
    let variance = sq_sum / T::from(n - 1).unwrap();

    if variance <= T::zero() {
        return Err(FittingError::NumericalError(
            "Variance is zero or negative".to_string(),
        ));
    }

    let std_dev = variance.sqrt();

    // Amplitude for normalised Gaussian: 1 / (sqrt(2*pi) * sigma)
    let two_pi = T::from(2.0 * std::f64::consts::PI).unwrap();
    let amplitude = T::one() / (two_pi.sqrt() * std_dev);

    Ok(GaussianFitResult::new(mean, std_dev, amplitude))
}

/// Fit a Gaussian to x-y data points representing a Gaussian curve.
///
/// Given data points (x, y) where y is expected to follow a Gaussian shape,
/// estimates the parameters (mean, std_dev, amplitude) by linearising the problem.
///
/// Uses the method: ln(y) = ln(A) - (x - mu)² / (2 * sigma²)
///
/// # Arguments
///
/// * `x` - X coordinates
/// * `y` - Y coordinates (must be positive)
///
/// # Returns
///
/// `GaussianFitResult` with estimated parameters.
///
/// # Note
///
/// This method works best when the data closely follows a Gaussian shape.
/// For noisy data or data that doesn't follow a Gaussian, results may be unreliable.
pub fn fit_gaussian_curve<T: Float>(
    x: &[T],
    y: &[T],
) -> Result<GaussianFitResult<T>, FittingError> {
    let n = x.len();
    if n != y.len() {
        return Err(FittingError::DimensionMismatch(
            "x and y must have the same length".to_string(),
        ));
    }
    if n < 3 {
        return Err(FittingError::InsufficientData { needed: 3, got: n });
    }

    // Filter out non-positive y values and transform
    let mut valid_x = Vec::with_capacity(n);
    let mut log_y = Vec::with_capacity(n);

    for i in 0..n {
        if y[i] > T::zero() {
            valid_x.push(x[i]);
            log_y.push(y[i].ln());
        }
    }

    if valid_x.len() < 3 {
        return Err(FittingError::InvalidData(
            "Need at least 3 positive y values".to_string(),
        ));
    }

    // Find the peak (maximum y value) as initial guess for mean
    let max_idx = y
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap();
    let peak_x = x[max_idx];
    let peak_y = y[max_idx];

    // Fit parabola to log(y): ln(y) = a + b*x + c*x²
    // This corresponds to: ln(y) = ln(A) - (x - mu)²/(2*sigma²)
    // Expanding: ln(y) = ln(A) - mu²/(2*sigma²) + (mu/sigma²)*x - x²/(2*sigma²)
    // So: a = ln(A) - mu²/(2*sigma²), b = mu/sigma², c = -1/(2*sigma²)

    let m = valid_x.len();
    let n_params = 3;

    // Build normal equations for quadratic fit
    let mut sums = [T::zero(); 5]; // sum of x^0, x^1, x^2, x^3, x^4
    let mut sy = [T::zero(); 3]; // sum of y*x^0, y*x^1, y*x^2

    for i in 0..m {
        let xi = valid_x[i];
        let yi = log_y[i];
        let mut power = T::one();
        for j in 0..5 {
            sums[j] = sums[j] + power;
            if j < 3 {
                sy[j] = sy[j] + yi * power;
            }
            power = power * xi;
        }
    }

    // Build 3x3 matrix and solve
    let mat = [
        sums[0], sums[1], sums[2], sums[1], sums[2], sums[3], sums[2], sums[3], sums[4],
    ];
    let rhs = [sy[0], sy[1], sy[2]];

    let coeffs = solve_3x3(&mat, &rhs)?;

    let a = coeffs[0];
    let b = coeffs[1];
    let c = coeffs[2];

    // Extract Gaussian parameters
    // c = -1/(2*sigma²), so sigma² = -1/(2*c), sigma = sqrt(-1/(2*c))
    if c >= T::zero() {
        // Not a proper Gaussian (would be convex, not concave)
        // Fall back to simpler estimation
        let mean = peak_x;
        let amplitude = peak_y;

        // Estimate std_dev from half-width at half-maximum
        let half_max = peak_y / T::from(2.0).unwrap();
        let mut left_idx = max_idx;
        let mut right_idx = max_idx;

        for i in (0..max_idx).rev() {
            if y[i] < half_max {
                left_idx = i;
                break;
            }
        }
        for i in (max_idx + 1)..n {
            if y[i] < half_max {
                right_idx = i;
                break;
            }
        }

        let fwhm = x[right_idx] - x[left_idx];
        let std_dev = fwhm / T::from(2.355).unwrap(); // FWHM = 2*sqrt(2*ln(2))*sigma ≈ 2.355*sigma

        return Ok(GaussianFitResult::new(mean, std_dev, amplitude));
    }

    let sigma_sq = -T::one() / (T::from(2.0).unwrap() * c);
    let sigma = sigma_sq.sqrt();

    // b = mu/sigma², so mu = b * sigma²
    let mu = b * sigma_sq;

    // a = ln(A) - mu²/(2*sigma²), so ln(A) = a + mu²/(2*sigma²)
    let ln_a = a + mu * mu / (T::from(2.0).unwrap() * sigma_sq);
    let amplitude = ln_a.exp();

    Ok(GaussianFitResult::new(mu, sigma, amplitude))
}

/// Solve a 3x3 linear system using Cramer's rule.
fn solve_3x3<T: Float>(mat: &[T; 9], rhs: &[T; 3]) -> Result<[T; 3], FittingError> {
    // Compute determinant of the main matrix
    let det = mat[0] * (mat[4] * mat[8] - mat[5] * mat[7])
        - mat[1] * (mat[3] * mat[8] - mat[5] * mat[6])
        + mat[2] * (mat[3] * mat[7] - mat[4] * mat[6]);

    if det.abs() < T::from(1e-14).unwrap() {
        return Err(FittingError::FittingFailed("Singular matrix".to_string()));
    }

    // Solve using Cramer's rule
    let det_x = rhs[0] * (mat[4] * mat[8] - mat[5] * mat[7])
        - mat[1] * (rhs[1] * mat[8] - mat[5] * rhs[2])
        + mat[2] * (rhs[1] * mat[7] - mat[4] * rhs[2]);

    let det_y = mat[0] * (rhs[1] * mat[8] - mat[5] * rhs[2])
        - rhs[0] * (mat[3] * mat[8] - mat[5] * mat[6])
        + mat[2] * (mat[3] * rhs[2] - rhs[1] * mat[6]);

    let det_z = mat[0] * (mat[4] * rhs[2] - rhs[1] * mat[7])
        - mat[1] * (mat[3] * rhs[2] - rhs[1] * mat[6])
        + rhs[0] * (mat[3] * mat[7] - mat[4] * mat[6]);

    Ok([det_x / det, det_y / det, det_z / det])
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_fit_gaussian_sample() {
        // Standard normal samples (approximate)
        let data = vec![-0.5, 0.2, -0.1, 0.3, 0.0, -0.2, 0.1, 0.4, -0.3, 0.15];

        let result = fit_gaussian(&data).unwrap();

        // Mean should be close to 0
        assert!(result.mean.abs() < 0.5);
        // Std dev should be reasonable
        assert!(result.std_dev > 0.0);
        assert!(result.std_dev < 1.0);
    }

    #[test]
    fn test_fit_gaussian_known_distribution() {
        // Generate samples from N(5, 2) - mean 5, std 2
        // Using deterministic "samples"
        let mean_true = 5.0;
        let std_true = 2.0;

        // Create samples that have known mean and variance
        // Use symmetric samples around mean for exact results
        // offsets = [-2, -1, 0, 1, 2], samples = [1, 3, 5, 7, 9]
        let offsets = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let data: Vec<f64> = offsets.iter().map(|o| mean_true + std_true * o).collect();

        let result = fit_gaussian(&data).unwrap();

        assert_relative_eq!(result.mean, mean_true, epsilon = 1e-10);
        // Unbiased variance estimate: sum((xi - mean)^2) / (n-1)
        // deviations from mean: [-4, -2, 0, 2, 4], squared: [16, 4, 0, 4, 16], sum = 40
        // variance = 40/4 = 10, std_dev = sqrt(10)
        let expected_std = 10.0_f64.sqrt();
        assert_relative_eq!(result.std_dev, expected_std, epsilon = 1e-10);
    }

    #[test]
    fn test_fit_gaussian_insufficient_data() {
        let data = vec![1.0];
        let result = fit_gaussian(&data);
        assert!(matches!(
            result,
            Err(FittingError::InsufficientData { .. })
        ));
    }

    #[test]
    fn test_fit_gaussian_curve_perfect() {
        // Generate perfect Gaussian curve: y = A * exp(-(x-mu)²/(2*sigma²))
        let mu = 2.0;
        let sigma = 0.5;
        let amplitude = 1.0;

        let x: Vec<f64> = (-20..=20).map(|i| mu + (i as f64) * 0.1).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|&xi| amplitude * (-(xi - mu).powi(2) / (2.0 * sigma.powi(2))).exp())
            .collect();

        let result = fit_gaussian_curve(&x, &y).unwrap();

        assert_relative_eq!(result.mean, mu, epsilon = 0.1);
        assert_relative_eq!(result.std_dev, sigma, epsilon = 0.1);
    }

    #[test]
    fn test_fit_gaussian_curve_dimension_mismatch() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0];

        let result = fit_gaussian_curve(&x, &y);
        assert!(matches!(result, Err(FittingError::DimensionMismatch(_))));
    }

    #[test]
    fn test_fit_gaussian_curve_insufficient_data() {
        let x = vec![1.0, 2.0];
        let y = vec![1.0, 2.0];

        let result = fit_gaussian_curve(&x, &y);
        assert!(matches!(
            result,
            Err(FittingError::InsufficientData { .. })
        ));
    }

    #[test]
    fn test_gaussian_result_fields() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = fit_gaussian(&data).unwrap();

        // Check that all fields are accessible and reasonable
        assert!(!result.mean.is_nan());
        assert!(!result.std_dev.is_nan());
        assert!(!result.amplitude.is_nan());
        assert!(result.std_dev > 0.0);
        assert!(result.amplitude > 0.0);
    }
}
