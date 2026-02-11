//! Book entity and builder.

use bon::Builder;

use super::{BookMetadata, BookOwnership, BookType, RegulatoryBookType};
use crate::ids::BookId;

/// A trading book.
#[derive(Clone, Debug, Builder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[allow(clippy::struct_field_names)]
pub struct Book {
    /// Unique identifier for this book.
    #[builder(into)]
    book_id: BookId,
    /// Human-readable name for this book.
    #[builder(into)]
    name: String,
    /// Optional description of the book's purpose.
    #[builder(into)]
    description: Option<String>,
    /// Type of trading book.
    #[builder(default)]
    book_type: BookType,
    /// Regulatory classification.
    regulatory_type: Option<RegulatoryBookType>,
    /// Ownership information.
    ownership: Option<BookOwnership>,
    /// Audit metadata.
    #[builder(default)]
    metadata: BookMetadata,
}

impl Book {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_builder_minimal() {
        let book = Book::builder().book_id("BOOK001").name("Test Book").build();
        assert_eq!(book.book_id().as_str(), "BOOK001");
        assert_eq!(book.name(), "Test Book");
        assert_eq!(book.book_type(), BookType::Trading);
        assert!(book.description().is_none());
        assert!(book.regulatory_type().is_none());
        assert!(book.ownership().is_none());
    }

    #[test]
    fn test_book_builder_with_description() {
        let book = Book::builder()
            .book_id("BOOK001")
            .name("Test Book")
            .description("A test book")
            .build();
        assert_eq!(book.description(), Some("A test book"));
    }

    #[test]
    fn test_book_builder_with_book_type() {
        let book = Book::builder()
            .book_id("BOOK001")
            .name("Test Book")
            .book_type(BookType::Banking)
            .build();
        assert_eq!(book.book_type(), BookType::Banking);
    }

    #[test]
    fn test_book_builder_with_regulatory_type() {
        let book = Book::builder()
            .book_id("BOOK001")
            .name("Test Book")
            .regulatory_type(RegulatoryBookType::TB)
            .build();
        assert_eq!(book.regulatory_type(), Some(RegulatoryBookType::TB));
    }

    #[test]
    fn test_book_builder_with_ownership() {
        let ownership = BookOwnership::new()
            .with_desk("FX Spot")
            .with_division("Markets");
        let book = Book::builder()
            .book_id("BOOK001")
            .name("Test Book")
            .ownership(ownership)
            .build();
        assert!(book.ownership().is_some());
        assert_eq!(book.ownership().unwrap().desk(), Some("FX Spot"));
    }

    #[test]
    fn test_book_builder_with_metadata() {
        let metadata = BookMetadata::new().with_creator("user1");
        let book = Book::builder()
            .book_id("BOOK001")
            .name("Test Book")
            .metadata(metadata)
            .build();
        assert_eq!(book.metadata().created_by(), Some("user1"));
    }

    #[test]
    fn test_book_builder_full_chain() {
        let ownership = BookOwnership::new()
            .with_desk("FX Spot")
            .with_division("Markets");
        let metadata = BookMetadata::new().with_creator("admin");

        let book = Book::builder()
            .book_id("BOOK001")
            .name("Main Trading Book")
            .description("Primary book for FX spot trading")
            .book_type(BookType::Trading)
            .regulatory_type(RegulatoryBookType::TB)
            .ownership(ownership)
            .metadata(metadata)
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
        let trading_book = Book::builder()
            .book_id("BOOK001")
            .name("Test")
            .book_type(BookType::Trading)
            .build();
        let banking_book = Book::builder()
            .book_id("BOOK002")
            .name("Test")
            .book_type(BookType::Banking)
            .build();

        assert!(trading_book.is_trading());
        assert!(!banking_book.is_trading());
    }

    #[test]
    fn test_book_is_banking() {
        let banking_book = Book::builder()
            .book_id("BOOK001")
            .name("Test")
            .book_type(BookType::Banking)
            .build();
        let trading_book = Book::builder()
            .book_id("BOOK002")
            .name("Test")
            .book_type(BookType::Trading)
            .build();

        assert!(banking_book.is_banking());
        assert!(!trading_book.is_banking());
    }

    #[test]
    fn test_book_clone() {
        let book = Book::builder()
            .book_id("BOOK001")
            .name("Test Book")
            .description("A test book")
            .build();
        let cloned = book.clone();
        assert_eq!(cloned.book_id().as_str(), "BOOK001");
        assert_eq!(cloned.description(), Some("A test book"));
    }

    #[test]
    fn test_book_id_from_string() {
        let book = Book::builder()
            .book_id("BOOK001".to_string())
            .name("Test Book")
            .build();
        assert_eq!(book.book_id().as_str(), "BOOK001");
    }

    #[test]
    fn test_book_id_from_book_id() {
        let book_id = BookId::new("BOOK001");
        let book = Book::builder().book_id(book_id).name("Test Book").build();
        assert_eq!(book.book_id().as_str(), "BOOK001");
    }
}
