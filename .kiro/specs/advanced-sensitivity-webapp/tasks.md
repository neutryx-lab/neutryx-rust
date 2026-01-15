# Implementation Plan

## Overview

本タスクリストは、Sensitivity（Greeks）計算の精細化と FrictionalWebApp の機能拡張を実装する。A-I-P-S アーキテクチャに従い、Pricer Layer → WebApp Layer → Frontend の順序で実装を進める。

---

## Tasks

### Pricer Layer Foundation

- [ ] 1. リスクファクター識別子の基盤実装
- [x] 1.1 (P) リスクファクター識別子 enum の定義と基本 trait 実装
  - 原資産、カーブ、ボラティリティサーフェスの 3 種別を識別する enum を追加
  - `Display`, `Hash`, `Eq`, `Clone` trait を実装
  - Serde 互換シリアライズ（feature flag `serde`）を追加
  - 単体テストでハッシュ衝突なし・等価性を検証
  - _Requirements: 1.3_

- [x] 1.2 ファクター毎 Greeks 結果構造体の実装
  - リスクファクター毎の Greeks 計算結果を保持する構造体を追加
  - 全ファクター合計の計算メソッド `total()` を実装
  - 計算モード（AAD/Bump）と計算時間を保持
  - AD 互換性維持（ジェネリック `T: Float`）
  - _Requirements: 1.1, 1.2, 1.3, 1.5_

- [x] 1.3 IRS Greeks 計算機にファクター別計算機能を追加
  - 既存 `IrsGreeksCalculator` にリスクファクター識別と分類ロジックを追加
  - カーブ毎の Delta、原資産毎の Gamma 等をファクター別に出力
  - NaN/Inf 検出とエラーハンドリングを実装
  - 一次・二次 Greeks 両方をファクター別に計算可能にする
  - _Requirements: 1.1, 1.2, 1.6_

### Bucket DV01 and Key Rate Duration

- [x] 2. バケット感応度計算機能の実装
- [x] 2.1 (P) バケット DV01 計算ロジックの実装
  - 標準テナーポイント（1M, 3M, 6M, 1Y, 2Y, 5Y, 10Y, 20Y, 30Y）に対する感応度計算
  - 各テナーに 1bp シフトを適用し、価格変化を計測
  - バケット合計と総 DV01 の整合性検証ロジックを追加
  - 計算結果の構造体とシリアライズ対応
  - _Requirements: 2.1, 2.3_
  - ✅ `BucketDv01Calculator` 実装完了（16 テスト合格）

- [x] 2.2 (P) Key Rate Duration 計算の実装
  - 指定されたキーレートポイントに対する修正デュレーションを計算
  - 任意のテナーポイント配列を受け付ける柔軟な API
  - 計算結果の正規化（年率換算）
  - _Requirements: 2.2_
  - ✅ `KeyRateDurationResult` 実装完了（bucket_dv01 モジュール内）

- [x] 2.3 カーブシフト機能の拡充
  - パラレルシフト（全テナー均一シフト）の実装
  - バタフライシフト（短期・長期の非均一シフト）の実装
  - シフト適用後のカーブ再構築
  - _Requirements: 2.4_
  - ✅ `CurveShifter` 実装完了（19 テスト合格）

### Performance Optimization

- [x] 3. パフォーマンス最適化と並列処理
- [x] 3.1 大規模ポートフォリオの Rayon 並列処理
  - 1000 件以上のトレードに対する並列 Greeks 計算を実装
  - ワークスティーリングによる負荷分散の最適化
  - 計算結果の集約ロジック（ファクター毎集計）
  - _Requirements: 3.1_
  - ✅ `ParallelPortfolioGreeksCalculator` 実装完了（23 テスト合格）

- [x] 3.2 AAD モードのパフォーマンスベンチマーク
  - Criterion ベンチマークスイートの追加
  - Bump-and-Revalue との速度比較テスト
  - 5 倍速度向上目標の達成検証
  - ベンチマーク結果のレポート出力
  - _Requirements: 3.2, 3.4_
  - ✅ `risk_benchmarks.rs` に 5 つのベンチマーク関数追加

- [x] 3.3 メモリ効率化とチェックポイント機構
  - `ThreadLocalWorkspacePool` の活用確認と最適化
  - メモリ使用量監視とアラート閾値の設定
  - チェックポイント機構の自動有効化ロジック
  - _Requirements: 3.3, 3.5_
  - ✅ `MemoryMonitor` 実装完了（26 テスト合格）

### WebApp Greeks Integration

- [ ] 4. IRS Greeks ワークフローの WebApp 統合
- [x] 4.1 Greeks 比較 API エンドポイントの実装
  - Bump 法と AAD 法の両方で Greeks を計算する API
  - 計算結果の差分（絶対誤差・相対誤差）を算出
  - パフォーマンス比較（計算時間、速度倍率）をレスポンスに含める
  - リクエスト・レスポンスの JSON スキーマ定義
  - _Requirements: 4.2, 4.3, 4.5, 4.6_
  - ✅ `/api/greeks/compare` エンドポイント実装完了（15 テスト合格）

- [x] 4.2 一次・二次 Greeks API エンドポイントの実装
  - `/api/v1/greeks/first-order` で Delta, Vega, Rho, Theta を返却
  - `/api/v1/greeks/second-order` で Gamma, Vanna, Volga を返却
  - リクエストバリデーションとエラーレスポンス
  - _Requirements: 4.1, 7.1, 7.2_
  - ✅ `/api/greeks/first-order`, `/api/greeks/second-order` エンドポイント実装完了（10 テスト合格）

- [ ] 4.3 バケット DV01 API エンドポイントの実装
  - `/api/v1/greeks/bucket-dv01` でテナー毎感応度を返却
  - Key Rate Duration のオプショナルパラメータ対応
  - _Requirements: 7.3_

- [ ] 4.4 (P) Bump vs AAD 比較 UI コンポーネントの実装
  - 比較結果をテーブル形式で並列表示
  - パフォーマンス比較チャート（バーチャート）を D3.js で実装
  - 精度比較（誤差）のハイライト表示
  - _Requirements: 4.3, 4.4, 4.5_

### Greeks Visualization

- [x] 5. Greeks 可視化機能の実装
- [x] 5.1 ヒートマップ API とフロントエンドの実装
  - `/api/greeks/heatmap` でテナー × ストライクのデータを返却
  - D3.js 互換 JSON フォーマット（x_axis, y_axis, values）
  - カラースケールとインタラクティブツールチップ
  - _Requirements: 5.1, 5.3_

- [x] 5.2 時系列チャート API とフロントエンドの実装
  - `/api/greeks/timeseries` で Greeks の時間推移データを返却
  - 複数 Greeks の重ね合わせ表示
  - ズーム・パン操作の実装
  - _Requirements: 5.2, 5.3_

- [x] 5.3 WebSocket リアルタイム更新の実装
  - Greeks 値更新時の WebSocket 配信
  - 既存 `WebSocketHub` パターンとの統合
  - 計算グラフノードハイライト連携
  - _Requirements: 5.4, 5.5_

### Scenario Analysis UI

- [ ] 6. シナリオ分析 UI の実装
- [ ] 6.1 プリセットシナリオ一覧 API の実装
  - `/api/scenarios/presets` で Rate Shock, Vol Spike, FX Crisis 等を返却
  - プリセットのパラメータ説明を含める
  - _Requirements: 6.2_

- [ ] 6.2 シナリオ実行 API の実装
  - `/api/scenarios/run` でカスタムシナリオを受け付け
  - 5 秒以上の計算はジョブ ID を返却（非同期化）
  - PnL 計算結果の返却
  - _Requirements: 6.1, 6.3, 6.5_

- [ ] 6.3 (P) シナリオパラメータ調整 UI の実装
  - リスクファクターシフト量のスライダー UI
  - パラメータ変更時の即座再計算トリガー
  - カスタムシナリオの保存・読み込み機能
  - _Requirements: 6.1, 6.3_

- [ ] 6.4 (P) シナリオ比較結果 UI の実装
  - シナリオ間 PnL 比較テーブル
  - 比較チャート（棒グラフ、ウォーターフォール）
  - _Requirements: 6.4_

### Async Job Management

- [x] 7. 非同期ジョブ管理機能の実装
- [x] 7.1 (P) ジョブマネージャーの実装
  - UUID ベースのジョブ ID 生成
  - 状態管理（Pending, Running, Completed, Failed）
  - TTL 付きジョブ結果キャッシュ（5 分）
  - _Requirements: 7.6_
  - ✅ `JobManager` 実装完了（24 テスト合格）

- [x] 7.2 ジョブ進捗 API とプログレスインジケータ
  - `/api/v1/jobs/{id}` で進捗状態を返却
  - WebSocket 経由の進捗プッシュ通知
  - UI プログレスインジケータ表示
  - _Requirements: 6.5, 7.6_
  - ✅ `GET /api/v1/jobs`, `GET /api/v1/jobs/:id` API 実装完了（8 テスト合格）

### API Documentation and Validation

- [ ] 8. API ドキュメントとバリデーションの実装
- [ ] 8.1 (P) OpenAPI ドキュメント生成の設定
  - `utoipa` + `utoipa-swagger-ui` の依存追加
  - 全 API エンドポイントに `ToSchema` derive を適用
  - `/api/docs` で Swagger UI を提供
  - _Requirements: 7.5_

- [ ] 8.2 (P) リクエストバリデーションとエラーハンドリング
  - `validator` crate によるリクエスト検証
  - HTTP 400 Bad Request の詳細エラーメッセージ
  - NaN/Inf 検出時の HTTP 422 レスポンス
  - _Requirements: 7.4_

### Metrics and Monitoring

- [ ] 9. パフォーマンスメトリクスと監視機能の実装
- [ ] 9.1 (P) Prometheus メトリクスエンドポイントの実装
  - `metrics` + `metrics-exporter-prometheus` の統合
  - `/metrics` でテキスト形式メトリクスを出力
  - Counter: API リクエスト数、エラー数
  - Histogram: レスポンスタイム
  - Gauge: アクティブ接続数
  - _Requirements: 8.2_

- [ ] 9.2 (P) メトリクス収集とダッシュボード表示
  - 計算時間、メモリ使用量、キャッシュヒット率の収集
  - エラー発生件数とエラータイプの統計
  - WebApp 内ダッシュボードウィジェット
  - _Requirements: 8.1, 8.3_

- [ ] 9.3 レスポンスタイム警告の実装
  - 閾値超過時の警告ログ出力
  - メトリクスへの閾値超過カウント
  - _Requirements: 8.4_

### Integration and Testing

- [ ] 10. 統合テストと最終検証
- [ ] 10.1 Greeks 計算統合テスト
  - Bump vs AAD の一致検証（許容誤差内）
  - ファクター別集計の正確性検証
  - バケット DV01 と総 DV01 の整合性検証
  - _Requirements: 1.4, 2.3_

- [ ] 10.2 API エンドポイント統合テスト
  - 全 REST API のレスポンスフォーマット検証
  - エラーレスポンスのスキーマ検証
  - WebSocket 配信の動作検証
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 10.3 パフォーマンス統合テスト
  - 1000 トレードポートフォリオの計算時間検証
  - AAD vs Bump の速度比（5 倍目標）の達成確認
  - メモリ使用量の許容範囲内確認
  - _Requirements: 3.1, 3.2, 3.5_

---

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| 1.1, 1.2 | 1.2, 1.3, 4.2 |
| 1.3 | 1.1, 1.2 |
| 1.4 | 10.1 |
| 1.5 | 1.2 |
| 1.6 | 1.3 |
| 2.1, 2.3 | 2.1 |
| 2.2 | 2.2 |
| 2.4 | 2.3 |
| 3.1 | 3.1 |
| 3.2, 3.4 | 3.2 |
| 3.3, 3.5 | 3.3, 10.3 |
| 4.1 | 4.2 |
| 4.2, 4.3, 4.5, 4.6 | 4.1 |
| 4.4 | 4.4 |
| 5.1, 5.3 | 5.1 |
| 5.2 | 5.2 |
| 5.4, 5.5 | 5.3 |
| 6.1 | 6.2, 6.3 |
| 6.2 | 6.1 |
| 6.3 | 6.2, 6.3 |
| 6.4 | 6.4 |
| 6.5 | 6.2, 7.2 |
| 7.1, 7.2 | 4.2, 10.2 |
| 7.3 | 4.3 |
| 7.4 | 8.2, 10.2 |
| 7.5 | 8.1 |
| 7.6 | 7.1, 7.2 |
| 8.1, 8.3 | 9.2 |
| 8.2 | 9.1 |
| 8.4 | 9.3 |

---

## Parallel Execution Summary

以下のタスクは並列実行可能（`(P)` マーク）：

**Pricer Layer**:
- 1.1, 2.1, 2.2 — 相互依存なし

**WebApp Layer**:
- 4.4, 6.3, 6.4, 7.1, 8.1, 8.2, 9.1, 9.2 — 異なるコンポーネント、ファイル競合なし

**依存関係の注意点**:
- 1.2 は 1.1 に依存
- 1.3 は 1.2 に依存
- 4.1, 4.2, 4.3 は 1.x, 2.x に依存
- 5.x は 4.x API に依存
- 10.x は全機能実装後に実行
