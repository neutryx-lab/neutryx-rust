# Development Roadmap

実装状況と今後の開発項目を追跡するドキュメント。

_Updated: 2026-01-11_

---

## Current State Summary

### Completed Specifications (11)

| Spec | Description | Completed |
|------|-------------|-----------|
| core-traits-types-2 | Core traits and type definitions | 2025-12 |
| rng-infrastructure | Random number generation (PRNG/QMC) | 2025-12 |
| enzyme-infrastructure-setup | Enzyme AD infrastructure | 2025-12 |
| interpolation-solvers | Interpolation and numerical solvers | 2025-12 |
| market-data-structures | Yield curves and volatility surfaces | 2025-12 |
| instrument-definitions | Financial instrument definitions | 2025-12 |
| monte-carlo-kernel-enzyme | Monte Carlo pricing kernel | 2026-01 |
| service-layer-rename | Crate renaming (kernel→pricing, xva→risk) | 2026-01 |
| stochastic-models | Heston, SABR, Hull-White stochastic models | 2026-01 |
| enzyme-autodiff-integration | Enzyme #[autodiff] macro integration | 2026-01 |
| frictional-bank | FrictionalBank demo system (TUI, Web, Workflows) | 2026-01 |

### Layer Implementation Status

```text
Legend: ✅ Complete | 🔶 Basic/Partial | ❌ Not Started
```

#### Pricer Layer (P) - Core Engine
| Crate | Layer | Status | Notes |
|-------|-------|--------|-------|
| pricer_core | L1 | ✅ | math, market_data, types, traits |
| pricer_models | L2 | ✅ | instruments, analytical, calibration, schedules, demo |
| pricer_optimiser | L2.5 | ✅ | bootstrapping, calibration, solvers, provider |
| pricer_pricing | L3 | ✅ | mc, rng, enzyme, greeks, path_dependent, checkpoint, context (l1l2-integration) |
| pricer_risk | L4 | ✅ | portfolio, exposure, xva, scenarios (engine/shifts/aggregator/presets), soa, demo |

#### Infra Layer (I) - Foundation
| Crate | Status | Notes |
|-------|--------|-------|
| infra_config | ✅ | Settings loading (TOML/YAML/Env) |
| infra_master | ✅ | Calendars, day count conventions |
| infra_store | 🔶 | Basic traits only, postgres optional |

#### Adapter Layer (A) - Input
| Crate | Status | Notes |
|-------|--------|-------|
| adapter_feeds | 🔶 | Basic quote types only |
| adapter_fpml | 🔶 | Basic FpML parser skeleton |
| adapter_loader | 🔶 | CSV loader, CSA terms |

#### Service Layer (S) - Output
| Crate | Status | Notes |
|-------|--------|-------|
| service_cli | 🔶 | Basic commands (calibrate/price/report/demo) |
| service_gateway | 🔶 | REST skeleton only, gRPC not implemented |
| service_python | 🔶 | Basic bindings (VanillaOption, Forward, HullWhite) |

---

## Future Development Items

### Phase 1: Enzyme AD Full Integration (Priority: High) ✅

| Item | Description | Status |
|------|-------------|--------|
| enzyme-full-integration | Full `#[autodiff]` macro integration | ✅ |
| greeks-enzyme-ad | Delta/Gamma/Vega computation via Enzyme AD | ✅ |
| enzyme-benchmarks | Criterion benchmarks (requires LLVM 18 runtime) | ⏳ Deferred |

**Note**: Core implementation complete (18/20 tasks). Benchmarks deferred pending LLVM 18 environment.

### Phase 2: Additional Products & Models (Priority: Medium)

| Item | Description | Status |
|------|-------------|--------|
| exotic-options | Barriers, Asians, Lookbacks, Digitals | ❌ |
| rates-instruments | Swaption, Cap/Floor pricing completion | ❌ |
| stochastic-models | Heston, SABR, Hull-White completion | ✅ |
| volatility-surface | Implied vol surface construction & interpolation | ❌ |

### Phase 3: Service Layer Enhancement (Priority: Medium)

| Item | Description | Status |
|------|-------------|--------|
| grpc-implementation | gRPC service with streaming support | ❌ |
| python-bindings-expansion | Expand PyO3 bindings (MC Pricer, Greeks, etc.) | ❌ |
| cli-commands-completion | Complete CLI commands implementation | ❌ |

### Phase 4: Adapter Layer Enhancement (Priority: Low)

| Item | Description | Status |
|------|-------------|--------|
| fpml-parser-complete | Complete FpML/XML parser (IRS, Swaption, etc.) | ❌ |
| market-data-feeds | Real-time market data connectivity | ❌ |
| parquet-loader | Parquet file loader implementation | ❌ |

### Phase 5: Production Hardening (Priority: Low-Medium)

| Item | Description | Status |
|------|-------------|--------|
| performance-optimization | Benchmarks, SIMD, parallelisation improvements | ❌ |
| error-handling | Unified error handling, logging enhancement | ❌ |
| documentation | API docs, user guides, examples | ❌ |
| testing-coverage | Test coverage, property-based testing | ❌ |

---

## Recommended Next Steps

1. **exotic-options** - Barriers, Asians, Lookbacks, Digitals for product coverage
2. **python-bindings-expansion** - Enables Jupyter research workflows
3. **rates-instruments** - Swaption, Cap/Floor pricing completion

---

## Changelog

| Date | Change |
|------|--------|
| 2026-01-11 | frictional-bank: Complete (all tasks including optional Chart and Web dashboard) |
| 2026-01-10 | enzyme-autodiff-integration: Complete (18/20 tasks, benchmarks deferred) |
| 2026-01-10 | stochastic-models: Implementation complete (17/17 tasks) |
| 2026-01-10 | stochastic-models: Calibration complete (Heston, SABR, Hull-White calibrators) |
| 2026-01-10 | Steering sync: Added scenarios, demo, context, provider modules to layer status |
| 2026-01-10 | Initial creation; cleaned 20 incomplete specs, documented current state |
