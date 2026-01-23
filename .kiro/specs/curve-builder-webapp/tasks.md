# Implementation Plan

## Overview
Curve Build画面の精緻化タスク。バックエンドAPI、フロントエンドUI、データファイル構造の実装。

**Design Decision**: BootstrapMethod::Global は「Coming Soon」としてUI表示のみ（無効化）

---

## Tasks

- [x] 1. Index別Instrumentデータファイルの作成

- [x] 1.1 (P) USD-SOFR Instrumentファイルの作成
  - Deposit（1M, 3M, 6M）、OIS（1Y, 2Y, 3Y, 5Y, 7Y, 10Y）、Swap（15Y, 20Y, 30Y）のレート定義
  - JSON形式でindex, currency, reference_date, instrumentsフィールドを含む
  - tenorとtenor_years（年換算値）の両方を記載
  - `demo/data/input/curves/usd-sofr.json` に保存
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 1.2 (P) EUR-ESTR Instrumentファイルの作成
  - Deposit（1M, 3M, 6M）、OIS（1Y, 2Y, 3Y, 5Y, 7Y, 10Y）、Swap（15Y, 20Y, 30Y）のレート定義
  - USD-SOFRと同じJSON構造を維持
  - `demo/data/input/curves/eur-estr.json` に保存
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 1.3 (P) JPY-TONA Instrumentファイルの作成
  - Deposit（1M, 3M, 6M）、OIS（1Y, 2Y, 3Y, 5Y, 7Y, 10Y）、Swap（15Y, 20Y, 30Y）のレート定義
  - USD-SOFRと同じJSON構造を維持
  - `demo/data/input/curves/jpy-tona.json` に保存
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

---

- [x] 2. バックエンドAPI型定義

- [x] 2.1 Curve Builder用リクエスト・レスポンス型の定義
  - InstrumentListResponse: index, currency, instruments配列
  - CurveBuildRequest: index, instruments, interpolation, bootstrap_method, tolerance, max_iterations
  - CurveBuildResponse: curve_id, status, pillars, discount_factors, zero_rates, processing_time_ms
  - ParameterResponse: curve_id, parameter_type, data配列
  - BuilderListResponse: interpolation_methods, bootstrap_methods配列
  - 全型にcamelCaseシリアライズを適用
  - 既存の `BootstrapCurveCache` を再利用
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [x] 2.2 InterpolationMethod・BootstrapMethod enumの定義
  - InterpolationMethod: Linear, LogLinear, CubicSpline, Monotonic
  - BootstrapMethod: Sequential, Global（Globalはバックエンドで未実装フラグを返す）
  - Serdeシリアライズでsnake_case形式
  - _Requirements: 3.1, 3.2_

- [x] 2.3 RFC 7807 ProblemDetails エラー型の定義
  - type, title, status, detail, instanceフィールド
  - 既存ErrorResponseとの互換性を維持
  - カーブ構築エラー、ファイル読み込みエラー用のバリエーション
  - _Requirements: 7.5_

---

- [x] 3. Index別Instrument読み込み機能

- [x] 3.1 CurveDataLoader実装
  - `demo/data/input/curves/` ディレクトリからJSONファイルを読み込む
  - 指定されたindex名に対応するファイルを取得
  - ファイル不在時はデフォルトInstrumentリストを返却
  - available_indices()で利用可能なIndex一覧を返却
  - _Requirements: 1.1, 1.3, 1.5_

- [x] 3.2 InstrumentListHandler APIエンドポイント実装
  - `GET /api/curves/instruments/{index}` ルートを追加
  - CurveDataLoaderを呼び出してInstrumentリストを取得
  - 未サポートIndexの場合は404エラーを返却
  - _Requirements: 1.1, 1.2, 1.3, 7.1_

---

- [x] 4. Builderモデル一覧API

- [x] 4.1 BuilderListHandler実装
  - `GET /api/curves/builders` ルートを追加
  - 4種類の補間手法（Linear, LogLinear, CubicSpline, Monotonic）を返却
  - 各手法にid, name, description, recommendedフラグを付与
  - ブートストラップ手法（Sequential: enabled, Global: disabled + "Coming Soon"メッセージ）
  - _Requirements: 3.1, 3.2, 7.4_

---

- [x] 5. カーブ構築API

- [x] 5.1 CurveBuildHandler実装
  - `POST /api/curves/build` ルートを追加
  - リクエストのバリデーション（レート範囲、Instrument数）
  - InstrumentInputをBootstrapInstrument型に変換
  - SequentialBootstrapperを使用してカーブ構築を実行
  - 処理時間を計測し、レスポンスに含める
  - _Requirements: 4.1, 4.3, 7.2_

- [x] 5.2 カーブ構築結果のキャッシュ保存
  - 構築成功時にUUID形式のcurve_idを発行
  - 既存BootstrapCurveCacheにカーブを保存
  - pillars, discount_factors, zero_ratesをレスポンスに含める
  - _Requirements: 4.3, 4.5_

- [x] 5.3 カーブ構築エラーハンドリング
  - 収束エラー時は422ステータスとProblemDetails形式で詳細を返却
  - 不正レート検出時は400ステータスでバリデーションエラーを返却
  - ブートストラップ失敗時のtenor情報を含める
  - _Requirements: 4.4, 7.5_

---

- [x] 6. Parameterカーブ取得API

- [x] 6.1 CurveParameterHandler実装
  - `GET /api/curves/{curveId}/parameters` ルートを追加
  - クエリパラメータ: type（必須）, start_year, end_year, grid_interval
  - curveIdでキャッシュからカーブを取得
  - 存在しないcurveIdの場合は404エラー
  - _Requirements: 5.1, 5.5, 7.3_

- [x] 6.2 Parameter計算ロジック実装
  - YieldCurve traitの discount_factor(), zero_rate(), forward_rate() を使用
  - 指定されたTenor範囲でグリッドポイントを生成
  - 各ポイントで該当Parameterを計算
  - _Requirements: 5.1, 5.2, 5.3_

---

- [ ] 7. IRS機能の削除

- [ ] 7.1 IRS関連UI要素の削除
  - `#irs-params-section` セクションをHTMLから削除
  - IRS Pricing結果表示カードを削除
  - Risk計算結果カードを削除（Curve Build画面から）
  - _Requirements: 6.1_

- [ ] 7.2 IRS関連JavaScript呼び出しの削除
  - handlePriceIrs()関数呼び出しを削除
  - handleRiskBump(), handleRiskAad()呼び出しを削除
  - IRS関連イベントリスナーを削除
  - _Requirements: 6.2_

- [ ] 7.3 View IDのリネーム
  - `#irs-bootstrap-view` を `#curve-builder-view` にリネーム
  - ナビゲーションメニューのラベルを「Curve Builder」に変更
  - 関連するCSS classの更新
  - _Requirements: 6.1, 6.3_

---

- [ ] 8. Curve Builder フロントエンドUI

- [ ] 8.1 Index選択ドロップダウンの実装
  - USD-SOFR, EUR-ESTR, JPY-TONAの選択肢を表示
  - 選択変更時にInstrument一覧APIを呼び出し
  - 選択状態を保持
  - _Requirements: 1.4, 2.1_

- [ ] 8.2 レート入力テーブルの実装
  - Instrumentタイプ、Tenor、Rate列を持つ編集可能テーブル
  - 数値入力フィールドにバリデーション（-10%～+50%、小数点4桁）
  - 変更されたセルのハイライト表示
  - 元の値を保持し、変更差分を視覚化
  - _Requirements: 2.1, 2.2, 2.3_

- [ ] 8.3 レートのエクスポート・インポート機能
  - 「Export JSON」ボタンでレートをJSONファイルとしてダウンロード
  - 「Import JSON」ボタンでファイル選択ダイアログを表示
  - インポート時にレートフィールドを更新
  - 「Reset」ボタンで元の値に戻す
  - _Requirements: 2.4, 2.5, 2.6_

---

- [ ] 9. Builder設定UIの実装

- [ ] 9.1 補間手法選択UIの実装
  - Linear, LogLinear, CubicSpline, Monotonicのラジオボタン/ドロップダウン
  - 推奨手法（LogLinear）にラベル表示
  - 選択変更時に説明テキストを更新
  - _Requirements: 3.1, 3.3_

- [ ] 9.2 ブートストラップ手法選択UIの実装
  - Sequentialを選択可能として表示
  - Globalは「Coming Soon」ラベル付きで無効化
  - 選択状態に応じてtoleranceとmax_iterationsパラメータを表示
  - _Requirements: 3.2, 3.3_

- [ ] 9.3 Builder設定プリセット機能
  - 現在の設定をLocalStorageに保存する機能
  - 保存済みプリセットをドロップダウンで選択
  - プリセット適用時に各設定フィールドを更新
  - _Requirements: 3.4_

---

- [ ] 10. カーブ構築UIの実装

- [ ] 10.1 Build Curveボタンと処理
  - 「Build Curve」ボタンのクリックイベント
  - 入力レートとBuilder設定を収集してAPIリクエスト送信
  - ボタン無効化とプログレス表示
  - _Requirements: 4.1, 4.2_

- [ ] 10.2 構築結果サマリの表示
  - 成功/失敗ステータスのバッジ表示
  - 処理時間、使用Instrument数の表示
  - エラー時は詳細メッセージを表示
  - _Requirements: 4.3, 4.4_

- [ ] 10.3 再構築通知の実装
  - Builder設定またはレート変更時に「Rebuild Required」通知を表示
  - 通知をクリアする条件（再構築完了時）
  - _Requirements: 4.5_

---

- [ ] 11. Parameterカーブ表示UIの実装

- [ ] 11.1 Parameter表示モード切替タブ
  - Discount Factor, Zero Rate, Forward Rateの3タブ
  - タブ切替時にParameter取得APIを呼び出し
  - アクティブタブのスタイル適用
  - _Requirements: 5.1_

- [ ] 11.2 Chart.jsでのカーブ描画
  - 構築済みカーブのチャート表示
  - X軸: Tenor（年）、Y軸: Parameter値
  - ホバー時にツールチップでTenorと値を表示
  - _Requirements: 5.2, 5.4_

- [ ] 11.3 テーブル形式でのデータ表示
  - TenorとValue列を持つテーブル
  - チャートと同じデータを表示
  - _Requirements: 5.3_

- [ ] 11.4 Tenor範囲カスタマイズUI
  - 開始年、終了年、グリッド間隔の入力フィールド
  - 「Apply」ボタンでParameterを再取得
  - デフォルト値: 0-30年、0.25年間隔
  - _Requirements: 5.5_

- [ ] 11.5 カーブデータエクスポート機能
  - 「Export CSV」ボタンでCSVファイルをダウンロード
  - 「Export JSON」ボタンでJSONファイルをダウンロード
  - ファイル名にcurve_idとparameter_typeを含める
  - _Requirements: 5.6_

---

- [x] 12. axum Routerへのエンドポイント登録

- [x] 12.1 curve_handlers.rsモジュールの作成とルート登録
  - 新規モジュールをmod.rsに追加
  - `/api/curves/instruments/{index}` ルートを登録
  - `/api/curves/builders` ルートを登録
  - `/api/curves/build` ルートを登録
  - `/api/curves/{curveId}/parameters` ルートを登録
  - 既存の `/api/bootstrap` ルートは維持（後方互換性）
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.6_

---

- [ ] 13. 統合テスト

- [ ] 13.1 APIエンドポイントのテスト
  - GET /api/curves/instruments/{index} の正常系・エラー系テスト
  - POST /api/curves/build のカーブ構築テスト
  - GET /api/curves/{curveId}/parameters のParameter取得テスト
  - GET /api/curves/builders のBuilder一覧テスト
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 13.2 E2Eフローの検証
  - Index選択 → レート編集 → Build Curve → Parameterチャート表示の一連フロー
  - 各Parameter表示モード（DF, ZeroRate, ForwardRate）の切替確認
  - エクスポート・インポート機能の動作確認
  - _Requirements: 1.1, 2.1, 4.1, 5.1, 5.2_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 1.1, 1.2, 1.3, 3.1, 3.2, 13.2 |
| 1.2 | 1.1, 1.2, 1.3, 3.2 |
| 1.3 | 1.1, 1.2, 1.3, 3.1, 3.2 |
| 1.4 | 1.1, 1.2, 1.3, 8.1 |
| 1.5 | 3.1 |
| 2.1 | 8.1, 8.2, 13.2 |
| 2.2 | 8.2 |
| 2.3 | 8.2 |
| 2.4 | 8.3 |
| 2.5 | 8.3 |
| 2.6 | 8.3 |
| 3.1 | 2.2, 4.1, 9.1 |
| 3.2 | 2.2, 4.1, 9.2 |
| 3.3 | 9.1, 9.2 |
| 3.4 | 9.3 |
| 3.5 | (Builderモデル互換性チェックは将来実装) |
| 4.1 | 5.1, 10.1, 13.2 |
| 4.2 | 10.1 |
| 4.3 | 5.1, 5.2, 10.2 |
| 4.4 | 5.3, 10.2 |
| 4.5 | 5.2, 10.3 |
| 5.1 | 6.1, 6.2, 11.1, 13.2 |
| 5.2 | 6.2, 11.2, 13.2 |
| 5.3 | 6.2, 11.3 |
| 5.4 | 11.2 |
| 5.5 | 6.1, 11.4 |
| 5.6 | 11.5 |
| 6.1 | 7.1, 7.3 |
| 6.2 | 7.2 |
| 6.3 | 7.3 |
| 6.4 | (ドキュメント更新はスコープ外) |
| 7.1 | 2.1, 3.2, 12.1, 13.1 |
| 7.2 | 2.1, 5.1, 12.1, 13.1 |
| 7.3 | 2.1, 6.1, 12.1, 13.1 |
| 7.4 | 2.1, 4.1, 12.1, 13.1 |
| 7.5 | 2.3, 5.3 |
| 7.6 | 12.1 |

## Notes
- Requirement 3.5（Builderモデル互換性チェック）は将来の拡張として延期
- Requirement 6.4（ドキュメント更新）はコード実装スコープ外
- GlobalBootstrapMethodは「Coming Soon」として表示のみ（無効化）
