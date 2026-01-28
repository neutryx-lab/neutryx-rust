//! Calibration matrices for curve and surface calibration.
//!
//! This module provides matrix-based representations for efficient
//! Jacobian computation in calibration algorithms.
//!
//! # Components
//!
//! - `CalibrationMatrix<T>`: General-purpose calibration matrix (N × M)
//! - `InterpolationMatrix<T>`: Maps pillar values to grid points via
//!   interpolation

use num_traits::Float;
use pricer_core::math::{
    linalg::{DMatrix, RealField},
    numeric::from_f64,
};

use super::grid::CalibrationGrid;

// =============================================================================
// CalibrationMatrix
// =============================================================================

/// Calibration matrix for multi-instrument calibration.
///
/// An N×M matrix where:
/// - N = number of instruments/quotes
/// - M = number of grid points (dates, strikes, etc.)
///
/// # Type Parameters
///
/// * `T` - Floating-point type for matrix elements
///
/// # Example
///
/// ```ignore
/// use pricer_models::builder::CalibrationMatrix;
///
/// // 3 instruments, 5 grid points
/// let mut matrix: CalibrationMatrix<f64> = CalibrationMatrix::zeros(3, 5);
/// matrix.set(0, 1, 100.0); // Instrument 0 has value 100 at grid point 1
/// ```
#[derive(Debug, Clone)]
pub struct CalibrationMatrix<T: Float + RealField + Copy> {
    /// The underlying matrix (N instruments × M grid points).
    matrix: DMatrix<T>,
    /// Number of rows (instruments).
    num_rows: usize,
    /// Number of columns (grid points).
    num_cols: usize,
}

impl<T: Float + RealField + Copy> CalibrationMatrix<T> {
    /// Create a new calibration matrix from a DMatrix.
    pub fn from_matrix(matrix: DMatrix<T>, num_rows: usize, num_cols: usize) -> Self {
        Self {
            matrix,
            num_rows,
            num_cols,
        }
    }

    /// Create a zero calibration matrix.
    pub fn zeros(num_rows: usize, num_cols: usize) -> Self {
        Self {
            matrix: DMatrix::zeros(num_rows, num_cols),
            num_rows,
            num_cols,
        }
    }

    /// Get the underlying matrix.
    pub fn matrix(&self) -> &DMatrix<T> { &self.matrix }

    /// Get a mutable reference to the underlying matrix.
    pub fn matrix_mut(&mut self) -> &mut DMatrix<T> { &mut self.matrix }

    /// Set a value at (row, col).
    pub fn set(&mut self, row: usize, col: usize, value: T) {
        if row < self.num_rows && col < self.num_cols {
            self.matrix[(row, col)] = value;
        }
    }

    /// Get a value at (row, col).
    pub fn get(&self, row: usize, col: usize) -> Option<T> {
        if row < self.num_rows && col < self.num_cols {
            Some(self.matrix[(row, col)])
        } else {
            None
        }
    }

    /// Get the number of rows.
    pub fn num_rows(&self) -> usize { self.num_rows }

    /// Get the number of columns.
    pub fn num_cols(&self) -> usize { self.num_cols }

    /// Check if a specific value is non-zero.
    pub fn is_nonzero(&self, row: usize, col: usize) -> bool {
        self.get(row, col)
            .map(|v| Float::abs(v) > from_f64(1e-15))
            .unwrap_or(false)
    }

    /// Get all values in a row.
    pub fn get_row(&self, row: usize) -> Option<Vec<T>> {
        if row < self.num_rows {
            Some((0..self.num_cols).map(|j| self.matrix[(row, j)]).collect())
        } else {
            None
        }
    }

    /// Get all values in a column.
    pub fn get_col(&self, col: usize) -> Option<Vec<T>> {
        if col < self.num_cols {
            Some((0..self.num_rows).map(|i| self.matrix[(i, col)]).collect())
        } else {
            None
        }
    }
}

// =============================================================================
// Convenience Methods for Curve Calibration
// =============================================================================

impl<T: Float + RealField + Copy> CalibrationMatrix<T> {
    /// Set a cashflow value (convenience method for curve calibration).
    pub fn set_cashflow(&mut self, instrument_idx: usize, date_idx: usize, value: T) {
        self.set(instrument_idx, date_idx, value);
    }

    /// Get a cashflow value (convenience method for curve calibration).
    pub fn get_cashflow(&self, instrument_idx: usize, date_idx: usize) -> Option<T> {
        self.get(instrument_idx, date_idx)
    }

    /// Get the number of instruments (alias for `num_rows()`).
    pub fn num_instruments(&self) -> usize { self.num_rows }

    /// Get the number of dates (alias for `num_cols()`).
    pub fn num_dates(&self) -> usize { self.num_cols }

    /// Check if a cashflow exists (alias for `is_nonzero()`).
    pub fn has_cashflow(&self, instrument_idx: usize, date_idx: usize) -> bool {
        self.is_nonzero(instrument_idx, date_idx)
    }

    /// Get all cashflows for an instrument (alias for `get_row()`).
    pub fn get_instrument_cashflows(&self, instrument_idx: usize) -> Option<Vec<T>> {
        self.get_row(instrument_idx)
    }

    /// Get all cashflows at a date (alias for `get_col()`).
    pub fn get_date_cashflows(&self, date_idx: usize) -> Option<Vec<T>> { self.get_col(date_idx) }
}

// =============================================================================
// InterpolationMatrix
// =============================================================================

/// Interpolation matrix for mapping pillar values to grid points.
///
/// An M×P matrix W where:
/// - M = number of grid points
/// - P = number of pillars
/// - W[j,k] = interpolation weight for pillar k contributing to grid point j
///
/// For log-linear interpolation:
/// - log(DF(t)) = (1-w) * log(DF(t_k)) + w * log(DF(t_{k+1}))
/// - where w = (t - t_k) / (t_{k+1} - t_k)
#[derive(Debug, Clone)]
pub struct InterpolationMatrix<T: Float + RealField + Copy> {
    /// The underlying matrix (M grid points × P pillars).
    matrix: DMatrix<T>,
    /// Number of grid points.
    num_points: usize,
    /// Number of pillars.
    num_pillars: usize,
}

impl<T: Float + RealField + Copy> InterpolationMatrix<T> {
    /// Create an interpolation matrix from pillar positions and a grid.
    ///
    /// Uses linear interpolation.
    ///
    /// # Arguments
    ///
    /// * `pillars` - Pillar positions (sorted)
    /// * `grid` - Calibration grid with all points
    ///
    /// # Returns
    ///
    /// An interpolation matrix W where W[j,k] is the weight of pillar k for
    /// point j.
    pub fn from_pillars(pillars: &[T], grid: &CalibrationGrid<T>) -> Self {
        let points = grid.points();
        let num_points = points.len();
        let num_pillars = pillars.len();

        if num_pillars == 0 || num_points == 0 {
            return Self {
                matrix: DMatrix::zeros(num_points, num_pillars),
                num_points,
                num_pillars,
            };
        }

        let mut matrix = DMatrix::zeros(num_points, num_pillars);

        for (j, &p) in points.iter().enumerate() {
            let (lower_idx, upper_idx, weight) = Self::find_pillar_interval(p, pillars);

            if lower_idx == upper_idx {
                matrix[(j, lower_idx)] = T::one();
            } else {
                matrix[(j, lower_idx)] = T::one() - weight;
                matrix[(j, upper_idx)] = weight;
            }
        }

        Self {
            matrix,
            num_points,
            num_pillars,
        }
    }

    /// Find the pillar interval containing a point.
    ///
    /// Returns (lower_index, upper_index, interpolation_weight).
    fn find_pillar_interval(p: T, pillars: &[T]) -> (usize, usize, T) {
        if pillars.is_empty() {
            return (0, 0, T::zero());
        }

        let pos = pillars.partition_point(|&pil| pil < p);

        if pos == 0 {
            (0, 0, T::zero())
        } else if pos >= pillars.len() {
            let last = pillars.len() - 1;
            (last, last, T::one())
        } else {
            let lower = pos - 1;
            let upper = pos;
            let p_lower = pillars[lower];
            let p_upper = pillars[upper];
            let weight = (p - p_lower) / (p_upper - p_lower);
            (lower, upper, weight)
        }
    }

    /// Get the underlying matrix.
    pub fn matrix(&self) -> &DMatrix<T> { &self.matrix }

    /// Get the number of grid points.
    pub fn num_points(&self) -> usize { self.num_points }

    /// Alias for `num_points()` - backward compatibility.
    pub fn num_dates(&self) -> usize { self.num_points }

    /// Get the number of pillars.
    pub fn num_pillars(&self) -> usize { self.num_pillars }

    /// Interpolate values from pillar values.
    ///
    /// # Arguments
    ///
    /// * `pillar_values` - Values at each pillar
    ///
    /// # Returns
    ///
    /// Interpolated values at each grid point.
    pub fn interpolate(&self, pillar_values: &[T]) -> Vec<T> {
        let mut result = Vec::with_capacity(self.num_points);

        for j in 0..self.num_points {
            let mut value = T::zero();
            for k in 0..self.num_pillars {
                value = value + self.matrix[(j, k)] * pillar_values[k];
            }
            result.push(value);
        }

        result
    }

    /// Alias for `interpolate()` - backward compatibility with log DF.
    pub fn interpolate_log_df(&self, log_df_pillars: &[T]) -> Vec<T> {
        self.interpolate(log_df_pillars)
    }

    /// Interpolate and exponentiate (for discount factors).
    ///
    /// # Arguments
    ///
    /// * `log_df_pillars` - log(DF) at each pillar
    ///
    /// # Returns
    ///
    /// DF at each grid point.
    pub fn interpolate_df(&self, log_df_pillars: &[T]) -> Vec<T> {
        self.interpolate_log_df(log_df_pillars)
            .into_iter()
            .map(Float::exp)
            .collect()
    }
}

// =============================================================================
// CalibrationMatrixBuilder
// =============================================================================

/// Builder for constructing calibration matrices.
#[derive(Debug, Clone)]
pub struct CalibrationMatrixBuilder<T: Float> {
    /// Tolerance for point matching.
    tolerance: T,
}

impl<T: Float> CalibrationMatrixBuilder<T> {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            tolerance: T::from(1e-10).unwrap(),
        }
    }

    /// Set the point matching tolerance.
    pub fn with_tolerance(mut self, tolerance: T) -> Self {
        self.tolerance = tolerance;
        self
    }
}

impl<T: Float> Default for CalibrationMatrixBuilder<T> {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_calibration_matrix_zeros() {
        let matrix: CalibrationMatrix<f64> = CalibrationMatrix::zeros(3, 5);

        assert_eq!(matrix.num_rows(), 3);
        assert_eq!(matrix.num_cols(), 5);
        assert_relative_eq!(matrix.get(0, 0).unwrap(), 0.0, epsilon = 1e-15);
    }

    #[test]
    fn test_calibration_matrix_set_get() {
        let mut matrix: CalibrationMatrix<f64> = CalibrationMatrix::zeros(2, 3);

        matrix.set(0, 1, 100.0);
        matrix.set(1, 2, -50.0);

        assert_relative_eq!(matrix.get(0, 1).unwrap(), 100.0, epsilon = 1e-10);
        assert_relative_eq!(matrix.get(1, 2).unwrap(), -50.0, epsilon = 1e-10);
        assert_relative_eq!(matrix.get(0, 0).unwrap(), 0.0, epsilon = 1e-15);
    }

    #[test]
    fn test_calibration_matrix_is_nonzero() {
        let mut matrix: CalibrationMatrix<f64> = CalibrationMatrix::zeros(2, 2);

        matrix.set(0, 0, 1.0);
        matrix.set(1, 1, 0.0);

        assert!(matrix.is_nonzero(0, 0));
        assert!(!matrix.is_nonzero(1, 1));
        assert!(!matrix.is_nonzero(0, 1));
    }

    #[test]
    fn test_calibration_matrix_get_row() {
        let mut matrix: CalibrationMatrix<f64> = CalibrationMatrix::zeros(2, 3);

        matrix.set(0, 0, 1.0);
        matrix.set(0, 1, 2.0);
        matrix.set(0, 2, 3.0);

        let row = matrix.get_row(0).unwrap();
        assert_eq!(row.len(), 3);
        assert_relative_eq!(row[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(row[1], 2.0, epsilon = 1e-10);
        assert_relative_eq!(row[2], 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_interpolation_matrix_single_pillar() {
        let pillars = vec![1.0];
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![0.5, 1.0, 1.5]);

        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        assert_eq!(interp.num_points(), 3);
        assert_eq!(interp.num_pillars(), 1);

        assert_relative_eq!(interp.matrix()[(0, 0)], 1.0, epsilon = 1e-10);
        assert_relative_eq!(interp.matrix()[(1, 0)], 1.0, epsilon = 1e-10);
        assert_relative_eq!(interp.matrix()[(2, 0)], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_interpolation_matrix_two_pillars() {
        let pillars = vec![1.0, 2.0];
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 1.5, 2.0]);

        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        assert_eq!(interp.num_points(), 3);
        assert_eq!(interp.num_pillars(), 2);

        // t=1.0: maps to pillar 0 only
        assert_relative_eq!(interp.matrix()[(0, 0)], 1.0, epsilon = 1e-10);
        assert_relative_eq!(interp.matrix()[(0, 1)], 0.0, epsilon = 1e-10);

        // t=1.5: interpolates between pillars 0 and 1
        assert_relative_eq!(interp.matrix()[(1, 0)], 0.5, epsilon = 1e-10);
        assert_relative_eq!(interp.matrix()[(1, 1)], 0.5, epsilon = 1e-10);

        // t=2.0: maps to pillar 1 only
        assert_relative_eq!(interp.matrix()[(2, 0)], 0.0, epsilon = 1e-10);
        assert_relative_eq!(interp.matrix()[(2, 1)], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_interpolation_matrix_interpolate() {
        let pillars = vec![1.0, 2.0];
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 1.5, 2.0]);

        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        let log_df_pillars = vec![-0.03, -0.06];
        let log_df_points = interp.interpolate(&log_df_pillars);

        assert_relative_eq!(log_df_points[0], -0.03, epsilon = 1e-10);
        assert_relative_eq!(log_df_points[1], -0.045, epsilon = 1e-10);
        assert_relative_eq!(log_df_points[2], -0.06, epsilon = 1e-10);
    }

    #[test]
    fn test_interpolation_matrix_interpolate_df() {
        let pillars = vec![1.0, 2.0];
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 2.0]);

        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        let log_df_pillars = vec![-0.03, -0.06];
        let df_points = interp.interpolate_df(&log_df_pillars);

        assert_relative_eq!(df_points[0], (-0.03f64).exp(), epsilon = 1e-8);
        assert_relative_eq!(df_points[1], (-0.06f64).exp(), epsilon = 1e-8);
    }

    #[test]
    fn test_interpolation_matrix_extrapolation() {
        let pillars = vec![1.0, 2.0];
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![0.5, 1.0, 2.0, 3.0]);

        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        // t=0.5: before first pillar (extrapolate flat)
        assert_relative_eq!(interp.matrix()[(0, 0)], 1.0, epsilon = 1e-10);
        assert_relative_eq!(interp.matrix()[(0, 1)], 0.0, epsilon = 1e-10);

        // t=3.0: after last pillar (extrapolate flat)
        assert_relative_eq!(interp.matrix()[(3, 0)], 0.0, epsilon = 1e-10);
        assert_relative_eq!(interp.matrix()[(3, 1)], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_calibration_matrix_builder() {
        let builder: CalibrationMatrixBuilder<f64> =
            CalibrationMatrixBuilder::new().with_tolerance(1e-8);

        assert!(builder.tolerance > 0.0);
    }

    #[test]
    fn test_cashflow_convenience_methods() {
        let mut matrix: CalibrationMatrix<f64> = CalibrationMatrix::zeros(2, 3);

        matrix.set_cashflow(0, 1, 100.0);
        assert_relative_eq!(matrix.get_cashflow(0, 1).unwrap(), 100.0, epsilon = 1e-10);
        assert_eq!(matrix.num_instruments(), 2);
        assert_eq!(matrix.num_dates(), 3);
        assert!(matrix.has_cashflow(0, 1));
    }
}
