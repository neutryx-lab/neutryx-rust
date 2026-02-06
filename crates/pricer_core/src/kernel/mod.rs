// SAFETY: This module contains aligned_buffer.rs which requires unsafe code
// for custom memory allocation with guaranteed 64-byte alignment.
#![allow(unsafe_code)]

//! Pricing Kernel module.
//!
//! This module provides the data structures for PricingKernel,
//! which transforms hierarchical `Trade` definitions into SoA (Structure of
//! Arrays) format optimised for SIMD operations and Enzyme automatic
//! differentiation.
//!
//! # Architecture
//!
//! ```text
//! Source: Trade (hierarchical, dates, calendars, strings)
//!    ↓
//! Compiler: TradeCompiler (date calculation, holiday adjustment, schedule expansion)
//!    ↓
//! Kernel: PricingKernel (flattened f64 and usize arrays)
//! ```
//!
//! # Components
//!
//! - [`AlignedBuffer`]: 64-byte aligned heap buffer for SIMD efficiency
//! - [`PricingKernel`]: SoA cashflow representation for linear products
//! - [`ScriptKernel`]: Event-driven kernel for path-dependent products
//! - [`CallableKernel`]: Block-structured kernel for callable/Bermudan products
//! - [`CompileError`]: Structured compilation error types
//!
//! # Example
//!
//! ```
//! use pricer_core::kernel::{AlignedBuffer, PricingKernel};
//!
//! // Create aligned buffer for payment dates
//! let payment_dates: AlignedBuffer<i32> = AlignedBuffer::from_vec(vec![19000, 19180, 19365]);
//!
//! // All arrays must have equal length
//! assert_eq!(payment_dates.len(), 3);
//! assert!(payment_dates.is_aligned());
//! ```

mod aligned_buffer;
mod callable_kernel;
mod error;
mod pricing_kernel;
mod script_kernel;

pub use aligned_buffer::{AlignedBuffer, ALIGNMENT};
pub use callable_kernel::{
    CallableBlock, CallableKernel, CallableKernelBuilder, ExerciseDef, ExerciseStyle,
};
pub use error::CompileError;
pub use pricing_kernel::{PricingKernel, PricingKernelBuilder};
pub use script_kernel::{BarrierType, ScriptKernel, ScriptKernelBuilder, ScriptOp};
