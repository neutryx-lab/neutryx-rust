# Research & Design Decisions: performance-optimization-2

---
**Purpose**: パフォーマンス改善機能のディスカバリー結果と設計決定の根拠を記録する。

---

## Summary

- **Feature**: `performance-optimization-2`
- **Discovery Scope**: Complex Integration（既存システムの拡張 + 新規コンポーネント）
- **Key Findings**:
  1. SIMD: `std::simd`はnightly専用。stable向けには`wide`クレートが推奨
  2. Enzyme/autodiff: 2025年後半にnightlyで利用可能になり、Linux/macOSでサポート
  3. ベンチマークCI: Bencher + criterion-compare-actionでリグレッション検出可能
  4. PGO: cargo-pgoで簡易化されたワークフローが利用可能

---

## Research Log

### SIMD実装オプション

- **Context**: Requirement 1.4でAVX2/AVX-512ベクトル化を要求
- **Sources Consulted**:
  - [The state of SIMD in Rust in 2025](https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d)
  - [std::simd - Rust](https://doc.rust-lang.org/std/simd/index.html)
  - [rust-lang/portable-simd](https://github.com/rust-lang/portable-simd)
- **Findings**:
  - `std::simd`は2025年時点でもnightly専用（安定化未定）
  - stableでは`wide`クレートが推奨（同等のAPIで安定）
  - `pulp`はランタイムディスパッチ内蔵で高レベル抽象化を提供
  - L3（pricer_kernel）は既にnightlyのため`std::simd`使用可能
- **Implications**:
  - L3: `std::simd`を使用（nightly環境）
  - フォールバック: スカラー実装を常に維持

### Enzyme autodiff統合

- **Context**: Requirement 2でEnzyme ADの本格統合を要求
- **Sources Consulted**:
  - [Rust Blog - Project goals update November 2025](https://blog.rust-lang.org/2025/12/16/Project-Goals-2025-November-Update.md/)
  - [autodiff - The Rust Unstable Book](https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/autodiff.html)
  - [GSoC 2025 - Making Enzyme AutoDiff Stable](https://blog.karanjanthe.me/posts/enzyme-autodiff-rust-gsoc/)
- **Findings**:
  - `#[autodiff_forward]`/`#[autodiff_reverse]`マクロがnightlyで利用可能
  - `-Zautodiff=Enable`フラグでEnzyme有効化
  - TypeTree実装によりメモリレイアウト記述が改善
  - Linux/macOS dist builderでサポート（Windows未対応）
  - rlib対応によりライブラリクレートでもautodiff使用可能
- **Implications**:
  - `#![feature(autodiff)]`を`pricer_kernel`で有効化
  - CI環境でのEnzyme可用性チェックは既存ワークフローで対応済み
  - num-dualフォールバックは機能フラグで制御

### ベンチマークCI統合

- **Context**: Requirement 6でCIリグレッション検出を要求
- **Sources Consulted**:
  - [Bencher - Continuous Benchmarking](https://bencher.dev/learn/track-in-ci/rust/criterion/)
  - [criterion-compare-action](https://github.com/matchai/criterion-compare-action)
  - [Criterion.rs FAQ](https://bheisler.github.io/criterion.rs/book/faq.html)
- **Findings**:
  - Criterion.rsの公式見解: 仮想化環境（GitHub Actions等）ではノイズが大きい
  - 代替案1: Iaiを使用（Cachegrindベースで命令数計測）
  - 代替案2: Bencherで継続的ベンチマーキング
  - criterion-compare-action: boa-devがメンテナンス継続
  - rust-benchmarkライブラリ: 厳格モード（PERF_COMPARE_STRICT=1）でCI失敗可能
- **Implications**:
  - Iai導入を検討（命令数ベースで再現性向上）
  - Criterionは開発時ローカル計測に使用
  - Bencherまたはcriterion-compare-actionでCI統合

### Profile-Guided Optimization (PGO)

- **Context**: Requirement 5.5でPGOサポートを要求
- **Sources Consulted**:
  - [Profile-guided Optimization - The rustc book](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
  - [cargo-pgo](https://github.com/Kobzol/cargo-pgo)
  - [Rust Performance Book](https://nnethercote.github.io/perf-book/build-configuration.html)
- **Findings**:
  - PGOはstable Rustで利用可能（PGO WGは目標達成で終了）
  - 手動ワークフロー: profile-generate → プロファイル収集 → profile-use
  - cargo-pgoで簡易化（`cargo pgo build` → 実行 → `cargo pgo optimize`）
  - 10%以上のパフォーマンス向上が期待可能
  - 注意: 収集ワークロードと異なる使用パターンでは効果減少
- **Implications**:
  - cargo-pgoを採用（手動ワークフローより使いやすい）
  - ベンチマークスイートをプロファイル収集に使用
  - ドキュメントでPGOビルド手順を提供

---

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **Hybrid Phased** | Phase 1-3の段階的実装 | リスク分散、効果測定可能 | 総期間が長い | **採用** |
| Big Bang | 全要件を一度に実装 | 統合コスト低減 | 高リスク、検証困難 | 却下 |
| Feature Flags Only | 全機能をフラグで制御 | 柔軟なロールアウト | 複雑度増加 | 部分採用 |

---

## Design Decisions

### Decision: SIMD抽象化レイヤー

- **Context**: L3はnightly、SIMD機能が必要
- **Alternatives Considered**:
  1. `std::simd`直接使用（nightly専用）
  2. `wide`クレート（stable互換）
  3. 手動`std::arch`（低レベル）
- **Selected Approach**: `std::simd`をL3で使用、スカラーフォールバック付き
- **Rationale**: L3は既にnightly必須のため追加制約なし
- **Trade-offs**:
  - ✅ 標準ライブラリの一部で将来の安定化が期待
  - ❌ nightly固定が必要
- **Follow-up**: Enzyme互換性をテストで検証

### Decision: Enzyme統合戦略

- **Context**: 現在の有限差分実装をEnzyme ADに置き換え
- **Alternatives Considered**:
  1. 完全Enzyme移行（num-dual廃止）
  2. デュアルモード維持（Enzyme + num-dual）
  3. 段階的移行（フォールバック付き）
- **Selected Approach**: 段階的移行 + 自動フォールバック
- **Rationale**: Enzyme可用性が環境依存のため、フォールバック必須
- **Trade-offs**:
  - ✅ 堅牢性確保
  - ❌ 両方のコードパス維持コスト
- **Follow-up**: CI環境でのEnzyme検出ロジック強化

### Decision: ベンチマークCI戦略

- **Context**: GitHub Actionsでのリグレッション検出
- **Alternatives Considered**:
  1. Criterion + criterion-compare-action
  2. Iai（Cachegrindベース）
  3. Bencher（SaaS）
- **Selected Approach**: Iai + Criterionハイブリッド
- **Rationale**: Iaiは仮想化環境でも再現性が高い、Criterionはローカル詳細分析用
- **Trade-offs**:
  - ✅ CI安定性向上
  - ❌ 2種類のベンチマーク維持
- **Follow-up**: 閾値設定のチューニング

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Enzyme nightly依存で破壊的変更 | バージョン固定 + フォールバック維持 |
| SIMD互換性問題 | スカラーフォールバック + テスト充実 |
| CIベンチマークノイズ | Iai採用で命令数ベース計測 |
| PGOワークロード不一致 | ベンチマークスイートを代表的ワークロードとして設計 |

---

## References

- [The state of SIMD in Rust in 2025](https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d) — SIMD現状分析
- [autodiff - The Rust Unstable Book](https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/autodiff.html) — Enzyme公式ドキュメント
- [Bencher - Continuous Benchmarking](https://bencher.dev/learn/track-in-ci/rust/criterion/) — CIベンチマーク統合
- [cargo-pgo](https://github.com/Kobzol/cargo-pgo) — PGO簡易化ツール
- [Criterion.rs FAQ](https://bheisler.github.io/criterion.rs/book/faq.html) — CI環境での注意事項
