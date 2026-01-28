//! Volatility market data types and conventions.
//!
//! This module provides types for volatility surface specification:
//!
//! - [`VolQuoteType`]: How volatility is quoted (Normal, Lognormal, Shifted)
//! - [`StrikeType`]: Strike convention (Absolute, Relative, Moneyness, Delta)
//! - [`CalibrationModel`]: Parametric model for fitting (SABR, SVI, LocalVol)
//! - [`StrikeAxisType`]: Strike axis for visualisation
//!
//! # Examples
//!
//! ```
//! use infra_master::market::volatility::{VolQuoteType, StrikeType, CalibrationModel};
//!
//! let vol_type = VolQuoteType::Normal;
//! assert_eq!(vol_type.unit(), "bp");
//!
//! let strike_type = StrikeType::Delta;
//! assert!(strike_type.requires_forward());
//!
//! let model = CalibrationModel::Sabr;
//! assert!(model.is_enabled());
//! assert_eq!(model.parameter_count(), 4);
//! ```

mod calibration_model;
mod strike_type;
mod vol_quote_type;

pub use calibration_model::{CalibrationModel, StrikeAxisType};
pub use strike_type::StrikeType;
pub use vol_quote_type::VolQuoteType;
