//! Limit enum for jump-aware curve interpolation.
//!
//! This module provides the [`Limit`] enum for specifying how to handle
//! interpolation queries at discontinuous points (jumps) in yield curves.
//!
//! # Examples
//!
//! ```
//! use pricer_core::types::Limit;
//!
//! // Default is Continuous (returns right-limit value)
//! let limit = Limit::default();
//! assert_eq!(limit, Limit::Continuous);
//!
//! // Query left-limit (before jump)
//! let left = Limit::Left;
//!
//! // Query right-limit (after jump)
//! let right = Limit::Right;
//! ```

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Specifies which limit to use when querying a curve at a jump date.
///
/// When a yield curve has discontinuities (jumps) at certain dates—such as
/// central bank meeting dates—the interpolator needs to know whether to
/// return the value just before the jump (left limit) or just after (right
/// limit).
///
/// # Variants
///
/// - [`Limit::Left`] - Returns the value immediately before the jump
/// - [`Limit::Right`] - Returns the value immediately after the jump
/// - [`Limit::Continuous`] - For continuous interpolation or defaults to right
///   limit
///
/// # Examples
///
/// ```
/// use pricer_core::types::Limit;
///
/// let limit = Limit::Left;
/// assert_ne!(limit, Limit::Right);
///
/// // Continuous is the default
/// assert_eq!(Limit::default(), Limit::Continuous);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Limit {
    /// Left limit - value immediately before the jump.
    ///
    /// Use this to query the discount factor or rate as it was
    /// just before the jump event occurred.
    Left,

    /// Right limit - value immediately after the jump.
    ///
    /// Use this to query the discount factor or rate after
    /// the jump event has been applied.
    Right,

    /// Continuous - for continuous curves or defaults to right limit at jumps.
    ///
    /// When no jump exists at the query date, returns the interpolated value.
    /// When a jump exists, behaves like [`Limit::Right`] (post-jump value).
    #[default]
    Continuous,
}

impl Limit {
    /// Returns `true` if this is the left limit.
    #[must_use]
    pub const fn is_left(&self) -> bool { matches!(self, Self::Left) }

    /// Returns `true` if this is the right limit.
    #[must_use]
    pub const fn is_right(&self) -> bool { matches!(self, Self::Right) }

    /// Returns `true` if this is continuous (no jump handling).
    #[must_use]
    pub const fn is_continuous(&self) -> bool { matches!(self, Self::Continuous) }

    /// Returns the display name of this limit type.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Continuous => "Continuous",
        }
    }
}

impl std::fmt::Display for Limit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_default_is_continuous() {
        assert_eq!(Limit::default(), Limit::Continuous);
    }

    #[test]
    fn test_limit_equality() {
        assert_eq!(Limit::Left, Limit::Left);
        assert_eq!(Limit::Right, Limit::Right);
        assert_eq!(Limit::Continuous, Limit::Continuous);

        assert_ne!(Limit::Left, Limit::Right);
        assert_ne!(Limit::Left, Limit::Continuous);
        assert_ne!(Limit::Right, Limit::Continuous);
    }

    #[test]
    fn test_limit_is_left() {
        assert!(Limit::Left.is_left());
        assert!(!Limit::Right.is_left());
        assert!(!Limit::Continuous.is_left());
    }

    #[test]
    fn test_limit_is_right() {
        assert!(!Limit::Left.is_right());
        assert!(Limit::Right.is_right());
        assert!(!Limit::Continuous.is_right());
    }

    #[test]
    fn test_limit_is_continuous() {
        assert!(!Limit::Left.is_continuous());
        assert!(!Limit::Right.is_continuous());
        assert!(Limit::Continuous.is_continuous());
    }

    #[test]
    fn test_limit_name() {
        assert_eq!(Limit::Left.name(), "Left");
        assert_eq!(Limit::Right.name(), "Right");
        assert_eq!(Limit::Continuous.name(), "Continuous");
    }

    #[test]
    fn test_limit_display() {
        assert_eq!(format!("{}", Limit::Left), "Left");
        assert_eq!(format!("{}", Limit::Right), "Right");
        assert_eq!(format!("{}", Limit::Continuous), "Continuous");
    }

    #[test]
    fn test_limit_clone_and_copy() {
        let limit = Limit::Left;
        let cloned = limit.clone();
        let copied = limit;

        assert_eq!(limit, cloned);
        assert_eq!(limit, copied);
    }

    #[test]
    fn test_limit_debug() {
        // Debug trait should be derived
        let debug_str = format!("{:?}", Limit::Left);
        assert!(debug_str.contains("Left"));
    }

    #[test]
    fn test_limit_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(Limit::Left);
        set.insert(Limit::Right);
        set.insert(Limit::Continuous);

        assert_eq!(set.len(), 3);
        assert!(set.contains(&Limit::Left));
        assert!(set.contains(&Limit::Right));
        assert!(set.contains(&Limit::Continuous));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_limit_serde_roundtrip() {
        let limits = [Limit::Left, Limit::Right, Limit::Continuous];

        for limit in limits {
            let json = serde_json::to_string(&limit).unwrap();
            let parsed: Limit = serde_json::from_str(&json).unwrap();
            assert_eq!(limit, parsed);
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_limit_serde_lowercase() {
        // Verify lowercase serialisation
        assert_eq!(serde_json::to_string(&Limit::Left).unwrap(), "\"left\"");
        assert_eq!(serde_json::to_string(&Limit::Right).unwrap(), "\"right\"");
        assert_eq!(
            serde_json::to_string(&Limit::Continuous).unwrap(),
            "\"continuous\""
        );

        // Verify deserialisation
        assert_eq!(
            serde_json::from_str::<Limit>("\"left\"").unwrap(),
            Limit::Left
        );
        assert_eq!(
            serde_json::from_str::<Limit>("\"right\"").unwrap(),
            Limit::Right
        );
        assert_eq!(
            serde_json::from_str::<Limit>("\"continuous\"").unwrap(),
            Limit::Continuous
        );
    }
}
