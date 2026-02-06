# Requirements Document

## Introduction

本仕様は、`crates/infra_domain/src/` 内の既存時間関連モジュールを `time/` サブディレクトリに再編成し、金融デリバティブプライシングエンジンの基盤となる時間管理機能を完成させる。既存実装の 80% 以上を活用しつつ、不足機能（Excel serial 変換、JointCalendar、Calendar trait 化、汎用 Period + TimeUnit）を追加する。

### Design Principles

* **Static Dispatch for Hot Paths:** 計算頻度の高い `DayCounter` は Enum と `match` 式を用いて実装し、VTable の参照コストを回避する。
* **New Type Pattern:** `chrono::NaiveDate` をラップし、ドメイン固有の型安全性と機能拡張を提供する。
* **Error Handling:** パニック（panic）を避け、全ての計算は `Result<T, TimeError>` を返却する。
* **Dynamic Dispatch for Configuration:** カレンダーの組み合わせ（`JointCalendar`）など、構成の柔軟性が必要な箇所には `Box<dyn Trait>` を使用する。
* **British English:** コード内のすべてのコメントおよびドキュメントは British English で統一する。

### Target Directory Structure

```text
crates/infra_domain/src/
├── lib.rs                    # time モジュールを追加 + re-exports
├── time/
│   ├── mod.rs                # Module definition & re-exports
│   ├── error.rs              # TimeError definitions
│   ├── types.rs              # Date wrapper
│   ├── calendars.rs          # Calendar trait, BusinessDayConvention, JointCalendar
│   ├── day_counters.rs       # DayCounter enum
│   └── period.rs             # Period, TimeUnit, Tenor, AccrualPeriod
├── currency.rs               # 既存維持
├── rate_index.rs             # 既存維持
├── direction.rs              # 既存維持
├── counterparty.rs           # 既存維持
└── frequency.rs              # 既存維持
```

---

## Requirements

### Requirement 1: モジュール再編成と構造移行

**Objective:** As a ライブラリ開発者, I want 時間関連モジュールを `time/` サブディレクトリに整理したい, so that コードベースが仕様書の構造に適合し、保守性が向上する。

#### Acceptance Criteria

1. The infra_domain crate shall 既存の `date.rs`, `calendar.rs`, `business_day.rs`, `day_count.rs`, `tenor.rs`, `period.rs` を `time/` サブディレクトリに移動する。
2. The infra_domain crate shall `time/mod.rs` を作成し、すべてのサブモジュールを re-export する。
3. The infra_domain crate shall `lib.rs` にて `pub mod time;` を宣言し、後方互換性のための re-exports を提供する。
4. When 既存の public API が使用された場合, the infra_domain crate shall 非推奨警告（deprecated）を表示しつつ正常に動作する。
5. The infra_domain crate shall 移行後もすべての既存テストがパスする。

---

### Requirement 2: TimeError 統一エラー型

**Objective:** As a ライブラリ利用者, I want 時間関連の全エラーを単一の型で処理したい, so that エラーハンドリングが一貫し、パニックを回避できる。

#### Acceptance Criteria

1. The time module shall `TimeError` enum を定義し、以下のバリアントを含む: `InvalidDate`, `ParseError`, `CalculationError`, `CalendarError`。
2. The TimeError type shall `thiserror::Error` を derive し、各バリアントに適切なエラーメッセージを提供する。
3. The TimeError type shall `Debug`, `Clone`, `PartialEq` を derive し、テスト時の比較を可能にする。
4. When 無効な日付コンポーネントが指定された場合, the TimeError shall `InvalidDate { year, month, day }` を返却する。
5. When 日付文字列のパースに失敗した場合, the TimeError shall `ParseError(String)` を返却する。

---

### Requirement 3: Date 型の Excel Serial 変換

**Objective:** As a クォンツアナリスト, I want Date と Excel serial number を相互変換したい, so that スプレッドシートとのデータ連携が容易になる。

#### Acceptance Criteria

1. The Date type shall `to_serial(&self) -> i64` メソッドを提供し、Excel serial date（1900-01-01 = 1）を返却する。
2. The Date type shall `from_serial(serial: i64) -> Result<Self, TimeError>` メソッドを提供し、serial number から Date を構築する。
3. When 1900-02-28 より後の日付を変換する場合, the Date type shall Excel の leap year bug を考慮して +1 日補正する。
4. When 無効な serial number（負数または範囲外）が指定された場合, the Date type shall `TimeError::CalculationError` を返却する。
5. The Date type shall `to_serial` と `from_serial` の往復変換で元の日付と一致する。

---

### Requirement 4: Calendar Trait 化と抽象化

**Objective:** As a ライブラリ開発者, I want Calendar を trait として定義したい, so that 新しいカレンダー実装を容易に追加でき、JointCalendar で結合できる。

#### Acceptance Criteria

1. The calendars module shall `Calendar` trait を定義し、以下のメソッドを含む: `is_business_day`, `is_holiday`, `adjust`, `next_business_day`, `prev_business_day`, `add_business_days`。
2. The Calendar trait shall `Send + Sync` を要求し、スレッドセーフな使用を可能にする。
3. The Calendar trait shall `is_holiday` と `adjust` にデフォルト実装を提供する。
4. The calendars module shall `ConcreteCalendar` struct を定義し、既存の `Calendar` struct の実装を移行する。
5. When `CalendarId` が指定された場合, the ConcreteCalendar shall 対応するカレンダー（Target, NewYork, Tokyo, London, WeekendOnly）の営業日判定を行う。

---

### Requirement 5: JointCalendar 複数カレンダー結合

**Objective:** As a トレーダー, I want 複数のカレンダーを結合して営業日判定したい, so that クロスボーダー取引の決済日を正確に計算できる。

#### Acceptance Criteria

1. The calendars module shall `JointCalendarRule` enum を定義し、`JoinHolidays` と `JoinBusinessDays` を含む。
2. The calendars module shall `JointCalendar` struct を定義し、`Vec<Box<dyn Calendar>>` と `JointCalendarRule` を保持する。
3. The JointCalendar shall `Calendar` trait を実装する。
4. When `JoinHolidays` ルールが指定された場合, the JointCalendar shall すべてのカレンダーが営業日の場合のみ営業日と判定する（休日の和集合）。
5. When `JoinBusinessDays` ルールが指定された場合, the JointCalendar shall いずれかのカレンダーが営業日であれば営業日と判定する（営業日の和集合）。
6. The JointCalendar shall `adjust`, `next_business_day`, `prev_business_day`, `add_business_days` を結合ルールに従って実装する。

---

### Requirement 6: BusinessDayConvention 営業日調整

**Objective:** As a クォンツアナリスト, I want 営業日調整規約に従って日付を調整したい, so that 金融契約の決済日を正確に計算できる。

#### Acceptance Criteria

1. The calendars module shall `BusinessDayConvention` enum を定義し、以下の 5 種を含む: `Unadjusted`, `Following`, `ModifiedFollowing`, `Preceding`, `ModifiedPreceding`。
2. When `Following` が指定された場合, the Calendar shall 非営業日を次の営業日に調整する。
3. When `ModifiedFollowing` が指定された場合, the Calendar shall 次の営業日が月をまたぐ場合は前の営業日に調整する。
4. When `Preceding` が指定された場合, the Calendar shall 非営業日を前の営業日に調整する。
5. When `ModifiedPreceding` が指定された場合, the Calendar shall 前の営業日が月をまたぐ場合は次の営業日に調整する。
6. When `Unadjusted` が指定された場合, the Calendar shall 日付を調整せずそのまま返却する。

---

### Requirement 7: DayCounter 日数計算規約

**Objective:** As a クォンツアナリスト, I want ISDA 標準の日数計算規約で年率換算したい, so that 金利計算が市場慣行に適合する。

#### Acceptance Criteria

1. The day_counters module shall `DayCounter` enum を定義し、以下の 7 種を含む: `Actual360`, `Actual365Fixed`, `Actual36525`, `ActualActualIsda`, `Thirty360Bond`, `Thirty360European`, `ThirtyE360Isda`。
2. The DayCounter type shall `year_fraction(&self, d1: Date, d2: Date) -> Result<f64, TimeError>` メソッドを提供する。
3. The DayCounter type shall `day_count(&self, d1: Date, d2: Date) -> i64` メソッドを提供する。
4. When `Actual360` が指定された場合, the DayCounter shall 実日数 / 360 を返却する。
5. When `Actual365Fixed` が指定された場合, the DayCounter shall 実日数 / 365 を返却する。
6. When `Thirty360Bond` が指定された場合, the DayCounter shall US Bond Basis の 30/360 計算を適用する。
7. When `Thirty360European` が指定された場合, the DayCounter shall European 30/360 計算を適用する。
8. The DayCounter type shall static dispatch（enum + match）で実装し、VTable コストを回避する。

---

### Requirement 8: TimeUnit と汎用 Period 構造

**Objective:** As a ライブラリ開発者, I want 汎用的な期間表現を使用したい, so that 任意の日数・週数・月数・年数を表現できる。

#### Acceptance Criteria

1. The period module shall `TimeUnit` enum を定義し、`Days`, `Weeks`, `Months`, `Years` を含む。
2. The period module shall `Period` struct を定義し、`length: i32` と `units: TimeUnit` を保持する。
3. The Period type shall `new(length: i32, units: TimeUnit) -> Self` コンストラクタを提供する。
4. The Period type shall 便利メソッド `days(n)`, `weeks(n)`, `months(n)`, `years(n)` を提供する。
5. The period module shall `Date + Period -> Date` の `Add` trait 実装を提供する。
6. When `Months` が加算された場合, the Date type shall 月末調整（End-of-Month ルール）を正しく処理する。
7. When 1月31日に 1ヶ月を加算した場合, the Date type shall 2月28日（または29日）を返却する。

---

### Requirement 9: Tenor 標準金融期間

**Objective:** As a トレーダー, I want 標準的な金融期間（ON, 1W, 3M, 1Y 等）を使用したい, so that 市場で一般的なテナー表記を利用できる。

#### Acceptance Criteria

1. The period module shall `Tenor` enum を定義し、以下の 17 種を含む: `Overnight`, `OneWeek`, `TwoWeeks`, `OneMonth`, `TwoMonths`, `ThreeMonths`, `SixMonths`, `NineMonths`, `OneYear`, `TwoYears`, `ThreeYears`, `FiveYears`, `SevenYears`, `TenYears`, `FifteenYears`, `TwentyYears`, `ThirtyYears`。
2. The Tenor type shall `code(&self) -> &'static str` メソッドを提供し、標準コード（"ON", "1W", "3M" 等）を返却する。
3. The Tenor type shall `to_months(&self) -> u32` メソッドを提供し、月数を返却する。
4. The Tenor type shall `to_period(&self) -> Period` メソッドを提供し、汎用 Period に変換する。
5. The Tenor type shall `FromStr` を実装し、文字列（"3M", "1Y" 等）からパースできる。
6. The Tenor type shall `add_to_date(&self, date: Date, eom_rule: EndOfMonthRule) -> Date` メソッドを提供する。

---

### Requirement 10: EndOfMonthRule 月末調整規則

**Objective:** As a クォンツアナリスト, I want 月末調整規則を指定したい, so that 金融契約の日付計算が市場慣行に適合する。

#### Acceptance Criteria

1. The period module shall `EndOfMonthRule` enum を定義し、`Adjust`, `Preserve`, `None` を含む。
2. When `Adjust` が指定された場合, the Date type shall 元日付が月末の場合、結果日付も月末に調整する。
3. When `Preserve` が指定された場合, the Date type shall 元日付の日を維持し、無効な場合は月末にフォールバックする。
4. When `None` が指定された場合, the Date type shall 単純な月加算を行い、無効な場合は月末にフォールバックする。
5. The EndOfMonthRule type shall `Default` を derive し、デフォルトを `Adjust` とする。

---

### Requirement 11: AccrualPeriod 計算期間

**Objective:** As a クォンツアナリスト, I want 計算期間（開始日、終了日、支払日）を表現したい, so that 固定収益商品の利息計算に使用できる。

#### Acceptance Criteria

1. The period module shall `AccrualPeriod` struct を定義し、`start: Date`, `end: Date`, `payment: Date` を保持する。
2. The AccrualPeriod type shall `new(start, end, payment) -> Self` コンストラクタを提供する。
3. The AccrualPeriod type shall `accrual_days(&self) -> i64` メソッドを提供し、計算期間の日数を返却する。
4. The AccrualPeriod type shall `year_fraction(&self, day_count: DayCounter) -> f64` メソッドを提供する。
5. The AccrualPeriod type shall `Copy`, `Clone`, `Debug`, `PartialEq` を derive する。

---

### Requirement 12: 後方互換性と非推奨警告

**Objective:** As a 既存ライブラリ利用者, I want 既存の API を引き続き使用したい, so that コードの移行を段階的に行える。

#### Acceptance Criteria

1. The infra_domain crate shall `lib.rs` にて既存の型（`Date`, `DateError`, `DayCountConvention`, `BusinessDayConvention`, `Calendar`, `CalendarId`, `Tenor`, `Period`, `Frequency`）を re-export する。
2. When 旧名称（`DateError`, `DayCountConvention`）が使用された場合, the infra_domain crate shall `#[deprecated]` 警告を表示する。
3. The infra_domain crate shall 新旧両方の型名でコンパイルが成功する。
4. The deprecated aliases shall 新しい型へのエイリアスとして実装する。
5. The infra_domain crate shall すべての既存の doctests と integration tests がパスする。

---

### Requirement 13: テスト要件

**Objective:** As a ライブラリ開発者, I want 全機能の単体テストを実装したい, so that 品質と正確性を保証できる。

#### Acceptance Criteria

1. The time module shall 各サブモジュール（error, types, calendars, day_counters, period）に単体テストを含む。
2. The Date tests shall Excel serial 変換の往復テストを含む。
3. The Date tests shall Excel leap year bug（1900-02-29）の正しい処理を検証する。
4. The Calendar tests shall `ModifiedFollowing` が月をまたがないことを検証する。
5. The JointCalendar tests shall `JoinHolidays` と `JoinBusinessDays` の結合ロジックを検証する。
6. The DayCounter tests shall QuantLib の既知のテストケースと数値を照合する。
7. The DayCounter tests shall 30/360 の US vs European の違いを検証する。
8. The Period tests shall 月末調整（1月31日 + 1ヶ月 = 2月28/29日）を検証する。

---

## Project Description (Reference)

以下は仕様初期化時に提供されたプロジェクト説明の参照情報である。

### Implementation Strategy

既存の `crates/infra_domain/src/` には以下の時間関連モジュールが実装済み：

| 既存ファイル | 内容 | 状態 |
|-------------|------|------|
| `date.rs` | `Date` wrapper (NaiveDate) | ✅ 完全実装 |
| `error.rs` | `DateError` | ✅ 完全実装 |
| `business_day.rs` | `BusinessDayConvention` (5種) | ✅ 完全実装 |
| `calendar.rs` | `Calendar` struct (4カレンダー) | ⚠️ struct、trait ではない |
| `day_count.rs` | `DayCountConvention` (7種) | ✅ 完全実装 |
| `tenor.rs` | `Tenor` enum (17種) + `EndOfMonthRule` | ✅ 完全実装 |
| `period.rs` | `Period` (accrual period) | ✅ 完全実装 |

### Migration Plan

既存ファイルを `src/time/` サブモジュールに再編成し、仕様書の構造に合わせる：

```text
crates/infra_domain/src/
├── lib.rs                    # 既存 (time モジュールを追加)
├── time/
│   ├── mod.rs                # 新規: Module definition & re-exports
│   ├── error.rs              # 移動: error.rs の TimeError 部分
│   ├── types.rs              # 移動: date.rs → types.rs
│   ├── calendars.rs          # 移動+拡張: calendar.rs + business_day.rs
│   ├── day_counters.rs       # 移動: day_count.rs → day_counters.rs
│   └── period.rs             # 移動+統合: tenor.rs + period.rs
├── currency.rs               # 既存維持
├── rate_index.rs             # 既存維持
├── direction.rs              # 既存維持
├── counterparty.rs           # 既存維持
└── frequency.rs              # 既存維持
```

### Dependencies

`Cargo.toml` に以下を含めること（既存で充足済み）：

* `chrono` (features = ["serde"])
* `thiserror` (for ergonomic error handling)
* `serde` (features = ["derive"])
