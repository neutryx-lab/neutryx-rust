//! Tree-based pricing methods.
//!
//! This module provides Binomial and Trinomial tree implementations for
//! option pricing, particularly useful for American-style options with
//! early exercise features.
//!
//! # Architecture
//!
//! - [`TreeConfig`]: Configuration for tree-based pricing
//! - [`BinomialTree`]: Cox-Ross-Rubinstein (CRR) binomial tree
//! - [`TreeMethod`]: High-level pricing method interface
//! - [`TreeType`]: Selection between Binomial and Trinomial trees
//!
//! # Example
//!
//! ```rust
//! use pricer_pricing::tree::{BinomialTree, TreeConfig, TreeType};
//!
//! // Create a binomial tree for a European call option
//! let tree = BinomialTree::new(
//!     100.0,  // spot
//!     100.0,  // strike
//!     1.0,    // time to expiry (years)
//!     0.05,   // risk-free rate
//!     0.2,    // volatility
//!     100,    // number of steps
//!     true,   // is call
//!     false,  // is American
//! ).unwrap();
//!
//! let price = tree.price();
//! let delta = tree.delta();
//! let gamma = tree.gamma();
//! ```

mod binomial;
mod config;
mod method;

pub use binomial::{BinomialTree, CrrParams};
pub use config::{TreeConfig, TreeConfigBuilder, TreeType};
pub use method::TreeMethod;
