# Portfolio Aggregation Requirements

## Overview

This specification defines requirements for the portfolio aggregation module in `pricer_xva` (Layer 4). The module provides trade structures, counterparty management, and netting set aggregation for XVA calculations with Rayon-based parallelisation and Structure of Arrays (SoA) memory layout for cache efficiency.

---

## REQ-PORT-001: Trade Structure

### REQ-PORT-001.1: Trade Identifier
**EARS**: The system **shall** provide a `Trade` structure with a unique identifier (UUID or string-based trade ID).

**Acceptance Criteria**:
- Trade IDs are unique within a portfolio
- Trade IDs are serialisable and hashable
- Supports both auto-generated and user-provided identifiers

### REQ-PORT-001.2: Trade Metadata
**EARS**: The system **shall** associate each trade with metadata including:
- Currency (ISO 4217)
- Counterparty ID
- Netting set ID
- Trade date (optional)
- Maturity date

**Acceptance Criteria**:
- Metadata fields are accessible via getter methods
- Currency uses `pricer_core::types::Currency`
- All optional fields use `Option<T>` pattern

### REQ-PORT-001.3: Trade Instrument Reference
**EARS**: The system **shall** associate each trade with an underlying instrument from `pricer_models::instruments::Instrument<f64>`.

**Acceptance Criteria**:
- Generic trade structure supports `Instrument<f64>` payoff computation
- Instrument payoff accessible via delegation
- Expiry accessible via delegation

---

## REQ-PORT-002: Counterparty Structure

### REQ-PORT-002.1: Counterparty Identifier
**EARS**: The system **shall** provide a `Counterparty` structure with a unique identifier and name.

**Acceptance Criteria**:
- Counterparty ID is unique within the system
- Name is an optional descriptive field
- Counterparty is hashable and equatable

### REQ-PORT-002.2: Credit Parameters
**EARS**: The system **shall** associate each counterparty with credit parameters:
- Probability of Default (PD) or hazard rate curve reference
- Loss Given Default (LGD) as a fraction [0, 1]
- Recovery rate (1 - LGD)
- Credit rating (optional)

**Acceptance Criteria**:
- PD can be a flat rate or curve reference
- LGD is validated to be in range [0, 1]
- Recovery rate is derived as `1 - lgd`
- Credit rating is an enum with standard ratings (AAA through D)

### REQ-PORT-002.3: Own Credit Parameters
**EARS**: The system **shall** support "own" credit parameters for DVA calculations representing the institution's own default risk.

**Acceptance Criteria**:
- Separate structure or marker for own-credit parameters
- Used in DVA calculations (counterparty perspective)

---

## REQ-PORT-003: Netting Set Structure

### REQ-PORT-003.1: Netting Set Identifier
**EARS**: The system **shall** provide a `NettingSet` structure grouping trades for exposure aggregation.

**Acceptance Criteria**:
- Netting set ID is unique
- Netting set references a single counterparty
- Trades can be added and removed from netting sets

### REQ-PORT-003.2: Collateral Agreement
**EARS**: WHEN a netting set has a collateral agreement, the system **shall** store:
- Threshold amount
- Minimum transfer amount
- Independent amount
- Collateral currency
- Margin period of risk (MPoR)

**Acceptance Criteria**:
- All amounts are non-negative
- MPoR is in years (typically 10/252 for bilateral, 5/252 for cleared)
- Collateral agreement is optional (None for uncollateralised trades)

### REQ-PORT-003.3: Trade Collection
**EARS**: The system **shall** maintain a collection of trade references within each netting set.

**Acceptance Criteria**:
- Trades are stored by reference (trade ID) not by value
- Supports iteration over trades
- Supports parallel iteration via Rayon

---

## REQ-PORT-004: Portfolio Structure

### REQ-PORT-004.1: Portfolio Container
**EARS**: The system **shall** provide a `Portfolio` structure as the top-level container for:
- Trades (indexed by trade ID)
- Counterparties (indexed by counterparty ID)
- Netting sets (indexed by netting set ID)

**Acceptance Criteria**:
- O(1) lookup by ID for all entities
- Supports bulk insertion
- Provides summary statistics (trade count, counterparty count, netting set count)

### REQ-PORT-004.2: Portfolio Building
**EARS**: The system **shall** provide a builder pattern for portfolio construction.

**Acceptance Criteria**:
- `PortfolioBuilder` supports fluent API
- Validation occurs on `build()` (e.g., trade references valid counterparties/netting sets)
- Builder returns `Result<Portfolio, PortfolioError>`

### REQ-PORT-004.3: Parallel Iteration
**EARS**: The system **shall** support parallel iteration over trades, netting sets, and counterparties via Rayon.

**Acceptance Criteria**:
- `trades_par_iter()` returns `ParallelIterator` over trades
- `netting_sets_par_iter()` returns `ParallelIterator` over netting sets
- Thread-safe read access during parallel iteration

---

## REQ-PORT-005: Structure of Arrays (SoA) Layout

### REQ-PORT-005.1: SoA Trade Data
**EARS**: The system **shall** provide an SoA representation for batch processing of trade data.

**Acceptance Criteria**:
- Separate arrays for: spots, strikes, maturities, notionals
- Arrays are contiguous in memory (Vec<f64>)
- Conversion from Portfolio to SoA representation

### REQ-PORT-005.2: SoA Exposure Data
**EARS**: The system **shall** provide an SoA representation for exposure profiles.

**Acceptance Criteria**:
- Time grid array (Vec<f64>)
- Exposure arrays per netting set (Vec<Vec<f64>>)
- Memory-aligned for SIMD operations where applicable

### REQ-PORT-005.3: Cache Efficiency
**EARS**: The system **shall** organise SoA data for cache-efficient access patterns.

**Acceptance Criteria**:
- Sequential access patterns preferred
- Strided access minimised
- Benchmark demonstrates >80% L1 cache hit rate for sequential iteration

---

## REQ-PORT-006: Exposure Aggregation

### REQ-PORT-006.1: Expected Exposure (EE)
**EARS**: The system **shall** compute Expected Exposure (EE) at each time point.

**Acceptance Criteria**:
- EE(t) = E[max(V(t), 0)]
- Computed per netting set
- Supports parallel computation across scenarios

### REQ-PORT-006.2: Expected Positive Exposure (EPE)
**EARS**: The system **shall** compute time-weighted average EPE.

**Acceptance Criteria**:
- EPE = (1/T) ∫₀ᵀ EE(t) dt
- Numerical integration over time grid
- Single scalar per netting set

### REQ-PORT-006.3: Potential Future Exposure (PFE)
**EARS**: The system **shall** compute PFE at specified confidence levels.

**Acceptance Criteria**:
- PFE at 95% and 99% confidence levels
- Uses quantile of exposure distribution
- Per time point or peak PFE across horizon

### REQ-PORT-006.4: Netting Benefit
**EARS**: The system **shall** compute exposure with and without netting.

**Acceptance Criteria**:
- Gross exposure: sum of absolute values
- Net exposure: max(sum of values, 0)
- Netting benefit ratio computable

---

## REQ-PORT-007: Error Handling

### REQ-PORT-007.1: Portfolio Errors
**EARS**: The system **shall** provide structured error types for portfolio operations.

**Acceptance Criteria**:
- `PortfolioError` enum with variants:
  - `TradeNotFound(TradeId)`
  - `CounterpartyNotFound(CounterpartyId)`
  - `NettingSetNotFound(NettingSetId)`
  - `DuplicateTrade(TradeId)`
  - `InvalidCollateralAgreement(String)`
  - `BuilderError(String)`
- Implements `std::error::Error`
- Uses `thiserror` for derivation

---

## REQ-PORT-008: Serialisation

### REQ-PORT-008.1: Serde Support
**EARS**: WHEN the `serde` feature is enabled, the system **shall** support serialisation and deserialisation of all portfolio structures.

**Acceptance Criteria**:
- All public structs derive `Serialize` and `Deserialize` under `#[cfg(feature = "serde")]`
- JSON round-trip preserves all data
- Trade IDs and counterparty IDs serialise as strings

---

## Non-Functional Requirements

### NFR-PORT-001: Performance
- Parallel iteration efficiency >80% on 8+ cores
- Memory allocation minimised during batch operations
- SoA operations vectorisable by LLVM

### NFR-PORT-002: Thread Safety
- All structures are `Send + Sync`
- No interior mutability except where explicitly documented
- Rayon integration uses safe parallelism patterns

### NFR-PORT-003: Documentation
- All public items documented with rustdoc
- Examples provided for core operations
- Module-level documentation explaining architecture

---

_Created: 2025-12-30_
_Spec: portfolio-aggregation_
_Phase: Requirements_
