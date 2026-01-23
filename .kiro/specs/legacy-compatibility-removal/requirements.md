# Requirements Document

## Introduction

本仕様は、A-I-P-S アーキテクチャの層責務を厳格に遵守するため、以下のレガシーコードを徹底的に削除する：

1. **Deprecated モジュール**: `infra_master::convention` 後方互換性シム
2. **クロスレイヤー re-export**: Pricer 層 (L1/L2) から Infra 層への違反 re-export
3. **責務外テスト**: 他クレートの型をテストする責務違反テスト
4. **重複型定義**: 高レイヤーに一時的に作られた低レイヤー責務の型

**アーキテクチャ原則**:
```
A: Adapter   → I: Infra → P: Pricer → S: Service
```
- **I**nfra は **P**ricer や **S**ervice に依存してはならない
- **P**ricer は **I**nfra の型を re-export すべきではない（直接インポートを強制）
- 型定義は責務に応じた最低レイヤーに一箇所のみ存在すべき

## Requirements

### Requirement 1: Deprecated Convention モジュールの削除

**Objective:** As a システム開発者, I want deprecated な後方互換性モジュールを削除したい, so that コードベースがクリーンになり、メンテナンス負荷が軽減される。

#### Acceptance Criteria

1. The infra_master crate shall `src/convention/mod.rs` ファイルを削除する。
2. The infra_master crate shall `src/convention/` ディレクトリが存在しない状態にする。
3. When 削除が完了した場合, the infra_master crate shall `lib.rs` から `pub mod convention;` 宣言を削除する。
4. The infra_master crate shall `lib.rs` のモジュールドキュメントから `convention` への参照を削除する。

### Requirement 2: pricer_core クロスレイヤー re-export の削除

**Objective:** As a アーキテクト, I want pricer_core (L1) から infra_master (I) への re-export を削除したい, so that 層責務が明確になり、依存関係が正しく表現される。

#### Acceptance Criteria

1. The pricer_core crate shall `src/types/mod.rs` から `pub use infra_master::{BusinessDayConvention, Currency, Date, DayCounter};` を削除する。
2. The pricer_core crate shall `src/types/time.rs` から `pub use infra_master::{BusinessDayConvention, Date, DayCounter};` を削除する。
3. The pricer_core crate shall `src/types/error.rs` から `pub use infra_master::{CurrencyError, DateError};` を削除する。
4. When re-export を削除した場合, the documentation shall re-export に関するドキュメントコメントを削除または更新する。
5. The pricer_core crate shall `src/types/time.rs` 内の `DayCountConvention` 型を保持する（これは pricer_core 独自の簡略型）。

### Requirement 3: pricer_models クロスレイヤー re-export の削除

**Objective:** As a アーキテクト, I want pricer_models (L2) から infra_master (I) への re-export を削除したい, so that 層責務が明確になる。

#### Acceptance Criteria

1. The pricer_models crate shall `src/lib.rs` から `pub use infra_master::{SwapDirection, TradeDirection};` を削除する。
2. The pricer_models crate shall `SwapDirectionExt` と `TradeDirectionExt` trait の re-export は保持する（これらは pricer_models 独自の拡張）。
3. When re-export を削除した場合, the documentation shall 関連するドキュメントコメントを更新する。

### Requirement 4: 責務外テストの削除

**Objective:** As a 品質管理担当者, I want 責務外のテストを削除したい, so that 各クレートが自身の責務のみをテストする。

#### Acceptance Criteria

1. The pricer_core crate shall `tests/module_exports.rs` から以下のテスト関数を削除する:
   - `test_types_module_exports` (infra_master の Date, DayCountConvention をテスト)
   - `test_types_reexports` (infra_master の Currency, Date をテスト)
   - `test_day_count_convention_variants` (pricer_core 独自型なので保持可能)
   - `test_chrono_integration` (infra_master の time_to_maturity をテスト)
   - `test_business_day_convention_exports` (infra_master の BusinessDayConvention をテスト)
   - `test_day_counter_exports` (infra_master の DayCounter をテスト)
   - `test_currency_exports` (infra_master の Currency をテスト)
2. The pricer_core crate shall pricer_core 独自機能のテスト（smoothing, traits, interpolators, solvers, DualNumber）は保持する。
3. If テストが削除後にコンパイルエラーになる場合, then the test file shall 必要なインポートを修正する。

### Requirement 5: pricer_models 重複型定義の削除

**Objective:** As a アーキテクト, I want 高レイヤー (L2) に一時的に定義された低レイヤー責務の型を削除したい, so that 型定義が一箇所に統一され、混乱を防ぐ。

#### Acceptance Criteria

1. The pricer_models crate shall `src/market/calibration/bootstrapping/date_utils.rs` から `BusinessDayAdjustment` enum を削除する。
2. When `BusinessDayAdjustment` を削除した場合, the code shall `infra_master::BusinessDayConvention` を代わりに使用する。
3. The pricer_models crate shall `src/market/calibration/bootstrapping/date_utils.rs` から `DayCount` enum を削除する。
4. When `DayCount` を削除した場合, the code shall `infra_master::DayCounter` を代わりに使用する。
5. The pricer_models crate shall `DateCalculator` 構造体と関連メソッドを `infra_master` の型を使用するよう更新する。
6. The pricer_models crate shall `SpotDateConvention` enum は保持する（bootstrapping 専用機能として許容、将来の移動候補）。

### Requirement 6: CurrencyPair 名前衝突の解決

**Objective:** As a 開発者, I want 同名で異なる構造の型による混乱を解消したい, so that コードベースの可読性と保守性が向上する。

#### Acceptance Criteria

1. The system shall 以下の2つの `CurrencyPair` 型の役割を明確化する:
   - `infra_master::trade::instrument_def::fx::CurrencyPair` — FX instrument 定義用（spot rate なし）
   - `pricer_core::types::currency_pair::CurrencyPair<T>` — FX pricing 用（spot rate あり、AD 対応）
2. The pricer_core crate shall `CurrencyPair<T>` を `FxRate<T>` または `SpotPair<T>` にリネームする。
3. When リネームが完了した場合, the code shall 全ての参照箇所を新しい名前に更新する。
4. The documentation shall 両型の違いと用途を明確に記述する。

### Requirement 7: 依存コードの更新

**Objective:** As a 開発者, I want re-export 削除後も全コードが正常に動作することを保証したい, so that 破壊的変更が適切に処理される。

#### Acceptance Criteria

1. When `pricer_core::types::Date` への参照が存在する場合, the code shall `infra_master::Date` に更新する。
2. When `pricer_core::types::Currency` への参照が存在する場合, the code shall `infra_master::Currency` に更新する。
3. When `pricer_core::types::DayCounter` への参照が存在する場合, the code shall `infra_master::DayCounter` に更新する。
4. When `pricer_core::types::BusinessDayConvention` への参照が存在する場合, the code shall `infra_master::BusinessDayConvention` に更新する。
5. When `pricer_models::SwapDirection` への参照が存在する場合, the code shall `infra_master::SwapDirection` に更新する。
6. When `pricer_models::TradeDirection` への参照が存在する場合, the code shall `infra_master::TradeDirection` に更新する。
7. When `pricer_models::market::calibration::bootstrapping::BusinessDayAdjustment` への参照が存在する場合, the code shall `infra_master::BusinessDayConvention` に更新する。
8. When `pricer_models::market::calibration::bootstrapping::DayCount` への参照が存在する場合, the code shall `infra_master::DayCounter` に更新する。
9. The system shall 全クレート（adapter_*, pricer_*, service_*, demo）の依存コードを検証・更新する。

### Requirement 8: ドキュメントとステアリングの更新

**Objective:** As a 開発者, I want ドキュメントが最新の設計を反映したい, so that 新規開発者が正しいパスを使用できる。

#### Acceptance Criteria

1. The steering documentation shall `.kiro/steering/structure.md` の pricer_core セクションから re-export の記述を削除する。
2. The steering documentation shall 各クレートの責務境界を明確に記述する。
3. When 削除が完了した場合, the documentation shall `infra_master::convention` への参照を削除または更新する。
4. The steering documentation shall pricer_models の date_utils モジュールの役割を更新する。

### Requirement 9: ビルドとテストの検証

**Objective:** As a CI/CD システム, I want 削除後もビルドとテストが成功することを保証したい, so that 本番環境への影響がない。

#### Acceptance Criteria

1. The workspace shall `cargo build --workspace` が成功する。
2. The workspace shall `cargo test --workspace` が全テストパスする。
3. The workspace shall `cargo clippy --workspace -- -D warnings` が警告なしで完了する。
4. The workspace shall `cargo doc --workspace --no-deps` がエラーなしで完了する。
5. If deprecated 警告が残っている場合, then the build process shall 当該警告が解消されていることを確認する。

### Requirement 10: ID 型安全性の統一（Newtype パターン）

**Objective:** As a 金融システム開発者, I want 全ての ID 型を Newtype パターンで実装したい, so that ID の取り違えがコンパイル時に検出され、オペレーションミスを防止できる。

#### Background

現状、`infra_master::trade::trade.rs` では `pub type TradeId = String;` という型エイリアスを使用している。これは「Stringly Typed」と呼ばれるアンチパターンであり、`TradeId` と `CounterpartyId`（どちらも実体は `String`）を誤って交換してもコンパイルエラーにならない。

一方、`pricer_risk::portfolio::ids` では既に Newtype パターン（`pub struct TradeId(String);`）が正しく実装されている。A-I-P-S アーキテクチャに従い、ID 型は最低レイヤー（I層: `infra_master`）に統一すべきである。

#### Acceptance Criteria

1. The infra_master crate shall `src/ids/mod.rs` モジュールを新設し、以下の Newtype ID 型を定義する:
   - `TradeId`
   - `CounterpartyId`
   - `PortfolioId`
   - `BookId`
   - `IssuerId`
   - `NettingSetId`
   - `LegalEntityId`
   - `CcpId`
2. Each ID type shall 以下の derive と実装を持つ:
   - `#[derive(Clone, Debug, PartialEq, Eq, Hash)]`
   - `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]`
   - `fn new(id: impl Into<String>) -> Self`
   - `fn as_str(&self) -> &str`
   - `impl Display`
   - `impl From<&str>` および `impl From<String>`
3. The infra_master crate shall `src/trade/trade.rs` から `pub type TradeId = String;` を削除する。
4. The infra_master crate shall `TradeMetadata` の以下のフィールドを Newtype に変更する:
   - `counterparty: Option<String>` → `counterparty: Option<CounterpartyId>`
   - `portfolio: Option<String>` → `portfolio: Option<PortfolioId>`
   - `book: Option<String>` → `book: Option<BookId>`
5. The infra_master crate shall `TradeType::Bond` の `issuer_id: Option<String>` を `issuer_id: Option<IssuerId>` に変更する。
6. The pricer_risk crate shall `src/portfolio/ids.rs` の `TradeId`, `CounterpartyId`, `NettingSetId` 定義を削除し、`infra_master::ids` から re-export する。
7. When ID 型を使用するコードが存在する場合, the code shall `infra_master::ids` から直接インポートするよう更新する。
8. The infra_master crate shall `lib.rs` で ID 型を公開エクスポートする。

## Appendix: 削除・変更対象一覧

### A. Deprecated モジュール
| ファイル | 内容 |
|---------|------|
| `infra_master/src/convention/mod.rs` | `trade::convention` への re-export シム |

### B. クロスレイヤー re-export
| クレート | ファイル | 削除対象 |
|---------|---------|---------|
| pricer_core | `src/types/mod.rs:36` | `pub use infra_master::{BusinessDayConvention, Currency, Date, DayCounter};` |
| pricer_core | `src/types/time.rs:28` | `pub use infra_master::{BusinessDayConvention, Date, DayCounter};` |
| pricer_core | `src/types/error.rs:14` | `pub use infra_master::{CurrencyError, DateError};` |
| pricer_models | `src/lib.rs:57` | `pub use infra_master::{SwapDirection, TradeDirection};` |

### C. 責務外テスト
| クレート | ファイル | 削除対象テスト |
|---------|---------|--------------|
| pricer_core | `tests/module_exports.rs` | `test_types_module_exports`, `test_types_reexports`, `test_chrono_integration`, `test_business_day_convention_exports`, `test_day_counter_exports`, `test_currency_exports` |

### D. 重複型定義
| クレート | ファイル | 削除対象 | 代替 |
|---------|---------|---------|------|
| pricer_models | `src/market/calibration/bootstrapping/date_utils.rs:77` | `BusinessDayAdjustment` | `infra_master::BusinessDayConvention` |
| pricer_models | `src/market/calibration/bootstrapping/date_utils.rs:114` | `DayCount` | `infra_master::DayCounter` |

### E. 名前衝突解決
| クレート | ファイル | 現在の名前 | 新しい名前 |
|---------|---------|-----------|-----------|
| pricer_core | `src/types/currency_pair.rs` | `CurrencyPair<T>` | `FxRate<T>` (提案) |

### F. 将来の移動候補（本仕様では保持）
| クレート | ファイル | 型 | 移動先候補 |
|---------|---------|-----|-----------|
| pricer_models | `src/market/calibration/bootstrapping/date_utils.rs:33` | `SpotDateConvention` | `infra_master::time` |

### G. ID 型 Newtype 化（Stringly Typed 解消）
| クレート | ファイル | 現在の定義 | 変更内容 |
|---------|---------|-----------|---------|
| infra_master | `src/trade/trade.rs:12` | `pub type TradeId = String;` | 削除（新規 ids モジュールへ移動） |
| infra_master | `src/trade/trade.rs:92-96` | `TradeMetadata` の String フィールド | Newtype ID に変更 |
| infra_master | `src/trade/trade.rs:56` | `issuer_id: Option<String>` | `Option<IssuerId>` に変更 |
| pricer_risk | `src/portfolio/ids.rs` | Newtype ID 定義 | infra_master からの re-export に変更 |

### H. 新規追加 ID 型一覧
| 型名 | 用途 | 使用箇所例 |
|------|------|-----------|
| `TradeId` | 取引識別子 | `Trade.id` |
| `CounterpartyId` | 取引相手識別子 | `TradeMetadata.counterparty` |
| `PortfolioId` | ポートフォリオ識別子 | `TradeMetadata.portfolio` |
| `BookId` | トレーディングブック識別子 | `TradeMetadata.book` |
| `IssuerId` | 発行体識別子 | `TradeType::Bond.issuer_id` |
| `NettingSetId` | ネッティングセット識別子 | XVA 計算 |
| `LegalEntityId` | 法人識別子 | カウンターパーティ管理 |
| `CcpId` | CCP 識別子 | 清算機関管理 |
