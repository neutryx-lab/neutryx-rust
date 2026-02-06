//! Market quote set collection.
//!
//! This module provides the [`MarketQuoteSet`] type for managing collections
//! of market quotes with O(1) lookup by (QuoteId, QuoteType) keys.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::{
//!     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
//! };
//! use infra_domain::time::Tenor;
//!
//! let mut quote_set = MarketQuoteSet::new();
//!
//! let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
//! let quote = MarketQuote::new(
//!     quote_id.clone(),
//!     QuoteType::Mid,
//!     0.05,
//!     1700000000000,
//!     DataSource::Bloomberg,
//! ).unwrap();
//!
//! quote_set.insert(quote);
//! assert!(quote_set.get_quote(&quote_id, QuoteType::Mid).is_some());
//! ```

use std::{collections::HashMap, time::Duration};

use crate::{
    market::{
        convention::MarketConvention,
        core::{Currency, RateType},
        source::{InstrumentMapper, SourcePriority},
        MarketInstrument,
    },
    time::Date,
    trade::Instrument,
};

use super::{
    error::MarketQuoteError, market_quote::MarketQuote, quote_id::QuoteId, quote_type::QuoteType,
};

/// A collection of market quotes with O(1) lookup.
///
/// Stores market quotes keyed by `(QuoteId, QuoteType)` for efficient access.
/// Supports multiple quote types (bid/ask/mid) for the same quote identifier.
///
/// # Examples
///
/// ```
/// use infra_domain::market::{
///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
/// };
/// use infra_domain::time::Tenor;
///
/// let mut quote_set = MarketQuoteSet::new();
///
/// // Insert bid and ask for the same quote
/// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
///
/// let bid = MarketQuote::new(quote_id.clone(), QuoteType::Bid, 0.049, 1700000000000, DataSource::Bloomberg).unwrap();
/// let ask = MarketQuote::new(quote_id.clone(), QuoteType::Ask, 0.051, 1700000000000, DataSource::Bloomberg).unwrap();
///
/// quote_set.insert(bid);
/// quote_set.insert(ask);
///
/// // Both quotes are stored
/// assert!(quote_set.get_quote(&quote_id, QuoteType::Bid).is_some());
/// assert!(quote_set.get_quote(&quote_id, QuoteType::Ask).is_some());
///
/// // Mid is computed from bid/ask
/// let mid = quote_set.get_mid_quote(&quote_id);
/// assert!(mid.is_some());
/// ```
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketQuoteSet {
    /// Quotes keyed by (QuoteId, QuoteType).
    quotes: HashMap<(QuoteId, QuoteType), MarketQuote>,
}

impl MarketQuoteSet {
    /// Creates a new empty `MarketQuoteSet`.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::MarketQuoteSet;
    ///
    /// let quote_set = MarketQuoteSet::new();
    /// assert!(quote_set.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            quotes: HashMap::new(),
        }
    }

    /// Inserts a quote into the set.
    ///
    /// If a quote with the same `(QuoteId, QuoteType)` already exists,
    /// it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `quote` - The market quote to insert
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::Tenor;
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let quote = MarketQuote::new(quote_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    ///
    /// quote_set.insert(quote);
    /// assert_eq!(quote_set.len(), 1);
    /// ```
    pub fn insert(&mut self, quote: MarketQuote) {
        let key = (quote.id.clone(), quote.quote_type);
        self.quotes.insert(key, quote);
    }

    /// Gets a quote by ID and quote type.
    ///
    /// # Arguments
    ///
    /// * `id` - The quote identifier
    /// * `quote_type` - The type of quote (bid/ask/mid/last)
    ///
    /// # Returns
    ///
    /// `Some(&MarketQuote)` if found, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::Tenor;
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let quote = MarketQuote::new(quote_id.clone(), QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    ///
    /// quote_set.insert(quote);
    ///
    /// assert!(quote_set.get_quote(&quote_id, QuoteType::Mid).is_some());
    /// assert!(quote_set.get_quote(&quote_id, QuoteType::Bid).is_none());
    /// ```
    #[must_use]
    pub fn get_quote(&self, id: &QuoteId, quote_type: QuoteType) -> Option<&MarketQuote> {
        self.quotes.get(&(id.clone(), quote_type))
    }

    /// Gets the mid quote value for a quote ID.
    ///
    /// If a mid quote exists, returns its value. Otherwise, computes
    /// the mid from bid and ask quotes if both are available.
    ///
    /// # Arguments
    ///
    /// * `id` - The quote identifier
    ///
    /// # Returns
    ///
    /// `Some(f64)` if mid can be determined, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::Tenor;
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    ///
    /// // Insert bid and ask
    /// let bid = MarketQuote::new(quote_id.clone(), QuoteType::Bid, 0.049, 1700000000000, DataSource::Bloomberg).unwrap();
    /// let ask = MarketQuote::new(quote_id.clone(), QuoteType::Ask, 0.051, 1700000000000, DataSource::Bloomberg).unwrap();
    ///
    /// quote_set.insert(bid);
    /// quote_set.insert(ask);
    ///
    /// // Mid is computed: (0.049 + 0.051) / 2 = 0.05
    /// let mid = quote_set.get_mid_quote(&quote_id).unwrap();
    /// assert!((mid - 0.05).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn get_mid_quote(&self, id: &QuoteId) -> Option<f64> {
        // First try direct mid quote
        if let Some(mid) = self.get_quote(id, QuoteType::Mid) {
            return Some(mid.value);
        }

        // Otherwise compute from bid/ask
        let bid = self.get_quote(id, QuoteType::Bid)?;
        let ask = self.get_quote(id, QuoteType::Ask)?;

        Some(f64::midpoint(bid.value, ask.value))
    }

    /// Removes a quote from the set.
    ///
    /// # Arguments
    ///
    /// * `id` - The quote identifier
    /// * `quote_type` - The type of quote to remove
    ///
    /// # Returns
    ///
    /// The removed `MarketQuote` if it existed, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::Tenor;
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let quote = MarketQuote::new(quote_id.clone(), QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    ///
    /// quote_set.insert(quote);
    /// assert_eq!(quote_set.len(), 1);
    ///
    /// let removed = quote_set.remove(&quote_id, QuoteType::Mid);
    /// assert!(removed.is_some());
    /// assert!(quote_set.is_empty());
    /// ```
    pub fn remove(&mut self, id: &QuoteId, quote_type: QuoteType) -> Option<MarketQuote> {
        self.quotes.remove(&(id.clone(), quote_type))
    }

    /// Returns an iterator over quotes of a specific rate type.
    ///
    /// # Arguments
    ///
    /// * `rate_type` - The type of rates to iterate
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::Tenor;
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    ///
    /// // Add deposit quotes
    /// let dep_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let dep = MarketQuote::new(dep_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(dep);
    ///
    /// // Add swap quotes
    /// let swap_id = QuoteId::new(Currency::USD, Tenor::FiveYears, RateType::Swap);
    /// let swap = MarketQuote::new(swap_id, QuoteType::Mid, 0.045, 1700000000000, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(swap);
    ///
    /// // Count deposit quotes
    /// let deposit_count = quote_set.quotes_by_type(RateType::Deposit).count();
    /// assert_eq!(deposit_count, 1);
    /// ```
    pub fn quotes_by_type(&self, rate_type: RateType) -> impl Iterator<Item = &MarketQuote> {
        self.quotes
            .values()
            .filter(move |quote| quote.id.rate_type == rate_type)
    }

    /// Returns quote IDs with stale timestamps.
    ///
    /// A quote is considered stale if its timestamp is older than
    /// `current_time - threshold`.
    ///
    /// # Arguments
    ///
    /// * `threshold` - The staleness threshold duration
    ///
    /// # Returns
    ///
    /// A vector of `QuoteId`s for stale quotes.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::Tenor;
    /// use std::time::Duration;
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    ///
    /// // Add an old quote (timestamp 0)
    /// let old_quote = MarketQuote::new(quote_id, QuoteType::Mid, 0.05, 0, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(old_quote);
    ///
    /// // Check for stale quotes (threshold: 1 hour)
    /// let stale = quote_set.stale_quotes(Duration::from_secs(3600));
    /// assert!(!stale.is_empty());
    /// ```
    #[must_use]
    pub fn stale_quotes(&self, threshold: Duration) -> Vec<QuoteId> {
        // Get current time in milliseconds
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
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::MarketQuoteSet;
    ///
    /// let quote_set = MarketQuoteSet::new();
    /// assert_eq!(quote_set.len(), 0);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.quotes.len()
    }

    /// Returns `true` if the set contains no quotes.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::MarketQuoteSet;
    ///
    /// let quote_set = MarketQuoteSet::new();
    /// assert!(quote_set.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.quotes.is_empty()
    }

    /// Returns a new `MarketQuoteSet` containing only quotes for the specified
    /// currency.
    ///
    /// # Arguments
    ///
    /// * `currency` - The currency to filter by
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::Tenor;
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    ///
    /// // Add USD quote
    /// let usd_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let usd = MarketQuote::new(usd_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(usd);
    ///
    /// // Add EUR quote
    /// let eur_id = QuoteId::new(Currency::EUR, Tenor::ThreeMonths, RateType::Deposit);
    /// let eur = MarketQuote::new(eur_id, QuoteType::Mid, 0.04, 1700000000000, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(eur);
    ///
    /// // Filter by USD
    /// let usd_only = quote_set.filter_by_currency(Currency::USD);
    /// assert_eq!(usd_only.len(), 1);
    /// ```
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

    /// Returns a new `MarketQuoteSet` containing only quotes valid at the given
    /// timestamp.
    ///
    /// Filters to quotes with timestamp <= the given timestamp in milliseconds.
    ///
    /// # Arguments
    ///
    /// * `timestamp_ms` - The cutoff timestamp in Unix milliseconds
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::Tenor;
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    ///
    /// // Add quote at timestamp 1000
    /// let quote = MarketQuote::new(quote_id, QuoteType::Mid, 0.05, 1000, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(quote);
    ///
    /// // Filter as of timestamp 2000 (includes the quote)
    /// let as_of_2000 = quote_set.as_of(2000);
    /// assert_eq!(as_of_2000.len(), 1);
    ///
    /// // Filter as of timestamp 500 (excludes the quote)
    /// let as_of_500 = quote_set.as_of(500);
    /// assert_eq!(as_of_500.len(), 0);
    /// ```
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
    ///
    /// When both sets contain the same `(QuoteId, QuoteType)`, the quote
    /// from the higher-priority source is kept.
    ///
    /// # Arguments
    ///
    /// * `other` - The quote set to merge from
    /// * `priority` - The source priority configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType,
    ///     DataSource, SourcePriority, Currency
    /// };
    /// use infra_domain::time::Tenor;
    ///
    /// let mut bloomberg_set = MarketQuoteSet::new();
    /// let mut reuters_set = MarketQuoteSet::new();
    ///
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    ///
    /// // Bloomberg quote
    /// let bbg_quote = MarketQuote::new(
    ///     quote_id.clone(), QuoteType::Mid, 0.050, 1700000000000, DataSource::Bloomberg
    /// ).unwrap();
    /// bloomberg_set.insert(bbg_quote);
    ///
    /// // Reuters quote (different value)
    /// let rtr_quote = MarketQuote::new(
    ///     quote_id.clone(), QuoteType::Mid, 0.051, 1700000000000, DataSource::Reuters
    /// ).unwrap();
    /// reuters_set.insert(rtr_quote);
    ///
    /// // Merge with Bloomberg priority
    /// let priority = SourcePriority::default_priority();
    /// bloomberg_set.merge(&reuters_set, &priority);
    ///
    /// // Bloomberg quote is kept (higher priority)
    /// let quote = bloomberg_set.get_quote(&quote_id, QuoteType::Mid).unwrap();
    /// assert_eq!(quote.source, DataSource::Bloomberg);
    /// ```
    pub fn merge(&mut self, other: &MarketQuoteSet, priority: &SourcePriority) {
        for quote in other.quotes.values() {
            let key = (quote.id.clone(), quote.quote_type);

            // Check if we already have this quote
            if let Some(existing) = self.quotes.get(&key) {
                // Only replace if the new quote has higher priority
                if priority.is_higher_priority(quote.source, existing.source) {
                    self.quotes.insert(key, quote.clone());
                }
            } else {
                // New quote, just insert
                self.quotes.insert(key, quote.clone());
            }
        }
    }

    /// Returns an iterator over all quotes in the set.
    pub fn iter(&self) -> impl Iterator<Item = &MarketQuote> {
        self.quotes.values()
    }

    /// Converts market quotes to instruments using the provided mapper.
    ///
    /// Maps all mid quotes (or computes mid from bid/ask) to instruments.
    /// Quotes that cannot be mapped (e.g., FX, Vol) are skipped with errors
    /// collected in the returned result.
    ///
    /// # Arguments
    ///
    /// * `mapper` - The instrument mapper to use
    /// * `valuation_date` - The valuation date for instrument date calculations
    ///
    /// # Returns
    ///
    /// A tuple of `(instruments, errors)` where:
    /// - `instruments` contains successfully mapped instruments
    /// - `errors` contains mapping failures with their quote IDs
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType,
    ///     DataSource, Currency, StandardInstrumentMapper
    /// };
    /// use infra_domain::time::{Date, Tenor};
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    ///
    /// // Add deposit quote
    /// let dep_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let dep = MarketQuote::new(dep_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(dep);
    ///
    /// // Add swap quote
    /// let swap_id = QuoteId::new(Currency::USD, Tenor::FiveYears, RateType::Swap);
    /// let swap = MarketQuote::new(swap_id, QuoteType::Mid, 0.045, 1700000000000, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(swap);
    ///
    /// let mapper = StandardInstrumentMapper::new();
    /// let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
    ///
    /// let (instruments, errors) = quote_set.to_instruments(&mapper, valuation_date);
    /// assert_eq!(instruments.len(), 2);
    /// assert!(errors.is_empty());
    /// ```
    #[must_use]
    pub fn to_instruments<M: InstrumentMapper>(
        &self,
        mapper: &M,
        valuation_date: Date,
    ) -> (Vec<Instrument>, Vec<(QuoteId, MarketQuoteError)>) {
        let mut instruments = Vec::new();
        let mut errors = Vec::new();

        // Collect unique quote IDs (ignoring quote type)
        let mut processed_ids = std::collections::HashSet::new();

        for quote in self.quotes.values() {
            // Skip if we've already processed this quote ID
            if processed_ids.contains(&quote.id) {
                continue;
            }
            processed_ids.insert(quote.id.clone());

            // Get the mid value (prefer direct mid, then compute from bid/ask)
            let mid_value = match self.get_mid_quote(&quote.id) {
                Some(v) => v,
                None => continue, // Skip if no mid available
            };

            // Create a synthetic mid quote for mapping
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

    /// Converts market quotes to instruments, returning only successful
    /// mappings.
    ///
    /// This is a convenience method that ignores mapping errors.
    /// Use [`to_instruments`](Self::to_instruments) if you need to handle
    /// errors.
    ///
    /// # Arguments
    ///
    /// * `mapper` - The instrument mapper to use
    /// * `valuation_date` - The valuation date for instrument date calculations
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType,
    ///     DataSource, Currency, StandardInstrumentMapper
    /// };
    /// use infra_domain::time::{Date, Tenor};
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let quote = MarketQuote::new(quote_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(quote);
    ///
    /// let mapper = StandardInstrumentMapper::new();
    /// let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
    ///
    /// let instruments = quote_set.to_instruments_lossy(&mapper, valuation_date);
    /// assert_eq!(instruments.len(), 1);
    /// ```
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
    ///
    /// This method creates `MarketInstrument` instances from the quotes in
    /// this set, using the appropriate `MarketConvention` for each quote.
    /// The resulting instruments are sorted by maturity date.
    ///
    /// Quotes that have no matching convention are skipped with a warning.
    ///
    /// # Arguments
    ///
    /// * `valuation_date` - The valuation date for instrument calculations
    /// * `notional` - Default notional amount for all instruments
    ///
    /// # Returns
    ///
    /// A tuple of `(instruments, skipped_ids)` where:
    /// - `instruments` are successfully created `MarketInstrument`s, sorted by maturity
    /// - `skipped_ids` are quote IDs that had no matching convention
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::{Date, Tenor};
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    ///
    /// let dep_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let dep = MarketQuote::new(dep_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(dep);
    ///
    /// let swap_id = QuoteId::new(Currency::USD, Tenor::FiveYears, RateType::Swap);
    /// let swap = MarketQuote::new(swap_id, QuoteType::Mid, 0.045, 1700000000000, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(swap);
    ///
    /// let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
    /// let (instruments, skipped) = quote_set.to_market_instruments(valuation_date, 1_000_000.0);
    ///
    /// assert_eq!(instruments.len(), 2);
    /// assert!(skipped.is_empty());
    /// // Instruments are sorted by maturity
    /// assert!(instruments[0].maturity_date <= instruments[1].maturity_date);
    /// ```
    #[must_use]
    pub fn to_market_instruments(
        &self,
        valuation_date: Date,
        notional: f64,
    ) -> (Vec<MarketInstrument>, Vec<QuoteId>) {
        let mut instruments = Vec::new();
        let mut skipped_ids = Vec::new();

        // Collect unique quote IDs (ignoring quote type)
        let mut processed_ids = std::collections::HashSet::new();

        for quote in self.quotes.values() {
            // Skip if we've already processed this quote ID
            if processed_ids.contains(&quote.id) {
                continue;
            }
            processed_ids.insert(quote.id.clone());

            // Get the mid value (prefer direct mid, then compute from bid/ask)
            let mid_value = match self.get_mid_quote(&quote.id) {
                Some(v) => v,
                None => continue, // Skip if no mid available
            };

            // Try to get a convention for this quote
            let convention = match MarketConvention::for_rate_id(&quote.id) {
                Some(c) => c,
                None => {
                    skipped_ids.push(quote.id.clone());
                    continue;
                }
            };

            // Create the MarketInstrument
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

        // Sort by maturity date
        instruments.sort_by_key(|i| i.maturity_date);

        (instruments, skipped_ids)
    }

    /// Converts market quotes to `MarketInstrument`, returning only successful conversions.
    ///
    /// This is a convenience method that ignores conversion errors.
    /// Use [`to_market_instruments`](Self::to_market_instruments) if you need to handle
    /// skipped quotes.
    ///
    /// # Arguments
    ///
    /// * `valuation_date` - The valuation date for instrument calculations
    /// * `notional` - Default notional amount for all instruments
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuoteSet, MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::{Date, Tenor};
    ///
    /// let mut quote_set = MarketQuoteSet::new();
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let quote = MarketQuote::new(quote_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// quote_set.insert(quote);
    ///
    /// let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
    /// let instruments = quote_set.to_market_instruments_lossy(valuation_date, 1_000_000.0);
    /// assert_eq!(instruments.len(), 1);
    /// ```
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

/// Type alias for backward compatibility.
#[deprecated(since = "0.2.0", note = "Use MarketQuoteSet instead")]
pub type MarketRateSet = MarketQuoteSet;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        market::{DataSource, StandardInstrumentMapper},
        time::Tenor,
    };

    fn create_quote(
        currency: Currency,
        tenor: Tenor,
        rate_type: RateType,
        quote_type: QuoteType,
        value: f64,
        timestamp: i64,
        source: DataSource,
    ) -> MarketQuote {
        let quote_id = QuoteId::new(currency, tenor, rate_type);
        MarketQuote::new(quote_id, quote_type, value, timestamp, source).unwrap()
    }

    #[test]
    fn test_quote_set_new() {
        let quote_set = MarketQuoteSet::new();
        assert!(quote_set.is_empty());
        assert_eq!(quote_set.len(), 0);
    }

    #[test]
    fn test_quote_set_default() {
        let quote_set = MarketQuoteSet::default();
        assert!(quote_set.is_empty());
    }

    #[test]
    fn test_quote_set_insert() {
        let mut quote_set = MarketQuoteSet::new();

        let quote = create_quote(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        );

        quote_set.insert(quote);
        assert_eq!(quote_set.len(), 1);
    }

    #[test]
    fn test_quote_set_get_quote() {
        let mut quote_set = MarketQuoteSet::new();

        let quote = create_quote(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        );

        quote_set.insert(quote);

        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);

        // Found
        assert!(quote_set.get_quote(&quote_id, QuoteType::Mid).is_some());

        // Not found (wrong quote type)
        assert!(quote_set.get_quote(&quote_id, QuoteType::Bid).is_none());
    }

    #[test]
    fn test_quote_set_get_mid_quote_computed() {
        let mut quote_set = MarketQuoteSet::new();

        let bid = create_quote(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Bid,
            0.049,
            1700000000000,
            DataSource::Bloomberg,
        );

        let ask = create_quote(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Ask,
            0.051,
            1700000000000,
            DataSource::Bloomberg,
        );

        quote_set.insert(bid);
        quote_set.insert(ask);

        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let mid = quote_set.get_mid_quote(&quote_id).unwrap();

        // (0.049 + 0.051) / 2 = 0.05
        assert!((mid - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_quote_set_filter_by_currency() {
        let mut quote_set = MarketQuoteSet::new();

        // USD
        quote_set.insert(create_quote(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        // EUR
        quote_set.insert(create_quote(
            Currency::EUR,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.04,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let usd_only = quote_set.filter_by_currency(Currency::USD);
        assert_eq!(usd_only.len(), 1);

        let eur_only = quote_set.filter_by_currency(Currency::EUR);
        assert_eq!(eur_only.len(), 1);
    }

    #[test]
    fn test_quote_set_merge() {
        let mut set1 = MarketQuoteSet::new();
        let mut set2 = MarketQuoteSet::new();

        set1.insert(create_quote(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        set2.insert(create_quote(
            Currency::EUR,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.04,
            1700000000000,
            DataSource::Reuters,
        ));

        let priority = SourcePriority::default_priority();
        set1.merge(&set2, &priority);

        assert_eq!(set1.len(), 2);
    }
}
