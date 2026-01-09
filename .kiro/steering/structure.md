# Project Structure

## Organization Philosophy

**Layer-Based Architecture with Dependency Flow**:
Strict bottom-up dependencies, isolating experimental technology (Enzyme) to Layer 3 while keeping foundation and application stable.

```
L1 (pricer_core)    → No dependencies, pure Rust traits/types
L2 (pricer_models)  → Depends on L1 only
L3 (pricer_pricing) → Currently isolated (Phase 3.0), will integrate L1+L2 in Phase 4
L4 (pricer_risk)    → Depends on L1+L2+L3, stable Rust
```

**Phase 3.0 Isolation**: L3 intentionally has zero pricer_* dependencies for complete Enzyme infrastructure isolation. L1/L2 integration planned for Phase 4.

## Directory Patterns

### Cargo Workspace Root

**Location**: `/`
**Purpose**: Workspace configuration and top-level metadata
**Key Files**:
- `Cargo.toml` - Workspace members, shared dependencies, release profile
- `rust-toolchain.toml` - Default stable toolchain (nightly pinned for L3)
- `README.md` - User-facing documentation

### Layer 1: Foundation (pricer_core)

**Location**: `crates/pricer_core/src/`
**Purpose**: Math types, traits, smoothing functions, market data abstractions (stable Rust)
**Structure**:
```
math/
├── smoothing.rs    → Smooth approximations (smooth_max, smooth_indicator)
├── interpolators/  → Interpolation methods (linear, bilinear, cubic_spline, monotonic, smooth_interp)
└── solvers/        → Root-finding algorithms (Newton-Raphson, Brent)

traits/     → Priceable, Differentiable, core abstractions
types/
├── dual.rs      → Dual numbers (num-dual) for AD
├── time.rs      → Date, DayCountConvention for financial calculations
├── currency.rs  → ISO 4217 currency codes with metadata
└── error.rs     → Structured error types (PricingError, DateError, etc.)

market_data/
├── curves/     → Yield curve abstractions (YieldCurve trait, FlatCurve, InterpolatedCurve)
├── surfaces/   → Volatility surface abstractions (VolatilitySurface trait, FlatVol, InterpolatedVolSurface)
└── error.rs    → MarketDataError for curve/surface validation
```

**Key Principles**:

- Zero dependencies on other pricer_* crates, pure foundation
- All market data structures generic over `T: Float` for AD compatibility

### Layer 2: Business Logic (pricer_models)

**Location**: `crates/pricer_models/src/`
**Purpose**: Financial instruments and pricing models (stable Rust)
**Structure**:

```text
instruments/
├── equity/    → Equity derivatives (VanillaOption, Forward)
├── rates/     → Interest rate derivatives (IRS, Swaption, Cap/Floor)
├── credit/    → Credit derivatives (CDS with simulation)
├── fx/        → FX derivatives (FxOption, FxForward)
└── traits.rs  → InstrumentTrait, CashflowInstrument

models/       → Stochastic models with unified trait interface
  ├── equity/   → Equity models (feature-gated)
  ├── rates/    → Interest rate models: Hull-White, CIR (feature-gated)
  └── hybrid/   → Correlated multi-factor models (feature-gated)
analytical/   → Closed-form solutions (Black-Scholes, Garman-Kohlhagen)
calibration/  → Model calibration (Levenberg-Marquardt, swaption vol surface)
schedules/    → Payment schedule generation (Frequency, Period, ScheduleBuilder)
```

**Key Principles**:

- **Hierarchical Instrument Architecture**: `InstrumentEnum<T>` wraps asset-class sub-enums (EquityInstrument, RatesInstrument, CreditInstrument, FxInstrument)
- **Feature-Flag Asset Classes**: Each asset class gated by feature (equity, rates, credit, fx)
- **InstrumentTrait**: Unified interface for all instruments (payoff, expiry, currency, notional)
- **CashflowInstrument**: Extended trait for instruments with scheduled payments (swaps, bonds)
- **Static Dispatch**: Enum-based dispatch at both top and sub-enum levels for Enzyme compatibility
- **Schedule Generation**: `ScheduleBuilder` pattern for IRS/CDS payment schedules
- **StochasticModel Trait**: Unified interface for stochastic processes (`evolve_step`, `initial_state`, `brownian_dim`)
- **StochasticModelEnum**: Static dispatch enum wrapping concrete models (GBM, Hull-White, CIR; future: Heston, SABR)

### Layer 3: AD Engine (pricer_pricing)

**Location**: `crates/pricer_pricing/src/`
**Purpose**: Monte Carlo + Enzyme AD (nightly Rust, LLVM plugin)
**Structure**:

```text
enzyme/          → Enzyme bindings, autodiff macros
mc/              → Monte Carlo kernel (GBM paths, workspace buffers, Greeks, MonteCarloPricer, thread_local)
path_dependent/  → Path-dependent options (Asian, Barrier, Lookback) with streaming statistics
rng/             → Random number generation (PRNG, QMC sequences)
verify/          → Enzyme vs num-dual verification tests
checkpoint/      → Memory management for checkpointing
analytical/      → Closed-form solutions (geometric Asian, barrier options)
greeks/          → Greeks calculation types (GreeksConfig, GreeksMode, GreeksResult<T>)
pool/            → Thread-local buffer pool (ThreadLocalPool, PooledBuffer, PoolStats)
```

**Key Principle**: **Only crate requiring nightly Rust and Enzyme**. Currently isolated (Phase 3.0) with zero pricer_* dependencies.

> **Note**: This crate was renamed from `pricer_kernel` to `pricer_engine` in version 0.7.0, then to `pricer_pricing` for alphabetical ordering with dependency hierarchy.

**RNG Design**: Zero-allocation batch operations, static dispatch only, Enzyme-compatible. Supports reproducible seeding for deterministic simulations.

**Monte Carlo Features** (Phase 3.2+):

- Pre-allocated workspace buffers (`PathWorkspace`, `CheckpointWorkspace`) for allocation-free simulation
- GBM path generation with log-space formulation
- Smooth payoff functions for AD compatibility
- Greeks via bump-and-revalue with forward-mode AD prototype
- **Thread-Local Buffer Pool** (`pool/`): RAII buffer management with `PooledBuffer` for zero-allocation hot paths
- **Parallel Workspaces** (`mc/thread_local.rs`): `ThreadLocalWorkspacePool` and `ParallelWorkspaces` for Rayon integration

**Path-Dependent Options** (Phase 4, Implemented):

- `PathObserver`: Streaming statistics accumulation (average, min, max) without storing full paths
- `PathDependentPayoff` trait: Unified interface for Asian, Barrier, Lookback payoffs
- `PathPayoffType` enum: Static dispatch for payoff types (Enzyme optimization)
- Asian: Arithmetic/geometric averaging with smooth approximations
- Barrier: All 8 variants (up/down, in/out, call/put) with smooth barrier detection
- Lookback: Fixed/floating strike with streaming min/max tracking

**Analytical Solutions** (Phase 4, Implemented):

- `analytical/asian.rs`: Geometric Asian options (Kemna-Vorst closed-form)
- `analytical/barrier.rs`: Barrier options (Merton/Rubinstein-Reiner formulas)
- Purpose: Verification benchmarks for Monte Carlo pricing accuracy

**Checkpointing** (Phase 4, Implemented):

- `checkpoint/`: Memory management for AD with long simulation paths
- `CheckpointStrategy`: Binomial checkpointing (Griewank/Walther algorithm)
- `MemoryBudget`: Configurable memory limits for checkpointing
- Integration: `MonteCarloPricer` with checkpointing support

**Greeks Module** (Phase 4+, Implemented):

- `GreeksConfig`: Configuration for bump widths and calculation modes (builder pattern)
- `GreeksMode`: Calculation mode selection (BumpAndRevalue, AAD, NumDual)
- `GreeksResult<T>`: Generic result type for Greeks calculations (AD-compatible)

### Layer 4: Application (pricer_risk)

**Location**: `crates/pricer_risk/src/`
**Purpose**: Portfolio analytics and XVA calculations (stable Rust)
**Structure**:

```text
portfolio/  → Trade, Counterparty, NettingSet, PortfolioBuilder
exposure/   → EE, EPE, PFE, EEPE, ENE calculators
xva/        → CVA, DVA, FVA calculators with XvaCalculator
soa/        → Structure of Arrays (TradeSoA, ExposureSoA)
parallel/   → Rayon-based parallelization config
```

**Key Principle**: Consumer of L1+L2+L3, orchestrates portfolio-level computations with parallel processing.

### Infrastructure

**Docker**: `docker/`

- `Dockerfile.stable` - L1/L2/L4 builds (no Enzyme)
- `Dockerfile.nightly` - L3 with Enzyme LLVM plugin

**Scripts**: `scripts/`

- `install_enzyme.sh` - Enzyme installation helper
- `verify_enzyme.sh` - Enzyme verification
- `check_iai_regression.sh` - Instruction-count regression checking

**CI/CD**: `.github/workflows/`

- `ci.yml` - Separate jobs for stable (L1/L2/L4) and nightly (L3)
- `release.yml` - Release automation and changelog generation (Phase 6)

## Naming Conventions

- **Crates**: `pricer_*` prefix, snake_case (`pricer_core`, `pricer_pricing`)
- **Modules**: snake_case (`monte_carlo`, `smoothing`)
- **Traits**: PascalCase (`Priceable`, `Differentiable`)
- **Types**: PascalCase (`DualNumber`, `VanillaOption`)
- **Functions**: snake_case (`smooth_max`, `price_european`)

## Import Organization

**Absolute imports** for cross-crate dependencies:

```rust
use pricer_core::traits::Priceable;
use pricer_models::instruments::Instrument;
use pricer_pricing::mc::MonteCarloPricer;
```

**Relative imports** within same crate:

```rust
use crate::math::smoothing::smooth_max;
use super::types::DualNumber;
```

**No path aliases** - workspace imports are explicit.

## Code Organization Principles

1. **Bottom-Up Dependencies**: No circular dependencies, L4 never imported by L1/L2/L3
2. **Feature Flag Isolation**: L1 supports both `num-dual-mode` (default) and `enzyme-mode`
3. **Static Dispatch**: Prefer `enum` over `Box<dyn Trait>` for Enzyme optimization
4. **Smooth by Default**: All discontinuous functions have smooth approximations
5. **Test Co-Location**: Unit tests in same file as implementation (`#[cfg(test)]`)

## Phase-Based Development

Current roadmap (see README.md):

- Phase 0: Workspace scaffolding (complete)
- Phase 1: L1 foundation - types, traits, smoothing, market data (complete)
- Phase 2: L2 business logic - instruments, models (complete)
- Phase 3: L3 Enzyme integration - AD infrastructure, MC kernel (largely complete; Phase 3.2 bump-and-revalue Greeks)
- Phase 4: Advanced MC - checkpointing, path-dependent options, analytical solutions (complete)
- Phase 5: L4 XVA - CVA/DVA/FVA, exposure metrics, parallelization (complete)
- Phase 6: Production hardening - docs, benchmarks, CI/CD (in progress)

---
_Created: 2025-12-29_
_Updated: 2026-01-09_ — Added rates models (Hull-White, CIR) and hybrid/correlated model structure
_Document patterns, not file trees. New files following patterns should not require updates_
