# Research & Design Decisions: Market Index-Keyed Access

## Summary
- **Feature**: `market-index-keyed-access`
- **Discovery Scope**: Extension（既存Market/CurveSetシステムの拡張）
- **Key Findings**:
  - `IndexCurveMapper` trait と `CurveSet::get_curve_for_index()` が既に存在
  - `CurrencyPair` は Hash/Eq を実装済みでHashMapキーとして使用可能
  - VolCubeは `VolCubeProviderKey` (Currency + UnderlyingIndex) を使用
  - `required_indices()` 機能は未実装、Trade構造体への拡張が必要

## Research Log

### 既存Index型とHashMap互換性
- **Findings**:
  - `RateIndex`: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` — HashMap互換
  - `CurrencyPair`: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` — HashMap互換
  - `IndexType` enum: Rate, SwapRate, Fx, Equity, Inflation, Commodity
- **Implications**: 新規Index型定義は不要、既存型をHashMapキーとして直接使用可能

### CurveSet/IndexCurveMapper現状分析
- **Findings**:
  - `CurveSet<T>`: `HashMap<CurveName, CurveEnum<T>>` で内部保持
  - `IndexCurveMapper` trait: `fn curve_name(&self, index: &RateIndex) -> Option<CurveName>`
  - `DefaultIndexCurveMapper`: RateIndex→CurveName のデフォルト実装
- **Implications**: ファサードパターンで既存機能をラップする方が安全

### PathObserver ストリーミングパターン
- **Findings**:
  - `observe(price: T)` でインクリメンタル統計更新
  - `running_sum`, `running_max`, `running_min` パターン
  - フルパス保存不要（メモリ効率的）

## Design Decisions

### Decision: ハイブリッドアプローチ（Option C）採用
- **Selected Approach**: `IndexedMarket<T>` ファサードを作成し、既存コンポーネント（CurveSet, MarketProvider, VolCubeCache）をラップ
- **Rationale**: 既存コードへの影響最小化、段階的移行可能、後方互換性維持

### Decision: VolatilityIndex型の設計
- **Selected Approach**: Swaption用は`RateIndex`を使用、FX用は`CurrencyPair`を使用（型による分離）
- **Rationale**: Swaption VolCubeはRateIndex（通貨+テナー）で一意に特定可能、新規型定義不要でシンプル

### Decision: IndexRequirement型の設計
- **Selected Approach**: `IndexRequirement` enum を新規定義（Static dispatch維持）
- **Rationale**: Static dispatch維持でEnzyme互換、明確な型による検証が可能、将来の拡張性

## Risks & Mitigations
- **Risk 1: Trade構造体への変更影響** — Mitigation: trait extension patternで`TradeIndexRequirements` traitを定義
- **Risk 2: VolCubeキャッシュ互換性** — Mitigation: IndexedMarket内部でRateIndex→VolCubeProviderKey変換を実装
- **Risk 3: パフォーマンス劣化** — Mitigation: HashMap lookupは既にCurveSetで使用済み、ベンチマークで検証
