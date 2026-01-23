# Requirements Document

## Project Description (Input)
複数Termを取り扱うときにアルファベット順ではなく、期間の短い順にデフォルトで並ぶようにしたい。他にも、アセットクラス単位や商品の並びなども、一般的に業務で並べるような順であることをデフォルトにしたい。モデルも簡単なモデルから順に並ぶようにしても良い。全体を見て他にも同様の例があれば同時に修正したい。

## Introduction

本仕様は、Neutryx プライシングライブラリにおける各種 enum 型のデフォルト並び順を、アルファベット順ではなく業務上自然な順序（ドメイン標準）に統一することを目的とする。

コードベース調査の結果、以下の enum が修正対象として特定された：

| Enum | 現在の並び順 | 推奨される並び順 | 優先度 |
|------|-------------|-----------------|--------|
| `Frequency` | Annual→Daily（逆順） | Daily→Annual（高頻度→低頻度） | 高 |
| `RateType` | アルファベット順 | アセットクラス別（Rates→FX→Vol） | 中 |
| `StochasticModelEnum` | アセットクラス混在 | 単純→複雑（GBM→Heston→SABR→HW→CIR） | 中 |
| `BootstrapInterpolation` | やや論理的 | 使用頻度順（LogLinear→FlatForward→...） | 低 |
| `CurveName` | アルファベット順 | 使用頻度/地域別 | 低 |

既に正しく並んでいる enum：
- `Tenor`: 期間順（Overnight→30Y）✓
- `AssetClass`: 銀行組織順（Rates→FX→Equity→Credit→Commodity）✓
- `QuoteType`: 市場慣行順（Bid→Ask→Mid→Last）✓
- `BusinessDayConvention`: 論理順（Following→Preceding→Unadjusted）✓
- `DayCounter`: ファミリー別グループ化 ✓

## Requirements

### Requirement 1: Frequency Enum の業務標準並び順

**Objective:** As a クオンツ開発者, I want Frequency enum が高頻度から低頻度の順（Daily→Annual）で並んでいること, so that 支払頻度を扱う際に自然な順序でイテレーションや比較ができる。

#### Acceptance Criteria
1. The Neutryx Library shall define `Frequency` enum variants in order of decreasing frequency: `Daily`, `Weekly`, `Monthly`, `Quarterly`, `SemiAnnual`, `Annual`.
2. When `Frequency` values are sorted using `Ord` trait, the Neutryx Library shall order them from highest frequency (Daily) to lowest frequency (Annual).
3. When `Frequency` enum derives `PartialOrd` and `Ord`, the Neutryx Library shall use enum variant declaration order for comparison, ensuring `Daily < Weekly < Monthly < Quarterly < SemiAnnual < Annual`.
4. The Neutryx Library shall provide `Frequency::periods_per_year()` method returning consistent values (Daily=252, Weekly=52, Monthly=12, Quarterly=4, SemiAnnual=2, Annual=1).

### Requirement 2: RateType Enum のアセットクラス別グループ化

**Objective:** As a マーケットデータ担当者, I want RateType enum がアセットクラス別にグループ化されていること, so that カーブ構築に使用するレートタイプを論理的に整理できる。

#### Acceptance Criteria
1. The Neutryx Library shall define `RateType` enum variants grouped by asset class: Interest Rate instruments first (`Deposit`, `Fra`, `Futures`, `Swap`, `Ois`, `BasisSwap`), then FX instruments (`FxSpot`, `FxForward`), then volatility (`Vol`).
2. When `RateType` variants are iterated, the Neutryx Library shall return Interest Rate types before FX types, and FX types before volatility types.
3. If `RateType` derives `Ord`, the Neutryx Library shall order variants following the declaration order (asset class grouping preserved).

### Requirement 3: StochasticModelEnum のモデル複雑度順

**Objective:** As a クオンツアナリスト, I want StochasticModelEnum が単純なモデルから複雑なモデルの順で並んでいること, so that モデル選択時に複雑度の階層が明確になる。

#### Acceptance Criteria
1. The Neutryx Library shall define `StochasticModelEnum` variants in order of increasing model complexity: `GBM` (simplest), `Heston`, `SABR`, `HullWhite`, `CIR`.
2. When model complexity ordering is required, the Neutryx Library shall treat GBM as baseline (complexity=1), Heston as intermediate (complexity=2), SABR as advanced (complexity=3), and rate models (HullWhite, CIR) as specialized (complexity=4+).
3. The Neutryx Library shall maintain this ordering regardless of feature-flag configuration (equity, rates enabled/disabled).

### Requirement 4: BootstrapInterpolation の使用頻度順

**Objective:** As a カーブ構築担当者, I want BootstrapInterpolation enum が業界で一般的に使用される順で並んでいること, so that デフォルト選択時に最も一般的な手法が最初に来る。

#### Acceptance Criteria
1. The Neutryx Library shall define `BootstrapInterpolation` enum variants in order of industry usage frequency: `LogLinear` (most common), `FlatForward`, `LinearZeroRate`, `CubicSpline`, `MonotonicCubic`.
2. When `BootstrapInterpolation::default()` is called, the Neutryx Library shall return `LogLinear` as the industry-standard default.

### Requirement 5: CurveName の論理的グループ化

**Objective:** As a リスク管理者, I want CurveName enum が論理的にグループ化されていること, so that カーブ管理時に関連するカーブを識別しやすい。

#### Acceptance Criteria
1. The Neutryx Library shall define `CurveName` enum variants grouped by: overnight rates (`Ois`, `Sofr`, `Tonar`), interbank rates (`Euribor`), functional types (`Forward`, `Discount`), then extensibility (`Custom`).
2. When CurveName variants are displayed or iterated, the Neutryx Library shall present overnight rates before interbank rates, and standard types before custom types.

### Requirement 6: 既存の正しい並び順の維持

**Objective:** As a 開発者, I want 既に業務標準に沿って並んでいる enum の順序が維持されること, so that 既存の動作が破壊されない。

#### Acceptance Criteria
1. The Neutryx Library shall maintain `Tenor` enum in duration order: `Overnight`, `OneWeek`, ..., `ThirtyYears`.
2. The Neutryx Library shall maintain `AssetClass` enum in bank organizational order: `Rates`, `Fx`, `Equity`, `Credit`, `Commodity`.
3. The Neutryx Library shall maintain `QuoteType` enum in market convention order: `Bid`, `Ask`, `Mid`, `Last`.
4. The Neutryx Library shall maintain `DayCounter` enum grouped by mathematical family (Actual/X first, then 30/360 family).
5. The Neutryx Library shall maintain `BusinessDayConvention` enum in logical order: `Following`, `ModifiedFollowing`, `Preceding`, `ModifiedPreceding`, `Unadjusted`.

### Requirement 7: 並び順変更の後方互換性

**Objective:** As a ライブラリ利用者, I want enum の並び順変更がシリアライゼーションやデータベース保存に影響しないこと, so that 既存データとの互換性が保たれる。

#### Acceptance Criteria
1. If `serde` feature is enabled, the Neutryx Library shall serialize enum values by name (string), not by ordinal position.
2. When deserializing enum values, the Neutryx Library shall use name-based matching, ensuring ordering changes do not affect deserialization.
3. The Neutryx Library shall not change any existing enum variant names during this refactoring.

### Requirement 8: ドキュメントでの並び順理由の明記

**Objective:** As a 新規開発者, I want 各 enum の並び順の理由がドキュメントに明記されていること, so that 将来の変更時に設計意図を理解できる。

#### Acceptance Criteria
1. The Neutryx Library shall include doc comments on each enum explaining the ordering rationale (e.g., "Ordered by frequency: highest to lowest").
2. When adding new enum variants, the Neutryx Library shall provide guidance in doc comments on where to place new variants to maintain ordering consistency.
