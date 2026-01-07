//! Identifier types for portfolio entities.
//!
//! This module provides strongly-typed identifiers for trades, counterparties,
//! and netting sets. Using newtypes ensures type safety and prevents accidental
//! misuse of identifiers.

use std::fmt;

/// Unique identifier for a trade.
///
/// # Examples
///
/// ```
/// use pricer_risk::portfolio::TradeId;
///
/// let id = TradeId::new("TRADE001");
/// assert_eq!(id.as_str(), "TRADE001");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeId(String);

impl TradeId {
    /// Creates a new trade ID.
    #[inline]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the ID as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TradeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TradeId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for TradeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Unique identifier for a counterparty.
///
/// # Examples
///
/// ```
/// use pricer_risk::portfolio::CounterpartyId;
///
/// let id = CounterpartyId::new("CP001");
/// assert_eq!(id.as_str(), "CP001");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CounterpartyId(String);

impl CounterpartyId {
    /// Creates a new counterparty ID.
    #[inline]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the ID as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CounterpartyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for CounterpartyId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for CounterpartyId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Unique identifier for a netting set.
///
/// # Examples
///
/// ```
/// use pricer_risk::portfolio::NettingSetId;
///
/// let id = NettingSetId::new("NS001");
/// assert_eq!(id.as_str(), "NS001");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NettingSetId(String);

impl NettingSetId {
    /// Creates a new netting set ID.
    #[inline]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the ID as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NettingSetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for NettingSetId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for NettingSetId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_trade_id_creation() {
        let id = TradeId::new("TRADE001");
        assert_eq!(id.as_str(), "TRADE001");
    }

    #[test]
    fn test_trade_id_from_str() {
        let id: TradeId = "TRADE002".into();
        assert_eq!(id.as_str(), "TRADE002");
    }

    #[test]
    fn test_trade_id_from_string() {
        let id: TradeId = String::from("TRADE003").into();
        assert_eq!(id.as_str(), "TRADE003");
    }

    #[test]
    fn test_trade_id_display() {
        let id = TradeId::new("TRADE001");
        assert_eq!(format!("{}", id), "TRADE001");
    }

    #[test]
    fn test_trade_id_equality() {
        let id1 = TradeId::new("TRADE001");
        let id2 = TradeId::new("TRADE001");
        let id3 = TradeId::new("TRADE002");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_trade_id_hash() {
        let mut set = HashSet::new();
        set.insert(TradeId::new("T1"));
        set.insert(TradeId::new("T2"));
        set.insert(TradeId::new("T1")); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_counterparty_id_creation() {
        let id = CounterpartyId::new("CP001");
        assert_eq!(id.as_str(), "CP001");
    }

    #[test]
    fn test_counterparty_id_display() {
        let id = CounterpartyId::new("CP001");
        assert_eq!(format!("{}", id), "CP001");
    }

    #[test]
    fn test_counterparty_id_hash() {
        let mut set = HashSet::new();
        set.insert(CounterpartyId::new("CP1"));
        set.insert(CounterpartyId::new("CP2"));
        set.insert(CounterpartyId::new("CP1")); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_netting_set_id_creation() {
        let id = NettingSetId::new("NS001");
        assert_eq!(id.as_str(), "NS001");
    }

    #[test]
    fn test_netting_set_id_display() {
        let id = NettingSetId::new("NS001");
        assert_eq!(format!("{}", id), "NS001");
    }

    #[test]
    fn test_netting_set_id_hash() {
        let mut set = HashSet::new();
        set.insert(NettingSetId::new("NS1"));
        set.insert(NettingSetId::new("NS2"));
        set.insert(NettingSetId::new("NS1")); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_clone() {
        let id1 = TradeId::new("T1");
        let id2 = id1.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_debug() {
        let id = TradeId::new("T1");
        let debug = format!("{:?}", id);
        assert!(debug.contains("T1"));
    }
}
