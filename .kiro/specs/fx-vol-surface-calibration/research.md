# Research Log: fx-vol-surface-calibration

## Discovery Summary

本リサーチでは、FXボラティリティサーフェスカリブレーションシステム実装に向けた技術調査を実施した。既存コードベースの分析結果、**80-95%の基盤が既存実装として利用可能**であり、主なギャップは新規型定義とオーケストレーション層に限定される。

---

## 1. Existing Implementation Analysis

### 1.1 FX Volatility Surface Infrastructure

**Location**: `crates/pricer_models/src/market/surfaces/fx.rs`

```rust
pub struct FxVolatilitySurface<T: Float> {
    pub currency_pair: CurrencyPair,
    pub reference_date: NaiveDate,
    pub smile_by_expiry: BTreeMap<NaiveDate, DeltaSmile<T>>,
    delta_type: DeltaType,
}
```

**Capabilities**:
- Delta空間での補間 (vol_by_delta)
- Expiry補間 (BilinearInterpolator)
- Generic `<T: Float>` によるAAD互換性
- DeltaType (SpotDelta, PremiumAdjustedDelta) サポート

**Gaps**:
- Strike空間への直接変換メソッドなし
- SABR/SVI parametric interpolator統合なし
- Builder patternなし

**Approach**: Extend existing struct with strike-space methods and builder wrapper

### 1.2 Delta-Strike Conversion

**Location**: `crates/pricer_models/src/market/fx_density.rs`

```rust
pub struct FxDensityCalculator<T: Float> {
    pub fn delta_to_strike(&self, delta: T, forward: T, sigma: T, expiry: T) -> T
    pub fn strike_to_delta(&self, strike: T, forward: T, sigma: T, expiry: T) -> T
}
```

**Status**: Production-ready, fully generic, AD-compatible
**Action**: Direct reuse with surface integration

### 1.3 Bootstrap Infrastructure

**Location**: `crates/pricer_models/src/market/calibration/bootstrapping/`

| Component | File | Reusability |
|-----------|------|-------------|
| `SequentialBootstrapper<T>` | `engine.rs` | 95% - Generic solver |
| `AdjointSolver<T>` | `adjoint_solver.rs` | 100% - AAD support |
| `CurveEngine` | `curve_engine.rs` | 90% - OIS curve pattern |
| `MultiCurveBuilder<T>` | `multi_curve.rs` | 85% - Dependency ordering |
| `CurveResultCache` | `cache.rs` | 95% - Caching pattern |

**Key Finding**: SequentialBootstrapper is fully generic and can be used directly for FX curve bootstrapping.

### 1.4 SABR Calibration (VolCube)

**Location**: `crates/pricer_models/src/market/volcube/`

```rust
pub struct SabrParameterSurface<T: Float> {
    expiries: Vec<T>,
    parameters: Vec<SabrParameters<T>>,
}
```

**Capabilities**:
- Per-expiry SABR calibration
- Breeden-Litzenberger density extraction
- VolCube grid interpolation

**Action**: Reuse calibration patterns for FX vol SABR integration

### 1.5 WebApp Handlers

**Location**: `demo/gui/src/web/`

| File | Status | Action |
|------|--------|--------|
| `fxvol_handlers.rs` | Skeleton | Implement full functionality |
| `fxvol_types.rs` | Basic types | Extend with new types |
| `curve_builder_handlers.rs` | Working | Reference pattern |
| `volcube_handlers.rs` | Working | Reference pattern |

---

## 2. Technology Alignment

### 2.1 Stack Consistency

| Layer | Technology | Alignment |
|-------|------------|-----------|
| Numeric | `num-traits`, `num-dual` | Existing |
| Interpolation | Custom implementations | Existing |
| Optimisation | `argmin` (L-BFGS, Nelder-Mead) | Existing |
| Serialisation | `serde` | Existing |
| Web | `axum`, `tower-http` | Existing |
| Error | `thiserror` | Existing |

**No new external dependencies required**

### 2.2 A-I-P-S Alignment

| Component | Layer | Crate |
|-----------|-------|-------|
| `FxVolInstrument` | Infra | `infra_domain` |
| `FxSwapInstrument` | Infra | `infra_domain` |
| `CrossCurrencyBasisSwap` | Infra | `infra_domain` |
| `FxCurve<T>` trait | Pricer | `pricer_models` |
| `FxVolSurfaceBuilder` | Pricer | `pricer_models` |
| `FxMarketBuilder` | Pricer | `pricer_models` |
| WebApp handlers | Demo | `demo/gui` |

---

## 3. Architecture Pattern Analysis

### 3.1 Evaluated Patterns

**Option A: Extend Existing Components**
- Pros: Minimal files, existing test reuse
- Cons: Single file bloat, breaking changes risk

**Option B: Create New Components**
- Pros: Clean separation, easy testing
- Cons: More files, interface design overhead

**Option C: Hybrid Approach (Selected)**
- Phase-based implementation
- Extend where appropriate, create new where necessary
- Risk mitigation through incremental rollout

### 3.2 Selected Pattern: Hybrid with Phased Implementation

```
Phase 1: Core Types (infra_domain)
  └── FxVolInstrument, FxSwapInstrument, CrossCurrencyBasisSwap

Phase 2: Builders (pricer_models)
  └── FxForwardCurveBuilder (wraps SequentialBootstrapper)
  └── FxVolSurfaceBuilder (extends FxVolatilitySurface)

Phase 3: Integration (pricer_models)
  └── FxMarketBuilder (orchestrates CurveEngine + FX builders)

Phase 4: WebApp & Cleanup (demo/gui)
  └── Handler implementation, deprecated code removal
```

---

## 4. External Dependency Investigation

### 4.1 No External Dependencies Required

All required functionality available through existing crates:
- SABR: `pricer_models::market::volcube::sabr_surface`
- Optimisation: `pricer_core::math::optimisers`
- Interpolation: `pricer_core::math::interpolators`

### 4.2 Internal Dependencies

| Consumer | Provider | Interface |
|----------|----------|-----------|
| FxForwardCurveBuilder | CurveEngine | Discount curves |
| FxVolSurfaceBuilder | FxForwardCurveBuilder | FX forward curve |
| FxMarketBuilder | All above | Orchestration |

---

## 5. Risk Assessment & Mitigation

### 5.1 Technical Risks

| Risk | Level | Mitigation |
|------|-------|------------|
| Tenor blending (1Y-2Y) | Medium | Smooth interpolation, extensive testing |
| Cache invalidation | Medium | Simple strategy first, optimize later |
| SABR edge cases | Medium | Leverage existing volcube tests |
| WebSocket complexity | Medium | Defer to Phase 2 if needed |

### 5.2 Integration Risks

| Risk | Level | Mitigation |
|------|-------|------------|
| Breaking existing API | Low | New module isolation |
| AD graph continuity | Low | Existing Float generics |
| Performance regression | Low | Benchmark existing paths |

---

## 6. Open Questions for Implementation

### 6.1 Resolved During Design

1. **Tenor blending algorithm**: Linear interpolation with configurable transition range
2. **XCCY swap pricing**: Support both MTM and non-MTM modes
3. **Cache granularity**: Expiry-level invalidation

### 6.2 Deferred to Implementation

1. **WebSocket protocol**: Message format details
2. **D3.js graph format**: Exact JSON structure
3. **Error message i18n**: Japanese/English support

---

## 7. File Location Summary

### 7.1 Files to Extend

| File | Changes |
|------|---------|
| `pricer_models/src/market/surfaces/fx.rs` | Add strike-space methods |
| `infra_domain/src/trade/instrument_def/fx.rs` | Extend FxSwap |

### 7.2 Files to Create

| File | Purpose |
|------|---------|
| `pricer_models/src/market/fx_calibration/mod.rs` | Module root |
| `pricer_models/src/market/fx_calibration/instruments.rs` | FxVolInstrument |
| `pricer_models/src/market/fx_calibration/builder.rs` | Builders |
| `pricer_models/src/market/fx_calibration/curve.rs` | FxCurve trait |
| `infra_domain/src/trade/instrument_def/fx_vol.rs` | FxVolInstrument |
| `infra_domain/src/trade/instrument_def/xccy.rs` | CrossCurrencyBasisSwap |
| `demo/gui/src/web/fxcurve_handlers.rs` | FX curve endpoints |

---

## 8. Conclusion

既存コードベースの成熟度が高く、実装リスクは低い。推奨アプローチ（Option C: Hybrid）により、**4フェーズで段階的に機能を追加**し、各フェーズで検証可能な成果物を提供する。

**Next Steps**:
1. Phase 1: `infra_domain`にインストルメント型定義
2. Phase 2: `pricer_models`にビルダー実装
3. Phase 3: `FxMarketBuilder`でオーケストレーション
4. Phase 4: WebApp統合とクリーンアップ
