//! Multi-asset simulation with correlated Brownian motions.
//!
//! Provides [`MultiAssetWorkspace`] for storing factor-level simulation data,
//! and [`MultiAssetSimulator`] for generating correlated geometric Brownian
//! motion paths with optional antithetic variance reduction.

use pricer_core::math::rng::PricerRng;

use super::config::XvaSimulationConfig;

/// Workspace for multi-factor simulation data.
///
/// Layout: `data[factor_idx * (n_times * n_paths) + time_idx * n_paths + path_idx]`
///
/// This layout groups all paths for a given factor and time together,
/// enabling efficient vectorised operations over paths.
#[derive(Clone, Debug)]
pub struct MultiAssetWorkspace {
    /// Flat data buffer.
    data: Vec<f64>,
    /// Number of risk factors.
    n_factors: usize,
    /// Number of time steps.
    n_times: usize,
    /// Number of Monte Carlo paths.
    n_paths: usize,
}

impl MultiAssetWorkspace {
    /// Creates a new workspace initialized to zero.
    pub fn new(n_factors: usize, n_times: usize, n_paths: usize) -> Self {
        Self {
            data: vec![0.0; n_factors * n_times * n_paths],
            n_factors,
            n_times,
            n_paths,
        }
    }

    /// Returns the linear index into the flat buffer.
    #[inline]
    fn index(&self, factor_idx: usize, time_idx: usize, path_idx: usize) -> usize {
        debug_assert!(factor_idx < self.n_factors);
        debug_assert!(time_idx < self.n_times);
        debug_assert!(path_idx < self.n_paths);
        factor_idx * (self.n_times * self.n_paths) + time_idx * self.n_paths + path_idx
    }

    /// Gets the value at the given indices.
    #[inline]
    pub fn get(&self, factor_idx: usize, time_idx: usize, path_idx: usize) -> f64 {
        self.data[self.index(factor_idx, time_idx, path_idx)]
    }

    /// Sets the value at the given indices.
    #[inline]
    pub fn set(&mut self, factor_idx: usize, time_idx: usize, path_idx: usize, value: f64) {
        let idx = self.index(factor_idx, time_idx, path_idx);
        self.data[idx] = value;
    }

    /// Returns a slice of all path values for a given factor and time step.
    ///
    /// This is a zero-copy operation returning a contiguous slice of length
    /// `n_paths`.
    #[inline]
    pub fn factor_time_slice(&self, factor_idx: usize, time_idx: usize) -> &[f64] {
        let start = self.index(factor_idx, time_idx, 0);
        &self.data[start..start + self.n_paths]
    }

    /// Returns the number of risk factors.
    #[inline]
    pub fn n_factors(&self) -> usize { self.n_factors }

    /// Returns the number of time steps.
    #[inline]
    pub fn n_times(&self) -> usize { self.n_times }

    /// Returns the number of Monte Carlo paths.
    #[inline]
    pub fn n_paths(&self) -> usize { self.n_paths }
}

/// Simulator for correlated multi-asset geometric Brownian motion paths.
#[derive(Clone, Debug, Default)]
pub struct MultiAssetSimulator;

impl MultiAssetSimulator {
    /// Simulates correlated GBM paths for multiple risk factors.
    ///
    /// Each factor evolves as:
    /// ```text
    /// S(t+dt) = S(t) * exp((drift - 0.5 * vol^2) * dt + vol * sqrt(dt) * dW)
    /// ```
    ///
    /// Correlation between factors is introduced via Cholesky decomposition of
    /// the correlation matrix applied to independent normal draws.
    ///
    /// If `config.antithetic()` is true, paths `0..n_paths/2` use the original
    /// normal draws and paths `n_paths/2..n_paths` use the negated draws.
    ///
    /// # Arguments
    ///
    /// * `config` - Simulation configuration (n_paths, time_grid, antithetic, etc.).
    /// * `n_factors` - Number of correlated risk factors.
    /// * `drift` - Drift rate for each factor (length `n_factors`).
    /// * `vol` - Volatility for each factor (length `n_factors`).
    /// * `correlation` - Correlation matrix (`n_factors x n_factors`).
    /// * `rng` - Random number generator.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `drift`, `vol`, or `correlation` dimensions are inconsistent with `n_factors`.
    /// - The correlation matrix is not square or not of size `n_factors x n_factors`.
    /// - Cholesky decomposition fails (matrix not positive semi-definite).
    pub fn simulate(
        config: &XvaSimulationConfig,
        n_factors: usize,
        drift: &[f64],
        vol: &[f64],
        correlation: &[Vec<f64>],
        rng: &mut PricerRng,
    ) -> MultiAssetWorkspace {
        assert_eq!(drift.len(), n_factors, "drift length must equal n_factors");
        assert_eq!(vol.len(), n_factors, "vol length must equal n_factors");
        assert_eq!(
            correlation.len(),
            n_factors,
            "correlation must have n_factors rows"
        );
        for (i, row) in correlation.iter().enumerate() {
            assert_eq!(
                row.len(),
                n_factors,
                "correlation row {i} must have n_factors columns"
            );
        }

        let n_paths = config.n_paths();
        let time_grid = config.time_grid();
        let n_times = time_grid.len();

        // Compute Cholesky decomposition of the correlation matrix (lower triangular L).
        let cholesky = cholesky_decomposition(correlation, n_factors);

        let mut workspace = MultiAssetWorkspace::new(n_factors, n_times, n_paths);

        // Determine how many independent normal draws we need per time step.
        let base_paths = if config.antithetic() {
            n_paths / 2
        } else {
            n_paths
        };

        for t_idx in 0..n_times {
            let dt = if t_idx == 0 {
                time_grid[0]
            } else {
                time_grid[t_idx] - time_grid[t_idx - 1]
            };
            let sqrt_dt = dt.sqrt();

            // Generate independent normal draws for base paths.
            // independent_normals[factor][base_path]
            let mut independent_normals = vec![vec![0.0; base_paths]; n_factors];
            for f in 0..n_factors {
                for p in 0..base_paths {
                    independent_normals[f][p] = rng.gen_normal();
                }
            }

            // Apply Cholesky to correlate the draws.
            // correlated[factor][base_path]
            let mut correlated = vec![vec![0.0; base_paths]; n_factors];
            for f in 0..n_factors {
                for p in 0..base_paths {
                    let mut sum = 0.0;
                    for k in 0..=f {
                        sum += cholesky[f][k] * independent_normals[k][p];
                    }
                    correlated[f][p] = sum;
                }
            }

            // Evolve each factor.
            for f in 0..n_factors {
                let drift_adj = (drift[f] - 0.5 * vol[f] * vol[f]) * dt;

                for p in 0..base_paths {
                    let prev = if t_idx == 0 {
                        1.0 // Initial spot normalised to 1.0
                    } else {
                        workspace.get(f, t_idx - 1, p)
                    };
                    let dw = vol[f] * sqrt_dt * correlated[f][p];
                    workspace.set(f, t_idx, p, prev * (drift_adj + dw).exp());
                }

                // Antithetic paths.
                if config.antithetic() {
                    for p in 0..base_paths {
                        let anti_p = base_paths + p;
                        if anti_p >= n_paths {
                            break;
                        }
                        let prev = if t_idx == 0 {
                            1.0
                        } else {
                            workspace.get(f, t_idx - 1, anti_p)
                        };
                        let dw = vol[f] * sqrt_dt * (-correlated[f][p]);
                        workspace.set(f, t_idx, anti_p, prev * (drift_adj + dw).exp());
                    }
                }
            }
        }

        workspace
    }
}

/// Computes the Cholesky decomposition (lower triangular) of a symmetric
/// positive semi-definite matrix.
///
/// # Panics
///
/// Panics if the matrix is not positive semi-definite.
fn cholesky_decomposition(matrix: &[Vec<f64>], n: usize) -> Vec<Vec<f64>> {
    let mut lower = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;
            if j == i {
                for k in 0..j {
                    sum += lower[j][k] * lower[j][k];
                }
                let diag = matrix[j][j] - sum;
                assert!(
                    diag >= -1e-12,
                    "correlation matrix is not positive semi-definite (diagonal element {j} = {diag})"
                );
                lower[j][j] = diag.max(0.0).sqrt();
            } else {
                for k in 0..j {
                    sum += lower[i][k] * lower[j][k];
                }
                if lower[j][j].abs() > 1e-15 {
                    lower[i][j] = (matrix[i][j] - sum) / lower[j][j];
                } else {
                    lower[i][j] = 0.0;
                }
            }
        }
    }

    lower
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_workspace_new() {
        let ws = MultiAssetWorkspace::new(2, 3, 4);
        assert_eq!(ws.n_factors(), 2);
        assert_eq!(ws.n_times(), 3);
        assert_eq!(ws.n_paths(), 4);

        for f in 0..2 {
            for t in 0..3 {
                for p in 0..4 {
                    assert_relative_eq!(ws.get(f, t, p), 0.0);
                }
            }
        }
    }

    #[test]
    fn test_workspace_set_and_get() {
        let mut ws = MultiAssetWorkspace::new(2, 3, 4);
        ws.set(1, 2, 3, 42.0);
        assert_relative_eq!(ws.get(1, 2, 3), 42.0);
        assert_relative_eq!(ws.get(0, 2, 3), 0.0);
    }

    #[test]
    fn test_workspace_factor_time_slice() {
        let mut ws = MultiAssetWorkspace::new(2, 3, 4);
        for p in 0..4 {
            ws.set(0, 1, p, (p + 1) as f64);
        }

        let slice = ws.factor_time_slice(0, 1);
        assert_eq!(slice.len(), 4);
        assert_relative_eq!(slice[0], 1.0);
        assert_relative_eq!(slice[3], 4.0);
    }

    #[test]
    fn test_cholesky_identity() {
        let identity = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let l = cholesky_decomposition(&identity, 3);

        for i in 0..3 {
            for j in 0..3 {
                if i == j {
                    assert_relative_eq!(l[i][j], 1.0, epsilon = 1e-12);
                } else {
                    assert_relative_eq!(l[i][j], 0.0, epsilon = 1e-12);
                }
            }
        }
    }

    #[test]
    fn test_cholesky_correlated() {
        let corr = vec![
            vec![1.0, 0.5],
            vec![0.5, 1.0],
        ];
        let l = cholesky_decomposition(&corr, 2);

        // L * L^T should reconstruct the correlation matrix.
        for i in 0..2 {
            for j in 0..2 {
                let mut sum = 0.0;
                for k in 0..2 {
                    sum += l[i][k] * l[j][k];
                }
                assert_relative_eq!(sum, corr[i][j], epsilon = 1e-12);
            }
        }
    }

    #[test]
    fn test_simulate_single_factor_deterministic() {
        let config = XvaSimulationConfig::builder()
            .n_paths(100)
            .time_grid(vec![0.25, 0.5, 1.0])
            .seed(42)
            .build()
            .unwrap();

        let drift = vec![0.05];
        let vol = vec![0.2];
        let correlation = vec![vec![1.0]];

        let mut rng = PricerRng::from_seed(config.seed().unwrap());
        let ws = MultiAssetSimulator::simulate(&config, 1, &drift, &vol, &correlation, &mut rng);

        assert_eq!(ws.n_factors(), 1);
        assert_eq!(ws.n_times(), 3);
        assert_eq!(ws.n_paths(), 100);

        // All values should be positive (GBM).
        for t in 0..3 {
            for p in 0..100 {
                assert!(ws.get(0, t, p) > 0.0, "GBM path values must be positive");
            }
        }
    }

    #[test]
    fn test_simulate_reproducible_with_seed() {
        let config = XvaSimulationConfig::builder()
            .n_paths(50)
            .time_grid(vec![0.5, 1.0])
            .seed(123)
            .build()
            .unwrap();

        let drift = vec![0.03];
        let vol = vec![0.15];
        let correlation = vec![vec![1.0]];

        let mut rng1 = PricerRng::from_seed(123);
        let ws1 = MultiAssetSimulator::simulate(&config, 1, &drift, &vol, &correlation, &mut rng1);

        let mut rng2 = PricerRng::from_seed(123);
        let ws2 = MultiAssetSimulator::simulate(&config, 1, &drift, &vol, &correlation, &mut rng2);

        for t in 0..2 {
            for p in 0..50 {
                assert_relative_eq!(ws1.get(0, t, p), ws2.get(0, t, p));
            }
        }
    }

    #[test]
    fn test_simulate_with_antithetic() {
        let config = XvaSimulationConfig::builder()
            .n_paths(100)
            .time_grid(vec![0.25, 0.5])
            .seed(99)
            .antithetic(true)
            .build()
            .unwrap();

        let drift = vec![0.05];
        let vol = vec![0.2];
        let correlation = vec![vec![1.0]];

        let mut rng = PricerRng::from_seed(99);
        let ws = MultiAssetSimulator::simulate(&config, 1, &drift, &vol, &correlation, &mut rng);

        assert_eq!(ws.n_paths(), 100);
        // All paths should be positive.
        for t in 0..2 {
            for p in 0..100 {
                assert!(ws.get(0, t, p) > 0.0);
            }
        }
    }

    #[test]
    fn test_simulate_two_factors_correlated() {
        let config = XvaSimulationConfig::builder()
            .n_paths(200)
            .time_grid(vec![0.5, 1.0])
            .seed(77)
            .build()
            .unwrap();

        let drift = vec![0.03, 0.05];
        let vol = vec![0.2, 0.3];
        let correlation = vec![
            vec![1.0, 0.6],
            vec![0.6, 1.0],
        ];

        let mut rng = PricerRng::from_seed(77);
        let ws =
            MultiAssetSimulator::simulate(&config, 2, &drift, &vol, &correlation, &mut rng);

        assert_eq!(ws.n_factors(), 2);
        assert_eq!(ws.n_paths(), 200);

        // Check that log-returns of the two factors are positively correlated.
        let t_idx = 1; // Final time step.
        let mut log_returns_0 = Vec::new();
        let mut log_returns_1 = Vec::new();
        for p in 0..200 {
            log_returns_0.push(ws.get(0, t_idx, p).ln());
            log_returns_1.push(ws.get(1, t_idx, p).ln());
        }

        let mean_0: f64 = log_returns_0.iter().sum::<f64>() / 200.0;
        let mean_1: f64 = log_returns_1.iter().sum::<f64>() / 200.0;

        let mut cov = 0.0;
        let mut var_0 = 0.0;
        let mut var_1 = 0.0;
        for i in 0..200 {
            let d0 = log_returns_0[i] - mean_0;
            let d1 = log_returns_1[i] - mean_1;
            cov += d0 * d1;
            var_0 += d0 * d0;
            var_1 += d1 * d1;
        }
        let sample_corr = cov / (var_0 * var_1).sqrt();

        // With 200 paths the sample correlation should be roughly positive.
        assert!(
            sample_corr > 0.0,
            "expected positive sample correlation, got {sample_corr}"
        );
    }

    #[test]
    #[should_panic(expected = "drift length must equal n_factors")]
    fn test_simulate_drift_length_mismatch_panics() {
        let config = XvaSimulationConfig::builder()
            .n_paths(10)
            .time_grid(vec![0.5])
            .build()
            .unwrap();

        let mut rng = PricerRng::from_seed(1);
        MultiAssetSimulator::simulate(&config, 2, &[0.05], &[0.2, 0.3], &[vec![1.0, 0.0], vec![0.0, 1.0]], &mut rng);
    }

    #[test]
    #[should_panic(expected = "vol length must equal n_factors")]
    fn test_simulate_vol_length_mismatch_panics() {
        let config = XvaSimulationConfig::builder()
            .n_paths(10)
            .time_grid(vec![0.5])
            .build()
            .unwrap();

        let mut rng = PricerRng::from_seed(1);
        MultiAssetSimulator::simulate(&config, 2, &[0.05, 0.03], &[0.2], &[vec![1.0, 0.0], vec![0.0, 1.0]], &mut rng);
    }

    #[test]
    fn test_simulate_zero_drift_zero_vol() {
        // With zero vol, all paths should follow the drift exactly.
        let config = XvaSimulationConfig::builder()
            .n_paths(10)
            .time_grid(vec![1.0])
            .seed(42)
            .build()
            .unwrap();

        let drift = vec![0.0];
        let vol = vec![0.0];
        let correlation = vec![vec![1.0]];

        let mut rng = PricerRng::from_seed(42);
        let ws = MultiAssetSimulator::simulate(&config, 1, &drift, &vol, &correlation, &mut rng);

        // S(1) = 1.0 * exp(0) = 1.0 for all paths.
        for p in 0..10 {
            assert_relative_eq!(ws.get(0, 0, p), 1.0, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_simulate_pure_drift_no_vol() {
        let config = XvaSimulationConfig::builder()
            .n_paths(5)
            .time_grid(vec![0.5, 1.0])
            .seed(42)
            .build()
            .unwrap();

        let drift = vec![0.1];
        let vol = vec![0.0];
        let correlation = vec![vec![1.0]];

        let mut rng = PricerRng::from_seed(42);
        let ws = MultiAssetSimulator::simulate(&config, 1, &drift, &vol, &correlation, &mut rng);

        // At t=0.5: exp(0.1 * 0.5) = exp(0.05)
        let expected_05 = (0.1 * 0.5_f64).exp();
        // At t=1.0: exp(0.1 * 1.0) = exp(0.1)
        let expected_10 = (0.1_f64).exp();

        for p in 0..5 {
            assert_relative_eq!(ws.get(0, 0, p), expected_05, epsilon = 1e-10);
            assert_relative_eq!(ws.get(0, 1, p), expected_10, epsilon = 1e-10);
        }
    }
}
