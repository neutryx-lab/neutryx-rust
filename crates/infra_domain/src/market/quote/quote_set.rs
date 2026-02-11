//! Market quote set collection.

use std::{collections::HashMap, time::Duration};

use super::{
    error::MarketQuoteError, market_quote::MarketQuote, quote_id::QuoteId, quote_type::QuoteType,
};
use crate::{
    market::{
        convention::MarketConvention,
        core::{Currency, QuoteCategory},
        source::{InstrumentMapper, SourcePriority},
        MarketInstrument,
    },
    time::Date,
    trade::Instrument,
};

/// A collection of market quotes with O(1) lookup.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketQuoteSet {
    /// Quotes keyed by (QuoteId, QuoteType).
    quotes: HashMap<(QuoteId, QuoteType), MarketQuote>,
}

impl MarketQuoteSet {
    /// Creates a new empty `MarketQuoteSet`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            quotes: HashMap::new(),
        }
    }

    /// Inserts a quote into the set.
    pub fn insert(&mut self, quote: MarketQuote) {
        let key = (quote.id.clone(), quote.quote_type);
        self.quotes.insert(key, quote);
    }

    /// Gets a quote by ID and quote type.
    #[must_use]
    pub fn get_quote(&self, id: &QuoteId, quote_type: QuoteType) -> Option<&MarketQuote> {
        self.quotes.get(&(id.clone(), quote_type))
    }

    /// Gets the mid quote value for a quote ID.
    #[must_use]
    pub fn get_mid_quote(&self, id: &QuoteId) -> Option<f64> {
        if let Some(mid) = self.get_quote(id, QuoteType::Mid) {
            return Some(mid.value);
        }

        let bid = self.get_quote(id, QuoteType::Bid)?;
        let ask = self.get_quote(id, QuoteType::Ask)?;

        Some(f64::midpoint(bid.value, ask.value))
    }

    /// Removes a quote from the set.
    pub fn remove(&mut self, id: &QuoteId, quote_type: QuoteType) -> Option<MarketQuote> {
        self.quotes.remove(&(id.clone(), quote_type))
    }

    /// Returns an iterator over quotes of a specific rate type.
    pub fn quotes_by_type(
        &self,
        quote_category: QuoteCategory,
    ) -> impl Iterator<Item = &MarketQuote> {
        self.quotes
            .values()
            .filter(move |quote| quote.id.quote_category == quote_category)
    }

    /// Returns quote IDs with stale timestamps.
    #[must_use]
    pub fn stale_quotes(&self, threshold: Duration) -> Vec<QuoteId> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let threshold_ms = threshold.as_millis() as i64;
        let cutoff = now_ms - threshold_ms;

        self.quotes
            .values()
            .filter(|quote| quote.timestamp < cutoff)
            .map(|quote| quote.id.clone())
            .collect()
    }

    /// Returns the number of quotes in the set.
    #[must_use]
    pub fn len(&self) -> usize { self.quotes.len() }

    /// Returns `true` if the set contains no quotes.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.quotes.is_empty() }

    /// Returns a new `MarketQuoteSet` containing only quotes for the specified.
    #[must_use]
    pub fn filter_by_currency(&self, currency: Currency) -> MarketQuoteSet {
        let mut result = MarketQuoteSet::new();

        for quote in self.quotes.values() {
            if quote.id.currency == currency {
                result.insert(quote.clone());
            }
        }

        result
    }

    /// Returns a new `MarketQuoteSet` containing only quotes valid at the
    /// given.
    #[must_use]
    pub fn as_of(&self, timestamp_ms: i64) -> MarketQuoteSet {
        let mut result = MarketQuoteSet::new();

        for quote in self.quotes.values() {
            if quote.timestamp <= timestamp_ms {
                result.insert(quote.clone());
            }
        }

        result
    }

    /// Merges another `MarketQuoteSet` into this one using source priority.
    pub fn merge(&mut self, other: &MarketQuoteSet, priority: &SourcePriority) {
        for quote in other.quotes.values() {
            let key = (quote.id.clone(), quote.quote_type);

            if let Some(existing) = self.quotes.get(&key) {
                if priority.is_higher_priority(quote.source, existing.source) {
                    self.quotes.insert(key, quote.clone());
                }
            } else {
                self.quotes.insert(key, quote.clone());
            }
        }
    }

    /// Returns an iterator over all quotes in the set.
    pub fn iter(&self) -> impl Iterator<Item = &MarketQuote> { self.quotes.values() }

    /// Converts market quotes to instruments using the provided mapper.
    #[must_use]
    pub fn to_instruments<M: InstrumentMapper>(
        &self,
        mapper: &M,
        valuation_date: Date,
    ) -> (Vec<Instrument>, Vec<(QuoteId, MarketQuoteError)>) {
        let mut instruments = Vec::new();
        let mut errors = Vec::new();

        let mut processed_ids = std::collections::HashSet::new();

        for quote in self.quotes.values() {
            if processed_ids.contains(&quote.id) {
                continue;
            }
            processed_ids.insert(quote.id.clone());

            let mid_value = match self.get_mid_quote(&quote.id) {
                Some(v) => v,
                None => continue,
            };

            let mid_quote = MarketQuote {
                id: quote.id.clone(),
                quote_type: QuoteType::Mid,
                value: mid_value,
                timestamp: quote.timestamp,
                source: quote.source,
            };

            match mapper.map_to_instrument(&mid_quote, valuation_date) {
                Ok(instrument) => instruments.push(instrument),
                Err(e) => errors.push((quote.id.clone(), e)),
            }
        }

        (instruments, errors)
    }

    /// Converts market quotes to instruments, returning only successful.
    #[must_use]
    pub fn to_instruments_lossy<M: InstrumentMapper>(
        &self,
        mapper: &M,
        valuation_date: Date,
    ) -> Vec<Instrument> {
        let (instruments, _) = self.to_instruments(mapper, valuation_date);
        instruments
    }

    /// Converts market quotes to `MarketInstrument` using `MarketConvention`.
    #[must_use]
    pub fn to_market_instruments(
        &self,
        valuation_date: Date,
        notional: f64,
    ) -> (Vec<MarketInstrument>, Vec<QuoteId>) {
        let mut instruments = Vec::new();
        let mut skipped_ids = Vec::new();

        let mut processed_ids = std::collections::HashSet::new();

        for quote in self.quotes.values() {
            if processed_ids.contains(&quote.id) {
                continue;
            }
            processed_ids.insert(quote.id.clone());

            let mid_value = match self.get_mid_quote(&quote.id) {
                Some(v) => v,
                None => continue,
            };

            let convention = match MarketConvention::for_quote_id(&quote.id) {
                Some(c) => c,
                None => {
                    skipped_ids.push(quote.id.clone());
                    continue;
                }
            };

            match MarketInstrument::new(
                quote.id.clone(),
                mid_value,
                convention,
                valuation_date,
                notional,
            ) {
                Ok(instrument) => instruments.push(instrument),
                Err(_) => skipped_ids.push(quote.id.clone()),
            }
        }

        instruments.sort_by_key(|i| i.maturity_date);

        (instruments, skipped_ids)
    }

    /// Converts market quotes to `MarketInstrument`, returning only successful.
    #[must_use]
    pub fn to_market_instruments_lossy(
        &self,
        valuation_date: Date,
        notional: f64,
    ) -> Vec<MarketInstrument> {
        let (instruments, _) = self.to_market_instruments(valuation_date, notional);
        instruments
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{market::DataSource, time::Tenor};

    fn make(
        ccy: Currency,
        tenor: Tenor,
        rt: QuoteCategory,
        qt: QuoteType,
        val: f64,
    ) -> MarketQuote {
        MarketQuote::new(
            QuoteId::new(ccy, tenor, rt),
            qt,
            val,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap()
    }

    #[test]
    fn test_insert_get_remove() {
        let mut qs = MarketQuoteSet::new();
        assert!(qs.is_empty());

        qs.insert(make(
            Currency::USD,
            Tenor::ThreeMonths,
            QuoteCategory::Deposit,
            QuoteType::Mid,
            0.05,
        ));
        assert_eq!(qs.len(), 1);

        let id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit);
        assert!(qs.get_quote(&id, QuoteType::Mid).is_some());
        assert!(qs.get_quote(&id, QuoteType::Bid).is_none());

        assert!(qs.remove(&id, QuoteType::Mid).is_some());
        assert!(qs.is_empty());
    }

    #[test]
    fn test_mid_computation_and_filter() {
        let mut qs = MarketQuoteSet::new();

        qs.insert(make(
            Currency::USD,
            Tenor::ThreeMonths,
            QuoteCategory::Deposit,
            QuoteType::Bid,
            0.049,
        ));
        qs.insert(make(
            Currency::USD,
            Tenor::ThreeMonths,
            QuoteCategory::Deposit,
            QuoteType::Ask,
            0.051,
        ));
        qs.insert(make(
            Currency::EUR,
            Tenor::ThreeMonths,
            QuoteCategory::Deposit,
            QuoteType::Mid,
            0.04,
        ));

        let id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit);
        let mid = qs.get_mid_quote(&id).unwrap();
        assert!((mid - 0.05).abs() < 1e-10);

        assert_eq!(qs.filter_by_currency(Currency::USD).len(), 2);
        assert_eq!(qs.filter_by_currency(Currency::EUR).len(), 1);

        let mut other = MarketQuoteSet::new();
        other.insert(make(
            Currency::GBP,
            Tenor::SixMonths,
            QuoteCategory::Swap,
            QuoteType::Mid,
            0.03,
        ));
        let priority = SourcePriority::default_priority();
        qs.merge(&other, &priority);
        assert_eq!(qs.len(), 4);
    }
}
