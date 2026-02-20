//! Conditional event legs for trades with exercise and barrier features.

use super::{
    cashflow::Cashflow,
    index::IndexType,
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

/// Barrier event classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BarrierEventType {
    /// No barrier.
    Nothing,
    /// Knock-in barrier (option activates when hit).
    KnockIn,
    /// Knock-out barrier (option deactivates when hit).
    KnockOut,
}

/// Specification of a single barrier level.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BarrierSpec {
    /// Type of barrier.
    pub barrier_type: BarrierEventType,
    /// Barrier level.
    pub level: f64,
    /// Rebate amount if barrier is triggered.
    pub rebate: f64,
}

/// Monitoring type for barrier observation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum MonitoringType {
    /// Continuous monitoring.
    #[default]
    Continuous,
    /// Discrete monitoring at specific dates.
    Discrete,
}

/// A barrier event that gates conditional cashflows.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BarrierEvent {
    /// Start date of barrier observation period.
    pub start_date: Date,
    /// End date of barrier observation period.
    pub end_date: Date,
    /// Upper barrier specification (if any).
    pub upper_barrier: Option<BarrierSpec>,
    /// Lower barrier specification (if any).
    pub lower_barrier: Option<BarrierSpec>,
    /// Monitoring type.
    pub monitoring: MonitoringType,
    /// Observable index for barrier monitoring.
    pub observable_index: IndexType,
    /// Payment date for rebate (if any).
    pub rebate_payment_date: Option<Date>,
}

/// Accumulation side for TARN structures.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum AccumSide {
    /// No accumulation.
    #[default]
    None,
    /// Accumulate pay-side coupons.
    Pay,
    /// Accumulate receive-side coupons.
    Receive,
    /// Accumulate pay minus receive.
    PayMinusReceive,
    /// Accumulate receive minus pay.
    ReceiveMinusPay,
}

/// Type of event that gates conditional legs.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EventKind {
    /// Exercise event (swaption, callable bond, etc.).
    Exercise(ExerciseEvent),
    /// Barrier event (knock-in/knock-out options).
    Barrier(BarrierEvent),
}

/// A conditional leg structure that activates upon an exercise or barrier event.
///
/// For a Swaption, the `EventLeg` contains the underlying swap's
/// fixed and floating legs, gated by the exercise event.
/// For a CDS Option, it contains the underlying CDS's premium
/// and protection legs.
/// For barrier options, it contains the conditional legs gated by barrier events.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EventLeg {
    /// The event that activates these legs.
    pub event: EventKind,

    /// The conditional legs (activated only upon event).
    legs: Vec<Leg>,

    /// TARN: redemption target amount.
    pub redemption_target: Option<f64>,

    /// TARN: accumulation side.
    pub accum_side: AccumSide,

    /// Whether this is a buy option (from holder's perspective).
    pub is_buy_option: bool,

    /// Whether the option has been exercised.
    pub is_option_exercised: bool,
}

impl EventLeg {
    /// Creates a new event leg with an exercise event.
    #[must_use]
    pub fn new(exercise: ExerciseEvent, legs: Vec<Leg>) -> Self {
        Self {
            event: EventKind::Exercise(exercise),
            legs,
            redemption_target: None,
            accum_side: AccumSide::default(),
            is_buy_option: true,
            is_option_exercised: false,
        }
    }

    /// Creates a new event leg with a barrier event.
    #[must_use]
    pub fn new_barrier(barrier: BarrierEvent, legs: Vec<Leg>) -> Self {
        Self {
            event: EventKind::Barrier(barrier),
            legs,
            redemption_target: None,
            accum_side: AccumSide::default(),
            is_buy_option: true,
            is_option_exercised: false,
        }
    }

    /// Sets the redemption target for TARN.
    #[must_use]
    pub fn with_redemption_target(mut self, target: f64) -> Self {
        self.redemption_target = Some(target);
        self
    }

    /// Sets the accumulation side for TARN.
    #[must_use]
    pub fn with_accum_side(mut self, side: AccumSide) -> Self {
        self.accum_side = side;
        self
    }

    /// Returns an iterator over the conditional legs.
    pub fn legs(&self) -> impl Iterator<Item = &Leg> {
        self.legs.iter()
    }

    /// Returns all cashflows across all conditional legs.
    pub fn all_cashflows(&self) -> impl Iterator<Item = &Cashflow> {
        self.legs.iter().flat_map(|leg| leg.cashflows())
    }

    /// Returns the number of conditional legs.
    #[must_use]
    pub fn num_legs(&self) -> usize {
        self.legs.len()
    }

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

    /// Returns true if this event is an exercise.
    #[must_use]
    pub fn is_exercise(&self) -> bool {
        matches!(self.event, EventKind::Exercise(_))
    }

    /// Returns true if this event is a barrier.
    #[must_use]
    pub fn is_barrier(&self) -> bool {
        matches!(self.event, EventKind::Barrier(_))
    }

    /// Returns the exercise event if this is an exercise type.
    #[must_use]
    pub fn exercise_event(&self) -> Option<&ExerciseEvent> {
        match &self.event {
            EventKind::Exercise(e) => Some(e),
            _ => None,
        }
    }

    /// Returns the barrier event if this is a barrier type.
    #[must_use]
    pub fn barrier_event(&self) -> Option<&BarrierEvent> {
        match &self.event {
            EventKind::Barrier(b) => Some(b),
            _ => None,
        }
    }

    /// Returns true if this event has a TARN redemption target.
    #[must_use]
    pub fn has_redemption_target(&self) -> bool {
        self.redemption_target.is_some()
    }

    /// Returns the exercise dates (only for exercise events).
    #[must_use]
    pub fn exercise_dates(&self) -> Option<&[Date]> {
        match &self.event {
            EventKind::Exercise(e) => Some(&e.exercise_dates),
            _ => None,
        }
    }

    /// Returns the exercise type (only for exercise events).
    #[must_use]
    pub fn exercise_type(&self) -> Option<ExerciseType> {
        match &self.event {
            EventKind::Exercise(e) => Some(e.exercise_type),
            _ => None,
        }
    }

    /// Returns the settlement type (only for exercise events).
    #[must_use]
    pub fn settlement_type(&self) -> Option<SettlementType> {
        match &self.event {
            EventKind::Exercise(e) => Some(e.settlement_type),
            _ => None,
        }
    }
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
        assert_eq!(event_leg.exercise_type(), Some(ExerciseType::European));
        assert_eq!(event_leg.settlement_type(), Some(SettlementType::Cash));
        assert_eq!(event_leg.exercise_dates().unwrap().len(), 1);
        assert!(event_leg.is_exercise());
        assert!(!event_leg.is_barrier());
    }

    #[test]
    fn test_barrier_event_leg() {
        let barrier = BarrierEvent {
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            end_date: Date::from_ymd(2026, 1, 1).unwrap(),
            upper_barrier: Some(BarrierSpec {
                barrier_type: BarrierEventType::KnockOut,
                level: 1.50,
                rebate: 0.01,
            }),
            lower_barrier: None,
            monitoring: MonitoringType::Continuous,
            observable_index: IndexType::Fx {
                base: "EUR".into(),
                quote: "USD".into(),
            },
            rebate_payment_date: None,
        };

        let event_leg = EventLeg::new_barrier(barrier, vec![]);

        assert!(event_leg.is_barrier());
        assert!(!event_leg.is_exercise());
        assert!(event_leg.exercise_type().is_none());
        assert!(event_leg.settlement_type().is_none());
        assert!(event_leg.exercise_dates().is_none());
        assert!(event_leg.barrier_event().is_some());
        assert!(event_leg.exercise_event().is_none());
    }

    #[test]
    fn test_tarn_event_leg() {
        let exercise = ExerciseEvent {
            exercise_dates: vec![Date::from_ymd(2026, 1, 15).unwrap()],
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
        };

        let event_leg = EventLeg::new(exercise, vec![])
            .with_redemption_target(0.10)
            .with_accum_side(AccumSide::Receive);

        assert!(event_leg.has_redemption_target());
        assert_eq!(event_leg.redemption_target, Some(0.10));
        assert_eq!(event_leg.accum_side, AccumSide::Receive);
    }
}
