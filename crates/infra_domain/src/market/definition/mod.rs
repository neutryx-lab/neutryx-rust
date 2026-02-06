//! Market object definitions for curve and surface construction.
//!
//! This module provides definition types that specify how to build market
//! objects:
//!
//! - [`CurveDefinition`]: Recipe for building yield curves
//! - [`VolSurfaceDefinition`]: Specification for volatility surfaces
//!   (calibration model, strike type)
//! - [`InstrumentDefinition`]: Calibration instrument definitions
//! - [`RateIndexDefinition`]: Benchmark rate index definitions
//! - [`JumpPillar`]: Rate jump definitions for central bank meetings
//!
//! # Architecture
//!
//! Definition types serve as master data that specify:
//! - **What** instruments to use for calibration
//! - **How** to construct the resulting curve/surface (interpolation, model,
//!   etc.)
//!
//! They reference [`InstrumentDefinition`]s and [`RateIndexDefinition`]s by ID,
//! enabling a clean separation between definition and runtime data.
//!
//! # Examples
//!
//! ## Curve Definition
//!
//! ```
//! use infra_domain::market::definition::{CurveDefinition, CalibrationMethod};
//!
//! let curve = CurveDefinition::new(
//!     "USD-SOFR-Discount",
//!     "USD-SOFR",
//!     vec![
//!         "USD-Depo-ON".to_string(),
//!         "USD-OIS-1Y".to_string(),
//!         "USD-OIS-5Y".to_string(),
//!     ],
//! )
//! .with_calibration_method(CalibrationMethod::Sequential);
//! ```
//!
//! ## Vol Surface Definition
//!
//! ```
//! use infra_domain::market::definition::{VolSurfaceDefinition, CalibrationModel, StrikeAxisType};
//!
//! let vol_surface = VolSurfaceDefinition::new(
//!     "USD-SOFR-Swaption-Vol",
//!     vec!["USD-SOFR-1Y1Y-ATM".to_string()],
//! )
//! .with_model(CalibrationModel::Sabr)
//! .with_strike_axis(StrikeAxisType::Delta);
//!
//! assert_eq!(vol_surface.name, "USD-SOFR-Swaption-Vol");
//! ```

mod curve;
mod index;
mod instrument;
mod vol_surface;

// Curve definitions (including JumpPillar types)
pub use curve::{
    CalibrationMethod, CurveDefError, CurveDefinition, InterpolationMethod, JumpPillar,
    JumpPillarBuilder,
};
// Index definitions (rate, FX, etc.)
pub use index::{IndexConventions, RateIndexDefError, RateIndexDefinition};
// Instrument definitions
pub use instrument::{
    InstrumentConventions, InstrumentDefError, InstrumentDefinition, InstrumentTemplate,
};
// Vol surface definitions
pub use vol_surface::{
    CalibrationModel, StrikeAxisType, StrikeInterpolation, TimeInterpolation, VolSurfaceDefError,
    VolSurfaceDefinition,
};
