//! Instrument pre-compilation module for efficient calibration.
//!
//! This module provides the infrastructure for compiling `MarketInstrument`
//! to static `CompiledInstrument` objects, eliminating calendar and convention
//! lookups during calibration iterations.
//!
//! # Requirements Coverage
//!
//! - **Requirement 1**: Instrument Compiler Infrastructure
//! - **Requirement 8**: Error Handling and Validation

use thiserror::Error;

// =============================================================================
// InstrumentType Enumeration (Requirement 1.4)
// =============================================================================

/// Enumeration of supported instrument types for compilation.
///
/// # Requirement 1.4
///
/// The Compiler shall support Deposit, Swap, OIS, FRA, Futures instrument types.
/// XCcyBasis, FxForward, FxSwap are explicitly unsupported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstrumentType {
    /// Simple deposit/money market instrument.
    Deposit,
    /// Interest rate swap (IRS).
    Swap,
    /// Overnight index swap.
    Ois,
    /// Forward rate agreement.
    Fra,
    /// Interest rate futures (with convexity adjustment).
    Futures,
}

impl InstrumentType {
    /// Returns the string representation of the instrument type.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::builder::compile::InstrumentType;
    ///
    /// assert_eq!(InstrumentType::Deposit.as_str(), "Deposit");
    /// assert_eq!(InstrumentType::Swap.as_str(), "Swap");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deposit => "Deposit",
            Self::Swap => "Swap",
            Self::Ois => "OIS",
            Self::Fra => "FRA",
            Self::Futures => "Futures",
        }
    }
}

impl std::fmt::Display for InstrumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// CompileError Type (Requirements 8.1-8.5, 1.3, 1.5)
// =============================================================================

/// Errors that can occur during instrument compilation.
///
/// # Requirement 8.4
///
/// The CompileError shall use `thiserror` to provide structured errors.
///
/// # Requirement 8.5
///
/// When an error occurs, the system shall include the problematic
/// instrument's index and rate ID.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum CompileError {
    /// Invalid maturity - maturity date is before the valuation date.
    ///
    /// # Requirement 8.1
    ///
    /// When the maturity date is before the valuation date, the Compiler
    /// shall return this error.
    #[error("Invalid maturity for instrument {index}: {rate_id}")]
    InvalidMaturity {
        /// Index of the problematic instrument (0-based).
        index: usize,
        /// Rate identifier of the problematic instrument.
        rate_id: String,
    },

    /// Invalid year fraction - cashflow has negative year fraction.
    ///
    /// # Requirement 8.2
    ///
    /// When a cashflow date has a negative year fraction, the Compiler
    /// shall return this error.
    #[error("Invalid year fraction at index {index} for instrument {rate_id}")]
    InvalidYearFraction {
        /// Index of the problematic instrument (0-based).
        index: usize,
        /// Rate identifier of the problematic instrument.
        rate_id: String,
    },

    /// Convention mismatch - convention and instrument type are inconsistent.
    ///
    /// # Requirement 8.3
    ///
    /// When the convention and instrument type are inconsistent, the Compiler
    /// shall return this error.
    #[error("Convention mismatch for instrument {index}: {rate_id}")]
    ConventionMismatch {
        /// Index of the problematic instrument (0-based).
        index: usize,
        /// Rate identifier of the problematic instrument.
        rate_id: String,
    },

    /// Invalid convention detected during compilation.
    ///
    /// # Requirement 1.3
    ///
    /// When an invalid convention is detected at compile time, the Compiler
    /// shall return this error.
    #[error("Invalid convention for instrument {index}: {rate_id}")]
    InvalidConvention {
        /// Index of the problematic instrument (0-based).
        index: usize,
        /// Rate identifier of the problematic instrument.
        rate_id: String,
    },

    /// Unsupported instrument type.
    ///
    /// # Requirement 1.5
    ///
    /// When the MarketConvention is XCcyBasis, FxForward, or FxSwap,
    /// the Compiler shall return this error.
    #[error("Unsupported instrument type at index {index}: {instrument_type}")]
    UnsupportedInstrument {
        /// Index of the problematic instrument (0-based).
        index: usize,
        /// Name of the unsupported instrument type.
        instrument_type: String,
    },
}

impl CompileError {
    /// Creates an invalid maturity error.
    pub fn invalid_maturity(index: usize, rate_id: impl Into<String>) -> Self {
        Self::InvalidMaturity {
            index,
            rate_id: rate_id.into(),
        }
    }

    /// Creates an invalid year fraction error.
    pub fn invalid_year_fraction(index: usize, rate_id: impl Into<String>) -> Self {
        Self::InvalidYearFraction {
            index,
            rate_id: rate_id.into(),
        }
    }

    /// Creates a convention mismatch error.
    pub fn convention_mismatch(index: usize, rate_id: impl Into<String>) -> Self {
        Self::ConventionMismatch {
            index,
            rate_id: rate_id.into(),
        }
    }

    /// Creates an invalid convention error.
    pub fn invalid_convention(index: usize, rate_id: impl Into<String>) -> Self {
        Self::InvalidConvention {
            index,
            rate_id: rate_id.into(),
        }
    }

    /// Creates an unsupported instrument error.
    pub fn unsupported_instrument(index: usize, instrument_type: impl Into<String>) -> Self {
        Self::UnsupportedInstrument {
            index,
            instrument_type: instrument_type.into(),
        }
    }

    /// Returns the instrument index where the error occurred.
    pub fn instrument_index(&self) -> usize {
        match self {
            Self::InvalidMaturity { index, .. }
            | Self::InvalidYearFraction { index, .. }
            | Self::ConventionMismatch { index, .. }
            | Self::InvalidConvention { index, .. }
            | Self::UnsupportedInstrument { index, .. } => *index,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // InstrumentType Tests (Requirement 1.4)
    // =========================================================================

    #[test]
    fn test_instrument_type_as_str() {
        assert_eq!(InstrumentType::Deposit.as_str(), "Deposit");
        assert_eq!(InstrumentType::Swap.as_str(), "Swap");
        assert_eq!(InstrumentType::Ois.as_str(), "OIS");
        assert_eq!(InstrumentType::Fra.as_str(), "FRA");
        assert_eq!(InstrumentType::Futures.as_str(), "Futures");
    }

    #[test]
    fn test_instrument_type_display() {
        assert_eq!(format!("{}", InstrumentType::Deposit), "Deposit");
        assert_eq!(format!("{}", InstrumentType::Swap), "Swap");
        assert_eq!(format!("{}", InstrumentType::Ois), "OIS");
        assert_eq!(format!("{}", InstrumentType::Fra), "FRA");
        assert_eq!(format!("{}", InstrumentType::Futures), "Futures");
    }

    #[test]
    fn test_instrument_type_clone_copy() {
        let original = InstrumentType::Swap;
        let copied = original;
        let cloned = original.clone();
        assert_eq!(original, copied);
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_instrument_type_debug() {
        let swap = InstrumentType::Swap;
        let debug_str = format!("{:?}", swap);
        assert!(debug_str.contains("Swap"));
    }

    #[test]
    fn test_instrument_type_eq() {
        assert_eq!(InstrumentType::Deposit, InstrumentType::Deposit);
        assert_ne!(InstrumentType::Deposit, InstrumentType::Swap);
    }

    #[test]
    fn test_instrument_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(InstrumentType::Deposit);
        set.insert(InstrumentType::Swap);
        set.insert(InstrumentType::Deposit); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // =========================================================================
    // CompileError Tests (Requirements 8.1-8.5)
    // =========================================================================

    #[test]
    fn test_invalid_maturity_error() {
        let err = CompileError::invalid_maturity(0, "USD-SOFR-1Y");
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid maturity"));
        assert!(msg.contains("0"));
        assert!(msg.contains("USD-SOFR-1Y"));
        assert_eq!(err.instrument_index(), 0);
    }

    #[test]
    fn test_invalid_year_fraction_error() {
        let err = CompileError::invalid_year_fraction(3, "EUR-ESTR-5Y");
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid year fraction"));
        assert!(msg.contains("3"));
        assert!(msg.contains("EUR-ESTR-5Y"));
        assert_eq!(err.instrument_index(), 3);
    }

    #[test]
    fn test_convention_mismatch_error() {
        let err = CompileError::convention_mismatch(5, "JPY-TONA-10Y");
        let msg = format!("{}", err);
        assert!(msg.contains("Convention mismatch"));
        assert!(msg.contains("5"));
        assert!(msg.contains("JPY-TONA-10Y"));
        assert_eq!(err.instrument_index(), 5);
    }

    #[test]
    fn test_invalid_convention_error() {
        let err = CompileError::invalid_convention(2, "GBP-SONIA-2Y");
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid convention"));
        assert!(msg.contains("2"));
        assert!(msg.contains("GBP-SONIA-2Y"));
        assert_eq!(err.instrument_index(), 2);
    }

    #[test]
    fn test_unsupported_instrument_error() {
        let err = CompileError::unsupported_instrument(1, "XCcyBasis");
        let msg = format!("{}", err);
        assert!(msg.contains("Unsupported instrument type"));
        assert!(msg.contains("1"));
        assert!(msg.contains("XCcyBasis"));
        assert_eq!(err.instrument_index(), 1);
    }

    #[test]
    fn test_compile_error_equality() {
        let err1 = CompileError::invalid_maturity(0, "USD-SOFR-1Y");
        let err2 = CompileError::invalid_maturity(0, "USD-SOFR-1Y");
        let err3 = CompileError::invalid_maturity(1, "USD-SOFR-1Y");
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_compile_error_debug() {
        let err = CompileError::invalid_maturity(0, "USD-SOFR-1Y");
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidMaturity"));
        assert!(debug_str.contains("USD-SOFR-1Y"));
    }

    #[test]
    fn test_compile_error_clone() {
        let original = CompileError::unsupported_instrument(5, "FxForward");
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}
