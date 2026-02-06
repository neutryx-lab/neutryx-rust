# Research Document: Financial Time Module

## 1. Existing Asset Analysis

### 1.1 Reusable Components (完全再利用可能)

| コンポーネント | ファイル | 状態 | 備考 |
|--------------|---------|------|------|
| `Date` | `date.rs` | ✅ 完全 | NaiveDate wrapper、Add/Sub 実装済、`to_serial`/`from_serial` 未実装 |
| `DateError` | `error.rs` | ✅ 完全 | `TimeError` にリネーム予定 |
| `BusinessDayConvention` | `business_day.rs` | ✅ 完全 | 5種、FromStr/Display 実装済 |
| `DayCountConvention` | `day_count.rs` | ✅ 完全 | 7種、static dispatch 実装済 |
| `Tenor` | `tenor.rs` | ✅ 完全 | 17種、`to_period()` 未実装 |
| `EndOfMonthRule` | `tenor.rs` | ✅ 完全 | 3種 (Adjust, Preserve, None) |
| `Period` (accrual) | `period.rs` | ✅ 完全 | start/end/payment フィールド、`AccrualPeriod` にリネーム予定 |
| `Calendar` struct | `calendar.rs` | ⚠️ 部分的 | struct、trait ではない。trait 化が必要 |
| `CalendarId` | `calendar.rs` | ✅ 完全 | 5種 (Target, NewYork, Tokyo, London, WeekendOnly) |

### 1.2 Existing Test Coverage

```text
infra_domain/src/
├── calendar.rs      → 11 tests (is_business_day, adjust, etc.)
├── tenor.rs         → 17 tests (add_to_date, EOM rules)
├── date.rs          → 15 tests (from_ymd, parse, arithmetic)
├── day_count.rs     → 12 tests (year_fraction, 30/360)
├── business_day.rs  → 9 tests (name, code, FromStr)
├── period.rs        → 5 tests (accrual_days, year_fraction)
├── error.rs         → 7 tests (Display, Clone, PartialEq)
────────────────────────────
時間関連: 76 tests
```

### 1.3 Dependency Analysis

**infra_domain を参照するクレート:**

```text
pricer_core::types::mod.rs (lines 47-62)
├── #[deprecated] pub use infra_domain::BusinessDayConvention;
├── #[deprecated] pub use infra_domain::Currency;
├── #[deprecated] pub use infra_domain::Date;
└── #[deprecated] pub use infra_domain::DayCountConvention;

pricer_core::trades::schedules::period.rs
└── 別の Period 型が存在 (day_count フィールド付き)

pricer_models::lib.rs (lines 56-59)
├── pub use infra_domain::{SwapDirection, TradeDirection};
└── direction_ext traits
```

**影響範囲:**
- `lib.rs` の re-exports 変更は上記クレートに影響
- 後方互換性のため deprecated alias が必須

---

## 2. Technical Research

### 2.1 Excel Serial Date Specification

**基準日:** 1900-01-01 = 1

**Lotus 1-2-3 互換バグ:**
- Excel は 1900 年を誤って閏年として扱う
- 1900-02-29 が存在するとして計算される（実際には存在しない）
- 1900-03-01 以降の日付は +1 日のオフセットが必要

**実装パターン:**
```rust
// to_serial: Date -> i64
pub fn to_serial(&self) -> i64 {
    // Excel epoch: 1900-01-01 = 1
    let epoch = NaiveDate::from_ymd_opt(1899, 12, 31).unwrap();
    let days = (self.0 - epoch).num_days();

    // Excel leap year bug: 1900-02-28 以降は +1
    if days > 59 { days + 1 } else { days }
}

// from_serial: i64 -> Date
pub fn from_serial(serial: i64) -> Result<Self, TimeError> {
    if serial < 1 {
        return Err(TimeError::CalculationError("Serial must be >= 1".into()));
    }

    let epoch = NaiveDate::from_ymd_opt(1899, 12, 31).unwrap();

    // Excel leap year bug: serial 60 = 1900-02-29 (invalid)
    let adjusted = if serial > 59 { serial - 1 } else { serial };

    epoch.checked_add_days(chrono::Days::new(adjusted as u64))
        .map(Date::from_naive)
        .ok_or_else(|| TimeError::CalculationError("Invalid serial".into()))
}
```

**テストケース:**
| Date | Excel Serial | 備考 |
|------|-------------|------|
| 1900-01-01 | 1 | Epoch |
| 1900-02-28 | 59 | Bug boundary |
| 1900-03-01 | 61 | After bug (skips 60) |
| 2024-01-01 | 45292 | Modern date |

### 2.2 Calendar Trait Design

**既存 Calendar struct のメソッド:**
```rust
impl Calendar {
    pub fn get(id: CalendarId) -> Self;
    pub fn is_business_day(&self, date: NaiveDate) -> bool;
    pub fn is_holiday(&self, date: NaiveDate) -> bool;
    pub fn next_business_day(&self, date: NaiveDate) -> NaiveDate;
    pub fn prev_business_day(&self, date: NaiveDate) -> NaiveDate;
    pub fn add_business_days(&self, date: NaiveDate, days: i32) -> NaiveDate;
    // Date-based wrappers
    pub fn is_business_day_date(&self, date: Date) -> bool;
    pub fn next_business_day_date(&self, date: Date) -> Date;
    pub fn prev_business_day_date(&self, date: Date) -> Date;
    pub fn add_business_days_date(&self, date: Date, days: i32) -> Date;
    pub fn adjust(&self, date: Date, convention: BusinessDayConvention) -> Date;
}
```

**Trait 設計:**
```rust
pub trait Calendar: Send + Sync {
    /// Required: Check if a date is a business day
    fn is_business_day(&self, date: Date) -> bool;

    // Default implementations based on is_business_day
    fn is_holiday(&self, date: Date) -> bool {
        !self.is_business_day(date)
    }

    fn next_business_day(&self, date: Date) -> Date {
        let mut current = date;
        while !self.is_business_day(current) {
            current = current + 1;
        }
        current
    }

    fn prev_business_day(&self, date: Date) -> Date { /* ... */ }
    fn add_business_days(&self, date: Date, days: i32) -> Date { /* ... */ }
    fn adjust(&self, date: Date, convention: BusinessDayConvention) -> Date { /* ... */ }
}
```

### 2.3 JointCalendar Design

**ユースケース:** USD/JPY 取引では NY と Tokyo 両方が営業日である必要がある

**結合ルール:**
```rust
pub enum JointCalendarRule {
    /// 全カレンダーが営業日の場合のみ営業日（休日の和集合）
    JoinHolidays,
    /// いずれかのカレンダーが営業日なら営業日（営業日の和集合）
    JoinBusinessDays,
}

pub struct JointCalendar {
    calendars: Vec<Box<dyn Calendar>>,
    rule: JointCalendarRule,
}

impl Calendar for JointCalendar {
    fn is_business_day(&self, date: Date) -> bool {
        match self.rule {
            JointCalendarRule::JoinHolidays => {
                self.calendars.iter().all(|c| c.is_business_day(date))
            }
            JointCalendarRule::JoinBusinessDays => {
                self.calendars.iter().any(|c| c.is_business_day(date))
            }
        }
    }
}
```

### 2.4 TimeUnit and Generic Period

**設計:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeUnit {
    Days,
    Weeks,
    Months,
    Years,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    pub length: i32,
    pub units: TimeUnit,
}

impl Period {
    pub fn new(length: i32, units: TimeUnit) -> Self { /* ... */ }
    pub fn days(n: i32) -> Self { Self::new(n, TimeUnit::Days) }
    pub fn weeks(n: i32) -> Self { Self::new(n, TimeUnit::Weeks) }
    pub fn months(n: i32) -> Self { Self::new(n, TimeUnit::Months) }
    pub fn years(n: i32) -> Self { Self::new(n, TimeUnit::Years) }
}

impl Add<Period> for Date {
    type Output = Date;
    fn add(self, period: Period) -> Date { /* ... */ }
}
```

### 2.5 Period Naming Conflict Resolution

**問題:** `Period` が 2 箇所に存在
- `infra_domain::Period` (accrual period) → `AccrualPeriod` にリネーム
- `pricer_core::trades::schedules::Period` (day_count フィールド付き)

**解決策:**
1. `infra_domain::Period` → `infra_domain::time::AccrualPeriod`
2. 新規 `infra_domain::time::Period` = 汎用期間 (length + TimeUnit)
3. `infra_domain::lib.rs` で deprecated alias:
   ```rust
   #[deprecated(since = "0.3.0", note = "Use AccrualPeriod instead")]
   pub type Period = time::AccrualPeriod;
   ```

---

## 3. Integration Points

### 3.1 pricer_core Integration

**現状 (pricer_core::types::mod.rs):**
```rust
#[deprecated(since = "0.2.0", note = "Use infra_domain::Date directly")]
pub use infra_domain::Date;
```

**移行後:**
```rust
// infra_domain::lib.rs での re-export を維持
// pricer_core での deprecated 警告はそのまま
```

### 3.2 pricer_core::trades::schedules::Period との関係

**pricer_core::trades::schedules::Period:**
- `start`, `end`, `payment` + `day_count` フィールド
- `year_fraction()` メソッドが `day_count` を内部保持

**infra_domain::time::AccrualPeriod:**
- `start`, `end`, `payment` のみ
- `year_fraction(day_count)` で外部から渡す

→ 異なる責務のため共存可能。名前衝突は `AccrualPeriod` リネームで解決。

---

## 4. Key Design Decisions

### 4.1 Static vs Dynamic Dispatch

| コンポーネント | Dispatch | 理由 |
|--------------|----------|------|
| `DayCounter` | Static (enum + match) | Hot path、Enzyme AD 最適化 |
| `Calendar` trait | Dynamic (Box<dyn>) | JointCalendar の柔軟性、呼び出し頻度低 |
| `BusinessDayConvention` | Static (enum) | シンプル、パフォーマンス |
| `TimeUnit` | Static (enum) | シンプル |

### 4.2 Error Handling Strategy

**TimeError 統合:**
```rust
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TimeError {
    #[error("Invalid date: {year}-{month:02}-{day:02}")]
    InvalidDate { year: i32, month: u32, day: u32 },

    #[error("Date parse error: {0}")]
    ParseError(String),

    #[error("Calculation error: {0}")]
    CalculationError(String),

    #[error("Calendar error: {0}")]
    CalendarError(String),
}

// Backward compatibility
pub type DateError = TimeError;
```

### 4.3 Serde Feature Flag

すべての新規型は既存パターンに従う:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Period { /* ... */ }
```

---

## 5. Risks and Mitigations

| リスク | 影響 | 軽減策 |
|-------|------|--------|
| 後方互換性の破壊 | Medium | deprecated re-exports、段階的移行 |
| Period 名前衝突 | Low | AccrualPeriod リネームで解決 |
| Calendar trait 化 | Medium | 既存 API は wrapper 関数で維持 |
| Excel serial バグ | Low | 既知のパターン、テストケースで検証 |
| JointCalendar パフォーマンス | Low | Box<dyn> オーバーヘッドは許容範囲 |

---

## 6. Open Questions (Resolved)

1. **TimeError vs DateError**: `TimeError` に統一、`DateError` は deprecated alias
2. **Calendar trait の object safety**: `Send + Sync` 要求で解決
3. **Period の名前衝突**: `AccrualPeriod` リネームで解決
4. **Excel serial の 1900-02-29**: +1 オフセット補正で対応
