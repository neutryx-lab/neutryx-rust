# Research Log: derive-more-newtype-migration

## Summary

本ドキュメントは derive_more クレート導入に関する調査結果を記録する。ギャップ分析と技術調査の結果、**ハイブリッドアプローチ（段階的移行）** を採用し、ID 型から優先的に移行することを決定した。

**主要な発見**:
- derive_more 最新バージョン: **2.1.1**
- 必要な features: `from`, `display`, `as_ref` (ID 型), `add`, `mul` (数値型)
- Enzyme AD との互換性: proc-macro 生成コードは静的展開のため **互換性あり**
- 移行対象: 10 型、移行除外: 5 型（カスタムロジック/複合構造体）

---

## 1. 現状調査

### 1.1 既存の NewType パターン

#### 数値型 NewType（f64 ラッパー）

| 型 | 場所 | 算術演算 | バリデーション |
|---|---|---|---|
| `Delta(f64)` | `infra_domain/trade/instrument_def/fx_vol.rs:167` | なし | あり (0 < delta <= 50) |
| `BasisSpread(f64)` | `infra_domain/trade/instrument_def/xccy.rs:28` | なし | なし |
| `TracedFloat` | `pricer_core/types/traced_float.rs:60` | 手動実装 | カスタムロジック |

#### ID 型 NewType（String ラッパー）

| 型 | 場所 | Display | From |
|---|---|---|---|
| `CounterPartyId(String)` | `infra_domain/counterparty/ids.rs:32` | 手動 | 手動 |
| `LegalEntityId(String)` | `infra_domain/counterparty/ids.rs:91` | 手動 | なし (バリデーションあり) |
| `NettingSetId(String)` | `infra_domain/counterparty/ids.rs:148` | 手動 | 手動 |
| `TradeId(String)` | `infra_domain/ids.rs` via macro | マクロ | マクロ |
| `PortfolioId(String)` | `infra_domain/ids.rs` via macro | マクロ | マクロ |
| `BookId(String)` | `infra_domain/ids.rs` via macro | マクロ | マクロ |

### 1.2 既存のマクロパターン

`infra_domain/src/ids.rs` に `define_id!` マクロが存在し、以下を自動生成:
- `new()`, `as_str()` メソッド
- `Display` 実装
- `From<String>`, `From<&str>` 実装
- `AsRef<str>` 実装

**ボイラープレート量**: 約 50 行/型 → マクロで約 10 行/型に削減済み

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

#### Feature 設定（コンパイル時間最適化）

```toml
# 最小構成（ID 型向け）
derive_more = { version = "2", features = ["from", "display", "as_ref"] }

# 数値型追加
derive_more = { version = "2", features = ["from", "display", "as_ref", "add", "mul"] }
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

## 3. 実装アプローチ決定

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

## 4. 移行対象・除外リスト

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

### 移行除外（5 型）

| 型 | 理由 |
|----|------|
| `TracedFloat` | 計算グラフトレースのカスタムロジック |
| `Delta` | バリデーションロジック (0 < delta <= 50) |
| `LegalEntityId` | LEI 形式検証ロジック (20 文字) |
| `Date` | 日付演算カスタムロジック |
| `RateId` | NewType ではない（複合構造体: currency, tenor, rate_type, rate_index） |

---

## 5. 工数・リスク評価

### 工数見積

**総工数**: **S (5-6日)**

### リスク評価

| リスク | レベル | 軽減策 |
|--------|--------|--------|
| AD 互換性問題 | Low | 技術調査で互換性確認済み、Phase 1 で検証 |
| 既存テスト破損 | Low | derive 追加は後方互換 |
| コンパイル時間増加 | Low | feature 最小化で軽減 |

**総合リスク**: **Low**
