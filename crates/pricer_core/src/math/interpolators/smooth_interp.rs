//! Branch-free smooth interpolation for Enzyme AD compatibility.

use num_traits::Float;

use crate::{
    math::{numeric::from_f64, smoothing::smooth_indicator},
    types::InterpolationError,
};

/// Branch-free smooth linear interpolation.
///
/// Uses sigmoid-based blending via `smooth_indicator` to select segments
/// without conditional branches. This ensures compatibility with Enzyme
/// automatic differentiation which requires branch-free code paths.
///
/// # Arguments
///
/// * `xs` - Slice of x-coordinates (must be sorted in ascending order)
/// * `ys` - Slice of corresponding y-values
/// * `x` - Query point
/// * `epsilon` - Smoothing parameter (recommended: 1e-6 to 1e-3)
///
/// # Returns
///
/// * `Ok(y)` - The smoothly interpolated value
/// * `Err(InterpolationError::InsufficientData)` - Fewer than 2 data points
/// * `Err(InterpolationError::InvalidInput)` - Mismatched array lengths
///
/// # Convergence
///
/// As `epsilon → 0`, `smooth_interp` converges to standard linear
/// interpolation.
///
/// # Branch-free Property
///
/// This function does not use `if` statements on Float values for segment
/// selection. Instead, all segments contribute with sigmoid-weighted factors.
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::smooth_interp;
///
/// let xs = [0.0, 1.0, 2.0, 3.0];
/// let ys = [0.0, 1.0, 4.0, 9.0];
///
/// let y = smooth_interp(&xs, &ys, 1.5, 1e-6).unwrap();
/// // y is approximately 2.5 (linear interpolation between 1.0 and 4.0)
/// ```
pub fn smooth_interp<T: Float>(
    xs: &[T],
    ys: &[T],
    x: T,
    epsilon: T,
) -> Result<T, InterpolationError> {
    // Validate array lengths match
    if xs.len() != ys.len() {
        return Err(InterpolationError::InvalidInput(format!(
            "xs and ys must have same length: got {} and {}",
            xs.len(),
            ys.len()
        )));
    }

    // Validate minimum data points
    if xs.len() < 2 {
        return Err(InterpolationError::InsufficientData {
            got: xs.len(),
            need: 2,
        });
    }

    let n = xs.len();

    // Compute segment weights using smooth_indicator
    // Weight for segment i: w_i = smooth_indicator(x - x_i, ε) - smooth_indicator(x
    // - x_{i+1}, ε) This gives w_i ≈ 1 when x is in segment [x_i, x_{i+1}], ≈ 0
    // otherwise

    let mut weighted_sum = T::zero();
    let mut weight_sum = T::zero();

    for i in 0..n - 1 {
        // Compute weight for segment i
        let indicator_left = smooth_indicator(x - xs[i], epsilon);
        let indicator_right = smooth_indicator(x - xs[i + 1], epsilon);
        let weight = indicator_left - indicator_right;

        // Linear interpolation value for this segment
        let h = xs[i + 1] - xs[i];
        let t = (x - xs[i]) / h;
        let y_segment = ys[i] + (ys[i + 1] - ys[i]) * t;

        // Accumulate weighted sum
        weighted_sum = weighted_sum + weight * y_segment;
        weight_sum = weight_sum + weight;
    }

    // Normalise by total weight (should be approximately 1 for points inside
    // domain) Add small epsilon to avoid division by zero for points far
    // outside domain
    let tiny: T = from_f64(1e-30);
    let result = weighted_sum / (weight_sum + tiny);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Construction/Validation Tests (Task 6.1)
    // ========================================

    #[test]
    fn test_insufficient_data_one_point() {
        let xs = [1.0];
        let ys = [2.0];
        let result = smooth_interp(&xs, &ys, 1.0, 1e-6);
        assert!(result.is_err());

        match result.unwrap_err() {
            InterpolationError::InsufficientData { got, need } => {
                assert_eq!(got, 1);
                assert_eq!(need, 2);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_mismatched_lengths() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0];
        let result = smooth_interp(&xs, &ys, 1.0, 1e-6);
        assert!(result.is_err());

        match result.unwrap_err() {
            InterpolationError::InvalidInput(msg) => {
                assert!(msg.contains("same length"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    // ========================================
    // Convergence Tests (Task 6.2)
    // ========================================

    #[test]
    fn test_convergence_to_linear_at_midpoint() {
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 1.0, 4.0, 9.0];

        // Test at midpoint x = 1.5
        // Linear interpolation: y = 1.0 + (4.0 - 1.0) * 0.5 = 2.5
        let expected = 2.5;

        // As epsilon decreases, should converge to expected
        let eps_values = [1e-2, 1e-4, 1e-6];
        let tolerances = [0.5, 0.1, 0.01];

        for (eps, tol) in eps_values.iter().zip(tolerances.iter()) {
            let result = smooth_interp(&xs, &ys, 1.5, *eps).unwrap();
            assert!(
                (result - expected).abs() < *tol,
                "At eps={}, expected ~{}, got {}",
                eps,
                expected,
                result
            );
        }
    }

    #[test]
    fn test_convergence_to_linear_at_knot() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 2.0, 4.0];

        // At knot point x = 1.0, should return y = 2.0
        let result = smooth_interp(&xs, &ys, 1.0, 1e-6).unwrap();
        assert!(
            (result - 2.0).abs() < 0.1,
            "At knot x=1.0, expected ~2.0, got {}",
            result
        );
    }

    #[test]
    fn test_linear_data() {
        // For linear data y = x, smooth_interp should produce linear output
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 1.0, 2.0, 3.0];

        for x in [0.25, 0.5, 1.0, 1.5, 2.0, 2.5] {
            let result = smooth_interp(&xs, &ys, x, 1e-6).unwrap();
            assert!(
                (result - x).abs() < 0.1,
                "At x={}, expected ~{}, got {}",
                x,
                x,
                result
            );
        }
    }

    #[test]
    fn test_different_epsilon_values() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0, 4.0];

        // All epsilon values should produce finite results
        for eps in [1e-3, 1e-4, 1e-5, 1e-6] {
            let result = smooth_interp(&xs, &ys, 0.5, eps).unwrap();
            assert!(
                result.is_finite(),
                "Result should be finite for eps={}",
                eps
            );
        }
    }

    #[test]
    fn test_boundary_behaviour() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 2.0, 4.0];

        // At boundaries
        let y_start = smooth_interp(&xs, &ys, 0.0, 1e-6).unwrap();
        let y_end = smooth_interp(&xs, &ys, 2.0, 1e-6).unwrap();

        assert!(y_start.is_finite());
        assert!(y_end.is_finite());
        // Should be close to boundary values
        assert!((y_start - 0.0).abs() < 0.5);
        assert!((y_end - 4.0).abs() < 0.5);
    }

    #[test]
    fn test_two_points() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 2.0];

        let result = smooth_interp(&xs, &ys, 0.5, 1e-6).unwrap();
        assert!(
            (result - 1.0).abs() < 0.2,
            "At x=0.5, expected ~1.0, got {}",
            result
        );
    }

    #[test]
    fn test_with_f32() {
        let xs: [f32; 3] = [0.0, 1.0, 2.0];
        let ys: [f32; 3] = [0.0, 1.0, 2.0];

        let result = smooth_interp(&xs, &ys, 0.5_f32, 1e-4_f32);
        assert!(result.is_ok());
        assert!(result.unwrap().is_finite());
    }
}
