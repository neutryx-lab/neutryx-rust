//! Greeks calculation result type.
//!
//! Provides [`GreeksResult<T>`], a generic struct for holding Greeks calculations
//! that is compatible with automatic differentiation via the `Float` trait bound.

use num_traits::Float;

/// Greeks calculation result with optional sensitivities.
///
/// A generic struct that holds the price, standard error, and optional Greeks
/// from a pricing calculation. The generic parameter `T` allows this struct
/// to work with both `f64` for production and AD-compatible types like
/// `Dual<f64>` for automatic differentiation.
///
/// # Type Parameters
///
/// * `T` - A floating-point type implementing `num_traits::Float`. This enables
///   compatibility with automatic differentiation libraries.
///
/// # First-Order Greeks
///
/// - `delta`: ∂V/∂S - Sensitivity to spot price
/// - `vega`: ∂V/∂σ - Sensitivity to volatility
/// - `theta`: ∂V/∂τ - Sensitivity to time (time decay)
/// - `rho`: ∂V/∂r - Sensitivity to interest rate
///
/// # Second-Order Greeks
///
/// - `gamma`: ∂²V/∂S² - Convexity with respect to spot
/// - `vanna`: ∂²V/∂S∂σ - Cross sensitivity (delta-vol)
/// - `volga`: ∂²V/∂σ² - Volatility convexity
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::greeks::GreeksResult;
///
/// // Create a result with first-order Greeks
/// let result = GreeksResult {
///     price: 10.5,
///     std_error: 0.05,
///     delta: Some(0.55),
///     gamma: None,
///     vega: Some(25.0),
///     theta: Some(-0.05),
///     rho: Some(15.0),
///     vanna: None,
///     volga: None,
/// };
///
/// // Access confidence intervals
/// let ci_95 = result.confidence_95();
/// println!("Price: {} ± {}", result.price, ci_95);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct GreeksResult<T: Float> {
    /// Present value of the instrument.
    pub price: T,
    /// Standard error of the Monte Carlo estimate.
    pub std_error: T,

    // First-order Greeks
    /// Delta: ∂V/∂S (sensitivity to spot price).
    pub delta: Option<T>,
    /// Vega: ∂V/∂σ (sensitivity to volatility).
    pub vega: Option<T>,
    /// Theta: ∂V/∂τ (sensitivity to time, time decay).
    pub theta: Option<T>,
    /// Rho: ∂V/∂r (sensitivity to interest rate).
    pub rho: Option<T>,

    // Second-order Greeks
    /// Gamma: ∂²V/∂S² (convexity with respect to spot).
    pub gamma: Option<T>,
    /// Vanna: ∂²V/∂S∂σ (cross sensitivity between spot and volatility).
    pub vanna: Option<T>,
    /// Volga: ∂²V/∂σ² (volatility convexity, also known as vomma).
    pub volga: Option<T>,
}

impl<T: Float> Default for GreeksResult<T> {
    fn default() -> Self {
        Self {
            price: T::zero(),
            std_error: T::zero(),
            delta: None,
            gamma: None,
            vega: None,
            theta: None,
            rho: None,
            vanna: None,
            volga: None,
        }
    }
}

impl<T: Float> GreeksResult<T> {
    /// Returns the 95% confidence interval half-width.
    ///
    /// For a Monte Carlo estimate, the 95% confidence interval is
    /// approximately ±1.96 × standard error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::greeks::GreeksResult;
    ///
    /// let result = GreeksResult::<f64> {
    ///     price: 10.0,
    ///     std_error: 0.1,
    ///     ..Default::default()
    /// };
    ///
    /// let ci = result.confidence_95();
    /// println!("Price: {:.2} ± {:.4}", result.price, ci);
    /// ```
    #[inline]
    pub fn confidence_95(&self) -> T {
        T::from(1.96).unwrap() * self.std_error
    }

    /// Returns the 99% confidence interval half-width.
    ///
    /// For a Monte Carlo estimate, the 99% confidence interval is
    /// approximately ±2.576 × standard error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::greeks::GreeksResult;
    ///
    /// let result = GreeksResult::<f64> {
    ///     price: 10.0,
    ///     std_error: 0.1,
    ///     ..Default::default()
    /// };
    ///
    /// let ci = result.confidence_99();
    /// println!("Price: {:.2} ± {:.4}", result.price, ci);
    /// ```
    #[inline]
    pub fn confidence_99(&self) -> T {
        T::from(2.576).unwrap() * self.std_error
    }

    /// Creates a new result with only price and standard error.
    ///
    /// All Greeks are set to `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::greeks::GreeksResult;
    ///
    /// let result = GreeksResult::new(10.5, 0.05);
    /// assert_eq!(result.price, 10.5);
    /// assert!(result.delta.is_none());
    /// ```
    #[inline]
    pub fn new(price: T, std_error: T) -> Self {
        Self {
            price,
            std_error,
            ..Default::default()
        }
    }

    /// Sets the delta and returns self for method chaining.
    #[inline]
    pub fn with_delta(mut self, delta: T) -> Self {
        self.delta = Some(delta);
        self
    }

    /// Sets the gamma and returns self for method chaining.
    #[inline]
    pub fn with_gamma(mut self, gamma: T) -> Self {
        self.gamma = Some(gamma);
        self
    }

    /// Sets the vega and returns self for method chaining.
    #[inline]
    pub fn with_vega(mut self, vega: T) -> Self {
        self.vega = Some(vega);
        self
    }

    /// Sets the theta and returns self for method chaining.
    #[inline]
    pub fn with_theta(mut self, theta: T) -> Self {
        self.theta = Some(theta);
        self
    }

    /// Sets the rho and returns self for method chaining.
    #[inline]
    pub fn with_rho(mut self, rho: T) -> Self {
        self.rho = Some(rho);
        self
    }

    /// Sets the vanna and returns self for method chaining.
    #[inline]
    pub fn with_vanna(mut self, vanna: T) -> Self {
        self.vanna = Some(vanna);
        self
    }

    /// Sets the volga and returns self for method chaining.
    #[inline]
    pub fn with_volga(mut self, volga: T) -> Self {
        self.volga = Some(volga);
        self
    }

    /// Returns true if any first-order Greek is computed.
    #[inline]
    pub fn has_first_order_greeks(&self) -> bool {
        self.delta.is_some() || self.vega.is_some() || self.theta.is_some() || self.rho.is_some()
    }

    /// Returns true if any second-order Greek is computed.
    #[inline]
    pub fn has_second_order_greeks(&self) -> bool {
        self.gamma.is_some() || self.vanna.is_some() || self.volga.is_some()
    }
}

// Serde support (optional feature)
#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::{Deserialize, Serialize};

    impl<T> Serialize for GreeksResult<T>
    where
        T: Float + Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("GreeksResult", 9)?;
            state.serialize_field("price", &self.price)?;
            state.serialize_field("std_error", &self.std_error)?;
            state.serialize_field("delta", &self.delta)?;
            state.serialize_field("gamma", &self.gamma)?;
            state.serialize_field("vega", &self.vega)?;
            state.serialize_field("theta", &self.theta)?;
            state.serialize_field("rho", &self.rho)?;
            state.serialize_field("vanna", &self.vanna)?;
            state.serialize_field("volga", &self.volga)?;
            state.end()
        }
    }

    impl<'de, T> Deserialize<'de> for GreeksResult<T>
    where
        T: Float + Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            #[derive(Deserialize)]
            struct GreeksResultHelper<T> {
                price: T,
                std_error: T,
                delta: Option<T>,
                gamma: Option<T>,
                vega: Option<T>,
                theta: Option<T>,
                rho: Option<T>,
                vanna: Option<T>,
                volga: Option<T>,
            }

            let helper = GreeksResultHelper::deserialize(deserializer)?;
            Ok(GreeksResult {
                price: helper.price,
                std_error: helper.std_error,
                delta: helper.delta,
                gamma: helper.gamma,
                vega: helper.vega,
                theta: helper.theta,
                rho: helper.rho,
                vanna: helper.vanna,
                volga: helper.volga,
            })
        }
    }
}

// Note: The existing mc::PricingResult is kept for backward compatibility.
// GreeksResult<T> is the new generic type that supports AD.
// Migration from PricingResult to GreeksResult<f64> will be done in Task 3.x.
