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
    #[strum(
        serialize = "Averaged",
        serialize = "average",
        serialize = "arithmetic"
    )]
    Averaged,
}

impl CompoundingMethod {
    /// Returns the human-readable name of this compounding method.
    #[must_use]
    pub fn name(&self) -> &str { self.as_ref() }

    /// Returns true if this is a compounding method (not simple).
    #[must_use]
    pub const fn is_compounding(&self) -> bool { matches!(self, Self::Compounded) }

    /// Returns true if this is a simple interest method.
    #[must_use]
    pub const fn is_simple(&self) -> bool { matches!(self, Self::Simple) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_synonyms() {
        for s in ["Simple", "simple", "SIMPLE", "none"] {
            assert_eq!(
                s.parse::<CompoundingMethod>().unwrap(),
                CompoundingMethod::Simple
            );
        }
        for s in ["Compounded", "compounded", "compound", "daily"] {
            assert_eq!(
                s.parse::<CompoundingMethod>().unwrap(),
                CompoundingMethod::Compounded
            );
        }
        for s in ["Averaged", "averaged", "average", "arithmetic"] {
            assert_eq!(
                s.parse::<CompoundingMethod>().unwrap(),
                CompoundingMethod::Averaged
            );
        }
        assert!("unknown".parse::<CompoundingMethod>().is_err());
        assert!("".parse::<CompoundingMethod>().is_err());
    }
}
