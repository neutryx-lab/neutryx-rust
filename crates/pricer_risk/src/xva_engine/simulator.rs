//! Full-valuation Monte Carlo XVA simulation engine.
//!
//! Generates multi-asset Monte Carlo paths with correlated factors,
//! supports antithetic variates, and computes exposure profiles.

use std::collections::HashMap;

use pricer_core::math::rng::PricerRng;

use super::{
    aggregator::ExposureAggregator, config::XvaEngineConfig, error::XvaEngineError,
    risk_indicators::XvaRiskIndicators,
};
use crate::portfolio::NettingSetId;

/// Results of an XVA Monte Carlo simulation.
#[derive(Clone, Debug)]
pub struct XvaSimulationResult {
    /// Time grid used in the simulation.
    pub time_grid: Vec<f64>,
    /// Number of paths simulated.
    pub n_paths: usize,
    /// Netted trade values per netting set, indexed as [time_idx][path_idx].
    pub netted_values: HashMap<NettingSetId, Vec<Vec<f64>>>,
    /// Expected Positive Exposure profiles per netting set.
    pub epe_profiles: HashMap<NettingSetId, Vec<f64>>,
    /// Expected Negative Exposure profiles per netting set.
    pub ene_profiles: HashMap<NettingSetId, Vec<f64>>,
    /// Risk indicator profiles per netting set.
    pub risk_indicators: HashMap<NettingSetId, XvaRiskIndicators>,
}

/// Full-valuation Monte Carlo XVA simulator.
///
/// Generates correlated multi-asset paths and computes exposure profiles
/// for each netting set in the portfolio hierarchy.
pub struct XvaSimulator {
    config: XvaEngineConfig,
}

impl XvaSimulator {
    /// Creates a new simulator with the given configuration.
    pub fn new(config: XvaEngineConfig) -> Self { Self { config } }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &XvaEngineConfig { &self.config }

    /// Generates multi-asset Monte Carlo paths.
    ///
    /// Returns `result[factor][time][path]` -- a 3D array of simulated values.
    ///
    /// Uses Cholesky decomposition of the correlation matrix to generate
    /// correlated factors. If `antithetic` is enabled, generates both original
    /// and negated paths (doubling the effective number of paths).
    ///
    /// # Arguments
    /// * `n_factors` - Number of risk factors (assets)
    /// * `drift` - Drift per factor (length = n_factors)
    /// * `vol` - Volatility per factor (length = n_factors)
    /// * `correlation` - Correlation matrix (n_factors x n_factors)
    pub fn simulate_paths(
        &self,
        n_factors: usize,
        drift: &[f64],
        vol: &[f64],
        correlation: &[Vec<f64>],
    ) -> Result<Vec<Vec<Vec<f64>>>, XvaEngineError> {
        // Validate dimensions
        if drift.len() != n_factors {
            return Err(XvaEngineError::DimensionMismatch {
                expected: n_factors,
                actual: drift.len(),
            });
        }
        if vol.len() != n_factors {
            return Err(XvaEngineError::DimensionMismatch {
                expected: n_factors,
                actual: vol.len(),
            });
        }
        if correlation.len() != n_factors {
            return Err(XvaEngineError::DimensionMismatch {
                expected: n_factors,
                actual: correlation.len(),
            });
        }
        for row in correlation {
            if row.len() != n_factors {
                return Err(XvaEngineError::DimensionMismatch {
                    expected: n_factors,
                    actual: row.len(),
                });
            }
        }

        let n_times = self.config.time_grid.len();
        let n_paths = self.config.n_paths;
        let seed = self.config.seed.unwrap_or(42);

        // Cholesky decomposition of correlation matrix
        let cholesky = cholesky_decompose(correlation, n_factors)?;

        let mut rng = PricerRng::from_seed(seed);

        // Determine actual paths to generate
        let gen_paths = if self.config.antithetic {
            n_paths / 2
        } else {
            n_paths
        };

        // Initialize result: [factor][time][path]
        let total_paths = if self.config.antithetic {
            gen_paths * 2
        } else {
            gen_paths
        };
        let mut result = vec![vec![vec![0.0; total_paths]; n_times]; n_factors];

        // Generate paths
        for p in 0..gen_paths {
            // Start from S0 = 1.0 for each factor
            let mut current = vec![1.0_f64; n_factors];

            for t in 0..n_times {
                let dt = if t == 0 {
                    self.config.time_grid[0]
                } else {
                    self.config.time_grid[t] - self.config.time_grid[t - 1]
                };

                let sqrt_dt = dt.sqrt();

                // Generate independent normals
                let mut z = vec![0.0; n_factors];
                for z_i in z.iter_mut() {
                    *z_i = rng.gen_normal();
                }

                // Apply Cholesky to correlate
                let mut corr_z = vec![0.0; n_factors];
                for i in 0..n_factors {
                    for j in 0..=i {
                        corr_z[i] += cholesky[i][j] * z[j];
                    }
                }

                // Update each factor using GBM dynamics
                for f in 0..n_factors {
                    let log_increment =
                        (drift[f] - 0.5 * vol[f] * vol[f]) * dt + vol[f] * sqrt_dt * corr_z[f];
                    current[f] *= log_increment.exp();
                    result[f][t][p] = current[f];
                }

                // Antithetic paths
                if self.config.antithetic {
                    let mut anti_current = if t == 0 {
                        vec![1.0_f64; n_factors]
                    } else {
                        // Reconstruct from previous antithetic values
                        (0..n_factors)
                            .map(|f| result[f][t - 1][gen_paths + p])
                            .collect()
                    };

                    for f in 0..n_factors {
                        let log_increment = (drift[f] - 0.5 * vol[f] * vol[f]) * dt
                            + vol[f] * sqrt_dt * (-corr_z[f]);
                        anti_current[f] *= log_increment.exp();
                        result[f][t][gen_paths + p] = anti_current[f];
                    }
                }
            }
        }

        Ok(result)
    }

    /// Computes exposure profiles for all netting sets.
    ///
    /// Returns (epe_profiles, ene_profiles) as HashMaps keyed by NettingSetId.
    ///
    /// EPE[t] = mean(max(V[t][p], 0)) over all paths p
    /// ENE[t] = mean(max(-V[t][p], 0)) over all paths p
    pub fn compute_exposure_profiles(
        &self,
        netted_values: &HashMap<NettingSetId, Vec<Vec<f64>>>,
    ) -> (
        HashMap<NettingSetId, Vec<f64>>,
        HashMap<NettingSetId, Vec<f64>>,
    ) {
        let mut epe_profiles = HashMap::new();
        let mut ene_profiles = HashMap::new();

        for (ns_id, values) in netted_values {
            let epe = ExposureAggregator::compute_epe(values);
            let ene = ExposureAggregator::compute_ene(values);
            epe_profiles.insert(ns_id.clone(), epe);
            ene_profiles.insert(ns_id.clone(), ene);
        }

        (epe_profiles, ene_profiles)
    }
}

/// Performs Cholesky decomposition of a symmetric positive-definite matrix.
///
/// Returns the lower-triangular matrix L such that A = L * L^T.
fn cholesky_decompose(matrix: &[Vec<f64>], n: usize) -> Result<Vec<Vec<f64>>, XvaEngineError> {
    let mut l = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;
            for k in 0..j {
                sum += l[i][k] * l[j][k];
            }

            if i == j {
                let diag = matrix[i][i] - sum;
                if diag < 0.0 {
                    return Err(XvaEngineError::SimulationError(
                        "Correlation matrix is not positive definite".to_string(),
                    ));
                }
                l[i][j] = diag.sqrt();
            } else {
                if l[j][j].abs() < f64::EPSILON {
                    return Err(XvaEngineError::SimulationError(
                        "Zero diagonal in Cholesky decomposition".to_string(),
                    ));
                }
                l[i][j] = (matrix[i][j] - sum) / l[j][j];
            }
        }
    }

    Ok(l)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn make_config(n_paths: usize, seed: u64) -> XvaEngineConfig {
        XvaEngineConfig {
            n_paths,
            time_grid: vec![0.25, 0.5, 0.75, 1.0],
            seed: Some(seed),
            antithetic: false,
            pfe_percentiles: vec![0.95],
            bilateral: true,
            compute_fva: false,
            compute_ecb: false,
        }
    }

    #[test]
    fn test_cholesky_identity() {
        let identity = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let l = cholesky_decompose(&identity, 2).unwrap();
        assert_relative_eq!(l[0][0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(l[1][1], 1.0, epsilon = 1e-10);
        assert_relative_eq!(l[1][0], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cholesky_correlated() {
        let corr = vec![vec![1.0, 0.5], vec![0.5, 1.0]];
        let l = cholesky_decompose(&corr, 2).unwrap();

        // Verify L * L^T = corr
        for i in 0..2 {
            for j in 0..2 {
                let mut val = 0.0;
                for k in 0..2 {
                    val += l[i][k] * l[j][k];
                }
                assert_relative_eq!(val, corr[i][j], epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_cholesky_not_positive_definite() {
        let bad = vec![vec![1.0, 2.0], vec![2.0, 1.0]];
        let result = cholesky_decompose(&bad, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_simulate_paths_1_factor() {
        let config = make_config(100, 42);
        let sim = XvaSimulator::new(config);

        let drift = vec![0.05];
        let vol = vec![0.2];
        let corr = vec![vec![1.0]];

        let paths = sim.simulate_paths(1, &drift, &vol, &corr).unwrap();

        assert_eq!(paths.len(), 1); // 1 factor
        assert_eq!(paths[0].len(), 4); // 4 time points
        assert_eq!(paths[0][0].len(), 100); // 100 paths

        // All values should be positive (log-normal)
        for t in 0..4 {
            for p in 0..100 {
                assert!(paths[0][t][p] > 0.0, "Path value must be positive");
            }
        }
    }

    #[test]
    fn test_simulate_paths_reproducible() {
        let config1 = make_config(50, 123);
        let config2 = make_config(50, 123);
        let sim1 = XvaSimulator::new(config1);
        let sim2 = XvaSimulator::new(config2);

        let drift = vec![0.05];
        let vol = vec![0.2];
        let corr = vec![vec![1.0]];

        let paths1 = sim1.simulate_paths(1, &drift, &vol, &corr).unwrap();
        let paths2 = sim2.simulate_paths(1, &drift, &vol, &corr).unwrap();

        for t in 0..4 {
            for p in 0..50 {
                assert_relative_eq!(paths1[0][t][p], paths2[0][t][p], epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_simulate_paths_dimension_mismatch() {
        let config = make_config(10, 42);
        let sim = XvaSimulator::new(config);

        // Wrong drift length
        let result = sim.simulate_paths(2, &[0.05], &[0.2, 0.3], &[vec![1.0, 0.0], vec![0.0, 1.0]]);
        assert!(result.is_err());

        // Wrong vol length
        let result =
            sim.simulate_paths(2, &[0.05, 0.05], &[0.2], &[vec![1.0, 0.0], vec![0.0, 1.0]]);
        assert!(result.is_err());

        // Wrong correlation dimensions
        let result = sim.simulate_paths(2, &[0.05, 0.05], &[0.2, 0.3], &[vec![1.0]]);
        assert!(result.is_err());
    }

    #[test]
    fn test_simulate_paths_antithetic() {
        let mut config = make_config(100, 42);
        config.antithetic = true;
        let sim = XvaSimulator::new(config);

        let drift = vec![0.05];
        let vol = vec![0.2];
        let corr = vec![vec![1.0]];

        let paths = sim.simulate_paths(1, &drift, &vol, &corr).unwrap();

        // Should have 100 total paths (50 original + 50 antithetic)
        assert_eq!(paths[0][0].len(), 100);
    }

    #[test]
    fn test_simulate_paths_2_factors() {
        let config = make_config(50, 42);
        let sim = XvaSimulator::new(config);

        let drift = vec![0.03, 0.05];
        let vol = vec![0.15, 0.25];
        let corr = vec![vec![1.0, 0.3], vec![0.3, 1.0]];

        let paths = sim.simulate_paths(2, &drift, &vol, &corr).unwrap();

        assert_eq!(paths.len(), 2); // 2 factors
        assert_eq!(paths[0].len(), 4); // 4 time points
        assert_eq!(paths[0][0].len(), 50); // 50 paths

        // All values positive
        for f in 0..2 {
            for t in 0..4 {
                for p in 0..50 {
                    assert!(paths[f][t][p] > 0.0);
                }
            }
        }
    }

    #[test]
    fn test_compute_exposure_profiles() {
        let config = make_config(4, 42);
        let sim = XvaSimulator::new(config);

        let mut netted = HashMap::new();
        netted.insert(
            NettingSetId::new("NS1"),
            vec![
                vec![1.0, -2.0, 3.0, -4.0], // t=0
                vec![5.0, -1.0, 2.0, -3.0], // t=1
            ],
        );

        let (epe, ene) = sim.compute_exposure_profiles(&netted);

        let epe_ns1 = epe.get(&NettingSetId::new("NS1")).unwrap();
        let ene_ns1 = ene.get(&NettingSetId::new("NS1")).unwrap();

        // EPE[0] = (1+0+3+0)/4 = 1.0
        assert_relative_eq!(epe_ns1[0], 1.0, epsilon = 1e-10);
        // ENE[0] = (0+2+0+4)/4 = 1.5
        assert_relative_eq!(ene_ns1[0], 1.5, epsilon = 1e-10);
    }

    #[test]
    fn test_gbm_mean_close_to_drift() {
        // With many paths, the mean of GBM should be close to E[S_T] = S_0 * exp(mu *
        // T)
        let config = make_config(10_000, 42);
        let sim = XvaSimulator::new(config);

        let mu = 0.05;
        let sigma = 0.2;
        let drift = vec![mu];
        let vol = vec![sigma];
        let corr = vec![vec![1.0]];

        let paths = sim.simulate_paths(1, &drift, &vol, &corr).unwrap();

        // At T=1.0 (last time point), E[S_T] = exp(0.05)
        let t_last = 3; // index for T=1.0
        let mean: f64 = paths[0][t_last].iter().sum::<f64>() / paths[0][t_last].len() as f64;
        let expected = (mu * 1.0).exp();

        // With 10K paths, should be within ~5% of theoretical
        assert_relative_eq!(mean, expected, epsilon = 0.1);
    }

    #[test]
    fn test_simulator_config_accessor() {
        let config = make_config(500, 42);
        let sim = XvaSimulator::new(config.clone());
        assert_eq!(sim.config().n_paths, 500);
        assert_eq!(sim.config().seed, Some(42));
    }
}
