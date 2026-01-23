//! Swaption convention definitions.
//!
//! This module provides types for representing swaption market conventions.

use super::SwapConvention;
use crate::Currency;

/// Settlement convention for swaption premium and exercise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SettlementConvention {
    /// Physical delivery (enter into underlying swap).
    Physical,
    /// Cash settlement (receive cash based on swap NPV).
    Cash,
}

/// Convention for a swaption.
///
/// Represents the market conventions for pricing and settling swaptions.
///
/// # Example
///
/// ```rust
/// use infra_master::trade::convention::{SwaptionConvention, SwapConvention, SettlementConvention};
/// use infra_master::Currency;
///
/// let conv = SwaptionConvention::usd_sofr();
/// assert_eq!(conv.premium_currency, Currency::USD);
/// assert_eq!(conv.exercise_settlement, SettlementConvention::Cash);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwaptionConvention {
    /// Convention for the underlying swap.
    pub underlying_swap: SwapConvention,
    /// Settlement convention for the premium payment.
    pub premium_settlement: SettlementConvention,
    /// Settlement convention for exercise.
    pub exercise_settlement: SettlementConvention,
    /// Currency for the premium.
    pub premium_currency: Currency,
    /// Number of business days from trade date to premium payment.
    pub premium_lag: u32,
    /// Number of business days from exercise to swap start.
    pub exercise_lag: u32,
}

impl SwaptionConvention {
    /// Creates a new swaption convention.
    #[must_use]
    pub fn new(
        underlying_swap: SwapConvention,
        premium_settlement: SettlementConvention,
        exercise_settlement: SettlementConvention,
        premium_currency: Currency,
        premium_lag: u32,
        exercise_lag: u32,
    ) -> Self {
        Self {
            underlying_swap,
            premium_settlement,
            exercise_settlement,
            premium_currency,
            premium_lag,
            exercise_lag,
        }
    }

    /// Returns the USD SOFR swaption convention.
    ///
    /// - Underlying: USD SOFR swap
    /// - Exercise settlement: Cash
    /// - Premium currency: USD
    /// - Premium lag: 2 days
    /// - Exercise lag: 2 days
    #[must_use]
    pub fn usd_sofr() -> Self {
        Self {
            underlying_swap: SwapConvention::usd_sofr(),
            premium_settlement: SettlementConvention::Cash,
            exercise_settlement: SettlementConvention::Cash,
            premium_currency: Currency::USD,
            premium_lag: 2,
            exercise_lag: 2,
        }
    }

    /// Returns the EUR EURIBOR swaption convention.
    ///
    /// - Underlying: EUR EURIBOR 6M swap
    /// - Exercise settlement: Cash
    /// - Premium currency: EUR
    /// - Premium lag: 2 days
    /// - Exercise lag: 2 days
    #[must_use]
    pub fn eur_euribor() -> Self {
        Self {
            underlying_swap: SwapConvention::eur_euribor_6m(),
            premium_settlement: SettlementConvention::Cash,
            exercise_settlement: SettlementConvention::Cash,
            premium_currency: Currency::EUR,
            premium_lag: 2,
            exercise_lag: 2,
        }
    }

    /// Returns the GBP SONIA swaption convention.
    ///
    /// - Underlying: GBP SONIA swap
    /// - Exercise settlement: Cash
    /// - Premium currency: GBP
    /// - Premium lag: 0 days
    /// - Exercise lag: 0 days
    #[must_use]
    pub fn gbp_sonia() -> Self {
        Self {
            underlying_swap: SwapConvention::gbp_sonia(),
            premium_settlement: SettlementConvention::Cash,
            exercise_settlement: SettlementConvention::Cash,
            premium_currency: Currency::GBP,
            premium_lag: 0,
            exercise_lag: 0,
        }
    }

    /// Returns the JPY TONAR swaption convention.
    ///
    /// - Underlying: JPY TONAR swap
    /// - Exercise settlement: Cash
    /// - Premium currency: JPY
    /// - Premium lag: 2 days
    /// - Exercise lag: 2 days
    #[must_use]
    pub fn jpy_tonar() -> Self {
        Self {
            underlying_swap: SwapConvention::jpy_tonar(),
            premium_settlement: SettlementConvention::Cash,
            exercise_settlement: SettlementConvention::Cash,
            premium_currency: Currency::JPY,
            premium_lag: 2,
            exercise_lag: 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RateIndex;

    #[test]
    fn test_swaption_convention_new() {
        let conv = SwaptionConvention::new(
            SwapConvention::usd_sofr(),
            SettlementConvention::Cash,
            SettlementConvention::Physical,
            Currency::USD,
            2,
            2,
        );

        assert_eq!(conv.premium_currency, Currency::USD);
        assert_eq!(conv.premium_settlement, SettlementConvention::Cash);
        assert_eq!(conv.exercise_settlement, SettlementConvention::Physical);
        assert_eq!(conv.premium_lag, 2);
        assert_eq!(conv.exercise_lag, 2);
    }

    #[test]
    fn test_usd_sofr_swaption_convention() {
        let conv = SwaptionConvention::usd_sofr();

        assert_eq!(conv.premium_currency, Currency::USD);
        assert_eq!(conv.exercise_settlement, SettlementConvention::Cash);
        assert_eq!(conv.underlying_swap.float_index, RateIndex::Sofr);
        assert_eq!(conv.premium_lag, 2);
    }

    #[test]
    fn test_eur_euribor_swaption_convention() {
        let conv = SwaptionConvention::eur_euribor();

        assert_eq!(conv.premium_currency, Currency::EUR);
        assert_eq!(conv.underlying_swap.float_index, RateIndex::Euribor6M);
    }

    #[test]
    fn test_gbp_sonia_swaption_convention() {
        let conv = SwaptionConvention::gbp_sonia();

        assert_eq!(conv.premium_currency, Currency::GBP);
        assert_eq!(conv.underlying_swap.float_index, RateIndex::Sonia);
        assert_eq!(conv.premium_lag, 0);
    }

    #[test]
    fn test_jpy_tonar_swaption_convention() {
        let conv = SwaptionConvention::jpy_tonar();

        assert_eq!(conv.premium_currency, Currency::JPY);
        assert_eq!(conv.underlying_swap.float_index, RateIndex::Tonar);
    }

    #[test]
    fn test_swaption_convention_clone() {
        let conv = SwaptionConvention::usd_sofr();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }

    #[test]
    fn test_settlement_convention_equality() {
        assert_eq!(SettlementConvention::Cash, SettlementConvention::Cash);
        assert_ne!(SettlementConvention::Cash, SettlementConvention::Physical);
    }
}
