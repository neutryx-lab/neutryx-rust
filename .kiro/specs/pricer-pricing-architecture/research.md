# Research & Design Decisions: pricer-pricing-architecture

---
**Purpose**: pricer_pricing クレートのアーキテクチャ再設計に関する調査結果と設計決定の記録

**Usage**:
- ディスカバリフェーズの調査活動と結果を記録
- design.md には詳細すぎる設計決定のトレードオフを文書化
- 将来の監査や再利用のための参照とエビデンスを提供
---

## Summary
- **Feature**: `pricer-pricing-architecture`
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  1. 既存の `GenericPricer` と `MonteCarloPricer` は独立しており、統一インターフェースが不在
  2. Tree手法（Binomial/Trinomial）は新規実装が必要、Cox-Ross-Rubinstein (CRR) モデルを推奨
  3. `PricingError` は拡張可能な構造を持ち、新規バリアント追加が容易

## Research Log

### Tree アルゴリズム選択: CRR vs Jarrow-Rudd

- **Context**: American オプションのプライシングに Tree 手法が必要。最適なアルゴリズムを選定。
- **Sources Consulted**:
  - [Cox-Ross-Rubinstein Model Formulas - Macroption](https://www.macroption.com/cox-ross-rubinstein-formulas/)
  - [Jarrow-Rudd Model Formulas - Macroption](https://www.macroption.com/jarrow-rudd-formulas/)
  - [binomial_tree crate - docs.rs](https://docs.rs/binomial_tree/latest/binomial_tree/)
  - [GitHub - danielhstahl/binomial_tree_rust](https://github.com/danielhstahl/binomial_tree_rust)
- **Findings**:
  - **CRR (Cox-Ross-Rubinstein)**: "equal jumps" モデル。対称的な価格ツリーを構築し、Theta の直接計算が可能
  - **Jarrow-Rudd**: "equal probabilities" モデル。上下移動確率が各50%で、局所ドリフト項を調整
  - 既存 Rust 実装 (`binomial_tree` crate) は 5000 ステップで約0.4秒（C++ 0.7秒比）
  - CRR の対称性は Greeks 計算に有利（Delta, Gamma のツリー直接計算）
- **Implications**: CRR を主要アルゴリズムとして採用。Trinomial は精度向上オプションとして提供。

### 既存 PricingError 構造の拡張性

- **Context**: 新規エラーバリアント追加の実現可能性を確認
- **Sources Consulted**: [generic_pricer/error.rs](crates/pricer_pricing/src/generic_pricer/error.rs)
- **Findings**:
  - `thiserror` ベースの enum 構造
  - `MissingMarketData`, `UnsupportedInstrument` 等のバリアントが既存
  - ヘルパーメソッド（`missing_market_data()`, `unsupported_instrument()`）パターン
  - `is_market_data_error()`, `is_instrument_error()` カテゴリ判定メソッド
- **Implications**: `UnsupportedMethod`, `ConvergenceFailed`, `NumericalInstability` を追加可能。既存パターンに従う。

### GenericPricer の手法ディスパッチ拡張

- **Context**: 既存の GenericPricer に手法選択ロジックを追加する方法を調査
- **Sources Consulted**:
  - [generic_pricer/pricer.rs](crates/pricer_pricing/src/generic_pricer/pricer.rs)
  - [generic_pricer/config.rs](crates/pricer_pricing/src/generic_pricer/config.rs)
- **Findings**:
  - `GenericPricer` は `ModelConfig` と `PricerConfig` を受け取る
  - `GreeksMode` enum (BumpAndRevalue, AAD, NumDual) の選択パターンが存在
  - Builder pattern による設定構築
  - `l1l2-integration` feature flag による条件付きコンパイル
- **Implications**: `PricingMethod` enum (Discount, MonteCarlo, Tree) を追加し、同様のディスパッチパターンを適用。

### 既存 PricingResult の統合可能性

- **Context**: `mc::PricingResult` と `generic_pricer::PricingResult` の統合方法を調査
- **Sources Consulted**:
  - [generic_pricer/result.rs](crates/pricer_pricing/src/generic_pricer/result.rs)
  - [mc/pricer.rs](crates/pricer_pricing/src/mc/pricer.rs)
- **Findings**:
  - `generic_pricer::PricingResult`: Trade/Leg/Cashflow 階層、f64 固定、AD は Greeks 専用
  - `mc::PricingResult`: price, std_error, Greeks (Optional)
  - 両者は異なる責務: generic_pricer は階層構造、mc は統計情報
- **Implications**: `PricingMetadata` 構造体を追加し、手法固有情報（MC: std_error、Tree: 収束ステップ数）を格納。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: GenericPricer拡張 | 既存 GenericPricer に手法選択追加 | 最小変更、既存テスト活用 | 肥大化リスク | 短期的には有効 |
| B: 新規階層構造 | pricer/, methods/, config/, result/ 分離 | 明確な責務分離 | 大規模リファクタリング | 長期的には理想 |
| **C: ハイブリッド** | Tree を独立追加、段階的整理 | リスク分散、互換性維持 | 一時的不整合 | **採用** |

## Design Decisions

### Decision: ハイブリッドアプローチ採用

- **Context**: 既存の成熟した `generic_pricer` と `mc` モジュールを活かしつつ、新規 Tree 手法を追加する必要がある
- **Alternatives Considered**:
  1. Option A: GenericPricer への直接拡張 — 肥大化リスク
  2. Option B: 完全な新規構造 — リファクタリングコスト大
- **Selected Approach**: ハイブリッド（Option C）
  - Phase 1: `tree/` モジュールを独立追加
  - Phase 2: `PricingMethodDispatcher` を `generic_pricer` に統合
  - Phase 3: モジュール構造の段階的整理
- **Rationale**: 既存機能の安定性を維持しつつ、段階的に改善可能
- **Trade-offs**: 一時的な構造の不整合を許容する代わりに、リスク分散と互換性維持を優先
- **Follow-up**: Phase 1 完了後に Phase 2 の必要性を再評価

### Decision: CRR アルゴリズム採用

- **Context**: Binomial Tree の具体的アルゴリズム選択
- **Alternatives Considered**:
  1. Cox-Ross-Rubinstein (CRR) — 対称ツリー、Greeks 直接計算
  2. Jarrow-Rudd (JR) — 等確率、局所ドリフト調整
  3. Leisen-Reimer — 高精度収束、複雑
- **Selected Approach**: CRR を主要実装、Trinomial を精度向上オプション
- **Rationale**:
  - CRR の対称性により Delta/Gamma がツリーから直接計算可能
  - 既存 Rust crate でベンチマーク済み（5000ステップ 0.4秒）
  - AD 統合時のコードパスがシンプル
- **Trade-offs**: JR の等確率性は失うが、Greeks 計算効率を優先
- **Follow-up**: 精度要件が厳しい場合に Leisen-Reimer 追加を検討

### Decision: PricingError バリアント拡張

- **Context**: Tree 手法固有のエラーを追加
- **Alternatives Considered**:
  1. 既存 `Internal` バリアントで代用 — 診断困難
  2. 新規 enum `TreeError` を作成 — 型の分散
- **Selected Approach**: `PricingError` に新規バリアント追加
  - `UnsupportedMethod { method: String, reason: String }`
  - `ConvergenceFailed { method: String, iterations: usize, tolerance: f64 }`
  - `NumericalInstability { method: String, details: String }`
- **Rationale**: 既存の thiserror パターンとカテゴリ判定メソッドに整合
- **Trade-offs**: enum サイズ増加を許容
- **Follow-up**: `is_convergence_error()` ヘルパーメソッド追加

## Risks & Mitigations

| Risk | Level | Mitigation |
|------|-------|------------|
| Tree AD 統合の複雑性 | High | bump-and-revalue を先行実装、Enzyme は後回し |
| 既存 API 破壊 | Medium | feature flag (`tree-pricing`) で段階導入 |
| パフォーマンス劣化 | Medium | ベンチマーク早期導入、criterion テスト |
| テストカバレッジ低下 | Low | TDD アプローチ、既存テストパターン踏襲 |

## References

- [Cox-Ross-Rubinstein Model Formulas - Macroption](https://www.macroption.com/cox-ross-rubinstein-formulas/) — CRR アルゴリズムの数式定義
- [binomial_tree crate - docs.rs](https://docs.rs/binomial_tree/latest/binomial_tree/) — Rust 実装リファレンス
- [GitHub - danielhstahl/binomial_tree_rust](https://github.com/danielhstahl/binomial_tree_rust) — パフォーマンスベンチマーク
- [Binomial Tree, Cox-Ross-Rubinstein, Method - Xilinx](https://xilinx.github.io/Vitis_Libraries/quantitative_finance/2019.2/methods/bt-crr.html) — ハードウェア実装リファレンス

---
_Generated: 2026-01-26_
_Spec: pricer-pricing-architecture_
