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
4. The Generic Pricer Engine shall `PricingResult<T>` 型でAD互換の結果を返す（`T: Float`）
5. If プライシングに必要なマーケットデータが欠落している場合, then the Generic Pricer Engine shall 具体的なエラーメッセージを含む `PricingError::MissingMarketData` を返す

### Requirement 2: マーケットデータ統合

**Objective:** As a クオンツ開発者, I want イールドカーブ、ボラティリティサーフェス、為替レート等のマーケットデータを柔軟に設定できる, so that 様々な市場環境でプライシングできる

#### Acceptance Criteria

1. When プライサーが初期化されるとき, the Generic Pricer Engine shall `MarketProvider` または `MarketSnapshot` からマーケットデータを受け取る
2. The Generic Pricer Engine shall `pricer_models::market::curves` の `CurveEnum`（FlatCurve, InterpolatedCurve, CreditCurve）をサポートする
3. The Generic Pricer Engine shall `pricer_models::market::surfaces` の `VolSurfaceEnum` をサポートする
4. When 複数通貨の商品をプライシングするとき, the Generic Pricer Engine shall `CurveSet` から各通貨のディスカウントカーブを解決する
5. If 要求されたカーブまたはサーフェスが `MarketProvider` に存在しない場合, then the Generic Pricer Engine shall `MarketDataError::CurveNotFound` または `MarketDataError::SurfaceNotFound` を返す

### Requirement 3: モデル構成

**Objective:** As a クオンツ開発者, I want プライシングに使用する確率モデルとシミュレーション設定を柔軟に選択・設定できる, so that 商品特性に応じた適切なモデルでプライシングできる

#### Acceptance Criteria

1. The Generic Pricer Engine shall `ModelConfig` 構造体で以下の設定を受け取る：使用モデル（`StochasticModelEnum`）、シミュレーションパス数、時間ステップ数、乱数シード
2. The Generic Pricer Engine shall `StochasticModelEnum`（GBM、Heston、SABR、Hull-White、CIR）をサポートする
3. When モデルが明示的に指定されていないとき, the Generic Pricer Engine shall 商品タイプに応じたデフォルトモデルを選択する
4. Where キャリブレーション済みモデルパラメータが利用可能な場合, the Generic Pricer Engine shall `pricer_models::market::calibration` からパラメータを取得する
5. The Generic Pricer Engine shall Builderパターンで `ModelConfig` を構築できる
6. If モデルパラメータが不正な場合（例：Hestonの vol-of-vol < 0）, then the Generic Pricer Engine shall `ConfigError::InvalidModelParameter` を返す

### Requirement 4: プライサー設定

**Objective:** As a クオンツ開発者, I want Greeks計算モードおよびパフォーマンス設定を調整できる, so that ユースケースに応じた最適なトレードオフを実現できる

#### Acceptance Criteria

1. The Generic Pricer Engine shall `PricerConfig` 構造体で以下の設定を受け取る：Greeks計算モード、デフォルト出力通貨、バッチ処理設定
2. When `greeks_mode` が `AAD` に設定されているとき, the Generic Pricer Engine shall Enzyme ADを使用してGreeksを計算する
3. When `greeks_mode` が `BumpAndRevalue` に設定されているとき, the Generic Pricer Engine shall バンプ幅設定に基づいて有限差分法でGreeksを計算する
4. The Generic Pricer Engine shall Builderパターンで `PricerConfig` を構築できる
5. While バッチプライシング実行中, the Generic Pricer Engine shall スレッドローカルバッファプールを使用してメモリ割り当てを最小化する

### Requirement 5: 商品インターフェース

**Objective:** As a クオンツ開発者, I want `infra_domain::trade` のTrade構造および `pricer_models::instruments` のInstrument定義と統合できる, so that 既存の商品定義を再利用できる

#### Acceptance Criteria

1. The Generic Pricer Engine shall `infra_domain::trade::Trade`（CF-expanded形式）を入力として受け取る
2. The Generic Pricer Engine shall `pricer_models::instruments::InstrumentEnum` を入力として受け取る
3. When `Trade` が入力されたとき, the Generic Pricer Engine shall Leg/Cashflow構造をパースしてプライシングする
4. The Generic Pricer Engine shall 静的ディスパッチ（enum）を使用してEnzyme最適化を維持する
5. If 未対応の商品タイプが渡された場合, then the Generic Pricer Engine shall `PricingError::UnsupportedInstrument` を返す

### Requirement 6: 通貨・為替処理とPricingResult構造

**Objective:** As a クオンツ開発者, I want マルチ通貨商品のPVを任意の通貨建てで取得でき、かつ任意の粒度（Cashflow、Leg、Trade）でPV内訳にアクセスできる, so that ポートフォリオレベルの集計および詳細分析が可能になる

#### Acceptance Criteria

1. The Generic Pricer Engine shall `infra_domain::market::Currency` 列挙型をサポートする
2. When 商品通貨と出力通貨が異なるとき, the Generic Pricer Engine shall `MarketProvider` から為替レートを取得してPVを換算する
3. The Generic Pricer Engine shall `PricingResult<T>` を階層構造（Trade → Leg → Cashflow）で設計し、任意のレベルでPV内訳にアクセスできる
4. The Generic Pricer Engine shall 各Leg/Cashflowの元通貨を保持し、`PricingResult::group_by_currency()`で通貨別PV内訳を取得できる
5. When `PricingResult::by_leg()` が呼び出されたとき, the Generic Pricer Engine shall Leg単位でのPV集計を返す
6. When `PricingResult::by_cashflow()` が呼び出されたとき, the Generic Pricer Engine shall Cashflow単位でのPV詳細を返す
7. When `PricingResult::by_path()` が呼び出されたとき, the Generic Pricer Engine shall MCシミュレーションのパス単位でのPV分布を返す（MC計算の場合のみ）
8. If 為替レートが利用不可の場合, then the Generic Pricer Engine shall `MarketDataError::FxRateNotFound` を返す
9. The Generic Pricer Engine shall デフォルト出力通貨を `PricerConfig` で設定できる

### Requirement 7: 日付・時間処理

**Objective:** As a クオンツ開発者, I want 評価日、決済日、カレンダー調整を正確に処理できる, so that 金融商品の日付ロジックが正しく適用される

#### Acceptance Criteria

1. The Generic Pricer Engine shall `infra_domain::time` の `Calendar`、`DayCountConvention`、`Frequency` をサポートする
2. When 評価日が休日の場合, the Generic Pricer Engine shall 設定された営業日調整（Following、ModifiedFollowing等）を適用する
3. The Generic Pricer Engine shall time_to_maturity計算に `DayCountConvention` を使用する
4. While フォワードレート計算中, the Generic Pricer Engine shall カーブのテナー構造に基づいて適切な日付補間を行う
5. The Generic Pricer Engine shall `chrono::NaiveDate` を日付型として使用する

### Requirement 8: バッチプライシング

**Objective:** As a リスク管理者, I want 複数商品を効率的にバッチプライシングできる, so that ポートフォリオレベルのリスク計算が高速に実行できる

#### Acceptance Criteria

1. When `price_batch(trades, market, config)` が呼び出されたとき, the Generic Pricer Engine shall 複数商品を並列プライシングする
2. The Generic Pricer Engine shall `rayon` を使用した並列処理をサポートする
3. While バッチプライシング中, the Generic Pricer Engine shall マーケットデータを共有（Arc-cached）してメモリ使用量を最適化する
4. The Generic Pricer Engine shall `BatchPricingResult` で成功/失敗を商品別に返す
5. If 一部の商品がエラーになった場合, then the Generic Pricer Engine shall 他の商品のプライシングを継続し、エラー商品のリストを結果に含める

