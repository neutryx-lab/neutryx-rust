//! Common types shared across instrument definitions.
//!
//! This module provides shared enums and structs used by multiple
//! instrument types across asset classes.

/// Asset class categorisation for financial instruments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AssetClass {
    /// Interest rate instruments (swaps, swaptions, caps/floors, etc.).
    Rates,
    /// Foreign exchange instruments (spots, forwards, options).
    Fx,
    /// Equity instruments (forwards, options, swaps).
    Equity,
    /// Credit instruments (CDS, CDX, etc.).
    Credit,
    /// Commodity instruments (forwards, swaps, options).
    Commodity,
}

impl AssetClass {
    /// Returns the string representation of the asset class.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetClass::Rates => "Rates",
            AssetClass::Fx => "FX",
            AssetClass::Equity => "Equity",
            AssetClass::Credit => "Credit",
            AssetClass::Commodity => "Commodity",
        }
    }
}

impl std::fmt::Display for AssetClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Exercise style for option instruments.
///
/// Note: This is distinct from `ExerciseType` in the trade module which is used
/// for trade-level exercise specifications. This enum is for instrument definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExerciseStyle {
    /// European: exercise only at expiry.
    European,
    /// American: exercise at any time until expiry.
    American,
    /// Bermudan: exercise at specific dates.
    Bermudan,
}

/// Payer/Receiver position indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PayerReceiver {
    /// Payer position (pay fixed, receive floating in swap context).
    Payer,
    /// Receiver position (receive fixed, pay floating in swap context).
    Receiver,
}

impl PayerReceiver {
    /// Returns the opposite position.
    #[must_use]
    pub fn opposite(&self) -> Self {
        match self {
            PayerReceiver::Payer => PayerReceiver::Receiver,
            PayerReceiver::Receiver => PayerReceiver::Payer,
        }
    }

    /// Returns 1.0 for Payer, -1.0 for Receiver.
    #[must_use]
    pub fn sign(&self) -> f64 {
        match self {
            PayerReceiver::Payer => 1.0,
            PayerReceiver::Receiver => -1.0,
        }
    }
}

/// Barrier type for barrier options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BarrierType {
    /// Knock-in: option becomes active when barrier is breached.
    KnockIn,
    /// Knock-out: option becomes void when barrier is breached.
    KnockOut,
}

/// Barrier direction for barrier options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BarrierDirection {
    /// Up barrier: triggered when spot rises above barrier level.
    Up,
    /// Down barrier: triggered when spot falls below barrier level.
    Down,
}

/// Notional schedule for amortising instruments.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NotionalSchedule {
    /// Notional amounts for each period.
    pub notionals: Vec<f64>,
}

impl NotionalSchedule {
    /// Creates a constant notional schedule.
    #[must_use]
    pub fn constant(notional: f64) -> Self {
        Self {
            notionals: vec![notional],
        }
    }

    /// Creates a notional schedule from a vector of amounts.
    #[must_use]
    pub fn from_schedule(notionals: Vec<f64>) -> Self {
        Self { notionals }
    }

    /// Returns the notional for a given period index.
    ///
    /// If the schedule has fewer entries than the period index,
    /// returns the last notional (constant extrapolation).
    #[must_use]
    pub fn notional_at(&self, period_index: usize) -> f64 {
        self.notionals
            .get(period_index)
            .or_else(|| self.notionals.last())
            .copied()
            .unwrap_or(0.0)
    }

    /// Returns true if this is a constant notional schedule.
    #[must_use]
    pub fn is_constant(&self) -> bool {
        self.notionals.len() <= 1
            || self.notionals.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-10)
    }
}

impl Default for NotionalSchedule {
    fn default() -> Self {
        Self {
            notionals: vec![1_000_000.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_class_display() {
        assert_eq!(AssetClass::Rates.to_string(), "Rates");
        assert_eq!(AssetClass::Fx.to_string(), "FX");
        assert_eq!(AssetClass::Equity.to_string(), "Equity");
        assert_eq!(AssetClass::Credit.to_string(), "Credit");
        assert_eq!(AssetClass::Commodity.to_string(), "Commodity");
    }

    #[test]
    fn test_asset_class_as_str() {
        assert_eq!(AssetClass::Rates.as_str(), "Rates");
        assert_eq!(AssetClass::Fx.as_str(), "FX");
    }

    #[test]
    fn test_asset_class_equality() {
        assert_eq!(AssetClass::Rates, AssetClass::Rates);
        assert_ne!(AssetClass::Rates, AssetClass::Fx);
    }

    #[test]
    fn test_asset_class_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(AssetClass::Rates);
        set.insert(AssetClass::Fx);
        set.insert(AssetClass::Rates); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_exercise_style_equality() {
        assert_eq!(ExerciseStyle::European, ExerciseStyle::European);
        assert_ne!(ExerciseStyle::European, ExerciseStyle::American);
    }

    #[test]
    fn test_payer_receiver_opposite() {
        assert_eq!(PayerReceiver::Payer.opposite(), PayerReceiver::Receiver);
        assert_eq!(PayerReceiver::Receiver.opposite(), PayerReceiver::Payer);
    }

    #[test]
    fn test_payer_receiver_sign() {
        assert_eq!(PayerReceiver::Payer.sign(), 1.0);
        assert_eq!(PayerReceiver::Receiver.sign(), -1.0);
    }

    #[test]
    fn test_barrier_type_equality() {
        assert_eq!(BarrierType::KnockIn, BarrierType::KnockIn);
        assert_ne!(BarrierType::KnockIn, BarrierType::KnockOut);
    }

    #[test]
    fn test_barrier_direction_equality() {
        assert_eq!(BarrierDirection::Up, BarrierDirection::Up);
        assert_ne!(BarrierDirection::Up, BarrierDirection::Down);
    }

    #[test]
    fn test_notional_schedule_constant() {
        let schedule = NotionalSchedule::constant(1_000_000.0);
        assert!(schedule.is_constant());
        assert_eq!(schedule.notional_at(0), 1_000_000.0);
        assert_eq!(schedule.notional_at(10), 1_000_000.0); // Extrapolation
    }

    #[test]
    fn test_notional_schedule_amortising() {
        let schedule = NotionalSchedule::from_schedule(vec![1_000_000.0, 800_000.0, 600_000.0]);
        assert!(!schedule.is_constant());
        assert_eq!(schedule.notional_at(0), 1_000_000.0);
        assert_eq!(schedule.notional_at(1), 800_000.0);
        assert_eq!(schedule.notional_at(2), 600_000.0);
        assert_eq!(schedule.notional_at(5), 600_000.0); // Extrapolation
    }

    #[test]
    fn test_notional_schedule_default() {
        let schedule = NotionalSchedule::default();
        assert!(schedule.is_constant());
        assert_eq!(schedule.notional_at(0), 1_000_000.0);
    }

    #[test]
    fn test_notional_schedule_clone() {
        let schedule = NotionalSchedule::from_schedule(vec![100.0, 200.0]);
        let cloned = schedule.clone();
        assert_eq!(schedule, cloned);
    }
}
