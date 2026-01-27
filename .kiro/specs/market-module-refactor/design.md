# Technical Design: market-module-refactor

## 概要

`pricer_models::market`モジュールを82ファイルから18ファイルへ再構成し、sensitivity系コードの削除、論理的なドメイン分離を実現する。

---

## 1. ターゲット構造

```
market/
├── mod.rs                           # 公開API
├── error.rs                         # 統合エラー型
│
├── context/                         # 市場データ管理（5ファイル）
│   ├── mod.rs
│   ├── provider.rs                  # MarketProvider
│   ├── indexed.rs                   # IndexedMarket + IndexCurveMapper
│   ├── requirements.rs              # TradeIndexRequirements
│   └── validator.rs                 # MarketValidator
│
├── curves/                          # 全カーブ（8ファイル）
│   ├── mod.rs
│   ├── traits.rs                    # YieldCurve, FxCurve traits
│   ├── flat.rs                      # FlatCurve
│   ├── interpolated.rs              # InterpolatedCurve
│   ├── credit.rs                    # CreditCurve, HazardRateCurve
│   ├── set.rs                       # CurveSet
│   ├── dispatch.rs                  # CurveEnum
│   ├── fx.rs                        # FX curves (統合)
│   └── bootstrapping.rs             # Bootstrapping (統合)
│
└── surfaces/                        # 全サーフェス（7ファイル）
    ├── mod.rs
    ├── traits.rs                    # VolatilitySurface trait
    ├── flat.rs                      # FlatVol
    ├── interpolated.rs              # InterpolatedVolSurface
    ├── dispatch.rs                  # VolSurfaceEnum
    ├── fx.rs                        # FX vol surfaces (統合)
    └── swaption.rs                  # Swaption vol cube (統合)
```

---

## 2. 統合エラー型設計

### market/error.rs

```rust
use thiserror::Error;

/// 市場モジュール統合エラー型
#[derive(Debug, Error)]
pub enum MarketError {
    #[error("curve error: {0}")]
    Curve(#[from] CurveError),

    #[error("surface error: {0}")]
    Surface(#[from] SurfaceError),

    #[error("context error: {0}")]
    Context(#[from] ContextError),
}

/// カーブエラー
#[derive(Debug, Error)]
pub enum CurveError {
    #[error("interpolation failed at t={time}: {reason}")]
    Interpolation { time: f64, reason: String },

    #[error("bootstrap failed: {0}")]
    Bootstrap(String),

    #[error("curve not found: {0}")]
    NotFound(String),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("fx curve error: {0}")]
    Fx(String),
}

/// サーフェスエラー
#[derive(Debug, Error)]
pub enum SurfaceError {
    #[error("volatility lookup failed: {0}")]
    VolLookup(String),

    #[error("calibration failed: {0}")]
    Calibration(String),

    #[error("invalid strike: {0}")]
    InvalidStrike(String),

    #[error("swaption vol error: {0}")]
    Swaption(String),

    #[error("fx vol error: {0}")]
    FxVol(String),
}

/// コンテキストエラー
#[derive(Debug, Error)]
pub enum ContextError {
    #[error("market data not found: {0}")]
    NotFound(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("provider error: {0}")]
    Provider(String),
}
```

---

## 3. ファイル統合詳細

### 3.1 curves/fx.rs

**統合元:**
- `fx_calibration/curve.rs`
- `fx_calibration/builder.rs`
- `fx_calibration/types.rs`（カーブ関連部分）

**内容:**
```rust
//! FX curves: forward curves and calibrated curves.

// Types
pub struct Strike(pub f64);
pub struct ForwardPoints(pub f64);

// Traits
pub trait FxCurve<T> {
    fn forward(&self, t: T) -> Result<T, CurveError>;
    fn spot(&self) -> T;
}

// Implementations
pub struct SimpleFxCurve<T> { /* ... */ }
pub struct CalibratedFxCurve<T> { /* ... */ }

// Builder
pub struct FxForwardCurveBuilder<T> { /* ... */ }
```

**想定行数:** ~800行

---

### 3.2 curves/bootstrapping.rs

**統合元:**
- `calibration/bootstrapping/config.rs`
- `calibration/bootstrapping/curve_config.rs`
- `calibration/bootstrapping/instrument.rs`
- `calibration/bootstrapping/definition.rs`
- `calibration/bootstrapping/curve.rs`
- `calibration/bootstrapping/curve_builder.rs`
- `calibration/bootstrapping/engine.rs`
- `calibration/bootstrapping/multi_curve.rs`
- `calibration/bootstrapping/cache.rs`
- `calibration/bootstrapping/result_cache.rs`
- `calibration/bootstrapping/date_utils.rs`
- `calibration/bootstrapping/adapter.rs`
- `calibration/bootstrapping/error.rs`
- `calibration/bootstrapping/engine_error.rs`

**削除（pricer_riskへ）:**
- `calibration/bootstrapping/sensitivity.rs`
- `calibration/bootstrapping/adjoint_solver.rs`

**内容:**
```rust
//! Yield curve bootstrapping from market instruments.

// Configuration
pub struct BootstrapConfig { /* ... */ }
pub enum BootstrapInterpolation { Linear, LogLinear, CubicSpline }

// Instruments
pub struct BootstrapInstrument<T> { /* ... */ }
pub struct CurveDefinition { /* ... */ }
pub struct InstrumentSpec { /* ... */ }

// Result curve
pub struct BootstrappedCurve<T> { /* ... */ }
impl<T: Float> YieldCurve<T> for BootstrappedCurve<T> { /* ... */ }

// Bootstrapper
pub struct CurveBootstrapper<T> { /* ... */ }
pub struct MultiCurveBuilder<T> { /* ... */ }
pub struct ParallelCurveSetBuilder<T> { /* ... */ }

// Cache
pub struct BootstrapCache<T> { /* ... */ }

// Date utilities
pub struct DateCalculator { /* ... */ }

// Error (CurveErrorに統合されるが、内部用に保持)
pub(crate) enum BootstrapErrorKind { /* ... */ }
```

**想定行数:** ~1500行

---

### 3.3 surfaces/fx.rs

**統合元:**
- `surfaces/fx.rs`（既存）
- `fx_calibration/surface.rs`
- `fx_calibration/vol_builder.rs`
- `fx_calibration/lazy_surface.rs`
- `fx_calibration/config.rs`
- `fx_density.rs`
- `fx_calibration/types.rs`（Vol関連部分）

**削除（pricer_riskへ）:**
- `fx_calibration/sensitivity.rs`

**内容:**
```rust
//! FX volatility surfaces: simple delta-based and calibrated SABR.

// Types
pub struct Vol(pub f64);
pub enum DeltaType { Spot, Forward }
pub struct FxDeltaPoint { /* ... */ }

// Configuration
pub struct FxVolSurfaceConfig { /* ... */ }

// Implementations
pub struct FxVolatilitySurface { /* simple delta-based */ }
pub struct CalibratedFxVolSurface<T> { /* SABR calibrated */ }
pub struct LazyFxVolSurface<T> { /* lazy evaluation */ }

// Builder
pub struct FxVolSurfaceBuilder<T> { /* ... */ }

// Density calculator
pub struct FxDensityCalculator<T> { /* ... */ }
pub struct DensityStatistics { /* ... */ }

impl<T: Float> VolatilitySurface<T> for FxVolatilitySurface { /* ... */ }
impl<T: Float> VolatilitySurface<T> for CalibratedFxVolSurface<T> { /* ... */ }
```

**想定行数:** ~1200行

---

### 3.4 surfaces/swaption.rs

**統合元:**
- `volcube/cube.rs`
- `volcube/types.rs`
- `volcube/quote.rs`
- `volcube/config.rs`
- `volcube/builder.rs`
- `volcube/calibrator.rs`
- `volcube/engine.rs`
- `volcube/interpolator.rs`
- `volcube/lazy_evaluator.rs`
- `volcube/cache.rs`
- `volcube/breeden_litzenberger.rs`
- `volcube/calibration_graph.rs`
- `volcube/error.rs`
- `surfaces/volcube_slice.rs`

**削除（pricer_riskへ）:**
- `volcube/vega.rs`
- `volcube/sensitivity_path.rs`
- `volcube/aad_validation.rs`

**削除（外部未使用）:**
- `volcube/loader_convert.rs`
- `volcube/graph.rs`（D3.js export）
- `volcube/proptest_tests.rs`（tests/へ移動）

**内容:**
```rust
//! Swaption volatility cube: 3D vol structure with SABR calibration.

// Core types
pub struct SabrParams { pub alpha: f64, pub beta: f64, pub rho: f64, pub nu: f64 }
pub struct VolInstrument { /* ... */ }
pub struct InstrumentId { /* ... */ }

// Quote types
pub struct VolQuote { /* ... */ }
pub struct VolQuoteSet { /* ... */ }
pub enum QuoteType { Normal, LogNormal }

// Configuration
pub struct VolCubeConfig { /* ... */ }
pub enum InterpolationMethod { Flat, Linear, Cubic }
pub enum ExtrapolationMethod { Flat, Linear }

// Core cube
pub struct VolCube<T> { /* 3D: expiry x tenor x strike */ }
pub struct VolCubeSlice<T> { /* 2D slice adapter */ }
pub trait VolatilityCube<T> { /* ... */ }

// Builder & Calibration
pub struct VolCubeBuilder<T> { /* ... */ }
pub struct SabrCalibrator { /* ... */ }
pub struct SviCalibrator { /* ... */ }
pub struct VolCubeCalibrationEngine<T> { /* ... */ }
pub struct CalibrationGraph { /* ... */ }

// Interpolation
pub struct VolCubeInterpolator<T> { /* ... */ }

// Lazy evaluation
pub struct VolLazyEvaluator<T> { /* ... */ }

// Cache
pub struct VolCubeCache<T> { /* ... */ }
pub struct SharedVolCubeCache<T> { /* ... */ }

// Density
pub struct BreedenLitzenberger<T> { /* ... */ }

impl<T: Float> VolatilitySurface<T> for VolCubeSlice<T> { /* ... */ }
```

**想定行数:** ~2500行

---

### 3.5 context/indexed.rs

**統合元:**
- `indexed_market.rs`
- `index_mapper.rs`

**内容:**
```rust
//! Indexed market data access with rate index keying.

// Mapper
pub trait IndexCurveMapper {
    fn curve_name(&self, index: &RateIndex) -> Option<CurveName>;
}
pub struct DefaultIndexCurveMapper { /* ... */ }

// Indexed market
pub struct IndexedMarket<T> { /* ... */ }
pub struct IndexedMarketBuilder<T> { /* ... */ }
```

**想定行数:** ~400行

---

## 4. 削除ファイル一覧

### 4.1 pricer_riskへ移動（sensitivity系）

| ファイル | 行数 | 移動先 |
|---------|------|--------|
| `volcube/vega.rs` | ~500 | `pricer_risk::enzyme` |
| `volcube/sensitivity_path.rs` | ~400 | `pricer_risk::scenarios` |
| `volcube/aad_validation.rs` | ~350 | `pricer_risk::enzyme::verification` |
| `calibration/bootstrapping/sensitivity.rs` | ~450 | `pricer_risk::scenarios` |
| `calibration/bootstrapping/adjoint_solver.rs` | ~300 | `pricer_risk::enzyme` |
| `fx_calibration/sensitivity.rs` | ~250 | `pricer_risk::scenarios` |

### 4.2 削除（Legacy/未使用）

| ファイル | 理由 |
|---------|------|
| `calibration/model_calibrator.rs` | engine.rsに置換済み |
| `calibration/heston.rs` | models/へ移動 |
| `calibration/hull_white.rs` | models/へ移動 |
| `calibration/sabr.rs` | surfaces/swaption.rsに統合 |
| `calibration/swaption_calibrator.rs` | surfaces/swaption.rsに統合 |
| `calibration/engine.rs` | bootstrapping.rsに統合 |
| `calibration/result.rs` | error.rsに統合 |
| `calibration/targets.rs` | 各calibratorに統合 |
| `volcube/loader_convert.rs` | 外部未使用 |
| `volcube/graph.rs` | demo専用、削除 |
| `volcube/proptest_tests.rs` | tests/へ移動 |

### 4.3 統合により消滅

| 旧ファイル | 統合先 |
|-----------|--------|
| `fx_calibration/` 全10ファイル | `curves/fx.rs` + `surfaces/fx.rs` |
| `volcube/` 全21ファイル | `surfaces/swaption.rs` |
| `calibration/bootstrapping/` 全17ファイル | `curves/bootstrapping.rs` |
| `calibration/` 全11ファイル | 各所に分散または削除 |

---

## 5. 公開API設計

### market/mod.rs

```rust
//! Market data structures and calibration.

pub mod context;
pub mod curves;
pub mod error;
pub mod surfaces;

// Prelude-style re-exports
pub use context::{IndexedMarket, IndexedMarketBuilder, MarketProvider, MarketValidator};
pub use curves::{
    BootstrappedCurve, CurveBootstrapper, CurveEnum, CurveSet, FlatCurve,
    FxCurve, InterpolatedCurve, YieldCurve,
};
pub use error::{ContextError, CurveError, MarketError, SurfaceError};
pub use surfaces::{
    FlatVol, FxVolatilitySurface, VolCube, VolCubeBuilder, VolCubeSlice,
    VolSurfaceEnum, VolatilitySurface,
};
```

---

## 6. 移行戦略

### Phase 1: 準備（Day 1）
1. 新ディレクトリ構造を作成
2. `context/`モジュールを作成（既存ファイルを移動）
3. テストが通ることを確認

### Phase 2: curves/統合（Day 2-3）
1. `curves/fx.rs`を作成（fx_calibration/curve系を統合）
2. `curves/bootstrapping.rs`を作成（bootstrapping/を統合）
3. 旧ファイルを削除
4. re-exportを更新
5. テストが通ることを確認

### Phase 3: surfaces/統合（Day 4-5）
1. `surfaces/fx.rs`を作成（fx_calibration/surface系 + fx_density統合）
2. `surfaces/swaption.rs`を作成（volcube/を統合）
3. 旧ファイルを削除
4. re-exportを更新
5. テストが通ることを確認

### Phase 4: クリーンアップ（Day 6）
1. sensitivity系ファイルを削除（またはpricer_riskへ移動）
2. 旧calibration/を削除
3. エラー型を統合
4. ドキュメント更新
5. 全テスト・ビルド確認

### Phase 5: 検証（Day 7）
1. `cargo test --workspace`
2. `cargo clippy --workspace`
3. `cargo doc --workspace`
4. demo/guiの動作確認
5. steering/structure.md更新

---

## 7. リスクと緩和策

| リスク | 緩和策 |
|--------|--------|
| 大規模ファイル統合による可読性低下 | 明確なセクション分け、十分なドキュメント |
| 外部依存コードの破損 | re-exportで後方互換性維持、deprecation警告 |
| テスト漏れ | Phase毎にテスト実行、CI確認 |
| sensitivity削除による機能欠落 | pricer_riskへの移動を別タスクとして追跡 |

---

## 8. 成功基準

- [ ] ファイル数: 82 → 18（78%削減）
- [ ] 全テストパス
- [ ] cargo clippy警告なし
- [ ] cargo doc生成成功
- [ ] demo/gui動作確認
- [ ] steering/structure.md更新完了
