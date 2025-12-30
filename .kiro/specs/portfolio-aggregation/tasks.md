# Portfolio Aggregation Implementation Tasks

## Overview

Implementation tasks for the portfolio aggregation module in `pricer_xva` (L4).

---

## Task 1: Error Types and Identifiers

**Status**: Pending

### 1.1 Create PortfolioError enum
- [ ] Create `portfolio/error.rs`
- [ ] Define `PortfolioError` enum with thiserror
- [ ] Implement Display and Error traits
- [ ] Add unit tests for error formatting

### 1.2 Create Identifier Types
- [ ] Create `portfolio/ids.rs` (or add to error.rs)
- [ ] Define `TradeId`, `CounterpartyId`, `NettingSetId` newtypes
- [ ] Implement Clone, Debug, PartialEq, Eq, Hash
- [ ] Add serde support under feature flag
- [ ] Unit tests for ID operations

---

## Task 2: Credit Parameters and Counterparty

**Status**: Pending

### 2.1 Create CreditRating enum
- [ ] Define credit rating variants (AAA through D)
- [ ] Add serde support

### 2.2 Create CreditParams struct
- [ ] Implement hazard_rate, lgd fields
- [ ] Validate LGD in [0, 1] on construction
- [ ] Implement `recovery_rate()`, `survival_prob()`, `default_prob()`
- [ ] Unit tests for probability computations

### 2.3 Create Counterparty struct
- [ ] Create `portfolio/counterparty.rs`
- [ ] Implement Counterparty with id, name, credit_params
- [ ] Add builder or constructor
- [ ] Unit tests

---

## Task 3: Collateral Agreement and Netting Set

**Status**: Pending

### 3.1 Create CollateralAgreement struct
- [ ] Define threshold, mta, independent_amount, currency, mpor fields
- [ ] Validate non-negative amounts
- [ ] Add `bilateral_mpor()` and `cleared_mpor()` constants
- [ ] Unit tests

### 3.2 Create NettingSet struct
- [ ] Create `portfolio/netting_set.rs`
- [ ] Implement NettingSet with trade_ids collection
- [ ] Add `add_trade()`, `remove_trade()`, `trade_ids()` methods
- [ ] Unit tests for trade management

---

## Task 4: Trade Structure

**Status**: Pending

### 4.1 Create Trade struct
- [ ] Create `portfolio/trade.rs`
- [ ] Define Trade with instrument, currency, notional, metadata
- [ ] Implement `payoff()` delegating to instrument
- [ ] Implement `expiry()` delegating to instrument
- [ ] Getters for all fields
- [ ] Unit tests with Instrument<f64>

### 4.2 Add MC parameter conversion
- [ ] Implement `to_mc_params()` for pricer_kernel integration
- [ ] Unit tests with GbmParams/PayoffParams output

---

## Task 5: Portfolio Container

**Status**: Pending

### 5.1 Create Portfolio struct
- [ ] Create `portfolio/mod.rs` with Portfolio struct
- [ ] Use HashMap for O(1) lookup by ID
- [ ] Implement trade/counterparty/netting_set accessors
- [ ] Implement `trades_in_netting_set()`
- [ ] Add count methods

### 5.2 Add iterator methods
- [ ] Implement `trades()` iterator
- [ ] Implement `netting_sets()` iterator
- [ ] Implement `counterparties()` iterator

---

## Task 6: Portfolio Builder

**Status**: Pending

### 6.1 Create PortfolioBuilder
- [ ] Create `portfolio/builder.rs`
- [ ] Implement fluent API for adding entities
- [ ] Implement `build()` with validation

### 6.2 Add validation logic
- [ ] Check for duplicate IDs
- [ ] Validate trade → counterparty references
- [ ] Validate trade → netting_set references
- [ ] Validate netting_set → counterparty references
- [ ] Integration tests for validation errors

---

## Task 7: Parallel Iteration

**Status**: Pending

### 7.1 Add Rayon parallel iterators
- [ ] Update `parallel/mod.rs`
- [ ] Implement `trades_par_iter()`
- [ ] Implement `netting_sets_par_iter()`
- [ ] Implement `counterparties_par_iter()`

### 7.2 Add parallel aggregation methods
- [ ] Implement `price_all_trades()`
- [ ] Implement `aggregate_by_netting_set()`
- [ ] Benchmark parallel vs sequential

---

## Task 8: Structure of Arrays (SoA)

**Status**: Pending

### 8.1 Create TradeSoA
- [ ] Create `soa/trade_soa.rs`
- [ ] Implement `from_trades()` conversion
- [ ] Implement `compute_payoffs()` vectorised method
- [ ] Unit tests for SoA correctness

### 8.2 Create ExposureSoA
- [ ] Create `soa/exposure_soa.rs`
- [ ] Define time_grid and exposures storage
- [ ] Implement accessor methods
- [ ] Unit tests

---

## Task 9: Exposure Aggregation

**Status**: Pending

### 9.1 Create ExposureCalculator
- [ ] Create `exposure/mod.rs`
- [ ] Implement `expected_exposure()`
- [ ] Implement `expected_positive_exposure()`
- [ ] Implement `potential_future_exposure()`
- [ ] Implement `netting_benefit()`

### 9.2 Add tests
- [ ] Unit tests with known values
- [ ] Property tests (net <= gross)
- [ ] Integration tests with simulated values

---

## Task 10: Integration and Documentation

**Status**: Pending

### 10.1 Update lib.rs exports
- [ ] Export all public types
- [ ] Add module documentation
- [ ] Add crate-level examples

### 10.2 Integration tests
- [ ] Full portfolio workflow test
- [ ] Parallel pricing integration test
- [ ] SoA batch processing test

### 10.3 Benchmarks
- [ ] Add criterion benchmarks for parallel scaling
- [ ] Benchmark SoA vs AoS access patterns

---

## Completion Checklist

- [ ] All unit tests passing
- [ ] All integration tests passing
- [ ] Documentation complete
- [ ] `cargo clippy` clean
- [ ] `cargo fmt` applied

---

_Created: 2025-12-30_
_Spec: portfolio-aggregation_
_Phase: Tasks_
