//! Market object definitions for curve and surface construction.
//!
//! This module provides definition types that specify how to build market objects:
//!
//! - [`CurveDefinition`]: Recipe for building yield curves
//! - [`VolSurfaceDefinition`]: Specification for volatility surfaces (calibration model, strike type)
//! - [`InstrumentDefinition`]: Calibration instrument definitions
//! - [`RateIndexDefinition`]: Benchmark rate index definitions
//! - [`JumpPillar`]: Rate jump definitions for central bank meetings
//!
//! # Architecture
//!
//! Definition types serve as master data that specify:
//! - **What** instruments to use for calibration
//! - **How** to construct the resulting curve/surface (interpolation, model, etc.)
//!
//! They reference [`InstrumentDefinition`]s and [`RateIndexDefinition`]s by ID,
//! enabling a clean separation between definition and runtime data.
//!
//! # Examples
//!
//! ## Curve Definition
//!
//! ```
//! use infra_master::market::definition::{CurveDefinition, CalibrationMethod};
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
//! use infra_master::market::definition::{CalibrationModel, StrikeAxisType};
//!
//! let model = CalibrationModel::Sabr;
//! assert!(model.is_enabled());
//! assert_eq!(model.parameter_count(), 4);
//! ```

mod curve;
mod instrument;
mod jump_pillar;
mod rate_index;
mod vol_surface;

// Curve definitions
pub use curve::{CalibrationMethod, CurveDefError, CurveDefinition, InterpolationMethod};

// Jump pillar definitions
pub use jump_pillar::JumpPillar;

// Vol surface definitions
pub use vol_surface::{CalibrationModel, StrikeAxisType};

// Instrument definitions
pub use instrument::{
    InstrumentConventions, InstrumentDefError, InstrumentDefinition, InstrumentTemplate,
};

// Rate index definitions
pub use rate_index::{IndexConventions, RateIndexDefError, RateIndexDefinition};
