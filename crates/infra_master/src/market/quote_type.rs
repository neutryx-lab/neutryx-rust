//! Market quote type classification.
//!
//! This module provides the [`QuoteType`] enum for classifying market quotes
//! as bid, ask, mid, or last prices.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::QuoteType;
//!
//! let quote = QuoteType::Mid;
//! assert_eq!(quote, QuoteType::Mid);
//! ```

use std::fmt;

/// Classification of market quote types.
///
/// Represents the different types of prices that can be quoted for
/// a financial instrument in the market.
///
/// # Variants
///
/// - `Bid`: The highest price a buyer is willing to pay
/// - `Ask`: The lowest price a seller is willing to accept
/// - `Mid`: The midpoint between bid and ask prices
/// - `Last`: The most recent traded price
///
/// # Examples
///
/// ```
/// use infra_master::market::QuoteType;
///
/// let bid = QuoteType::Bid;
/// let ask = QuoteType::Ask;
///
/// // QuoteType implements Copy and Clone
/// let bid_copy = bid;
/// assert_eq!(bid, bid_copy);
///
/// // Can be used as HashMap keys
/// use std::collections::HashMap;
/// let mut quotes: HashMap<QuoteType, f64> = HashMap::new();
/// quotes.insert(QuoteType::Bid, 100.0);
/// quotes.insert(QuoteType::Ask, 101.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QuoteType {
    /// Bid price - the highest price a buyer is willing to pay.
    Bid,
    /// Ask price - the lowest price a seller is willing to accept.
    Ask,
    /// Mid price - the midpoint between bid and ask.
    Mid,
    /// Last traded price.
    Last,
}

impl QuoteType {
    /// Returns a short code for this quote type.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::QuoteType;
    ///
    /// assert_eq!(QuoteType::Bid.code(), "BID");
    /// assert_eq!(QuoteType::Ask.code(), "ASK");
    /// assert_eq!(QuoteType::Mid.code(), "MID");
    /// assert_eq!(QuoteType::Last.code(), "LAST");
    /// ```
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            QuoteType::Bid => "BID",
            QuoteType::Ask => "ASK",
            QuoteType::Mid => "MID",
            QuoteType::Last => "LAST",
        }
    }
}

impl fmt::Display for QuoteType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_quote_type_variants() {
        let bid = QuoteType::Bid;
        let ask = QuoteType::Ask;
        let mid = QuoteType::Mid;
        let last = QuoteType::Last;

        assert_eq!(bid, QuoteType::Bid);
        assert_eq!(ask, QuoteType::Ask);
        assert_eq!(mid, QuoteType::Mid);
        assert_eq!(last, QuoteType::Last);
    }

    #[test]
    fn test_quote_type_code() {
        assert_eq!(QuoteType::Bid.code(), "BID");
        assert_eq!(QuoteType::Ask.code(), "ASK");
        assert_eq!(QuoteType::Mid.code(), "MID");
        assert_eq!(QuoteType::Last.code(), "LAST");
    }

    #[test]
    fn test_quote_type_display() {
        assert_eq!(format!("{}", QuoteType::Bid), "BID");
        assert_eq!(format!("{}", QuoteType::Ask), "ASK");
        assert_eq!(format!("{}", QuoteType::Mid), "MID");
        assert_eq!(format!("{}", QuoteType::Last), "LAST");
    }

    #[test]
    fn test_quote_type_copy() {
        let original = QuoteType::Bid;
        let copied = original;
        assert_eq!(original, copied);
    }

    #[test]
    fn test_quote_type_clone() {
        let original = QuoteType::Ask;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_quote_type_eq() {
        assert_eq!(QuoteType::Bid, QuoteType::Bid);
        assert_ne!(QuoteType::Bid, QuoteType::Ask);
        assert_ne!(QuoteType::Mid, QuoteType::Last);
    }

    #[test]
    fn test_quote_type_hash() {
        let mut set = HashSet::new();
        set.insert(QuoteType::Bid);
        set.insert(QuoteType::Ask);
        set.insert(QuoteType::Bid); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&QuoteType::Bid));
        assert!(set.contains(&QuoteType::Ask));
    }

    #[test]
    fn test_quote_type_as_hashmap_key() {
        let mut map: HashMap<QuoteType, f64> = HashMap::new();
        map.insert(QuoteType::Bid, 100.0);
        map.insert(QuoteType::Ask, 101.0);
        map.insert(QuoteType::Mid, 100.5);
        map.insert(QuoteType::Last, 100.25);

        assert_eq!(map.get(&QuoteType::Bid), Some(&100.0));
        assert_eq!(map.get(&QuoteType::Ask), Some(&101.0));
        assert_eq!(map.get(&QuoteType::Mid), Some(&100.5));
        assert_eq!(map.get(&QuoteType::Last), Some(&100.25));
    }

    #[test]
    fn test_quote_type_debug() {
        assert_eq!(format!("{:?}", QuoteType::Bid), "Bid");
        assert_eq!(format!("{:?}", QuoteType::Ask), "Ask");
        assert_eq!(format!("{:?}", QuoteType::Mid), "Mid");
        assert_eq!(format!("{:?}", QuoteType::Last), "Last");
    }
}
