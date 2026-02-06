# Gap Analysis: fx-vol-surface-calibration

## Executive Summary

既存コードベースはFXボラティリティサーフェスカリブレーション実装の**80-95%の基盤**を既に持っている。主なギャップは新規型定義とオーケストレーション層であり、アルゴリズム的な複雑さではない。

### 主要な発見

1. **強固な基盤**: `FxVolatilitySurface`, `FxDensityCalculator`, `SequentialBootstrapper`, `AdjointSolver`が既に存在
2. **ギャップは組織的**: 不足しているのは主にインストルメント型(`FxVolInstrument`, `CrossCurrencyBasisSwap`)とビルダー層
3. **高い再利用性**: 既存コンポーネントの直接再利用で推定40%のコード削減が可能
4. **明確な統合ポイント**: `BootstrapInstrument` enumの拡張、FXアダプター追加、ビルダーラッパー作成

---

## 1. Requirement-to-Asset Map

### Requirement 1-4: FX Vol Instruments & Surface

| 要件 | 既存アセット | ギャップ | ステータス |
|------|-------------|---------|-----------|
| `FxVolInstrument` enum | `FxQuoteEntry` (webapp層のみ) | infra層への型定義移行、ATM/BF/RR variants | **Missing** |
| `FxVolConvention` | 部分的 (`DeltaType` enum存在) | premium currency, cut-off, calendar | **Missing** |
| `FxVolSurfaceConfig` | なし | interpolator type, extrapolation method | **Missing** |
| `CalibratedFxVolSurface` | `FxVolatilitySurface<T>` (delta空間のみ) | strike空間への拡張、SABR/SVI補間 | **Adapt** |
| Vol interpolation | `BilinearInterpolator`, `SabrParameterSurface` | strike次元統合 | **Adapt** |
| Delta-Strike変換 | `FxDensityCalculator` (完全実装) | surface統合 | **Direct** |

### Requirement 5-6: Lazy Evaluation & AAD

| 要件 | 既存アセット | ギャップ | ステータス |
|------|-------------|---------|-----------|
| `LazyFxVolSurface` | `VolCubeCache`, `CurveResultCache` | 遅延評価ラッパー | **New** |
| Cache invalidation | なし (静的キャッシュのみ) | クォート変更時の自動無効化 | **New** |
| `CacheStats` | `VolCubeCache::CacheStats` | 再利用可能 | **Direct** |
| AAD compatibility | Generic `<T: Float>` 全面適用 | 既に対応済み | **Direct** |
| `Differentiable` trait | 暗黙的 (Float制約のみ) | 明示的trait定義 | **New** |
| `VolSurfaceSensitivity` | `SolveResultWithSensitivities` | Vol用拡張 | **Adapt** |
| Computation graph JSON | なし | D3.js export | **New** |

### Requirement 7-10: FX Curve Infrastructure

| 要件 | 既存アセット | ギャップ | ステータス |
|------|-------------|---------|-----------|
| `FxSwapInstrument` | `FxSwap` (基本的) | tenor field, implied forward rate | **Adapt** |
| `FxSwapConvention` | なし | spot lag, settlement calendar | **New** |
| `CrossCurrencyBasisSwap` | なし | 完全新規 | **New** |
| `XccyBasisConvention` | なし | notional exchange, MTM flag | **New** |
| `FxCurve<T>` trait | `YieldCurve<T>` trait (参考) | forward_rate, forward_points | **New** |
| `FxForwardCurveBuilder` | `SequentialBootstrapper<T>` | FX用ラッパー | **Adapt** |
| Tenor blending | なし | 1Y-2Y transition interpolation | **New** |

### Requirement 11: FxMarketBuilder Orchestration

| 要件 | 既存アセット | ギャップ | ステータス |
|------|-------------|---------|-----------|
| `FxMarketBuilder` | `CurveEngine` (OIS用) | FX用オーケストレーター | **Adapt** |
| Dependency ordering | `MultiCurveBuilder<T>` | FX依存チェーン | **Adapt** |
| Partial build methods | なし | step-by-step構築 | **New** |
| `FxMarket` result | `CurveSet` | FX専用結果構造体 | **New** |

### Requirement 12: WebApp Integration

| 要件 | 既存アセット | ギャップ | ステータス |
|------|-------------|---------|-----------|
| `/api/fxcurve/build` | なし | 新規エンドポイント | **New** |
| `/api/fxvol/calibrate` | `fxvol_handlers.rs` (スケルトン) | 実装完了 | **Adapt** |
| `/api/fxvol/smile` | スケルトン存在 | 実装完了 | **Adapt** |
| Diagnostics output | なし | iterations, residual, convergence | **New** |
| WebSocket real-time | なし | 新規インフラ | **New** |
| Quote editing UI | なし | フロントエンド作業 | **New** |

### Requirement 13-14: Cleanup & Type Safety

| 要件 | 既存アセット | ギャップ | ステータス |
|------|-------------|---------|-----------|
| Deprecated code removal | `FxVolatilitySurface` | 新API移行後削除 | **Constraint** |
| Newtype pattern | `Currency`, `Tenor` | `Delta`, `Strike`, `Vol` 追加 | **New** |
| `FxCalibrationError` | `CalibrationError`, `BootstrapError` | FX統合エラー型 | **New** |
| thiserror integration | 全面適用済み | 再利用可能 | **Direct** |

---

## 2. Implementation Approach Options

### Option A: Extend Existing Components (推奨度: ★★★☆☆)

**対象**: 既存の`FxVolatilitySurface`を直接拡張

**変更ファイル**:
- `crates/pricer_models/src/market/surfaces/fx.rs` (拡張)
- `crates/infra_domain/src/trade/instrument_def/fx.rs` (拡張)

**トレードオフ**:
- ✅ ファイル数最小化
- ✅ 既存テスト活用
- ❌ 既存APIの破壊的変更リスク
- ❌ 単一ファイル肥大化 (fx.rs: 761行 → 1500行超)

### Option B: Create New Components (推奨度: ★★★★☆)

**対象**: 新規モジュールとして分離

**新規ファイル**:
```
crates/pricer_models/src/market/
├── fx_calibration/           # 新規モジュール
│   ├── mod.rs
│   ├── instruments.rs        # FxVolInstrument, FxSwapInstrument
│   ├── conventions.rs        # FxVolConvention, FxSwapConvention
│   ├── config.rs             # FxVolSurfaceConfig
│   ├── builder.rs            # FxVolSurfaceBuilder, FxForwardCurveBuilder
│   ├── surface.rs            # CalibratedFxVolSurface
│   ├── curve.rs              # CalibratedFxCurve, FxCurve trait
│   ├── lazy.rs               # LazyFxVolSurface
│   ├── sensitivity.rs        # VolSurfaceSensitivity
│   └── error.rs              # FxCalibrationError

crates/infra_domain/src/trade/instrument_def/
├── fx_vol.rs                 # FxVolInstrument (infra層)
└── xccy.rs                   # CrossCurrencyBasisSwap
```

**トレードオフ**:
- ✅ 明確な責務分離
- ✅ テスト容易性
- ✅ 既存コード影響最小
- ❌ ファイル数増加
- ❌ インターフェース設計必要

### Option C: Hybrid Approach (推奨度: ★★★★★)

**対象**: 段階的実装 + 既存活用

**Phase 1 (Core Types)**:
- `infra_domain`に新規インストルメント型追加
- 既存`FxSwap`は保持、新規`FxSwapInstrument`追加

**Phase 2 (Builders)**:
- `FxForwardCurveBuilder` 新規作成 (`SequentialBootstrapper`ラップ)
- `FxVolSurfaceBuilder` 新規作成 (既存`FxVolatilitySurface`活用)

**Phase 3 (Integration)**:
- `FxMarketBuilder`オーケストレーター
- 既存`CurveEngine`パターン踏襲

**Phase 4 (Cleanup)**:
- 重複コード削除
- API統一

**トレードオフ**:
- ✅ リスク分散
- ✅ 早期価値提供
- ✅ 既存機能との共存
- ❌ 一時的な重複
- ❌ 複数フェーズ管理

---

## 3. Effort & Risk Assessment

### Effort Estimate

| コンポーネント | 工数 | 根拠 |
|---------------|------|------|
| FX Vol Instruments (Req 1) | **S** (1-2日) | 型定義のみ、既存パターン踏襲 |
| Vol Config (Req 2) | **S** (1日) | 設定構造体、バリデーション |
| FxVolSurfaceBuilder (Req 3) | **M** (3-5日) | 既存surface + calibration統合 |
| CalibratedFxVolSurface (Req 4) | **M** (3-5日) | SABR/SVI補間統合 |
| Lazy Evaluation (Req 5) | **S** (2日) | キャッシュパターン既存 |
| AAD Support (Req 6) | **M** (3-4日) | 既存Float generics活用 |
| FxSwapInstrument (Req 7) | **S** (1日) | 既存FxSwap拡張 |
| CrossCurrencyBasisSwap (Req 8) | **M** (2-3日) | 新規インストルメント |
| FxCurve Trait (Req 9) | **S** (1-2日) | YieldCurve参考 |
| FxForwardCurveBuilder (Req 10) | **M** (4-5日) | Bootstrapper統合 + blending |
| FxMarketBuilder (Req 11) | **M** (3-4日) | CurveEngine参考 |
| WebApp Handlers (Req 12) | **M** (3-4日) | 既存パターン踏襲 |
| Cleanup (Req 13) | **S** (2日) | 依存関係更新 |
| Type Safety (Req 14) | **S** (1日) | 横断的適用 |

**Total: L-XL (2-3週間)**

### Risk Assessment

| リスク | レベル | 緩和策 |
|--------|--------|--------|
| Tenor blending (1Y-2Y transition) | **Medium** | スムーズ補間設計、テスト強化 |
| Cache invalidation coherency | **Medium** | 単純なinvalidation first、最適化後 |
| AD through bootstrap solver | **Low** | 既存AdjointSolver実績あり |
| Breaking existing API | **Low** | 新規モジュールで分離 |
| WebSocket complexity | **Medium** | Phase 2以降に延期可能 |
| SABR calibration edge cases | **Medium** | 既存volcubeテスト活用 |

---

## 4. Recommendations for Design Phase

### 推奨アプローチ: Option C (Hybrid)

1. **Phase 1から開始**: `infra_domain`にインストルメント型追加（最小リスク、他作業のブロック解除）

2. **SequentialBootstrapper直接活用**: FXカーブ構築に既存ブートストラッパーを90%再利用

3. **FxVolatilitySurfaceの段階的拡張**:
   - 最初: delta空間のみ（既存機能）
   - 次: strike空間追加（SABR統合）

4. **既存WebAppパターン踏襲**: `volcube_handlers`からコピー＆適応

5. **AAD/Differentiable traitはMVP後**: コア機能はFloat genericsで既に動作

### Design Phase Research Items

1. **Tenor Blending Algorithm**: 1Y-2Y transition point での補間方法（線形 vs スプライン）
2. **XCCY Swap Pricing**: MTM vs Non-MTM の評価ロジック詳細
3. **Cache Invalidation Semantics**: どの粒度で無効化するか（expiry単位 vs 全体）
4. **WebSocket Protocol**: リアルタイム更新のメッセージフォーマット

---

## 5. File Location Summary

### 既存ファイル（再利用/拡張）

| ファイル | 用途 |
|---------|------|
| `pricer_models/src/market/surfaces/fx.rs` | FxVolatilitySurface基盤 |
| `pricer_models/src/market/fx_density.rs` | Delta-Strike変換 |
| `pricer_models/src/market/volcube/` | SABR calibration参考 |
| `pricer_models/src/market/calibration/bootstrapping/` | Bootstrap engine |
| `infra_domain/src/trade/instrument_def/fx.rs` | FxSwap, FxForward |
| `demo/gui/src/web/fxvol_handlers.rs` | Handler skeleton |
| `demo/gui/src/web/fxvol_types.rs` | Quote types |

### 新規ファイル（作成予定）

| ファイル | 用途 |
|---------|------|
| `pricer_models/src/market/fx_calibration/mod.rs` | FXカリブレーションモジュール |
| `infra_domain/src/trade/instrument_def/fx_vol.rs` | FxVolInstrument |
| `infra_domain/src/trade/instrument_def/xccy.rs` | CrossCurrencyBasisSwap |
| `demo/gui/src/web/fxcurve_handlers.rs` | FX curve endpoints |
| `demo/gui/src/web/fxcurve_types.rs` | FX curve types |
