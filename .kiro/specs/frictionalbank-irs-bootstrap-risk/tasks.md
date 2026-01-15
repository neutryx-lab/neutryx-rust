# Implementation Plan

## タスク一覧

- [x] 1. API型定義とカーブキャッシュ基盤
- [x] 1.1 Bootstrap API のリクエスト・レスポンス型を定義する
  - Par Rate入力（テナー、レート）の構造体を作成
  - Bootstrap結果（curveId、pillars、discountFactors、zeroRates、processingTimeMs）の構造体を作成
  - 補間方式（linear、log_linear）のenum定義
  - camelCase シリアライゼーション属性を適用
  - _Requirements: 2.3, 2.4, 9.1, 9.6_

- [x] 1.2 (P) IRS Pricing API のリクエスト・レスポンス型を定義する
  - IRSパラメータ（curveId、notional、fixedRate、tenorYears、paymentFrequency）の構造体を作成
  - Pricing結果（npv、fixedLegPv、floatLegPv、processingTimeUs）の構造体を作成
  - 支払頻度（annual、semi_annual、quarterly）のenum定義
  - _Requirements: 3.3, 3.4, 9.2, 9.6_

- [x] 1.3 (P) Risk API のリクエスト・レスポンス型を定義する
  - リスク計算リクエスト（curveId、IRSパラメータ、bumpSizeBps）の構造体を作成
  - Delta結果（テナー、delta、processingTimeUs）の構造体を作成
  - TimingStats（meanUs、stdDevUs、minUs、maxUs、totalMs）の構造体を作成
  - 比較結果（bump、aad、aadAvailable、speedupRatio、comparison）の構造体を作成
  - _Requirements: 4.2, 4.3, 5.3, 6.2, 9.3, 9.4, 9.5, 9.6_

- [x] 1.4 エラーレスポンス型とバリデーションロジックを実装する
  - ErrorResponse型（error、message、details）を作成
  - Par Rateバリデーション（正値、数値チェック）
  - IRSパラメータバリデーション（想定元本正値、期間範囲）
  - HTTP 400/404/422 エラーのマッピング
  - _Requirements: 1.2, 1.3, 2.5, 9.7, 9.8_

- [x] 1.5 CurveCache状態管理モジュールを実装する
  - RwLock<HashMap<Uuid, CachedCurve>> 構造体の作成
  - カーブ追加・取得・存在確認のメソッド実装
  - AppState への CurveCache 統合
  - _Requirements: 2.3, 3.2_
  - _Contracts: CurveCache State_

- [x] 2. Bootstrap API エンドポイント
- [x] 2.1 bootstrap_handler を実装する
  - POST /api/bootstrap ルートの追加
  - リクエストのパースとバリデーション
  - SequentialBootstrapper を使用したカーブ構築呼び出し
  - 処理時間の計測（std::time::Instant）
  - 結果のCurveCacheへの保存とUUID生成
  - _Requirements: 2.1, 2.2, 2.4, 9.1_
  - _Contracts: bootstrap_handler API_

- [x] 2.2 Bootstrap収束失敗時のエラーハンドリングを実装する
  - BootstrapError のキャッチと変換
  - 収束失敗テナーの特定と詳細メッセージ作成
  - HTTP 422 レスポンスの返却
  - _Requirements: 2.5, 9.8_

- [x] 3. IRS Pricing API エンドポイント
- [x] 3.1 price_irs_handler を実装する
  - POST /api/price-irs ルートの追加
  - CurveCacheからのカーブ取得（curve_id検証）
  - IrsGreeksCalculator を使用したNPV計算呼び出し
  - 固定レグPV、変動レグPVの個別計算
  - 処理時間の計測（マイクロ秒単位）
  - _Requirements: 3.2, 3.3, 3.4, 9.2_
  - _Contracts: price_irs_handler API_

- [x] 3.2 カーブ未発見時のエラーハンドリングを実装する
  - CurveCacheでの curve_id 存在確認
  - HTTP 404 レスポンスの返却
  - エラーメッセージにcurve_idを含める
  - _Requirements: 3.6, 9.7_

- [x] 4. Bump法リスク計算 API エンドポイント
- [x] 4.1 risk_bump_handler を実装する
  - POST /api/risk/bump ルートの追加
  - BenchmarkRunner を使用したBump法Delta計算呼び出し
  - 各テナーのDelta値と個別計算時間の収集
  - DV01（全テナーDelta合計）の計算
  - 総計算時間の計測と統計情報作成
  - _Requirements: 4.1, 4.2, 4.3, 4.5, 9.3_
  - _Contracts: risk_bump_handler API_

- [x] 5. AAD法リスク計算 API エンドポイント
- [x] 5.1 risk_aad_handler を実装する
  - POST /api/risk/aad ルートの追加
  - enzyme-ad feature フラグの条件分岐
  - BenchmarkRunner を使用したAAD法Delta計算呼び出し
  - 単一逆伝播での全テナーDelta取得
  - AAD未対応時のフォールバック処理（Bump法代替またはエラー）
  - _Requirements: 5.1, 5.2, 5.3, 9.4_
  - _Contracts: risk_aad_handler API_

- [x] 6. リスク比較 API と WebSocket 通知
- [x] 6.1 risk_compare_handler を実装する
  - POST /api/risk/compare ルートの追加
  - Bump法とAAD法の両方を実行
  - 結果差分（相対誤差）の計算
  - Speedup Ratio（Bump時間 / AAD時間）の計算
  - aadAvailable フラグの設定
  - _Requirements: 5.4, 5.5, 6.2, 9.5_
  - _Contracts: risk_compare_handler API_

- [x] 6.2 WebSocket ブロードキャストイベントを実装する
  - BroadcastMessage enum に bootstrap_complete、risk_complete、calculation_error を追加
  - BootstrapCompleteEvent（curveId、tenorCount、processingTimeMs）
  - RiskCompleteEvent（curveId、method、dv01、speedupRatio）
  - CalculationErrorEvent（operation、error、failedTenor）
  - ハンドラからの state.tx.send() 呼び出し
  - _Requirements: 8.1, 8.2, 8.3, 8.4_
  - _Contracts: WebSocket Event_

- [x] 7. フロントエンド UI 基盤
- [x] 7.1 (P) IRS Bootstrap & Risk サブビューの HTML 構造を作成する
  - Pricerタブ内に「IRS Bootstrap & Risk」サブビュー追加
  - 3ステップワークフロー構造（Step 1: Market Data、Step 2: Bootstrap、Step 3: Pricing & Risk）
  - 各ステップのコンテナとインジケーター要素
  - ライト/ダークテーマ対応のクラス適用
  - _Requirements: 7.1, 7.2, 7.3, 7.6_

- [x] 7.2 (P) Par Rate 入力フォームを作成する
  - 9テナーポイント（1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y）の入力フィールド
  - 数値バリデーション（クライアントサイド）
  - 不正入力時のエラーメッセージ表示
  - 小数形式とパーセント形式の両方をサポート
  - _Requirements: 1.1, 1.2, 1.3, 1.6_

- [x] 7.3 (P) プリセットデータとサンプル値機能を実装する
  - 現実的なサンプルPar Rate値の定義
  - プリセット選択ボタンの作成
  - ボタンクリック時のフォーム自動入力
  - _Requirements: 1.4, 1.5_

- [x] 7.4 IRS パラメータ入力フォームを作成する
  - 想定元本、固定レート、期間、支払頻度の入力フィールド
  - Bootstrap未実行時のボタン無効化とメッセージ表示
  - _Requirements: 3.1, 3.6_

- [x] 7.5 (P) ローディングスピナーとステップインジケーターを実装する
  - 計算中のローディングスピナー表示
  - 各ステップの完了状態インジケーター更新
  - _Requirements: 7.3, 7.4_

- [x] 7.6 (P) レスポンシブレイアウトを実装する
  - モバイル向けスタックレイアウト
  - タブレット向け2カラムレイアウト
  - デスクトップ向け3カラムレイアウト
  - _Requirements: 7.5_

- [x] 8. カーブ可視化とプライシング UI
- [x] 8.1 Bootstrap 結果のカーブ可視化を実装する
  - Bootstrap API 呼び出しとレスポンス処理
  - テーブル形式でのDF、ゼロレート表示
  - Chart.js を使用したカーブグラフ描画
  - 処理時間の表示
  - _Requirements: 2.6_

- [x] 8.2 IRS プライシング結果表示を実装する
  - Price IRS API 呼び出しとレスポンス処理
  - NPV結果の3桁区切りフォーマット表示
  - 固定レグPV、変動レグPVの個別表示
  - 計算時間（マイクロ秒）の表示
  - _Requirements: 3.5_

- [x] 9. リスク計算結果と比較表示 UI
- [x] 9.1 Delta テーブル表示を実装する
  - 各テナーのDelta値を表形式で表示
  - Bump法とAAD法の結果を並列表示
  - DV01の計算と表示
  - 各テナーの計算時間（マイクロ秒）表示
  - _Requirements: 4.4, 4.5, 4.6, 5.4_

- [x] 9.2 計算時間比較バーチャートを実装する
  - Chart.js を使用した比較バーチャート描画
  - Bump法とAAD法の総計算時間並列表示
  - Speedup Ratio の計算と表示
  - 速度比に応じた色分け（10倍以上=緑、5倍以上=黄、それ以下=赤）
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [x] 9.3 AAD/Bump 差分警告とベンチマーク機能を実装する
  - AADとBumpの差分が許容誤差（1e-6）超過時の警告表示
  - 統計情報（平均、最小、最大時間）の表示オプション
  - 「Run Benchmark」ボタンで10回繰り返し実行
  - 繰り返し実行結果の統計集計と表示
  - _Requirements: 5.6, 6.5, 6.6_

- [x] 10. 統合テストと E2E 検証
- [x] 10.1 バックエンド統合テストを実装する
  - Bootstrap → Pricing フローの正常系テスト
  - Bootstrap → Risk Compare フローの正常系テスト
  - 不正 curve_id での 404 エラーテスト
  - Bootstrap 収束失敗時の 422 エラーテスト
  - WebSocket イベント送信のテスト
  - _Requirements: 2.1, 3.2, 4.1, 5.1, 8.1, 8.2_

- [x] 10.2 E2E ワークフローテストを実行する
  - Par Rate 入力 → Bootstrap → カーブ可視化の動作確認
  - IRS パラメータ入力 → Pricing → NPV 表示の動作確認
  - Risk Compare → Delta テーブル + 速度比較チャート表示の動作確認
  - 3ステップワークフローの状態遷移確認
  - _Requirements: 7.1, 7.2, 7.3_

---

## 要件カバレッジマトリクス

| 要件 | タスク |
|------|--------|
| 1.1, 1.2, 1.3, 1.6 | 7.2 |
| 1.4, 1.5 | 7.3 |
| 2.1, 2.2, 2.4 | 2.1 |
| 2.3, 2.4 | 1.1 |
| 2.5 | 1.4, 2.2 |
| 2.6 | 8.1 |
| 3.1, 3.6 | 7.4 |
| 3.2, 3.3, 3.4 | 3.1 |
| 3.5 | 8.2 |
| 4.1, 4.2, 4.3, 4.5 | 4.1 |
| 4.4, 4.5, 4.6 | 9.1 |
| 5.1, 5.2, 5.3 | 5.1 |
| 5.4 | 6.1, 9.1 |
| 5.5 | 6.1 |
| 5.6 | 9.3 |
| 6.1, 6.2, 6.3, 6.4 | 9.2 |
| 6.5, 6.6 | 9.3 |
| 7.1, 7.2, 7.3, 7.6 | 7.1 |
| 7.3, 7.4 | 7.5 |
| 7.5 | 7.6 |
| 8.1, 8.2, 8.3, 8.4 | 6.2 |
| 9.1, 9.6 | 1.1, 2.1 |
| 9.2, 9.6 | 1.2, 3.1 |
| 9.3, 9.4, 9.5, 9.6 | 1.3, 4.1, 5.1, 6.1 |
| 9.7, 9.8 | 1.4, 2.2, 3.2 |

---

_作成日: 2026-01-14_
_言語: 日本語 (spec.json.language: ja)_
