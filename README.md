# Neutryx Rust - XVA Pricing Library with Enzyme AD

A production-grade XVA (Credit Valuation Adjustment) pricing library in Rust, powered by **Enzyme automatic differentiation** for high-performance Greeks computation.

## 🎯 Project Goals

- **Bank-grade pricing**: CVA, DVA, FVA calculations for derivatives portfolios
- **Cutting-edge AD**: Enzyme (LLVM-level AD) for C++-competitive performance
- **Production stability**: 4-layer architecture isolating experimental code
- **Dual-mode verification**: Enzyme vs num-dual for correctness validation

## 🏗️ Architecture

### 4-Layer Design

```
neutryx-rust/
├── crates/
│   ├── pricer_core/      # L1: Foundation (Stable Rust)
│   ├── pricer_models/    # L2: Business Logic (Stable Rust)
│   ├── pricer_pricing/    # L3: AD Engine (Nightly Rust + Enzyme)
│   └── pricer_risk/       # L4: Application (Stable Rust)
```

| Layer | Purpose | Rust | Enzyme | Status |
|-------|---------|------|--------|--------|
| L1: pricer_core | Math types, traits, smoothing | Stable | No | ✅ Complete |
| L2: pricer_models | Instruments, models, analytical | Stable | No | ✅ Complete |
| L3: pricer_pricing | Monte Carlo, Enzyme AD | **Nightly** | **Yes** | 🚧 In Progress |
| L4: pricer_risk | Portfolio, XVA, parallelization | Stable | No | ✅ Complete |

### Design Principles

1. **Enzyme Isolation**: Experimental AD code confined to L3 only
2. **Static Dispatch**: Enum-based instruments (not `dyn Trait`) for Enzyme optimization
3. **Smooth Approximations**: All discontinuities smoothed for differentiability
4. **Dual-Mode Verification**: Both Enzyme and num-dual backends for validation

## 🚀 Quick Start

### Prerequisites

- **Rust**: Stable (for L1/L2/L4) + Nightly (for L3)
- **LLVM 18**: Required for Enzyme
- **Docker**: Recommended for reproducible builds

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly-2025-01-15
rustup component add --toolchain nightly-2025-01-15 rustfmt clippy
```

### Build (Stable Crates Only)

```bash
# Build L1, L2, L4 (no Enzyme required)
cargo build --workspace --exclude pricer_pricing

# Run tests
cargo test --workspace --exclude pricer_pricing
```

### Build with Enzyme (L3)

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

## 📚 Documentation

- **[Implementation Plan](/.claude/plans/bubbly-conjuring-torvalds.md)**: Complete technical design
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

### Project Structure

```
neutryx-rust/
├── crates/
│   ├── pricer_core/src/
│   │   ├── math/              # Smoothing functions
│   │   ├── traits/            # Priceable, Differentiable
│   │   └── types/             # Dual numbers, time
│   ├── pricer_models/src/
│   │   ├── analytical/        # Black-Scholes, barriers
│   │   ├── instruments/       # Options, swaps
│   │   └── models/            # Stochastic models
│   │       ├── equity/        # GBM (feature-gated)
│   │       ├── rates/         # Hull-White, CIR (feature-gated)
│   │       └── hybrid/        # Correlated multi-factor (feature-gated)
│   ├── pricer_pricing/src/
│   │   ├── enzyme/            # Enzyme bindings
│   │   ├── mc/                # Monte Carlo kernel
│   │   ├── checkpoint/        # Memory management
│   │   └── verify/            # Verification tests
│   └── pricer_risk/src/
│       ├── portfolio/         # Trade structures
│       ├── xva/               # CVA, DVA, FVA
│       ├── soa/               # Structure of Arrays
│       └── parallel/          # Rayon parallelization
├── docker/
│   ├── Dockerfile.stable      # L1/L2/L4 builds
│   └── Dockerfile.nightly     # L3 with Enzyme
├── scripts/
│   ├── install_enzyme.sh      # Enzyme installation
│   └── verify_enzyme.sh       # Enzyme verification
└── .github/workflows/
    └── ci.yml                 # CI/CD pipeline
```

### Coding Guidelines

1. **Smoothing**: Use `smooth_max`, `smooth_indicator` instead of `if` conditions
2. **Static Dispatch**: Prefer `enum` over `Box<dyn Trait>`
3. **Per-Instrument Epsilon**: Each instrument has configurable `smoothing_epsilon`
4. **Enzyme-Friendly Loops**: Use fixed-size `for` loops, not `while`

### Feature Flags

- **pricer_core**:
  - `num-dual-mode` (default): Verification with dual numbers
  - `enzyme-mode`: Production mode (f64 only)
- **Asset Classes** (pricer_models):
  - `equity` (default): Equity models (GBM)
  - `rates`: Interest rate models (Hull-White, CIR)
  - `credit`: Credit models
  - `fx`: FX models
  - `commodity`: Commodity models
  - `exotic`: Exotic derivatives
  - `all`: Enable all asset classes

## 🎯 Roadmap

- [x] **Phase 0**: Workspace scaffolding (Completed)
- [x] **Phase 1**: Foundation (L1) - types, traits, smoothing
- [x] **Phase 2**: Business logic (L2) - instruments, models
- [ ] **Phase 3**: Enzyme integration (L3) - AD bindings, verification
- [ ] **Phase 4**: Advanced MC - checkpointing, path-dependent options
- [x] **Phase 5**: XVA application (L4) - CVA, DVA, FVA, exposure metrics
- [ ] **Phase 6**: Production hardening - docs, benchmarks, CI/CD

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

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Ensure all tests pass:
```bash
# Stable crates
cargo test --workspace --exclude pricer_pricing

# Pricer kernel (requires Enzyme)
cargo +nightly test -p pricer_pricing

# Formatting and linting
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

## 🔗 References

- [Enzyme AD](https://enzyme.mit.edu/) - LLVM-level automatic differentiation
- [XVA Pricing](https://en.wikipedia.org/wiki/XVA) - Credit valuation adjustment
- [Implementation Plan](/.claude/plans/bubbly-conjuring-torvalds.md) - Full technical design

---

**Status**: ✅ Phases 1, 2, 5 complete | 🚧 Phase 3 (Enzyme AD) in progress
