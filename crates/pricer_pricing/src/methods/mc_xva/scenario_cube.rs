//! Scenario cube for storing XVA exposure profiles.
//!
//! Provides [`ScenarioCube`], a three-dimensional data structure indexed by
//! netting set, time step, and Monte Carlo path. Uses a flat `Vec<f64>`
//! buffer for cache-friendly access.

/// A three-dimensional cube storing simulated exposures.
///
/// Layout: `data[ns_idx * (n_times * n_paths) + time_idx * n_paths + path_idx]`
///
/// This enables efficient access to all paths at a given time for a netting set
/// (a contiguous slice), which is the most common access pattern in XVA
/// calculations.
#[derive(Clone, Debug)]
pub struct ScenarioCube {
    /// Flat data buffer.
    data: Vec<f64>,
    /// Number of netting sets.
    n_netting_sets: usize,
    /// Number of time steps.
    n_times: usize,
    /// Number of Monte Carlo paths.
    n_paths: usize,
    /// Netting set identifiers (String-based since pricer_pricing does not
    /// depend on pricer_risk).
    netting_set_ids: Vec<String>,
}

impl ScenarioCube {
    /// Creates a new scenario cube initialized to zero.
    ///
    /// # Panics
    ///
    /// Panics if `netting_set_ids.len() != n_netting_sets`.
    pub fn new(
        n_netting_sets: usize,
        n_times: usize,
        n_paths: usize,
        netting_set_ids: Vec<String>,
    ) -> Self {
        assert_eq!(
            netting_set_ids.len(),
            n_netting_sets,
            "netting_set_ids length ({}) must equal n_netting_sets ({n_netting_sets})",
            netting_set_ids.len()
        );

        Self {
            data: vec![0.0; n_netting_sets * n_times * n_paths],
            n_netting_sets,
            n_times,
            n_paths,
            netting_set_ids,
        }
    }

    /// Returns the linear index into the flat buffer.
    #[inline]
    fn index(&self, ns_idx: usize, time_idx: usize, path_idx: usize) -> usize {
        debug_assert!(ns_idx < self.n_netting_sets);
        debug_assert!(time_idx < self.n_times);
        debug_assert!(path_idx < self.n_paths);
        ns_idx * (self.n_times * self.n_paths) + time_idx * self.n_paths + path_idx
    }

    /// Gets the value at the given indices.
    ///
    /// # Panics
    ///
    /// Panics if any index is out of bounds.
    #[inline]
    pub fn get(&self, ns_idx: usize, time_idx: usize, path_idx: usize) -> f64 {
        self.data[self.index(ns_idx, time_idx, path_idx)]
    }

    /// Sets the value at the given indices.
    ///
    /// # Panics
    ///
    /// Panics if any index is out of bounds.
    #[inline]
    pub fn set(&mut self, ns_idx: usize, time_idx: usize, path_idx: usize, value: f64) {
        let idx = self.index(ns_idx, time_idx, path_idx);
        self.data[idx] = value;
    }

    /// Returns a slice of all path values at a given time for a netting set.
    ///
    /// This is a zero-copy operation returning a contiguous slice of length
    /// `n_paths`.
    ///
    /// # Panics
    ///
    /// Panics if `ns_idx >= n_netting_sets` or `time_idx >= n_times`.
    #[inline]
    pub fn time_slice(&self, ns_idx: usize, time_idx: usize) -> &[f64] {
        let start = self.index(ns_idx, time_idx, 0);
        &self.data[start..start + self.n_paths]
    }

    /// Returns all time-step values for a given path within a netting set.
    ///
    /// This allocates a new `Vec<f64>` because the values are not contiguous
    /// in memory.
    pub fn path_values(&self, ns_idx: usize, path_idx: usize) -> Vec<f64> {
        (0..self.n_times)
            .map(|t| self.get(ns_idx, t, path_idx))
            .collect()
    }

    /// Returns the number of netting sets.
    #[inline]
    pub fn n_netting_sets(&self) -> usize { self.n_netting_sets }

    /// Returns the number of time steps.
    #[inline]
    pub fn n_times(&self) -> usize { self.n_times }

    /// Returns the number of Monte Carlo paths.
    #[inline]
    pub fn n_paths(&self) -> usize { self.n_paths }

    /// Returns the netting set identifiers.
    #[inline]
    pub fn netting_set_ids(&self) -> &[String] { &self.netting_set_ids }

    /// Finds the index of a netting set by its identifier.
    ///
    /// Returns `None` if the identifier is not found.
    pub fn find_netting_set(&self, id: &str) -> Option<usize> {
        self.netting_set_ids.iter().position(|ns| ns == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn sample_cube() -> ScenarioCube {
        ScenarioCube::new(
            2,
            3,
            4,
            vec!["NS_001".to_string(), "NS_002".to_string()],
        )
    }

    #[test]
    fn test_new_initialized_to_zero() {
        let cube = sample_cube();
        assert_eq!(cube.n_netting_sets(), 2);
        assert_eq!(cube.n_times(), 3);
        assert_eq!(cube.n_paths(), 4);

        for ns in 0..2 {
            for t in 0..3 {
                for p in 0..4 {
                    assert_relative_eq!(cube.get(ns, t, p), 0.0);
                }
            }
        }
    }

    #[test]
    fn test_set_and_get() {
        let mut cube = sample_cube();
        cube.set(0, 1, 2, 42.5);
        assert_relative_eq!(cube.get(0, 1, 2), 42.5);

        // Other values remain zero.
        assert_relative_eq!(cube.get(0, 1, 0), 0.0);
        assert_relative_eq!(cube.get(1, 1, 2), 0.0);
    }

    #[test]
    fn test_time_slice() {
        let mut cube = sample_cube();
        for p in 0..4 {
            cube.set(0, 1, p, (p + 1) as f64 * 10.0);
        }

        let slice = cube.time_slice(0, 1);
        assert_eq!(slice.len(), 4);
        assert_relative_eq!(slice[0], 10.0);
        assert_relative_eq!(slice[1], 20.0);
        assert_relative_eq!(slice[2], 30.0);
        assert_relative_eq!(slice[3], 40.0);
    }

    #[test]
    fn test_path_values() {
        let mut cube = sample_cube();
        for t in 0..3 {
            cube.set(1, t, 2, (t + 1) as f64 * 100.0);
        }

        let pv = cube.path_values(1, 2);
        assert_eq!(pv.len(), 3);
        assert_relative_eq!(pv[0], 100.0);
        assert_relative_eq!(pv[1], 200.0);
        assert_relative_eq!(pv[2], 300.0);
    }

    #[test]
    fn test_netting_set_ids() {
        let cube = sample_cube();
        assert_eq!(cube.netting_set_ids(), &["NS_001", "NS_002"]);
    }

    #[test]
    fn test_find_netting_set() {
        let cube = sample_cube();
        assert_eq!(cube.find_netting_set("NS_001"), Some(0));
        assert_eq!(cube.find_netting_set("NS_002"), Some(1));
        assert_eq!(cube.find_netting_set("NS_003"), None);
    }

    #[test]
    #[should_panic(expected = "netting_set_ids length")]
    fn test_new_mismatched_ids_panics() {
        ScenarioCube::new(2, 3, 4, vec!["NS_001".to_string()]);
    }

    #[test]
    fn test_zero_dimensions() {
        let cube = ScenarioCube::new(0, 0, 0, vec![]);
        assert_eq!(cube.n_netting_sets(), 0);
        assert_eq!(cube.n_times(), 0);
        assert_eq!(cube.n_paths(), 0);
    }

    #[test]
    fn test_independence_between_netting_sets() {
        let mut cube = sample_cube();
        cube.set(0, 0, 0, 1.0);
        cube.set(1, 0, 0, 2.0);

        assert_relative_eq!(cube.get(0, 0, 0), 1.0);
        assert_relative_eq!(cube.get(1, 0, 0), 2.0);
    }

    #[test]
    fn test_time_slice_is_contiguous() {
        let mut cube = ScenarioCube::new(1, 2, 5, vec!["NS_A".to_string()]);
        for p in 0..5 {
            cube.set(0, 0, p, p as f64);
        }

        let slice = cube.time_slice(0, 0);
        // Verify we get a proper slice (contiguous in memory).
        assert_eq!(slice.len(), 5);
        for (i, &val) in slice.iter().enumerate() {
            assert_relative_eq!(val, i as f64);
        }
    }

    #[test]
    fn test_large_cube_addressing() {
        let n_ns = 3;
        let n_t = 10;
        let n_p = 100;
        let ids: Vec<String> = (0..n_ns).map(|i| format!("NS_{i:03}")).collect();
        let mut cube = ScenarioCube::new(n_ns, n_t, n_p, ids);

        // Write a unique value to each cell.
        for ns in 0..n_ns {
            for t in 0..n_t {
                for p in 0..n_p {
                    let val = (ns * 10_000 + t * 100 + p) as f64;
                    cube.set(ns, t, p, val);
                }
            }
        }

        // Verify all values.
        for ns in 0..n_ns {
            for t in 0..n_t {
                for p in 0..n_p {
                    let expected = (ns * 10_000 + t * 100 + p) as f64;
                    assert_relative_eq!(cube.get(ns, t, p), expected);
                }
            }
        }
    }
}
