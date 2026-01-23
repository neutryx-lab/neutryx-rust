# Implementation Plan

## Task Overview

Demo WebAppのAnalysisセクションにPricer検証機能を実装する。GenericPricerおよびGreeks計算のAPIエンドポイント追加、フロントエンドUI構築を行う。

## Requirement Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 (UI配置とナビゲーション) | 3.1 |
| 2 (Trade選択とInstrumentList統合) | 4.1 |
| 3 (Cashflow展開と編集) | 4.2 |
| 4 (マーケットデータ設定) | 4.3 |
| 5 (モデル設定) | 4.3 |
| 6 (プライシング実行) | 1.1, 2.1, 4.4 |
| 7 (PricingResult表示) | 4.4 |
| 8 (Greeks計算と表示) | 1.2, 2.2, 4.5 |
| 9 (結果比較と検証) | 4.6 |
| 10 (APIエンドポイント) | 1.1, 1.2, 2.1, 2.2, 2.3, 2.4 |

---

## Tasks

- [x] 1. バックエンド型定義
- [x] 1.1 (P) GenericPricer関連のリクエスト・レスポンス型を定義する
  - プライシングリクエスト型を定義（Leg配列、評価日、報告通貨、ModelConfig）
  - プライシングレスポンス型を定義（Total PV、Leg別PV、Cashflow別PV、エラー情報）
  - Leg入力型とCashflow入力型を定義（通貨、Direction、支払日、金額）
  - ModelConfig入力型を定義（num_paths、num_steps、seedオプション）
  - camelCaseフィールド名でJavaScript互換のシリアライゼーション属性を設定
  - バリデーション関数を各型に追加
  - _Requirements: 6.2, 10.6_

- [x] 1.2 (P) Greeks関連のリクエスト・レスポンス型を定義する
  - Greeksリクエスト型を定義（Leg配列、評価日、報告通貨、BumpSizes）
  - BumpSizes入力型を定義（rate_bump_bp、fx_bump_pct、vol_bump_pctとデフォルト値）
  - Greeksレスポンス型を定義（Delta、Gamma、Theta、Vega、FX Delta、エラー情報）
  - 商品タイプ一覧レスポンス型を定義
  - _Requirements: 8.4, 10.6_

- [x] 2. バックエンドAPIハンドラ
- [x] 2.1 プライシングエンドポイントを実装する
  - `/api/pricer/price` POSTハンドラを作成
  - リクエストバリデーション関数を実装（必須パラメータ、範囲チェック）
  - LegInput→SimpleLeg、CashflowInput→SimpleCashflow型変換関数を実装
  - GenericPricerインスタンス生成とget_pv_simple呼び出し
  - PricingResult→GenericPricerResponse変換（Leg別、Cashflow別のPV内訳を含む）
  - PricingErrorのJSON化とHTTPステータスコードマッピング（400/422/500）
  - _Requirements: 6.1, 6.2, 6.4, 6.6, 10.1, 10.4, 10.7_

- [x] 2.2 Greeksエンドポイントを実装する
  - `/api/pricer/greeks` POSTハンドラを作成
  - BumpSizesデフォルト値適用ロジックを実装
  - BumpAndRevalueCalculatorインスタンス生成
  - Delta、Gamma、Theta、Vega、FX Deltaの各計算関数呼び出し
  - TradeGreeks→GreeksResponse変換
  - エラーハンドリング（計算失敗時のレスポンス）
  - _Requirements: 8.2, 8.5, 8.7, 10.2, 10.5, 10.7_

- [x] 2.3 商品タイプ一覧エンドポイントを実装する
  - `/api/pricer/instruments` GETハンドラを作成
  - 利用可能な商品タイプ（IRS、Swaption、FX Forward等）をリスト返却
  - _Requirements: 10.3_

- [x] 2.4 ルーター統合とモジュール登録を行う
  - mod.rsに`pub mod generic_pricer_handlers;`を追加
  - build_router関数にpricerルートグループを追加（`/api/pricer`プレフィックス）
  - 各エンドポイントをルーターに登録
  - _Requirements: 10.1, 10.2, 10.3_

- [x] 3. フロントエンドUI構造
- [x] 3.1 (P) Pricer画面のHTML構造を追加する
  - Analysisアコーディオン内にPricerナビゲーション項目を追加（Model Calib下）
  - 計算機アイコン（bi-calculator）を設定
  - `#pricer-view`セクションを作成（data-view属性）
  - 2パネルレイアウト（左：入力パネル、右：結果パネル）を構築
  - glassmorphismデザインクラス（.glass）を適用
  - Trade選択、CF展開、マーケットデータ、モデル設定の各入力エリアを配置
  - 結果表示エリア（PricingResult、Greeks、履歴比較）を配置
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [x] 4. フロントエンドPricerモジュール
- [x] 4.1 Trade選択とInstrumentList統合を実装する
  - pricer.jsモジュールを作成し初期化関数を定義
  - 商品タイプドロップダウンを`/api/pricer/instruments`から動的生成
  - 商品タイプ選択時のパラメータフォーム表示切り替え
  - `demo_portfolio.json`からのサンプルTrade読み込み機能
  - サンプルTrade選択時のフォーム自動入力
  - 手動パラメータ入力による新規Trade作成
  - 必須パラメータの即時バリデーションとエラー表示
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [x] 4.2 Cashflow展開と編集機能を実装する
  - 「Expand CF」ボタンクリック時の`/api/trade/expand`呼び出し
  - Leg/Cashflow構造のテーブル表示（支払日、金額、通貨、Direction）
  - Cashflow金額の編集可能インプットフィールド
  - 編集済みフラグ（isModified）の管理とビジュアル表示
  - 「Reset」ボタンによる元値復元機能
  - Leg単位のPayer/Receiver方向表示
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [x] 4.3 マーケットデータとモデル設定パネルを実装する
  - カーブ選択ドロップダウン（USD-SOFR、EUR-ESTR、JPY-TONA）
  - カーブ選択時の`/api/curves/instruments/{index}`呼び出し
  - FXスポットレート表示（webapp_market_data.jsonから）
  - 評価日の日付ピッカー
  - 報告通貨ドロップダウン（USD、EUR、JPY、GBP）
  - ModelConfig設定セクション（num_paths、num_steps、seed）
  - 「Use Default」チェックボックスとデフォルト値適用
  - パラメータバリデーション（正の整数チェック）
  - マーケットデータロード失敗時のエラートースト表示
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [x] 4.4 プライシング実行と結果表示を実装する
  - 「Price」ボタンクリック時の`/api/pricer/price`呼び出し
  - リクエストペイロード構築（展開済みCashflow、評価日、報告通貨、ModelConfig）
  - ローディングインジケータ表示
  - Total PVの大フォント表示
  - Leg単位PV内訳テーブル（元通貨、FXレート、Direction）
  - Leg行クリックによるCashflow詳細展開
  - Cashflow詳細表示（PV、ディスカウントファクター、支払日）
  - 通貨別PV集計表示（フロントエンドでLegデータから集計）
  - PricingErrorのエラーメッセージ表示
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_

- [x] 4.5 Greeks計算と表示を実装する
  - 「Calculate Greeks」ボタン配置
  - バンプ幅設定入力フィールド（rate_bump_bp、fx_bump_pct、vol_bump_pct）
  - `/api/pricer/greeks`呼び出し
  - Greeksテーブル表示（Greek名、値、単位）
  - 計算モード表示（Bump-and-Revalue）
  - エラーメッセージ表示
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7_

- [x] 4.6 結果履歴と比較機能を実装する
  - プライシング結果の履歴配列管理（最大5件）
  - 新規プライシング時の前回結果とのPV差分計算・表示
  - 「Compare」モードでの2結果並列表示
  - PV差分の絶対値・割合（%）表示
  - パラメータ変更箇所のハイライト表示
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [x] 5. 統合とテスト
- [x] 5.1 エンドツーエンド統合テストを実施する
  - `/api/pricer/price`エンドポイントのE2Eテスト（正常系・異常系）
  - `/api/pricer/greeks`エンドポイントのE2Eテスト
  - エラーレスポンス検証（400、422ステータス）
  - フロントエンドからバックエンドへの一連フロー検証
  - _Requirements: 6.1, 8.2, 10.1, 10.2_

- [x]*5.2 バックエンドユニットテストを作成する
  - validate_generic_pricer_request関数のテスト
  - convert_to_simple_legs関数のテスト
  - BumpSizesデフォルト値テスト
  - Response型シリアライゼーションテスト
  - _Requirements: 10.6, 10.7_
