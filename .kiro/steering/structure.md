# Project Structure

## Organisation Philosophy

**A-I-P-S Unidirectional Data Flow**:
The workspace structure enforces a strict unidirectional data flow that mirrors the alphabetical order of the directory names (**A**dapter → **I**nfra → **P**ricer → **S**ervice). This logical progression ensures that the file system itself acts as an architectural map, guiding developers from data ingestion to computation and finally to delivery.

```text
A: Adapter   → Ingestion and normalisation of external data (The Raw Inputs)
I: Infra     → System-wide definitions, persistence, and configuration (The Foundation)
P: Pricer    → Mathematical modelling, optimisation, and risk computation (The Kernel)
S: Service   → Execution environments and interfaces (The Outputs)
```

### Dependency Rules

1. **S**ervices may depend on any **P**, **I**, or **A** crate.
2. **P**ricer crates must never depend on **S** or **A** crates.
3. **I**nfra crates must never depend on **P** or **S** crates.
4. **A**dapter crates depend only on **I** (for definitions) or **P** (for target types), never on **S**.

---

## Directory Patterns

### Cargo Workspace Root

**Location**: `/`
**Purpose**: Workspace configuration and top-level metadata
**Key Files**:
- `Cargo.toml` - Workspace members, shared dependencies, release profile
- `rust-toolchain.toml` - Default stable toolchain (nightly pinned for pricer_pricing)
- `README.md` - User-facing documentation

---

## A: Adapter Layer (Input)

**Responsibility**: To sanitise external chaos into internal order. This layer depends only on `pricer_core` (for types) and `infra_master` (for identifiers).

### adapter_feeds

**Location**: `crates/adapter_feeds/src/`
**Purpose**: Real-time/Snapshot market data parsers
**Function**: Handles connectivity to market data providers (Reuters, Bloomberg, internal lakes).
**Scope**: Normalises raw quotes (Bid/Ask, Last) into standardised `MarketQuote` structs.

### adapter_fpml

**Location**: `crates/adapter_fpml/src/`
**Purpose**: Trade definition parsers (FpML/XML)
**Function**: Parses complex XML/FpML trade structures.
**Scope**: Maps FpML elements to `pricer_models::Instrument` enums.

### adapter_loader

**Location**: `crates/adapter_loader/src/`
**Purpose**: Flat file loaders (CSV/Parquet) & CSA details
**Function**: Bulk loading of CSV, JSON, or Parquet files.
**Scope**: Manages CSA (Credit Support Annex) terms, counterparty details, and netting set configurations.

---

## I: Infra Layer (Foundation)

**Responsibility**: To provide the static foundation and persistence mechanisms required before calculation begins.

### infra_config

**Location**: `crates/infra_config/src/`
**Purpose**: System configuration & environment management
**Function**: Loads runtime settings (TOML/YAML/Env Vars).
**Scope**: Defines memory limits for the AD engine, thread pool sizes, and database connection strings.

### infra_master

**Location**: `crates/infra_master/src/`
**Purpose**: Static master data (Calendars, Currencies, ISINs)
**Function**: The "Source of Truth" for static finance data.
**Scope**: Holiday calendars (TARGET, NY, JP), Currency definitions (ISO 4217), and Day Count Convention lookups.

### infra_store

**Location**: `crates/infra_store/src/`
**Purpose**: Persistence & State (SQLx, Redis, TimeScale)
**Function**: Database Access Layer (DAL).
**Scope**: Implements `Save` and `Load` traits for Trades and Risk Reports using `sqlx` (Postgres) or other backends. Isolates I/O dependencies from the kernel.

---

## P: Pricer Layer (The Kernel)

**Responsibility**: Pure quantitative computation. Experimental AD technology (Enzyme) confined to pricer_pricing, keeping 75% of codebase production-stable.

```text
L1: pricer_core      → Foundation (Stable)
L2: pricer_models    → Business Logic (Stable)
L2.5: pricer_optimiser → Calibration & Solvers (Stable)
L3: pricer_pricing   → AD Engine (Nightly + Enzyme)
L4: pricer_risk      → Application (Stable)
```

### pricer_core (L1)

**Location**: `crates/pricer_core/src/`
**Purpose**: Math types, traits, smoothing functions, market data abstractions (stable Rust)
**Structure**:
```text
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

### pricer_models (L2)

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
  ├── hybrid/   → Correlated multi-factor models (feature-gated)
  ├── heston.rs → Heston stochastic volatility model
  └── sabr.rs   → SABR stochastic volatility model
calibration/  → Model calibration infrastructure
  ├── heston.rs      → Heston calibration (characteristic function pricing)
  ├── sabr.rs        → SABR calibration (Hagan formula)
  ├── hull_white.rs  → Hull-White swaption calibration
  └── swaption_calibrator.rs → Generic swaption calibrator
analytical/   → Closed-form solutions (Black-Scholes, Garman-Kohlhagen)
schedules/    → Payment schedule generation (Frequency, Period, ScheduleBuilder)
demo.rs       → Demo types for 3-stage rocket: ModelEnum, InstrumentEnum, CurveEnum, VolSurfaceEnum
```

**Key Principles**:

- **Hierarchical Instrument Architecture**: `InstrumentEnum<T>` wraps asset-class sub-enums (EquityInstrument, RatesInstrument, CreditInstrument, FxInstrument)
- **Feature-Flag Asset Classes**: Each asset class gated by feature (equity, rates, credit, fx)
- **InstrumentTrait**: Unified interface for all instruments (payoff, expiry, currency, notional)
- **CashflowInstrument**: Extended trait for instruments with scheduled payments (swaps, bonds)
- **Static Dispatch**: Enum-based dispatch at both top and sub-enum levels for Enzyme compatibility
- **Schedule Generation**: `ScheduleBuilder` pattern for IRS/CDS payment schedules
- **StochasticModel Trait**: Unified interface for stochastic processes (`evolve_step`, `initial_state`, `brownian_dim`)
- **StochasticModelEnum**: Static dispatch enum wrapping concrete models (GBM, Heston, SABR, Hull-White, CIR)

### pricer_optimiser (L2.5) [NEW]

**Location**: `crates/pricer_optimiser/src/`
**Purpose**: Calibration, Bootstrapping & Solvers
**Position**: Logically sits between Models (Definition) and Pricing (Calculation).
**Function**: Solves inverse problems to construct valid model objects.
**Structure**:

```text
bootstrapping/  → Yield Curve stripping from OIS/Swap rates (multi-curve)
calibration/    → Stochastic model calibration (Hull-White α/σ from swaptions)
solvers/        → Levenberg-Marquardt, BFGS algorithms
provider.rs     → MarketProvider for lazy market data resolution (Arc-cached curves/vols)
```

**Integration**: Calls `pricer_pricing` to obtain gradients (via Enzyme) for efficient parameter search.

### pricer_pricing (L3)

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
context.rs       → [l1l2-integration] 3-stage rocket: PricingContext, price_single_trade
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

### pricer_risk (L4)

**Location**: `crates/pricer_risk/src/`
**Purpose**: Portfolio analytics, XVA, and risk metrics (stable Rust)
**Structure**:

```text
portfolio/  → Trade, Counterparty, NettingSet, PortfolioBuilder
exposure/   → EE, EPE, PFE, EEPE, ENE calculators
xva/        → CVA, DVA, FVA calculators with XvaCalculator
scenarios/  → Scenario analysis: ScenarioEngine, RiskFactorShift, PresetScenario, GreeksAggregator
regulatory/ → SA-CCR, FRTB, SIMM (planned)
soa/        → Structure of Arrays (TradeSoA, ExposureSoA)
parallel/   → Rayon-based parallelisation config
demo.rs     → Portfolio orchestration demo (DemoTrade, Pull-then-Push pattern)
```

**Key Principle**: Consumer of L1+L2+L3, orchestrates portfolio-level computations with parallel processing.

---

## S: Service Layer (Output)

**Responsibility**: Delivery of results to end-users or systems.

### service_cli

**Location**: `crates/service_cli/src/`
**Purpose**: Command Line Operations (Batch/Ops)
**Function**: Operational entry point.
**Commands**: `neutryx calibrate`, `neutryx price --portfolio trade_file.csv`.
**Structure**:

```text
commands/   → Subcommand implementations (calibrate, price, report)
config/     → CLI configuration loading
main.rs     → Entry point with clap argument parsing
```

### service_gateway

**Location**: `crates/service_gateway/src/`
**Purpose**: gRPC/REST API Gateway (Microservices)
**Function**: Production integration point.
**Scope**: gRPC (Tonic) and REST (Axum) endpoints for microservice deployment.
**Structure**:

```text
grpc/       → Tonic service implementations
rest/       → Axum route handlers
proto/      → Protocol buffer definitions
main.rs     → Server entry point
```

### service_python

**Location**: `crates/service_python/src/`
**Purpose**: PyO3 Bindings (Research/Jupyter)
**Function**: Research interface (critical for PhD/JAX comparison).
**Scope**: Exposes Rust structs as Python classes via PyO3. Allows direct manipulation of `pricer_optimiser` for notebook-based calibration experiments.
**Structure**:

```text
bindings/   → PyO3 class wrappers (PyInstrument, PyModel, PyOptimiser)
lib.rs      → Module registration and Python module definition
```

---

## D: Demo Layer (Reference Implementations)

**Responsibility**: Reference implementations and showcases demonstrating A-I-P-S integration. External to the core architecture.

**Location**: `demo/`

### FrictionalBank Demo

**Location**: `demo/frictional_bank/`
**Purpose**: Full A-I-P-S workflow orchestration (TUI + Web interfaces)
**Structure**:

```text
workflow/     → Business workflows (eod_batch, intraday, stress_test)
config.rs     → Demo configuration
error.rs      → Demo-specific errors
main.rs       → TUI entry point
```

**Supported Modes**: EOD Batch, Intraday Monitoring, Stress Testing

### Demo Inputs

**Location**: `demo/inputs/`
**Purpose**: Simulated market data providers and trade sources
**Pattern**: Mirrors Adapter layer interfaces for demonstration

### Demo Outputs

**Location**: `demo/outputs/`
**Purpose**: Report sinks, risk dashboards, regulatory outputs
**Pattern**: Mirrors Service layer outputs for demonstration

### Demo GUI

**Location**: `demo/gui/`
**Purpose**: TUI (ratatui) and Web dashboard (axum + WebSocket)
**Structure**:

```text
app.rs        → TUI application state
screens.rs    → TUI screen definitions
web/          → Web server (handlers, websocket)
static/       → Web assets (HTML, CSS, JS)
```

### Demo Data & Notebooks

- `demo/data/` - Sample trades, market data, configuration files
- `demo/notebooks/` - Jupyter notebooks for pricing, calibration, XVA demos

---

## Infrastructure

**Docker**: `docker/`

- `Dockerfile.stable` - A/I/P/S builds (no Enzyme)
- `Dockerfile.nightly` - pricer_pricing with Enzyme LLVM plugin

**Scripts**: `scripts/`

- `install_enzyme.sh` - Enzyme installation helper
- `verify_enzyme.sh` - Enzyme verification
- `check_iai_regression.sh` - Instruction-count regression checking

**CI/CD**: `.github/workflows/`

- `ci.yml` - Separate jobs for stable and nightly builds
- `release.yml` - Release automation and changelog generation

## Naming Conventions (British English)

- **Spelling**: Strictly adhere to British English (e.g., `optimiser`, `serialisation`, `visualisation`, `modelling`)
- **Crates**: Layer prefix, snake_case (`adapter_feeds`, `infra_config`, `pricer_core`, `service_cli`)
- **Modules**: snake_case (`monte_carlo`, `smoothing`)
- **Traits**: PascalCase (`Priceable`, `Differentiable`)
- **Types**: PascalCase (`DualNumber`, `VanillaOption`, `CalibrationEngine`)
- **Functions**: snake_case (`smooth_max`, `price_european`)

## Import Organisation

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

## Code Organisation Principles

1. **A-I-P-S Data Flow**: Unidirectional dependencies from Adapter → Infra → Pricer → Service
2. **Feature Flag Isolation**: pricer_core supports both `num-dual-mode` (default) and `enzyme-mode`
3. **Static Dispatch**: Prefer `enum` over `Box<dyn Trait>` for Enzyme optimisation
4. **Smooth by Default**: All discontinuous functions have smooth approximations
5. **Test Co-Location**: Unit tests in same file as implementation (`#[cfg(test)]`)

---
_Created: 2025-12-29_
_Updated: 2026-01-11_ — Added Demo Layer (D) documentation for FrictionalBank reference implementation
_Document patterns, not file trees. New files following patterns should not require updates_
