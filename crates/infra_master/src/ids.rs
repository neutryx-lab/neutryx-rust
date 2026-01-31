//! Type-safe ID types for financial entities.
//!
//! This module provides newtype wrappers for various identifiers used
//! across the Neutryx codebase, ensuring type safety at compile time.
//!
//! # Design Rationale
//!
//! Using newtypes for identifiers prevents common bugs:
//! - Cannot accidentally pass a `TradeId` where a `CounterpartyId` is expected
//! - Compiler enforces correct ID usage
//! - Clear semantics in function signatures
//!
//! # Examples
//!
//! ```
//! use infra_master::ids::{TradeId, PortfolioId, BookId};
//!
//! let trade = TradeId::new("TRADE001");
//! let portfolio = PortfolioId::new("PORTFOLIO001");
//! let book = BookId::new("BOOK001");
//!
//! // This would fail to compile:
//! // fn process(id: TradeId) { ... }
//! // process(portfolio); // Error: expected TradeId, found PortfolioId
//! ```

#![allow(clippy::must_use_candidate)]

use derive_more::{AsRef, Display, From};

// Re-export counterparty IDs for unified access
pub use crate::counterparty::{CcpId, CounterPartyId, LegalEntityId, NettingSetId};

// ============================================================================
// Macro for common ID implementation
// ============================================================================

/// Macro to define a type-safe ID with standard implementations.
///
/// Uses `derive_more` for Display, From<String>, and AsRef<str> implementations.
/// Provides `new()` and `as_str()` convenience methods.
macro_rules! define_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Display, From, AsRef)]
        #[as_ref(str)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(String);

        impl $name {
            /// Creates a new ID.
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

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

// ============================================================================
// TradeId
// ============================================================================

define_id! {
    /// Type-safe trade identifier.
    ///
    /// Wraps a string identifier for trades, providing type safety
    /// to prevent accidental mixing with other ID types.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::ids::TradeId;
    ///
    /// let id = TradeId::new("TRADE001");
    /// assert_eq!(id.as_str(), "TRADE001");
    /// ```
    TradeId
}

// ============================================================================
// PortfolioId
// ============================================================================

define_id! {
    /// Type-safe portfolio identifier.
    ///
    /// Wraps a string identifier for portfolios, providing type safety
    /// to prevent accidental mixing with other ID types.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::ids::PortfolioId;
    ///
    /// let id = PortfolioId::new("PORTFOLIO001");
    /// assert_eq!(id.as_str(), "PORTFOLIO001");
    /// ```
    PortfolioId
}

// ============================================================================
// BookId
// ============================================================================

define_id! {
    /// Type-safe trading book identifier.
    ///
    /// Wraps a string identifier for trading books, providing type safety
    /// to prevent accidental mixing with other ID types.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::ids::BookId;
    ///
    /// let id = BookId::new("BOOK001");
    /// assert_eq!(id.as_str(), "BOOK001");
    /// ```
    BookId
}

// ============================================================================
// IssuerId
// ============================================================================

define_id! {
    /// Type-safe issuer identifier.
    ///
    /// Wraps a string identifier for bond/security issuers, providing type safety
    /// to prevent accidental mixing with other ID types.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::ids::IssuerId;
    ///
    /// let id = IssuerId::new("ISSUER001");
    /// assert_eq!(id.as_str(), "ISSUER001");
    /// ```
    IssuerId
}

// ============================================================================
// Type aliases for naming consistency
// ============================================================================

/// Alias for `CounterPartyId` with lowercase 'p' (American English style).
///
/// Both `CounterpartyId` and `CounterPartyId` are supported for flexibility.
pub type CounterpartyId = CounterPartyId;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    // ========================================================================
    // TradeId tests
    // ========================================================================

    #[test]
    fn test_trade_id_new() {
        let id = TradeId::new("TRADE001");
        assert_eq!(id.as_str(), "TRADE001");
    }

    #[test]
    fn test_trade_id_from_string() {
        let id: TradeId = "TRADE002".to_string().into();
        assert_eq!(id.as_str(), "TRADE002");
    }

    #[test]
    fn test_trade_id_from_str() {
        let id: TradeId = "TRADE003".into();
        assert_eq!(id.as_str(), "TRADE003");
    }

    #[test]
    fn test_trade_id_display() {
        let id = TradeId::new("TRADE001");
        assert_eq!(format!("{}", id), "TRADE001");
    }

    #[test]
    fn test_trade_id_as_ref() {
        let id = TradeId::new("TRADE001");
        let s: &str = id.as_ref();
        assert_eq!(s, "TRADE001");
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

    // ========================================================================
    // PortfolioId tests
    // ========================================================================

    #[test]
    fn test_portfolio_id_new() {
        let id = PortfolioId::new("PORTFOLIO001");
        assert_eq!(id.as_str(), "PORTFOLIO001");
    }

    #[test]
    fn test_portfolio_id_from_string() {
        let id: PortfolioId = "PORTFOLIO002".to_string().into();
        assert_eq!(id.as_str(), "PORTFOLIO002");
    }

    #[test]
    fn test_portfolio_id_display() {
        let id = PortfolioId::new("PORTFOLIO001");
        assert_eq!(format!("{}", id), "PORTFOLIO001");
    }

    #[test]
    fn test_portfolio_id_equality() {
        let id1 = PortfolioId::new("P001");
        let id2 = PortfolioId::new("P001");
        let id3 = PortfolioId::new("P002");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    // ========================================================================
    // BookId tests
    // ========================================================================

    #[test]
    fn test_book_id_new() {
        let id = BookId::new("BOOK001");
        assert_eq!(id.as_str(), "BOOK001");
    }

    #[test]
    fn test_book_id_from_string() {
        let id: BookId = "BOOK002".to_string().into();
        assert_eq!(id.as_str(), "BOOK002");
    }

    #[test]
    fn test_book_id_display() {
        let id = BookId::new("BOOK001");
        assert_eq!(format!("{}", id), "BOOK001");
    }

    #[test]
    fn test_book_id_equality() {
        let id1 = BookId::new("B001");
        let id2 = BookId::new("B001");
        let id3 = BookId::new("B002");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    // ========================================================================
    // IssuerId tests
    // ========================================================================

    #[test]
    fn test_issuer_id_new() {
        let id = IssuerId::new("ISSUER001");
        assert_eq!(id.as_str(), "ISSUER001");
    }

    #[test]
    fn test_issuer_id_from_string() {
        let id: IssuerId = "ISSUER002".to_string().into();
        assert_eq!(id.as_str(), "ISSUER002");
    }

    #[test]
    fn test_issuer_id_display() {
        let id = IssuerId::new("ISSUER001");
        assert_eq!(format!("{}", id), "ISSUER001");
    }

    #[test]
    fn test_issuer_id_equality() {
        let id1 = IssuerId::new("I001");
        let id2 = IssuerId::new("I001");
        let id3 = IssuerId::new("I002");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    // ========================================================================
    // Type safety tests
    // ========================================================================

    #[test]
    fn test_type_safety_different_id_types() {
        // This test verifies that different ID types are distinct at compile time
        let trade_id = TradeId::new("ID001");
        let portfolio_id = PortfolioId::new("ID001");
        let book_id = BookId::new("ID001");
        let issuer_id = IssuerId::new("ID001");

        // Same string content, but different types
        assert_eq!(trade_id.as_str(), portfolio_id.as_str());
        assert_eq!(portfolio_id.as_str(), book_id.as_str());
        assert_eq!(book_id.as_str(), issuer_id.as_str());

        // But they cannot be compared directly (different types)
        // This is the key benefit of newtypes
    }

    #[test]
    fn test_counterparty_id_alias() {
        // Verify that CounterpartyId is an alias for CounterPartyId
        let id1 = CounterpartyId::new("CP001");
        let id2 = CounterPartyId::new("CP001");
        assert_eq!(id1, id2);
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
