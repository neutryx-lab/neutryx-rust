# Technical Design: derive-more-newtype-migration

## Overview

**Purpose**: 本設計は `derive_more` クレートを導入し、New Type パターンにおけるトレイト実装のボイラープレートコードを削減する。金融計算における型安全性を維持しながら、コードの一貫性と保守性を向上させる。

**Users**: Neutryx 開発者が New Type を作成・保守する際に、宣言的なトレイト導出を利用できるようになる。

**Impact**: 既存の手動トレイト実装（Display, From, 算術演算子）を derive マクロに段階的に置き換える。公開 API の変更はなく、後方互換性を維持。

### Goals

- derive_more v2.1.1 をワークスペース依存関係として追加
- ID 型 9 種に Display, From derive を適用（ボイラープレート削減）
- 数値型 NewType に算術演算 derive を適用
- Enzyme AD との互換性を維持
- steering ドキュメントに使用ガイドラインを追加

---

## Architecture

### Existing Architecture Analysis

**現行パターン**:
1. `define_id!` マクロによる ID 型生成（`infra_domain/src/ids.rs`）
2. 手動トレイト実装（`counterparty/ids.rs` の 7 ID 型）
3. カスタムロジックを含む算術演算実装（`TracedFloat`）

**制約**:
- A-I-P-S 依存方向を維持（derive_more は Infra 層で使用）
- Enzyme AD との静的ディスパッチ互換性

### Architecture Pattern & Boundary Map

**Architecture Integration**:
- **Selected pattern**: 既存拡張（Hybrid approach）
- **Domain boundaries**: derive_more は Infra 層（infra_domain）で主に使用、Pricer 層は検証後に導入
- **Existing patterns preserved**: `define_id!` マクロは簡略化して維持（`new()`, `as_str()` メソッド生成）
- **Steering compliance**: A-I-P-S 依存方向維持、British English 命名規則遵守

---

## Requirements Traceability

| Requirement | Summary | Components | Phase |
|-------------|---------|------------|-------|
| 1.1-1.3 | 依存関係追加 | Cargo.toml | 1 |
| 2.1-2.5 | 算術トレイト導出 | BasisSpread | 3 |
| 3.1-3.4 | 変換トレイト導出 | ID 型 10 種 | 1, 2 |
| 4.1-4.3 | 表示トレイト導出 | ID 型 10 種 | 1, 2 |
| 5.1-5.4 | 既存移行 | counterparty/ids.rs, ids.rs | 1, 2 |
| 6.1-6.3 | AD 互換性 | - | 1（検証） |
| 7.1-7.3 | テスト | tests/newtype_derive.rs | 3 |
| 8.1-8.3 | ドキュメント | steering/ai_rules.md | 3 |

---

## Components and Interfaces

### Component Summary

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies |
|-----------|--------------|--------|--------------|------------------|
| Cargo.toml | Workspace | derive_more 依存追加 | 1.1-1.3 | - |
| counterparty/ids.rs | Infra | 6 ID 型の derive 移行 | 3.1-3.4, 4.1-4.3, 5.1-5.4 | derive_more (P0) |
| ids.rs | Infra | define_id! マクロ簡略化 | 3.1-3.4, 4.1-4.3 | derive_more (P0) |
| BasisSpread | Infra | 数値型 derive 移行 | 2.1-2.5 | derive_more (P0) |

---

### I: Infra Layer

#### Cargo.toml (Workspace)

**Responsibilities & Constraints**
- `[workspace.dependencies]` に derive_more を追加
- features を最小限に指定（コンパイル時間最適化）

**Configuration**

```toml
[workspace.dependencies]
derive_more = { version = "2", features = ["from", "display", "as_ref", "add", "mul"] }
```

---

#### counterparty/ids.rs

**Responsibilities & Constraints**
- CounterPartyId, NettingSetId, CcpId, IsdaAgreementId, VariationMarginAgreementId, CrossBookNettingAgreementId の移行
- LegalEntityId は除外（バリデーションロジック維持）

**Dependencies**
- External: derive_more (P0)

**Before/After Pattern**

Before: 手動 Display/From 実装（約15行/型）

After:
```rust
use derive_more::{Display, From};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Display, From)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct CounterPartyId(String);

impl CounterPartyId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

**削減行数**: 約 15 行/型 × 6 型 = 約 90 行削減

---

#### ids.rs (define_id! マクロ)

**Responsibilities & Constraints**
- TradeId, PortfolioId, BookId の定義
- `new()`, `as_str()`, `AsRef<str>` はマクロで維持
- Display, From は derive_more に移行

**Simplified Macro Pattern**

```rust
use derive_more::{Display, From};

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Display, From)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
            pub fn as_str(&self) -> &str { &self.0 }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str { &self.0 }
        }
    };
}
```

---

#### BasisSpread (数値型)

```rust
use derive_more::{Add, Sub, Mul, Div, Display, From};

#[derive(Debug, Clone, Copy, PartialEq, Add, Sub, Mul, Div, Display, From)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BasisSpread(f64);

impl BasisSpread {
    pub fn new(value: f64) -> Self { Self(value) }
    pub fn value(&self) -> f64 { self.0 }
}
```

---

### Documentation

#### steering/ai_rules.md 更新

**追加セクション**

```markdown
## 5. NewType パターンガイドライン

### derive_more の使用

New Type を作成する際は `derive_more` を使用してボイラープレートを削減する。

**ID 型（String ラッパー）**:
```rust
use derive_more::{Display, From};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Display, From)]
pub struct MyId(String);
```

**数値型（f64 ラッパー）**:
```rust
use derive_more::{Add, Sub, Mul, Div, Display, From};

#[derive(Clone, Copy, PartialEq, Add, Sub, Mul, Div, Display, From)]
pub struct MyValue(f64);
```

### derive_more を使用しない場合

以下の場合は手動実装を維持する:
1. **バリデーションロジック**: コンストラクタで値を検証する型（例: `Delta`, `LegalEntityId`）
2. **カスタム演算**: 演算時に副作用を持つ型（例: `TracedFloat`）
3. **カスタム表示形式**: 特殊なフォーマットが必要な型
```

---

## Data Models

本機能はデータモデル変更なし。既存の NewType 構造体の derive 属性のみ変更。

---

## Error Handling

### Error Strategy

derive_more の導出は全てコンパイル時に処理されるため、ランタイムエラーは発生しない。

**コンパイルエラー**: derive_more が対応していない型構造に適用した場合、コンパイルエラーが発生。明確なエラーメッセージが表示される。

---

## Testing Strategy

### Unit Tests

トレイト実装の動作確認:

```rust
#[test]
fn test_counter_party_id_display() {
    let id = CounterPartyId::new("CP001");
    assert_eq!(format!("{}", id), "CP001");
}

#[test]
fn test_counter_party_id_from() {
    let id: CounterPartyId = "CP001".into();
    assert_eq!(id.as_str(), "CP001");
}
```

### Property-Based Tests

算術演算の数学的性質を検証（交換性、結合性）。

### Integration Tests

```bash
cargo build -p infra_domain --features serde
cargo build -p pricer_risk --features enzyme-ad
cargo test -p infra_domain
```
