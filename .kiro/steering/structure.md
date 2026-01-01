# Project Structure

## Organization Philosophy

**Layer-Based Architecture with Dependency Flow**:
Strict bottom-up dependencies, isolating experimental technology (Enzyme) to Layer 3 while keeping foundation and application stable.

```
L1 (pricer_core)    → No dependencies, pure Rust traits/types
L2 (pricer_models)  → Depends on L1 only
L3 (pricer_kernel)  → Currently isolated (Phase 3.0), will integrate L1+L2 in Phase 4
L4 (pricer_xva)     → Depends on L1+L2+L3, stable Rust
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
```
instruments/  → Enum-based instrument definitions (VanillaOption, Forward, Swap)
models/       → Stochastic models with unified trait interface
analytical/   → Closed-form solutions (Black-Scholes, barrier formulas)
```

**Key Principles**:

- Static dispatch via `enum Instrument<T: Float>` for Enzyme compatibility
- **StochasticModel Trait**: Unified interface for stochastic processes (`evolve_step`, `initial_state`, `brownian_dim`)
- **StochasticModelEnum**: Static dispatch enum wrapping concrete models (GBM, future: Heston, SABR)
- **State Types**: `SingleState<T>` (1-factor), `TwoFactorState<T>` (2-factor) via `StochasticState` trait
- **ModelParams/ModelState**: Unified enums for type-safe parameter and state handling

### Layer 3: AD Engine (pricer_kernel)

**Location**: `crates/pricer_kernel/src/`
**Purpose**: Monte Carlo + Enzyme AD (nightly Rust, LLVM plugin)
**Structure**:
```
enzyme/       → Enzyme bindings, autodiff macros
mc/           → Monte Carlo kernel (GBM paths, workspace buffers, Greeks)
rng/          → Random number generation (PRNG, QMC sequences)
verify/       → Enzyme vs num-dual verification tests
checkpoint/   → (Phase 4) Memory management for path-dependent options
```

**Key Principle**: **Only crate requiring nightly Rust and Enzyme**. Currently isolated (Phase 3.0) with zero pricer_* dependencies.

**RNG Design**: Zero-allocation batch operations, static dispatch only, Enzyme-compatible. Supports reproducible seeding for deterministic simulations.

**Monte Carlo Features** (Phase 3.2):
- Pre-allocated workspace buffers (`PathWorkspace`) for allocation-free simulation
- GBM path generation with log-space formulation
- Smooth payoff functions for AD compatibility
- Greeks via bump-and-revalue with forward-mode AD prototype

### Layer 4: Application (pricer_xva)

**Location**: `crates/pricer_xva/src/`
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

**CI/CD**: `.github/workflows/ci.yml`
- Separate jobs for stable (L1/L2/L4) and nightly (L3)

## Naming Conventions

- **Crates**: `pricer_*` prefix, snake_case (`pricer_core`, `pricer_kernel`)
- **Modules**: snake_case (`monte_carlo`, `smoothing`)
- **Traits**: PascalCase (`Priceable`, `Differentiable`)
- **Types**: PascalCase (`DualNumber`, `VanillaOption`)
- **Functions**: snake_case (`smooth_max`, `price_european`)

## Import Organization

**Absolute imports** for cross-crate dependencies:
```rust
use pricer_core::traits::Priceable;
use pricer_models::instruments::Instrument;
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
- Phase 3: L3 Enzyme integration - AD infrastructure, MC kernel (largely complete; Phase 3.2 bump-and-revalue Greeks, Phase 4 will add `#[autodiff]` macros)
- Phase 4: Advanced MC - checkpointing, path-dependent options (planned)
- Phase 5: L4 XVA - CVA/DVA/FVA, exposure metrics, parallelization (complete)
- Phase 6: Production hardening - docs, benchmarks, CI/CD (in progress)

---
_Created: 2025-12-29_
_Updated: 2026-01-01_ — Phase 3 status clarified (3.2 complete, autodiff integration Phase 4), Phase 6 now in progress
_Document patterns, not file trees. New files following patterns should not require updates_
