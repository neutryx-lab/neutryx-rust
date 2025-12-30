//! Bilinear 2D interpolation for surfaces.

use crate::types::InterpolationError;
use num_traits::Float;

/// Bilinear interpolator for 2D grid data.
///
/// Stores a 2D grid of values z(x, y) and performs bilinear interpolation
/// to compute values at arbitrary (x, y) coordinates within the grid.
/// Ideal for volatility surfaces and correlation matrices.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Grid Layout
///
/// The grid is stored as `zs[i][j] = z(xs[i], ys[j])` where:
/// - `xs` defines the x-axis coordinates (rows)
/// - `ys` defines the y-axis coordinates (columns)
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::BilinearInterpolator;
///
/// let xs = [0.0, 1.0, 2.0];
/// let ys = [0.0, 1.0];
/// let zs = [
///     &[0.0, 1.0][..],
///     &[2.0, 3.0][..],
///     &[4.0, 5.0][..],
/// ];
///
/// let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();
/// let z = interp.interpolate(0.5, 0.5).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct BilinearInterpolator<T: Float> {
    /// X-axis coordinates
    xs: Vec<T>,
    /// Y-axis coordinates
    ys: Vec<T>,
    /// Grid values: zs[i][j] = z(xs[i], ys[j])
    zs: Vec<Vec<T>>,
}

impl<T: Float> BilinearInterpolator<T> {
    /// Construct a bilinear interpolator from grid data.
    ///
    /// # Arguments
    ///
    /// * `xs` - Slice of x-axis coordinates (must be sorted, length >= 2)
    /// * `ys` - Slice of y-axis coordinates (must be sorted, length >= 2)
    /// * `zs` - Slice of slices representing the grid values
    ///
    /// # Returns
    ///
    /// * `Ok(BilinearInterpolator)` - Successfully constructed interpolator
    /// * `Err(InterpolationError::InsufficientData)` - Fewer than 2 points on an axis
    /// * `Err(InterpolationError::InvalidInput)` - Grid dimensions don't match axis lengths
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::interpolators::BilinearInterpolator;
    ///
    /// let xs = [0.0, 1.0];
    /// let ys = [0.0, 1.0];
    /// let zs = [&[0.0, 1.0][..], &[2.0, 3.0][..]];
    ///
    /// let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();
    /// ```
    pub fn new(xs: &[T], ys: &[T], zs: &[&[T]]) -> Result<Self, InterpolationError> {
        // Validate minimum axis lengths
        if xs.len() < 2 {
            return Err(InterpolationError::InsufficientData {
                got: xs.len(),
                need: 2,
            });
        }
        if ys.len() < 2 {
            return Err(InterpolationError::InsufficientData {
                got: ys.len(),
                need: 2,
            });
        }

        // Validate grid dimensions
        if zs.len() != xs.len() {
            return Err(InterpolationError::InvalidInput(format!(
                "Grid rows ({}) must match x-axis length ({})",
                zs.len(),
                xs.len()
            )));
        }

        for (i, row) in zs.iter().enumerate() {
            if row.len() != ys.len() {
                return Err(InterpolationError::InvalidInput(format!(
                    "Grid row {} length ({}) must match y-axis length ({})",
                    i,
                    row.len(),
                    ys.len()
                )));
            }
        }

        // Copy data
        let xs_vec: Vec<T> = xs.to_vec();
        let ys_vec: Vec<T> = ys.to_vec();
        let zs_vec: Vec<Vec<T>> = zs.iter().map(|row| row.to_vec()).collect();

        Ok(Self {
            xs: xs_vec,
            ys: ys_vec,
            zs: zs_vec,
        })
    }

    /// Interpolate value at point (x, y) using bilinear interpolation.
    ///
    /// # Formula
    ///
    /// ```text
    /// z = (1-u)(1-v)*z00 + u*(1-v)*z10 + (1-u)*v*z01 + u*v*z11
    /// ```
    ///
    /// where `u` and `v` are the normalised coordinates within the grid cell.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    ///
    /// # Returns
    ///
    /// * `Ok(z)` - The interpolated value
    /// * `Err(InterpolationError::OutOfBounds)` - If (x, y) is outside the grid
    pub fn interpolate(&self, x: T, y: T) -> Result<T, InterpolationError> {
        let (x_min, x_max) = self.domain_x();
        let (y_min, y_max) = self.domain_y();

        // Check x bounds
        if x < x_min || x > x_max {
            return Err(InterpolationError::OutOfBounds {
                x: x.to_f64().unwrap_or(f64::NAN),
                min: x_min.to_f64().unwrap_or(f64::NAN),
                max: x_max.to_f64().unwrap_or(f64::NAN),
            });
        }

        // Check y bounds
        if y < y_min || y > y_max {
            return Err(InterpolationError::OutOfBounds {
                x: y.to_f64().unwrap_or(f64::NAN),
                min: y_min.to_f64().unwrap_or(f64::NAN),
                max: y_max.to_f64().unwrap_or(f64::NAN),
            });
        }

        // Find grid cell indices
        let i = self.find_x_index(x);
        let j = self.find_y_index(y);

        // Get cell corner coordinates
        let x0 = self.xs[i];
        let x1 = self.xs[i + 1];
        let y0 = self.ys[j];
        let y1 = self.ys[j + 1];

        // Get cell corner values
        let z00 = self.zs[i][j];
        let z10 = self.zs[i + 1][j];
        let z01 = self.zs[i][j + 1];
        let z11 = self.zs[i + 1][j + 1];

        // Compute normalised coordinates
        let u = (x - x0) / (x1 - x0);
        let v = (y - y0) / (y1 - y0);

        // Bilinear interpolation
        let one = T::one();
        let z =
            (one - u) * (one - v) * z00 + u * (one - v) * z10 + (one - u) * v * z01 + u * v * z11;

        Ok(z)
    }

    /// Return the valid interpolation domain for x.
    #[inline]
    pub fn domain_x(&self) -> (T, T) {
        (self.xs[0], self.xs[self.xs.len() - 1])
    }

    /// Return the valid interpolation domain for y.
    #[inline]
    pub fn domain_y(&self) -> (T, T) {
        (self.ys[0], self.ys[self.ys.len() - 1])
    }

    /// Find the x-axis grid index using binary search.
    #[inline]
    fn find_x_index(&self, x: T) -> usize {
        let pos = self.xs.partition_point(|&xi| xi <= x);
        if pos == 0 {
            0
        } else if pos >= self.xs.len() {
            self.xs.len() - 2
        } else {
            pos - 1
        }
    }

    /// Find the y-axis grid index using binary search.
    #[inline]
    fn find_y_index(&self, y: T) -> usize {
        let pos = self.ys.partition_point(|&yi| yi <= y);
        if pos == 0 {
            0
        } else if pos >= self.ys.len() {
            self.ys.len() - 2
        } else {
            pos - 1
        }
    }

    /// Returns a reference to the x-axis coordinates.
    #[inline]
    pub fn xs(&self) -> &[T] {
        &self.xs
    }

    /// Returns a reference to the y-axis coordinates.
    #[inline]
    pub fn ys(&self) -> &[T] {
        &self.ys
    }

    /// Returns a reference to the grid values.
    #[inline]
    pub fn zs(&self) -> &[Vec<T>] {
        &self.zs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Construction Tests (Task 7.1)
    // ========================================

    #[test]
    fn test_new_minimum_grid() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let zs = [&[0.0, 1.0][..], &[2.0, 3.0][..]];

        let result = BilinearInterpolator::new(&xs, &ys, &zs);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_larger_grid() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0, 2.0];
        let zs = [
            &[0.0, 1.0, 2.0][..],
            &[3.0, 4.0, 5.0][..],
            &[6.0, 7.0, 8.0][..],
        ];

        let result = BilinearInterpolator::new(&xs, &ys, &zs);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_insufficient_x_axis() {
        let xs = [0.0];
        let ys = [0.0, 1.0];
        let zs = [&[0.0, 1.0][..]];

        let result = BilinearInterpolator::new(&xs, &ys, &zs);
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
    fn test_new_insufficient_y_axis() {
        let xs = [0.0, 1.0];
        let ys = [0.0];
        let zs = [&[0.0][..], &[1.0][..]];

        let result = BilinearInterpolator::new(&xs, &ys, &zs);
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
    fn test_new_grid_rows_mismatch() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0];
        let zs = [&[0.0, 1.0][..], &[2.0, 3.0][..]]; // Only 2 rows, need 3

        let result = BilinearInterpolator::new(&xs, &ys, &zs);
        assert!(result.is_err());

        match result.unwrap_err() {
            InterpolationError::InvalidInput(msg) => {
                assert!(msg.contains("rows"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_new_grid_cols_mismatch() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0, 2.0];
        let zs = [&[0.0, 1.0][..], &[2.0, 3.0][..]]; // Rows have 2 cols, need 3

        let result = BilinearInterpolator::new(&xs, &ys, &zs);
        assert!(result.is_err());

        match result.unwrap_err() {
            InterpolationError::InvalidInput(msg) => {
                assert!(msg.contains("row"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    // ========================================
    // Domain Tests (Task 7.1)
    // ========================================

    #[test]
    fn test_domain_x() {
        let xs = [1.0, 2.0, 3.0];
        let ys = [0.0, 1.0];
        let zs = [&[0.0, 1.0][..], &[2.0, 3.0][..], &[4.0, 5.0][..]];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();
        assert_eq!(interp.domain_x(), (1.0, 3.0));
    }

    #[test]
    fn test_domain_y() {
        let xs = [0.0, 1.0];
        let ys = [2.0, 3.0, 4.0];
        let zs = [&[0.0, 1.0, 2.0][..], &[3.0, 4.0, 5.0][..]];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();
        assert_eq!(interp.domain_y(), (2.0, 4.0));
    }

    // ========================================
    // Interpolation Tests (Task 7.2)
    // ========================================

    #[test]
    fn test_interpolate_at_corners() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let zs = [&[1.0, 2.0][..], &[3.0, 4.0][..]];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();

        // Test at all four corners
        assert!((interp.interpolate(0.0, 0.0).unwrap() - 1.0).abs() < 1e-10);
        assert!((interp.interpolate(1.0, 0.0).unwrap() - 3.0).abs() < 1e-10);
        assert!((interp.interpolate(0.0, 1.0).unwrap() - 2.0).abs() < 1e-10);
        assert!((interp.interpolate(1.0, 1.0).unwrap() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_at_center() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let zs = [&[0.0, 2.0][..], &[2.0, 4.0][..]];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();

        // At center (0.5, 0.5):
        // z = (1-0.5)(1-0.5)*0 + 0.5*(1-0.5)*2 + (1-0.5)*0.5*2 + 0.5*0.5*4
        //   = 0.25*0 + 0.25*2 + 0.25*2 + 0.25*4 = 0 + 0.5 + 0.5 + 1.0 = 2.0
        let z = interp.interpolate(0.5, 0.5).unwrap();
        assert!((z - 2.0).abs() < 1e-10, "Expected 2.0, got {}", z);
    }

    #[test]
    fn test_interpolate_along_edge() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let zs = [&[0.0, 2.0][..], &[4.0, 6.0][..]];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();

        // Along x=0 edge: linear interpolation between z(0,0)=0 and z(0,1)=2
        let z = interp.interpolate(0.0, 0.5).unwrap();
        assert!((z - 1.0).abs() < 1e-10);

        // Along y=0 edge: linear interpolation between z(0,0)=0 and z(1,0)=4
        let z = interp.interpolate(0.5, 0.0).unwrap();
        assert!((z - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_out_of_bounds_x_low() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let zs = [&[0.0, 1.0][..], &[2.0, 3.0][..]];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();
        let result = interp.interpolate(-0.1, 0.5);

        assert!(result.is_err());
        match result.unwrap_err() {
            InterpolationError::OutOfBounds { .. } => {}
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_interpolate_out_of_bounds_x_high() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let zs = [&[0.0, 1.0][..], &[2.0, 3.0][..]];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();
        let result = interp.interpolate(1.1, 0.5);

        assert!(result.is_err());
        match result.unwrap_err() {
            InterpolationError::OutOfBounds { .. } => {}
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_interpolate_out_of_bounds_y_low() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let zs = [&[0.0, 1.0][..], &[2.0, 3.0][..]];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();
        let result = interp.interpolate(0.5, -0.1);

        assert!(result.is_err());
        match result.unwrap_err() {
            InterpolationError::OutOfBounds { .. } => {}
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_interpolate_out_of_bounds_y_high() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let zs = [&[0.0, 1.0][..], &[2.0, 3.0][..]];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();
        let result = interp.interpolate(0.5, 1.1);

        assert!(result.is_err());
        match result.unwrap_err() {
            InterpolationError::OutOfBounds { .. } => {}
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    // ========================================
    // Multi-cell Grid Tests (Task 7.3)
    // ========================================

    #[test]
    fn test_interpolate_multi_cell_grid() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0, 2.0];
        // Grid where z = x + y
        let zs = [
            &[0.0, 1.0, 2.0][..], // x=0: y=0,1,2
            &[1.0, 2.0, 3.0][..], // x=1: y=0,1,2
            &[2.0, 3.0, 4.0][..], // x=2: y=0,1,2
        ];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();

        // For z = x + y surface, interpolation should give exact values
        for (x, y) in [(0.5, 0.5), (1.5, 0.5), (0.5, 1.5), (1.5, 1.5)] {
            let z = interp.interpolate(x, y).unwrap();
            let expected = x + y;
            assert!(
                (z - expected).abs() < 1e-10,
                "At ({}, {}), expected {}, got {}",
                x,
                y,
                expected,
                z
            );
        }
    }

    #[test]
    fn test_clone() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let zs = [&[0.0, 1.0][..], &[2.0, 3.0][..]];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();
        let cloned = interp.clone();

        assert_eq!(interp.xs(), cloned.xs());
        assert_eq!(interp.ys(), cloned.ys());
    }

    #[test]
    fn test_with_f32() {
        let xs: [f32; 2] = [0.0, 1.0];
        let ys: [f32; 2] = [0.0, 1.0];
        let row0: [f32; 2] = [0.0, 1.0];
        let row1: [f32; 2] = [2.0, 3.0];
        let zs: [&[f32]; 2] = [&row0[..], &row1[..]];

        let result = BilinearInterpolator::new(&xs, &ys, &zs);
        assert!(result.is_ok());

        let interp = result.unwrap();
        let z = interp.interpolate(0.5_f32, 0.5_f32).unwrap();
        assert!(z.is_finite());
    }
}
