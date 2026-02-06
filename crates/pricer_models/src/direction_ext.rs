//! Extension traits for direction types.
//!
//! This module provides extension methods for direction types defined in
//! `infra_domain`. The `sign()` method is provided here to avoid adding
//! num_traits dependency to infra_domain.
//!
//! # Examples
//!
//! ```
//! use infra_domain::{TradeDirection, SwapDirection};
//! use pricer_models::{TradeDirectionExt, SwapDirectionExt};
//!
//! let long = TradeDirection::Long;
//! assert_eq!(long.sign::<f64>(), 1.0);
//!
//! let short = TradeDirection::Short;
//! assert_eq!(short.sign::<f64>(), -1.0);
//! ```

use infra_domain::{SwapDirection, TradeDirection};
use num_traits::Float;

/// Extension trait for `TradeDirection` providing numeric sign.
pub trait TradeDirectionExt {
    /// Returns the numeric sign for this direction.
    ///
    /// - `Long` returns `1.0`
    /// - `Short` returns `-1.0`
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::TradeDirection;
    /// use pricer_models::TradeDirectionExt;
    ///
    /// let long = TradeDirection::Long;
    /// assert_eq!(long.sign::<f64>(), 1.0);
    ///
    /// let short = TradeDirection::Short;
    /// assert_eq!(short.sign::<f64>(), -1.0);
    /// ```
    fn sign<F: Float>(&self) -> F;
}

impl TradeDirectionExt for TradeDirection {
    fn sign<F: Float>(&self) -> F {
        match self {
            TradeDirection::Long => F::one(),
            TradeDirection::Short => -F::one(),
        }
    }
}

/// Extension trait for `SwapDirection` providing numeric sign.
pub trait SwapDirectionExt {
    /// Returns the numeric sign for this direction.
    ///
    /// - `ReceiveFixed` returns `1.0` (long fixed leg)
    /// - `PayFixed` returns `-1.0` (short fixed leg)
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::SwapDirection;
    /// use pricer_models::SwapDirectionExt;
    ///
    /// let receive = SwapDirection::ReceiveFixed;
    /// assert_eq!(receive.sign::<f64>(), 1.0);
    ///
    /// let pay = SwapDirection::PayFixed;
    /// assert_eq!(pay.sign::<f64>(), -1.0);
    /// ```
    fn sign<F: Float>(&self) -> F;
}

impl SwapDirectionExt for SwapDirection {
    fn sign<F: Float>(&self) -> F {
        match self {
            SwapDirection::ReceiveFixed => F::one(),
            SwapDirection::PayFixed => -F::one(),
        }
    }
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
        assert_eq!(TradeDirection::Long.sign::<f32>(), 1.0f32);
        assert_eq!(TradeDirection::Short.sign::<f32>(), -1.0f32);
    }

    #[test]
    fn test_swap_direction_sign() {
        assert_eq!(SwapDirection::ReceiveFixed.sign::<f64>(), 1.0);
        assert_eq!(SwapDirection::PayFixed.sign::<f64>(), -1.0);
    }

    #[test]
    fn test_swap_direction_sign_f32() {
        assert_eq!(SwapDirection::ReceiveFixed.sign::<f32>(), 1.0f32);
        assert_eq!(SwapDirection::PayFixed.sign::<f32>(), -1.0f32);
    }

    #[test]
    fn test_consistency_with_trade_direction() {
        // SwapDirection converts to TradeDirection
        let pay_fixed_sign: f64 = SwapDirection::PayFixed.sign();
        let short_sign: f64 = TradeDirection::Short.sign();
        assert_eq!(pay_fixed_sign, short_sign);

        let receive_fixed_sign: f64 = SwapDirection::ReceiveFixed.sign();
        let long_sign: f64 = TradeDirection::Long.sign();
        assert_eq!(receive_fixed_sign, long_sign);
    }
}
