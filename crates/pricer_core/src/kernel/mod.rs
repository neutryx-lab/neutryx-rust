// SAFETY: This module contains aligned_buffer.rs which requires unsafe code
// for custom memory allocation with guaranteed 64-byte alignment.
#![allow(unsafe_code)]

//! Pricing Kernel: SoA data structures for SIMD-optimised pricing and Enzyme
//! AD.

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
