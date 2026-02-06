# Requirements Document

## Project Description (Input)
以下は、Tradeモジュールを実装するための詳細仕様書です。

---

# Specification: Trade & Instrument Module Implementation in Rust

## 1. Overview & Architectural Design

本モジュールは、金融取引の構造（Structure）を定義する。価格評価（Pricing）のアルゴリズムは含まず、あくまで「キャッシュフローがいつ、どのような条件で発生するか」の記述に特化する。

### Core Hierarchy

```text
Trade (共通フォーマット - Pricingへの入力)
├── legs: Vec<Leg>
├── id: TradeId
└── metadata: TradeMetadata

Instrument (マーケット用規格化商品)
├── 市場データ構築用の特殊Trade
├── Par Swap, Deposit, FRA, Futures など
└── Trade に変換可能 (impl Into<Trade>)
```

**設計思想:**
- **Trade**: 全ての金融取引を表現する共通フォーマット。CF展開済みの `Leg` → `Cashflow` 構造として Pricer に渡される。
- **Instrument**: マーケットデータ構築（カーブキャリブレーション等）用に規格化された特殊なTrade。最終的には `Trade` に変換されて評価される。

### Design Principles

* **Flattened Hierarchy:** C++の深い継承ツリーを廃止し、`Trade` を共通フォーマットとして集約する。
* **Composition over Inheritance:** `Trade` は `Leg` の集合であり、`Leg` は `Cashflow` の集合であるという包含関係で表現する。
* **Static Dispatch via Enums:** `Payoff` や `Index` などの多態性が必要な箇所は、Trait Object (`Box<dyn Payoff>`) ではなく Enum を使用し、分岐予測とインライン化を有利にする。
* **Separation of Data & Logic:** データ（条件定義）と構築ロジック（Builder）を明確に分離する。
* **Unified Pricing Interface:** 全ての商品は最終的に `Trade` → `Vec<Leg>` → `Vec<Cashflow>` として展開され、共通インターフェースで Pricer に渡される。

## 2. Directory Structure (Minimalist Approach)

`crates/infra_domain/src/` 配下に `trade/` サブモジュールを追加する。

既存の `infra_domain` アセット（`Date`, `Period`, `RateIndex`, `Direction` など）を再利用し、新規定義は `trade/` に集約する。

```text
crates/infra_domain/src/
├── lib.rs                   # Module definition & re-exports (既存)
├── date.rs                  # Date type (既存)
├── period.rs                # Period type (既存)
├── tenor.rs                 # Tenor type (既存)
├── frequency.rs             # Frequency type (既存)
├── day_count.rs             # DayCountConvention (既存)
├── rate_index.rs            # RateIndex: SOFR, EURIBOR, etc. (既存)
├── direction.rs             # TradeDirection, SwapDirection (既存)
├── calendar.rs              # Calendar, BusinessDay (既存)
├── currency.rs              # Currency (既存)
├── error.rs                 # MasterDataError (既存)
│
├── convention/              # NEW: Market conventions (静的マスターデータ)
│   ├── mod.rs               # Module definition & re-exports
│   ├── swap.rs              # SwapConvention, SwapLegConvention, OisConvention
│   ├── fra.rs               # FraConvention
│   ├── futures.rs           # FuturesConvention (IR, Bond)
│   ├── capfloor.rs          # CapFloorConvention
│   ├── fx.rs                # FxConvention, FxSwapConvention
│   ├── bond.rs              # BondConvention
│   ├── cds.rs               # CdsConvention, IsdaCdsConvention
│   └── presets.rs           # USD_SOFR, EUR_EURIBOR_6M, JPY_TONAR など定義済み規約
│
└── trade/                   # NEW: Trade module
    ├── mod.rs               # Module definition & re-exports
    ├── error.rs             # Trade-specific errors
    ├── index.rs             # IndexObservation (RateIndexを拡張)
    ├── payoff.rs            # Payoff logic (Fixed, Floating, Cap/Floor, Digital)
    ├── cashflow.rs          # Nodes (Coupons, Principals, Events)
    ├── leg.rs               # Leg definition (Vector of cashflows)
    ├── trade.rs             # Trade: 共通フォーマット（Pricingへの入力）
    ├── instrument.rs        # Instrument: マーケット用規格化商品
    └── builder.rs           # Schedule & Leg builders (Convention + Instrument → Trade)
```

### 既存型の再利用

| 既存型 | 用途 |
|--------|------|
| `Date` | 支払日、Accrual期間 |
| `Period` | Tenor、観測ラグ |
| `RateIndex` | Index定義のベース |
| `TradeDirection` | Long/Short |
| `SwapDirection` | PayFixed/ReceiveFixed |
| `DayCountConvention` | Year Fraction計算 |
| `Currency` | 通貨コード |
| `Calendar` | 営業日調整 |

## 3. Implementation Details

### 3.1 `trade/index.rs`

市場参照指標の観測条件を定義する。既存の `RateIndex` を拡張し、非金利系（FX, Equity, Inflation, Commodity）もカバーする。

**Requirements:**

1. **Enum `IndexType`**: 既存の `RateIndex` をラップしつつ、非金利系指標も含む。
2. **Struct `IndexObservation`**: 観測に必要なメタデータ（Fixing Date, 観測ラグなど）を保持する。

**Interface Draft:**

```rust
use crate::{Currency, Period, RateIndex};

/// 全ての市場参照指標を表現
/// 既存の RateIndex を拡張し、非金利系もカバー
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IndexType {
    /// 金利指標（既存の RateIndex を再利用）
    Rate(RateIndex),
    /// CMS Swap Rate
    SwapRate { currency: Currency, tenor: Period },
    /// FX Spot Rate
    Fx { base: Currency, quote: Currency },
    /// Equity Index
    Equity(String),             // e.g., "SPX", "NKY"
    /// Inflation Index
    Inflation(String),          // e.g., "UK-RPI", "US-CPI"
    /// Commodity
    Commodity(String),          // e.g., "WTI", "GOLD"
}

/// 特定の観測条件を表現
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IndexObservation {
    pub index_type: IndexType,
    pub observation_lag: Period, // e.g., 2BD before payment
    pub fixing_source: Option<String>, // e.g., "REUTERS", "BLOOMBERG"
}

impl From<RateIndex> for IndexType {
    fn from(rate: RateIndex) -> Self {
        IndexType::Rate(rate)
    }
}
```

### 3.2 `trade/payoff.rs`

キャッシュフローの計算式（Payoff）を定義する。

**Requirements:**

1. **Enum `Payoff`**: `Fixed`, `Linear` (Vanilla Floater), `Option` (Cap/Floor), `Digital` などを網羅する。
2. 数式計算ロジックは `impl Payoff` ブロック内に記述し、`match` 式で分岐する。

**Interface Draft:**

```rust
use super::index::IndexObservation;

/// Call/Put フラグ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OptionType {
    Call,
    Put,
}

/// キャッシュフロー計算式
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Payoff {
    /// Fixed rate coupon (independent of market)
    Fixed { rate: f64 },

    /// Linear payoff: gearing * (index + spread)
    Linear {
        gearing: f64,
        spread: f64,
        index: IndexObservation,
    },

    /// Vanilla option payoff: max(option_type * (index - strike), 0)
    VanillaOption {
        gearing: f64,
        strike: f64,
        option_type: OptionType,
        index: IndexObservation,
    },

    /// Digital option payoff
    Digital {
        payout: f64,
        strike: f64,
        option_type: OptionType,
        index: IndexObservation,
    },
}

impl Payoff {
    /// Returns the required index observation (if any).
    pub fn required_index(&self) -> Option<&IndexObservation> {
        match self {
            Payoff::Fixed { .. } => None,
            Payoff::Linear { index, .. }
            | Payoff::VanillaOption { index, .. }
            | Payoff::Digital { index, .. } => Some(index),
        }
    }

    /// Returns true if this payoff is market-independent (fixed).
    pub fn is_fixed(&self) -> bool {
        matches!(self, Payoff::Fixed { .. })
    }
}
```

### 3.3 `trade/cashflow.rs`

キャッシュフローの最小単位（Node）を定義する。

**Requirements:**

1. **Enum `CashflowType`**: `Coupon`, `Principal`, `Fee`, `Settlement`。
2. **Struct `Cashflow`**: 支払日、期間（Accrual Period）、想定元本（Notional）、および `Payoff` を保持する。

**Interface Draft:**

```rust
use crate::{Currency, Date};
use super::payoff::Payoff;

/// キャッシュフローの種別
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CashflowType {
    Coupon,
    Principal,
    Fee,
    Settlement,
}

/// キャッシュフローの最小単位（Node）
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cashflow {
    pub cf_type: CashflowType,
    pub payment_date: Date,
    pub accrual_start: Date,
    pub accrual_end: Date,
    pub year_fraction: f64, // Pre-calculated day count fraction
    pub notional: f64,
    pub payoff: Payoff,
    pub currency: Currency,
}

impl Cashflow {
    /// Checks if the cashflow is known (fixed) at the given reference date.
    pub fn is_fixed(&self, ref_date: Date) -> bool {
        // Fixed payoff is always known
        if self.payoff.is_fixed() {
            return true;
        }
        // For floating, check if fixing date has passed
        // (実装時に IndexObservation の fixing_date を確認)
        false
    }

    /// Returns true if payment date is after the reference date.
    pub fn is_future(&self, ref_date: Date) -> bool {
        self.payment_date > ref_date
    }
}
```

### 3.4 `trade/leg.rs`

一連のキャッシュフロー列（Leg）を定義する。

**Requirements:**

1. **Struct `Leg`**: `Vec<Cashflow>` のラッパー。
2. **Enum `LegType`**: `FixedLeg`, `FloatingLeg`, `CapFloorLeg` などのタグ付け（Pricerの識別用）。
3. イテレータの実装や、特定の期間内のCF抽出などのユーティリティを提供する。
4. 既存の `SwapDirection` を `Direction` として再利用（PayFixed → Payer）。

**Interface Draft:**

```rust
use crate::{Currency, Date};
use super::cashflow::Cashflow;

/// Leg の方向（Payer/Receiver）
/// 既存の SwapDirection と互換性を持たせる
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Direction {
    /// Pay this leg (sign = -1)
    Payer,
    /// Receive this leg (sign = +1)
    Receiver,
}

impl Direction {
    /// Returns the sign multiplier for NPV calculation.
    pub fn sign(&self) -> f64 {
        match self {
            Direction::Payer => -1.0,
            Direction::Receiver => 1.0,
        }
    }
}

/// Leg の種別（Pricerの識別用）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LegType {
    Fixed,
    Floating,
    CapFloor,
    Principal,
    Generic,
}

/// 一連のキャッシュフロー列
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Leg {
    pub cashflows: Vec<Cashflow>,
    pub direction: Direction,
    pub leg_type: LegType,
    pub currency: Currency,
}

impl Leg {
    pub fn new(cashflows: Vec<Cashflow>, direction: Direction, leg_type: LegType, currency: Currency) -> Self {
        Self { cashflows, direction, leg_type, currency }
    }

    /// Returns iterator over future cashflows.
    pub fn future_cashflows(&self, ref_date: Date) -> impl Iterator<Item = &Cashflow> {
        self.cashflows.iter().filter(move |cf| cf.is_future(ref_date))
    }

    /// Returns the total notional (assumes constant notional).
    pub fn notional(&self) -> f64 {
        self.cashflows.first().map(|cf| cf.notional).unwrap_or(0.0)
    }
}
```

### 3.5 `trade/trade.rs` (共通フォーマット)

全ての金融取引を表現する共通フォーマット。**Pricingへの統一入力インターフェース**。

**Requirements:**

1. **Struct `Trade`**: 全ての取引タイプを共通フォーマットで保持する。
2. 全ての商品は最終的に `Trade` に変換され、`Vec<Leg>` → `Vec<Cashflow>` として Pricer に渡される。
3. `TradeType` Enum でオプショナルなメタデータ（行使条件等）を保持する。

**Interface Draft:**

```rust
use crate::Date;
use super::leg::Leg;
use super::cashflow::Cashflow;

/// Trade ID (type alias for clarity)
pub type TradeId = String;

/// 全ての金融取引を表現する共通フォーマット
/// Pricingへの統一入力インターフェース
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Trade {
    pub id: TradeId,
    pub legs: Vec<Leg>,
    pub trade_type: TradeType,
    pub metadata: TradeMetadata,
}

/// 取引タイプ固有の追加情報
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TradeType {
    /// 通常のSwap（追加情報なし）
    Swap,

    /// Swaption: 行使条件を保持
    Swaption {
        exercise_dates: Vec<Date>,
        exercise_type: ExerciseType,
        settlement_type: SettlementType,
    },

    /// Bond: 発行体情報を保持
    Bond {
        issuer_id: String,
        seniority: String,
    },

    /// Cap/Floor
    CapFloor,

    /// FX Forward / Spot
    FxForward,

    /// その他
    Generic,
}

/// 取引メタデータ
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeMetadata {
    pub trade_date: Option<Date>,
    pub counterparty: Option<String>,
    pub portfolio: Option<String>,
    pub book: Option<String>,
}

/// 行使タイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExerciseType {
    European,
    Bermudan,
    American,
}

/// 決済タイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SettlementType {
    Cash,
    Physical,
}

impl Trade {
    /// 全てのCashflowをフラットに取得
    pub fn all_cashflows(&self) -> impl Iterator<Item = &Cashflow> {
        self.legs.iter().flat_map(|leg| leg.cashflows.iter())
    }

    /// 指定日以降のCashflowをフィルタ
    pub fn future_cashflows(&self, ref_date: Date) -> impl Iterator<Item = &Cashflow> {
        self.all_cashflows().filter(move |cf| cf.is_future(ref_date))
    }

    /// Returns the number of legs.
    pub fn num_legs(&self) -> usize {
        self.legs.len()
    }

    /// Returns true if this is a vanilla swap (2 legs, no optionality).
    pub fn is_vanilla_swap(&self) -> bool {
        matches!(self.trade_type, TradeType::Swap) && self.legs.len() == 2
    }
}
```

### 3.6 `trade/instrument.rs` (マーケット用規格化商品)

マーケットデータ構築（カーブキャリブレーション等）用に規格化された特殊なTrade。

**Requirements:**

1. **Enum `Instrument`**: Par Swap, Deposit, FRA, Futures などマーケット規格商品を表現。
2. **impl Into<Trade>**: 全ての `Instrument` は `Trade` に変換可能。
3. キャリブレーション用のヘルパーメソッドを提供。

**Interface Draft:**

```rust
use crate::{Currency, Date, Period, RateIndex};
use super::trade::Trade;
use super::index::IndexType;

/// マーケット用規格化商品
/// カーブキャリブレーション等で使用される標準的な商品定義
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Instrument {
    /// Money Market Deposit
    Deposit {
        currency: Currency,
        start_date: Date,
        tenor: Period,
        rate: f64,
    },

    /// Forward Rate Agreement
    Fra {
        currency: Currency,
        start_date: Date,
        tenor: Period,
        rate: f64,
    },

    /// Interest Rate Futures (e.g., Eurodollar, SOFR futures)
    Futures {
        currency: Currency,
        expiry: Date,
        price: f64,
    },

    /// Par Swap (標準金利スワップ)
    ParSwap {
        currency: Currency,
        start_date: Date,
        tenor: Period,
        fixed_rate: f64,
        float_index: RateIndex,
        fixed_frequency: Period,
        float_frequency: Period,
    },

    /// Overnight Index Swap
    Ois {
        currency: Currency,
        start_date: Date,
        tenor: Period,
        fixed_rate: f64,
        ois_index: RateIndex,
    },

    /// Basis Swap
    BasisSwap {
        currency: Currency,
        start_date: Date,
        tenor: Period,
        spread: f64,
        index1: RateIndex,
        index2: RateIndex,
    },

    /// Cross Currency Swap
    CrossCurrencySwap {
        pay_currency: Currency,
        rcv_currency: Currency,
        start_date: Date,
        tenor: Period,
        spread: f64,
    },
}

impl Instrument {
    /// Instrumentを評価用のTradeに変換
    pub fn to_trade(&self) -> Trade {
        // BuilderロジックでCF展開してTradeを構築
        todo!()
    }

    /// キャリブレーション用: Par Rate / Quote を返す
    pub fn quote(&self) -> f64 {
        match self {
            Instrument::Deposit { rate, .. } => *rate,
            Instrument::Fra { rate, .. } => *rate,
            Instrument::Futures { price, .. } => 100.0 - price,
            Instrument::ParSwap { fixed_rate, .. } => *fixed_rate,
            Instrument::Ois { fixed_rate, .. } => *fixed_rate,
            Instrument::BasisSwap { spread, .. } => *spread,
            Instrument::CrossCurrencySwap { spread, .. } => *spread,
        }
    }

    /// Returns the maturity date of the instrument.
    pub fn maturity(&self) -> Date {
        match self {
            Instrument::Deposit { start_date, tenor, .. }
            | Instrument::Fra { start_date, tenor, .. }
            | Instrument::ParSwap { start_date, tenor, .. }
            | Instrument::Ois { start_date, tenor, .. }
            | Instrument::BasisSwap { start_date, tenor, .. }
            | Instrument::CrossCurrencySwap { start_date, tenor, .. } => {
                start_date.add_period(tenor) // 要実装
            }
            Instrument::Futures { expiry, .. } => *expiry,
        }
    }

    /// Returns the currency of the instrument.
    pub fn currency(&self) -> Currency {
        match self {
            Instrument::Deposit { currency, .. }
            | Instrument::Fra { currency, .. }
            | Instrument::Futures { currency, .. }
            | Instrument::ParSwap { currency, .. }
            | Instrument::Ois { currency, .. }
            | Instrument::BasisSwap { currency, .. } => *currency,
            Instrument::CrossCurrencySwap { pay_currency, .. } => *pay_currency,
        }
    }
}

impl From<Instrument> for Trade {
    fn from(inst: Instrument) -> Self {
        inst.to_trade()
    }
}
```

### 3.7 `trade/builder.rs`

スケジュール生成とLeg構築のロジック。
ここはデータ構造ではなく「処理ロジック」であるため、複雑な計算はここに集約する。

**Requirements:**

1. **Struct `ScheduleBuilder`**: 開始日、終了日、頻度、カレンダーを入力とし、日付のリスト(`Vec<Date>`)を生成する。
2. **Struct `LegBuilder`**: スケジュール情報とレート定義、Notional情報を受け取り、`Leg` (i.e., `Vec<Cashflow>`) を構築して返す。
3. **Struct `TradeBuilder`**: 複数のLegを組み合わせて `Trade` を構築する。

**Interface Draft:**

```rust
use crate::{
    BusinessDayConvention, Calendar, CalendarId, Currency, Date,
    DayCountConvention, Frequency, Period, RateIndex,
};
use super::{
    cashflow::{Cashflow, CashflowType},
    index::{IndexObservation, IndexType},
    leg::{Direction, Leg, LegType},
    payoff::Payoff,
    trade::{Trade, TradeId, TradeMetadata, TradeType},
};

/// スケジュール生成用ビルダー
#[derive(Debug, Clone)]
pub struct ScheduleBuilder {
    start_date: Date,
    end_date: Date,
    frequency: Frequency,
    calendar: CalendarId,
    business_day_convention: BusinessDayConvention,
    end_of_month: bool,
}

impl ScheduleBuilder {
    pub fn new(start_date: Date, end_date: Date, frequency: Frequency) -> Self {
        Self {
            start_date,
            end_date,
            frequency,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            end_of_month: false,
        }
    }

    pub fn calendar(mut self, calendar: CalendarId) -> Self {
        self.calendar = calendar;
        self
    }

    pub fn business_day_convention(mut self, bdc: BusinessDayConvention) -> Self {
        self.business_day_convention = bdc;
        self
    }

    /// Generates the schedule dates.
    pub fn build(&self) -> Vec<Date> {
        // 日付生成ロジック（周期に基づいてstart→endを分割）
        todo!()
    }
}

/// Leg構築用ビルダー
#[derive(Debug, Clone)]
pub struct LegBuilder {
    schedule: Vec<Date>,
    notional: f64,
    currency: Currency,
    direction: Direction,
    day_count: DayCountConvention,
}

impl LegBuilder {
    pub fn new(schedule: Vec<Date>, notional: f64, currency: Currency) -> Self {
        Self {
            schedule,
            notional,
            currency,
            direction: Direction::Receiver,
            day_count: DayCountConvention::Actual365Fixed,
        }
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn day_count(mut self, day_count: DayCountConvention) -> Self {
        self.day_count = day_count;
        self
    }

    /// Builds a fixed leg.
    pub fn build_fixed(self, rate: f64) -> Leg {
        let cashflows = self.build_cashflows(Payoff::Fixed { rate });
        Leg::new(cashflows, self.direction, LegType::Fixed, self.currency)
    }

    /// Builds a floating leg.
    pub fn build_floating(self, index: RateIndex, spread: f64) -> Leg {
        let index_obs = IndexObservation {
            index_type: IndexType::Rate(index),
            observation_lag: Period::days(2),
            fixing_source: None,
        };
        let payoff = Payoff::Linear {
            gearing: 1.0,
            spread,
            index: index_obs,
        };
        let cashflows = self.build_cashflows(payoff);
        Leg::new(cashflows, self.direction, LegType::Floating, self.currency)
    }

    fn build_cashflows(&self, payoff: Payoff) -> Vec<Cashflow> {
        // スケジュールからCashflowを生成
        todo!()
    }
}

/// Trade構築用ビルダー
#[derive(Debug, Clone)]
pub struct TradeBuilder {
    id: TradeId,
    legs: Vec<Leg>,
    trade_type: TradeType,
    metadata: TradeMetadata,
}

impl TradeBuilder {
    pub fn new(id: impl Into<TradeId>) -> Self {
        Self {
            id: id.into(),
            legs: Vec::new(),
            trade_type: TradeType::Generic,
            metadata: TradeMetadata::default(),
        }
    }

    pub fn add_leg(mut self, leg: Leg) -> Self {
        self.legs.push(leg);
        self
    }

    pub fn trade_type(mut self, trade_type: TradeType) -> Self {
        self.trade_type = trade_type;
        self
    }

    pub fn metadata(mut self, metadata: TradeMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn build(self) -> Trade {
        Trade {
            id: self.id,
            legs: self.legs,
            trade_type: self.trade_type,
            metadata: self.metadata,
        }
    }
}
```

---

## 4. Specific Instructions for "Fixing"

C++の `Fixing/` フォルダは、過去の確定レート（Historical Fixings）を扱う部分です。Rustの実装では、これは Trade 構造体の一部ではなく、Pricing Engine に渡される **「環境（Environment/Context）」** として扱われるべきです。

したがって、`Trade` モジュール内にはデータ保持用のコンテナ定義のみを置き、取得ロジックは切り離します。

**Action:** `src/trade/fixing.rs` (Optional) または `src/market/fixing.rs` (別モジュール) として以下を定義する：

```rust
use std::collections::HashMap;
use crate::time::types::Date;
use crate::trade::index::IndexType;

/// Container for historical market data.
pub trait FixingProvider {
    /// Returns the fixing value if available.
    fn get_fixing(&self, index: &IndexType, date: Date) -> Result<f64, FixingError>;
}

// Simple in-memory implementation for testing/light use
pub struct InMemoryFixingStore {
    data: HashMap<(IndexType, Date), f64>,
}
```

---

## 5. Data Flow Summary

```text
┌─────────────────────────────────────────────────────────────────┐
│                        INPUT SOURCES                            │
├─────────────────────────────────────────────────────────────────┤
│  FpML/XML        JSON Config       Market Data (Quotes)         │
│      │               │                    │                     │
│      ▼               ▼                    ▼                     │
│  ┌────────┐    ┌──────────┐        ┌─────────────┐              │
│  │Adapter │    │ Builder  │        │ Instrument  │              │
│  └────┬───┘    └────┬─────┘        │(Par Swap等) │              │
│       │             │              └──────┬──────┘              │
│       │             │                     │                     │
│       │             │    ┌────────────────┘                     │
│       │             │    │                                      │
│       │             │    │  ┌─────────────┐                     │
│       │             │    │  │ Convention  │ (静的マスターデータ) │
│       │             │    │  │ SwapConv,   │                     │
│       │             │    │  │ FxConv, ... │                     │
│       │             │    │  └──────┬──────┘                     │
│       │             │    │         │                            │
│       │             ▼    ▼         ▼                            │
│       │        ┌─────────────────────────┐                      │
│       │        │    TradeBuilder         │                      │
│       │        │  from_instrument(       │                      │
│       │        │    instrument,          │                      │
│       │        │    convention           │                      │
│       │        │  )                      │                      │
│       │        └───────────┬─────────────┘                      │
│       │                    │                                    │
│       ▼                    ▼                                    │
│  ┌──────────────────────────────────────────────┐               │
│  │                   Trade                       │               │
│  │  ┌─────────────────────────────────────────┐ │               │
│  │  │ legs: Vec<Leg>                          │ │               │
│  │  │   ┌───────────────────────────────────┐ │ │               │
│  │  │   │ cashflows: Vec<Cashflow>          │ │ │               │
│  │  │   │   ┌─────────────────────────────┐ │ │ │               │
│  │  │   │   │ payoff: Payoff              │ │ │ │               │
│  │  │   │   │ payment_date, notional, ... │ │ │ │               │
│  │  │   │   └─────────────────────────────┘ │ │ │               │
│  │  │   └───────────────────────────────────┘ │ │               │
│  │  └─────────────────────────────────────────┘ │               │
│  └──────────────────────────────────────────────┘               │
│                          │                                      │
│                          ▼                                      │
│  ┌──────────────────────────────────────────────┐               │
│  │              Pricing Engine                   │               │
│  │  (Market Environment + Trade → Price/Greeks) │               │
│  └──────────────────────────────────────────────┘               │
└─────────────────────────────────────────────────────────────────┘
```

## Requirements

### Requirement 1: IndexType と IndexObservation

**Objective:** 開発者として、金利・FX・株式・インフレ・コモディティを含む全ての市場参照指標を統一的に表現したい。これにより、Payoff定義時に一貫したインターフェースで指標を参照できる。

#### Acceptance Criteria

1. The `IndexType` enum shall include a `Rate(RateIndex)` variant that wraps the existing `infra_domain::RateIndex`.
2. The `IndexType` enum shall include variants for `SwapRate`, `Fx`, `Equity`, `Inflation`, and `Commodity` index types.
3. When an `IndexType::Rate` variant is created, the `infra_domain` module shall provide an `impl From<RateIndex> for IndexType` conversion.
4. The `IndexObservation` struct shall contain `index_type: IndexType`, `observation_lag: Period`, and optional `fixing_source: Option<String>`.
5. The `IndexType` enum shall derive `Debug`, `Clone`, `PartialEq`, `Eq`, and `Hash` traits.
6. Where the `serde` feature is enabled, the `infra_domain` module shall derive `Serialize` and `Deserialize` for `IndexType` and `IndexObservation`.

---

### Requirement 2: Payoff 定義

**Objective:** 開発者として、Fixed/Linear/VanillaOption/Digital の4種類のペイオフを表現したい。これにより、キャッシュフローの計算式を型安全に定義できる。

#### Acceptance Criteria

1. The `Payoff` enum shall include `Fixed { rate: f64 }` variant for market-independent fixed coupons.
2. The `Payoff` enum shall include `Linear { gearing, spread, index }` variant for vanilla floating coupons.
3. The `Payoff` enum shall include `VanillaOption { gearing, strike, option_type, index }` variant for cap/floor payoffs.
4. The `Payoff` enum shall include `Digital { payout, strike, option_type, index }` variant for digital option payoffs.
5. When `required_index()` is called on a `Payoff::Fixed` variant, the method shall return `None`.
6. When `required_index()` is called on a non-Fixed `Payoff` variant, the method shall return `Some(&IndexObservation)`.
7. When `is_fixed()` is called, the method shall return `true` only for `Payoff::Fixed` variants.
8. The `OptionType` enum shall include `Call` and `Put` variants.

---

### Requirement 3: Cashflow 定義

**Objective:** 開発者として、支払日・計算期間・想定元本・ペイオフを含むキャッシュフローの最小単位を表現したい。これにより、Leg構築時に個別のキャッシュフローを型安全に生成できる。

#### Acceptance Criteria

1. The `Cashflow` struct shall contain `cf_type: CashflowType`, `payment_date: Date`, `accrual_start: Date`, `accrual_end: Date`, `year_fraction: f64`, `notional: f64`, `payoff: Payoff`, and `currency: Currency`.
2. The `CashflowType` enum shall include `Coupon`, `Principal`, `Fee`, and `Settlement` variants.
3. When `is_fixed(ref_date)` is called on a `Cashflow` with `Payoff::Fixed`, the method shall return `true`.
4. When `is_future(ref_date)` is called, the method shall return `true` if `payment_date > ref_date`.
5. The `Cashflow` struct shall derive `Debug` and `Clone` traits.
6. Where the `serde` feature is enabled, the `infra_domain` module shall derive `Serialize` and `Deserialize` for `Cashflow` and `CashflowType`.

---

### Requirement 4: Leg 定義

**Objective:** 開発者として、キャッシュフロー列と方向・種別を含むLegを表現したい。これにより、Trade構築時に固定Leg・変動Legなどを型安全に組み合わせられる。

#### Acceptance Criteria

1. The `Leg` struct shall contain `cashflows: Vec<Cashflow>`, `direction: Direction`, `leg_type: LegType`, and `currency: Currency`.
2. The `Direction` enum shall include `Payer` and `Receiver` variants.
3. When `sign()` is called on `Direction::Payer`, the method shall return `-1.0`.
4. When `sign()` is called on `Direction::Receiver`, the method shall return `1.0`.
5. The `LegType` enum shall include `Fixed`, `Floating`, `CapFloor`, `Principal`, and `Generic` variants.
6. When `future_cashflows(ref_date)` is called, the method shall return an iterator over cashflows where `payment_date > ref_date`.
7. When `notional()` is called, the method shall return the notional of the first cashflow, or `0.0` if empty.

---

### Requirement 5: Trade 定義（共通フォーマット）

**Objective:** 開発者として、全ての金融取引をCF展開済みの共通フォーマットで表現したい。これにより、Pricerは単一のインターフェースで全商品を評価できる。

#### Acceptance Criteria

1. The `Trade` struct shall contain `id: TradeId`, `legs: Vec<Leg>`, `trade_type: TradeType`, and `metadata: TradeMetadata`.
2. The `TradeType` enum shall include `Swap`, `Swaption`, `Bond`, `CapFloor`, `FxForward`, and `Generic` variants.
3. When `TradeType::Swaption` is used, the variant shall contain `exercise_dates: Vec<Date>`, `exercise_type: ExerciseType`, and `settlement_type: SettlementType`.
4. When `TradeType::Bond` is used, the variant shall contain `issuer_id: String` and `seniority: String`.
5. The `ExerciseType` enum shall include `European`, `Bermudan`, and `American` variants.
6. The `SettlementType` enum shall include `Cash` and `Physical` variants.
7. When `all_cashflows()` is called, the method shall return a flattened iterator over all cashflows in all legs.
8. When `future_cashflows(ref_date)` is called, the method shall return an iterator over cashflows where `payment_date > ref_date`.
9. When `is_vanilla_swap()` is called, the method shall return `true` if `trade_type` is `Swap` and `legs.len() == 2`.
10. The `TradeMetadata` struct shall contain `trade_date: Option<Date>`, `counterparty: Option<String>`, `portfolio: Option<String>`, and `book: Option<String>`.

---

### Requirement 6: Instrument 定義（マーケット用規格化商品）

**Objective:** 開発者として、カーブキャリブレーション用の規格化商品（Deposit, FRA, Futures, Par Swap, OIS, Basis Swap, XCCY）を表現したい。これにより、マーケットデータ構築時に標準的な商品定義を使用できる。

#### Acceptance Criteria

1. The `Instrument` enum shall include `Deposit`, `Fra`, `Futures`, `ParSwap`, `Ois`, `BasisSwap`, and `CrossCurrencySwap` variants.
2. When `Instrument::Deposit` is used, the variant shall contain `currency: Currency`, `start_date: Date`, `tenor: Period`, and `rate: f64`.
3. When `Instrument::ParSwap` is used, the variant shall contain `currency`, `start_date`, `tenor`, `fixed_rate`, `float_index: RateIndex`, `fixed_frequency: Period`, and `float_frequency: Period`.
4. When `to_trade()` is called on an `Instrument`, the method shall return a fully CF-expanded `Trade` structure.
5. When `quote()` is called on an `Instrument`, the method shall return the market quote (rate, spread, or `100 - price` for Futures).
6. When `maturity()` is called on an `Instrument`, the method shall return the maturity date calculated from `start_date + tenor` (or `expiry` for Futures).
7. When `currency()` is called on an `Instrument`, the method shall return the primary currency of the instrument.
8. The `Instrument` enum shall implement `impl From<Instrument> for Trade`.

---

### Requirement 7: Builder パターン

**Objective:** 開発者として、Schedule/Leg/Trade を段階的に構築するBuilderパターンを使用したい。これにより、複雑な取引構造を型安全かつ読みやすく構築できる。

#### Acceptance Criteria

1. The `ScheduleBuilder` struct shall accept `start_date`, `end_date`, and `frequency` as required parameters.
2. When `ScheduleBuilder::calendar()` is called, the builder shall allow setting a `CalendarId`.
3. When `ScheduleBuilder::business_day_convention()` is called, the builder shall allow setting a `BusinessDayConvention`.
4. When `ScheduleBuilder::build()` is called, the method shall return a `Vec<Date>` representing the schedule dates.
5. The `LegBuilder` struct shall accept `schedule`, `notional`, and `currency` as required parameters.
6. When `LegBuilder::build_fixed(rate)` is called, the method shall return a `Leg` with `LegType::Fixed` and `Payoff::Fixed` cashflows.
7. When `LegBuilder::build_floating(index, spread)` is called, the method shall return a `Leg` with `LegType::Floating` and `Payoff::Linear` cashflows.
8. The `TradeBuilder` struct shall accept `id` as a required parameter.
9. When `TradeBuilder::add_leg(leg)` is called, the builder shall append the leg to the internal `Vec<Leg>`.
10. When `TradeBuilder::build()` is called, the method shall return a fully constructed `Trade`.

---

### Requirement 8: エラーハンドリング

**Objective:** 開発者として、Trade構築時のバリデーションエラーを構造化された型で受け取りたい。これにより、エラー原因を明確に特定できる。

#### Acceptance Criteria

1. The `trade/error.rs` module shall define a `TradeError` enum.
2. The `TradeError` enum shall include `InvalidSchedule`, `EmptyLeg`, `InvalidNotional`, `MismatchedCurrency`, and `InvalidPayoff` variants.
3. If an empty schedule is provided to `LegBuilder`, the builder shall return `Err(TradeError::InvalidSchedule)`.
4. If a negative notional is provided, the builder shall return `Err(TradeError::InvalidNotional)`.
5. The `TradeError` type shall implement `std::error::Error` and `std::fmt::Display` traits.
6. The `TradeError` type shall derive `Debug` trait.

---

### Requirement 9: 既存型との統合

**Objective:** 開発者として、`infra_domain` の既存型（Date, Period, Currency, RateIndex, DayCountConvention, Calendar）を再利用したい。これにより、型の重複を避け、一貫性を保てる。

#### Acceptance Criteria

1. The `trade` module shall use `crate::Date` for all date fields.
2. The `trade` module shall use `crate::Period` for tenor and observation lag fields.
3. The `trade` module shall use `crate::Currency` for all currency fields.
4. The `trade` module shall use `crate::RateIndex` within `IndexType::Rate` variant.
5. The `trade` module shall use `crate::DayCountConvention` for year fraction calculations in `LegBuilder`.
6. The `trade` module shall use `crate::CalendarId` and `crate::BusinessDayConvention` in `ScheduleBuilder`.

---

### Requirement 10: Serde シリアライゼーション

**Objective:** 開発者として、Trade/Leg/Cashflow/Instrument を JSON/YAML でシリアライズ・デシリアライズしたい。これにより、外部システムとのデータ交換が可能になる。

#### Acceptance Criteria

1. Where the `serde` feature is enabled, all trade module types shall derive `Serialize` and `Deserialize`.
2. Where the `serde` feature is not enabled, the trade module types shall not have serde dependencies.
3. The serde derivation shall use `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]` pattern.
4. When a `Trade` is serialized to JSON, the output shall be valid JSON that can be deserialized back to an equivalent `Trade`.

---

### Requirement 11: Convention 定義（市場規約）

**Objective:** 開発者として、通貨・商品タイプごとの市場規約（日数計算、支払頻度、営業日調整など）を静的マスターデータとして定義したい。これにより、Instrument と Convention を組み合わせて正確な Trade を構築できる。

#### Acceptance Criteria

1. The `convention/` module shall be located under `crates/infra_domain/src/convention/`.
2. The `SwapLegConvention` struct shall contain `day_count: DayCountConvention`, `payment_frequency: Frequency`, `calendar: CalendarId`, `business_day_convention: BusinessDayConvention`, and `payment_lag: i32`.
3. The `SwapConvention` struct shall contain `fixed_leg: SwapLegConvention`, `float_leg: SwapLegConvention`, `float_index: RateIndex`, and `spot_lag: i32`.
4. The `FraConvention` struct shall contain `day_count: DayCountConvention`, `calendar: CalendarId`, `business_day_convention: BusinessDayConvention`, and `index: RateIndex`.
5. The `FuturesConvention` struct shall contain `contract_size: f64`, `tick_size: f64`, `day_count: DayCountConvention`, and `calendar: CalendarId`.
6. The `CapFloorConvention` struct shall contain `day_count: DayCountConvention`, `payment_frequency: Frequency`, `calendar: CalendarId`, `business_day_convention: BusinessDayConvention`, and `index: RateIndex`.
7. The `FxConvention` struct shall contain `spot_days: i32`, `calendar: CalendarId`, and `business_day_convention: BusinessDayConvention`.
8. The `BondConvention` struct shall contain `day_count: DayCountConvention`, `coupon_frequency: Frequency`, `calendar: CalendarId`, `business_day_convention: BusinessDayConvention`, and `settlement_days: i32`.
9. The `CdsConvention` struct shall contain `day_count: DayCountConvention`, `payment_frequency: Frequency`, `calendar: CalendarId`, `business_day_convention: BusinessDayConvention`, and `recovery_rate: f64`.
10. Where the `serde` feature is enabled, all convention types shall derive `Serialize` and `Deserialize`.

---

### Requirement 12: Convention プリセット

**Objective:** 開発者として、主要通貨の標準的な市場規約をプリセットとして使用したい。これにより、毎回規約を手動定義する必要がなくなる。

#### Acceptance Criteria

1. The `convention/presets.rs` module shall provide `SwapConvention::usd_sofr()` for USD SOFR swap conventions.
2. The `convention/presets.rs` module shall provide `SwapConvention::eur_euribor_6m()` for EUR EURIBOR 6M swap conventions.
3. The `convention/presets.rs` module shall provide `SwapConvention::jpy_tonar()` for JPY TONAR swap conventions.
4. The `convention/presets.rs` module shall provide `SwapConvention::gbp_sonia()` for GBP SONIA swap conventions.
5. The `convention/presets.rs` module shall provide `FxConvention::usd_jpy()` for USD/JPY FX conventions.
6. The `convention/presets.rs` module shall provide `FxConvention::eur_usd()` for EUR/USD FX conventions.
7. When a preset convention is retrieved, the method shall return a static reference or owned value with correct market-standard parameters.
8. The preset conventions shall use appropriate `CalendarId` values (e.g., `CalendarId::NewYork` for USD, `CalendarId::Target` for EUR).

---

### Requirement 13: Convention と Instrument の統合

**Objective:** 開発者として、Convention と Instrument を組み合わせて完全な Trade を構築したい。これにより、規約情報を手動で指定せずに正確なCF展開ができる。

#### Acceptance Criteria

1. The `TradeBuilder` struct shall provide a `from_instrument(instrument: &Instrument, convention: &impl Convention)` method.
2. When `from_instrument()` is called with a `ParSwap` instrument and `SwapConvention`, the builder shall generate fixed and floating legs with correct day counts, frequencies, and calendars.
3. When `from_instrument()` is called with a `Deposit` instrument and `SwapLegConvention`, the builder shall generate a single-cashflow leg.
4. When `from_instrument()` is called with a `Fra` instrument and `FraConvention`, the builder shall generate appropriate FRA cashflows.
5. If an incompatible instrument and convention combination is provided, the builder shall return `Err(TradeError::IncompatibleConvention)`.
6. The `Convention` trait shall define a `validate(&self) -> Result<(), ConventionError>` method for runtime validation.
7. The `TradeError` enum shall include an `IncompatibleConvention` variant for convention mismatch errors.
