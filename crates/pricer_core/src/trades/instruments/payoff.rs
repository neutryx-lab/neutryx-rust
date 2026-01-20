//! Payoff type definitions with smooth approximations.

use num_traits::Float;

use crate::math::smoothing::{smooth_indicator, smooth_max};

/// Type of option payoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PayoffType {
    /// Call option: max(S - K, 0)
    Call,
    /// Put option: max(K - S, 0)
    Put,
    /// Digital call: 1 if S > K else 0
    DigitalCall,
    /// Digital put: 1 if S < K else 0
    DigitalPut,
}

impl PayoffType {
    /// Evaluate the payoff for given spot and strike.
    #[inline]
    pub fn evaluate<T: Float>(&self, spot: T, strike: T, epsilon: T) -> T {
        let zero = T::zero();
        match self {
            PayoffType::Call => smooth_max(spot - strike, zero, epsilon),
            PayoffType::Put => smooth_max(strike - spot, zero, epsilon),
            PayoffType::DigitalCall => smooth_indicator(spot - strike, epsilon),
            PayoffType::DigitalPut => smooth_indicator(strike - spot, epsilon),
        }
    }

    /// Returns whether this payoff is a call-type (Call or DigitalCall).
    #[inline]
    pub fn is_call(&self) -> bool { matches!(self, PayoffType::Call | PayoffType::DigitalCall) }

    /// Returns whether this payoff is a put-type (Put or DigitalPut).
    #[inline]
    pub fn is_put(&self) -> bool { matches!(self, PayoffType::Put | PayoffType::DigitalPut) }

    /// Returns whether this payoff is digital (DigitalCall or DigitalPut).
    #[inline]
    pub fn is_digital(&self) -> bool {
        matches!(self, PayoffType::DigitalCall | PayoffType::DigitalPut)
    }
}
