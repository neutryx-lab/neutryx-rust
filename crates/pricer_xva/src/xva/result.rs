//! XVA result structures.
//!
//! Provides structured result types for XVA calculations at
//! netting set, counterparty, and portfolio levels.

use crate::portfolio::{CounterpartyId, NettingSetId};

/// XVA results for a single netting set.
///
/// Contains CVA, DVA, and FVA (FCA/FBA) for one netting set.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NettingSetXva {
    /// Netting set identifier.
    pub netting_set_id: NettingSetId,
    /// Counterparty identifier.
    pub counterparty_id: CounterpartyId,
    /// Credit Valuation Adjustment (always non-negative).
    pub cva: f64,
    /// Debit Valuation Adjustment (always non-negative).
    pub dva: f64,
    /// Funding Cost Adjustment (always non-negative).
    pub fca: f64,
    /// Funding Benefit Adjustment (always non-negative).
    pub fba: f64,
}

impl NettingSetXva {
    /// Creates a new netting set XVA result.
    pub fn new(
        netting_set_id: NettingSetId,
        counterparty_id: CounterpartyId,
        cva: f64,
        dva: f64,
        fca: f64,
        fba: f64,
    ) -> Self {
        Self {
            netting_set_id,
            counterparty_id,
            cva,
            dva,
            fca,
            fba,
        }
    }

    /// Returns the net Funding Valuation Adjustment.
    ///
    /// FVA = FCA - FBA
    ///
    /// Positive FVA represents a cost, negative represents a benefit.
    #[inline]
    pub fn fva(&self) -> f64 {
        self.fca - self.fba
    }

    /// Returns the total XVA impact.
    ///
    /// Total XVA = CVA - DVA + FVA
    ///
    /// This represents the total valuation adjustment to apply
    /// to the risk-free price.
    #[inline]
    pub fn total_xva(&self) -> f64 {
        self.cva - self.dva + self.fva()
    }

    /// Returns bilateral CVA (CVA - DVA).
    #[inline]
    pub fn bilateral_cva(&self) -> f64 {
        self.cva - self.dva
    }
}

/// Aggregated XVA results for a counterparty.
///
/// Contains the sum of XVA metrics across all netting sets
/// with this counterparty.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CounterpartyXva {
    /// Counterparty identifier.
    pub counterparty_id: CounterpartyId,
    /// Total CVA across all netting sets.
    pub cva: f64,
    /// Total DVA across all netting sets.
    pub dva: f64,
    /// Total FCA across all netting sets.
    pub fca: f64,
    /// Total FBA across all netting sets.
    pub fba: f64,
    /// Individual netting set results.
    pub netting_set_xvas: Vec<NettingSetXva>,
}

impl CounterpartyXva {
    /// Creates a new counterparty XVA from netting set results.
    pub fn from_netting_sets(
        counterparty_id: CounterpartyId,
        netting_set_xvas: Vec<NettingSetXva>,
    ) -> Self {
        let cva = netting_set_xvas.iter().map(|x| x.cva).sum();
        let dva = netting_set_xvas.iter().map(|x| x.dva).sum();
        let fca = netting_set_xvas.iter().map(|x| x.fca).sum();
        let fba = netting_set_xvas.iter().map(|x| x.fba).sum();

        Self {
            counterparty_id,
            cva,
            dva,
            fca,
            fba,
            netting_set_xvas,
        }
    }

    /// Returns the net FVA.
    #[inline]
    pub fn fva(&self) -> f64 {
        self.fca - self.fba
    }

    /// Returns the total XVA.
    #[inline]
    pub fn total_xva(&self) -> f64 {
        self.cva - self.dva + self.fva()
    }

    /// Returns bilateral CVA.
    #[inline]
    pub fn bilateral_cva(&self) -> f64 {
        self.cva - self.dva
    }

    /// Returns the number of netting sets.
    #[inline]
    pub fn netting_set_count(&self) -> usize {
        self.netting_set_xvas.len()
    }
}

/// Portfolio-level XVA results.
///
/// Contains aggregated XVA metrics across all counterparties
/// in the portfolio.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PortfolioXva {
    /// Total CVA across all counterparties.
    pub cva: f64,
    /// Total DVA across all counterparties.
    pub dva: f64,
    /// Total FCA across all counterparties.
    pub fca: f64,
    /// Total FBA across all counterparties.
    pub fba: f64,
    /// Results by counterparty.
    pub by_counterparty: Vec<CounterpartyXva>,
}

impl PortfolioXva {
    /// Creates a new portfolio XVA from counterparty results.
    pub fn from_counterparties(by_counterparty: Vec<CounterpartyXva>) -> Self {
        let cva = by_counterparty.iter().map(|c| c.cva).sum();
        let dva = by_counterparty.iter().map(|c| c.dva).sum();
        let fca = by_counterparty.iter().map(|c| c.fca).sum();
        let fba = by_counterparty.iter().map(|c| c.fba).sum();

        Self {
            cva,
            dva,
            fca,
            fba,
            by_counterparty,
        }
    }

    /// Returns the net FVA.
    #[inline]
    pub fn fva(&self) -> f64 {
        self.fca - self.fba
    }

    /// Returns the total XVA.
    #[inline]
    pub fn total_xva(&self) -> f64 {
        self.cva - self.dva + self.fva()
    }

    /// Returns bilateral CVA.
    #[inline]
    pub fn bilateral_cva(&self) -> f64 {
        self.cva - self.dva
    }

    /// Returns the number of counterparties.
    #[inline]
    pub fn counterparty_count(&self) -> usize {
        self.by_counterparty.len()
    }

    /// Returns the total number of netting sets.
    pub fn netting_set_count(&self) -> usize {
        self.by_counterparty
            .iter()
            .map(|c| c.netting_set_count())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netting_set_xva_fva() {
        let xva = NettingSetXva::new(
            NettingSetId::new("NS001"),
            CounterpartyId::new("CP001"),
            100.0, // CVA
            20.0,  // DVA
            50.0,  // FCA
            10.0,  // FBA
        );

        assert_eq!(xva.fva(), 40.0); // FCA - FBA = 50 - 10
    }

    #[test]
    fn test_netting_set_xva_total() {
        let xva = NettingSetXva::new(
            NettingSetId::new("NS001"),
            CounterpartyId::new("CP001"),
            100.0, // CVA
            20.0,  // DVA
            50.0,  // FCA
            10.0,  // FBA
        );

        // Total = CVA - DVA + FVA = 100 - 20 + 40 = 120
        assert_eq!(xva.total_xva(), 120.0);
    }

    #[test]
    fn test_netting_set_xva_bilateral_cva() {
        let xva = NettingSetXva::new(
            NettingSetId::new("NS001"),
            CounterpartyId::new("CP001"),
            100.0,
            20.0,
            0.0,
            0.0,
        );

        assert_eq!(xva.bilateral_cva(), 80.0); // CVA - DVA
    }

    #[test]
    fn test_counterparty_xva_from_netting_sets() {
        let ns1 = NettingSetXva::new(
            NettingSetId::new("NS001"),
            CounterpartyId::new("CP001"),
            50.0,
            10.0,
            20.0,
            5.0,
        );
        let ns2 = NettingSetXva::new(
            NettingSetId::new("NS002"),
            CounterpartyId::new("CP001"),
            30.0,
            5.0,
            15.0,
            3.0,
        );

        let cp_xva =
            CounterpartyXva::from_netting_sets(CounterpartyId::new("CP001"), vec![ns1, ns2]);

        assert_eq!(cp_xva.cva, 80.0);
        assert_eq!(cp_xva.dva, 15.0);
        assert_eq!(cp_xva.fca, 35.0);
        assert_eq!(cp_xva.fba, 8.0);
        assert_eq!(cp_xva.netting_set_count(), 2);
    }

    #[test]
    fn test_portfolio_xva_from_counterparties() {
        let cp1 = CounterpartyXva {
            counterparty_id: CounterpartyId::new("CP001"),
            cva: 100.0,
            dva: 20.0,
            fca: 30.0,
            fba: 10.0,
            netting_set_xvas: vec![],
        };
        let cp2 = CounterpartyXva {
            counterparty_id: CounterpartyId::new("CP002"),
            cva: 50.0,
            dva: 10.0,
            fca: 15.0,
            fba: 5.0,
            netting_set_xvas: vec![],
        };

        let portfolio = PortfolioXva::from_counterparties(vec![cp1, cp2]);

        assert_eq!(portfolio.cva, 150.0);
        assert_eq!(portfolio.dva, 30.0);
        assert_eq!(portfolio.fca, 45.0);
        assert_eq!(portfolio.fba, 15.0);
        assert_eq!(portfolio.counterparty_count(), 2);
    }

    #[test]
    fn test_portfolio_xva_total() {
        let portfolio = PortfolioXva {
            cva: 100.0,
            dva: 30.0,
            fca: 40.0,
            fba: 10.0,
            by_counterparty: vec![],
        };

        // Total = 100 - 30 + (40 - 10) = 100
        assert_eq!(portfolio.total_xva(), 100.0);
    }

    #[test]
    fn test_default() {
        let xva = NettingSetXva::default();
        assert_eq!(xva.cva, 0.0);
        assert_eq!(xva.dva, 0.0);
        assert_eq!(xva.fva(), 0.0);
        assert_eq!(xva.total_xva(), 0.0);
    }
}
