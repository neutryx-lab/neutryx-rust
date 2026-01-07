//! Monte Carlo pricing kernels with Enzyme AD integration.
//!
//! This module provides the Monte Carlo simulation infrastructure for pricing
//! financial derivatives with automatic differentiation support via Enzyme.
//!
//! # Phase 3.2 Status
//!
//! This is the **Phase 3.2 implementation** of the Monte Carlo kernel. Features include:
//!
//! - GBM path generation with log-space formulation
//! - Pre-allocated workspace buffers for allocation-free simulation
//! - Smooth payoff functions for AD compatibility
//! - Greeks via bump-and-revalue (placeholder for Enzyme AD)
//! - Manual tangent propagation for Delta (forward-mode AD prototype)
//!
//! Phase 4 will integrate actual Enzyme `#[autodiff]` macros.
//!
//! # Architecture
//!
//! ```text
//! MonteCarloPricer
//! ├── MonteCarloConfig  (simulation parameters)
//! ├── PathWorkspace     (pre-allocated buffers)
//! ├── PricerRng         (random number generation)
//! └── Orchestration
//!     ├── generate_gbm_paths()
//!     ├── compute_payoffs()
//!     └── Greeks computation
//! ```
//!
//! # Workspace Buffer Hoisting
//!
//! All memory allocations are hoisted outside the simulation loop via
//! [`PathWorkspace`]. This is critical for Enzyme LLVM-level optimisation:
//!
//! ```rust
//! use pricer_pricing::mc::{MonteCarloPricer, MonteCarloConfig, GbmParams, PayoffParams};
//!
//! let config = MonteCarloConfig::builder()
//!     .n_paths(10_000)
//!     .n_steps(252)
//!     .seed(42)
//!     .build()
//!     .unwrap();
//!
//! let mut pricer = MonteCarloPricer::new(config).unwrap();
//!
//! // Workspace is reused across pricing calls
//! let result1 = pricer.price_european(GbmParams::default(), PayoffParams::call(100.0), 0.95);
//! let result2 = pricer.price_european(GbmParams::default(), PayoffParams::put(100.0), 0.95);
//! ```
//!
//! # Enzyme Activity Analysis
//!
//! For AD compatibility, parameters have defined activities:
//!
//! | Parameter | Activity | Mode |
//! |-----------|----------|------|
//! | spot | Dual | Forward (Delta) |
//! | volatility | Dual | Forward (Vega) |
//! | rate | Dual | Forward (Rho) |
//! | randoms | Const | Frozen during AD |
//! | payoff | Active | Reverse (adjoint) |
//!
//! # Examples
//!
//! ## Basic European Option Pricing
//!
//! ```rust
//! use pricer_pricing::mc::{
//!     MonteCarloPricer, MonteCarloConfig, GbmParams, PayoffParams,
//! };
//!
//! let config = MonteCarloConfig::builder()
//!     .n_paths(10_000)
//!     .n_steps(252)
//!     .seed(42)
//!     .build()
//!     .unwrap();
//!
//! let mut pricer = MonteCarloPricer::new(config).unwrap();
//!
//! let gbm = GbmParams {
//!     spot: 100.0,
//!     rate: 0.05,
//!     volatility: 0.2,
//!     maturity: 1.0,
//! };
//! let payoff = PayoffParams::call(100.0);
//! let discount_factor = (-0.05_f64).exp();
//!
//! let result = pricer.price_european(gbm, payoff, discount_factor);
//! println!("Price: {:.4} +/- {:.4}", result.price, result.std_error);
//! ```
//!
//! ## Pricing with Greeks
//!
//! ```rust
//! use pricer_pricing::mc::{
//!     MonteCarloPricer, MonteCarloConfig, GbmParams, PayoffParams, Greek,
//! };
//!
//! let config = MonteCarloConfig::builder()
//!     .n_paths(50_000)
//!     .n_steps(252)
//!     .seed(42)
//!     .build()
//!     .unwrap();
//!
//! let mut pricer = MonteCarloPricer::new(config).unwrap();
//! let gbm = GbmParams::default();
//! let payoff = PayoffParams::call(100.0);
//! let df = (-0.05_f64).exp();
//!
//! let result = pricer.price_with_greeks(gbm, payoff, df, &[Greek::Delta, Greek::Vega]);
//!
//! println!("Price: {:.4}", result.price);
//! println!("Delta: {:.4}", result.delta.unwrap());
//! println!("Vega:  {:.4}", result.vega.unwrap());
//! ```
//!
//! ## Forward-Mode AD for Delta
//!
//! ```rust
//! use pricer_pricing::mc::{
//!     MonteCarloPricer, MonteCarloConfig, GbmParams, PayoffParams,
//! };
//!
//! let config = MonteCarloConfig::builder()
//!     .n_paths(10_000)
//!     .n_steps(50)
//!     .seed(42)
//!     .build()
//!     .unwrap();
//!
//! let mut pricer = MonteCarloPricer::new(config).unwrap();
//! let gbm = GbmParams::default();
//! let payoff = PayoffParams::call(100.0);
//! let df = (-0.05_f64).exp();
//!
//! // Compute price and delta in a single forward pass
//! let (price, delta) = pricer.price_with_delta_ad(gbm, payoff, df);
//! println!("Price: {:.4}, Delta: {:.4}", price, delta);
//! ```

pub mod config;
pub mod error;
pub mod paths;
pub mod payoff;
pub mod pricer;
pub mod pricer_checkpoint;
pub mod thread_local;
pub mod workspace;
pub mod workspace_checkpoint;

// Re-exports for convenient access
pub use config::{AdMode, MonteCarloConfig, MonteCarloConfigBuilder};
pub use error::ConfigError;
pub use paths::{generate_gbm_paths, GbmParams};
pub use payoff::{
    asian_arithmetic_call_smooth, asian_arithmetic_put_smooth, compute_payoff, compute_payoffs,
    european_call_smooth, european_put_smooth, soft_plus, soft_plus_derivative, PayoffParams,
    PayoffType,
};
pub use pricer::{Greek, MonteCarloPricer, PricingResult};
pub use thread_local::{
    current_thread_index, DefaultWorkspaceFactory, ParallelWorkspaces, ThreadLocalWorkspacePool,
    WorkspaceFactory,
};
pub use workspace::PathWorkspace;
pub use workspace_checkpoint::CheckpointWorkspace;
