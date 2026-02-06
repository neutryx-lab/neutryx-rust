//! Trading Book management module.
//!
//! This module provides comprehensive types for managing trading books,
//! including book types, ownership information, and metadata.
//!
//! Uses `bon::Builder` for fluent construction with compile-time safety.
//!
//! # Module Structure
//!
//! - `book`: Book entity and builder
//! - `types`: BookType, RegulatoryBookType, BookOwnership, BookMetadata
//!
//! # Example
//!
//! ```
//! use infra_domain::book::{Book, BookType, RegulatoryBookType};
//!
//! let book = Book::builder()
//!     .book_id("BOOK001")
//!     .name("Main Trading Book")
//!     .book_type(BookType::Trading)
//!     .regulatory_type(RegulatoryBookType::TB)
//!     .build();
//!
//! assert_eq!(book.name(), "Main Trading Book");
//! assert_eq!(book.book_type(), BookType::Trading);
//! ```

mod book;
mod types;

pub use book::*;
pub use types::*;

/// Prelude for commonly used book types.
pub mod prelude {
    pub use super::{Book, BookBuilder, BookMetadata, BookOwnership, BookType, RegulatoryBookType};
}
