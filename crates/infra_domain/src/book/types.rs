//! Book-related types.
//!
//! This module provides type definitions for trading books including
//! book types, regulatory classifications, and ownership information.

use chrono::{DateTime, Utc};

use crate::counterparty::LegalEntityId;

// ============================================================================
// BookType
// ============================================================================

/// Type of trading book.
///
/// Classifies the purpose and usage of a trading book within the organisation.
///
/// # Examples
///
/// ```
/// use infra_domain::book::BookType;
///
/// let book_type = BookType::Trading;
/// assert_eq!(book_type, BookType::default());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum BookType {
    /// Trading book for market-making and proprietary trading.
    #[default]
    Trading,
    /// Banking book for held-to-maturity positions.
    Banking,
    /// Hedge book for risk management positions.
    Hedge,
    /// Internal book for inter-desk transfers.
    Internal,
}

impl BookType {
    /// Returns true if this is a trading book.
    #[inline]
    #[must_use]
    pub fn is_trading(&self) -> bool { matches!(self, BookType::Trading) }

    /// Returns true if this is a banking book.
    #[inline]
    #[must_use]
    pub fn is_banking(&self) -> bool { matches!(self, BookType::Banking) }

    /// Returns true if this is a hedge book.
    #[inline]
    #[must_use]
    pub fn is_hedge(&self) -> bool { matches!(self, BookType::Hedge) }

    /// Returns true if this is an internal book.
    #[inline]
    #[must_use]
    pub fn is_internal(&self) -> bool { matches!(self, BookType::Internal) }
}

// ============================================================================
// RegulatoryBookType
// ============================================================================

/// Regulatory classification of a trading book.
///
/// Used for regulatory reporting purposes (Basel III/IV, FRTB).
///
/// # Examples
///
/// ```
/// use infra_domain::book::RegulatoryBookType;
///
/// let reg_type = RegulatoryBookType::TB;
/// assert!(reg_type.is_trading_book());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RegulatoryBookType {
    /// Trading Book - subject to market risk capital requirements.
    TB,
    /// Non-Trading Book Regulatory - positions that are neither TB nor BB.
    NTBR,
    /// Banking Book - subject to credit risk capital requirements.
    BB,
}

impl RegulatoryBookType {
    /// Returns true if this is a Trading Book.
    #[inline]
    #[must_use]
    pub fn is_trading_book(&self) -> bool { matches!(self, RegulatoryBookType::TB) }

    /// Returns true if this is a Banking Book.
    #[inline]
    #[must_use]
    pub fn is_banking_book(&self) -> bool { matches!(self, RegulatoryBookType::BB) }

    /// Returns true if this is a Non-Trading Book Regulatory.
    #[inline]
    #[must_use]
    pub fn is_ntbr(&self) -> bool { matches!(self, RegulatoryBookType::NTBR) }
}

// ============================================================================
// BookOwnership
// ============================================================================

/// Ownership information for a trading book.
///
/// Captures the organisational hierarchy of book ownership including
/// desk, division, and legal entity.
///
/// # Examples
///
/// ```
/// use infra_domain::book::BookOwnership;
///
/// let ownership = BookOwnership::new()
///     .with_desk("FX Spot")
///     .with_division("Markets");
///
/// assert_eq!(ownership.desk(), Some("FX Spot"));
/// ```
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BookOwnership {
    desk: Option<String>,
    division: Option<String>,
    legal_entity_id: Option<LegalEntityId>,
}

impl BookOwnership {
    /// Creates new empty ownership information.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Sets the desk name.
    #[must_use]
    pub fn with_desk(mut self, desk: impl Into<String>) -> Self {
        self.desk = Some(desk.into());
        self
    }

    /// Sets the division name.
    #[must_use]
    pub fn with_division(mut self, division: impl Into<String>) -> Self {
        self.division = Some(division.into());
        self
    }

    /// Sets the legal entity ID.
    #[must_use]
    pub fn with_legal_entity(mut self, legal_entity_id: LegalEntityId) -> Self {
        self.legal_entity_id = Some(legal_entity_id);
        self
    }

    /// Returns the desk name.
    #[inline]
    #[must_use]
    pub fn desk(&self) -> Option<&str> { self.desk.as_deref() }

    /// Returns the division name.
    #[inline]
    #[must_use]
    pub fn division(&self) -> Option<&str> { self.division.as_deref() }

    /// Returns the legal entity ID.
    #[inline]
    #[must_use]
    pub fn legal_entity_id(&self) -> Option<&LegalEntityId> { self.legal_entity_id.as_ref() }
}

// ============================================================================
// BookMetadata
// ============================================================================

/// Metadata for a trading book.
///
/// Captures audit information including creation and modification timestamps
/// and user identifiers.
///
/// # Examples
///
/// ```
/// use infra_domain::book::BookMetadata;
///
/// let metadata = BookMetadata::new();
/// assert!(metadata.created_by().is_none());
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BookMetadata {
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    created_by: Option<String>,
    updated_by: Option<String>,
}

impl BookMetadata {
    /// Creates new metadata with current timestamp.
    #[must_use]
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            created_by: None,
            updated_by: None,
        }
    }

    /// Creates metadata with specified creator.
    #[must_use]
    pub fn with_creator(mut self, creator: impl Into<String>) -> Self {
        let creator = creator.into();
        self.created_by = Some(creator.clone());
        self.updated_by = Some(creator);
        self
    }

    /// Updates the modification timestamp and user.
    #[must_use]
    pub fn with_updater(mut self, updater: impl Into<String>) -> Self {
        self.updated_at = Utc::now();
        self.updated_by = Some(updater.into());
        self
    }

    /// Returns the creation timestamp.
    #[inline]
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }

    /// Returns the last update timestamp.
    #[inline]
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> { self.updated_at }

    /// Returns the creator user ID.
    #[inline]
    #[must_use]
    pub fn created_by(&self) -> Option<&str> { self.created_by.as_deref() }

    /// Returns the last updater user ID.
    #[inline]
    #[must_use]
    pub fn updated_by(&self) -> Option<&str> { self.updated_by.as_deref() }
}

impl Default for BookMetadata {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // BookType tests
    // ========================================================================

    #[test]
    fn test_book_type_default() {
        let book_type = BookType::default();
        assert_eq!(book_type, BookType::Trading);
    }

    #[test]
    fn test_book_type_is_trading() {
        assert!(BookType::Trading.is_trading());
        assert!(!BookType::Banking.is_trading());
    }

    #[test]
    fn test_book_type_is_banking() {
        assert!(BookType::Banking.is_banking());
        assert!(!BookType::Trading.is_banking());
    }

    #[test]
    fn test_book_type_is_hedge() {
        assert!(BookType::Hedge.is_hedge());
        assert!(!BookType::Trading.is_hedge());
    }

    #[test]
    fn test_book_type_is_internal() {
        assert!(BookType::Internal.is_internal());
        assert!(!BookType::Trading.is_internal());
    }

    #[test]
    fn test_book_type_clone_and_equality() {
        let t1 = BookType::Trading;
        let t2 = t1;
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_book_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BookType::Trading);
        set.insert(BookType::Banking);
        set.insert(BookType::Trading); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // ========================================================================
    // RegulatoryBookType tests
    // ========================================================================

    #[test]
    fn test_regulatory_book_type_is_trading_book() {
        assert!(RegulatoryBookType::TB.is_trading_book());
        assert!(!RegulatoryBookType::BB.is_trading_book());
    }

    #[test]
    fn test_regulatory_book_type_is_banking_book() {
        assert!(RegulatoryBookType::BB.is_banking_book());
        assert!(!RegulatoryBookType::TB.is_banking_book());
    }

    #[test]
    fn test_regulatory_book_type_is_ntbr() {
        assert!(RegulatoryBookType::NTBR.is_ntbr());
        assert!(!RegulatoryBookType::TB.is_ntbr());
    }

    #[test]
    fn test_regulatory_book_type_clone_and_equality() {
        let t1 = RegulatoryBookType::TB;
        let t2 = t1;
        assert_eq!(t1, t2);
    }

    // ========================================================================
    // BookOwnership tests
    // ========================================================================

    #[test]
    fn test_book_ownership_new() {
        let ownership = BookOwnership::new();
        assert!(ownership.desk().is_none());
        assert!(ownership.division().is_none());
        assert!(ownership.legal_entity_id().is_none());
    }

    #[test]
    fn test_book_ownership_with_desk() {
        let ownership = BookOwnership::new().with_desk("FX Spot");
        assert_eq!(ownership.desk(), Some("FX Spot"));
    }

    #[test]
    fn test_book_ownership_with_division() {
        let ownership = BookOwnership::new().with_division("Markets");
        assert_eq!(ownership.division(), Some("Markets"));
    }

    #[test]
    fn test_book_ownership_with_legal_entity() {
        let lei = LegalEntityId::new_unchecked("529900T8BM49AURSDO55");
        let ownership = BookOwnership::new().with_legal_entity(lei);
        assert!(ownership.legal_entity_id().is_some());
    }

    #[test]
    fn test_book_ownership_builder_chain() {
        let lei = LegalEntityId::new_unchecked("529900T8BM49AURSDO55");
        let ownership = BookOwnership::new()
            .with_desk("FX Spot")
            .with_division("Markets")
            .with_legal_entity(lei);
        assert_eq!(ownership.desk(), Some("FX Spot"));
        assert_eq!(ownership.division(), Some("Markets"));
        assert!(ownership.legal_entity_id().is_some());
    }

    #[test]
    fn test_book_ownership_clone() {
        let ownership = BookOwnership::new().with_desk("FX Spot");
        let cloned = ownership.clone();
        assert_eq!(cloned.desk(), Some("FX Spot"));
    }

    // ========================================================================
    // BookMetadata tests
    // ========================================================================

    #[test]
    fn test_book_metadata_new() {
        let metadata = BookMetadata::new();
        assert!(metadata.created_by().is_none());
        assert!(metadata.updated_by().is_none());
    }

    #[test]
    fn test_book_metadata_with_creator() {
        let metadata = BookMetadata::new().with_creator("user1");
        assert_eq!(metadata.created_by(), Some("user1"));
        assert_eq!(metadata.updated_by(), Some("user1"));
    }

    #[test]
    fn test_book_metadata_with_updater() {
        let metadata = BookMetadata::new()
            .with_creator("user1")
            .with_updater("user2");
        assert_eq!(metadata.created_by(), Some("user1"));
        assert_eq!(metadata.updated_by(), Some("user2"));
    }

    #[test]
    fn test_book_metadata_timestamps() {
        let metadata = BookMetadata::new();
        assert!(metadata.created_at() <= metadata.updated_at());
    }

    #[test]
    fn test_book_metadata_default() {
        let metadata = BookMetadata::default();
        assert!(metadata.created_by().is_none());
    }

    #[test]
    fn test_book_metadata_clone() {
        let metadata = BookMetadata::new().with_creator("user1");
        let cloned = metadata.clone();
        assert_eq!(cloned.created_by(), Some("user1"));
    }
}
