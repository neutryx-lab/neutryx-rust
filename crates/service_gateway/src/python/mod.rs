//! PyO3 bindings for Neutryx types
//!
//! Exposes Rust pricing primitives as Python classes and functions
//! for research and Jupyter notebook workflows.

mod bindings;

use pyo3::prelude::*;

/// Register all Python-visible types and functions on the given module.
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Instrument types
    m.add_class::<bindings::PyVanillaOption>()?;
    m.add_class::<bindings::PyForward>()?;

    // Model types
    m.add_class::<bindings::PyHullWhite>()?;

    // Pricing functions
    m.add_function(wrap_pyfunction!(bindings::price_black_scholes, m)?)?;
    m.add_function(wrap_pyfunction!(bindings::price_garman_kohlhagen, m)?)?;

    // Utility functions
    m.add_function(wrap_pyfunction!(version, m)?)?;

    Ok(())
}

/// Get the Neutryx library version
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
