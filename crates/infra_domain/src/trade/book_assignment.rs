//! Trade-Book assignment types.
//!
//! This module provides types for tracking trade-to-book assignments
//! and maintaining assignment history for audit purposes.

use crate::{
    ids::{BookId, TradeId},
    time::Date,
};

// ============================================================================
// BookTransferReason
// ============================================================================

/// Reason for a trade's book assignment or transfer.
///
/// Tracks why a trade was assigned to a particular book, supporting
/// audit trail and regulatory reporting requirements.
///
/// # Examples
///
/// ```
/// use infra_domain::trade::BookTransferReason;
///
/// let reason = BookTransferReason::NewTrade;
/// assert_eq!(reason, BookTransferReason::default());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum BookTransferReason {
    /// Initial assignment when trade is created.
    #[default]
    NewTrade,
    /// Reallocation to different book for risk management purposes.
    Reallocation,
    /// Novation (transfer of trade to new counterparty).
    Novation,
    /// Internal transfer between desks/books.
    InternalTransfer,
}

impl BookTransferReason {
    /// Returns true if this is a new trade assignment.
    #[inline]
    #[must_use]
    pub fn is_new_trade(&self) -> bool { matches!(self, BookTransferReason::NewTrade) }

    /// Returns true if this is a reallocation.
    #[inline]
    #[must_use]
    pub fn is_reallocation(&self) -> bool { matches!(self, BookTransferReason::Reallocation) }

    /// Returns true if this is a novation.
    #[inline]
    #[must_use]
    pub fn is_novation(&self) -> bool { matches!(self, BookTransferReason::Novation) }

    /// Returns true if this is an internal transfer.
    #[inline]
    #[must_use]
    pub fn is_internal_transfer(&self) -> bool {
        matches!(self, BookTransferReason::InternalTransfer)
    }
}

// ============================================================================
// TradeBookAssignment
// ============================================================================

/// Record of a trade's book assignment.
///
/// Tracks the assignment of a trade to a book with effective date,
/// reason, and optional previous book for audit trail.
///
/// # Examples
///
/// ```
/// use infra_domain::trade::TradeBookAssignment;
/// use infra_domain::time::Date;
///
/// let assignment = TradeBookAssignment::new_trade(
///     "TRADE001",
///     "BOOK001",
///     Date::from_ymd(2025, 1, 15).unwrap(),
/// );
///
/// assert_eq!(assignment.trade_id().as_str(), "TRADE001");
/// assert_eq!(assignment.book_id().as_str(), "BOOK001");
/// assert!(assignment.is_initial_assignment());
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TradeBookAssignment {
    trade_id: TradeId,
    book_id: BookId,
    effective_date: Date,
    reason: BookTransferReason,
    previous_book_id: Option<BookId>,
}

impl TradeBookAssignment {
    /// Creates a new trade book assignment for a new trade.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - The trade being assigned
    /// * `book_id` - The book to assign the trade to
    /// * `effective_date` - When the assignment takes effect
    #[must_use]
    pub fn new_trade(
        trade_id: impl Into<TradeId>,
        book_id: impl Into<BookId>,
        effective_date: Date,
    ) -> Self {
        Self {
            trade_id: trade_id.into(),
            book_id: book_id.into(),
            effective_date,
            reason: BookTransferReason::NewTrade,
            previous_book_id: None,
        }
    }

    /// Creates a new assignment record for a book transfer.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - The trade being transferred
    /// * `from_book` - The previous book
    /// * `to_book` - The new book
    /// * `effective_date` - When the transfer takes effect
    /// * `reason` - Why the transfer is occurring
    #[must_use]
    pub fn transfer(
        trade_id: impl Into<TradeId>,
        from_book: impl Into<BookId>,
        to_book: impl Into<BookId>,
        effective_date: Date,
        reason: BookTransferReason,
    ) -> Self {
        Self {
            trade_id: trade_id.into(),
            book_id: to_book.into(),
            effective_date,
            reason,
            previous_book_id: Some(from_book.into()),
        }
    }

    /// Returns the trade ID.
    #[inline]
    #[must_use]
    pub fn trade_id(&self) -> &TradeId { &self.trade_id }

    /// Returns the book ID.
    #[inline]
    #[must_use]
    pub fn book_id(&self) -> &BookId { &self.book_id }

    /// Returns the effective date of the assignment.
    #[inline]
    #[must_use]
    pub fn effective_date(&self) -> Date { self.effective_date }

    /// Returns the reason for the assignment.
    #[inline]
    #[must_use]
    pub fn reason(&self) -> BookTransferReason { self.reason }

    /// Returns the previous book ID, if this is a transfer.
    #[inline]
    #[must_use]
    pub fn previous_book_id(&self) -> Option<&BookId> { self.previous_book_id.as_ref() }

    /// Returns true if this is an initial assignment (not a transfer).
    #[inline]
    #[must_use]
    pub fn is_initial_assignment(&self) -> bool { self.previous_book_id.is_none() }

    /// Returns true if this is a transfer from another book.
    #[inline]
    #[must_use]
    pub fn is_transfer(&self) -> bool { self.previous_book_id.is_some() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_transfer_reason() {
        assert_eq!(BookTransferReason::default(), BookTransferReason::NewTrade);
        assert!(BookTransferReason::NewTrade.is_new_trade());
        assert!(BookTransferReason::Reallocation.is_reallocation());
        assert!(BookTransferReason::Novation.is_novation());
        assert!(BookTransferReason::InternalTransfer.is_internal_transfer());
    }

    #[test]
    fn test_new_trade_assignment() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let assignment = TradeBookAssignment::new_trade("T001", "B001", date);

        assert_eq!(assignment.trade_id().as_str(), "T001");
        assert_eq!(assignment.book_id().as_str(), "B001");
        assert_eq!(assignment.effective_date(), date);
        assert_eq!(assignment.reason(), BookTransferReason::NewTrade);
        assert!(assignment.is_initial_assignment());
        assert!(!assignment.is_transfer());
    }

    #[test]
    fn test_transfer_assignment() {
        let date = Date::from_ymd(2025, 6, 1).unwrap();
        let assignment = TradeBookAssignment::transfer(
            "T001",
            "B001",
            "B002",
            date,
            BookTransferReason::Reallocation,
        );

        assert_eq!(assignment.book_id().as_str(), "B002");
        assert_eq!(assignment.previous_book_id().unwrap().as_str(), "B001");
        assert!(assignment.is_transfer());
    }
}
