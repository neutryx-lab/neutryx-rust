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
//! use infra_domain::ids::{TradeId, PortfolioId, BookId};
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
/// Uses `derive_more` for Display, From<String>, and AsRef<str>
/// implementations. Provides `new()` and `as_str()` convenience methods.
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
    /// use infra_domain::ids::TradeId;
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
    /// use infra_domain::ids::PortfolioId;
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
    /// use infra_domain::ids::BookId;
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
    /// use infra_domain::ids::IssuerId;
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

