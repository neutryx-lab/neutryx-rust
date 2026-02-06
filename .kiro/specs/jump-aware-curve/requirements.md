# Requirements Document

## Introduction

本仕様書は、`infra_master` における `CurveDefinition` を拡張し、中央銀行会合などのイベントに起因する金利の不連続性（ジャンプ）を曲線構築プロセスで明示的に扱うための要件を定義する。

現状、`EventInstrument` や `CentralBankMeeting` などのイベントデータは存在するが、`CurveDefinition` レベルでこれらを「ジャンプピラー」として統合するインターフェースが欠如している。`BootstrapInterpolation` は連続的な内挿を前提としており、政策金利発表日等における不連続性を無視すると、フォワードカーブに不自然な振動が生じる。

本機能は、内挿アルゴリズムが特定の日付において「右極限」と「左極限」を分離して扱えるようにし、金融モデルとしての整合性を向上させる。

## Project Description (Input)

金融モデルとしての整合性（Central Bank Jump 等）
infra_master には CentralBankMeeting などのイベントデータが存在しますが、これを曲線構築の「ジャンプ」として明示的に扱うためのインターフェースが CurveDefinition レベルで統合されていません。

課題：不連続性の扱い BootstrapInterpolation は連続的な内挿を前提としている可能性が高く、政策金利発表日などの特定の期日における不連続性（Jump）が無視されると、フォワードカーブに不自然な振動が生じます。

洗練化案：Jump-Aware Curve Definition CurveDefinition に JumpPillar を含む構造を明示的に持たせ、内挿アルゴリズムが特定の日付において「右極限」と「左極限」を分離して扱えるように infra_master の定義を拡張すべきです。これは Kentaro 氏の研究テーマである「PINNs を用いた金融モデル較正」における制約条件付学習とも通底する論点です。

## Requirements

### Requirement 1: JumpPillar 定義構造

**Objective:** As a quant developer, I want JumpPillar を CurveDefinition 内で明示的に定義できる構造, so that 中央銀行会合等のイベント日における金利ジャンプを曲線構築で考慮できる。

#### Acceptance Criteria

1. The `CurveDefinition` shall include an optional `jump_pillars` field that accepts a list of `JumpPillar` references.
2. When JumpPillar が CurveDefinition に追加された場合, the `CurveDefinition` shall validate that each JumpPillar date falls within the curve's date range.
3. The `JumpPillar` shall contain the following fields:
   - `jump_date`: Date - ジャンプ発生日
   - `expected_jump_bps`: f64 - 予想ジャンプ幅（ベーシスポイント）
   - `event_reference`: Option<String> - EventInstrument への参照ID
   - `confidence`: f64 - ジャンプ発生確率（0.0〜1.0）
4. The `JumpPillar` shall provide a `from_event_instrument` constructor to create a JumpPillar from an existing `EventInstrument`.
5. When serialising CurveDefinition with serde, the system shall preserve all JumpPillar data in the JSON output.

### Requirement 2: 不連続性対応の内挿インターフェース

**Objective:** As a curve builder, I want 内挿アルゴリズムがジャンプ日において左極限と右極限を分離して扱える, so that フォワードカーブに不自然な振動が発生しない。

#### Acceptance Criteria

1. The interpolation interface shall support querying values at a date with optional `Limit` specification (`Left`, `Right`, or `Continuous`).
2. When the query date equals a JumpPillar date and `Limit::Left` is specified, the interpolator shall return the value immediately before the jump.
3. When the query date equals a JumpPillar date and `Limit::Right` is specified, the interpolator shall return the value immediately after the jump.
4. When the query date equals a JumpPillar date and `Limit::Continuous` is specified (or no limit specified), the interpolator shall return the right-limit value by default.
5. If no JumpPillar exists for the query date, the interpolator shall return the same value regardless of the Limit specification.
6. The `BootstrappedCurve` in `pricer_models` shall implement the jump-aware interpolation interface.

### Requirement 3: ブートストラップエンジンの拡張

**Objective:** As a curve calibrator, I want ブートストラップエンジンが JumpPillar を考慮した曲線構築を実行できる, so that 中央銀行会合日を跨ぐ商品の評価が正確になる。

#### Acceptance Criteria

1. The `CurveBootstrapper` shall accept CurveDefinition with JumpPillar specifications.
2. When calibrating pillars around a JumpPillar date, the `CurveBootstrapper` shall apply the jump offset to discount factors after the jump date.
3. The calibration engine shall correctly price instruments that span multiple JumpPillar dates.
4. The `CurveBootstrapper` shall provide a method to retrieve the effective jump amount at any given date.
5. If multiple JumpPillars exist on the same date, the `CurveBootstrapper` shall aggregate their effects.
6. While calibrating, the `CurveBootstrapper` shall log diagnostic information about applied jumps when debug mode is enabled.

### Requirement 4: EventInstrument との統合

**Objective:** As a market data user, I want EventInstrument から自動的に JumpPillar を生成できる, so that 中央銀行会合カレンダーから曲線定義を効率的に構築できる。

#### Acceptance Criteria

1. The system shall provide a `JumpPillarBuilder` that accepts a list of `EventInstrument` and produces corresponding `JumpPillar` entries.
2. When filtering EventInstruments for a specific RateIndex, the `JumpPillarBuilder` shall only include events matching that index.
3. The `JumpPillarBuilder` shall support filtering by date range to exclude past events.
4. The `JumpPillarBuilder` shall support minimum confidence threshold filtering.
5. When a CurveDefinition references a RateIndex, the system shall provide a method to auto-populate JumpPillars from the registered EventInstruments for that index.

### Requirement 5: フォワードレート計算の整合性

**Objective:** As a derivatives pricer, I want フォワードレート計算がジャンプを正しく反映する, so that OIS スワップや FRA の価格付けが正確になる。

#### Acceptance Criteria

1. The forward rate calculation shall account for jumps when computing forward rates across jump dates.
2. When calculating forward rate between two dates that span a JumpPillar, the calculation shall properly attribute the jump to the correct segment.
3. The system shall provide a method to decompose forward rates into continuous and jump components.
4. If a JumpPillar falls exactly on an instrument's fixing date, the forward rate calculation shall use the post-jump rate.
5. The forward rate calculation results shall be consistent whether computed from discount factors or zero rates.

### Requirement 6: バリデーションとエラーハンドリング

**Objective:** As a system operator, I want 曲線定義とジャンプピラー設定の妥当性が検証される, so that 設定ミスによる計算エラーを事前に防げる。

#### Acceptance Criteria

1. The `CurveDefinition` validation shall verify that JumpPillar dates are unique within the definition.
2. If a JumpPillar date conflicts with an instrument pillar date, the validation shall emit a warning but allow the configuration.
3. The validation shall reject JumpPillar entries with confidence values outside the [0.0, 1.0] range.
4. The validation shall reject JumpPillar entries with expected_jump_bps that would result in negative discount factors.
5. When validation fails, the system shall return a structured error with specific field and reason.
6. The `CurveDefError` enum shall include new variants for JumpPillar-related validation failures.

### Requirement 7: 後方互換性

**Objective:** As an existing user, I want 既存の CurveDefinition 設定が変更なく動作し続ける, so that 本機能導入によるリグレッションが発生しない。

#### Acceptance Criteria

1. The existing CurveDefinition API shall remain unchanged; JumpPillar support shall be purely additive.
2. When deserializing CurveDefinition JSON without `jumpPillars` field, the system shall default to an empty list.
3. The `CurveBootstrapper` shall produce identical results for CurveDefinitions without JumpPillars compared to the current implementation.
4. While maintaining backward compatibility, the system shall not require changes to existing caller code.
5. The existing interpolation methods shall continue to work when no JumpPillars are defined.
