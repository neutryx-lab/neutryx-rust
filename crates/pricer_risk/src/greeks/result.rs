//! Greeks calculation result type.

use num_traits::Float;

#[inline]
fn from_f64<T: Float>(value: f64) -> T { T::from(value).unwrap_or_else(|| T::zero()) }

/// Greeks calculation result with optional sensitivities, generic over Float
/// for AD compatibility.
#[derive(Clone, Debug, PartialEq)]
pub struct GreeksResult<T: Float> {
    /// Calculated price.
    pub price: T,
    /// Standard error of the price estimate.
    pub std_error: T,
    /// Delta sensitivity.
    pub delta: Option<T>,
    /// Vega sensitivity.
    pub vega: Option<T>,
    /// Theta sensitivity.
    pub theta: Option<T>,
    /// Rho sensitivity.
    pub rho: Option<T>,
    /// Gamma sensitivity.
    pub gamma: Option<T>,
    /// Vanna (cross-gamma) sensitivity.
    pub vanna: Option<T>,
    /// Volga (vol-of-vol) sensitivity.
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
    /// Returns the 95% confidence interval half-width (1.96 * std_error).
    #[inline]
    pub fn confidence_95(&self) -> T { from_f64::<T>(1.96) * self.std_error }

    /// Returns the 99% confidence interval half-width (2.576 * std_error).
    #[inline]
    pub fn confidence_99(&self) -> T { from_f64::<T>(2.576) * self.std_error }

    /// Creates a new result with only price and standard error (all Greeks
    /// None).
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

mod serde_impl {
    use serde::{Deserialize, Serialize};

    use super::*;

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
