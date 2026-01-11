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
│   ├── infra_master/         # Static master data (Calendars, Currencies, ISINs)
│   ├── infra_store/          # Persistence & State (SQLx, Redis, TimeScale)
│   │
│   │   # --- P: Pricer Layer (The Kernel) ---
│   ├── pricer_core/          # L1: Math, Traits, Types (Stable)
│   ├── pricer_models/        # L2: Instrument Definitions & Stochastic Models
│   ├── pricer_optimiser/     # L2.5: Calibration, Bootstrapping & Solvers
│   ├── pricer_pricing/       # L3: AD Engine (Enzyme) & Monte Carlo Kernel
│   ├── pricer_risk/          # L4: Risk Analytics, XVA & Portfolio Aggregation
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
| **P**ricer | pricer_* | Quantitative computation | Mixed | L3 only |
| **S**ervice | service_* | User interfaces | Stable | No |

### Dependency Rules

1. **S**ervices may depend on any **P**, **I**, or **A** crate.
2. **P**ricer crates must never depend on **S** or **A** crates.
3. **I**nfra crates must never depend on **P** or **S** crates.
4. **A**dapter crates depend only on **I** (for definitions) or **P** (for target types), never on **S**.

## 🚀 Quick Start

### Prerequisites

- **Rust**: Stable (for most crates) + Nightly (for pricer_pricing)
- **LLVM 18**: Required for Enzyme
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

### Build with Enzyme (pricer_pricing)

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

# Build pricer_pricing with Enzyme
export RUSTFLAGS="-C llvm-args=-load=/usr/local/lib/LLVMEnzyme-18.so"
cargo +nightly build -p pricer_pricing

# Run tests
cargo +nightly test -p pricer_pricing
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

## 📚 Documentation

- **[System Design Document](docs/design/SDD.md)**: Architecture details
- **API Docs**: `cargo doc --open` (stable crates)

## 🧪 Testing

### Unit Tests

```bash
# Stable crates
cargo test --workspace --exclude pricer_pricing

# Pricer kernel (requires Enzyme)
cargo +nightly test -p pricer_pricing
```

### Verification Tests

```bash
# Dual-mode: Enzyme vs num-dual
cargo +nightly test -p pricer_pricing --test verification
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
- **pricer_models**:
  - `equity` (default): Equity models (GBM)
  - `rates`: Interest rate models (Hull-White, CIR)
  - `credit`: Credit models
  - `fx`: FX models
  - `commodity`: Commodity models
  - `exotic`: Exotic derivatives
  - `all`: Enable all asset classes
- **pricer_pricing**:
  - `enzyme-ad`: Enable Enzyme automatic differentiation
  - `num-dual-fallback`: Fallback to num-dual for verification

## 🎯 Roadmap

- [x] **Phase 0**: Workspace scaffolding (Completed)
- [x] **Phase 1**: Foundation (L1) - types, traits, smoothing
- [x] **Phase 2**: Business logic (L2) - instruments, stochastic models (Heston, SABR, Hull-White)
- [x] **Phase 3**: Enzyme integration (L3) - AD bindings, `#[autodiff]` macro, Greeks computation
- [x] **Phase 4**: Monte Carlo kernel - path-dependent options, checkpointing
- [x] **Phase 5**: Risk Analytics (L4) - XVA (CVA, DVA, FVA), exposure metrics, scenarios
- [x] **Phase 6**: A-I-P-S Architecture - adapters, infra, service layers
- [ ] **Phase 7**: Exotic Options - Barriers, Asians, Lookbacks, Digitals
- [ ] **Phase 8**: Service Layer Enhancement - gRPC, Python bindings expansion
- [ ] **Phase 9**: Production hardening - docs, benchmarks, CI/CD

## 📊 Completed Specifications

| Specification | Description | Date |
|---------------|-------------|------|
| core-traits-types-2 | Core traits and type definitions | 2025-12 |
| rng-infrastructure | Random number generation (PRNG/QMC) | 2025-12 |
| enzyme-infrastructure-setup | Enzyme AD infrastructure | 2025-12 |
| interpolation-solvers | Interpolation and numerical solvers | 2025-12 |
| market-data-structures | Yield curves and volatility surfaces | 2025-12 |
| instrument-definitions | Financial instrument definitions | 2025-12 |
| monte-carlo-kernel-enzyme | Monte Carlo pricing kernel | 2026-01 |
| stochastic-models | Heston, SABR, Hull-White stochastic models | 2026-01 |
| enzyme-autodiff-integration | Enzyme `#[autodiff]` macro integration | 2026-01 |

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

**Status**: ✅ A-I-P-S architecture complete | ✅ Enzyme AD integration complete | ✅ Stochastic models (Heston, SABR, Hull-White) | 🚧 Exotic options in progress
