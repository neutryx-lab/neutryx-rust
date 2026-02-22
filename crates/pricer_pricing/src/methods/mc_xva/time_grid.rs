//! XVA time grid construction and manipulation.
//!
//! Provides [`XvaTimeGrid`] for building regular, tenor-based, or merged
//! time grids used in XVA Monte Carlo simulations.

/// Tolerance for deduplicating nearby time points.
const DEDUP_TOL: f64 = 1e-10;

/// A time grid for XVA simulations, represented as monotonically increasing
/// year-fraction time points from the valuation date.
#[derive(Clone, Debug)]
pub struct XvaTimeGrid {
    /// Sorted, deduplicated time points (year fractions).
    points: Vec<f64>,
}

impl XvaTimeGrid {
    /// Creates a regular time grid from `start` to `end` (inclusive) with
    /// the given frequency in years.
    ///
    /// # Panics
    ///
    /// Panics if `frequency_years <= 0.0` or `end < start`.
    pub fn regular(start: f64, end: f64, frequency_years: f64) -> Self {
        assert!(
            frequency_years > 0.0,
            "frequency_years must be positive, got {frequency_years}"
        );
        assert!(end >= start, "end ({end}) must be >= start ({start})");

        let mut points = Vec::new();
        let mut t = start;
        while t <= end + DEDUP_TOL {
            points.push(t);
            t += frequency_years;
        }
        // Ensure the endpoint is included if not already present.
        if let Some(&last) = points.last() {
            if (last - end).abs() > DEDUP_TOL {
                points.push(end);
            }
        }

        Self { points }
    }

    /// Creates a time grid from explicit tenor points.
    ///
    /// The tenors are sorted and deduplicated.
    pub fn from_tenors(tenors: &[f64]) -> Self {
        let mut points: Vec<f64> = tenors.to_vec();
        points.sort_by(|a, b| a.partial_cmp(b).unwrap());
        points.dedup_by(|a, b| (*a - *b).abs() < DEDUP_TOL);

        Self { points }
    }

    /// Merges multiple time grids into a single deduplicated, sorted grid.
    pub fn merge(grids: &[&XvaTimeGrid]) -> Self {
        let total_len: usize = grids.iter().map(|g| g.points.len()).sum();
        let mut combined = Vec::with_capacity(total_len);
        for grid in grids {
            combined.extend_from_slice(&grid.points);
        }
        combined.sort_by(|a, b| a.partial_cmp(b).unwrap());
        combined.dedup_by(|a, b| (*a - *b).abs() < DEDUP_TOL);

        Self { points: combined }
    }

    /// Returns the time points as a slice.
    #[inline]
    pub fn time_points(&self) -> &[f64] { &self.points }

    /// Returns the number of time points.
    #[inline]
    pub fn n_times(&self) -> usize { self.points.len() }

    /// Finds the index of the nearest time point to `t` within tolerance.
    ///
    /// Returns `None` if no point is within `DEDUP_TOL` of `t`.
    pub fn find_index(&self, t: f64) -> Option<usize> {
        self.points
            .iter()
            .position(|&p| (p - t).abs() < DEDUP_TOL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_regular_grid_quarterly() {
        let grid = XvaTimeGrid::regular(0.25, 2.0, 0.25);
        let pts = grid.time_points();

        assert_eq!(pts.len(), 8);
        assert_relative_eq!(pts[0], 0.25, epsilon = 1e-12);
        assert_relative_eq!(pts[7], 2.0, epsilon = 1e-12);
    }

    #[test]
    fn test_regular_grid_includes_endpoint() {
        let grid = XvaTimeGrid::regular(0.0, 1.0, 0.3);
        let pts = grid.time_points();

        // 0.0, 0.3, 0.6, 0.9, 1.0
        assert_eq!(pts.len(), 5);
        assert_relative_eq!(*pts.last().unwrap(), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_regular_grid_single_point() {
        let grid = XvaTimeGrid::regular(1.0, 1.0, 0.25);
        assert_eq!(grid.n_times(), 1);
        assert_relative_eq!(grid.time_points()[0], 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_from_tenors_sorted_and_deduped() {
        let grid = XvaTimeGrid::from_tenors(&[1.0, 0.5, 0.5, 2.0, 0.25]);
        let pts = grid.time_points();

        assert_eq!(pts.len(), 4);
        assert_relative_eq!(pts[0], 0.25, epsilon = 1e-12);
        assert_relative_eq!(pts[1], 0.5, epsilon = 1e-12);
        assert_relative_eq!(pts[2], 1.0, epsilon = 1e-12);
        assert_relative_eq!(pts[3], 2.0, epsilon = 1e-12);
    }

    #[test]
    fn test_from_tenors_empty() {
        let grid = XvaTimeGrid::from_tenors(&[]);
        assert_eq!(grid.n_times(), 0);
    }

    #[test]
    fn test_merge_grids() {
        let g1 = XvaTimeGrid::from_tenors(&[0.25, 0.5, 1.0]);
        let g2 = XvaTimeGrid::from_tenors(&[0.5, 0.75, 1.5]);
        let merged = XvaTimeGrid::merge(&[&g1, &g2]);
        let pts = merged.time_points();

        assert_eq!(pts.len(), 5);
        assert_relative_eq!(pts[0], 0.25, epsilon = 1e-12);
        assert_relative_eq!(pts[1], 0.5, epsilon = 1e-12);
        assert_relative_eq!(pts[2], 0.75, epsilon = 1e-12);
        assert_relative_eq!(pts[3], 1.0, epsilon = 1e-12);
        assert_relative_eq!(pts[4], 1.5, epsilon = 1e-12);
    }

    #[test]
    fn test_merge_empty_grids() {
        let merged = XvaTimeGrid::merge(&[]);
        assert_eq!(merged.n_times(), 0);
    }

    #[test]
    fn test_find_index_exact() {
        let grid = XvaTimeGrid::from_tenors(&[0.25, 0.5, 1.0, 2.0]);

        assert_eq!(grid.find_index(0.25), Some(0));
        assert_eq!(grid.find_index(1.0), Some(2));
        assert_eq!(grid.find_index(2.0), Some(3));
    }

    #[test]
    fn test_find_index_not_found() {
        let grid = XvaTimeGrid::from_tenors(&[0.25, 0.5, 1.0]);

        assert_eq!(grid.find_index(0.75), None);
        assert_eq!(grid.find_index(3.0), None);
    }

    #[test]
    fn test_n_times() {
        let grid = XvaTimeGrid::from_tenors(&[0.25, 0.5, 1.0]);
        assert_eq!(grid.n_times(), 3);
    }

    #[test]
    #[should_panic(expected = "frequency_years must be positive")]
    fn test_regular_grid_zero_frequency_panics() {
        XvaTimeGrid::regular(0.0, 1.0, 0.0);
    }

    #[test]
    #[should_panic(expected = "end")]
    fn test_regular_grid_end_before_start_panics() {
        XvaTimeGrid::regular(2.0, 1.0, 0.25);
    }
}
