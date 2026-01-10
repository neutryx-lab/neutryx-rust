//! Netting engine for payment consolidation.

use super::{PaymentInstruction, PaymentType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Netting engine for consolidating payments
pub struct NettingEngine {
    /// Payments by netting set
    payments: HashMap<String, Vec<PaymentInstruction>>,
}

/// Netted payment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NettedPayment {
    /// Netting set ID
    pub netting_set_id: String,
    /// Net payer
    pub payer: String,
    /// Net payee
    pub payee: String,
    /// Net amount
    pub net_amount: f64,
    /// Currency
    pub currency: String,
    /// Value date
    pub value_date: String,
    /// Original payment count
    pub original_count: usize,
    /// Original gross amount
    pub gross_amount: f64,
    /// Netting efficiency
    pub netting_efficiency: f64,
}

impl NettingEngine {
    /// Create a new netting engine
    pub fn new() -> Self {
        Self {
            payments: HashMap::new(),
        }
    }

    /// Add a payment to a netting set
    pub fn add_payment(&mut self, netting_set_id: &str, payment: PaymentInstruction) {
        self.payments
            .entry(netting_set_id.to_string())
            .or_default()
            .push(payment);
    }

    /// Add multiple payments
    pub fn add_payments(&mut self, netting_set_id: &str, payments: Vec<PaymentInstruction>) {
        for payment in payments {
            self.add_payment(netting_set_id, payment);
        }
    }

    /// Calculate netted payments for a netting set
    pub fn calculate_net(&self, netting_set_id: &str) -> Option<Vec<NettedPayment>> {
        let payments = self.payments.get(netting_set_id)?;

        // Group by currency and value date
        let mut by_ccy_date: HashMap<(String, String), Vec<&PaymentInstruction>> = HashMap::new();
        for payment in payments {
            by_ccy_date
                .entry((payment.currency.clone(), payment.value_date.clone()))
                .or_default()
                .push(payment);
        }

        let mut results = Vec::new();

        for ((currency, value_date), group) in by_ccy_date {
            // Calculate net by counterparty pair
            let mut net_by_pair: HashMap<(String, String), f64> = HashMap::new();
            let mut gross_amount = 0.0;

            for payment in &group {
                gross_amount += payment.amount.abs();
                
                // Normalize pair (always smaller first)
                let (party1, party2) = if payment.payer < payment.payee {
                    (payment.payer.clone(), payment.payee.clone())
                } else {
                    (payment.payee.clone(), payment.payer.clone())
                };

                let amount = if payment.payer < payment.payee {
                    payment.amount
                } else {
                    -payment.amount
                };

                *net_by_pair.entry((party1, party2)).or_insert(0.0) += amount;
            }

            // Create netted payments
            for ((party1, party2), net) in net_by_pair {
                if net.abs() > 0.01 {
                    let (payer, payee, amount) = if net > 0.0 {
                        (party1, party2, net)
                    } else {
                        (party2, party1, -net)
                    };

                    let netting_efficiency = if gross_amount > 0.0 {
                        1.0 - (amount / gross_amount)
                    } else {
                        0.0
                    };

                    results.push(NettedPayment {
                        netting_set_id: netting_set_id.to_string(),
                        payer,
                        payee,
                        net_amount: amount,
                        currency: currency.clone(),
                        value_date: value_date.clone(),
                        original_count: group.len(),
                        gross_amount,
                        netting_efficiency,
                    });
                }
            }
        }

        Some(results)
    }

    /// Calculate all netted payments
    pub fn calculate_all_nets(&self) -> Vec<NettedPayment> {
        let mut all_results = Vec::new();
        for netting_set_id in self.payments.keys() {
            if let Some(results) = self.calculate_net(netting_set_id) {
                all_results.extend(results);
            }
        }
        all_results
    }

    /// Get netting statistics
    pub fn get_statistics(&self) -> NettingStatistics {
        let all_nets = self.calculate_all_nets();
        
        let total_gross: f64 = all_nets.iter().map(|n| n.gross_amount).sum();
        let total_net: f64 = all_nets.iter().map(|n| n.net_amount).sum();
        let original_count: usize = all_nets.iter().map(|n| n.original_count).sum();
        let netted_count = all_nets.len();

        NettingStatistics {
            netting_set_count: self.payments.len(),
            original_payment_count: original_count,
            netted_payment_count: netted_count,
            gross_amount: total_gross,
            net_amount: total_net,
            overall_efficiency: if total_gross > 0.0 {
                1.0 - (total_net / total_gross)
            } else {
                0.0
            },
        }
    }

    /// Clear all payments
    pub fn clear(&mut self) {
        self.payments.clear();
    }
}

impl Default for NettingEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Netting statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NettingStatistics {
    /// Number of netting sets
    pub netting_set_count: usize,
    /// Original payment count
    pub original_payment_count: usize,
    /// Netted payment count
    pub netted_payment_count: usize,
    /// Gross amount
    pub gross_amount: f64,
    /// Net amount
    pub net_amount: f64,
    /// Overall netting efficiency
    pub overall_efficiency: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netting_engine() {
        let mut engine = NettingEngine::new();
        
        engine.add_payment("NS001", PaymentInstruction {
            payment_id: "P1".to_string(),
            payer: "A".to_string(),
            payee: "B".to_string(),
            amount: 1000.0,
            currency: "USD".to_string(),
            value_date: "2026-01-10".to_string(),
            payment_type: PaymentType::Interest,
            reference: "T1".to_string(),
        });
        
        engine.add_payment("NS001", PaymentInstruction {
            payment_id: "P2".to_string(),
            payer: "B".to_string(),
            payee: "A".to_string(),
            amount: 600.0,
            currency: "USD".to_string(),
            value_date: "2026-01-10".to_string(),
            payment_type: PaymentType::Interest,
            reference: "T2".to_string(),
        });

        let nets = engine.calculate_net("NS001").unwrap();
        assert_eq!(nets.len(), 1);
        assert!((nets[0].net_amount - 400.0).abs() < 0.01);
    }
}
