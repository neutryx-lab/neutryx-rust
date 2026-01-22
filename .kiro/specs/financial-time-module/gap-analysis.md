# Gap Analysis: financial-time-module

## 1. Current State Investigation

### 1.1 Key Files and Directory Layout

現在の `crates/infra_master/src/` はフラット構造で、時間関連モジュールは以下のファイルで構成されている：

| ファイル | 行数（概算） | 主要型 | テスト数 |
|---------|-------------|--------|---------|
| `date.rs` | ~200 | `Date` | 15 |
| `error.rs` | ~140 | `DateError`, `CurrencyError`, `MasterDataError` | 7 |
| `calendar.rs` | ~300 | `Calendar` (struct), `CalendarId` | 11 |
| `business_day.rs` | ~150 | `BusinessDayConvention` | 9 |
| `day_count.rs` | ~200 | `DayCountConvention` | 12 |
| `tenor.rs` | ~350 | `Tenor`, `EndOfMonthRule` | 17 |
| `period.rs` | ~150 | `Period` (accrual) | 5 |
| **合計** | ~1,500 | 10 型 | 76 |

### 1.2 Reusable Components

**完全に再利用可能:**
- `Date` - NaiveDate wrapper（`Copy`, `Clone`, `Ord`, `Hash`, `FromStr`, `Add`/`Sub` 実装済み）
- `DateError` - 構造化エラー型
- `BusinessDayConvention` - 5 種の営業日調整規約
- `DayCountConvention` - 7 種の日数計算規約（static dispatch）
- `Tenor` - 17 種の標準金融期間
- `EndOfMonthRule` - 3 種の月末調整規則
- `CalendarId` - 5 種のカレンダー識別子

**部分的に再利用可能（拡張が必要）:**
- `Calendar` (struct) → trait 化が必要
- `Period` → `AccrualPeriod` にリネームし、汎用 `Period` を新規追加

### 1.3 Dominant Architecture Patterns

- **Static Dispatch**: `DayCountConvention` は enum + `match` で実装（Enzyme 最適化対応）
- **New Type Pattern**: `Date(NaiveDate)` タプル構造体
- **Error Handling**: `thiserror` による derive、パニックなし
- **Optional Serde**: `serde` feature flag で条件付きシリアライゼーション
- **Co-located Tests**: 各ファイル末尾に `#[cfg(test)]` モジュール

### 1.4 Dependency Hotspots

**infra_master を参照するクレート:**
- `pricer_core::types` - `Date`, `DayCountConvention`, `BusinessDayConvention`, `Currency` を re-export（deprecated 警告付き）
- `pricer_models` - `SwapDirection`, `TradeDirection` を re-export
- `adapter_loader::csa` - `CsaTerms`, `NettingSetConfig` を re-export

**影響範囲:**
- `lib.rs` の re-exports を変更すると、上記クレートの import が影響を受ける
- 後方互換性のため deprecated alias が必要

### 1.5 Integration Surfaces

- **pricer_core::trades::schedules::Period**: 別の `Period` 型が存在（accrual period + day_count）
- **pricer_core::types::time**: `time_to_maturity`, `time_to_maturity_dates` 関数が infra_master の型に依存

---

## 2. Requirements Feasibility Analysis

### 2.1 Technical Needs from EARS Requirements

| Req # | 技術要件 | 種類 |
|-------|---------|------|
| 1 | モジュール再編成（`time/` サブディレクトリ） | ファイル移動 + mod.rs |
| 2 | `TimeError` 統一エラー型 | エラー型拡張 |
| 3 | `to_serial()`, `from_serial()` | メソッド追加 |
| 4 | `Calendar` trait 化 | trait 定義 + impl |
| 5 | `JointCalendar` | 新規 struct + trait impl |
| 6 | `BusinessDayConvention` | 移動のみ |
| 7 | `DayCounter` リネーム | リネーム + 移動 |
| 8 | `TimeUnit` + 汎用 `Period` | 新規型定義 |
| 9 | `Tenor` | 移動 + メソッド追加 |
| 10 | `EndOfMonthRule` | 移動のみ |
| 11 | `AccrualPeriod` | リネーム（既存 `Period` から） |
| 12 | 後方互換性 | deprecated re-exports |
| 13 | テスト | 既存 + 新規テスト追加 |

### 2.2 Gaps and Constraints

#### Missing Capabilities

| 機能 | 状態 | 実装難易度 |
|-----|------|----------|
| `to_serial()` / `from_serial()` | ❌ 未実装 | Low |
| `Calendar` trait | ❌ 未実装（struct のみ） | Medium |
| `JointCalendar` | ❌ 未実装 | Medium |
| `JointCalendarRule` | ❌ 未実装 | Low |
| `TimeUnit` enum | ❌ 未実装 | Low |
| 汎用 `Period` struct | ❌ 未実装 | Medium |
| `Date + Period` trait impl | ❌ 未実装 | Medium |

#### Constraints from Existing Architecture

1. **後方互換性**: `pricer_core`, `pricer_models`, `adapter_loader` が infra_master の型を参照
2. **名前衝突**: `Period` が 2 箇所に存在（`infra_master::Period` と `pricer_core::trades::schedules::Period`）
3. **Serde feature**: 新規型も `#[cfg_attr(feature = "serde", ...)]` パターンに従う必要あり

#### Research Needed

- **Excel leap year bug**: 1900-02-29 の正確な処理（Excel は 1900 を誤って閏年として扱う）
- **QuantLib テストケース**: 30/360 計算の Gold Standard 値の取得

### 2.3 Complexity Signals

| 種類 | 該当箇所 |
|-----|---------|
| **Simple CRUD** | - |
| **Algorithmic Logic** | Excel serial 変換、30/360 計算 |
| **Workflows** | - |
| **External Integrations** | - |

---

## 3. Implementation Approach Options

### Option A: Extend Existing Components (In-Place Migration)

**概要**: 既存ファイルを `time/` に移動し、その場で拡張する。

**変更対象:**
- `date.rs` → `time/types.rs`（`to_serial()`, `from_serial()` 追加）
- `error.rs` → `time/error.rs`（`DateError` を `TimeError` にリネーム）
- `calendar.rs` + `business_day.rs` → `time/calendars.rs`（trait 化 + `JointCalendar` 追加）
- `day_count.rs` → `time/day_counters.rs`（リネームのみ）
- `tenor.rs` + `period.rs` → `time/period.rs`（統合 + `TimeUnit`, 汎用 `Period` 追加）

**Trade-offs:**
- ✅ 最小限の新規ファイル
- ✅ 既存テストをそのまま活用
- ✅ git history が追跡しやすい
- ❌ `Calendar` の trait 化で既存 API を壊す可能性
- ❌ `Period` の名前衝突を解決する必要あり

### Option B: Create New Components (Fresh Implementation)

**概要**: `time/` モジュールを新規作成し、既存コードは非推奨として残す。

**新規作成:**
- `time/mod.rs`
- `time/error.rs` - `TimeError`（新規）
- `time/types.rs` - `Date`（コピー + 拡張）
- `time/calendars.rs` - `Calendar` trait, `ConcreteCalendar`, `JointCalendar`（新規）
- `time/day_counters.rs` - `DayCounter`（コピー + リネーム）
- `time/period.rs` - `TimeUnit`, `Period`, `Tenor`, `AccrualPeriod`（新規 + コピー）

**Trade-offs:**
- ✅ クリーンな API 設計
- ✅ 既存コードへの影響なし（移行期間を設けられる）
- ❌ コード重複
- ❌ 移行完了後に旧コードを削除する必要あり
- ❌ テストを新規に書く必要あり

### Option C: Hybrid Approach (Recommended)

**概要**: 移動 + trait 化を段階的に実施し、既存 API は deprecated re-export で維持。

**Phase 1: 構造移行（低リスク）**
1. `time/` ディレクトリ作成
2. 既存ファイルを移動（内容変更なし）
3. `time/mod.rs` で re-export
4. `lib.rs` で deprecated re-export

**Phase 2: 機能拡張（中リスク）**
1. `Date` に `to_serial()`, `from_serial()` 追加
2. `DateError` → `TimeError` にリネーム（alias 維持）
3. `DayCountConvention` → `DayCounter` にリネーム（alias 維持）
4. `TimeUnit`, 汎用 `Period`, `AccrualPeriod` 追加

**Phase 3: Trait 化（高リスク）**
1. `Calendar` trait を定義
2. 既存 `Calendar` struct を `ConcreteCalendar` にリネーム
3. `JointCalendar` + `JointCalendarRule` 追加
4. 既存 API は wrapper 関数で互換性維持

**Trade-offs:**
- ✅ 段階的なリスク管理
- ✅ 各 Phase でテスト・検証可能
- ✅ 後方互換性を維持しつつ新 API を提供
- ❌ 複数の移行ステップが必要
- ❌ 一時的なコード重複

---

## 4. Requirement-to-Asset Map

| Req # | 要件 | 既存アセット | Gap |
|-------|-----|-------------|-----|
| 1 | モジュール再編成 | フラット構造 | Missing: `time/` ディレクトリ |
| 2 | TimeError | `DateError` | Rename + extend |
| 3 | Excel Serial | `Date` | Missing: `to_serial()`, `from_serial()` |
| 4 | Calendar Trait | `Calendar` struct | Missing: trait definition |
| 5 | JointCalendar | - | Missing: 全体 |
| 6 | BusinessDayConvention | ✅ 完全実装 | 移動のみ |
| 7 | DayCounter | `DayCountConvention` | Rename |
| 8 | TimeUnit + Period | - | Missing: 全体 |
| 9 | Tenor | ✅ 完全実装 | 移動 + `to_period()` 追加 |
| 10 | EndOfMonthRule | ✅ 完全実装 | 移動のみ |
| 11 | AccrualPeriod | `Period` | Rename |
| 12 | 後方互換性 | - | Missing: deprecated re-exports |
| 13 | テスト | 76 existing tests | 新規テスト追加が必要 |

---

## 5. Implementation Complexity & Risk

### Effort Estimate

| Phase | 内容 | Effort | 理由 |
|-------|-----|--------|-----|
| Phase 1 | 構造移行 | S (1-3 days) | ファイル移動 + mod.rs + re-exports |
| Phase 2 | 機能拡張 | M (3-7 days) | 新規型定義 + メソッド追加 + テスト |
| Phase 3 | Trait 化 | M (3-7 days) | trait 定義 + 既存実装移行 + JointCalendar |
| **合計** | | **L (1-2 weeks)** | 3 Phase の総和 |

### Risk Assessment

| リスク | レベル | 理由 |
|-------|-------|-----|
| 後方互換性の破壊 | Medium | deprecated re-exports で軽減可能 |
| `Period` 名前衝突 | Low | `AccrualPeriod` リネームで解決 |
| `Calendar` trait 化 | Medium | 既存 API を壊さない wrapper が必要 |
| Excel serial バグ | Low | 既知の問題、実装パターンが確立 |

**Overall Risk: Medium**

理由: 既存パターンが確立されており、段階的な移行で リスクを管理可能。ただし `Calendar` trait 化は既存 API への影響を慎重に評価する必要あり。

---

## 6. Recommendations for Design Phase

### Preferred Approach

**Option C: Hybrid Approach** を推奨。

理由:
1. 段階的な移行でリスクを分散
2. 各 Phase で検証可能
3. 後方互換性を維持しつつ新 API を提供
4. 既存の 76 テストを活用可能

### Key Decisions for Design Phase

1. **`Calendar` trait の API 設計**
   - 既存 `Calendar` struct のメソッドをそのまま trait に昇格するか
   - `adjust()` のデフォルト実装の範囲

2. **`Period` 名前空間の整理**
   - `infra_master::time::Period`（汎用）vs `infra_master::time::AccrualPeriod`（計算期間）
   - `pricer_core::trades::schedules::Period` との関係

3. **`TimeError` の統合範囲**
   - `DateError` + `CalendarError` + `CalculationError` の統合か分離か

4. **後方互換性の維持期間**
   - deprecated 警告の表示バージョン（`since = "0.3.0"` など）

### Research Items to Carry Forward

1. **Excel Serial Date 仕様**
   - 1900-02-29 の扱い（Lotus 1-2-3 互換バグ）
   - 負の serial number の扱い

2. **QuantLib 30/360 テストケース**
   - US Bond Basis vs European の差異
   - 月末処理のエッジケース

3. **`JointCalendar` のユースケース**
   - 実際のクロスボーダー取引での使用パターン
   - パフォーマンス要件（`Box<dyn Calendar>` のオーバーヘッド）

---

## Appendix: Existing Test Coverage

```text
infra_master/src/
├── calendar.rs      → 11 tests
├── tenor.rs         → 17 tests
├── date.rs          → 15 tests
├── day_count.rs     → 12 tests
├── business_day.rs  →  9 tests
├── currency.rs      →  9 tests
├── rate_index.rs    →  9 tests
├── error.rs         →  7 tests
├── frequency.rs     →  6 tests
├── direction.rs     →  6 tests
├── period.rs        →  5 tests
└── counterparty.rs  →  4 tests
────────────────────────
Total: 110 tests
```

時間関連モジュールのみ: **76 tests** (calendar + tenor + date + day_count + business_day + period)
