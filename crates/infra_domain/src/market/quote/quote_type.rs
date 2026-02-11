//! Market quote type classification.
//!
//! This module provides the [`QuoteType`] enum for classifying market quotes
//! as bid, ask, mid, or last prices.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::QuoteType;
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
/// use infra_domain::market::QuoteType;
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
    /// use infra_domain::market::QuoteType;
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.code()) }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_code_display_and_hash() {
        let cases = [
            ("BID", QuoteType::Bid),
            ("ASK", QuoteType::Ask),
            ("MID", QuoteType::Mid),
            ("LAST", QuoteType::Last),
        ];
        for (code, qt) in &cases {
            assert_eq!(qt.code(), *code);
            assert_eq!(format!("{}", qt), *code);
        }

        let mut set = HashSet::new();
        set.insert(QuoteType::Bid);
        set.insert(QuoteType::Ask);
        set.insert(QuoteType::Bid); // duplicate
        assert_eq!(set.len(), 2);
    }
}
