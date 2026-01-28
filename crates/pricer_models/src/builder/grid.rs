//! Calibration grid for unified axis management.
//!
//! This module provides `CalibrationGrid<T>` which manages a unified axis
//! (time, strike, tenor, etc.) for calibration, collecting and de-duplicating points.
//!
//! # Usage
//!
//! - **Curve calibration**: Collect cashflow dates from all instruments
//! - **Vol surface calibration**: Manage expiry/strike grids
//! - **Vol cube calibration**: Manage expiry/tenor/strike grids

use num_traits::Float;

use super::CalibrationInstrument;

// =============================================================================
// CalibrationGrid
// =============================================================================

/// Calibration grid for unified axis management.
///
/// Collects, sorts, and deduplicates points to create a unified axis
/// for matrix-based calculations.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for axis values
///
/// # Example
///
/// ```ignore
/// use pricer_models::builder::CalibrationGrid;
///
/// let mut grid: CalibrationGrid<f64> = CalibrationGrid::new();
/// grid.add_point(1.0);
/// grid.add_point(2.0);
/// grid.add_point(1.0); // Duplicate, ignored
/// assert_eq!(grid.len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct CalibrationGrid<T: Float> {
    /// Sorted, deduplicated values.
    points: Vec<T>,
    /// Tolerance for considering two values as equal.
    tolerance: T,
}

impl<T: Float> CalibrationGrid<T> {
    /// Create an empty grid with default tolerance.
    pub fn new() -> Self {
        Self::with_tolerance(T::from(1e-10).unwrap())
    }

    /// Create an empty grid with specified tolerance.
    pub fn with_tolerance(tolerance: T) -> Self {
        Self {
            points: Vec::new(),
            tolerance,
        }
    }

    /// Create a grid from calibration instruments (using maturities).
    ///
    /// # Arguments
    ///
    /// * `instruments` - Slice of calibration instruments
    pub fn from_instruments<I: CalibrationInstrument<T>>(instruments: &[I]) -> Self {
        let mut grid = Self::new();

        for instrument in instruments {
            grid.add_point(instrument.maturity());
        }

        grid
    }

    /// Create a grid from explicit values.
    pub fn from_points(points: impl IntoIterator<Item = T>) -> Self {
        let mut grid = Self::new();
        for p in points {
            grid.add_point(p);
        }
        grid
    }

    /// Add a single point to the grid.
    ///
    /// If the point already exists (within tolerance), it is not added again.
    pub fn add_point(&mut self, point: T) {
        if self.contains(point) {
            return;
        }

        self.points.push(point);
        self.points.sort_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Add multiple points to the grid.
    pub fn add_points(&mut self, points: impl IntoIterator<Item = T>) {
        for p in points {
            if !self.contains(p) {
                self.points.push(p);
            }
        }

        self.points.sort_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Check if a point exists in the grid (within tolerance).
    pub fn contains(&self, point: T) -> bool {
        self.points.iter().any(|&p| Float::abs(p - point) < self.tolerance)
    }

    /// Get the index of a point.
    ///
    /// Returns `None` if the point is not in the grid.
    pub fn get_index(&self, point: T) -> Option<usize> {
        self.points.iter().position(|&p| Float::abs(p - point) < self.tolerance)
    }

    /// Get interpolation indices and weight for a point.
    ///
    /// Returns (lower_index, upper_index, weight) for linear interpolation.
    ///
    /// # Arguments
    ///
    /// * `point` - Point to locate
    ///
    /// # Returns
    ///
    /// Tuple of (lower_index, upper_index, weight) where:
    /// - `lower_index` is the largest grid point ≤ query point
    /// - `upper_index` is the smallest grid point ≥ query point
    /// - `weight` is the interpolation factor: 0.0 = lower, 1.0 = upper
    pub fn get_interpolation_indices(&self, point: T) -> Option<(usize, usize, T)> {
        if self.points.is_empty() {
            return None;
        }

        let pos = self.points.partition_point(|&p| p < point);

        if pos == 0 {
            Some((0, 0, T::zero()))
        } else if pos >= self.points.len() {
            let last = self.points.len() - 1;
            Some((last, last, T::one()))
        } else {
            let lower = pos - 1;
            let upper = pos;
            let p_lower = self.points[lower];
            let p_upper = self.points[upper];
            let weight = (point - p_lower) / (p_upper - p_lower);
            Some((lower, upper, weight))
        }
    }

    /// Get the value at a given index.
    pub fn get(&self, index: usize) -> Option<T> {
        self.points.get(index).copied()
    }

    /// Get all points in the grid.
    pub fn points(&self) -> &[T] {
        &self.points
    }

    /// Alias for `points()` - useful for time-based grids.
    pub fn times(&self) -> &[T] {
        &self.points
    }

    /// Get the number of points in the grid.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Check if the grid is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Get the minimum value in the grid.
    pub fn min(&self) -> Option<T> {
        self.points.first().copied()
    }

    /// Get the maximum value in the grid.
    pub fn max(&self) -> Option<T> {
        self.points.last().copied()
    }
}

impl<T: Float> Default for CalibrationGrid<T> {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_empty_grid() {
        let grid: CalibrationGrid<f64> = CalibrationGrid::new();
        assert!(grid.is_empty());
        assert_eq!(grid.len(), 0);
        assert!(grid.min().is_none());
        assert!(grid.max().is_none());
    }

    #[test]
    fn test_add_single_point() {
        let mut grid: CalibrationGrid<f64> = CalibrationGrid::new();
        grid.add_point(1.0);

        assert_eq!(grid.len(), 1);
        assert_relative_eq!(grid.points()[0], 1.0, epsilon = 1e-10);
        assert_eq!(grid.get_index(1.0), Some(0));
    }

    #[test]
    fn test_add_multiple_points() {
        let mut grid: CalibrationGrid<f64> = CalibrationGrid::new();
        grid.add_points(vec![2.0, 1.0, 3.0]);

        assert_eq!(grid.len(), 3);
        assert_relative_eq!(grid.points()[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(grid.points()[1], 2.0, epsilon = 1e-10);
        assert_relative_eq!(grid.points()[2], 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_deduplicate() {
        let mut grid: CalibrationGrid<f64> = CalibrationGrid::new();
        grid.add_point(1.0);
        grid.add_point(1.0);
        grid.add_point(1.0 + 1e-12);

        assert_eq!(grid.len(), 1);
    }

    #[test]
    fn test_from_points() {
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![3.0, 1.0, 2.0, 1.0]);

        assert_eq!(grid.len(), 3);
        assert_relative_eq!(grid.points()[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(grid.points()[1], 2.0, epsilon = 1e-10);
        assert_relative_eq!(grid.points()[2], 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_get_index() {
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 2.0, 5.0]);

        assert_eq!(grid.get_index(1.0), Some(0));
        assert_eq!(grid.get_index(2.0), Some(1));
        assert_eq!(grid.get_index(5.0), Some(2));
        assert_eq!(grid.get_index(3.0), None);
    }

    #[test]
    fn test_get() {
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 2.0, 5.0]);

        assert_relative_eq!(grid.get(0).unwrap(), 1.0, epsilon = 1e-10);
        assert_relative_eq!(grid.get(1).unwrap(), 2.0, epsilon = 1e-10);
        assert_relative_eq!(grid.get(2).unwrap(), 5.0, epsilon = 1e-10);
        assert!(grid.get(3).is_none());
    }

    #[test]
    fn test_min_max() {
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![3.0, 1.0, 5.0, 2.0]);

        assert_relative_eq!(grid.min().unwrap(), 1.0, epsilon = 1e-10);
        assert_relative_eq!(grid.max().unwrap(), 5.0, epsilon = 1e-10);
    }

    #[test]
    fn test_interpolation_indices() {
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 2.0, 5.0]);

        // Exact match at first point
        let (lower, upper, weight) = grid.get_interpolation_indices(1.0).unwrap();
        assert_eq!(lower, 0);
        assert_eq!(upper, 0);
        assert_relative_eq!(weight, 0.0, epsilon = 1e-10);

        // Between points
        let (lower, upper, weight) = grid.get_interpolation_indices(1.5).unwrap();
        assert_eq!(lower, 0);
        assert_eq!(upper, 1);
        assert_relative_eq!(weight, 0.5, epsilon = 1e-10);

        // Between points 2 and 5
        let (lower, upper, weight) = grid.get_interpolation_indices(3.0).unwrap();
        assert_eq!(lower, 1);
        assert_eq!(upper, 2);
        assert_relative_eq!(weight, 1.0 / 3.0, epsilon = 1e-10);

        // Before first point
        let (lower, upper, _weight) = grid.get_interpolation_indices(0.5).unwrap();
        assert_eq!(lower, 0);
        assert_eq!(upper, 0);

        // After last point
        let (lower, upper, _weight) = grid.get_interpolation_indices(6.0).unwrap();
        assert_eq!(lower, 2);
        assert_eq!(upper, 2);
    }

    #[test]
    fn test_contains() {
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 2.0, 5.0]);

        assert!(grid.contains(1.0));
        assert!(grid.contains(2.0));
        assert!(grid.contains(5.0));
        assert!(!grid.contains(3.0));
        assert!(!grid.contains(0.5));
    }

    #[test]
    fn test_times_alias() {
        let grid: CalibrationGrid<f64> = CalibrationGrid::from_points(vec![1.0, 2.0]);
        assert_eq!(grid.times().len(), 2);
    }
}
