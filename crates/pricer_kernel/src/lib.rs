//! # Pricer Kernel (Layer 3: AD Engine)
//!
//! ## Layer 3 Role
//!
//! pricer_kernel serves as the AD (Automatic Differentiation) engine in the 4-layer architecture:
//! - Enzyme LLVM-level automatic differentiation
//! - Monte Carlo pricing kernels with AD integration (Phase 3.2)
//! - Gradient verification utilities
//!
//! ## Nightly Rust Requirement
//!
//! This is the **only crate** that requires nightly Rust toolchain (`nightly-2025-01-15`).
//! Enzyme operates at LLVM level and requires nightly features for optimal performance.
//!
//! ## Layer Integration (Phase 4)
//!
//! Phase 4 adds optional L1/L2 integration via the `l1l2-integration` feature:
//! - Layer 1 (pricer_core): smoothing functions, Float trait, YieldCurve
//! - Layer 2 (pricer_models): StochasticModel trait, Instrument enum
//!
//! Without the feature flag, pricer_kernel remains fully isolated.
//!
//! ## Usage Example
//!
//! ```rust
//! use pricer_kernel::verify::{square, square_gradient};
//!
//! // Function value
//! let value = square(3.0);
//! assert_eq!(value, 9.0);
//!
//! // Gradient verification (placeholder in Phase 3.0)
//! let gradient = square_gradient(3.0);
//! assert!((gradient - 6.0).abs() < 1e-10);
//! ```
//!
//! ## Installation
//!
//! ### Docker (Recommended)
//!
//! ```bash
//! docker build -f docker/Dockerfile.nightly -t neutryx-enzyme .
//! docker run -it neutryx-enzyme
//! ```
//!
//! ### Manual Installation
//!
//! 1. Install LLVM 18:
//!    ```bash
//!    # Ubuntu/Debian
//!    wget https://apt.llvm.org/llvm.sh
//!    chmod +x llvm.sh
//!    sudo ./llvm.sh 18
//!    ```
//!
//! 2. Install nightly Rust:
//!    ```bash
//!    rustup toolchain install nightly-2025-01-15
//!    ```
//!
//! 3. Build pricer_kernel:
//!    ```bash
//!    cargo +nightly build -p pricer_kernel
//!    cargo +nightly test -p pricer_kernel
//!    ```
//!
//! ## Known Constraints
//!
//! - **Nightly Rust Required**: This crate uses `rust-toolchain.toml` to enforce nightly-2025-01-15
//! - **LLVM 18 Dependency**: llvm-sys requires LLVM 18 to be installed on the system (enzyme-ad feature)
//! - **Optional L1/L2**: Use `--features l1l2-integration` to enable pricer_core/pricer_models

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
// Enzyme-specific nightly features (commented until Enzyme is integrated in Phase 4)
// #![feature(autodiff)]

// Phase 3.0: Core modules
pub mod verify;

// Phase 3.0: Enzyme autodiff infrastructure (placeholder implementation)
pub mod enzyme;

// Phase 3.0: Enzyme gradient verification tests
mod verify_enzyme;

// Phase 4: L1/L2 integration tests (conditional compilation)
#[cfg(all(test, feature = "l1l2-integration"))]
mod integration_tests;

// Phase 3.1a: Random number generation infrastructure
pub mod rng;

// Phase 3.2: Monte Carlo kernel with Enzyme AD integration
pub mod mc;

// Phase 4: Path-dependent options and checkpointing
pub mod path_dependent;

// Phase 4: Checkpointing for memory-efficient AD
pub mod checkpoint;

// Phase 4: Analytical solutions for verification
pub mod analytical;

// Greeks calculation types and configuration
pub mod greeks;

// Re-export commonly used items for convenience
pub use enzyme::{gradient, gradient_with_step, Activity};
pub use greeks::{GreeksConfig, GreeksMode, GreeksResult};
pub use mc::{GbmParams, Greek, MonteCarloConfig, MonteCarloPricer, PayoffParams, PricingResult};
