//! Conditional event legs for trades with exercise features.

use super::{
    cashflow::Cashflow,
    leg::{Leg, LegType},
    trade::{ExerciseType, SettlementType},
};
use crate::time::Date;

/// Describes the exercise event that gates conditional legs.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExerciseEvent {
    /// Dates at which exercise may occur.
    /// European: exactly one date. Bermudan: multiple sorted dates.
    pub exercise_dates: Vec<Date>,

    /// Style of exercise (European / Bermudan / American).
    pub exercise_type: ExerciseType,

    /// Settlement method upon exercise.
    pub settlement_type: SettlementType,
}

/// A conditional leg structure that activates upon an exercise event.
///
/// For a Swaption, the `EventLeg` contains the underlying swap's
/// fixed and floating legs, gated by the exercise event.
/// For a CDS Option, it contains the underlying CDS's premium
/// and protection legs.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EventLeg {
    /// The exercise event that activates these legs.
    pub exercise: ExerciseEvent,

    /// The conditional legs (activated only upon exercise).
    legs: Vec<Leg>,
}

impl EventLeg {
    /// Creates a new event leg.
    #[must_use]
    pub fn new(exercise: ExerciseEvent, legs: Vec<Leg>) -> Self { Self { exercise, legs } }

    /// Returns an iterator over the conditional legs.
    pub fn legs(&self) -> impl Iterator<Item = &Leg> { self.legs.iter() }

    /// Returns all cashflows across all conditional legs.
    pub fn all_cashflows(&self) -> impl Iterator<Item = &Cashflow> {
        self.legs.iter().flat_map(|leg| leg.cashflows())
    }

    /// Returns the number of conditional legs.
    #[must_use]
    pub fn num_legs(&self) -> usize { self.legs.len() }

    /// Returns the fixed leg among the conditional legs, if any.
    #[must_use]
    pub fn fixed_leg(&self) -> Option<&Leg> {
        self.legs.iter().find(|leg| leg.leg_type == LegType::Fixed)
    }

    /// Returns the floating leg among the conditional legs, if any.
    #[must_use]
    pub fn floating_leg(&self) -> Option<&Leg> {
        self.legs
            .iter()
            .find(|leg| leg.leg_type == LegType::Floating)
    }

    /// Returns the exercise dates.
    #[must_use]
    pub fn exercise_dates(&self) -> &[Date] { &self.exercise.exercise_dates }

    /// Returns the exercise type.
    #[must_use]
    pub fn exercise_type(&self) -> ExerciseType { self.exercise.exercise_type }

    /// Returns the settlement type.
    #[must_use]
    pub fn settlement_type(&self) -> SettlementType { self.exercise.settlement_type }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        market::Currency,
        trade::{CashflowType, Direction, Payoff},
    };

    #[test]
    fn test_event_leg_construction() {
        let fixed_cf = Cashflow::new(
            CashflowType::Coupon,
            Date::from_ymd(2027, 1, 15).unwrap(),
            Date::from_ymd(2026, 1, 15).unwrap(),
            Date::from_ymd(2027, 1, 15).unwrap(),
            1.0,
            10_000_000.0,
            Payoff::fixed(0.03),
            Currency::USD,
        );
        let fixed_leg = Leg::new(
            vec![fixed_cf],
            Direction::Receiver,
            LegType::Fixed,
            Currency::USD,
        );

        let floating_cf = Cashflow::new(
            CashflowType::Coupon,
            Date::from_ymd(2027, 1, 15).unwrap(),
            Date::from_ymd(2026, 1, 15).unwrap(),
            Date::from_ymd(2027, 1, 15).unwrap(),
            1.0,
            10_000_000.0,
            Payoff::fixed(0.0),
            Currency::USD,
        );
        let floating_leg = Leg::new(
            vec![floating_cf],
            Direction::Payer,
            LegType::Floating,
            Currency::USD,
        );

        let exercise = ExerciseEvent {
            exercise_dates: vec![Date::from_ymd(2026, 1, 15).unwrap()],
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
        };

        let event_leg = EventLeg::new(exercise, vec![fixed_leg, floating_leg]);

        assert_eq!(event_leg.num_legs(), 2);
        assert!(event_leg.fixed_leg().is_some());
        assert!(event_leg.floating_leg().is_some());
        assert_eq!(event_leg.all_cashflows().count(), 2);
        assert_eq!(event_leg.exercise_type(), ExerciseType::European);
        assert_eq!(event_leg.settlement_type(), SettlementType::Cash);
        assert_eq!(event_leg.exercise_dates().len(), 1);
    }
}
