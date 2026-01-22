//! Result types for fitting operations.

/// Result of a fitting operation.
///
/// Contains the fitted parameters along with diagnostic information
/// such as residuals and goodness-of-fit measures.
#[derive(Debug, Clone)]
pub struct FittingResult<T> {
    /// Fitted parameters/coefficients.
    pub params: Vec<T>,
    /// Residuals (y - y_fitted) for each data point.
    pub residuals: Vec<T>,
    /// Coefficient of determination (R²).
    ///
    /// R² = 1 - SS_res / SS_tot
    /// where SS_res is sum of squared residuals
    /// and SS_tot is total sum of squares.
    pub r_squared: T,
    /// Sum of squared residuals.
    pub ss_residual: T,
    /// Total sum of squares.
    pub ss_total: T,
    /// Number of data points.
    pub n_points: usize,
    /// Number of parameters fitted.
    pub n_params: usize,
}

impl<T: Copy> FittingResult<T> {
    /// Create a new fitting result.
    #[must_use]
    pub fn new(
        params: Vec<T>,
        residuals: Vec<T>,
        r_squared: T,
        ss_residual: T,
        ss_total: T,
        n_params: usize,
    ) -> Self {
        let n_points = residuals.len();
        Self {
            params,
            residuals,
            r_squared,
            ss_residual,
            ss_total,
            n_points,
            n_params,
        }
    }
}

/// Result of Gaussian fitting.
#[derive(Debug, Clone, Copy)]
pub struct GaussianFitResult<T> {
    /// Estimated mean (mu).
    pub mean: T,
    /// Estimated standard deviation (sigma).
    pub std_dev: T,
    /// Estimated amplitude (peak height).
    pub amplitude: T,
}

impl<T: Copy> GaussianFitResult<T> {
    /// Create a new Gaussian fit result.
    #[must_use]
    pub fn new(mean: T, std_dev: T, amplitude: T) -> Self {
        Self {
            mean,
            std_dev,
            amplitude,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fitting_result_creation() {
        let result: FittingResult<f64> =
            FittingResult::new(vec![1.0, 2.0], vec![0.1, -0.1, 0.05], 0.95, 0.05, 1.0, 2);
        assert_eq!(result.params, vec![1.0, 2.0]);
        assert_eq!(result.n_points, 3);
        assert_eq!(result.n_params, 2);
        assert!((result.r_squared - 0.95).abs() < 1e-10);
    }

    #[test]
    fn test_fitting_result_clone() {
        let result: FittingResult<f64> =
            FittingResult::new(vec![1.0, 2.0], vec![0.1, -0.1], 0.95, 0.05, 1.0, 2);
        let cloned = result.clone();
        assert_eq!(result.params, cloned.params);
        assert_eq!(result.r_squared, cloned.r_squared);
    }

    #[test]
    fn test_gaussian_fit_result_creation() {
        let result: GaussianFitResult<f64> = GaussianFitResult::new(0.0, 1.0, 0.398);
        assert!((result.mean - 0.0).abs() < 1e-10);
        assert!((result.std_dev - 1.0).abs() < 1e-10);
        assert!((result.amplitude - 0.398).abs() < 1e-10);
    }

    #[test]
    fn test_gaussian_fit_result_copy() {
        let result: GaussianFitResult<f64> = GaussianFitResult::new(0.0, 1.0, 0.398);
        let copied = result;
        assert_eq!(result.mean, copied.mean);
        assert_eq!(result.std_dev, copied.std_dev);
    }
}
