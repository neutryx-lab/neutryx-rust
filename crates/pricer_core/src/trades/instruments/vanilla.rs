//! Vanilla option definitions.

use num_traits::Float;

use super::{exercise::ExerciseStyle, params::InstrumentParams, payoff::PayoffType};

/// Vanilla option instrument.
#[derive(Debug, Clone)]
pub struct VanillaOption<T: Float> {
    params: InstrumentParams<T>,
    payoff_type: PayoffType,
    exercise_style: ExerciseStyle<T>,
    epsilon: T,
}

impl<T: Float> VanillaOption<T> {
    /// Creates a new vanilla option.
    pub fn new(
        params: InstrumentParams<T>,
        payoff_type: PayoffType,
        exercise_style: ExerciseStyle<T>,
        epsilon: T,
    ) -> Self {
        Self {
            params,
            payoff_type,
            exercise_style,
            epsilon,
        }
    }

    /// Calculates the payoff at expiry for a given spot price.
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        let unit_payoff = self
            .payoff_type
            .evaluate(spot, self.params.strike(), self.epsilon);
        self.params.notional() * unit_payoff
    }

    /// Returns a reference to the instrument parameters.
    #[inline]
    pub fn params(&self) -> &InstrumentParams<T> { &self.params }

    /// Returns the payoff type.
    #[inline]
    pub fn payoff_type(&self) -> PayoffType { self.payoff_type }

    /// Returns a reference to the exercise style.
    #[inline]
    pub fn exercise_style(&self) -> &ExerciseStyle<T> { &self.exercise_style }

    /// Returns the smoothing epsilon.
    #[inline]
    pub fn epsilon(&self) -> T { self.epsilon }

    /// Returns the strike price.
    #[inline]
    pub fn strike(&self) -> T { self.params.strike() }

    /// Returns the time to expiry.
    #[inline]
    pub fn expiry(&self) -> T { self.params.expiry() }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> T { self.params.notional() }
}
