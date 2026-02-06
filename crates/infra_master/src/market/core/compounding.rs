//! Compounding method definitions for interest rate calculations.
//!
//! This module provides compounding method types for financial instruments.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::CompoundingMethod;
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
/// use infra_master::market::CompoundingMethod;
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
    /// use infra_master::market::CompoundingMethod;
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
    /// use infra_master::market::CompoundingMethod;
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
    /// use infra_master::market::CompoundingMethod;
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
    /// use infra_master::market::CompoundingMethod;
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

    // ========================================
    // Basic Properties Tests
    // ========================================

    #[test]
    fn test_default_is_simple() {
        assert_eq!(CompoundingMethod::default(), CompoundingMethod::Simple);
    }

    #[test]
    fn test_name() {
        assert_eq!(CompoundingMethod::Simple.name(), "Simple");
        assert_eq!(CompoundingMethod::Compounded.name(), "Compounded");
        assert_eq!(CompoundingMethod::Averaged.name(), "Averaged");
    }

    #[test]
    fn test_is_compounding() {
        assert!(!CompoundingMethod::Simple.is_compounding());
        assert!(CompoundingMethod::Compounded.is_compounding());
        assert!(!CompoundingMethod::Averaged.is_compounding());
    }

    #[test]
    fn test_is_simple() {
        assert!(CompoundingMethod::Simple.is_simple());
        assert!(!CompoundingMethod::Compounded.is_simple());
        assert!(!CompoundingMethod::Averaged.is_simple());
    }

    // ========================================
    // Display and FromStr Tests
    // ========================================

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", CompoundingMethod::Simple), "Simple");
        assert_eq!(format!("{}", CompoundingMethod::Compounded), "Compounded");
        assert_eq!(format!("{}", CompoundingMethod::Averaged), "Averaged");
    }

    #[test]
    fn test_from_str_simple() {
        assert_eq!(
            "Simple".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Simple
        );
        assert_eq!(
            "simple".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Simple
        );
        assert_eq!(
            "SIMPLE".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Simple
        );
        assert_eq!(
            "none".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Simple
        );
    }

    #[test]
    fn test_from_str_compounded() {
        assert_eq!(
            "Compounded".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Compounded
        );
        assert_eq!(
            "compounded".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Compounded
        );
        assert_eq!(
            "compound".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Compounded
        );
        assert_eq!(
            "daily".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Compounded
        );
    }

    #[test]
    fn test_from_str_averaged() {
        assert_eq!(
            "Averaged".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Averaged
        );
        assert_eq!(
            "averaged".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Averaged
        );
        assert_eq!(
            "average".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Averaged
        );
        assert_eq!(
            "arithmetic".parse::<CompoundingMethod>().unwrap(),
            CompoundingMethod::Averaged
        );
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("unknown".parse::<CompoundingMethod>().is_err());
        assert!("".parse::<CompoundingMethod>().is_err());
    }

    // ========================================
    // Trait Implementation Tests
    // ========================================

    #[test]
    fn test_clone() {
        let method = CompoundingMethod::Compounded;
        let cloned = method;
        assert_eq!(method, cloned);
    }

    #[test]
    fn test_copy() {
        let method = CompoundingMethod::Simple;
        let copied = method;
        assert_eq!(method, copied);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(CompoundingMethod::Simple);
        set.insert(CompoundingMethod::Compounded);
        set.insert(CompoundingMethod::Simple); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_debug() {
        let debug = format!("{:?}", CompoundingMethod::Compounded);
        assert!(debug.contains("Compounded"));
    }

    #[test]
    fn test_equality() {
        assert_eq!(CompoundingMethod::Simple, CompoundingMethod::Simple);
        assert_ne!(CompoundingMethod::Simple, CompoundingMethod::Compounded);
        assert_ne!(CompoundingMethod::Compounded, CompoundingMethod::Averaged);
    }
}
