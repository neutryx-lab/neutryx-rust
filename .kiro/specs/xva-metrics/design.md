# XVA Metrics Technical Design

## Overview

This document specifies the technical design for the XVA metrics module in `pricer_xva` (L4). The design covers CVA, DVA, and FVA calculations with Rayon parallelisation.

---

## Architecture

### Module Structure

```
crates/pricer_xva/src/xva/
├── mod.rs           # Module exports and XvaCalculator
├── cva.rs           # CVA calculation
├── dva.rs           # DVA calculation
├── fva.rs           # FVA calculation (FCA/FBA)
├── result.rs        # XvaResult structures
├── error.rs         # XvaError types
└── integration.rs   # Numerical integration utilities
```

### Dependency Flow

```
XvaCalculator
    ├── Portfolio (trades, counterparties, netting sets)
    ├── ExposureSoA (exposure profiles)
    ├── CreditParams (per counterparty and own)
    └── FundingParams (spreads)
         ↓
    XvaResult (CVA, DVA, FVA per netting set)
```

---

## Core Data Structures

### XvaError (REQ-XVA-008)

```rust
use thiserror::Error;
use crate::portfolio::{NettingSetId, CounterpartyId};

#[derive(Debug, Error)]
pub enum XvaError {
    #[error("Missing exposure profile for netting set: {0}")]
    MissingExposureProfile(String),

    #[error("Missing credit parameters for counterparty: {0}")]
    MissingCreditParams(String),

    #[error("Missing own credit parameters")]
    MissingOwnCreditParams,

    #[error("Invalid funding spread: {0}")]
    InvalidFundingSpread(String),

    #[error("Integration error: {0}")]
    IntegrationError(String),

    #[error("Time grid mismatch: expected {expected}, got {actual}")]
    TimeGridMismatch { expected: usize, actual: usize },
}
```

### FundingParams (REQ-XVA-003.3)

```rust
/// Funding spread parameters.
#[derive(Clone, Debug)]
pub struct FundingParams {
    /// Funding spread for borrowing (positive exposure)
    /// Expressed as annualised decimal (e.g., 0.01 = 100bp)
    pub spread_borrow: f64,
    /// Funding spread for lending (negative exposure)
    pub spread_lend: f64,
}

impl FundingParams {
    /// Creates symmetric funding spreads.
    pub fn symmetric(spread: f64) -> Self {
        Self {
            spread_borrow: spread,
            spread_lend: spread,
        }
    }

    /// Creates asymmetric funding spreads.
    pub fn asymmetric(spread_borrow: f64, spread_lend: f64) -> Self {
        Self {
            spread_borrow,
            spread_lend,
        }
    }

    /// Creates from basis points.
    pub fn from_bps(borrow_bps: f64, lend_bps: f64) -> Self {
        Self {
            spread_borrow: borrow_bps / 10_000.0,
            spread_lend: lend_bps / 10_000.0,
        }
    }
}
```

### OwnCreditParams (REQ-XVA-002.2)

```rust
/// Own credit parameters for DVA calculation.
#[derive(Clone, Debug)]
pub struct OwnCreditParams {
    /// Own hazard rate
    pub hazard_rate: f64,
    /// Own Loss Given Default [0, 1]
    pub lgd: f64,
}

impl OwnCreditParams {
    pub fn new(hazard_rate: f64, lgd: f64) -> Result<Self, XvaError> {
        if hazard_rate < 0.0 {
            return Err(XvaError::InvalidFundingSpread(
                "Hazard rate must be non-negative".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&lgd) {
            return Err(XvaError::InvalidFundingSpread(
                "LGD must be in [0, 1]".to_string(),
            ));
        }
        Ok(Self { hazard_rate, lgd })
    }

    /// Survival probability to time t.
    #[inline]
    pub fn survival_prob(&self, t: f64) -> f64 {
        (-self.hazard_rate * t).exp()
    }

    /// Marginal default probability between t1 and t2.
    #[inline]
    pub fn marginal_pd(&self, t1: f64, t2: f64) -> f64 {
        self.survival_prob(t1) - self.survival_prob(t2)
    }
}
```

### XvaResult (REQ-XVA-004.2)

```rust
/// XVA calculation results for a netting set.
#[derive(Clone, Debug, Default)]
pub struct NettingSetXva {
    /// Netting set ID
    pub netting_set_id: NettingSetId,
    /// Counterparty ID
    pub counterparty_id: CounterpartyId,
    /// Credit Valuation Adjustment
    pub cva: f64,
    /// Debit Valuation Adjustment
    pub dva: f64,
    /// Funding Cost Adjustment
    pub fca: f64,
    /// Funding Benefit Adjustment
    pub fba: f64,
}

impl NettingSetXva {
    /// Net FVA = FCA - FBA
    #[inline]
    pub fn fva(&self) -> f64 {
        self.fca - self.fba
    }

    /// Total XVA = CVA - DVA + FVA
    #[inline]
    pub fn total_xva(&self) -> f64 {
        self.cva - self.dva + self.fva()
    }
}

/// Aggregated XVA results for counterparty.
#[derive(Clone, Debug, Default)]
pub struct CounterpartyXva {
    pub counterparty_id: CounterpartyId,
    pub cva: f64,
    pub dva: f64,
    pub fca: f64,
    pub fba: f64,
    /// Individual netting set results
    pub netting_set_xvas: Vec<NettingSetXva>,
}

impl CounterpartyXva {
    pub fn fva(&self) -> f64 {
        self.fca - self.fba
    }

    pub fn total_xva(&self) -> f64 {
        self.cva - self.dva + self.fva()
    }
}

/// Portfolio-level XVA results.
#[derive(Clone, Debug, Default)]
pub struct PortfolioXva {
    /// Total CVA across all counterparties
    pub cva: f64,
    /// Total DVA
    pub dva: f64,
    /// Total FCA
    pub fca: f64,
    /// Total FBA
    pub fba: f64,
    /// Results by counterparty
    pub by_counterparty: Vec<CounterpartyXva>,
}

impl PortfolioXva {
    pub fn fva(&self) -> f64 {
        self.fca - self.fba
    }

    pub fn total_xva(&self) -> f64 {
        self.cva - self.dva + self.fva()
    }
}
```

---

## XVA Calculation Functions

### CVA Calculation (REQ-XVA-001)

```rust
/// Computes unilateral CVA for a netting set.
///
/// CVA = LGD × ∫₀ᵀ EE(t) × dPD(t)
///
/// # Arguments
/// * `ee` - Expected Exposure profile
/// * `time_grid` - Time points
/// * `credit_params` - Counterparty credit parameters
///
/// # Returns
/// CVA value (non-negative)
pub fn compute_cva(
    ee: &[f64],
    time_grid: &[f64],
    credit_params: &CreditParams,
) -> f64 {
    if time_grid.len() < 2 || ee.len() != time_grid.len() {
        return 0.0;
    }

    let lgd = credit_params.lgd();
    let mut cva = 0.0;

    // Numerical integration using trapezoidal rule
    for i in 0..time_grid.len() - 1 {
        let t1 = time_grid[i];
        let t2 = time_grid[i + 1];

        // Marginal default probability
        let marginal_pd = credit_params.marginal_default_prob(t1, t2);

        // Average EE over interval
        let avg_ee = 0.5 * (ee[i] + ee[i + 1]);

        cva += lgd * avg_ee * marginal_pd;
    }

    cva.max(0.0) // Ensure non-negative
}
```

### DVA Calculation (REQ-XVA-002)

```rust
/// Computes DVA for a netting set.
///
/// DVA = LGD_own × ∫₀ᵀ ENE(t) × dPD_own(t)
///
/// # Arguments
/// * `ene` - Expected Negative Exposure profile
/// * `time_grid` - Time points
/// * `own_credit` - Own credit parameters
///
/// # Returns
/// DVA value (non-negative)
pub fn compute_dva(
    ene: &[f64],
    time_grid: &[f64],
    own_credit: &OwnCreditParams,
) -> f64 {
    if time_grid.len() < 2 || ene.len() != time_grid.len() {
        return 0.0;
    }

    let lgd = own_credit.lgd;
    let mut dva = 0.0;

    for i in 0..time_grid.len() - 1 {
        let t1 = time_grid[i];
        let t2 = time_grid[i + 1];

        let marginal_pd = own_credit.marginal_pd(t1, t2);
        let avg_ene = 0.5 * (ene[i] + ene[i + 1]);

        dva += lgd * avg_ene * marginal_pd;
    }

    dva.max(0.0)
}
```

### FVA Calculation (REQ-XVA-003)

```rust
/// Computes FCA (Funding Cost Adjustment).
///
/// FCA = ∫₀ᵀ EE(t) × s_f^+ × df(t) dt
pub fn compute_fca(
    ee: &[f64],
    time_grid: &[f64],
    funding_spread: f64,
    discount_factors: &[f64],
) -> f64 {
    if time_grid.len() < 2 {
        return 0.0;
    }

    let mut fca = 0.0;

    for i in 0..time_grid.len() - 1 {
        let dt = time_grid[i + 1] - time_grid[i];
        let avg_ee = 0.5 * (ee[i] + ee[i + 1]);
        let avg_df = 0.5 * (discount_factors[i] + discount_factors[i + 1]);

        fca += avg_ee * funding_spread * avg_df * dt;
    }

    fca.max(0.0)
}

/// Computes FBA (Funding Benefit Adjustment).
///
/// FBA = ∫₀ᵀ ENE(t) × s_f^- × df(t) dt
pub fn compute_fba(
    ene: &[f64],
    time_grid: &[f64],
    lending_spread: f64,
    discount_factors: &[f64],
) -> f64 {
    if time_grid.len() < 2 {
        return 0.0;
    }

    let mut fba = 0.0;

    for i in 0..time_grid.len() - 1 {
        let dt = time_grid[i + 1] - time_grid[i];
        let avg_ene = 0.5 * (ene[i] + ene[i + 1]);
        let avg_df = 0.5 * (discount_factors[i] + discount_factors[i + 1]);

        fba += avg_ene * lending_spread * avg_df * dt;
    }

    fba.max(0.0)
}
```

---

## XVA Calculator (REQ-XVA-004)

```rust
use rayon::prelude::*;

/// XVA calculation engine.
pub struct XvaCalculator {
    /// Own credit parameters
    own_credit: OwnCreditParams,
    /// Funding parameters
    funding: FundingParams,
    /// Risk-free discount factors (same time grid as exposures)
    discount_factors: Vec<f64>,
}

impl XvaCalculator {
    /// Creates a new XVA calculator.
    pub fn new(
        own_credit: OwnCreditParams,
        funding: FundingParams,
        discount_factors: Vec<f64>,
    ) -> Self {
        Self {
            own_credit,
            funding,
            discount_factors,
        }
    }

    /// Computes XVA for a single netting set.
    pub fn compute_netting_set_xva(
        &self,
        netting_set_id: &NettingSetId,
        counterparty_id: &CounterpartyId,
        ee: &[f64],
        ene: &[f64],
        time_grid: &[f64],
        credit_params: &CreditParams,
    ) -> NettingSetXva {
        let cva = compute_cva(ee, time_grid, credit_params);
        let dva = compute_dva(ene, time_grid, &self.own_credit);
        let fca = compute_fca(ee, time_grid, self.funding.spread_borrow, &self.discount_factors);
        let fba = compute_fba(ene, time_grid, self.funding.spread_lend, &self.discount_factors);

        NettingSetXva {
            netting_set_id: netting_set_id.clone(),
            counterparty_id: counterparty_id.clone(),
            cva,
            dva,
            fca,
            fba,
        }
    }

    /// Computes XVA for entire portfolio in parallel.
    pub fn compute_portfolio_xva(
        &self,
        portfolio: &Portfolio,
        exposure_soa: &ExposureSoA,
        ene_soa: &ExposureSoA, // Expected Negative Exposure
    ) -> Result<PortfolioXva, XvaError> {
        let time_grid = exposure_soa.time_grid();

        // Validate time grids match
        if ene_soa.time_grid().len() != time_grid.len() {
            return Err(XvaError::TimeGridMismatch {
                expected: time_grid.len(),
                actual: ene_soa.time_grid().len(),
            });
        }

        // Compute XVA per netting set in parallel
        let ns_xvas: Vec<NettingSetXva> = portfolio
            .netting_sets_par_iter()
            .filter_map(|(ns_id, ns)| {
                // Get exposure profile index
                let ns_idx = exposure_soa.find_netting_set(ns_id)?;
                let ee = exposure_soa.exposure_profile(ns_idx);
                let ene = ene_soa.exposure_profile(ns_idx);

                // Get counterparty credit params
                let cp = portfolio.counterparty(ns.counterparty_id())?;
                let credit_params = cp.credit_params();

                Some(self.compute_netting_set_xva(
                    ns_id,
                    ns.counterparty_id(),
                    ee,
                    ene,
                    time_grid,
                    credit_params,
                ))
            })
            .collect();

        // Aggregate by counterparty
        let by_counterparty = self.aggregate_by_counterparty(&ns_xvas);

        // Portfolio totals
        let cva: f64 = by_counterparty.iter().map(|c| c.cva).sum();
        let dva: f64 = by_counterparty.iter().map(|c| c.dva).sum();
        let fca: f64 = by_counterparty.iter().map(|c| c.fca).sum();
        let fba: f64 = by_counterparty.iter().map(|c| c.fba).sum();

        Ok(PortfolioXva {
            cva,
            dva,
            fca,
            fba,
            by_counterparty,
        })
    }

    /// Aggregates netting set XVAs by counterparty.
    fn aggregate_by_counterparty(&self, ns_xvas: &[NettingSetXva]) -> Vec<CounterpartyXva> {
        use std::collections::HashMap;

        let mut cp_map: HashMap<CounterpartyId, Vec<NettingSetXva>> = HashMap::new();

        for xva in ns_xvas {
            cp_map
                .entry(xva.counterparty_id.clone())
                .or_default()
                .push(xva.clone());
        }

        cp_map
            .into_iter()
            .map(|(cp_id, xvas)| {
                let cva: f64 = xvas.iter().map(|x| x.cva).sum();
                let dva: f64 = xvas.iter().map(|x| x.dva).sum();
                let fca: f64 = xvas.iter().map(|x| x.fca).sum();
                let fba: f64 = xvas.iter().map(|x| x.fba).sum();

                CounterpartyXva {
                    counterparty_id: cp_id,
                    cva,
                    dva,
                    fca,
                    fba,
                    netting_set_xvas: xvas,
                }
            })
            .collect()
    }
}
```

---

## Integration with Exposure Module

### Computing ENE for DVA/FBA

```rust
impl XvaCalculator {
    /// Helper to compute ENE from simulated values.
    pub fn compute_ene_profile(values: &[Vec<f64>]) -> Vec<f64> {
        ExposureCalculator::expected_negative_exposure(values)
    }
}
```

---

## Collateral Impact (REQ-XVA-006)

### Collateralised XVA

```rust
impl XvaCalculator {
    /// Adjusts exposure for collateral before XVA calculation.
    pub fn apply_collateral(
        &self,
        ee: &[f64],
        collateral: &CollateralAgreement,
    ) -> Vec<f64> {
        ee.iter()
            .map(|&e| collateral.collateralised_exposure(e))
            .collect()
    }
}
```

---

## Testing Strategy

### Unit Tests
- CVA formula verification with known values
- DVA formula verification
- FCA/FBA formula verification
- Time grid integration accuracy

### Integration Tests
- Portfolio XVA calculation
- Parallel computation correctness
- Collateral impact verification

### Property-Based Tests
- CVA ≥ 0 always
- DVA ≥ 0 always
- Higher credit risk → higher CVA
- FVA sign consistency

### Numerical Tests
- Compare against analytical formulas for simple cases
- Convergence with time grid refinement

---

_Created: 2025-12-30_
_Spec: xva-metrics_
_Phase: Design_
