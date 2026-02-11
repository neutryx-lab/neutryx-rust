# Technical Design Document

## Overview

**Purpose**: 本機能は、Neutryx プライシングライブラリの各種 enum 型について、アルファベット順ではなく業務上自然な順序（ドメイン標準）をデフォルトとして統一する。

**Users**: クオンツ開発者、マーケットデータ担当者、リスク管理者が、enum のイテレーションや比較時に業務直感に沿った順序で操作できるようになる。

**Impact**: `Frequency` enum の variant 順序を変更し、`BootstrapInterpolation` の順序を微調整する。その他の enum はドキュメント追加のみ。

### Goals
- `Frequency` を高頻度→低頻度（Daily→Annual）の順序に変更
- `BootstrapInterpolation` を業界使用頻度順に調整
- 既存の正しい順序（Tenor, AssetClass, RateType 等）を維持
- 各 enum にドキュメントで並び順の理由を明記
- serde の name-based serialization による後方互換性維持

---

## Architecture

### Existing Architecture Analysis

本機能は既存の enum 定義を修正するリファクタリングであり、アーキテクチャ変更は発生しない。

**影響を受けるクレート**:
- `infra_domain`: `Frequency` enum（time/frequency.rs）
- `pricer_models`: `Frequency` enum（bootstrapping/instrument.rs）、`BootstrapInterpolation` enum

**既存パターン**:
- Rust の `#[derive(PartialOrd, Ord)]` は enum variant の宣言順を使用
- serde はデフォルトで variant 名による文字列シリアライゼーション

### Architecture Integration

- Selected pattern: In-place refactoring（variant 順序変更のみ）
- Domain boundaries: 各 enum は定義元クレートの責務内
- Existing patterns preserved: `Ord` 派生、serde 統合
- Steering compliance: A-I-P-S レイヤー構造維持

---

## Requirements Traceability

| Requirement | Summary | Components |
|-------------|---------|------------|
| 1.1-1.4 | Frequency を高頻度→低頻度順に | Frequency (infra_domain, pricer_models) |
| 2.1-2.3 | RateType をアセットクラス別に | RateType (infra_domain) |
| 3.1-3.3 | StochasticModelEnum を複雑度順に | StochasticModelEnum (pricer_models) |
| 4.1-4.2 | BootstrapInterpolation を使用頻度順に | BootstrapInterpolation (pricer_models) |
| 5.1-5.2 | CurveName を論理グループ順に | CurveName (pricer_models) |
| 6.1-6.5 | 既存正順序の維持 | Tenor, AssetClass, QuoteType, DayCounter, BDC |
| 7.1-7.3 | Serde 後方互換性 | 全対象 enum |
| 8.1-8.2 | ドキュメント追加 | 全対象 enum |

---

## Components and Interfaces

| Component | Domain/Layer | Intent | Req Coverage |
|-----------|--------------|--------|--------------|
| Frequency | infra_domain/time | 支払頻度定義 | 1.1-1.4 |
| RateType | infra_domain/market | レートタイプ定義 | 2.1-2.3 |
| StochasticModelEnum | pricer_models/models | 確率モデル列挙 | 3.1-3.3 |
| BootstrapInterpolation | pricer_models/market | 補間方式定義 | 4.1-4.2 |
| CurveName | pricer_models/market | カーブ名定義 | 5.1-5.2 |

### infra_domain Layer

#### Frequency

**Responsibilities & Constraints**
- 支払頻度（Daily〜Annual）を表現
- `Ord` 派生で `Daily < Weekly < ... < Annual` を保証
- `periods_per_year()` で年間支払回数を返却

**Service Interface**
```rust
/// Payment frequency ordered from highest (Daily) to lowest (Annual).
///
/// Ordering rationale: Financial schedules typically progress from
/// higher frequency to lower frequency when iterating payment dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Frequency {
    Daily,
    Weekly,
    #[default]
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
}

impl Frequency {
    pub fn periods_per_year(&self) -> u32;
}
```

---

### pricer_models Layer

#### BootstrapInterpolation

**Responsibilities & Constraints**
- 補間方式（LogLinear〜MonotonicCubic）を表現
- `Default` で `LogLinear` を返却
- 業界使用頻度順に並べる

**Service Interface**
```rust
/// Bootstrap interpolation methods ordered by industry usage frequency.
///
/// Ordering rationale: LogLinear is the industry default for discount
/// curves. FlatForward is second most common.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BootstrapInterpolation {
    #[default]
    LogLinear,
    FlatForward,
    LinearZeroRate,
    CubicSpline,
    MonotonicCubic,
}
```

---

#### RateType (Documentation Only)

```rust
/// Rate types grouped by asset class.
///
/// Ordering rationale: Interest rate instruments first (core curve
/// building inputs), then FX instruments (secondary), then volatility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RateType {
    // Interest Rate instruments
    Deposit,
    Fra,
    Futures,
    Swap,
    Ois,
    BasisSwap,
    // FX instruments
    FxSpot,
    FxForward,
    // Volatility
    Vol,
}
```

---

#### StochasticModelEnum (Documentation Only)

```rust
/// Stochastic models ordered by increasing complexity.
///
/// Ordering rationale: GBM is the simplest baseline (1-factor, constant vol).
/// Heston/SABR add stochastic volatility (2-factor). HullWhite/CIR are
/// specialized rate models (1-factor with mean reversion).
#[derive(Debug, Clone)]
pub enum StochasticModelEnum<T: Float> {
    GBM(GBMModel<T>),
    Heston(HestonModel<T>),
    SABR(SABRModel<T>),
    #[cfg(feature = "rates")]
    HullWhite(HullWhiteModel<T>),
    #[cfg(feature = "rates")]
    CIR(CIRModel<T>),
}
```

---

#### CurveName (Documentation Only)

```rust
/// Curve names grouped by rate type and region.
///
/// Ordering rationale: Overnight risk-free rates first (primary
/// discounting), then interbank rates, then functional types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurveName {
    Ois,
    Sofr,
    Tonar,
    Euribor,
    Forward,
    Discount,
    Custom(&'static str),
}
```

---

## Data Models

本機能はデータモデル変更なし。enum variant の順序は論理的なメタデータであり、永続化には影響しない。

**Serde Serialization**:
- すべての対象 enum は name-based serialization を使用
- variant 順序変更はシリアライゼーション形式に影響なし
- デシリアライゼーションは variant 名でマッチングするため後方互換

---

## Testing Strategy

### Unit Tests

1. **Frequency Ord テスト**: 全 variant ペアの順序検証
2. **Frequency periods_per_year テスト**: 年間支払回数検証
3. **BootstrapInterpolation Default テスト**: `LogLinear` 確認
4. **Serde ラウンドトリップテスト**: 順序変更前後で同一 JSON 表現を確認

### Integration Tests

1. **Frequency ソートテスト**: `Vec<Frequency>` をソートして期待順序を確認
2. **既存テスト回帰確認**: `cargo test --workspace` で全テスト通過を確認
