# Requirements Document

## Introduction

本仕様は、Neutryx コードベースにおける New Type パターンの強化を目的として、`derive_more` クレートへの移行を定義する。金融計算では型安全性を確保するため、`f64` をラップした New Type（例: `Notional(f64)`, `Rate(f64)`, `Strike(f64)`）が推奨されるが、これらの型に対する算術演算子（`+`, `-`, `*`, `/`）やトレイト実装を手動で行うとコード量が爆発的に増加する。`derive_more` はこれらの標準トレイト実装を宣言的に追加可能とし、ボイラープレートコードを大幅に削減する。

## Project Description (Input)

derive_more (New Type パターンの強化)に移行。
金融計算では、誤用を防ぐために f64 をラップした型（例: struct Notional(f64), struct Rate(f64)）を作る「New Type パターン」が推奨されます。しかし、これを行うと +, -, *, / などの演算子をすべて手動実装する必要があり、コード量が爆発します。

derive_more はこれらの標準トレイト実装を一行で片付けます。

画期的な点:

Add, Sub, Mul, Display, From, Into などを宣言的に追加可能。

## Requirements

### Requirement 1: 依存関係の追加

**Objective:** As a 開発者, I want derive_more クレートをワークスペース依存関係として追加する, so that 全クレートで統一されたバージョンを使用できる。

#### Acceptance Criteria

1. The Cargo workspace configuration shall include `derive_more` in `[workspace.dependencies]` with appropriate version and features.
2. When derive_more is added to workspace dependencies, the build system shall resolve a single version across all crates.
3. The derive_more dependency shall use `{ workspace = true }` inheritance pattern for crate-level Cargo.toml files.

---

### Requirement 2: 算術演算トレイトの自動導出

**Objective:** As a 金融エンジニア, I want New Type 構造体に算術演算トレイト（Add, Sub, Mul, Div）を宣言的に追加する, so that 手動実装によるボイラープレートを排除できる。

#### Acceptance Criteria

1. When a New Type struct is annotated with `#[derive(Add)]`, the pricer crate shall automatically implement the `std::ops::Add` trait.
2. The derived arithmetic operations shall preserve the New Type wrapper semantics (e.g., `Notional + Notional` returns `Notional`).

---

### Requirement 3: 変換トレイトの自動導出

**Objective:** As a 開発者, I want New Type に From/Into トレイトを宣言的に追加する, so that 内部値との相互変換が容易になる。

#### Acceptance Criteria

1. When a New Type struct is annotated with `#[derive(From)]`, the crate shall automatically implement `From<inner_type>` for the New Type.
2. The derived From implementation shall allow `Notional::from(100.0)` syntax for construction.

---

### Requirement 4: 表示トレイトの自動導出

**Objective:** As a ユーザー, I want New Type に Display トレイトを自動実装する, so that デバッグやログ出力が容易になる。

#### Acceptance Criteria

1. When a New Type struct is annotated with `#[derive(Display)]`, the crate shall automatically implement `std::fmt::Display`.
2. The derived Display implementation shall output the inner value in a readable format.

---

### Requirement 5: 既存 New Type の移行

**Objective:** As a メンテナー, I want 既存の手動トレイト実装を derive_more マクロに置き換える, so that コードの一貫性と保守性が向上する。

#### Acceptance Criteria

1. The migration shall identify all existing New Type structs with manual arithmetic trait implementations across the codebase.
2. When manual trait implementations are replaced with derive macros, the crate shall maintain identical runtime behaviour.
3. The migration shall preserve all existing public API signatures and behaviour.

---

### Requirement 6: AD 互換性の維持

**Objective:** As a クオンツ開発者, I want derive_more による自動導出が Enzyme AD と互換性を持つ, so that 自動微分機能が影響を受けない。

#### Acceptance Criteria

1. The derived trait implementations shall be compatible with Enzyme AD's autodiff macro requirements.
2. While enzyme-ad feature is enabled, the derived arithmetic operations shall support gradient computation.

---

### Requirement 7: テストカバレッジ

**Objective:** As a QA エンジニア, I want 移行後の New Type に対するテストを整備する, so that 機能の正確性が検証される。

#### Acceptance Criteria

1. The test suite shall include unit tests for all derived arithmetic operations on migrated New Types.
2. The test suite shall include property-based tests (`proptest`) verifying mathematical properties (commutativity, associativity where applicable).

---

### Requirement 8: ドキュメンテーション

**Objective:** As a 新規開発者, I want derive_more の使用パターンがドキュメント化される, so that 新しい New Type を作成する際のガイダンスがある。

#### Acceptance Criteria

1. The steering documentation shall include guidelines for using derive_more with New Type patterns.
2. The documentation shall specify which derive macros are recommended for different use cases (numeric types, identifiers, etc.).
