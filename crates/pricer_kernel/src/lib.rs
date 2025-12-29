//! # Pricer Kernel (Layer 3: AD Engine)
//!
//! ## Layer 3 Role
//!
//! pricer_kernel serves as the AD (Automatic Differentiation) engine in the 4-layer architecture:
//! - Enzyme LLVM-level automatic differentiation
//! - Monte Carlo pricing kernels (Phase 4)
//! - Gradient verification utilities
//!
//! ## Nightly Rust Requirement
//!
//! This is the **only crate** that requires nightly Rust toolchain (`nightly-2025-01-15`).
//! Enzyme operates at LLVM level and requires nightly features for optimal performance.
//!
//! ## Zero Dependency Principle (Phase 3.0)
//!
//! Phase 3.0 focuses on infrastructure setup with **complete isolation**:
//! - NO dependencies on Layer 1 (pricer_core)
//! - NO dependencies on Layer 2 (pricer_models)
//! - Only LLVM bindings (llvm-sys) and basic numeric traits
//!
//! Layer 1/2 integration will be added in Phase 4.
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
//! - **LLVM 18 Dependency**: llvm-sys requires LLVM 18 to be installed on the system
//! - **Enzyme Status**: Phase 3.0 uses placeholder implementation; actual Enzyme integration in Phase 4
//! - **Phase 3.0 Isolation**: No pricer_* crate dependencies; complete Layer 3 isolation

#![warn(missing_docs)]
// Enzyme-specific nightly features (commented until Enzyme is integrated in Phase 4)
// #![feature(autodiff)]

// Phase 3.0: Core modules
pub mod verify;

// Phase 3.0: Enzyme autodiff infrastructure (placeholder implementation)
pub mod enzyme;

// Phase 3.0: Enzyme gradient verification tests
mod verify_enzyme;

// Phase 3.1a: Random number generation infrastructure
pub mod rng;

// Phase 4: These modules will be activated for Monte Carlo
// pub mod checkpoint;
// pub mod mc;

// Re-export commonly used items for convenience
pub use enzyme::{Activity, gradient, gradient_with_step};
