//! FX probability density calculator for risk-neutral density analysis.
//!
//! Note: The implementations have been consolidated into `surfaces/fx.rs`.
//! This module re-exports them for backward compatibility.
//!
//! # Available Types
//!
//! - [`FxDensityCalculator`]: Delta-Strike conversion and probability density
//!   calculation for FX options
//! - [`DeltaType`]: Delta convention types (Spot, Forward, Premium-adjusted)
//! - [`DensityStatistics`]: Distribution statistics from density analysis
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::market::fx_density::{FxDensityCalculator, DeltaType};
//! use pricer_models::market::surfaces::FxVolatilitySurface;
//!
//! let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();
//! let calculator = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);
//!
//! // Delta to strike conversion
//! let strike = calculator.delta_to_strike(0.25, 0.5, 0.10, DeltaType::SpotDelta).unwrap();
//!
//! // Probability density at a given strike
//! let density = calculator.probability_density(1.10, 0.5).unwrap();
//! ```

// Re-export from the canonical location (surfaces/fx.rs)
pub use crate::market::surfaces::{DeltaType, DensityStatistics, FxDensityCalculator};
