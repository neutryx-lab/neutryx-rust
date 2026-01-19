//! CLI configuration loading
//!
//! Re-exports configuration from `infra_config` for CLI use.
//! This module provides a unified configuration interface, eliminating duplicate
//! config definitions across service crates.

#![allow(unused_imports)]
pub use infra_config::{ConfigError, Settings};
