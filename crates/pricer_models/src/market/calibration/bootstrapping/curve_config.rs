//! Extended curve configuration types.
//!
//! This module provides the `CurveConfig` structure that extends
//! `GenericBootstrapConfig` with parameter representation settings.

use num_traits::Float;

use super::config::{BootstrapInterpolation, GenericBootstrapConfig};
use super::engine_error::{CurveEngineError, CurveParameterRepresentation};

/// Extended configuration for curve bootstrapping.
///
/// This structure wraps `GenericBootstrapConfig` and adds parameter
/// representation settings to control how curve values are stored
/// and interpolated internally.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
///
/// # Examples
///
/// ```
/// use pricer_models::market::calibration::bootstrapping::{
///     CurveConfig, CurveParameterRepresentation, BootstrapInterpolation,
/// };
///
/// // Use default configuration
/// let config: CurveConfig<f64> = CurveConfig::default();
/// assert_eq!(config.parameter_representation, CurveParameterRepresentation::LogDiscountFactor);
///
/// // Custom configuration with builder
/// let config = CurveConfig::<f64>::builder()
///     .parameter_representation(CurveParameterRepresentation::ZeroRate)
///     .interpolation(BootstrapInterpolation::LinearZeroRate)
///     .build();
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(bound(serialize = "T: serde::Serialize", deserialize = "T: serde::Deserialize<'de>"))
)]
pub struct CurveConfig<T: Float> {
    /// Basic bootstrap configuration.
    ///
    /// Contains tolerance, max_iterations, interpolation method,
    /// extrapolation settings, and negative rate handling.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub bootstrap: GenericBootstrapConfig<T>,

    /// Parameter representation for internal storage.
    ///
    /// Determines how curve values are stored and interpolated:
    /// - `LogDiscountFactor`: Store log(DF), interpolate for flat forwards
    /// - `ZeroRate`: Store zero rates, interpolate directly
    /// - `InstantaneousForward`: Store forward rates
    #[cfg_attr(feature = "serde", serde(default))]
    pub parameter_representation: CurveParameterRepresentation,
}

impl<T: Float> Default for CurveConfig<T> {
    fn default() -> Self {
        Self {
            bootstrap: GenericBootstrapConfig::default(),
            parameter_representation: CurveParameterRepresentation::LogDiscountFactor,
        }
    }
}

impl<T: Float> CurveConfig<T> {
    /// Create a new configuration with default values.
    pub fn new() -> Self { Self::default() }

    /// Create a configuration builder for fluent construction.
    pub fn builder() -> CurveConfigBuilder<T> { CurveConfigBuilder::new() }

    /// Create a high-precision configuration.
    ///
    /// Uses tighter tolerance (1e-14) and more iterations (500).
    pub fn high_precision() -> Self {
        Self {
            bootstrap: GenericBootstrapConfig::high_precision(),
            parameter_representation: CurveParameterRepresentation::LogDiscountFactor,
        }
    }

    /// Create a fast configuration for interactive use.
    ///
    /// Uses relaxed tolerance (1e-8) and fewer iterations (50).
    pub fn fast() -> Self {
        Self {
            bootstrap: GenericBootstrapConfig::fast(),
            parameter_representation: CurveParameterRepresentation::LogDiscountFactor,
        }
    }

    /// Validate the configuration for internal consistency.
    ///
    /// Checks that the parameter representation and interpolation
    /// method are compatible. Some combinations may produce
    /// suboptimal results or numerical issues.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the configuration is valid, or `Err(CurveEngineError)`
    /// describing the invalid combination.
    ///
    /// # Valid Combinations
    ///
    /// - `LogDiscountFactor` + `LogLinear` (default, recommended)
    /// - `LogDiscountFactor` + `FlatForward`
    /// - `ZeroRate` + `LinearZeroRate`
    /// - `ZeroRate` + `CubicSpline`
    /// - `ZeroRate` + `MonotonicCubic`
    /// - Any combination with `MonotonicCubic` (preserves monotonicity)
    ///
    /// # Warnings (allowed but suboptimal)
    ///
    /// - `LogDiscountFactor` + `LinearZeroRate` (works but less natural)
    /// - `ZeroRate` + `LogLinear` (works but less natural)
    pub fn validate(&self) -> Result<(), CurveEngineError> {
        use BootstrapInterpolation::*;
        use CurveParameterRepresentation::*;

        match (self.parameter_representation, self.bootstrap.interpolation) {
            // Optimal combinations
            (LogDiscountFactor, LogLinear) => Ok(()),
            (LogDiscountFactor, FlatForward) => Ok(()),
            (ZeroRate, LinearZeroRate) => Ok(()),
            (ZeroRate, CubicSpline) => Ok(()),

            // MonotonicCubic works with any representation
            (_, MonotonicCubic) => Ok(()),

            // InstantaneousForward requires special handling
            (InstantaneousForward, FlatForward) => Ok(()),
            (InstantaneousForward, _) => Err(CurveEngineError::invalid_config(
                self.parameter_representation,
                self.bootstrap.interpolation,
            )),

            // Suboptimal but allowed combinations
            (LogDiscountFactor, LinearZeroRate) => Ok(()),
            (LogDiscountFactor, CubicSpline) => Ok(()),
            (ZeroRate, LogLinear) => Ok(()),
            (ZeroRate, FlatForward) => Ok(()),
        }
    }

    /// Set the parameter representation.
    pub fn with_parameter_representation(
        mut self,
        repr: CurveParameterRepresentation,
    ) -> Self {
        self.parameter_representation = repr;
        self
    }

    /// Set the convergence tolerance.
    pub fn with_tolerance(mut self, tolerance: T) -> Self {
        self.bootstrap.tolerance = tolerance;
        self
    }

    /// Set the maximum iterations.
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.bootstrap.max_iterations = max_iterations;
        self
    }

    /// Set the interpolation method.
    pub fn with_interpolation(mut self, interpolation: BootstrapInterpolation) -> Self {
        self.bootstrap.interpolation = interpolation;
        self
    }

    /// Set whether extrapolation is allowed.
    pub fn with_extrapolation(mut self, allow: bool) -> Self {
        self.bootstrap.allow_extrapolation = allow;
        self
    }

    /// Set whether negative rates are allowed.
    pub fn with_negative_rates(mut self, allow: bool) -> Self {
        self.bootstrap.allow_negative_rates = allow;
        self
    }

    /// Set the maximum maturity.
    pub fn with_max_maturity(mut self, max_maturity: T) -> Self {
        self.bootstrap.max_maturity = max_maturity;
        self
    }
}

/// Builder for `CurveConfig`.
///
/// Provides a fluent interface for constructing curve configurations.
#[derive(Debug, Clone)]
pub struct CurveConfigBuilder<T: Float> {
    config: CurveConfig<T>,
}

impl<T: Float> CurveConfigBuilder<T> {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self {
            config: CurveConfig::default(),
        }
    }

    /// Set the parameter representation.
    pub fn parameter_representation(mut self, repr: CurveParameterRepresentation) -> Self {
        self.config.parameter_representation = repr;
        self
    }

    /// Set the convergence tolerance.
    pub fn tolerance(mut self, tolerance: T) -> Self {
        self.config.bootstrap.tolerance = tolerance;
        self
    }

    /// Set the maximum iterations.
    pub fn max_iterations(mut self, max_iterations: usize) -> Self {
        self.config.bootstrap.max_iterations = max_iterations;
        self
    }

    /// Set the interpolation method.
    pub fn interpolation(mut self, interpolation: BootstrapInterpolation) -> Self {
        self.config.bootstrap.interpolation = interpolation;
        self
    }

    /// Set whether extrapolation is allowed.
    pub fn allow_extrapolation(mut self, allow: bool) -> Self {
        self.config.bootstrap.allow_extrapolation = allow;
        self
    }

    /// Set whether negative rates are allowed.
    pub fn allow_negative_rates(mut self, allow: bool) -> Self {
        self.config.bootstrap.allow_negative_rates = allow;
        self
    }

    /// Set the maximum maturity.
    pub fn max_maturity(mut self, max_maturity: T) -> Self {
        self.config.bootstrap.max_maturity = max_maturity;
        self
    }

    /// Build the configuration.
    pub fn build(self) -> CurveConfig<T> { self.config }

    /// Build and validate the configuration.
    ///
    /// Returns an error if the configuration is invalid.
    pub fn build_validated(self) -> Result<CurveConfig<T>, CurveEngineError> {
        let config = self.config;
        config.validate()?;
        Ok(config)
    }
}

impl<T: Float> Default for CurveConfigBuilder<T> {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Default Configuration Tests
    // ========================================

    #[test]
    fn test_default_config() {
        let config: CurveConfig<f64> = CurveConfig::default();
        assert_eq!(
            config.parameter_representation,
            CurveParameterRepresentation::LogDiscountFactor
        );
        assert!((config.bootstrap.tolerance - 1e-12).abs() < 1e-17);
        assert_eq!(config.bootstrap.max_iterations, 100);
        assert_eq!(config.bootstrap.interpolation, BootstrapInterpolation::LogLinear);
    }

    #[test]
    fn test_new_equals_default() {
        let config1: CurveConfig<f64> = CurveConfig::new();
        let config2: CurveConfig<f64> = CurveConfig::default();
        assert_eq!(config1.parameter_representation, config2.parameter_representation);
        assert!((config1.bootstrap.tolerance - config2.bootstrap.tolerance).abs() < 1e-17);
    }

    // ========================================
    // Preset Configuration Tests
    // ========================================

    #[test]
    fn test_high_precision_config() {
        let config: CurveConfig<f64> = CurveConfig::high_precision();
        assert!(config.bootstrap.tolerance < 1e-12);
        assert!(config.bootstrap.max_iterations >= 500);
        assert_eq!(
            config.parameter_representation,
            CurveParameterRepresentation::LogDiscountFactor
        );
    }

    #[test]
    fn test_fast_config() {
        let config: CurveConfig<f64> = CurveConfig::fast();
        assert!(config.bootstrap.tolerance > 1e-10);
        assert!(config.bootstrap.max_iterations <= 50);
    }

    // ========================================
    // Validation Tests
    // ========================================

    #[test]
    fn test_validate_default_config() {
        let config: CurveConfig<f64> = CurveConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_log_df_log_linear() {
        let config: CurveConfig<f64> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::LogDiscountFactor)
            .interpolation(BootstrapInterpolation::LogLinear)
            .build();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_rate_linear_zero_rate() {
        let config: CurveConfig<f64> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::ZeroRate)
            .interpolation(BootstrapInterpolation::LinearZeroRate)
            .build();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_rate_cubic_spline() {
        let config: CurveConfig<f64> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::ZeroRate)
            .interpolation(BootstrapInterpolation::CubicSpline)
            .build();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_monotonic_cubic_any_repr() {
        // MonotonicCubic should work with any representation
        for repr in [
            CurveParameterRepresentation::LogDiscountFactor,
            CurveParameterRepresentation::ZeroRate,
            CurveParameterRepresentation::InstantaneousForward,
        ] {
            let config: CurveConfig<f64> = CurveConfig::builder()
                .parameter_representation(repr)
                .interpolation(BootstrapInterpolation::MonotonicCubic)
                .build();
            assert!(config.validate().is_ok(), "Failed for {:?}", repr);
        }
    }

    #[test]
    fn test_validate_instantaneous_forward_invalid() {
        let config: CurveConfig<f64> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::InstantaneousForward)
            .interpolation(BootstrapInterpolation::LogLinear)
            .build();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_instantaneous_forward_flat_forward() {
        let config: CurveConfig<f64> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::InstantaneousForward)
            .interpolation(BootstrapInterpolation::FlatForward)
            .build();
        assert!(config.validate().is_ok());
    }

    // ========================================
    // Builder Tests
    // ========================================

    #[test]
    fn test_builder_default() {
        let config: CurveConfig<f64> = CurveConfig::builder().build();
        assert_eq!(
            config.parameter_representation,
            CurveParameterRepresentation::LogDiscountFactor
        );
    }

    #[test]
    fn test_builder_parameter_representation() {
        let config: CurveConfig<f64> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::ZeroRate)
            .build();
        assert_eq!(
            config.parameter_representation,
            CurveParameterRepresentation::ZeroRate
        );
    }

    #[test]
    fn test_builder_tolerance() {
        let config: CurveConfig<f64> = CurveConfig::builder().tolerance(1e-14).build();
        assert!((config.bootstrap.tolerance - 1e-14).abs() < 1e-19);
    }

    #[test]
    fn test_builder_max_iterations() {
        let config: CurveConfig<f64> = CurveConfig::builder().max_iterations(200).build();
        assert_eq!(config.bootstrap.max_iterations, 200);
    }

    #[test]
    fn test_builder_interpolation() {
        let config: CurveConfig<f64> = CurveConfig::builder()
            .interpolation(BootstrapInterpolation::CubicSpline)
            .build();
        assert_eq!(config.bootstrap.interpolation, BootstrapInterpolation::CubicSpline);
    }

    #[test]
    fn test_builder_allow_extrapolation() {
        let config: CurveConfig<f64> = CurveConfig::builder().allow_extrapolation(false).build();
        assert!(!config.bootstrap.allow_extrapolation);
    }

    #[test]
    fn test_builder_allow_negative_rates() {
        let config: CurveConfig<f64> = CurveConfig::builder().allow_negative_rates(true).build();
        assert!(config.bootstrap.allow_negative_rates);
    }

    #[test]
    fn test_builder_max_maturity() {
        let config: CurveConfig<f64> = CurveConfig::builder().max_maturity(100.0).build();
        assert!((config.bootstrap.max_maturity - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_builder_chained() {
        let config: CurveConfig<f64> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::ZeroRate)
            .tolerance(1e-14)
            .max_iterations(200)
            .interpolation(BootstrapInterpolation::LinearZeroRate)
            .allow_extrapolation(false)
            .allow_negative_rates(true)
            .max_maturity(60.0)
            .build();

        assert_eq!(
            config.parameter_representation,
            CurveParameterRepresentation::ZeroRate
        );
        assert!((config.bootstrap.tolerance - 1e-14).abs() < 1e-19);
        assert_eq!(config.bootstrap.max_iterations, 200);
        assert_eq!(
            config.bootstrap.interpolation,
            BootstrapInterpolation::LinearZeroRate
        );
        assert!(!config.bootstrap.allow_extrapolation);
        assert!(config.bootstrap.allow_negative_rates);
        assert!((config.bootstrap.max_maturity - 60.0).abs() < 1e-10);
    }

    #[test]
    fn test_builder_validated_ok() {
        let result: Result<CurveConfig<f64>, _> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::ZeroRate)
            .interpolation(BootstrapInterpolation::LinearZeroRate)
            .build_validated();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_validated_err() {
        let result: Result<CurveConfig<f64>, _> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::InstantaneousForward)
            .interpolation(BootstrapInterpolation::CubicSpline)
            .build_validated();
        assert!(result.is_err());
    }

    // ========================================
    // With Method Tests
    // ========================================

    #[test]
    fn test_with_parameter_representation() {
        let config: CurveConfig<f64> = CurveConfig::default()
            .with_parameter_representation(CurveParameterRepresentation::ZeroRate);
        assert_eq!(
            config.parameter_representation,
            CurveParameterRepresentation::ZeroRate
        );
    }

    #[test]
    fn test_with_tolerance() {
        let config: CurveConfig<f64> = CurveConfig::default().with_tolerance(1e-14);
        assert!((config.bootstrap.tolerance - 1e-14).abs() < 1e-19);
    }

    #[test]
    fn test_with_max_iterations() {
        let config: CurveConfig<f64> = CurveConfig::default().with_max_iterations(200);
        assert_eq!(config.bootstrap.max_iterations, 200);
    }

    #[test]
    fn test_with_interpolation() {
        let config: CurveConfig<f64> =
            CurveConfig::default().with_interpolation(BootstrapInterpolation::FlatForward);
        assert_eq!(config.bootstrap.interpolation, BootstrapInterpolation::FlatForward);
    }

    #[test]
    fn test_with_extrapolation() {
        let config: CurveConfig<f64> = CurveConfig::default().with_extrapolation(false);
        assert!(!config.bootstrap.allow_extrapolation);
    }

    #[test]
    fn test_with_negative_rates() {
        let config: CurveConfig<f64> = CurveConfig::default().with_negative_rates(true);
        assert!(config.bootstrap.allow_negative_rates);
    }

    #[test]
    fn test_with_max_maturity() {
        let config: CurveConfig<f64> = CurveConfig::default().with_max_maturity(75.0);
        assert!((config.bootstrap.max_maturity - 75.0).abs() < 1e-10);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_config_clone() {
        let config1: CurveConfig<f64> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::ZeroRate)
            .tolerance(1e-14)
            .build();
        let config2 = config1.clone();
        assert_eq!(config1.parameter_representation, config2.parameter_representation);
        assert!((config1.bootstrap.tolerance - config2.bootstrap.tolerance).abs() < 1e-19);
    }

    #[test]
    fn test_builder_clone() {
        let builder1: CurveConfigBuilder<f64> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::ZeroRate);
        let builder2 = builder1.clone();
        let config1 = builder1.build();
        let config2 = builder2.build();
        assert_eq!(config1.parameter_representation, config2.parameter_representation);
    }

    // ========================================
    // Type Parameter Tests
    // ========================================

    #[test]
    fn test_config_with_f32() {
        let config: CurveConfig<f32> = CurveConfig::default();
        assert!(config.bootstrap.tolerance > 0.0);
        assert_eq!(config.bootstrap.max_iterations, 100);
    }

    #[test]
    fn test_builder_with_f32() {
        let config: CurveConfig<f32> = CurveConfig::builder()
            .tolerance(1e-6_f32)
            .max_iterations(50)
            .build();
        assert!((config.bootstrap.tolerance - 1e-6_f32).abs() < 1e-10);
    }
}
