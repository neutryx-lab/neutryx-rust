//! Greeks calculation result type.

use num_traits::Float;
use serde::{Deserialize, Serialize};

#[inline]
fn from_f64<T: Float>(value: f64) -> T { T::from(value).unwrap_or_else(|| T::zero()) }

/// Generate `with_*` builder methods for Greek fields.
macro_rules! greek_builder {
    ($($method:ident => $field:ident),* $(,)?) => {
        $(
            #[inline]
            pub fn $method(mut self, value: T) -> Self {
                self.$field = Some(value);
                self
            }
        )*
    };
}

/// Greeks calculation result with optional sensitivities, generic over Float
/// for AD compatibility.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Float + Serialize",
    deserialize = "T: Float + Deserialize<'de>"
))]
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

    greek_builder!(
        with_delta => delta,
        with_gamma => gamma,
        with_vega => vega,
        with_theta => theta,
        with_rho => rho,
        with_vanna => vanna,
        with_volga => volga,
    );

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
