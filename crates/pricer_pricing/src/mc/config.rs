//! Monte Carlo simulation configuration.
//!
//! This module provides configuration types and builders for Monte Carlo
//! pricing simulations with automatic differentiation support.

use super::error::ConfigError;

/// Maximum number of simulation paths allowed.
pub const MAX_PATHS: usize = 10_000_000;

/// Maximum number of time steps allowed per path.
pub const MAX_STEPS: usize = 10_000;

/// Automatic differentiation mode for gradient computation.
///
/// Specifies which AD mode to use for computing sensitivities (Greeks).
///
/// # Activity Analysis
///
/// - `NoAd`: No differentiation; primal computation only
/// - `Forward`: Forward mode (tangent propagation); efficient for few inputs
/// - `Reverse`: Reverse mode (adjoint accumulation); efficient for few outputs
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AdMode {
    /// No automatic differentiation.
    ///
    /// Use for primal-only computation or manual finite difference.
    #[default]
    NoAd,

    /// Forward mode AD (tangent propagation).
    ///
    /// Efficient when number of inputs << number of outputs.
    /// Use for Delta (single spot sensitivity).
    Forward,

    /// Reverse mode AD (adjoint accumulation).
    ///
    /// Efficient when number of outputs << number of inputs.
    /// Use for Vega (sensitivity to volatility surface).
    Reverse,
}

/// Monte Carlo simulation configuration.
///
/// Immutable configuration specifying simulation parameters.
/// Use [`MonteCarloConfigBuilder`] to construct instances.
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::mc::{MonteCarloConfig, AdMode};
///
/// let config = MonteCarloConfig::builder()
///     .n_paths(10_000)
///     .n_steps(252)
///     .ad_mode(AdMode::Forward)
///     .seed(42)
///     .build()
///     .expect("valid configuration");
///
/// assert_eq!(config.n_paths(), 10_000);
/// assert_eq!(config.n_steps(), 252);
/// ```
#[derive(Clone, Debug)]
pub struct MonteCarloConfig {
    /// Number of simulation paths.
    n_paths: usize,
    /// Number of time steps per path.
    n_steps: usize,
    /// AD mode for gradient computation.
    ad_mode: AdMode,
    /// Optional seed for reproducibility.
    seed: Option<u64>,
}

impl MonteCarloConfig {
    /// Creates a new configuration builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::MonteCarloConfig;
    ///
    /// let config = MonteCarloConfig::builder()
    ///     .n_paths(1000)
    ///     .n_steps(100)
    ///     .build()
    ///     .unwrap();
    /// ```
    #[inline]
    pub fn builder() -> MonteCarloConfigBuilder {
        MonteCarloConfigBuilder::default()
    }

    /// Returns the number of simulation paths.
    #[inline]
    pub fn n_paths(&self) -> usize {
        self.n_paths
    }

    /// Returns the number of time steps per path.
    #[inline]
    pub fn n_steps(&self) -> usize {
        self.n_steps
    }

    /// Returns the AD mode for gradient computation.
    #[inline]
    pub fn ad_mode(&self) -> AdMode {
        self.ad_mode
    }

    /// Returns the optional seed for reproducibility.
    #[inline]
    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if:
    /// - `n_paths` is 0 or greater than 10,000,000
    /// - `n_steps` is 0 or greater than 10,000
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.n_paths == 0 || self.n_paths > MAX_PATHS {
            return Err(ConfigError::InvalidPathCount(self.n_paths));
        }
        if self.n_steps == 0 || self.n_steps > MAX_STEPS {
            return Err(ConfigError::InvalidStepCount(self.n_steps));
        }
        Ok(())
    }
}

/// Builder for [`MonteCarloConfig`].
///
/// Provides a fluent API for constructing Monte Carlo configurations
/// with validation at build time.
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::mc::MonteCarloConfig;
///
/// let config = MonteCarloConfig::builder()
///     .n_paths(50_000)
///     .n_steps(252)  // Daily steps for 1 year
///     .seed(12345)
///     .build()
///     .expect("valid config");
/// ```
#[derive(Clone, Debug, Default)]
pub struct MonteCarloConfigBuilder {
    n_paths: Option<usize>,
    n_steps: Option<usize>,
    ad_mode: AdMode,
    seed: Option<u64>,
}

impl MonteCarloConfigBuilder {
    /// Sets the number of simulation paths.
    ///
    /// # Arguments
    ///
    /// * `n_paths` - Number of paths in [1, 10_000_000]
    #[inline]
    pub fn n_paths(mut self, n_paths: usize) -> Self {
        self.n_paths = Some(n_paths);
        self
    }

    /// Sets the number of time steps per path.
    ///
    /// # Arguments
    ///
    /// * `n_steps` - Number of steps in [1, 10_000]
    #[inline]
    pub fn n_steps(mut self, n_steps: usize) -> Self {
        self.n_steps = Some(n_steps);
        self
    }

    /// Sets the AD mode for gradient computation.
    ///
    /// # Arguments
    ///
    /// * `ad_mode` - Automatic differentiation mode
    #[inline]
    pub fn ad_mode(mut self, ad_mode: AdMode) -> Self {
        self.ad_mode = ad_mode;
        self
    }

    /// Sets the seed for reproducibility.
    ///
    /// # Arguments
    ///
    /// * `seed` - 64-bit seed value
    #[inline]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Builds the configuration.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if:
    /// - `n_paths` not set or invalid
    /// - `n_steps` not set or invalid
    pub fn build(self) -> Result<MonteCarloConfig, ConfigError> {
        let n_paths = self.n_paths.ok_or(ConfigError::InvalidParameter {
            name: "n_paths",
            value: "must be specified".to_string(),
        })?;

        let n_steps = self.n_steps.ok_or(ConfigError::InvalidParameter {
            name: "n_steps",
            value: "must be specified".to_string(),
        })?;

        let config = MonteCarloConfig {
            n_paths,
            n_steps,
            ad_mode: self.ad_mode,
            seed: self.seed,
        };

        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder_valid() {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(252)
            .build()
            .unwrap();

        assert_eq!(config.n_paths(), 10_000);
        assert_eq!(config.n_steps(), 252);
        assert_eq!(config.ad_mode(), AdMode::NoAd);
        assert_eq!(config.seed(), None);
    }

    #[test]
    fn test_config_builder_with_seed() {
        let config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .seed(42)
            .build()
            .unwrap();

        assert_eq!(config.seed(), Some(42));
    }

    #[test]
    fn test_config_builder_with_ad_mode() {
        let config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .ad_mode(AdMode::Forward)
            .build()
            .unwrap();

        assert_eq!(config.ad_mode(), AdMode::Forward);
    }

    #[test]
    fn test_config_invalid_zero_paths() {
        let result = MonteCarloConfig::builder().n_paths(0).n_steps(100).build();

        assert!(matches!(result, Err(ConfigError::InvalidPathCount(0))));
    }

    #[test]
    fn test_config_invalid_too_many_paths() {
        let result = MonteCarloConfig::builder()
            .n_paths(MAX_PATHS + 1)
            .n_steps(100)
            .build();

        assert!(matches!(result, Err(ConfigError::InvalidPathCount(_))));
    }

    #[test]
    fn test_config_invalid_zero_steps() {
        let result = MonteCarloConfig::builder().n_paths(1000).n_steps(0).build();

        assert!(matches!(result, Err(ConfigError::InvalidStepCount(0))));
    }

    #[test]
    fn test_config_invalid_too_many_steps() {
        let result = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(MAX_STEPS + 1)
            .build();

        assert!(matches!(result, Err(ConfigError::InvalidStepCount(_))));
    }

    #[test]
    fn test_config_missing_paths() {
        let result = MonteCarloConfig::builder().n_steps(100).build();

        assert!(matches!(
            result,
            Err(ConfigError::InvalidParameter {
                name: "n_paths",
                ..
            })
        ));
    }

    #[test]
    fn test_config_missing_steps() {
        let result = MonteCarloConfig::builder().n_paths(1000).build();

        assert!(matches!(
            result,
            Err(ConfigError::InvalidParameter {
                name: "n_steps",
                ..
            })
        ));
    }

    #[test]
    fn test_ad_mode_default() {
        assert_eq!(AdMode::default(), AdMode::NoAd);
    }
}
