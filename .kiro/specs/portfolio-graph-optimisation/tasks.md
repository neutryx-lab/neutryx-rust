# Implementation Plan

## Feature: portfolio-graph-optimisation

Portfolio計算グラフ最適化機能の実装タスク。既存`pricer_pricing::graph`モジュールをPortfolioレベルに拡張し、共有ノード重複排除、サブグラフ抽出、およびWeb Dashboard統合を実現する。

---

## Tasks

### Phase 1: GraphNode拡張と基盤型

- [x] 1. GraphNode拡張とPortfolio用メタデータ型の追加
- [x] 1.1 GraphNodeに`trade_ids`フィールドを追加
  - 既存`GraphNode`構造体に`trade_ids: Vec<String>`フィールドを追加
  - serde属性`#[serde(default, skip_serializing_if = "Vec::is_empty")]`で後方互換性を維持
  - 単一トレードグラフでは空ベクタのままJSONから省略される動作を確保
  - `Default`トレイト実装を追加して既存テストとの互換性を確保
  - _Requirements: 1.4_

- [x] 1.2 (P) GraphBuilderにtrade_id操作メソッドを追加
  - `add_trade_id(node_id, trade_id)`メソッドで個別トレードIDを追加
  - `set_trade_ids(node_id, trade_ids)`メソッドでトレードIDリストを一括設定
  - 重複チェック（同一trade_idの二重追加防止）
  - 存在しないnode_idへの操作時は`Option<()>`でNone返却
  - _Requirements: 1.4_

- [x] 1.3 (P) PortfolioGraphMetadata型を追加
  - 基本メタデータ（node_count, edge_count, depth, generated_at）を含む
  - Portfolio固有フィールド（trade_count, shared_node_count, optimisation_ratio）を追加
  - `optimisation_ratio`は0 < ratio <= 1の範囲で重複排除効率を表現
  - serde Serialize実装
  - _Requirements: 1.5_

- [x] 1.4 (P) PortfolioComputationGraph型を追加
  - 既存`GraphNode`（拡張済み）と`GraphEdge`を使用
  - `edges`フィールドを`links`としてシリアライズ（D3.js互換維持）
  - `PortfolioGraphMetadata`をメタデータとして含む
  - _Requirements: 1.1, 1.2, 1.5_

### Phase 2: PortfolioGraphExtractor実装

- [x] 2. PortfolioGraphExtractorの実装
- [x] 2.1 PortfolioGraphExtractableトレイトを定義
  - `extract_portfolio_graph(portfolio) -> Result<PortfolioComputationGraph, GraphError>`
  - `extract_subgraph(portfolio, trade_ids) -> Result<PortfolioComputationGraph, GraphError>`
  - `extract_portfolio_updates(portfolio) -> Result<Vec<GraphNodeUpdate>, GraphError>`
  - _Requirements: 1.1, 3.1_

- [x] 2.2 PortfolioGraphExtractor構造体を実装
  - `SimpleGraphExtractor`をコンポジションで内包
  - タイムアウト設定（デフォルト500ms）とbuilder capacity設定をサポート
  - `new()`, `with_timeout()`, `with_capacity()`ビルダーメソッド
  - _Requirements: 1.1, 6.4_

- [x] 2.3 Portfolio統合グラフ抽出ロジックを実装
  - Portfolio内の全トレードからサブグラフを生成
  - 各トレードの`trade_id`をノードに設定
  - `(label, node_type)`タプルをキーとした共有ノード検出用HashMap
  - 共有ノード統合時に`trade_ids`をマージ
  - _Requirements: 1.2, 1.3, 1.4_

- [x] 2.4 共有ノード重複排除最適化を実装
  - マーケットデータノード（Input, YieldCurve, VolSurface等）の共有検出
  - 複数トレードで同一`(label, node_type)`を持つノードを単一ノードに統合
  - 統合時にエッジのリダイレクト処理
  - 最適化率（optimisation_ratio）の計算
  - _Requirements: 1.3, 6.2_

- [x] 2.5 サブグラフ抽出機能を実装
  - 指定`trade_ids`リストに基づくノードフィルタリング
  - 選択トレード間で共有されるノードを保持
  - 選択されていないトレード専用ノードを除外
  - フィルタ後のエッジ整合性検証（両端点がサブグラフ内に存在）
  - 存在しないtrade_id指定時は`GraphError::TradeNotFound`を返却
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 2.6 (P) GraphError拡張
  - `TradeNotFound(String)`バリアントを追加
  - `Timeout`バリアントを追加
  - `ExtractionFailed(String)`バリアントを追加
  - _Requirements: 3.5, 4.5_

### Phase 3: SamplePortfolioBuilder実装

- [x] 3. SamplePortfolioBuilderの実装
- [x] 3.1 SamplePortfolioBuilder構造体を定義
  - トレード数設定（デフォルト: 10〜100件、範囲指定可能）
  - アセットミックス比率設定（equity_ratio, rates_ratio, fx_ratio）
  - `new()`, `with_trade_count()`, `with_asset_mix()`ビルダーメソッド
  - _Requirements: 2.1, 2.2_

- [x] 3.2 複数アセットクラスのサンプルトレード生成
  - VanillaOption（Equity）の生成ロジック
  - IRS（Rates）の生成ロジック
  - FxOption（FX）の生成ロジック
  - 最低3種類のInstrument含有を保証
  - _Requirements: 2.1, 2.4_

- [x] 3.3 共有マーケットデータを持つトレード配置
  - 同一通貨ペア（USD/JPY等）を複数FxOptionで共有
  - 同一満期日を複数トレードで共有
  - 同一YieldCurve参照を複数Ratesトレードで共有
  - 意図的な共有パターンで20%以上のノード削減を達成可能に
  - _Requirements: 2.3, 6.2_

- [x] 3.4 (P) エラーハンドリングとバリデーション
  - 無効なtrade_count（0以下）のバリデーション
  - アセット比率合計が1.0でない場合のエラー
  - 生成失敗時の詳細エラーメッセージ（`PortfolioError`）
  - _Requirements: 2.5_

### Phase 4: REST API実装

- [x] 4. Web Dashboard REST APIの実装
- [x] 4.1 `/api/v1/portfolio/graph`エンドポイント実装
  - GETリクエストでPortfolio統合グラフを取得
  - クエリパラメータ`trade_ids`（カンマ区切り）でサブグラフフィルタリング
  - D3.js互換JSON形式（nodes, links, metadata）でレスポンス
  - `PortfolioGraphMetadata`を含む拡張メタデータ
  - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [x] 4.2 タイムアウトとエラーハンドリング
  - 500msタイムアウト保護（`tokio::time::timeout`）
  - タイムアウト時はHTTP 504 Gateway Timeout
  - 存在しないtrade_id時はHTTP 404 Not Found
  - 内部エラー時はHTTP 500 Internal Server Error
  - _Requirements: 4.5_

- [x] 4.3 `/api/v1/portfolio/trades`エンドポイント実装
  - GETリクエストでPortfolio内トレード一覧を取得
  - 各トレードのID、Instrument種別、通貨、想定元本、満期日を含む
  - 統計情報（total_count, by_instrument_type内訳）を含むレスポンス
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 4.4 (P) GraphCacheの実装
  - `RwLock<HashMap>`による5秒TTLキャッシュ
  - キャッシュキーはPortfolioのハッシュまたはID
  - キャッシュヒット時は即座にレスポンス
  - キャッシュミス時のみ`PortfolioGraphExtractor`を呼び出し
  - _Requirements: 6.3_

### Phase 5: WebSocket統合

- [x] 5. WebSocketリアルタイム更新
- [x] 5.1 `select_trades`イベントハンドラ実装
  - クライアントからの`{"type": "select_trades", "trade_ids": [...]}`受信
  - trade_idsバリデーション
  - `extract_subgraph`呼び出しでサブグラフ生成
  - _Requirements: 5.4_

- [x] 5.2 `subgraph_update`ブロードキャスト実装
  - サブグラフ生成完了後に接続クライアントへ送信
  - `{"update_type": "subgraph_update", "data": {...}}`形式
  - エラー時はエラーメッセージをブロードキャスト
  - セッション管理（同一セッション内の選択状態追跡）
  - _Requirements: 5.5_

### Phase 6: パフォーマンス最適化と検証

- [x] 6. パフォーマンス最適化
- [x] 6.1 事前割り当てキャパシティの最適化
  - `GraphBuilder::with_capacity()`でノード・エッジ数を事前割り当て
  - Portfolio内トレード数からキャパシティを推定
  - メモリ再割り当て回数の最小化
  - _Requirements: 6.4_

- [x] 6.2 (P) 10,000ノード超過時の警告実装
  - グラフ生成時にノード数をチェック
  - 10,000ノード超過時はログ警告を出力
  - メタデータに`large_graph_warning: bool`フラグを追加（Phase 2のLODモード準備）
  - _Requirements: 6.5_

### Phase 7: テスト

- [x] 7. テスト実装
- [x] 7.1 GraphNode拡張のユニットテスト
  - `trade_ids`フィールドのserde動作テスト（空時省略、非空時出力）
  - `Default`トレイト動作確認
  - 既存テストとの後方互換性確認
  - _Requirements: 1.4_

- [x] 7.2 (P) PortfolioGraphExtractorのユニットテスト
  - `extract_portfolio_graph()`複数トレード統合テスト
  - `extract_subgraph()`トレード選択フィルタリングテスト
  - 共有ノード検出・統合テスト（`merge_shared_nodes`）
  - 存在しないtrade_id指定時のエラーテスト
  - _Requirements: 1.1, 1.2, 1.3, 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 7.3 (P) SamplePortfolioBuilderのユニットテスト
  - アセットミックス生成テスト（3種類以上のInstrument含有）
  - 共有マーケットデータ含有テスト
  - エラーケーステスト（無効なtrade_count、無効なアセット比率）
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 7.4 REST API統合テスト
  - `/api/v1/portfolio/graph`エンドツーエンドテスト
  - サブグラフ取得テスト（`?trade_ids=T001,T002`）
  - `/api/v1/portfolio/trades`トレード一覧取得テスト
  - タイムアウト動作テスト（504レスポンス）
  - 既存`/api/graph`との後方互換性テスト
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 5.1, 5.2, 5.3_

- [x] 7.5 (P) パフォーマンステスト
  - 100トレードPortfolioで1秒以内抽出テスト
  - 共有マーケットデータで20%以上ノード削減達成テスト
  - 10,000ノードグラフでのタイムアウト動作テスト
  - _Requirements: 6.1, 6.2_

---

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 1.4, 2.1, 2.2, 2.3, 7.2 |
| 1.2 | 1.4, 2.3, 7.2 |
| 1.3 | 2.4, 7.2 |
| 1.4 | 1.1, 1.2, 2.3, 7.1 |
| 1.5 | 1.3, 1.4 |
| 2.1 | 3.1, 3.2, 7.3 |
| 2.2 | 3.1, 7.3 |
| 2.3 | 3.3, 7.3 |
| 2.4 | 3.2, 7.3 |
| 2.5 | 3.4, 7.3 |
| 3.1 | 2.1, 2.5, 7.2 |
| 3.2 | 2.5, 7.2 |
| 3.3 | 2.5, 7.2 |
| 3.4 | 2.5, 7.2 |
| 3.5 | 2.5, 2.6, 7.2 |
| 4.1 | 4.1 |
| 4.2 | 4.1 |
| 4.3 | 4.1 |
| 4.4 | 4.1 |
| 4.5 | 2.6, 4.2, 7.4 |
| 5.1 | 4.3, 7.4 |
| 5.2 | 4.3, 7.4 |
| 5.3 | 4.3, 7.4 |
| 5.4 | 5.1 |
| 5.5 | 5.2 |
| 6.1 | 7.5 |
| 6.2 | 2.4, 3.3, 7.5 |
| 6.3 | 4.4 |
| 6.4 | 2.2, 6.1 |
| 6.5 | 6.2 |

---

## Summary

- **Major Tasks**: 7
- **Sub-Tasks**: 25
- **Parallel Tasks (P)**: 11
- **Requirements Covered**: 30/30 (100%)
- **Estimated Task Size**: 1-3時間/サブタスク

---
_Generated: 2026-01-19_
