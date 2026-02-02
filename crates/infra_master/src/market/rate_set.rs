//! Market rate set collection.
//!
//! This module provides the [`MarketRateSet`] type for managing collections
//! of market rates with O(1) lookup by (RateId, QuoteType) keys.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::{
//!     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
//! };
//! use infra_master::time::Tenor;
//!
//! let mut rate_set = MarketRateSet::new();
//!
//! let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
//! let rate = MarketRate::new(
//!     rate_id.clone(),
//!     QuoteType::Mid,
//!     0.05,
//!     1700000000000,
//!     DataSource::Bloomberg,
//! ).unwrap();
//!
//! rate_set.insert(rate);
//! assert!(rate_set.get_rate(&rate_id, QuoteType::Mid).is_some());
//! ```

use std::{collections::HashMap, time::Duration};

use super::{
    convention::MarketConvention,
    data_source::SourcePriority,
    error::MarketRateError,
    instrument::MarketInstrument,
    mapper::InstrumentMapper,
    quote_type::QuoteType,
    rate::MarketRate,
    rate_id::RateId,
    rate_type::RateType,
};
use crate::{market::Currency, time::Date, trade::Instrument};

/// A collection of market rates with O(1) lookup.
///
/// Stores market rates keyed by `(RateId, QuoteType)` for efficient access.
/// Supports multiple quote types (bid/ask/mid) for the same rate identifier.
///
/// # Examples
///
/// ```
/// use infra_master::market::{
///     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
/// };
/// use infra_master::time::Tenor;
///
/// let mut rate_set = MarketRateSet::new();
///
/// // Insert bid and ask for the same rate
/// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
///
/// let bid = MarketRate::new(rate_id.clone(), QuoteType::Bid, 0.049, 1700000000000, DataSource::Bloomberg).unwrap();
/// let ask = MarketRate::new(rate_id.clone(), QuoteType::Ask, 0.051, 1700000000000, DataSource::Bloomberg).unwrap();
///
/// rate_set.insert(bid);
/// rate_set.insert(ask);
///
/// // Both quotes are stored
/// assert!(rate_set.get_rate(&rate_id, QuoteType::Bid).is_some());
/// assert!(rate_set.get_rate(&rate_id, QuoteType::Ask).is_some());
///
/// // Mid is computed from bid/ask
/// let mid = rate_set.get_mid_rate(&rate_id);
/// assert!(mid.is_some());
/// ```
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketRateSet {
    /// Rates keyed by (RateId, QuoteType).
    rates: HashMap<(RateId, QuoteType), MarketRate>,
}

impl MarketRateSet {
    /// Creates a new empty `MarketRateSet`.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::MarketRateSet;
    ///
    /// let rate_set = MarketRateSet::new();
    /// assert!(rate_set.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            rates: HashMap::new(),
        }
    }

    /// Inserts a rate into the set.
    ///
    /// If a rate with the same `(RateId, QuoteType)` already exists,
    /// it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `rate` - The market rate to insert
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::Tenor;
    ///
    /// let mut rate_set = MarketRateSet::new();
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let rate = MarketRate::new(rate_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    ///
    /// rate_set.insert(rate);
    /// assert_eq!(rate_set.len(), 1);
    /// ```
    pub fn insert(&mut self, rate: MarketRate) {
        let key = (rate.id.clone(), rate.quote_type);
        self.rates.insert(key, rate);
    }

    /// Gets a rate by ID and quote type.
    ///
    /// # Arguments
    ///
    /// * `id` - The rate identifier
    /// * `quote_type` - The type of quote (bid/ask/mid/last)
    ///
    /// # Returns
    ///
    /// `Some(&MarketRate)` if found, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::Tenor;
    ///
    /// let mut rate_set = MarketRateSet::new();
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let rate = MarketRate::new(rate_id.clone(), QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    ///
    /// rate_set.insert(rate);
    ///
    /// assert!(rate_set.get_rate(&rate_id, QuoteType::Mid).is_some());
    /// assert!(rate_set.get_rate(&rate_id, QuoteType::Bid).is_none());
    /// ```
    #[must_use]
    pub fn get_rate(&self, id: &RateId, quote_type: QuoteType) -> Option<&MarketRate> {
        self.rates.get(&(id.clone(), quote_type))
    }

    /// Gets the mid rate value for a rate ID.
    ///
    /// If a mid quote exists, returns its value. Otherwise, computes
    /// the mid from bid and ask quotes if both are available.
    ///
    /// # Arguments
    ///
    /// * `id` - The rate identifier
    ///
    /// # Returns
    ///
    /// `Some(f64)` if mid can be determined, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::Tenor;
    ///
    /// let mut rate_set = MarketRateSet::new();
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    ///
    /// // Insert bid and ask
    /// let bid = MarketRate::new(rate_id.clone(), QuoteType::Bid, 0.049, 1700000000000, DataSource::Bloomberg).unwrap();
    /// let ask = MarketRate::new(rate_id.clone(), QuoteType::Ask, 0.051, 1700000000000, DataSource::Bloomberg).unwrap();
    ///
    /// rate_set.insert(bid);
    /// rate_set.insert(ask);
    ///
    /// // Mid is computed: (0.049 + 0.051) / 2 = 0.05
    /// let mid = rate_set.get_mid_rate(&rate_id).unwrap();
    /// assert!((mid - 0.05).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn get_mid_rate(&self, id: &RateId) -> Option<f64> {
        // First try direct mid quote
        if let Some(mid) = self.get_rate(id, QuoteType::Mid) {
            return Some(mid.value);
        }

        // Otherwise compute from bid/ask
        let bid = self.get_rate(id, QuoteType::Bid)?;
        let ask = self.get_rate(id, QuoteType::Ask)?;

        Some(f64::midpoint(bid.value, ask.value))
    }

    /// Removes a rate from the set.
    ///
    /// # Arguments
    ///
    /// * `id` - The rate identifier
    /// * `quote_type` - The type of quote to remove
    ///
    /// # Returns
    ///
    /// The removed `MarketRate` if it existed, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::Tenor;
    ///
    /// let mut rate_set = MarketRateSet::new();
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let rate = MarketRate::new(rate_id.clone(), QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    ///
    /// rate_set.insert(rate);
    /// assert_eq!(rate_set.len(), 1);
    ///
    /// let removed = rate_set.remove(&rate_id, QuoteType::Mid);
    /// assert!(removed.is_some());
    /// assert!(rate_set.is_empty());
    /// ```
    pub fn remove(&mut self, id: &RateId, quote_type: QuoteType) -> Option<MarketRate> {
        self.rates.remove(&(id.clone(), quote_type))
    }

    /// Returns an iterator over rates of a specific type.
    ///
    /// # Arguments
    ///
    /// * `rate_type` - The type of rates to iterate
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::Tenor;
    ///
    /// let mut rate_set = MarketRateSet::new();
    ///
    /// // Add deposit rates
    /// let dep_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let dep = MarketRate::new(dep_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(dep);
    ///
    /// // Add swap rates
    /// let swap_id = RateId::new(Currency::USD, Tenor::FiveYears, RateType::Swap);
    /// let swap = MarketRate::new(swap_id, QuoteType::Mid, 0.045, 1700000000000, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(swap);
    ///
    /// // Count deposit rates
    /// let deposit_count = rate_set.rates_by_type(RateType::Deposit).count();
    /// assert_eq!(deposit_count, 1);
    /// ```
    pub fn rates_by_type(&self, rate_type: RateType) -> impl Iterator<Item = &MarketRate> {
        self.rates
            .values()
            .filter(move |rate| rate.id.rate_type == rate_type)
    }

    /// Returns rate IDs with stale timestamps.
    ///
    /// A rate is considered stale if its timestamp is older than
    /// `current_time - threshold`.
    ///
    /// # Arguments
    ///
    /// * `threshold` - The staleness threshold duration
    ///
    /// # Returns
    ///
    /// A vector of `RateId`s for stale rates.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::Tenor;
    /// use std::time::Duration;
    ///
    /// let mut rate_set = MarketRateSet::new();
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    ///
    /// // Add an old rate (timestamp 0)
    /// let old_rate = MarketRate::new(rate_id, QuoteType::Mid, 0.05, 0, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(old_rate);
    ///
    /// // Check for stale rates (threshold: 1 hour)
    /// let stale = rate_set.stale_rates(Duration::from_secs(3600));
    /// assert!(!stale.is_empty());
    /// ```
    #[must_use]
    pub fn stale_rates(&self, threshold: Duration) -> Vec<RateId> {
        // Get current time in milliseconds
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let threshold_ms = threshold.as_millis() as i64;
        let cutoff = now_ms - threshold_ms;

        self.rates
            .values()
            .filter(|rate| rate.timestamp < cutoff)
            .map(|rate| rate.id.clone())
            .collect()
    }

    /// Returns the number of rates in the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::MarketRateSet;
    ///
    /// let rate_set = MarketRateSet::new();
    /// assert_eq!(rate_set.len(), 0);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize { self.rates.len() }

    /// Returns `true` if the set contains no rates.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::MarketRateSet;
    ///
    /// let rate_set = MarketRateSet::new();
    /// assert!(rate_set.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool { self.rates.is_empty() }

    /// Returns a new `MarketRateSet` containing only rates for the specified
    /// currency.
    ///
    /// # Arguments
    ///
    /// * `currency` - The currency to filter by
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::Tenor;
    ///
    /// let mut rate_set = MarketRateSet::new();
    ///
    /// // Add USD rate
    /// let usd_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let usd = MarketRate::new(usd_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(usd);
    ///
    /// // Add EUR rate
    /// let eur_id = RateId::new(Currency::EUR, Tenor::ThreeMonths, RateType::Deposit);
    /// let eur = MarketRate::new(eur_id, QuoteType::Mid, 0.04, 1700000000000, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(eur);
    ///
    /// // Filter by USD
    /// let usd_only = rate_set.filter_by_currency(Currency::USD);
    /// assert_eq!(usd_only.len(), 1);
    /// ```
    #[must_use]
    pub fn filter_by_currency(&self, currency: Currency) -> MarketRateSet {
        let mut result = MarketRateSet::new();

        for rate in self.rates.values() {
            if rate.id.currency == currency {
                result.insert(rate.clone());
            }
        }

        result
    }

    /// Returns a new `MarketRateSet` containing only rates valid at the given
    /// timestamp.
    ///
    /// Filters to rates with timestamp <= the given timestamp in milliseconds.
    ///
    /// # Arguments
    ///
    /// * `timestamp_ms` - The cutoff timestamp in Unix milliseconds
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::Tenor;
    ///
    /// let mut rate_set = MarketRateSet::new();
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    ///
    /// // Add rate at timestamp 1000
    /// let rate = MarketRate::new(rate_id, QuoteType::Mid, 0.05, 1000, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(rate);
    ///
    /// // Filter as of timestamp 2000 (includes the rate)
    /// let as_of_2000 = rate_set.as_of(2000);
    /// assert_eq!(as_of_2000.len(), 1);
    ///
    /// // Filter as of timestamp 500 (excludes the rate)
    /// let as_of_500 = rate_set.as_of(500);
    /// assert_eq!(as_of_500.len(), 0);
    /// ```
    #[must_use]
    pub fn as_of(&self, timestamp_ms: i64) -> MarketRateSet {
        let mut result = MarketRateSet::new();

        for rate in self.rates.values() {
            if rate.timestamp <= timestamp_ms {
                result.insert(rate.clone());
            }
        }

        result
    }

    /// Merges another `MarketRateSet` into this one using source priority.
    ///
    /// When both sets contain the same `(RateId, QuoteType)`, the rate
    /// from the higher-priority source is kept.
    ///
    /// # Arguments
    ///
    /// * `other` - The rate set to merge from
    /// * `priority` - The source priority configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType,
    ///     DataSource, SourcePriority, Currency
    /// };
    /// use infra_master::time::Tenor;
    ///
    /// let mut bloomberg_set = MarketRateSet::new();
    /// let mut reuters_set = MarketRateSet::new();
    ///
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    ///
    /// // Bloomberg rate
    /// let bbg_rate = MarketRate::new(
    ///     rate_id.clone(), QuoteType::Mid, 0.050, 1700000000000, DataSource::Bloomberg
    /// ).unwrap();
    /// bloomberg_set.insert(bbg_rate);
    ///
    /// // Reuters rate (different value)
    /// let rtr_rate = MarketRate::new(
    ///     rate_id.clone(), QuoteType::Mid, 0.051, 1700000000000, DataSource::Reuters
    /// ).unwrap();
    /// reuters_set.insert(rtr_rate);
    ///
    /// // Merge with Bloomberg priority
    /// let priority = SourcePriority::default_priority();
    /// bloomberg_set.merge(&reuters_set, &priority);
    ///
    /// // Bloomberg rate is kept (higher priority)
    /// let rate = bloomberg_set.get_rate(&rate_id, QuoteType::Mid).unwrap();
    /// assert_eq!(rate.source, DataSource::Bloomberg);
    /// ```
    pub fn merge(&mut self, other: &MarketRateSet, priority: &SourcePriority) {
        for rate in other.rates.values() {
            let key = (rate.id.clone(), rate.quote_type);

            // Check if we already have this rate
            if let Some(existing) = self.rates.get(&key) {
                // Only replace if the new rate has higher priority
                if priority.is_higher_priority(rate.source, existing.source) {
                    self.rates.insert(key, rate.clone());
                }
            } else {
                // New rate, just insert
                self.rates.insert(key, rate.clone());
            }
        }
    }

    /// Returns an iterator over all rates in the set.
    pub fn iter(&self) -> impl Iterator<Item = &MarketRate> { self.rates.values() }

    /// Converts market rates to instruments using the provided mapper.
    ///
    /// Maps all mid rates (or computes mid from bid/ask) to instruments.
    /// Rates that cannot be mapped (e.g., FX, Vol) are skipped with errors
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
    /// - `errors` contains mapping failures with their rate IDs
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType,
    ///     DataSource, Currency, StandardInstrumentMapper
    /// };
    /// use infra_master::time::{Date, Tenor};
    ///
    /// let mut rate_set = MarketRateSet::new();
    ///
    /// // Add deposit rate
    /// let dep_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let dep = MarketRate::new(dep_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(dep);
    ///
    /// // Add swap rate
    /// let swap_id = RateId::new(Currency::USD, Tenor::FiveYears, RateType::Swap);
    /// let swap = MarketRate::new(swap_id, QuoteType::Mid, 0.045, 1700000000000, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(swap);
    ///
    /// let mapper = StandardInstrumentMapper::new();
    /// let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
    ///
    /// let (instruments, errors) = rate_set.to_instruments(&mapper, valuation_date);
    /// assert_eq!(instruments.len(), 2);
    /// assert!(errors.is_empty());
    /// ```
    #[must_use]
    pub fn to_instruments<M: InstrumentMapper>(
        &self,
        mapper: &M,
        valuation_date: Date,
    ) -> (Vec<Instrument>, Vec<(RateId, MarketRateError)>) {
        let mut instruments = Vec::new();
        let mut errors = Vec::new();

        // Collect unique rate IDs (ignoring quote type)
        let mut processed_ids = std::collections::HashSet::new();

        for rate in self.rates.values() {
            // Skip if we've already processed this rate ID
            if processed_ids.contains(&rate.id) {
                continue;
            }
            processed_ids.insert(rate.id.clone());

            // Get the mid value (prefer direct mid, then compute from bid/ask)
            let mid_value = match self.get_mid_rate(&rate.id) {
                Some(v) => v,
                None => continue, // Skip if no mid available
            };

            // Create a synthetic mid rate for mapping
            let mid_rate = MarketRate {
                id: rate.id.clone(),
                quote_type: QuoteType::Mid,
                value: mid_value,
                timestamp: rate.timestamp,
                source: rate.source,
            };

            match mapper.map_to_instrument(&mid_rate, valuation_date) {
                Ok(instrument) => instruments.push(instrument),
                Err(e) => errors.push((rate.id.clone(), e)),
            }
        }

        (instruments, errors)
    }

    /// Converts market rates to instruments, returning only successful
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
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType,
    ///     DataSource, Currency, StandardInstrumentMapper
    /// };
    /// use infra_master::time::{Date, Tenor};
    ///
    /// let mut rate_set = MarketRateSet::new();
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let rate = MarketRate::new(rate_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(rate);
    ///
    /// let mapper = StandardInstrumentMapper::new();
    /// let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
    ///
    /// let instruments = rate_set.to_instruments_lossy(&mapper, valuation_date);
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

    /// Converts market rates to `MarketInstrument` using `MarketConvention`.
    ///
    /// This method creates `MarketInstrument` instances from the rates in
    /// this set, using the appropriate `MarketConvention` for each rate.
    /// The resulting instruments are sorted by maturity date.
    ///
    /// Rates that have no matching convention are skipped with a warning.
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
    /// - `skipped_ids` are rate IDs that had no matching convention
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::{Date, Tenor};
    ///
    /// let mut rate_set = MarketRateSet::new();
    ///
    /// let dep_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let dep = MarketRate::new(dep_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(dep);
    ///
    /// let swap_id = RateId::new(Currency::USD, Tenor::FiveYears, RateType::Swap);
    /// let swap = MarketRate::new(swap_id, QuoteType::Mid, 0.045, 1700000000000, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(swap);
    ///
    /// let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
    /// let (instruments, skipped) = rate_set.to_market_instruments(valuation_date, 1_000_000.0);
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
    ) -> (Vec<MarketInstrument>, Vec<RateId>) {
        let mut instruments = Vec::new();
        let mut skipped_ids = Vec::new();

        // Collect unique rate IDs (ignoring quote type)
        let mut processed_ids = std::collections::HashSet::new();

        for rate in self.rates.values() {
            // Skip if we've already processed this rate ID
            if processed_ids.contains(&rate.id) {
                continue;
            }
            processed_ids.insert(rate.id.clone());

            // Get the mid value (prefer direct mid, then compute from bid/ask)
            let mid_value = match self.get_mid_rate(&rate.id) {
                Some(v) => v,
                None => continue, // Skip if no mid available
            };

            // Try to get a convention for this rate
            let convention = match MarketConvention::for_rate_id(&rate.id) {
                Some(c) => c,
                None => {
                    skipped_ids.push(rate.id.clone());
                    continue;
                }
            };

            // Create the MarketInstrument
            match MarketInstrument::new(rate.id.clone(), mid_value, convention, valuation_date, notional) {
                Ok(instrument) => instruments.push(instrument),
                Err(_) => skipped_ids.push(rate.id.clone()),
            }
        }

        // Sort by maturity date
        instruments.sort_by_key(|i| i.maturity_date);

        (instruments, skipped_ids)
    }

    /// Converts market rates to `MarketInstrument`, returning only successful conversions.
    ///
    /// This is a convenience method that ignores conversion errors.
    /// Use [`to_market_instruments`](Self::to_market_instruments) if you need to handle
    /// skipped rates.
    ///
    /// # Arguments
    ///
    /// * `valuation_date` - The valuation date for instrument calculations
    /// * `notional` - Default notional amount for all instruments
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRateSet, MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::{Date, Tenor};
    ///
    /// let mut rate_set = MarketRateSet::new();
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let rate = MarketRate::new(rate_id, QuoteType::Mid, 0.05, 1700000000000, DataSource::Bloomberg).unwrap();
    /// rate_set.insert(rate);
    ///
    /// let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
    /// let instruments = rate_set.to_market_instruments_lossy(valuation_date, 1_000_000.0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        market::{DataSource, StandardInstrumentMapper},
        time::Tenor,
    };

    fn create_rate(
        currency: Currency,
        tenor: Tenor,
        rate_type: RateType,
        quote_type: QuoteType,
        value: f64,
        timestamp: i64,
        source: DataSource,
    ) -> MarketRate {
        let rate_id = RateId::new(currency, tenor, rate_type);
        MarketRate::new(rate_id, quote_type, value, timestamp, source).unwrap()
    }

    #[test]
    fn test_rate_set_new() {
        let rate_set = MarketRateSet::new();
        assert!(rate_set.is_empty());
        assert_eq!(rate_set.len(), 0);
    }

    #[test]
    fn test_rate_set_default() {
        let rate_set = MarketRateSet::default();
        assert!(rate_set.is_empty());
    }

    #[test]
    fn test_rate_set_insert() {
        let mut rate_set = MarketRateSet::new();

        let rate = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        );

        rate_set.insert(rate);
        assert_eq!(rate_set.len(), 1);
    }

    #[test]
    fn test_rate_set_insert_overwrites() {
        let mut rate_set = MarketRateSet::new();

        let rate1 = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        );

        let rate2 = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.06, // Different value
            1700000000001,
            DataSource::Bloomberg,
        );

        rate_set.insert(rate1);
        rate_set.insert(rate2);

        assert_eq!(rate_set.len(), 1);

        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let retrieved = rate_set.get_rate(&rate_id, QuoteType::Mid).unwrap();
        assert!((retrieved.value - 0.06).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rate_set_get_rate() {
        let mut rate_set = MarketRateSet::new();

        let rate = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        );

        rate_set.insert(rate);

        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);

        // Found
        assert!(rate_set.get_rate(&rate_id, QuoteType::Mid).is_some());

        // Not found (wrong quote type)
        assert!(rate_set.get_rate(&rate_id, QuoteType::Bid).is_none());

        // Not found (wrong rate id)
        let other_id = RateId::new(Currency::EUR, Tenor::ThreeMonths, RateType::Deposit);
        assert!(rate_set.get_rate(&other_id, QuoteType::Mid).is_none());
    }

    #[test]
    fn test_rate_set_get_mid_rate_direct() {
        let mut rate_set = MarketRateSet::new();

        let rate = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        );

        rate_set.insert(rate);

        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let mid = rate_set.get_mid_rate(&rate_id).unwrap();
        assert!((mid - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_rate_set_get_mid_rate_computed() {
        let mut rate_set = MarketRateSet::new();

        let bid = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Bid,
            0.049,
            1700000000000,
            DataSource::Bloomberg,
        );

        let ask = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Ask,
            0.051,
            1700000000000,
            DataSource::Bloomberg,
        );

        rate_set.insert(bid);
        rate_set.insert(ask);

        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let mid = rate_set.get_mid_rate(&rate_id).unwrap();

        // (0.049 + 0.051) / 2 = 0.05
        assert!((mid - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_rate_set_get_mid_rate_prefers_direct() {
        let mut rate_set = MarketRateSet::new();

        let bid = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Bid,
            0.049,
            1700000000000,
            DataSource::Bloomberg,
        );

        let ask = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Ask,
            0.051,
            1700000000000,
            DataSource::Bloomberg,
        );

        let mid = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.0505, // Slightly different from computed mid
            1700000000000,
            DataSource::Bloomberg,
        );

        rate_set.insert(bid);
        rate_set.insert(ask);
        rate_set.insert(mid);

        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let mid_value = rate_set.get_mid_rate(&rate_id).unwrap();

        // Direct mid is used, not computed
        assert!((mid_value - 0.0505).abs() < 1e-10);
    }

    #[test]
    fn test_rate_set_get_mid_rate_missing() {
        let mut rate_set = MarketRateSet::new();

        // Only bid, no ask
        let bid = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Bid,
            0.049,
            1700000000000,
            DataSource::Bloomberg,
        );

        rate_set.insert(bid);

        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        assert!(rate_set.get_mid_rate(&rate_id).is_none());
    }

    #[test]
    fn test_rate_set_remove() {
        let mut rate_set = MarketRateSet::new();

        let rate = create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        );

        rate_set.insert(rate);
        assert_eq!(rate_set.len(), 1);

        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let removed = rate_set.remove(&rate_id, QuoteType::Mid);

        assert!(removed.is_some());
        assert!(rate_set.is_empty());
    }

    #[test]
    fn test_rate_set_remove_nonexistent() {
        let mut rate_set = MarketRateSet::new();
        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);

        let removed = rate_set.remove(&rate_id, QuoteType::Mid);
        assert!(removed.is_none());
    }

    #[test]
    fn test_rate_set_rates_by_type() {
        let mut rate_set = MarketRateSet::new();

        // Add deposits
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::OneMonth,
            RateType::Deposit,
            QuoteType::Mid,
            0.04,
            1700000000000,
            DataSource::Bloomberg,
        ));
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        // Add swaps
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::FiveYears,
            RateType::Swap,
            QuoteType::Mid,
            0.045,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let deposits: Vec<_> = rate_set.rates_by_type(RateType::Deposit).collect();
        assert_eq!(deposits.len(), 2);

        let swaps: Vec<_> = rate_set.rates_by_type(RateType::Swap).collect();
        assert_eq!(swaps.len(), 1);

        let futures: Vec<_> = rate_set.rates_by_type(RateType::Futures).collect();
        assert_eq!(futures.len(), 0);
    }

    #[test]
    fn test_rate_set_filter_by_currency() {
        let mut rate_set = MarketRateSet::new();

        // USD
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        // EUR
        rate_set.insert(create_rate(
            Currency::EUR,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.04,
            1700000000000,
            DataSource::Bloomberg,
        ));
        rate_set.insert(create_rate(
            Currency::EUR,
            Tenor::SixMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.041,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let usd_only = rate_set.filter_by_currency(Currency::USD);
        assert_eq!(usd_only.len(), 1);

        let eur_only = rate_set.filter_by_currency(Currency::EUR);
        assert_eq!(eur_only.len(), 2);

        let gbp_only = rate_set.filter_by_currency(Currency::GBP);
        assert!(gbp_only.is_empty());
    }

    #[test]
    fn test_rate_set_as_of() {
        let mut rate_set = MarketRateSet::new();

        // Older rate
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1000,
            DataSource::Bloomberg,
        ));

        // Newer rate
        rate_set.insert(create_rate(
            Currency::EUR,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.04,
            2000,
            DataSource::Bloomberg,
        ));

        // As of 1500 - only includes first rate
        let as_of_1500 = rate_set.as_of(1500);
        assert_eq!(as_of_1500.len(), 1);

        // As of 2500 - includes both
        let as_of_2500 = rate_set.as_of(2500);
        assert_eq!(as_of_2500.len(), 2);

        // As of 500 - includes neither
        let as_of_500 = rate_set.as_of(500);
        assert_eq!(as_of_500.len(), 0);
    }

    #[test]
    fn test_rate_set_merge_new_rates() {
        let mut set1 = MarketRateSet::new();
        let mut set2 = MarketRateSet::new();

        set1.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        set2.insert(create_rate(
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

    #[test]
    fn test_rate_set_merge_priority_keeps_higher() {
        let mut bloomberg_set = MarketRateSet::new();
        let mut reuters_set = MarketRateSet::new();

        bloomberg_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.050,
            1700000000000,
            DataSource::Bloomberg,
        ));

        reuters_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.051,
            1700000000000,
            DataSource::Reuters,
        ));

        let priority = SourcePriority::default_priority();
        bloomberg_set.merge(&reuters_set, &priority);

        // Should still have Bloomberg rate (higher priority)
        assert_eq!(bloomberg_set.len(), 1);

        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let rate = bloomberg_set.get_rate(&rate_id, QuoteType::Mid).unwrap();
        assert_eq!(rate.source, DataSource::Bloomberg);
        assert!((rate.value - 0.050).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rate_set_merge_priority_replaces_lower() {
        let mut reuters_set = MarketRateSet::new();
        let mut bloomberg_set = MarketRateSet::new();

        reuters_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.051,
            1700000000000,
            DataSource::Reuters,
        ));

        bloomberg_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.050,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let priority = SourcePriority::default_priority();
        reuters_set.merge(&bloomberg_set, &priority);

        // Should have Bloomberg rate (higher priority replaces lower)
        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let rate = reuters_set.get_rate(&rate_id, QuoteType::Mid).unwrap();
        assert_eq!(rate.source, DataSource::Bloomberg);
    }

    #[test]
    fn test_rate_set_iter() {
        let mut rate_set = MarketRateSet::new();

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));
        rate_set.insert(create_rate(
            Currency::EUR,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.04,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let count = rate_set.iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_rate_set_clone() {
        let mut original = MarketRateSet::new();

        original.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let cloned = original.clone();
        assert_eq!(original.len(), cloned.len());

        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        assert!(cloned.get_rate(&rate_id, QuoteType::Mid).is_some());
    }

    #[test]
    fn test_rate_set_debug() {
        let rate_set = MarketRateSet::new();
        let debug_str = format!("{:?}", rate_set);
        assert!(debug_str.contains("MarketRateSet"));
    }

    // to_instruments tests

    #[test]
    fn test_to_instruments_basic() {
        let mut rate_set = MarketRateSet::new();

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::FiveYears,
            RateType::Swap,
            QuoteType::Mid,
            0.045,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let mapper = StandardInstrumentMapper::new();
        let vd = Date::from_ymd(2024, 1, 15).unwrap();

        let (instruments, errors) = rate_set.to_instruments(&mapper, vd);

        assert_eq!(instruments.len(), 2);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_to_instruments_from_bid_ask() {
        let mut rate_set = MarketRateSet::new();

        // Only bid and ask, no mid
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Bid,
            0.049,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Ask,
            0.051,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let mapper = StandardInstrumentMapper::new();
        let vd = Date::from_ymd(2024, 1, 15).unwrap();

        let (instruments, errors) = rate_set.to_instruments(&mapper, vd);

        assert_eq!(instruments.len(), 1);
        assert!(errors.is_empty());

        // Verify mid was computed correctly
        match &instruments[0] {
            Instrument::Deposit { rate, .. } => {
                assert!((*rate - 0.05).abs() < 1e-10);
            }
            _ => panic!("Expected Deposit instrument"),
        }
    }

    #[test]
    fn test_to_instruments_skips_unmappable() {
        let mut rate_set = MarketRateSet::new();

        // Mappable: Deposit
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        // Unmappable: FxSpot
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::TwoWeeks,
            RateType::FxSpot,
            QuoteType::Mid,
            1.1,
            1700000000000,
            DataSource::Bloomberg,
        ));

        // Unmappable: Vol
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::OneYear,
            RateType::Vol,
            QuoteType::Mid,
            0.2,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let mapper = StandardInstrumentMapper::new();
        let vd = Date::from_ymd(2024, 1, 15).unwrap();

        let (instruments, errors) = rate_set.to_instruments(&mapper, vd);

        assert_eq!(instruments.len(), 1);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_to_instruments_lossy() {
        let mut rate_set = MarketRateSet::new();

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::OneYear,
            RateType::Vol,
            QuoteType::Mid,
            0.2,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let mapper = StandardInstrumentMapper::new();
        let vd = Date::from_ymd(2024, 1, 15).unwrap();

        let instruments = rate_set.to_instruments_lossy(&mapper, vd);

        // Only the deposit is returned, vol error is ignored
        assert_eq!(instruments.len(), 1);
    }

    #[test]
    fn test_to_instruments_empty_set() {
        let rate_set = MarketRateSet::new();
        let mapper = StandardInstrumentMapper::new();
        let vd = Date::from_ymd(2024, 1, 15).unwrap();

        let (instruments, errors) = rate_set.to_instruments(&mapper, vd);

        assert!(instruments.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_to_instruments_multiple_quote_types_same_id() {
        let mut rate_set = MarketRateSet::new();

        // Same rate ID with different quote types
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Bid,
            0.049,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Ask,
            0.051,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.0505,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let mapper = StandardInstrumentMapper::new();
        let vd = Date::from_ymd(2024, 1, 15).unwrap();

        let (instruments, errors) = rate_set.to_instruments(&mapper, vd);

        // Should only create one instrument (deduped by rate ID)
        assert_eq!(instruments.len(), 1);
        assert!(errors.is_empty());

        // Should use direct mid value
        match &instruments[0] {
            Instrument::Deposit { rate, .. } => {
                assert!((*rate - 0.0505).abs() < 1e-10);
            }
            _ => panic!("Expected Deposit instrument"),
        }
    }

    #[test]
    fn test_to_instruments_all_rate_types() {
        let mut rate_set = MarketRateSet::new();

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::SixMonths,
            RateType::Fra,
            QuoteType::Mid,
            0.055,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Futures,
            QuoteType::Mid,
            0.045,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::FiveYears,
            RateType::Swap,
            QuoteType::Mid,
            0.04,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::OneYear,
            RateType::Ois,
            QuoteType::Mid,
            0.035,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::TenYears,
            RateType::BasisSwap,
            QuoteType::Mid,
            0.0025,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let mapper = StandardInstrumentMapper::new();
        let vd = Date::from_ymd(2024, 1, 15).unwrap();

        let (instruments, errors) = rate_set.to_instruments(&mapper, vd);

        assert_eq!(instruments.len(), 6);
        assert!(errors.is_empty());
    }

    // to_market_instruments tests

    #[test]
    fn test_to_market_instruments_basic() {
        let mut rate_set = MarketRateSet::new();

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::FiveYears,
            RateType::Swap,
            QuoteType::Mid,
            0.045,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let vd = Date::from_ymd(2024, 1, 15).unwrap();
        let (instruments, skipped) = rate_set.to_market_instruments(vd, 1_000_000.0);

        assert_eq!(instruments.len(), 2);
        assert!(skipped.is_empty());
    }

    #[test]
    fn test_to_market_instruments_sorted_by_maturity() {
        let mut rate_set = MarketRateSet::new();

        // Insert in reverse order (longer tenor first)
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::TenYears,
            RateType::Swap,
            QuoteType::Mid,
            0.04,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::FiveYears,
            RateType::Swap,
            QuoteType::Mid,
            0.045,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let vd = Date::from_ymd(2024, 1, 15).unwrap();
        let (instruments, _) = rate_set.to_market_instruments(vd, 1_000_000.0);

        // Check sorted by maturity
        assert_eq!(instruments.len(), 3);
        for i in 0..(instruments.len() - 1) {
            assert!(
                instruments[i].maturity_date <= instruments[i + 1].maturity_date,
                "Instruments should be sorted by maturity"
            );
        }
    }

    #[test]
    fn test_to_market_instruments_skips_unmappable() {
        let mut rate_set = MarketRateSet::new();

        // Mappable: Deposit
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        // Unmappable: Vol (no convention)
        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::OneYear,
            RateType::Vol,
            QuoteType::Mid,
            0.2,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let vd = Date::from_ymd(2024, 1, 15).unwrap();
        let (instruments, skipped) = rate_set.to_market_instruments(vd, 1_000_000.0);

        assert_eq!(instruments.len(), 1);
        assert_eq!(skipped.len(), 1);
        assert!(matches!(skipped[0].rate_type, RateType::Vol));
    }

    #[test]
    fn test_to_market_instruments_lossy() {
        let mut rate_set = MarketRateSet::new();

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        ));

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::OneYear,
            RateType::Vol,
            QuoteType::Mid,
            0.2,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let vd = Date::from_ymd(2024, 1, 15).unwrap();
        let instruments = rate_set.to_market_instruments_lossy(vd, 1_000_000.0);

        // Only the deposit is returned, vol is skipped silently
        assert_eq!(instruments.len(), 1);
    }

    #[test]
    fn test_to_market_instruments_empty_set() {
        let rate_set = MarketRateSet::new();
        let vd = Date::from_ymd(2024, 1, 15).unwrap();

        let (instruments, skipped) = rate_set.to_market_instruments(vd, 1_000_000.0);

        assert!(instruments.is_empty());
        assert!(skipped.is_empty());
    }

    #[test]
    fn test_to_market_instruments_preserves_rate_value() {
        let mut rate_set = MarketRateSet::new();

        rate_set.insert(create_rate(
            Currency::USD,
            Tenor::ThreeMonths,
            RateType::Deposit,
            QuoteType::Mid,
            0.0525,
            1700000000000,
            DataSource::Bloomberg,
        ));

        let vd = Date::from_ymd(2024, 1, 15).unwrap();
        let instruments = rate_set.to_market_instruments_lossy(vd, 2_000_000.0);

        assert_eq!(instruments.len(), 1);
        assert!((instruments[0].rate_value - 0.0525).abs() < 1e-10);
        assert!((instruments[0].notional - 2_000_000.0).abs() < 1e-10);
    }
}
