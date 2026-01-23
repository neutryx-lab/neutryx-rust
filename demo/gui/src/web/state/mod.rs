//! State management module for the FrictionalBank Web API.
//!
//! This module organises application state into focused components:
//!
//! - [`metrics`]: Performance metrics with O(1) ring buffer implementation
//!
//! # Architecture
//!
//! The state module follows separation of concerns, with each submodule
//! handling a specific aspect of application state. This makes the codebase
//! easier to maintain and test.

pub mod metrics;

// Re-export commonly used types
pub use metrics::{PerformanceMetrics, RingBuffer};
