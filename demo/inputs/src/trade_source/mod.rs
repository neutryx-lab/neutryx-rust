//! Trade source simulations.
//!
//! This module provides mock implementations of trade booking systems
//! that generate trade data for the Neutryx adapter layer.

mod fpml_generator;
mod front_office;

pub use fpml_generator::FpmlGenerator;
pub use front_office::FrontOffice;

/// Trait for trade sources
pub trait TradeSource: Send + Sync {
    /// Generate a batch of trades
    fn generate_trades(&self, count: usize) -> Vec<TradeRecord>;
}

/// A trade record from the booking system
#[derive(Debug, Clone)]
pub struct TradeRecord {
    /// Trade ID
    pub trade_id: String,
    /// Instrument type
    pub instrument_type: InstrumentType,
    /// Counterparty ID
    pub counterparty_id: String,
    /// Netting set ID
    pub netting_set_id: String,
    /// Notional amount
    pub notional: f64,
    /// Currency
    pub currency: String,
    /// Trade date
    pub trade_date: String,
    /// Maturity date
    pub maturity_date: String,
    /// Additional parameters
    pub params: TradeParams,
}

/// Instrument type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentType {
    /// Equity vanilla option
    EquityOption,
    /// Equity forward
    EquityForward,
    /// Interest rate swap
    InterestRateSwap,
    /// FX forward
    FxForward,
    /// FX option
    FxOption,
    /// Credit default swap
    CreditDefaultSwap,
}

/// Trade-specific parameters
#[derive(Debug, Clone)]
pub enum TradeParams {
    /// Equity option parameters
    EquityOption {
        underlying: String,
        strike: f64,
        is_call: bool,
    },
    /// Forward parameters
    Forward {
        underlying: String,
        forward_price: f64,
    },
    /// IRS parameters
    InterestRateSwap {
        fixed_rate: f64,
        float_index: String,
        pay_fixed: bool,
    },
    /// FX forward parameters
    FxForward {
        buy_currency: String,
        sell_currency: String,
        rate: f64,
    },
    /// FX option parameters
    FxOption {
        currency_pair: String,
        strike: f64,
        is_call: bool,
    },
    /// CDS parameters
    CreditDefaultSwap {
        reference_entity: String,
        spread_bps: f64,
        is_protection_buyer: bool,
    },
}
