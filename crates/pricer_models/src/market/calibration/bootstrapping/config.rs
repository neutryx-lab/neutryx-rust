//! Bootstrap configuration types.
//!
//! This module provides configuration structures for yield curve bootstrapping
//! with generic type support for automatic differentiation compatibility.

use num_traits::Float;
use pricer_core::math::numeric::from_f64;

/// Bootstrap interpolation methods ordered by industry usage frequency.
///
/// Determines how discount factors are interpolated between pillar points.
/// All methods are AAD-compatible with smooth approximations.
///
/// # Ordering Rationale
///
/// Variants are ordered by industry usage frequency:
/// 1. `LogLinear` - Most common, industry default for discount curves
/// 2. `FlatForward` - Second most common, constant forward between pillars
/// 3. `LinearZeroRate` - Simple linear interpolation on zero rates
/// 4. `CubicSpline` - Smooth interpolation for presentation purposes
/// 5. `MonotonicCubic` - Advanced method to prevent arbitrage
///
/// # Adding New Variants
///
/// When adding new interpolation methods, place them according to their
/// expected industry usage frequency.
///
/// # Example
///
/// ```
/// use pricer_models::market::calibration::bootstrapping::BootstrapInterpolation;
///
/// // LogLinear is the default (most commonly used)
/// assert_eq!(BootstrapInterpolation::default(), BootstrapInterpolation::LogLinear);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BootstrapInterpolation {
    /// Log-linear interpolation (default) - piecewise constant forward rates.
    /// Most commonly used in industry for discount curve construction.
    #[default]
    LogLinear,
    /// Flat forward interpolation - constant forward between pillars.
    /// Second most common, useful for simple curve construction.
    FlatForward,
    /// Linear interpolation on zero rates.
    /// Simple and intuitive, but may produce non-smooth forwards.
    LinearZeroRate,
    /// Cubic spline interpolation on zero rates.
    /// Produces smooth curves, primarily used for presentation.
    CubicSpline,
    /// Monotonic cubic interpolation - prevents arbitrage.
    /// Ensures monotonicity of discount factors.
    MonotonicCubic,
}

/// Configuration for yield curve bootstrapping.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
///
/// # Examples
///
/// ```
/// use pricer_models::market::calibration::bootstrapping::GenericBootstrapConfig;
///
/// // Default configuration
/// let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::default();
///
/// // Fluent configuration
/// let config = GenericBootstrapConfig::<f64>::default()
///     .with_tolerance(1e-14)
///     .with_max_iterations(200);
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(bound(serialize = "T: serde::Serialize", deserialize = "T: serde::Deserialize<'de>"))
)]
pub struct GenericBootstrapConfig<T: Float> {
    /// Convergence tolerance for solver. Default: 1e-12
    pub tolerance: T,
    /// Maximum iterations per pillar. Default: 100
    pub max_iterations: usize,
    /// Interpolation method. Default: LogLinear
    pub interpolation: BootstrapInterpolation,
    /// Allow extrapolation beyond pillar range. Default: true
    pub allow_extrapolation: bool,
    /// Allow negative rates. Default: false
    pub allow_negative_rates: bool,
    /// Maximum supported maturity in years. Default: 50.0
    pub max_maturity: T,
}

impl<T: Float> Default for GenericBootstrapConfig<T> {
    fn default() -> Self {
        Self {
            tolerance: from_f64(1e-12),
            max_iterations: 100,
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
            allow_negative_rates: false,
            max_maturity: from_f64(50.0),
        }
    }
}

impl<T: Float> GenericBootstrapConfig<T> {
    /// Create a new configuration with default values.
    pub fn new() -> Self { Self::default() }

    /// Create a builder (deprecated: use `with_*` methods instead).
    #[deprecated(since = "0.8.0", note = "Use GenericBootstrapConfig::default().with_*() instead")]
    pub fn builder() -> GenericBootstrapConfigBuilder<T> { GenericBootstrapConfigBuilder::new() }

    /// High-precision configuration (tolerance: 1e-14, iterations: 500).
    pub fn high_precision() -> Self {
        Self {
            tolerance: from_f64(1e-14),
            max_iterations: 500,
            ..Self::default()
        }
    }

    /// Fast configuration for interactive use (tolerance: 1e-8, iterations: 50).
    pub fn fast() -> Self {
        Self {
            tolerance: from_f64(1e-8),
            max_iterations: 50,
            ..Self::default()
        }
    }

    /// Set the convergence tolerance.
    pub fn with_tolerance(mut self, tolerance: T) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set the maximum iterations.
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Set the interpolation method.
    pub fn with_interpolation(mut self, interpolation: BootstrapInterpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Set whether extrapolation is allowed.
    pub fn with_extrapolation(mut self, allow: bool) -> Self {
        self.allow_extrapolation = allow;
        self
    }

    /// Set whether negative rates are allowed.
    pub fn with_negative_rates(mut self, allow: bool) -> Self {
        self.allow_negative_rates = allow;
        self
    }

    /// Set the maximum maturity.
    pub fn with_max_maturity(mut self, max_maturity: T) -> Self {
        self.max_maturity = max_maturity;
        self
    }
}

/// Builder for `GenericBootstrapConfig` (kept for backward compatibility).
#[derive(Debug, Clone)]
pub struct GenericBootstrapConfigBuilder<T: Float> {
    config: GenericBootstrapConfig<T>,
}

impl<T: Float> GenericBootstrapConfigBuilder<T> {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self {
            config: GenericBootstrapConfig::default(),
        }
    }

    /// Set the convergence tolerance.
    pub fn tolerance(mut self, tolerance: T) -> Self {
        self.config.tolerance = tolerance;
        self
    }

    /// Set the maximum iterations.
    pub fn max_iterations(mut self, max_iterations: usize) -> Self {
        self.config.max_iterations = max_iterations;
        self
    }

    /// Set the interpolation method.
    pub fn interpolation(mut self, interpolation: BootstrapInterpolation) -> Self {
        self.config.interpolation = interpolation;
        self
    }

    /// Set whether extrapolation is allowed.
    pub fn allow_extrapolation(mut self, allow: bool) -> Self {
        self.config.allow_extrapolation = allow;
        self
    }

    /// Set whether negative rates are allowed.
    pub fn allow_negative_rates(mut self, allow: bool) -> Self {
        self.config.allow_negative_rates = allow;
        self
    }

    /// Set the maximum maturity.
    pub fn max_maturity(mut self, max_maturity: T) -> Self {
        self.config.max_maturity = max_maturity;
        self
    }

    /// Build the configuration.
    pub fn build(self) -> GenericBootstrapConfig<T> { self.config }
}

impl<T: Float> Default for GenericBootstrapConfigBuilder<T> {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    // ========================================
    // Default and Preset Tests
    // ========================================

    #[test]
    fn test_default_config() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::default();
        assert!((config.tolerance - 1e-12).abs() < 1e-17);
        assert_eq!(config.max_iterations, 100);
        assert_eq!(config.interpolation, BootstrapInterpolation::LogLinear);
        assert!(config.allow_extrapolation);
        assert!(!config.allow_negative_rates);
        assert!((config.max_maturity - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_high_precision_config() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::high_precision();
        assert!(config.tolerance < 1e-12);
        assert!(config.max_iterations >= 500);
    }

    #[test]
    fn test_fast_config() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::fast();
        assert!(config.tolerance > 1e-10);
        assert!(config.max_iterations <= 50);
    }

    // ========================================
    // Parameterized With Method Tests
    // ========================================

    #[rstest]
    #[case(1e-14)]
    #[case(1e-8)]
    #[case(1e-16)]
    fn test_with_tolerance(#[case] tol: f64) {
        let config = GenericBootstrapConfig::<f64>::default().with_tolerance(tol);
        assert!((config.tolerance - tol).abs() < 1e-20);
    }

    #[rstest]
    #[case(50)]
    #[case(200)]
    #[case(1000)]
    fn test_with_max_iterations(#[case] iters: usize) {
        let config = GenericBootstrapConfig::<f64>::default().with_max_iterations(iters);
        assert_eq!(config.max_iterations, iters);
    }

    #[rstest]
    #[case(BootstrapInterpolation::LogLinear)]
    #[case(BootstrapInterpolation::LinearZeroRate)]
    #[case(BootstrapInterpolation::CubicSpline)]
    #[case(BootstrapInterpolation::MonotonicCubic)]
    #[case(BootstrapInterpolation::FlatForward)]
    fn test_with_interpolation(#[case] interp: BootstrapInterpolation) {
        let config = GenericBootstrapConfig::<f64>::default().with_interpolation(interp);
        assert_eq!(config.interpolation, interp);
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    fn test_with_extrapolation(#[case] allow: bool) {
        let config = GenericBootstrapConfig::<f64>::default().with_extrapolation(allow);
        assert_eq!(config.allow_extrapolation, allow);
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    fn test_with_negative_rates(#[case] allow: bool) {
        let config = GenericBootstrapConfig::<f64>::default().with_negative_rates(allow);
        assert_eq!(config.allow_negative_rates, allow);
    }

    #[rstest]
    #[case(30.0)]
    #[case(75.0)]
    #[case(100.0)]
    fn test_with_max_maturity(#[case] mat: f64) {
        let config = GenericBootstrapConfig::<f64>::default().with_max_maturity(mat);
        assert!((config.max_maturity - mat).abs() < 1e-10);
    }

    // ========================================
    // Chained Configuration Test
    // ========================================

    #[test]
    fn test_chained_configuration() {
        let config = GenericBootstrapConfig::<f64>::default()
            .with_tolerance(1e-14)
            .with_max_iterations(200)
            .with_interpolation(BootstrapInterpolation::MonotonicCubic)
            .with_extrapolation(false)
            .with_negative_rates(true)
            .with_max_maturity(60.0);

        assert!((config.tolerance - 1e-14).abs() < 1e-19);
        assert_eq!(config.max_iterations, 200);
        assert_eq!(config.interpolation, BootstrapInterpolation::MonotonicCubic);
        assert!(!config.allow_extrapolation);
        assert!(config.allow_negative_rates);
        assert!((config.max_maturity - 60.0).abs() < 1e-10);
    }

    // ========================================
    // Legacy Builder Tests (backward compat)
    // ========================================

    #[test]
    #[allow(deprecated)]
    fn test_legacy_builder() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::builder()
            .tolerance(1e-14)
            .max_iterations(200)
            .build();
        assert!((config.tolerance - 1e-14).abs() < 1e-19);
        assert_eq!(config.max_iterations, 200);
    }

    // ========================================
    // Type Parameter Tests
    // ========================================

    #[test]
    fn test_config_with_f32() {
        let config: GenericBootstrapConfig<f32> = GenericBootstrapConfig::default();
        assert!(config.tolerance > 0.0);
        assert_eq!(config.max_iterations, 100);
    }

    // ========================================
    // Interpolation Enum Tests
    // ========================================

    #[test]
    fn test_interpolation_default() {
        assert_eq!(
            BootstrapInterpolation::default(),
            BootstrapInterpolation::LogLinear
        );
    }

    #[test]
    fn test_interpolation_clone_copy() {
        let interp1 = BootstrapInterpolation::CubicSpline;
        let interp2 = interp1; // Copy
        let interp3 = interp1.clone();
        assert_eq!(interp1, interp2);
        assert_eq!(interp1, interp3);
    }
}
