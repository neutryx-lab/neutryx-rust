//! Configuration for IRS Greeks calculation.

use crate::greeks::GreeksMode;

/// Configuration for IRS Greeks calculation.
///
/// Controls bump widths, calculation modes, and verification tolerances.
///
/// # Default Values
///
/// | Parameter | Default | Description |
/// |-----------|---------|-------------|
/// | `bump_size` | 0.0001 (1bp) | Bump size for finite differences |
/// | `tolerance` | 1e-6 | Tolerance for AAD vs Bump verification |
/// | `mode` | BumpRevalue | Default calculation mode |
///
/// # Examples
///
/// ```rust,ignore
/// use pricer_pricing::irs_greeks::IrsGreeksConfig;
/// use pricer_pricing::greeks::GreeksMode;
///
/// let config = IrsGreeksConfig::default();
/// assert!((config.bump_size - 0.0001).abs() < 1e-10);
///
/// let custom = IrsGreeksConfig {
///     bump_size: 0.0005, // 5bp
///     tolerance: 1e-8,
///     mode: GreeksMode::BumpRevalue,
/// };
/// ```
#[derive(Clone, Debug)]
pub struct IrsGreeksConfig {
    /// Bump size for finite differences (default: 1bp = 0.0001).
    pub bump_size: f64,

    /// Tolerance for AAD vs Bump-and-Revalue verification (default: 1e-6).
    pub tolerance: f64,

    /// Calculation mode.
    pub mode: GreeksMode,
}

impl Default for IrsGreeksConfig {
    fn default() -> Self {
        Self {
            bump_size: 0.0001, // 1 basis point
            tolerance: 1e-6,
            mode: GreeksMode::default(),
        }
    }
}

impl IrsGreeksConfig {
    /// Creates a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the bump size.
    pub fn with_bump_size(mut self, bump_size: f64) -> Self {
        self.bump_size = bump_size;
        self
    }

    /// Sets the tolerance.
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Sets the calculation mode.
    pub fn with_mode(mut self, mode: GreeksMode) -> Self {
        self.mode = mode;
        self
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), super::IrsGreeksError> {
        if self.bump_size <= 0.0 {
            return Err(super::IrsGreeksError::InvalidConfig(
                "bump_size must be positive".to_string(),
            ));
        }
        if self.tolerance <= 0.0 {
            return Err(super::IrsGreeksError::InvalidConfig(
                "tolerance must be positive".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IrsGreeksConfig::default();
        assert!((config.bump_size - 0.0001).abs() < 1e-10);
        assert!((config.tolerance - 1e-6).abs() < 1e-10);
        assert_eq!(config.mode, GreeksMode::BumpRevalue);
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = IrsGreeksConfig::new()
            .with_bump_size(0.0005)
            .with_tolerance(1e-8)
            .with_mode(GreeksMode::BumpRevalue);

        assert!((config.bump_size - 0.0005).abs() < 1e-10);
        assert!((config.tolerance - 1e-8).abs() < 1e-10);
    }

    #[test]
    fn test_config_validation_valid() {
        let config = IrsGreeksConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_bump() {
        let config = IrsGreeksConfig {
            bump_size: -0.0001,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_tolerance() {
        let config = IrsGreeksConfig {
            tolerance: 0.0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }
}
