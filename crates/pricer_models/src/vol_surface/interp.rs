//! Interpolation utilities for volatility surfaces.
//!
//! Provides linear and bilinear interpolation routines used by
//! the various surface implementations.

use pricer_core::traits::Float;

/// Binary-search a sorted grid and return the bracket indices `(lo, hi)`
/// such that `grid[lo] <= value <= grid[hi]`.
///
/// If `value` is at or below the first element, returns `(0, 0)`.
/// If `value` is at or above the last element, returns `(n-1, n-1)`.
pub fn find_bracket<T: Float>(grid: &[T], value: T) -> (usize, usize) {
    let n = grid.len();
    if n == 0 {
        return (0, 0);
    }
    if n == 1 || value <= grid[0] {
        return (0, 0);
    }
    if value >= grid[n - 1] {
        return (n - 1, n - 1);
    }

    // Standard binary search for the lower bound
    let mut lo: usize = 0;
    let mut hi: usize = n - 1;
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if grid[mid] <= value {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo, hi)
}

/// Simple linear interpolation between two points.
///
/// Computes `y0 + (y1 - y0) * (x - x0) / (x1 - x0)`.
/// If `x0 == x1` the function returns `y0` to avoid division by zero.
pub fn linear_interp<T: Float>(x0: T, y0: T, x1: T, y1: T, x: T) -> T {
    let dx = x1 - x0;
    if dx.abs() < T::epsilon() {
        return y0;
    }
    y0 + (y1 - y0) * (x - x0) / dx
}

/// Bilinear interpolation on a 2-D grid stored in row-major order.
///
/// `xs` are column coordinates, `ys` are row coordinates, and `values`
/// has length `ys.len() * xs.len()` laid out row-by-row.
///
/// Returns `None` if the grid dimensions are inconsistent or empty.
pub fn bilinear_interp<T: Float>(xs: &[T], ys: &[T], values: &[T], x: T, y: T) -> Option<T> {
    let nx = xs.len();
    let ny = ys.len();
    if nx == 0 || ny == 0 || values.len() != ny * nx {
        return None;
    }

    let (xi0, xi1) = find_bracket(xs, x);
    let (yi0, yi1) = find_bracket(ys, y);

    // Helper to index row-major layout: row = y-index, col = x-index
    let idx = |row: usize, col: usize| -> usize { row * nx + col };

    // Degenerate cases (boundary or single-element grids)
    if xi0 == xi1 && yi0 == yi1 {
        return Some(values[idx(yi0, xi0)]);
    }
    if xi0 == xi1 {
        let v0 = values[idx(yi0, xi0)];
        let v1 = values[idx(yi1, xi0)];
        return Some(linear_interp(ys[yi0], v0, ys[yi1], v1, y));
    }
    if yi0 == yi1 {
        let v0 = values[idx(yi0, xi0)];
        let v1 = values[idx(yi0, xi1)];
        return Some(linear_interp(xs[xi0], v0, xs[xi1], v1, x));
    }

    // Full bilinear interpolation
    let q00 = values[idx(yi0, xi0)];
    let q10 = values[idx(yi0, xi1)];
    let q01 = values[idx(yi1, xi0)];
    let q11 = values[idx(yi1, xi1)];

    let fx0 = linear_interp(xs[xi0], q00, xs[xi1], q10, x);
    let fx1 = linear_interp(xs[xi0], q01, xs[xi1], q11, x);

    Some(linear_interp(ys[yi0], fx0, ys[yi1], fx1, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ── find_bracket ──────────────────────────────────────────────────

    #[test]
    fn test_find_bracket_empty() {
        let grid: &[f64] = &[];
        assert_eq!(find_bracket(grid, 1.0), (0, 0));
    }

    #[test]
    fn test_find_bracket_single() {
        assert_eq!(find_bracket(&[5.0_f64], 3.0), (0, 0));
        assert_eq!(find_bracket(&[5.0_f64], 7.0), (0, 0));
    }

    #[test]
    fn test_find_bracket_below() {
        let grid = [1.0_f64, 2.0, 3.0, 4.0];
        assert_eq!(find_bracket(&grid, 0.5), (0, 0));
    }

    #[test]
    fn test_find_bracket_above() {
        let grid = [1.0_f64, 2.0, 3.0, 4.0];
        assert_eq!(find_bracket(&grid, 5.0), (3, 3));
    }

    #[test]
    fn test_find_bracket_interior() {
        let grid = [1.0_f64, 2.0, 3.0, 4.0];
        assert_eq!(find_bracket(&grid, 2.5), (1, 2));
    }

    #[test]
    fn test_find_bracket_exact_node() {
        let grid = [1.0_f64, 2.0, 3.0, 4.0];
        assert_eq!(find_bracket(&grid, 2.0), (1, 2));
    }

    // ── linear_interp ─────────────────────────────────────────────────

    #[test]
    fn test_linear_interp_midpoint() {
        let v = linear_interp(0.0_f64, 0.0, 2.0, 4.0, 1.0);
        assert_relative_eq!(v, 2.0, epsilon = 1e-12);
    }

    #[test]
    fn test_linear_interp_endpoints() {
        assert_relative_eq!(linear_interp(0.0_f64, 1.0, 1.0, 3.0, 0.0), 1.0);
        assert_relative_eq!(linear_interp(0.0_f64, 1.0, 1.0, 3.0, 1.0), 3.0);
    }

    #[test]
    fn test_linear_interp_degenerate() {
        // x0 == x1 should return y0
        assert_relative_eq!(linear_interp(2.0_f64, 5.0, 2.0, 7.0, 2.0), 5.0);
    }

    // ── bilinear_interp ───────────────────────────────────────────────

    #[test]
    fn test_bilinear_interp_simple() {
        // 2x2 grid:  xs = [0, 1], ys = [0, 1]
        // row-major values:
        //   (y=0, x=0)=0, (y=0, x=1)=1,
        //   (y=1, x=0)=2, (y=1, x=1)=3
        let xs = [0.0_f64, 1.0];
        let ys = [0.0_f64, 1.0];
        let vals = [0.0, 1.0, 2.0, 3.0];

        let v = bilinear_interp(&xs, &ys, &vals, 0.5, 0.5).unwrap();
        assert_relative_eq!(v, 1.5, epsilon = 1e-12);
    }

    #[test]
    fn test_bilinear_interp_at_corner() {
        let xs = [0.0_f64, 1.0];
        let ys = [0.0_f64, 1.0];
        let vals = [10.0, 20.0, 30.0, 40.0];

        assert_relative_eq!(
            bilinear_interp(&xs, &ys, &vals, 0.0, 0.0).unwrap(),
            10.0,
            epsilon = 1e-12
        );
        assert_relative_eq!(
            bilinear_interp(&xs, &ys, &vals, 1.0, 1.0).unwrap(),
            40.0,
            epsilon = 1e-12
        );
    }

    #[test]
    fn test_bilinear_interp_invalid_dimensions() {
        let xs = [0.0_f64, 1.0];
        let ys = [0.0_f64, 1.0];
        let vals = [1.0, 2.0, 3.0]; // wrong length
        assert!(bilinear_interp(&xs, &ys, &vals, 0.5, 0.5).is_none());
    }

    #[test]
    fn test_bilinear_interp_empty() {
        let xs: &[f64] = &[];
        let ys: &[f64] = &[];
        let vals: &[f64] = &[];
        assert!(bilinear_interp(xs, ys, vals, 0.5, 0.5).is_none());
    }

    #[test]
    fn test_bilinear_interp_3x3() {
        // 3x3 grid: xs = [0, 1, 2], ys = [0, 1, 2]
        // All values = 5.0 (constant surface)
        let xs = [0.0_f64, 1.0, 2.0];
        let ys = [0.0_f64, 1.0, 2.0];
        let vals = [5.0; 9];

        let v = bilinear_interp(&xs, &ys, &vals, 0.7, 1.3).unwrap();
        assert_relative_eq!(v, 5.0, epsilon = 1e-12);
    }
}
