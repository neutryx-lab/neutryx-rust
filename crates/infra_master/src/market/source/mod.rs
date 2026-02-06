//! Market data sources and mappings.
//!
//! This module provides types for data source identification and mapping:
//!
//! - [`DataSource`]: Data provider identification (Bloomberg, Reuters, etc.)
//! - [`SourcePriority`]: Priority levels for data sources
//! - [`TickerMapping`]: External ticker to internal ID mapping
//! - [`InstrumentMapper`]: Trait for rate-to-instrument conversion
//! - [`StandardInstrumentMapper`]: Default instrument mapping
//!
//! # Examples
//!
//! ```
//! use infra_master::market::source::{DataSource, SourcePriority};
//!
//! let source = DataSource::Bloomberg;
//! assert_eq!(source.code(), "BBG");
//! ```

mod data_source;
mod mapper;
mod ticker;

pub use data_source::{DataSource, SourcePriority};
pub use mapper::{InstrumentMapper, StandardInstrumentMapper};
pub use ticker::TickerMapping;
