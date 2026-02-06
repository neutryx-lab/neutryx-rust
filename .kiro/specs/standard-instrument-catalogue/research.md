# Research & Design Decisions

---
**Purpose**: standard-instrument-catalogue 仕様の設計判断とアーキテクチャ調査を記録する。
---

## Summary
- **Feature**: `standard-instrument-catalogue`
- **Discovery Scope**: Extension（既存 `trade/` モジュールの拡張）
- **Key Findings**:
  - 既存 `convention/` モジュールを `trade/convention/` に移動し統合
  - `InstrumentDefinition` enum で全資産クラスの商品を統一表現
  - 既存の `Trade` → `Leg` → `Cashflow` パターンを活用した CF 展開

## Research Log

### モジュール配置の決定
- **Context**: convention と instrument の配置先を決定する必要があった
- **Sources Consulted**: `.kiro/steering/structure.md`, A-I-P-S アーキテクチャルール
- **Findings**:
  - Infra 層は Pricer/Service に依存不可（ルール3）
  - Convention は trade 関連のため `trade/` 配下が適切
  - 既存 `infra_domain/src/convention/` → `infra_domain/src/trade/convention/` への移動
- **Implications**: re-export パスの変更が必要、破壊的変更を最小化する migration 戦略

### 既存パターン分析
- **Context**: 新規 enum/struct 設計のパターン参照
- **Sources Consulted**: `trade/instrument.rs`, `trade/payoff.rs`, `trade/pricing_instrument.rs`
- **Findings**:
  - `#[derive(Debug, Clone, PartialEq)]` 標準パターン
  - `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]` で条件付き serde
  - ヘルパーメソッド（`is_*()`, `quote()`, `currency()`）の提供パターン
  - Builder パターン（`LegBuilder`, `TradeBuilder`）
- **Implications**: 新規 `InstrumentDefinition` も同パターンに従う

### エキゾチック商品の既存実装
- **Context**: Asian, Barrier, Lookback の定義場所確認
- **Sources Consulted**: `pricer_pricing/src/path_dependent/`, `pricer_pricing/src/analytical/`
- **Findings**:
  - `AsianParams`, `BarrierParams`, `LookbackParams` が pricer_pricing に存在
  - `BarrierType` enum（Up/Down, In/Out 組み合わせ）
  - Payoff は `PathDependentPayoff` trait で抽象化
- **Implications**: infra_domain の instrument 定義は pricer_pricing と分離、商品「定義」のみ担当

### CF展開パターン
- **Context**: InstrumentDefinition → Trade 変換の設計
- **Sources Consulted**: `trade/builder.rs`, `trade/trade.rs`
- **Findings**:
  - `LegBuilder::build_fixed()`, `build_floating()` で Leg 生成
  - `TradeBuilder::add_leg().build()` で Trade 構築
  - 既存パターンは schedule + notional + currency から構築
- **Implications**: `InstrumentExpander` trait で CF 展開を抽象化、convention を活用

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: 既存拡張 | `instrument.rs` にバリアント追加 | ファイル数最小 | 肥大化（1000行超見込み） | ❌ 非推奨 |
| B: サブモジュール | `trade/instrument/` + `trade/convention/` | 責務分離、保守性 | ファイル数増加 | ✅ 採用 |
| C: ハイブリッド | 既存維持＋新規別モジュール | 影響最小 | 二重管理リスク | 🔶 条件付き |

## Design Decisions

### Decision: モジュール構造
- **Context**: 商品定義と convention の配置
- **Alternatives Considered**:
  1. Option A — 既存ファイル拡張
  2. Option B — trade/ 配下にサブモジュール作成
  3. Option C — 別モジュールで新規作成
- **Selected Approach**: Option B — `trade/instrument/` と `trade/convention/` サブモジュール
- **Rationale**:
  - 既存 `trade/` の責務と一貫性
  - 資産クラス別ファイル分割で保守性向上
  - ユーザー決定に基づく
- **Trade-offs**:
  - ✅ 責務分離、テスト容易
  - ❌ ファイル数増加、re-export パス変更
- **Follow-up**: 既存 `convention/` からの migration パス、deprecation 警告

### Decision: InstrumentDefinition enum 設計
- **Context**: 全資産クラスを統一表現する列挙型
- **Alternatives Considered**:
  1. 単一フラット enum（全バリアント同一レベル）
  2. ネスト enum（AssetClass → Instrument）
  3. trait object（`Box<dyn Instrument>`）
- **Selected Approach**: 単一フラット enum（資産クラスごとにプレフィックス）
- **Rationale**:
  - Enzyme 互換（static dispatch）
  - 既存 `Instrument` enum パターン踏襲
  - パターンマッチ容易
- **Trade-offs**:
  - ✅ 静的ディスパッチ、型安全
  - ❌ enum 肥大化の可能性（50+ バリアント見込み）
- **Follow-up**: バリアント数増加時の分割検討

### Decision: CF展開トレイト
- **Context**: InstrumentDefinition → Trade 変換の抽象化
- **Alternatives Considered**:
  1. `Into<Trade>` impl
  2. `InstrumentExpander` trait with convention
  3. Builder パターン拡張
- **Selected Approach**: `InstrumentExpander` trait（convention パラメータ付き）
- **Rationale**:
  - CF 展開には市場慣行（convention）が必要
  - エラーハンドリングのため `Result` 返却
  - テスト時に convention モック可能
- **Trade-offs**:
  - ✅ 柔軟、テスト容易
  - ❌ 呼び出し時に convention 指定必要
- **Follow-up**: デフォルト convention のファクトリメソッド提供

## Risks & Mitigations
- **Risk 1**: 既存 `convention/` パス変更による破壊的変更 → `pub use` による re-export で互換性維持、deprecation 警告
- **Risk 2**: enum 肥大化 → 資産クラス別サブモジュールで分割管理
- **Risk 3**: CF展開の convention 不足 → `InstrumentError::MissingConvention` エラーで明示

## References
- [A-I-P-S Architecture](.kiro/steering/tech.md) — 依存ルール
- [Project Structure](.kiro/steering/structure.md) — モジュール配置パターン
- [Trade Architecture](crates/infra_domain/src/trade/mod.rs) — CF 展開パターン
