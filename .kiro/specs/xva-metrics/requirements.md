# XVA Metrics Requirements

## Overview

This specification defines requirements for the XVA (Valuation Adjustments) metrics module in `pricer_xva` (Layer 4). The module provides CVA, DVA, and FVA calculations for derivatives portfolios with Rayon-based parallelisation.

---

## REQ-XVA-001: Credit Valuation Adjustment (CVA)

### REQ-XVA-001.1: Unilateral CVA Formula
**EARS**: The system **shall** compute unilateral CVA using the formula:

CVA = LGD_c × ∫₀ᵀ EE(t) × dPD_c(t)

Where:
- LGD_c = Loss Given Default of counterparty
- EE(t) = Expected Exposure at time t
- dPD_c(t) = Marginal default probability of counterparty

**Acceptance Criteria**:
- CVA is non-negative
- CVA increases with credit risk (higher PD or LGD)
- Numerical integration uses time grid from exposure profile

### REQ-XVA-001.2: Bilateral CVA
**EARS**: WHEN both parties have default risk, the system **shall** compute bilateral CVA incorporating both counterparty and own default.

**Acceptance Criteria**:
- Bilateral CVA = Unilateral CVA - DVA
- Accounts for first-to-default timing
- Optional survival probability weighting

### REQ-XVA-001.3: CVA by Netting Set
**EARS**: The system **shall** compute CVA at netting set level and aggregate to counterparty level.

**Acceptance Criteria**:
- Individual netting set CVAs available
- Counterparty-level CVA = sum of netting set CVAs
- Portfolio-level CVA = sum of counterparty CVAs

---

## REQ-XVA-002: Debit Valuation Adjustment (DVA)

### REQ-XVA-002.1: DVA Formula
**EARS**: The system **shall** compute DVA using the formula:

DVA = LGD_own × ∫₀ᵀ ENE(t) × dPD_own(t)

Where:
- LGD_own = Own Loss Given Default
- ENE(t) = Expected Negative Exposure at time t
- dPD_own(t) = Own marginal default probability

**Acceptance Criteria**:
- DVA is non-negative
- DVA reflects benefit from own default risk
- Uses own credit parameters (separate from counterparty)

### REQ-XVA-002.2: DVA Input Parameters
**EARS**: The system **shall** accept own-credit parameters as input.

**Acceptance Criteria**:
- Own hazard rate or credit spread
- Own LGD
- Optional: own recovery rate derived from LGD

---

## REQ-XVA-003: Funding Valuation Adjustment (FVA)

### REQ-XVA-003.1: FVA Formula
**EARS**: The system **shall** compute FVA using the formula:

FVA = ∫₀ᵀ [EE(t) × s_f^+ - ENE(t) × s_f^-] × df(t) dt

Where:
- s_f^+ = Funding spread for borrowing
- s_f^- = Funding spread for lending
- df(t) = Discount factor

**Acceptance Criteria**:
- FVA can be positive (cost) or negative (benefit)
- Symmetric spreads supported (s_f^+ = s_f^-)
- Asymmetric spreads supported (s_f^+ ≠ s_f^-)

### REQ-XVA-003.2: Funding Cost (FCA) and Benefit (FBA)
**EARS**: The system **shall** separately compute:
- FCA: Funding Cost Adjustment (positive exposure component)
- FBA: Funding Benefit Adjustment (negative exposure component)

**Acceptance Criteria**:
- FCA ≥ 0
- FBA ≥ 0
- FVA = FCA - FBA (net funding impact)

### REQ-XVA-003.3: Funding Spread Parameters
**EARS**: The system **shall** accept funding spread parameters.

**Acceptance Criteria**:
- Flat funding spreads supported
- Asymmetric borrowing/lending spreads
- Spreads expressed as annualised basis points or decimal

---

## REQ-XVA-004: XVA Calculation Engine

### REQ-XVA-004.1: XVA Calculator Structure
**EARS**: The system **shall** provide an `XvaCalculator` structure orchestrating XVA calculations.

**Acceptance Criteria**:
- Accepts portfolio and exposure profiles as input
- Accepts credit parameters (counterparty and own)
- Accepts funding parameters
- Returns structured XVA results

### REQ-XVA-004.2: XVA Result Structure
**EARS**: The system **shall** return XVA results in a structured format.

**Acceptance Criteria**:
- `XvaResult` containing: CVA, DVA, FVA (FCA, FBA)
- Results per netting set
- Aggregated results at counterparty and portfolio level
- Optional: XVA sensitivities (Greeks)

### REQ-XVA-004.3: Time Grid Integration
**EARS**: The system **shall** use numerical integration over time grid.

**Acceptance Criteria**:
- Trapezoidal integration as default
- Consistent with exposure profile time grid
- Handles irregular time steps

---

## REQ-XVA-005: Parallel Computation

### REQ-XVA-005.1: Parallel by Netting Set
**EARS**: The system **shall** compute XVA in parallel across netting sets.

**Acceptance Criteria**:
- Uses Rayon for parallel iteration
- Thread-safe aggregation
- Linear scaling with netting set count

### REQ-XVA-005.2: Parallel by Counterparty
**EARS**: The system **shall** support parallel computation by counterparty.

**Acceptance Criteria**:
- Counterparty-level parallelism for large portfolios
- Proper credit parameter lookup per counterparty

---

## REQ-XVA-006: Collateral Impact

### REQ-XVA-006.1: Collateralised Exposure for XVA
**EARS**: WHEN a netting set has a collateral agreement, the system **shall** use collateralised exposure for XVA calculations.

**Acceptance Criteria**:
- Applies threshold and independent amount
- Accounts for margin period of risk (MPoR)
- Reduces CVA/FVA for collateralised netting sets

### REQ-XVA-006.2: Close-Out Period
**EARS**: The system **shall** account for closeout risk during MPoR.

**Acceptance Criteria**:
- Exposure can increase during MPoR
- Uses worst-case exposure during closeout
- Configurable MPoR per netting set

---

## REQ-XVA-007: Wrong-Way Risk (Optional)

### REQ-XVA-007.1: General Wrong-Way Risk
**EARS**: WHEN enabled, the system **shall** support correlation between exposure and default.

**Acceptance Criteria**:
- Correlation parameter between exposure and default
- Increases CVA for positive correlation
- Optional feature (not required for MVP)

---

## REQ-XVA-008: Error Handling

### REQ-XVA-008.1: XVA Errors
**EARS**: The system **shall** provide structured error types for XVA calculations.

**Acceptance Criteria**:
- `XvaError` enum with variants:
  - `MissingExposureProfile(NettingSetId)`
  - `MissingCreditParams(CounterpartyId)`
  - `InvalidFundingSpread(String)`
  - `IntegrationError(String)`
- Implements `std::error::Error`

---

## Non-Functional Requirements

### NFR-XVA-001: Performance
- Parallel XVA calculation for 1000+ netting sets in <1 second
- Memory-efficient: reuse exposure arrays
- Avoid allocation in integration loops

### NFR-XVA-002: Accuracy
- Numerical integration error <1% for smooth exposure profiles
- CVA formula consistent with BCBS 325 regulatory standards

### NFR-XVA-003: Documentation
- All public items documented with rustdoc
- Mathematical formulas in documentation
- Examples for common use cases

---

_Created: 2025-12-30_
_Spec: xva-metrics_
_Phase: Requirements_
