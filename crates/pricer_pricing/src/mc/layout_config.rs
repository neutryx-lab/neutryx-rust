//! Memory layout configuration for Monte Carlo simulation.
//!
//! This module provides configuration types for controlling memory layout
//! and streaming behaviour in Monte Carlo simulations.
//!
//! # Layout Modes
//!
//! - [`PathLayout::PathFirst`]: Traditional `[path][step]` layout (default)
//! - [`PathLayout::TimeStepFirst`]: Optimised `[step][path]` layout for cache efficiency
//!
//! # Streaming Mode
//!
//! When enabled, streaming mode processes paths step-by-step, reducing memory
//! usage from O(paths × steps) to O(paths).

use super::error::LayoutConfigError;

/// Memory layout mode for path storage.
///
/// Determines the order of dimensions in the path buffer, affecting
/// cache locality and vectorisation efficiency.
///
/// # Cache Behaviour
///
/// - `PathFirst`: Efficient for path-wise operations (each path contiguous)
/// - `TimeStepFirst`: Efficient for step-wise operations (all paths at step t contiguous)
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::mc::PathLayout;
///
/// let layout = PathLayout::default();
/// assert_eq!(layout, PathLayout::PathFirst);
///
/// let optimised = PathLayout::TimeStepFirst;
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PathLayout {
    /// Path-first layout: `[path_idx][step_idx]`
    ///
    /// Traditional layout where each path is contiguous in memory.
    /// Default for backward compatibility.
    #[default]
    PathFirst,

    /// Time-step-first layout: `[step_idx][path_idx]`
    ///
    /// Optimised layout where all path values at a given time step
    /// are contiguous, enabling efficient SIMD vectorisation.
    TimeStepFirst,
}

/// Configuration for path memory layout.
///
/// Specifies the memory layout mode and alignment requirements for
/// path buffers in Monte Carlo simulation.
///
/// # Default Configuration
///
/// - Layout: `PathFirst` (backward compatible)
/// - Alignment: 64 bytes (AVX-512 cache line)
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::mc::{PathLayoutConfig, PathLayout};
///
/// // Default configuration
/// let config = PathLayoutConfig::default();
/// assert_eq!(config.layout(), PathLayout::PathFirst);
/// assert_eq!(config.alignment(), 64);
///
/// // Optimised configuration
/// let config = PathLayoutConfig::new(PathLayout::TimeStepFirst, 64);
/// assert_eq!(config.layout(), PathLayout::TimeStepFirst);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathLayoutConfig {
    /// Memory layout mode.
    layout: PathLayout,
    /// Alignment in bytes (must be power of 2).
    alignment: usize,
}

impl PathLayoutConfig {
    /// Default alignment for AVX-512 cache lines.
    pub const DEFAULT_ALIGNMENT: usize = 64;

    /// Creates a new layout configuration.
    ///
    /// # Arguments
    ///
    /// * `layout` - Memory layout mode
    /// * `alignment` - Alignment in bytes (must be power of 2)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::{PathLayoutConfig, PathLayout};
    ///
    /// let config = PathLayoutConfig::new(PathLayout::TimeStepFirst, 64);
    /// ```
    #[inline]
    pub const fn new(layout: PathLayout, alignment: usize) -> Self {
        Self { layout, alignment }
    }

    /// Creates a configuration with default alignment.
    ///
    /// # Arguments
    ///
    /// * `layout` - Memory layout mode
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::{PathLayoutConfig, PathLayout};
    ///
    /// let config = PathLayoutConfig::with_layout(PathLayout::TimeStepFirst);
    /// assert_eq!(config.alignment(), 64);
    /// ```
    #[inline]
    pub const fn with_layout(layout: PathLayout) -> Self {
        Self {
            layout,
            alignment: Self::DEFAULT_ALIGNMENT,
        }
    }

    /// Returns the layout mode.
    #[inline]
    pub const fn layout(&self) -> PathLayout {
        self.layout
    }

    /// Returns the alignment in bytes.
    #[inline]
    pub const fn alignment(&self) -> usize {
        self.alignment
    }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns `LayoutConfigError::InvalidAlignment` if alignment is not
    /// a power of 2 or is zero.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::{PathLayoutConfig, PathLayout};
    ///
    /// let valid = PathLayoutConfig::new(PathLayout::PathFirst, 64);
    /// assert!(valid.validate().is_ok());
    ///
    /// let invalid = PathLayoutConfig::new(PathLayout::PathFirst, 7);
    /// assert!(invalid.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), LayoutConfigError> {
        if self.alignment == 0 || !self.alignment.is_power_of_two() {
            return Err(LayoutConfigError::InvalidAlignment(self.alignment));
        }
        Ok(())
    }
}

impl Default for PathLayoutConfig {
    fn default() -> Self {
        Self {
            layout: PathLayout::PathFirst,
            alignment: Self::DEFAULT_ALIGNMENT,
        }
    }
}

/// Configuration for streaming mode processing.
///
/// When enabled, streaming mode processes simulation step-by-step,
/// dramatically reducing memory usage for large simulations.
///
/// # Memory Reduction
///
/// - Batch mode: O(paths × steps) memory
/// - Streaming mode: O(paths) memory (constant in steps)
///
/// # Default Configuration
///
/// - Enabled: `false` (batch mode for backward compatibility)
/// - Buffer steps: `2` (current and previous step)
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::mc::StreamingConfig;
///
/// // Default (disabled)
/// let config = StreamingConfig::default();
/// assert!(!config.enabled());
///
/// // Enabled streaming
/// let config = StreamingConfig::enabled();
/// assert!(config.enabled());
/// assert_eq!(config.buffer_steps(), 2);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamingConfig {
    /// Whether streaming mode is enabled.
    enabled: bool,
    /// Number of step buffers to maintain (minimum 2).
    buffer_steps: usize,
}

impl StreamingConfig {
    /// Minimum number of buffer steps required.
    pub const MIN_BUFFER_STEPS: usize = 2;

    /// Creates a new streaming configuration.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether streaming mode is enabled
    /// * `buffer_steps` - Number of step buffers (minimum 2)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::StreamingConfig;
    ///
    /// let config = StreamingConfig::new(true, 2);
    /// assert!(config.enabled());
    /// ```
    #[inline]
    pub const fn new(enabled: bool, buffer_steps: usize) -> Self {
        Self {
            enabled,
            buffer_steps,
        }
    }

    /// Creates an enabled streaming configuration with default buffer steps.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::StreamingConfig;
    ///
    /// let config = StreamingConfig::enabled();
    /// assert!(config.enabled());
    /// assert_eq!(config.buffer_steps(), 2);
    /// ```
    #[inline]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            buffer_steps: Self::MIN_BUFFER_STEPS,
        }
    }

    /// Creates a disabled streaming configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::StreamingConfig;
    ///
    /// let config = StreamingConfig::disabled();
    /// assert!(!config.enabled());
    /// ```
    #[inline]
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            buffer_steps: Self::MIN_BUFFER_STEPS,
        }
    }

    /// Returns whether streaming mode is enabled.
    #[inline]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the number of buffer steps.
    #[inline]
    pub const fn buffer_steps(&self) -> usize {
        self.buffer_steps
    }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns `LayoutConfigError::InvalidBufferSteps` if buffer_steps < 2.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::StreamingConfig;
    ///
    /// let valid = StreamingConfig::new(true, 2);
    /// assert!(valid.validate().is_ok());
    ///
    /// let invalid = StreamingConfig::new(true, 1);
    /// assert!(invalid.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), LayoutConfigError> {
        if self.buffer_steps < Self::MIN_BUFFER_STEPS {
            return Err(LayoutConfigError::InvalidBufferSteps(self.buffer_steps));
        }
        Ok(())
    }
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            buffer_steps: Self::MIN_BUFFER_STEPS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== PathLayout Tests ==========

    #[test]
    fn test_path_layout_default() {
        let layout = PathLayout::default();
        assert_eq!(layout, PathLayout::PathFirst);
    }

    #[test]
    fn test_path_layout_equality() {
        assert_eq!(PathLayout::PathFirst, PathLayout::PathFirst);
        assert_eq!(PathLayout::TimeStepFirst, PathLayout::TimeStepFirst);
        assert_ne!(PathLayout::PathFirst, PathLayout::TimeStepFirst);
    }

    #[test]
    fn test_path_layout_clone_copy() {
        let layout = PathLayout::TimeStepFirst;
        let cloned = layout;
        assert_eq!(layout, cloned);
    }

    #[test]
    fn test_path_layout_debug() {
        let debug_str = format!("{:?}", PathLayout::PathFirst);
        assert!(debug_str.contains("PathFirst"));

        let debug_str = format!("{:?}", PathLayout::TimeStepFirst);
        assert!(debug_str.contains("TimeStepFirst"));
    }

    #[test]
    fn test_path_layout_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PathLayout::PathFirst);
        set.insert(PathLayout::TimeStepFirst);
        assert_eq!(set.len(), 2);
    }

    // ========== PathLayoutConfig Tests ==========

    #[test]
    fn test_path_layout_config_default() {
        let config = PathLayoutConfig::default();
        assert_eq!(config.layout(), PathLayout::PathFirst);
        assert_eq!(config.alignment(), 64);
    }

    #[test]
    fn test_path_layout_config_new() {
        let config = PathLayoutConfig::new(PathLayout::TimeStepFirst, 128);
        assert_eq!(config.layout(), PathLayout::TimeStepFirst);
        assert_eq!(config.alignment(), 128);
    }

    #[test]
    fn test_path_layout_config_with_layout() {
        let config = PathLayoutConfig::with_layout(PathLayout::TimeStepFirst);
        assert_eq!(config.layout(), PathLayout::TimeStepFirst);
        assert_eq!(config.alignment(), PathLayoutConfig::DEFAULT_ALIGNMENT);
    }

    #[test]
    fn test_path_layout_config_validate_valid() {
        let valid_alignments = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024];
        for alignment in valid_alignments {
            let config = PathLayoutConfig::new(PathLayout::PathFirst, alignment);
            assert!(
                config.validate().is_ok(),
                "alignment {} should be valid",
                alignment
            );
        }
    }

    #[test]
    fn test_path_layout_config_validate_invalid_alignment() {
        let invalid_alignments = [0, 3, 5, 6, 7, 9, 10, 12, 15, 17, 63, 65, 100];
        for alignment in invalid_alignments {
            let config = PathLayoutConfig::new(PathLayout::PathFirst, alignment);
            let result = config.validate();
            assert!(
                matches!(result, Err(LayoutConfigError::InvalidAlignment(_))),
                "alignment {} should be invalid",
                alignment
            );
        }
    }

    #[test]
    fn test_path_layout_config_clone_copy() {
        let config = PathLayoutConfig::new(PathLayout::TimeStepFirst, 64);
        let cloned = config;
        assert_eq!(config, cloned);
    }

    // ========== StreamingConfig Tests ==========

    #[test]
    fn test_streaming_config_default() {
        let config = StreamingConfig::default();
        assert!(!config.enabled());
        assert_eq!(config.buffer_steps(), 2);
    }

    #[test]
    fn test_streaming_config_enabled() {
        let config = StreamingConfig::enabled();
        assert!(config.enabled());
        assert_eq!(config.buffer_steps(), StreamingConfig::MIN_BUFFER_STEPS);
    }

    #[test]
    fn test_streaming_config_disabled() {
        let config = StreamingConfig::disabled();
        assert!(!config.enabled());
        assert_eq!(config.buffer_steps(), StreamingConfig::MIN_BUFFER_STEPS);
    }

    #[test]
    fn test_streaming_config_new() {
        let config = StreamingConfig::new(true, 3);
        assert!(config.enabled());
        assert_eq!(config.buffer_steps(), 3);
    }

    #[test]
    fn test_streaming_config_validate_valid() {
        for buffer_steps in [2, 3, 4, 10, 100] {
            let config = StreamingConfig::new(true, buffer_steps);
            assert!(
                config.validate().is_ok(),
                "buffer_steps {} should be valid",
                buffer_steps
            );
        }
    }

    #[test]
    fn test_streaming_config_validate_invalid_buffer_steps() {
        for buffer_steps in [0, 1] {
            let config = StreamingConfig::new(true, buffer_steps);
            let result = config.validate();
            assert!(
                matches!(result, Err(LayoutConfigError::InvalidBufferSteps(_))),
                "buffer_steps {} should be invalid",
                buffer_steps
            );
        }
    }

    #[test]
    fn test_streaming_config_clone_copy() {
        let config = StreamingConfig::new(true, 4);
        let cloned = config;
        assert_eq!(config, cloned);
    }

    #[test]
    fn test_streaming_config_debug() {
        let config = StreamingConfig::enabled();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("enabled: true"));
        assert!(debug_str.contains("buffer_steps: 2"));
    }
}
