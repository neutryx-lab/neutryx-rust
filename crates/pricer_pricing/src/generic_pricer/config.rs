//! Configuration structures for Generic Pricer Engine.
//!
//! This module provides:
//! - [`ModelConfig`]: Model selection and simulation parameters
//! - [`PricerConfig`]: Pricer settings including Greeks mode and default
//!   currency

#[cfg(feature = "l1l2-integration")]
use infra_master::market::Currency;

use super::error::ConfigError;

// =============================================================================
// Greeks Configuration (local definitions)
// =============================================================================

/// Calculation mode for Greeks computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GreeksMode {
    /// Bump-and-revalue using finite differences.
    #[default]
    BumpRevalue,
    /// Forward-mode AD using num-dual library.
    NumDual,
    /// Enzyme LLVM-level automatic differentiation.
    #[cfg(feature = "enzyme-ad")]
    EnzymeAAD,
}

/// Configuration for Greeks calculation.
#[derive(Clone, Debug)]
pub struct GreeksConfig {
    /// Calculation mode.
    pub mode: GreeksMode,
    /// Relative bump for spot price (default: 0.01 = 1%).
    pub spot_bump_relative: f64,
    /// Absolute bump for volatility (default: 0.01).
    pub vol_bump_absolute: f64,
    /// Time bump in years (default: 1/252).
    pub time_bump_years: f64,
    /// Absolute bump for interest rate (default: 0.01).
    pub rate_bump_absolute: f64,
    /// Tolerance for verification (default: 1e-6).
    pub verification_tolerance: f64,
}

impl Default for GreeksConfig {
    fn default() -> Self {
        Self {
            mode: GreeksMode::default(),
            spot_bump_relative: 0.01,
            vol_bump_absolute: 0.01,
            time_bump_years: 1.0 / 252.0,
            rate_bump_absolute: 0.01,
            verification_tolerance: 1e-6,
        }
    }
}

impl GreeksConfig {
    /// Creates a new builder.
    pub fn builder() -> GreeksConfigBuilder { GreeksConfigBuilder::default() }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), GreeksConfigError> {
        if self.spot_bump_relative <= 0.0 || self.spot_bump_relative > 1.0 {
            return Err(GreeksConfigError::InvalidSpotBump);
        }
        if self.vol_bump_absolute <= 0.0 || self.vol_bump_absolute > 0.5 {
            return Err(GreeksConfigError::InvalidVolBump);
        }
        if self.time_bump_years <= 0.0 || self.time_bump_years > 1.0 {
            return Err(GreeksConfigError::InvalidTimeBump);
        }
        if self.rate_bump_absolute <= 0.0 || self.rate_bump_absolute > 0.1 {
            return Err(GreeksConfigError::InvalidRateBump);
        }
        if self.verification_tolerance <= 0.0 {
            return Err(GreeksConfigError::InvalidTolerance);
        }
        Ok(())
    }
}

/// Builder for GreeksConfig.
#[derive(Debug, Default)]
pub struct GreeksConfigBuilder {
    mode: Option<GreeksMode>,
    spot_bump_relative: Option<f64>,
    vol_bump_absolute: Option<f64>,
    time_bump_years: Option<f64>,
    rate_bump_absolute: Option<f64>,
    verification_tolerance: Option<f64>,
}

impl GreeksConfigBuilder {
    /// Sets the calculation mode.
    pub fn mode(mut self, mode: GreeksMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Sets the relative spot bump.
    pub fn spot_bump_relative(mut self, bump: f64) -> Self {
        self.spot_bump_relative = Some(bump);
        self
    }

    /// Sets the absolute volatility bump.
    pub fn vol_bump_absolute(mut self, bump: f64) -> Self {
        self.vol_bump_absolute = Some(bump);
        self
    }

    /// Sets the time bump in years.
    pub fn time_bump_years(mut self, bump: f64) -> Self {
        self.time_bump_years = Some(bump);
        self
    }

    /// Sets the absolute rate bump.
    pub fn rate_bump_absolute(mut self, bump: f64) -> Self {
        self.rate_bump_absolute = Some(bump);
        self
    }

    /// Sets the verification tolerance.
    pub fn verification_tolerance(mut self, tolerance: f64) -> Self {
        self.verification_tolerance = Some(tolerance);
        self
    }

    /// Builds the configuration.
    pub fn build(self) -> Result<GreeksConfig, GreeksConfigError> {
        let config = GreeksConfig {
            mode: self.mode.unwrap_or_default(),
            spot_bump_relative: self.spot_bump_relative.unwrap_or(0.01),
            vol_bump_absolute: self.vol_bump_absolute.unwrap_or(0.01),
            time_bump_years: self.time_bump_years.unwrap_or(1.0 / 252.0),
            rate_bump_absolute: self.rate_bump_absolute.unwrap_or(0.01),
            verification_tolerance: self.verification_tolerance.unwrap_or(1e-6),
        };
        config.validate()?;
        Ok(config)
    }
}

/// Error type for GreeksConfig validation.
#[derive(Debug, Clone, PartialEq)]
pub enum GreeksConfigError {
    /// Invalid spot bump value.
    InvalidSpotBump,
    /// Invalid volatility bump value.
    InvalidVolBump,
    /// Invalid time bump value.
    InvalidTimeBump,
    /// Invalid rate bump value.
    InvalidRateBump,
    /// Invalid verification tolerance.
    InvalidTolerance,
}

impl std::fmt::Display for GreeksConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSpotBump => write!(f, "Invalid spot bump"),
            Self::InvalidVolBump => write!(f, "Invalid vol bump"),
            Self::InvalidTimeBump => write!(f, "Invalid time bump"),
            Self::InvalidRateBump => write!(f, "Invalid rate bump"),
            Self::InvalidTolerance => write!(f, "Invalid tolerance"),
        }
    }
}

impl std::error::Error for GreeksConfigError {}

/// Model configuration for Generic Pricer.
///
/// Contains model selection and simulation parameters (path count, steps,
/// seed). Use [`ModelConfigBuilder`] for construction.
///
/// # Default Values
///
/// | Parameter | Default | Description |
/// |-----------|---------|-------------|
/// | `model` | `None` | Model selection (None = auto-select by instrument) |
/// | `num_paths` | 10,000 | Number of Monte Carlo paths |
/// | `num_steps` | 100 | Number of time steps per path |
/// | `seed` | `None` | Random seed (None = non-deterministic) |
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::generic_pricer::ModelConfig;
///
/// // Use defaults
/// let config = ModelConfig::default();
/// assert_eq!(config.num_paths, 10_000);
///
/// // Use builder for custom values
/// let config = ModelConfig::builder()
///     .num_paths(50_000)
///     .num_steps(200)
///     .seed(42)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Number of Monte Carlo simulation paths.
    pub num_paths: usize,

    /// Number of time steps per path.
    pub num_steps: usize,

    /// Random seed for reproducibility (None = non-deterministic).
    pub seed: Option<u64>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            num_paths: 10_000,
            num_steps: 100,
            seed: None,
        }
    }
}

impl ModelConfig {
    /// Creates a new builder for constructing a `ModelConfig`.
    pub fn builder() -> ModelConfigBuilder { ModelConfigBuilder::default() }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if any parameter is invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.num_paths == 0 {
            return Err(ConfigError::invalid_model_parameter(
                "num_paths",
                "must be > 0",
            ));
        }
        if self.num_paths > 10_000_000 {
            return Err(ConfigError::invalid_model_parameter(
                "num_paths",
                "must be <= 10,000,000",
            ));
        }
        if self.num_steps == 0 {
            return Err(ConfigError::invalid_model_parameter(
                "num_steps",
                "must be > 0",
            ));
        }
        if self.num_steps > 10_000 {
            return Err(ConfigError::invalid_model_parameter(
                "num_steps",
                "must be <= 10,000",
            ));
        }
        Ok(())
    }
}

/// Builder for [`ModelConfig`].
#[derive(Debug, Default)]
pub struct ModelConfigBuilder {
    num_paths: Option<usize>,
    num_steps: Option<usize>,
    seed: Option<u64>,
}

impl ModelConfigBuilder {
    /// Sets the number of Monte Carlo paths.
    pub fn num_paths(mut self, n: usize) -> Self {
        self.num_paths = Some(n);
        self
    }

    /// Sets the number of time steps.
    pub fn num_steps(mut self, n: usize) -> Self {
        self.num_steps = Some(n);
        self
    }

    /// Sets the random seed for reproducibility.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Builds the configuration, validating all parameters.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if any parameter is invalid.
    pub fn build(self) -> Result<ModelConfig, ConfigError> {
        let config = ModelConfig {
            num_paths: self.num_paths.unwrap_or(10_000),
            num_steps: self.num_steps.unwrap_or(100),
            seed: self.seed,
        };
        config.validate()?;
        Ok(config)
    }
}

/// Default currency for standalone pricing (always available).
///
/// This type is always exported regardless of l1l2-integration feature.
/// Use this for standalone pricing without full market data integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DefaultCurrency {
    /// US Dollar
    #[default]
    USD,
    /// Euro
    EUR,
    /// British Pound
    GBP,
    /// Japanese Yen
    JPY,
    /// Swiss Franc
    CHF,
}

impl DefaultCurrency {
    /// Creates a currency from a string code.
    ///
    /// Returns USD for unknown codes.
    pub fn new(code: &str) -> Self {
        match code.to_uppercase().as_str() {
            "USD" => Self::USD,
            "EUR" => Self::EUR,
            "GBP" => Self::GBP,
            "JPY" => Self::JPY,
            "CHF" => Self::CHF,
            _ => Self::USD, // Default to USD for unknown codes
        }
    }

    /// Returns the ISO 4217 currency code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::USD => "USD",
            Self::EUR => "EUR",
            Self::GBP => "GBP",
            Self::JPY => "JPY",
            Self::CHF => "CHF",
        }
    }
}

impl std::fmt::Display for DefaultCurrency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Pricer configuration for Generic Pricer.
///
/// Contains Greeks calculation settings and output preferences.
/// Use [`PricerConfigBuilder`] for construction.
///
/// # Default Values
///
/// | Parameter | Default | Description |
/// |-----------|---------|-------------|
/// | `greeks_config` | `GreeksConfig::default()` | Greeks calculation settings |
/// | `default_currency` | `Currency::USD` | Default reporting currency |
/// | `use_thread_local_buffers` | `true` | Use thread-local buffer pool |
///
/// # Examples
///
/// ```rust,ignore
/// use pricer_pricing::generic_pricer::{PricerConfig, GreeksMode};
///
/// let config = PricerConfig::builder()
///     .use_thread_local_buffers(true)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct PricerConfig {
    /// Greeks calculation configuration.
    pub greeks_config: GreeksConfig,

    /// Default reporting currency.
    #[cfg(feature = "l1l2-integration")]
    pub default_currency: Currency,

    /// Default reporting currency (without l1l2-integration).
    #[cfg(not(feature = "l1l2-integration"))]
    pub default_currency: DefaultCurrency,

    /// Whether to use thread-local buffer pool for batch processing.
    pub use_thread_local_buffers: bool,
}

impl Default for PricerConfig {
    fn default() -> Self {
        Self {
            greeks_config: GreeksConfig::default(),
            #[cfg(feature = "l1l2-integration")]
            default_currency: Currency::USD,
            #[cfg(not(feature = "l1l2-integration"))]
            default_currency: DefaultCurrency::USD,
            use_thread_local_buffers: true,
        }
    }
}

impl PricerConfig {
    /// Creates a new builder for constructing a `PricerConfig`.
    pub fn builder() -> PricerConfigBuilder { PricerConfigBuilder::default() }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if any parameter is invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.greeks_config
            .validate()
            .map_err(|e| ConfigError::invalid_pricer_config(e.to_string()))?;
        Ok(())
    }
}

/// Builder for [`PricerConfig`].
#[derive(Debug, Default)]
pub struct PricerConfigBuilder {
    greeks_config: Option<GreeksConfig>,
    #[cfg(feature = "l1l2-integration")]
    default_currency: Option<Currency>,
    #[cfg(not(feature = "l1l2-integration"))]
    default_currency: Option<DefaultCurrency>,
    use_thread_local_buffers: Option<bool>,
}

impl PricerConfigBuilder {
    /// Sets the Greeks configuration.
    pub fn greeks_config(mut self, config: GreeksConfig) -> Self {
        self.greeks_config = Some(config);
        self
    }

    /// Sets the Greeks calculation mode (convenience method).
    pub fn greeks_mode(mut self, mode: GreeksMode) -> Self {
        let mut config = self.greeks_config.take().unwrap_or_default();
        config.mode = mode;
        self.greeks_config = Some(config);
        self
    }

    /// Sets the default reporting currency.
    #[cfg(feature = "l1l2-integration")]
    pub fn default_currency(mut self, currency: Currency) -> Self {
        self.default_currency = Some(currency);
        self
    }

    /// Sets the default reporting currency (without l1l2-integration).
    #[cfg(not(feature = "l1l2-integration"))]
    pub fn default_currency(mut self, currency: DefaultCurrency) -> Self {
        self.default_currency = Some(currency);
        self
    }

    /// Sets whether to use thread-local buffer pool.
    pub fn use_thread_local_buffers(mut self, use_buffers: bool) -> Self {
        self.use_thread_local_buffers = Some(use_buffers);
        self
    }

    /// Builds the configuration, validating all parameters.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if any parameter is invalid.
    pub fn build(self) -> Result<PricerConfig, ConfigError> {
        let config = PricerConfig {
            greeks_config: self.greeks_config.unwrap_or_default(),
            #[cfg(feature = "l1l2-integration")]
            default_currency: self.default_currency.unwrap_or(Currency::USD),
            #[cfg(not(feature = "l1l2-integration"))]
            default_currency: self.default_currency.unwrap_or(DefaultCurrency::USD),
            use_thread_local_buffers: self.use_thread_local_buffers.unwrap_or(true),
        };
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ModelConfig Tests (Task 2.1)
    // =========================================================================

    #[test]
    fn test_model_config_default() {
        let config = ModelConfig::default();
        assert_eq!(config.num_paths, 10_000);
        assert_eq!(config.num_steps, 100);
        assert!(config.seed.is_none());
    }

    #[test]
    fn test_model_config_builder() {
        let config = ModelConfig::builder()
            .num_paths(50_000)
            .num_steps(200)
            .seed(42)
            .build()
            .unwrap();

        assert_eq!(config.num_paths, 50_000);
        assert_eq!(config.num_steps, 200);
        assert_eq!(config.seed, Some(42));
    }

    #[test]
    fn test_model_config_builder_defaults() {
        let config = ModelConfig::builder().build().unwrap();
        assert_eq!(config.num_paths, 10_000);
        assert_eq!(config.num_steps, 100);
    }

    #[test]
    fn test_model_config_validate_num_paths_zero() {
        let result = ModelConfig::builder().num_paths(0).build();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            ConfigError::InvalidModelParameter {
                name: "num_paths",
                ..
            }
        ));
    }

    #[test]
    fn test_model_config_validate_num_paths_too_large() {
        let result = ModelConfig::builder().num_paths(20_000_000).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_model_config_validate_num_steps_zero() {
        let result = ModelConfig::builder().num_steps(0).build();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            ConfigError::InvalidModelParameter {
                name: "num_steps",
                ..
            }
        ));
    }

    #[test]
    fn test_model_config_validate_num_steps_too_large() {
        let result = ModelConfig::builder().num_steps(20_000).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_model_config_clone() {
        let config1 = ModelConfig::builder()
            .num_paths(5000)
            .seed(123)
            .build()
            .unwrap();
        let config2 = config1.clone();
        assert_eq!(config1.num_paths, config2.num_paths);
        assert_eq!(config1.seed, config2.seed);
    }

    // =========================================================================
    // PricerConfig Tests (Task 2.2)
    // =========================================================================

    #[test]
    fn test_pricer_config_default() {
        let config = PricerConfig::default();
        assert!(config.use_thread_local_buffers);
        assert_eq!(config.greeks_config.mode, GreeksMode::BumpRevalue);
    }

    #[test]
    fn test_pricer_config_builder() {
        let config = PricerConfig::builder()
            .greeks_mode(GreeksMode::NumDual)
            .use_thread_local_buffers(false)
            .build()
            .unwrap();

        assert_eq!(config.greeks_config.mode, GreeksMode::NumDual);
        assert!(!config.use_thread_local_buffers);
    }

    #[test]
    fn test_pricer_config_builder_with_greeks_config() {
        let greeks_config = GreeksConfig::builder()
            .spot_bump_relative(0.02)
            .build()
            .unwrap();

        let config = PricerConfig::builder()
            .greeks_config(greeks_config)
            .build()
            .unwrap();

        assert!((config.greeks_config.spot_bump_relative - 0.02).abs() < 1e-10);
    }

    #[test]
    fn test_pricer_config_validate_invalid_greeks() {
        // Create invalid greeks config manually
        let mut pricer_config = PricerConfig::default();
        pricer_config.greeks_config.spot_bump_relative = -1.0;

        let result = pricer_config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_pricer_config_clone() {
        let config1 = PricerConfig::builder()
            .use_thread_local_buffers(false)
            .build()
            .unwrap();
        let config2 = config1.clone();
        assert_eq!(
            config1.use_thread_local_buffers,
            config2.use_thread_local_buffers
        );
    }

    // =========================================================================
    // Validation Tests (Task 2.3)
    // =========================================================================

    #[test]
    fn test_model_config_boundary_values() {
        // Minimum valid values
        let config = ModelConfig::builder()
            .num_paths(1)
            .num_steps(1)
            .build()
            .unwrap();
        assert_eq!(config.num_paths, 1);
        assert_eq!(config.num_steps, 1);

        // Maximum valid values
        let config = ModelConfig::builder()
            .num_paths(10_000_000)
            .num_steps(10_000)
            .build()
            .unwrap();
        assert_eq!(config.num_paths, 10_000_000);
        assert_eq!(config.num_steps, 10_000);
    }

    #[test]
    fn test_config_error_messages() {
        let result = ModelConfig::builder().num_paths(0).build();
        let err = result.unwrap_err();
        assert!(err.to_string().contains("num_paths"));
        assert!(err.to_string().contains("must be > 0"));
    }
}
