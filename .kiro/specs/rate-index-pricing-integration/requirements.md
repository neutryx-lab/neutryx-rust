# Requirements Document

## Project Description (Input)
Implement comprehensive RateIndex integration across the Neutryx pricing pipeline

## Current State
- RateIndex enum exists in infra_master with SOFR, TONAR, EURIBOR3M/6M, SONIA, SARON
- IndexType enum wraps Rate, SwapRate, Fx, Equity, Inflation, Commodity indices
- Payoff::Linear and Payoff::VanillaOption contain index: IndexType field
- Cashflow.payoff carries index information

## Problem
Index information is defined but NOT USED in the actual pricing process:
1. GenericPricer ignores cf.payoff - calculates cf_amount = year_fraction * notional (hardcoded)
2. MarketProvider only maps Currency → Curve, not RateIndex → Curve
3. No forward rate lookup for floating cashflows
4. No spread application (rate + spread)
5. Demo WebApp API doesn't expose Index in request/response DTOs

## Required Changes

### Phase 1: Enhance infra_master Index model
- Add fixing calendar, publication lag, fixing offset to RateIndex
- Add compounding method (Simple, Compounded, Averaged) to IndexObservation
- Add reset frequency for floating legs
- Ensure IndexType has all metadata needed for pricing

### Phase 2: Market data index-to-curve mapping
- Add RateIndex → CurveName mapping in pricer_models
- Extend CurveSet to lookup curves by RateIndex
- Add index-aware forward rate methods

### Phase 3: GenericPricer floating rate support
- Evaluate Payoff::Linear: get index rate from curve, apply spread and multiplier
- Evaluate Payoff::VanillaOption: Black/Bachelier cap/floor pricing
- Support OIS compounding with daily_accruals
- Handle index fixing observation correctly

### Phase 4: Demo WebApp API updates
- Add rate_index field to SwapParams, RatesParams input DTOs
- Add rate_index field to LegDto, CashflowDto output DTOs
- Propagate index through convert_trade_to_dto()

## Constraints
- Maintain backward compatibility with existing tests
- Support automatic differentiation (Dual64 generic)
- Follow A-I-P-S architecture (no cross-layer violations)
- British English naming (optimiser, serialisation)

## Requirements
<!-- Will be generated in /kiro:spec-requirements phase -->
