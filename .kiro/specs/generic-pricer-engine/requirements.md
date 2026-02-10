# Requirements Document

## Introduction

本仕様は、`pricer_pricing`クレート（L3層）に汎用プライサーエンジンを実装するための要件を定義する。このエンジンは、A-I-P-Sアーキテクチャの3-stage rocketパターンに従い、マーケットデータ（イールドカーブ、ボラティリティサーフェス）、ユーザー設定、およびモデル構成を統合して、任意の金融商品のPV（現在価値）およびリスク指標を計算する。

## Requirements

### Requirement 1: コアプライシングインターフェース

**Objective:** As a クオンツ開発者, I want 統一されたプライシングAPIを通じてPVおよびGreeksを取得できる, so that 様々な商品を一貫した方法で評価できる

#### Acceptance Criteria


1. When `get_pv(valuation_date, reporting_currency)` が呼び出されたとき, the Generic Pricer Engine shall 指定された評価日時点で、指定された報告通貨建てのPV（現在価値）を計算して返す
2. ※廃止: `reporting_currency`は`get_pv()`の必須引数に統合（リスク計算の前提条件）
3. When `get_greeks(valuation_date, greeks_config)` が呼び出されたとき, the Generic Pricer Engine shall 設定に基づいてDelta、Gamma、Vega等のGreeksを計算して返す
*[2 additional criteria omitted]*
### Requirement 2: マーケットデータ統合

**Objective:** As a クオンツ開発者, I want イールドカーブ、ボラティリティサーフェス、為替レート等のマーケットデータを柔軟に設定できる, so that 様々な市場環境でプライシングできる

#### Acceptance Criteria


1. When プライサーが初期化されるとき, the Generic Pricer Engine shall `MarketProvider` または `MarketSnapshot` からマーケットデータを受け取る
2. The Generic Pricer Engine shall `pricer_models::market::curves` の `CurveEnum`（FlatCurve, InterpolatedCurve, CreditCurve）をサポートする
3. The Generic Pricer Engine shall `pricer_models::market::surfaces` の `VolSurfaceEnum` をサポートする
*[2 additional criteria omitted]*
### Requirement 3: モデル構成

**Objective:** As a クオンツ開発者, I want プライシングに使用する確率モデルとシミュレーション設定を柔軟に選択・設定できる, so that 商品特性に応じた適切なモデルでプライシングできる

#### Acceptance Criteria


1. The Generic Pricer Engine shall `ModelConfig` 構造体で以下の設定を受け取る：使用モデル（`StochasticModelEnum`）、シミュレーションパス数、時間ステップ数、乱数シード
2. The Generic Pricer Engine shall `StochasticModelEnum`（GBM、Heston、SABR、Hull-White、CIR）をサポートする
3. When モデルが明示的に指定されていないとき, the Generic Pricer Engine shall 商品タイプに応じたデフォルトモデルを選択する
*[3 additional criteria omitted]*
### Requirement 4: プライサー設定

**Objective:** As a クオンツ開発者, I want Greeks計算モードおよびパフォーマンス設定を調整できる, so that ユースケースに応じた最適なトレードオフを実現できる

#### Acceptance Criteria


1. The Generic Pricer Engine shall `PricerConfig` 構造体で以下の設定を受け取る：Greeks計算モード、デフォルト出力通貨、バッチ処理設定
2. When `greeks_mode` が `AAD` に設定されているとき, the Generic Pricer Engine shall Enzyme ADを使用してGreeksを計算する
3. When `greeks_mode` が `BumpAndRevalue` に設定されているとき, the Generic Pricer Engine shall バンプ幅設定に基づいて有限差分法でGreeksを計算する
*[2 additional criteria omitted]*
### Requirement 5: 商品インターフェース

**Objective:** As a クオンツ開発者, I want `infra_domain::trade` のTrade構造および `pricer_models::instruments` のInstrument定義と統合できる, so that 既存の商品定義を再利用できる

#### Acceptance Criteria


1. The Generic Pricer Engine shall `infra_domain::trade::Trade`（CF-expanded形式）を入力として受け取る
2. The Generic Pricer Engine shall `pricer_models::instruments::InstrumentEnum` を入力として受け取る
3. When `Trade` が入力されたとき, the Generic Pricer Engine shall Leg/Cashflow構造をパースしてプライシングする
*[2 additional criteria omitted]*
### Requirement 6: 通貨・為替処理とPricingResult構造

**Objective:** As a クオンツ開発者, I want マルチ通貨商品のPVを任意の通貨建てで取得でき、かつ任意の粒度（Cashflow、Leg、Trade）でPV内訳にアクセスできる, so that ポートフォリオレベルの集計および詳細分析が可能になる

#### Acceptance Criteria


1. The Generic Pricer Engine shall `infra_domain::market::Currency` 列挙型をサポートする
2. When 商品通貨と出力通貨が異なるとき, the Generic Pricer Engine shall `MarketProvider` から為替レートを取得してPVを換算する
3. The Generic Pricer Engine shall `PricingResult<T>` を階層構造（Trade → Leg → Cashflow）で設計し、任意のレベルでPV内訳にアクセスできる
*[6 additional criteria omitted]*
### Requirement 7: 日付・時間処理

**Objective:** As a クオンツ開発者, I want 評価日、決済日、カレンダー調整を正確に処理できる, so that 金融商品の日付ロジックが正しく適用される

#### Acceptance Criteria


1. The Generic Pricer Engine shall `infra_domain::time` の `Calendar`、`DayCountConvention`、`Frequency` をサポートする
2. When 評価日が休日の場合, the Generic Pricer Engine shall 設定された営業日調整（Following、ModifiedFollowing等）を適用する
3. The Generic Pricer Engine shall time_to_maturity計算に `DayCountConvention` を使用する
*[2 additional criteria omitted]*
### Requirement 8: バッチプライシング

**Objective:** As a リスク管理者, I want 複数商品を効率的にバッチプライシングできる, so that ポートフォリオレベルのリスク計算が高速に実行できる

#### Acceptance Criteria


