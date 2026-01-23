//! Extended curve configuration types.

use std::ops::{Deref, DerefMut};

use num_traits::Float;

use super::{
    config::{BootstrapInterpolation, GenericBootstrapConfig},
    engine_error::{CurveEngineError, CurveParameterRepresentation},
};

/// Extended configuration for curve bootstrapping.
///
/// Wraps `GenericBootstrapConfig` and adds parameter representation settings.
/// Access inner config fields directly via `Deref` to `GenericBootstrapConfig`.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(bound(
        serialize = "T: serde::Serialize",
        deserialize = "T: serde::Deserialize<'de>"
    ))
)]
pub struct CurveConfig<T: Float> {
    /// Basic bootstrap configuration.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub bootstrap: GenericBootstrapConfig<T>,

    /// Parameter representation for internal storage.
    #[cfg_attr(feature = "serde", serde(default))]
    pub parameter_representation: CurveParameterRepresentation,
}

impl<T: Float> Deref for CurveConfig<T> {
    type Target = GenericBootstrapConfig<T>;
    fn deref(&self) -> &Self::Target { &self.bootstrap }
}

impl<T: Float> DerefMut for CurveConfig<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.bootstrap }
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

    /// Create a configuration builder.
    pub fn builder() -> CurveConfigBuilder<T> { CurveConfigBuilder::new() }

    /// High-precision configuration (tolerance: 1e-14, iterations: 500).
    pub fn high_precision() -> Self {
        Self {
            bootstrap: GenericBootstrapConfig::high_precision(),
            parameter_representation: CurveParameterRepresentation::LogDiscountFactor,
        }
    }

    /// Fast configuration for interactive use (tolerance: 1e-8, iterations:
    /// 50).
    pub fn fast() -> Self {
        Self {
            bootstrap: GenericBootstrapConfig::fast(),
            parameter_representation: CurveParameterRepresentation::LogDiscountFactor,
        }
    }

    /// Validate the configuration for internal consistency.
    pub fn validate(&self) -> Result<(), CurveEngineError> {
        use BootstrapInterpolation::*;
        use CurveParameterRepresentation::*;

        match (self.parameter_representation, self.bootstrap.interpolation) {
            // Optimal combinations
            (LogDiscountFactor, LogLinear | FlatForward) => Ok(()),
            (ZeroRate, LinearZeroRate | CubicSpline) => Ok(()),
            // MonotonicCubic works with any representation
            (_, MonotonicCubic) => Ok(()),
            // InstantaneousForward requires special handling
            (InstantaneousForward, FlatForward) => Ok(()),
            (InstantaneousForward, _) => Err(CurveEngineError::invalid_config(
                self.parameter_representation,
                self.bootstrap.interpolation,
            )),
            // Suboptimal but allowed combinations
            (LogDiscountFactor, LinearZeroRate | CubicSpline) => Ok(()),
            (ZeroRate, LogLinear | FlatForward) => Ok(()),
        }
    }

    /// Set the parameter representation.
    pub fn with_parameter_representation(mut self, repr: CurveParameterRepresentation) -> Self {
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
    use rstest::rstest;

    use super::*;

    // ========================================
    // Default and Preset Tests
    // ========================================

    #[test]
    fn test_default_config() {
        let config: CurveConfig<f64> = CurveConfig::default();
        assert_eq!(
            config.parameter_representation,
            CurveParameterRepresentation::LogDiscountFactor
        );
        assert!((config.tolerance - 1e-12).abs() < 1e-17);
        assert_eq!(config.max_iterations, 100);
        assert_eq!(config.interpolation, BootstrapInterpolation::LogLinear);
    }

    #[test]
    fn test_high_precision_config() {
        let config: CurveConfig<f64> = CurveConfig::high_precision();
        assert!(config.tolerance < 1e-12);
        assert!(config.max_iterations >= 500);
    }

    #[test]
    fn test_fast_config() {
        let config: CurveConfig<f64> = CurveConfig::fast();
        assert!(config.tolerance > 1e-10);
        assert!(config.max_iterations <= 50);
    }

    // ========================================
    // Parameterised Validation Tests
    // ========================================

    #[rstest]
    #[case(
        CurveParameterRepresentation::LogDiscountFactor,
        BootstrapInterpolation::LogLinear,
        true
    )]
    #[case(
        CurveParameterRepresentation::LogDiscountFactor,
        BootstrapInterpolation::FlatForward,
        true
    )]
    #[case(
        CurveParameterRepresentation::ZeroRate,
        BootstrapInterpolation::LinearZeroRate,
        true
    )]
    #[case(
        CurveParameterRepresentation::ZeroRate,
        BootstrapInterpolation::CubicSpline,
        true
    )]
    #[case(
        CurveParameterRepresentation::InstantaneousForward,
        BootstrapInterpolation::FlatForward,
        true
    )]
    #[case(
        CurveParameterRepresentation::InstantaneousForward,
        BootstrapInterpolation::LogLinear,
        false
    )]
    #[case(
        CurveParameterRepresentation::InstantaneousForward,
        BootstrapInterpolation::CubicSpline,
        false
    )]
    fn test_validation(
        #[case] repr: CurveParameterRepresentation,
        #[case] interp: BootstrapInterpolation,
        #[case] expected_valid: bool,
    ) {
        let config: CurveConfig<f64> = CurveConfig::builder()
            .parameter_representation(repr)
            .interpolation(interp)
            .build();
        assert_eq!(config.validate().is_ok(), expected_valid);
    }

    #[rstest]
    #[case(CurveParameterRepresentation::LogDiscountFactor)]
    #[case(CurveParameterRepresentation::ZeroRate)]
    #[case(CurveParameterRepresentation::InstantaneousForward)]
    fn test_monotonic_cubic_any_repr(#[case] repr: CurveParameterRepresentation) {
        let config: CurveConfig<f64> = CurveConfig::builder()
            .parameter_representation(repr)
            .interpolation(BootstrapInterpolation::MonotonicCubic)
            .build();
        assert!(config.validate().is_ok());
    }

    // ========================================
    // Parameterised With Method Tests
    // ========================================

    #[rstest]
    #[case(1e-14)]
    #[case(1e-8)]
    fn test_with_tolerance(#[case] tol: f64) {
        let config = CurveConfig::<f64>::default().with_tolerance(tol);
        assert!((config.tolerance - tol).abs() < 1e-20);
    }

    #[rstest]
    #[case(50)]
    #[case(200)]
    fn test_with_max_iterations(#[case] iters: usize) {
        let config = CurveConfig::<f64>::default().with_max_iterations(iters);
        assert_eq!(config.max_iterations, iters);
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    fn test_with_extrapolation(#[case] allow: bool) {
        let config = CurveConfig::<f64>::default().with_extrapolation(allow);
        assert_eq!(config.allow_extrapolation, allow);
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    fn test_with_negative_rates(#[case] allow: bool) {
        let config = CurveConfig::<f64>::default().with_negative_rates(allow);
        assert_eq!(config.allow_negative_rates, allow);
    }

    // ========================================
    // Builder Chained Test
    // ========================================

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
        assert!((config.tolerance - 1e-14).abs() < 1e-19);
        assert_eq!(config.max_iterations, 200);
        assert_eq!(config.interpolation, BootstrapInterpolation::LinearZeroRate);
        assert!(!config.allow_extrapolation);
        assert!(config.allow_negative_rates);
    }

    #[test]
    fn test_builder_validated() {
        let ok: Result<CurveConfig<f64>, _> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::ZeroRate)
            .interpolation(BootstrapInterpolation::LinearZeroRate)
            .build_validated();
        assert!(ok.is_ok());

        let err: Result<CurveConfig<f64>, _> = CurveConfig::builder()
            .parameter_representation(CurveParameterRepresentation::InstantaneousForward)
            .interpolation(BootstrapInterpolation::CubicSpline)
            .build_validated();
        assert!(err.is_err());
    }

    // ========================================
    // Deref Access Test
    // ========================================

    #[test]
    fn test_deref_access() {
        let config: CurveConfig<f64> = CurveConfig::default();
        // Access inner fields directly via Deref
        assert!((config.tolerance - 1e-12).abs() < 1e-17);
        assert_eq!(config.max_iterations, 100);
        assert_eq!(config.interpolation, BootstrapInterpolation::LogLinear);
    }

    #[test]
    fn test_deref_mut_access() {
        let mut config: CurveConfig<f64> = CurveConfig::default();
        config.tolerance = 1e-14;
        config.max_iterations = 200;
        assert!((config.tolerance - 1e-14).abs() < 1e-19);
        assert_eq!(config.max_iterations, 200);
    }

    // ========================================
    // Type Parameter Test
    // ========================================

    #[test]
    fn test_config_with_f32() {
        let config: CurveConfig<f32> = CurveConfig::default();
        assert!(config.tolerance > 0.0);
        assert_eq!(config.max_iterations, 100);
    }
}
