//! Pricing instrument definitions.
//!
//! This module provides types for financial instruments used in pricing
//! and risk calculations. These are distinct from market instruments
//! used for curve calibration.

use std::fmt;

use super::payoff::OptionType;

/// Alias for OptionType to maintain compatibility with pricing code.
pub type PayoffType = OptionType;

/// Exercise style for options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExerciseStyle {
    /// European option (exercise only at expiry).
    European,
    /// American option (exercise any time before expiry).
    American,
    /// Bermudan option (exercise on specified dates).
    Bermudan,
    /// Asian option (payoff depends on average price).
    Asian,
}

impl ExerciseStyle {
    /// Returns true if this is European exercise.
    #[inline]
    #[must_use]
    pub fn is_european(&self) -> bool { matches!(self, ExerciseStyle::European) }

    /// Returns true if this is American exercise.
    #[inline]
    #[must_use]
    pub fn is_american(&self) -> bool { matches!(self, ExerciseStyle::American) }

    /// Returns true if this is Bermudan exercise.
    #[inline]
    #[must_use]
    pub fn is_bermudan(&self) -> bool { matches!(self, ExerciseStyle::Bermudan) }

    /// Returns true if this is Asian exercise.
    #[inline]
    #[must_use]
    pub fn is_asian(&self) -> bool { matches!(self, ExerciseStyle::Asian) }
}

impl fmt::Display for ExerciseStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExerciseStyle::European => write!(f, "European"),
            ExerciseStyle::American => write!(f, "American"),
            ExerciseStyle::Bermudan => write!(f, "Bermudan"),
            ExerciseStyle::Asian => write!(f, "Asian"),
        }
    }
}

/// Direction for forwards and swaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ForwardDirection {
    /// Long position (buyer).
    Long,
    /// Short position (seller).
    Short,
}

impl ForwardDirection {
    /// Returns the sign of the direction (+1 for Long, -1 for Short).
    #[must_use]
    pub fn sign(&self) -> f64 {
        match self {
            ForwardDirection::Long => 1.0,
            ForwardDirection::Short => -1.0,
        }
    }
}

impl fmt::Display for ForwardDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ForwardDirection::Long => write!(f, "Long"),
            ForwardDirection::Short => write!(f, "Short"),
        }
    }
}

/// Parameters for instrument pricing.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InstrumentParams<T> {
    strike: T,
    expiry: T,
    notional: T,
}

impl<T: Copy> InstrumentParams<T> {
    /// Creates new instrument parameters.
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price
    /// * `expiry` - Time to expiry in years
    /// * `notional` - Notional amount
    #[must_use]
    pub fn new(strike: T, expiry: T, notional: T) -> Result<Self, &'static str> {
        Ok(Self {
            strike,
            expiry,
            notional,
        })
    }

    /// Returns the strike price.
    #[must_use]
    pub fn strike(&self) -> T { self.strike }

    /// Returns the time to expiry.
    #[must_use]
    pub fn expiry(&self) -> T { self.expiry }

    /// Returns the notional amount.
    #[must_use]
    pub fn notional(&self) -> T { self.notional }
}

/// Vanilla option instrument.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VanillaOption<T> {
    params: InstrumentParams<T>,
    payoff_type: PayoffType,
    exercise_style: ExerciseStyle,
    epsilon: T,
}

impl<T: Copy> VanillaOption<T> {
    /// Creates a new vanilla option.
    ///
    /// # Arguments
    ///
    /// * `params` - Instrument parameters (strike, expiry, notional)
    /// * `payoff_type` - Call or Put
    /// * `exercise_style` - European or American
    /// * `epsilon` - Smoothing parameter for payoff
    #[must_use]
    pub fn new(
        params: InstrumentParams<T>,
        payoff_type: PayoffType,
        exercise_style: ExerciseStyle,
        epsilon: T,
    ) -> Self {
        Self {
            params,
            payoff_type,
            exercise_style,
            epsilon,
        }
    }

    /// Returns the instrument parameters.
    #[must_use]
    pub fn params(&self) -> &InstrumentParams<T> { &self.params }

    /// Returns the strike price.
    #[inline]
    #[must_use]
    pub fn strike(&self) -> T { self.params.strike() }

    /// Returns the time to expiry.
    #[inline]
    #[must_use]
    pub fn expiry(&self) -> T { self.params.expiry() }

    /// Returns the notional amount.
    #[inline]
    #[must_use]
    pub fn notional(&self) -> T { self.params.notional() }

    /// Returns the payoff type.
    #[must_use]
    pub fn payoff_type(&self) -> PayoffType { self.payoff_type }

    /// Returns the exercise style.
    #[must_use]
    pub fn exercise_style(&self) -> ExerciseStyle { self.exercise_style }

    /// Returns the smoothing epsilon.
    #[must_use]
    pub fn epsilon(&self) -> T { self.epsilon }
}

impl VanillaOption<f64> {
    /// Computes the payoff at the given spot price.
    ///
    /// Uses smooth max for AD compatibility.
    #[must_use]
    pub fn payoff(&self, spot: f64) -> f64 {
        let intrinsic = self.payoff_type.sign() * (spot - self.params.strike);
        smooth_max(intrinsic, 0.0, self.epsilon)
    }
}

/// Forward contract instrument.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Forward<T> {
    strike: T,
    expiry: T,
    notional: T,
    direction: ForwardDirection,
}

impl<T: Copy> Forward<T> {
    /// Creates a new forward contract.
    ///
    /// # Arguments
    ///
    /// * `strike` - Forward price
    /// * `expiry` - Time to expiry in years
    /// * `notional` - Notional amount
    /// * `direction` - Long or Short
    pub fn new(
        strike: T,
        expiry: T,
        notional: T,
        direction: ForwardDirection,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            strike,
            expiry,
            notional,
            direction,
        })
    }

    /// Returns the strike price.
    #[must_use]
    pub fn strike(&self) -> T { self.strike }

    /// Returns the time to expiry.
    #[must_use]
    pub fn expiry(&self) -> T { self.expiry }

    /// Returns the notional amount.
    #[must_use]
    pub fn notional(&self) -> T { self.notional }

    /// Returns the direction.
    #[must_use]
    pub fn direction(&self) -> ForwardDirection { self.direction }
}

impl Forward<f64> {
    /// Computes the payoff at the given spot price.
    #[must_use]
    pub fn payoff(&self, spot: f64) -> f64 {
        self.direction.sign() * (spot - self.strike) * self.notional
    }
}

/// A pricing instrument for valuation and risk calculations.
///
/// This enum represents different types of financial instruments
/// that can be priced and have Greeks computed.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PricingInstrument<T> {
    /// Vanilla option (call or put).
    Vanilla(VanillaOption<T>),
    /// Forward contract.
    Forward(Forward<T>),
}

impl<T: Copy> PricingInstrument<T> {
    /// Returns the expiry time.
    #[must_use]
    pub fn expiry(&self) -> T {
        match self {
            PricingInstrument::Vanilla(opt) => opt.params().expiry(),
            PricingInstrument::Forward(fwd) => fwd.expiry(),
        }
    }

    /// Returns true if this is a vanilla option.
    #[must_use]
    pub fn is_vanilla(&self) -> bool { matches!(self, PricingInstrument::Vanilla(_)) }

    /// Returns true if this is a forward.
    #[must_use]
    pub fn is_forward(&self) -> bool { matches!(self, PricingInstrument::Forward(_)) }

    /// Returns true if this is a swap (currently always false).
    #[must_use]
    pub fn is_swap(&self) -> bool { false }

    /// Returns a reference to the vanilla option if this is one.
    #[must_use]
    pub fn as_vanilla(&self) -> Option<&VanillaOption<T>> {
        match self {
            PricingInstrument::Vanilla(opt) => Some(opt),
            _ => None,
        }
    }

    /// Returns a reference to the forward if this is one.
    #[must_use]
    pub fn as_forward(&self) -> Option<&Forward<T>> {
        match self {
            PricingInstrument::Forward(fwd) => Some(fwd),
            _ => None,
        }
    }
}

impl PricingInstrument<f64> {
    /// Computes the payoff at the given spot price.
    #[must_use]
    pub fn payoff(&self, spot: f64) -> f64 {
        match self {
            PricingInstrument::Vanilla(opt) => opt.payoff(spot),
            PricingInstrument::Forward(fwd) => fwd.payoff(spot),
        }
    }
}

/// FX option type (Call or Put).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FxOptionType {
    /// Call option - right to buy the base currency.
    Call,
    /// Put option - right to sell the base currency.
    Put,
}

impl FxOptionType {
    /// Returns true if this is a call option.
    #[inline]
    #[must_use]
    pub fn is_call(&self) -> bool { matches!(self, FxOptionType::Call) }

    /// Returns true if this is a put option.
    #[inline]
    #[must_use]
    pub fn is_put(&self) -> bool { matches!(self, FxOptionType::Put) }
}

/// Smooth maximum function for AD compatibility.
///
/// Approximates max(a, b) using a smooth function.
#[inline]
fn smooth_max(a: f64, b: f64, epsilon: f64) -> f64 {
    let diff = a - b;
    if diff.abs() < epsilon * 10.0 {
        // Use smooth approximation near the transition
        0.5 * (a + b + (diff * diff + epsilon * epsilon).sqrt())
    } else if diff > 0.0 {
        a
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vanilla_option_payoffs() {
        let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();

        // Call: ITM and OTM
        let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
        assert!((call.payoff(110.0) - 10.0).abs() < 0.01);
        assert!(call.payoff(90.0) < 0.01);

        // Put: ITM and OTM
        let put = VanillaOption::new(params, PayoffType::Put, ExerciseStyle::European, 1e-6);
        assert!((put.payoff(90.0) - 10.0).abs() < 0.01);
        assert!(put.payoff(110.0) < 0.01);
    }

    #[test]
    fn test_forward_payoffs() {
        let long = Forward::new(100.0, 1.0, 1.0, ForwardDirection::Long).unwrap();
        assert!((long.payoff(110.0) - 10.0).abs() < 1e-10);
        assert!((long.payoff(90.0) - (-10.0)).abs() < 1e-10);

        let short = Forward::new(100.0, 1.0, 1.0, ForwardDirection::Short).unwrap();
        assert!((short.payoff(110.0) - (-10.0)).abs() < 1e-10);
        assert!((short.payoff(90.0) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_pricing_instrument_dispatch() {
        let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
        let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
        let vanilla = PricingInstrument::Vanilla(call);
        assert!(vanilla.is_vanilla());
        assert_eq!(vanilla.expiry(), 1.0);
        assert!((vanilla.payoff(110.0) - 10.0).abs() < 0.01);

        let fwd = Forward::new(100.0, 2.0, 1.0, ForwardDirection::Long).unwrap();
        let forward = PricingInstrument::Forward(fwd);
        assert!(forward.is_forward());
        assert_eq!(forward.expiry(), 2.0);
    }

    #[test]
    fn test_exercise_style_and_direction() {
        assert_eq!(format!("{}", ExerciseStyle::European), "European");
        assert!(ExerciseStyle::European.is_european());
        assert!(ExerciseStyle::American.is_american());
        assert_eq!(ForwardDirection::Long.sign(), 1.0);
        assert_eq!(ForwardDirection::Short.sign(), -1.0);
    }
}
