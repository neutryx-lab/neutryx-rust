//! Configuration for the XVA engine.

use super::error::XvaEngineError;

/// Configuration for the full-valuation Monte Carlo XVA engine.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct XvaEngineConfig {
    /// Number of Monte Carlo simulation paths.
    pub n_paths: usize,
    /// Time grid in year fractions (e.g., [0.25, 0.5, 0.75, ...]).
    pub time_grid: Vec<f64>,
    /// Optional seed for reproducible simulations.
    pub seed: Option<u64>,
    /// Whether to use antithetic variates for variance reduction.
    pub antithetic: bool,
    /// Percentiles for Potential Future Exposure computation.
    pub pfe_percentiles: Vec<f64>,
    /// Whether to compute bilateral (CVA + DVA) or unilateral (CVA only).
    pub bilateral: bool,
    /// Whether to compute FVA (Funding Valuation Adjustment).
    pub compute_fva: bool,
    /// Whether to compute ECB (Expected Collateral Balance).
    pub compute_ecb: bool,
}

impl XvaEngineConfig {
    /// Creates a new configuration with default values.
    ///
    /// Defaults:
    /// - `n_paths`: 10,000
    /// - `time_grid`: quarterly for 5 years (20 points)
    /// - `seed`: None (random)
    /// - `antithetic`: true
    /// - `pfe_percentiles`: [0.95, 0.975, 0.99]
    /// - `bilateral`: true
    /// - `compute_fva`: true
    /// - `compute_ecb`: true
    pub fn new() -> Self { Self::default() }

    /// Returns a builder for constructing an `XvaEngineConfig`.
    pub fn builder() -> XvaEngineConfigBuilder { XvaEngineConfigBuilder::default() }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), XvaEngineError> {
        if self.n_paths == 0 {
            return Err(XvaEngineError::ConfigError(
                "n_paths must be positive".to_string(),
            ));
        }

        if self.time_grid.is_empty() {
            return Err(XvaEngineError::InvalidTimeGrid(
                "time grid must not be empty".to_string(),
            ));
        }

        // Check time grid is strictly positive and monotonically increasing
        for (i, &t) in self.time_grid.iter().enumerate() {
            if t <= 0.0 {
                return Err(XvaEngineError::InvalidTimeGrid(format!(
                    "time grid values must be positive, got {} at index {}",
                    t, i
                )));
            }
            if i > 0 && t <= self.time_grid[i - 1] {
                return Err(XvaEngineError::InvalidTimeGrid(format!(
                    "time grid must be strictly increasing, but t[{}]={} <= t[{}]={}",
                    i,
                    t,
                    i - 1,
                    self.time_grid[i - 1]
                )));
            }
        }

        // Check PFE percentiles are in (0, 1)
        for &p in &self.pfe_percentiles {
            if p <= 0.0 || p >= 1.0 {
                return Err(XvaEngineError::ConfigError(format!(
                    "PFE percentile must be in (0, 1), got {}",
                    p
                )));
            }
        }

        Ok(())
    }
}

impl Default for XvaEngineConfig {
    fn default() -> Self {
        // Quarterly time grid for 5 years
        let time_grid: Vec<f64> = (1..=20).map(|i| i as f64 * 0.25).collect();

        Self {
            n_paths: 10_000,
            time_grid,
            seed: None,
            antithetic: true,
            pfe_percentiles: vec![0.95, 0.975, 0.99],
            bilateral: true,
            compute_fva: true,
            compute_ecb: true,
        }
    }
}

/// Builder for `XvaEngineConfig`.
#[derive(Debug, Default)]
pub struct XvaEngineConfigBuilder {
    n_paths: Option<usize>,
    time_grid: Option<Vec<f64>>,
    seed: Option<u64>,
    antithetic: Option<bool>,
    pfe_percentiles: Option<Vec<f64>>,
    bilateral: Option<bool>,
    compute_fva: Option<bool>,
    compute_ecb: Option<bool>,
}

impl XvaEngineConfigBuilder {
    /// Sets the number of Monte Carlo paths.
    pub fn n_paths(mut self, n_paths: usize) -> Self {
        self.n_paths = Some(n_paths);
        self
    }

    /// Sets the time grid.
    pub fn time_grid(mut self, time_grid: Vec<f64>) -> Self {
        self.time_grid = Some(time_grid);
        self
    }

    /// Sets the random seed.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Sets the antithetic variates flag.
    pub fn antithetic(mut self, antithetic: bool) -> Self {
        self.antithetic = Some(antithetic);
        self
    }

    /// Sets the PFE percentiles.
    pub fn pfe_percentiles(mut self, percentiles: Vec<f64>) -> Self {
        self.pfe_percentiles = Some(percentiles);
        self
    }

    /// Sets the bilateral flag.
    pub fn bilateral(mut self, bilateral: bool) -> Self {
        self.bilateral = Some(bilateral);
        self
    }

    /// Sets the compute FVA flag.
    pub fn compute_fva(mut self, compute_fva: bool) -> Self {
        self.compute_fva = Some(compute_fva);
        self
    }

    /// Sets the compute ECB flag.
    pub fn compute_ecb(mut self, compute_ecb: bool) -> Self {
        self.compute_ecb = Some(compute_ecb);
        self
    }

    /// Builds and validates the configuration.
    pub fn build(self) -> Result<XvaEngineConfig, XvaEngineError> {
        let defaults = XvaEngineConfig::default();

        let config = XvaEngineConfig {
            n_paths: self.n_paths.unwrap_or(defaults.n_paths),
            time_grid: self.time_grid.unwrap_or(defaults.time_grid),
            seed: self.seed,
            antithetic: self.antithetic.unwrap_or(defaults.antithetic),
            pfe_percentiles: self.pfe_percentiles.unwrap_or(defaults.pfe_percentiles),
            bilateral: self.bilateral.unwrap_or(defaults.bilateral),
            compute_fva: self.compute_fva.unwrap_or(defaults.compute_fva),
            compute_ecb: self.compute_ecb.unwrap_or(defaults.compute_ecb),
        };

        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = XvaEngineConfig::new();
        assert_eq!(config.n_paths, 10_000);
        assert_eq!(config.time_grid.len(), 20);
        assert!(config.seed.is_none());
        assert!(config.antithetic);
        assert_eq!(config.pfe_percentiles, vec![0.95, 0.975, 0.99]);
        assert!(config.bilateral);
        assert!(config.compute_fva);
        assert!(config.compute_ecb);
    }

    #[test]
    fn test_default_time_grid_quarterly_5y() {
        let config = XvaEngineConfig::new();
        assert_eq!(config.time_grid.len(), 20);
        assert!((config.time_grid[0] - 0.25).abs() < f64::EPSILON);
        assert!((config.time_grid[19] - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_validate_default() {
        let config = XvaEngineConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_paths() {
        let mut config = XvaEngineConfig::new();
        config.n_paths = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_empty_time_grid() {
        let mut config = XvaEngineConfig::new();
        config.time_grid = vec![];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_negative_time() {
        let mut config = XvaEngineConfig::new();
        config.time_grid = vec![-0.25, 0.5];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_non_monotonic_time() {
        let mut config = XvaEngineConfig::new();
        config.time_grid = vec![0.25, 0.5, 0.3];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_bad_pfe_percentile() {
        let mut config = XvaEngineConfig::new();
        config.pfe_percentiles = vec![0.95, 1.5];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_pfe_percentile() {
        let mut config = XvaEngineConfig::new();
        config.pfe_percentiles = vec![0.0];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_builder_defaults() {
        let config = XvaEngineConfig::builder().build().unwrap();
        assert_eq!(config.n_paths, 10_000);
        assert!(config.antithetic);
    }

    #[test]
    fn test_builder_custom() {
        let config = XvaEngineConfig::builder()
            .n_paths(5000)
            .time_grid(vec![0.5, 1.0, 1.5, 2.0])
            .seed(42)
            .antithetic(false)
            .pfe_percentiles(vec![0.95])
            .bilateral(false)
            .compute_fva(false)
            .compute_ecb(false)
            .build()
            .unwrap();

        assert_eq!(config.n_paths, 5000);
        assert_eq!(config.time_grid, vec![0.5, 1.0, 1.5, 2.0]);
        assert_eq!(config.seed, Some(42));
        assert!(!config.antithetic);
        assert_eq!(config.pfe_percentiles, vec![0.95]);
        assert!(!config.bilateral);
        assert!(!config.compute_fva);
        assert!(!config.compute_ecb);
    }

    #[test]
    fn test_builder_validation_failure() {
        let result = XvaEngineConfig::builder().n_paths(0).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_clone() {
        let config = XvaEngineConfig::new();
        let cloned = config.clone();
        assert_eq!(config.n_paths, cloned.n_paths);
        assert_eq!(config.time_grid, cloned.time_grid);
    }

    #[test]
    fn test_config_debug() {
        let config = XvaEngineConfig::new();
        let debug = format!("{:?}", config);
        assert!(debug.contains("n_paths"));
        assert!(debug.contains("10000"));
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let config = XvaEngineConfig::new();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: XvaEngineConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.n_paths, deserialized.n_paths);
        assert_eq!(config.time_grid, deserialized.time_grid);
    }
}
