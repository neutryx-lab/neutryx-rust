//! Reverse mode automatic differentiation types.
//!
//! This module provides:
//! - `ReverseAD<T>`: Holds all first-order Greeks computed via reverse mode AD
//! - `GammaAD<T>`: Holds second-order derivative (Gamma) via nested AD
//!
//! # Reverse Mode AD
//!
//! Reverse mode AD (adjoint mode) computes gradients by propagating adjoint
//! values from outputs back to inputs. This is efficient when computing
//! derivatives with respect to many inputs (all Greeks in one pass).
//!
//! # Requirements Coverage
//!
//! - Requirement 3.4: spot, rate, vol, time の勾配フィールド
//! - Requirement 3.5: GreeksResult<T> への変換
//! - Requirement 4.2: Gamma 計算 (nested AD or finite difference)
//!
//! # Usage
//!
//! ```rust
//! use pricer_pricing::enzyme::reverse::{ReverseAD, GammaAD};
//!
//! // Compute all first-order Greeks
//! let greeks = ReverseAD::new(
//!     10.45,  // price
//!     0.637,  // delta
//!     55.0,   // rho
//!     37.5,   // vega
//!     -6.41,  // theta
//! );
//!
//! // Compute Gamma
//! let gamma = GammaAD::new(0.0188);
//! ```

use crate::greeks::GreeksResult;
use num_traits::Float;

/// Reverse mode AD result containing all first-order Greeks.
///
/// This struct holds the results of a reverse mode AD pass where all
/// first-order sensitivities are computed simultaneously by propagating
/// adjoint values from the output back through the computation graph.
///
/// # Mathematical Background
///
/// In reverse mode AD:
/// - Set output adjoint to 1.0 (seed)
/// - Propagate adjoints backward: adj(x) = Σ (adj(y) × ∂y/∂x)
/// - Collect adjoints at inputs → gradients
///
/// # Type Parameter
///
/// * `T` - Floating point type (typically `f64`)
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::enzyme::reverse::ReverseAD;
///
/// let greeks = ReverseAD::new(10.45, 0.637, 55.0, 37.5, -6.41);
/// println!("Price: {}, Delta: {}", greeks.price, greeks.delta);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReverseAD<T: Float> {
    /// Option price (primal value)
    pub price: T,
    /// Delta: ∂V/∂S (sensitivity to spot)
    pub delta: T,
    /// Rho: ∂V/∂r (sensitivity to rate)
    pub rho: T,
    /// Vega: ∂V/∂σ (sensitivity to volatility)
    pub vega: T,
    /// Theta: ∂V/∂T (sensitivity to time, typically negative)
    pub theta: T,
}

impl<T: Float> ReverseAD<T> {
    /// Creates a new ReverseAD result with all Greeks.
    ///
    /// # Arguments
    ///
    /// * `price` - Option price
    /// * `delta` - ∂V/∂S
    /// * `rho` - ∂V/∂r
    /// * `vega` - ∂V/∂σ
    /// * `theta` - ∂V/∂T
    #[inline]
    pub fn new(price: T, delta: T, rho: T, vega: T, theta: T) -> Self {
        Self {
            price,
            delta,
            rho,
            vega,
            theta,
        }
    }

    /// Creates a ReverseAD with only price (all Greeks zero).
    #[inline]
    pub fn price_only(price: T) -> Self {
        Self {
            price,
            delta: T::zero(),
            rho: T::zero(),
            vega: T::zero(),
            theta: T::zero(),
        }
    }

    /// Converts to GreeksResult<T> for compatibility with existing API.
    ///
    /// Note: Standard error is set to zero as AD doesn't compute MC error.
    #[inline]
    pub fn to_greeks_result(self) -> GreeksResult<T> {
        GreeksResult {
            price: self.price,
            std_error: T::zero(),
            delta: Some(self.delta),
            vega: Some(self.vega),
            theta: Some(self.theta),
            rho: Some(self.rho),
            gamma: None,
            vanna: None,
            volga: None,
        }
    }
}

impl<T: Float> Default for ReverseAD<T> {
    fn default() -> Self {
        Self::price_only(T::zero())
    }
}

/// Second-order derivative container for Gamma computation.
///
/// Gamma (∂²V/∂S²) measures the rate of change of Delta with respect to
/// spot price. It can be computed via:
/// 1. Nested AD (forward-over-reverse or reverse-over-forward)
/// 2. Finite difference on Delta
///
/// # Requirements Coverage
///
/// - Requirement 4.2: Gamma 計算
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::enzyme::reverse::GammaAD;
///
/// let gamma = GammaAD::new(0.0188);
/// println!("Gamma: {}", gamma.gamma);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GammaAD<T: Float> {
    /// Gamma: ∂²V/∂S² (convexity with respect to spot)
    pub gamma: T,
    /// Vanna: ∂²V/∂S∂σ (cross sensitivity, optional)
    pub vanna: Option<T>,
    /// Volga: ∂²V/∂σ² (volatility convexity, optional)
    pub volga: Option<T>,
}

impl<T: Float> GammaAD<T> {
    /// Creates a new GammaAD with only gamma.
    #[inline]
    pub fn new(gamma: T) -> Self {
        Self {
            gamma,
            vanna: None,
            volga: None,
        }
    }

    /// Creates a GammaAD with all second-order Greeks.
    #[inline]
    pub fn with_all(gamma: T, vanna: T, volga: T) -> Self {
        Self {
            gamma,
            vanna: Some(vanna),
            volga: Some(volga),
        }
    }

    /// Computes Gamma using finite difference on Delta function.
    ///
    /// Gamma ≈ (Delta(S+h) - Delta(S-h)) / (2h)
    ///
    /// # Arguments
    ///
    /// * `delta_fn` - Function that computes Delta for a given spot
    /// * `spot` - Current spot price
    /// * `bump` - Finite difference bump size
    ///
    /// # Type Parameters
    ///
    /// * `F` - Delta function type
    #[inline]
    pub fn from_delta_fd<F>(delta_fn: F, spot: T, bump: T) -> Self
    where
        F: Fn(T) -> T,
    {
        let delta_up = delta_fn(spot + bump);
        let delta_down = delta_fn(spot - bump);
        let two = T::from(2.0).unwrap();
        let gamma = (delta_up - delta_down) / (two * bump);
        Self::new(gamma)
    }
}

impl<T: Float> Default for GammaAD<T> {
    fn default() -> Self {
        Self::new(T::zero())
    }
}

/// Complete Greeks result combining first and second order sensitivities.
///
/// This struct combines ReverseAD (first-order) and GammaAD (second-order)
/// into a single container for comprehensive risk management.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompleteGreeks<T: Float> {
    /// First-order Greeks (Delta, Rho, Vega, Theta)
    pub first_order: ReverseAD<T>,
    /// Second-order Greeks (Gamma, Vanna, Volga)
    pub second_order: GammaAD<T>,
}

impl<T: Float> CompleteGreeks<T> {
    /// Creates a new CompleteGreeks from first and second order results.
    #[inline]
    pub fn new(first_order: ReverseAD<T>, second_order: GammaAD<T>) -> Self {
        Self {
            first_order,
            second_order,
        }
    }

    /// Converts to GreeksResult<T> for compatibility.
    #[inline]
    pub fn to_greeks_result(self) -> GreeksResult<T> {
        GreeksResult {
            price: self.first_order.price,
            std_error: T::zero(),
            delta: Some(self.first_order.delta),
            vega: Some(self.first_order.vega),
            theta: Some(self.first_order.theta),
            rho: Some(self.first_order.rho),
            gamma: Some(self.second_order.gamma),
            vanna: self.second_order.vanna,
            volga: self.second_order.volga,
        }
    }

    /// Convenience accessor for price.
    #[inline]
    pub fn price(&self) -> T {
        self.first_order.price
    }

    /// Convenience accessor for Delta.
    #[inline]
    pub fn delta(&self) -> T {
        self.first_order.delta
    }

    /// Convenience accessor for Gamma.
    #[inline]
    pub fn gamma(&self) -> T {
        self.second_order.gamma
    }
}

impl<T: Float> Default for CompleteGreeks<T> {
    fn default() -> Self {
        Self::new(ReverseAD::default(), GammaAD::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_reverse_ad_creation() {
        let greeks = ReverseAD::new(10.45, 0.637, 55.0, 37.5, -6.41);
        assert_eq!(greeks.price, 10.45);
        assert_eq!(greeks.delta, 0.637);
        assert_eq!(greeks.rho, 55.0);
        assert_eq!(greeks.vega, 37.5);
        assert_eq!(greeks.theta, -6.41);
    }

    #[test]
    fn test_reverse_ad_price_only() {
        let greeks = ReverseAD::price_only(10.0);
        assert_eq!(greeks.price, 10.0);
        assert_eq!(greeks.delta, 0.0);
        assert_eq!(greeks.rho, 0.0);
    }

    #[test]
    fn test_reverse_ad_to_greeks_result() {
        let greeks = ReverseAD::new(10.45, 0.637, 55.0, 37.5, -6.41);
        let result = greeks.to_greeks_result();

        assert_eq!(result.price, 10.45);
        assert_eq!(result.delta, Some(0.637));
        assert_eq!(result.vega, Some(37.5));
        assert_eq!(result.theta, Some(-6.41));
        assert_eq!(result.rho, Some(55.0));
        assert!(result.gamma.is_none());
    }

    #[test]
    fn test_reverse_ad_default() {
        let greeks: ReverseAD<f64> = ReverseAD::default();
        assert_eq!(greeks.price, 0.0);
        assert_eq!(greeks.delta, 0.0);
    }

    #[test]
    fn test_gamma_ad_creation() {
        let gamma = GammaAD::new(0.0188);
        assert_eq!(gamma.gamma, 0.0188);
        assert!(gamma.vanna.is_none());
        assert!(gamma.volga.is_none());
    }

    #[test]
    fn test_gamma_ad_with_all() {
        let gamma = GammaAD::with_all(0.0188, 0.0032, 0.0045);
        assert_eq!(gamma.gamma, 0.0188);
        assert_eq!(gamma.vanna, Some(0.0032));
        assert_eq!(gamma.volga, Some(0.0045));
    }

    #[test]
    fn test_gamma_ad_from_delta_fd() {
        // Delta(S) = S / 100 (linear for testing)
        // Gamma = d(Delta)/dS = 1/100 = 0.01
        let delta_fn = |s: f64| s / 100.0;
        let gamma = GammaAD::from_delta_fd(delta_fn, 100.0, 1.0);

        assert_relative_eq!(gamma.gamma, 0.01, epsilon = 1e-10);
    }

    #[test]
    fn test_gamma_ad_from_quadratic_delta() {
        // If V(S) = S^2, then Delta = 2S, Gamma = 2
        // Delta(S) = 2*S
        let delta_fn = |s: f64| 2.0 * s;
        let gamma = GammaAD::from_delta_fd(delta_fn, 100.0, 1.0);

        assert_relative_eq!(gamma.gamma, 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_gamma_ad_default() {
        let gamma: GammaAD<f64> = GammaAD::default();
        assert_eq!(gamma.gamma, 0.0);
    }

    #[test]
    fn test_complete_greeks_creation() {
        let first = ReverseAD::new(10.45, 0.637, 55.0, 37.5, -6.41);
        let second = GammaAD::new(0.0188);
        let complete = CompleteGreeks::new(first, second);

        assert_eq!(complete.price(), 10.45);
        assert_eq!(complete.delta(), 0.637);
        assert_eq!(complete.gamma(), 0.0188);
    }

    #[test]
    fn test_complete_greeks_to_greeks_result() {
        let first = ReverseAD::new(10.45, 0.637, 55.0, 37.5, -6.41);
        let second = GammaAD::with_all(0.0188, 0.0032, 0.0045);
        let complete = CompleteGreeks::new(first, second);
        let result = complete.to_greeks_result();

        assert_eq!(result.price, 10.45);
        assert_eq!(result.delta, Some(0.637));
        assert_eq!(result.gamma, Some(0.0188));
        assert_eq!(result.vanna, Some(0.0032));
        assert_eq!(result.volga, Some(0.0045));
    }

    #[test]
    fn test_complete_greeks_default() {
        let complete: CompleteGreeks<f64> = CompleteGreeks::default();
        assert_eq!(complete.price(), 0.0);
        assert_eq!(complete.delta(), 0.0);
        assert_eq!(complete.gamma(), 0.0);
    }

    #[test]
    fn test_reverse_ad_copy() {
        let greeks = ReverseAD::new(10.0, 0.5, 50.0, 30.0, -5.0);
        let copied = greeks;
        assert_eq!(greeks.price, copied.price);
    }

    #[test]
    fn test_gamma_ad_copy() {
        let gamma = GammaAD::new(0.02);
        let copied = gamma;
        assert_eq!(gamma.gamma, copied.gamma);
    }
}
