//! Structure of Arrays (SoA) for cache-efficient batch processing.
//!
//! This module provides SoA representations of trade and exposure data
//! optimised for vectorised operations and cache efficiency.
//!
//! # Architecture
//!
//! Traditional Array of Structures (AoS) stores data as:
//! ```text
//! [Trade1{strike, maturity, notional}, Trade2{...}, ...]
//! ```
//!
//! Structure of Arrays (SoA) stores data as:
//! ```text
//! strikes:    [s1, s2, s3, ...]
//! maturities: [m1, m2, m3, ...]
//! notionals:  [n1, n2, n3, ...]
//! ```
//!
//! SoA provides better cache locality for operations that iterate
//! over a single field across many elements.

mod exposure_soa;
mod trade_soa;

pub use exposure_soa::ExposureSoA;
pub use trade_soa::TradeSoA;
