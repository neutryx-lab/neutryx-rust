//! Numeraire ratio computation for Monte Carlo XVA simulations.
//!
//! Provides [`NumeraireRatios`] for storing and computing the inverse
//! numeraire values across simulation time steps and paths, used when
//! switching between pricing measures.

/// Numeraire ratios indexed by `[time][path]`.
///
/// Each entry stores `1.0 / N(t, path)` where `N` is the chosen numeraire.
/// For the risk-neutral measure, the numeraire is the money-market account.
#[derive(Clone, Debug)]
pub struct NumeraireRatios {
    /// Flat storage in row-major order: `ratios[t * n_paths + p]`.
    ratios: Vec<f64>,
    /// Number of time steps.
    n_times: usize,
    /// Number of paths.
    n_paths: usize,
}

impl NumeraireRatios {
    /// Creates a new `NumeraireRatios` initialized to 1.0 for all entries.
    pub fn new(n_times: usize, n_paths: usize) -> Self {
        Self {
            ratios: vec![1.0; n_times * n_paths],
            n_times,
            n_paths,
        }
    }

    /// Computes numeraire ratios from discount factor paths.
    ///
    /// `discount_paths` is indexed as `discount_paths[time_idx][path_idx]`,
    /// where each value is the discount factor `D(0, t)` for that path.
    /// The numeraire ratio at each point is `1.0 / D(0, t)` (i.e., the
    /// money-market account value).
    ///
    /// # Panics
    ///
    /// Panics if any discount factor is zero.
    pub fn compute_from_discount_paths(discount_paths: &[Vec<f64>]) -> Self {
        if discount_paths.is_empty() {
            return Self {
                ratios: Vec::new(),
                n_times: 0,
                n_paths: 0,
            };
        }

        let n_times = discount_paths.len();
        let n_paths = discount_paths[0].len();
        let mut ratios = Vec::with_capacity(n_times * n_paths);

        for time_slice in discount_paths {
            assert_eq!(
                time_slice.len(),
                n_paths,
                "all time slices must have the same number of paths"
            );
            for &df in time_slice {
                assert!(df != 0.0, "discount factor must not be zero");
                ratios.push(1.0 / df);
            }
        }

        Self {
            ratios,
            n_times,
            n_paths,
        }
    }

    /// Returns the numeraire ratio at a given time and path index.
    ///
    /// # Panics
    ///
    /// Panics if indices are out of bounds.
    #[inline]
    pub fn ratio(&self, time_idx: usize, path_idx: usize) -> f64 {
        self.ratios[time_idx * self.n_paths + path_idx]
    }

    /// Returns the number of time steps.
    #[inline]
    pub fn n_times(&self) -> usize { self.n_times }

    /// Returns the number of paths.
    #[inline]
    pub fn n_paths(&self) -> usize { self.n_paths }

    /// Creates numeraire ratios from a flat (deterministic) rate.
    ///
    /// The discount factor at time `t` is `exp(-rate * t)`, so the numeraire
    /// ratio is `exp(rate * t)`.
    pub fn from_flat_rate(rate: f64, time_grid: &[f64], n_paths: usize) -> Self {
        let n_times = time_grid.len();
        let mut ratios = Vec::with_capacity(n_times * n_paths);

        for &t in time_grid {
            let ratio = (rate * t).exp();
            for _ in 0..n_paths {
                ratios.push(ratio);
            }
        }

        Self {
            ratios,
            n_times,
            n_paths,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_new_initialized_to_one() {
        let nr = NumeraireRatios::new(3, 4);
        assert_eq!(nr.n_times(), 3);
        assert_eq!(nr.n_paths(), 4);

        for t in 0..3 {
            for p in 0..4 {
                assert_relative_eq!(nr.ratio(t, p), 1.0);
            }
        }
    }

    #[test]
    fn test_compute_from_discount_paths() {
        // Two time steps, three paths.
        let discount_paths = vec![
            vec![0.99, 0.98, 0.97],  // t=0
            vec![0.96, 0.95, 0.94],  // t=1
        ];

        let nr = NumeraireRatios::compute_from_discount_paths(&discount_paths);
        assert_eq!(nr.n_times(), 2);
        assert_eq!(nr.n_paths(), 3);

        assert_relative_eq!(nr.ratio(0, 0), 1.0 / 0.99, epsilon = 1e-12);
        assert_relative_eq!(nr.ratio(0, 2), 1.0 / 0.97, epsilon = 1e-12);
        assert_relative_eq!(nr.ratio(1, 1), 1.0 / 0.95, epsilon = 1e-12);
    }

    #[test]
    fn test_compute_from_empty_discount_paths() {
        let nr = NumeraireRatios::compute_from_discount_paths(&[]);
        assert_eq!(nr.n_times(), 0);
        assert_eq!(nr.n_paths(), 0);
    }

    #[test]
    #[should_panic(expected = "discount factor must not be zero")]
    fn test_compute_from_zero_discount_factor_panics() {
        let discount_paths = vec![vec![0.99, 0.0, 0.97]];
        NumeraireRatios::compute_from_discount_paths(&discount_paths);
    }

    #[test]
    fn test_from_flat_rate() {
        let rate = 0.05;
        let time_grid = vec![0.25, 0.5, 1.0];
        let n_paths = 2;

        let nr = NumeraireRatios::from_flat_rate(rate, &time_grid, n_paths);
        assert_eq!(nr.n_times(), 3);
        assert_eq!(nr.n_paths(), 2);

        // At t=0.25: exp(0.05 * 0.25) = exp(0.0125)
        assert_relative_eq!(nr.ratio(0, 0), (0.05 * 0.25_f64).exp(), epsilon = 1e-12);
        assert_relative_eq!(nr.ratio(0, 1), (0.05 * 0.25_f64).exp(), epsilon = 1e-12);

        // At t=1.0: exp(0.05 * 1.0) = exp(0.05)
        assert_relative_eq!(nr.ratio(2, 0), (0.05_f64).exp(), epsilon = 1e-12);
    }

    #[test]
    fn test_from_flat_rate_zero() {
        let nr = NumeraireRatios::from_flat_rate(0.0, &[0.5, 1.0], 3);
        for t in 0..2 {
            for p in 0..3 {
                assert_relative_eq!(nr.ratio(t, p), 1.0, epsilon = 1e-12);
            }
        }
    }

    #[test]
    fn test_flat_rate_consistent_with_discount_paths() {
        let rate = 0.03;
        let time_grid = vec![0.5, 1.0, 2.0];
        let n_paths = 4;

        let from_rate = NumeraireRatios::from_flat_rate(rate, &time_grid, n_paths);

        // Build equivalent discount paths.
        let discount_paths: Vec<Vec<f64>> = time_grid
            .iter()
            .map(|&t| vec![(-rate * t).exp(); n_paths])
            .collect();
        let from_discount = NumeraireRatios::compute_from_discount_paths(&discount_paths);

        for t in 0..3 {
            for p in 0..n_paths {
                assert_relative_eq!(
                    from_rate.ratio(t, p),
                    from_discount.ratio(t, p),
                    epsilon = 1e-10
                );
            }
        }
    }
}
