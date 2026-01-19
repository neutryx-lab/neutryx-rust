//! Neutryx Python Bindings
//!
//! This crate provides PyO3 bindings for the Neutryx XVA pricing library,
//! enabling research and notebook-based workflows in Python/Jupyter.
//!
//! # Architecture
//!
//! As part of the **S**ervice layer in the A-I-P-S architecture, this crate
//! exposes Rust functionality to Python for research and validation purposes.
//!
//! # Usage
//!
//! ```python
//! import neutryx
//!
//! # Create a vanilla option
//! option = neutryx.VanillaOption(
//!     strike=100.0,
//!     expiry=1.0,
//!     is_call=True
//! )
//!
//! # Price using Black-Scholes
//! price = neutryx.price_black_scholes(option, spot=100.0, vol=0.2, rate=0.05)
//! print(f"Option price: {price}")
//! ```

use pyo3::prelude::*;

mod bindings;

/// Neutryx XVA Pricing Library for Python
///
/// A high-performance derivatives pricing library with automatic
/// differentiation for Greeks computation.
#[pymodule]
fn neutryx(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register instrument types
    m.add_class::<bindings::PyVanillaOption>()?;
    m.add_class::<bindings::PyForward>()?;

    // Register model types
    m.add_class::<bindings::PyHullWhite>()?;

    // Register pricing functions
    m.add_function(wrap_pyfunction!(bindings::price_black_scholes, m)?)?;
    m.add_function(wrap_pyfunction!(bindings::price_garman_kohlhagen, m)?)?;

    // Register utility functions
    m.add_function(wrap_pyfunction!(version, m)?)?;

    Ok(())
}

/// Get the Neutryx library version
#[pyfunction]
fn version() -> &'static str { env!("CARGO_PKG_VERSION") }
