# Implementation Plan

## Overview

FrictionalBankデモシステムの実装タスク。既存demo/inputs、demo/outputsを活用し、新規frictional_bank、demo_gui、demo_data、notebooksを作成する。

---

## Tasks

- [x] 1. Workspace統合とプロジェクト構造セットアップ
- [x] 1.1 (P) 既存demoクレートのworkspace登録
  - demo/inputs、demo/outputsをCargo.toml workspace.membersに追加
  - 依存関係の解決確認（cargo check）
  - 既存コードのコンパイルエラー修正
  - _Requirements: 2.1, 2.2, 3.1, 3.4_

- [x] 1.2 (P) 新規クレート作成とディレクトリ構造準備
  - demo/frictional_bank クレート作成（Cargo.toml、lib.rs）
  - demo/gui クレート作成（TUI用）
  - demo/data ディレクトリ構造作成
  - demo/notebooks ディレクトリ作成
  - _Requirements: 1.1, 4.1, 5.1_

---

- [x] 2. サンプルデータ基盤構築
- [x] 2.1 (P) 取引サンプルデータ作成
  - Equity取引（オプション、フォワード）CSVファイル作成
  - Rates取引（IRS、スワップション）CSVファイル作成
  - FX取引（FXフォワード、FXオプション）CSVファイル作成
  - Credit取引（CDS）CSVファイル作成
  - FpML形式のIRSサンプルXML作成
  - adapter_loader互換フォーマット確認
  - _Requirements: 1.1_

- [x] 2.2 (P) マーケットデータサンプル作成
  - イールドカーブデータ（USD OIS、EUR ESTR）CSV作成
  - ボラティリティサーフェス（Equity、FX）CSV作成
  - クレジットスプレッドデータCSV作成
  - _Requirements: 1.2, 1.3, 1.4_

- [x] 2.3 (P) マスターデータと設定サンプル作成
  - カウンターパーティマスターCSV作成
  - ネッティングセット・CSA設定CSV作成
  - neutryx.toml サンプル設定ファイル作成
  - demo_config.toml デモ設定ファイル作成
  - 休日カレンダー（TARGET、NY、JP）CSV作成
  - _Requirements: 1.5, 1.6, 1.7_

---

- [x] 3. 仮想入力システム拡張
- [x] 3.1 合成マーケットデータジェネレーター強化
  - ランダムウォークベースの価格生成ロジック追加
  - 平均回帰モデルの価格生成ロジック追加
  - adapter_feeds互換形式での出力確認
  - _Requirements: 2.3, 2.4_

- [x] 3.2 取引データソース実装
  - フロントオフィスブッキングシミュレーター作成
  - FpML形式取引データ生成機能追加
  - adapter_fpml/adapter_loader互換出力
  - _Requirements: 2.5, 2.6, 2.7_

- [x] 3.3 (P) ファイルソース実装
  - CSV形式バルクデータ生成機能
  - Parquet形式生成（feature flag対応）
  - _Requirements: 2.8, 2.9_

---

- [x] 4. 仮想出力システム拡張
- [x] 4.1 (P) 規制当局システムモック完成
  - RegulatorApi受信ログ記録機能追加
  - 監査証跡ストア永続化機能追加
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 4.2 (P) 決済システムモック完成
  - SWIFT風メッセージ受信シミュレーション
  - ネッティング処理モックロジック
  - 決済内容ログ記録
  - _Requirements: 3.4, 3.5, 3.6_

- [x] 4.3 (P) リスクダッシュボードシンク強化
  - WebSocketサーバーでのメトリクス受信
  - 受信メトリクス蓄積・集計機能
  - リアルタイム更新状況の表示
  - _Requirements: 3.7, 3.8, 3.9_

- [x] 4.4 (P) レポート出力先実装
  - ファイル出力（CSV、JSON）機能
  - メール送信モック実装
  - _Requirements: 3.10, 3.11_

---

- [x] 5. オーケストレーターコア実装
- [x] 5.1 DemoConfig設定管理実装
  - demo_config.toml読込機能（tomlクレート使用）
  - 環境変数オーバーライド対応
  - DemoMode（Full/Quick/Custom）選択機能
  - デフォルト値とバリデーション
  - _Requirements: 4.10, 4.11, 9.5_

- [x] 5.2 DemoWorkflow trait定義
  - trait定義（name、run、cancel）
  - ProgressCallback型定義
  - WorkflowStep enum定義
  - WorkflowResult構造体定義
  - _Requirements: 4.1, 4.3_

- [x] 5.3 エラーハンドリング基盤
  - DemoError enum定義（thiserror使用）
  - 構造化ログ出力設定
  - 非致命的エラー時の処理継続ロジック
  - _Requirements: 4.4, 9.3, 9.4_

---

- [x] 6. EODバッチワークフロー実装
- [x] 6.1 EodBatchWorkflow構造体実装
  - DemoWorkflow trait実装
  - A-I-P-S順序でのシーケンシャル処理設計
  - 取引データ読込ステップ
  - マーケットデータ読込ステップ
  - _Requirements: 4.1, 4.2, 7.3_

- [x] 6.2 キャリブレーション・評価ステップ実装
  - pricer_risk::demo活用でのキャリブレーション呼び出し
  - run_portfolio_pricing統合
  - XVA計算ステップ統合
  - _Requirements: 4.2_

- [x] 6.3 レポート・出力ステップ実装
  - 規制報告生成とdemo_outputs送信
  - 決済指図生成とdemo_outputs送信
  - 各ステップ進捗報告コールバック発火
  - _Requirements: 4.2, 4.3_

---

- [x] 7. イントラデイワークフロー実装
- [x] 7.1 IntradayWorkflow構造体実装
  - DemoWorkflow trait実装
  - MarketDataProvider async_channel購読設定
  - ティッカー受信イベントループ
  - _Requirements: 4.5, 4.6_

- [x] 7.2 リアルタイム再評価ロジック実装
  - ポートフォリオ再評価処理
  - RiskMetricsUpdatedイベント発行
  - demo_outputs::risk_dashboardへの送信
  - 100ms以内の応答性確保
  - _Requirements: 4.6, 4.7, 9.2_

---

- [x] 8. ストレステストワークフロー実装
- [x] 8.1 StressTestWorkflow構造体実装
  - DemoWorkflow trait実装
  - pricer_risk::scenarios::ScenarioEngine統合
  - PresetScenarioType enum定義
  - _Requirements: 4.8_

- [x] 8.2 シナリオ実行ロジック実装
  - 金利ショックシナリオ（±100bp）
  - ボラティリティショックシナリオ（+20%）
  - クレジットイベントシナリオ
  - シナリオ別P&L計算と結果集約
  - _Requirements: 4.9_

---

- [x] 9. ターミナルUI実装
- [x] 9.1 TuiApp基本構造実装
  - ratatui + crossterm依存追加
  - TuiApp構造体とScreen enum定義
  - イベントループ（crossterm EventStream）
  - 終了処理（Ctrl+C、q）
  - _Requirements: 5.1, 5.8_

- [x] 9.2 ダッシュボード画面実装
  - ポートフォリオサマリーウィジェット
  - リスクメトリクスサマリーウィジェット
  - レイアウト構成（Block、Layout）
  - _Requirements: 5.2_

- [x] 9.3 ポートフォリオビュー実装
  - 取引一覧テーブル（Table widget）
  - PV、Greeks列表示
  - 行選択ナビゲーション
  - _Requirements: 5.3_

- [x] 9.4 リスクビュー実装
  - CVA、DVA、FVA、Exposure表示
  - ネッティングセット別集計
  - _Requirements: 5.4_

- [x] 9.5 トレードブロッター実装
  - 選択取引の詳細表示
  - 取引属性一覧
  - _Requirements: 5.5_

- [x] 9.6 (P) APIクライアント実装
  - reqwest HTTPクライアント設定
  - service_gateway /portfolio エンドポイント呼び出し
  - service_gateway /exposure エンドポイント呼び出し
  - レスポンスキャッシュ機構
  - _Requirements: 5.7_

- [x] 9.7 画面切替とキーボードショートカット
  - 1-4キーで画面切替
  - Tab/Shift+Tabでフォーカス移動
  - 矢印キーでリスト選択
  - _Requirements: 5.8_

- [x] 9.8 時系列チャート実装（オプション）
  - エクスポージャー推移チャート
  - ratatuiのChart widget活用
  - _Requirements: 5.6_

---

- [x] 10. Jupyterノートブック作成
- [x] 10.1 (P) プライシングデモノートブック
  - バニラオプション評価デモ
  - エキゾチック商品評価デモ
  - service_python呼び出しコード
  - matplotlib/plotly可視化
  - _Requirements: 6.1, 6.6, 6.7_

- [x] 10.2 (P) キャリブレーションデモノートブック
  - Hestonモデルキャリブレーション
  - SABRモデルキャリブレーション
  - Hull-Whiteモデルキャリブレーション
  - パラメータ可視化
  - _Requirements: 6.2, 6.6, 6.7_

- [x] 10.3 (P) リスク分析デモノートブック
  - Greeks計算と感応度分析
  - シナリオ分析と結果可視化
  - _Requirements: 6.3, 6.6, 6.7_

- [x] 10.4 (P) XVA計算デモノートブック
  - CVA、DVA、FVA計算デモ
  - エクスポージャープロファイル可視化
  - _Requirements: 6.4, 6.6, 6.7_

- [x] 10.5 (P) パフォーマンスベンチマークノートブック
  - Enzyme vs num-dual比較
  - 処理時間計測と可視化
  - _Requirements: 6.5, 6.6, 6.7_

---

- [x] 11. A-I-P-Sデータフロー検証と統合テスト
- [x] 11.1 A-I-P-Sフロー検証テスト作成
  - demo_inputs → Adapter層接続テスト
  - Service層 → demo_outputs接続テスト
  - A-I-P-S順序違反時のエラー検出テスト
  - _Requirements: 7.1, 7.2, 7.5_

- [x] 11.2 TUI-Gateway統合テスト
  - TUIからservice_gateway API呼び出しテスト
  - Pricer層結果のService経由アクセス確認
  - _Requirements: 7.4_

- [x] 11.3 E2Eインテグレーションテスト
  - EODバッチフル実行テスト（サンプルデータ使用）
  - 100取引60秒以内完了確認
  - イントラデイフロー動作確認
  - _Requirements: 7.3, 9.1_

- [x] 11.4 ログレベル動的変更機能
  - 環境変数によるログレベル切替
  - 実行時ログレベル変更対応
  - _Requirements: 9.6_

---

- [x] 12. Webダッシュボード実装（オプション）
- [x] 12.1 (P) Webフロントエンド基盤
  - ブラウザベースダッシュボードUI
  - レスポンシブレイアウト
  - _Requirements: 8.1, 8.4_

- [x] 12.2 (P) Web-Gateway統合
  - service_gateway REST API呼び出し
  - WebSocketリアルタイム更新受信
  - _Requirements: 8.2, 8.3_

---

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 1.2, 2.1 |
| 1.2 | 2.2 |
| 1.3 | 2.2 |
| 1.4 | 2.2 |
| 1.5 | 2.3 |
| 1.6 | 2.3 |
| 1.7 | 2.3 |
| 2.1 | 1.1 |
| 2.2 | 1.1 |
| 2.3 | 3.1 |
| 2.4 | 3.1 |
| 2.5 | 3.2 |
| 2.6 | 3.2 |
| 2.7 | 3.2 |
| 2.8 | 3.3 |
| 2.9 | 3.3 |
| 3.1 | 1.1, 4.1 |
| 3.2 | 4.1 |
| 3.3 | 4.1 |
| 3.4 | 1.1, 4.2 |
| 3.5 | 4.2 |
| 3.6 | 4.2 |
| 3.7 | 4.3 |
| 3.8 | 4.3 |
| 3.9 | 4.3 |
| 3.10 | 4.4 |
| 3.11 | 4.4 |
| 4.1 | 1.2, 5.2, 6.1 |
| 4.2 | 6.1, 6.2, 6.3 |
| 4.3 | 5.2, 6.3 |
| 4.4 | 5.3 |
| 4.5 | 7.1 |
| 4.6 | 7.1, 7.2 |
| 4.7 | 7.2 |
| 4.8 | 8.1 |
| 4.9 | 8.2 |
| 4.10 | 5.1 |
| 4.11 | 5.1 |
| 5.1 | 1.2, 9.1 |
| 5.2 | 9.2 |
| 5.3 | 9.3 |
| 5.4 | 9.4 |
| 5.5 | 9.5 |
| 5.6 | 9.8 |
| 5.7 | 9.6 |
| 5.8 | 9.1, 9.7 |
| 6.1 | 10.1 |
| 6.2 | 10.2 |
| 6.3 | 10.3 |
| 6.4 | 10.4 |
| 6.5 | 10.5 |
| 6.6 | 10.1, 10.2, 10.3, 10.4, 10.5 |
| 6.7 | 10.1, 10.2, 10.3, 10.4, 10.5 |
| 7.1 | 11.1 |
| 7.2 | 11.1 |
| 7.3 | 6.1, 11.3 |
| 7.4 | 11.2 |
| 7.5 | 11.1 |
| 8.1 | 12.1 |
| 8.2 | 12.2 |
| 8.3 | 12.2 |
| 8.4 | 12.1 |
| 9.1 | 11.3 |
| 9.2 | 7.2 |
| 9.3 | 5.3 |
| 9.4 | 5.3 |
| 9.5 | 5.1 |
| 9.6 | 11.4 |
| 9.7 | Deferred (README作成は実装範囲外) |
| 9.8 | Deferred (ドキュメント作成は実装範囲外) |

**Deferred Requirements Rationale**:
- 9.7, 9.8: タスク生成ルールによりドキュメント作成タスクは除外。実装完了後に別途対応。
