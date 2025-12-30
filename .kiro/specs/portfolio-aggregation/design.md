# Portfolio Aggregation Technical Design

## Overview

This document specifies the technical design for the portfolio aggregation module in `pricer_xva` (L4). The design focuses on Rayon parallelisation, Structure of Arrays (SoA) memory layout, and integration with the pricer_kernel (L3) Monte Carlo engine.

---

## Architecture

### Module Structure

```
crates/pricer_xva/src/
├── lib.rs              # Module exports
├── portfolio/
│   ├── mod.rs          # Portfolio container
│   ├── trade.rs        # Trade structure
│   ├── counterparty.rs # Counterparty with credit params
│   ├── netting_set.rs  # Netting set with collateral
│   ├── builder.rs      # PortfolioBuilder
│   └── error.rs        # PortfolioError
├── soa/
│   ├── mod.rs          # SoA exports
│   ├── trade_soa.rs    # SoA trade representation
│   └── exposure_soa.rs # SoA exposure profiles
├── parallel/
│   └── mod.rs          # Rayon parallel iterators
└── exposure/
    └── mod.rs          # EE, EPE, PFE computations
```

### Dependency Flow

```
pricer_xva (L4)
    ├── pricer_core (L1)    → Currency, types
    ├── pricer_models (L2)  → Instrument<f64>
    ├── pricer_kernel (L3)  → MonteCarloPricer, PricingResult
    └── rayon               → Parallel iteration
```

---

## Core Data Structures

### Trade (REQ-PORT-001)

```rust
/// Unique identifier for a trade.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TradeId(String);

impl TradeId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}

/// Trade with instrument and metadata.
#[derive(Clone, Debug)]
pub struct Trade {
    id: TradeId,
    instrument: Instrument<f64>,
    currency: Currency,
    counterparty_id: CounterpartyId,
    netting_set_id: NettingSetId,
    notional: f64,
    trade_date: Option<Date>,
    maturity_date: Option<Date>,
}

impl Trade {
    /// Computes payoff at given spot price.
    #[inline]
    pub fn payoff(&self, spot: f64) -> f64 {
        self.instrument.payoff(spot) * self.notional
    }

    /// Returns instrument expiry.
    #[inline]
    pub fn expiry(&self) -> f64 {
        self.instrument.expiry()
    }

    // Getters for all fields...
}
```

**Design Rationale**:
- `TradeId` is a newtype for type safety
- Instrument is stored by value (enum dispatch, no heap allocation for trait objects)
- Notional is separate from instrument for flexibility

### Counterparty (REQ-PORT-002)

```rust
/// Unique identifier for a counterparty.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CounterpartyId(String);

/// Credit rating enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditRating {
    AAA, AA, A, BBB, BB, B, CCC, CC, C, D,
}

/// Credit parameters for a counterparty.
#[derive(Clone, Debug)]
pub struct CreditParams {
    /// Hazard rate (flat or curve reference)
    hazard_rate: f64,
    /// Loss Given Default [0, 1]
    lgd: f64,
    /// Optional credit rating
    rating: Option<CreditRating>,
}

impl CreditParams {
    pub fn new(hazard_rate: f64, lgd: f64) -> Result<Self, PortfolioError>;

    /// Recovery rate = 1 - LGD
    #[inline]
    pub fn recovery_rate(&self) -> f64 {
        1.0 - self.lgd
    }

    /// Survival probability to time t
    #[inline]
    pub fn survival_prob(&self, t: f64) -> f64 {
        (-self.hazard_rate * t).exp()
    }

    /// Default probability to time t
    #[inline]
    pub fn default_prob(&self, t: f64) -> f64 {
        1.0 - self.survival_prob(t)
    }
}

/// Counterparty entity.
#[derive(Clone, Debug)]
pub struct Counterparty {
    id: CounterpartyId,
    name: Option<String>,
    credit_params: CreditParams,
}
```

**Design Rationale**:
- Flat hazard rate initially; curve reference can be added later
- LGD validated on construction
- Survival/default probability methods inlined for performance

### NettingSet (REQ-PORT-003)

```rust
/// Unique identifier for a netting set.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NettingSetId(String);

/// Collateral agreement parameters.
#[derive(Clone, Debug)]
pub struct CollateralAgreement {
    /// Threshold amount
    threshold: f64,
    /// Minimum transfer amount
    mta: f64,
    /// Independent amount (positive = we post)
    independent_amount: f64,
    /// Collateral currency
    currency: Currency,
    /// Margin period of risk in years
    mpor: f64,
}

impl CollateralAgreement {
    pub fn new(
        threshold: f64,
        mta: f64,
        independent_amount: f64,
        currency: Currency,
        mpor: f64,
    ) -> Result<Self, PortfolioError>;

    /// Standard bilateral MPoR (10 business days)
    pub fn bilateral_mpor() -> f64 { 10.0 / 252.0 }

    /// Standard cleared MPoR (5 business days)
    pub fn cleared_mpor() -> f64 { 5.0 / 252.0 }
}

/// Netting set grouping trades.
#[derive(Clone, Debug)]
pub struct NettingSet {
    id: NettingSetId,
    counterparty_id: CounterpartyId,
    trade_ids: Vec<TradeId>,
    collateral: Option<CollateralAgreement>,
}

impl NettingSet {
    pub fn add_trade(&mut self, trade_id: TradeId);
    pub fn remove_trade(&mut self, trade_id: &TradeId) -> bool;
    pub fn trade_ids(&self) -> &[TradeId];
    pub fn is_collateralised(&self) -> bool;
}
```

**Design Rationale**:
- Trade IDs stored by reference to avoid duplication
- Collateral agreement optional for uncollateralised netting sets
- MPoR constants provided for common cases

### Portfolio (REQ-PORT-004)

```rust
use std::collections::HashMap;

/// Portfolio container.
pub struct Portfolio {
    trades: HashMap<TradeId, Trade>,
    counterparties: HashMap<CounterpartyId, Counterparty>,
    netting_sets: HashMap<NettingSetId, NettingSet>,
}

impl Portfolio {
    /// Gets a trade by ID.
    pub fn trade(&self, id: &TradeId) -> Option<&Trade>;

    /// Gets all trades in a netting set.
    pub fn trades_in_netting_set(&self, ns_id: &NettingSetId) -> Vec<&Trade>;

    /// Gets a counterparty by ID.
    pub fn counterparty(&self, id: &CounterpartyId) -> Option<&Counterparty>;

    /// Gets a netting set by ID.
    pub fn netting_set(&self, id: &NettingSetId) -> Option<&NettingSet>;

    /// Returns iterator over all trades.
    pub fn trades(&self) -> impl Iterator<Item = &Trade>;

    /// Returns parallel iterator over all trades.
    pub fn trades_par_iter(&self) -> impl ParallelIterator<Item = &Trade>;

    /// Returns parallel iterator over netting sets.
    pub fn netting_sets_par_iter(&self) -> impl ParallelIterator<Item = &NettingSet>;

    /// Summary statistics.
    pub fn trade_count(&self) -> usize;
    pub fn counterparty_count(&self) -> usize;
    pub fn netting_set_count(&self) -> usize;
}
```

### PortfolioBuilder (REQ-PORT-004.2)

```rust
/// Builder for constructing portfolios.
#[derive(Default)]
pub struct PortfolioBuilder {
    trades: Vec<Trade>,
    counterparties: Vec<Counterparty>,
    netting_sets: Vec<NettingSet>,
}

impl PortfolioBuilder {
    pub fn new() -> Self;

    pub fn add_trade(self, trade: Trade) -> Self;
    pub fn add_trades(self, trades: impl IntoIterator<Item = Trade>) -> Self;

    pub fn add_counterparty(self, counterparty: Counterparty) -> Self;
    pub fn add_counterparties(self, cps: impl IntoIterator<Item = Counterparty>) -> Self;

    pub fn add_netting_set(self, ns: NettingSet) -> Self;
    pub fn add_netting_sets(self, nss: impl IntoIterator<Item = NettingSet>) -> Self;

    /// Builds and validates the portfolio.
    pub fn build(self) -> Result<Portfolio, PortfolioError>;
}
```

---

## Structure of Arrays (SoA) Design (REQ-PORT-005)

### TradeSoA

```rust
/// SoA representation of trade data for vectorised operations.
pub struct TradeSoA {
    /// Trade IDs (for reference back)
    pub trade_ids: Vec<TradeId>,
    /// Strike prices
    pub strikes: Vec<f64>,
    /// Maturities (years)
    pub maturities: Vec<f64>,
    /// Notional amounts
    pub notionals: Vec<f64>,
    /// Payoff types encoded as i8 (Call=1, Put=-1)
    pub payoff_signs: Vec<i8>,
}

impl TradeSoA {
    /// Creates SoA from portfolio trades.
    pub fn from_trades(trades: &[&Trade]) -> Self;

    /// Number of trades.
    #[inline]
    pub fn len(&self) -> usize { self.trade_ids.len() }

    /// Computes payoffs for all trades at given spots.
    ///
    /// # Arguments
    /// * `spots` - Terminal spot prices (one per trade)
    /// * `payoffs` - Output buffer (must be same length)
    pub fn compute_payoffs(&self, spots: &[f64], payoffs: &mut [f64]);
}
```

**Design Rationale**:
- Separate arrays enable SIMD vectorisation
- `payoff_signs` encodes Call/Put without branching
- Pre-allocated output buffer avoids allocation in hot path

### ExposureSoA

```rust
/// SoA representation of exposure profiles.
pub struct ExposureSoA {
    /// Time grid (shared across all netting sets)
    pub time_grid: Vec<f64>,
    /// Exposure profiles [netting_set_idx][time_idx]
    pub exposures: Vec<Vec<f64>>,
    /// Netting set IDs (for reference)
    pub netting_set_ids: Vec<NettingSetId>,
}

impl ExposureSoA {
    /// Creates empty exposure grid.
    pub fn new(time_grid: Vec<f64>, n_netting_sets: usize) -> Self;

    /// Gets exposure profile for a netting set.
    pub fn exposure_profile(&self, ns_idx: usize) -> &[f64];

    /// Gets exposure at a specific time for all netting sets.
    pub fn exposures_at_time(&self, time_idx: usize) -> Vec<f64>;
}
```

---

## Exposure Aggregation (REQ-PORT-006)

### Exposure Calculator

```rust
/// Exposure calculation utilities.
pub struct ExposureCalculator;

impl ExposureCalculator {
    /// Computes Expected Exposure at each time point.
    ///
    /// EE(t) = E[max(V(t), 0)]
    ///
    /// # Arguments
    /// * `values` - Simulated values [scenario_idx][time_idx]
    /// * `time_grid` - Time points
    ///
    /// # Returns
    /// Expected exposure at each time point.
    pub fn expected_exposure(values: &[Vec<f64>], time_grid: &[f64]) -> Vec<f64> {
        let n_times = time_grid.len();
        let n_scenarios = values.len();

        (0..n_times)
            .into_par_iter()
            .map(|t| {
                let sum: f64 = values.iter()
                    .map(|path| path[t].max(0.0))
                    .sum();
                sum / n_scenarios as f64
            })
            .collect()
    }

    /// Computes time-weighted EPE.
    ///
    /// EPE = (1/T) ∫₀ᵀ EE(t) dt
    pub fn expected_positive_exposure(ee: &[f64], time_grid: &[f64]) -> f64 {
        if time_grid.len() < 2 {
            return ee.get(0).copied().unwrap_or(0.0);
        }

        // Trapezoidal integration
        let mut integral = 0.0;
        for i in 0..time_grid.len() - 1 {
            let dt = time_grid[i + 1] - time_grid[i];
            integral += 0.5 * (ee[i] + ee[i + 1]) * dt;
        }

        let total_time = time_grid.last().unwrap() - time_grid.first().unwrap();
        integral / total_time
    }

    /// Computes PFE at given confidence level.
    ///
    /// # Arguments
    /// * `values` - Simulated values [scenario_idx][time_idx]
    /// * `confidence` - Confidence level (e.g., 0.95)
    ///
    /// # Returns
    /// PFE at each time point.
    pub fn potential_future_exposure(
        values: &[Vec<f64>],
        time_grid: &[f64],
        confidence: f64,
    ) -> Vec<f64> {
        let n_times = time_grid.len();
        let n_scenarios = values.len();
        let quantile_idx = ((n_scenarios as f64) * confidence) as usize;

        (0..n_times)
            .into_par_iter()
            .map(|t| {
                let mut exposures: Vec<f64> = values.iter()
                    .map(|path| path[t].max(0.0))
                    .collect();
                exposures.sort_by(|a, b| a.partial_cmp(b).unwrap());
                exposures.get(quantile_idx).copied().unwrap_or(0.0)
            })
            .collect()
    }

    /// Computes gross and net exposure.
    pub fn netting_benefit(trade_values: &[f64]) -> (f64, f64) {
        let gross = trade_values.iter().map(|v| v.abs()).sum();
        let net = trade_values.iter().sum::<f64>().max(0.0);
        (gross, net)
    }
}
```

---

## Parallel Iteration (REQ-PORT-004.3)

### Rayon Integration

```rust
use rayon::prelude::*;

impl Portfolio {
    /// Parallel iterator over trades.
    pub fn trades_par_iter(&self) -> impl ParallelIterator<Item = (&TradeId, &Trade)> {
        self.trades.par_iter()
    }

    /// Parallel iterator over netting sets.
    pub fn netting_sets_par_iter(&self) -> impl ParallelIterator<Item = (&NettingSetId, &NettingSet)> {
        self.netting_sets.par_iter()
    }

    /// Parallel pricing of all trades.
    pub fn price_all_trades<F>(&self, pricer_fn: F) -> HashMap<TradeId, f64>
    where
        F: Fn(&Trade) -> f64 + Sync,
    {
        self.trades
            .par_iter()
            .map(|(id, trade)| (id.clone(), pricer_fn(trade)))
            .collect()
    }

    /// Parallel aggregation by netting set.
    pub fn aggregate_by_netting_set<F>(&self, agg_fn: F) -> HashMap<NettingSetId, f64>
    where
        F: Fn(&[&Trade]) -> f64 + Sync,
    {
        self.netting_sets
            .par_iter()
            .map(|(ns_id, ns)| {
                let trades: Vec<&Trade> = ns.trade_ids()
                    .iter()
                    .filter_map(|tid| self.trades.get(tid))
                    .collect();
                (ns_id.clone(), agg_fn(&trades))
            })
            .collect()
    }
}
```

---

## Error Handling (REQ-PORT-007)

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PortfolioError {
    #[error("Trade not found: {0}")]
    TradeNotFound(String),

    #[error("Counterparty not found: {0}")]
    CounterpartyNotFound(String),

    #[error("Netting set not found: {0}")]
    NettingSetNotFound(String),

    #[error("Duplicate trade ID: {0}")]
    DuplicateTrade(String),

    #[error("Duplicate counterparty ID: {0}")]
    DuplicateCounterparty(String),

    #[error("Duplicate netting set ID: {0}")]
    DuplicateNettingSet(String),

    #[error("Invalid credit parameters: {0}")]
    InvalidCreditParams(String),

    #[error("Invalid collateral agreement: {0}")]
    InvalidCollateralAgreement(String),

    #[error("Trade references unknown counterparty: trade={0}, counterparty={1}")]
    UnknownCounterpartyReference(String, String),

    #[error("Trade references unknown netting set: trade={0}, netting_set={1}")]
    UnknownNettingSetReference(String, String),

    #[error("Builder error: {0}")]
    BuilderError(String),
}
```

---

## Integration with pricer_kernel (L3)

### Monte Carlo Pricing Integration

```rust
use pricer_kernel::mc::{MonteCarloPricer, MonteCarloConfig, GbmParams, PayoffParams};

impl Trade {
    /// Converts trade to pricer_kernel parameters.
    pub fn to_mc_params(&self, spot: f64, rate: f64, volatility: f64) -> (GbmParams, PayoffParams) {
        let gbm = GbmParams {
            spot,
            rate,
            volatility,
            maturity: self.expiry(),
        };

        let payoff = match self.instrument {
            Instrument::Vanilla(ref opt) => {
                let strike = opt.params().strike();
                match opt.payoff_type() {
                    PayoffType::Call => PayoffParams::call(strike),
                    PayoffType::Put => PayoffParams::put(strike),
                    _ => PayoffParams::call(strike), // Fallback
                }
            }
            _ => PayoffParams::call(100.0), // Placeholder for non-vanilla
        };

        (gbm, payoff)
    }
}
```

---

## Memory Layout Considerations

### Cache Line Alignment

```rust
// Ensure SoA arrays are cache-line aligned (64 bytes)
#[repr(align(64))]
pub struct AlignedF64Vec(Vec<f64>);

impl TradeSoA {
    /// Creates with cache-aligned storage.
    pub fn with_capacity_aligned(capacity: usize) -> Self {
        Self {
            trade_ids: Vec::with_capacity(capacity),
            strikes: Vec::with_capacity(capacity),
            maturities: Vec::with_capacity(capacity),
            notionals: Vec::with_capacity(capacity),
            payoff_signs: Vec::with_capacity(capacity),
        }
    }
}
```

### Batch Processing Pattern

```rust
impl TradeSoA {
    /// Batch process trades in chunks for cache efficiency.
    pub fn process_in_batches<F>(&self, batch_size: usize, processor: F)
    where
        F: Fn(&[f64], &[f64], &[f64], &mut [f64]) + Sync,
    {
        let n = self.len();
        (0..n)
            .into_par_iter()
            .step_by(batch_size)
            .for_each(|start| {
                let end = (start + batch_size).min(n);
                // Process batch [start..end]
                let mut output = vec![0.0; end - start];
                processor(
                    &self.strikes[start..end],
                    &self.maturities[start..end],
                    &self.notionals[start..end],
                    &mut output,
                );
            });
    }
}
```

---

## Testing Strategy

### Unit Tests
- Trade creation and payoff computation
- Counterparty credit parameter validation
- Netting set trade management
- Portfolio builder validation
- SoA conversion correctness

### Integration Tests
- Portfolio with multiple netting sets
- Parallel iteration correctness
- Exposure aggregation accuracy

### Property-Based Tests
- Netting benefit: net <= gross always
- EPE: bounded by max(EE)
- Survival probability: monotonically decreasing

### Benchmarks
- Sequential vs parallel iteration scaling
- SoA vs AoS memory access patterns
- Exposure computation with varying portfolio sizes

---

_Created: 2025-12-30_
_Spec: portfolio-aggregation_
_Phase: Design_
