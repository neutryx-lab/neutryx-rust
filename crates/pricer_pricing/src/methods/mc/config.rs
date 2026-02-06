//! Monte Carlo simulation configuration.
//!
//! This module provides configuration types and builders for Monte Carlo
//! pricing simulations with automatic differentiation support.
//!
//! # Memory Layout Configuration
//!
//! The configuration supports two memory layouts:
//! - `PathFirst`: Traditional `[path][step]` layout (default, backward
//!   compatible)
//! - `TimeStepFirst`: Optimised `[step][path]` layout for cache efficiency and
//!   SIMD
//!
//! # Streaming Mode
//!
//! When streaming is enabled, simulation processes step-by-step with O(paths)
//! memory instead of O(paths × steps). Streaming requires `TimeStepFirst`
//! layout.

use super::{
    error::{LayoutConfigError, MonteCarloConfigError},
    layout_config::{PathLayout, PathLayoutConfig, StreamingConfig},
};

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
///
/// # Layout and Streaming Configuration
///
/// ```rust
/// use pricer_pricing::mc::{MonteCarloConfig, PathLayoutConfig, PathLayout, StreamingConfig};
///
/// let config = MonteCarloConfig::builder()
///     .n_paths(100_000)
///     .n_steps(252)
///     .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
///     .streaming(StreamingConfig::enabled())
///     .seed(42)
///     .build()
///     .expect("valid configuration");
///
/// assert_eq!(config.layout().layout(), PathLayout::TimeStepFirst);
/// assert!(config.streaming().is_enabled());
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
    /// Memory layout configuration.
    layout: PathLayoutConfig,
    /// Streaming mode configuration.
    streaming: StreamingConfig,
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
    pub fn builder() -> MonteCarloConfigBuilder { MonteCarloConfigBuilder::default() }

    /// Returns the number of simulation paths.
    #[inline]
    pub fn n_paths(&self) -> usize { self.n_paths }

    /// Returns the number of time steps per path.
    #[inline]
    pub fn n_steps(&self) -> usize { self.n_steps }

    /// Returns the AD mode for gradient computation.
    #[inline]
    pub fn ad_mode(&self) -> AdMode { self.ad_mode }

    /// Returns the optional seed for reproducibility.
    #[inline]
    pub fn seed(&self) -> Option<u64> { self.seed }

    /// Returns the memory layout configuration.
    #[inline]
    pub fn layout(&self) -> &PathLayoutConfig { &self.layout }

    /// Returns the streaming mode configuration.
    #[inline]
    pub fn streaming(&self) -> &StreamingConfig { &self.streaming }

    /// Returns true if streaming mode is enabled.
    #[inline]
    pub fn is_streaming(&self) -> bool { self.streaming.is_enabled() }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns `MonteCarloConfigError` if:
    /// - `n_paths` is 0 or greater than 10,000,000
    /// - `n_steps` is 0 or greater than 10,000
    /// - Streaming enabled with PathFirst layout
    /// - Invalid alignment or buffer steps
    pub fn validate(&self) -> Result<(), MonteCarloConfigError> {
        if self.n_paths == 0 || self.n_paths > MAX_PATHS {
            return Err(MonteCarloConfigError::InvalidPathCount(self.n_paths));
        }
        if self.n_steps == 0 || self.n_steps > MAX_STEPS {
            return Err(MonteCarloConfigError::InvalidStepCount(self.n_steps));
        }

        // Validate layout configuration
        self.layout.validate()?;

        // Validate streaming configuration
        self.streaming.validate()?;

        // Streaming mode requires TimeStepFirst layout
        if self.streaming.is_enabled() && self.layout.layout() == PathLayout::PathFirst {
            return Err(MonteCarloConfigError::LayoutError(
                LayoutConfigError::StreamingRequiresTimeStepFirst,
            ));
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
///
/// # Layout and Streaming
///
/// ```rust
/// use pricer_pricing::mc::{MonteCarloConfig, PathLayoutConfig, PathLayout, StreamingConfig};
///
/// let config = MonteCarloConfig::builder()
///     .n_paths(100_000)
///     .n_steps(252)
///     .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
///     .streaming(StreamingConfig::enabled())
///     .build()
///     .expect("valid config");
/// ```
#[derive(Clone, Debug, Default)]
pub struct MonteCarloConfigBuilder {
    n_paths: Option<usize>,
    n_steps: Option<usize>,
    ad_mode: AdMode,
    seed: Option<u64>,
    layout: PathLayoutConfig,
    streaming: StreamingConfig,
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

    /// Sets the memory layout configuration.
    ///
    /// # Arguments
    ///
    /// * `layout` - Path layout configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::{MonteCarloConfig, PathLayoutConfig, PathLayout};
    ///
    /// let config = MonteCarloConfig::builder()
    ///     .n_paths(10_000)
    ///     .n_steps(100)
    ///     .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
    ///     .build()
    ///     .unwrap();
    /// ```
    #[inline]
    pub fn layout(mut self, layout: PathLayoutConfig) -> Self {
        self.layout = layout;
        self
    }

    /// Sets the streaming mode configuration.
    ///
    /// # Arguments
    ///
    /// * `streaming` - Streaming configuration
    ///
    /// # Note
    ///
    /// Streaming mode requires `TimeStepFirst` layout. The build will fail
    /// if streaming is enabled with `PathFirst` layout.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::{MonteCarloConfig, PathLayoutConfig, PathLayout, StreamingConfig};
    ///
    /// let config = MonteCarloConfig::builder()
    ///     .n_paths(100_000)
    ///     .n_steps(252)
    ///     .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
    ///     .streaming(StreamingConfig::enabled())
    ///     .build()
    ///     .unwrap();
    /// ```
    #[inline]
    pub fn streaming(mut self, streaming: StreamingConfig) -> Self {
        self.streaming = streaming;
        self
    }

    /// Builds the configuration.
    ///
    /// # Errors
    ///
    /// Returns `MonteCarloConfigError` if:
    /// - `n_paths` not set or invalid
    /// - `n_steps` not set or invalid
    pub fn build(self) -> Result<MonteCarloConfig, MonteCarloConfigError> {
        let n_paths = self
            .n_paths
            .ok_or(MonteCarloConfigError::InvalidParameter {
                name: "n_paths",
                value: "must be specified".to_string(),
            })?;

        let n_steps = self
            .n_steps
            .ok_or(MonteCarloConfigError::InvalidParameter {
                name: "n_steps",
                value: "must be specified".to_string(),
            })?;

        let config = MonteCarloConfig {
            n_paths,
            n_steps,
            ad_mode: self.ad_mode,
            seed: self.seed,
            layout: self.layout,
            streaming: self.streaming,
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

        assert!(matches!(
            result,
            Err(MonteCarloConfigError::InvalidPathCount(0))
        ));
    }

    #[test]
    fn test_config_invalid_too_many_paths() {
        let result = MonteCarloConfig::builder()
            .n_paths(MAX_PATHS + 1)
            .n_steps(100)
            .build();

        assert!(matches!(
            result,
            Err(MonteCarloConfigError::InvalidPathCount(_))
        ));
    }

    #[test]
    fn test_config_invalid_zero_steps() {
        let result = MonteCarloConfig::builder().n_paths(1000).n_steps(0).build();

        assert!(matches!(
            result,
            Err(MonteCarloConfigError::InvalidStepCount(0))
        ));
    }

    #[test]
    fn test_config_invalid_too_many_steps() {
        let result = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(MAX_STEPS + 1)
            .build();

        assert!(matches!(
            result,
            Err(MonteCarloConfigError::InvalidStepCount(_))
        ));
    }

    #[test]
    fn test_config_missing_paths() {
        let result = MonteCarloConfig::builder().n_steps(100).build();

        assert!(matches!(
            result,
            Err(MonteCarloConfigError::InvalidParameter {
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
            Err(MonteCarloConfigError::InvalidParameter {
                name: "n_steps",
                ..
            })
        ));
    }

    #[test]
    fn test_ad_mode_default() {
        assert_eq!(AdMode::default(), AdMode::NoAd);
    }

    // ========================================================================
    // Layout and Streaming Configuration Tests
    // ========================================================================

    #[test]
    fn test_config_default_layout() {
        let config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .build()
            .unwrap();

        assert_eq!(config.layout().layout(), PathLayout::PathFirst);
        assert_eq!(config.layout().alignment(), 64);
        assert!(!config.is_streaming());
    }

    #[test]
    fn test_config_with_timestep_first_layout() {
        let config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
            .build()
            .unwrap();

        assert_eq!(config.layout().layout(), PathLayout::TimeStepFirst);
    }

    #[test]
    fn test_config_with_streaming_enabled() {
        let config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
            .streaming(StreamingConfig::enabled())
            .build()
            .unwrap();

        assert!(config.is_streaming());
        assert_eq!(config.streaming().buffer_steps(), 2);
    }

    #[test]
    fn test_config_streaming_requires_timestep_first() {
        let result = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .layout(PathLayoutConfig::with_layout(PathLayout::PathFirst))
            .streaming(StreamingConfig::enabled())
            .build();

        assert!(matches!(
            result,
            Err(MonteCarloConfigError::LayoutError(
                LayoutConfigError::StreamingRequiresTimeStepFirst
            ))
        ));
    }

    #[test]
    fn test_config_invalid_alignment() {
        let result = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .layout(PathLayoutConfig::new(PathLayout::PathFirst, 7)) // Not power of 2
            .build();

        assert!(matches!(
            result,
            Err(MonteCarloConfigError::LayoutError(
                LayoutConfigError::InvalidAlignment(7)
            ))
        ));
    }

    #[test]
    fn test_config_invalid_buffer_steps() {
        let result = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
            .streaming(StreamingConfig::new(true, 1)) // Less than 2
            .build();

        assert!(matches!(
            result,
            Err(MonteCarloConfigError::LayoutError(
                LayoutConfigError::InvalidBufferSteps(1)
            ))
        ));
    }

    #[test]
    fn test_config_custom_alignment() {
        let config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .layout(PathLayoutConfig::new(PathLayout::TimeStepFirst, 128))
            .build()
            .unwrap();

        assert_eq!(config.layout().alignment(), 128);
    }

    #[test]
    fn test_config_streaming_disabled_with_path_first() {
        // PathFirst with streaming disabled is valid
        let config = MonteCarloConfig::builder()
            .n_paths(1000)
            .n_steps(100)
            .layout(PathLayoutConfig::with_layout(PathLayout::PathFirst))
            .streaming(StreamingConfig::disabled())
            .build()
            .unwrap();

        assert!(!config.is_streaming());
        assert_eq!(config.layout().layout(), PathLayout::PathFirst);
    }
}
