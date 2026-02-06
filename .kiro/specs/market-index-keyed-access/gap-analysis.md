# Gap Analysis: Market Index-Keyed Access

## 1. 現状調査

### 1.1 既存アセットの調査

#### Index型定義（infra_domain）

| ファイル | 状態 | 説明 |
|---------|------|------|
| `infra_domain::market::rate_index.rs` | ✅ 存在 | `RateIndex` enum (SOFR, TONAR, ESTR, EURIBOR3M, EURIBOR6M, SONIA, SARON) with Hash/Eq |
| `infra_domain::trade::index.rs` | ✅ 存在 | `IndexType` enum (Rate, SwapRate, Fx, Equity, Inflation, Commodity) |
| `infra_domain::trade::instrument_def::fx.rs` | ✅ 存在 | `CurrencyPair` struct (base, quote) with Hash/Eq |
| VolatilityIndex型 | ❌ 不在 | Volatility Surface/Cube用のIndex型が未定義 |

#### Curve関連（pricer_models::market）

| ファイル | 状態 | 説明 |
|---------|------|------|
| `curves/curve_set.rs` | ✅ 存在 | `CurveSet<T>` - HashMap<CurveName, CurveEnum<T>>、`get_curve_for_index(RateIndex)` メソッド有 |
| `index_mapper.rs` | ✅ 存在 | `IndexCurveMapper` trait + `DefaultIndexCurveMapper` (RateIndex → CurveName) |
| `fx_calibration/curve.rs` | ✅ 存在 | `FxCurve<T>` trait + `CalibratedFxCurve`, `SimpleFxCurve` |
| `provider.rs` | ✅ 存在 | `MarketProvider` - Currency-keyed cache (not CurrencyPair) |

#### VolCube/VolSurface関連

| ファイル | 状態 | 説明 |
|---------|------|------|
| `volcube/` | ✅ 存在 | `VolCube`, `VolLazyEvaluator`, `VolCubeProviderKey` (Currency + UnderlyingIndex) |
| `surfaces/` | ✅ 存在 | `VolSurfaceEnum`, `FxVolatilitySurface` |
| Index-keyed VolCube access | ⚠️ 部分的 | VolCubeProviderKeyでCurrency + UnderlyingIndex使用、RateIndex直接キー化なし |

### 1.2 アーキテクチャパターン

**現状のデータフロー:**
```
RateIndex → IndexCurveMapper → CurveName → CurveSet → CurveEnum
                                                      ↓
                                               discount_factor(t)
```

**現状の問題点:**
1. CurveSetは`CurveName`（enum）でキー化、`RateIndex`は間接参照
2. FxCurveは`CurrencyPair`でキー化されるべきだが、MarketProviderは`Currency`でキー化
3. VolCubeは`VolCubeProviderKey`(Currency + UnderlyingIndex)を使用、統一されていない
4. 統一Market構造体が存在しない

### 1.3 統合ポイント

| 統合ポイント | 現状 | 必要な変更 |
|-------------|------|-----------|
| Curve → Index | CurveName経由の間接参照 | HashMap<RateIndex, Arc<CurveEnum>> |
| FxCurve → CurrencyPair | Currency単位のキャッシュ | HashMap<CurrencyPair, Arc<FxCurve>> |
| VolCube → Index | VolCubeProviderKey | 統一MarketからのIndex-keyedアクセス |
| Trade → required_indices | ❌ 未実装 | Trade trait拡張 |

---

## 2. 要件実現可能性分析

### Requirement 1: Index型定義の標準化

| 技術要件 | ギャップ | 難易度 |
|---------|--------|--------|
| RateIndex as primary key | ⚠️ CurveName経由 | 低 - 直接キー化可能 |
| CurrencyPair for FxCurve | ⚠️ Currency使用中 | 低 - 型変更のみ |
| VolatilityIndex定義 | ❌ Missing | 中 - 新型定義必要 |
| IndexNotFoundエラー | ⚠️ 部分的 | 低 - エラー型追加 |

**Research Needed:** VolatilityIndex型の設計（RateIndex vs SwaptionKey vs 汎用Index）

### Requirement 2: Curve Index-Keyed Access API

| 技術要件 | ギャップ | 難易度 |
|---------|--------|--------|
| `get_df(index, term)` | ⚠️ 間接API存在 | 低 - ラッパー追加 |
| `get_forward_rate(index, start, end)` | ✅ forward_rate_for_index存在 | なし |
| OIS/IBOR multi-curve | ✅ CurveSet対応済 | なし |
| CurveBuilder Index紐付け | ⚠️ 部分的 | 中 - Builder拡張 |

**既存コード活用:** `CurveSet::get_curve_for_index()`, `forward_rate_for_index()` を拡張

### Requirement 3: VolCube/VolSurface Index-Keyed Access API

| 技術要件 | ギャップ | 難易度 |
|---------|--------|--------|
| `get_bs_vol(index, expiry, strike)` | ❌ Missing | 中 - 新API |
| `get_swaption_vol(index, expiry, tenor, strike)` | ⚠️ VolCube経由 | 中 - 統合API |
| `get_fx_vol(pair, expiry, strike)` | ⚠️ 分散実装 | 中 - 統合API |
| VolCubeBuilder Index紐付け | ⚠️ VolCubeProviderKey使用 | 中 |

**Constraint:** VolCubeの既存キャッシュメカニズムとの互換性維持

### Requirement 4: IndexCurveMapper統合

| 技術要件 | ギャップ | 難易度 |
|---------|--------|--------|
| RateIndex → YieldCurve | ✅ 存在 | なし |
| RateIndex → VolCube | ⚠️ 部分的 | 低 |
| CurrencyPair → FxCurve | ❌ Missing | 中 |
| CurrencyPair → FxVolSurface | ❌ Missing | 中 |

**既存コード活用:** `DefaultIndexCurveMapper` を拡張して`IndexMarketMapper`に統合

### Requirement 5: Market構造体のIndex-Keyed設計

| 技術要件 | ギャップ | 難易度 |
|---------|--------|--------|
| HashMap<RateIndex, Arc<YieldCurve>> | ❌ CurveName使用 | 中 - 構造変更 |
| HashMap<RateIndex, Arc<VolCube>> | ❌ VolCubeProviderKey | 中 |
| HashMap<CurrencyPair, Arc<FxCurve>> | ❌ Currency使用 | 中 |
| Thread-safe immutable | ✅ Arc + RwLock | なし |

**Research Needed:** 既存MarketProvider拡張 vs 新Market構造体作成

### Requirement 6: Builder APIのIndex対応

| 技術要件 | ギャップ | 難易度 |
|---------|--------|--------|
| CurveBuilder.for_index() | ⚠️ 部分的 | 低 |
| VolCubeBuilder.for_index() | ⚠️ 部分的 | 低 |
| FxCurveBuilder.for_pair() | ✅ 存在 | なし |
| MarketBuilder集約 | ❌ Missing | 中 |

### Requirement 7: 網羅性検証機能

| 技術要件 | ギャップ | 難易度 |
|---------|--------|--------|
| validate_completeness() | ❌ Missing | 中 |
| Trade.required_indices() | ❌ Missing | 高 - 全Trade型拡張 |
| Portfolio.required_indices() | ❌ Missing | 中 |
| MissingIndexエラー | ❌ Missing | 低 |

**Constraint:** infra_domain::trade::Trade構造への侵入的変更

### Requirement 8: 後方互換性

| 技術要件 | ギャップ | 難易度 |
|---------|--------|--------|
| Deprecated API維持 | N/A | 低 |
| deprecation warning | N/A | 低 |
| Migration guide | N/A | 低 |

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**対象ファイル:**
- `pricer_models::market::provider.rs` - MarketProvider拡張
- `pricer_models::market::curves::curve_set.rs` - CurveSet拡張
- `pricer_models::market::index_mapper.rs` - Mapper拡張

**変更内容:**
1. CurveSetに`HashMap<RateIndex, CurveName>`インデックスを追加
2. MarketProviderにIndex-keyed APIを追加
3. IndexCurveMapperを`IndexMarketMapper`に拡張（VolCube, FxCurve対応）

**Trade-offs:**
- ✅ 最小限の新規ファイル
- ✅ 既存テスト継続使用可能
- ❌ CurveSet/MarketProviderの複雑化
- ❌ 責務の混在

### Option B: 新規Market構造体作成

**新規ファイル:**
- `pricer_models::market::indexed_market.rs` - 新Market構造体
- `infra_domain::market::volatility_index.rs` - VolatilityIndex型

**変更内容:**
1. 新`IndexedMarket<T>`構造体を作成
2. 内部でHashMap<RateIndex, Arc<YieldCurve>>等を保持
3. 統一API (`get_df`, `get_bs_vol`, `get_fx_vol`) を提供
4. MarketBuilderで構築

**Trade-offs:**
- ✅ クリーンな責務分離
- ✅ 既存コードに影響なし
- ❌ 新規ファイル増加
- ❌ 既存MarketProviderとの重複

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

**Trade-offs:**
- ✅ 段階的移行が可能
- ✅ 既存コードとの互換性維持
- ✅ テスト容易性
- ❌ 計画的な実装が必要

---

## 4. 複雑性・リスク評価

### Effort: M (3-7 days)

**理由:**
- 既存パターン（CurveSet, IndexCurveMapper）の拡張
- 新規型（VolatilityIndex, IndexRequirement）の定義
- ファサードパターンによる統合

### Risk: Medium

**リスク要因:**
1. Trade構造体への`required_indices()`追加の影響範囲
2. VolCube既存キャッシュとの互換性
3. パフォーマンス影響（HashMap lookup追加）

**緩和策:**
- `required_indices()`はtrait extension patternで実装
- VolCubeは既存ProviderKeyとの互換レイヤー提供
- Benchmarkで性能検証

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ: Option C（ハイブリッド）

### 主要設計決定

1. **IndexedMarket<T>構造体の設計**
   - 内部HashMap: `RateIndex → Arc<CurveEnum<T>>`, `CurrencyPair → Arc<FxCurve<T>>`
   - MarketProvider/CurveSetの既存機能をラップ
   - 統一API: `get_df()`, `get_forward_rate()`, `get_bs_vol()`, `get_fx_vol()`

2. **VolatilityIndex型の設計**
   - RateIndex（Swaption用）+ SwaptionKey（expiry/tenor）
   - または汎用`VolatilityIndexType` enum

3. **Trade.required_indices()の実装方式**
   - infra_domainへの直接変更 vs trait extension pattern
   - `IndexRequirement`型（RateIndex | CurrencyPair | VolatilityIndex）

### Research Items for Design Phase

1. **VolatilityIndex設計調査**: RateIndex再利用 vs 新型定義
2. **Trade trait extension**: infra_domain変更の影響範囲調査
3. **性能ベンチマーク**: HashMap lookup overhead測定

---

## 6. 要件-アセット対応表

| 要件 | 既存アセット | ギャップ |
|------|-------------|---------|
| Req 1.1 RateIndex primary key | `RateIndex` ✅ | CurveName経由 → 直接化 |
| Req 1.2 CurrencyPair for FxCurve | `CurrencyPair` ✅ | MarketProvider未対応 |
| Req 1.3 VolatilityIndex | `IndexType::SwapRate` 部分的 | **Missing** - 新型必要 |
| Req 2.1 get_df() | `CurveSet::get_curve_for_index` | ラッパー追加 |
| Req 2.2 get_forward_rate() | `CurveSet::forward_rate_for_index` ✅ | なし |
| Req 3.1 get_bs_vol() | `VolSurfaceEnum::volatility` | **Missing** - Index統合 |
| Req 3.2 get_swaption_vol() | `VolCube` | **Missing** - Index統合 |
| Req 4 IndexCurveMapper | `IndexCurveMapper` ✅ | FxCurve/VolCube拡張 |
| Req 5 Market HashMap | `CurveSet`, `MarketProvider` | 統合Market構造体 |
| Req 6 Builder.for_index() | Builder各種 部分的 | 統一化 |
| Req 7 validate_completeness() | なし | **Missing** - 新機能 |
| Req 7.3 Trade.required_indices() | なし | **Missing** - Trade拡張 |
| Req 8 後方互換 | N/A | deprecation追加 |

