# Requirements Document

## Introduction

本仕様は、A-I-P-Sアーキテクチャの依存関係ルール違反を解消するため、`pricer_core`で定義されている基本型（primitives）を`infra_domain`へ移動するリファクタリングプロジェクトを定義する。

### 背景

現在、`Currency`、`Date`、`DayCountConvention`、`BusinessDayConvention`などの基本型は`pricer_core`（Pricerレイヤー L1）に定義されている。しかし、`infra_domain`（Infraレイヤー）がこれらの型を使用するには、Infra→Pricer の依存が発生し、A-I-P-Sルール「InfraはPricerに依存してはいけない」に違反する。

また、`DayCountConvention`は現在`pricer_core`と`infra_domain`の両方に異なる実装が存在しており、統一が必要である。

### 対象範囲

**Phase 1: 基盤型の移動（pricer_core → infra_domain）**
- `Currency` enum（ISO 4217通貨コード）
- `Date` struct（chrono::NaiveDateラッパー）
- `DayCountConvention` enum（日数計算規約）- 統合
- `BusinessDayConvention` enum（営業日調整規約）
- 関連エラー型（`DateError`、`CurrencyError`）

**Phase 2: マスターデータ型の移動（pricer_models → infra_domain）**
- `RateIndex` enum（SOFR, TONAR, EURIBOR等のベンチマーク金利）
- `Frequency` enum（Annual, SemiAnnual, Quarterly等の頻度）
- `Period` struct（単一のaccrual期間）
- `TradeDirection` enum（Long/Short, PayFixed/ReceiveFixed等の統合）
- `Tenor` enum（新規追加: 3M, 6M, 1Y等の金融期間表現）

**移動対象外の型（理由付き）**:
- `CurrencyPair<T>` - ジェネリック型`T: Float`がAD対応のため。`rate`フィールドがpricing計算に直接関与
- `time_to_maturity()` - 数学計算関数であり、AD互換のジェネリック実装が必要
- `Dual`型 - ADバックエンド特化、pricer_coreの中核機能
- `PayoffType` - `evaluate()`メソッドが`smooth_max`/`smooth_indicator`に依存しており、pricing計算と一体
- `ExerciseStyle<T>` - ジェネリック型で時間値をAD対応型で保持。Bermudan/Asianは複雑なタイミングデータ含む

## Requirements

### Requirement 1: 基本型の`infra_domain`への移動

**Objective:** As a 開発者, I want Currency, Date, DayCountConvention, BusinessDayConventionが`infra_domain`に定義されている, so that A-I-P-Sアーキテクチャの依存関係ルールを遵守でき、Infraレイヤーから基本型を安全に使用できる

#### Acceptance Criteria

*[2 additional criteria omitted]*
1. The `infra_domain` crate shall export `Currency` enum with at least USD, EUR, GBP, JPY, CHF variants
2. The `infra_domain` crate shall export `Date` struct wrapping `chrono::NaiveDate`
3. The `infra_domain` crate shall export unified `DayCountConvention` enum supporting Actual360, Actual365Fixed, ActualActualIsda, Thirty360Bond variants
### Requirement 2: `DayCountConvention`の統一

**Objective:** As a 開発者, I want DayCountConventionが単一の統一された定義を持つ, so that コードベース全体で一貫した日数計算規約を使用できる

#### Acceptance Criteria

*[1 additional criteria omitted]*
1. The `infra_domain` crate shall define a single authoritative `DayCountConvention` enum
2. The unified `DayCountConvention` shall include: Actual360, Actual365Fixed, Actual36525, ActualActualIsda, Thirty360Bond, Thirty360European, ThirtyE360Isda
3. When year fraction calculation is performed, the `infra_domain::DayCountConvention` shall provide the `year_fraction(start: Date, end: Date) -> f64` method
### Requirement 3: `pricer_core`からの再エクスポート（後方互換性）

**Objective:** As a 既存コード利用者, I want pricer_coreからの既存インポートパスが引き続き動作する, so that 移行期間中にコードの大規模変更なしに段階的な移行が可能である

#### Acceptance Criteria

*[2 additional criteria omitted]*
1. When existing code imports `pricer_core::types::Currency`, the import shall continue to work via re-export
2. When existing code imports `pricer_core::types::Date`, the import shall continue to work via re-export
3. The `pricer_core` crate shall add `infra_domain` as a dependency
### Requirement 4: 関連エラー型の移動

**Objective:** As a 開発者, I want DateErrorとCurrencyErrorがinfra_domainに定義されている, so that エラーハンドリングが基本型と同じ場所に配置され一貫性が保たれる

#### Acceptance Criteria

*[1 additional criteria omitted]*
1. The `infra_domain` crate shall define `DateError` enum for date-related errors
2. The `infra_domain` crate shall define `CurrencyError` enum for currency-related errors
3. When `Date::from_ymd(year, month, day)` receives invalid values, the function shall return `Result<Date, DateError>`
### Requirement 5: Calendar統合（既存機能の連携強化）

**Objective:** As a 開発者, I want DateとCalendarが同じクレートで連携する, so that 営業日調整計算が一箇所で完結し、依存関係が簡素化される

#### Acceptance Criteria

*[1 additional criteria omitted]*
1. The `infra_domain::Calendar` shall use `infra_domain::Date` directly (no external type dependency)
2. When `calendar.add_business_days(date, n)` is called, the function shall return `Date`
3. When `calendar.is_business_day(date)` is called, the function shall return `bool`
### Requirement 6: serdeフィーチャーフラグの維持

**Objective:** As a API開発者, I want 基本型のシリアライゼーションがオプショナル機能として維持される, so that バイナリサイズの最適化と必要な場合のJSON/TOML変換が両立できる

#### Acceptance Criteria

1. Where the `serde` feature is enabled, the `Currency`, `Date`, `DayCountConvention`, `BusinessDayConvention` types shall derive `Serialize` and `Deserialize`
2. Where the `serde` feature is disabled, the types shall not include serde dependencies
3. The `infra_domain/Cargo.toml` shall declare serde as `optional = true`
### Requirement 7: 依存関係のアーキテクチャ準拠検証

**Objective:** As a アーキテクト, I want 移行後の依存関係がA-I-P-Sルールに準拠していることを検証可能, so that アーキテクチャの整合性が保証される

#### Acceptance Criteria

*[1 additional criteria omitted]*
1. The `infra_domain` crate shall NOT have dependencies on any `pricer_*`, `adapter_*`, or `service_*` crate
2. The `pricer_core` crate shall have `infra_domain` as a dependency for type re-exports
3. When `cargo tree -p infra_domain` is executed, the output shall show no pricer/adapter/service dependencies
### Requirement 8: RateIndex（ベンチマーク金利指標）の移動

**Objective:** As a アダプター開発者, I want RateIndexがinfra_domainに定義されている, so that adapter_feedsやadapter_fpmlからベンチマーク金利の参照データにアクセスできる

#### Acceptance Criteria

*[2 additional criteria omitted]*
1. The `infra_domain` crate shall export `RateIndex` enum with SOFR, TONAR, EURIBOR3M, EURIBOR6M, GBPLIBOR3M, JPYLIBOR6M variants
2. The `RateIndex` enum shall provide `currency() -> Currency` method returning the associated currency
3. The `RateIndex` enum shall provide `tenor() -> Tenor` method returning the standard fixing tenor
### Requirement 9: Frequency（支払頻度）の移動

**Objective:** As a 開発者, I want Frequencyがinfra_domainに定義されている, so that スケジュール生成の基本列挙型がマスターデータとして一元管理される

#### Acceptance Criteria

*[1 additional criteria omitted]*
1. The `infra_domain` crate shall export `Frequency` enum with Annual, SemiAnnual, Quarterly, Monthly, Weekly, Daily variants
2. The `Frequency` enum shall provide `months_per_period() -> u32` method
3. The `Frequency` enum shall provide `periods_per_year() -> u32` method
### Requirement 10: Period（単一期間）の移動

**Objective:** As a 開発者, I want Periodがinfra_domainに定義されている, so that アクルーアル期間の構造体がスケジュール構築の基盤としてInfraレイヤーで利用可能になる

#### Acceptance Criteria

*[1 additional criteria omitted]*
1. The `infra_domain` crate shall export `Period` struct with `start: Date`, `end: Date`, `payment: Date` fields
2. The `Period` struct shall provide `accrual_days(day_count: DayCountConvention) -> i64` method
3. The `Period` struct shall provide `year_fraction(day_count: DayCountConvention) -> f64` method
### Requirement 11: TradeDirection（取引方向）の統合と移動

**Objective:** As a 開発者, I want 取引方向を表す型が統一されてinfra_domainに定義されている, so that 散在するDirection型が一元化され、アダプターからも利用可能になる

#### Acceptance Criteria

*[1 additional criteria omitted]*
1. The `infra_domain` crate shall export unified `TradeDirection` enum with Long, Short variants
2. The `infra_domain` crate shall export `SwapDirection` enum with PayFixed, ReceiveFixed variants
3. When existing code imports direction types from `pricer_models`, the imports shall continue to work via re-export
### Requirement 12: CsaTermsのCurrency型統合

**Objective:** As a 開発者, I want CsaTermsがString型ではなくCurrency型を使用する, so that 型安全性が向上し、不正な通貨コード入力を防止できる

#### Acceptance Criteria

1. When `CsaTerms` is defined, the `collateral_currency` field shall use `Currency` type instead of `String`
2. When `CsaTerms` is constructed with an invalid currency, the construction shall fail at compile time (not runtime)
3. The migration shall update all existing `CsaTerms` usages to use `Currency` type
### Requirement 13: Tenor（期間表現）の追加

**Objective:** As a 開発者, I want Tenor型がinfra_domainに定義されている, so that "3M", "6M", "1Y"などの金融期間表現を型安全に扱える

#### Acceptance Criteria
