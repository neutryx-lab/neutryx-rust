//! Fallback implementation for when Enzyme AD is disabled.
//!
//! This module provides the bridge between the Enzyme AD module and the
//! existing Greeks calculation infrastructure. When the `enzyme-ad` feature
//! is disabled, all AD-based Greeks computations fall back to bump-and-revalue
//! finite difference methods.
//!
//! # Mode Resolution
//!
//! The [`FallbackResolver`] handles the mapping between enzyme-specific modes
//! and the general Greeks calculation modes:
//!
//! | Enzyme Mode | Enzyme Enabled | Enzyme Disabled |
//! |-------------|----------------|-----------------|
//! | Auto | ReverseMode | BumpRevalue |
//! | EnzymeOnly | Enzyme AD | panic! |
//! | FiniteDifference | BumpRevalue | BumpRevalue |
//! | ForwardMode | Enzyme Forward | BumpRevalue |
//! | ReverseMode | Enzyme Reverse | BumpRevalue |
//!
//! # Usage
//!
//! ```rust
//! use pricer_pricing::enzyme::fallback::{FallbackResolver, FallbackConfig};
//! use pricer_pricing::enzyme::greeks::GreeksMode as EnzymeMode;
//!
//! let config = FallbackConfig::default();
//! let resolver = FallbackResolver::new(config);
//!
//! // Check if enzyme is available
//! if !resolver.enzyme_available() {
//!     println!("Falling back to finite differences");
//! }
//!
//! // Resolve mode
//! let resolved = resolver.resolve_mode(EnzymeMode::Auto);
//! ```

use crate::greeks::{GreeksConfig, GreeksMode as CoreGreeksMode};

use super::greeks::GreeksMode as EnzymeGreeksMode;

/// Configuration for fallback behaviour.
#[derive(Clone, Debug)]
pub struct FallbackConfig {
    /// Whether to warn when falling back from Enzyme to FD.
    pub warn_on_fallback: bool,

    /// Whether to allow EnzymeOnly mode to fall back (or panic).
    pub strict_enzyme_only: bool,

    /// Configuration for bump-and-revalue calculations.
    pub greeks_config: GreeksConfig,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            warn_on_fallback: true,
            strict_enzyme_only: true,
            greeks_config: GreeksConfig::default(),
        }
    }
}

impl FallbackConfig {
    /// Creates a new fallback configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method: enable/disable fallback warnings.
    #[inline]
    pub fn with_warn_on_fallback(mut self, warn: bool) -> Self {
        self.warn_on_fallback = warn;
        self
    }

    /// Builder method: enable/disable strict EnzymeOnly mode.
    #[inline]
    pub fn with_strict_enzyme_only(mut self, strict: bool) -> Self {
        self.strict_enzyme_only = strict;
        self
    }

    /// Builder method: set Greeks configuration.
    #[inline]
    pub fn with_greeks_config(mut self, config: GreeksConfig) -> Self {
        self.greeks_config = config;
        self
    }
}

/// Resolves Enzyme AD modes to fallback implementations.
///
/// This struct provides the logic for determining whether to use Enzyme AD
/// or fall back to finite differences based on feature availability and
/// requested mode.
#[derive(Clone, Debug)]
pub struct FallbackResolver {
    config: FallbackConfig,
}

impl FallbackResolver {
    /// Creates a new fallback resolver with the given configuration.
    #[inline]
    pub fn new(config: FallbackConfig) -> Self {
        Self { config }
    }

    /// Creates a resolver with default configuration.
    #[inline]
    pub fn default_resolver() -> Self {
        Self::new(FallbackConfig::default())
    }

    /// Returns whether Enzyme AD is available.
    #[inline]
    pub fn enzyme_available(&self) -> bool {
        cfg!(feature = "enzyme-ad")
    }

    /// Resolves an Enzyme mode to a concrete implementation mode.
    ///
    /// Returns the resolved mode and whether fallback was used.
    pub fn resolve_mode(&self, mode: EnzymeGreeksMode) -> ResolvedMode {
        let enzyme_available = self.enzyme_available();

        match mode {
            EnzymeGreeksMode::Auto => {
                if enzyme_available {
                    ResolvedMode::enzyme(EnzymeGreeksMode::ReverseMode)
                } else {
                    ResolvedMode::fallback(CoreGreeksMode::BumpRevalue)
                }
            }

            EnzymeGreeksMode::EnzymeOnly => {
                if enzyme_available {
                    ResolvedMode::enzyme(EnzymeGreeksMode::ReverseMode)
                } else if self.config.strict_enzyme_only {
                    ResolvedMode::error(FallbackError::EnzymeNotAvailable)
                } else {
                    ResolvedMode::fallback(CoreGreeksMode::BumpRevalue)
                }
            }

            EnzymeGreeksMode::FiniteDifference => {
                ResolvedMode::fallback(CoreGreeksMode::BumpRevalue)
            }

            EnzymeGreeksMode::ForwardMode => {
                if enzyme_available {
                    ResolvedMode::enzyme(EnzymeGreeksMode::ForwardMode)
                } else {
                    ResolvedMode::fallback(CoreGreeksMode::BumpRevalue)
                }
            }

            EnzymeGreeksMode::ReverseMode => {
                if enzyme_available {
                    ResolvedMode::enzyme(EnzymeGreeksMode::ReverseMode)
                } else {
                    ResolvedMode::fallback(CoreGreeksMode::BumpRevalue)
                }
            }
        }
    }

    /// Returns the Greeks configuration for fallback.
    #[inline]
    pub fn greeks_config(&self) -> &GreeksConfig {
        &self.config.greeks_config
    }

    /// Returns the fallback configuration.
    #[inline]
    pub fn config(&self) -> &FallbackConfig {
        &self.config
    }

    /// Checks if warnings should be issued for fallback.
    #[inline]
    pub fn should_warn(&self) -> bool {
        self.config.warn_on_fallback && !self.enzyme_available()
    }
}

impl Default for FallbackResolver {
    fn default() -> Self {
        Self::default_resolver()
    }
}

/// Result of mode resolution.
#[derive(Clone, Debug)]
pub struct ResolvedMode {
    /// The resolved computation method.
    pub method: ComputationMethod,

    /// Whether this is a fallback from the requested mode.
    pub is_fallback: bool,

    /// Error if resolution failed.
    pub error: Option<FallbackError>,
}

impl ResolvedMode {
    /// Creates a resolved mode using Enzyme AD.
    fn enzyme(mode: EnzymeGreeksMode) -> Self {
        Self {
            method: ComputationMethod::Enzyme(mode),
            is_fallback: false,
            error: None,
        }
    }

    /// Creates a resolved mode using fallback.
    fn fallback(mode: CoreGreeksMode) -> Self {
        Self {
            method: ComputationMethod::Fallback(mode),
            is_fallback: true,
            error: None,
        }
    }

    /// Creates an error result.
    fn error(err: FallbackError) -> Self {
        Self {
            method: ComputationMethod::Error,
            is_fallback: false,
            error: Some(err),
        }
    }

    /// Returns true if this is using Enzyme AD.
    #[inline]
    pub fn uses_enzyme(&self) -> bool {
        matches!(self.method, ComputationMethod::Enzyme(_))
    }

    /// Returns true if this is using fallback.
    #[inline]
    pub fn uses_fallback(&self) -> bool {
        matches!(self.method, ComputationMethod::Fallback(_))
    }

    /// Returns true if resolution failed.
    #[inline]
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Converts to Result, returning error if resolution failed.
    pub fn into_result(self) -> Result<ComputationMethod, FallbackError> {
        match self.error {
            Some(err) => Err(err),
            None => Ok(self.method),
        }
    }
}

/// The resolved computation method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComputationMethod {
    /// Use Enzyme AD with the specified mode.
    Enzyme(EnzymeGreeksMode),

    /// Use fallback with the specified Greeks mode.
    Fallback(CoreGreeksMode),

    /// Error - no valid method available.
    Error,
}

/// Errors that can occur during fallback resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallbackError {
    /// EnzymeOnly was requested but Enzyme is not available.
    EnzymeNotAvailable,

    /// The requested mode is not supported.
    UnsupportedMode,
}

impl std::fmt::Display for FallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EnzymeNotAvailable => {
                write!(
                    f,
                    "Enzyme AD not available. Enable the 'enzyme-ad' feature."
                )
            }
            Self::UnsupportedMode => {
                write!(f, "The requested computation mode is not supported.")
            }
        }
    }
}

impl std::error::Error for FallbackError {}

/// Helper trait for types that can fall back to finite differences.
pub trait FallbackAware {
    /// Returns the computation method being used.
    fn computation_method(&self) -> ComputationMethod;

    /// Returns true if using fallback.
    fn is_using_fallback(&self) -> bool {
        matches!(self.computation_method(), ComputationMethod::Fallback(_))
    }

    /// Returns true if using Enzyme.
    fn is_using_enzyme(&self) -> bool {
        matches!(self.computation_method(), ComputationMethod::Enzyme(_))
    }
}

/// Utility function to check if Enzyme AD is available.
#[inline]
pub fn is_enzyme_available() -> bool {
    cfg!(feature = "enzyme-ad")
}

/// Utility function to get the default computation method.
#[inline]
pub fn default_method() -> ComputationMethod {
    if is_enzyme_available() {
        ComputationMethod::Enzyme(EnzymeGreeksMode::ReverseMode)
    } else {
        ComputationMethod::Fallback(CoreGreeksMode::BumpRevalue)
    }
}

/// Utility function to resolve Enzyme mode with default config.
#[inline]
pub fn resolve_enzyme_mode(mode: EnzymeGreeksMode) -> ResolvedMode {
    FallbackResolver::default().resolve_mode(mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_config_default() {
        let config = FallbackConfig::default();

        assert!(config.warn_on_fallback);
        assert!(config.strict_enzyme_only);
    }

    #[test]
    fn test_fallback_config_builder() {
        let config = FallbackConfig::new()
            .with_warn_on_fallback(false)
            .with_strict_enzyme_only(false);

        assert!(!config.warn_on_fallback);
        assert!(!config.strict_enzyme_only);
    }

    #[test]
    fn test_fallback_resolver_new() {
        let resolver = FallbackResolver::default();

        // Enzyme availability depends on feature flag
        let _available = resolver.enzyme_available();
    }

    #[test]
    fn test_resolve_mode_auto() {
        let resolver = FallbackResolver::default();
        let resolved = resolver.resolve_mode(EnzymeGreeksMode::Auto);

        // Without enzyme-ad feature, should fall back
        #[cfg(not(feature = "enzyme-ad"))]
        {
            assert!(resolved.is_fallback);
            assert!(resolved.uses_fallback());
        }

        #[cfg(feature = "enzyme-ad")]
        {
            assert!(!resolved.is_fallback);
            assert!(resolved.uses_enzyme());
        }
    }

    #[test]
    fn test_resolve_mode_finite_difference() {
        let resolver = FallbackResolver::default();
        let resolved = resolver.resolve_mode(EnzymeGreeksMode::FiniteDifference);

        // Always uses fallback
        assert!(resolved.uses_fallback());
        assert_eq!(
            resolved.method,
            ComputationMethod::Fallback(CoreGreeksMode::BumpRevalue)
        );
    }

    #[test]
    fn test_resolve_mode_enzyme_only_without_enzyme() {
        let resolver = FallbackResolver::new(FallbackConfig {
            strict_enzyme_only: true,
            ..Default::default()
        });

        let resolved = resolver.resolve_mode(EnzymeGreeksMode::EnzymeOnly);

        #[cfg(not(feature = "enzyme-ad"))]
        {
            assert!(resolved.is_error());
            assert_eq!(resolved.error, Some(FallbackError::EnzymeNotAvailable));
        }
    }

    #[test]
    fn test_resolve_mode_enzyme_only_non_strict() {
        let resolver = FallbackResolver::new(FallbackConfig {
            strict_enzyme_only: false,
            ..Default::default()
        });

        let resolved = resolver.resolve_mode(EnzymeGreeksMode::EnzymeOnly);

        #[cfg(not(feature = "enzyme-ad"))]
        {
            // Should fall back instead of error
            assert!(resolved.uses_fallback());
            assert!(!resolved.is_error());
        }
    }

    #[test]
    fn test_resolved_mode_into_result() {
        let resolved = ResolvedMode::fallback(CoreGreeksMode::BumpRevalue);
        let result = resolved.into_result();

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            ComputationMethod::Fallback(CoreGreeksMode::BumpRevalue)
        );
    }

    #[test]
    fn test_resolved_mode_error_into_result() {
        let resolved = ResolvedMode::error(FallbackError::EnzymeNotAvailable);
        let result = resolved.into_result();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), FallbackError::EnzymeNotAvailable);
    }

    #[test]
    fn test_computation_method_equality() {
        let enzyme = ComputationMethod::Enzyme(EnzymeGreeksMode::ReverseMode);
        let fallback = ComputationMethod::Fallback(CoreGreeksMode::BumpRevalue);

        assert_ne!(enzyme, fallback);
        assert_eq!(
            enzyme,
            ComputationMethod::Enzyme(EnzymeGreeksMode::ReverseMode)
        );
    }

    #[test]
    fn test_is_enzyme_available() {
        let available = is_enzyme_available();

        #[cfg(feature = "enzyme-ad")]
        assert!(available);

        #[cfg(not(feature = "enzyme-ad"))]
        assert!(!available);
    }

    #[test]
    fn test_default_method() {
        let method = default_method();

        #[cfg(feature = "enzyme-ad")]
        assert!(matches!(method, ComputationMethod::Enzyme(_)));

        #[cfg(not(feature = "enzyme-ad"))]
        assert!(matches!(method, ComputationMethod::Fallback(_)));
    }

    #[test]
    fn test_resolve_enzyme_mode_utility() {
        let resolved = resolve_enzyme_mode(EnzymeGreeksMode::FiniteDifference);

        assert!(resolved.uses_fallback());
    }

    #[test]
    fn test_fallback_error_display() {
        let err = FallbackError::EnzymeNotAvailable;
        let msg = format!("{}", err);

        assert!(msg.contains("Enzyme AD not available"));
    }

    #[test]
    fn test_resolver_greeks_config() {
        let resolver = FallbackResolver::default();
        let config = resolver.greeks_config();

        // Should have default bump values
        assert!((config.spot_bump_relative - 0.01).abs() < 1e-10);
    }

    #[test]
    fn test_resolver_should_warn() {
        let resolver = FallbackResolver::new(FallbackConfig {
            warn_on_fallback: true,
            ..Default::default()
        });

        #[cfg(not(feature = "enzyme-ad"))]
        assert!(resolver.should_warn());

        #[cfg(feature = "enzyme-ad")]
        assert!(!resolver.should_warn());
    }
}
