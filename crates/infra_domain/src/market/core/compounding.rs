//! Compounding method definitions for interest rate calculations.
//!
//! This module provides compounding method types for financial instruments.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::CompoundingMethod;
//!
//! let method = CompoundingMethod::Compounded;
//! assert_eq!(method.name(), "Compounded");
//!
//! // Default is Simple (for IBOR indices)
//! let default_method = CompoundingMethod::default();
//! assert_eq!(default_method, CompoundingMethod::Simple);
//! ```

use std::{fmt, str::FromStr};

/// Compounding method for interest rate calculations.
///
/// Defines how interest accrues over a period for floating rate instruments.
///
/// # Variants
///
/// - `Simple`: Simple interest (no compounding within period)
/// - `Compounded`: Daily compounding (OIS indices)
/// - `Averaged`: Arithmetic average (some futures)
///
/// # Examples
///
/// ```
/// use infra_domain::market::CompoundingMethod;
///
/// // OIS indices use compounded method
/// let ois_method = CompoundingMethod::Compounded;
/// assert_eq!(ois_method.name(), "Compounded");
///
/// // IBOR indices use simple method
/// let ibor_method = CompoundingMethod::Simple;
/// assert_eq!(ibor_method.name(), "Simple");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CompoundingMethod {
    /// Simple interest calculation (no compounding within period).
    ///
    /// Used for IBOR indices (EURIBOR, etc.) where the rate is
    /// applied once for the entire accrual period.
    ///
    /// Formula: `interest = principal × rate × time`
    #[default]
    Simple,

    /// Daily compounding calculation.
    ///
    /// Used for OIS indices (SOFR, SONIA, TONAR, SARON) where
    /// overnight rates are compounded daily.
    ///
    /// Formula: `∏(1 + r_i × δ_i) - 1`
    Compounded,

    /// Arithmetic average calculation.
    ///
    /// Used for some futures contracts where the rate is the
    /// arithmetic average of daily observations.
    ///
    /// Formula: `(Σ r_i) / n`
    Averaged,
}

impl CompoundingMethod {
    /// Returns the human-readable name of this compounding method.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::CompoundingMethod;
    ///
    /// assert_eq!(CompoundingMethod::Simple.name(), "Simple");
    /// assert_eq!(CompoundingMethod::Compounded.name(), "Compounded");
    /// assert_eq!(CompoundingMethod::Averaged.name(), "Averaged");
    /// ```
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Simple => "Simple",
            Self::Compounded => "Compounded",
            Self::Averaged => "Averaged",
        }
    }

    /// Returns true if this is a compounding method (not simple).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::CompoundingMethod;
    ///
    /// assert!(!CompoundingMethod::Simple.is_compounding());
    /// assert!(CompoundingMethod::Compounded.is_compounding());
    /// assert!(!CompoundingMethod::Averaged.is_compounding());
    /// ```
    #[must_use]
    pub const fn is_compounding(&self) -> bool { matches!(self, Self::Compounded) }

    /// Returns true if this is a simple interest method.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::CompoundingMethod;
    ///
    /// assert!(CompoundingMethod::Simple.is_simple());
    /// assert!(!CompoundingMethod::Compounded.is_simple());
    /// ```
    #[must_use]
    pub const fn is_simple(&self) -> bool { matches!(self, Self::Simple) }
}

impl fmt::Display for CompoundingMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.name()) }
}

impl FromStr for CompoundingMethod {
    type Err = String;

    /// Parses compounding method from string (case-insensitive).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::CompoundingMethod;
    ///
    /// assert_eq!("Simple".parse::<CompoundingMethod>().unwrap(), CompoundingMethod::Simple);
    /// assert_eq!("compounded".parse::<CompoundingMethod>().unwrap(), CompoundingMethod::Compounded);
    /// assert_eq!("AVERAGED".parse::<CompoundingMethod>().unwrap(), CompoundingMethod::Averaged);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "simple" | "none" => Ok(Self::Simple),
            "compounded" | "compound" | "daily" => Ok(Self::Compounded),
            "averaged" | "average" | "arithmetic" => Ok(Self::Averaged),
            _ => Err(format!("Unknown compounding method: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_synonyms() {
        // Simple synonyms
        for s in ["Simple", "simple", "SIMPLE", "none"] {
            assert_eq!(
                s.parse::<CompoundingMethod>().unwrap(),
                CompoundingMethod::Simple
            );
        }
        // Compounded synonyms
        for s in ["Compounded", "compounded", "compound", "daily"] {
            assert_eq!(
                s.parse::<CompoundingMethod>().unwrap(),
                CompoundingMethod::Compounded
            );
        }
        // Averaged synonyms
        for s in ["Averaged", "averaged", "average", "arithmetic"] {
            assert_eq!(
                s.parse::<CompoundingMethod>().unwrap(),
                CompoundingMethod::Averaged
            );
        }
        // Invalid
        assert!("unknown".parse::<CompoundingMethod>().is_err());
        assert!("".parse::<CompoundingMethod>().is_err());
    }
}
