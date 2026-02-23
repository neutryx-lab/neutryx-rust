//! XVA simulation configuration and builder.
//!
//! Provides [`XvaSimulationConfig`] for controlling Monte Carlo XVA
//! simulations, including path counts, time grids, seeding, antithetic variance
//! reduction, and the choice of pricing measure.

/// Simulation pricing measure.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SimulationMeasure {
    /// Risk-neutral (spot) measure.
    #[default]
    RiskNeutral,
    /// T-forward measure associated with a specific maturity index.
    TForward {
        /// Index into the time grid for the forward measure maturity.
        maturity_idx: usize,
    },
    /// Terminal measure (forward measure at the last time point).
    Terminal,
}

/// Configuration for XVA Monte Carlo simulations.
#[derive(Clone, Debug)]
pub struct XvaSimulationConfig {
    /// Number of Monte Carlo paths.
    n_paths: usize,
    /// Time grid points (year fractions from valuation date).
    time_grid: Vec<f64>,
    /// Optional seed for reproducibility.
    seed: Option<u64>,
    /// Whether to use antithetic variance reduction.
    antithetic: bool,
    /// Pricing measure for the simulation.
    measure: SimulationMeasure,
}

impl XvaSimulationConfig {
    /// Creates a new configuration builder.
    #[inline]
    pub fn builder() -> XvaSimulationConfigBuilder { XvaSimulationConfigBuilder::default() }

    /// Returns the number of Monte Carlo paths.
    #[inline]
    pub fn n_paths(&self) -> usize { self.n_paths }

    /// Returns the time grid points.
    #[inline]
    pub fn time_grid(&self) -> &[f64] { &self.time_grid }

    /// Returns the optional seed for reproducibility.
    #[inline]
    pub fn seed(&self) -> Option<u64> { self.seed }

    /// Returns whether antithetic variance reduction is enabled.
    #[inline]
    pub fn antithetic(&self) -> bool { self.antithetic }

    /// Returns the simulation pricing measure.
    #[inline]
    pub fn measure(&self) -> SimulationMeasure { self.measure }

    /// Returns the number of time grid points.
    #[inline]
    pub fn n_times(&self) -> usize { self.time_grid.len() }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), XvaSimulationConfigError> {
        if self.n_paths == 0 {
            return Err(XvaSimulationConfigError::InvalidPathCount(self.n_paths));
        }
        if self.time_grid.is_empty() {
            return Err(XvaSimulationConfigError::EmptyTimeGrid);
        }
        for window in self.time_grid.windows(2) {
            if window[1] <= window[0] {
                return Err(XvaSimulationConfigError::NonMonotonicTimeGrid {
                    prev: window[0],
                    next: window[1],
                });
            }
        }
        Ok(())
    }
}

/// Errors arising from invalid XVA simulation configuration.
#[derive(Clone, Debug, thiserror::Error)]
pub enum XvaSimulationConfigError {
    /// The path count is invalid (must be > 0).
    #[error("invalid path count: {0} (must be > 0)")]
    InvalidPathCount(usize),

    /// The time grid is empty.
    #[error("time grid must not be empty")]
    EmptyTimeGrid,

    /// The time grid is not monotonically increasing.
    #[error("time grid is not monotonically increasing: {prev} >= {next}")]
    NonMonotonicTimeGrid {
        /// The earlier time point.
        prev: f64,
        /// The later time point that violates monotonicity.
        next: f64,
    },

    /// A required parameter was not set on the builder.
    #[error("missing required parameter: {name}")]
    MissingParameter {
        /// Name of the missing parameter.
        name: &'static str,
    },
}

/// Builder for [`XvaSimulationConfig`].
#[derive(Clone, Debug, Default)]
pub struct XvaSimulationConfigBuilder {
    n_paths: Option<usize>,
    time_grid: Option<Vec<f64>>,
    seed: Option<u64>,
    antithetic: bool,
    measure: SimulationMeasure,
}

impl XvaSimulationConfigBuilder {
    /// Sets the number of Monte Carlo paths.
    #[inline]
    pub fn n_paths(mut self, n_paths: usize) -> Self {
        self.n_paths = Some(n_paths);
        self
    }

    /// Sets the time grid points (year fractions).
    #[inline]
    pub fn time_grid(mut self, time_grid: Vec<f64>) -> Self {
        self.time_grid = Some(time_grid);
        self
    }

    /// Sets the seed for reproducibility.
    #[inline]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Enables or disables antithetic variance reduction.
    #[inline]
    pub fn antithetic(mut self, antithetic: bool) -> Self {
        self.antithetic = antithetic;
        self
    }

    /// Sets the simulation pricing measure.
    #[inline]
    pub fn measure(mut self, measure: SimulationMeasure) -> Self {
        self.measure = measure;
        self
    }

    /// Builds and validates the configuration.
    pub fn build(self) -> Result<XvaSimulationConfig, XvaSimulationConfigError> {
        let n_paths = self
            .n_paths
            .ok_or(XvaSimulationConfigError::MissingParameter { name: "n_paths" })?;

        let time_grid = self
            .time_grid
            .ok_or(XvaSimulationConfigError::MissingParameter { name: "time_grid" })?;

        let config = XvaSimulationConfig {
            n_paths,
            time_grid,
            seed: self.seed,
            antithetic: self.antithetic,
            measure: self.measure,
        };

        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_valid_config() {
        let config = XvaSimulationConfig::builder()
            .n_paths(10_000)
            .time_grid(vec![0.25, 0.5, 1.0, 2.0])
            .build()
            .unwrap();

        assert_eq!(config.n_paths(), 10_000);
        assert_eq!(config.time_grid(), &[0.25, 0.5, 1.0, 2.0]);
        assert_eq!(config.seed(), None);
        assert!(!config.antithetic());
        assert_eq!(config.measure(), SimulationMeasure::RiskNeutral);
        assert_eq!(config.n_times(), 4);
    }

    #[test]
    fn test_builder_with_all_options() {
        let config = XvaSimulationConfig::builder()
            .n_paths(50_000)
            .time_grid(vec![0.5, 1.0])
            .seed(42)
            .antithetic(true)
            .measure(SimulationMeasure::TForward { maturity_idx: 3 })
            .build()
            .unwrap();

        assert_eq!(config.n_paths(), 50_000);
        assert_eq!(config.seed(), Some(42));
        assert!(config.antithetic());
        assert_eq!(
            config.measure(),
            SimulationMeasure::TForward { maturity_idx: 3 }
        );
    }

    #[test]
    fn test_builder_missing_n_paths() {
        let result = XvaSimulationConfig::builder()
            .time_grid(vec![0.25, 0.5])
            .build();

        assert!(matches!(
            result,
            Err(XvaSimulationConfigError::MissingParameter { name: "n_paths" })
        ));
    }

    #[test]
    fn test_builder_missing_time_grid() {
        let result = XvaSimulationConfig::builder().n_paths(1000).build();

        assert!(matches!(
            result,
            Err(XvaSimulationConfigError::MissingParameter { name: "time_grid" })
        ));
    }

    #[test]
    fn test_invalid_zero_paths() {
        let result = XvaSimulationConfig::builder()
            .n_paths(0)
            .time_grid(vec![0.25])
            .build();

        assert!(matches!(
            result,
            Err(XvaSimulationConfigError::InvalidPathCount(0))
        ));
    }

    #[test]
    fn test_empty_time_grid() {
        let result = XvaSimulationConfig::builder()
            .n_paths(1000)
            .time_grid(vec![])
            .build();

        assert!(matches!(
            result,
            Err(XvaSimulationConfigError::EmptyTimeGrid)
        ));
    }

    #[test]
    fn test_non_monotonic_time_grid() {
        let result = XvaSimulationConfig::builder()
            .n_paths(1000)
            .time_grid(vec![0.25, 0.5, 0.3, 1.0])
            .build();

        assert!(matches!(
            result,
            Err(XvaSimulationConfigError::NonMonotonicTimeGrid { .. })
        ));
    }

    #[test]
    fn test_duplicate_time_grid_points() {
        let result = XvaSimulationConfig::builder()
            .n_paths(1000)
            .time_grid(vec![0.25, 0.5, 0.5, 1.0])
            .build();

        assert!(matches!(
            result,
            Err(XvaSimulationConfigError::NonMonotonicTimeGrid { .. })
        ));
    }

    #[test]
    fn test_single_time_point_is_valid() {
        let config = XvaSimulationConfig::builder()
            .n_paths(100)
            .time_grid(vec![1.0])
            .build()
            .unwrap();

        assert_eq!(config.n_times(), 1);
    }

    #[test]
    fn test_simulation_measure_default() {
        assert_eq!(SimulationMeasure::default(), SimulationMeasure::RiskNeutral);
    }

    #[test]
    fn test_terminal_measure() {
        let config = XvaSimulationConfig::builder()
            .n_paths(1000)
            .time_grid(vec![0.5, 1.0])
            .measure(SimulationMeasure::Terminal)
            .build()
            .unwrap();

        assert_eq!(config.measure(), SimulationMeasure::Terminal);
    }
}
