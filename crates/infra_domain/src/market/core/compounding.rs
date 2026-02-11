//! Compounding method definitions for interest rate calculations.

/// Compounding method for interest rate calculations.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    strum::Display,
    strum::EnumString,
    strum::AsRefStr,
)]
#[strum(ascii_case_insensitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CompoundingMethod {
    /// Simple interest (no compounding within period).
    #[default]
    #[strum(to_string = "Simple", serialize = "none")]
    Simple,

    /// Daily compounding (OIS indices).
    #[strum(to_string = "Compounded", serialize = "compound", serialize = "daily")]
    Compounded,

    /// Arithmetic average calculation.
    ///
    /// Used for some futures contracts where the rate is the
    /// arithmetic average of daily observations.
    ///
    /// Formula: `(Σ r_i) / n`
    #[strum(
        serialize = "Averaged",
        serialize = "average",
        serialize = "arithmetic"
    )]
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
    pub fn name(&self) -> &str { self.as_ref() }

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
