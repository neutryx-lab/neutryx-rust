# Technical Design Document

## Overview

本ドキュメントは Trade & Instrument Module の技術設計を定義する。金融取引の構造（Structure）を型安全に表現し、全ての商品を `Trade` → `Vec<Leg>` → `Vec<Cashflow>` として CF 展開し、Pricer へ統一インターフェースで渡すアーキテクチャを実現する。

### 設計原則

1. **Flattened Hierarchy**: C++ の深い継承ツリーを廃止し、`Trade` を共通フォーマットとして集約
2. **Composition over Inheritance**: 包含関係（Trade → Leg → Cashflow）で表現
3. **Static Dispatch via Enums**: Trait Object を避け、Enum で静的ディスパッチを実現
4. **Separation of Data & Logic**: データ定義と構築ロジック（Builder）を明確に分離
5. **Convention-Driven Construction**: 市場規約（Convention）と商品定義（Instrument）を組み合わせて Trade を構築

---

## Architecture

### System Context

```text
┌─────────────────────────────────────────────────────────────────┐
│                     A-I-P-S Architecture                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐     ┌─────────────────────────────────┐       │
│  │   Adapter   │────▶│         Infra Layer             │       │
│  │  (FpML等)   │     │  ┌───────────────────────────┐  │       │
│  └─────────────┘     │  │      infra_domain         │  │       │
│                      │  │  ┌─────────────────────┐  │  │       │
│                      │  │  │   convention/       │  │  │       │
│                      │  │  │   (市場規約)        │  │  │       │
│                      │  │  └─────────────────────┘  │  │       │
│                      │  │  ┌─────────────────────┐  │  │       │
│                      │  │  │   trade/            │  │  │       │
│                      │  │  │   (取引構造)        │  │  │       │
│                      │  │  └─────────────────────┘  │  │       │
│                      │  │  ┌─────────────────────┐  │  │       │
│                      │  │  │   既存型            │  │  │       │
│                      │  │  │   Date, Currency... │  │  │       │
│                      │  │  └─────────────────────┘  │  │       │
│                      │  └───────────────────────────┘  │       │
│                      └─────────────────────────────────┘       │
│                                      │                          │
│                                      ▼                          │
│                      ┌─────────────────────────────────┐       │
│                      │         Pricer Layer            │       │
│                      │   Trade → Price/Greeks          │       │
│                      └─────────────────────────────────┘       │
│                                      │                          │
│                                      ▼                          │
│                      ┌─────────────────────────────────┐       │
│                      │        Service Layer            │       │
│                      │   CLI, Gateway, Python          │       │
│                      └─────────────────────────────────┘       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Core Hierarchy

```text
Trade (共通フォーマット - Pricing への入力)
├── id: TradeId
├── legs: Vec<Leg>
│   └── Leg
│       ├── cashflows: Vec<Cashflow>
│       │   └── Cashflow
│       │       ├── payoff: Payoff
│       │       ├── payment_date: Date
│       │       ├── notional: f64
│       │       └── ...
│       ├── direction: Direction
│       ├── leg_type: LegType
│       └── currency: Currency
├── trade_type: TradeType
└── metadata: TradeMetadata

Convention (静的マスターデータ)
├── SwapConvention
├── FraConvention
├── FuturesConvention
├── CapFloorConvention
├── FxConvention
├── BondConvention
└── CdsConvention

Instrument (マーケット用規格化商品)
├── Deposit, Fra, Futures
├── ParSwap, Ois, BasisSwap
└── CrossCurrencySwap
    │
    │ Convention + Instrument → TradeBuilder → Trade
    ▼
```

---

## Components

### Component 1: Index Module (`trade/index.rs`)

**Purpose:** 全ての市場参照指標を統一的に表現

**Requirements Addressed:** Requirement 1

#### Design

```rust
/// 全ての市場参照指標を表現（静的ディスパッチ用 Enum）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IndexType {
    /// 金利指標（既存の RateIndex を再利用）
    Rate(RateIndex),
    /// CMS Swap Rate
    SwapRate { currency: Currency, tenor: Period },
    /// FX Spot Rate
    Fx { base: Currency, quote: Currency },
    /// Equity Index (e.g., "SPX", "NKY")
    Equity(String),
    /// Inflation Index (e.g., "UK-RPI", "US-CPI")
    Inflation(String),
    /// Commodity (e.g., "WTI", "GOLD")
    Commodity(String),
}

/// 特定の観測条件を表現
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IndexObservation {
    pub index_type: IndexType,
    pub observation_lag: Period,
    pub fixing_source: Option<String>,
}
```

**Design Decisions:**

1. **Enum による静的ディスパッチ**: `Box<dyn Index>` ではなく `IndexType` Enum を使用。Enzyme AD との互換性を維持し、分岐予測とインライン化を有利にする。

2. **既存型の再利用**: `RateIndex` を `IndexType::Rate(RateIndex)` としてラップ。新規定義を最小限に抑える。

3. **From トレイト実装**: `impl From<RateIndex> for IndexType` で既存コードからの移行を容易にする。

---

### Component 2: Payoff Module (`trade/payoff.rs`)

**Purpose:** キャッシュフロー計算式を型安全に定義

**Requirements Addressed:** Requirement 2

#### Design

```rust
/// Call/Put フラグ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OptionType {
    Call,
    Put,
}

/// キャッシュフロー計算式（静的ディスパッチ用 Enum）
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Payoff {
    /// Fixed rate coupon
    Fixed { rate: f64 },
    /// Linear payoff: gearing * (index + spread)
    Linear {
        gearing: f64,
        spread: f64,
        index: IndexObservation,
    },
    /// Vanilla option: max(option_type * (index - strike), 0)
    VanillaOption {
        gearing: f64,
        strike: f64,
        option_type: OptionType,
        index: IndexObservation,
    },
    /// Digital option
    Digital {
        payout: f64,
        strike: f64,
        option_type: OptionType,
        index: IndexObservation,
    },
}

impl Payoff {
    #[must_use]
    pub fn required_index(&self) -> Option<&IndexObservation> {
        match self {
            Self::Fixed { .. } => None,
            Self::Linear { index, .. }
            | Self::VanillaOption { index, .. }
            | Self::Digital { index, .. } => Some(index),
        }
    }

    #[must_use]
    pub const fn is_fixed(&self) -> bool {
        matches!(self, Self::Fixed { .. })
    }
}
```

**Design Decisions:**

1. **4 種類のペイオフ**: Fixed（固定）、Linear（変動）、VanillaOption（Cap/Floor）、Digital（デジタル）で金利系商品を網羅。

2. **計算ロジックは Pricer 側**: Payoff は「何を計算するか」を定義。実際の数値計算は Pricer が実行。

---

### Component 3: Cashflow Module (`trade/cashflow.rs`)

**Purpose:** キャッシュフローの最小単位（Node）を定義

**Requirements Addressed:** Requirement 3

#### Design

```rust
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
    pub year_fraction: f64,
    pub notional: f64,
    pub payoff: Payoff,
    pub currency: Currency,
}

impl Cashflow {
    /// Checks if the cashflow is known (fixed) at the given reference date.
    #[must_use]
    pub fn is_fixed(&self, _ref_date: Date) -> bool {
        self.payoff.is_fixed()
    }

    /// Returns true if payment date is after the reference date.
    #[must_use]
    pub fn is_future(&self, ref_date: Date) -> bool {
        self.payment_date > ref_date
    }
}
```

**Design Decisions:**

1. **事前計算された year_fraction**: DayCount 計算は Builder 段階で実行し、Cashflow には結果のみ保持。Pricer のホットパスでの計算を回避。

2. **Payoff の埋め込み**: 各 Cashflow が計算式を直接保持。Pricer は Cashflow を走査するだけで評価可能。

---

### Component 4: Leg Module (`trade/leg.rs`)

**Purpose:** キャッシュフロー列と方向・種別を定義

**Requirements Addressed:** Requirement 4

#### Design

```rust
/// Leg の方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Direction {
    Payer,
    Receiver,
}

impl Direction {
    #[must_use]
    pub const fn sign(&self) -> f64 {
        match self {
            Self::Payer => -1.0,
            Self::Receiver => 1.0,
        }
    }
}

/// Leg の種別（Pricer の識別用）
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
    pub fn new(
        cashflows: Vec<Cashflow>,
        direction: Direction,
        leg_type: LegType,
        currency: Currency,
    ) -> Self {
        Self { cashflows, direction, leg_type, currency }
    }

    pub fn future_cashflows(&self, ref_date: Date) -> impl Iterator<Item = &Cashflow> {
        self.cashflows.iter().filter(move |cf| cf.is_future(ref_date))
    }

    #[must_use]
    pub fn notional(&self) -> f64 {
        self.cashflows.first().map(|cf| cf.notional).unwrap_or(0.0)
    }
}
```

**Design Decisions:**

1. **Direction::sign()**: NPV 計算で符号を適用。Payer = -1.0, Receiver = +1.0。

2. **LegType**: Pricer が Leg の種類を識別するためのタグ。最適化ヒントとして使用可能。

---

### Component 5: Trade Module (`trade/trade.rs`)

**Purpose:** 全ての金融取引を表現する共通フォーマット

**Requirements Addressed:** Requirement 5

#### Design

```rust
/// Trade ID
pub type TradeId = String;

/// 全ての金融取引を表現する共通フォーマット
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
    Swap,
    Swaption {
        exercise_dates: Vec<Date>,
        exercise_type: ExerciseType,
        settlement_type: SettlementType,
    },
    Bond {
        issuer_id: String,
        seniority: String,
    },
    CapFloor,
    FxForward,
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
    pub fn all_cashflows(&self) -> impl Iterator<Item = &Cashflow> {
        self.legs.iter().flat_map(|leg| leg.cashflows.iter())
    }

    pub fn future_cashflows(&self, ref_date: Date) -> impl Iterator<Item = &Cashflow> {
        self.all_cashflows().filter(move |cf| cf.is_future(ref_date))
    }

    #[must_use]
    pub fn num_legs(&self) -> usize {
        self.legs.len()
    }

    #[must_use]
    pub fn is_vanilla_swap(&self) -> bool {
        matches!(self.trade_type, TradeType::Swap) && self.legs.len() == 2
    }
}
```

**Design Decisions:**

1. **TradeType Enum**: Swaption や Bond など、追加情報が必要な取引タイプをバリアントで表現。Generic は汎用。

2. **TradeMetadata**: オプショナルなメタデータ。Pricer には不要だが、レポートや監査用に保持。

3. **イテレータ API**: `all_cashflows()`, `future_cashflows()` で Pricer が必要な情報に効率的にアクセス。

---

### Component 6: Instrument Module (`trade/instrument.rs`)

**Purpose:** マーケット用規格化商品（カーブキャリブレーション用）

**Requirements Addressed:** Requirement 6

#### Design

```rust
/// マーケット用規格化商品
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Instrument {
    Deposit {
        currency: Currency,
        start_date: Date,
        tenor: Period,
        rate: f64,
    },
    Fra {
        currency: Currency,
        start_date: Date,
        tenor: Period,
        rate: f64,
    },
    Futures {
        currency: Currency,
        expiry: Date,
        price: f64,
    },
    ParSwap {
        currency: Currency,
        start_date: Date,
        tenor: Period,
        fixed_rate: f64,
        float_index: RateIndex,
        fixed_frequency: Period,
        float_frequency: Period,
    },
    Ois {
        currency: Currency,
        start_date: Date,
        tenor: Period,
        fixed_rate: f64,
        ois_index: RateIndex,
    },
    BasisSwap {
        currency: Currency,
        start_date: Date,
        tenor: Period,
        spread: f64,
        index1: RateIndex,
        index2: RateIndex,
    },
    CrossCurrencySwap {
        pay_currency: Currency,
        rcv_currency: Currency,
        start_date: Date,
        tenor: Period,
        spread: f64,
    },
}

impl Instrument {
    #[must_use]
    pub fn quote(&self) -> f64 {
        match self {
            Self::Deposit { rate, .. } | Self::Fra { rate, .. } => *rate,
            Self::Futures { price, .. } => 100.0 - price,
            Self::ParSwap { fixed_rate, .. } | Self::Ois { fixed_rate, .. } => *fixed_rate,
            Self::BasisSwap { spread, .. } | Self::CrossCurrencySwap { spread, .. } => *spread,
        }
    }

    #[must_use]
    pub fn currency(&self) -> Currency {
        match self {
            Self::Deposit { currency, .. }
            | Self::Fra { currency, .. }
            | Self::Futures { currency, .. }
            | Self::ParSwap { currency, .. }
            | Self::Ois { currency, .. }
            | Self::BasisSwap { currency, .. } => *currency,
            Self::CrossCurrencySwap { pay_currency, .. } => *pay_currency,
        }
    }
}
```

**Design Decisions:**

1. **最小限のフィールド**: Instrument はマーケットクォートと基本情報のみ保持。詳細な規約は Convention で定義。

2. **to_trade() は Builder 経由**: Instrument 単体では Trade に変換不可。Convention と組み合わせて Builder で構築。

---

### Component 7: Convention Module (`convention/`)

**Purpose:** 通貨・商品タイプごとの市場規約を静的マスターデータとして定義

**Requirements Addressed:** Requirements 11, 12

#### Design

**convention/swap.rs:**
```rust
/// Swap Leg の規約
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwapLegConvention {
    pub day_count: DayCountConvention,
    pub payment_frequency: Frequency,
    pub calendar: CalendarId,
    pub business_day_convention: BusinessDayConvention,
    pub payment_lag: i32,
}

/// Swap 全体の規約
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwapConvention {
    pub fixed_leg: SwapLegConvention,
    pub float_leg: SwapLegConvention,
    pub float_index: RateIndex,
    pub spot_lag: i32,
}
```

**convention/presets.rs:**
```rust
impl SwapConvention {
    /// USD SOFR swap convention
    #[must_use]
    pub fn usd_sofr() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual360,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual360,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_index: RateIndex::Sofr,
            spot_lag: 2,
        }
    }

    /// EUR EURIBOR 6M swap convention
    #[must_use]
    pub fn eur_euribor_6m() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCountConvention::Thirty360,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual360,
                payment_frequency: Frequency::SemiAnnual,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_index: RateIndex::Euribor6m,
            spot_lag: 2,
        }
    }

    /// JPY TONAR swap convention
    #[must_use]
    pub fn jpy_tonar() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::Tokyo,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::Tokyo,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_index: RateIndex::Tonar,
            spot_lag: 2,
        }
    }

    /// GBP SONIA swap convention
    #[must_use]
    pub fn gbp_sonia() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::London,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 0,
            },
            float_leg: SwapLegConvention {
                day_count: DayCountConvention::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::London,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 0,
            },
            float_index: RateIndex::Sonia,
            spot_lag: 0,
        }
    }
}
```

**convention/fx.rs:**
```rust
/// FX 規約
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxConvention {
    pub spot_days: i32,
    pub calendar: CalendarId,
    pub business_day_convention: BusinessDayConvention,
}

impl FxConvention {
    #[must_use]
    pub fn usd_jpy() -> Self {
        Self {
            spot_days: 2,
            calendar: CalendarId::NewYork, // TODO: Combined NY+TKY
            business_day_convention: BusinessDayConvention::Following,
        }
    }

    #[must_use]
    pub fn eur_usd() -> Self {
        Self {
            spot_days: 2,
            calendar: CalendarId::Target, // TODO: Combined TARGET+NY
            business_day_convention: BusinessDayConvention::Following,
        }
    }
}
```

**Design Decisions:**

1. **静的メソッドによるプリセット**: `SwapConvention::usd_sofr()` のように、主要通貨の規約を静的メソッドで提供。

2. **Calendar の組み合わせ**: 実運用では複数カレンダーの組み合わせが必要。初期実装では単一カレンダーで対応し、TODO コメントを残す。

3. **Serde フィーチャーゲート**: 全 Convention 型に `#[cfg_attr(feature = "serde", ...)]` を適用。

---

### Component 8: Builder Module (`trade/builder.rs`)

**Purpose:** Schedule/Leg/Trade の段階的構築

**Requirements Addressed:** Requirements 7, 13

#### Design

```rust
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

    pub fn end_of_month(mut self, eom: bool) -> Self {
        self.end_of_month = eom;
        self
    }

    pub fn build(&self) -> Result<Vec<Date>, TradeError> {
        // Frequency に基づいて start → end を分割
        // BusinessDayConvention で調整
        // Calendar で祝日チェック
        todo!("Schedule generation logic")
    }
}

/// Leg 構築用ビルダー
#[derive(Debug, Clone)]
pub struct LegBuilder {
    schedule: Vec<Date>,
    notional: f64,
    currency: Currency,
    direction: Direction,
    day_count: DayCountConvention,
}

impl LegBuilder {
    pub fn new(schedule: Vec<Date>, notional: f64, currency: Currency) -> Result<Self, TradeError> {
        if schedule.is_empty() {
            return Err(TradeError::InvalidSchedule("Empty schedule".into()));
        }
        if notional < 0.0 {
            return Err(TradeError::InvalidNotional(notional));
        }
        Ok(Self {
            schedule,
            notional,
            currency,
            direction: Direction::Receiver,
            day_count: DayCountConvention::Actual365Fixed,
        })
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn day_count(mut self, day_count: DayCountConvention) -> Self {
        self.day_count = day_count;
        self
    }

    pub fn build_fixed(self, rate: f64) -> Leg {
        let payoff = Payoff::Fixed { rate };
        let cashflows = self.build_cashflows(payoff);
        Leg::new(cashflows, self.direction, LegType::Fixed, self.currency)
    }

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
        self.schedule
            .windows(2)
            .map(|window| {
                let accrual_start = window[0];
                let accrual_end = window[1];
                let year_fraction = self.day_count.year_fraction(accrual_start, accrual_end);
                Cashflow {
                    cf_type: CashflowType::Coupon,
                    payment_date: accrual_end,
                    accrual_start,
                    accrual_end,
                    year_fraction,
                    notional: self.notional,
                    payoff: payoff.clone(),
                    currency: self.currency,
                }
            })
            .collect()
    }
}

/// Trade 構築用ビルダー
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

    /// Convention と Instrument から Trade を構築
    pub fn from_par_swap(
        id: impl Into<TradeId>,
        instrument: &Instrument,
        convention: &SwapConvention,
    ) -> Result<Trade, TradeError> {
        match instrument {
            Instrument::ParSwap {
                currency,
                start_date,
                tenor,
                fixed_rate,
                ..
            } => {
                let end_date = start_date.add_period(tenor)?;

                // Fixed leg schedule
                let fixed_schedule = ScheduleBuilder::new(*start_date, end_date, convention.fixed_leg.payment_frequency)
                    .calendar(convention.fixed_leg.calendar)
                    .business_day_convention(convention.fixed_leg.business_day_convention)
                    .build()?;

                // Float leg schedule
                let float_schedule = ScheduleBuilder::new(*start_date, end_date, convention.float_leg.payment_frequency)
                    .calendar(convention.float_leg.calendar)
                    .business_day_convention(convention.float_leg.business_day_convention)
                    .build()?;

                let notional = 1_000_000.0; // Default notional

                let fixed_leg = LegBuilder::new(fixed_schedule, notional, *currency)?
                    .day_count(convention.fixed_leg.day_count)
                    .direction(Direction::Payer)
                    .build_fixed(*fixed_rate);

                let float_leg = LegBuilder::new(float_schedule, notional, *currency)?
                    .day_count(convention.float_leg.day_count)
                    .direction(Direction::Receiver)
                    .build_floating(convention.float_index, 0.0);

                Ok(Self::new(id)
                    .add_leg(fixed_leg)
                    .add_leg(float_leg)
                    .trade_type(TradeType::Swap)
                    .build())
            }
            _ => Err(TradeError::IncompatibleConvention),
        }
    }
}
```

**Design Decisions:**

1. **Fluent API**: `builder.calendar(x).business_day_convention(y).build()` のようにチェイン可能。

2. **バリデーションは new() で実行**: `LegBuilder::new()` で空スケジュールや負の Notional をチェック。

3. **from_par_swap() パターン**: Instrument と Convention の組み合わせごとに専用メソッドを提供。型安全性を維持。

---

### Component 9: Error Module (`trade/error.rs`)

**Purpose:** Trade 構築時のバリデーションエラー

**Requirements Addressed:** Requirement 8

#### Design

```rust
use thiserror::Error;

/// Trade 構築時のエラー
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TradeError {
    #[error("Invalid schedule: {0}")]
    InvalidSchedule(String),

    #[error("Empty leg: no cashflows generated")]
    EmptyLeg,

    #[error("Invalid notional: {0}")]
    InvalidNotional(f64),

    #[error("Mismatched currency: expected {expected}, got {actual}")]
    MismatchedCurrency {
        expected: String,
        actual: String,
    },

    #[error("Invalid payoff configuration")]
    InvalidPayoff,

    #[error("Incompatible convention for this instrument type")]
    IncompatibleConvention,

    #[error("Date calculation error: {0}")]
    DateError(#[from] crate::DateError),
}
```

**Design Decisions:**

1. **thiserror 使用**: 既存の `infra_domain` パターンに従う。

2. **構造化エラー**: `MismatchedCurrency` のように、デバッグに必要な情報を保持。

3. **From 実装**: `DateError` からの自動変換で `?` 演算子を使用可能に。

---

## Data Flow

### Construction Flow

```text
┌─────────────────────────────────────────────────────────────────┐
│                        INPUT SOURCES                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Direct Builder API:                                         │
│     ┌──────────────────────────────────────────────────────┐   │
│     │ let schedule = ScheduleBuilder::new(start, end, freq)│   │
│     │     .calendar(CalendarId::NewYork)                   │   │
│     │     .build()?;                                       │   │
│     │                                                      │   │
│     │ let fixed_leg = LegBuilder::new(schedule, notional,  │   │
│     │                                  Currency::USD)?     │   │
│     │     .day_count(DayCountConvention::Actual360)        │   │
│     │     .direction(Direction::Payer)                     │   │
│     │     .build_fixed(0.025);                             │   │
│     │                                                      │   │
│     │ let trade = TradeBuilder::new("SWAP001")             │   │
│     │     .add_leg(fixed_leg)                              │   │
│     │     .add_leg(float_leg)                              │   │
│     │     .trade_type(TradeType::Swap)                     │   │
│     │     .build();                                        │   │
│     └──────────────────────────────────────────────────────┘   │
│                                                                 │
│  2. Convention + Instrument API:                                │
│     ┌──────────────────────────────────────────────────────┐   │
│     │ let convention = SwapConvention::usd_sofr();         │   │
│     │                                                      │   │
│     │ let instrument = Instrument::ParSwap {               │   │
│     │     currency: Currency::USD,                         │   │
│     │     start_date: Date::from_ymd(2024, 1, 15)?,        │   │
│     │     tenor: Period::years(5),                         │   │
│     │     fixed_rate: 0.035,                               │   │
│     │     float_index: RateIndex::Sofr,                    │   │
│     │     fixed_frequency: Period::years(1),              │   │
│     │     float_frequency: Period::years(1),              │   │
│     │ };                                                   │   │
│     │                                                      │   │
│     │ let trade = TradeBuilder::from_par_swap(             │   │
│     │     "SWAP002",                                       │   │
│     │     &instrument,                                     │   │
│     │     &convention,                                     │   │
│     │ )?;                                                  │   │
│     └──────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                          TRADE                                  │
├─────────────────────────────────────────────────────────────────┤
│ Trade {                                                         │
│   id: "SWAP001",                                                │
│   legs: [                                                       │
│     Leg { direction: Payer, leg_type: Fixed, ... },             │
│     Leg { direction: Receiver, leg_type: Floating, ... },       │
│   ],                                                            │
│   trade_type: TradeType::Swap,                                  │
│   metadata: TradeMetadata { ... },                              │
│ }                                                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      PRICER LAYER                               │
├─────────────────────────────────────────────────────────────────┤
│ fn price(trade: &Trade, env: &MarketEnvironment) -> f64 {       │
│     trade.all_cashflows()                                       │
│         .filter(|cf| cf.is_future(env.valuation_date))          │
│         .map(|cf| discount(cf, env) * evaluate(cf, env))        │
│         .sum()                                                  │
│ }                                                               │
└─────────────────────────────────────────────────────────────────┘
```

---

## File Structure

```text
crates/infra_domain/src/
├── lib.rs                   # Module declarations + re-exports
├── error.rs                 # MasterDataError (既存)
├── date.rs                  # Date (既存)
├── period.rs                # Period (既存)
├── ...                      # 他の既存モジュール
│
├── convention/              # NEW: Market conventions
│   ├── mod.rs               # pub mod + re-exports
│   ├── swap.rs              # SwapConvention, SwapLegConvention
│   ├── fra.rs               # FraConvention
│   ├── futures.rs           # FuturesConvention
│   ├── capfloor.rs          # CapFloorConvention
│   ├── fx.rs                # FxConvention
│   ├── bond.rs              # BondConvention
│   ├── cds.rs               # CdsConvention
│   └── presets.rs           # SwapConvention::usd_sofr() etc.
│
└── trade/                   # NEW: Trade module
    ├── mod.rs               # pub mod + re-exports
    ├── error.rs             # TradeError
    ├── index.rs             # IndexType, IndexObservation
    ├── payoff.rs            # Payoff, OptionType
    ├── cashflow.rs          # Cashflow, CashflowType
    ├── leg.rs               # Leg, Direction, LegType
    ├── trade.rs             # Trade, TradeType, TradeMetadata
    ├── instrument.rs        # Instrument enum
    └── builder.rs           # ScheduleBuilder, LegBuilder, TradeBuilder
```

---

## Module Exports

### lib.rs への追加

```rust
// 既存の mod 宣言の後に追加
mod convention;
mod trade;

// Re-exports
pub use convention::{
    BondConvention, CapFloorConvention, CdsConvention, FraConvention,
    FuturesConvention, FxConvention, SwapConvention, SwapLegConvention,
};
pub use trade::{
    Cashflow, CashflowType, Direction, ExerciseType, IndexObservation,
    IndexType, Instrument, Leg, LegBuilder, LegType, OptionType, Payoff,
    ScheduleBuilder, SettlementType, Trade, TradeBuilder, TradeError,
    TradeId, TradeMetadata, TradeType,
};

// Prelude への追加
pub mod prelude {
    // 既存の exports...
    pub use crate::{
        // Convention
        SwapConvention, SwapLegConvention, FxConvention,
        // Trade
        Trade, TradeBuilder, Leg, LegBuilder, Cashflow, Payoff,
        Direction, LegType, TradeType, Instrument,
    };
}
```

---

## Testing Strategy

### Unit Tests

各モジュールに `#[cfg(test)] mod tests` を配置：

1. **index.rs**: `IndexType` の `From<RateIndex>` 変換テスト
2. **payoff.rs**: `required_index()`, `is_fixed()` のテスト
3. **cashflow.rs**: `is_fixed()`, `is_future()` のテスト
4. **leg.rs**: `Direction::sign()`, `future_cashflows()` のテスト
5. **trade.rs**: `all_cashflows()`, `is_vanilla_swap()` のテスト
6. **builder.rs**: バリデーションエラー、正常構築のテスト
7. **convention/**: プリセット値の正確性テスト

### Integration Tests

`crates/infra_domain/tests/`:

1. **trade_construction.rs**: Builder API で様々な Trade を構築
2. **convention_integration.rs**: Convention + Instrument → Trade のラウンドトリップ
3. **serde_roundtrip.rs**: JSON シリアライズ/デシリアライズのテスト

---

## Implementation Notes

### 依存関係の追加

`Cargo.toml` への変更は不要（既存の `thiserror`, `serde` を使用）。

### Enzyme AD との互換性

1. **Enum による静的ディスパッチ**: `Box<dyn Trait>` を避け、Enum で表現
2. **Clone 可能な Payoff**: `#[derive(Clone)]` で値コピーを許容
3. **f64 フィールド**: `year_fraction`, `notional`, `rate` は全て `f64`

### 将来の拡張

1. **FixingProvider trait**: Historical Fixing の取得インターフェース（別モジュール）
2. **FpML Adapter**: `adapter_fpml` での Trade 構築
3. **追加 Instrument**: BarrierOption, AsianOption, Commodity Forward

---

## References

- Requirements Document: `.kiro/specs/trade-instrument-module/requirements.md`
- Research Document: `.kiro/specs/trade-instrument-module/research.md`
- Steering Documents: `.kiro/steering/product.md`, `.kiro/steering/tech.md`, `.kiro/steering/structure.md`
- Existing Code: `crates/infra_domain/src/` (Date, Period, Currency, RateIndex, etc.)
