//! Bootstrap configuration types.
//!
//! This module provides configuration structures for yield curve bootstrapping
//! with generic type support for automatic differentiation compatibility.

use num_traits::Float;

/// Interpolation method for bootstrapped curves.
///
/// Determines how discount factors are interpolated between pillar points.
/// All methods are AAD-compatible with smooth approximations.
///
/// # Variants
///
/// - `LogLinear`: Linear interpolation on log(DF) - default, arbitrage-free
/// - `LinearZeroRate`: Linear interpolation on zero rates
/// - `CubicSpline`: Cubic spline interpolation on zero rates
/// - `MonotonicCubic`: Monotone-preserving cubic interpolation
/// - `FlatForward`: Piecewise constant forward rates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BootstrapInterpolation {
    /// Log-linear interpolation (default).
    ///
    /// Interpolates linearly on log(discount_factor), which is equivalent
    /// to assuming piecewise constant forward rates between pillars.
    /// This is the most common method in practice.
    #[default]
    LogLinear,

    /// Linear interpolation on zero rates.
    ///
    /// Simple linear interpolation on continuously compounded zero rates.
    /// May produce small arbitrage opportunities.
    LinearZeroRate,

    /// Cubic spline interpolation.
    ///
    /// Smooth interpolation using natural cubic splines on zero rates.
    /// Provides continuous second derivatives.
    CubicSpline,

    /// Monotonic cubic interpolation.
    ///
    /// Uses Fritsch-Carlson or similar algorithm to ensure monotonicity
    /// of discount factors, preventing arbitrage.
    MonotonicCubic,

    /// Flat forward interpolation.
    ///
    /// Assumes constant forward rate between each pair of pillars.
    /// Produces discontinuous forward rates at pillars.
    FlatForward,
}

/// Configuration for yield curve bootstrapping.
///
/// Provides all parameters needed to control the bootstrapping process,
/// including convergence criteria, interpolation method, and validation options.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
///
/// # Examples
///
/// ```
/// use pricer_optimiser::bootstrapping::GenericBootstrapConfig;
///
/// // Use default configuration
/// let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::default();
/// assert!(config.tolerance < 1e-10);
///
/// // Custom configuration
/// let config = GenericBootstrapConfig::<f64>::builder()
///     .tolerance(1e-14)
///     .max_iterations(200)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct GenericBootstrapConfig<T: Float> {
    /// Convergence tolerance for solver.
    ///
    /// The solver stops when the residual is below this value.
    /// Default: 1e-12
    pub tolerance: T,

    /// Maximum number of iterations per pillar.
    ///
    /// If the solver doesn't converge within this limit,
    /// it falls back to Brent method or returns an error.
    /// Default: 100
    pub max_iterations: usize,

    /// Interpolation method for the resulting curve.
    ///
    /// Determines how discount factors are interpolated between pillars.
    /// Default: LogLinear
    pub interpolation: BootstrapInterpolation,

    /// Allow extrapolation beyond pillar range.
    ///
    /// If true, queries outside the pillar range use flat extrapolation.
    /// If false, such queries return an error.
    /// Default: true
    pub allow_extrapolation: bool,

    /// Allow negative rates.
    ///
    /// If true, negative rates produce a warning but bootstrapping continues.
    /// If false, negative rates cause an error.
    /// Default: false
    pub allow_negative_rates: bool,

    /// Maximum supported maturity in years.
    ///
    /// Instruments with maturity beyond this value are rejected.
    /// Default: 50.0
    pub max_maturity: T,
}

impl<T: Float> Default for GenericBootstrapConfig<T> {
    fn default() -> Self {
        Self {
            tolerance: T::from(1e-12).unwrap(),
            max_iterations: 100,
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
            allow_negative_rates: false,
            max_maturity: T::from(50.0).unwrap(),
        }
    }
}

impl<T: Float> GenericBootstrapConfig<T> {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration builder for fluent construction.
    pub fn builder() -> GenericBootstrapConfigBuilder<T> {
        GenericBootstrapConfigBuilder::new()
    }

    /// Create a high-precision configuration.
    ///
    /// Uses tighter tolerance (1e-14) and more iterations (500).
    pub fn high_precision() -> Self {
        Self {
            tolerance: T::from(1e-14).unwrap(),
            max_iterations: 500,
            ..Self::default()
        }
    }

    /// Create a fast configuration for interactive use.
    ///
    /// Uses relaxed tolerance (1e-8) and fewer iterations (50).
    pub fn fast() -> Self {
        Self {
            tolerance: T::from(1e-8).unwrap(),
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

/// Builder for `GenericBootstrapConfig`.
///
/// Provides a fluent interface for constructing bootstrap configurations.
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
    pub fn build(self) -> GenericBootstrapConfig<T> {
        self.config
    }
}

impl<T: Float> Default for GenericBootstrapConfigBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Default Configuration Tests
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
    fn test_new_equals_default() {
        let config1: GenericBootstrapConfig<f64> = GenericBootstrapConfig::new();
        let config2: GenericBootstrapConfig<f64> = GenericBootstrapConfig::default();
        assert!((config1.tolerance - config2.tolerance).abs() < 1e-17);
        assert_eq!(config1.max_iterations, config2.max_iterations);
    }

    // ========================================
    // Preset Configuration Tests
    // ========================================

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
    // Builder Tests
    // ========================================

    #[test]
    fn test_builder_default() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::builder().build();
        assert!((config.tolerance - 1e-12).abs() < 1e-17);
    }

    #[test]
    fn test_builder_tolerance() {
        let config: GenericBootstrapConfig<f64> =
            GenericBootstrapConfig::builder().tolerance(1e-14).build();
        assert!((config.tolerance - 1e-14).abs() < 1e-19);
    }

    #[test]
    fn test_builder_max_iterations() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::builder()
            .max_iterations(200)
            .build();
        assert_eq!(config.max_iterations, 200);
    }

    #[test]
    fn test_builder_interpolation() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::builder()
            .interpolation(BootstrapInterpolation::CubicSpline)
            .build();
        assert_eq!(config.interpolation, BootstrapInterpolation::CubicSpline);
    }

    #[test]
    fn test_builder_allow_extrapolation() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::builder()
            .allow_extrapolation(false)
            .build();
        assert!(!config.allow_extrapolation);
    }

    #[test]
    fn test_builder_allow_negative_rates() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::builder()
            .allow_negative_rates(true)
            .build();
        assert!(config.allow_negative_rates);
    }

    #[test]
    fn test_builder_max_maturity() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::builder()
            .max_maturity(100.0)
            .build();
        assert!((config.max_maturity - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_builder_chained() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::builder()
            .tolerance(1e-14)
            .max_iterations(200)
            .interpolation(BootstrapInterpolation::MonotonicCubic)
            .allow_extrapolation(false)
            .allow_negative_rates(true)
            .max_maturity(60.0)
            .build();

        assert!((config.tolerance - 1e-14).abs() < 1e-19);
        assert_eq!(config.max_iterations, 200);
        assert_eq!(config.interpolation, BootstrapInterpolation::MonotonicCubic);
        assert!(!config.allow_extrapolation);
        assert!(config.allow_negative_rates);
        assert!((config.max_maturity - 60.0).abs() < 1e-10);
    }

    // ========================================
    // With Method Tests
    // ========================================

    #[test]
    fn test_with_tolerance() {
        let config: GenericBootstrapConfig<f64> =
            GenericBootstrapConfig::default().with_tolerance(1e-14);
        assert!((config.tolerance - 1e-14).abs() < 1e-19);
    }

    #[test]
    fn test_with_max_iterations() {
        let config: GenericBootstrapConfig<f64> =
            GenericBootstrapConfig::default().with_max_iterations(200);
        assert_eq!(config.max_iterations, 200);
    }

    #[test]
    fn test_with_interpolation() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::default()
            .with_interpolation(BootstrapInterpolation::FlatForward);
        assert_eq!(config.interpolation, BootstrapInterpolation::FlatForward);
    }

    #[test]
    fn test_with_extrapolation() {
        let config: GenericBootstrapConfig<f64> =
            GenericBootstrapConfig::default().with_extrapolation(false);
        assert!(!config.allow_extrapolation);
    }

    #[test]
    fn test_with_negative_rates() {
        let config: GenericBootstrapConfig<f64> =
            GenericBootstrapConfig::default().with_negative_rates(true);
        assert!(config.allow_negative_rates);
    }

    #[test]
    fn test_with_max_maturity() {
        let config: GenericBootstrapConfig<f64> =
            GenericBootstrapConfig::default().with_max_maturity(75.0);
        assert!((config.max_maturity - 75.0).abs() < 1e-10);
    }

    // ========================================
    // Interpolation Enum Tests
    // ========================================

    #[test]
    fn test_interpolation_default() {
        let interp: BootstrapInterpolation = Default::default();
        assert_eq!(interp, BootstrapInterpolation::LogLinear);
    }

    #[test]
    fn test_interpolation_clone() {
        let interp1 = BootstrapInterpolation::CubicSpline;
        let interp2 = interp1.clone();
        assert_eq!(interp1, interp2);
    }

    #[test]
    fn test_interpolation_copy() {
        let interp1 = BootstrapInterpolation::MonotonicCubic;
        let interp2 = interp1; // Copy
        assert_eq!(interp1, interp2);
    }

    #[test]
    fn test_interpolation_debug() {
        let interp = BootstrapInterpolation::FlatForward;
        let debug_str = format!("{:?}", interp);
        assert!(debug_str.contains("FlatForward"));
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

    #[test]
    fn test_builder_with_f32() {
        let config: GenericBootstrapConfig<f32> = GenericBootstrapConfig::builder()
            .tolerance(1e-6_f32)
            .max_iterations(50)
            .build();
        assert!((config.tolerance - 1e-6_f32).abs() < 1e-10);
        assert_eq!(config.max_iterations, 50);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_config_clone() {
        let config1: GenericBootstrapConfig<f64> = GenericBootstrapConfig::builder()
            .tolerance(1e-14)
            .max_iterations(200)
            .build();
        let config2 = config1.clone();
        assert!((config1.tolerance - config2.tolerance).abs() < 1e-19);
        assert_eq!(config1.max_iterations, config2.max_iterations);
    }

    #[test]
    fn test_builder_clone() {
        let builder1: GenericBootstrapConfigBuilder<f64> =
            GenericBootstrapConfig::builder().tolerance(1e-14);
        let builder2 = builder1.clone();
        let config1 = builder1.build();
        let config2 = builder2.build();
        assert!((config1.tolerance - config2.tolerance).abs() < 1e-19);
    }
}
