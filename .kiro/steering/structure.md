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
**Purpose**: Static master data and financial primitives
**Function**: The "Source of Truth" for static finance data.
**Scope**: Holiday calendars, Day count conventions, Counterparty/CSA data, Financial date and time primitives, Trade structures, Market conventions, Portfolio/Book organisation.
**Structure**:

```text
time/              → Time-related primitives
  ├── calendars.rs      → Holiday calendars (Calendar, CalendarId: Target, NewYork, Tokyo)
  ├── day_counters.rs   → Day count conventions (DayCountConvention: Act360, Act365, Thirty360)
  ├── frequency.rs      → Payment frequencies (Frequency: Annual, SemiAnnual, Quarterly)
  ├── period.rs         → Period definitions (Period)
  └── types.rs          → Date type and business day conventions

market/            → Market data references and rate infrastructure
  ├── currency.rs       → ISO 4217 currency codes (Currency enum with metadata)
  ├── rate_index.rs     → Rate index definitions (RateIndex)
  ├── rate.rs           → Rate values (Rate, RateQuote)
  ├── rate_id.rs        → Rate identifiers (RateId)
  ├── rate_type.rs      → Rate type classification (RateType)
  ├── rate_set.rs       → Rate set collections (RateSet)
  ├── ticker.rs         → Market tickers (Ticker)
  ├── quote_type.rs     → Quote types (Bid, Ask, Mid)
  ├── data_source.rs    → Data source definitions
  ├── mapper.rs         → Rate/Ticker mapping
  └── validation.rs     → Market data validation

counterparty/      → Counterparty, netting, and XVA configuration
  ├── csa.rs            → CSA terms (CsaTerms)
  ├── netting_set.rs    → Netting set configuration
  ├── netting_agreement.rs → Netting agreement structures
  ├── credit.rs         → Credit data (hazard rates)
  ├── margin.rs         → Margin requirements
  ├── ccp.rs            → CCP (Central Counterparty) definitions
  ├── counterparty_entity.rs → Counterparty entity types
  ├── counterparty_portfolio.rs → Counterparty portfolio mapping
  ├── xva_config.rs     → XVA configuration parameters
  ├── aggregation.rs    → Exposure aggregation rules
  └── ids.rs            → Counterparty ID types (IsdaAgreementId, VmAgreementId)

book/              → Trading book organisation
  ├── book.rs           → Book entity (Book, BookId)
  └── types.rs          → Book types and hierarchy

portfolio/         → Portfolio organisation
  ├── portfolio.rs      → Portfolio entity (Portfolio, PortfolioId)
  └── types.rs          → Portfolio types

trade/             → Trade representation (CF-expanded format)
  ├── error.rs          → Trade construction errors (TradeError)
  ├── index.rs          → Market indices (IndexType, IndexObservation)
  ├── payoff.rs         → Payoff definitions (Fixed, Linear, VanillaOption, Digital)
  ├── cashflow.rs       → Cashflow representation (Cashflow, CashflowType)
  ├── leg.rs            → Trade legs (Leg, Direction, LegType)
  ├── trade.rs          → Trade structure (Trade, TradeId, TradeMetadata, TradeType)
  ├── instrument.rs     → Market instruments (Deposit, FRA, Futures, ParSwap)
  ├── pricing_instrument.rs → Pricing instrument types (VanillaOption, Forward)
  ├── book_assignment.rs → Trade-to-Book assignment
  ├── builder.rs        → Builder API (TradeBuilder, LegBuilder)
  └── convention/       → Market conventions (swap, swaption, fx, equity, credit, commodity, etc.)

  instrument_def/   → Standard instrument definitions (multi-asset catalogue)
    ├── rates.rs        → Rates instruments (Swaption, CapFloor, Frn, CmsSwap, InflationSwap)
    ├── fx.rs           → FX instruments (FxSpot, FxForward, FxVanillaOption, FxBarrierOption, FxSwap)
    ├── equity.rs       → Equity instruments (EquityForward, EquityVanillaOption, AsianOption, etc.)
    ├── credit.rs       → Credit instruments (Cds, CdsIndex, CdsOption, NtdBasket)
    ├── commodity.rs    → Commodity instruments (CommodityForward, CommoditySwap, etc.)
    ├── common.rs       → Shared types (AssetClass, ExerciseStyle, PayerReceiver)
    ├── error.rs        → InstrumentError for validation failures
    └── expander.rs     → InstrumentExpander for trade expansion
```

**Trade Architecture**: `Trade` → `Vec<Leg>` → `Vec<Cashflow>` (CF-expanded common format)
**Prelude**: `infra_master::prelude` exports all commonly used types.

### infra_store

**Location**: `crates/infra_store/src/`
**Purpose**: Persistence & State (SQLx, Redis, TimeScale)
**Function**: Database Access Layer (DAL).
**Scope**: Implements `Save` and `Load` traits for Trades and Risk Reports using `sqlx` (Postgres) or other backends. Isolates I/O dependencies from the kernel.

---

## P: Pricer Layer (The Kernel)

**Responsibility**: Pure quantitative computation. Experimental AD technology (Enzyme) confined to pricer_pricing, keeping 75% of codebase production-stable.

```text
L1: pricer_core      → Foundation (Stable) - math (smoothing, interpolators, solvers), types, traits
L2: pricer_models    → Business Logic (Stable) - instruments, market (curves, surfaces, calibration), models, schedules
L3: pricer_pricing   → MC Engine (Stable) - mc, rng, greeks, context, path_dependent, checkpoint
L4: pricer_risk      → Application + Enzyme AD (Stable, Nightly with enzyme-ad) - portfolio, exposure, xva, scenarios, enzyme
```

> **Note**: L2.5 (`pricer_optimiser`) was removed in 2026-01. Market data functionality (curves, surfaces, bootstrapping, provider) consolidated into `pricer_models::market`, calibration engine into `pricer_models::market::calibration`.

### pricer_core (L1)

**Location**: `crates/pricer_core/src/`
**Purpose**: Math types, traits, smoothing functions, IR data structures (stable Rust, pure foundation)
**Structure**:
```text
math/
├── smoothing.rs      → Smooth approximations (smooth_max, smooth_indicator)
├── numeric.rs        → Numeric conversion utilities (from_f64, from_usize)
├── distributions/    → Probability distributions (normal, bivariate_normal, chi_squared, copula)
├── calculus/         → Numerical differentiation (finite_difference, bump_selection)
├── utilities/        → Basic math functions (sign, clamp, lerp), combinatorics, special functions
├── interpolators/    → Interpolation methods (linear, bilinear, cubic_spline, monotonic, smooth_interp, flat, log_linear, hermite, svi, search)
├── solvers/          → Root-finding algorithms (Newton-Raphson, Brent, bisection, backtracking_newton)
├── integrators/      → Numerical integration (Gauss-Legendre, Gauss-Kronrod, adaptive, Runge-Kutta)
├── optimisers/       → Optimisation algorithms (Nelder-Mead, L-BFGS via argmin)
├── fitting/          → Curve fitting (least_squares, gaussian)
├── mesh/             → Grid generation (grid_1d, grid_2d)
└── linalg/           → Linear algebra (feature-gated, nalgebra wrappers)
    ├── strategy.rs   → LinearSolveStrategy trait (LU, LowerTriangular) for pluggable solve strategies
    ├── wrappers.rs   → nalgebra wrappers (cholesky, lu, qr, svd)
    └── error.rs      → LinearAlgebraError types

ir/         → Pricing Kernel Intermediate Representation (SIMD/Enzyme-optimised)
├── aligned_buffer.rs → 64-byte aligned heap buffer (AlignedBuffer<T>)
├── pricing_kernel.rs → SoA cashflow representation (PricingKernel, PricingKernelBuilder)
├── script_kernel.rs  → Event-driven IR for path-dependent (ScriptKernel, ScriptOp, BarrierType)
├── callable_kernel.rs → Block-structured IR for Bermudan (CallableKernel, CallableBlock, ExerciseDef)
└── error.rs          → Compilation errors (CompileError)

traits/     → Priceable, Differentiable, Float, core abstractions
types/
├── time.rs          → DayCountConvention, time_to_maturity for financial calculations
├── currency_pair.rs → FxRate type for FX rate representation (deprecated alias: CurrencyPair)
└── error.rs         → Structured error types (PricingError, SolverError, InterpolationError, CalibrationError)
```

**Key Principles**:

- Zero dependencies on other pricer_* crates, pure foundation
- Minimal scope: math utilities, core traits, basic types, IR data structures
- All numeric types generic over `T: Float` for AD compatibility
- IR module provides SIMD-aligned (64-byte) data structures for pricing kernels

### pricer_models (L2)

**Location**: `crates/pricer_models/src/`
**Purpose**: Financial instruments, market data, stochastic models, calibration (stable Rust)
**Structure**:

```text
analytic.rs       → Analytical pricing with instrument integration (re-exports pricer_core formulas)

builder/          → Yield curve bootstrapping and market data calibration
  ├── Shared Infrastructure
  │   ├── grid.rs         → CalibrationGrid for axis management
  │   ├── matrix.rs       → CalibrationMatrix, InterpolationMatrix
  │   ├── problem.rs      → CalibrationProblem (SystemOfEquations impl), JacobianMethod
  │   ├── error.rs        → CalibrationError, BootstrapError types
  │   └── instrument.rs   → CalibrationInstrument trait
  │
  ├── curve/              → Yield curve calibration
  │   ├── bootstrap.rs    → Sequential bootstrapping (CurveBootstrapper, BootstrapConfig)
  │   └── global.rs       → Global calibration (feature = "global-bootstrap")
  │                         GlobalBootstrapper, GlobalBootstrapConfig, GlobalBootstrapResult
  │
  └── vol/                → Volatility surface/cube calibration
      ├── surface.rs      → FX vol surfaces (2D), SabrSliceCalibrator, FxVolBuilder
      └── cube.rs         → Swaption vol cubes (3D), VolCubeBuilder

compiler/         → Trade compiler for IR generation
  ├── mod.rs          → Module exports
  ├── index_mapper.rs → IndexMapper (RateIndex/Currency → numeric ID)
  ├── linear.rs       → LinearProductsCompiler (IRS, Bond, FRA, CMS)
  ├── xccy.rs         → XCcyCompiler (cross-currency swaps)
  ├── exotic.rs       → ExoticCompiler (barriers, Asians)
  └── callable.rs     → CallableCompiler (Bermudan swaptions, block-partitioned)

market.rs         → Market data structures (single-file module with inline submodules)
  └── curves::        → YieldCurve trait, FlatCurve, BootstrappedCurve, MarketInstrument
      CurveEnum       → Static dispatch wrapper (Flat, Bootstrapped)
      CurveName       → Named curve identifiers (Sofr, Euribor, Estr, Tonar, Sonia)
      CurveSet        → Named curve collection (CurveName → CurveEnum)
      MarketProvider  → Market data provider placeholder

stochastic/       → Stochastic process models
  ├── mod.rs          → Module exports, StochasticModel trait
  ├── model_enum.rs   → StochasticModelEnum for static dispatch
  ├── gbm.rs          → Geometric Brownian Motion
  ├── heston.rs       → Heston stochastic volatility
  ├── hull_white.rs   → Hull-White interest rate model
  ├── cir.rs          → Cox-Ingersoll-Ross model
  ├── correlated.rs   → Correlated multi-factor models
  ├── validation.rs   → Model parameter validation
  └── error.rs        → StochasticModelError

instruments/      → Re-exports from infra_master::trade for backwards compatibility
```

**Calibration Patterns** (documented in `builder/mod.rs`):

| Pattern | Module | Description |
|---------|--------|-------------|
| Sequential | `curve::bootstrap` | Solve one pillar at a time (yield curves) |
| Slice-wise | `vol::surface`, `vol::cube` | Calibrate each slice independently (vol surfaces/cubes) |
| Global | `curve::global` | Solve all parameters simultaneously via Newton-Raphson (feature-gated) |

**Shared Infrastructure** (`builder/`):
- `CalibrationGrid`: Axis management for time/strike dimensions
- `CalibrationMatrix`: N×M matrix representation for instrument cashflows
- `InterpolationMatrix`: Maps pillar discount factors to cashflow dates via log-linear interpolation
- `CalibrationProblem`: Implements `SystemOfEquations<T>` for MultidimensionalNewtonSolver
- `JacobianMethod`: Finite Difference, Central Difference, or Automatic Differentiation (AAD feature-gated)

**Global Curve Calibration** (`curve::global`, feature = "global-bootstrap"):
- AAD Preparation: Stores J⁻¹ in `GlobalBootstrapResult` for implicit function theorem (∂x*/∂m = J⁻¹)

**Key Principles**:

- **TradeCompiler Pattern**: `TradeCompiler<T>` trait compiles Trade → PricingKernel IR; `IndexMapper` maps indices to numeric IDs
- **StochasticModel Trait**: Unified interface for stochastic processes (`evolve_step`, `initial_state`, `brownian_dim`)
- **StochasticModelEnum**: Static dispatch enum wrapping concrete models (GBM, Heston, SABR, Hull-White, CIR)
- **CalibrationEngine**: Uses `pricer_core::math::solvers` for parameter optimisation
- **Static Dispatch**: Enum-based dispatch for Enzyme compatibility

### pricer_pricing (L3)

**Location**: `crates/pricer_pricing/src/`
**Purpose**: Monte Carlo Engine (stable Rust)
**Structure**:

```text
mc/              → Monte Carlo kernel (GBM paths, workspace buffers, Greeks, MonteCarloPricer, thread_local)
path_dependent/  → Path-dependent options (Asian, Barrier, Lookback) with streaming statistics
rng/             → Random number generation (PRNG, QMC sequences)
verify/          → Verification tests
checkpoint/      → Memory management for checkpointing
analytical/      → Closed-form solutions (geometric Asian, barrier options)
greeks/          → Greeks calculation types (GreeksConfig, GreeksMode, GreeksResult<T>)
pool/            → Thread-local buffer pool (ThreadLocalPool, PooledBuffer, PoolStats)
tree/            → Tree-based pricing methods (Binomial/Trinomial)
kernel/          → IR-based pricing engines (static dispatch, SIMD-friendly)
  ├── engine.rs         → LinearEngine (branchless PV calculation for PricingKernel)
  ├── script_engine.rs  → ScriptEngine (event-driven execution for ScriptKernel)
  ├── callable_engine.rs → CallableEngine (forward/backward pass for Bermudan)
  ├── lsmc.rs           → Longstaff-Schwartz Monte Carlo regression (LSMCRegressor)
  ├── provider.rs       → CurveProvider trait implementations
  ├── context.rs        → KernelContext for static dispatch market data access
  └── integration.rs    → Full pipeline integration (Trade→IR→PV)
  ├── binomial.rs   → CRR binomial tree (BinomialTree, CrrParams)
  ├── trinomial.rs  → Kamrad-Ritchken trinomial tree (TrinomialTree, KrParams)
  ├── config.rs     → TreeConfig, TreeConfigBuilder, TreeType
  └── method.rs     → TreeMethod high-level interface
result/          → Unified pricing result types
  └── mod.rs        → UnifiedPricingResult, UnifiedGreeks, PricingMetadata
dispatcher/      → Pricing method dispatcher
  └── mod.rs        → PricingMethodDispatcher, DispatcherConfig
generic_pricer/  → Generic pricer API and configuration
context.rs       → [l1l2-integration] 3-stage rocket: PricingContext, price_single_trade
irs_greeks/      → IRS Greeks workflow (AAD vs Bump-and-Revalue, lazy evaluation, benchmarks)
graph/           → Computation graph extraction (D3.js-compatible JSON for DAG visualisation)
```

**Key Principle**: Monte Carlo simulation engine with stable Rust. Enzyme AD has been moved to pricer_risk for L4 integration.

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
- `GreeksMode`: Calculation mode selection (BumpAndRevalue, EnzymeAAD)
- `GreeksResult<T>`: Generic result type for Greeks calculations (AD-compatible)

**IRS Greeks Workflow** (Phase 5+, Implemented):

- `irs_greeks/calculator.rs`: IRS Greeks calculator with AAD support
- `irs_greeks/lazy_evaluator.rs`: Lazy evaluation with caching for repeated calculations
- `irs_greeks/benchmark.rs`: Performance comparison harness (AAD vs Bump-and-Revalue)
- `irs_greeks/xva_demo.rs`: XVA demo integration with IRS pricing
- Feature-gated (`l1l2-integration`) for full L1/L2 access

**Computation Graph Module** (Phase 5+, Implemented):

- `graph/types.rs`: `GraphNode`, `GraphEdge`, `ComputationGraph` types for D3.js visualisation
- `graph/extractor.rs`: `GraphExtractable` trait with `SimpleGraphExtractor` implementation
- Serialises to JSON format for browser-based DAG visualisation
- Integrated into demo web dashboard (`/api/graph` endpoint)

### pricer_risk (L4)

**Location**: `crates/pricer_risk/src/`
**Purpose**: Portfolio analytics, XVA, risk metrics, and Enzyme AD (stable Rust, nightly with enzyme-ad feature)
**Structure**:

```text
portfolio/  → Trade, Counterparty, NettingSet, PortfolioBuilder
exposure/   → EE, EPE, PFE, EEPE, ENE calculators
xva/        → CVA, DVA, FVA calculators with XvaCalculator
scenarios/  → Scenario analysis and risk factor management
            → engine.rs (ScenarioEngine, ScenarioPnL), shifts.rs (RiskFactorShift, BumpScenario)
            → presets.rs (PresetScenario), aggregator.rs (GreeksAggregator, PortfolioGreeks)
            → bucket_dv01.rs (BucketDv01Calculator, KeyRateDuration), curve_shifts.rs (CurveShifter)
            → risk_factor.rs (RiskFactorId), greeks_by_factor.rs (GreeksResultByFactor)
            → irs_greeks_by_factor.rs (IrsGreeksByFactorCalculator)
enzyme/     → Enzyme autodiff bindings (ADMode, Activity, gradient, GreeksEnzyme trait)
            → greeks.rs (GreeksEnzyme trait, EnzymeGreeksResult, GreeksMode)
            → verification.rs (VerificationConfig, analytical module)
            → checkpoint_ad.rs (CheckpointedAD, PathDependentAD)
            → smooth.rs (smooth_max, smooth_indicator, smooth_call_payoff)
            → forward.rs, reverse.rs, loops.rs, parallel.rs (AD infrastructure)
            → shadow.rs (ShadowObject trait, shadow buffers for reverse mode)
            → kernel.rs (PricingKernel integration for AAD)
            → binder.rs (AAD binder layer for market risk calculations)
regulatory/ → SA-CCR, FRTB, SIMM (planned)
soa/        → Structure of Arrays (TradeSoA, ExposureSoA)
parallel/   → Rayon-based parallelisation utilities (>80% efficiency on 8+ cores)
            → portfolio_greeks.rs (ParallelPortfolioGreeksCalculator for 1000+ trades)
            → memory_monitor.rs (MemoryMonitor, SharedMemoryMonitor, auto-checkpoint)
            → Batch helpers (process_in_batches, parallel_map, parallel_reduce, parallel_sum)
demo.rs     → Portfolio orchestration demo (DemoTrade, Pull-then-Push pattern)
```

**Key Principle**: Consumer of L1+L2+L3, orchestrates portfolio-level computations with parallel processing. Enzyme AD feature (`enzyme-ad`) requires nightly Rust (nightly-2025-01-15) and LLVM 18.

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
**Scope**: REST (Axum) and gRPC (Tonic) endpoints for microservice deployment.
**Structure**:

```text
rest/
├── handlers.rs       → Core API handlers (price, batch, calibrate, exposure)
├── graph_handlers.rs → Portfolio graph REST handlers (subgraph extraction, caching)
├── ws_handlers.rs    → WebSocket handlers (real-time graph updates)
└── mod.rs            → Router configuration (with/without WebSocket state)
grpc/       → Tonic service implementations (skeleton)
config.rs   → Server configuration
error.rs    → Structured error types (ServerError)
main.rs     → Server entry point
```

**REST API Endpoints**:
- `/health` - Health check
- `/api/v1/price` - Single instrument pricing
- `/api/v1/price/batch` - Portfolio batch pricing
- `/api/v1/calibrate` - Model calibration
- `/api/v1/exposure` - Exposure calculation
- `/api/v1/portfolio/graph` - Portfolio computation graph (D3.js-compatible)
- `/api/v1/portfolio/trades` - Portfolio trade listing with filters

**WebSocket Endpoint**:
- `/ws` - Real-time graph updates (select_trades, subgraph_update events)

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
**Purpose**: Full A-I-P-S workflow orchestration (TUI + REST API)
**Structure**:

```text
workflow/     → Business workflows (eod_batch, intraday, stress_test, irs_aad)
config.rs     → Demo configuration
error.rs      → Demo-specific errors
main.rs       → REST API entry point (HTTP server for workflow orchestration)
```

**Supported Modes**: EOD Batch, Intraday Monitoring, Stress Testing, IRS AAD Demo

**REST API Endpoints**:
- `/health` - Health check (Kubernetes/Cloud Run readiness probe)
- `/api/v1/workflow/eod` - EOD batch processing
- `/api/v1/workflow/intraday` - Intraday monitoring
- `/api/v1/workflow/stress` - Stress testing

**Deployment**: Cloud Run-compatible with environment-based configuration (`PORT`, `FB_DEBUG_MODE`, `FB_LOG_LEVEL`)

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
**Purpose**: Dual-interface dashboard (TUI + Web)
**Structure**:

```text
app.rs           → TUI application state
screens.rs       → TUI screen definitions (dashboard, trades, exposure, IRS AAD)
visualisation.rs → Benchmark visualisation (speed comparison charts, AAD vs Bump-and-Revalue)
api_client.rs    → HTTP client for service communication
web/             → Web server module (feature-gated)
  ├── main.rs         → Web dashboard entry point
  ├── mod.rs          → Router configuration, middleware, AppState
  │
  ├── handlers/       → REST API handlers (feature-organised)
  │   ├── mod.rs          → Handler module exports
  │   ├── types.rs        → Consolidated API type definitions
  │   ├── config.rs       → Handler configuration and utilities
  │   ├── health.rs       → Health check endpoints
  │   ├── curves.rs       → Curve Builder API (/api/curves/*)
  │   ├── volcube.rs      → IR VolCube API (/api/volcube/*)
  │   ├── irvol.rs        → IR Vol Surface API (/api/irvol/*)
  │   ├── fxvol.rs        → FX Vol Surface API (/api/fxvol/*)
  │   ├── fxcurve.rs      → FX Curve API (/api/fxcurve/*)
  │   ├── trades.rs       → Trade expansion API (/api/trades/*)
  │   ├── market.rs       → Market data API (/api/market/*)
  │   ├── generic_pricer.rs → Generic Pricer API (/api/pricer/*)
  │   ├── risk_engine.rs  → Risk Engine API (/api/risk/*)
  │   ├── scenario_analysis.rs → Scenario analysis endpoints
  │   ├── pricing.rs      → Pricing endpoints (feature = "calibration")
  │   ├── greeks.rs       → Greeks calculation endpoints (feature = "calibration")
  │   ├── risk.rs         → XVA and risk metric calculations (feature = "calibration")
  │   ├── portfolio.rs    → Portfolio management endpoints
  │   ├── graphs.rs       → Computation graph endpoints
  │   ├── pricer_graph.rs → Pricing kernel computation graph extraction
  │   ├── exposure.rs     → Exposure calculation and metrics
  │   ├── scenarios.rs    → Scenario management and analysis
  │   └── benchmarks.rs   → Performance benchmark comparison endpoints
  │
  ├── state/          → Application state management
  ├── pricing_service.rs  → Pricing service integration
  ├── schedule_utils.rs   → Schedule generation utilities
  ├── websocket.rs    → WebSocket real-time updates
  ├── jobs.rs         → Async job manager for background processing
  ├── metrics.rs      → Prometheus-style metrics collection
  ├── openapi.rs      → OpenAPI/Swagger UI (feature = "openapi")
  ├── market_data.rs  → Market data module
  └── error.rs        → API error handling (ApiError, ApiResult)
static/          → Web assets (HTML, CSS, JS)
```

**Dual-Mode Architecture**:
- **TUI Mode**: `demo/gui/bin/demo-tui` (traditional terminal interface)
- **Web Mode**: `demo/gui/bin/demo-web` (browser-based dashboard, `feature = "web"`)
- Both share underlying application logic
- WebSocket for real-time chart updates
- Feature-gated compilation (default TUI, optional web)

**Web Features**:
- REST API handlers (`web/handlers/` - feature-organised modules)
- Computation graph visualisation (`/api/graph` endpoint)
- Performance metrics collection (API response times, WebSocket latency)
- CORS and Content Security Policy headers
- Environment-based configuration (`FB_CORS_ORIGINS`, `FB_CSP`)
- OpenAPI/Swagger documentation (`feature = "openapi"`, `web/openapi.rs`)
- Async job management (`web/jobs.rs`) - background processing for long-running tasks
- Prometheus-style metrics export (`web/metrics.rs`)

### Demo Data & Notebooks

**Location**: `demo/data/`
**Purpose**: Sample data for demo applications

```text
demo/data/
├── input/               → External data inputs
│   ├── curves/          → Yield curve instruments (usd-sofr.json, eur-estr.json, jpy-tona.json)
│   ├── volsurface/      → Volatility surface data (swaption, fx)
│   ├── market_data/     → Market data snapshots
│   ├── counterparties.json → Counterparty definitions
│   ├── netting_sets.json   → Netting set configurations
│   └── demo_portfolio.json → Sample portfolio
└── output/              → Generated outputs (reports, exports)
```

**Data File Pattern**: Index-based JSON files (e.g., `{index}.json` in `curves/`)

**Notebooks**: `demo/notebooks/` - Jupyter notebooks for pricing, calibration, XVA demos

---

## Infrastructure

**Docker**: `docker/`

- `Dockerfile.stable` - A/I/P/S builds (no Enzyme)
- `Dockerfile.nightly` - pricer_pricing with Enzyme LLVM plugin
- `Dockerfile.gui` - Multi-stage build for demo web dashboard

**Cloud Deployment**: Root directory

- `cloudbuild.yaml` - Google Cloud Build CI/CD pipeline (build→push→deploy)
- `.dockerignore` - Build optimisation (exclude unnecessary files)
- `.gcloudignore` - Cloud deployment optimisation
- `demo/frictional_bank/Dockerfile` - Cloud Run deployment container

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
2. **Feature Flag Isolation**: pricer_core supports `enzyme-mode` for advanced AD
3. **Static Dispatch**: Prefer `enum` over `Box<dyn Trait>` for Enzyme optimisation
4. **Smooth by Default**: All discontinuous functions have smooth approximations
5. **Test Co-Location**: Unit tests in same file as implementation (`#[cfg(test)]`)

---
_Created: 2025-12-29_
_Updated: 2026-01-30_ — Added linear solve strategies (linalg/strategy.rs), updated handlers list (7 new handlers: config, risk, exposure, scenarios, pricer_graph, benchmarks)
_Document patterns, not file trees. New files following patterns should not require updates_
