# Research: Trade & Instrument Module

## 概要

Trade & Instrument Module 設計のための調査結果。既存の `infra_domain` パターンを分析し、新しい `trade/` および `convention/` モジュールの設計指針を策定した。

## 調査日時

2026-01-20

## 調査範囲

- `crates/infra_domain/src/` 全体のコードパターン
- 既存型定義: Date, Period, Currency, DayCount, Calendar, RateIndex, Frequency, Direction
- エラーハンドリング、ビルダーパターン、Serde 使用パターン

---

## 発見事項

### 1. モジュール構造パターン

```rust
// lib.rs パターン
mod business_day;      // プライベートモジュール宣言
pub use business_day::BusinessDayConvention;  // 選択的エクスポート

pub mod prelude {
    pub use crate::{BusinessDayConvention, Calendar, ...};
}
```

**要点:**
- プライベート `mod` 宣言 + `pub use` で選択的エクスポート
- `prelude` モジュールで便利インポートを提供
- 内部実装とパブリック API の明確な分離

### 2. 型定義パターン

#### A. Newtype ラッパー

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Date(NaiveDate);
```

- `transparent` でクリーンなシリアライズ
- 包括的なトレイト実装

#### B. Enum パターン（静的ディスパッチ）

```rust
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Currency {
    USD, EUR, GBP, JPY, CHF,
}
```

- `#[non_exhaustive]` で将来の拡張を許容
- Copy/Clone でゼロコスト抽象化

#### C. Builder パターン

```rust
impl CsaTerms {
    pub fn new(csa_id: impl Into<String>) -> Self { ... }
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }
}
```

- `new()` コンストラクタ + `with_*` メソッド
- `mut self` で状態変更、`self` を返却
- チェイン可能な設定

### 3. エラーハンドリングパターン

```rust
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DateError {
    #[error("Invalid date: {year}-{month:02}-{day:02}")]
    InvalidDate { year: i32, month: u32, day: u32 },

    #[error("Date parse error: {0}")]
    ParseError(String),
}
```

- `thiserror` クレートで人間工学的なエラー処理
- 名前付きフィールドで構造化エラー
- Clone + PartialEq でテスト容易性

### 4. Serde 使用パターン（フィーチャーゲート）

```rust
// 一般パターン
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

// Newtype の場合
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Date(NaiveDate);
```

**Cargo.toml パターン:**
```toml
[dependencies]
serde = { workspace = true, optional = true }

[features]
default = []
serde = ["dep:serde"]
```

### 5. トレイト実装パターン

#### FromStr パターン
```rust
impl FromStr for Currency {
    type Err = CurrencyError;

    fn from_str(s: &str) -> Result<Self, CurrencyError> {
        match s.to_uppercase().as_str() {
            "USD" => Ok(Currency::USD),
            _ => Err(CurrencyError::UnknownCurrency(s.to_string())),
        }
    }
}
```

#### From/Into 変換
```rust
impl From<SwapDirection> for TradeDirection {
    fn from(swap: SwapDirection) -> Self {
        match swap {
            SwapDirection::PayFixed => TradeDirection::Short,
            SwapDirection::ReceiveFixed => TradeDirection::Long,
        }
    }
}
```

### 6. const fn パターン

```rust
impl RateIndex {
    #[must_use]
    pub const fn currency(&self) -> Currency {
        match self {
            Self::Sofr => Currency::USD,
            Self::Tonar => Currency::JPY,
        }
    }
}
```

- コンパイル時評価可能
- `#[must_use]` で結果の無視を警告

### 7. テストパターン

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() { }

    #[test]
    fn test_error_cases() { }

    #[test]
    fn test_roundtrip() { }  // parse -> Display -> parse
}
```

**テストカテゴリ:**
1. コンストラクタ/ファクトリテスト
2. Parse/FromStr テスト
3. Display/fmt テスト
4. トレイト実装テスト
5. エラーケーステスト
6. ラウンドトリップテスト

---

## 再利用すべき既存型

| 型 | ファイル | 用途 |
|---|---|---|
| `Date` | date.rs | 全日付フィールド |
| `Period` | period.rs | 期間表現（2Y, 6M） |
| `Tenor` | tenor.rs | 標準テナー |
| `Currency` | currency.rs | 通貨コード |
| `DayCountConvention` | day_count.rs | 日数計算規約 |
| `BusinessDayConvention` | business_day.rs | 営業日調整 |
| `Calendar` / `CalendarId` | calendar.rs | カレンダー |
| `RateIndex` | rate_index.rs | 金利指標 |
| `Frequency` | frequency.rs | 支払頻度 |
| `SwapDirection` / `TradeDirection` | direction.rs | 方向性 |
| `EndOfMonthRule` | period.rs | EOM ルール |

---

## 設計への影響

### 新モジュール構造

```text
crates/infra_domain/src/
├── lib.rs              # mod trade; mod convention; pub use ...
├── trade/
│   ├── mod.rs          # サブモジュール宣言 + 再エクスポート
│   ├── error.rs        # TradeError
│   ├── index.rs        # IndexType, IndexObservation
│   ├── payoff.rs       # Payoff, OptionType
│   ├── cashflow.rs     # Cashflow, CashflowType
│   ├── leg.rs          # Leg, Direction, LegType
│   ├── trade.rs        # Trade, TradeType, TradeMetadata
│   ├── instrument.rs   # Instrument
│   └── builder.rs      # ScheduleBuilder, LegBuilder, TradeBuilder
└── convention/
    ├── mod.rs          # サブモジュール宣言 + 再エクスポート
    ├── swap.rs         # SwapLegConvention, SwapConvention
    ├── fra.rs          # FraConvention
    ├── futures.rs      # FuturesConvention
    ├── capfloor.rs     # CapFloorConvention
    ├── fx.rs           # FxConvention
    ├── bond.rs         # BondConvention
    ├── cds.rs          # CdsConvention
    └── presets.rs      # プリセット関数
```

### 採用パターン

1. **Enum ベースの静的ディスパッチ** - IndexType, CashflowType, LegType, TradeType, Instrument
2. **Builder パターン** - ScheduleBuilder, LegBuilder, TradeBuilder, InstrumentBuilder
3. **フィーチャーゲート Serde** - 全型に適用
4. **thiserror エラー** - TradeError
5. **From/Into 変換** - Instrument → Trade, Convention + Instrument → Trade
6. **const fn ゲッター** - 可能な箇所で適用

---

## 参照ファイル

- `crates/infra_domain/src/lib.rs`
- `crates/infra_domain/src/error.rs`
- `crates/infra_domain/src/date.rs`
- `crates/infra_domain/src/currency.rs`
- `crates/infra_domain/src/period.rs`
- `crates/infra_domain/src/day_count.rs`
- `crates/infra_domain/src/calendar.rs`
- `crates/infra_domain/src/rate_index.rs`
- `crates/infra_domain/src/business_day.rs`
- `crates/infra_domain/src/counterparty.rs`
- `crates/infra_domain/src/frequency.rs`
- `crates/infra_domain/src/direction.rs`
