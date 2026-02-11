//! Neutryx Service Gateway.

pub mod config;
pub mod error;
pub mod rest;
pub mod services;
pub mod state;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "python")]
pub mod python;

pub use error::ServerError;
#[cfg(feature = "python")]
use pyo3::prelude::*;
pub use rest::{GraphAppState, WsAppState};
pub use state::AppState;

/// PyO3 module entry point for the `neutryx_py` Python package.
#[cfg(feature = "python")]
#[pymodule]
fn neutryx_py(m: &Bound<'_, PyModule>) -> PyResult<()> { python::register_module(m) }
