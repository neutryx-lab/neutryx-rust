//! Book entity and builder.
//!
//! This module provides the Book struct representing a trading book
//! and its builder for fluent construction.

use crate::ids::BookId;

use super::{BookMetadata, BookOwnership, BookType, RegulatoryBookType};

// ============================================================================
// Book
// ============================================================================

/// A trading book.
///
/// Represents a logical grouping of trades for risk management, P&L attribution,
/// and regulatory reporting purposes.
///
/// # Examples
///
/// ```
/// use infra_master::book::{Book, BookType};
///
/// let book = Book::builder("BOOK001", "Main Trading Book")
///     .book_type(BookType::Trading)
///     .description("Primary book for FX spot trading")
///     .build();
///
/// assert_eq!(book.book_id().as_str(), "BOOK001");
/// assert_eq!(book.name(), "Main Trading Book");
/// assert_eq!(book.book_type(), BookType::Trading);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Book {
    book_id: BookId,
    name: String,
    description: Option<String>,
    book_type: BookType,
    regulatory_type: Option<RegulatoryBookType>,
    ownership: Option<BookOwnership>,
    metadata: BookMetadata,
}

impl Book {
    /// Creates a new book builder.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the book
    /// * `name` - Human-readable name for the book
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::book::Book;
    ///
    /// let book = Book::builder("BOOK001", "Main Trading Book").build();
    /// ```
    #[must_use]
    pub fn builder(id: impl Into<BookId>, name: impl Into<String>) -> BookBuilder {
        BookBuilder::new(id, name)
    }

    /// Returns the book's unique identifier.
    #[inline]
    #[must_use]
    pub fn book_id(&self) -> &BookId { &self.book_id }

    /// Returns the book's name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str { &self.name }

    /// Returns the book's description.
    #[inline]
    #[must_use]
    pub fn description(&self) -> Option<&str> { self.description.as_deref() }

    /// Returns the book's type.
    #[inline]
    #[must_use]
    pub fn book_type(&self) -> BookType { self.book_type }

    /// Returns the book's regulatory classification.
    #[inline]
    #[must_use]
    pub fn regulatory_type(&self) -> Option<RegulatoryBookType> { self.regulatory_type }

    /// Returns the book's ownership information.
    #[inline]
    #[must_use]
    pub fn ownership(&self) -> Option<&BookOwnership> { self.ownership.as_ref() }

    /// Returns the book's metadata.
    #[inline]
    #[must_use]
    pub fn metadata(&self) -> &BookMetadata { &self.metadata }

    /// Returns true if this is a trading book.
    #[inline]
    #[must_use]
    pub fn is_trading(&self) -> bool { self.book_type.is_trading() }

    /// Returns true if this is a banking book.
    #[inline]
    #[must_use]
    pub fn is_banking(&self) -> bool { self.book_type.is_banking() }
}

// ============================================================================
// BookBuilder
// ============================================================================

/// Builder for constructing Book instances.
///
/// Provides a fluent API for setting book properties before construction.
///
/// # Examples
///
/// ```
/// use infra_master::book::{Book, BookType, BookOwnership, RegulatoryBookType};
///
/// let ownership = BookOwnership::new()
///     .with_desk("FX Spot")
///     .with_division("Markets");
///
/// let book = Book::builder("BOOK001", "Main Trading Book")
///     .description("Primary book for FX spot trading")
///     .book_type(BookType::Trading)
///     .regulatory_type(RegulatoryBookType::TB)
///     .ownership(ownership)
///     .build();
///
/// assert_eq!(book.book_type(), BookType::Trading);
/// assert!(book.ownership().is_some());
/// ```
#[derive(Clone, Debug)]
pub struct BookBuilder {
    book_id: BookId,
    name: String,
    description: Option<String>,
    book_type: BookType,
    regulatory_type: Option<RegulatoryBookType>,
    ownership: Option<BookOwnership>,
    metadata: BookMetadata,
}

impl BookBuilder {
    /// Creates a new book builder with required fields.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the book
    /// * `name` - Human-readable name for the book
    #[must_use]
    pub fn new(id: impl Into<BookId>, name: impl Into<String>) -> Self {
        Self {
            book_id: id.into(),
            name: name.into(),
            description: None,
            book_type: BookType::default(),
            regulatory_type: None,
            ownership: None,
            metadata: BookMetadata::new(),
        }
    }

    /// Sets the book's description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the book's type.
    #[must_use]
    pub fn book_type(mut self, book_type: BookType) -> Self {
        self.book_type = book_type;
        self
    }

    /// Sets the book's regulatory classification.
    #[must_use]
    pub fn regulatory_type(mut self, regulatory_type: RegulatoryBookType) -> Self {
        self.regulatory_type = Some(regulatory_type);
        self
    }

    /// Sets the book's ownership information.
    #[must_use]
    pub fn ownership(mut self, ownership: BookOwnership) -> Self {
        self.ownership = Some(ownership);
        self
    }

    /// Sets the book's metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: BookMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Sets the creator in metadata.
    #[must_use]
    pub fn created_by(mut self, creator: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_creator(creator);
        self
    }

    /// Builds the Book instance.
    ///
    /// This method consumes the builder and returns a fully constructed Book.
    #[must_use]
    pub fn build(self) -> Book {
        Book {
            book_id: self.book_id,
            name: self.name,
            description: self.description,
            book_type: self.book_type,
            regulatory_type: self.regulatory_type,
            ownership: self.ownership,
            metadata: self.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Book tests
    // ========================================================================

    #[test]
    fn test_book_builder_minimal() {
        let book = Book::builder("BOOK001", "Test Book").build();
        assert_eq!(book.book_id().as_str(), "BOOK001");
        assert_eq!(book.name(), "Test Book");
        assert_eq!(book.book_type(), BookType::Trading); // Default
        assert!(book.description().is_none());
        assert!(book.regulatory_type().is_none());
        assert!(book.ownership().is_none());
    }

    #[test]
    fn test_book_builder_with_description() {
        let book = Book::builder("BOOK001", "Test Book")
            .description("A test book")
            .build();
        assert_eq!(book.description(), Some("A test book"));
    }

    #[test]
    fn test_book_builder_with_book_type() {
        let book = Book::builder("BOOK001", "Test Book")
            .book_type(BookType::Banking)
            .build();
        assert_eq!(book.book_type(), BookType::Banking);
    }

    #[test]
    fn test_book_builder_with_regulatory_type() {
        let book = Book::builder("BOOK001", "Test Book")
            .regulatory_type(RegulatoryBookType::TB)
            .build();
        assert_eq!(book.regulatory_type(), Some(RegulatoryBookType::TB));
    }

    #[test]
    fn test_book_builder_with_ownership() {
        let ownership = BookOwnership::new()
            .with_desk("FX Spot")
            .with_division("Markets");
        let book = Book::builder("BOOK001", "Test Book")
            .ownership(ownership)
            .build();
        assert!(book.ownership().is_some());
        assert_eq!(book.ownership().unwrap().desk(), Some("FX Spot"));
    }

    #[test]
    fn test_book_builder_with_metadata() {
        let metadata = BookMetadata::new().with_creator("user1");
        let book = Book::builder("BOOK001", "Test Book")
            .metadata(metadata)
            .build();
        assert_eq!(book.metadata().created_by(), Some("user1"));
    }

    #[test]
    fn test_book_builder_with_created_by() {
        let book = Book::builder("BOOK001", "Test Book")
            .created_by("user1")
            .build();
        assert_eq!(book.metadata().created_by(), Some("user1"));
    }

    #[test]
    fn test_book_builder_full_chain() {
        let ownership = BookOwnership::new()
            .with_desk("FX Spot")
            .with_division("Markets");

        let book = Book::builder("BOOK001", "Main Trading Book")
            .description("Primary book for FX spot trading")
            .book_type(BookType::Trading)
            .regulatory_type(RegulatoryBookType::TB)
            .ownership(ownership)
            .created_by("admin")
            .build();

        assert_eq!(book.book_id().as_str(), "BOOK001");
        assert_eq!(book.name(), "Main Trading Book");
        assert_eq!(book.description(), Some("Primary book for FX spot trading"));
        assert_eq!(book.book_type(), BookType::Trading);
        assert_eq!(book.regulatory_type(), Some(RegulatoryBookType::TB));
        assert!(book.ownership().is_some());
        assert_eq!(book.metadata().created_by(), Some("admin"));
    }

    #[test]
    fn test_book_is_trading() {
        let trading_book = Book::builder("BOOK001", "Test")
            .book_type(BookType::Trading)
            .build();
        let banking_book = Book::builder("BOOK002", "Test")
            .book_type(BookType::Banking)
            .build();

        assert!(trading_book.is_trading());
        assert!(!banking_book.is_trading());
    }

    #[test]
    fn test_book_is_banking() {
        let banking_book = Book::builder("BOOK001", "Test")
            .book_type(BookType::Banking)
            .build();
        let trading_book = Book::builder("BOOK002", "Test")
            .book_type(BookType::Trading)
            .build();

        assert!(banking_book.is_banking());
        assert!(!trading_book.is_banking());
    }

    #[test]
    fn test_book_clone() {
        let book = Book::builder("BOOK001", "Test Book")
            .description("A test book")
            .build();
        let cloned = book.clone();
        assert_eq!(cloned.book_id().as_str(), "BOOK001");
        assert_eq!(cloned.description(), Some("A test book"));
    }

    #[test]
    fn test_book_id_from_string() {
        let book = Book::builder("BOOK001".to_string(), "Test Book").build();
        assert_eq!(book.book_id().as_str(), "BOOK001");
    }

    #[test]
    fn test_book_id_from_book_id() {
        let book_id = BookId::new("BOOK001");
        let book = Book::builder(book_id, "Test Book").build();
        assert_eq!(book.book_id().as_str(), "BOOK001");
    }

    // ========================================================================
    // BookBuilder tests
    // ========================================================================

    #[test]
    fn test_book_builder_new() {
        let builder = BookBuilder::new("BOOK001", "Test Book");
        let book = builder.build();
        assert_eq!(book.book_id().as_str(), "BOOK001");
    }

    #[test]
    fn test_book_builder_clone() {
        let builder = BookBuilder::new("BOOK001", "Test Book")
            .description("A test book");
        let cloned = builder.clone();
        let book = cloned.build();
        assert_eq!(book.description(), Some("A test book"));
    }
}
