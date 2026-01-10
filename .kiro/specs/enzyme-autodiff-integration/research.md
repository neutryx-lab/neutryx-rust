# Research & Design Decisions: enzyme-autodiff-integration

---
**Purpose**: Enzyme `#[autodiff]` 本格統合に向けた調査結果と設計決定の記録
**Discovery Scope**: Complex Integration（既存Placeholder→本格AD統合）
---

## Summary

- **Feature**: enzyme-autodiff-integration
- **Discovery Scope**: Complex Integration
- **Key Findings**:
  1. Rust nightly `#![feature(autodiff)]` と `-Zautodiff=Enable` フラグが必須
  2. `#[autodiff_forward]`/`#[autodiff_reverse]` マクロで Activity 指定（Const/Dual/Active/Duplicated）
  3. 既存 enzyme モジュールの `ADMode`/`Activity` enum は Enzyme API と整合済み
  4. MC Pricer 統合では並列実行時の adjoint 集約が課題

---

## Research Log

### Rust Autodiff API 調査

- **Context**: Enzyme `#[autodiff]` マクロの最新 API を確認
- **Sources Consulted**:
  - [The Rust Unstable Book - autodiff](https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/autodiff.html)
  - [std::intrinsics::autodiff](https://doc.rust-lang.org/nightly/std/intrinsics/fn.autodiff.html)
  - [rust-lang/rust PR #129458](https://github.com/rust-lang/rust/pull/129458)
  - [Enzyme Rust Ecosystem](https://enzyme.mit.edu/rust/ecosystem.html)
- **Findings**:
  - Feature gate: `#![feature(autodiff)]`
  - Compiler flag: `-Zautodiff=Enable` (RUSTFLAGS経由)
  - Forward mode: `#[autodiff_forward(df_name, Activity1, Activity2, ...)]`
  - Reverse mode: `#[autodiff_reverse(df_name, Activity1, Activity2, ...)]`
  - Activity annotations: `Const`, `Dual`, `Active`, `Duplicated`, `DuplicatedOnly`
- **Implications**:
  - lib.rs の `// #![feature(autodiff)]` をアンコメント（feature gate付き）
  - CI で `-Zautodiff=Enable` は既に設定済み
  - 既存 `Activity` enum は Enzyme API と一致

### 並列 AD 集約パターン

- **Context**: rayon 並列実行時の adjoint 値集約方法
- **Sources Consulted**: 既存 `mc/thread_local.rs`, rayon 公式ドキュメント
- **Findings**:
  - `ThreadLocalWorkspacePool` パターンが既に存在
  - 各スレッドで独立に adjoint を計算し、最後に reduce で集約可能
  - `rayon::iter::ParallelIterator::reduce` で O(log n) 集約
- **Implications**:
  - 既存パターンを拡張して adjoint 集約に適用
  - スレッドローカル `AdjointAccumulator` 導入

### Enzyme 互換関数パターン

- **Context**: Enzyme が微分可能な関数の制約
- **Sources Consulted**: Enzyme documentation, 既存 `smoothing.rs`
- **Findings**:
  - 不連続関数（if/max/min）は smooth approximation が必須
  - 固定サイズ `for` ループ推奨、`while` 禁止
  - `inline_asm` は微分不可
  - 既存 `smooth_max`, `smooth_indicator` は Enzyme 互換
- **Implications**:
  - MC payoff 計算で既存 smoothing 関数を活用
  - 動的ループは固定回数に変換

---

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: Extend enzyme/mod.rs | 既存モジュールに `#[autodiff]` 追加 | 最小変更、既存テスト活用 | mod.rs 肥大化 | 初期段階向け |
| B: New submodules | forward.rs, reverse.rs, wrappers.rs 分離 | 明確な責務分離、テスト容易 | ファイル数増加 | スケーラブル |
| C: Hybrid (推奨) | Phase 1: extend, Phase 2: refactor | 段階的実装、リスク軽減 | 計画複雑 | Gap analysis 推奨 |

**Selected**: Option C (Hybrid) - 段階的に実装しリスクを軽減

---

## Design Decisions

### Decision: Feature Gate 構造

- **Context**: `enzyme-ad` feature 有効時のみ `#![feature(autodiff)]` を有効化
- **Alternatives Considered**:
  1. 常時 nightly 必須 — stable ビルド不可
  2. cfg_attr で条件付き有効化 — 柔軟性確保
- **Selected Approach**: `#![cfg_attr(feature = "enzyme-ad", feature(autodiff))]`
- **Rationale**: stable Rust との互換性維持、CI で両モードテスト可能
- **Trade-offs**: feature flag 管理が複雑化
- **Follow-up**: CI で enzyme-ad feature 有効/無効両方をテスト

### Decision: Greeks 計算モード切り替え

- **Context**: Enzyme AD vs 有限差分 fallback の切り替え
- **Alternatives Considered**:
  1. Runtime 判定 — 柔軟だが分岐コスト
  2. Compile-time (cfg) — 高速だがバイナリ分離
  3. GreeksMode enum — 既存パターン活用
- **Selected Approach**: `GreeksMode` enum 拡張 + cfg で実装切り替え
- **Rationale**: 既存 API との互換性維持、ユーザー選択可能
- **Trade-offs**: 両実装のメンテナンスコスト

### Decision: Adjoint 集約戦略

- **Context**: 並列 MC シミュレーションでの勾配集約
- **Alternatives Considered**:
  1. Mutex 共有 — シンプルだが競合
  2. Thread-local + reduce — 競合なし
  3. Channel ベース — 非同期対応
- **Selected Approach**: Thread-local + rayon reduce
- **Rationale**: 既存 `ThreadLocalWorkspacePool` パターンと整合
- **Trade-offs**: メモリ使用量増加（スレッド数分）

### Decision: Gamma（二次微分）計算戦略

- **Context**: Gamma (∂²V/∂S²) は二次微分であり、Enzyme の標準 AD は一次微分のみ提供
- **Sources Consulted**:
  - [Enzyme.jl - Forward-over-Reverse](https://enzymead.github.io/Enzyme.jl/stable/generated/autodiff/)
  - Enzyme LLVM documentation
- **Alternatives Considered**:
  1. Nested AD (Forward-over-Reverse) — Delta 関数に Forward mode を適用
  2. Delta への有限差分 — シンプルだが精度低下
  3. num-dual での二次微分 — 別ライブラリ依存
- **Selected Approach**: Nested AD (Forward-over-Reverse) を優先、fallback として Delta への有限差分
- **Rationale**:
  - Enzyme は Forward-over-Reverse パターンをサポート
  - Rust macro ベースでの nested AD は実験的だが、内部関数として実装可能
  - fallback により nested AD 未対応時も機能保証
- **Implementation Pattern**:

  ```rust
  // Inner function: Reverse mode で Delta を計算
  #[autodiff_reverse(d_delta, Active, Duplicated, ...)]
  fn price_for_delta(spot: &f64, ...) -> f64;

  // Outer function: Forward mode で Delta の微分（= Gamma）を計算
  #[autodiff_forward(d_gamma, Dual, ...)]
  fn compute_delta(spot: f64, ...) -> f64 {
      // call d_delta internally
  }
  ```

- **Trade-offs**:
  - 複雑性増加（nested 関数構造）
  - Rust nightly での nested AD サポートが不安定な可能性
- **Follow-up**:
  - Phase 1 で nested AD を試行
  - 失敗時は有限差分 fallback を自動適用

### Decision: チェックポイント戦略

- **Context**: path-dependent オプション（Asian, Lookback等）での AD メモリ管理
- **Alternatives Considered**:
  1. Enzyme 内部自動チェックポイント — Enzyme が自動的に再計算ポイントを決定
  2. 既存 `checkpoint/` モジュール活用 — 手動セグメント管理
  3. Hybrid — Enzyme 自動 + カスタム制御
- **Selected Approach**: Enzyme 内部自動チェックポイント機能を使用
- **Rationale**:
  - Enzyme は LLVM レベルで最適なチェックポイント位置を自動決定
  - 手動管理の複雑性を回避
  - 既存 `checkpoint/` モジュールは Enzyme 外部の用途（状態保存等）に限定
- **Implementation**:
  - `#[autodiff_reverse]` マクロは Enzyme の自動チェックポイントを使用
  - path 長が大きい場合は Enzyme の `recompute` ヒントを検討
- **Trade-offs**:
  - Enzyme の自動決定に依存（最適でない可能性）
  - 既存 `checkpoint/` との役割重複を整理必要
- **Follow-up**:
  - path 1000+ での メモリ使用量をベンチマーク
  - 必要に応じて手動ヒント追加

---

## Risks & Mitigations

- **Enzyme API 変更リスク** — CI で nightly ビルド監視、fallback 実装維持
- **パフォーマンス未達** — ベンチマーク駆動開発、段階的最適化
- **LLVM 18 環境依存** — Docker 環境提供、CI での自動検証
- **チェックポイント統合複雑化** — Phase 2 で段階的に統合

---

## References

- [The Rust Unstable Book - autodiff](https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/autodiff.html)
- [std::intrinsics::autodiff](https://doc.rust-lang.org/nightly/std/intrinsics/fn.autodiff.html)
- [Enzyme Rust Motivation](https://enzyme.mit.edu/rust/motivation.html)
- [rust-lang/rust PR #129458 - Autodiff Upstreaming](https://github.com/rust-lang/rust/pull/129458)
- [GSoC 2025 - Enzyme AutoDiff Toolchain](https://blog.karanjanthe.me/posts/enzyme-autodiff-rust-gsoc/)
- [Enzyme.jl - Forward-over-Reverse (Nested AD)](https://enzymead.github.io/Enzyme.jl/stable/generated/autodiff/) — Gamma/Hessian 計算パターン
