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
    linalg::{DMatrix, DVector, RealField},
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
    /// The underlying matrix (N instruments x M grid points).
    pub inner: DMatrix<T>,
}

impl<T: Float + RealField + Copy> CalibrationMatrix<T> {
    /// Create a zero calibration matrix.
    pub fn zeros(num_rows: usize, num_cols: usize) -> Self {
        Self {
            inner: DMatrix::zeros(num_rows, num_cols),
        }
    }

    /// Set a value at (row, col), silently ignoring out-of-bounds.
    pub fn set(&mut self, row: usize, col: usize, value: T) {
        if row < self.inner.nrows() && col < self.inner.ncols() {
            self.inner[(row, col)] = value;
        }
    }

    /// Get a value at (row, col).
    pub fn get(&self, row: usize, col: usize) -> Option<T> {
        if row < self.inner.nrows() && col < self.inner.ncols() {
            Some(self.inner[(row, col)])
        } else {
            None
        }
    }

    /// Check if a specific value is non-zero.
    pub fn is_nonzero(&self, row: usize, col: usize) -> bool {
        self.get(row, col)
            .map(|v| Float::abs(v) > from_f64(1e-15))
            .unwrap_or(false)
    }
}

// =============================================================================
// InterpolationMatrix
// =============================================================================

/// Interpolation matrix for mapping pillar values to grid points.
///
/// An M×P matrix W where:
/// - M = number of grid points
/// - P = number of pillars
/// - W\[j,k\] = interpolation weight for pillar k contributing to grid point j
///
/// For log-linear interpolation:
/// - log(DF(t)) = (1-w) * log(DF(t_k)) + w * log(DF(t_{k+1}))
/// - where w = (t - t_k) / (t_{k+1} - t_k)
#[derive(Debug, Clone)]
pub struct InterpolationMatrix<T: Float + RealField + Copy> {
    /// The underlying matrix (M grid points x P pillars).
    pub inner: DMatrix<T>,
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
    /// An interpolation matrix W where W\[j,k\] is the weight of pillar k for
    /// point j.
    pub fn from_pillars(pillars: &[T], grid: &CalibrationGrid<T>) -> Self {
        let points = grid.points();
        let num_points = points.len();
        let num_pillars = pillars.len();

        if num_pillars == 0 || num_points == 0 {
            return Self {
                inner: DMatrix::zeros(num_points, num_pillars),
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

        Self { inner: matrix }
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

    /// Interpolate values from pillar values.
    pub fn interpolate(&self, pillar_values: &[T]) -> Vec<T> {
        let nrows = self.inner.nrows();
        let ncols = self.inner.ncols();
        let mut result = Vec::with_capacity(nrows);

        for j in 0..nrows {
            let mut value = T::zero();
            for k in 0..ncols {
                value = value + self.inner[(j, k)] * pillar_values[k];
            }
            result.push(value);
        }

        result
    }

    /// Interpolate and exponentiate (for discount factors).
    pub fn interpolate_df(&self, log_df_pillars: &[T]) -> Vec<T> {
        self.interpolate(log_df_pillars)
            .into_iter()
            .map(Float::exp)
            .collect()
    }

    /// Compute all cashflow DFs from pillar DFs using vector product.
    ///
    /// # Requirement 4.1, 4.3
    pub fn apply(&self, pillar_dfs: &DVector<T>) -> DVector<T> {
        &self.inner * pillar_dfs
    }

    /// Compute all cashflow DFs using log-linear interpolation.
    ///
    /// # Requirement 4.5
    pub fn apply_log_linear(&self, pillar_log_dfs: &DVector<T>) -> DVector<T> {
        let log_result = &self.inner * pillar_log_dfs;
        log_result.map(|x| Float::exp(x))
    }
}

// =============================================================================
// Jump-Aware Interpolation Extensions
// =============================================================================

/// Jump pillar information for interpolation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpInfo<T: Float> {
    /// Time to jump in years.
    pub time: T,
    /// Jump size in absolute rate (not bps).
    pub jump_rate: T,
}

impl<T: Float> JumpInfo<T> {
    /// Create a new jump info.
    pub fn new(time: T, jump_rate: T) -> Self { Self { time, jump_rate } }

    /// Create from time and basis points.
    pub fn from_bps(time: T, jump_bps: T) -> Self {
        Self {
            time,
            jump_rate: jump_bps * from_f64::<T>(0.0001),
        }
    }
}

impl<T: Float + RealField + Copy> InterpolationMatrix<T> {
    /// Create an interpolation matrix with jump pillars as segment boundaries.
    ///
    /// Jump pillars create discontinuities in the forward rate curve.
    /// This method ensures interpolation respects those boundaries.
    ///
    /// # Arguments
    ///
    /// * `pillars` - Regular curve pillars (sorted)
    /// * `jump_times` - Jump pillar times (sorted)
    /// * `grid` - Calibration grid with all points
    ///
    /// # Returns
    ///
    /// An interpolation matrix that treats jump times as segment boundaries.
    pub fn with_jump_pillars(pillars: &[T], jump_times: &[T], grid: &CalibrationGrid<T>) -> Self {
        // Merge pillars and jump times into sorted unique list
        let mut all_breaks: Vec<T> = pillars.to_vec();
        for &jt in jump_times {
            if !all_breaks
                .iter()
                .any(|&p| Float::abs(p - jt) < from_f64::<T>(1e-10))
            {
                all_breaks.push(jt);
            }
        }
        all_breaks.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Use standard interpolation with enhanced break points
        Self::from_pillars(&all_breaks, grid)
    }

    /// Interpolate log discount factors with jump adjustments.
    ///
    /// Applies jump effects multiplicatively to discount factors:
    /// DF(t) = DF_smooth(t) × Π(1 - jump_i × Δt_i) for all jumps before t
    ///
    /// # Arguments
    ///
    /// * `log_df_pillars` - log(DF) at regular pillars
    /// * `jumps` - Jump information (time and size)
    /// * `grid_points` - Times at which to evaluate
    ///
    /// # Returns
    ///
    /// Adjusted log(DF) values at each grid point.
    pub fn interpolate_with_jumps(
        &self,
        log_df_pillars: &[T],
        jumps: &[JumpInfo<T>],
        grid_points: &[T],
    ) -> Vec<T> {
        // First, get smooth interpolated values
        let smooth_log_df = self.interpolate(log_df_pillars);

        if jumps.is_empty() {
            return smooth_log_df;
        }

        // Apply cumulative jump adjustments
        let mut result = Vec::with_capacity(grid_points.len());

        for (i, &t) in grid_points.iter().enumerate() {
            let base_log_df = smooth_log_df[i];

            // Calculate cumulative jump effect for all jumps before time t
            let jump_adjustment = self.calculate_jump_adjustment(t, jumps);

            // Apply adjustment: log(DF_adjusted) = log(DF_smooth) + log(1 -
            // cumulative_jump_effect) For small jumps, log(1 - x) ≈ -x
            result.push(base_log_df + jump_adjustment);
        }

        result
    }

    /// Calculate the cumulative jump adjustment for a given time.
    ///
    /// For a forward rate jump Δf at time t_j, the discount factor effect is:
    /// DF(t) = DF_smooth(t) × exp(-Δf × (t - t_j)) for t > t_j
    fn calculate_jump_adjustment(&self, t: T, jumps: &[JumpInfo<T>]) -> T {
        let mut adjustment = T::zero();

        for jump in jumps {
            if jump.time < t {
                // Time from jump to current point
                let dt = t - jump.time;
                // Forward rate jump affects DF as: -Δf × dt
                adjustment = adjustment - jump.jump_rate * dt;
            }
        }

        adjustment
    }

    /// Interpolate discount factors with jump adjustments.
    ///
    /// # Arguments
    ///
    /// * `log_df_pillars` - log(DF) at regular pillars
    /// * `jumps` - Jump information (time and size)
    /// * `grid_points` - Times at which to evaluate
    ///
    /// # Returns
    ///
    /// Adjusted discount factors at each grid point.
    pub fn interpolate_df_with_jumps(
        &self,
        log_df_pillars: &[T],
        jumps: &[JumpInfo<T>],
        grid_points: &[T],
    ) -> Vec<T> {
        self.interpolate_with_jumps(log_df_pillars, jumps, grid_points)
            .into_iter()
            .map(Float::exp)
            .collect()
    }
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

    // =========================================================================
    // Jump-Aware Interpolation Tests
    // =========================================================================

    #[test]
    fn test_jump_info_creation() {
        let jump = JumpInfo::new(0.5, 0.0025);
        assert_relative_eq!(jump.time, 0.5, epsilon = 1e-10);
        assert_relative_eq!(jump.jump_rate, 0.0025, epsilon = 1e-10);

        let jump_bps = JumpInfo::from_bps(1.0, 25.0);
        assert_relative_eq!(jump_bps.time, 1.0, epsilon = 1e-10);
        assert_relative_eq!(jump_bps.jump_rate, 0.0025, epsilon = 1e-10);
    }

    #[test]
    fn test_interpolation_with_jump_pillars() {
        let pillars = vec![1.0, 2.0, 5.0];
        let jump_times = vec![0.5, 1.5]; // Jump at 6M and 18M
        let grid: CalibrationGrid<f64> =
            CalibrationGrid::from_points(vec![0.5, 1.0, 1.5, 2.0, 5.0]);

        let interp = InterpolationMatrix::with_jump_pillars(&pillars, &jump_times, &grid);

        // Should have 5 grid points
        assert_eq!(interp.num_points(), 5);
        // Should have 5 "pillars" (3 regular + 2 jump times)
        assert_eq!(interp.num_pillars(), 5);
    }

    #[test]
    fn test_interpolate_with_jumps_no_jumps() {
        let pillars = vec![1.0, 2.0];
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 1.5, 2.0]);
        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        let log_df_pillars = vec![-0.03, -0.06];
        let jumps: Vec<JumpInfo<f64>> = vec![];
        let grid_points = vec![1.0, 1.5, 2.0];

        let result = interp.interpolate_with_jumps(&log_df_pillars, &jumps, &grid_points);

        // Without jumps, should be same as regular interpolation
        assert_relative_eq!(result[0], -0.03, epsilon = 1e-10);
        assert_relative_eq!(result[1], -0.045, epsilon = 1e-10);
        assert_relative_eq!(result[2], -0.06, epsilon = 1e-10);
    }

    #[test]
    fn test_interpolate_with_jumps_single_jump() {
        let pillars = vec![1.0, 2.0];
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![0.5, 1.0, 1.5, 2.0]);
        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        let log_df_pillars = vec![-0.03, -0.06];
        // 25bps jump at 0.5Y
        let jumps = vec![JumpInfo::from_bps(0.5, 25.0)];
        let grid_points = vec![0.5, 1.0, 1.5, 2.0];

        let result = interp.interpolate_with_jumps(&log_df_pillars, &jumps, &grid_points);

        // At t=0.5: No adjustment (jump happens at this time, effect starts after)
        // At t=1.0: Adjustment for dt=0.5 from jump at 0.5
        // At t=1.5: Adjustment for dt=1.0 from jump at 0.5
        // At t=2.0: Adjustment for dt=1.5 from jump at 0.5

        // Jump effect: -0.0025 × (t - 0.5) for t > 0.5
        let smooth_log_df = interp.interpolate(&log_df_pillars);

        // Before jump - no effect
        assert_relative_eq!(result[0], smooth_log_df[0], epsilon = 1e-10);

        // After jump - should see effect
        let expected_1 = smooth_log_df[1] - 0.0025 * 0.5; // dt = 0.5
        assert_relative_eq!(result[1], expected_1, epsilon = 1e-10);

        let expected_1_5 = smooth_log_df[2] - 0.0025 * 1.0; // dt = 1.0
        assert_relative_eq!(result[2], expected_1_5, epsilon = 1e-10);
    }

    #[test]
    fn test_interpolate_df_with_jumps() {
        let pillars = vec![1.0, 2.0];
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 2.0]);
        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        let log_df_pillars = vec![-0.03, -0.06];
        let jumps = vec![JumpInfo::from_bps(0.5, 25.0)];
        let grid_points = vec![1.0, 2.0];

        let df_result = interp.interpolate_df_with_jumps(&log_df_pillars, &jumps, &grid_points);

        // Should get positive discount factors
        assert!(df_result[0] > 0.0 && df_result[0] < 1.0);
        assert!(df_result[1] > 0.0 && df_result[1] < 1.0);

        // Jump should make DF smaller (higher forward rates)
        let df_no_jump = interp.interpolate_df(&log_df_pillars);
        assert!(df_result[0] < df_no_jump[0]);
        assert!(df_result[1] < df_no_jump[1]);
    }

    // =========================================================================
    // apply() and apply_log_linear() Tests (Requirement 4)
    // =========================================================================

    #[test]
    fn test_apply_basic() {
        let pillars = vec![1.0, 2.0];
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 1.5, 2.0]);
        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        // Test with discount factors directly
        let pillar_dfs = DVector::from_vec(vec![0.97, 0.94]);
        let result = interp.apply(&pillar_dfs);

        assert_eq!(result.len(), 3);
        // At t=1.0: DF should be 0.97 (pillar 0)
        assert_relative_eq!(result[0], 0.97, epsilon = 1e-10);
        // At t=1.5: DF should be 0.5*0.97 + 0.5*0.94 = 0.955
        assert_relative_eq!(result[1], 0.955, epsilon = 1e-10);
        // At t=2.0: DF should be 0.94 (pillar 1)
        assert_relative_eq!(result[2], 0.94, epsilon = 1e-10);
    }

    #[test]
    fn test_apply_consistency_with_interpolate() {
        let pillars = vec![1.0, 2.0, 5.0];
        let grid: CalibrationGrid<f64> =
            CalibrationGrid::from_points(vec![0.5, 1.0, 1.5, 2.0, 3.0, 5.0]);
        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        let pillar_values = vec![0.97, 0.94, 0.85];

        // Compare apply() with interpolate()
        let vec_result = interp.interpolate(&pillar_values);
        let dvec_result = interp.apply(&DVector::from_vec(pillar_values));

        for (&v1, v2) in vec_result.iter().zip(dvec_result.iter()) {
            assert_relative_eq!(v1, *v2, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_apply_log_linear_basic() {
        let pillars = vec![1.0, 2.0];
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 1.5, 2.0]);
        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        let pillar_log_dfs = DVector::from_vec(vec![-0.03, -0.06]);
        let result = interp.apply_log_linear(&pillar_log_dfs);

        assert_eq!(result.len(), 3);

        // Result should be discount factors, not log(DF)
        assert!(result[0] > 0.0 && result[0] < 1.0);
        assert!(result[1] > 0.0 && result[1] < 1.0);
        assert!(result[2] > 0.0 && result[2] < 1.0);

        // At t=1.0: DF should be exp(-0.03)
        assert_relative_eq!(result[0], (-0.03f64).exp(), epsilon = 1e-10);
        // At t=2.0: DF should be exp(-0.06)
        assert_relative_eq!(result[2], (-0.06f64).exp(), epsilon = 1e-10);
    }

    #[test]
    fn test_apply_log_linear_consistency_with_interpolate_df() {
        let pillars = vec![1.0, 2.0, 5.0];
        let grid: CalibrationGrid<f64> =
            CalibrationGrid::from_points(vec![1.0, 1.5, 2.0, 3.0, 5.0]);
        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        let log_df_pillars = vec![-0.03, -0.06, -0.15];

        // Compare apply_log_linear() with interpolate_df()
        let vec_result = interp.interpolate_df(&log_df_pillars);
        let dvec_result = interp.apply_log_linear(&DVector::from_vec(log_df_pillars));

        for (&v1, v2) in vec_result.iter().zip(dvec_result.iter()) {
            assert_relative_eq!(v1, *v2, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_apply_simd_friendly_layout() {
        // Verify that the matrix layout is contiguous and suitable for SIMD
        let pillars = vec![1.0, 2.0, 5.0, 10.0];
        let grid: CalibrationGrid<f64> =
            CalibrationGrid::from_points(vec![1.0, 2.0, 3.0, 5.0, 7.0, 10.0]);
        let interp = InterpolationMatrix::from_pillars(&pillars, &grid);

        // The matrix should be column-major (nalgebra default)
        // This is suitable for SIMD operations on columns
        let matrix = interp.matrix();
        assert_eq!(matrix.nrows(), 6);
        assert_eq!(matrix.ncols(), 4);

        // Matrix-vector multiplication should work efficiently
        let pillar_dfs = DVector::from_vec(vec![0.97, 0.94, 0.85, 0.75]);
        let result = interp.apply(&pillar_dfs);
        assert_eq!(result.len(), 6);
    }
}
