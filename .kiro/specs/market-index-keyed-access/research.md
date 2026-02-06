# Research & Design Decisions: Market Index-Keyed Access

---
**Purpose**: 設計ドキュメントを支える調査結果と設計決定の根拠を記録
---

## Summary
- **Feature**: `market-index-keyed-access`
- **Discovery Scope**: Extension（既存Market/CurveSetシステムの拡張）
- **Key Findings**:
  - `IndexCurveMapper` trait と `CurveSet::get_curve_for_index()` が既に存在し、RateIndex→CurveNameの間接参照を提供
  - `CurrencyPair` は Hash/Eq を実装済みでHashMapキーとして使用可能
  - VolCubeは `VolCubeProviderKey` (Currency + UnderlyingIndex) を使用しており、RateIndex直接キー化ではない
  - `required_indices()` 機能は未実装、Trade構造体への拡張が必要

## Research Log

### Topic: 既存Index型とHashMap互換性

- **Context**: RateIndex, CurrencyPairがHashMapキーとして使用可能か確認
- **Sources Consulted**:
  - `infra_domain::market::rate_index.rs`
  - `infra_domain::trade::instrument_def::fx.rs`
- **Findings**:
  - `RateIndex`: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` — HashMap互換 ✅
  - `CurrencyPair`: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` — HashMap互換 ✅
  - `IndexType` enum: Rate, SwapRate, Fx, Equity, Inflation, Commodity — 汎用Indexとして使用可能
- **Implications**: 新規Index型定義は不要、既存型をHashMapキーとして直接使用可能

### Topic: CurveSet/IndexCurveMapper現状分析

- **Context**: 既存のIndex→Curveマッピング機構の調査
- **Sources Consulted**:
  - `pricer_models::market::curves::curve_set.rs`
  - `pricer_models::market::index_mapper.rs`
- **Findings**:
  - `CurveSet<T>`: `HashMap<CurveName, CurveEnum<T>>` で内部保持
  - `IndexCurveMapper` trait: `fn curve_name(&self, index: &RateIndex) -> Option<CurveName>`
  - `DefaultIndexCurveMapper`: RateIndex→CurveName のデフォルト実装
  - `CurveSet::get_curve_for_index()`: IndexCurveMapper経由でCurve取得
  - `CurveSet::forward_rate_for_index()`: Index指定でforward rate取得
- **Implications**:
  - CurveNameを介した間接参照パターンが確立済み
  - 直接HashMap<RateIndex, Arc<CurveEnum>>への変更は互換性リスクあり
  - ファサードパターンで既存機能をラップする方が安全

### Topic: VolCubeキャッシュ機構

- **Context**: VolCubeのIndex-keyedアクセス対応方法の調査
- **Sources Consulted**:
  - `pricer_models::market::volcube/cache.rs`
  - `pricer_models::market::volcube/types.rs`
  - `pricer_models::market::provider.rs`
- **Findings**:
  - `VolCubeProviderKey`: `(Currency, UnderlyingIndex)` タプル
  - `SharedVolCubeCache`: `Arc<RwLock<HashMap<VolCubeProviderKey, Arc<VolCube>>>>`
  - `MarketProvider`: volcube_cache をVolCubeProviderKeyでキャッシュ
  - RateIndexとVolCubeProviderKeyの間に変換レイヤー必要
- **Implications**:
  - VolCubeはRateIndexから直接取得できない
  - RateIndex→(Currency, UnderlyingIndex)変換メソッドが必要
  - 既存キャッシュ機構との互換性維持が重要

### Topic: FxCurve/FxVolSurface統合

- **Context**: FX関連データのCurrencyPairキー化対応
- **Sources Consulted**:
  - `pricer_models::market::fx_calibration/curve.rs`
  - `pricer_models::market::fx_calibration/surface.rs`
  - `pricer_models::market::provider.rs`
- **Findings**:
  - `FxCurve<T>` trait: `currency_pair(&self) -> CurrencyPair` メソッド有
  - `CalibratedFxCurve`, `SimpleFxCurve`: CurrencyPair保持
  - `MarketProvider`: curve_cache は `Currency` でキー化（CurrencyPairではない）
  - FxVolSurfaceも同様にCurrencyベースのキャッシュ
- **Implications**:
  - FxCurve自体はCurrencyPair対応済み
  - MarketProvider/キャッシュ層がCurrency→CurrencyPair対応必要
  - IndexedMarketファサードでCurrencyPair→FxCurveマッピング提供

### Topic: Trade.required_indices()実装方式

- **Context**: Trade構造体への拡張方法の調査
- **Sources Consulted**:
  - `infra_domain::trade/trade.rs`
  - `infra_domain::trade/cashflow.rs`
  - `infra_domain::trade/index.rs`
- **Findings**:
  - `Trade`: legs: Vec<Leg>, metadata: TradeMetadata
  - `Leg`: cashflows: Vec<Cashflow>, direction: Direction
  - `Cashflow`: `index_observation: Option<IndexObservation>` フィールド有
  - `IndexObservation`: `index_type: IndexType` を保持
- **Implications**:
  - Cashflow経由でIndexTypeを抽出可能
  - Trade→Vec<Leg>→Vec<Cashflow>→IndexObservation→IndexType
  - trait extension patternで`required_indices()`を追加可能
  - infra_domainへの直接変更不要

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: 既存拡張 | CurveSet/MarketProviderを直接拡張 | 最小限の新規ファイル | 既存コードの複雑化、責務混在 | 互換性リスク高 |
| B: 新Market | 完全新規IndexedMarket構造体 | クリーンな責務分離 | 既存コードとの重複、移行コスト | 並行運用必要 |
| C: ハイブリッド | ファサードパターンで既存ラップ | 段階的移行、互換性維持 | 計画的実装が必要 | **推奨** |

## Design Decisions

### Decision: ハイブリッドアプローチ（Option C）採用

- **Context**: 既存Market機構を維持しつつIndex-keyed APIを提供する必要
- **Alternatives Considered**:
  1. CurveSet/MarketProviderの直接拡張 — 既存コードの複雑化
  2. 完全新規Market構造体 — 既存との重複、移行リスク
- **Selected Approach**: `IndexedMarket<T>` ファサードを作成し、既存コンポーネント（CurveSet, MarketProvider, VolCubeCache）をラップ
- **Rationale**:
  - 既存コードへの影響最小化
  - 段階的移行が可能
  - 後方互換性の維持
- **Trade-offs**:
  - (+) 既存テスト継続使用可能
  - (+) 並列開発可能
  - (-) 一時的な二重構造
- **Follow-up**: Phase 2で内部最適化、Phase 3で非推奨APIの削除

### Decision: VolatilityIndex型の設計

- **Context**: VolCube/VolSurfaceのIndex-keyedアクセスに必要なキー型
- **Alternatives Considered**:
  1. RateIndex再利用 — Swaption VolCubeには十分
  2. 新規VolatilityIndex enum — 汎用性高いが複雑
  3. IndexType活用 — 既存enum拡張
- **Selected Approach**: Swaption用は`RateIndex`を使用、FX用は`CurrencyPair`を使用（型による分離）
- **Rationale**:
  - Swaption VolCubeはRateIndex（通貨+テナー）で一意に特定可能
  - FX VolSurfaceはCurrencyPairで特定
  - 新規型定義不要でシンプル
- **Trade-offs**:
  - (+) 既存型の再利用
  - (-) 将来的にEquity Vol等には別途対応必要
- **Follow-up**: Equity/Commodity Vol追加時に`VolatilityIndex` enum検討

### Decision: IndexRequirement型の設計

- **Context**: Trade.required_indices()の戻り値型
- **Alternatives Considered**:
  1. Vec<IndexType> — 既存IndexTypeを使用
  2. 新規IndexRequirement enum — RateIndex | CurrencyPair | VolatilityIndex
  3. trait object Vec<Box<dyn Index>> — 柔軟だがEnzyme非互換
- **Selected Approach**: `IndexRequirement` enum を新規定義（Static dispatch維持）
- **Rationale**:
  - Static dispatch維持でEnzyme互換
  - 明確な型による検証が可能
  - 将来の拡張性（Credit, Equity等）
- **Trade-offs**:
  - (+) 型安全
  - (+) Static dispatch
  - (-) 新規型定義必要
- **Follow-up**: infra_domain::trade::index_requirement.rs に配置

## Risks & Mitigations

- **Risk 1: Trade構造体への変更影響** — Mitigation: trait extension patternで`TradeIndexRequirements` traitを定義、infra_domain本体への変更不要
- **Risk 2: VolCubeキャッシュ互換性** — Mitigation: IndexedMarket内部でRateIndex→VolCubeProviderKey変換を実装、既存キャッシュ機構を再利用
- **Risk 3: パフォーマンス劣化** — Mitigation: HashMap lookupは既にCurveSetで使用済み、追加オーバーヘッドは最小。ベンチマークで検証

## References

- [pricer_models::market::index_mapper.rs](crates/pricer_models/src/market/index_mapper.rs) — 既存IndexCurveMapper実装
- [pricer_models::market::curves::curve_set.rs](crates/pricer_models/src/market/curves/curve_set.rs) — 既存CurveSet実装
- [pricer_models::market::provider.rs](crates/pricer_models/src/market/provider.rs) — 既存MarketProvider実装
- [infra_domain::trade::index.rs](crates/infra_domain/src/trade/index.rs) — IndexType, IndexObservation定義
