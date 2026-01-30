# Research Log: derive-more-newtype-migration

## Summary

本ドキュメントは derive_more クレート導入に関する調査結果を記録する。ギャップ分析と技術調査の結果、**ハイブリッドアプローチ（段階的移行）** を採用し、ID 型から優先的に移行することを決定した。

**主要な発見**:
- derive_more 最新バージョン: **2.1.1**
- 必要な features: `from`, `display`, `as_ref` (ID 型), `add`, `mul` (数値型)
- Enzyme AD との互換性: proc-macro 生成コードは静的展開のため **互換性あり**
- 移行対象: 10 型、移行除外: 4 型（カスタムロジック）

---

## 1. 現状調査

### 1.1 既存の NewType パターン

コードベースには以下の NewType パターンが存在する:

#### 数値型 NewType（f64 ラッパー）

| 型 | 場所 | 算術演算 | バリデーション |
|---|---|---|---|
| `Delta(f64)` | `infra_master/trade/instrument_def/fx_vol.rs:167` | なし | あり (0 < delta <= 50) |
| `BasisSpread(f64)` | `infra_master/trade/instrument_def/xccy.rs:28` | なし | なし |
| `TracedFloat` | `pricer_core/types/traced_float.rs:60` | 手動実装 | カスタムロジック |
| `SimpleDate(i32)` | `pricer_pricing/generic_pricer/result.rs:55` | なし | なし |

#### ID 型 NewType（String ラッパー）

| 型 | 場所 | Display | From |
|---|---|---|---|
| `CounterPartyId(String)` | `infra_master/counterparty/ids.rs:32` | 手動 | 手動 |
| `LegalEntityId(String)` | `infra_master/counterparty/ids.rs:91` | 手動 | なし (バリデーションあり) |
| `NettingSetId(String)` | `infra_master/counterparty/ids.rs:148` | 手動 | 手動 |
| `CcpId(String)` | `infra_master/counterparty/ids.rs:194` | 手動 | 手動 |
| `IsdaAgreementId(String)` | `infra_master/counterparty/ids.rs:240` | 手動 | 手動 |
| `VariationMarginAgreementId(String)` | `infra_master/counterparty/ids.rs:286` | 手動 | 手動 |
| `CrossBookNettingAgreementId(String)` | `infra_master/counterparty/ids.rs:332` | 手動 | 手動 |
| `TradeId(String)` | `infra_master/ids.rs` via macro | マクロ | マクロ |
| `PortfolioId(String)` | `infra_master/ids.rs` via macro | マクロ | マクロ |
| `BookId(String)` | `infra_master/ids.rs` via macro | マクロ | マクロ |

#### その他の NewType

| 型 | 場所 | 備考 |
|---|---|---|
| `NodeId(u64)` | `pricer_core/types/traced.rs:36` | 計算グラフ用 |
| `ScopeId(u64)` | `pricer_core/types/traced.rs:57` | 計算グラフ用 |
| `Date(NaiveDate)` | `infra_master/time/types.rs:65` | Sub 手動実装 |

### 1.2 既存のマクロパターン

`infra_master/src/ids.rs` に `define_id!` マクロが存在し、以下を自動生成:
- `new()`, `as_str()` メソッド
- `Display` 実装
- `From<String>`, `From<&str>` 実装
- `AsRef<str>` 実装

**ボイラープレート量**: 約 50 行/型 → マクロで約 10 行/型に削減済み

### 1.3 手動算術演算実装

`TracedFloat` の例 ([traced_float.rs:193-231](crates/pricer_core/src/types/traced_float.rs#L193-L231)):
```rust
impl Add for TracedFloat {
    type Output = Self;
    #[track_caller]
    fn add(self, rhs: Self) -> Self::Output {
        let result = self.value + rhs.value;
        self.binary_op(rhs, Operation::Add, result)  // カスタムロジック
    }
}
// Sub, Mul, Div も同様のパターン
```

**注意**: `TracedFloat` は計算グラフをトレースするカスタムロジックを含むため、`derive_more` での置き換え不可。

### 1.4 依存関係の現状

- `derive_more` は `Cargo.lock` に存在（推移的依存）
- ワークスペース依存関係として明示的に定義されていない

---

## 2. 技術調査

### 2.1 derive_more クレート調査

**調査日**: 2026-01-30
**ソース**: [docs.rs/derive_more](https://docs.rs/derive_more/latest/derive_more/)

#### バージョン情報

| 項目 | 値 |
|------|-----|
| 最新バージョン | 2.1.1 |
| 最小 Rust バージョン | 1.81 |
| ライセンス | MIT |

#### 利用可能な derive マクロ

| カテゴリ | マクロ |
|----------|--------|
| 変換 | `From`, `Into`, `FromStr`, `TryFrom`, `TryInto`, `IntoIterator`, `AsRef`, `AsMut` |
| 表示 | `Debug`, `Display`, `Binary`, `Octal`, `LowerHex`, `UpperHex`, `LowerExp`, `UpperExp`, `Pointer` |
| エラー | `Error` |
| 演算子 | `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`, `Not`, `BitAnd`, `BitOr`, `BitXor`, `Shr`, `Shl` |
| 代入演算子 | `AddAssign`, `SubAssign`, `MulAssign`, `DivAssign`, etc. |
| その他 | `Deref`, `DerefMut`, `Index`, `IndexMut`, `Constructor`, `IsVariant`, `Unwrap`, `TryUnwrap` |

#### Feature 設定（コンパイル時間最適化）

```toml
# 最小構成（ID 型向け）
derive_more = { version = "2", features = ["from", "display", "as_ref"] }

# 数値型追加
derive_more = { version = "2", features = ["from", "display", "as_ref", "add", "mul"] }

# 全機能（開発時のみ推奨）
derive_more = { version = "2", features = ["full"] }
```

### 2.2 Enzyme AD 互換性調査

**結論**: **互換性あり**

**根拠**:
1. `derive_more` は proc-macro クレートであり、コンパイル時に標準的な Rust コードに展開される
2. 展開後のコードは通常の `impl Add for T` と同等であり、Enzyme の静的解析に影響しない
3. Enzyme は LLVM IR レベルで動作するため、proc-macro の出自は関係ない
4. `Box<dyn Trait>` は生成されず、静的ディスパッチが維持される

**検証方法**: Phase 1 完了後に `enzyme-ad` feature を有効化してビルド確認

### 2.3 既存マクロとの互換性

`define_id!` マクロは以下を生成:
- `new()`, `as_str()` メソッド → **derive_more では生成不可**
- `Display`, `From<String>`, `From<&str>`, `AsRef<str>` → **derive_more で代替可能**

**決定**: `define_id!` マクロは簡略化して維持（`new()`, `as_str()` のみ生成）、トレイト導出は derive_more に移行

---

## 3. 要件実現可能性分析

### 3.1 要件と既存アセットのマッピング

| 要件 | 技術的ニーズ | 既存アセット | ギャップ |
|------|-------------|-------------|---------|
| Req 1: 依存関係追加 | `[workspace.dependencies]` 更新 | なし | **Missing**: 明示的依存追加が必要 |
| Req 2: 算術トレイト | `#[derive(Add, Sub, Mul, Div)]` | 手動実装 | **Partial**: 単純型のみ移行可能 |
| Req 3: 変換トレイト | `#[derive(From, Into)]` | `define_id!` マクロ, 手動実装 | **Partial**: マクロ置換で削減可能 |
| Req 4: 表示トレイト | `#[derive(Display)]` | 手動実装, マクロ | **Partial**: マクロ置換で削減可能 |
| Req 5: 既存移行 | コードベース走査 | 14+ NewType 型 | **Constraint**: カスタムロジック型は除外 |
| Req 6: AD 互換性 | Enzyme 互換性検証 | `enzyme-ad` feature | **Resolved**: 互換性あり |
| Req 7: テスト | proptest 追加 | 既存テスト | **Partial**: 移行型にテスト追加 |
| Req 8: ドキュメント | steering 更新 | `ai_rules.md` 参照 | **Missing**: ガイドライン追加 |

### 3.2 制約事項

1. **カスタムロジック型**: `TracedFloat`, `Delta`, `LegalEntityId`, `Date` は手動実装を維持
2. **既存マクロ**: `define_id!` を簡略化して維持（メソッド生成のみ）
3. **feature flags**: `derive_more` の機能を最小限に指定（コンパイル時間最適化）

---

## 4. 実装アプローチ決定

### 採用アプローチ: Option C（ハイブリッド）

**根拠**:
1. インクリメンタル検証でリスク軽減
2. AD 互換性を Phase 1 で早期検証可能
3. 既存マクロからの段階的移行

### Phase 構成

| Phase | 内容 | リスク | 対象 |
|-------|------|--------|------|
| Phase 1 | ワークスペース依存追加 + counterparty ID 型移行 | Low | 6 型 |
| Phase 2 | `define_id!` マクロ簡略化 + 残り ID 型移行 | Medium | 4 型 |
| Phase 3 | 数値型 NewType 移行 + ドキュメント | Low | BasisSpread 等 |

---

## 5. 移行対象・除外リスト

### 移行対象（10 型）

| 型 | Phase | 適用 derive |
|----|-------|-------------|
| `CounterPartyId` | 1 | `Display`, `From` |
| `NettingSetId` | 1 | `Display`, `From` |
| `CcpId` | 1 | `Display`, `From` |
| `IsdaAgreementId` | 1 | `Display`, `From` |
| `VariationMarginAgreementId` | 1 | `Display`, `From` |
| `CrossBookNettingAgreementId` | 1 | `Display`, `From` |
| `TradeId` | 2 | `Display`, `From` |
| `PortfolioId` | 2 | `Display`, `From` |
| `BookId` | 2 | `Display`, `From` |
| `BasisSpread` | 3 | `Display`, `From`, `Add`, `Sub` |

### 移行除外（4 型）

| 型 | 理由 |
|----|------|
| `TracedFloat` | 計算グラフトレースのカスタムロジック |
| `Delta` | バリデーションロジック (0 < delta <= 50) |
| `LegalEntityId` | LEI 形式検証ロジック (20 文字) |
| `Date` | 日付演算カスタムロジック |

---

## 6. 工数・リスク評価

### 工数見積

| 項目 | 工数 | 根拠 |
|------|------|------|
| 依存関係追加・設定 | S (1日) | Cargo.toml 更新、feature 選定確定済み |
| Phase 1: counterparty ID 型移行 | S (1日) | 6 型、パターン同一 |
| Phase 2: マクロ簡略化 + 残り ID 型 | S (2日) | 影響範囲限定的 |
| Phase 3: 数値型 + ドキュメント | S (1日) | 対象少数 |
| AD 互換性検証 | S (0.5日) | Phase 1 後にビルド確認のみ |

**総工数**: **S (5-6日)**

### リスク評価

| リスク | レベル | 軽減策 |
|--------|--------|--------|
| AD 互換性問題 | Low | 技術調査で互換性確認済み、Phase 1 で検証 |
| 既存テスト破損 | Low | derive 追加は後方互換 |
| コンパイル時間増加 | Low | feature 最小化で軽減 |
| 公開 API 変更 | Low | 追加のみ、削除なし |

**総合リスク**: **Low**

---

## 参考資料

- [derive_more - Docs.rs](https://docs.rs/derive_more/latest/derive_more/)
- [derive_more - GitHub](https://github.com/JelteF/derive_more)
- [derive_more - crates.io](https://lib.rs/crates/derive_more)
