//! PyO3 bindings for Neutryx types.

mod bindings;

use pyo3::prelude::*;

/// Register all Python-visible types and functions on the given module.
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<bindings::PyVanillaOption>()?;
    m.add_class::<bindings::PyForward>()?;

    m.add_class::<bindings::PyHullWhite>()?;

    m.add_function(wrap_pyfunction!(bindings::price_black_scholes, m)?)?;
    m.add_function(wrap_pyfunction!(bindings::price_garman_kohlhagen, m)?)?;

    m.add_function(wrap_pyfunction!(version, m)?)?;

    Ok(())
}

/// Get the Neutryx library version.
#[pyfunction]
fn version() -> &'static str { env!("CARGO_PKG_VERSION") }
