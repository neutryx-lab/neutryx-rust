//! Mock SWIFT message receiver.

use super::{PaymentInstruction, PaymentType, SettlementStatus};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;

/// Mock SWIFT message receiver
pub struct SwiftReceiver {
    /// Received messages
    messages: Arc<RwLock<HashMap<String, SwiftMessage>>>,
    /// Message counter
    counter: Arc<RwLock<u64>>,
}

/// SWIFT message (simplified MT103/MT202 representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwiftMessage {
    /// Message reference
    pub message_ref: String,
    /// Message type (e.g., MT103, MT202)
    pub message_type: String,
    /// Sender BIC
    pub sender_bic: String,
    /// Receiver BIC
    pub receiver_bic: String,
    /// Payment instruction
    pub payment: PaymentInstruction,
    /// Status
    pub status: SettlementStatus,
    /// Received timestamp
    pub received_at: String,
    /// Raw message (mock SWIFT format)
    pub raw_message: String,
}

impl SwiftReceiver {
    /// Create a new SWIFT receiver
    pub fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(HashMap::new())),
            counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Receive a payment instruction and generate SWIFT message
    pub fn receive(&self, payment: PaymentInstruction) -> SwiftMessage {
        let message_ref = {
            let mut counter = self.counter.write().unwrap();
            *counter += 1;
            format!("SWIFT{:012}", *counter)
        };

        let message_type = match payment.payment_type {
            PaymentType::Principal | PaymentType::Interest | PaymentType::Premium => "MT103",
            PaymentType::Collateral => "MT202",
            PaymentType::Fee => "MT103",
        };

        let raw_message = self.format_swift_message(&message_ref, message_type, &payment);

        let message = SwiftMessage {
            message_ref: message_ref.clone(),
            message_type: message_type.to_string(),
            sender_bic: "NEUTRYXXXX".to_string(),
            receiver_bic: format!("{}XXXX", payment.payee.chars().take(4).collect::<String>().to_uppercase()),
            payment,
            status: SettlementStatus::Pending,
            received_at: Utc::now().to_rfc3339(),
            raw_message,
        };

        {
            let mut messages = self.messages.write().unwrap();
            messages.insert(message_ref.clone(), message.clone());
        }

        info!(
            message_ref = %message_ref,
            message_type = %message_type,
            "SWIFT message received"
        );

        message
    }

    /// Format a mock SWIFT message
    fn format_swift_message(&self, message_ref: &str, message_type: &str, payment: &PaymentInstruction) -> String {
        format!(
            r#"{{1:F01NEUTRYXXXX0000000000}}
{{2:O{msg_type}0000000000XXXXXXXXXXXX0000000000N}}
{{3:{{108:{msg_ref}}}}}
{{4:
:20:{payment_id}
:23B:CRED
:32A:{value_date}{ccy}{amount}
:50K:/{payer}
:59:/{payee}
:70:{reference}
:71A:SHA
-}}"#,
            msg_type = message_type,
            msg_ref = message_ref,
            payment_id = payment.payment_id,
            value_date = payment.value_date.replace('-', ""),
            ccy = payment.currency,
            amount = format!("{:.2}", payment.amount).replace('.', ","),
            payer = payment.payer,
            payee = payment.payee,
            reference = payment.reference
        )
    }

    /// Update message status
    pub fn update_status(&self, message_ref: &str, status: SettlementStatus) -> bool {
        let mut messages = self.messages.write().unwrap();
        if let Some(message) = messages.get_mut(message_ref) {
            message.status = status;
            true
        } else {
            false
        }
    }

    /// Get a message by reference
    pub fn get_message(&self, message_ref: &str) -> Option<SwiftMessage> {
        let messages = self.messages.read().unwrap();
        messages.get(message_ref).cloned()
    }

    /// Get all messages with a specific status
    pub fn get_messages_by_status(&self, status: SettlementStatus) -> Vec<SwiftMessage> {
        let messages = self.messages.read().unwrap();
        messages
            .values()
            .filter(|m| m.status == status)
            .cloned()
            .collect()
    }

    /// Get message statistics
    pub fn get_statistics(&self) -> MessageStatistics {
        let messages = self.messages.read().unwrap();
        let total = messages.len();
        let pending = messages.values().filter(|m| m.status == SettlementStatus::Pending).count();
        let settled = messages.values().filter(|m| m.status == SettlementStatus::Settled).count();
        let failed = messages.values().filter(|m| m.status == SettlementStatus::Failed).count();

        let total_amount: f64 = messages.values().map(|m| m.payment.amount).sum();

        MessageStatistics {
            total,
            pending,
            settled,
            failed,
            total_amount,
        }
    }
}

impl Default for SwiftReceiver {
    fn default() -> Self {
        Self::new()
    }
}

/// Message statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStatistics {
    /// Total messages
    pub total: usize,
    /// Pending messages
    pub pending: usize,
    /// Settled messages
    pub settled: usize,
    /// Failed messages
    pub failed: usize,
    /// Total amount
    pub total_amount: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swift_receiver() {
        let receiver = SwiftReceiver::new();
        let payment = PaymentInstruction {
            payment_id: "PAY001".to_string(),
            payer: "BANK_A".to_string(),
            payee: "BANK_B".to_string(),
            amount: 1_000_000.0,
            currency: "USD".to_string(),
            value_date: "2026-01-10".to_string(),
            payment_type: PaymentType::Principal,
            reference: "Trade T001".to_string(),
        };
        let message = receiver.receive(payment);
        assert!(message.message_ref.starts_with("SWIFT"));
        assert_eq!(message.message_type, "MT103");
    }
}
