//! Structure of Arrays for trade data.
//!
//! Provides vectorised storage of trade parameters for batch processing.

use crate::portfolio::{Trade, TradeId};
use pricer_models::instruments::PayoffType;
use rayon::prelude::*;

/// SoA representation of trade data for vectorised operations.
///
/// Stores trade parameters in separate arrays for cache-efficient
/// access during batch computations.
///
/// # Examples
///
/// ```ignore
/// use pricer_xva::soa::TradeSoA;
///
/// let trades: Vec<&Trade> = portfolio.trades().collect();
/// let soa = TradeSoA::from_trades(&trades);
///
/// // Compute payoffs in batch
/// let spots = vec![110.0; soa.len()];
/// let mut payoffs = vec![0.0; soa.len()];
/// soa.compute_payoffs(&spots, &mut payoffs);
/// ```
#[derive(Debug, Clone)]
pub struct TradeSoA {
    /// Trade IDs for reference back to original trades
    pub trade_ids: Vec<TradeId>,
    /// Strike prices
    pub strikes: Vec<f64>,
    /// Maturities in years
    pub maturities: Vec<f64>,
    /// Notional amounts
    pub notionals: Vec<f64>,
    /// Payoff signs: +1 for Call, -1 for Put
    pub payoff_signs: Vec<i8>,
}

impl TradeSoA {
    /// Creates an empty SoA with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            trade_ids: Vec::with_capacity(capacity),
            strikes: Vec::with_capacity(capacity),
            maturities: Vec::with_capacity(capacity),
            notionals: Vec::with_capacity(capacity),
            payoff_signs: Vec::with_capacity(capacity),
        }
    }

    /// Creates SoA from a slice of trade references.
    ///
    /// Only includes vanilla options (forwards and swaps are skipped).
    pub fn from_trades(trades: &[&Trade]) -> Self {
        let mut soa = Self::with_capacity(trades.len());

        for trade in trades {
            if let Some(strike) = trade.strike() {
                let sign = match trade.payoff_type() {
                    Some(PayoffType::Call) => 1i8,
                    Some(PayoffType::Put) => -1i8,
                    _ => 1i8, // Default to call-like
                };

                soa.trade_ids.push(trade.id().clone());
                soa.strikes.push(strike);
                soa.maturities.push(trade.expiry());
                soa.notionals.push(trade.notional());
                soa.payoff_signs.push(sign);
            }
        }

        soa
    }

    /// Returns the number of trades in the SoA.
    #[inline]
    pub fn len(&self) -> usize {
        self.trade_ids.len()
    }

    /// Returns whether the SoA is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.trade_ids.is_empty()
    }

    /// Computes payoffs for all trades at given spot prices.
    ///
    /// Uses simple max(0, S - K) or max(0, K - S) payoff
    /// (without smoothing for performance).
    ///
    /// # Arguments
    ///
    /// * `spots` - Terminal spot prices (one per trade)
    /// * `payoffs` - Output buffer (must be same length as spots)
    ///
    /// # Panics
    ///
    /// Panics if `spots.len() != self.len()` or `payoffs.len() != self.len()`.
    pub fn compute_payoffs(&self, spots: &[f64], payoffs: &mut [f64]) {
        assert_eq!(spots.len(), self.len());
        assert_eq!(payoffs.len(), self.len());

        for i in 0..self.len() {
            let sign = self.payoff_signs[i] as f64;
            let intrinsic = sign * (spots[i] - self.strikes[i]);
            payoffs[i] = intrinsic.max(0.0) * self.notionals[i];
        }
    }

    /// Computes payoffs in parallel using Rayon.
    ///
    /// # Arguments
    ///
    /// * `spots` - Terminal spot prices
    ///
    /// # Returns
    ///
    /// Vector of computed payoffs.
    pub fn compute_payoffs_par(&self, spots: &[f64]) -> Vec<f64> {
        assert_eq!(spots.len(), self.len());

        (0..self.len())
            .into_par_iter()
            .map(|i| {
                let sign = self.payoff_signs[i] as f64;
                let intrinsic = sign * (spots[i] - self.strikes[i]);
                intrinsic.max(0.0) * self.notionals[i]
            })
            .collect()
    }

    /// Computes deltas for all trades (1 for ITM call, 0 for OTM call, etc.).
    ///
    /// This is a simplified delta (no smoothing).
    ///
    /// # Arguments
    ///
    /// * `spots` - Current spot prices
    ///
    /// # Returns
    ///
    /// Vector of delta values scaled by notional.
    pub fn compute_deltas(&self, spots: &[f64]) -> Vec<f64> {
        assert_eq!(spots.len(), self.len());

        (0..self.len())
            .into_par_iter()
            .map(|i| {
                let sign = self.payoff_signs[i] as f64;
                let intrinsic = sign * (spots[i] - self.strikes[i]);
                // Delta is sign if ITM, 0 if OTM
                if intrinsic > 0.0 {
                    sign * self.notionals[i]
                } else {
                    0.0
                }
            })
            .collect()
    }

    /// Returns slice of strikes.
    #[inline]
    pub fn strikes(&self) -> &[f64] {
        &self.strikes
    }

    /// Returns slice of maturities.
    #[inline]
    pub fn maturities(&self) -> &[f64] {
        &self.maturities
    }

    /// Returns slice of notionals.
    #[inline]
    pub fn notionals(&self) -> &[f64] {
        &self.notionals
    }

    /// Returns slice of payoff signs.
    #[inline]
    pub fn payoff_signs(&self) -> &[i8] {
        &self.payoff_signs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::{CounterpartyId, NettingSetId};
    use approx::assert_relative_eq;
    use pricer_core::types::Currency;
    use pricer_models::instruments::{ExerciseStyle, Instrument, InstrumentParams, VanillaOption};

    fn create_test_trade(id: &str, strike: f64, payoff: PayoffType, notional: f64) -> Trade {
        let params = InstrumentParams::new(strike, 1.0, 1.0).unwrap();
        let option = VanillaOption::new(params, payoff, ExerciseStyle::European, 1e-6);
        Trade::new(
            TradeId::new(id),
            Instrument::Vanilla(option),
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            notional,
        )
    }

    #[test]
    fn test_soa_from_trades() {
        let t1 = create_test_trade("T1", 100.0, PayoffType::Call, 1_000_000.0);
        let t2 = create_test_trade("T2", 110.0, PayoffType::Put, 500_000.0);
        let trades: Vec<&Trade> = vec![&t1, &t2];

        let soa = TradeSoA::from_trades(&trades);

        assert_eq!(soa.len(), 2);
        assert_eq!(soa.strikes(), &[100.0, 110.0]);
        assert_eq!(soa.notionals(), &[1_000_000.0, 500_000.0]);
        assert_eq!(soa.payoff_signs(), &[1, -1]);
    }

    #[test]
    fn test_soa_compute_payoffs_call() {
        let t1 = create_test_trade("T1", 100.0, PayoffType::Call, 1.0);
        let trades: Vec<&Trade> = vec![&t1];
        let soa = TradeSoA::from_trades(&trades);

        let spots = vec![110.0];
        let mut payoffs = vec![0.0];
        soa.compute_payoffs(&spots, &mut payoffs);

        assert_relative_eq!(payoffs[0], 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_soa_compute_payoffs_call_otm() {
        let t1 = create_test_trade("T1", 100.0, PayoffType::Call, 1.0);
        let trades: Vec<&Trade> = vec![&t1];
        let soa = TradeSoA::from_trades(&trades);

        let spots = vec![90.0];
        let mut payoffs = vec![0.0];
        soa.compute_payoffs(&spots, &mut payoffs);

        assert_eq!(payoffs[0], 0.0);
    }

    #[test]
    fn test_soa_compute_payoffs_put() {
        let t1 = create_test_trade("T1", 100.0, PayoffType::Put, 1.0);
        let trades: Vec<&Trade> = vec![&t1];
        let soa = TradeSoA::from_trades(&trades);

        let spots = vec![90.0];
        let mut payoffs = vec![0.0];
        soa.compute_payoffs(&spots, &mut payoffs);

        assert_relative_eq!(payoffs[0], 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_soa_compute_payoffs_put_otm() {
        let t1 = create_test_trade("T1", 100.0, PayoffType::Put, 1.0);
        let trades: Vec<&Trade> = vec![&t1];
        let soa = TradeSoA::from_trades(&trades);

        let spots = vec![110.0];
        let mut payoffs = vec![0.0];
        soa.compute_payoffs(&spots, &mut payoffs);

        assert_eq!(payoffs[0], 0.0);
    }

    #[test]
    fn test_soa_compute_payoffs_with_notional() {
        let t1 = create_test_trade("T1", 100.0, PayoffType::Call, 1_000_000.0);
        let trades: Vec<&Trade> = vec![&t1];
        let soa = TradeSoA::from_trades(&trades);

        let spots = vec![110.0];
        let mut payoffs = vec![0.0];
        soa.compute_payoffs(&spots, &mut payoffs);

        assert_relative_eq!(payoffs[0], 10_000_000.0, epsilon = 1e-10);
    }

    #[test]
    fn test_soa_compute_payoffs_par() {
        let t1 = create_test_trade("T1", 100.0, PayoffType::Call, 1.0);
        let t2 = create_test_trade("T2", 100.0, PayoffType::Put, 1.0);
        let trades: Vec<&Trade> = vec![&t1, &t2];
        let soa = TradeSoA::from_trades(&trades);

        let spots = vec![110.0, 110.0];
        let payoffs = soa.compute_payoffs_par(&spots);

        assert_relative_eq!(payoffs[0], 10.0, epsilon = 1e-10); // Call ITM
        assert_eq!(payoffs[1], 0.0); // Put OTM
    }

    #[test]
    fn test_soa_compute_deltas() {
        let t1 = create_test_trade("T1", 100.0, PayoffType::Call, 1.0);
        let t2 = create_test_trade("T2", 100.0, PayoffType::Put, 1.0);
        let trades: Vec<&Trade> = vec![&t1, &t2];
        let soa = TradeSoA::from_trades(&trades);

        // Call ITM, Put OTM
        let spots = vec![110.0, 110.0];
        let deltas = soa.compute_deltas(&spots);

        assert_relative_eq!(deltas[0], 1.0, epsilon = 1e-10); // Call delta = +1 when ITM
        assert_eq!(deltas[1], 0.0); // Put delta = 0 when OTM
    }

    #[test]
    fn test_soa_empty() {
        let trades: Vec<&Trade> = vec![];
        let soa = TradeSoA::from_trades(&trades);

        assert!(soa.is_empty());
        assert_eq!(soa.len(), 0);
    }

    #[test]
    fn test_soa_with_capacity() {
        let soa = TradeSoA::with_capacity(100);
        assert!(soa.is_empty());
    }
}
