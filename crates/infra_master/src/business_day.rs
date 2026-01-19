//! Business day convention definitions.
//!
//! This module provides business day adjustment conventions for
//! handling dates that fall on non-business days.
//!
//! # Examples
//!
//! ```
//! use infra_master::BusinessDayConvention;
//!
//! let conv = BusinessDayConvention::ModifiedFollowing;
//! assert_eq!(conv.name(), "Modified Following");
//! assert_eq!(conv.code(), "MF");
//! ```

use std::{fmt, str::FromStr};

/// Business Day Convention for date adjustments.
///
/// Defines how to adjust dates that fall on non-business days
/// (weekends, holidays).
///
/// # Variants
///
/// - `Following`: Move to the next business day
/// - `ModifiedFollowing`: Move to the next business day, unless it crosses a
///   month boundary
/// - `Preceding`: Move to the previous business day
/// - `ModifiedPreceding`: Move to the previous business day, unless it crosses
///   a month boundary
/// - `Unadjusted`: Do not adjust the date
///
/// # Examples
///
/// ```
/// use infra_master::BusinessDayConvention;
///
/// let conv = BusinessDayConvention::ModifiedFollowing;
/// assert_eq!(conv.name(), "Modified Following");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BusinessDayConvention {
    /// Move to the next business day.
    ///
    /// If a date falls on a weekend or holiday, move forward to the
    /// next valid business day.
    Following,

    /// Move to the next business day, unless it crosses a month boundary.
    ///
    /// If moving forward would cross into a new month, move backward
    /// to the previous business day instead. This is the most common
    /// convention for money market instruments.
    ModifiedFollowing,

    /// Move to the previous business day.
    ///
    /// If a date falls on a weekend or holiday, move backward to the
    /// previous valid business day.
    Preceding,

    /// Move to the previous business day, unless it crosses a month boundary.
    ///
    /// If moving backward would cross into a previous month, move forward
    /// to the next business day instead.
    ModifiedPreceding,

    /// Do not adjust the date.
    ///
    /// The date is used as-is, even if it falls on a weekend or holiday.
    Unadjusted,
}

impl BusinessDayConvention {
    /// Returns the standard name for this convention.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::BusinessDayConvention;
    ///
    /// assert_eq!(BusinessDayConvention::Following.name(), "Following");
    /// assert_eq!(BusinessDayConvention::ModifiedFollowing.name(), "Modified Following");
    /// ```
    #[inline]
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            BusinessDayConvention::Following => "Following",
            BusinessDayConvention::ModifiedFollowing => "Modified Following",
            BusinessDayConvention::Preceding => "Preceding",
            BusinessDayConvention::ModifiedPreceding => "Modified Preceding",
            BusinessDayConvention::Unadjusted => "Unadjusted",
        }
    }

    /// Returns a short code for this convention.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::BusinessDayConvention;
    ///
    /// assert_eq!(BusinessDayConvention::Following.code(), "F");
    /// assert_eq!(BusinessDayConvention::ModifiedFollowing.code(), "MF");
    /// ```
    #[inline]
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            BusinessDayConvention::Following => "F",
            BusinessDayConvention::ModifiedFollowing => "MF",
            BusinessDayConvention::Preceding => "P",
            BusinessDayConvention::ModifiedPreceding => "MP",
            BusinessDayConvention::Unadjusted => "U",
        }
    }
}

impl fmt::Display for BusinessDayConvention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.name()) }
}

impl FromStr for BusinessDayConvention {
    type Err = String;

    /// Parses business day convention from string (case-insensitive).
    ///
    /// Supports full names and short codes:
    /// - Following: "following", "f"
    /// - ModifiedFollowing: "modified following", "modifiedfollowing", "mf"
    /// - Preceding: "preceding", "p"
    /// - ModifiedPreceding: "modified preceding", "modifiedpreceding", "mp"
    /// - Unadjusted: "unadjusted", "u", "none"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace([' ', '_', '-'], "").as_str() {
            "following" | "f" => Ok(BusinessDayConvention::Following),
            "modifiedfollowing" | "mf" => Ok(BusinessDayConvention::ModifiedFollowing),
            "preceding" | "p" => Ok(BusinessDayConvention::Preceding),
            "modifiedpreceding" | "mp" => Ok(BusinessDayConvention::ModifiedPreceding),
            "unadjusted" | "u" | "none" => Ok(BusinessDayConvention::Unadjusted),
            _ => Err(format!("Unknown business day convention: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(BusinessDayConvention::Following.name(), "Following");
        assert_eq!(
            BusinessDayConvention::ModifiedFollowing.name(),
            "Modified Following"
        );
        assert_eq!(BusinessDayConvention::Preceding.name(), "Preceding");
        assert_eq!(
            BusinessDayConvention::ModifiedPreceding.name(),
            "Modified Preceding"
        );
        assert_eq!(BusinessDayConvention::Unadjusted.name(), "Unadjusted");
    }

    #[test]
    fn test_code() {
        assert_eq!(BusinessDayConvention::Following.code(), "F");
        assert_eq!(BusinessDayConvention::ModifiedFollowing.code(), "MF");
        assert_eq!(BusinessDayConvention::Preceding.code(), "P");
        assert_eq!(BusinessDayConvention::ModifiedPreceding.code(), "MP");
        assert_eq!(BusinessDayConvention::Unadjusted.code(), "U");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", BusinessDayConvention::Following), "Following");
        assert_eq!(
            format!("{}", BusinessDayConvention::ModifiedFollowing),
            "Modified Following"
        );
    }

    #[test]
    fn test_from_str_full_names() {
        assert_eq!(
            "Following".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Following
        );
        assert_eq!(
            "modified following"
                .parse::<BusinessDayConvention>()
                .unwrap(),
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(
            "Modified_Following"
                .parse::<BusinessDayConvention>()
                .unwrap(),
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(
            "PRECEDING".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Preceding
        );
        assert_eq!(
            "unadjusted".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Unadjusted
        );
    }

    #[test]
    fn test_from_str_short_codes() {
        assert_eq!(
            "F".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Following
        );
        assert_eq!(
            "MF".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(
            "P".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Preceding
        );
        assert_eq!(
            "MP".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::ModifiedPreceding
        );
        assert_eq!(
            "U".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Unadjusted
        );
        assert_eq!(
            "none".parse::<BusinessDayConvention>().unwrap(),
            BusinessDayConvention::Unadjusted
        );
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("invalid".parse::<BusinessDayConvention>().is_err());
        assert!("FFF".parse::<BusinessDayConvention>().is_err());
    }

    #[test]
    fn test_clone_and_copy() {
        let bdc1 = BusinessDayConvention::ModifiedFollowing;
        let bdc2 = bdc1; // Copy
        let bdc3 = bdc1.clone(); // Clone

        assert_eq!(bdc1, bdc2);
        assert_eq!(bdc1, bdc3);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BusinessDayConvention::Following);
        set.insert(BusinessDayConvention::ModifiedFollowing);
        set.insert(BusinessDayConvention::Following); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_debug() {
        let debug_str = format!("{:?}", BusinessDayConvention::ModifiedFollowing);
        assert!(debug_str.contains("ModifiedFollowing"));
    }
}
