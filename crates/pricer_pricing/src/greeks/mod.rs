//! Greeks calculation types and configuration (internal use only).
//!
//! This module provides internal types for the generic_pricer module.
//! External users should use `pricer_risk::greeks` instead.

mod config;
mod result;

// Keep error module for potential future use
#[allow(dead_code)]
mod error;

pub(crate) use config::{GreeksConfig, GreeksMode};
pub(crate) use result::GreeksResult;
