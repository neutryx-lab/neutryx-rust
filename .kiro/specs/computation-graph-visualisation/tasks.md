# Implementation Plan

## Task Format

各タスクは要件を完全にカバーし、A-I-P-S アーキテクチャに従って実装する。グラフ抽出は Pricer レイヤー、API/UI は Service レイヤーに配置。

---

- [ ] 1. 計算グラフのデータ構造定義
- [x] 1.1 (P) グラフノードとエッジの Rust 構造体を定義する
  - `GraphNode` 構造体を作成し、一意識別子、演算タイプ、ラベル、値、感応度フラグ、グループを保持する
  - `GraphEdge` 構造体を作成し、ソースノード ID、ターゲットノード ID、オプションの重みを保持する
  - `NodeType` 列挙型を定義し、入力、加算、乗算、exp、log、sqrt、div、出力、カスタムをサポートする
  - `NodeGroup` 列挙型を定義し、入力、中間、出力、感応度をサポートする
  - すべての構造体に `#[derive(Debug, Clone, Serialize)]` を付与し JSON シリアライゼーションに対応する
  - D3.js 互換のフィールド名（`type` → `node_type`、`links` → `edges`）にマッピングする
  - _Requirements: 1.2, 1.3, 1.4, 1.5_

- [x] 1.2 (P) ComputationGraph とメタデータ構造体を定義する
  - `ComputationGraph` 構造体を作成し、ノード配列、エッジ配列、メタデータを保持する
  - `GraphMetadata` 構造体を作成し、トレード ID、総ノード数、総エッジ数、グラフ深度、生成日時を保持する
  - エッジフィールドを `#[serde(rename = "links")]` で D3.js 互換にする
  - 生成日時は ISO 8601 形式の文字列とする
  - `find_node`、`find_path`、`get_critical_path` メソッドを実装する
  - _Requirements: 1.5, 2.4_

- [x] 1.3 (P) グラフエラー型を定義する
  - `GraphError` 列挙型を作成し、TradeNotFound、ExtractionFailed、Timeout バリアントを定義する
  - `#[derive(Debug, Clone, Serialize)]` を付与しエラーレスポンスに対応する
  - 各バリアントに対応する HTTP ステータスコード（404、500）のマッピングを定義する
  - _Requirements: 2.5_

- [ ] 2. グラフ抽出トレイトと実装
- [x] 2.1 GraphExtractable トレイトを定義する
  - `extract_graph` メソッドを定義し、オプションの trade_id を受け取り `ComputationGraph` を返す
  - `extract_affected_nodes` メソッドを定義し、差分更新用の `GraphNodeUpdate` 配列を返す
  - `GraphNodeUpdate` 構造体を作成し、ノード ID、新しい値、変化量（delta）を保持する
  - トレイトを pricer_pricing の graph モジュールに配置する
  - _Requirements: 1.1, 3.2, 3.3_

- [x] 2.2 グラフ抽出の基本実装を作成する
  - `SimpleGraphExtractor` 構造体を作成し、`GraphExtractable` トレイトを実装する
  - プライシング関数のノードとエッジ情報を収集するロジックを実装する
  - 微分対象パラメータを `is_sensitivity_target: true` でマークする
  - グラフ深度（最長パス）を計算するアルゴリズムを実装する
  - グラフが DAG（非循環有向グラフ）であることを検証する
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 2.3 グラフ抽出のパフォーマンス最適化を行う
  - 10,000 ノード規模のグラフを 1 秒以内に抽出できるよう最適化する
  - プライシング計算への影響を 5% 以内に制限する
  - ノード・エッジの事前割り当てバッファを使用してメモリアロケーションを削減する
  - Criterion ベンチマークを追加して抽出時間を計測する
  - _Requirements: 7.1, 7.3_

- [ ] 3. REST API エンドポイント実装
- [x] 3.1 /api/graph GET ハンドラを実装する
  - `get_graph` 非同期ハンドラ関数を作成する
  - `State<Arc<AppState>>` と `Query<GraphQueryParams>` を受け取る
  - `GraphQueryParams` 構造体を定義し、オプションの `trade_id` フィールドを持つ
  - `GraphResponse` 構造体を定義し、nodes、links、metadata を含む
  - グラフが存在しない場合は 404 エラーレスポンスを返す
  - 既存の handlers.rs パターンに従って実装する
  - _Requirements: 2.1, 2.2, 2.3, 2.5_

- [x] 3.2 API ルーティングを追加する
  - `demo/gui/src/web/mod.rs` のルーターに `/api/graph` ルートを追加する
  - 既存の REST API エンドポイント（/api/portfolio、/api/metrics 等）と一貫したパターンに従う
  - _Requirements: 2.1_

- [x] 3.3 API レスポンスタイムの最適化を行う
  - 500ms 以内のレスポンスタイムを達成する
  - トレード ID ベースのグラフキャッシュ（TTL: 5 秒）を実装する
  - 大規模グラフ向けのページネーションまたはストリーミングレスポンスをサポートする
  - _Requirements: 2.6, 7.5_

- [ ] 4. WebSocket グラフ更新機能
- [x] 4.1 graph_update メッセージタイプを追加する
  - `RealTimeUpdate` 構造体に `graph_update` メッセージタイプを追加する
  - `graph_update` コンストラクタメソッドを実装し、trade_id と updated_nodes を受け取る
  - メッセージペイロードに trade_id と更新ノード配列を含める
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 4.2 グラフ更新のブロードキャスト機能を実装する
  - マーケットデータ更新時に影響ノードの差分をブロードキャストする
  - 既存の `broadcast::channel` パターンを使用する
  - 高頻度更新時のメッセージバッチ化を検討する
  - _Requirements: 3.2, 3.4_

- [x] 4.3 (P) サブスクリプション機能を実装する
  - クライアントが特定トレードのグラフ更新のみを購読できる機能を追加する
  - 購読中のトレード ID セットを AppState で管理する
  - サブスクリプション要求メッセージを処理するロジックを追加する
  - _Requirements: 3.5_

- [ ] 5. フロントエンド Graph タブの基盤実装
- [x] 5.1 Graph タブと GraphManager クラスを実装する
  - 新規「Graph」タブを Bento Grid レイアウトに追加する
  - `navigateTo('graph')` でグラフ画面に遷移できるようにする
  - `GraphManager` クラスを作成し、グラフデータの取得と状態管理を行う
  - `fetchGraph(tradeId)` メソッドで REST API からグラフを取得する
  - `handleGraphUpdate(message)` メソッドで WebSocket 差分更新を処理する
  - サブスクリプション管理（subscribe/unsubscribe）を実装する
  - 既存のライト/ダークテーマに準拠する
  - _Requirements: 4.1, 3.4, 6.5_

- [x] 5.2 D3.js force-directed グラフレンダリングを実装する
  - `initGraphView()` 関数を作成し、SVG 要素と D3 force simulation を初期化する
  - `renderGraph(data)` 関数を作成し、ノードとエッジを描画する
  - D3.js force-directed layout（forceSimulation, forceLink, forceManyBody, forceCenter）を設定する
  - ノードを円形、エッジを線で描画する
  - ノードタイプ別の色分けを適用する（入力: 青、中間: グレー、出力: 緑、感応度: オレンジ）
  - _Requirements: 4.2, 4.8_

- [x] 5.3 ズーム、パン、ドラッグ機能を実装する
  - D3.js zoom behavior を設定しズーム・パン操作を有効にする
  - ノードドラッグで位置を移動できるようにする
  - ズームレベルに応じてラベル表示を調整する
  - _Requirements: 4.4_

- [ ] 6. インタラクティブ機能の実装
- [x] 6.1 ノードホバーとツールチップを実装する
  - ノードホバー時にツールチップを表示する
  - ツールチップに演算タイプ、入力値、出力値、感応度情報を表示する
  - ツールチップのスタイルを既存テーマに合わせる
  - _Requirements: 4.3_

- [x] 6.2 ノード検索とハイライト機能を実装する
  - 検索入力フィールドを追加する
  - 検索クエリに一致するノードをハイライト表示する
  - 検索結果ノードを画面中央にフォーカスする
  - _Requirements: 4.5_

- [ ] 6.3 Sensitivity Path ハイライト機能を実装する
  - 微分対象パラメータから出力までのパスを計算する
  - パス上のノードとエッジをハイライト表示する
  - パスハイライトのトグル機能を追加する
  - _Requirements: 4.7_

- [ ] 7. 統計パネルの実装
- [x] 7.1 (P) 基本統計表示を実装する
  - 統計パネルを Graph 画面に追加する
  - 総ノード数、総エッジ数、グラフ深度を表示する
  - 生成日時を表示する
  - _Requirements: 5.1_

- [ ] 7.2 (P) 演算タイプ別統計チャートを実装する
  - 演算タイプ別のノード数を集計する
  - 棒グラフまたは円グラフで表示する
  - 既存の D3.js チャートスタイルに準拠する
  - _Requirements: 5.2_

- [ ] 7.3 感応度依存ノード統計を実装する
  - 微分対象パラメータごとの依存ノード数を計算する
  - リスト形式で表示する
  - パラメータをクリックしてグラフ上でフィルタリングできるようにする
  - _Requirements: 5.3_

- [ ] 7.4 クリティカルパス表示機能を実装する
  - クリティカルパス（最長依存チェーン）を計算する
  - パスの長さとノード列を表示する
  - クリティカルパスをクリックでグラフ上にハイライト表示する
  - _Requirements: 5.4, 5.5_

- [ ] 8. 大規模グラフ対応
- [ ] 8.1 Canvas レンダリングモードを実装する
  - 500 ノード超で Canvas レンダリングに自動切り替えする
  - D3.js + Canvas の描画ロジックを実装する
  - SVG モードと同等のインタラクション（ホバー、クリック）をサポートする
  - レンダリングモード切り替えの状態管理を追加する
  - _Requirements: 7.2, 7.4_

- [ ] 8.2 LOD（Level of Detail）機能を実装する
  - 10,000 ノード超で階層的な折りたたみ表示を有効にする
  - ノードのクラスタリングロジックを実装する
  - クラスタの展開/折りたたみ操作を追加する
  - _Requirements: 4.6_

- [ ] 8.3 WebSocket 更新による差分レンダリングを実装する
  - `updateGraphNodes(updatedNodes)` 関数を作成する
  - 更新されたノードのみを再描画する
  - 値の変化をアニメーションで表示する
  - _Requirements: 3.4, 7.2_

- [ ] 9. FrictionalBank ワークフロー統合
- [ ] 9.1 EOD バッチ処理のグラフ表示を実装する
  - EOD 処理実行時に各ステップ（Model Calibration、Pricing、XVA Calculation）のグラフを表示する
  - ステップ切り替えセレクタを追加する
  - 処理進行に応じてグラフを自動更新する
  - _Requirements: 6.1_

- [ ] 9.2 Intraday リアルタイム更新統合を実装する
  - マーケットデータ更新に連動してグラフをリアルタイム更新する
  - WebSocket graph_update メッセージを Graph 画面で自動反映する
  - 更新頻度の調整オプションを追加する
  - _Requirements: 6.2_

- [ ] 9.3 Stress Test シナリオ比較機能を実装する
  - ベースケースとストレスケースのグラフを並列表示する
  - 差分ノード（値が変化したノード）をハイライトする
  - シナリオ選択セレクタを追加する
  - _Requirements: 6.3_

- [ ] 9.4 ポートフォリオビューからグラフ遷移を実装する
  - ポートフォリオビューでトレードを選択してグラフ画面に遷移する機能を追加する
  - 選択したトレード ID をパラメータとして渡す
  - 戻るナビゲーションを追加する
  - _Requirements: 6.4_

- [ ] 10. 統合テストとパフォーマンス検証
- [ ] 10.1 ユニットテストを追加する
  - `GraphExtractor::extract_graph` の正常系、空グラフ、大規模グラフテスト
  - `ComputationGraph::find_path` のパス探索テスト
  - `GraphNode`、`GraphEdge` のシリアライゼーションテスト
  - `NodeType`、`NodeGroup` 列挙値テスト
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [ ] 10.2 統合テストを追加する
  - REST API `/api/graph` エンドポイントの正常系、404、タイムアウトテスト
  - WebSocket `graph_update` ブロードキャストテスト
  - PricingContext → GraphExtractor 連携テスト
  - _Requirements: 2.1, 2.2, 2.3, 2.5, 3.1, 3.2_

- [ ] 10.3 パフォーマンステストを追加する
  - 10,000 ノードのグラフ抽出時間 < 1 秒を検証する
  - API レスポンス時間 < 500ms を検証する
  - プライシング計算への影響 < 5% を検証する
  - _Requirements: 7.1, 7.3, 2.6_

- [ ]* 10.4 E2E/UI テストを追加する
  - Graph タブナビゲーションテスト
  - ノードホバーでツールチップ表示テスト
  - ズーム・パン操作テスト
  - 検索機能テスト
  - _Requirements: 4.1, 4.3, 4.4, 4.5_
