//! Tree-based pricing methods for vanilla options.
//!
//! This module provides Binomial and Trinomial tree implementations for
//! option pricing, particularly useful for American-style options with
//! early exercise features.
//!
//! # Overview
//!
//! Tree methods discretise the underlying price process into a recombining
//! lattice, enabling efficient backward induction for option valuation. The
//! Cox-Ross-Rubinstein (CRR) binomial tree is the primary implementation,
//! providing:
//!
//! - Accurate European option pricing (converges to Black-Scholes)
//! - American option pricing with early exercise
//! - Direct calculation of Delta and Gamma from tree structure
//!
//! # Architecture
//!
//! | Type | Description |
//! |------|-------------|
//! | [`TreeConfig`] | Configuration for tree-based pricing (steps, type, Greeks) |
//! | [`TreeConfigBuilder`] | Builder pattern for constructing `TreeConfig` |
//! | [`BinomialTree`] | Cox-Ross-Rubinstein (CRR) binomial tree implementation |
//! | [`TrinomialTree`] | Kamrad-Ritchken trinomial tree implementation |
//! | [`CrrParams`] | CRR model parameters (u, d, p, dt) |
//! | [`TrinomialParams`] | Trinomial model parameters (u, d, p_u, p_m, p_d, dt) |
//! | [`TreeMethod`] | High-level pricing interface with unified result |
//! | [`TreeType`] | Selection between Binomial and Trinomial trees |
//!
//! # Examples
//!
//! ## Basic European Option Pricing
//!
//! ```rust
//! use pricer_pricing::tree::BinomialTree;
//!
//! // Create a binomial tree for a European call option
//! let tree = BinomialTree::new(
//!     100.0,  // spot price
//!     100.0,  // strike price
//!     1.0,    // time to expiry (years)
//!     0.05,   // risk-free rate (5%)
//!     0.2,    // volatility (20%)
//!     100,    // number of steps
//!     true,   // is_call: true for call, false for put
//!     false,  // is_american: false for European
//! ).expect("Valid parameters");
//!
//! let price = tree.price();
//! assert!(price > 0.0);
//! // European call converges to Black-Scholes (~10.45)
//! ```
//!
//! ## American Option with Early Exercise
//!
//! ```rust
//! use pricer_pricing::tree::BinomialTree;
//!
//! // American put option - early exercise value may exceed European
//! let european_put = BinomialTree::new(
//!     100.0, 110.0, 1.0, 0.05, 0.2, 200, false, false
//! ).unwrap();
//!
//! let american_put = BinomialTree::new(
//!     100.0, 110.0, 1.0, 0.05, 0.2, 200, false, true  // is_american = true
//! ).unwrap();
//!
//! // American put >= European put due to early exercise premium
//! assert!(american_put.price() >= european_put.price() - 1e-6);
//! ```
//!
//! ## Greeks Calculation
//!
//! ```rust
//! use pricer_pricing::tree::BinomialTree;
//!
//! let tree = BinomialTree::new(
//!     100.0, 100.0, 1.0, 0.05, 0.2, 500, true, false
//! ).unwrap();
//!
//! // Delta: sensitivity to spot price change
//! let delta = tree.delta();
//! assert!(delta > 0.0 && delta < 1.0); // ATM call delta ~0.5-0.6
//!
//! // Gamma: sensitivity of delta to spot price change
//! let gamma = tree.gamma();
//! assert!(gamma > 0.0); // Gamma is always positive for vanilla options
//! ```
//!
//! ## Using TreeMethod with Configuration
//!
//! ```rust
//! use pricer_pricing::tree::{TreeConfig, TreeMethod, TreeType};
//!
//! // Build configuration with custom settings
//! let config = TreeConfig::builder()
//!     .num_steps(500)
//!     .tree_type(TreeType::Binomial)
//!     .compute_greeks(true)
//!     .build()
//!     .expect("Valid config");
//!
//! // Create method instance
//! let method = TreeMethod::new(config);
//!
//! // Price with unified result
//! let result = method.price(
//!     100.0, 100.0, 1.0, 0.05, 0.2, true, false
//! ).expect("Pricing succeeded");
//!
//! assert!(result.pv > 0.0);
//! assert!(result.greeks.is_some());
//! ```
//!
//! # Convergence
//!
//! The binomial tree converges to the Black-Scholes price as the number of
//! steps increases. For ATM options:
//!
//! | Steps | Error vs Black-Scholes |
//! |-------|------------------------|
//! | 50    | ~0.05                  |
//! | 100   | ~0.02                  |
//! | 500   | ~0.005                 |
//!
//! For practical pricing, 100-500 steps provide a good balance between accuracy
//! and computational speed.
//!
//! # Theory
//!
//! The CRR binomial tree models price movement with:
//!
//! - **Up factor**: `u = exp(σ√Δt)`
//! - **Down factor**: `d = 1/u`
//! - **Risk-neutral probability**: `p = (exp(rΔt) - d) / (u - d)`
//!
//! At each node, the option value is computed via backward induction:
//!
//! - **European**: `V = exp(-rΔt) * (p * V_up + (1-p) * V_down)`
//! - **American**: `V = max(intrinsic, continuation_value)`
//!
//! # See Also
//!
//! - [`crate::methods::mc::MonteCarloPricer`] - Monte Carlo pricing for path-dependent
//!   options

mod binomial;
mod config;
mod method;
mod trinomial;

pub use binomial::{BinomialTree, CrrParams};
pub use config::{TreeConfig, TreeConfigBuilder, TreeType};
pub use method::TreeMethod;
pub use trinomial::{TrinomialParams, TrinomialTree};
