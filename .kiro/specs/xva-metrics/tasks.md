# XVA Metrics Implementation Tasks

## Overview

Implementation tasks for the XVA metrics module in `pricer_xva` (L4).

---

## Task 1: Error Types and Structures

**Status**: Pending

### 1.1 Create XvaError enum
- [ ] Create `xva/error.rs`
- [ ] Define `XvaError` enum with thiserror
- [ ] Unit tests for error formatting

### 1.2 Create FundingParams struct
- [ ] Define FundingParams with borrow/lend spreads
- [ ] Add constructors (symmetric, asymmetric, from_bps)
- [ ] Unit tests

### 1.3 Create OwnCreditParams struct
- [ ] Define OwnCreditParams with hazard_rate, lgd
- [ ] Add survival_prob and marginal_pd methods
- [ ] Validation on construction
- [ ] Unit tests

---

## Task 2: XVA Result Structures

**Status**: Pending

### 2.1 Create NettingSetXva struct
- [ ] Create `xva/result.rs`
- [ ] Define NettingSetXva with CVA, DVA, FCA, FBA
- [ ] Add fva() and total_xva() methods
- [ ] Unit tests

### 2.2 Create CounterpartyXva struct
- [ ] Define aggregated counterparty results
- [ ] Include netting_set_xvas vector
- [ ] Unit tests

### 2.3 Create PortfolioXva struct
- [ ] Define portfolio-level aggregation
- [ ] Include by_counterparty vector
- [ ] Unit tests

---

## Task 3: CVA Calculation

**Status**: Pending

### 3.1 Implement compute_cva function
- [ ] Create `xva/cva.rs`
- [ ] Implement CVA = LGD × ∫ EE × dPD formula
- [ ] Use trapezoidal integration
- [ ] Ensure non-negative result
- [ ] Unit tests with known values

### 3.2 Add CVA documentation
- [ ] Mathematical formula in rustdoc
- [ ] Example usage

---

## Task 4: DVA Calculation

**Status**: Pending

### 4.1 Implement compute_dva function
- [ ] Create `xva/dva.rs`
- [ ] Implement DVA = LGD_own × ∫ ENE × dPD_own formula
- [ ] Use trapezoidal integration
- [ ] Ensure non-negative result
- [ ] Unit tests

---

## Task 5: FVA Calculation

**Status**: Pending

### 5.1 Implement compute_fca function
- [ ] Create `xva/fva.rs`
- [ ] Implement FCA = ∫ EE × s_borrow × df × dt
- [ ] Unit tests

### 5.2 Implement compute_fba function
- [ ] Implement FBA = ∫ ENE × s_lend × df × dt
- [ ] Unit tests

### 5.3 Add FVA helper
- [ ] fva() = fca - fba
- [ ] Unit tests

---

## Task 6: XVA Calculator

**Status**: Pending

### 6.1 Create XvaCalculator struct
- [ ] Update `xva/mod.rs`
- [ ] Define XvaCalculator with own_credit, funding, discount_factors
- [ ] Constructor

### 6.2 Implement compute_netting_set_xva
- [ ] Compute all XVA metrics for single netting set
- [ ] Return NettingSetXva
- [ ] Unit tests

### 6.3 Implement compute_portfolio_xva
- [ ] Parallel computation across netting sets
- [ ] Aggregate by counterparty
- [ ] Return PortfolioXva
- [ ] Integration tests

---

## Task 7: Integration Utilities

**Status**: Pending

### 7.1 Create integration helpers
- [ ] Create `xva/integration.rs`
- [ ] Trapezoidal integration function
- [ ] Unit tests for integration accuracy

### 7.2 Add collateral adjustment
- [ ] apply_collateral method
- [ ] Uses CollateralAgreement from portfolio
- [ ] Unit tests

---

## Task 8: Module Exports and Documentation

**Status**: Pending

### 8.1 Update xva/mod.rs
- [ ] Export all public types
- [ ] Module documentation
- [ ] Examples

### 8.2 Update lib.rs
- [ ] Add xva re-exports
- [ ] Update crate documentation

---

## Task 9: Integration Tests

**Status**: Pending

### 9.1 End-to-end XVA test
- [ ] Create portfolio with trades
- [ ] Compute exposure profiles
- [ ] Calculate XVA
- [ ] Verify results

### 9.2 Parallel correctness test
- [ ] Large portfolio test
- [ ] Verify parallel vs sequential match

---

## Completion Checklist

- [ ] All unit tests passing
- [ ] All integration tests passing
- [ ] Documentation complete
- [ ] `cargo clippy` clean
- [ ] `cargo fmt` applied

---

_Created: 2025-12-30_
_Spec: xva-metrics_
_Phase: Tasks_
