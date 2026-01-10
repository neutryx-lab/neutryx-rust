//! File source generators.
//!
//! This module generates CSV and other file formats
//! for batch loading via adapter_loader.

mod csv_generator;

pub use csv_generator::CsvGenerator;

/// Trait for file generators
pub trait FileGenerator: Send + Sync {
    /// Generate file content as string
    fn generate(&self) -> String;

    /// Write to file
    fn write_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.generate())
    }
}
