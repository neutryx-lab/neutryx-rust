//! # Pricer XVA (L4: Application)
//!
//! Portfolio aggregation, XVA calculations, and parallelization.
//!
//! This crate provides:
//! - Portfolio and trade structures
//! - CVA, DVA, FVA, KVA calculations
//! - Structure of Arrays (SoA) for cache efficiency
//! - Rayon-based parallelization for Greeks computation
//!
//! ## Performance
//!
//! - Uses SoA layout for SIMD-friendly memory access
//! - Parallel computation with Rayon (target: >80% efficiency on 32 cores)
//! - Batch processing for large portfolios

#![warn(missing_docs)]

pub mod parallel;
pub mod portfolio;
pub mod soa;
pub mod xva;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
