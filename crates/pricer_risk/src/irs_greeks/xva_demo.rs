//! XVA demonstration module for IRS Greeks.
//!
//! Provides exposure profile calculation, CVA/DVA computation, and XVA
//! sensitivity benchmarking.

use thiserror::Error;

/// XVA demo error types.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum XvaDemoError {
    /// Invalid configuration.
    #[error("Invalid XVA configuration: {0}")]
    InvalidConfig(String),
    /// Calculation failed.
    #[error("XVA calculation failed: {0}")]
    CalculationFailed(String),
}

/// Credit parameters for XVA calculation.
#[derive(Clone, Debug)]
pub struct CreditParams {
    /// Hazard rate (annual default probability).
    pub hazard_rate: f64,
    /// Loss given default (0-1).
    pub lgd: f64,
}

impl CreditParams {
    /// Create new credit parameters.
    pub fn new(hazard_rate: f64, lgd: f64) -> Result<Self, XvaDemoError> {
        if hazard_rate < 0.0 || hazard_rate > 1.0 {
            return Err(XvaDemoError::InvalidConfig(
                "hazard_rate must be between 0 and 1".to_string(),
            ));
        }
        if lgd < 0.0 || lgd > 1.0 {
            return Err(XvaDemoError::InvalidConfig(
                "lgd must be between 0 and 1".to_string(),
            ));
        }
        Ok(Self { hazard_rate, lgd })
    }
}

impl Default for CreditParams {
    fn default() -> Self {
        Self {
            hazard_rate: 0.02,
            lgd: 0.4,
        }
    }
}

/// XVA demo configuration.
#[derive(Clone, Debug)]
pub struct XvaDemoConfig {
    /// Number of simulation time points.
    pub time_points: usize,
    /// Time horizon in years.
    pub time_horizon: f64,
    /// Number of Monte Carlo paths.
    pub num_paths: usize,
}

impl Default for XvaDemoConfig {
    fn default() -> Self {
        Self {
            time_points: 20,
            time_horizon: 5.0,
            num_paths: 1000,
        }
    }
}

impl XvaDemoConfig {
    /// Create a new XVA demo configuration.
    pub fn new() -> Self { Self::default() }

    /// Set the number of time points.
    pub fn with_time_points(mut self, time_points: usize) -> Self {
        self.time_points = time_points;
        self
    }

    /// Set the time horizon.
    pub fn with_time_horizon(mut self, time_horizon: f64) -> Self {
        self.time_horizon = time_horizon;
        self
    }

    /// Set the number of Monte Carlo paths.
    pub fn with_num_paths(mut self, num_paths: usize) -> Self {
        self.num_paths = num_paths;
        self
    }
}

/// XVA demo runner.
#[derive(Clone, Debug)]
pub struct XvaDemoRunner {
    config: XvaDemoConfig,
}

impl XvaDemoRunner {
    /// Create a new XVA demo runner.
    pub fn new(config: XvaDemoConfig) -> Self { Self { config } }

    /// Get the configuration.
    pub fn config(&self) -> &XvaDemoConfig { &self.config }
}

impl Default for XvaDemoRunner {
    fn default() -> Self { Self::new(XvaDemoConfig::default()) }
}

/// Exposure profile result.
#[derive(Clone, Debug, Default)]
pub struct ExposureProfile {
    /// Time points.
    pub times: Vec<f64>,
    /// Expected exposure at each time point.
    pub expected_exposure: Vec<f64>,
    /// Potential future exposure (95th percentile).
    pub pfe_95: Vec<f64>,
}

/// XVA calculation result.
#[derive(Clone, Debug, Default)]
pub struct XvaResult {
    /// Credit Value Adjustment.
    pub cva: f64,
    /// Debt Value Adjustment.
    pub dva: f64,
    /// Bilateral CVA (CVA - DVA).
    pub bilateral_cva: f64,
    /// Exposure profile.
    pub exposure_profile: ExposureProfile,
}

/// XVA sensitivity benchmark result.
#[derive(Clone, Debug, Default)]
pub struct XvaSensitivityBenchmark {
    /// AAD sensitivity time in nanoseconds.
    pub aad_time_ns: u64,
    /// Bump sensitivity time in nanoseconds.
    pub bump_time_ns: u64,
    /// Speedup ratio.
    pub speedup_ratio: f64,
}
