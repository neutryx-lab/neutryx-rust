//! Trading Book management module.

mod book;
mod types;

pub use book::*;
pub use types::*;

/// Prelude for commonly used book types.
pub mod prelude {
    pub use super::{Book, BookBuilder, BookMetadata, BookOwnership, BookType, RegulatoryBookType};
}
