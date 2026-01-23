//! Identifier types for portfolio entities.
//!
//! This module re-exports ID types from `infra_master::ids` for convenience.
//! All ID types are defined centrally in the Infra layer to ensure type safety
//! across the entire codebase.
//!
//! # Migration Note
//!
//! Previously, this module defined its own ID types. As of the legacy
//! compatibility removal, all ID types are now defined in `infra_master::ids`
//! and re-exported here for backward compatibility.
//!
//! Prefer importing directly from `infra_master::ids` for new code.

// Re-export all ID types from infra_master for backward compatibility.
// Some types may not be used internally but are re-exported for public API.
// Also re-export CounterPartyId for code that uses the CamelCase variant
#[allow(unused_imports)]
pub use infra_master::counterparty::CounterPartyId;
#[allow(unused_imports)]
pub use infra_master::ids::{
    BookId, CcpId, CounterpartyId, IssuerId, LegalEntityId, NettingSetId, PortfolioId, TradeId,
};

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

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

    #[test]
    fn test_counterparty_id_alias() {
        // Verify CounterpartyId and CounterPartyId are compatible
        let id1 = CounterpartyId::new("CP001");
        let id2 = CounterPartyId::new("CP001");
        assert_eq!(id1, id2);
    }
}
