# Research & Design Decisions: portfolio-graph-optimisation

## Summary
- **Feature**: portfolio-graph-optimisation
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  - 既存の`GraphExtractable`トレイトと`SimpleGraphExtractor`は単一trade_idまたはNone（全トレード）での抽出をサポート済み
  - `GraphBuilder`は事前割り当てメモリ管理とDAG検証を提供、100トレード×10パラメータを5秒以内に処理可能
  - `pricer_risk::portfolio::Portfolio`は`HashMap<TradeId, Trade>`でO(1)ルックアップと`trades_par_iter()`並列処理をサポート
  - Web Dashboard既存API（`/api/graph`、WebSocket `graph_update`）はD3.js互換JSON形式を出力

## Research Log

### 1. GraphNode拡張（trade_ids フィールド追加）
- **Context**: Requirement 1.4で各ノードに所属トレードID追跡が必要
- **Sources Consulted**:
  - [types.rs](crates/pricer_pricing/src/graph/types.rs) - 既存GraphNode構造
  - serde互換性パターン
- **Findings**:
  - 現在の`GraphNode`フィールド: `id`, `node_type`, `label`, `value`, `is_sensitivity_target`, `group`
  - `trade_ids`追加は後方互換性のため`Option<Vec<String>>`またはデフォルト空`Vec<String>`で実装可能
  - `#[serde(default, skip_serializing_if = "Vec::is_empty")]`で既存API互換性維持
- **Implications**:
  - 既存の単一トレードグラフでは`trade_ids`が空または単一要素
  - Portfolio統合グラフでは共有ノードが複数`trade_ids`を持つ

### 2. ノード重複排除アルゴリズム
- **Context**: Requirement 1.3でマーケットデータノードの重複排除が必要
- **Sources Consulted**:
  - [extractor.rs](crates/pricer_pricing/src/graph/extractor.rs) - `GraphBuilder.has_node()`
  - ハッシュベース同一性判定パターン
- **Findings**:
  - 現在の`has_node()`はID完全一致のみ
  - 共有マーケットデータ検出には`(label, node_type)`または明示的キーでの同一性判定が必要
  - `NodeKey = (String, NodeType)` タプルでHashMapキーとしてO(1)検索可能
- **Implications**:
  - 同一`label`（例: "USD_YieldCurve"）かつ同一`node_type`のノードを共有
  - 共有ノード発見時は既存ノードの`trade_ids`に追加

### 3. サブグラフ抽出アルゴリズム
- **Context**: Requirement 3で複数trade_id指定によるサブグラフ抽出が必要
- **Sources Consulted**:
  - BFS/DFSグラフ探索パターン
  - [types.rs](crates/pricer_pricing/src/graph/types.rs) - `ComputationGraph.find_path()`
- **Findings**:
  - 選択トレードの出力ノードから逆方向BFSで到達可能ノードを収集
  - 共有ノード（複数`trade_ids`を持つ）は選択トレードのいずれかを含めば保持
  - エッジは両端ノードがサブグラフに含まれる場合のみ保持
- **Implications**:
  - O(V + E)の時間計算量でサブグラフ抽出可能
  - 選択トレード数に比例したパフォーマンス

### 4. SamplePortfolioBuilder設計
- **Context**: Requirement 2で複数アセットクラスのサンプルPortfolio生成が必要
- **Sources Consulted**:
  - [demo.rs](crates/pricer_risk/src/demo.rs) - 既存`DemoTrade`
  - [handlers.rs](demo/gui/src/web/handlers.rs) - `sample_trades()`
- **Findings**:
  - 既存`sample_trades()`は12件の固定トレード（IRS、Swaption、Cap）
  - `DemoTrade`は`id`, `ccy`, `model`, `instrument`を持つ
  - 共有マーケットデータ生成には同一通貨・同一満期のトレード組み合わせが必要
- **Implications**:
  - 新規`SamplePortfolioBuilder`で設定可能なトレード数とアセットミックスを提供
  - `with_trade_count(n)`, `with_asset_mix(equity_pct, rates_pct, fx_pct)`メソッド

### 5. LODモード設計
- **Context**: Requirement 6.5で10,000ノード超のグラフ簡略化が必要
- **Sources Consulted**:
  - D3.js force-directed graph パフォーマンス特性
  - グラフ要約アルゴリズム（ノードクラスタリング、階層集約）
- **Findings**:
  - D3.js SVGは~10,000要素で実用限界
  - LOD戦略: (1) 同一`node_type`ノードをクラスタ化、(2) 中間ノード省略
  - 閾値は設定可能（デフォルト10,000）が推奨
- **Implications**:
  - `LodMode::Full | LodMode::Clustered | LodMode::Summary`の3レベル
  - Phase 2で実装、Phase 1はMVPとしてスキップ可能

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: Extend SimpleGraphExtractor | 既存クラスにPortfolio対応メソッド追加 | 既存コード再利用最大化、学習コスト低 | クラス肥大化、単一責任違反 | - |
| B: New PortfolioGraphExtractor | Portfolio専用の新クラス、コンポジション | 責務分離、テスト容易性 | 新規ファイル増、重複コード可能性 | - |
| **C: Hybrid (推奨)** | Phase 1: Extend、Phase 2: Refactor | 段階的リスク軽減、早期デモ可能 | 2フェーズ管理必要 | ギャップ分析で推奨 |

## Design Decisions

### Decision: GraphNode.trade_ids フィールド型
- **Context**: 複数トレード所属追跡のためGraphNodeに`trade_ids`フィールド追加が必要
- **Alternatives Considered**:
  1. `Option<Vec<String>>` — 後方互換性重視、Noneで単一トレード
  2. `Vec<String>` (default empty) — シンプル、空ベクタで単一トレード
  3. `HashSet<String>` — 重複防止、順序不保証
- **Selected Approach**: `Vec<String>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
- **Rationale**:
  - 空ベクタはOption::Noneと同義で、単一トレードグラフでは省略可能
  - 順序保持で最初に追加されたトレードが識別可能
  - serde属性で既存APIとの後方互換性維持
- **Trade-offs**: 重複チェックは呼び出し側で必要（パフォーマンス影響は軽微）
- **Follow-up**: 既存テストが新フィールドで破損しないことを確認

### Decision: Portfolio対応の設計アプローチ
- **Context**: PortfolioグラフでのGraphNode拡張をどのように実装するか
- **Alternatives Considered**:
  1. **アプローチ1（Fromトレイト）** — 別型`PortfolioGraphNode`を定義し、`From<GraphNode>`で変換
     - 利点: 既存`GraphNode`を変更しない
     - 欠点: 型変換コスト、2つの型定義の保守
  2. **アプローチ2（GraphNode拡張）** — 既存`GraphNode`に直接`trade_ids`フィールドを追加
     - 利点: 単一定義、型変換不要、コード重複なし
     - 欠点: 既存構造体の変更
  3. **アプローチ3（Generic）** — `GraphNode<T>`としてジェネリック化
     - 利点: 柔軟性最大
     - 欠点: 過度な複雑性、既存コード大幅変更
- **Selected Approach**: アプローチ2（GraphNode拡張）
- **Rationale**:
  - 単一定義でコード重複を排除
  - `SimpleGraphExtractor`の戻り値をそのまま`PortfolioGraphExtractor`で使用可能
  - serde属性（`skip_serializing_if = "Vec::is_empty"`）により後方互換性維持
  - `GraphBuilder`への変更は`add_trade_id()`、`set_trade_ids()`メソッド追加のみ
- **Trade-offs**: 既存`GraphNode`構造体の変更が必要（影響は軽微）
- **Follow-up**: 既存テストの動作確認、新フィールド追加による影響評価

### Decision: ノード同一性判定キー
- **Context**: 共有マーケットデータノードの検出基準
- **Alternatives Considered**:
  1. `id`完全一致 — 現状維持、共有検出不可
  2. `(label, node_type)`タプル — ラベルと型で同一性判定
  3. 明示的`shared_key: Option<String>` — 共有意図を明示
- **Selected Approach**: `(label, node_type)`タプルでハッシュマップキー
- **Rationale**:
  - マーケットデータノードは通常同一ラベル（例: "USD_YieldCurve"）を持つ
  - 追加フィールド不要で既存構造を維持
  - O(1)検索でパフォーマンス維持
- **Trade-offs**: 異なるトレードで同名ノードが意図せず共有される可能性
- **Follow-up**: ラベル命名規約をドキュメント化

### Decision: PortfolioGraphMetadata構造
- **Context**: Requirement 1.5で拡張メタデータが必要
- **Alternatives Considered**:
  1. 既存`GraphMetadata`を拡張 — フィールド追加
  2. 新規`PortfolioGraphMetadata`構造体 — 責務分離
  3. `GraphMetadata`内に`Option<PortfolioStats>`をネスト
- **Selected Approach**: 新規`PortfolioGraphMetadata`構造体
- **Rationale**:
  - Portfolio固有フィールド（`trade_count`, `shared_node_count`, `optimisation_ratio`）を明確に分離
  - 既存`GraphMetadata`との互換性維持
  - `PortfolioComputationGraph`専用のメタデータとして設計
- **Trade-offs**: 新規型によるコード増加
- **Follow-up**: D3.js互換のJSON構造を維持

### Decision: REST APIエンドポイント設計
- **Context**: Requirement 4, 5で新規APIエンドポイントが必要
- **Alternatives Considered**:
  1. 既存`/api/graph`を拡張 — クエリパラメータで分岐
  2. 新規`/api/v1/portfolio/graph` — 明確な分離
  3. GraphQL — 柔軟なクエリ
- **Selected Approach**: 新規RESTエンドポイント `/api/v1/portfolio/graph`, `/api/v1/portfolio/trades`
- **Rationale**:
  - 既存`/api/graph`との後方互換性維持
  - RESTful規約に従った階層的URL構造
  - 既存プロジェクトパターン（`/api/v1/...`）に準拠
- **Trade-offs**: エンドポイント数増加
- **Follow-up**: OpenAPI仕様への追加

## Risks & Mitigations
- **Risk 1: ノード統合アルゴリズムの複雑性** — ハッシュベースO(n)設計で緩和、ベンチマークテスト実施
- **Risk 2: GraphNode拡張の後方互換性** — `serde(default, skip_serializing_if)`で既存API維持
- **Risk 3: LODモードの設計不明確さ** — Phase 2に分離、Phase 1はMVPとして実装
- **Risk 4: 大規模Portfolioでのパフォーマンス劣化** — 事前割り当て、並列処理、キャッシュで緩和

## References
- [pricer_pricing::graph types.rs](crates/pricer_pricing/src/graph/types.rs) — 既存GraphNode、ComputationGraph構造
- [pricer_pricing::graph extractor.rs](crates/pricer_pricing/src/graph/extractor.rs) — GraphExtractable trait、SimpleGraphExtractor
- [pricer_risk::portfolio mod.rs](crates/pricer_risk/src/portfolio/mod.rs) — Portfolio構造、並列イテレーション
- [demo/gui/src/web/handlers.rs](demo/gui/src/web/handlers.rs) — 既存API実装パターン
- [D3.js Force-Directed Graph](https://d3js.org/d3-force) — グラフ可視化ライブラリ

---
_Generated: 2026-01-19_
