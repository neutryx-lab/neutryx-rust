# Research & Design Decisions

## Summary
- **Feature**: `pricer-view-rebuild`
- **Discovery Scope**: Extension（既存モノリシックコンポーネントの再構築）
- **Key Findings**:
  - 既存の `services/api.ts` に全 Pricer API 関数が実装済み。エンドポイントはバックエンドのデュアルルート（`/api/instruments` & `/api/pricer/instruments`）と整合
  - `types/api.ts` + `types/index.ts` に `PricerState`, `CashflowEdit` を含む全型定義が存在。ローカル再宣言は完全に不要
  - `stores/config.ts` が Composition API スタイルの Pinia ストアパターンを提供。新規 `stores/pricer.ts` はこのパターンに従う

## Research Log

### 既存 API サービス分析
- **Context**: 現行 PricerView は raw `fetch()` を使用。`services/api.ts` の関数を再利用可能か検証
- **Findings**:
  - `fetchInstruments()` → `GET /api/instruments` → `InstrumentsResponse`
  - `expandTrade(req)` → `POST /api/trade/expand` → `ExpandedTrade`
  - `priceTrade(req)` → `POST /api/pricer/price` → `PricingResult`
  - `calculateGreeks(req)` → `POST /api/pricer/greeks` → `GreeksResult`
  - 全関数が `fetchJson<T>` / `postJson<TReq, TRes>` ヘルパーを使用し、エラーハンドリング込み
- **Implications**: 新規 API 関数の作成は不要。既存関数をそのまま composable/store から呼び出す

### 既存型定義の重複分析
- **Context**: 現行 PricerView は 12 インターフェースをローカルに再宣言
- **Findings**:
  - `types/api.ts`: `Instrument`, `ParameterDef`, `ExpandedTrade`, `TradeLeg`, `Cashflow`, `PricingRequest`, `PricingResult`, `GreeksResult`, `BumpSizes` 等が定義済み
  - `types/index.ts`: `PricerState`, `CashflowEdit` が定義済み
  - 現行ローカル型との差分: `HistoryEntry`, `StochasticModelConfig`, `ModelParamDef`, `ValidationError` は `types/api.ts` に未定義
- **Implications**: `HistoryEntry`, `StochasticModelConfig`, `ModelParamDef`, `ValidationError` は `constants/pricer.ts` または新規型ファイルに定義が必要

### Pinia ストアパターン分析
- **Context**: `stores/config.ts` のパターンを分析し、pricer ストアの設計に適用
- **Findings**:
  - Composition API (`setup`) スタイル: `defineStore('name', () => { ... })`
  - State は `ref()` で定義、Getters は `computed()` で定義
  - Actions は通常の関数として定義
  - `return` で公開 API を明示
- **Implications**: `stores/pricer.ts` は同一パターンに従う。既存 `PricerState` インターフェースをベースに状態を構成

### フォーマットユーティリティの重複分析
- **Context**: 現行 PricerView は `formatCurrency`, `formatNumberCompact`, `parseFormattedNumber` をローカルに再定義
- **Findings**:
  - `utils/format.ts` に同名関数が存在
  - ローカル版は `$` プレフィックス付きだが、`utils/format.ts` 版は `Intl.NumberFormat` ベース
  - `utils/format.ts` はさらに `formatDate`, `formatTimestamp`, `formatVol` 等も提供
- **Implications**: `utils/format.ts` を使用。通貨フォーマットの表示差異（`$` プレフィクス vs `Intl`）は UI 層で吸収

### コンポーネント分割戦略
- **Context**: 1050 行のモノリシックコンポーネントをどう分割するか
- **Findings**:
  - 現行テンプレートは大きく 4 ゾーン: サマリーバー (579-591)、左パネル設定 (602-911)、右パネル CF テーブル (914-1035)、フォールバック (594-597)
  - 左パネル内はさらに 7 セクション: 商品選択、評価設定、マーケットデータ、モデル選択、アクション、PV 結果、Greeks、メトリクス、履歴
  - VolcubeBuilderView（1410行）も同様のモノリシック構造であり、将来的にはこちらも同様のリファクタリング候補
- **Implications**: 13 サブコンポーネントへの分割は妥当。各コンポーネントは 30-180 行の範囲に収まる

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Pinia Store + Composables | 中央ストア + ドメイン特化 composable | 状態の一元管理、composable による関心の分離、Vue DevTools 対応 | composable 間の暗黙的依存 | 既存 config store パターンと整合 |
| Props/Emit Chain | 親子間の props down / events up | Vue の標準パターン、明示的データフロー | 深いネスト時の prop drilling | 8+ 階層で煩雑に |
| Provide/Inject | 祖先から子孫への DI | prop drilling 回避 | 型安全性の欠如、暗黙的依存 | Pinia の方が型安全 |

## Design Decisions

### Decision: Pinia Store を状態管理の中核とする
- **Context**: 40+ refs を構造化し、8+ コンポーネント間で共有する必要がある
- **Alternatives Considered**:
  1. Props/Emit チェーン — 明示的だが prop drilling が深くなる
  2. Provide/Inject — DI パターンだが型安全性が弱い
  3. Pinia Store — 一元管理、型安全、DevTools 対応
- **Selected Approach**: Pinia Store + Composables のハイブリッド
- **Rationale**: 既存 `stores/config.ts` パターンとの整合性、`PricerState` インターフェースの存在、DevTools によるデバッグ容易性
- **Trade-offs**: ストアへの依存が集中するが、composable 層で責務を分離することで緩和
- **Follow-up**: ストアの肥大化を監視し、150 行を超える場合は分割を検討

### Decision: 未定義型は constants/pricer.ts に同居
- **Context**: `HistoryEntry`, `StochasticModelConfig`, `ModelParamDef`, `ValidationError` は既存型ファイルに未定義
- **Selected Approach**: Pricer 固有の型は `constants/pricer.ts` に定数と共に定義
- **Rationale**: 別途型ファイルを作成するほどの量ではない。定数と型は密結合（`STOCHASTIC_MODELS` の型が `StochasticModelConfig`）

## Risks & Mitigations
- **リスク 1**: `utils/format.ts` の `formatCurrency` と現行のローカル版で表示が異なる → UI テストで確認、必要なら wrapper 関数を追加
- **リスク 2**: Pinia ストアが肥大化する可能性 → composable 層でロジックを分離し、ストアは状態とシンプルな mutation に限定
- **リスク 3**: フェーズ間の機能追加で既存コンポーネントの Props/インターフェースが変更される → 初期設計で拡張ポイントを明示

## References
- [Vue 3 Composition API](https://vuejs.org/guide/extras/composition-api-faq.html) — composable 設計パターン
- [Pinia Documentation](https://pinia.vuejs.org/) — Setup Store パターン
- 既存 `stores/config.ts` — プロジェクト内の Pinia ストア参考実装
- 既存 `services/api.ts` — API クライアント関数群
