//! Leg types for financial instruments.

use super::cashflow::Cashflow;
use crate::{market::Currency, time::Date};

/// Direction of a leg from the perspective of the trade holder.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Payer: pays this leg's cashflows (negative NPV contribution).
    Payer,
    /// Receiver: receives this leg's cashflows (positive NPV contribution).
    #[default]
    Receiver,
}

impl Direction {
    /// Returns the sign for NPV calculation.
    #[must_use]
    pub fn sign(&self) -> f64 {
        match self {
            Direction::Payer => -1.0,
            Direction::Receiver => 1.0,
        }
    }

    /// Returns true if this is a payer direction.
    #[must_use]
    pub fn is_payer(&self) -> bool { matches!(self, Direction::Payer) }

    /// Returns true if this is a receiver direction.
    #[must_use]
    pub fn is_receiver(&self) -> bool { matches!(self, Direction::Receiver) }

    /// Returns the opposite direction.
    #[must_use]
    pub fn opposite(&self) -> Self {
        match self {
            Direction::Payer => Direction::Receiver,
            Direction::Receiver => Direction::Payer,
        }
    }
}

/// Type of leg in a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LegType {
    /// Fixed rate leg.
    Fixed,
    /// Floating rate leg (indexed to a rate).
    Floating,
    /// Cap/Floor leg (with option payoffs).
    CapFloor,
    /// Principal exchange leg.
    Principal,
    /// Premium payment leg (option premium).
    Premium,
    /// Protection leg (credit protection payout).
    Protection,
    /// Generic leg (catch-all).
    Generic,
}

impl LegType {
    /// Returns true if this is a fixed leg.
    #[must_use]
    pub fn is_fixed(&self) -> bool { matches!(self, LegType::Fixed) }

    /// Returns true if this is a floating leg.
    #[must_use]
    pub fn is_floating(&self) -> bool { matches!(self, LegType::Floating) }
}

/// A leg (stream of cashflows) in a financial instrument.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Leg {
    /// Cashflows in this leg, ordered by payment date.
    cashflows: Vec<Cashflow>,

    /// Direction of this leg from the holder's perspective.
    pub direction: Direction,

    /// Type of this leg.
    pub leg_type: LegType,

    /// Currency of the leg.
    pub currency: Currency,
}

impl Leg {
    /// Creates a new leg.
    #[must_use]
    pub fn new(
        cashflows: Vec<Cashflow>,
        direction: Direction,
        leg_type: LegType,
        currency: Currency,
    ) -> Self {
        Self {
            cashflows,
            direction,
            leg_type,
            currency,
        }
    }

    /// Returns an iterator over all cashflows in this leg.
    pub fn cashflows(&self) -> impl Iterator<Item = &Cashflow> { self.cashflows.iter() }

    /// Returns an iterator over future cashflows (payment_date > ref_date).
    pub fn future_cashflows(&self, ref_date: Date) -> impl Iterator<Item = &Cashflow> {
        self.cashflows
            .iter()
            .filter(move |cf| cf.is_future(ref_date))
    }

    /// Returns the number of cashflows in this leg.
    #[must_use]
    pub fn len(&self) -> usize { self.cashflows.len() }

    /// Returns true if this leg has no cashflows.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.cashflows.is_empty() }

    /// Returns the total notional of the leg.
    #[must_use]
    pub fn notional(&self) -> f64 { self.cashflows.first().map_or(0.0, |cf| cf.notional) }

    /// Returns the first payment date in this leg.
    #[must_use]
    pub fn first_payment_date(&self) -> Option<Date> {
        self.cashflows.first().map(|cf| cf.payment_date)
    }

    /// Returns the last payment date in this leg.
    #[must_use]
    pub fn last_payment_date(&self) -> Option<Date> {
        self.cashflows.last().map(|cf| cf.payment_date)
    }

    /// Returns the number of future cashflows.
    #[must_use]
    pub fn future_cashflow_count(&self, ref_date: Date) -> usize {
        self.future_cashflows(ref_date).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::{CashflowType, Payoff};

    fn make_test_leg() -> Leg {
        let cashflows = vec![
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 1, 1).unwrap(),
                Date::from_ymd(2024, 7, 1).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                0.5,
                1_000_000.0,
                Payoff::fixed(0.05),
                Currency::USD,
            ),
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 7, 1).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                Date::from_ymd(2025, 7, 1).unwrap(),
                0.5,
                1_000_000.0,
                Payoff::fixed(0.05),
                Currency::USD,
            ),
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2026, 1, 1).unwrap(),
                Date::from_ymd(2025, 7, 1).unwrap(),
                Date::from_ymd(2026, 1, 1).unwrap(),
                0.5,
                1_000_000.0,
                Payoff::fixed(0.05),
                Currency::USD,
            ),
        ];

        Leg::new(
            cashflows,
            Direction::Receiver,
            LegType::Fixed,
            Currency::USD,
        )
    }

    #[test]
    fn test_direction_sign() {
        assert_eq!(Direction::Payer.sign(), -1.0);
        assert_eq!(Direction::Receiver.sign(), 1.0);
    }

    #[test]
    fn test_direction_is_payer() {
        assert!(Direction::Payer.is_payer());
        assert!(!Direction::Receiver.is_payer());
    }

    #[test]
    fn test_direction_is_receiver() {
        assert!(!Direction::Payer.is_receiver());
        assert!(Direction::Receiver.is_receiver());
    }

    #[test]
    fn test_direction_opposite() {
        assert_eq!(Direction::Payer.opposite(), Direction::Receiver);
        assert_eq!(Direction::Receiver.opposite(), Direction::Payer);
    }

    #[test]
    fn test_leg_type_is_fixed() {
        assert!(LegType::Fixed.is_fixed());
        assert!(!LegType::Floating.is_fixed());
    }

    #[test]
    fn test_leg_type_is_floating() {
        assert!(!LegType::Fixed.is_floating());
        assert!(LegType::Floating.is_floating());
    }

    #[test]
    fn test_leg_new() {
        let leg = make_test_leg();

        assert_eq!(leg.direction, Direction::Receiver);
        assert_eq!(leg.leg_type, LegType::Fixed);
        assert_eq!(leg.currency, Currency::USD);
    }

    #[test]
    fn test_leg_cashflows() {
        let leg = make_test_leg();
        let count = leg.cashflows().count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_leg_future_cashflows() {
        let leg = make_test_leg();
        let ref_date = Date::from_ymd(2025, 3, 1).unwrap();

        let future_count = leg.future_cashflows(ref_date).count();
        assert_eq!(future_count, 2);
    }

    #[test]
    fn test_leg_future_cashflows_all_past() {
        let leg = make_test_leg();
        let ref_date = Date::from_ymd(2027, 1, 1).unwrap();

        let future_count = leg.future_cashflows(ref_date).count();
        assert_eq!(future_count, 0);
    }

    #[test]
    fn test_leg_future_cashflows_all_future() {
        let leg = make_test_leg();
        let ref_date = Date::from_ymd(2024, 1, 1).unwrap();

        let future_count = leg.future_cashflows(ref_date).count();
        assert_eq!(future_count, 3);
    }

    #[test]
    fn test_leg_len() {
        let leg = make_test_leg();
        assert_eq!(leg.len(), 3);
    }

    #[test]
    fn test_leg_is_empty() {
        let leg = make_test_leg();
        assert!(!leg.is_empty());

        let empty_leg = Leg::new(vec![], Direction::Payer, LegType::Fixed, Currency::USD);
        assert!(empty_leg.is_empty());
    }

    #[test]
    fn test_leg_notional() {
        let leg = make_test_leg();
        assert_eq!(leg.notional(), 1_000_000.0);
    }

    #[test]
    fn test_leg_notional_empty() {
        let empty_leg = Leg::new(vec![], Direction::Payer, LegType::Fixed, Currency::USD);
        assert_eq!(empty_leg.notional(), 0.0);
    }

    #[test]
    fn test_leg_first_payment_date() {
        let leg = make_test_leg();
        assert_eq!(
            leg.first_payment_date(),
            Some(Date::from_ymd(2025, 1, 1).unwrap())
        );
    }

    #[test]
    fn test_leg_last_payment_date() {
        let leg = make_test_leg();
        assert_eq!(
            leg.last_payment_date(),
            Some(Date::from_ymd(2026, 1, 1).unwrap())
        );
    }

    #[test]
    fn test_leg_payment_dates_empty() {
        let empty_leg = Leg::new(vec![], Direction::Payer, LegType::Fixed, Currency::USD);
        assert!(empty_leg.first_payment_date().is_none());
        assert!(empty_leg.last_payment_date().is_none());
    }

    #[test]
    fn test_leg_future_cashflow_count() {
        let leg = make_test_leg();
        let ref_date = Date::from_ymd(2025, 3, 1).unwrap();

        assert_eq!(leg.future_cashflow_count(ref_date), 2);
    }

    #[test]
    fn test_leg_clone() {
        let leg = make_test_leg();
        let cloned = leg.clone();
        assert_eq!(leg, cloned);
    }

    #[test]
    fn test_direction_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(Direction::Payer);
        set.insert(Direction::Receiver);
        set.insert(Direction::Payer);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_leg_type_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(LegType::Fixed);
        set.insert(LegType::Floating);
        set.insert(LegType::Fixed);
        assert_eq!(set.len(), 2);
    }
}
