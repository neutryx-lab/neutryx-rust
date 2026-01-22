# Implementation Plan

## Phase 1: Convention Migration

- [x] 1. Convention モジュール移動
- [x] 1.1 Convention ファイルを trade/ 配下に移動
  - 既存の convention/ ディレクトリを trade/convention/ にコピー
  - trade/mod.rs に `pub mod convention;` を追加
  - convention/mod.rs の re-export を維持
  - _Requirements: 6.7_

- [x] 1.2 旧パス互換性のための re-export 設定
  - lib.rs に `pub use trade::convention;` を追加
  - 旧パス (`infra_master::convention`) からのアクセスを維持
  - deprecation 警告を追加（0.8.0 で deprecation 予定）
  - _Requirements: 6.7_

## Phase 2: Common Types & Infrastructure

- [x] 2. 共通型とエラー定義
- [x] 2.1 資産クラスと共通列挙型の定義
  - AssetClass enum（Rates, Fx, Equity, Credit, Commodity）を定義
  - OptionType（Call/Put）、ExerciseStyle（European/American/Bermudan）を定義
  - SettlementType（Cash/Physical）、PayerReceiver を定義
  - serde feature-gating を適用
  - _Requirements: 7.1, 7.2, 7.3_

- [x] 2.2 InstrumentError 定義
  - InvalidParameter、MissingConvention、InvalidDate バリアントを追加
  - ValidationFailed、ExpansionFailed バリアントを追加
  - TradeError からの変換を実装
  - thiserror を使用した構造化エラー
  - _Requirements: 6.5, 7.4_

- [x] 2.3 (P) ConventionSet コンテナ実装
  - 各 Convention を Option<T> で保持する構造体を定義
  - get_*() メソッドで未設定時は MissingConvention エラー
  - with_*() builder パターンを実装
  - usd_standard()、eur_standard() プリセットを追加
  - _Requirements: 6.7_

## Phase 3: Instrument Definitions

- [x] 3. 金利商品（Rates）定義
- [x] 3.1 (P) Swaption 構造体定義
  - underlying_swap_tenor、expiry、exercise_type フィールドを定義
  - settlement_type、strike、notional、currency を追加
  - payer_receiver でロング/ショートを表現
  - serde feature-gating を適用
  - _Requirements: 1.1_

- [x] 3.2 (P) CapFloor 構造体定義
  - cap_floor_type（Cap/Floor/Collar）を定義
  - strikes、index、notional_schedule を追加
  - payment_frequency、start_date、tenor を設定
  - _Requirements: 1.2_

- [x] 3.3 (P) FRN（変動金利債）構造体定義
  - coupon_index、spread、reset_frequency を定義
  - principal_schedule で元本償還を表現
  - _Requirements: 1.3_

- [x] 3.4 (P) CmsSwap 構造体定義
  - cms_tenor（参照 CMS テナー）を定義
  - convexity_adjustment パラメータを追加
  - _Requirements: 1.4_

- [x] 3.5 (P) InflationSwap 構造体定義
  - inflation_index（CPI 等）を定義
  - lag_period、swap_type（ZeroCoupon/YearOnYear）を追加
  - _Requirements: 1.5_

- [x] 4. FX商品定義
- [x] 4.1 (P) FxSpot 構造体定義
  - currency_pair、spot_rate、settlement_date を定義
  - notional、notional_currency を追加
  - _Requirements: 2.1_

- [x] 4.2 (P) FxForward 構造体定義
  - currency_pair、forward_rate、settlement_date を定義
  - notional フィールドを追加
  - _Requirements: 2.2_

- [x] 4.3 (P) FxVanillaOption 構造体定義
  - currency_pair、strike、expiry、delivery_date を定義
  - option_type、exercise_style、notional を追加
  - _Requirements: 2.3_

- [x] 4.4 (P) FxBarrierOption 構造体定義
  - vanilla オプションをベースに barrier_level を追加
  - barrier_type（KnockIn/KnockOut）、barrier_direction（Up/Down）を定義
  - rebate オプションを追加
  - _Requirements: 2.4_

- [x] 4.5 (P) FxSwap（短期）構造体定義
  - near_leg_date、far_leg_date を定義
  - near_rate、far_rate を追加
  - _Requirements: 2.5_

- [x] 5. 株式商品（Equity）定義
- [x] 5.1 (P) EquityForward 構造体定義
  - underlying（単一株式/インデックス）を定義
  - forward_price、settlement_date を追加
  - _Requirements: 3.1_

- [x] 5.2 (P) EquityVanillaOption 構造体定義
  - underlying、strike、expiry を定義
  - option_type、exercise_style を追加
  - _Requirements: 3.2_

- [x] 5.3 (P) EquityBarrierOption 構造体定義
  - バニラオプションをベースにバリア情報を追加
  - monitoring_frequency（Continuous/Discrete）を定義
  - _Requirements: 3.3_

- [x] 5.4 (P) AsianOption 構造体定義
  - averaging_type（Arithmetic/Geometric）を定義
  - observation_frequency、observed_values を追加
  - _Requirements: 3.4_

- [x] 5.5 (P) LookbackOption 構造体定義
  - lookback_type（FixedStrike/FloatingStrike）を定義
  - observation_period を追加
  - _Requirements: 3.5_

- [x] 5.6 (P) EquitySwap 構造体定義
  - equity_leg（return_type: Price/TotalReturn）を定義
  - funding_leg（金利レグ）を追加
  - _Requirements: 3.6_

- [x] 5.7 (P) BasketOption 構造体定義
  - components（構成銘柄リスト）を定義
  - weights、correlation_matrix_ref を追加
  - _Requirements: 3.7_

- [x] 6. クレジット商品（Credit）定義
- [x] 6.1 (P) Cds 構造体定義
  - reference_entity、notional、spread を定義
  - start_date、maturity、recovery_rate を追加
  - _Requirements: 4.1_

- [x] 6.2 (P) CdsIndex 構造体定義
  - index_name（CDX/iTraxx）、series、version を定義
  - constituent_count を追加
  - _Requirements: 4.2_

- [x] 6.3 (P) CdsOption 構造体定義
  - underlying_cds への参照を定義
  - strike_spread、exercise_date を追加
  - _Requirements: 4.3_

- [x] 6.4 (P) NtdBasket 構造体定義
  - basket_constituents を定義
  - nth_to_default パラメータを追加
  - correlation_parameter を定義
  - _Requirements: 4.4_

- [x] 6.5 (P) CreditEvent 列挙型定義
  - Bankruptcy、FailureToPay、Restructuring を定義
  - ObligationAcceleration、ObligationDefault を追加
  - ISDA 標準定義に準拠
  - _Requirements: 4.5_

- [x] 7. コモディティ商品（Commodity）定義
- [x] 7.1 (P) CommodityForward 構造体定義
  - commodity（CommodityType）、delivery_location を定義
  - delivery_date、quantity、unit を追加
  - forward_price を定義
  - _Requirements: 5.1_

- [x] 7.2 (P) CommoditySwap 構造体定義
  - fixed_price_leg を定義
  - floating_price_leg（インデックス参照）を追加
  - _Requirements: 5.2_

- [x] 7.3 (P) CommodityVanillaOption 構造体定義
  - underlying_commodity、strike、expiry を定義
  - settlement_type（Cash/Physical）を追加
  - _Requirements: 5.3_

- [x] 7.4 (P) CommodityAsianOption 構造体定義
  - averaging_period、observation_frequency を定義
  - _Requirements: 5.4_

- [x] 7.5 (P) SpreadOption 構造体定義
  - commodity_1、commodity_2 を定義
  - spread_strike を追加
  - _Requirements: 5.5_

- [x] 7.6 (P) CommodityType 列挙型定義
  - Energy（EnergyType）、Metals（MetalType）を定義
  - Agriculture（AgricultureType）を追加
  - サブタイプ列挙型も定義
  - _Requirements: 5.6_

## Phase 4: InstrumentDefinition Integration

- [x] 8. InstrumentDefinition 統合
- [x] 8.1 InstrumentDefinition enum 定義
  - 全資産クラスのバリアントを統合
  - 既存キャリブレーション商品を re-export & extend
  - CrossCurrencySwap を Existing セクションに統合
  - serde feature-gating を適用
  - _Requirements: 7.1, 7.2, 7.3, 1.6_

- [x] 8.2 ヘルパーメソッド実装
  - asset_class() で資産クラスを返却
  - is_option()、is_swap()、is_forward() 判定メソッド
  - 各商品バリアントのパターンマッチで実装
  - _Requirements: 7.5_

- [x] 8.3 バリデーション機能実装
  - validate() メソッドを InstrumentDefinition に実装
  - 各バリアントの個別バリデーションロジック
  - 不正パラメータ検出（負のノーショナル、無効な日付等）
  - _Requirements: 7.4_

## Phase 5: Convention Extensions

- [x] 9. 新規 Convention 定義
- [x] 9.1 (P) SwaptionConvention 定義
  - underlying_swap（SwapConvention）を参照
  - premium_settlement、exercise_settlement を追加
  - premium_currency を定義
  - _Requirements: 1.1, 6.7_

- [x] 9.2 (P) FxOptionConvention 定義
  - premium_currency（Base/Quote/Custom）を定義
  - delta_convention（SpotDelta/ForwardDelta）を追加
  - cut_off_time、settlement_days を定義
  - _Requirements: 2.3, 2.4, 6.7_

- [x] 9.3 (P) EquityConvention 定義
  - settlement_days、calendar を定義
  - dividend_convention を追加
  - _Requirements: 3.1, 3.2, 6.7_

- [x] 9.4 (P) CommodityConvention 定義
  - delivery_convention を定義
  - pricing_calendar、settlement_days を追加
  - _Requirements: 5.1, 5.2, 6.7_

- [x] 9.5 (P) InflationSwapConvention 定義
  - inflation_index_convention を定義
  - lag_convention、interpolation_method を追加
  - _Requirements: 1.5, 6.7_

## Phase 6: CF Expansion Implementation

- [x] 10. InstrumentExpander トレイト実装
- [x] 10.1 InstrumentExpander トレイト定義
  - expand_to_trade() メソッドシグネチャを定義
  - ConventionSet を引数として受け取る
  - Result<Trade, InstrumentError> を返却
  - _Requirements: 6.1_

- [x] 10.2 金利商品の CF 展開実装
  - Swaption、CapFloor の展開ロジック
  - FRN、CmsSwap、InflationSwap の展開
  - 適切な Cashflow 生成
  - _Requirements: 6.1, 1.1, 1.2, 1.3, 1.4, 1.5_

- [x] 10.3 FX 商品の CF 展開実装
  - FxSpot、FxForward の元本交換 Cashflow
  - FxSwap の Near/Far leg 展開
  - FxVanillaOption、FxBarrierOption の条件付きペイオフ
  - _Requirements: 6.2, 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 10.4 株式商品の CF 展開実装
  - EquityForward、EquityVanillaOption の展開
  - AsianOption、LookbackOption の経路依存ペイオフ
  - EquitySwap、BasketOption の展開
  - _Requirements: 6.3, 6.4, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_

- [x] 10.5 クレジット商品の CF 展開実装
  - Cds のプレミアムレグと保護レグ
  - CdsIndex、CdsOption の展開
  - NtdBasket の展開
  - _Requirements: 6.1, 4.1, 4.2, 4.3, 4.4_

- [x] 10.6 コモディティ商品の CF 展開実装
  - CommodityForward の受渡 Cashflow
  - CommoditySwap の固定/変動レグ
  - オプション商品の条件付きペイオフ
  - _Requirements: 6.1, 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 10.7 CF 展開結果の検証
  - Trade::all_cashflows() との互換性確認
  - 展開結果の Cashflow 列が正しく列挙されることを検証
  - _Requirements: 6.6_

## Phase 7: Testing

- [x] 11. テスト実装
- [x] 11.1 単体テスト実装
  - 各商品タイプの構築テスト
  - validate() メソッドのテスト
  - is_option()、is_swap()、asset_class() のテスト
  - serde 往復変換テスト
  - _Requirements: 8.1_

- [x] 11.2 CF 展開統合テスト
  - InstrumentExpander::expand_to_trade() のテスト
  - 展開結果の Cashflow 列検証
  - Convention との整合性テスト
  - _Requirements: 8.2_

- [x] 11.3 エッジケーステスト
  - ゼロノーショナルの検証
  - 同一日の開始/終了日の検証
  - 空の観測値リスト（Asian）の検証
  - 負のストライクの検証
  - _Requirements: 8.3, 8.4_

- [x] 11.4 プロパティベーステスト
  - proptest による任意パラメータでの validate() テスト
  - 有効な商品の展開可能性検証
  - 展開後の Cashflow 数の一貫性検証
  - _Requirements: 8.5_

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 3.1, 9.1, 10.2 |
| 1.2 | 3.2, 10.2 |
| 1.3 | 3.3, 10.2 |
| 1.4 | 3.4, 10.2 |
| 1.5 | 3.5, 9.5, 10.2 |
| 1.6 | 8.1 |
| 2.1 | 4.1, 10.3 |
| 2.2 | 4.2, 10.3 |
| 2.3 | 4.3, 9.2, 10.3 |
| 2.4 | 4.4, 9.2, 10.3 |
| 2.5 | 4.5, 10.3 |
| 2.6 | 8.1 (CrossCurrencySwap 統合) |
| 3.1 | 5.1, 9.3, 10.4 |
| 3.2 | 5.2, 9.3, 10.4 |
| 3.3 | 5.3, 10.4 |
| 3.4 | 5.4, 10.4 |
| 3.5 | 5.5, 10.4 |
| 3.6 | 5.6, 10.4 |
| 3.7 | 5.7, 10.4 |
| 4.1 | 6.1, 10.5 |
| 4.2 | 6.2, 10.5 |
| 4.3 | 6.3, 10.5 |
| 4.4 | 6.4, 10.5 |
| 4.5 | 6.5 |
| 5.1 | 7.1, 9.4, 10.6 |
| 5.2 | 7.2, 9.4, 10.6 |
| 5.3 | 7.3, 10.6 |
| 5.4 | 7.4, 10.6 |
| 5.5 | 7.5, 10.6 |
| 5.6 | 7.6 |
| 6.1 | 10.1, 10.2, 10.5, 10.6 |
| 6.2 | 10.3 |
| 6.3 | 10.4 |
| 6.4 | 10.4 |
| 6.5 | 2.2 |
| 6.6 | 10.7 |
| 6.7 | 1.1, 1.2, 2.3, 9.1, 9.2, 9.3, 9.4, 9.5 |
| 7.1 | 2.1, 8.1 |
| 7.2 | 2.1, 8.1 |
| 7.3 | 2.1, 8.1 |
| 7.4 | 2.2, 8.3 |
| 7.5 | 8.2 |
| 7.6 | 8.1 |
| 8.1 | 11.1 |
| 8.2 | 11.2 |
| 8.3 | 11.3 |
| 8.4 | 11.3 |
| 8.5 | 11.4 |
