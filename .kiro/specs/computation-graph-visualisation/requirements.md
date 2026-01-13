# Requirements Document

## Introduction

本ドキュメントは、FrictionalBank デモの Web ダッシュボードに計算グラフの動的可視化機能を追加する要件を定義する。計算グラフとは、Enzyme ベースの自動微分（AD）エンジンが Greeks およびリスク感応度の計算に使用する依存関係グラフを指す。本機能により、ユーザーはダッシュボード上でプライシング計算の構造と依存関係をリアルタイムに視覚的に理解・分析できるようになる。

## Scope

- **対象**: FrictionalBank Web ダッシュボード（localhost:3000）への統合
- **除外**: スタンドアロン CLI 出力（DOT/JSON ファイル生成）、Python バインディング
- **活用**: 既存の D3.js、WebSocket、axum REST API 基盤

## Requirements

### Requirement 1: 計算グラフの抽出とデータ構造

**Objective:** As a クオンツ開発者, I want Enzyme AD 計算グラフをデータ構造として抽出する機能, so that Web ダッシュボードでグラフ構造を可視化できる。

#### Acceptance Criteria 1.1

1. When ユーザーがプライシング関数を実行する, the Graph Extractor shall 計算グラフのノードとエッジ情報を収集する。
2. The Graph Extractor shall 各ノードに対して一意の識別子、演算タイプ（加算、乗算、exp、log 等）、入力変数名を保持する。
3. The Graph Extractor shall エッジ情報として入力ノードと出力ノードの依存関係を記録する。
4. When グラフに微分対象パラメータが含まれる, the Graph Extractor shall 当該ノードを感応度計算の起点として明示的にマークする。
5. The Graph Data Structure shall JSON シリアライゼーションに対応し、REST API で返却可能な形式とする。

### Requirement 2: REST API エンドポイント

**Objective:** As a フロントエンド開発者, I want 計算グラフデータを取得する REST API, so that ダッシュボードでグラフを描画できる。

#### Acceptance Criteria 2.1

1. The Web Server shall `GET /api/graph` エンドポイントを提供し、現在のプライシング計算グラフを JSON 形式で返却する。
2. The API Response shall ノード配列（id, type, label, value）とエッジ配列（source, target）を含む。
3. When トレード ID が指定される（`GET /api/graph?trade_id=xxx`）, the API shall 当該トレードの計算グラフのみを返却する。
4. The API shall グラフのメタデータ（総ノード数、総エッジ数、グラフ深度、生成日時）を含める。
5. If 計算グラフが存在しない, then the API shall 適切なエラーメッセージ（404）を返す。
6. The API Response Time shall 10,000 ノード規模のグラフで 500ms 以内とする。

### Requirement 3: WebSocket リアルタイム更新

**Objective:** As a クオンツ・リサーチャー, I want 計算グラフの変更をリアルタイムで受信する機能, so that マーケットデータ更新時のグラフ変化を動的に確認できる。

#### Acceptance Criteria 3.1

1. The WebSocket Server shall 既存の `/api/ws` エンドポイントに `graph_update` メッセージタイプを追加する。
2. When マーケットデータが更新される, the WebSocket shall 影響を受けるノードの差分情報をブロードキャストする。
3. The WebSocket Message shall 更新されたノード ID リストと新しい値を含む。
4. When ユーザーがグラフ画面を表示中, the Dashboard shall 自動的にグラフを再描画する。
5. The WebSocket shall サブスクリプション機能を提供し、特定トレードのグラフ更新のみを受信可能とする。

### Requirement 4: D3.js インタラクティブ可視化

**Objective:** As a クオンツ・リサーチャー, I want ダッシュボード上で計算グラフをインタラクティブに操作できる機能, so that グラフ構造を直感的に探索できる。

#### Acceptance Criteria 4.1

1. The Dashboard shall 新規「Graph」タブを追加し、計算グラフ可視化画面を提供する。
2. The Graph Visualisation shall D3.js force-directed layout を使用してノードとエッジを描画する。
3. The Visualisation shall ノードのホバー時に演算詳細（演算タイプ、入力値、出力値、感応度）をツールチップで表示する。
4. The Visualisation shall ズーム、パン、ドラッグによるノード移動機能を提供する。
5. The Visualisation shall ノード検索機能を提供し、検索結果をハイライト表示する。
6. Where グラフのノード数が 500 を超える, the Visualisation shall 階層的な折りたたみ表示または LOD（Level of Detail）を提供する。
7. The Visualisation shall 微分対象パラメータから出力までのパスをハイライト表示する「Sensitivity Path」機能を提供する。
8. The Visualisation shall ノードタイプ別の色分け（入力: 青、中間演算: グレー、出力: 緑、微分対象: オレンジ）を適用する。

### Requirement 5: グラフ統計パネル

**Objective:** As a クオンツ開発者, I want 計算グラフの統計情報をダッシュボードで確認できる機能, so that 計算の複雑度を定量的に評価できる。

#### Acceptance Criteria 5.1

1. The Graph Screen shall 統計パネルを表示し、総ノード数、総エッジ数、グラフ深度を表示する。
2. The Statistics Panel shall 演算タイプ別のノード数を棒グラフまたは円グラフで表示する。
3. The Statistics Panel shall 微分対象パラメータごとの依存ノード数を表示する。
4. The Statistics Panel shall クリティカルパス（最長依存チェーン）の長さとノード列を表示する。
5. When ユーザーがクリティカルパスをクリックする, the Visualisation shall 当該パスをグラフ上でハイライト表示する。

### Requirement 6: FrictionalBank ワークフロー統合

**Objective:** As a デモ利用者, I want 既存の FrictionalBank ワークフロー（EOD、Intraday、Stress Test）で計算グラフを確認できること, so that プライシング処理の透明性を高められる。

#### Acceptance Criteria 6.1

1. When EOD バッチ処理が実行される, the Dashboard shall 各ステップ（Model Calibration、Pricing、XVA Calculation）のグラフを切り替え表示できる。
2. When Intraday リアルタイム処理中, the Dashboard shall マーケットデータ更新に連動してグラフをリアルタイム更新する。
3. When Stress Test シナリオが実行される, the Dashboard shall ベースケースとストレスケースのグラフを比較表示できる。
4. The Graph Screen shall ポートフォリオビューから特定トレードを選択し、そのグラフに遷移する機能を提供する。
5. The Graph Screen shall 既存の Bento Grid レイアウトおよびテーマ（ライト/ダーク）に準拠する。

### Requirement 7: パフォーマンスとメモリ効率

**Objective:** As a 開発者, I want 大規模な計算グラフでもダッシュボードがスムーズに動作すること, so that 本番環境相当のデータで検証できる。

#### Acceptance Criteria 7.1

1. The Graph Extractor shall 10,000 ノード規模のグラフを 1 秒以内に抽出する。
2. The D3.js Visualisation shall 10,000 ノードのグラフを 60fps でレンダリングする（WebGL 併用可）。
3. While グラフ抽出中, the Graph Extractor shall 元のプライシング計算のパフォーマンスに 5% 以上の影響を与えない。
4. The Frontend shall 仮想化または Canvas レンダリングを活用し、大規模グラフでもブラウザがフリーズしない。
5. The API shall ページネーションまたはストリーミングレスポンスをサポートし、大規模グラフの段階的読み込みを可能とする。
