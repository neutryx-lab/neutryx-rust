# Requirements Document

## Introduction

本仕様は、neutryx-rust プロジェクトにおける Rust ボイラープレートコードの削減を目的としています。特に `bon` クレートを活用した Builder パターンの自動生成により、`infra_domain` を中心とした手書き Builder 実装を置き換え、コード量の大幅な削減と保守性の向上を実現します。

金融ライブラリの特性（多数の構造体定義、オプショナルフィールド、複雑な初期化ロジック）を踏まえ、`bon` の `#[derive(Builder)]` マクロを導入し、既存の Builder パターンを段階的に移行します。

## Requirements

### Requirement 1: ワークスペース依存関係への bon クレート追加

**Objective:** As a 開発者, I want ワークスペースルートに `bon` クレートを追加する, so that 全てのクレートで統一されたバージョンの Builder 自動生成機能を利用できる。

#### Acceptance Criteria

1. The workspace Cargo.toml shall include `bon` in `[workspace.dependencies]` with version specification.
2. When `bon` is added to workspace dependencies, the dependency-management.md guidelines (workspace inheritance pattern) shall be followed.
3. The workspace shall compile successfully with `cargo build --workspace` after adding the `bon` dependency.

---

### Requirement 2: infra_domain トレード関連構造体への bon 適用

**Objective:** As a 開発者, I want `infra_domain::trade` モジュールの主要構造体に `#[derive(Builder)]` を適用する, so that 手書きの `TradeBuilder`, `LegBuilder` 等を削除し、コード量を削減できる。

#### Acceptance Criteria

1. When `#[derive(bon::Builder)]` is applied to `Trade` struct, the bon crate shall generate a type-safe builder with the same API surface as the existing hand-written builder.
2. When `#[derive(bon::Builder)]` is applied to `Leg` struct, the bon crate shall generate a builder supporting `Direction` and `Vec<Cashflow>` fields.
3. When `#[derive(bon::Builder)]` is applied to `Cashflow` struct, the bon crate shall generate a builder with proper handling of optional fields.
4. If existing builder.rs contains custom validation logic, the validation shall be preserved via `#[builder(with = ...)]` or post-build validation methods.
5. The existing hand-written builder implementations in `trade/builder.rs` shall be removed after migration.

---

### Requirement 3: bon 属性を用いたフィールドカスタマイズ

**Objective:** As a 開発者, I want bon の属性（`#[builder(default)]`, `#[builder(into)]`, `#[builder(skip)]`）を適切に設定する, so that 既存の API 互換性を維持しながら使いやすさを向上できる。

#### Acceptance Criteria

1. Where a field has a sensible default value, the struct shall use `#[builder(default = ...)]` to provide the default.
2. Where a field accepts `&str` to `String` conversion, the struct shall use `#[builder(into)]` to enable ergonomic API.
3. Where a field should not be exposed in the builder, the struct shall use `#[builder(skip)]` with appropriate initialization.
4. The bon crate shall generate compile-time errors if required fields are not provided, ensuring type safety.

---

### Requirement 4: 既存テストとの互換性確保

**Objective:** As a 開発者, I want bon への移行後も既存のテストスイートがパスする, so that リグレッションなく移行できる。

#### Acceptance Criteria

1. When bon builders replace hand-written builders, all existing unit tests in `infra_domain` shall pass without modification (except for import path changes).
2. When builder method signatures change, the migration shall update all call sites across the workspace.
3. The migration shall not change the runtime behavior of constructed objects.
4. While migration is in progress, the workspace shall remain compilable with `cargo build --workspace`.

---

### Requirement 5: ドキュメント整備

**Objective:** As a 開発者, I want bon Builder の使用方法がコード内に文書化されている, so that 将来の開発者が一貫したパターンを踏襲できる。

#### Acceptance Criteria

1. The migrated structs shall include doc comments explaining builder usage patterns.
2. When bon attributes are used with non-obvious semantics, inline comments shall explain the rationale.
3. The `dependency-management.md` steering document shall be updated to reflect `bon` as a workspace dependency.

---

### Requirement 6: 追加の構造体への段階的適用

**Objective:** As a 開発者, I want `infra_domain` 以外の適用可能な構造体も特定する, so that 将来のボイラープレート削減の対象を明確化できる。

#### Acceptance Criteria

1. The implementation shall identify and document additional structs in `pricer_models` (e.g., `GreeksConfig`, `MonteCarloPricer`) that could benefit from bon builders.
2. Where structs already use manual builder patterns, the migration plan shall prioritize high-impact targets (>10 lines of boilerplate reduction).
3. The migration shall follow the A-I-P-S dependency rules: Infra layer first, then Pricer layer.

---

## Out of Scope

- 他のボイラープレート削減クレート（`strum`, `derive_more`, `thiserror` 拡張等）の導入は本仕様の範囲外
- `service_*` クレートへの bon 適用（現在一時的に無効化中）
- 既存の `#[derive(Default)]` 実装の bon への置き換え（Default trait は維持）

## Project Description (Input)

なるべく総コード量を減らしたい。strum のように「Rust特有の冗長な記述（ボイラープレート）」を劇的に削減し、コードの保守性を高めるための「画期的な仕組み（クレートやパターン）」は他にもいくつか存在します。

neutryx-rust のような金融ライブラリの特性（多数の構造体定義、数値計算、複雑なエラー処理）を踏まえ、導入効果が特に高いものを厳選して提案します。

1. bon (次世代ビルダー自動生成)
現在、infra_domain/src/trade/builder.rs などで手書きの Builder パターンを実装されていると思われますが、Rust の手書き Builder は非常にコード量がかさみます。

bon は2024年に登場した比較的新しいクレートで、従来の derive_builder よりも遥かに強力かつ柔軟で、関数引数や構造体に対して直感的な Builder を自動生成します。

画期的な点:

Optional なフィールドの扱いやデフォルト値を、属性（Attribute）だけで制御可能。

コンストラクタ関数 (fn new(...)) に直接アタッチして、関数引数を名前付き引数のように扱える。

削減イメージ:

Rust
// Before: 手書きだと数十行必要
pub struct Trade {
    id: String,
    notional: f64,
    currency: String,
}

impl Trade {
    pub fn builder() -> TradeBuilder { ... }
}
// ... TradeBuilderの実装が続く ...

// After: bon を使用
use bon::Builder;

#[derive(Builder)] // これだけで強力な builder() が生える
pub struct Trade {
    id: String,
    #[builder(default = 1_000_000.0)] // デフォルト値
    notional: f64,
    #[builder(into)] // String <- &str 自動変換
    currency: String,
}

// 呼び出し側
let t = Trade::builder()
    .id("TRD-1")
    .currency("USD") // .into() が自動で効く
    .build();
bon: Builderパターンの記述量を90%削減できます。特に infra_domain 周りで効果絶大です。
