# Neutryx Rust - Bank Derivatives Pricing Library with Enzyme AD

A production-grade **derivatives pricing library** for Tier-1 banks in Rust, powered by **Enzyme automatic differentiation** for high-performance Greeks computation.

## 🎯 Project Goals

- **Multi-Asset Class Pricing**: Comprehensive coverage of Rates, FX, Equity, Credit, and Commodity derivatives
- **Cutting-edge AD**: Enzyme (LLVM-level AD) with `#[autodiff]` macro for C++-competitive performance
- **Stochastic Models**: Heston, SABR, Hull-White with integrated calibration
- **Production stability**: A-I-P-S architecture isolating experimental code
- **Dual-mode verification**: Enzyme vs num-dual for correctness validation
- **XVA & Risk Analytics**: CVA, DVA, FVA calculations with exposure metrics (EE, EPE, PFE)

## 🏗️ Architecture

### A-I-P-S Unidirectional Data Flow

The workspace structure enforces a strict unidirectional data flow that mirrors the alphabetical order (**A**dapter → **I**nfra → **P**ricer → **S**ervice). This logical progression ensures that the file system itself acts as an architectural map.

```text
neutryx-rust/
├── crates/
│   │
│   │   # --- A: Adapter Layer (Input) ---
│   ├── adapter_feeds/        # Real-time/Snapshot market data parsers
│   ├── adapter_fpml/         # Trade definition parsers (FpML/XML)
│   ├── adapter_loader/       # Flat file loaders (CSV/Parquet) & CSA details
│   │
│   │   # --- I: Infra Layer (Foundation) ---
│   ├── infra_config/         # System configuration & environment management
│   ├── infra_domain/         # Static master data (Calendars, Currencies, ISINs)
│   ├── infra_store/          # Persistence & State (SQLx, Redis, TimeScale)
│   │
│   │   # --- P: Pricer Layer (The Kernel) ---
│   ├── pricer_core/          # L1: Math (smoothing, solvers, integrators), Types, Traits
│   ├── pricer_models/        # L2: Instruments, Market (curves, surfaces, calibration), Models
│   ├── pricer_pricing/       # L3: Monte Carlo, Tree Pricing, RNG, Greeks
│   ├── pricer_risk/          # L4: Portfolio, XVA, Scenarios, Enzyme AD
│   │
│   │   # --- S: Service Layer (Output) ---
│   ├── service_cli/          # Command Line Operations (Batch/Ops)
│   ├── service_gateway/      # gRPC/REST API Gateway (Microservices)
│   └── service_python/       # PyO3 Bindings (Research/Jupyter)
```

### Layer Overview

| Layer | Crates | Purpose | Rust | Enzyme |
|-------|--------|---------|------|--------|
| **A**dapter | adapter_* | External data ingestion | Stable | No |
| **I**nfra | infra_* | Configuration, persistence | Stable | No |
| **P**ricer | pricer_* | Quantitative computation | Mixed | L4 only |
| **S**ervice | service_* | User interfaces | Stable | No |

> **Note**: Enzyme AD has been moved from pricer_pricing (L3) to pricer_risk (L4) for better integration with portfolio-level risk calculations.

### Dependency Rules

1. **S**ervices may depend on any **P**, **I**, or **A** crate.
2. **P**ricer crates must never depend on **S** or **A** crates.
3. **I**nfra crates must never depend on **P** or **S** crates.
4. **A**dapter crates depend only on **I** (for definitions) or **P** (for target types), never on **S**.

## 🚀 Quick Start

### Using the Neutryx Facade Crate

The easiest way to use Neutryx is through the unified facade crate:

```toml
[dependencies]
neutryx = { path = "." }  # or from crates.io when published
```

```rust
use neutryx::prelude::*;
use neutryx::models::market::YieldCurve;

// Access dates, currencies, and trade definitions
let date = Date::from_ymd(2024, 1, 15).unwrap();
let usd = Currency::USD;

// Build yield curves and price derivatives
// ... see documentation for full examples
```

**Feature Tiers**:
- `minimal` — Master data only (dates, currencies, trade definitions)
- `analytics` — Curve building, models, analytical pricing
- `full` (default) — Complete pricing and risk functionality

### Prerequisites

- **Rust**: Stable (for most crates) + Nightly (for pricer_risk with enzyme-ad feature)
- **LLVM 18**: Required for Enzyme AD
- **Docker**: Recommended for reproducible builds

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly-2025-01-15
rustup component add --toolchain nightly-2025-01-15 rustfmt clippy llvm-tools-preview
```

### Build (Stable Crates Only)

```bash
# Build all except pricer_pricing (no Enzyme required)
cargo build --workspace --exclude pricer_pricing

# Run tests
cargo test --workspace --exclude pricer_pricing
```

### Build with Enzyme (pricer_risk)

#### Option 1: Docker (Recommended)

```bash
# Build Docker image with Enzyme pre-installed
docker build -f docker/Dockerfile.nightly -t neutryx-enzyme .

# Run container
docker run -it neutryx-enzyme
```

#### Option 2: Local Installation

```bash
# Install Enzyme LLVM plugin
./scripts/install_enzyme.sh

# Verify installation
./scripts/verify_enzyme.sh

# Build pricer_risk with Enzyme AD
export RUSTFLAGS="-C llvm-args=-load=/usr/local/lib/LLVMEnzyme-18.so"
cargo +nightly build -p pricer_risk --features enzyme-ad

# Run tests
cargo +nightly test -p pricer_risk --features enzyme-ad
```

### CLI Usage

```bash
# Build the CLI
cargo build -p service_cli --release

# Check system configuration
./target/release/neutryx check

# Price a portfolio
./target/release/neutryx price --portfolio trades.csv

# Calibrate a model
./target/release/neutryx calibrate --market-data swaptions.csv --model-type hull-white
```

### Server Usage

```bash
# Start the REST API server
cargo run -p service_gateway

# Health check
curl http://localhost:8080/health

# Price an instrument
curl -X POST http://localhost:8080/api/v1/price \
  -H "Content-Type: application/json" \
  -d '{"instrument_type": "vanilla_option", "strike": 100, "expiry": 1.0, "spot": 100, "volatility": 0.2, "rate": 0.05}'
```

### Python Usage

```bash
# Build Python bindings (requires maturin)
pip install maturin
cd crates/service_python
maturin develop

# Use in Python
python -c "import neutryx; print(neutryx.version())"
```

### Frictional Bank Demo

The Frictional Bank demo showcases the A-I-P-S architecture with a complete end-to-end workflow.

```bash
# From workspace root
cargo run --bin frictional-bank

# Release build (recommended for performance)
cargo run --release --bin frictional-bank

# Or from demo/frictional_bank directory
cd demo/frictional_bank
cargo run
```

#### TUI (Terminal UI) Demo

Interactive terminal-based dashboard:

```bash
cargo run --release --bin demo-tui
```

**Controls:**
- Arrow keys / Tab: Navigate menus
- Enter: Select / Execute
- q / Esc: Exit

#### Web Dashboard

Browser-based interface:

```bash
cargo run --release -p demo_gui --bin demo-web
```

After startup, open `http://localhost:8080` in your browser.

## 📚 Documentation

- **[System Design Document](docs/design/SDD.md)**: Architecture details
- **API Docs**: `cargo doc --open` (stable crates)

> **Note**: The `pricer_optimiser` crate (L2.5) was removed in 2026-01. Market data (curves, surfaces, bootstrapping, provider) has been consolidated into `pricer_models::market`, and calibration engine into `pricer_models::market::calibration`. Enzyme AD has been moved from `pricer_pricing` (L3) to `pricer_risk` (L4).

## 🧪 Testing

### Unit Tests

```bash
# Stable crates (most of the workspace)
cargo test --workspace

# With Enzyme AD (requires nightly + LLVM 18)
cargo +nightly test -p pricer_risk --features enzyme-ad
```

### Verification Tests

```bash
# Dual-mode: Enzyme vs num-dual
cargo +nightly test -p pricer_risk --features enzyme-ad --test verification
```

### Benchmarks

```bash
cargo bench
```

## 🛠️ Development

### Coding Guidelines

1. **British English**: Use `optimiser`, `serialisation`, `modelling`
2. **Smoothing**: Use `smooth_max`, `smooth_indicator` instead of `if` conditions
3. **Static Dispatch**: Prefer `enum` over `Box<dyn Trait>`
4. **Per-Instrument Epsilon**: Each instrument has configurable `smoothing_epsilon`
5. **Enzyme-Friendly Loops**: Use fixed-size `for` loops, not `while`

### Feature Flags

- **pricer_core**:
  - `num-dual-mode` (default): Verification with dual numbers
  - `enzyme-mode`: Production mode (f64 only)
  - `linalg`: Linear algebra support (nalgebra wrappers)
- **pricer_models**:
  - `equity` (default): Equity models (GBM, Heston, SABR)
  - `rates`: Interest rate models (Hull-White, CIR)
  - `credit`: Credit models
  - `fx`: FX models and calibration
  - `commodity`: Commodity models
  - `exotic`: Exotic derivatives
  - `all`: Enable all asset classes
- **pricer_pricing**:
  - `l1l2-integration`: Full L1/L2 access for IRS Greeks workflow
- **pricer_risk**:
  - `enzyme-ad`: Enable Enzyme automatic differentiation (requires nightly)

## 🎯 Roadmap

- [x] **Phase 0**: Workspace scaffolding
- [x] **Phase 1**: Foundation (L1) - types, traits, smoothing, math library
- [x] **Phase 2**: Business logic (L2) - instruments, stochastic models (Heston, SABR, Hull-White)
- [x] **Phase 3**: Enzyme integration - AD bindings, `#[autodiff]` macro, Greeks computation
- [x] **Phase 4**: Monte Carlo kernel - path-dependent options, checkpointing, tree pricing
- [x] **Phase 5**: Risk Analytics (L4) - XVA, exposure metrics, scenarios, Enzyme AD (moved to L4)
- [x] **Phase 6**: A-I-P-S Architecture - adapters, infra, service layers
- [x] **Phase 7**: Architecture Refactoring - pricer_optimiser removal, infra_domain consolidation
- [x] **Phase 8**: Calibration Infrastructure - curve bootstrapping, FX/IR vol surface calibration, SABR
- [x] **Phase 9**: IndexedMarket Pattern - index-keyed market access, TradeIndexRequirements
- [ ] **Phase 10**: Exotic Options - Barriers, Asians, Lookbacks, Digitals
- [ ] **Phase 11**: Service Layer Enhancement - gRPC, Python bindings expansion
- [ ] **Phase 12**: Production hardening - docs, benchmarks, CI/CD

## 📊 Completed Specifications (44 Total)

| Specification | Description | Date |
|---------------|-------------|------|
| core-traits-types-2 | Core traits and type definitions | 2025-12 |
| rng-infrastructure | Random number generation (PRNG/QMC) | 2025-12 |
| enzyme-infrastructure-setup | Enzyme AD infrastructure | 2025-12 |
| interpolation-solvers | Interpolation and numerical solvers | 2025-12 |
| market-data-structures | Yield curves and volatility surfaces | 2025-12 |
| instrument-definitions | Financial instrument definitions | 2025-12 |
| monte-carlo-kernel-enzyme | Monte Carlo pricing kernel | 2026-01 |
| service-layer-rename | Crate renaming (kernel→pricing, xva→risk) | 2026-01 |
| stochastic-models | Heston, SABR, Hull-White stochastic models | 2026-01 |
| enzyme-autodiff-integration | Enzyme `#[autodiff]` macro integration | 2026-01 |
| frictional-bank | FrictionalBank demo system (TUI, Web, Workflows) | 2026-01 |
| frictionalbank-irs-bootstrap-risk | IRS bootstrapping and risk workflows | 2026-01 |
| frictional-bank-webapp-polish | Web dashboard UX improvements | 2026-01 |
| frictionalbank-webapp-pricer | Web dashboard pricer integration | 2026-01 |
| advanced-sensitivity-webapp | Advanced sensitivity analysis for web dashboard | 2026-01 |
| codebase-cleanup-optimisation | Codebase cleanup and optimisation | 2026-01 |
| portfolio-graph-optimisation | Portfolio Graph REST API and WebSocket handlers | 2026-01 |
| infra-primitives-migration | Financial primitives migration to infra_domain | 2026-01 |
| model-architecture-refactoring | pricer_optimiser removal, consolidation | 2026-01 |
| counterparty-netting-module | Counterparty and netting set data structures | 2026-01 |
| financial-time-module | Financial time primitives (calendars, frequencies) | 2026-01 |
| trade-instrument-module | Trade/Instrument module with CF-expanded architecture | 2026-01 |
| pricer-core-math-library | Comprehensive math library (distributions, integrators, optimisers) | 2026-01 |
| codebase-simplification | Code deduplication, API surface minimisation | 2026-01 |
| standard-instrument-catalogue | Standard instrument definitions (Rates, FX, Equity, Credit, Commodity) | 2026-01 |
| domain-ordering-defaults | Domain enum ordering and documentation | 2026-01 |
| portfolio-book-model | Portfolio/Book organisation model with XVA/Exposure/Netting support | 2026-01 |
| curve-builder-webapp | Curve Builder WebApp with instrument editing | 2026-01 |
| generic-pricer-engine | Generic Pricer engine with market provider integration | 2026-01 |
| demo-webapp-pricer | Demo WebApp Pricer with daily accruals display | 2026-01 |
| volcube-calibration-ui | VolCube/FxVol calibration UI with SABR, 3D surface | 2026-01 |
| curve-bootstrap-engine | Multi-curve yield curve bootstrapping engine (537 tests) | 2026-01 |
| legacy-compatibility-removal | Legacy code removal and ID type safety | 2026-01 |
| rate-index-pricing-integration | RateIndex pricing integration across L1/L2/L3 layers | 2026-01 |
| fx-vol-surface-calibration | FX curve + vol surface calibration with SABR (139 tests) | 2026-01 |
| move-enzyme-to-pricer-risk | Enzyme AD module moved from L3 to L4 | 2026-01 |
| ir-vol-cube-calibration | IR VolCube calibration engine with SABR, AAD Vega | 2026-01 |
| pricer-pricing-architecture | Tree pricing (Binomial/Trinomial), UnifiedPricingResult | 2026-01 |
| market-index-keyed-access | IndexedMarket, TradeIndexRequirements, MarketValidator | 2026-01 |
| shadow-object-aad | Shadow trait, slice-based kernels, AAD binder layer | 2026-01 |
| external-numerics-migration | argmin/levenberg-marquardt integration | 2026-01 |
| mc-memory-layout-optimisation | PathLayout, AlignedPathBuffer, StreamingEngine | 2026-01 |
| pricing-kernel-ir | PricingKernel IR, TradeCompiler, IndexMapper, LSMC, CMS | 2026-01 |
| curve-global-solver | Global curve calibration with Newton-Raphson (422 tests) | 2026-01 |

## 📊 Performance Targets

| Operation | Target | Status |
|-----------|--------|--------|
| Vanilla option (analytical) | < 1 μs | 🎯 Future |
| Barrier option (1K paths) | < 100 μs | 🎯 Future |
| Asian option (10K paths) | < 1 ms | 🎯 Future |
| CVA (100 trades, 50 steps) | < 5 s | 🎯 Future |
| Enzyme delta overhead | < 2x vs forward | 🎯 Future |

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🤝 Contributing

Contributions welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
# Before submitting
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace --exclude pricer_pricing
```

## 🔗 References

- [Enzyme AD](https://enzyme.mit.edu/) - LLVM-level automatic differentiation
- [Derivatives Pricing](https://en.wikipedia.org/wiki/Derivative_(finance)) - Financial derivatives
- [XVA](https://en.wikipedia.org/wiki/XVA) - Credit valuation adjustments

---

**Status**: ✅ A-I-P-S architecture complete | ✅ Neutryx facade crate | ✅ Enzyme AD integration (L4) | ✅ Stochastic models (Heston, SABR, Hull-White) | ✅ Curve & Vol Surface Calibration (FX/IR) | ✅ 44 specifications complete
