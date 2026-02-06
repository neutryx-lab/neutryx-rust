# Requirements Document

## Introduction

本仕様は、Neutryx プライシングパイプライン全体における RateIndex（金利指標）の包括的な統合を定義する。現状、RateIndex は infra_domain で定義されているが、実際のプライシングプロセスでは使用されていない。GenericPricer は cf.payoff を無視し、MarketProvider は Currency のみでカーブをマッピングしている。本仕様により、変動金利キャッシュフローの正確な評価を実現する。

**対象レイヤー**: infra_domain (I) → pricer_models (P/L2) → pricer_pricing (P/L3) → demo/gui (D)

## Requirements

### Requirement 1: RateIndex メタデータ拡張

**Objective:** As a クオンツ開発者, I want RateIndex に必要なフィクシングメタデータを含める, so that プライシングエンジンが正確なレート観測を行える

#### Acceptance Criteria

1. The RateIndex shall フィクシングカレンダー（fixing_calendar: CalendarId）を保持する
2. The RateIndex shall 公表ラグ（publication_lag: i32、営業日数）を保持する
3. The RateIndex shall フィクシングオフセット（fixing_offset: i32、アクルーアル開始からの営業日数）を保持する
4. The RateIndex shall コンパウンディング方式（compounding_method: Simple | Compounded | Averaged）を保持する
5. When RateIndex::metadata() が呼び出された場合, the RateIndex shall すべてのフィクシングパラメータを含む IndexMetadata 構造体を返す
6. The RateIndex shall 既存の currency(), tenor(), day_counter(), name(), code() メソッドとの後方互換性を維持する

### Requirement 2: IndexObservation 強化

**Objective:** As a クオンツ開発者, I want IndexObservation に完全な観測パラメータを含める, so that OIS コンパウンディングや複雑なフィクシングルールに対応できる

#### Acceptance Criteria

1. The IndexObservation shall リセット頻度（reset_frequency: Frequency）を保持する
2. The IndexObservation shall コンパウンディング方式（compounding_method: CompoundingMethod）を保持する
3. The IndexObservation shall ルックバック期間（lookback_period: Option<i32>）を保持する
4. The IndexObservation shall ロックアウト期間（lockout_period: Option<i32>）を保持する
5. When IndexObservation が OIS インデックスで作成された場合, the IndexObservation shall デフォルトで Compounded 方式を設定する
6. When IndexObservation が IBOR インデックスで作成された場合, the IndexObservation shall デフォルトで Simple 方式を設定する

### Requirement 3: RateIndex からカーブへのマッピング

**Objective:** As a クオンツ開発者, I want RateIndex から適切なカーブを解決する, so that 変動金利キャッシュフローに正しいフォワードレートを適用できる

#### Acceptance Criteria

1. The pricer_models shall RateIndex を CurveName にマッピングする IndexCurveMapper トレイトを提供する
2. When RateIndex::Sofr がマッピングされた場合, the IndexCurveMapper shall CurveName::Sofr を返す
3. When RateIndex::Euribor3M または Euribor6M がマッピングされた場合, the IndexCurveMapper shall CurveName::Euribor を返す
4. When RateIndex::Sonia がマッピングされた場合, the IndexCurveMapper shall CurveName::Sonia を返す
5. The CurveSet shall get_curve_for_index(index: RateIndex) メソッドを提供する
6. If 指定された RateIndex に対応するカーブが存在しない場合, the CurveSet shall MarketDataError::CurveNotFound を返す

### Requirement 4: インデックス対応フォワードレート計算

**Objective:** As a クオンツ開発者, I want インデックス固有のフォワードレート計算を行う, so that 日数計算規則やコンパウンディング方式が正しく適用される

#### Acceptance Criteria

1. The pricer_models shall forward_rate_for_index(index: RateIndex, start: f64, end: f64) メソッドを提供する
2. When フォワードレートが計算される場合, the pricer_models shall インデックスの day_counter() に基づいた年率換算を行う
3. When OIS インデックスのフォワードレートが計算される場合, the pricer_models shall 日次複利計算をサポートする
4. When IBOR インデックスのフォワードレートが計算される場合, the pricer_models shall 単利計算を使用する
5. The フォワードレート計算 shall Float トレイト境界を使用し、AD（自動微分）と互換性を持つ

### Requirement 5: GenericPricer Payoff 評価

**Objective:** As a クオンツ開発者, I want GenericPricer が Payoff を正しく評価する, so that 変動金利キャッシュフローが正確に価格付けされる

#### Acceptance Criteria

1. When Payoff::Fixed が評価される場合, the GenericPricer shall notional × rate × year_fraction を計算する
2. When Payoff::Linear が評価される場合, the GenericPricer shall インデックスからフォワードレートを取得し、notional × (forward_rate + spread) × multiplier × year_fraction を計算する
3. When Payoff::Linear の index が IndexType::Rate(RateIndex) の場合, the GenericPricer shall 対応するカーブからフォワードレートを取得する
4. If Payoff::Linear のインデックスに対応するカーブが存在しない場合, the GenericPricer shall PricingError::MissingMarketData を返す
5. The GenericPricer shall cf.notional を使用し、ハードコードされた値を使用しない
6. The Payoff 評価 shall Float トレイト境界を使用し、Dual64 による AD をサポートする

### Requirement 6: OIS コンパウンディングサポート

**Objective:** As a クオンツ開発者, I want OIS キャッシュフローの日次複利計算を行う, so that SOFR/SONIA スワップを正確に評価できる

#### Acceptance Criteria

1. When Cashflow に daily_accruals が存在する場合, the GenericPricer shall 日次複利計算を使用する
2. The GenericPricer shall 各 DailyAccrual の overnight_rate と day_fraction を使用して複利計算する
3. When daily_accruals が空の場合, the GenericPricer shall 期間全体のフォワードレートを使用する
4. The OIS 複利計算 shall ∏(1 + r_i × δ_i) - 1 の公式を使用する
5. The OIS 複利計算 shall Float トレイト境界を使用し、AD と互換性を持つ

### Requirement 7: Cap/Floor オプション評価

**Objective:** As a クオンツ開発者, I want Cap/Floor オプションを評価する, so that Payoff::VanillaOption を正確に価格付けできる

#### Acceptance Criteria

1. When Payoff::VanillaOption が評価される場合, the GenericPricer shall Black/Bachelier モデルを使用してオプション価値を計算する
2. The GenericPricer shall インデックスに対応するボラティリティサーフェスを取得する
3. When OptionType::Call の場合, the GenericPricer shall max(forward_rate - strike, 0) のペイオフを評価する
4. When OptionType::Put の場合, the GenericPricer shall max(strike - forward_rate, 0) のペイオフを評価する
5. If ボラティリティサーフェスが存在しない場合, the GenericPricer shall PricingError::MissingVolatility を返す

### Requirement 8: Demo WebApp 入力 DTO 拡張

**Objective:** As a API ユーザー, I want トレード作成時にインデックスを指定する, so that 任意の金利指標でスワップを構築できる

#### Acceptance Criteria

1. The SwapParams shall rate_index: Option<String> フィールドを持つ
2. The RatesParams shall rate_index: Option<String> フィールドを持つ
3. When rate_index が指定されない場合, the trade_handlers shall 通貨に基づくデフォルトインデックスを使用する
4. When rate_index に無効な値が指定された場合, the trade_handlers shall ApiError::InvalidInput を返す
5. The rate_index shall "SOFR", "EURIBOR3M", "EURIBOR6M", "SONIA", "TONAR", "SARON" を受け付ける

### Requirement 9: Demo WebApp 出力 DTO 拡張

**Objective:** As a API ユーザー, I want レスポンスでインデックス情報を確認する, so that トレード構造を完全に理解できる

#### Acceptance Criteria

1. The LegDto shall rate_index: Option<String> フィールドを持つ
2. The CashflowDto shall rate_index: Option<String> フィールドを持つ
3. When Leg が変動金利の場合, the LegDto shall 対応する rate_index を含む
4. When Cashflow の payoff が Linear または VanillaOption の場合, the CashflowDto shall 対応する rate_index を含む
5. The convert_trade_to_dto() shall Payoff::required_index() を使用してインデックス情報を抽出する

### Requirement 10: 後方互換性と AD サポート

**Objective:** As a クオンツ開発者, I want 既存のテストとAD機能が維持される, so that 安全にアップグレードできる

#### Acceptance Criteria

1. The 全ての変更 shall 既存の単体テストを破壊しない
2. The 全ての数値計算 shall Float トレイト境界を使用し、f64 と Dual64 の両方で動作する
3. The RateIndex の変更 shall 既存の API シグネチャを維持する（新規メソッドの追加のみ）
4. The CurveSet の変更 shall 既存の get(), insert() メソッドを維持する
5. When l1l2-integration フィーチャーが無効の場合, the pricer_pricing shall スタンドアロンモードで動作し続ける
