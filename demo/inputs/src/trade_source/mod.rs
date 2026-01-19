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
        /// Underlying asset identifier
        underlying: String,
        /// Strike price
        strike: f64,
        /// True for call option, false for put
        is_call: bool,
    },
    /// Forward parameters
    Forward {
        /// Underlying asset identifier
        underlying: String,
        /// Forward price
        forward_price: f64,
    },
    /// IRS parameters
    InterestRateSwap {
        /// Fixed rate of the swap
        fixed_rate: f64,
        /// Floating rate index (e.g., "LIBOR", "SOFR")
        float_index: String,
        /// True if paying fixed rate
        pay_fixed: bool,
    },
    /// FX forward parameters
    FxForward {
        /// Currency to buy
        buy_currency: String,
        /// Currency to sell
        sell_currency: String,
        /// Exchange rate
        rate: f64,
    },
    /// FX option parameters
    FxOption {
        /// Currency pair (e.g., "EURUSD")
        currency_pair: String,
        /// Strike price
        strike: f64,
        /// True for call option, false for put
        is_call: bool,
    },
    /// CDS parameters
    CreditDefaultSwap {
        /// Reference entity identifier
        reference_entity: String,
        /// Credit spread in basis points
        spread_bps: f64,
        /// True if buying protection
        is_protection_buyer: bool,
    },
}
