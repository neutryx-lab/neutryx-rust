//! Swaption (option on interest rate swap) implementation.
//!
//! This module provides the [`Swaption`] structure for options on
//! interest rate swaps.
//!
//! # Types
//!
//! - **Payer Swaption**: Right to enter a payer swap (pay fixed, receive floating)
//! - **Receiver Swaption**: Right to enter a receiver swap (receive fixed, pay floating)
//!
//! # Exercise Styles
//!
//! - **European**: Exercise only at expiry
//! - **Bermudan**: Exercise at specific dates before expiry
//!
//! # Example
//!
//! ```
//! use pricer_models::instruments::rates::{
//!     Swaption, SwaptionType, SwaptionStyle,
//!     InterestRateSwap, FixedLeg, FloatingLeg, RateIndex, SwapDirection,
//! };
//! use pricer_models::schedules::{ScheduleBuilder, Frequency};
//! use pricer_core::types::{Currency, time::{Date, DayCountConvention}};
//!
//! let start = Date::from_ymd(2024, 1, 15).unwrap();
//! let end = Date::from_ymd(2029, 1, 15).unwrap();
//!
//! let schedule = ScheduleBuilder::new()
//!     .start(start)
//!     .end(end)
//!     .frequency(Frequency::SemiAnnual)
//!     .day_count(DayCountConvention::Thirty360)
//!     .build()
//!     .unwrap();
//!
//! let fixed_leg = FixedLeg::new(schedule.clone(), 0.03, DayCountConvention::Thirty360);
//! let floating_leg = FloatingLeg::new(
//!     schedule,
//!     0.0,
//!     RateIndex::Sofr,
//!     DayCountConvention::ActualActual360,
//! );
//!
//! let underlying = InterestRateSwap::new(
//!     1_000_000.0,
//!     fixed_leg,
//!     floating_leg,
//!     Currency::USD,
//!     SwapDirection::PayFixed,
//! );
//!
//! let swaption = Swaption::new(
//!     underlying,
//!     1.0,  // 1 year to expiry
//!     0.03, // strike rate
//!     SwaptionType::Payer,
//!     SwaptionStyle::European,
//! );
//!
//! assert_eq!(swaption.swaption_type(), SwaptionType::Payer);
//! ```

use num_traits::Float;
use pricer_core::types::Currency;
use std::fmt;

use super::InterestRateSwap;

/// Swaption type (payer or receiver).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwaptionType {
    /// Right to enter a payer swap (pay fixed, receive floating).
    Payer,
    /// Right to enter a receiver swap (receive fixed, pay floating).
    Receiver,
}

impl fmt::Display for SwaptionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwaptionType::Payer => write!(f, "Payer"),
            SwaptionType::Receiver => write!(f, "Receiver"),
        }
    }
}

/// Swaption exercise style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwaptionStyle {
    /// European: exercise only at expiry.
    European,
    /// Bermudan: exercise at specific dates.
    Bermudan,
}

impl fmt::Display for SwaptionStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwaptionStyle::European => write!(f, "European"),
            SwaptionStyle::Bermudan => write!(f, "Bermudan"),
        }
    }
}

/// Swaption (option on interest rate swap).
///
/// A swaption gives the holder the right, but not the obligation,
/// to enter into an interest rate swap at a predetermined rate
/// (the strike rate) on or before the expiry date.
#[derive(Debug, Clone)]
pub struct Swaption<T: Float> {
    /// Underlying swap that will be entered upon exercise.
    underlying: InterestRateSwap<T>,
    /// Time to expiry in years.
    expiry_time: T,
    /// Strike rate (the fixed rate of the underlying swap).
    strike: T,
    /// Swaption type (payer or receiver).
    swaption_type: SwaptionType,
    /// Exercise style.
    style: SwaptionStyle,
}

impl<T: Float> Swaption<T> {
    /// Create a new swaption.
    ///
    /// # Arguments
    ///
    /// * `underlying` - The underlying swap
    /// * `expiry_time` - Time to expiry in years
    /// * `strike` - Strike rate (fixed rate upon exercise)
    /// * `swaption_type` - Payer or receiver
    /// * `style` - European or Bermudan
    pub fn new(
        underlying: InterestRateSwap<T>,
        expiry_time: T,
        strike: T,
        swaption_type: SwaptionType,
        style: SwaptionStyle,
    ) -> Self {
        Self {
            underlying,
            expiry_time,
            strike,
            swaption_type,
            style,
        }
    }

    /// Returns the underlying swap.
    #[inline]
    pub fn underlying(&self) -> &InterestRateSwap<T> {
        &self.underlying
    }

    /// Returns the time to expiry in years.
    #[inline]
    pub fn expiry(&self) -> T {
        self.expiry_time
    }

    /// Returns the strike rate.
    #[inline]
    pub fn strike(&self) -> T {
        self.strike
    }

    /// Returns the swaption type.
    #[inline]
    pub fn swaption_type(&self) -> SwaptionType {
        self.swaption_type
    }

    /// Returns the exercise style.
    #[inline]
    pub fn style(&self) -> SwaptionStyle {
        self.style
    }

    /// Returns the settlement currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        self.underlying.currency()
    }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> T {
        self.underlying.notional()
    }

    /// Returns whether this is a payer swaption.
    #[inline]
    pub fn is_payer(&self) -> bool {
        self.swaption_type == SwaptionType::Payer
    }

    /// Returns whether this is a receiver swaption.
    #[inline]
    pub fn is_receiver(&self) -> bool {
        self.swaption_type == SwaptionType::Receiver
    }

    /// Returns whether this is a European swaption.
    #[inline]
    pub fn is_european(&self) -> bool {
        self.style == SwaptionStyle::European
    }

    /// Returns the swap tenor (length of the underlying swap).
    pub fn swap_tenor(&self) -> T {
        self.underlying.maturity() - self.expiry_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::rates::{FixedLeg, FloatingLeg, RateIndex, SwapDirection};
    use crate::schedules::{Frequency, ScheduleBuilder};
    use pricer_core::types::time::{Date, DayCountConvention};

    fn create_test_swap() -> InterestRateSwap<f64> {
        let start = Date::from_ymd(2024, 1, 15).unwrap();
        let end = Date::from_ymd(2029, 1, 15).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::SemiAnnual)
            .day_count(DayCountConvention::Thirty360)
            .build()
            .unwrap();

        let fixed_leg = FixedLeg::new(schedule.clone(), 0.03, DayCountConvention::Thirty360);
        let floating_leg = FloatingLeg::new(
            schedule,
            0.0,
            RateIndex::Sofr,
            DayCountConvention::ActualActual360,
        );

        InterestRateSwap::new(
            1_000_000.0,
            fixed_leg,
            floating_leg,
            Currency::USD,
            SwapDirection::PayFixed,
        )
    }

    #[test]
    fn test_swaption_type_display() {
        assert_eq!(format!("{}", SwaptionType::Payer), "Payer");
        assert_eq!(format!("{}", SwaptionType::Receiver), "Receiver");
    }

    #[test]
    fn test_swaption_style_display() {
        assert_eq!(format!("{}", SwaptionStyle::European), "European");
        assert_eq!(format!("{}", SwaptionStyle::Bermudan), "Bermudan");
    }

    #[test]
    fn test_swaption_new() {
        let underlying = create_test_swap();
        let swaption = Swaption::new(
            underlying,
            1.0,
            0.03,
            SwaptionType::Payer,
            SwaptionStyle::European,
        );

        assert!((swaption.expiry() - 1.0).abs() < 1e-10);
        assert!((swaption.strike() - 0.03).abs() < 1e-10);
        assert_eq!(swaption.swaption_type(), SwaptionType::Payer);
        assert_eq!(swaption.style(), SwaptionStyle::European);
    }

    #[test]
    fn test_swaption_accessors() {
        let underlying = create_test_swap();
        let swaption = Swaption::new(
            underlying,
            1.0,
            0.03,
            SwaptionType::Payer,
            SwaptionStyle::European,
        );

        assert_eq!(swaption.currency(), Currency::USD);
        assert!((swaption.notional() - 1_000_000.0).abs() < 1e-6);
        assert!(swaption.is_payer());
        assert!(!swaption.is_receiver());
        assert!(swaption.is_european());
    }

    #[test]
    fn test_swaption_receiver() {
        let underlying = create_test_swap();
        let swaption = Swaption::new(
            underlying,
            1.0,
            0.03,
            SwaptionType::Receiver,
            SwaptionStyle::European,
        );

        assert!(!swaption.is_payer());
        assert!(swaption.is_receiver());
    }

    #[test]
    fn test_swaption_swap_tenor() {
        let underlying = create_test_swap();
        let swaption = Swaption::new(
            underlying,
            1.0,
            0.03,
            SwaptionType::Payer,
            SwaptionStyle::European,
        );

        // Underlying swap is ~5 years, expiry is 1 year
        // Swap tenor should be ~4 years
        let tenor = swaption.swap_tenor();
        assert!(tenor > 3.0 && tenor < 5.0);
    }

    #[test]
    fn test_swaption_clone() {
        let underlying = create_test_swap();
        let swaption1 = Swaption::new(
            underlying,
            1.0,
            0.03,
            SwaptionType::Payer,
            SwaptionStyle::European,
        );
        let swaption2 = swaption1.clone();

        assert!((swaption1.expiry() - swaption2.expiry()).abs() < 1e-10);
        assert_eq!(swaption1.swaption_type(), swaption2.swaption_type());
    }
}
