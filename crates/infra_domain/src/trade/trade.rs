//! Trade definition types.

use bon::Builder;

use super::{
    cashflow::Cashflow,
    leg::{Leg, LegType},
};
use crate::{
    ids::{BookId, CounterpartyId, IssuerId, PortfolioId, TradeId},
    time::Date,
};

/// Type of option exercise (alias for `ExerciseStyle`).
pub type ExerciseType = crate::market::instrument::ExerciseStyle;

/// Type of settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SettlementType {
    /// Cash settlement.
    Cash,
    /// Physical delivery.
    Physical,
}

/// Type of trade.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum TradeType {
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
        /// Barrier type (knock-in or knock-out).
        barrier_type: crate::market::instrument::BarrierType,
        /// Barrier direction (up or down).
        barrier_direction: crate::market::instrument::BarrierDirection,
        /// Exercise style.
        exercise_type: ExerciseType,
        /// Expiry date.
        expiry_date: Date,
    },

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

/// Protection side for credit derivatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Builder, serde::Serialize, serde::Deserialize)]
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
    #[must_use]
    pub fn new(id: impl Into<TradeId>, legs: Vec<Leg>, trade_type: TradeType) -> Self {
        Self::builder()
            .id(id)
            .legs(legs)
            .trade_type(trade_type)
            .build()
    }

    /// Creates a new trade with metadata.
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

    /// Returns true if this is a vanilla swap (exactly 2 legs: one fixed, one.
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

    /// Returns the floating leg if this is a swap with exactly one floating.
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
        market::Currency,
        trade::{CashflowType, Direction, Payoff},
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
        use crate::{market::RateIndex, trade::IndexType};
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
    fn test_trade_type_classification() {
        assert!(TradeType::Swap.is_swap());
        assert!(TradeType::Ois.is_swap());
        assert!(!TradeType::CapFloor.is_swap());

        let swaption = TradeType::Swaption {
            exercise_dates: vec![Date::from_ymd(2025, 1, 1).unwrap()],
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
        };
        assert!(swaption.is_swaption());
        assert!(swaption.is_option());

        assert!(TradeType::FxSpot.is_fx());
        assert!(TradeType::EquitySwap {
            underlyer: "SPX".into()
        }
        .is_equity());
        assert!(TradeType::CreditDefaultSwap {
            reference_entity: "X".into(),
            entity_id: None,
            protection_side: ProtectionSide::Buyer,
        }
        .is_credit());
        assert!(TradeType::CommodityForward {
            commodity: "WTI".into(),
            delivery_date: Date::from_ymd(2025, 6, 1).unwrap(),
            forward_price: 80.0,
            quantity: 1000.0,
            quantity_unit: "BBL".into(),
        }
        .is_commodity());
    }

    #[test]
    fn test_trade_metadata() {
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
    fn test_trade_construction_and_legs() {
        let legs = vec![make_fixed_leg(), make_floating_leg()];
        let trade = Trade::new("SWAP001", legs, TradeType::Swap);

        assert_eq!(trade.id.as_str(), "SWAP001");
        assert_eq!(trade.num_legs(), 2);
        assert_eq!(trade.all_cashflows().count(), 2);
        assert_eq!(trade.total_cashflows(), 2);
        assert!(trade.fixed_leg().is_some());
        assert!(trade.floating_leg().is_some());
    }

    #[test]
    fn test_vanilla_swap_detection() {
        let swap = Trade::new(
            "S1",
            vec![make_fixed_leg(), make_floating_leg()],
            TradeType::Swap,
        );
        assert!(swap.is_vanilla_swap());

        let generic = Trade::new(
            "G1",
            vec![make_fixed_leg(), make_floating_leg()],
            TradeType::Generic,
        );
        assert!(!generic.is_vanilla_swap());

        let single = Trade::new("S2", vec![make_fixed_leg()], TradeType::Swap);
        assert!(!single.is_vanilla_swap());

        let both_fixed = Trade::new(
            "S3",
            vec![make_fixed_leg(), make_fixed_leg()],
            TradeType::Swap,
        );
        assert!(!both_fixed.is_vanilla_swap());
    }

    #[test]
    fn test_trade_with_metadata() {
        let metadata = TradeMetadata::new().with_counterparty("Bank B");
        let trade =
            Trade::with_metadata("T002", vec![make_fixed_leg()], TradeType::Generic, metadata);
        assert_eq!(
            trade.metadata.counterparty,
            Some(CounterpartyId::new("Bank B"))
        );
    }
}
