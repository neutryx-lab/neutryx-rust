//! Market quote type classification.

use std::fmt;

/// Classification of market quote types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
        set.insert(QuoteType::Bid);
        assert_eq!(set.len(), 2);
    }
}
