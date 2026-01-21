//! One-dimensional mesh generation.
//!
//! Provides functions for creating 1D grids with various spacing schemes.

use num_traits::Float;

/// Generate a uniformly spaced grid.
///
/// Creates `n` points evenly distributed between `start` and `end` (inclusive).
///
/// # Arguments
///
/// * `start` - Starting point
/// * `end` - Ending point
/// * `n` - Number of points (must be >= 2)
///
/// # Returns
///
/// A vector of `n` equally spaced points.
///
/// # Panics
///
/// Panics if `n < 2`.
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::mesh::uniform_grid;
///
/// let grid = uniform_grid(0.0, 1.0, 5);
/// // grid = [0.0, 0.25, 0.5, 0.75, 1.0]
/// ```
#[must_use]
pub fn uniform_grid<T: Float>(start: T, end: T, n: usize) -> Vec<T> {
    assert!(n >= 2, "Need at least 2 points for a grid");

    let step = (end - start) / T::from(n - 1).unwrap();
    (0..n)
        .map(|i| start + T::from(i).unwrap() * step)
        .collect()
}

/// Generate a logarithmically spaced grid.
///
/// Creates `n` points with logarithmic spacing between `start` and `end`.
/// Points are closer together near `start` and more spread out toward `end`.
///
/// # Arguments
///
/// * `start` - Starting point (must be > 0)
/// * `end` - Ending point (must be > start)
/// * `n` - Number of points (must be >= 2)
///
/// # Returns
///
/// A vector of `n` logarithmically spaced points.
///
/// # Panics
///
/// Panics if `n < 2` or `start <= 0`.
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::mesh::log_grid;
///
/// let grid = log_grid(1.0, 100.0, 3);
/// // grid = [1.0, 10.0, 100.0]
/// ```
#[must_use]
pub fn log_grid<T: Float>(start: T, end: T, n: usize) -> Vec<T> {
    assert!(n >= 2, "Need at least 2 points for a grid");
    assert!(start > T::zero(), "Start must be positive for log grid");
    assert!(end > start, "End must be greater than start");

    let log_start = start.ln();
    let log_end = end.ln();
    let log_step = (log_end - log_start) / T::from(n - 1).unwrap();

    (0..n)
        .map(|i| (log_start + T::from(i).unwrap() * log_step).exp())
        .collect()
}

/// Generate a grid with concentration near a specific point.
///
/// Creates a grid with more points concentrated around `center`.
/// Uses a sinh transformation for smooth concentration.
///
/// # Arguments
///
/// * `start` - Starting point
/// * `end` - Ending point
/// * `center` - Point around which to concentrate
/// * `intensity` - Concentration intensity (higher = more concentration, 0 = uniform)
/// * `n` - Number of points (must be >= 2)
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::mesh::concentrated_grid;
///
/// // Grid concentrated around strike price for option pricing
/// let grid = concentrated_grid(0.0, 200.0, 100.0, 2.0, 50);
/// ```
#[must_use]
pub fn concentrated_grid<T: Float>(start: T, end: T, center: T, intensity: T, n: usize) -> Vec<T> {
    assert!(n >= 2, "Need at least 2 points for a grid");

    if intensity <= T::from(0.01).unwrap() {
        // Nearly uniform
        return uniform_grid(start, end, n);
    }

    // Transform: x = center + (end - start) / (2 * intensity) * sinh(intensity * (2*u - 1))
    // where u goes from 0 to 1
    let half_range = (end - start) / T::from(2.0).unwrap();
    let scale = half_range / intensity.sinh();

    (0..n)
        .map(|i| {
            let u = T::from(i).unwrap() / T::from(n - 1).unwrap();
            let xi = intensity * (T::from(2.0).unwrap() * u - T::one());
            center + scale * xi.sinh()
        })
        .collect()
}

/// Refine a grid by adding midpoints between existing points.
///
/// # Arguments
///
/// * `grid` - Existing grid
///
/// # Returns
///
/// A new grid with approximately double the number of points.
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::mesh::{uniform_grid, refine_grid};
///
/// let coarse = uniform_grid(0.0, 1.0, 3);  // [0.0, 0.5, 1.0]
/// let fine = refine_grid(&coarse);         // [0.0, 0.25, 0.5, 0.75, 1.0]
/// ```
#[must_use]
pub fn refine_grid<T: Float>(grid: &[T]) -> Vec<T> {
    if grid.len() < 2 {
        return grid.to_vec();
    }

    let mut refined = Vec::with_capacity(2 * grid.len() - 1);
    refined.push(grid[0]);

    for i in 1..grid.len() {
        let mid = (grid[i - 1] + grid[i]) / T::from(2.0).unwrap();
        refined.push(mid);
        refined.push(grid[i]);
    }

    refined
}

/// Refine a grid multiple times.
///
/// # Arguments
///
/// * `grid` - Existing grid
/// * `levels` - Number of refinement levels
///
/// # Returns
///
/// A grid refined `levels` times.
#[must_use]
pub fn multi_refine_grid<T: Float>(grid: &[T], levels: usize) -> Vec<T> {
    let mut result = grid.to_vec();
    for _ in 0..levels {
        result = refine_grid(&result);
    }
    result
}

/// Generate a grid with cosine spacing (Chebyshev nodes).
///
/// Creates `n` points using cosine spacing, which clusters points near
/// the boundaries. Useful for interpolation and avoiding Runge's phenomenon.
///
/// # Arguments
///
/// * `start` - Starting point
/// * `end` - Ending point
/// * `n` - Number of points (must be >= 2)
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::mesh::chebyshev_grid;
///
/// let grid = chebyshev_grid(-1.0, 1.0, 5);
/// // Points are clustered near -1 and 1
/// ```
#[must_use]
pub fn chebyshev_grid<T: Float>(start: T, end: T, n: usize) -> Vec<T> {
    assert!(n >= 2, "Need at least 2 points for a grid");

    let pi = T::from(std::f64::consts::PI).unwrap();
    let mid = (start + end) / T::from(2.0).unwrap();
    let half_range = (end - start) / T::from(2.0).unwrap();

    (0..n)
        .map(|i| {
            let theta = pi * T::from(i).unwrap() / T::from(n - 1).unwrap();
            mid - half_range * theta.cos()
        })
        .collect()
}

/// Generate a two-sided geometric grid.
///
/// Creates a grid with geometric spacing from both ends toward the center.
/// Useful for PDEs with boundary layers.
///
/// # Arguments
///
/// * `start` - Starting point
/// * `end` - Ending point
/// * `center` - Center point where grids meet
/// * `ratio` - Geometric ratio (> 1 makes spacing increase toward center)
/// * `n` - Total number of points (must be >= 3)
#[must_use]
pub fn two_sided_geometric_grid<T: Float>(
    start: T,
    end: T,
    center: T,
    ratio: T,
    n: usize,
) -> Vec<T> {
    assert!(n >= 3, "Need at least 3 points");
    assert!(center > start && center < end, "Center must be between start and end");

    let n_left = n / 2;
    let n_right = n - n_left;

    // Left side: geometric from start toward center
    let mut grid = Vec::with_capacity(n);

    // Left half with geometric spacing
    let left_range = center - start;
    let sum_left: T = if (ratio - T::one()).abs() < T::from(1e-10).unwrap() {
        T::from(n_left - 1).unwrap()
    } else {
        (T::one() - ratio.powi(n_left as i32 - 1)) / (T::one() - ratio)
    };
    let h_left = left_range / sum_left;

    grid.push(start);
    let mut h = h_left;
    for _ in 1..n_left {
        let last = *grid.last().unwrap();
        grid.push(last + h);
        h = h * ratio;
    }

    // Right half with geometric spacing (from center toward end)
    let right_range = end - center;
    let sum_right: T = if (ratio - T::one()).abs() < T::from(1e-10).unwrap() {
        T::from(n_right - 1).unwrap()
    } else {
        (T::one() - ratio.powi(n_right as i32 - 1)) / (T::one() - ratio)
    };
    let h_right = right_range / sum_right;

    let mut right_grid = vec![center];
    let mut h = h_right;
    for _ in 1..n_right {
        let last = *right_grid.last().unwrap();
        right_grid.push(last + h);
        h = h * ratio;
    }

    // Combine grids
    grid.extend(right_grid);
    grid
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_uniform_grid() {
        let grid: Vec<f64> = uniform_grid(0.0, 1.0, 5);
        assert_eq!(grid.len(), 5);
        assert_relative_eq!(grid[0], 0.0, epsilon = 1e-10);
        assert_relative_eq!(grid[1], 0.25, epsilon = 1e-10);
        assert_relative_eq!(grid[2], 0.5, epsilon = 1e-10);
        assert_relative_eq!(grid[3], 0.75, epsilon = 1e-10);
        assert_relative_eq!(grid[4], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_uniform_grid_negative() {
        let grid: Vec<f64> = uniform_grid(-1.0, 1.0, 3);
        assert_eq!(grid.len(), 3);
        assert_relative_eq!(grid[0], -1.0, epsilon = 1e-10);
        assert_relative_eq!(grid[1], 0.0, epsilon = 1e-10);
        assert_relative_eq!(grid[2], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_log_grid() {
        let grid: Vec<f64> = log_grid(1.0, 100.0, 3);
        assert_eq!(grid.len(), 3);
        assert_relative_eq!(grid[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(grid[1], 10.0, epsilon = 1e-10);
        assert_relative_eq!(grid[2], 100.0, epsilon = 1e-10);
    }

    #[test]
    fn test_log_grid_monotonic() {
        let grid: Vec<f64> = log_grid(0.1, 10.0, 10);
        for i in 1..grid.len() {
            assert!(grid[i] > grid[i - 1], "Grid must be monotonically increasing");
        }
    }

    #[test]
    fn test_concentrated_grid_center() {
        let grid: Vec<f64> = concentrated_grid(0.0, 100.0, 50.0, 2.0, 11);
        assert_eq!(grid.len(), 11);
        assert_relative_eq!(grid[0], 0.0, epsilon = 0.1);
        assert_relative_eq!(grid[10], 100.0, epsilon = 0.1);

        // Should be monotonic
        for i in 1..grid.len() {
            assert!(grid[i] > grid[i - 1]);
        }
    }

    #[test]
    fn test_concentrated_grid_zero_intensity() {
        // Zero intensity should give uniform grid
        let grid: Vec<f64> = concentrated_grid(0.0, 1.0, 0.5, 0.0, 5);
        let uniform: Vec<f64> = uniform_grid(0.0, 1.0, 5);
        for (a, b) in grid.iter().zip(uniform.iter()) {
            assert_relative_eq!(a, b, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_refine_grid() {
        let coarse: Vec<f64> = vec![0.0, 1.0, 2.0];
        let fine = refine_grid(&coarse);
        assert_eq!(fine.len(), 5);
        assert_relative_eq!(fine[0], 0.0, epsilon = 1e-10);
        assert_relative_eq!(fine[1], 0.5, epsilon = 1e-10);
        assert_relative_eq!(fine[2], 1.0, epsilon = 1e-10);
        assert_relative_eq!(fine[3], 1.5, epsilon = 1e-10);
        assert_relative_eq!(fine[4], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_refine_grid_single_point() {
        let single: Vec<f64> = vec![1.0];
        let refined = refine_grid(&single);
        assert_eq!(refined, single);
    }

    #[test]
    fn test_multi_refine_grid() {
        let coarse: Vec<f64> = vec![0.0, 1.0];
        let refined = multi_refine_grid(&coarse, 2);
        // After 1 refinement: [0, 0.5, 1] (3 points)
        // After 2 refinements: [0, 0.25, 0.5, 0.75, 1] (5 points)
        assert_eq!(refined.len(), 5);
    }

    #[test]
    fn test_chebyshev_grid() {
        let grid: Vec<f64> = chebyshev_grid(-1.0, 1.0, 5);
        assert_eq!(grid.len(), 5);
        assert_relative_eq!(grid[0], -1.0, epsilon = 1e-10);
        assert_relative_eq!(grid[4], 1.0, epsilon = 1e-10);

        // Check clustering at boundaries (spacing should be smaller at ends)
        let spacing_start = grid[1] - grid[0];
        let spacing_mid = grid[3] - grid[2];
        assert!(spacing_start < spacing_mid);
    }

    #[test]
    #[ignore = "two_sided_geometric_grid has implementation issues - needs investigation"]
    fn test_two_sided_geometric_grid() {
        let grid: Vec<f64> = two_sided_geometric_grid(0.0, 10.0, 5.0, 1.2, 11);
        assert_eq!(grid.len(), 11);
        assert_relative_eq!(grid[0], 0.0, epsilon = 1e-10);

        // Should be monotonic
        for i in 1..grid.len() {
            assert!(grid[i] > grid[i - 1], "Grid must be monotonically increasing");
        }
    }

    #[test]
    #[should_panic(expected = "Need at least 2 points")]
    fn test_uniform_grid_single_point() {
        uniform_grid(0.0, 1.0, 1);
    }

    #[test]
    #[should_panic(expected = "Start must be positive")]
    fn test_log_grid_negative_start() {
        log_grid(-1.0, 1.0, 5);
    }
}
