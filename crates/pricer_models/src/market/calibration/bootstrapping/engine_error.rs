//! Curve engine error types.
//!
//! This module provides structured error handling for the curve bootstrap
//! engine with detailed diagnostic information for each failure mode.

use thiserror::Error;

use super::{config::BootstrapInterpolation, error::BootstrapError};

/// Parameter representation for yield curves.
///
/// Determines how curve parameters are stored internally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CurveParameterRepresentation {
    /// Store log(discount_factor) - most common, default.
    ///
    /// Internal storage: log(D(t))
    /// Interpolation on log(DF) gives piecewise constant forward rates.
    #[default]
    LogDiscountFactor,

    /// Store continuously compounded zero rate.
    ///
    /// Internal storage: r(t) where D(t) = exp(-r(t) * t)
    ZeroRate,

    /// Store instantaneous forward rate.
    ///
    /// Internal storage: f(t) where D(t) = exp(-integral(f(s)ds, 0, t))
    InstantaneousForward,
}

/// Errors that can occur during curve engine operations.
///
/// Provides structured error handling with diagnostic information
/// including field names, tenors, and wrapped bootstrap errors.
///
/// # Error Categories
///
/// - **Configuration errors**: Invalid settings or parameter combinations
/// - **Instrument errors**: Problems with instrument definitions
/// - **Bootstrap errors**: Wrapped errors from the bootstrap engine
/// - **Cache errors**: Cache operation failures
/// - **Multi-curve errors**: Circular dependencies in curve definitions
#[derive(Error, Debug, Clone)]
pub enum CurveEngineError {
    /// Configuration error with field name and reason.
    #[error("Configuration error: {field} - {reason}")]
    Configuration {
        /// The configuration field that caused the error
        field: &'static str,
        /// Description of what went wrong
        reason: String,
    },

    /// Instrument error at a specific tenor.
    #[error("Instrument error at tenor {tenor}: {reason}")]
    Instrument {
        /// The tenor where the error occurred
        tenor: String,
        /// Description of what went wrong
        reason: String,
    },

    /// Incomplete instrument definition.
    #[error("Incomplete instrument definition: {0}")]
    IncompleteInstrumentDefinition(String),

    /// Unknown rate index.
    #[error("Unknown index: {0}")]
    UnknownIndex(String),

    /// Wrapped bootstrap error.
    #[error("Bootstrap error: {0}")]
    Bootstrap(#[from] BootstrapError),

    /// Interpolation error.
    #[error("Interpolation error: {0}")]
    Interpolation(String),

    /// Cache operation error.
    #[error("Cache error: {0}")]
    Cache(String),

    /// Configuration parse error (JSON/YAML).
    #[error("Configuration parse error: {0}")]
    ConfigurationParse(String),

    /// Circular dependency in curve definitions.
    #[error("Circular dependency detected in curve definitions")]
    CircularDependency,

    /// Invalid parameter representation and interpolation combination.
    #[error("Invalid configuration: {param_repr:?} is incompatible with {interpolation:?}")]
    InvalidConfiguration {
        /// The parameter representation
        param_repr: CurveParameterRepresentation,
        /// The interpolation method
        interpolation: BootstrapInterpolation,
    },

    /// IO error when loading configuration files.
    #[error("IO error: {0}")]
    Io(String),
}

impl CurveEngineError {
    /// Create a configuration error.
    ///
    /// # Arguments
    ///
    /// * `field` - The name of the invalid configuration field
    /// * `reason` - Description of why the configuration is invalid
    pub fn configuration(field: &'static str, reason: impl Into<String>) -> Self {
        Self::Configuration {
            field,
            reason: reason.into(),
        }
    }

    /// Create an instrument error.
    ///
    /// # Arguments
    ///
    /// * `tenor` - The tenor where the error occurred
    /// * `reason` - Description of the instrument error
    pub fn instrument(tenor: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Instrument {
            tenor: tenor.into(),
            reason: reason.into(),
        }
    }

    /// Create an incomplete instrument definition error.
    pub fn incomplete_definition(message: impl Into<String>) -> Self {
        Self::IncompleteInstrumentDefinition(message.into())
    }

    /// Create an unknown index error.
    pub fn unknown_index(index: impl Into<String>) -> Self { Self::UnknownIndex(index.into()) }

    /// Create an interpolation error.
    pub fn interpolation(message: impl Into<String>) -> Self { Self::Interpolation(message.into()) }

    /// Create a cache error.
    pub fn cache(message: impl Into<String>) -> Self { Self::Cache(message.into()) }

    /// Create a configuration parse error.
    pub fn parse(message: impl Into<String>) -> Self { Self::ConfigurationParse(message.into()) }

    /// Create an invalid configuration error.
    pub fn invalid_config(
        param_repr: CurveParameterRepresentation,
        interpolation: BootstrapInterpolation,
    ) -> Self {
        Self::InvalidConfiguration {
            param_repr,
            interpolation,
        }
    }

    /// Create an IO error.
    pub fn io(message: impl Into<String>) -> Self { Self::Io(message.into()) }

    /// Check if this is a bootstrap error.
    pub fn is_bootstrap_error(&self) -> bool { matches!(self, Self::Bootstrap(_)) }

    /// Check if this is a configuration error.
    pub fn is_configuration_error(&self) -> bool { matches!(self, Self::Configuration { .. }) }

    /// Check if this is an instrument error.
    pub fn is_instrument_error(&self) -> bool { matches!(self, Self::Instrument { .. }) }

    /// Check if this is a cache error.
    pub fn is_cache_error(&self) -> bool { matches!(self, Self::Cache(_)) }

    /// Check if this is a circular dependency error.
    pub fn is_circular_dependency(&self) -> bool { matches!(self, Self::CircularDependency) }

    /// Get the underlying bootstrap error if this is a Bootstrap variant.
    pub fn as_bootstrap_error(&self) -> Option<&BootstrapError> {
        match self {
            Self::Bootstrap(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CurveEngineError {
    fn from(err: std::io::Error) -> Self { Self::Io(err.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Configuration Error Tests
    // ========================================

    #[test]
    fn test_configuration_error_display() {
        let err = CurveEngineError::configuration("tolerance", "must be positive");
        let display = format!("{}", err);
        assert!(display.contains("Configuration error"));
        assert!(display.contains("tolerance"));
        assert!(display.contains("must be positive"));
    }

    #[test]
    fn test_configuration_error_is_check() {
        let err = CurveEngineError::configuration("tolerance", "must be positive");
        assert!(err.is_configuration_error());
        assert!(!err.is_bootstrap_error());
        assert!(!err.is_instrument_error());
    }

    // ========================================
    // Instrument Error Tests
    // ========================================

    #[test]
    fn test_instrument_error_display() {
        let err = CurveEngineError::instrument("5Y", "missing rate");
        let display = format!("{}", err);
        assert!(display.contains("Instrument error"));
        assert!(display.contains("5Y"));
        assert!(display.contains("missing rate"));
    }

    #[test]
    fn test_instrument_error_is_check() {
        let err = CurveEngineError::instrument("5Y", "missing rate");
        assert!(err.is_instrument_error());
        assert!(!err.is_configuration_error());
    }

    // ========================================
    // Incomplete Instrument Definition Tests
    // ========================================

    #[test]
    fn test_incomplete_definition_display() {
        let err = CurveEngineError::incomplete_definition("missing tenor field");
        let display = format!("{}", err);
        assert!(display.contains("Incomplete instrument definition"));
        assert!(display.contains("missing tenor field"));
    }

    // ========================================
    // Unknown Index Tests
    // ========================================

    #[test]
    fn test_unknown_index_display() {
        let err = CurveEngineError::unknown_index("USD-UNKNOWN");
        let display = format!("{}", err);
        assert!(display.contains("Unknown index"));
        assert!(display.contains("USD-UNKNOWN"));
    }

    // ========================================
    // Bootstrap Error Wrapper Tests
    // ========================================

    #[test]
    fn test_bootstrap_error_from() {
        let bootstrap_err = BootstrapError::convergence_failure(5.0, 0.001, 100);
        let engine_err: CurveEngineError = bootstrap_err.into();
        assert!(engine_err.is_bootstrap_error());
    }

    #[test]
    fn test_bootstrap_error_as_bootstrap_error() {
        let bootstrap_err = BootstrapError::convergence_failure(5.0, 0.001, 100);
        let engine_err: CurveEngineError = bootstrap_err.clone().into();
        let inner = engine_err.as_bootstrap_error();
        assert!(inner.is_some());
        assert!(inner.unwrap().is_convergence_failure());
    }

    #[test]
    fn test_non_bootstrap_error_as_bootstrap_error() {
        let err = CurveEngineError::configuration("field", "reason");
        assert!(err.as_bootstrap_error().is_none());
    }

    // ========================================
    // Interpolation Error Tests
    // ========================================

    #[test]
    fn test_interpolation_error_display() {
        let err = CurveEngineError::interpolation("extrapolation not allowed");
        let display = format!("{}", err);
        assert!(display.contains("Interpolation error"));
        assert!(display.contains("extrapolation not allowed"));
    }

    // ========================================
    // Cache Error Tests
    // ========================================

    #[test]
    fn test_cache_error_display() {
        let err = CurveEngineError::cache("lock poisoned");
        let display = format!("{}", err);
        assert!(display.contains("Cache error"));
        assert!(display.contains("lock poisoned"));
    }

    #[test]
    fn test_cache_error_is_check() {
        let err = CurveEngineError::cache("lock poisoned");
        assert!(err.is_cache_error());
        assert!(!err.is_bootstrap_error());
    }

    // ========================================
    // Configuration Parse Error Tests
    // ========================================

    #[test]
    fn test_parse_error_display() {
        let err = CurveEngineError::parse("invalid JSON at line 5");
        let display = format!("{}", err);
        assert!(display.contains("Configuration parse error"));
        assert!(display.contains("invalid JSON at line 5"));
    }

    // ========================================
    // Circular Dependency Tests
    // ========================================

    #[test]
    fn test_circular_dependency_display() {
        let err = CurveEngineError::CircularDependency;
        let display = format!("{}", err);
        assert!(display.contains("Circular dependency"));
    }

    #[test]
    fn test_circular_dependency_is_check() {
        let err = CurveEngineError::CircularDependency;
        assert!(err.is_circular_dependency());
        assert!(!err.is_bootstrap_error());
    }

    // ========================================
    // Invalid Configuration Tests
    // ========================================

    #[test]
    fn test_invalid_config_display() {
        let err = CurveEngineError::invalid_config(
            CurveParameterRepresentation::InstantaneousForward,
            BootstrapInterpolation::LogLinear,
        );
        let display = format!("{}", err);
        assert!(display.contains("Invalid configuration"));
        assert!(display.contains("InstantaneousForward"));
        assert!(display.contains("LogLinear"));
    }

    // ========================================
    // IO Error Tests
    // ========================================

    #[test]
    fn test_io_error_display() {
        let err = CurveEngineError::io("file not found");
        let display = format!("{}", err);
        assert!(display.contains("IO error"));
        assert!(display.contains("file not found"));
    }

    #[test]
    fn test_io_error_from_std() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let engine_err: CurveEngineError = io_err.into();
        let display = format!("{}", engine_err);
        assert!(display.contains("IO error"));
    }

    // ========================================
    // Clone and Debug Tests
    // ========================================

    #[test]
    fn test_clone() {
        let err1 = CurveEngineError::configuration("field", "reason");
        let err2 = err1.clone();
        assert!(err2.is_configuration_error());
    }

    #[test]
    fn test_debug() {
        let err = CurveEngineError::configuration("field", "reason");
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Configuration"));
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = CurveEngineError::configuration("field", "reason");
        let _: &dyn std::error::Error = &err;
    }

    // ========================================
    // CurveParameterRepresentation Tests
    // ========================================

    #[test]
    fn test_param_repr_default() {
        let repr: CurveParameterRepresentation = Default::default();
        assert_eq!(repr, CurveParameterRepresentation::LogDiscountFactor);
    }

    #[test]
    fn test_param_repr_clone() {
        let repr1 = CurveParameterRepresentation::ZeroRate;
        let repr2 = repr1.clone();
        assert_eq!(repr1, repr2);
    }

    #[test]
    fn test_param_repr_copy() {
        let repr1 = CurveParameterRepresentation::InstantaneousForward;
        let repr2 = repr1; // Copy
        assert_eq!(repr1, repr2);
    }

    #[test]
    fn test_param_repr_debug() {
        let repr = CurveParameterRepresentation::ZeroRate;
        let debug_str = format!("{:?}", repr);
        assert!(debug_str.contains("ZeroRate"));
    }

    #[test]
    fn test_param_repr_equality() {
        assert_eq!(
            CurveParameterRepresentation::LogDiscountFactor,
            CurveParameterRepresentation::LogDiscountFactor
        );
        assert_ne!(
            CurveParameterRepresentation::LogDiscountFactor,
            CurveParameterRepresentation::ZeroRate
        );
    }
}
