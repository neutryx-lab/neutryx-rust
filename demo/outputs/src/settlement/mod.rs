//! Settlement systems.
//!
//! This module provides mock implementations of settlement
//! and payment processing systems.

mod netting_engine;
mod swift_receiver;

pub use netting_engine::NettingEngine;
pub use swift_receiver::SwiftReceiver;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Payment instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInstruction {
    /// Payment ID
    pub payment_id: String,
    /// Payer
    pub payer: String,
    /// Payee
    pub payee: String,
    /// Amount
    pub amount: f64,
    /// Currency
    pub currency: String,
    /// Value date
    pub value_date: String,
    /// Payment type
    pub payment_type: PaymentType,
    /// Reference
    pub reference: String,
}

/// Payment types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentType {
    /// Principal exchange
    Principal,
    /// Interest payment
    Interest,
    /// Collateral transfer
    Collateral,
    /// Fee payment
    Fee,
    /// Premium payment
    Premium,
}

/// Settlement status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementStatus {
    /// Pending
    Pending,
    /// Matched
    Matched,
    /// Settled
    Settled,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
}
