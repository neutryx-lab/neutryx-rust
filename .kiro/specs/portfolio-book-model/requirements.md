# Requirements Document

## Project Description (Input)
Portfolioについての定義をInfra_masterに実装したい。目的はXVA計算、Exposure計算、Netting計算など。更にその上位or複合概念であるBookについても定義したい。ここは大事なので想定漏れが内容に慎重かつ詳細に検討したい。

## Introduction

本仕様書は、`infra_domain`クレートにおけるPortfolioおよびBook定義の要件を定義する。これらの構造体は、XVA計算（CVA/DVA/FVA/KVA/MVA）、Exposure計算（EE/EPE/PFE/ENE/EEPE）、Netting計算の基盤となる。

### 現状分析

**既存の実装**:
- `pricer_risk::portfolio`: Portfolio, Trade, Counterparty, NettingSet構造体（計算用、ランタイム最適化）
- `infra_domain::counterparty`: CounterParty, NettingSet, CsaTerms, MarginTerms（静的マスターデータ）
- `infra_domain::ids`: BookId, PortfolioId, TradeId（ID型定義済み、未統合）
- `infra_domain::trade::TradeMetadata`: `book: Option<BookId>`フィールド存在

**ギャップ**:
- Book概念の実体定義なし
- PortfolioとBookの階層関係未定義
- infra_domainにおけるPortfolioコンテナ不在
- ID間の参照整合性バリデーション不足

### 設計方針

**A-I-P-S階層分離**:
- `infra_domain`（I層）: 静的定義、マスターデータ、ビルダー、バリデーション
- `pricer_risk`（P層）: ランタイム計算最適化、並列処理、キャッシュ

本仕様は**I層（infra_domain）の定義**に焦点を当てる。

---

## Requirements

### Requirement 1: Book概念の定義

**Objective:** リスク管理者として、トレーディングブックの概念を定義し、トレードを論理的にグループ化できるようにしたい。これにより、リスク集計とP&L帰属が可能になる。

#### Acceptance Criteria

1. The infra_domain shall provide a `Book` struct with unique identifier (`BookId`), name, and optional description.

2. The infra_domain shall support `BookType` enum to classify books as `Trading`（トレーディング勘定）, `Banking`（銀行勘定）, `Hedge`（ヘッジ勘定）, or `Internal`（内部勘定）.

3. The infra_domain shall provide `BookOwnership` struct to define ownership hierarchy with desk, division, and legal entity references.

4. The infra_domain shall provide a `BookBuilder` with fluent API for constructing valid `Book` instances.

5. When a `Book` is created without a `BookType`, the infra_domain shall default to `BookType::Trading`.

6. If a `Book` is created with duplicate `BookId` within the same container, the infra_domain shall return `BookError::DuplicateId`.

7. The infra_domain shall provide `BookMetadata` struct containing creation timestamp, last modification timestamp, and audit trail information.

8. Where regulatory reporting is required, the infra_domain shall support regulatory classification attributes (`RegulatoryBookType`: `TB`/`NTBR`/`BB`).

---

### Requirement 2: Portfolio定義とBook階層

**Objective:** ポートフォリオマネージャーとして、Portfolioを複数のBookの集合体として定義し、階層的なリスク集計を実現したい。

#### Acceptance Criteria

1. The infra_domain shall provide a `PortfolioDefinition` struct with unique identifier (`PortfolioId`), name, description, and associated books.

2. The infra_domain shall support a many-to-many relationship between `PortfolioDefinition` and `Book` through `PortfolioBookMapping`.

3. When a `Book` is added to a `PortfolioDefinition`, the infra_domain shall validate that the `BookId` references an existing book definition.

4. The infra_domain shall provide `PortfolioBuilder` with validation rules for referential integrity.

5. The infra_domain shall support portfolio hierarchy with parent-child relationships through optional `parent_portfolio_id`.

6. If circular portfolio references are detected during validation, the infra_domain shall return `PortfolioError::CircularReference`.

7. The infra_domain shall provide `PortfolioMetadata` containing ownership, regulatory scope, and reporting currency.

8. The infra_domain shall support `PortfolioScope` enum: `Legal`（法的エンティティ）, `Regulatory`（規制報告用）, `Internal`（内部管理用）, `Consolidated`（連結）.

---

### Requirement 3: Book-Trade関連付け

**Objective:** トレーダーとして、各トレードを特定のBookに関連付け、トレード帰属を明確にしたい。

#### Acceptance Criteria

1. The infra_domain shall extend `TradeMetadata` to include mandatory `book_id: BookId` field（現在の`Option<BookId>`から変更）.

2. When a `Trade` is created, the infra_domain shall validate that the referenced `BookId` exists.

3. The infra_domain shall provide `TradeBookAssignment` struct for tracking book assignment history with effective date and reason.

4. When a trade is reassigned to a different book, the infra_domain shall preserve assignment history in `TradeBookAssignment`.

5. The infra_domain shall support `BookTransferReason` enum: `Initial`（新規）, `Reallocation`（再配置）, `Novation`（ノベーション）, `InternalTransfer`（内部移管）.

6. If a trade references a non-existent `BookId`, the infra_domain shall return `TradeError::InvalidBookReference`.

7. The infra_domain shall provide query capability to retrieve all trades by `BookId`.

---

### Requirement 4: Book-NettingSet関係

**Objective:** リスクアナリストとして、BookとNettingSetの関係を定義し、ネッティングスコープを明確にしたい。

#### Acceptance Criteria

1. The infra_domain shall support multiple `NettingSet` instances within a single `Book`.

2. The infra_domain shall enforce that each `NettingSet` belongs to exactly one primary `Book`（netting_set.book_id参照）.

3. When calculating netting benefits, the infra_domain shall only consider trades within the same `NettingSet`.

4. The infra_domain shall provide `BookNettingConfig` struct to define netting rules at book level.

5. While a `NettingSet` spans multiple books（cross-book netting）, the infra_domain shall require explicit `CrossBookNettingAgreement` configuration.

6. The infra_domain shall provide validation to ensure all trades in a `NettingSet` reference the same `Counterparty`.

7. If trades with different counterparties are added to the same `NettingSet`, the infra_domain shall return `NettingSetError::CounterpartyMismatch`.

---

### Requirement 5: XVA計算サポート構造

**Objective:** XVAデスクとして、CVA/DVA/FVA/KVA/MVA計算に必要な構造を定義し、XVA計算エンジンへの入力を提供したい。

#### Acceptance Criteria

1. The infra_domain shall provide `XvaScope` struct defining calculation scope: netting sets, time horizon, simulation parameters.

2. The infra_domain shall provide `XvaConfig` struct with CVA, DVA, FVA, KVA, MVA calculation flags and method selection.

3. When XVA calculation is requested, the infra_domain shall aggregate relevant netting sets by counterparty.

4. The infra_domain shall provide `FundingConfig` struct for FVA calculation: funding spread curves, collateral rates.

5. The infra_domain shall provide `CapitalConfig` struct for KVA calculation: regulatory method (SA-CCR, IMM), capital rates.

6. The infra_domain shall support `WrongWayRiskConfig` for modelling correlation between counterparty credit and exposure.

7. Where bilateral CVA is calculated, the infra_domain shall require both counterparty and own credit parameters.

8. The infra_domain shall provide `XvaCalculationLevel` enum: `Trade`, `NettingSet`, `Counterparty`, `Book`, `Portfolio`.

---

### Requirement 6: Exposure計算サポート構造

**Objective:** リスク管理者として、Exposure計算（EE/EPE/PFE/ENE/EEPE）に必要な構造を定義し、規制報告要件を満たしたい。

#### Acceptance Criteria

1. The infra_domain shall provide `ExposureProfile` struct to store time-series exposure metrics（EE, EPE, PFE, ENE per time point）.

2. The infra_domain shall provide `ExposureConfig` struct with time grid, confidence levels, simulation parameters.

3. When exposure is calculated at portfolio level, the infra_domain shall aggregate by netting set respecting netting benefits.

4. The infra_domain shall support `ExposureAggregation` enum: `Gross`, `NetWithinNettingSet`, `NetWithinCounterparty`.

5. The infra_domain shall provide `MporConfig`（Margin Period of Risk）with values for collateralised and uncollateralised trades.

6. The infra_domain shall support `PfeConfidenceLevel` enum: `Q95`, `Q97_5`, `Q99`, `Custom(f64)`.

7. While collateral is posted, the infra_domain shall account for collateral through `CollateralizedExposureConfig`.

8. The infra_domain shall provide `EepeCalculator` config with regulatory EEPE calculation parameters（5-year horizon, 1-year effective maturity）.

---

### Requirement 7: Netting計算サポート構造

**Objective:** リーガル担当者として、ネッティング計算に必要な法的構造を定義し、ネッティングの有効性を保証したい。

#### Acceptance Criteria

1. The infra_domain shall provide `NettingAgreement` struct with legal entity pairs, agreement type, enforceability jurisdiction.

2. The infra_domain shall support `NettingAgreementType` enum: `ISDA`, `GMRA`, `GMSLA`, `CSA`, `Custom`.

3. When netting is applied, the infra_domain shall validate that a valid `NettingAgreement` exists for the counterparty pair.

4. The infra_domain shall provide `CloseoutNetting` struct defining closeout calculation method and timeline.

5. The infra_domain shall support `NettingJurisdiction` struct with enforceability flags per jurisdiction.

6. If netting is requested for a jurisdiction without enforceability, the infra_domain shall return `NettingError::NotEnforceable`.

7. The infra_domain shall provide `PaymentNetting` struct for operational netting separate from closeout netting.

8. Where cross-product netting is allowed, the infra_domain shall validate product eligibility through `CrossProductNettingEligibility`.

---

### Requirement 8: 階層集計機能

**Objective:** レポーティングチームとして、階層的な集計機能を提供し、様々なレベルでのリスク報告を可能にしたい。

#### Acceptance Criteria

1. The infra_domain shall provide `AggregationHierarchy` enum: `Trade`, `NettingSet`, `Book`, `Counterparty`, `Portfolio`, `LegalEntity`.

2. The infra_domain shall provide `AggregationConfig` struct defining grouping keys and aggregation methods.

3. When aggregation is requested, the infra_domain shall validate that the requested hierarchy is consistent with data relationships.

4. The infra_domain shall support `AggregationMethod` enum: `Sum`, `Average`, `Max`, `Min`, `WeightedAverage`.

5. The infra_domain shall provide `DrillDownPath` struct to enable navigation from aggregated to granular data.

6. The infra_domain shall support multi-dimensional aggregation（e.g., by Book and Currency simultaneously）.

7. If aggregation is requested for incompatible dimensions, the infra_domain shall return `AggregationError::IncompatibleDimensions`.

---

### Requirement 9: バリデーションとエラーハンドリング

**Objective:** システム管理者として、データ整合性を保証し、明確なエラーメッセージを提供したい。

#### Acceptance Criteria

1. The infra_domain shall provide `BookError` enum with variants: `DuplicateId`, `InvalidOwnership`, `InvalidType`, `MissingRequiredField`.

2. The infra_domain shall provide `PortfolioError` enum with variants: `DuplicateId`, `CircularReference`, `InvalidBookReference`, `InvalidScope`.

3. The infra_domain shall provide `NettingError` enum with variants: `CounterpartyMismatch`, `NotEnforceable`, `InvalidAgreement`, `CrossBookViolation`.

4. When validation fails, the infra_domain shall return structured errors using `thiserror` derive macro.

5. The infra_domain shall provide `ValidationResult<T>` type alias for `Result<T, ValidationError>`.

6. The infra_domain shall implement `From` conversions between error types for propagation.

7. If multiple validation errors occur, the infra_domain shall collect all errors before returning `ValidationError::Multiple(Vec<Error>)`.

---

### Requirement 10: シリアライゼーションとAPI互換性

**Objective:** インテグレーションエンジニアとして、外部システムとのデータ交換を可能にしたい。

#### Acceptance Criteria

1. Where the `serde` feature is enabled, the infra_domain shall derive `Serialize` and `Deserialize` for all public structs.

2. The infra_domain shall provide `#[serde(rename_all = "camelCase")]` for JSON API compatibility.

3. When serialising `BookId` and `PortfolioId`, the infra_domain shall use string representation for readability.

4. The infra_domain shall support `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields.

5. The infra_domain shall provide schema documentation through `#[serde(doc = "...")]` attributes.

6. The infra_domain shall maintain backward compatibility with existing `pricer_risk::portfolio` serialisation format.

---

### Requirement 11: 既存コードとの統合

**Objective:** 開発者として、既存の`pricer_risk::portfolio`との統合を実現し、クリーンな設計を維持したい。

#### Acceptance Criteria

1. The infra_domain shall provide `From` implementations to convert `infra_domain::Book` to `pricer_risk::portfolio` compatible types.

2. The infra_domain shall update existing `TradeMetadata` to use mandatory `book_id: BookId`（`Option<BookId>`から変更）.

3. When `pricer_risk` portfolio is constructed, the infra_domain definitions shall be used as the source of truth.

4. The infra_domain shall update all existing code that references optional book assignment to use mandatory assignment.

5. If a `Trade` is created without `BookId`, the infra_domain shall return `TradeError::MissingBookId` at compile time or construction time.

6. The infra_domain shall remove any legacy optional book field handling from the codebase.

---

### Requirement 12: ISDA Master Agreement構造

**Objective:** リーガル担当者として、ISDAマスター契約を明確に定義し、CSA契約との階層関係を管理したい。各カウンターパーティは複数のISDA契約を持ち、各ISDAは複数のCSAを持つ可能性がある。

#### Acceptance Criteria

1. The infra_domain shall provide `IsdaMasterAgreement` struct with unique identifier, counterparty reference, and agreement date.

2. The infra_domain shall support `IsdaPaymentMethod` enum: `Full`（全額決済）, `Limited`（限定決済）, `OnewayCounterparty`（CP一方向）, `OnewayOwn`（自社一方向）.

3. The infra_domain shall support one-to-many relationship from `IsdaMasterAgreement` to `VmCsa`（1つのISDAに複数のCSA）.

4. The infra_domain shall support `nonCsaTrades` collection within `IsdaMasterAgreement` for trades under ISDA but without CSA coverage.

5. When an ISDA contains both CSA-covered and non-CSA trades, the infra_domain shall calculate netting separately for each group.

6. The infra_domain shall provide `IsdaInitialMargin` struct at ISDA level with posted IM (`im_post`), received IM (`im_recv`), IM currency, and IM interest rate curve reference.

7. The infra_domain shall support pre-calculated exposure paths for non-CSA trades through `other_non_csa_exposure_path` field.

---

### Requirement 13: VM CSA詳細構造（非対称条件）

**Objective:** コラテラル管理者として、VM CSA（変動証拠金担保契約）の詳細条件を定義し、カウンターパーティ側と自社側の非対称条件を正確にモデル化したい。

#### Acceptance Criteria

1. The infra_domain shall provide `VmCsa` struct with name, base currency, call frequency, and eligible collaterals.

2. The infra_domain shall support asymmetric threshold amounts through `threshold_counterparty: f64`（正値）and `threshold_own: f64`（負値）.

3. The infra_domain shall support asymmetric minimum transfer amounts through `mta_counterparty: f64`（正値）and `mta_own: f64`（負値）.

4. The infra_domain shall support dynamic independent amount calculation through `IndependentAmountConfig`:
   - `ia_counterparty: f64` + `k_counterparty: f64` × max(PV, 0) for counterparty
   - `ia_own: f64` + `k_own: f64` × min(PV, 0) for own side

5. The infra_domain shall support asymmetric haircuts through `haircut_counterparty: f64`（typically negative, e.g., -0.05）and `haircut_own: f64`（typically positive, e.g., 0.05）.

6. The infra_domain shall provide `CsaCallFrequency` enum: `Daily`, `Weekly`, `Biweekly`, `Monthly` with corresponding MPOR determination.

7. The infra_domain shall support `EligibleCollateral` with current balance tracking through `current_collateral_balances: Vec<f64>`.

8. The infra_domain shall support pre-calculated exposure for other trades in the CSA through `other_exposure_path` and `other_exposure_currency`.

---

### Requirement 14: NoDoc取引（ネッティング不可取引）

**Objective:** リスク管理者として、ISDA契約が存在しないネッティング不可取引を管理し、グロスエクスポージャー計算を可能にしたい。

#### Acceptance Criteria

1. The infra_domain shall provide `NoDocTrades` struct to group trades without netting documentation.

2. The infra_domain shall support `NettingEligibility` enum: `FullNetting`（CSA付きISDA）, `IsdaOnly`（CSA無しISDA）, `NoNetting`（NoDoc）.

3. When calculating exposure for NoDoc trades, the infra_domain shall compute positive and negative exposure separately:
   - `positive_exposure`: Σ max(PV_i, 0) ≥ 0
   - `negative_exposure`: Σ min(PV_i, 0) ≤ 0

4. The infra_domain shall support pre-calculated exposure paths for NoDoc trades through `other_nodoc_positive_exposure_path` and `other_nodoc_negative_exposure_path`.

5. The infra_domain shall provide `TradeNettingClassification` to classify each trade into appropriate netting category.

6. If a trade is classified as NoDoc, the infra_domain shall exclude it from closeout netting calculations.

---

### Requirement 15: CounterpartyPortfolio階層構造

**Objective:** XVAデスクとして、カウンターパーティ単位でのポートフォリオ階層構造を定義し、XVA計算の入力構造を明確にしたい。

#### Acceptance Criteria

1. The infra_domain shall provide `CounterpartyPortfolio` struct containing:
   - `counterparty_credit_index`: Credit index reference for the counterparty
   - `isda_agreements: Vec<IsdaMasterAgreement>`: Multiple ISDA contracts
   - `nodoc_trades: NoDocTrades`: Trades without netting documentation

2. The infra_domain shall support the following nesting hierarchy:
   ```
   CounterpartyPortfolio
   ├── CreditIndex
   ├── IsdaMasterAgreement[]
   │   ├── VmCsa[]
   │   │   └── Trades[]
   │   ├── NonCsaTrades[]
   │   └── InitialMargin
   └── NoDocTrades[]
   ```

3. The infra_domain shall provide `loop_all_trades` iterator to traverse all trades across the hierarchy.

4. When aggregating exposure at counterparty level, the infra_domain shall:
   - Net within each CSA（with collateral）
   - Net within each ISDA（non-CSA trades）
   - Sum gross exposure for NoDoc trades

5. The infra_domain shall provide utility methods: `get_all_currencies()`, `get_all_payment_dates()`, `get_all_fixing_dates()`, `get_all_exercise_dates()`.

6. The infra_domain shall support `Vec<CounterpartyPortfolio>` as the top-level input structure for XVA calculations.

---

### Requirement 16: 事前計算Exposure構造

**Objective:** パフォーマンス最適化として、事前計算されたエクスポージャーパスを取り込み、インクリメンタル計算を可能にしたい。

#### Acceptance Criteria

1. The infra_domain shall provide `PreCalculatedExposurePath` struct with:
   - `exposure_by_date: BTreeMap<Date, Vec<f64>>`: exposure[date][path_index]
   - `currency: Currency`: Unit currency of the exposure values

2. The infra_domain shall support pre-calculated exposure at multiple levels:
   - `VmCsa::other_exposure_path`: Other trades in the same CSA
   - `IsdaMasterAgreement::other_non_csa_exposure_path`: Other non-CSA trades in the same ISDA
   - `NoDocTrades::other_positive_exposure_path`: Other NoDoc positive exposure
   - `NoDocTrades::other_negative_exposure_path`: Other NoDoc negative exposure

3. When calculating total exposure, the infra_domain shall add pre-calculated exposure to newly calculated exposure: `Total = Calculated + PreCalculated`.

4. The infra_domain shall validate that pre-calculated exposure dates align with simulation time grid.

5. If currency mismatch occurs between pre-calculated and calculated exposure, the infra_domain shall return `ExposureError::CurrencyMismatch`.

6. The infra_domain shall provide `ExposurePathBuilder` for constructing pre-calculated exposure paths from external systems.

---

### Requirement 17: MPOR（Margin Period of Risk）決定

**Objective:** リスク管理者として、担保条件に基づいたMPOR（証拠金リスク期間）を自動決定し、規制要件を満たしたい。

#### Acceptance Criteria

1. The infra_domain shall provide `MporDetermination` logic based on `CsaCallFrequency`:
   - `Daily`: 10 business days（標準）
   - `Weekly`: 10 business days
   - `Biweekly`: 14 business days（extended）
   - `Monthly`: 20 business days（extended）

2. The infra_domain shall support regulatory MPOR overrides for disputed trades（20 business days minimum）.

3. The infra_domain shall support MPOR extension for illiquid collateral through `collateral_liquidity_adjustment`.

4. When MPOR is calculated, the infra_domain shall consider:
   - Call frequency
   - Collateral type（cash vs non-cash）
   - Trade complexity（OTC vs exchange-traded）
   - Counterparty dispute history

5. The infra_domain shall provide `MporResult` struct containing determined MPOR and calculation rationale.

6. The infra_domain shall support regulatory minimum MPOR（5 business days for cleared, 10 for bilateral）.
