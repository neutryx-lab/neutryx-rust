# XVA Metrics Implementation Tasks

## Overview

Implementation tasks for the XVA metrics module in `pricer_xva` (L4).

---

## Task 1: Error Types and Structures

**Status**: Complete

### 1.1 Create XvaError enum
- [x] Create `xva/error.rs`
- [x] Define `XvaError` enum with thiserror
- [x] Unit tests for error formatting

### 1.2 Create FundingParams struct
- [x] Define FundingParams with borrow/lend spreads
- [x] Add constructors (symmetric, asymmetric, from_bps)
- [x] Unit tests

### 1.3 Create OwnCreditParams struct
- [x] Define OwnCreditParams with hazard_rate, lgd
- [x] Add survival_prob and marginal_pd methods
- [x] Validation on construction
- [x] Unit tests

---

## Task 2: XVA Result Structures

**Status**: Complete

### 2.1 Create NettingSetXva struct
- [x] Create `xva/result.rs`
- [x] Define NettingSetXva with CVA, DVA, FCA, FBA
- [x] Add fva() and total_xva() methods
- [x] Unit tests

### 2.2 Create CounterpartyXva struct
- [x] Define aggregated counterparty results
- [x] Include netting_set_xvas vector
- [x] Unit tests

### 2.3 Create PortfolioXva struct
- [x] Define portfolio-level aggregation
- [x] Include by_counterparty vector
- [x] Unit tests

---

## Task 3: CVA Calculation

**Status**: Complete

### 3.1 Implement compute_cva function
- [x] Create `xva/cva.rs`
- [x] Implement CVA = LGD × ∫ EE × dPD formula
- [x] Use trapezoidal integration
- [x] Ensure non-negative result
- [x] Unit tests with known values

### 3.2 Add CVA documentation
- [x] Mathematical formula in rustdoc
- [x] Example usage

---

## Task 4: DVA Calculation

**Status**: Complete

### 4.1 Implement compute_dva function
- [x] Create `xva/dva.rs`
- [x] Implement DVA = LGD_own × ∫ ENE × dPD_own formula
- [x] Use trapezoidal integration
- [x] Ensure non-negative result
- [x] Unit tests

---

## Task 5: FVA Calculation

**Status**: Complete

### 5.1 Implement compute_fca function
- [x] Create `xva/fva.rs`
- [x] Implement FCA = ∫ EE × s_borrow × df × dt
- [x] Unit tests

### 5.2 Implement compute_fba function
- [x] Implement FBA = ∫ ENE × s_lend × df × dt
- [x] Unit tests

### 5.3 Add FVA helper
- [x] fva() = fca - fba
- [x] Unit tests

---

## Task 6: XVA Calculator

**Status**: Complete

### 6.1 Create XvaCalculator struct
- [x] Update `xva/mod.rs`
- [x] Define XvaCalculator with own_credit, funding, discount_factors
- [x] Constructor with fluent builder API

### 6.2 Implement compute_netting_set_xva
- [x] Compute all XVA metrics for single netting set
- [x] Return NettingSetXva
- [x] Unit tests

### 6.3 Implement compute_portfolio_xva
- [x] Parallel computation across counterparties using Rayon
- [x] Aggregate by counterparty
- [x] Return PortfolioXva
- [x] Unit tests

### 6.4 Implement compute_portfolio_xva_soa
- [x] ExposureSoA-based computation for cache efficiency
- [x] Parallel computation across counterparties
- [x] Unit tests

---

## Task 7: Integration Utilities

**Status**: Complete

### 7.1 Create integration helpers
- [x] Trapezoidal integration implemented inline in CVA/DVA/FVA functions
- [x] Unit tests for integration accuracy (validated via proportionality tests)

### 7.2 Add discount factor helper
- [x] generate_flat_discount_factors function
- [x] Unit tests

---

## Task 8: Module Exports and Documentation

**Status**: Complete

### 8.1 Update xva/mod.rs
- [x] Export all public types
- [x] Module documentation with architecture diagram
- [x] Examples

### 8.2 Update lib.rs
- [x] Add xva re-exports
- [x] Update crate documentation

---

## Task 9: Integration Tests

**Status**: Complete

### 9.1 End-to-end XVA test
- [x] Create portfolio with trades (test_compute_portfolio_xva)
- [x] Compute exposure profiles
- [x] Calculate XVA
- [x] Verify results

### 9.2 Parallel correctness test
- [x] test_xva_totals_consistency verifies counterparty sums match portfolio totals
- [x] test_compute_portfolio_xva_soa tests SoA-based parallel computation

---

## Completion Checklist

- [x] All unit tests implemented
- [x] All integration tests implemented
- [x] Documentation complete
- [ ] `cargo clippy` clean (requires local verification)
- [ ] `cargo fmt` applied (requires local verification)

---

_Created: 2025-12-30_
_Updated: 2025-12-30_
_Spec: xva-metrics_
_Phase: Tasks - Implementation Complete_
