//! PyO3 bindings for Neutryx types.

use pyo3::prelude::*;

/// A vanilla European option.
#[pyclass]
#[derive(Clone)]
pub struct PyVanillaOption {
    /// Strike price.
    #[pyo3(get, set)]
    pub strike: f64,

    /// Time to expiry in years.
    #[pyo3(get, set)]
    pub expiry: f64,

    /// True for call, False for put.
    #[pyo3(get, set)]
    pub is_call: bool,
}

#[pymethods]
impl PyVanillaOption {
    /// Create a new vanilla option.
    #[new]
    pub fn new(strike: f64, expiry: f64, is_call: bool) -> Self {
        Self {
            strike,
            expiry,
            is_call,
        }
    }

    fn __repr__(&self) -> String {
        let option_type = if self.is_call { "Call" } else { "Put" };
        format!(
            "VanillaOption(strike={}, expiry={}, type={})",
            self.strike, self.expiry, option_type
        )
    }
}

/// A forward contract.
#[pyclass]
#[derive(Clone)]
pub struct PyForward {
    /// Strike price (delivery price).
    #[pyo3(get, set)]
    pub strike: f64,

    /// Time to maturity in years.
    #[pyo3(get, set)]
    pub maturity: f64,
}

#[pymethods]
impl PyForward {
    /// Create a new forward contract.
    #[new]
    pub fn new(strike: f64, maturity: f64) -> Self { Self { strike, maturity } }

    fn __repr__(&self) -> String {
        format!(
            "Forward(strike={}, maturity={})",
            self.strike, self.maturity
        )
    }
}

/// Hull-White short rate model.
#[pyclass]
#[derive(Clone)]
pub struct PyHullWhite {
    /// Mean reversion speed (alpha).
    #[pyo3(get, set)]
    pub alpha: f64,

    /// Volatility (sigma).
    #[pyo3(get, set)]
    pub sigma: f64,
}

#[pymethods]
impl PyHullWhite {
    /// Create a new Hull-White model.
    #[new]
    pub fn new(alpha: f64, sigma: f64) -> Self { Self { alpha, sigma } }

    fn __repr__(&self) -> String {
        format!("HullWhite(alpha={}, sigma={})", self.alpha, self.sigma)
    }
}

/// Price a vanilla option using Black-Scholes formula.
#[pyfunction]
#[pyo3(signature = (option, spot, vol, rate, dividend=0.0))]
pub fn price_black_scholes(
    option: &PyVanillaOption,
    spot: f64,
    vol: f64,
    rate: f64,
    dividend: f64,
) -> f64 {
    let _ = dividend;
    let d1 = ((spot / option.strike).ln() + (rate + 0.5 * vol * vol) * option.expiry)
        / (vol * option.expiry.sqrt());
    let d2 = d1 - vol * option.expiry.sqrt();

    let nd1 = normal_cdf(d1);
    let nd2 = normal_cdf(d2);

    if option.is_call {
        spot * nd1 - option.strike * (-rate * option.expiry).exp() * nd2
    } else {
        option.strike * (-rate * option.expiry).exp() * normal_cdf(-d2) - spot * normal_cdf(-d1)
    }
}

/// Price an FX option using Garman-Kohlhagen formula.
#[pyfunction]
pub fn price_garman_kohlhagen(
    option: &PyVanillaOption,
    spot: f64,
    vol: f64,
    domestic_rate: f64,
    foreign_rate: f64,
) -> f64 {
    price_black_scholes(option, spot, vol, domestic_rate, foreign_rate)
}

/// Standard normal CDF approximation (Abramowitz and Stegun).
fn normal_cdf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs() / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    0.5 * (1.0 + sign * y)
}
