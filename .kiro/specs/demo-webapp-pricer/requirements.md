# Requirements Document

## Introduction

本仕様は、Demo WebAppのAnalysisセクション内にPricer検証機能を実装するための要件を定義する。Model Calib画面の下に新規「Pricer」画面を追加し、InstrumentListからのTrade選択・CF展開・編集機能、および`pricer_pricing::generic_pricer`モジュール（`GenericPricer::get_pv()`、`greeks_calculator.rs`）を使用したプライシング結果の検証UIを提供する。

## Requirements

### Requirement 1: UI配置とナビゲーション

**Objective:** As a クオンツ開発者, I want Analysis内のModel Calib画面の下にPricer画面にアクセスできる, so that カーブ構築・モデルキャリブレーション後にプライシング検証をシームレスに実行できる

#### Acceptance Criteria

1. The Demo WebApp shall Analysisアコーディオン内にModel Calibの下に「Pricer」ナビゲーション項目を追加する
2. When 「Pricer」ナビゲーション項目がクリックされたとき, the Demo WebApp shall Pricer検証画面（`#pricer-view`）を表示する
3. The Demo WebApp shall Pricer画面のアイコンとして計算機またはチャートアイコン（例：`bi-calculator`）を使用する
4. The Demo WebApp shall 既存のglassmorphismデザインシステムに従ったUIを提供する
5. The Demo WebApp shall レスポンシブ2パネルレイアウト（左：入力、右：結果）を採用する

### Requirement 2: Trade選択とInstrumentList統合

**Objective:** As a クオンツ開発者, I want InstrumentListから商品を選択してプライシング対象とできる, so that 既存のTrade定義を再利用してプライシング検証ができる

#### Acceptance Criteria

1. The Demo WebApp shall InstrumentListドロップダウンから商品タイプを選択できる（IRS、Swaption、FX Forward等）
2. When 商品タイプが選択されたとき, the Demo WebApp shall 対応するパラメータ入力フォームを表示する
3. The Demo WebApp shall `demo/data/input/demo_portfolio.json` からサンプルTradeをロードできる
4. When サンプルTradeが選択されたとき, the Demo WebApp shall Trade詳細をパラメータフォームに展開する
5. The Demo WebApp shall 手動でのパラメータ入力による新規Trade作成をサポートする
6. If 必須パラメータが未入力の場合, then the Demo WebApp shall バリデーションエラーを表示する

### Requirement 3: Cashflow展開と編集

**Objective:** As a クオンツ開発者, I want TradeをCashflow展開し、個別のCashflowを確認・編集できる, so that プライシング入力を詳細に検証・調整できる

#### Acceptance Criteria

1. When 「Expand CF」ボタンがクリックされたとき, the Demo WebApp shall TradeをLeg/Cashflow構造に展開して表示する
2. The Demo WebApp shall 展開されたCashflowをテーブル形式で表示する（支払日、金額、通貨、Direction）
3. The Demo WebApp shall 各Cashflowの金額を編集可能なインプットフィールドで提供する
4. When Cashflow金額が編集されたとき, the Demo WebApp shall 編集済みフラグ（`isModified`）を設定する
5. The Demo WebApp shall 「Reset」ボタンで元のCashflow値に戻せる
6. The Demo WebApp shall Leg単位でのPayer/Receiver方向を表示する

### Requirement 4: マーケットデータ設定

**Objective:** As a クオンツ開発者, I want `demo/data/input/` 内のマーケットデータを選択・ロードできる, so that 実際の市場環境に近い条件でプライシング検証ができる

#### Acceptance Criteria

1. The Demo WebApp shall `demo/data/input/curves/` からカーブデータ（USD-SOFR、EUR-ESTR、JPY-TONA）を選択できる
2. When カーブが選択されたとき, the Demo WebApp shall `/api/curves/instruments/{index}` APIからカーブデータをロードする
3. The Demo WebApp shall `demo/data/input/market_data/webapp_market_data.json` からFXスポットレートを表示する
4. The Demo WebApp shall 評価日（Valuation Date）を日付ピッカーで設定できる
5. The Demo WebApp shall 報告通貨（Reporting Currency）をドロップダウンで選択できる（USD、EUR、JPY、GBP）
6. If マーケットデータのロードに失敗した場合, then the Demo WebApp shall エラートーストを表示する

### Requirement 5: モデル設定

**Objective:** As a クオンツ開発者, I want プライシングに使用するモデルパラメータを設定できる, so that 様々なモデル設定でのプライシング結果を比較検証できる

#### Acceptance Criteria

1. The Demo WebApp shall ModelConfig設定セクションを提供する
2. The Demo WebApp shall シミュレーションパス数（num_paths）を入力できる（デフォルト：10,000）
3. The Demo WebApp shall 時間ステップ数（num_steps）を入力できる（デフォルト：100）
4. The Demo WebApp shall 乱数シード（seed）を入力できる（オプション）
5. When 「Use Default」チェックボックスがオンのとき, the Demo WebApp shall デフォルト設定を適用する
6. If num_pathsまたはnum_stepsが0以下の場合, then the Demo WebApp shall バリデーションエラーを表示する

### Requirement 6: プライシング実行

**Objective:** As a クオンツ開発者, I want `GenericPricer::get_pv()` を呼び出してプライシング結果を取得できる, so that バックエンドエンジンの動作を検証できる

#### Acceptance Criteria

1. When 「Price」ボタンがクリックされたとき, the Demo WebApp shall `/api/pricer/price` エンドポイントを呼び出す
2. The Demo WebApp shall リクエストペイロードにTrade情報、評価日、報告通貨、ModelConfigを含める
3. While プライシング処理中, the Demo WebApp shall ローディングインジケータを表示する
4. When プライシングが完了したとき, the Demo WebApp shall `PricingResult` を結果パネルに表示する
5. The Demo WebApp shall Total PV（報告通貨建て）を大きなフォントで表示する
6. If プライシングがエラーになった場合, then the Demo WebApp shall `PricingError` のエラーメッセージを表示する

### Requirement 7: PricingResult表示

**Objective:** As a クオンツ開発者, I want PricingResultの階層構造（Trade→Leg→Cashflow）を確認できる, so that プライシング結果の内訳を詳細に検証できる

#### Acceptance Criteria

1. The Demo WebApp shall Leg単位のPV内訳を表示する（`by_leg()`相当）
2. The Demo WebApp shall Cashflow単位のPV内訳を展開表示できる（`by_cashflow()`相当）
3. The Demo WebApp shall 各Legの元通貨、FXレート、Direction（Payer/Receiver）を表示する
4. The Demo WebApp shall 通貨別PV集計を表示する（`group_by_currency()`相当）
5. When Leg行がクリックされたとき, the Demo WebApp shall 当該Legに属するCashflow詳細を展開表示する
6. The Demo WebApp shall ディスカウントファクターと支払日を各Cashflowに表示する

### Requirement 8: Greeks計算と表示

**Objective:** As a クオンツ開発者, I want Greeks（Delta、Gamma、Theta、Vega、FX Delta）を計算・表示できる, so that リスク感応度を検証できる

#### Acceptance Criteria

1. The Demo WebApp shall 「Calculate Greeks」ボタンを提供する
2. When 「Calculate Greeks」ボタンがクリックされたとき, the Demo WebApp shall `/api/pricer/greeks` エンドポイントを呼び出す
3. The Demo WebApp shall Greeks計算モード（Bump-and-Revalue）を選択できる
4. The Demo WebApp shall バンプ幅設定を入力できる（rate_bump_bp、fx_bump_pct、vol_bump_pct）
5. When Greeks計算が完了したとき, the Demo WebApp shall Delta、Gamma、Theta、Vega、FX Deltaを結果パネルに表示する
6. The Demo WebApp shall Greeksをテーブル形式で表示する（Greek名、値、単位）
7. If Greeks計算がエラーになった場合, then the Demo WebApp shall エラーメッセージを表示する

### Requirement 9: 結果比較と検証

**Objective:** As a クオンツ開発者, I want 複数のプライシング結果を比較できる, so that パラメータ変更による影響を検証できる

#### Acceptance Criteria

1. The Demo WebApp shall 直近のプライシング結果を履歴として保持する（最大5件）
2. When 新しいプライシングが実行されたとき, the Demo WebApp shall 前回結果とのPV差分を表示する
3. The Demo WebApp shall 「Compare」モードで2つの結果を並べて表示できる
4. The Demo WebApp shall PV差分を絶対値と割合（%）で表示する
5. When パラメータが変更されたとき, the Demo WebApp shall 変更箇所をハイライト表示する

### Requirement 10: APIエンドポイント

**Objective:** As a バックエンド開発者, I want Pricer機能のRESTエンドポイントを提供する, so that フロントエンドからGenericPricerを呼び出せる

#### Acceptance Criteria

1. The Demo WebApp Backend shall `POST /api/pricer/price` エンドポイントを提供する
2. The Demo WebApp Backend shall `POST /api/pricer/greeks` エンドポイントを提供する
3. The Demo WebApp Backend shall `GET /api/pricer/instruments` エンドポイントで利用可能な商品タイプを返す
4. When `/api/pricer/price` が呼び出されたとき, the Demo WebApp Backend shall `GenericPricer::get_pv_simple()` を実行する
5. When `/api/pricer/greeks` が呼び出されたとき, the Demo WebApp Backend shall `BumpAndRevalueCalculator` を使用してGreeksを計算する
6. The Demo WebApp Backend shall JSON形式でリクエスト/レスポンスを処理する
7. If リクエストペイロードが不正な場合, then the Demo WebApp Backend shall HTTP 400 Bad Requestを返す
