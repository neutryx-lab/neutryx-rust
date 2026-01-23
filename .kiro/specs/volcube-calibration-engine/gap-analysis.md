# Gap Analysis: volcube-calibration-engine

## 概要

本分析では、VolatilityCubeカリブレーションエンジンの要件と既存コードベースのギャップを調査し、実装戦略の選択肢を提示する。

---

## 1. 現状調査

### 1.1 関連モジュール構造

```text
pricer_models/src/market/
├── surfaces/
│   ├── traits.rs          → VolatilitySurface<T> trait (2D: strike, expiry)
│   ├── vol_surface_enum.rs → VolSurfaceEnum (static dispatch)
│   ├── interpolated.rs    → InterpolatedVolSurface (grid-based)
│   ├── flat.rs            → FlatVol
│   └── fx.rs              → FxVolatilitySurface (delta-based)
├── calibration/
│   ├── engine.rs          → CalibrationEngine (Levenberg-Marquardt)
│   ├── sabr.rs            → SABRCalibrator (Hagan formula)
│   ├── heston.rs          → HestonCalibrator
│   ├── hull_white.rs      → HullWhiteCalibrator
│   └── bootstrapping/
│       ├── result_cache.rs → CurveResultCache<T> (LRU)
│       ├── cache.rs        → BootstrapCache<T>
│       └── curve.rs        → BootstrappedCurve<T>, BootstrappedCurveBuilder<T>
├── curves/                → YieldCurve trait, implementations
└── provider.rs            → MarketProvider (Arc-cached)

pricer_core/src/math/interpolators/
├── svi.rs                 → SviParams<T>, svi_implied_vol()
├── bilinear.rs            → BilinearInterpolator
├── cubic_spline.rs        → CubicSplineInterpolator
├── monotonic.rs           → MonotonicInterpolator
└── traits.rs              → Interpolator trait

pricer_pricing/src/graph/
├── extractor.rs           → GraphExtractable trait
└── types.rs               → ComputationGraph, GraphNode, GraphEdge
```

### 1.2 再利用可能コンポーネント

| コンポーネント | 場所 | 再利用可能度 |
|---------------|------|-------------|
| `VolatilitySurface<T>` trait | surfaces/traits.rs | ✅ 拡張可能 |
| `VolSurfaceEnum` | surfaces/vol_surface_enum.rs | ✅ 新variant追加 |
| `CalibrationEngine` | calibration/engine.rs | ✅ 直接利用 |
| `SABRCalibrator` | calibration/sabr.rs | ✅ 直接利用 |
| `SviParams`, `svi_implied_vol` | pricer_core/svi.rs | ✅ 直接利用 |
| `CurveResultCache<T>` | bootstrapping/result_cache.rs | ✅ パターン参考 |
| `CurveKey` | bootstrapping/result_cache.rs | ✅ パターン参考 |
| `GraphExtractable` trait | pricer_pricing/graph | ✅ 実装可能 |
| `BilinearInterpolator` | pricer_core/interpolators | ✅ 2D補間用 |
| `BootstrappedCurveBuilder<T>` | bootstrapping/curve.rs | ✅ Builderパターン参考 |

### 1.3 アーキテクチャパターン

- **AD互換ジェネリクス**: `T: Float` で全数値型をパラメータ化
- **Static Dispatch**: enumによるtrait object回避（Enzyme最適化）
- **Builder Pattern**: fluent APIで設定を構築
- **LRU Cache**: `parking_lot::RwLock` + `lru::LruCache` でスレッドセーフ
- **Hash-based Key**: `OrderedFloat` で浮動小数点のhash化
- **thiserror**: 構造化エラーハンドリング

---

## 2. 要件と既存資産のマッピング

### Requirement 1: VolCubeBuilder コア構築

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 1.1 Instrumentリスト→VolCube | `CalibrationEngine` | ✅ 利用可能 |
| 1.2 Interpolator指定 | `SviParams`, `SABRCalibrator` | ✅ 利用可能 |
| 1.3 Builder pattern | `BootstrappedCurveBuilder<T>` | ✅ パターン参考 |
| 1.4 空リストエラー | `CalibrationError` | ✅ 利用可能 |
| 1.5 3次元軸(Expiry, Tenor, Strike) | なし | ❌ **新規実装** |

**ギャップ**: 3次元構造（Swaption tenor軸追加）は新規設計が必要

### Requirement 2: VolCube インターフェース

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 2.1 `get_vol(expiry, tenor, strike)` | `VolatilitySurface::volatility(strike, expiry)` | ❌ **tenor軸追加** |
| 2.2 Extrapolation | `InterpolatedVolSurface::allow_extrapolation` | ✅ パターン参考 |
| 2.3 `T: Float` AD互換 | 全既存コンポーネント | ✅ 一貫 |
| 2.4 `Send + Sync` | `CurveResultCache<T>` | ✅ パターン参考 |
| 2.5 domain範囲メソッド | `strike_domain()`, `expiry_domain()` | ✅ 拡張可能 |

**ギャップ**: 2Dから3Dへの次元拡張、新トレイト `VolatilityCube<T>` が必要

### Requirement 3: 確率密度関数

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 3.1 Breeden-Litzenberger PDF | なし | ❌ **新規実装** |
| 3.2 累積確率分布 | なし | ❌ **新規実装** |
| 3.3 smooth approximation | `pricer_core::math::smoothing` | ✅ 利用可能 |
| 3.4 範囲外エラー | `MarketDataError::OutOfBounds` | ✅ 利用可能 |

**ギャップ**: Breeden-Litzenberger公式実装が必要（数値微分 d²C/dK²）

### Requirement 4: 計算グラフ接続

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 4.1 ソースInstrument参照 | なし（Curveは持たない） | ❌ **設計検討** |
| 4.2 DAG依存関係記録 | `GraphExtractable` trait | ✅ 実装可能 |
| 4.3 D3.js互換出力 | `ComputationGraph`, `GraphNode` | ✅ 利用可能 |
| 4.4 AAD感度計算 | pricer_pricing enzyme | ✅ 統合可能 |

**ギャップ**: VolCubeとInstrument間の依存関係追跡メカニズム設計が必要

### Requirement 5: キャッシュと再カリブレーション回避

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 5.1 ハッシュキー生成 | `CurveKey::hash_rates()` | ✅ パターン参考 |
| 5.2 キャッシュヒット時返却 | `CurveResultCache::lookup()` | ✅ パターン参考 |
| 5.3 市場データ更新時無効化 | なし（明示的clear） | ⚠️ **設計検討** |
| 5.4 LRUキャッシュ | `lru::LruCache` | ✅ 利用可能 |
| 5.5 メトリクス公開 | `CacheStats` | ✅ パターン参考 |

**ギャップ**: 自動無効化メカニズム（market data timestamp比較）の設計が必要

### Requirement 6: カリブレーション設定

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 6.1 Interpolation方式 | `BootstrapInterpolation` enum | ✅ 拡張可能 |
| 6.2 Extrapolation方式 | `allow_extrapolation: bool` | ✅ enum化推奨 |
| 6.3 Strike軸表現 | なし | ❌ **新規実装** |
| 6.4 最適化アルゴリズム | `CalibrationEngine` (L-M) | ✅ Nelder-Mead追加可能 |
| 6.5 Builder + Default | `GenericBootstrapConfig` | ✅ パターン参考 |

**ギャップ**: Strike軸表現（Absolute, Moneyness, LogMoneyness, Delta）のenum定義が必要

### Requirement 7-10: エラー、アーキテクチャ、テスト、拡張性

| 要件 | 既存資産 | ギャップ |
|-----|---------|---------|
| エラーハンドリング | `CalibrationError`, `MarketDataError` | ✅ 拡張可能 |
| A-I-P-S準拠 | 既存構造 | ✅ 配置ルール明確 |
| Arbitrage-free検証 | なし | ❌ **新規実装** |
| proptest | 既存テスト | ✅ パターン参考 |
| enum static dispatch | `VolSurfaceEnum` | ✅ パターン参考 |

---

## 3. 実装アプローチ選択肢

### Option A: 既存コンポーネント拡張

**戦略**: `VolatilitySurface<T>` を3D対応に拡張、`VolSurfaceEnum` に `Cube` variant追加

**対象ファイル**:
- `surfaces/traits.rs`: `VolatilityCube<T>` trait追加
- `surfaces/vol_surface_enum.rs`: `Cube(VolCubeImpl<T>)` variant追加
- `surfaces/mod.rs`: 新モジュール公開

**利点**:
- ✅ 最小限のファイル追加
- ✅ 既存`VolatilitySurface`との互換性維持
- ✅ `VolSurfaceEnum` の既存消費者への影響最小

**欠点**:
- ❌ 2Dと3Dの混在でtrait設計が複雑化
- ❌ tenor軸オプショナル化が必要

### Option B: 新規コンポーネント作成

**戦略**: `volcube/` サブモジュールを新設、独立したtrait体系を構築

**新規ファイル**:
```text
pricer_models/src/market/
└── volcube/
    ├── mod.rs              → モジュール公開
    ├── traits.rs           → VolatilityCube<T> trait
    ├── cube.rs             → VolCube<T> 実装
    ├── builder.rs          → VolCubeBuilder<T>
    ├── config.rs           → VolCubeConfig, InterpolationMethod, StrikeAxis
    ├── cache.rs            → VolCubeCache<T>, VolCubeKey
    ├── density.rs          → BreedenLitzenberger, probability_density()
    └── enum.rs             → VolCubeEnum<T>
```

**利点**:
- ✅ 明確な責任分離
- ✅ 2D/3Dの設計独立
- ✅ テスト容易性

**欠点**:
- ❌ ファイル数増加
- ❌ 既存コードとの統合ポイント設計が必要

### Option C: ハイブリッドアプローチ（推奨）

**戦略**: 新規`volcube/`モジュール作成 + 既存calibration/cacheパターン再利用

**構成**:
- **新規作成**: `volcube/` モジュール（traits, cube, builder, density）
- **再利用**: `CalibrationEngine`, `SviParams`, `SABRCalibrator`
- **パターン適用**: `CurveResultCache<T>` → `VolCubeCache<T>`
- **拡張**: `VolSurfaceEnum` に `Cube` variant追加（互換性用）

**Phase 1 (MVP)**:
1. `VolatilityCube<T>` trait定義
2. `VolCubeBuilder<T>` 基本実装
3. SABR/SVI per-expiry calibration
4. 3D補間（Bilinear expiry-tenor + 1D strike）

**Phase 2 (拡張)**:
1. Breeden-Litzenberger PDF
2. `VolCubeCache<T>` LRUキャッシュ
3. `GraphExtractable` 実装
4. Arbitrage-free検証

---

## 4. 複雑性とリスク評価

### 工数見積もり

| タスク | 見積もり | 根拠 |
|--------|---------|------|
| `VolatilityCube<T>` trait + 実装 | **M** (3-7日) | 3D補間は2D拡張、SABRカリブレータ再利用 |
| `VolCubeBuilder<T>` | **S** (1-3日) | `BootstrappedCurveBuilder` パターン踏襲 |
| Breeden-Litzenberger PDF | **M** (3-7日) | 数値微分実装、数値安定性検証 |
| `VolCubeCache<T>` | **S** (1-3日) | `CurveResultCache` 完全踏襲 |
| `GraphExtractable` 実装 | **S** (1-3日) | 既存パターン適用 |
| Arbitrage-free検証 | **M** (3-7日) | Butterfly/Calendar spread検証ロジック |

**総合見積もり**: **L** (1-2週間)

### リスク評価

| リスク | レベル | 緩和策 |
|--------|--------|--------|
| 3D補間精度 | Medium | proptest + 既知パラメータ再現テスト |
| AAD互換性 | Low | 既存`T: Float`パターン一貫適用 |
| キャッシュ無効化 | Medium | timestamp比較 + 明示的invalidation API |
| Arbitrage-free検証 | Medium | 既存文献参照、段階的実装 |

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option C (ハイブリッド)** を推奨。理由:
1. 既存calibrationインフラの再利用でリスク低減
2. 独立モジュールで責任分離明確
3. 段階的実装（MVP→拡張）が可能

### 設計フェーズで検討すべき項目

1. **3D補間戦略**: expiry-tenor平面のBilinear + strike軸SABR/SVI vs 完全3D補間
2. **Strike軸表現**: `StrikeAxis` enum設計（Absolute, Moneyness, LogMoneyness, Delta）
3. **キャッシュ無効化**: market data timestamp vs explicit invalidation API
4. **Instrument参照**: `InstrumentId` vs `Arc<Instrument>` vs trait object
5. **Arbitrage-free検証**: 構築時検証 vs lazy検証 vs 両方

### Research Needed

- **Breeden-Litzenberger数値実装**: 二次微分の数値安定性確保方法
- **SABR per-expiry-tenor calibration**: 複数スライスの同時カリブレーション戦略
- **Arbitrage-free条件**: Swaption cube固有の条件（rate vs equity）

---

_Generated: 2026-01-23_
_Document patterns and gaps, not exhaustive file listings_
