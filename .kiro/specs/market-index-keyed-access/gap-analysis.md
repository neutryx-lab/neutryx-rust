# Gap Analysis: Market Index-Keyed Access

## 1. 現状調査

### 1.1 既存アセットの調査

#### Index型定義（infra_domain）

| ファイル | 状態 | 説明 |
|---------|------|------|
| `infra_domain::market::rate_index.rs` | Complete | `RateIndex` enum with Hash/Eq |
| `infra_domain::trade::index.rs` | Complete | `IndexType` enum |
| `infra_domain::trade::instrument_def::fx.rs` | Complete | `CurrencyPair` struct with Hash/Eq |
| VolatilityIndex型 | Missing | Volatility Surface/Cube用のIndex型が未定義 |

#### Curve関連（pricer_models::market）

| ファイル | 状態 | 説明 |
|---------|------|------|
| `curves/curve_set.rs` | Complete | `CurveSet<T>` - HashMap<CurveName, CurveEnum<T>>、`get_curve_for_index(RateIndex)` メソッド有 |
| `index_mapper.rs` | Complete | `IndexCurveMapper` trait + `DefaultIndexCurveMapper` |
| `fx_calibration/curve.rs` | Complete | `FxCurve<T>` trait + `CalibratedFxCurve`, `SimpleFxCurve` |
| `provider.rs` | Complete | `MarketProvider` - Currency-keyed cache |

#### VolCube/VolSurface関連

| ファイル | 状態 | 説明 |
|---------|------|------|
| `volcube/` | Complete | `VolCube`, `VolLazyEvaluator`, `VolCubeProviderKey` |
| `surfaces/` | Complete | `VolSurfaceEnum`, `FxVolatilitySurface` |
| Index-keyed VolCube access | Partial | VolCubeProviderKeyでCurrency + UnderlyingIndex使用 |

### 1.2 アーキテクチャパターン

**現状のデータフロー:**
```
RateIndex → IndexCurveMapper → CurveName → CurveSet → CurveEnum
```

**現状の問題点:**
1. CurveSetは`CurveName`（enum）でキー化、`RateIndex`は間接参照
2. FxCurveは`CurrencyPair`でキー化されるべきだが、MarketProviderは`Currency`でキー化
3. VolCubeは`VolCubeProviderKey`(Currency + UnderlyingIndex)を使用
4. 統一Market構造体が存在しない

### 1.3 統合ポイント

| 統合ポイント | 現状 | 必要な変更 |
|-------------|------|-----------|
| Curve → Index | CurveName経由の間接参照 | HashMap<RateIndex, Arc<CurveEnum>> |
| FxCurve → CurrencyPair | Currency単位のキャッシュ | HashMap<CurrencyPair, Arc<FxCurve>> |
| VolCube → Index | VolCubeProviderKey | 統一MarketからのIndex-keyedアクセス |
| Trade → required_indices | Missing | Trade trait拡張 |

## 2. 要件実現可能性分析

### Requirement 1: Index型定義の標準化

| 技術要件 | ギャップ | 難易度 |
|---------|--------|--------|
| RateIndex as primary key | Partial - CurveName経由 | Low |
| CurrencyPair for FxCurve | Partial - Currency使用中 | Low |
| VolatilityIndex定義 | Missing | Medium |
| IndexNotFoundエラー | Partial | Low |

### Requirement 2-6: API実装

All requirements are technically feasible. Existing infrastructure supports the facade pattern with minimal modifications.

### Requirement 7: 網羅性検証機能

| 技術要件 | ギャップ | 難易度 |
|---------|--------|--------|
| validate_completeness() | Missing | Medium |
| Trade.required_indices() | Missing | High - 全Trade型拡張 |
| Portfolio.required_indices() | Missing | Medium |
| MissingIndexエラー | Missing | Low |

## 3. 実装アプローチ選択肢

### Option C: ハイブリッドアプローチ（推奨）

**戦略:**
1. **Phase 1:** `IndexedMarket`ファサードを作成（既存コンポーネントをラップ）
2. **Phase 2:** 徐々に内部実装を最適化
3. **Phase 3:** 非推奨APIの段階的削除

**新規ファイル:**
- `pricer_models::market::indexed_market.rs` - ファサード
- `infra_domain::market::volatility_index.rs` - VolatilityIndex型
- `infra_domain::trade::index_requirement.rs` - IndexRequirement型

**既存ファイル変更:**
- `pricer_models::market::mod.rs` - re-export追加
- `pricer_models::market::error.rs` - エラー型追加

## 4. 複雑性・リスク評価

### Effort: M (3-7 days)
### Risk: Medium

**リスク要因:**
1. Trade構造体への`required_indices()`追加の影響範囲
2. VolCube既存キャッシュとの互換性
3. パフォーマンス影響（HashMap lookup追加）

**緩和策:**
- `required_indices()`はtrait extension patternで実装
- VolCubeは既存ProviderKeyとの互換レイヤー提供
- Benchmarkで性能検証

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ: Option C（ハイブリッド）

### 主要設計決定

1. **IndexedMarket<T>構造体の設計**
   - 内部HashMap: `RateIndex → Arc<CurveEnum<T>>`, `CurrencyPair → Arc<FxCurve<T>>`
   - MarketProvider/CurveSetの既存機能をラップ
   - 統一API: `get_df()`, `get_forward_rate()`, `get_bs_vol()`, `get_fx_vol()`

2. **VolatilityIndex型の設計**
   - RateIndex（Swaption用）+ SwaptionKey（expiry/tenor）または汎用`VolatilityIndexType` enum

3. **Trade.required_indices()の実装方式**
   - trait extension pattern使用
   - `IndexRequirement`型（RateIndex | CurrencyPair | VolatilityIndex）
