//! Historical fixing rate management.

use std::collections::BTreeMap;

use crate::time::Date;

/// Historical fixing rates for an index.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Fixing {
    rates: BTreeMap<Date, f64>,
}

impl Default for Fixing {
    fn default() -> Self { Self::new() }
}

impl Fixing {
    /// Creates a new empty fixing store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rates: BTreeMap::new(),
        }
    }

    /// Creates a fixing store from an existing map.
    #[must_use]
    pub fn from_map(rates: BTreeMap<Date, f64>) -> Self { Self { rates } }

    /// Inserts a fixing rate for a date.
    pub fn insert(&mut self, date: Date, rate: f64) { self.rates.insert(date, rate); }

    /// Returns the fixing rate for a date, if available.
    #[must_use]
    pub fn get(&self, date: Date) -> Option<f64> { self.rates.get(&date).copied() }

    /// Returns true if a fixing rate exists for the given date.
    #[must_use]
    pub fn has_rate(&self, date: Date) -> bool { self.rates.contains_key(&date) }

    /// Returns a reference to the underlying rate map.
    #[must_use]
    pub fn rates(&self) -> &BTreeMap<Date, f64> { &self.rates }

    /// Returns the number of fixing rates stored.
    #[must_use]
    pub fn len(&self) -> usize { self.rates.len() }

    /// Returns true if no fixing rates are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.rates.is_empty() }
}

/// Layered fixing view providing priority-based lookup.
///
/// Looks up rates in order: additionals (overrides) -> base fixings.
/// Useful for Monte Carlo simulation where simulated rates overlay historical
/// data.
#[derive(Debug, Clone)]
pub struct FixingView<'a> {
    base: &'a Fixing,
    additionals: BTreeMap<Date, f64>,
}

impl<'a> FixingView<'a> {
    /// Creates a new fixing view over a base fixing store.
    #[must_use]
    pub fn new(base: &'a Fixing) -> Self {
        Self {
            base,
            additionals: BTreeMap::new(),
        }
    }

    /// Creates a fixing view with pre-populated additional rates.
    #[must_use]
    pub fn with_additionals(base: &'a Fixing, additionals: BTreeMap<Date, f64>) -> Self {
        Self { base, additionals }
    }

    /// Looks up a fixing rate with priority: additionals -> base.
    #[must_use]
    pub fn get(&self, date: Date) -> Option<f64> {
        self.additionals
            .get(&date)
            .copied()
            .or_else(|| self.base.get(date))
    }

    /// Inserts an additional (override) rate.
    pub fn insert_additional(&mut self, date: Date, rate: f64) {
        self.additionals.insert(date, rate);
    }

    /// Returns true if a rate exists in either layer.
    #[must_use]
    pub fn has_rate(&self, date: Date) -> bool {
        self.additionals.contains_key(&date) || self.base.has_rate(date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixing_insert_and_get() {
        let mut fixing = Fixing::new();
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        fixing.insert(date, 0.0425);
        assert_eq!(fixing.get(date), Some(0.0425));
        assert!(fixing.has_rate(date));
    }

    #[test]
    fn test_fixing_missing_rate() {
        let fixing = Fixing::new();
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        assert_eq!(fixing.get(date), None);
        assert!(!fixing.has_rate(date));
    }

    #[test]
    fn test_fixing_from_map() {
        let mut map = BTreeMap::new();
        let d1 = Date::from_ymd(2025, 1, 15).unwrap();
        let d2 = Date::from_ymd(2025, 1, 16).unwrap();
        map.insert(d1, 0.04);
        map.insert(d2, 0.041);
        let fixing = Fixing::from_map(map);
        assert_eq!(fixing.len(), 2);
        assert_eq!(fixing.get(d1), Some(0.04));
    }

    #[test]
    fn test_fixing_len_and_empty() {
        let fixing = Fixing::new();
        assert!(fixing.is_empty());
        assert_eq!(fixing.len(), 0);
    }

    #[test]
    fn test_fixing_view_priority() {
        let mut base = Fixing::new();
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        base.insert(date, 0.04);

        let mut additionals = BTreeMap::new();
        additionals.insert(date, 0.05);

        let view = FixingView::with_additionals(&base, additionals);
        // Additional should take priority
        assert_eq!(view.get(date), Some(0.05));
    }

    #[test]
    fn test_fixing_view_fallback_to_base() {
        let mut base = Fixing::new();
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        base.insert(date, 0.04);

        let view = FixingView::new(&base);
        assert_eq!(view.get(date), Some(0.04));
    }

    #[test]
    fn test_fixing_view_insert_additional() {
        let base = Fixing::new();
        let date = Date::from_ymd(2025, 1, 15).unwrap();

        let mut view = FixingView::new(&base);
        view.insert_additional(date, 0.035);
        assert_eq!(view.get(date), Some(0.035));
        assert!(view.has_rate(date));
    }

    #[test]
    fn test_fixing_view_missing() {
        let base = Fixing::new();
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let view = FixingView::new(&base);
        assert_eq!(view.get(date), None);
        assert!(!view.has_rate(date));
    }
}
