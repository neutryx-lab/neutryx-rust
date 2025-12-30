//! Netting set structures with collateral agreements.
//!
//! This module provides netting set definitions for grouping trades
//! and managing collateral agreements.

use pricer_core::types::Currency;

use super::error::PortfolioError;
use super::ids::{CounterpartyId, NettingSetId, TradeId};

/// Collateral agreement parameters.
///
/// Defines the terms of a Credit Support Annex (CSA) or similar
/// collateral arrangement between counterparties.
///
/// # Examples
///
/// ```
/// use pricer_xva::portfolio::CollateralAgreement;
/// use pricer_core::types::Currency;
///
/// let csa = CollateralAgreement::new(
///     1_000_000.0,  // threshold
///     500_000.0,    // minimum transfer amount
///     0.0,          // independent amount
///     Currency::USD,
///     CollateralAgreement::bilateral_mpor(),
/// ).unwrap();
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CollateralAgreement {
    /// Threshold amount below which no collateral is required
    threshold: f64,
    /// Minimum transfer amount
    mta: f64,
    /// Independent amount (positive = we post, negative = we receive)
    independent_amount: f64,
    /// Collateral currency
    currency: Currency,
    /// Margin period of risk in years
    mpor: f64,
}

impl CollateralAgreement {
    /// Standard bilateral margin period of risk (10 business days).
    ///
    /// Assumes 252 business days per year.
    #[inline]
    pub fn bilateral_mpor() -> f64 {
        10.0 / 252.0
    }

    /// Standard cleared margin period of risk (5 business days).
    ///
    /// Assumes 252 business days per year.
    #[inline]
    pub fn cleared_mpor() -> f64 {
        5.0 / 252.0
    }

    /// Creates a new collateral agreement.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Threshold amount (must be non-negative)
    /// * `mta` - Minimum transfer amount (must be non-negative)
    /// * `independent_amount` - Independent amount (can be positive or negative)
    /// * `currency` - Collateral currency
    /// * `mpor` - Margin period of risk in years (must be positive)
    ///
    /// # Errors
    ///
    /// Returns `PortfolioError::InvalidCollateralAgreement` if:
    /// - Threshold is negative
    /// - MTA is negative
    /// - MPoR is not positive
    pub fn new(
        threshold: f64,
        mta: f64,
        independent_amount: f64,
        currency: Currency,
        mpor: f64,
    ) -> Result<Self, PortfolioError> {
        if threshold < 0.0 {
            return Err(PortfolioError::InvalidCollateralAgreement(
                "Threshold must be non-negative".to_string(),
            ));
        }
        if mta < 0.0 {
            return Err(PortfolioError::InvalidCollateralAgreement(
                "Minimum transfer amount must be non-negative".to_string(),
            ));
        }
        if mpor <= 0.0 {
            return Err(PortfolioError::InvalidCollateralAgreement(
                "Margin period of risk must be positive".to_string(),
            ));
        }

        Ok(Self {
            threshold,
            mta,
            independent_amount,
            currency,
            mpor,
        })
    }

    /// Creates a zero-threshold (fully collateralised) agreement.
    ///
    /// # Arguments
    ///
    /// * `currency` - Collateral currency
    /// * `mpor` - Margin period of risk in years
    pub fn zero_threshold(currency: Currency, mpor: f64) -> Result<Self, PortfolioError> {
        Self::new(0.0, 0.0, 0.0, currency, mpor)
    }

    /// Returns the threshold amount.
    #[inline]
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Returns the minimum transfer amount.
    #[inline]
    pub fn mta(&self) -> f64 {
        self.mta
    }

    /// Returns the independent amount.
    #[inline]
    pub fn independent_amount(&self) -> f64 {
        self.independent_amount
    }

    /// Returns the collateral currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns the margin period of risk in years.
    #[inline]
    pub fn mpor(&self) -> f64 {
        self.mpor
    }

    /// Returns the margin period of risk in business days (assuming 252 days/year).
    #[inline]
    pub fn mpor_days(&self) -> f64 {
        self.mpor * 252.0
    }

    /// Computes the collateralised exposure given an uncollateralised exposure.
    ///
    /// The collateralised exposure accounts for threshold and independent amount:
    /// CE = max(E - Threshold - IA, 0)
    ///
    /// # Arguments
    ///
    /// * `exposure` - Uncollateralised exposure
    ///
    /// # Returns
    ///
    /// Collateralised exposure (always non-negative).
    #[inline]
    pub fn collateralised_exposure(&self, exposure: f64) -> f64 {
        (exposure - self.threshold - self.independent_amount).max(0.0)
    }
}

/// Netting set grouping trades for exposure aggregation.
///
/// A netting set represents a collection of trades with the same counterparty
/// that can be legally netted in the event of default.
///
/// # Examples
///
/// ```
/// use pricer_xva::portfolio::{NettingSet, NettingSetId, CounterpartyId, TradeId};
///
/// let mut ns = NettingSet::new(
///     NettingSetId::new("NS001"),
///     CounterpartyId::new("CP001"),
/// );
///
/// ns.add_trade(TradeId::new("T001"));
/// ns.add_trade(TradeId::new("T002"));
///
/// assert_eq!(ns.trade_count(), 2);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NettingSet {
    id: NettingSetId,
    counterparty_id: CounterpartyId,
    trade_ids: Vec<TradeId>,
    collateral: Option<CollateralAgreement>,
}

impl NettingSet {
    /// Creates a new netting set without collateral.
    #[inline]
    pub fn new(id: NettingSetId, counterparty_id: CounterpartyId) -> Self {
        Self {
            id,
            counterparty_id,
            trade_ids: Vec::new(),
            collateral: None,
        }
    }

    /// Creates a new netting set with a collateral agreement.
    #[inline]
    pub fn with_collateral(
        id: NettingSetId,
        counterparty_id: CounterpartyId,
        collateral: CollateralAgreement,
    ) -> Self {
        Self {
            id,
            counterparty_id,
            trade_ids: Vec::new(),
            collateral: Some(collateral),
        }
    }

    /// Sets the collateral agreement.
    pub fn set_collateral(&mut self, collateral: CollateralAgreement) {
        self.collateral = Some(collateral);
    }

    /// Removes the collateral agreement.
    pub fn remove_collateral(&mut self) {
        self.collateral = None;
    }

    /// Returns the netting set ID.
    #[inline]
    pub fn id(&self) -> &NettingSetId {
        &self.id
    }

    /// Returns the counterparty ID.
    #[inline]
    pub fn counterparty_id(&self) -> &CounterpartyId {
        &self.counterparty_id
    }

    /// Returns the trade IDs in this netting set.
    #[inline]
    pub fn trade_ids(&self) -> &[TradeId] {
        &self.trade_ids
    }

    /// Returns the collateral agreement if present.
    #[inline]
    pub fn collateral(&self) -> Option<&CollateralAgreement> {
        self.collateral.as_ref()
    }

    /// Returns whether this netting set has a collateral agreement.
    #[inline]
    pub fn is_collateralised(&self) -> bool {
        self.collateral.is_some()
    }

    /// Returns the number of trades in this netting set.
    #[inline]
    pub fn trade_count(&self) -> usize {
        self.trade_ids.len()
    }

    /// Returns whether the netting set is empty (no trades).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.trade_ids.is_empty()
    }

    /// Adds a trade to the netting set.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Trade ID to add
    pub fn add_trade(&mut self, trade_id: TradeId) {
        self.trade_ids.push(trade_id);
    }

    /// Adds multiple trades to the netting set.
    ///
    /// # Arguments
    ///
    /// * `trade_ids` - Iterator of trade IDs to add
    pub fn add_trades(&mut self, trade_ids: impl IntoIterator<Item = TradeId>) {
        self.trade_ids.extend(trade_ids);
    }

    /// Removes a trade from the netting set.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Trade ID to remove
    ///
    /// # Returns
    ///
    /// `true` if the trade was found and removed, `false` otherwise.
    pub fn remove_trade(&mut self, trade_id: &TradeId) -> bool {
        if let Some(pos) = self.trade_ids.iter().position(|id| id == trade_id) {
            self.trade_ids.remove(pos);
            true
        } else {
            false
        }
    }

    /// Checks if the netting set contains a specific trade.
    #[inline]
    pub fn contains_trade(&self, trade_id: &TradeId) -> bool {
        self.trade_ids.contains(trade_id)
    }

    /// Clears all trades from the netting set.
    pub fn clear_trades(&mut self) {
        self.trade_ids.clear();
    }

    /// Returns an iterator over trade IDs.
    #[inline]
    pub fn iter_trade_ids(&self) -> impl Iterator<Item = &TradeId> {
        self.trade_ids.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_collateral_agreement_valid() {
        let csa = CollateralAgreement::new(
            1_000_000.0,
            500_000.0,
            0.0,
            Currency::USD,
            CollateralAgreement::bilateral_mpor(),
        )
        .unwrap();

        assert_eq!(csa.threshold(), 1_000_000.0);
        assert_eq!(csa.mta(), 500_000.0);
        assert_eq!(csa.independent_amount(), 0.0);
        assert_eq!(csa.currency(), Currency::USD);
        assert_relative_eq!(csa.mpor(), 10.0 / 252.0, epsilon = 1e-10);
    }

    #[test]
    fn test_collateral_agreement_invalid_threshold() {
        let result = CollateralAgreement::new(
            -1_000_000.0,
            0.0,
            0.0,
            Currency::USD,
            CollateralAgreement::bilateral_mpor(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_collateral_agreement_invalid_mta() {
        let result = CollateralAgreement::new(
            0.0,
            -100_000.0,
            0.0,
            Currency::USD,
            CollateralAgreement::bilateral_mpor(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_collateral_agreement_invalid_mpor() {
        let result = CollateralAgreement::new(0.0, 0.0, 0.0, Currency::USD, 0.0);
        assert!(result.is_err());

        let result = CollateralAgreement::new(0.0, 0.0, 0.0, Currency::USD, -0.01);
        assert!(result.is_err());
    }

    #[test]
    fn test_collateral_agreement_zero_threshold() {
        let csa =
            CollateralAgreement::zero_threshold(Currency::EUR, CollateralAgreement::cleared_mpor())
                .unwrap();

        assert_eq!(csa.threshold(), 0.0);
        assert_eq!(csa.mta(), 0.0);
        assert_relative_eq!(csa.mpor(), 5.0 / 252.0, epsilon = 1e-10);
    }

    #[test]
    fn test_collateral_agreement_mpor_days() {
        let csa = CollateralAgreement::new(0.0, 0.0, 0.0, Currency::USD, 10.0 / 252.0).unwrap();
        assert_relative_eq!(csa.mpor_days(), 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_collateralised_exposure() {
        let csa = CollateralAgreement::new(
            1_000_000.0, // threshold
            0.0,
            100_000.0, // we post IA
            Currency::USD,
            CollateralAgreement::bilateral_mpor(),
        )
        .unwrap();

        // Exposure below threshold + IA: no collateralised exposure
        assert_eq!(csa.collateralised_exposure(500_000.0), 0.0);

        // Exposure above threshold + IA
        assert_eq!(csa.collateralised_exposure(2_000_000.0), 900_000.0);

        // Negative exposure: no collateralised exposure
        assert_eq!(csa.collateralised_exposure(-500_000.0), 0.0);
    }

    #[test]
    fn test_netting_set_creation() {
        let ns = NettingSet::new(NettingSetId::new("NS001"), CounterpartyId::new("CP001"));

        assert_eq!(ns.id().as_str(), "NS001");
        assert_eq!(ns.counterparty_id().as_str(), "CP001");
        assert!(ns.is_empty());
        assert!(!ns.is_collateralised());
    }

    #[test]
    fn test_netting_set_with_collateral() {
        let csa = CollateralAgreement::zero_threshold(
            Currency::USD,
            CollateralAgreement::bilateral_mpor(),
        )
        .unwrap();

        let ns = NettingSet::with_collateral(
            NettingSetId::new("NS001"),
            CounterpartyId::new("CP001"),
            csa,
        );

        assert!(ns.is_collateralised());
        assert!(ns.collateral().is_some());
    }

    #[test]
    fn test_netting_set_add_trade() {
        let mut ns = NettingSet::new(NettingSetId::new("NS001"), CounterpartyId::new("CP001"));

        ns.add_trade(TradeId::new("T001"));
        ns.add_trade(TradeId::new("T002"));

        assert_eq!(ns.trade_count(), 2);
        assert!(ns.contains_trade(&TradeId::new("T001")));
        assert!(ns.contains_trade(&TradeId::new("T002")));
    }

    #[test]
    fn test_netting_set_add_trades() {
        let mut ns = NettingSet::new(NettingSetId::new("NS001"), CounterpartyId::new("CP001"));

        ns.add_trades(vec![
            TradeId::new("T001"),
            TradeId::new("T002"),
            TradeId::new("T003"),
        ]);

        assert_eq!(ns.trade_count(), 3);
    }

    #[test]
    fn test_netting_set_remove_trade() {
        let mut ns = NettingSet::new(NettingSetId::new("NS001"), CounterpartyId::new("CP001"));

        ns.add_trade(TradeId::new("T001"));
        ns.add_trade(TradeId::new("T002"));

        assert!(ns.remove_trade(&TradeId::new("T001")));
        assert_eq!(ns.trade_count(), 1);
        assert!(!ns.contains_trade(&TradeId::new("T001")));

        // Remove non-existent trade
        assert!(!ns.remove_trade(&TradeId::new("T999")));
    }

    #[test]
    fn test_netting_set_clear_trades() {
        let mut ns = NettingSet::new(NettingSetId::new("NS001"), CounterpartyId::new("CP001"));

        ns.add_trades(vec![TradeId::new("T001"), TradeId::new("T002")]);
        ns.clear_trades();

        assert!(ns.is_empty());
    }

    #[test]
    fn test_netting_set_set_remove_collateral() {
        let mut ns = NettingSet::new(NettingSetId::new("NS001"), CounterpartyId::new("CP001"));

        assert!(!ns.is_collateralised());

        let csa = CollateralAgreement::zero_threshold(
            Currency::USD,
            CollateralAgreement::bilateral_mpor(),
        )
        .unwrap();
        ns.set_collateral(csa);

        assert!(ns.is_collateralised());

        ns.remove_collateral();
        assert!(!ns.is_collateralised());
    }

    #[test]
    fn test_netting_set_iter_trade_ids() {
        let mut ns = NettingSet::new(NettingSetId::new("NS001"), CounterpartyId::new("CP001"));

        ns.add_trades(vec![TradeId::new("T001"), TradeId::new("T002")]);

        let ids: Vec<_> = ns.iter_trade_ids().collect();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_netting_set_clone() {
        let mut ns1 = NettingSet::new(NettingSetId::new("NS001"), CounterpartyId::new("CP001"));
        ns1.add_trade(TradeId::new("T001"));

        let ns2 = ns1.clone();
        assert_eq!(ns1.trade_count(), ns2.trade_count());
    }
}
