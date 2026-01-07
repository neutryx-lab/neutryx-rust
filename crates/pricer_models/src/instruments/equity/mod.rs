//! Equity derivative instruments.
//!
//! This module provides equity-linked derivative instruments including:
//! - Vanilla options (European, American, Bermudan)
//! - Forward contracts
//!
//! # Feature Flag
//!
//! This module is available when the `equity` feature is enabled (default).
//!
//! # Examples
//!
//! ```
//! use pricer_models::instruments::equity::{VanillaOption, Forward};
//! use pricer_models::instruments::{InstrumentParams, PayoffType, ExerciseStyle, Direction};
//!
//! // Create a vanilla call option
//! let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
//! let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
//!
//! // Create a forward contract
//! let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
//! ```

// Re-export equity instruments from parent module for organized access
pub use super::forward::{Direction, Forward};
pub use super::vanilla::VanillaOption;
