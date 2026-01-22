//! Search algorithms for interpolation.
//!
//! This module provides efficient search algorithms for finding the interval
//! containing a query point in a sorted array of grid points.

use num_traits::Float;

/// Binary search for the interval containing x.
///
/// Given a sorted array `xs` and a value `x`, returns the index `i` such that
/// `xs[i] <= x < xs[i+1]`. If `x` equals the last element, returns `n-2`.
///
/// # Arguments
///
/// * `xs` - Sorted array of x values (ascending order)
/// * `x` - Value to search for
///
/// # Returns
///
/// Index `i` such that `xs[i] <= x < xs[i+1]`, or 0 if `x < xs[0]`,
/// or `n-2` if `x >= xs[n-1]`.
///
/// # Time Complexity
///
/// O(log n)
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::binary_search;
///
/// let xs = [0.0_f64, 1.0, 2.0, 3.0, 4.0];
///
/// assert_eq!(binary_search(&xs, 0.5), 0);  // 0.0 <= 0.5 < 1.0
/// assert_eq!(binary_search(&xs, 1.5), 1);  // 1.0 <= 1.5 < 2.0
/// assert_eq!(binary_search(&xs, 2.0), 2);  // 2.0 <= 2.0 < 3.0
/// assert_eq!(binary_search(&xs, 4.0), 3);  // At boundary, returns n-2
/// ```
#[must_use]
pub fn binary_search<T: Float>(xs: &[T], x: T) -> usize {
    let n = xs.len();
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 0;
    }

    // Handle boundaries
    if x <= xs[0] {
        return 0;
    }
    if x >= xs[n - 1] {
        return n - 2;
    }

    // Binary search
    let mut lo = 0;
    let mut hi = n - 1;

    while hi - lo > 1 {
        let mid = usize::midpoint(lo, hi);
        if x >= xs[mid] {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    lo
}

/// Linear search for the interval containing x.
///
/// Simple O(n) search that scans from the beginning. Useful for small arrays
/// or when the query point is expected to be near the start of the array.
///
/// # Arguments
///
/// * `xs` - Sorted array of x values (ascending order)
/// * `x` - Value to search for
///
/// # Returns
///
/// Index `i` such that `xs[i] <= x < xs[i+1]`.
///
/// # Time Complexity
///
/// O(n)
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::linear_search;
///
/// let xs = [0.0_f64, 1.0, 2.0, 3.0];
///
/// assert_eq!(linear_search(&xs, 0.5), 0);
/// assert_eq!(linear_search(&xs, 1.5), 1);
/// ```
#[must_use]
pub fn linear_search<T: Float>(xs: &[T], x: T) -> usize {
    let n = xs.len();
    if n <= 1 {
        return 0;
    }

    // Handle boundaries
    if x <= xs[0] {
        return 0;
    }

    for i in 1..n {
        if x < xs[i] {
            return i - 1;
        }
    }

    n - 2
}

/// Hunt search for the interval containing x.
///
/// An adaptive search that starts from a hint index and "hunts" in the
/// appropriate direction, then uses binary search. Efficient when
/// successive queries are close together.
///
/// # Arguments
///
/// * `xs` - Sorted array of x values (ascending order)
/// * `x` - Value to search for
/// * `hint` - Starting index hint from a previous search
///
/// # Returns
///
/// Index `i` such that `xs[i] <= x < xs[i+1]`.
///
/// # Time Complexity
///
/// O(log d) where d is the distance from hint to the target, then O(log d) for
/// binary search.
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::hunt_search;
///
/// let xs = [0.0_f64, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
///
/// // Start from hint = 3 and search for 3.5
/// let idx = hunt_search(&xs, 3.5, 3);
/// assert_eq!(idx, 3);
///
/// // Use previous result as hint for nearby query
/// let idx2 = hunt_search(&xs, 4.5, idx);
/// assert_eq!(idx2, 4);
/// ```
#[must_use]
pub fn hunt_search<T: Float>(xs: &[T], x: T, hint: usize) -> usize {
    let n = xs.len();
    if n <= 1 {
        return 0;
    }

    // Handle boundaries
    if x <= xs[0] {
        return 0;
    }
    if x >= xs[n - 1] {
        return n - 2;
    }

    let mut lo = hint.min(n - 2);
    let mut hi = lo + 1;

    // Determine direction and hunt
    if x >= xs[lo] {
        // Hunt upward
        let mut inc = 1;
        while x >= xs[hi] {
            lo = hi;
            inc *= 2;
            hi = (hi + inc).min(n - 1);
        }
    } else {
        // Hunt downward
        hi = lo;
        let mut inc = 1;
        while x < xs[lo] && lo > 0 {
            hi = lo;
            if inc > lo {
                lo = 0;
            } else {
                lo = lo - inc;
            }
            inc *= 2;
        }
    }

    // Binary search in [lo, hi]
    while hi - lo > 1 {
        let mid = usize::midpoint(lo, hi);
        if x >= xs[mid] {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_search_basic() {
        let xs = [0.0_f64, 1.0, 2.0, 3.0, 4.0];

        assert_eq!(binary_search(&xs, 0.5), 0);
        assert_eq!(binary_search(&xs, 1.5), 1);
        assert_eq!(binary_search(&xs, 2.5), 2);
        assert_eq!(binary_search(&xs, 3.5), 3);
    }

    #[test]
    fn test_binary_search_at_points() {
        let xs = [0.0_f64, 1.0, 2.0, 3.0, 4.0];

        assert_eq!(binary_search(&xs, 0.0), 0);
        assert_eq!(binary_search(&xs, 1.0), 1);
        assert_eq!(binary_search(&xs, 2.0), 2);
        assert_eq!(binary_search(&xs, 3.0), 3);
        assert_eq!(binary_search(&xs, 4.0), 3); // At boundary
    }

    #[test]
    fn test_binary_search_outside_range() {
        let xs = [1.0_f64, 2.0, 3.0];

        assert_eq!(binary_search(&xs, 0.5), 0); // Below range
        assert_eq!(binary_search(&xs, 3.5), 1); // Above range
    }

    #[test]
    fn test_binary_search_two_elements() {
        let xs = [1.0_f64, 2.0];

        assert_eq!(binary_search(&xs, 0.5), 0);
        assert_eq!(binary_search(&xs, 1.5), 0);
        assert_eq!(binary_search(&xs, 2.5), 0);
    }

    #[test]
    fn test_binary_search_single_element() {
        let xs = [1.0_f64];
        assert_eq!(binary_search(&xs, 0.5), 0);
        assert_eq!(binary_search(&xs, 1.5), 0);
    }

    #[test]
    fn test_binary_search_empty() {
        let xs: [f64; 0] = [];
        assert_eq!(binary_search(&xs, 1.0), 0);
    }

    #[test]
    fn test_linear_search_basic() {
        let xs = [0.0_f64, 1.0, 2.0, 3.0];

        assert_eq!(linear_search(&xs, 0.5), 0);
        assert_eq!(linear_search(&xs, 1.5), 1);
        assert_eq!(linear_search(&xs, 2.5), 2);
    }

    #[test]
    fn test_linear_search_boundaries() {
        let xs = [0.0_f64, 1.0, 2.0, 3.0];

        assert_eq!(linear_search(&xs, -0.5), 0);
        assert_eq!(linear_search(&xs, 0.0), 0);
        assert_eq!(linear_search(&xs, 3.0), 2);
        assert_eq!(linear_search(&xs, 3.5), 2);
    }

    #[test]
    fn test_hunt_search_basic() {
        let xs = [0.0_f64, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

        // Start from hint = 0
        assert_eq!(hunt_search(&xs, 3.5, 0), 3);

        // Start from hint = 5, search backward
        assert_eq!(hunt_search(&xs, 2.5, 5), 2);

        // Start from hint = 5, search forward
        assert_eq!(hunt_search(&xs, 7.5, 5), 7);
    }

    #[test]
    fn test_hunt_search_sequential() {
        let xs = [0.0_f64, 1.0, 2.0, 3.0, 4.0, 5.0];

        // Simulate sequential queries with hint from previous result
        let idx1 = hunt_search(&xs, 0.5, 0);
        assert_eq!(idx1, 0);

        let idx2 = hunt_search(&xs, 1.5, idx1);
        assert_eq!(idx2, 1);

        let idx3 = hunt_search(&xs, 2.5, idx2);
        assert_eq!(idx3, 2);
    }

    #[test]
    fn test_hunt_search_boundaries() {
        let xs = [0.0_f64, 1.0, 2.0, 3.0];

        assert_eq!(hunt_search(&xs, -1.0, 1), 0);
        assert_eq!(hunt_search(&xs, 3.5, 1), 2);
    }

    #[test]
    fn test_binary_vs_linear_consistency() {
        let xs = [0.0_f64, 0.5, 1.0, 2.0, 5.0, 10.0];

        for &x in &[0.25, 0.75, 1.5, 3.0, 7.0, 12.0] {
            assert_eq!(
                binary_search(&xs, x),
                linear_search(&xs, x),
                "Mismatch at x={}",
                x
            );
        }
    }

    #[test]
    fn test_all_searches_consistent() {
        let xs = [0.0_f64, 1.0, 2.0, 3.0, 4.0, 5.0];

        for &x in &[0.5, 1.5, 2.5, 3.5, 4.5] {
            let binary = binary_search(&xs, x);
            let linear = linear_search(&xs, x);
            let hunt = hunt_search(&xs, x, 2);

            assert_eq!(binary, linear, "binary vs linear at x={}", x);
            assert_eq!(binary, hunt, "binary vs hunt at x={}", x);
        }
    }
}
