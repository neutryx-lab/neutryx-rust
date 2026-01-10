//! Calibration target types.
//!
//! This module defines types for calibration targets - the market
//! instruments used to calibrate model parameters.

use pricer_core::traits::Float;

/// Option calibration target.
///
/// Represents a vanilla option used for calibration.
#[derive(Clone, Debug)]
pub struct OptionTarget<T: Float> {
    /// Strike price
    pub strike: T,
    /// Time to expiry (years)
    pub expiry: T,
    /// Market implied volatility or price
    pub market_value: T,
    /// Weight for this target in calibration
    pub weight: T,
    /// Whether market_value is price (true) or implied vol (false)
    pub is_price: bool,
}

impl<T: Float> OptionTarget<T> {
    /// Create a new option target with implied volatility.
    pub fn from_implied_vol(strike: T, expiry: T, implied_vol: T) -> Self {
        Self {
            strike,
            expiry,
            market_value: implied_vol,
            weight: T::one(),
            is_price: false,
        }
    }

    /// Create a new option target with market price.
    pub fn from_price(strike: T, expiry: T, price: T) -> Self {
        Self {
            strike,
            expiry,
            market_value: price,
            weight: T::one(),
            is_price: true,
        }
    }

    /// Set the weight for this target.
    pub fn with_weight(mut self, weight: T) -> Self {
        self.weight = weight;
        self
    }

    /// Get the moneyness (strike / forward).
    pub fn moneyness(&self, forward: T) -> T {
        self.strike / forward
    }

    /// Get the log-moneyness.
    pub fn log_moneyness(&self, forward: T) -> T {
        (self.strike / forward).ln()
    }
}

/// Swaption calibration target.
///
/// Represents a swaption used for interest rate model calibration.
#[derive(Clone, Debug)]
pub struct SwaptionTarget<T: Float> {
    /// Option expiry (years)
    pub expiry: T,
    /// Underlying swap tenor (years)
    pub tenor: T,
    /// Strike rate
    pub strike: T,
    /// Market implied volatility (normal or lognormal)
    pub market_vol: T,
    /// Weight for this target
    pub weight: T,
    /// Whether vol is normal (true) or lognormal (false)
    pub is_normal_vol: bool,
}

impl<T: Float> SwaptionTarget<T> {
    /// Create a new swaption target with lognormal volatility.
    pub fn from_lognormal_vol(expiry: T, tenor: T, strike: T, vol: T) -> Self {
        Self {
            expiry,
            tenor,
            strike,
            market_vol: vol,
            weight: T::one(),
            is_normal_vol: false,
        }
    }

    /// Create a new swaption target with normal volatility.
    pub fn from_normal_vol(expiry: T, tenor: T, strike: T, vol: T) -> Self {
        Self {
            expiry,
            tenor,
            strike,
            market_vol: vol,
            weight: T::one(),
            is_normal_vol: true,
        }
    }

    /// Set the weight for this target.
    pub fn with_weight(mut self, weight: T) -> Self {
        self.weight = weight;
        self
    }
}

/// Generic calibration target.
///
/// Wrapper enum for different target types used in calibration.
#[derive(Clone, Debug)]
pub enum CalibrationTarget<T: Float> {
    /// Vanilla option target (for SABR, Heston, etc.)
    Option(OptionTarget<T>),
    /// Swaption target (for Hull-White, etc.)
    Swaption(SwaptionTarget<T>),
    /// Custom target with raw data
    Custom {
        /// Identifier for this target
        id: String,
        /// Input data (e.g., strike, expiry)
        inputs: Vec<T>,
        /// Market value to match
        market_value: T,
        /// Weight
        weight: T,
    },
}

impl<T: Float> CalibrationTarget<T> {
    /// Get the weight of this target.
    pub fn weight(&self) -> T {
        match self {
            CalibrationTarget::Option(opt) => opt.weight,
            CalibrationTarget::Swaption(swp) => swp.weight,
            CalibrationTarget::Custom { weight, .. } => *weight,
        }
    }

    /// Get the market value of this target.
    pub fn market_value(&self) -> T {
        match self {
            CalibrationTarget::Option(opt) => opt.market_value,
            CalibrationTarget::Swaption(swp) => swp.market_vol,
            CalibrationTarget::Custom { market_value, .. } => *market_value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_target_from_implied_vol() {
        let target = OptionTarget::from_implied_vol(100.0_f64, 1.0, 0.2);
        assert_eq!(target.strike, 100.0);
        assert_eq!(target.expiry, 1.0);
        assert_eq!(target.market_value, 0.2);
        assert!(!target.is_price);
    }

    #[test]
    fn test_option_target_moneyness() {
        let target = OptionTarget::from_implied_vol(110.0_f64, 1.0, 0.2);
        let moneyness = target.moneyness(100.0);
        assert!((moneyness - 1.1).abs() < 1e-10);
    }

    #[test]
    fn test_swaption_target() {
        let target = SwaptionTarget::from_lognormal_vol(1.0_f64, 5.0, 0.03, 0.15);
        assert_eq!(target.expiry, 1.0);
        assert_eq!(target.tenor, 5.0);
        assert_eq!(target.market_vol, 0.15);
        assert!(!target.is_normal_vol);
    }

    #[test]
    fn test_calibration_target_weight() {
        let opt_target: CalibrationTarget<f64> =
            CalibrationTarget::Option(OptionTarget::from_implied_vol(100.0, 1.0, 0.2).with_weight(2.0));
        assert_eq!(opt_target.weight(), 2.0);
    }
}
