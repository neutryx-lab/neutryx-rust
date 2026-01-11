# Requirements Document

## Introduction

FrictionalBankは、Neutryxライブラリの全機能を活用した仮想銀行システムのデモンストレーションです。A-I-P-S（Adapter → Infra → Pricer → Service）アーキテクチャの正しいデータフローに沿って、外部システムとの連携を含めた統合的なデモを提供します。

本仕様は以下の目的を達成します：
- Neutryxの全機能（プライシング、キャリブレーション、リスク計算、XVA）を統合的にデモンストレーション
- A-I-P-Sアーキテクチャの正しいデータフローを実演
- 外部システム（入力元・出力先）との連携パターンを提示
- 開発者・エンドユーザー向けの参照実装を提供

詳細仕様: [docs/demo/FRICTIONAL_BANK_SPEC.md](../../docs/demo/FRICTIONAL_BANK_SPEC.md)

---

## Requirements

### Requirement 1: サンプルデータ基盤

**Objective:** As a デモ実行者, I want サンプルデータが整備されていること, so that デモシナリオを即座に実行できる

#### Acceptance Criteria

1. The demo_data module shall 取引データサンプル（Equity、Rates、FX、Credit）を提供する
2. The demo_data module shall イールドカーブデータ（OIS、LIBOR）を提供する
3. The demo_data module shall ボラティリティサーフェスデータを提供する
4. The demo_data module shall クレジットスプレッドデータを提供する
5. The demo_data module shall カウンターパーティ・ネッティングセット・CSA設定データを提供する
6. The demo_data module shall 設定ファイル（neutryx.toml）のサンプルを提供する
7. The demo_data module shall 休日カレンダー（TARGET、NY、JP）のサンプルを提供する

---

### Requirement 2: 仮想入力システム（demo_inputs）

**Objective:** As a デモ実行者, I want 外部データソースをシミュレートする入力システム, so that A-I-P-Sアーキテクチャの入力フローを実演できる

#### Acceptance Criteria

##### 2.1 マーケットデータプロバイダー

1. The market_data_provider module shall Reuters風のティッカーストリームを生成する
2. The market_data_provider module shall Bloomberg風のスナップショットデータを生成する
3. The market_data_provider module shall 合成マーケットデータ（ランダムウォーク、平均回帰）を生成する
4. When マーケットデータが生成された場合, the market_data_provider module shall adapter_feedsが受信可能な形式で出力する

##### 2.2 取引データソース

5. The trade_source module shall フロントオフィスブッキングをシミュレートする
6. The trade_source module shall FpML形式の取引データを生成する
7. When 取引データが生成された場合, the trade_source module shall adapter_fpmlまたはadapter_loaderが受信可能な形式で出力する

##### 2.3 ファイルソース

8. The file_source module shall CSV形式のバルクデータを生成する
9. Where Parquet機能が有効な場合, the file_source module shall Parquet形式のデータを生成する

---

### Requirement 3: 仮想出力システム（demo_outputs）

**Objective:** As a デモ実行者, I want 外部システムをシミュレートする出力先, so that A-I-P-Sアーキテクチャの出力フローを実演できる

#### Acceptance Criteria

##### 3.1 規制当局システム

1. The regulatory module shall 規制報告APIのモックを提供する
2. The regulatory module shall 監査証跡ストアを提供する
3. When 規制報告を受信した場合, the regulatory module shall 受信内容をログに記録する

##### 3.2 決済システム

4. The settlement module shall SWIFT風メッセージ受信をシミュレートする
5. The settlement module shall ネッティング処理のモックを提供する
6. When 決済指図を受信した場合, the settlement module shall 決済内容をログに記録する

##### 3.3 リスクダッシュボード

7. The risk_dashboard module shall WebSocketサーバーとしてリスクメトリクスを受信する
8. The risk_dashboard module shall 受信したメトリクスを蓄積・集計する
9. When リスクメトリクスを受信した場合, the risk_dashboard module shall リアルタイムで更新状況を表示する

##### 3.4 レポート出力先

10. The report_sink module shall ファイル出力（CSV、JSON）をサポートする
11. The report_sink module shall メール送信のモックを提供する

---

### Requirement 4: デモオーケストレーター（frictional_bank）

**Objective:** As a デモ実行者, I want 統合ワークフローを制御するオーケストレーター, so that 複数のデモシナリオを実行できる

#### Acceptance Criteria

##### 4.1 EODバッチ処理

1. The orchestrator module shall EODバッチワークフローを実行する
2. When EODバッチが開始された場合, the orchestrator module shall 以下の順序で処理を実行する：取引データ読込 → マーケットデータ読込 → キャリブレーション → 評価 → XVA計算 → 規制報告生成 → 決済指図生成
3. The orchestrator module shall 各処理ステップの進捗状況を報告する
4. If 処理中にエラーが発生した場合, the orchestrator module shall エラー内容をログに記録し、可能な限り処理を継続する

##### 4.2 イントラデイ処理

5. The orchestrator module shall イントラデイリアルタイム処理を実行する
6. When ティッカーストリームを受信した場合, the orchestrator module shall ポートフォリオを再評価する
7. The orchestrator module shall リスクメトリクスをリアルタイムで更新する

##### 4.3 ストレステスト

8. The orchestrator module shall ストレステストシナリオを実行する
9. The orchestrator module shall 複数のシナリオ（金利ショック、ボラティリティショック、クレジットイベント）をサポートする

##### 4.4 デモ設定

10. The orchestrator module shall 設定ファイル（demo_config.toml）から設定を読み込む
11. The orchestrator module shall デモモード（フル、クイック、カスタム）を選択可能にする

---

### Requirement 5: ターミナルUI（demo_gui TUI）

**Objective:** As a デモ実行者, I want ターミナルベースのダッシュボード, so that デモの進行状況とリスクメトリクスを視覚的に確認できる

#### Acceptance Criteria

1. The tui module shall ratatuiを使用したターミナルUIを提供する
2. The tui module shall ダッシュボード画面（ポートフォリオサマリー、リスクメトリクス）を表示する
3. The tui module shall ポートフォリオビュー（取引一覧、PV、Greeks）を表示する
4. The tui module shall リスクビュー（CVA、DVA、FVA、Exposure）を表示する
5. The tui module shall トレードブロッター（取引詳細）を表示する
6. Where チャート機能が有効な場合, the tui module shall 時系列チャート（エクスポージャー推移）を描画する
7. The tui module shall service_gatewayのREST APIからデータを取得する
8. When キーボードショートカットが入力された場合, the tui module shall 画面を切り替える

---

### Requirement 6: Jupyter連携（demo_notebooks）

**Objective:** As a クオンツアナリスト, I want Jupyterノートブックでの分析環境, so that インタラクティブな検証と可視化ができる

#### Acceptance Criteria

1. The notebooks shall プライシングデモ（バニラオプション、エキゾチック）を提供する
2. The notebooks shall キャリブレーションデモ（Heston、SABR、Hull-White）を提供する
3. The notebooks shall リスク分析デモ（Greeks、シナリオ分析）を提供する
4. The notebooks shall XVA計算デモ（CVA、DVA、FVA）を提供する
5. The notebooks shall パフォーマンスベンチマークデモ（Enzyme vs num-dual）を提供する
6. The notebooks shall service_pythonバインディングを使用してNeutryx機能を呼び出す
7. The notebooks shall matplotlib/plotlyを使用した可視化を含む

---

### Requirement 7: A-I-P-Sデータフロー準拠

**Objective:** As a アーキテクト, I want 全コンポーネントがA-I-P-Sデータフローに準拠すること, so that アーキテクチャの正しさを実証できる

#### Acceptance Criteria

1. The demo_inputs module shall Adapter層（adapter_feeds、adapter_fpml、adapter_loader）にのみデータを供給する
2. The demo_outputs module shall Service層（service_cli、service_gateway）からのみデータを受信する
3. The frictional_bank module shall A-I-P-S順序（Adapter → Infra → Pricer → Service）でデータを処理する
4. The demo_gui module shall service_gatewayのAPIを介してのみPricer層の結果にアクセスする
5. If A-I-P-S順序に違反するデータフローが検出された場合, the system shall コンパイルエラーまたは実行時エラーを発生させる

---

### Requirement 8: Webダッシュボード（demo_gui Web）- オプション

**Objective:** As a デモ実行者, I want ブラウザベースのダッシュボード, so that リモートからデモを監視できる

#### Acceptance Criteria

1. Where Web機能が有効な場合, the web module shall ブラウザベースのダッシュボードを提供する
2. Where Web機能が有効な場合, the web module shall service_gatewayのREST APIを呼び出す
3. Where Web機能が有効な場合, the web module shall WebSocketでリアルタイム更新を受信する
4. Where Web機能が有効な場合, the web module shall レスポンシブなUIを提供する

---

### Requirement 9: 非機能要件

**Objective:** As a システム管理者, I want 安定した実行環境, so that デモを確実に実行できる

#### Acceptance Criteria

##### 9.1 パフォーマンス

1. The frictional_bank module shall 100取引のEODバッチを60秒以内に完了する
2. The frictional_bank module shall リアルタイム再評価を100ms以内に完了する

##### 9.2 エラーハンドリング

3. The system shall 全てのエラーを構造化されたログとして出力する
4. If 致命的でないエラーが発生した場合, the system shall 処理を継続する

##### 9.3 設定可能性

5. The system shall 環境変数による設定オーバーライドをサポートする
6. The system shall ログレベルの動的変更をサポートする

##### 9.4 ドキュメント

7. The demo module shall README.mdを含む
8. The demo module shall 各デモシナリオの実行手順を文書化する

---

## Summary

| 要件カテゴリ | 要件数 | 主要コンポーネント |
|------------|--------|------------------|
| サンプルデータ | 7 | demo/data |
| 仮想入力システム | 9 | demo/inputs |
| 仮想出力システム | 11 | demo/outputs |
| オーケストレーター | 11 | demo/frictional_bank |
| ターミナルUI | 8 | demo/gui/tui |
| Jupyter連携 | 7 | demo/notebooks |
| A-I-P-Sデータフロー | 5 | 全体 |
| Webダッシュボード | 4 | demo/gui/web |
| 非機能要件 | 8 | 全体 |
| **合計** | **70** | - |
