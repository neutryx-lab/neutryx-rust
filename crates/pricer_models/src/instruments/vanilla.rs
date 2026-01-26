//! Vanilla option types for the Black-Scholes model.

use num_traits::Float;
use pricer_core::math::numeric::from_f64;

use crate::analytical::error::AnalyticalError;

/// Option payoff type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PayoffType {
    /// Call option - payoff is max(S - K, 0).
    Call,
    /// Put option - payoff is max(K - S, 0).
    Put,
    /// Digital call - pays 1 if S > K, else 0.
    DigitalCall,
    /// Digital put - pays 1 if S < K, else 0.
    DigitalPut,
}

impl PayoffType {
    /// Returns true if this is a call-like payoff.
    #[inline]
    #[must_use]
    pub fn is_call(&self) -> bool { matches!(self, PayoffType::Call | PayoffType::DigitalCall) }

    /// Returns true if this is a put-like payoff.
    #[inline]
    #[must_use]
    pub fn is_put(&self) -> bool { matches!(self, PayoffType::Put | PayoffType::DigitalPut) }
}

/// Exercise style for options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExerciseStyle {
    /// European - can only exercise at expiry.
    European,
    /// American - can exercise any time before expiry.
    American,
    /// Bermudan - can exercise on specified dates.
    Bermudan,
    /// Asian - payoff depends on average price.
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

/// Common parameters for vanilla instruments.
#[derive(Debug, Clone, Copy)]
pub struct InstrumentParams<T: Float> {
    strike: T,
    expiry: T,
    notional: T,
}

impl<T: Float> InstrumentParams<T> {
    /// Creates new instrument parameters.
    ///
    /// # Arguments
    /// * `strike` - Strike price (must be positive)
    /// * `expiry` - Time to expiry in years (must be positive)
    /// * `notional` - Notional amount (must be positive)
    ///
    /// # Errors
    /// Returns an error if any parameter is non-positive.
    pub fn new(strike: T, expiry: T, notional: T) -> Result<Self, AnalyticalError> {
        let zero = T::zero();

        if strike <= zero {
            return Err(AnalyticalError::InvalidSpot {
                spot: strike.to_f64().unwrap_or(0.0),
            });
        }

        if expiry < zero {
            return Err(AnalyticalError::NumericalInstability {
                message: format!(
                    "Expiry must be non-negative: {}",
                    expiry.to_f64().unwrap_or(0.0)
                ),
            });
        }

        if notional <= zero {
            return Err(AnalyticalError::NumericalInstability {
                message: format!(
                    "Notional must be positive: {}",
                    notional.to_f64().unwrap_or(0.0)
                ),
            });
        }

        Ok(Self {
            strike,
            expiry,
            notional,
        })
    }

    /// Returns the strike price.
    #[inline]
    pub fn strike(&self) -> T { self.strike }

    /// Returns the time to expiry.
    #[inline]
    pub fn expiry(&self) -> T { self.expiry }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> T { self.notional }
}

/// Vanilla option instrument.
#[derive(Debug, Clone)]
pub struct VanillaOption<T: Float> {
    params: InstrumentParams<T>,
    payoff_type: PayoffType,
    exercise_style: ExerciseStyle,
    #[allow(dead_code)]
    epsilon: T,
}

impl<T: Float> VanillaOption<T> {
    /// Creates a new vanilla option.
    ///
    /// # Arguments
    /// * `params` - Common instrument parameters (strike, expiry, notional)
    /// * `payoff_type` - Type of payoff (Call, Put, etc.)
    /// * `exercise_style` - Exercise style (European, American, etc.)
    /// * `epsilon` - Smoothing parameter for differentiable payoff (typically
    ///   1e-6)
    #[must_use]
    pub fn new(
        params: InstrumentParams<T>,
        payoff_type: PayoffType,
        exercise_style: ExerciseStyle,
        epsilon: f64,
    ) -> Self {
        Self {
            params,
            payoff_type,
            exercise_style,
            epsilon: from_f64(epsilon),
        }
    }

    /// Returns the strike price.
    #[inline]
    pub fn strike(&self) -> T { self.params.strike() }

    /// Returns the time to expiry.
    #[inline]
    pub fn expiry(&self) -> T { self.params.expiry() }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> T { self.params.notional() }

    /// Returns the payoff type.
    #[inline]
    pub fn payoff_type(&self) -> PayoffType { self.payoff_type }

    /// Returns the exercise style.
    #[inline]
    pub fn exercise_style(&self) -> ExerciseStyle { self.exercise_style }
}
