//! Market quote types and normalisation.

use infra_domain::market::Currency;

/// Type of market quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteType {
    /// Bid price (buy side).
    Bid,
    /// Ask price (sell side).
    Ask,
    /// Last traded price.
    Last,
    /// Mid price (average of bid and ask).
    Mid,
}

/// Normalised market quote from external data sources.
#[derive(Debug, Clone)]
pub struct MarketQuote {
    /// Instrument identifier (e.g., ISIN, ticker).
    pub identifier: String,
    /// Bid price.
    pub bid: Option<f64>,
    /// Ask price.
    pub ask: Option<f64>,
    /// Last traded price.
    pub last: Option<f64>,
    /// Quote currency.
    pub currency: Currency,
    /// Timestamp (Unix milliseconds).
    pub timestamp_ms: i64,
}

impl MarketQuote {
    /// Create a new market quote with bid and ask prices.
    pub fn new(identifier: impl Into<String>, bid: f64, ask: f64) -> Self {
        Self {
            identifier: identifier.into(),
            bid: Some(bid),
            ask: Some(ask),
            last: None,
            currency: Currency::USD,
            timestamp_ms: 0,
        }
    }

    /// Create a new market quote with a last traded price.
    pub fn with_last(identifier: impl Into<String>, last: f64) -> Self {
        Self {
            identifier: identifier.into(),
            bid: None,
            ask: None,
            last: Some(last),
            currency: Currency::USD,
            timestamp_ms: 0,
        }
    }

    /// Set the currency for this quote.
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    /// Set the timestamp for this quote.
    pub fn with_timestamp(mut self, timestamp_ms: i64) -> Self {
        self.timestamp_ms = timestamp_ms;
        self
    }

    /// Calculate the mid price if both bid and ask are available.
    pub fn mid(&self) -> Option<f64> {
        match (self.bid, self.ask) {
            (Some(b), Some(a)) => Some(f64::midpoint(b, a)),
            _ => None,
        }
    }

    /// Calculate the spread if both bid and ask are available.
    pub fn spread(&self) -> Option<f64> {
        match (self.bid, self.ask) {
            (Some(b), Some(a)) => Some(a - b),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_quote_mid() {
        let quote = MarketQuote::new("AAPL", 150.0, 151.0);
        assert_eq!(quote.mid(), Some(150.5));
    }

    #[test]
    fn test_market_quote_spread() {
        let quote = MarketQuote::new("AAPL", 150.0, 151.0);
        assert_eq!(quote.spread(), Some(1.0));
    }
}
