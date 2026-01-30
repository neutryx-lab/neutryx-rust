# Research Log: boilerplate-reduction

_作成日: 2026-01-30_
_更新日: 2026-01-30_

---

## Summary

本ドキュメントは `boilerplate-reduction` 仕様の調査結果を記録します。主な調査対象は `bon` クレートの機能と既存コードベースへの適用可能性です。

**調査範囲**: Light Discovery（既存システムの拡張）
**主要結論**: bon v3.x は neutryx-rust の Builder パターンに適用可能。段階的移行アプローチを推奨。

---

## 1. 技術調査

### 1.1 bon クレート概要

**ソース**: [docs.rs/bon](https://docs.rs/bon/latest/bon/), [bon-rs.com](https://bon-rs.com/reference/builder)

| 項目 | 詳細 |
|------|------|
| 最新バージョン | 3.6.5 (2025-10時点) |
| Rust Edition | 2021 |
| MSRV | 確認必要 |
| 主要ユーザー | crates.io backend, tantivy, apache-avro, google-cloud-auth |

### 1.2 bon 主要属性

| 属性 | 機能 | 用途 |
|------|------|------|
| `#[builder(default)]` | デフォルト値設定 | オプショナルフィールド |
| `#[builder(default = expr)]` | 明示的デフォルト値 | 特定値のデフォルト |
| `#[builder(into)]` | `impl Into<T>` 受け入れ | `&str` → `String` 変換等 |
| `#[builder(skip)]` | setter 生成スキップ | 内部フィールド |
| `#[builder(with = closure)]` | カスタム setter | バリデーション、変換 |
| `#[builder(finish_fn)]` | 終端関数カスタマイズ | 名前・可視性変更 |

### 1.3 ジェネリック構造体サポート

**結論**: bon はジェネリック構造体をサポート。

```rust
use bon::Builder;

#[derive(Builder)]
struct Container<T: Float> {
    values: Vec<T>,
    #[builder(default)]
    epsilon: T,
}
```

ただし、`CalibrationMatrixBuilder<T: Float>` 等の複雑なジェネリックについては PoC での検証を推奨。

### 1.4 バリデーション統合パターン

**課題**: `LegBuilder::new()` は `Result<Self, TradeError>` を返す。

**解決策1**: `#[builder(with = ...)]` でフィールドレベルバリデーション

```rust
#[derive(Builder)]
struct Leg {
    #[builder(with = |schedule: Vec<Date>| -> Result<Vec<Date>, TradeError> {
        if schedule.len() < 2 {
            return Err(TradeError::InvalidSchedule("Schedule must have at least 2 dates".into()));
        }
        Ok(schedule)
    })]
    schedule: Vec<Date>,
}
```

**解決策2**: `build()` 後の `validate()` メソッド（推奨）

```rust
impl Leg {
    pub fn validate(&self) -> Result<(), TradeError> {
        // バリデーションロジック
    }
}
```

### 1.5 複数終端メソッド

**課題**: `LegBuilder` は `build_fixed(rate)` と `build_floating(index, spread)` を持つ。

**解決策**: bon では単一の `build()` のみ生成されるため、設計変更が必要。

**推奨パターン**:
```rust
#[derive(Builder)]
struct LegConfig {
    schedule: Vec<Date>,
    notional: f64,
    currency: Currency,
    #[builder(default)]
    direction: Direction,
    #[builder(default)]
    day_count: DayCounter,
}

impl LegConfig {
    pub fn into_fixed_leg(self, rate: f64) -> Leg { ... }
    pub fn into_floating_leg(self, index: RateIndex, spread: f64) -> Leg { ... }
}
```

---

## 2. 既存コードベース分析

### 2.1 Builder 構造体一覧

コードベース全体で **31個** の手書き Builder を確認。

#### 移行優先度マトリクス

| 優先度 | クレート | Builder数 | 複雑度 | 移行難易度 |
|--------|---------|----------|--------|-----------|
| P0 | infra_master (book, portfolio) | 2 | 低 | 低 |
| P1 | infra_master (counterparty_entity) | 1 | 低 | 低 |
| P1 | infra_master (csa) | 1 | 中 | 中 |
| P2 | infra_master (trade/builder) | 2 | 中 | 中〜高 |
| P2 | infra_master (counterparty/*) | 8 | 高 | 高 |
| P3 | pricer_pricing (config) | 5 | 低 | 低 |
| P4 | pricer_core (kernel) | 3 | 高 | 高 |
| P4 | pricer_models (builder) | 3 | 高 | 要PoC |
| P5 | pricer_risk (portfolio) | 4 | 低 | 低 |

### 2.2 既存パターン分析

**共通実装パターン**:
- `fn new(required) -> Self` または `fn new(required) -> Result<Self, Error>`
- `fn field(mut self, value) -> Self` チェーンメソッド
- `fn build(self) -> Target` 終端メソッド
- `#[must_use]` 属性の一貫した使用

**bon 移行互換性**:
| パターン | bon 互換性 | 対応方針 |
|---------|-----------|---------|
| 単純チェーン | ✅ 完全互換 | 直接移行 |
| デフォルト値 | ✅ 完全互換 | `#[builder(default)]` |
| `impl Into` | ✅ 完全互換 | `#[builder(into)]` |
| バリデーション | ⚠️ 要調整 | post-build validate() |
| 複数終端メソッド | ⚠️ 要調整 | Config + 変換メソッド |
| ジェネリック | ⚠️ 要PoC | PoC で検証 |

---

## 3. アーキテクチャ決定

### 3.1 採用アプローチ

**決定**: Option A（段階的移行）を採用

**理由**:
1. リスク最小化：各フェーズで動作確認可能
2. 学習曲線：bon の習熟度を段階的に向上
3. ロールバック容易：問題発生時の影響範囲限定

### 3.2 移行フェーズ設計

| Phase | 対象 | 目的 | 成功基準 |
|-------|------|------|---------|
| 0 | 依存関係追加 | bon をワークスペースに追加 | `cargo build` 成功 |
| 1 | BookBuilder, PortfolioBuilder | 単純ケースの検証 | 既存テスト全パス |
| 2 | CounterPartyBuilder, CsaTermsBuilder | デフォルト値パターン検証 | API互換性維持 |
| 3 | TradeBuilder | 中程度の複雑さ検証 | 既存テスト全パス |
| 4 | LegBuilder | 設計変更を伴う移行 | 新API + 既存テスト更新 |
| 5 | counterparty_portfolio.rs | 複雑ケース移行 | 全機能維持 |

### 3.3 API互換性戦略

**原則**: 可能な限り既存 API を維持

**許容される変更**:
- import パスの変更（`use crate::trade::LegBuilder` → `use crate::trade::LegConfig`）
- `build_fixed()` / `build_floating()` → `build().into_fixed_leg()` パターン

**禁止される変更**:
- 必須フィールドのオプショナル化
- 型シグネチャの破壊的変更

---

## 4. リスクと緩和策

| リスク | 発生確率 | 影響度 | 緩和策 |
|--------|---------|--------|--------|
| bon API 非互換 | 低 | 高 | Phase 1 で PoC 検証 |
| テストリグレッション | 中 | 中 | 段階的移行、各フェーズでテスト実行 |
| ジェネリック Builder 失敗 | 中 | 低 | 対象外として残すオプション |
| コンパイル時間増加 | 低 | 低 | マクロ展開は一般的に軽量 |

---

## 5. 未解決事項

1. **bon MSRV 確認**: neutryx-rust の Rust バージョンとの互換性
2. **ジェネリック Builder PoC**: `CalibrationMatrixBuilder<T>` での動作検証
3. **IDE サポート**: rust-analyzer での補完体験

---

## 参考資料

- [bon - Rust (docs.rs)](https://docs.rs/bon/latest/bon/)
- [bon-rs.com - Reference](https://bon-rs.com/reference/builder)
- [bon v2.2 Release Blog](https://bon-rs.com/blog/bon-builder-v2-2-release)
- [derive_builder - Alternative (docs.rs)](https://docs.rs/derive_builder)
