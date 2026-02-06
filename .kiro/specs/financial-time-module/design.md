# Technical Design Document: Financial Time Module

## 1. Overview

### 1.1 Summary

本設計は、`crates/infra_domain/src/` 内の時間関連モジュールを `time/` サブディレクトリに再編成し、金融デリバティブプライシングエンジンの基盤となる時間管理機能を完成させる。既存の 80% 以上の実装を活用しつつ、不足機能（Excel serial 変換、Calendar trait 化、JointCalendar、汎用 Period + TimeUnit）を追加する。

### 1.2 Goals

1. **モジュール再編成**: 時間関連コードを `time/` サブディレクトリに集約し、構造を明確化する
2. **Calendar trait 化**: 既存の struct を trait に昇格し、JointCalendar で複数カレンダーを結合可能にする
3. **Excel 互換性**: `to_serial()`/`from_serial()` メソッドでスプレッドシート連携を実現する
4. **汎用 Period**: `TimeUnit` + `Period` で任意の期間を表現可能にする
5. **後方互換性**: deprecated re-exports で既存 API を維持する

### 1.3 Design Approach

**Hybrid Approach (Option C)** を採用し、3 フェーズで段階的に実装する:

| Phase | 内容 | リスク |
|-------|------|--------|
| Phase 1 | 構造移行（ファイル移動 + mod.rs + re-exports） | Low |
| Phase 2 | 機能拡張（Excel serial, TimeUnit, Period, AccrualPeriod） | Medium |
| Phase 3 | Trait 化（Calendar trait, ConcreteCalendar, JointCalendar） | Medium |

---

## 2. Architecture

### 2.1 Target Directory Structure

```text
crates/infra_domain/src/
├── lib.rs                    # time モジュール追加 + deprecated re-exports
├── time/
│   ├── mod.rs                # Module definition & re-exports
│   ├── error.rs              # TimeError enum
│   ├── types.rs              # Date wrapper (with Excel serial)
│   ├── calendars.rs          # Calendar trait, ConcreteCalendar, JointCalendar
│   ├── day_counters.rs       # DayCounter enum (renamed from DayCountConvention)
│   └── period.rs             # TimeUnit, Period, Tenor, EndOfMonthRule, AccrualPeriod
├── currency.rs               # 既存維持
├── rate_index.rs             # 既存維持
├── direction.rs              # 既存維持
├── counterparty.rs           # 既存維持
└── frequency.rs              # 既存維持
```

### 2.2 Module Dependency Graph

```text
time/mod.rs
├── error.rs          ← (no deps)
├── types.rs          ← error.rs
├── calendars.rs      ← error.rs, types.rs
├── day_counters.rs   ← types.rs
└── period.rs         ← error.rs, types.rs, day_counters.rs
```

### 2.3 External Dependencies

| Crate | Purpose | Existing |
|-------|---------|----------|
| `chrono` | Date/time handling | ✅ |
| `thiserror` | Error handling | ✅ |
| `serde` (optional) | Serialisation | ✅ |

---

## 3. Component Design

### 3.1 TimeError (error.rs)

**Purpose:** 時間関連の全エラーを統一する型

```rust
//! Time-related error types.

use thiserror::Error;

/// Unified error type for time-related operations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TimeError {
    /// Invalid date components (e.g., February 30th).
    #[error("Invalid date: {year}-{month:02}-{day:02}")]
    InvalidDate {
        year: i32,
        month: u32,
        day: u32,
    },

    /// Failed to parse date string.
    #[error("Date parse error: {0}")]
    ParseError(String),

    /// Calculation error (e.g., invalid serial number).
    #[error("Calculation error: {0}")]
    CalculationError(String),

    /// Calendar-related error.
    #[error("Calendar error: {0}")]
    CalendarError(String),
}

/// Backward compatibility alias.
#[deprecated(since = "0.3.0", note = "Use TimeError instead")]
pub type DateError = TimeError;
```

**Acceptance Criteria Coverage:** Req 2 (全項目)

---

### 3.2 Date (types.rs)

**Purpose:** NaiveDate wrapper with Excel serial conversion

**既存コード:** `date.rs` を移動し、以下のメソッドを追加

```rust
//! Date types for financial calculations.

use std::{fmt, ops::{Add, Sub}, str::FromStr};
use chrono::{Datelike, Days, Local, NaiveDate};
use crate::time::error::TimeError;

/// Type-safe date wrapper around chrono::NaiveDate.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Date(NaiveDate);

impl Date {
    // ... existing methods (from_ymd, today, parse, year, month, day, etc.) ...

    /// Convert to Excel serial date (1900-01-01 = 1).
    ///
    /// Accounts for Excel's leap year bug where 1900 is incorrectly
    /// treated as a leap year.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::Date;
    ///
    /// let date = Date::from_ymd(2024, 1, 1).unwrap();
    /// assert_eq!(date.to_serial(), 45292);
    /// ```
    #[must_use]
    pub fn to_serial(&self) -> i64 {
        // Excel epoch: 1899-12-31 (day 0), so 1900-01-01 = 1
        let epoch = NaiveDate::from_ymd_opt(1899, 12, 31).unwrap();
        let days = (self.0 - epoch).num_days();

        // Excel leap year bug: dates after 1900-02-28 are +1
        // Serial 59 = 1900-02-28, Serial 60 = 1900-02-29 (invalid)
        if days > 59 { days + 1 } else { days }
    }

    /// Create a Date from Excel serial number.
    ///
    /// # Arguments
    /// * `serial` - Excel serial date (1 = 1900-01-01)
    ///
    /// # Errors
    /// Returns `TimeError::CalculationError` if serial is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::time::Date;
    ///
    /// let date = Date::from_serial(45292).unwrap();
    /// assert_eq!(date, Date::from_ymd(2024, 1, 1).unwrap());
    /// ```
    pub fn from_serial(serial: i64) -> Result<Self, TimeError> {
        if serial < 1 {
            return Err(TimeError::CalculationError(
                format!("Serial number must be >= 1, got {}", serial)
            ));
        }

        // Excel leap year bug: serial 60 = 1900-02-29 (invalid)
        if serial == 60 {
            return Err(TimeError::CalculationError(
                "Serial 60 represents invalid date 1900-02-29".into()
            ));
        }

        let epoch = NaiveDate::from_ymd_opt(1899, 12, 31).unwrap();

        // Adjust for Excel leap year bug
        let adjusted = if serial > 60 { serial - 1 } else { serial };

        epoch.checked_add_days(Days::new(adjusted as u64))
            .map(Date::from_naive)
            .ok_or_else(|| TimeError::CalculationError(
                format!("Serial {} out of valid date range", serial)
            ))
    }
}

// ... existing trait implementations (Sub, Add, FromStr, Display, From) ...
```

**Acceptance Criteria Coverage:** Req 3 (全項目)

---

### 3.3 Calendar Trait (calendars.rs)

**Purpose:** カレンダーの抽象化と JointCalendar 結合機能

```rust
//! Holiday calendar definitions and abstractions.

use crate::time::{Date, TimeError};

/// Business day adjustment convention.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BusinessDayConvention {
    Following,
    ModifiedFollowing,
    Preceding,
    ModifiedPreceding,
    Unadjusted,
}

// ... existing BusinessDayConvention impl (name, code, FromStr, Display) ...

/// Calendar trait for business day calculations.
///
/// Implementors must provide `is_business_day`. Other methods have
/// default implementations based on this.
pub trait Calendar: Send + Sync {
    /// Check if a date is a business day.
    fn is_business_day(&self, date: Date) -> bool;

    /// Check if a date is a holiday (non-business day).
    fn is_holiday(&self, date: Date) -> bool {
        !self.is_business_day(date)
    }

    /// Get the next business day on or after the given date.
    fn next_business_day(&self, mut date: Date) -> Date {
        while !self.is_business_day(date) {
            date = date + 1;
        }
        date
    }

    /// Get the previous business day on or before the given date.
    fn prev_business_day(&self, mut date: Date) -> Date {
        while !self.is_business_day(date) {
            date = date + (-1);
        }
        date
    }

    /// Add business days to a date.
    fn add_business_days(&self, mut date: Date, days: i32) -> Date {
        let step = if days >= 0 { 1i64 } else { -1i64 };
        let mut remaining = days.abs();

        while remaining > 0 {
            date = date + step;
            if self.is_business_day(date) {
                remaining -= 1;
            }
        }
        date
    }

    /// Adjust a date according to a business day convention.
    fn adjust(&self, date: Date, convention: BusinessDayConvention) -> Date {
        match convention {
            BusinessDayConvention::Unadjusted => date,
            BusinessDayConvention::Following => self.next_business_day(date),
            BusinessDayConvention::Preceding => self.prev_business_day(date),
            BusinessDayConvention::ModifiedFollowing => {
                let adjusted = self.next_business_day(date);
                if adjusted.month() != date.month() {
                    self.prev_business_day(date)
                } else {
                    adjusted
                }
            }
            BusinessDayConvention::ModifiedPreceding => {
                let adjusted = self.prev_business_day(date);
                if adjusted.month() != date.month() {
                    self.next_business_day(date)
                } else {
                    adjusted
                }
            }
        }
    }
}

/// Calendar identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CalendarId {
    Target,
    NewYork,
    Tokyo,
    London,
    WeekendOnly,
}

/// Concrete calendar implementation.
#[derive(Debug, Clone)]
pub struct ConcreteCalendar {
    id: CalendarId,
}

impl ConcreteCalendar {
    /// Create a calendar by identifier.
    #[must_use]
    pub fn new(id: CalendarId) -> Self { Self { id } }

    /// Get a calendar by identifier (convenience method).
    #[must_use]
    pub fn get(id: CalendarId) -> Self { Self::new(id) }
}

impl Calendar for ConcreteCalendar {
    fn is_business_day(&self, date: Date) -> bool {
        use chrono::Weekday;
        let naive = date.into_inner();

        // Weekend check
        if matches!(naive.weekday(), Weekday::Sat | Weekday::Sun) {
            return false;
        }

        // Holiday check based on calendar
        !self.is_holiday_internal(naive)
    }
}

impl ConcreteCalendar {
    fn is_holiday_internal(&self, date: chrono::NaiveDate) -> bool {
        // ... existing holiday logic from calendar.rs ...
    }
}

/// Rule for combining multiple calendars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum JointCalendarRule {
    /// A date is a business day only if ALL calendars agree.
    /// (Union of holidays)
    JoinHolidays,
    /// A date is a business day if ANY calendar says so.
    /// (Union of business days)
    JoinBusinessDays,
}

/// A calendar that combines multiple calendars.
pub struct JointCalendar {
    calendars: Vec<Box<dyn Calendar>>,
    rule: JointCalendarRule,
}

impl JointCalendar {
    /// Create a new joint calendar.
    pub fn new(calendars: Vec<Box<dyn Calendar>>, rule: JointCalendarRule) -> Self {
        Self { calendars, rule }
    }
}

impl Calendar for JointCalendar {
    fn is_business_day(&self, date: Date) -> bool {
        match self.rule {
            JointCalendarRule::JoinHolidays => {
                // All calendars must agree it's a business day
                self.calendars.iter().all(|c| c.is_business_day(date))
            }
            JointCalendarRule::JoinBusinessDays => {
                // Any calendar saying it's a business day is enough
                self.calendars.iter().any(|c| c.is_business_day(date))
            }
        }
    }
}

// Backward compatibility: type alias for existing code
#[deprecated(since = "0.3.0", note = "Use ConcreteCalendar instead")]
pub type OldCalendar = ConcreteCalendar;
```

**Acceptance Criteria Coverage:** Req 4 (全項目), Req 5 (全項目), Req 6 (全項目)

---

### 3.4 DayCounter (day_counters.rs)

**Purpose:** ISDA 標準日数計算規約（リネームのみ）

```rust
//! Day count convention definitions.

use std::{fmt, str::FromStr};
use chrono::{Datelike, NaiveDate};
use crate::time::{Date, TimeError};

/// Day count convention for interest calculations.
///
/// Also known as day count fraction or accrual factor.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DayCounter {
    Actual360,
    #[default]
    Actual365Fixed,
    Actual36525,
    ActualActualIsda,
    Thirty360Bond,
    Thirty360European,
    ThirtyE360Isda,
}

impl DayCounter {
    /// Calculate the year fraction between two dates.
    ///
    /// Returns a negative value if start > end.
    #[must_use]
    pub fn year_fraction(&self, start: Date, end: Date) -> f64 {
        // ... existing implementation from day_count.rs ...
    }

    /// Calculate the number of days between two dates.
    #[must_use]
    pub fn day_count(&self, start: Date, end: Date) -> i64 {
        end - start
    }

    // ... existing methods (name, FromStr, Display) ...
}

// Backward compatibility alias
#[deprecated(since = "0.3.0", note = "Use DayCounter instead")]
pub type DayCountConvention = DayCounter;
```

**Acceptance Criteria Coverage:** Req 7 (全項目)

---

### 3.5 Period, TimeUnit, Tenor, AccrualPeriod (period.rs)

**Purpose:** 期間表現の統一と拡張

```rust
//! Period and tenor definitions.

use std::{fmt, ops::Add, str::FromStr};
use chrono::{Datelike, Months, NaiveDate};
use crate::time::{Date, DayCounter, TimeError};

/// Time unit for period calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TimeUnit {
    Days,
    Weeks,
    Months,
    Years,
}

impl fmt::Display for TimeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeUnit::Days => write!(f, "D"),
            TimeUnit::Weeks => write!(f, "W"),
            TimeUnit::Months => write!(f, "M"),
            TimeUnit::Years => write!(f, "Y"),
        }
    }
}

/// A generic time period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Period {
    /// Number of units (can be negative).
    pub length: i32,
    /// Unit of time.
    pub units: TimeUnit,
}

impl Period {
    /// Create a new period.
    #[must_use]
    pub fn new(length: i32, units: TimeUnit) -> Self {
        Self { length, units }
    }

    /// Create a period in days.
    #[must_use]
    pub fn days(n: i32) -> Self { Self::new(n, TimeUnit::Days) }

    /// Create a period in weeks.
    #[must_use]
    pub fn weeks(n: i32) -> Self { Self::new(n, TimeUnit::Weeks) }

    /// Create a period in months.
    #[must_use]
    pub fn months(n: i32) -> Self { Self::new(n, TimeUnit::Months) }

    /// Create a period in years.
    #[must_use]
    pub fn years(n: i32) -> Self { Self::new(n, TimeUnit::Years) }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.length, self.units)
    }
}

impl Add<Period> for Date {
    type Output = Date;

    fn add(self, period: Period) -> Date {
        let naive = self.into_inner();
        let result = match period.units {
            TimeUnit::Days => {
                if period.length >= 0 {
                    naive.checked_add_days(chrono::Days::new(period.length as u64))
                } else {
                    naive.checked_sub_days(chrono::Days::new((-period.length) as u64))
                }
            }
            TimeUnit::Weeks => {
                let days = period.length * 7;
                if days >= 0 {
                    naive.checked_add_days(chrono::Days::new(days as u64))
                } else {
                    naive.checked_sub_days(chrono::Days::new((-days) as u64))
                }
            }
            TimeUnit::Months => {
                if period.length >= 0 {
                    naive.checked_add_months(Months::new(period.length as u32))
                } else {
                    naive.checked_sub_months(Months::new((-period.length) as u32))
                }
            }
            TimeUnit::Years => {
                let months = period.length * 12;
                if months >= 0 {
                    naive.checked_add_months(Months::new(months as u32))
                } else {
                    naive.checked_sub_months(Months::new((-months) as u32))
                }
            }
        };
        Date::from_naive(result.unwrap_or(naive))
    }
}

/// End of month handling rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EndOfMonthRule {
    #[default]
    Adjust,
    Preserve,
    None,
}

/// Financial tenor (standard market periods).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tenor {
    Overnight,
    OneWeek,
    TwoWeeks,
    OneMonth,
    TwoMonths,
    ThreeMonths,
    SixMonths,
    NineMonths,
    OneYear,
    TwoYears,
    ThreeYears,
    FiveYears,
    SevenYears,
    TenYears,
    FifteenYears,
    TwentyYears,
    ThirtyYears,
}

impl Tenor {
    // ... existing methods (code, to_months, to_days, add_to_date) ...

    /// Convert to a generic Period.
    #[must_use]
    pub fn to_period(&self) -> Period {
        match self {
            Tenor::Overnight => Period::days(1),
            Tenor::OneWeek => Period::weeks(1),
            Tenor::TwoWeeks => Period::weeks(2),
            Tenor::OneMonth => Period::months(1),
            Tenor::TwoMonths => Period::months(2),
            Tenor::ThreeMonths => Period::months(3),
            Tenor::SixMonths => Period::months(6),
            Tenor::NineMonths => Period::months(9),
            Tenor::OneYear => Period::years(1),
            Tenor::TwoYears => Period::years(2),
            Tenor::ThreeYears => Period::years(3),
            Tenor::FiveYears => Period::years(5),
            Tenor::SevenYears => Period::years(7),
            Tenor::TenYears => Period::years(10),
            Tenor::FifteenYears => Period::years(15),
            Tenor::TwentyYears => Period::years(20),
            Tenor::ThirtyYears => Period::years(30),
        }
    }
}

// ... existing FromStr, Display implementations ...

/// A single accrual period for fixed income instruments.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccrualPeriod {
    /// Start date of the accrual period.
    pub start: Date,
    /// End date of the accrual period.
    pub end: Date,
    /// Payment date for this period.
    pub payment: Date,
}

impl AccrualPeriod {
    /// Create a new accrual period.
    #[must_use]
    pub fn new(start: Date, end: Date, payment: Date) -> Self {
        Self { start, end, payment }
    }

    /// Returns the number of days in the accrual period.
    #[must_use]
    pub fn accrual_days(&self) -> i64 {
        self.end - self.start
    }

    /// Calculate the year fraction using the specified day counter.
    #[must_use]
    pub fn year_fraction(&self, day_count: DayCounter) -> f64 {
        day_count.year_fraction(self.start, self.end)
    }
}
```

**Acceptance Criteria Coverage:** Req 8 (全項目), Req 9 (全項目), Req 10 (全項目), Req 11 (全項目)

---

### 3.6 Module Re-exports (time/mod.rs)

```rust
//! Time management module for financial calculations.
//!
//! This module provides date handling, calendar operations,
//! day count conventions, and period calculations.

mod calendars;
mod day_counters;
mod error;
mod period;
mod types;

// Primary exports
pub use calendars::{
    BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar,
    JointCalendar, JointCalendarRule,
};
pub use day_counters::DayCounter;
pub use error::TimeError;
pub use period::{AccrualPeriod, EndOfMonthRule, Period, Tenor, TimeUnit};
pub use types::Date;

// Backward compatibility aliases
#[deprecated(since = "0.3.0", note = "Use TimeError instead")]
pub use error::DateError;
#[deprecated(since = "0.3.0", note = "Use DayCounter instead")]
pub use day_counters::DayCountConvention;
```

### 3.7 lib.rs Re-exports

```rust
// ... existing module declarations ...
pub mod time;

// Re-export time types at crate root for convenience
pub use time::{
    AccrualPeriod, BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar,
    Date, DayCounter, EndOfMonthRule, JointCalendar, JointCalendarRule,
    Period, Tenor, TimeError, TimeUnit,
};

// Backward compatibility: deprecated aliases
#[deprecated(since = "0.3.0", note = "Use TimeError instead")]
pub use time::DateError;
#[deprecated(since = "0.3.0", note = "Use DayCounter instead")]
pub use time::DayCountConvention;
#[deprecated(since = "0.3.0", note = "Use ConcreteCalendar instead")]
pub type OldCalendar = ConcreteCalendar;
```

**Acceptance Criteria Coverage:** Req 1 (全項目), Req 12 (全項目)

---

## 4. Data Models

### 4.1 Core Types Summary

| Type | Category | Derives | Serde |
|------|----------|---------|-------|
| `TimeError` | Error | `Error, Debug, Clone, PartialEq, Eq` | No |
| `Date` | Value | `Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash` | Optional |
| `BusinessDayConvention` | Enum | `Debug, Clone, Copy, PartialEq, Eq, Hash` | Optional |
| `CalendarId` | Enum | `Debug, Clone, Copy, PartialEq, Eq, Hash` | Optional |
| `ConcreteCalendar` | Struct | `Debug, Clone` | No |
| `JointCalendarRule` | Enum | `Debug, Clone, Copy, PartialEq, Eq` | Optional |
| `JointCalendar` | Struct | None (has Box<dyn>) | No |
| `DayCounter` | Enum | `Debug, Clone, Copy, PartialEq, Eq, Hash, Default` | Optional |
| `TimeUnit` | Enum | `Debug, Clone, Copy, PartialEq, Eq, Hash` | Optional |
| `Period` | Struct | `Debug, Clone, Copy, PartialEq, Eq` | Optional |
| `Tenor` | Enum | `Debug, Clone, Copy, PartialEq, Eq, Hash` | Optional |
| `EndOfMonthRule` | Enum | `Debug, Clone, Copy, PartialEq, Eq, Hash, Default` | Optional |
| `AccrualPeriod` | Struct | `Debug, Clone, Copy, PartialEq` | Optional |

### 4.2 Serde Feature Flag Pattern

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Period { /* ... */ }
```

---

## 5. Error Handling

### 5.1 TimeError Variants

| Variant | Trigger | Example |
|---------|---------|---------|
| `InvalidDate` | 無効な日付コンポーネント | `Date::from_ymd(2024, 2, 30)` |
| `ParseError` | 文字列パース失敗 | `Date::parse("invalid")` |
| `CalculationError` | 計算エラー | `Date::from_serial(-1)` |
| `CalendarError` | カレンダー操作エラー | (将来の拡張用) |

### 5.2 Result Pattern

```rust
// Good: Return Result for fallible operations
pub fn from_serial(serial: i64) -> Result<Date, TimeError>

// Good: Return value directly for infallible operations
pub fn to_serial(&self) -> i64
```

---

## 6. Testing Strategy

### 6.1 Test Categories

| Category | Focus | Location |
|----------|-------|----------|
| Unit Tests | 個別関数の動作 | 各ファイル末尾 `#[cfg(test)]` |
| Edge Cases | 境界値、エラーケース | 各ファイル末尾 |
| Round-trip Tests | 変換の往復 | `types.rs` (Excel serial) |
| Integration Tests | クレート間の連携 | `tests/` directory |

### 6.2 Key Test Cases

**Excel Serial (Req 3, Req 13):**
```rust
#[test]
fn test_excel_serial_epoch() {
    let date = Date::from_ymd(1900, 1, 1).unwrap();
    assert_eq!(date.to_serial(), 1);
}

#[test]
fn test_excel_serial_leap_year_bug() {
    // 1900-02-28 = serial 59
    let feb28 = Date::from_ymd(1900, 2, 28).unwrap();
    assert_eq!(feb28.to_serial(), 59);

    // 1900-03-01 = serial 61 (skipping 60 due to bug)
    let mar1 = Date::from_ymd(1900, 3, 1).unwrap();
    assert_eq!(mar1.to_serial(), 61);
}

#[test]
fn test_excel_serial_roundtrip() {
    let original = Date::from_ymd(2024, 1, 1).unwrap();
    let serial = original.to_serial();
    let restored = Date::from_serial(serial).unwrap();
    assert_eq!(original, restored);
}
```

**JointCalendar (Req 5, Req 13):**
```rust
#[test]
fn test_joint_calendar_join_holidays() {
    let ny = Box::new(ConcreteCalendar::get(CalendarId::NewYork));
    let tokyo = Box::new(ConcreteCalendar::get(CalendarId::Tokyo));
    let joint = JointCalendar::new(vec![ny, tokyo], JointCalendarRule::JoinHolidays);

    // Only business day if BOTH calendars agree
    // ...
}
```

**30/360 Day Count (Req 7, Req 13):**
```rust
#[test]
fn test_thirty_360_bond_vs_european() {
    let start = Date::from_ymd(2024, 1, 31).unwrap();
    let end = Date::from_ymd(2024, 3, 31).unwrap();

    let bond = DayCounter::Thirty360Bond.year_fraction(start, end);
    let euro = DayCounter::Thirty360European.year_fraction(start, end);

    // Different results due to d1/d2 adjustment rules
    // ...
}
```

**Acceptance Criteria Coverage:** Req 13 (全項目)

---

## 7. Migration Strategy

### 7.1 Phase 1: Structure Migration

1. Create `time/` directory
2. Move files (rename only, no content changes):
   - `date.rs` → `time/types.rs`
   - `error.rs` (DateError part) → `time/error.rs`
   - `calendar.rs` + `business_day.rs` → `time/calendars.rs`
   - `day_count.rs` → `time/day_counters.rs`
   - `tenor.rs` + `period.rs` → `time/period.rs`
3. Create `time/mod.rs` with re-exports
4. Update `lib.rs` with deprecated re-exports
5. Run existing tests

### 7.2 Phase 2: Feature Extension

1. Add `to_serial()`/`from_serial()` to Date
2. Add `TimeUnit` enum
3. Add `Period` struct (generic)
4. Rename existing `Period` to `AccrualPeriod`
5. Add `Tenor::to_period()` method
6. Rename `DayCountConvention` to `DayCounter`
7. Rename `DateError` to `TimeError`, add variants

### 7.3 Phase 3: Calendar Trait

1. Define `Calendar` trait
2. Rename `Calendar` struct to `ConcreteCalendar`
3. Implement `Calendar` for `ConcreteCalendar`
4. Add `JointCalendarRule` enum
5. Add `JointCalendar` struct
6. Implement `Calendar` for `JointCalendar`

---

## 8. Backward Compatibility

### 8.1 Deprecated Aliases

| Old Name | New Name | Deprecation |
|----------|----------|-------------|
| `DateError` | `TimeError` | `since = "0.3.0"` |
| `DayCountConvention` | `DayCounter` | `since = "0.3.0"` |
| `Calendar` (struct) | `ConcreteCalendar` | `since = "0.3.0"` |
| `Period` (accrual) | `AccrualPeriod` | `since = "0.3.0"` |

### 8.2 Re-export Pattern

```rust
// lib.rs
#[deprecated(since = "0.3.0", note = "Use TimeError instead")]
pub use time::error::TimeError as DateError;
```

---

## 9. Appendices

### 9.1 Excel Serial Date Reference

| Date | Serial | Notes |
|------|--------|-------|
| 1900-01-01 | 1 | Epoch |
| 1900-02-28 | 59 | Last valid date before bug |
| 1900-02-29 | 60 | Invalid (bug) |
| 1900-03-01 | 61 | First date after bug |
| 2024-01-01 | 45292 | Modern reference |

### 9.2 Day Count Convention Reference

| Convention | Numerator | Denominator | Standard |
|------------|-----------|-------------|----------|
| ACT/360 | Actual days | 360 | Money market |
| ACT/365F | Actual days | 365 | Derivatives |
| ACT/365.25 | Actual days | 365.25 | Averaging |
| ACT/ACT ISDA | Actual days | Actual year days | ISDA |
| 30/360 Bond | 30/360 calc | 360 | US bonds |
| 30E/360 | 30E/360 calc | 360 | European |
| 30E/360 ISDA | 30E/360 ISDA calc | 360 | ISDA variant |

### 9.3 Requirements Traceability

| Requirement | Components | Tests |
|-------------|------------|-------|
| Req 1 | time/mod.rs, lib.rs | Structure tests |
| Req 2 | TimeError | error_tests |
| Req 3 | Date::to_serial, from_serial | serial_tests |
| Req 4 | Calendar trait | calendar_trait_tests |
| Req 5 | JointCalendar | joint_calendar_tests |
| Req 6 | BusinessDayConvention | bdc_tests |
| Req 7 | DayCounter | day_counter_tests |
| Req 8 | TimeUnit, Period | period_tests |
| Req 9 | Tenor | tenor_tests |
| Req 10 | EndOfMonthRule | eom_tests |
| Req 11 | AccrualPeriod | accrual_tests |
| Req 12 | Deprecated aliases | compat_tests |
| Req 13 | All | All test files |
