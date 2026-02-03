//! Trade definition types.
//!
//! This module provides the main Trade struct and related types.
//!
//! Uses `bon::Builder` for fluent construction with compile-time safety.

use bon::Builder;

use super::{
    cashflow::Cashflow,
    leg::{Leg, LegType},
};
use crate::{
    ids::{BookId, CounterpartyId, IssuerId, PortfolioId, TradeId},
    Date,
};

/// Type of option exercise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExerciseType {
    /// European: exercise only at expiry.
    European,
    /// Bermudan: exercise at specific dates.
    Bermudan,
    /// American: exercise at any time until expiry.
    American,
}

/// Type of settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SettlementType {
    /// Cash settlement.
    Cash,
    /// Physical delivery.
    Physical,
}

/// Type of trade.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TradeType {
    // ========================================
    // Rates
    // ========================================
    /// Money market deposit.
    Deposit,

    /// Forward Rate Agreement.
    Fra,

    /// Interest rate futures.
    Futures,

    /// Interest rate swap.
    Swap,

    /// Overnight Index Swap.
    Ois,

    /// Basis swap (float-float).
    BasisSwap,

    /// Cross-currency swap.
    CrossCurrencySwap,

    /// Swaption (option on a swap).
    Swaption {
        /// Exercise dates.
        exercise_dates: Vec<Date>,
        /// Type of exercise.
        exercise_type: ExerciseType,
        /// Settlement method.
        settlement_type: SettlementType,
    },

    /// Bond or fixed income security.
    Bond {
        /// Issuer identifier.
        issuer_id: Option<IssuerId>,
        /// Seniority level.
        seniority: Option<String>,
    },

    /// Cap or floor.
    CapFloor,

    // ========================================
    // FX
    // ========================================
    /// FX spot transaction.
    FxSpot,

    /// FX forward or spot.
    FxForward,

    /// FX swap (near + far legs).
    FxSwap,

    /// FX option (vanilla).
    FxOption {
        /// Option type (Call/Put).
        option_type: super::OptionType,
        /// Strike price.
        strike: f64,
        /// Exercise style.
        exercise_type: ExerciseType,
        /// Settlement method.
        settlement_type: SettlementType,
        /// Expiry date.
        expiry_date: Date,
    },

    /// FX barrier option.
    FxBarrierOption {
        /// Option type (Call/Put).
        option_type: super::OptionType,
        /// Strike price.
        strike: f64,
        /// Barrier level.
        barrier: f64,
        /// Barrier type.
        barrier_type: BarrierType,
        /// Exercise style.
        exercise_type: ExerciseType,
        /// Expiry date.
        expiry_date: Date,
    },

    // ========================================
    // Equity
    // ========================================
    /// Equity forward.
    EquityForward {
        /// Underlying equity ticker/identifier.
        underlyer: String,
        /// Forward price.
        forward_price: f64,
        /// Settlement date.
        settlement_date: Date,
    },

    /// Equity option (vanilla).
    EquityOption {
        /// Underlying equity ticker/identifier.
        underlyer: String,
        /// Option type (Call/Put).
        option_type: super::OptionType,
        /// Strike price.
        strike: f64,
        /// Exercise style.
        exercise_type: ExerciseType,
        /// Settlement method.
        settlement_type: SettlementType,
        /// Expiry date.
        expiry_date: Date,
        /// Number of shares per contract.
        contract_multiplier: f64,
    },

    /// Equity swap (total return swap).
    EquitySwap {
        /// Underlying equity/index.
        underlyer: String,
    },

    // ========================================
    // Credit
    // ========================================
    /// Credit default swap (single name).
    CreditDefaultSwap {
        /// Reference entity name.
        reference_entity: String,
        /// Reference entity identifier (RED code).
        entity_id: Option<String>,
        /// Protection buyer or seller.
        protection_side: ProtectionSide,
    },

    /// Credit default swap index (CDX, iTraxx).
    CreditDefaultSwapIndex {
        /// Index name (e.g., "CDX.NA.IG").
        index_name: String,
        /// Index series.
        series: u32,
        /// Index version.
        version: Option<u32>,
        /// Protection buyer or seller.
        protection_side: ProtectionSide,
    },

    /// Credit default swap option.
    CreditDefaultSwapOption {
        /// Underlying CDS reference entity.
        reference_entity: String,
        /// Option type (payer/receiver).
        option_type: super::OptionType,
        /// Exercise style.
        exercise_type: ExerciseType,
        /// Expiry date.
        expiry_date: Date,
    },

    // ========================================
    // Commodity
    // ========================================
    /// Commodity forward.
    CommodityForward {
        /// Commodity name/code.
        commodity: String,
        /// Delivery date.
        delivery_date: Date,
        /// Forward price.
        forward_price: f64,
        /// Quantity.
        quantity: f64,
        /// Quantity unit (e.g., "BBL", "MT").
        quantity_unit: String,
    },

    /// Commodity swap.
    CommoditySwap {
        /// Commodity name/code.
        commodity: String,
        /// Fixed price.
        fixed_price: f64,
        /// Price unit.
        price_unit: String,
        /// Total quantity.
        total_quantity: f64,
        /// Quantity unit.
        quantity_unit: String,
    },

    /// Commodity option.
    CommodityOption {
        /// Commodity name/code.
        commodity: String,
        /// Option type (Call/Put).
        option_type: super::OptionType,
        /// Strike price.
        strike: f64,
        /// Exercise style.
        exercise_type: ExerciseType,
        /// Expiry date.
        expiry_date: Date,
        /// Quantity.
        quantity: f64,
        /// Quantity unit.
        quantity_unit: String,
    },

    /// Generic trade (catch-all).
    #[default]
    Generic,
}

/// Barrier type for barrier options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BarrierType {
    /// Up-and-in: option activates when price goes above barrier.
    UpAndIn,
    /// Up-and-out: option deactivates when price goes above barrier.
    UpAndOut,
    /// Down-and-in: option activates when price goes below barrier.
    DownAndIn,
    /// Down-and-out: option deactivates when price goes below barrier.
    DownAndOut,
}

/// Protection side for credit derivatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProtectionSide {
    /// Protection buyer (pays premium, receives protection).
    Buyer,
    /// Protection seller (receives premium, provides protection).
    Seller,
}

impl TradeType {
    /// Returns true if this is a swap (IRS, OIS, or basis).
    #[must_use]
    pub fn is_swap(&self) -> bool {
        matches!(
            self,
            TradeType::Swap | TradeType::Ois | TradeType::BasisSwap
        )
    }

    /// Returns true if this is a swaption.
    #[must_use]
    pub fn is_swaption(&self) -> bool { matches!(self, TradeType::Swaption { .. }) }

    /// Returns true if this is a bond.
    #[must_use]
    pub fn is_bond(&self) -> bool { matches!(self, TradeType::Bond { .. }) }

    /// Returns true if this is an FX product.
    #[must_use]
    pub fn is_fx(&self) -> bool {
        matches!(
            self,
            TradeType::FxSpot
                | TradeType::FxForward
                | TradeType::FxSwap
                | TradeType::FxOption { .. }
                | TradeType::FxBarrierOption { .. }
        )
    }

    /// Returns true if this is an equity product.
    #[must_use]
    pub fn is_equity(&self) -> bool {
        matches!(
            self,
            TradeType::EquityForward { .. }
                | TradeType::EquityOption { .. }
                | TradeType::EquitySwap { .. }
        )
    }

    /// Returns true if this is a credit product.
    #[must_use]
    pub fn is_credit(&self) -> bool {
        matches!(
            self,
            TradeType::CreditDefaultSwap { .. }
                | TradeType::CreditDefaultSwapIndex { .. }
                | TradeType::CreditDefaultSwapOption { .. }
        )
    }

    /// Returns true if this is a commodity product.
    #[must_use]
    pub fn is_commodity(&self) -> bool {
        matches!(
            self,
            TradeType::CommodityForward { .. }
                | TradeType::CommoditySwap { .. }
                | TradeType::CommodityOption { .. }
        )
    }

    /// Returns true if this is an option.
    #[must_use]
    pub fn is_option(&self) -> bool {
        matches!(
            self,
            TradeType::Swaption { .. }
                | TradeType::CapFloor
                | TradeType::FxOption { .. }
                | TradeType::FxBarrierOption { .. }
                | TradeType::EquityOption { .. }
                | TradeType::CreditDefaultSwapOption { .. }
                | TradeType::CommodityOption { .. }
        )
    }
}

/// Trade metadata.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeMetadata {
    /// Date the trade was executed.
    pub trade_date: Option<Date>,
    /// Counterparty identifier.
    pub counterparty: Option<CounterpartyId>,
    /// Portfolio identifier.
    pub portfolio: Option<PortfolioId>,
    /// Trading book identifier.
    pub book: Option<BookId>,
}

impl TradeMetadata {
    /// Creates new empty metadata.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Sets the trade date.
    #[must_use]
    pub fn with_trade_date(mut self, date: Date) -> Self {
        self.trade_date = Some(date);
        self
    }

    /// Sets the counterparty.
    #[must_use]
    pub fn with_counterparty(mut self, counterparty: impl Into<CounterpartyId>) -> Self {
        self.counterparty = Some(counterparty.into());
        self
    }

    /// Sets the portfolio.
    #[must_use]
    pub fn with_portfolio(mut self, portfolio: impl Into<PortfolioId>) -> Self {
        self.portfolio = Some(portfolio.into());
        self
    }

    /// Sets the book.
    #[must_use]
    pub fn with_book(mut self, book: impl Into<BookId>) -> Self {
        self.book = Some(book.into());
        self
    }
}

/// A financial trade.
///
/// Represents a complete trade as a collection of legs with metadata.
///
/// Uses `bon::Builder` for fluent construction with compile-time safety.
///
/// # Examples
///
/// ```ignore
/// use infra_master::trade::{Trade, TradeType, Leg};
///
/// let trade = Trade::builder()
///     .id("TRADE001")
///     .legs(vec![fixed_leg, floating_leg])
///     .trade_type(TradeType::Swap)
///     .build();
/// ```
#[derive(Debug, Clone, PartialEq, Builder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Trade {
    /// Unique identifier for this trade.
    #[builder(into)]
    pub id: TradeId,
    /// Legs comprising this trade.
    #[builder(default)]
    legs: Vec<Leg>,
    /// Type of trade.
    #[builder(default)]
    pub trade_type: TradeType,
    /// Additional metadata.
    #[builder(default)]
    pub metadata: TradeMetadata,
}

impl Trade {
    /// Creates a new trade.
    ///
    /// Convenience constructor. For full control, use `Trade::builder()`.
    #[must_use]
    pub fn new(id: impl Into<TradeId>, legs: Vec<Leg>, trade_type: TradeType) -> Self {
        Self::builder()
            .id(id)
            .legs(legs)
            .trade_type(trade_type)
            .build()
    }

    /// Creates a new trade with metadata.
    ///
    /// Convenience constructor. For full control, use `Trade::builder()`.
    #[must_use]
    pub fn with_metadata(
        id: impl Into<TradeId>,
        legs: Vec<Leg>,
        trade_type: TradeType,
        metadata: TradeMetadata,
    ) -> Self {
        Self::builder()
            .id(id)
            .legs(legs)
            .trade_type(trade_type)
            .metadata(metadata)
            .build()
    }

    /// Returns an iterator over all legs in this trade.
    pub fn legs(&self) -> impl Iterator<Item = &Leg> { self.legs.iter() }

    /// Returns the number of legs in this trade.
    #[must_use]
    pub fn num_legs(&self) -> usize { self.legs.len() }

    /// Returns an iterator over all cashflows in all legs.
    pub fn all_cashflows(&self) -> impl Iterator<Item = &Cashflow> {
        self.legs.iter().flat_map(|leg| leg.cashflows())
    }

    /// Returns an iterator over future cashflows in all legs.
    pub fn future_cashflows(&self, ref_date: Date) -> impl Iterator<Item = &Cashflow> {
        self.legs
            .iter()
            .flat_map(move |leg| leg.future_cashflows(ref_date))
    }

    /// Returns the total number of cashflows across all legs.
    #[must_use]
    pub fn total_cashflows(&self) -> usize { self.legs.iter().map(Leg::len).sum() }

    /// Returns true if this is a vanilla swap (exactly 2 legs: one fixed, one
    /// floating).
    #[must_use]
    pub fn is_vanilla_swap(&self) -> bool {
        if !self.trade_type.is_swap() || self.legs.len() != 2 {
            return false;
        }

        let has_fixed = self.legs.iter().any(|leg| leg.leg_type == LegType::Fixed);
        let has_floating = self
            .legs
            .iter()
            .any(|leg| leg.leg_type == LegType::Floating);

        has_fixed && has_floating
    }

    /// Returns the first leg if present.
    #[must_use]
    pub fn first_leg(&self) -> Option<&Leg> { self.legs.first() }

    /// Returns the fixed leg if this is a swap with exactly one fixed leg.
    #[must_use]
    pub fn fixed_leg(&self) -> Option<&Leg> {
        self.legs.iter().find(|leg| leg.leg_type == LegType::Fixed)
    }

    /// Returns the floating leg if this is a swap with exactly one floating
    /// leg.
    #[must_use]
    pub fn floating_leg(&self) -> Option<&Leg> {
        self.legs
            .iter()
            .find(|leg| leg.leg_type == LegType::Floating)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        trade::{CashflowType, Direction, Payoff},
        Currency,
    };

    fn make_fixed_leg() -> Leg {
        let cashflows = vec![Cashflow::new(
            CashflowType::Coupon,
            Date::from_ymd(2025, 7, 1).unwrap(),
            Date::from_ymd(2025, 1, 1).unwrap(),
            Date::from_ymd(2025, 7, 1).unwrap(),
            0.5,
            1_000_000.0,
            Payoff::fixed(0.05),
            Currency::USD,
        )];

        Leg::new(
            cashflows,
            Direction::Receiver,
            LegType::Fixed,
            Currency::USD,
        )
    }

    fn make_floating_leg() -> Leg {
        use crate::{trade::IndexType, RateIndex};

        let cashflows = vec![Cashflow::new(
            CashflowType::Coupon,
            Date::from_ymd(2025, 7, 1).unwrap(),
            Date::from_ymd(2025, 1, 1).unwrap(),
            Date::from_ymd(2025, 7, 1).unwrap(),
            0.5,
            1_000_000.0,
            Payoff::floating(IndexType::Rate(RateIndex::Sofr)),
            Currency::USD,
        )];

        Leg::new(
            cashflows,
            Direction::Payer,
            LegType::Floating,
            Currency::USD,
        )
    }

    #[test]
    fn test_exercise_type() {
        assert_eq!(ExerciseType::European, ExerciseType::European);
        assert_ne!(ExerciseType::European, ExerciseType::American);
    }

    #[test]
    fn test_settlement_type() {
        assert_eq!(SettlementType::Cash, SettlementType::Cash);
        assert_ne!(SettlementType::Cash, SettlementType::Physical);
    }

    #[test]
    fn test_trade_type_is_swap() {
        assert!(TradeType::Swap.is_swap());
        assert!(!TradeType::CapFloor.is_swap());
    }

    #[test]
    fn test_trade_type_is_swaption() {
        let swaption = TradeType::Swaption {
            exercise_dates: vec![Date::from_ymd(2025, 1, 1).unwrap()],
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
        };
        assert!(swaption.is_swaption());
        assert!(!TradeType::Swap.is_swaption());
    }

    #[test]
    fn test_trade_type_is_bond() {
        let bond = TradeType::Bond {
            issuer_id: Some(IssuerId::new("ABC")),
            seniority: Some("Senior".into()),
        };
        assert!(bond.is_bond());
        assert!(!TradeType::Swap.is_bond());
    }

    #[test]
    fn test_trade_metadata_builder() {
        let metadata = TradeMetadata::new()
            .with_trade_date(Date::from_ymd(2025, 1, 1).unwrap())
            .with_counterparty("Bank A")
            .with_portfolio("Portfolio1")
            .with_book("Trading");

        assert_eq!(
            metadata.trade_date,
            Some(Date::from_ymd(2025, 1, 1).unwrap())
        );
        assert_eq!(metadata.counterparty, Some(CounterpartyId::new("Bank A")));
        assert_eq!(metadata.portfolio, Some(PortfolioId::new("Portfolio1")));
        assert_eq!(metadata.book, Some(BookId::new("Trading")));
    }

    #[test]
    fn test_trade_new() {
        let legs = vec![make_fixed_leg()];
        let trade = Trade::new("TRADE001", legs, TradeType::Generic);

        assert_eq!(trade.id.as_str(), "TRADE001");
        assert_eq!(trade.num_legs(), 1);
    }

    #[test]
    fn test_trade_with_metadata() {
        let legs = vec![make_fixed_leg()];
        let metadata = TradeMetadata::new().with_counterparty("Bank B");
        let trade = Trade::with_metadata("TRADE002", legs, TradeType::Generic, metadata);

        assert_eq!(
            trade.metadata.counterparty,
            Some(CounterpartyId::new("Bank B"))
        );
    }

    #[test]
    fn test_trade_legs() {
        let legs = vec![make_fixed_leg(), make_floating_leg()];
        let trade = Trade::new("SWAP001", legs, TradeType::Swap);

        assert_eq!(trade.legs().count(), 2);
    }

    #[test]
    fn test_trade_all_cashflows() {
        let legs = vec![make_fixed_leg(), make_floating_leg()];
        let trade = Trade::new("SWAP001", legs, TradeType::Swap);

        assert_eq!(trade.all_cashflows().count(), 2);
    }

    #[test]
    fn test_trade_future_cashflows() {
        let legs = vec![make_fixed_leg(), make_floating_leg()];
        let trade = Trade::new("SWAP001", legs, TradeType::Swap);
        let ref_date = Date::from_ymd(2025, 1, 1).unwrap();

        assert_eq!(trade.future_cashflows(ref_date).count(), 2);
    }

    #[test]
    fn test_trade_total_cashflows() {
        let legs = vec![make_fixed_leg(), make_floating_leg()];
        let trade = Trade::new("SWAP001", legs, TradeType::Swap);

        assert_eq!(trade.total_cashflows(), 2);
    }

    #[test]
    fn test_trade_is_vanilla_swap_true() {
        let legs = vec![make_fixed_leg(), make_floating_leg()];
        let trade = Trade::new("SWAP001", legs, TradeType::Swap);

        assert!(trade.is_vanilla_swap());
    }

    #[test]
    fn test_trade_is_vanilla_swap_false_wrong_type() {
        let legs = vec![make_fixed_leg(), make_floating_leg()];
        let trade = Trade::new("TRADE001", legs, TradeType::Generic);

        assert!(!trade.is_vanilla_swap());
    }

    #[test]
    fn test_trade_is_vanilla_swap_false_single_leg() {
        let legs = vec![make_fixed_leg()];
        let trade = Trade::new("SWAP001", legs, TradeType::Swap);

        assert!(!trade.is_vanilla_swap());
    }

    #[test]
    fn test_trade_is_vanilla_swap_false_both_fixed() {
        let legs = vec![make_fixed_leg(), make_fixed_leg()];
        let trade = Trade::new("SWAP001", legs, TradeType::Swap);

        assert!(!trade.is_vanilla_swap());
    }

    #[test]
    fn test_trade_first_leg() {
        let legs = vec![make_fixed_leg()];
        let trade = Trade::new("TRADE001", legs, TradeType::Generic);

        assert!(trade.first_leg().is_some());
        assert_eq!(trade.first_leg().unwrap().leg_type, LegType::Fixed);
    }

    #[test]
    fn test_trade_fixed_leg() {
        let legs = vec![make_fixed_leg(), make_floating_leg()];
        let trade = Trade::new("SWAP001", legs, TradeType::Swap);

        let fixed = trade.fixed_leg();
        assert!(fixed.is_some());
        assert_eq!(fixed.unwrap().leg_type, LegType::Fixed);
    }

    #[test]
    fn test_trade_floating_leg() {
        let legs = vec![make_fixed_leg(), make_floating_leg()];
        let trade = Trade::new("SWAP001", legs, TradeType::Swap);

        let floating = trade.floating_leg();
        assert!(floating.is_some());
        assert_eq!(floating.unwrap().leg_type, LegType::Floating);
    }

    #[test]
    fn test_trade_clone() {
        let legs = vec![make_fixed_leg()];
        let trade = Trade::new("TRADE001", legs, TradeType::Generic);
        let cloned = trade.clone();
        assert_eq!(trade, cloned);
    }

    #[test]
    fn test_exercise_type_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ExerciseType::European);
        set.insert(ExerciseType::Bermudan);
        set.insert(ExerciseType::European); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_settlement_type_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(SettlementType::Cash);
        set.insert(SettlementType::Physical);
        set.insert(SettlementType::Cash); // Duplicate
        assert_eq!(set.len(), 2);
    }
}
