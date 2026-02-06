# Research & Design Decisions

## Summary
- **Feature**: `legacy-compatibility-removal`
- **Discovery Scope**: Extension（既存コードベースのリファクタリング）
- **Key Findings**:
  - `pricer_core::types` の re-export を使用するファイルは12件（主に pricer_risk）
  - `pricer_models` の重複型は内部使用のみ（外部参照なし）
  - `infra_domain::convention` は使用箇所ゼロで安全に削除可能

## Research Log

### re-export 依存関係の調査
- **Context**: pricer_core からの re-export 削除による影響範囲
- **Sources Consulted**: `grep -r "pricer_core::types::"` 検索結果
- **Findings**:
  - 12ファイルが `pricer_core::types::{Date, Currency, DayCounter, BusinessDayConvention}` を使用
  - 主な依存先: `pricer_risk` (7件), `service_cli` (1件), `adapter_feeds` (1件)
  - `pricer_models::provider.rs` が唯一の L2 内依存
- **Implications**: 各ファイルで import 文を `infra_domain::` に更新必要

### date_utils 重複型の調査
- **Context**: `BusinessDayAdjustment` と `DayCount` の使用状況
- **Sources Consulted**: `grep` 検索、`mod.rs` の re-export 確認
- **Findings**:
  - `bootstrapping::mod.rs` で re-export されているが、外部から参照なし
  - `DateCalculator` 構造体内での内部使用のみ
  - `infra_domain::DayCounter` と機能的に同等
- **Implications**: 削除は API 互換性に影響なし

### CurrencyPair 命名衝突の調査
- **Context**: 2つの同名型の共存状況
- **Sources Consulted**: コードベース検索、型定義の比較
- **Findings**:
  - `infra_domain::trade::instrument_def::fx::CurrencyPair`: spot rate なし、instrument 定義用
  - `pricer_core::types::currency_pair::CurrencyPair<T>`: spot rate あり、AD 対応 pricing 用
  - `pricer_core` 版は外部で使用されていない（re-export されていたが削除対象）
- **Implications**: `pricer_core` 版を `FxRate<T>` にリネームして役割を明確化

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 段階的削除 | 各要件を順番に実行 | リスク分散、ロールバック容易 | 中間状態でのビルドエラー可能性 | 採用 |
| 一括削除 | 全変更を一度に適用 | 中間状態なし | リスク高、デバッグ困難 | 不採用 |

## Design Decisions

### Decision: 削除順序の決定
- **Context**: 依存関係を考慮した安全な削除順序が必要
- **Alternatives Considered**:
  1. 下流から上流（pricer_risk → pricer_core）
  2. 上流から下流（infra_domain → pricer_core → pricer_models）
- **Selected Approach**: 上流から下流（infra_domain から開始）
- **Rationale**:
  - 依存される側を先に修正することで、各段階でビルドが通る
  - deprecated モジュール削除 → re-export 削除 → 依存コード更新の順
- **Trade-offs**: 各段階でコンパイルエラーを修正する必要があるが、問題の特定が容易
- **Follow-up**: 各タスク完了後に `cargo check --workspace` で検証

### Decision: FxRate へのリネーム
- **Context**: `CurrencyPair` 名前衝突の解決方法
- **Alternatives Considered**:
  1. `FxRate<T>` — 為替レートを表す
  2. `SpotPair<T>` — spot rate を含むペア
  3. `FxSpotRate<T>` — より明示的
- **Selected Approach**: `FxRate<T>`
- **Rationale**:
  - 簡潔で用途が明確
  - `infra_domain::CurrencyPair` との区別が容易
  - AD 対応のジェネリック型であることが名前から推測可能
- **Trade-offs**: 「Rate」は通常スカラー値を想起させるが、ペア情報も含む型名としては許容範囲

## Risks & Mitigations

- **Risk 1: 依存コードのコンパイルエラー** — タスクを細分化し、各段階で `cargo check` を実行
- **Risk 2: テスト失敗の見落とし** — 最終検証タスクで全テスト実行を必須化
- **Risk 3: ドキュメント更新漏れ** — steering ファイルの更新をタスクに明示的に含める

## References

- [Rust API Guidelines - Re-exports](https://rust-lang.github.io/api-guidelines/interoperability.html)
- プロジェクト steering: `.kiro/steering/structure.md` (A-I-P-S アーキテクチャ)
