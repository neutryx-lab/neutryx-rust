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

// ============================================================================
// TradeBookHistory
// ============================================================================

/// History of book assignments for a trade.
///
/// Maintains a chronological record of all book assignments for a single trade,
/// enabling audit trail and historical analysis.
///
/// # Examples
///
/// ```
/// use infra_domain::trade::{TradeBookHistory, TradeBookAssignment, BookTransferReason};
/// use infra_domain::time::Date;
///
/// let mut history = TradeBookHistory::new("TRADE001");
/// history.add_assignment(TradeBookAssignment::new_trade(
///     "TRADE001",
///     "BOOK001",
///     Date::from_ymd(2025, 1, 15).unwrap(),
/// ));
///
/// assert_eq!(history.current_book().unwrap().as_str(), "BOOK001");
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TradeBookHistory {
    trade_id: TradeId,
    assignments: Vec<TradeBookAssignment>,
}

impl TradeBookHistory {
    /// Creates a new empty history for a trade.
    #[must_use]
    pub fn new(trade_id: impl Into<TradeId>) -> Self {
        Self {
            trade_id: trade_id.into(),
            assignments: Vec::new(),
        }
    }

    /// Creates a new history with an initial assignment.
    #[must_use]
    pub fn with_initial_assignment(
        trade_id: impl Into<TradeId>,
        book_id: impl Into<BookId>,
        effective_date: Date,
    ) -> Self {
        let trade_id = trade_id.into();
        let assignment = TradeBookAssignment::new_trade(trade_id.clone(), book_id, effective_date);
        Self {
            trade_id,
            assignments: vec![assignment],
        }
    }

    /// Returns the trade ID.
    #[inline]
    #[must_use]
    pub fn trade_id(&self) -> &TradeId { &self.trade_id }

    /// Adds an assignment to the history.
    pub fn add_assignment(&mut self, assignment: TradeBookAssignment) {
        self.assignments.push(assignment);
    }

    /// Returns the current book ID (most recent assignment).
    #[must_use]
    pub fn current_book(&self) -> Option<&BookId> { self.assignments.last().map(|a| a.book_id()) }

    /// Returns an iterator over all assignments.
    pub fn assignments(&self) -> impl Iterator<Item = &TradeBookAssignment> {
        self.assignments.iter()
    }

    /// Returns the number of assignments in the history.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize { self.assignments.len() }

    /// Returns true if there are no assignments.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool { self.assignments.is_empty() }

    /// Returns the assignment effective at a given date.
    #[must_use]
    pub fn assignment_at(&self, date: Date) -> Option<&TradeBookAssignment> {
        self.assignments
            .iter()
            .rfind(|a| a.effective_date() <= date)
    }

    /// Returns the book ID effective at a given date.
    #[must_use]
    pub fn book_at(&self, date: Date) -> Option<&BookId> {
        self.assignment_at(date).map(|a| a.book_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // BookTransferReason tests
    // ========================================================================

    #[test]
    fn test_book_transfer_reason_default() {
        let reason = BookTransferReason::default();
        assert_eq!(reason, BookTransferReason::NewTrade);
    }

    #[test]
    fn test_book_transfer_reason_is_new_trade() {
        assert!(BookTransferReason::NewTrade.is_new_trade());
        assert!(!BookTransferReason::Reallocation.is_new_trade());
    }

    #[test]
    fn test_book_transfer_reason_is_reallocation() {
        assert!(BookTransferReason::Reallocation.is_reallocation());
        assert!(!BookTransferReason::NewTrade.is_reallocation());
    }

    #[test]
    fn test_book_transfer_reason_is_novation() {
        assert!(BookTransferReason::Novation.is_novation());
        assert!(!BookTransferReason::NewTrade.is_novation());
    }

    #[test]
    fn test_book_transfer_reason_is_internal_transfer() {
        assert!(BookTransferReason::InternalTransfer.is_internal_transfer());
        assert!(!BookTransferReason::NewTrade.is_internal_transfer());
    }

    #[test]
    fn test_book_transfer_reason_clone_and_equality() {
        let r1 = BookTransferReason::Reallocation;
        let r2 = r1;
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_book_transfer_reason_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BookTransferReason::NewTrade);
        set.insert(BookTransferReason::Reallocation);
        set.insert(BookTransferReason::NewTrade); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // ========================================================================
    // TradeBookAssignment tests
    // ========================================================================

    #[test]
    fn test_trade_book_assignment_new_trade() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let assignment = TradeBookAssignment::new_trade("T001", "B001", date);

        assert_eq!(assignment.trade_id().as_str(), "T001");
        assert_eq!(assignment.book_id().as_str(), "B001");
        assert_eq!(assignment.effective_date(), date);
        assert_eq!(assignment.reason(), BookTransferReason::NewTrade);
        assert!(assignment.previous_book_id().is_none());
        assert!(assignment.is_initial_assignment());
        assert!(!assignment.is_transfer());
    }

    #[test]
    fn test_trade_book_assignment_transfer() {
        let date = Date::from_ymd(2025, 6, 1).unwrap();
        let assignment = TradeBookAssignment::transfer(
            "T001",
            "B001",
            "B002",
            date,
            BookTransferReason::Reallocation,
        );

        assert_eq!(assignment.trade_id().as_str(), "T001");
        assert_eq!(assignment.book_id().as_str(), "B002");
        assert_eq!(assignment.previous_book_id().unwrap().as_str(), "B001");
        assert_eq!(assignment.reason(), BookTransferReason::Reallocation);
        assert!(!assignment.is_initial_assignment());
        assert!(assignment.is_transfer());
    }

    #[test]
    fn test_trade_book_assignment_clone() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let assignment = TradeBookAssignment::new_trade("T001", "B001", date);
        let cloned = assignment.clone();
        assert_eq!(cloned.trade_id().as_str(), "T001");
    }

    // ========================================================================
    // TradeBookHistory tests
    // ========================================================================

    #[test]
    fn test_trade_book_history_new() {
        let history = TradeBookHistory::new("T001");
        assert_eq!(history.trade_id().as_str(), "T001");
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
        assert!(history.current_book().is_none());
    }

    #[test]
    fn test_trade_book_history_with_initial_assignment() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let history = TradeBookHistory::with_initial_assignment("T001", "B001", date);

        assert_eq!(history.trade_id().as_str(), "T001");
        assert!(!history.is_empty());
        assert_eq!(history.len(), 1);
        assert_eq!(history.current_book().unwrap().as_str(), "B001");
    }

    #[test]
    fn test_trade_book_history_add_assignment() {
        let mut history = TradeBookHistory::new("T001");
        let date1 = Date::from_ymd(2025, 1, 15).unwrap();
        let date2 = Date::from_ymd(2025, 6, 1).unwrap();

        history.add_assignment(TradeBookAssignment::new_trade("T001", "B001", date1));
        assert_eq!(history.current_book().unwrap().as_str(), "B001");

        history.add_assignment(TradeBookAssignment::transfer(
            "T001",
            "B001",
            "B002",
            date2,
            BookTransferReason::Reallocation,
        ));
        assert_eq!(history.current_book().unwrap().as_str(), "B002");
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_trade_book_history_assignments_iterator() {
        let date1 = Date::from_ymd(2025, 1, 15).unwrap();
        let date2 = Date::from_ymd(2025, 6, 1).unwrap();
        let mut history = TradeBookHistory::new("T001");
        history.add_assignment(TradeBookAssignment::new_trade("T001", "B001", date1));
        history.add_assignment(TradeBookAssignment::transfer(
            "T001",
            "B001",
            "B002",
            date2,
            BookTransferReason::Reallocation,
        ));

        let assignments: Vec<_> = history.assignments().collect();
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0].book_id().as_str(), "B001");
        assert_eq!(assignments[1].book_id().as_str(), "B002");
    }

    #[test]
    fn test_trade_book_history_assignment_at() {
        let date1 = Date::from_ymd(2025, 1, 15).unwrap();
        let date2 = Date::from_ymd(2025, 6, 1).unwrap();
        let mut history = TradeBookHistory::new("T001");
        history.add_assignment(TradeBookAssignment::new_trade("T001", "B001", date1));
        history.add_assignment(TradeBookAssignment::transfer(
            "T001",
            "B001",
            "B002",
            date2,
            BookTransferReason::Reallocation,
        ));

        // Before first assignment
        let early_date = Date::from_ymd(2025, 1, 1).unwrap();
        assert!(history.assignment_at(early_date).is_none());

        // After first, before second
        let mid_date = Date::from_ymd(2025, 3, 1).unwrap();
        assert_eq!(
            history.assignment_at(mid_date).unwrap().book_id().as_str(),
            "B001"
        );

        // After second
        let late_date = Date::from_ymd(2025, 7, 1).unwrap();
        assert_eq!(
            history.assignment_at(late_date).unwrap().book_id().as_str(),
            "B002"
        );
    }

    #[test]
    fn test_trade_book_history_book_at() {
        let date1 = Date::from_ymd(2025, 1, 15).unwrap();
        let date2 = Date::from_ymd(2025, 6, 1).unwrap();
        let mut history = TradeBookHistory::new("T001");
        history.add_assignment(TradeBookAssignment::new_trade("T001", "B001", date1));
        history.add_assignment(TradeBookAssignment::transfer(
            "T001",
            "B001",
            "B002",
            date2,
            BookTransferReason::Reallocation,
        ));

        let mid_date = Date::from_ymd(2025, 3, 1).unwrap();
        assert_eq!(history.book_at(mid_date).unwrap().as_str(), "B001");

        let late_date = Date::from_ymd(2025, 7, 1).unwrap();
        assert_eq!(history.book_at(late_date).unwrap().as_str(), "B002");
    }

    #[test]
    fn test_trade_book_history_clone() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let history = TradeBookHistory::with_initial_assignment("T001", "B001", date);
        let cloned = history.clone();
        assert_eq!(cloned.trade_id().as_str(), "T001");
        assert_eq!(cloned.current_book().unwrap().as_str(), "B001");
    }
}
