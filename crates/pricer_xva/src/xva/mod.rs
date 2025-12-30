//! XVA calculations (CVA, DVA, FVA).
//!
//! This module provides valuation adjustments for counterparty credit risk
//! and funding costs.
//!
//! # Supported Metrics
//!
//! - **CVA** (Credit Valuation Adjustment): Expected loss from counterparty default
//! - **DVA** (Debit Valuation Adjustment): Benefit from own default risk
//! - **FVA** (Funding Valuation Adjustment): Cost/benefit of funding exposures
//!   - FCA (Funding Cost Adjustment): Cost of funding positive exposure
//!   - FBA (Funding Benefit Adjustment): Benefit from negative exposure
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                  XvaCalculator                       │
//! ├─────────────────────────────────────────────────────┤
//! │  Inputs:                                            │
//! │    - Portfolio (trades, counterparties, netting)   │
//! │    - ExposureSoA (EE/ENE profiles)                 │
//! │    - OwnCreditParams (own hazard rate, LGD)        │
//! │    - FundingParams (borrow/lend spreads)           │
//! │    - DiscountFactors (risk-free)                   │
//! ├─────────────────────────────────────────────────────┤
//! │  Outputs:                                           │
//! │    - NettingSetXva (per netting set)               │
//! │    - CounterpartyXva (aggregated per counterparty) │
//! │    - PortfolioXva (total portfolio XVA)            │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use pricer_xva::xva::{XvaCalculator, FundingParams, OwnCreditParams};
//! use pricer_xva::soa::ExposureSoA;
//!
//! let calculator = XvaCalculator::new()
//!     .with_own_credit(OwnCreditParams::new(0.02, 0.4).unwrap())
//!     .with_funding(FundingParams::from_bps(50.0, 30.0));
//!
//! let xva = calculator.compute_portfolio_xva(
//!     &portfolio,
//!     &ee_profiles,
//!     &ene_profiles,
//!     &discount_factors,
//! )?;
//!
//! println!("Total CVA: {}", xva.cva);
//! println!("Total DVA: {}", xva.dva);
//! println!("Net FVA: {}", xva.fva());
//! ```

mod cva;
mod dva;
mod error;
mod fva;
mod params;
mod result;

pub use cva::{compute_cva, compute_cva_with_survival};
pub use dva::{compute_dva, compute_dva_with_survival};
pub use error::XvaError;
pub use fva::{compute_fba, compute_fca, compute_fva};
pub use params::{FundingParams, OwnCreditParams};
pub use result::{CounterpartyXva, NettingSetXva, PortfolioXva};

use crate::portfolio::{CounterpartyId, CreditParams, NettingSetId, Portfolio};
use crate::soa::ExposureSoA;
use rayon::prelude::*;
use std::collections::HashMap;

/// Configuration for XVA calculations.
///
/// Holds parameters that apply across all netting sets and counterparties.
#[derive(Clone, Debug)]
pub struct XvaConfig {
    /// Own credit parameters for DVA calculation.
    pub own_credit: Option<OwnCreditParams>,
    /// Funding parameters for FVA calculation.
    pub funding: FundingParams,
    /// Whether to enable bilateral CVA/DVA calculations.
    pub bilateral: bool,
}

impl Default for XvaConfig {
    fn default() -> Self {
        Self {
            own_credit: None,
            funding: FundingParams::zero(),
            bilateral: false,
        }
    }
}

impl XvaConfig {
    /// Creates a new XVA configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets own credit parameters for DVA calculation.
    pub fn with_own_credit(mut self, own_credit: OwnCreditParams) -> Self {
        self.own_credit = Some(own_credit);
        self
    }

    /// Sets funding parameters for FVA calculation.
    pub fn with_funding(mut self, funding: FundingParams) -> Self {
        self.funding = funding;
        self
    }

    /// Enables bilateral CVA/DVA calculations.
    pub fn bilateral(mut self) -> Self {
        self.bilateral = true;
        self
    }
}

/// XVA calculator for portfolio-level valuation adjustments.
///
/// Computes CVA, DVA, and FVA for netting sets, counterparties, and
/// the entire portfolio using parallel processing.
///
/// # Performance
///
/// Uses Rayon for parallel computation across netting sets and
/// counterparties. Performance scales well with core count for
/// large portfolios.
#[derive(Clone, Debug)]
pub struct XvaCalculator {
    config: XvaConfig,
}

impl Default for XvaCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl XvaCalculator {
    /// Creates a new XVA calculator with default configuration.
    pub fn new() -> Self {
        Self {
            config: XvaConfig::default(),
        }
    }

    /// Creates a calculator with the given configuration.
    pub fn with_config(config: XvaConfig) -> Self {
        Self { config }
    }

    /// Sets own credit parameters for DVA calculation.
    pub fn with_own_credit(mut self, own_credit: OwnCreditParams) -> Self {
        self.config.own_credit = Some(own_credit);
        self
    }

    /// Sets funding parameters for FVA calculation.
    pub fn with_funding(mut self, funding: FundingParams) -> Self {
        self.config.funding = funding;
        self
    }

    /// Enables bilateral CVA/DVA calculations.
    pub fn bilateral(mut self) -> Self {
        self.config.bilateral = true;
        self
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &XvaConfig {
        &self.config
    }

    /// Computes XVA for a single netting set.
    ///
    /// # Arguments
    ///
    /// * `netting_set_id` - Netting set identifier
    /// * `counterparty_id` - Counterparty identifier
    /// * `ee` - Expected Exposure profile
    /// * `ene` - Expected Negative Exposure profile
    /// * `time_grid` - Time points in years
    /// * `credit_params` - Counterparty credit parameters
    /// * `discount_factors` - Risk-free discount factors
    ///
    /// # Returns
    ///
    /// XVA result for the netting set.
    #[allow(clippy::too_many_arguments)]
    pub fn compute_netting_set_xva(
        &self,
        netting_set_id: NettingSetId,
        counterparty_id: CounterpartyId,
        ee: &[f64],
        ene: &[f64],
        time_grid: &[f64],
        credit_params: &CreditParams,
        discount_factors: &[f64],
    ) -> NettingSetXva {
        // Compute CVA
        let cva = compute_cva(ee, time_grid, credit_params);

        // Compute DVA if own credit parameters are available
        let dva = self
            .config
            .own_credit
            .as_ref()
            .map(|own| compute_dva(ene, time_grid, own))
            .unwrap_or(0.0);

        // Compute FVA components
        let (fca, fba, _fva) = compute_fva(
            ee,
            ene,
            time_grid,
            self.config.funding.spread_borrow,
            self.config.funding.spread_lend,
            discount_factors,
        );

        NettingSetXva::new(netting_set_id, counterparty_id, cva, dva, fca, fba)
    }

    /// Computes XVA for all netting sets of a counterparty.
    ///
    /// # Arguments
    ///
    /// * `counterparty_id` - Counterparty identifier
    /// * `netting_set_ids` - IDs of netting sets for this counterparty
    /// * `ee_profiles` - Expected Exposure profiles by netting set
    /// * `ene_profiles` - Expected Negative Exposure profiles by netting set
    /// * `time_grid` - Shared time grid
    /// * `credit_params` - Counterparty credit parameters
    /// * `discount_factors` - Risk-free discount factors
    ///
    /// # Returns
    ///
    /// Aggregated XVA for the counterparty.
    #[allow(clippy::too_many_arguments)]
    pub fn compute_counterparty_xva(
        &self,
        counterparty_id: CounterpartyId,
        netting_set_ids: &[NettingSetId],
        ee_profiles: &HashMap<NettingSetId, Vec<f64>>,
        ene_profiles: &HashMap<NettingSetId, Vec<f64>>,
        time_grid: &[f64],
        credit_params: &CreditParams,
        discount_factors: &[f64],
    ) -> CounterpartyXva {
        let netting_set_xvas: Vec<NettingSetXva> = netting_set_ids
            .iter()
            .filter_map(|ns_id| {
                let ee = ee_profiles.get(ns_id)?;
                let ene = ene_profiles.get(ns_id)?;

                Some(self.compute_netting_set_xva(
                    ns_id.clone(),
                    counterparty_id.clone(),
                    ee,
                    ene,
                    time_grid,
                    credit_params,
                    discount_factors,
                ))
            })
            .collect();

        CounterpartyXva::from_netting_sets(counterparty_id, netting_set_xvas)
    }

    /// Computes XVA for the entire portfolio.
    ///
    /// Uses parallel processing across counterparties for efficiency.
    ///
    /// # Arguments
    ///
    /// * `portfolio` - Portfolio containing counterparties and netting sets
    /// * `ee_profiles` - Expected Exposure profiles by netting set
    /// * `ene_profiles` - Expected Negative Exposure profiles by netting set
    /// * `time_grid` - Shared time grid
    /// * `discount_factors` - Risk-free discount factors
    ///
    /// # Returns
    ///
    /// Portfolio-level XVA result, or error if required data is missing.
    pub fn compute_portfolio_xva(
        &self,
        portfolio: &Portfolio,
        ee_profiles: &HashMap<NettingSetId, Vec<f64>>,
        ene_profiles: &HashMap<NettingSetId, Vec<f64>>,
        time_grid: &[f64],
        discount_factors: &[f64],
    ) -> Result<PortfolioXva, XvaError> {
        // Validate inputs
        if time_grid.is_empty() {
            return Err(XvaError::EmptyTimeGrid);
        }

        if discount_factors.len() != time_grid.len() {
            return Err(XvaError::DiscountFactorMismatch {
                expected: time_grid.len(),
                actual: discount_factors.len(),
            });
        }

        // Group netting sets by counterparty
        let mut ns_by_counterparty: HashMap<CounterpartyId, Vec<NettingSetId>> = HashMap::new();
        for ns in portfolio.netting_sets() {
            ns_by_counterparty
                .entry(ns.counterparty_id().clone())
                .or_default()
                .push(ns.id().clone());
        }

        // Process counterparties in parallel
        let counterparty_xvas: Vec<CounterpartyXva> = ns_by_counterparty
            .par_iter()
            .filter_map(|(cp_id, ns_ids)| {
                // Get credit parameters for this counterparty
                let counterparty = portfolio.counterparty(cp_id)?;
                let credit_params = counterparty.credit_params();

                Some(self.compute_counterparty_xva(
                    cp_id.clone(),
                    ns_ids,
                    ee_profiles,
                    ene_profiles,
                    time_grid,
                    credit_params,
                    discount_factors,
                ))
            })
            .collect();

        Ok(PortfolioXva::from_counterparties(counterparty_xvas))
    }

    /// Computes XVA using ExposureSoA for efficient memory access.
    ///
    /// This method is optimised for large portfolios with many netting sets.
    ///
    /// # Arguments
    ///
    /// * `portfolio` - Portfolio containing counterparties and netting sets
    /// * `ee_soa` - Expected Exposure in SoA format
    /// * `ene_soa` - Expected Negative Exposure in SoA format
    /// * `discount_factors` - Risk-free discount factors
    ///
    /// # Returns
    ///
    /// Portfolio-level XVA result.
    pub fn compute_portfolio_xva_soa(
        &self,
        portfolio: &Portfolio,
        ee_soa: &ExposureSoA,
        ene_soa: &ExposureSoA,
        discount_factors: &[f64],
    ) -> Result<PortfolioXva, XvaError> {
        let time_grid = ee_soa.time_grid();

        // Validate inputs
        if time_grid.is_empty() {
            return Err(XvaError::EmptyTimeGrid);
        }

        if discount_factors.len() != time_grid.len() {
            return Err(XvaError::DiscountFactorMismatch {
                expected: time_grid.len(),
                actual: discount_factors.len(),
            });
        }

        // Validate ENE has same time grid
        if ene_soa.n_times() != ee_soa.n_times() {
            return Err(XvaError::TimeGridMismatch {
                expected: ee_soa.n_times(),
                actual: ene_soa.n_times(),
            });
        }

        // Build lookup maps for netting set indices
        let ee_lookup: HashMap<&NettingSetId, usize> = ee_soa
            .netting_set_ids()
            .iter()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect();

        let ene_lookup: HashMap<&NettingSetId, usize> = ene_soa
            .netting_set_ids()
            .iter()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect();

        // Group netting sets by counterparty
        let mut ns_by_counterparty: HashMap<CounterpartyId, Vec<&NettingSetId>> = HashMap::new();
        for ns in portfolio.netting_sets() {
            ns_by_counterparty
                .entry(ns.counterparty_id().clone())
                .or_default()
                .push(ns.id());
        }

        // Process counterparties in parallel
        let counterparty_xvas: Vec<CounterpartyXva> = ns_by_counterparty
            .par_iter()
            .filter_map(|(cp_id, ns_ids)| {
                let counterparty = portfolio.counterparty(cp_id)?;
                let credit_params = counterparty.credit_params();

                let netting_set_xvas: Vec<NettingSetXva> = ns_ids
                    .iter()
                    .filter_map(|ns_id| {
                        let ee_idx = ee_lookup.get(ns_id)?;
                        let ene_idx = ene_lookup.get(ns_id)?;

                        let ee = ee_soa.exposure_profile(*ee_idx);
                        let ene = ene_soa.exposure_profile(*ene_idx);

                        Some(self.compute_netting_set_xva(
                            (*ns_id).clone(),
                            cp_id.clone(),
                            ee,
                            ene,
                            time_grid,
                            credit_params,
                            discount_factors,
                        ))
                    })
                    .collect();

                Some(CounterpartyXva::from_netting_sets(
                    cp_id.clone(),
                    netting_set_xvas,
                ))
            })
            .collect();

        Ok(PortfolioXva::from_counterparties(counterparty_xvas))
    }
}

/// Generates flat discount factors for a given rate and time grid.
///
/// Useful for testing and simple cases with flat yield curves.
///
/// # Arguments
///
/// * `rate` - Flat interest rate (annualised)
/// * `time_grid` - Time points in years
///
/// # Returns
///
/// Discount factors at each time point.
///
/// # Examples
///
/// ```
/// use pricer_xva::xva::generate_flat_discount_factors;
///
/// let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
/// let df = generate_flat_discount_factors(0.05, &time_grid);
///
/// assert!((df[0] - 1.0).abs() < 1e-10); // df(0) = 1
/// assert!(df[4] < 1.0); // df(1) < 1
/// ```
pub fn generate_flat_discount_factors(rate: f64, time_grid: &[f64]) -> Vec<f64> {
    time_grid.iter().map(|&t| (-rate * t).exp()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::{Counterparty, NettingSet, PortfolioBuilder};
    use approx::assert_relative_eq;

    fn create_test_portfolio() -> Portfolio {
        let credit1 = CreditParams::new(0.02, 0.4).unwrap();
        let credit2 = CreditParams::new(0.03, 0.5).unwrap();

        let cp1 = Counterparty::new(CounterpartyId::new("CP001"), credit1);
        let cp2 = Counterparty::new(CounterpartyId::new("CP002"), credit2);

        let ns1 = NettingSet::new(NettingSetId::new("NS001"), CounterpartyId::new("CP001"));
        let ns2 = NettingSet::new(NettingSetId::new("NS002"), CounterpartyId::new("CP001"));
        let ns3 = NettingSet::new(NettingSetId::new("NS003"), CounterpartyId::new("CP002"));

        PortfolioBuilder::new()
            .add_counterparty(cp1)
            .add_counterparty(cp2)
            .add_netting_set(ns1)
            .add_netting_set(ns2)
            .add_netting_set(ns3)
            .build()
            .unwrap()
    }

    fn create_test_time_grid() -> Vec<f64> {
        vec![0.0, 0.25, 0.5, 0.75, 1.0]
    }

    fn create_test_ee_profiles() -> HashMap<NettingSetId, Vec<f64>> {
        let mut profiles = HashMap::new();
        profiles.insert(
            NettingSetId::new("NS001"),
            vec![0.0, 100.0, 150.0, 100.0, 50.0],
        );
        profiles.insert(
            NettingSetId::new("NS002"),
            vec![0.0, 50.0, 80.0, 60.0, 30.0],
        );
        profiles.insert(
            NettingSetId::new("NS003"),
            vec![0.0, 200.0, 180.0, 150.0, 100.0],
        );
        profiles
    }

    fn create_test_ene_profiles() -> HashMap<NettingSetId, Vec<f64>> {
        let mut profiles = HashMap::new();
        profiles.insert(
            NettingSetId::new("NS001"),
            vec![0.0, 30.0, 40.0, 30.0, 10.0],
        );
        profiles.insert(NettingSetId::new("NS002"), vec![0.0, 20.0, 25.0, 15.0, 5.0]);
        profiles.insert(
            NettingSetId::new("NS003"),
            vec![0.0, 50.0, 60.0, 40.0, 20.0],
        );
        profiles
    }

    #[test]
    fn test_xva_calculator_default() {
        let calc = XvaCalculator::new();
        assert!(calc.config.own_credit.is_none());
        assert!(!calc.config.bilateral);
    }

    #[test]
    fn test_xva_calculator_with_config() {
        let own_credit = OwnCreditParams::new(0.02, 0.4).unwrap();
        let funding = FundingParams::from_bps(50.0, 30.0);

        let calc = XvaCalculator::new()
            .with_own_credit(own_credit)
            .with_funding(funding)
            .bilateral();

        assert!(calc.config.own_credit.is_some());
        assert!(calc.config.bilateral);
        assert_relative_eq!(calc.config.funding.spread_borrow, 0.005, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_netting_set_xva() {
        let own_credit = OwnCreditParams::new(0.03, 0.4).unwrap();
        let calc = XvaCalculator::new()
            .with_own_credit(own_credit)
            .with_funding(FundingParams::from_bps(50.0, 30.0));

        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let ene = vec![0.0, 30.0, 30.0, 30.0, 30.0];
        let time_grid = create_test_time_grid();
        let credit = CreditParams::new(0.02, 0.4).unwrap();
        let df = generate_flat_discount_factors(0.05, &time_grid);

        let xva = calc.compute_netting_set_xva(
            NettingSetId::new("NS001"),
            CounterpartyId::new("CP001"),
            &ee,
            &ene,
            &time_grid,
            &credit,
            &df,
        );

        assert!(xva.cva > 0.0);
        assert!(xva.dva > 0.0);
        assert!(xva.fca > 0.0);
        assert!(xva.fba > 0.0);
    }

    #[test]
    fn test_compute_netting_set_xva_no_own_credit() {
        let calc = XvaCalculator::new();

        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let ene = vec![0.0, 30.0, 30.0, 30.0, 30.0];
        let time_grid = create_test_time_grid();
        let credit = CreditParams::new(0.02, 0.4).unwrap();
        let df = generate_flat_discount_factors(0.05, &time_grid);

        let xva = calc.compute_netting_set_xva(
            NettingSetId::new("NS001"),
            CounterpartyId::new("CP001"),
            &ee,
            &ene,
            &time_grid,
            &credit,
            &df,
        );

        assert!(xva.cva > 0.0);
        assert_eq!(xva.dva, 0.0); // No own credit params
    }

    #[test]
    fn test_compute_portfolio_xva() {
        let portfolio = create_test_portfolio();
        let time_grid = create_test_time_grid();
        let ee_profiles = create_test_ee_profiles();
        let ene_profiles = create_test_ene_profiles();
        let df = generate_flat_discount_factors(0.05, &time_grid);

        let own_credit = OwnCreditParams::new(0.025, 0.4).unwrap();
        let calc = XvaCalculator::new()
            .with_own_credit(own_credit)
            .with_funding(FundingParams::from_bps(50.0, 30.0));

        let xva = calc
            .compute_portfolio_xva(&portfolio, &ee_profiles, &ene_profiles, &time_grid, &df)
            .unwrap();

        assert!(xva.cva > 0.0);
        assert!(xva.dva > 0.0);
        assert!(xva.fca > 0.0);
        assert!(xva.fba > 0.0);
        assert_eq!(xva.counterparty_count(), 2);
        assert_eq!(xva.netting_set_count(), 3);
    }

    #[test]
    fn test_compute_portfolio_xva_empty_time_grid() {
        let portfolio = create_test_portfolio();
        let ee_profiles = create_test_ee_profiles();
        let ene_profiles = create_test_ene_profiles();

        let calc = XvaCalculator::new();
        let result = calc.compute_portfolio_xva(&portfolio, &ee_profiles, &ene_profiles, &[], &[]);

        assert!(matches!(result, Err(XvaError::EmptyTimeGrid)));
    }

    #[test]
    fn test_compute_portfolio_xva_df_mismatch() {
        let portfolio = create_test_portfolio();
        let time_grid = create_test_time_grid();
        let ee_profiles = create_test_ee_profiles();
        let ene_profiles = create_test_ene_profiles();
        let df = vec![1.0, 0.99]; // Wrong length

        let calc = XvaCalculator::new();
        let result =
            calc.compute_portfolio_xva(&portfolio, &ee_profiles, &ene_profiles, &time_grid, &df);

        assert!(matches!(
            result,
            Err(XvaError::DiscountFactorMismatch { .. })
        ));
    }

    #[test]
    fn test_compute_portfolio_xva_soa() {
        let portfolio = create_test_portfolio();
        let time_grid = create_test_time_grid();
        let df = generate_flat_discount_factors(0.05, &time_grid);

        // Create ExposureSoA for EE
        let ns_ids = vec![
            NettingSetId::new("NS001"),
            NettingSetId::new("NS002"),
            NettingSetId::new("NS003"),
        ];
        let mut ee_soa = ExposureSoA::new(time_grid.clone(), ns_ids.clone());
        let mut ene_soa = ExposureSoA::new(time_grid.clone(), ns_ids);

        // Set exposure values
        for (t, &val) in [0.0, 100.0, 150.0, 100.0, 50.0].iter().enumerate() {
            ee_soa.set_exposure(0, t, val);
        }
        for (t, &val) in [0.0, 50.0, 80.0, 60.0, 30.0].iter().enumerate() {
            ee_soa.set_exposure(1, t, val);
        }
        for (t, &val) in [0.0, 200.0, 180.0, 150.0, 100.0].iter().enumerate() {
            ee_soa.set_exposure(2, t, val);
        }

        for (t, &val) in [0.0, 30.0, 40.0, 30.0, 10.0].iter().enumerate() {
            ene_soa.set_exposure(0, t, val);
        }
        for (t, &val) in [0.0, 20.0, 25.0, 15.0, 5.0].iter().enumerate() {
            ene_soa.set_exposure(1, t, val);
        }
        for (t, &val) in [0.0, 50.0, 60.0, 40.0, 20.0].iter().enumerate() {
            ene_soa.set_exposure(2, t, val);
        }

        let own_credit = OwnCreditParams::new(0.025, 0.4).unwrap();
        let calc = XvaCalculator::new()
            .with_own_credit(own_credit)
            .with_funding(FundingParams::from_bps(50.0, 30.0));

        let xva = calc
            .compute_portfolio_xva_soa(&portfolio, &ee_soa, &ene_soa, &df)
            .unwrap();

        assert!(xva.cva > 0.0);
        assert!(xva.dva > 0.0);
        assert_eq!(xva.counterparty_count(), 2);
    }

    #[test]
    fn test_generate_flat_discount_factors() {
        let time_grid = vec![0.0, 0.5, 1.0, 2.0];
        let df = generate_flat_discount_factors(0.05, &time_grid);

        assert_eq!(df.len(), 4);
        assert_relative_eq!(df[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(df[1], (-0.05 * 0.5_f64).exp(), epsilon = 1e-10);
        assert_relative_eq!(df[2], (-0.05 * 1.0_f64).exp(), epsilon = 1e-10);
        assert_relative_eq!(df[3], (-0.05 * 2.0_f64).exp(), epsilon = 1e-10);
    }

    #[test]
    fn test_xva_totals_consistency() {
        let portfolio = create_test_portfolio();
        let time_grid = create_test_time_grid();
        let ee_profiles = create_test_ee_profiles();
        let ene_profiles = create_test_ene_profiles();
        let df = generate_flat_discount_factors(0.05, &time_grid);

        let own_credit = OwnCreditParams::new(0.025, 0.4).unwrap();
        let calc = XvaCalculator::new()
            .with_own_credit(own_credit)
            .with_funding(FundingParams::from_bps(50.0, 30.0));

        let xva = calc
            .compute_portfolio_xva(&portfolio, &ee_profiles, &ene_profiles, &time_grid, &df)
            .unwrap();

        // Verify that portfolio totals equal sum of counterparty totals
        let sum_cva: f64 = xva.by_counterparty.iter().map(|c| c.cva).sum();
        let sum_dva: f64 = xva.by_counterparty.iter().map(|c| c.dva).sum();
        let sum_fca: f64 = xva.by_counterparty.iter().map(|c| c.fca).sum();
        let sum_fba: f64 = xva.by_counterparty.iter().map(|c| c.fba).sum();

        assert_relative_eq!(xva.cva, sum_cva, epsilon = 1e-10);
        assert_relative_eq!(xva.dva, sum_dva, epsilon = 1e-10);
        assert_relative_eq!(xva.fca, sum_fca, epsilon = 1e-10);
        assert_relative_eq!(xva.fba, sum_fba, epsilon = 1e-10);
    }

    #[test]
    fn test_xva_config_builder() {
        let config = XvaConfig::new()
            .with_own_credit(OwnCreditParams::new(0.02, 0.4).unwrap())
            .with_funding(FundingParams::symmetric(0.005))
            .bilateral();

        assert!(config.own_credit.is_some());
        assert!(config.bilateral);
        assert_eq!(config.funding.spread_borrow, 0.005);
    }
}
