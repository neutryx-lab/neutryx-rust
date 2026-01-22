//! Mathematical utility functions.
//!
//! This module provides common mathematical utility functions used throughout
//! the pricing library, including basic functions, combinatorics, and special
//! functions.
//!
//! ## Submodules
//!
//! - `basic`: Sign, clamp, and linear interpolation functions
//! - `combinatorics`: Factorial and binomial coefficient functions
//! - `special`: Special functions like log-gamma and beta
//!
//! ## AD Compatibility
//!
//! All functions are generic over `T: Float` to support automatic
//! differentiation through dual numbers.

mod basic;
mod combinatorics;
mod special;

pub use basic::{clamp, lerp, sign};
pub use combinatorics::{binomial, factorial, falling_factorial};
pub use special::{beta, log_gamma};
