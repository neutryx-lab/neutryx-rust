//! Two-dimensional mesh generation.
//!
//! Provides structures and functions for creating 2D grids.

use num_traits::Float;

/// A 2D rectangular grid.
///
/// Stores the grid as two vectors (x coordinates and y coordinates)
/// representing a tensor product grid.
#[derive(Debug, Clone)]
pub struct Grid2D<T> {
    /// X-coordinates (first dimension).
    pub x: Vec<T>,
    /// Y-coordinates (second dimension).
    pub y: Vec<T>,
}

impl<T: Float> Grid2D<T> {
    /// Create a new 2D grid from x and y vectors.
    ///
    /// # Arguments
    ///
    /// * `x` - X-coordinates
    /// * `y` - Y-coordinates
    #[must_use]
    pub fn new(x: Vec<T>, y: Vec<T>) -> Self { Self { x, y } }

    /// Get the number of x points.
    #[must_use]
    pub fn nx(&self) -> usize { self.x.len() }

    /// Get the number of y points.
    #[must_use]
    pub fn ny(&self) -> usize { self.y.len() }

    /// Get the total number of grid points.
    #[must_use]
    pub fn total_points(&self) -> usize { self.x.len() * self.y.len() }

    /// Get the coordinate at index (i, j).
    ///
    /// # Arguments
    ///
    /// * `i` - X index
    /// * `j` - Y index
    ///
    /// # Returns
    ///
    /// Tuple (x\[i\], y\[j\])
    #[must_use]
    pub fn point(&self, i: usize, j: usize) -> (T, T) { (self.x[i], self.y[j]) }

    /// Iterate over all grid points in row-major order.
    ///
    /// Returns an iterator over ((i, j), (x, y)) tuples.
    pub fn iter(&self) -> impl Iterator<Item = ((usize, usize), (T, T))> + '_ {
        (0..self.ny())
            .flat_map(move |j| (0..self.nx()).map(move |i| ((i, j), (self.x[i], self.y[j]))))
    }

    /// Get the x-spacing at index i (between i and i+1).
    ///
    /// Returns None if i >= nx - 1.
    #[must_use]
    pub fn dx(&self, i: usize) -> Option<T> {
        if i + 1 < self.x.len() {
            Some(self.x[i + 1] - self.x[i])
        } else {
            None
        }
    }

    /// Get the y-spacing at index j (between j and j+1).
    ///
    /// Returns None if j >= ny - 1.
    #[must_use]
    pub fn dy(&self, j: usize) -> Option<T> {
        if j + 1 < self.y.len() {
            Some(self.y[j + 1] - self.y[j])
        } else {
            None
        }
    }

    /// Get the domain bounds.
    ///
    /// Returns ((x_min, x_max), (y_min, y_max))
    #[must_use]
    pub fn bounds(&self) -> ((T, T), (T, T)) {
        let x_min = self.x.first().copied().unwrap_or_else(T::zero);
        let x_max = self.x.last().copied().unwrap_or_else(T::zero);
        let y_min = self.y.first().copied().unwrap_or_else(T::zero);
        let y_max = self.y.last().copied().unwrap_or_else(T::zero);
        ((x_min, x_max), (y_min, y_max))
    }
}

/// Create a 2D tensor product grid from two 1D grids.
///
/// # Arguments
///
/// * `x` - X-coordinates
/// * `y` - Y-coordinates
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::mesh::{uniform_grid, tensor_product_grid};
///
/// let x = uniform_grid(0.0, 1.0, 5);
/// let y = uniform_grid(0.0, 2.0, 3);
/// let grid = tensor_product_grid(x, y);
/// assert_eq!(grid.total_points(), 15);
/// ```
#[must_use]
pub fn tensor_product_grid<T: Float>(x: Vec<T>, y: Vec<T>) -> Grid2D<T> { Grid2D::new(x, y) }

/// Create a uniform 2D grid.
///
/// # Arguments
///
/// * `x_start`, `x_end` - X-range
/// * `y_start`, `y_end` - Y-range
/// * `nx` - Number of x points
/// * `ny` - Number of y points
#[must_use]
pub fn uniform_grid_2d<T: Float>(
    x_start: T,
    x_end: T,
    y_start: T,
    y_end: T,
    nx: usize,
    ny: usize,
) -> Grid2D<T> {
    let x = super::grid_1d::uniform_grid(x_start, x_end, nx);
    let y = super::grid_1d::uniform_grid(y_start, y_end, ny);
    Grid2D::new(x, y)
}

/// Flatten a 2D index to a 1D index (row-major order).
///
/// # Arguments
///
/// * `i` - X index
/// * `j` - Y index
/// * `nx` - Number of x points
///
/// # Returns
///
/// Linear index `j * nx + i`
#[must_use]
pub const fn flatten_index(i: usize, j: usize, nx: usize) -> usize { j * nx + i }

/// Unflatten a 1D index to 2D indices (row-major order).
///
/// # Arguments
///
/// * `idx` - Linear index
/// * `nx` - Number of x points
///
/// # Returns
///
/// Tuple (i, j)
#[must_use]
pub const fn unflatten_index(idx: usize, nx: usize) -> (usize, usize) { (idx % nx, idx / nx) }

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_grid2d_creation() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 0.5, 1.0, 1.5];
        let grid: Grid2D<f64> = Grid2D::new(x, y);

        assert_eq!(grid.nx(), 3);
        assert_eq!(grid.ny(), 4);
        assert_eq!(grid.total_points(), 12);
    }

    #[test]
    fn test_grid2d_point() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 0.5, 1.0];
        let grid: Grid2D<f64> = Grid2D::new(x, y);

        let (px, py) = grid.point(1, 2);
        assert_relative_eq!(px, 1.0, epsilon = 1e-10);
        assert_relative_eq!(py, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_grid2d_spacing() {
        let x = vec![0.0, 1.0, 3.0]; // non-uniform
        let y = vec![0.0, 0.5, 1.0];
        let grid: Grid2D<f64> = Grid2D::new(x, y);

        assert_relative_eq!(grid.dx(0).unwrap(), 1.0, epsilon = 1e-10);
        assert_relative_eq!(grid.dx(1).unwrap(), 2.0, epsilon = 1e-10);
        assert!(grid.dx(2).is_none());

        assert_relative_eq!(grid.dy(0).unwrap(), 0.5, epsilon = 1e-10);
        assert_relative_eq!(grid.dy(1).unwrap(), 0.5, epsilon = 1e-10);
        assert!(grid.dy(2).is_none());
    }

    #[test]
    fn test_grid2d_bounds() {
        let x = vec![1.0, 2.0, 5.0];
        let y = vec![-1.0, 0.0, 1.0];
        let grid: Grid2D<f64> = Grid2D::new(x, y);

        let ((x_min, x_max), (y_min, y_max)) = grid.bounds();
        assert_relative_eq!(x_min, 1.0, epsilon = 1e-10);
        assert_relative_eq!(x_max, 5.0, epsilon = 1e-10);
        assert_relative_eq!(y_min, -1.0, epsilon = 1e-10);
        assert_relative_eq!(y_max, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_grid2d_iter() {
        let x = vec![0.0, 1.0];
        let y = vec![0.0, 1.0, 2.0];
        let grid: Grid2D<f64> = Grid2D::new(x, y);

        let points: Vec<_> = grid.iter().collect();
        assert_eq!(points.len(), 6);

        // Check first and last points
        assert_eq!(points[0], ((0, 0), (0.0, 0.0)));
        assert_eq!(points[5], ((1, 2), (1.0, 2.0)));
    }

    #[test]
    fn test_tensor_product_grid() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 0.5];
        let grid = tensor_product_grid(x, y);

        assert_eq!(grid.nx(), 3);
        assert_eq!(grid.ny(), 2);
    }

    #[test]
    fn test_uniform_grid_2d() {
        let grid: Grid2D<f64> = uniform_grid_2d(0.0, 1.0, 0.0, 2.0, 3, 5);

        assert_eq!(grid.nx(), 3);
        assert_eq!(grid.ny(), 5);
        assert_relative_eq!(grid.x[0], 0.0, epsilon = 1e-10);
        assert_relative_eq!(grid.x[2], 1.0, epsilon = 1e-10);
        assert_relative_eq!(grid.y[0], 0.0, epsilon = 1e-10);
        assert_relative_eq!(grid.y[4], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_flatten_unflatten_index() {
        let nx = 5;
        for j in 0..4 {
            for i in 0..5 {
                let flat = flatten_index(i, j, nx);
                let (i2, j2) = unflatten_index(flat, nx);
                assert_eq!(i, i2);
                assert_eq!(j, j2);
            }
        }
    }

    #[test]
    fn test_grid2d_clone() {
        let grid: Grid2D<f64> = uniform_grid_2d(0.0, 1.0, 0.0, 1.0, 3, 3);
        let cloned = grid.clone();
        assert_eq!(grid.nx(), cloned.nx());
        assert_eq!(grid.ny(), cloned.ny());
        assert_eq!(grid.x, cloned.x);
        assert_eq!(grid.y, cloned.y);
    }
}
