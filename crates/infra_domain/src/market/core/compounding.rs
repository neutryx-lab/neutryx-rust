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
#[derive(serde::Serialize, serde::Deserialize)]
pub enum CompoundingMethod {
    /// Simple interest (no compounding within period).
    #[default]
    #[strum(to_string = "Simple")]
    Simple,

    /// Daily compounding (OIS indices).
    #[strum(to_string = "Compounded")]
    Compounded,

    /// Arithmetic average calculation.
    #[strum(to_string = "Averaged")]
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
    fn test_from_str_canonical() {
        // Case-insensitive matching via ascii_case_insensitive
        for s in ["Simple", "simple", "SIMPLE"] {
            assert_eq!(
                s.parse::<CompoundingMethod>().unwrap(),
                CompoundingMethod::Simple
            );
        }
        for s in ["Compounded", "compounded", "COMPOUNDED"] {
            assert_eq!(
                s.parse::<CompoundingMethod>().unwrap(),
                CompoundingMethod::Compounded
            );
        }
        for s in ["Averaged", "averaged", "AVERAGED"] {
            assert_eq!(
                s.parse::<CompoundingMethod>().unwrap(),
                CompoundingMethod::Averaged
            );
        }
        assert!("unknown".parse::<CompoundingMethod>().is_err());
        assert!("".parse::<CompoundingMethod>().is_err());
        // Former aliases are no longer accepted
        assert!("none".parse::<CompoundingMethod>().is_err());
        assert!("compound".parse::<CompoundingMethod>().is_err());
        assert!("daily".parse::<CompoundingMethod>().is_err());
        assert!("average".parse::<CompoundingMethod>().is_err());
        assert!("arithmetic".parse::<CompoundingMethod>().is_err());
    }
}
