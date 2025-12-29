//! # Pricer Kernel (L3: AD Engine)
//!
//! Monte Carlo engine with Enzyme automatic differentiation.
//!
//! This crate provides:
//! - Enzyme AD bindings (nightly Rust required)
//! - Monte Carlo pricing kernels
//! - Checkpointing for memory-efficient reverse-mode AD
//! - Verification tests (Enzyme vs finite differences vs num-dual)
//!
//! ## Safety
//!
//! This is the **only crate** that requires nightly Rust and contains
//! Enzyme-specific code. All experimental/unsafe code is isolated here.
//!
//! ## Warning
//!
//! Requires nightly Rust toolchain and Enzyme LLVM plugin to be installed.
//! See `docker/Dockerfile.nightly` and `scripts/install_enzyme.sh`.

#![warn(missing_docs)]
// Enzyme-specific nightly features (commented until Enzyme is installed)
// #![feature(autodiff)]

pub mod checkpoint;
pub mod enzyme;
pub mod mc;
pub mod verify;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
