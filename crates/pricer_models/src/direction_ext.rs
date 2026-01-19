//! Extension traits for direction types.
//!
//! This module provides `sign()` methods and other calculation utilities
//! for [`TradeDirection`] and [`SwapDirection`] types from `infra_master`.
//!
//! The `sign()` method is provided here (rather than in infra_master) to:
//! 1. Avoid adding `num-traits` dependency to infra_master
//! 2. Keep infra_master focused on data definitions
//! 3. Enable AD-compatible generic implementations
//!
//! # Examples
//!
//! ```
//! use pricer_models::{TradeDirection, TradeDirectionExt};
//!
//! let long = TradeDirection::Long;
//! let short = TradeDirection::Short;
//!
//! assert_eq!(long.sign::<f64>(), 1.0);
//! assert_eq!(short.sign::<f64>(), -1.0);
//! ```

use infra_master::{SwapDirection, TradeDirection};
use num_traits::Float;

/// Extension trait for [`TradeDirection`] providing calculation methods.
///
/// # Examples
///
/// ```
/// use pricer_models::{TradeDirection, TradeDirectionExt};
///
/// let direction = TradeDirection::Long;
/// let notional = 1_000_000.0_f64;
/// let signed_notional = direction.sign::<f64>() * notional;
/// assert_eq!(signed_notional, 1_000_000.0);
/// ```
pub trait TradeDirectionExt {
    /// Returns the sign multiplier for this direction.
    ///
    /// - `Long` returns `+1.0`
    /// - `Short` returns `-1.0`
    ///
    /// # Type Parameters
    ///
    /// * `T` - A floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::{TradeDirection, TradeDirectionExt};
    ///
    /// assert_eq!(TradeDirection::Long.sign::<f64>(), 1.0);
    /// assert_eq!(TradeDirection::Short.sign::<f64>(), -1.0);
    ///
    /// // Works with f32 too
    /// assert_eq!(TradeDirection::Long.sign::<f32>(), 1.0_f32);
    /// ```
    fn sign<T: Float>(&self) -> T;

    /// Returns whether this direction is long.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::{TradeDirection, TradeDirectionExt};
    ///
    /// assert!(TradeDirection::Long.is_long());
    /// assert!(!TradeDirection::Short.is_long());
    /// ```
    fn is_long(&self) -> bool;

    /// Returns whether this direction is short.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::{TradeDirection, TradeDirectionExt};
    ///
    /// assert!(!TradeDirection::Long.is_short());
    /// assert!(TradeDirection::Short.is_short());
    /// ```
    fn is_short(&self) -> bool;
}

impl TradeDirectionExt for TradeDirection {
    #[inline]
    fn sign<T: Float>(&self) -> T {
        match self {
            TradeDirection::Long => T::one(),
            TradeDirection::Short => -T::one(),
        }
    }

    #[inline]
    fn is_long(&self) -> bool { *self == TradeDirection::Long }

    #[inline]
    fn is_short(&self) -> bool { *self == TradeDirection::Short }
}

/// Extension trait for [`SwapDirection`] providing calculation methods.
///
/// # Examples
///
/// ```
/// use pricer_models::{SwapDirection, SwapDirectionExt};
///
/// let pay_fixed = SwapDirection::PayFixed;
/// assert_eq!(pay_fixed.fixed_leg_sign::<f64>(), -1.0);
/// assert_eq!(pay_fixed.floating_leg_sign::<f64>(), 1.0);
/// ```
pub trait SwapDirectionExt {
    /// Returns the sign multiplier for the fixed leg.
    ///
    /// - `PayFixed` returns `-1.0` (paying fixed)
    /// - `ReceiveFixed` returns `+1.0` (receiving fixed)
    ///
    /// # Type Parameters
    ///
    /// * `T` - A floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::{SwapDirection, SwapDirectionExt};
    ///
    /// assert_eq!(SwapDirection::PayFixed.fixed_leg_sign::<f64>(), -1.0);
    /// assert_eq!(SwapDirection::ReceiveFixed.fixed_leg_sign::<f64>(), 1.0);
    /// ```
    fn fixed_leg_sign<T: Float>(&self) -> T;

    /// Returns the sign multiplier for the floating leg.
    ///
    /// - `PayFixed` returns `+1.0` (receiving floating)
    /// - `ReceiveFixed` returns `-1.0` (paying floating)
    ///
    /// # Type Parameters
    ///
    /// * `T` - A floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::{SwapDirection, SwapDirectionExt};
    ///
    /// assert_eq!(SwapDirection::PayFixed.floating_leg_sign::<f64>(), 1.0);
    /// assert_eq!(SwapDirection::ReceiveFixed.floating_leg_sign::<f64>(), -1.0);
    /// ```
    fn floating_leg_sign<T: Float>(&self) -> T;

    /// Returns whether this direction is pay-fixed.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::{SwapDirection, SwapDirectionExt};
    ///
    /// assert!(SwapDirection::PayFixed.is_pay_fixed());
    /// assert!(!SwapDirection::ReceiveFixed.is_pay_fixed());
    /// ```
    fn is_pay_fixed(&self) -> bool;

    /// Returns whether this direction is receive-fixed.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::{SwapDirection, SwapDirectionExt};
    ///
    /// assert!(!SwapDirection::PayFixed.is_receive_fixed());
    /// assert!(SwapDirection::ReceiveFixed.is_receive_fixed());
    /// ```
    fn is_receive_fixed(&self) -> bool;
}

impl SwapDirectionExt for SwapDirection {
    #[inline]
    fn fixed_leg_sign<T: Float>(&self) -> T {
        match self {
            SwapDirection::PayFixed => -T::one(),
            SwapDirection::ReceiveFixed => T::one(),
        }
    }

    #[inline]
    fn floating_leg_sign<T: Float>(&self) -> T {
        match self {
            SwapDirection::PayFixed => T::one(),
            SwapDirection::ReceiveFixed => -T::one(),
        }
    }

    #[inline]
    fn is_pay_fixed(&self) -> bool { *self == SwapDirection::PayFixed }

    #[inline]
    fn is_receive_fixed(&self) -> bool { *self == SwapDirection::ReceiveFixed }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_direction_sign() {
        assert_eq!(TradeDirection::Long.sign::<f64>(), 1.0);
        assert_eq!(TradeDirection::Short.sign::<f64>(), -1.0);
    }

    #[test]
    fn test_trade_direction_sign_f32() {
        assert_eq!(TradeDirection::Long.sign::<f32>(), 1.0_f32);
        assert_eq!(TradeDirection::Short.sign::<f32>(), -1.0_f32);
    }

    #[test]
    fn test_trade_direction_is_long_short() {
        assert!(TradeDirection::Long.is_long());
        assert!(!TradeDirection::Long.is_short());
        assert!(!TradeDirection::Short.is_long());
        assert!(TradeDirection::Short.is_short());
    }

    #[test]
    fn test_swap_direction_fixed_leg_sign() {
        assert_eq!(SwapDirection::PayFixed.fixed_leg_sign::<f64>(), -1.0);
        assert_eq!(SwapDirection::ReceiveFixed.fixed_leg_sign::<f64>(), 1.0);
    }

    #[test]
    fn test_swap_direction_floating_leg_sign() {
        assert_eq!(SwapDirection::PayFixed.floating_leg_sign::<f64>(), 1.0);
        assert_eq!(SwapDirection::ReceiveFixed.floating_leg_sign::<f64>(), -1.0);
    }

    #[test]
    fn test_swap_direction_is_pay_receive() {
        assert!(SwapDirection::PayFixed.is_pay_fixed());
        assert!(!SwapDirection::PayFixed.is_receive_fixed());
        assert!(!SwapDirection::ReceiveFixed.is_pay_fixed());
        assert!(SwapDirection::ReceiveFixed.is_receive_fixed());
    }

    #[test]
    fn test_swap_direction_signs_opposite() {
        // Fixed and floating leg signs should always be opposite
        for dir in [SwapDirection::PayFixed, SwapDirection::ReceiveFixed] {
            let fixed_sign: f64 = dir.fixed_leg_sign();
            let floating_sign: f64 = dir.floating_leg_sign();
            assert_eq!(fixed_sign + floating_sign, 0.0);
        }
    }

    #[test]
    fn test_trade_direction_with_notional() {
        let notional = 1_000_000.0_f64;

        let long_value = TradeDirection::Long.sign::<f64>() * notional;
        assert_eq!(long_value, 1_000_000.0);

        let short_value = TradeDirection::Short.sign::<f64>() * notional;
        assert_eq!(short_value, -1_000_000.0);
    }
}
