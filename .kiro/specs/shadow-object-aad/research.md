# Research & Design Decisions: shadow-object-aad

---
**Purpose**: Enzyme Rust スライス対応と Shadow Object パターンの技術調査結果
**Generated**: 2026-01-26
---

## Summary

- **Feature**: `shadow-object-aad`
- **Discovery Scope**: Complex Integration (新規 AAD パターン + 既存 enzyme モジュール統合)
- **Key Findings**:
  1. **Enzyme Rust スライス対応済み**: PR #144197 (2025年9月) で TypeTree にスライス/配列サポート追加
  2. **既存 `#[autodiff]` パターン**: `Duplicated` モードで shadow バッファを自動生成
  3. **`#[no_mangle]` 不要**: Rust `#[autodiff]` マクロは FFI 経由ではなく LLVM レベルで動作

## Research Log

### Enzyme Rust TypeTree スライスサポート

- **Context**: 要件 2.1-2.6 のスライスベースカーネルが `#[autodiff]` マクロで動作するか調査
- **Sources Consulted**:
  - [rust-lang/rust PR #144197](https://github.com/rust-lang/rust/pull/144197) - TypeTree support for arrays/slices
  - [GSoC 2025 Final Report](https://sa4dus.github.io/posts/gsoc2025-final-report/) - Enzyme Rust 安定化作業
  - [Rust Unstable Book - autodiff](https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/autodiff.html)
- **Findings**:
  - 配列は特殊オフセット `-1` を使用（「全位置に適用」を意味）
  - スライス `&[f64]` は TypeTree で正しく処理される
  - `Duplicated` モードでスライスに対応可能
- **Implications**: 要件 2.5 の `#[no_mangle]` は不要。`#[autodiff]` マクロで直接スライス対応可能

### 既存 Enzyme 統合パターン分析

- **Context**: 既存 pricer_risk::enzyme::wrappers.rs のパターンを拡張対象として分析
- **Sources Consulted**: `crates/pricer_risk/src/enzyme/wrappers.rs`
- **Findings**:
  ```rust
  // 現行パターン（スカラー）
  #[autodiff(d_price_all, Reverse, Duplicated, Const, Duplicated, Duplicated, Duplicated, Active)]
  pub fn price_european_call_adjoint(spot: f64, ...) -> f64

  // 拡張パターン（スライス）
  #[autodiff(d_kernel, Reverse, Duplicated, Const, Duplicated, Active)]
  pub fn pricing_kernel(rates: &[f64], times: &[f64], output: &mut f64) -> ()
  ```
  - `Duplicated` は shadow スライス `d_rates: &mut [f64]` を自動生成
  - `Const` は勾配を計算しない（times は定数）
  - `Active` は出力のシード値
- **Implications**: 既存パターンをスライスに拡張可能。API 互換性維持

### Activity モード仕様

- **Context**: 要件 3.6, 3.7 の `ENZYME_DUP`/`ENZYME_CONST` フラグの実装方法
- **Sources Consulted**: wrappers.rs, Enzyme Rust documentation
- **Findings**:
  | Activity | Rust マクロ名 | 用途 |
  |----------|--------------|------|
  | `Const` | `Const` | 定数（微分対象外）|
  | `Dual` | `Dual` | Forward mode tangent |
  | `Active` | `Active` | Reverse mode スカラー出力 |
  | `Duplicated` | `Duplicated` | Reverse mode shadow バッファ |
  | `DuplicatedOnly` | `DuplicatedOnly` | shadow のみ（primal なし）|
- **Implications**: 要件で言及される `ENZYME_DUP`/`ENZYME_CONST` は Rust マクロの `Duplicated`/`Const` に対応

### Shadow Trait 設計調査

- **Context**: 要件 1.1-1.5 の Shadow trait 実装方針
- **Sources Consulted**: Rust Clone trait, mem::take patterns
- **Findings**:
  - `Clone` bound で構造体複製可能
  - `zero_out()` は各 `f64` を 0.0 に、`Vec<f64>` を `.fill(0.0)` で初期化
  - ネスト構造は再帰的に `zero_out()` を呼び出し
  - `create_shadow()` は `clone()` + `zero_out()` の組み合わせ
- **Implications**: derive マクロまたは手動実装で対応。proc-macro は複雑すぎるため手動実装を推奨

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **Hybrid Modules** | shadow.rs + kernel.rs + binder.rs を enzyme/ 内に追加 | クリーンな分離、独立テスト可能 | ファイル数増加 | **採用** |
| Extend Existing | mod.rs/wrappers.rs に直接追加 | 既存 API 統一 | モジュール肥大化 | 却下 |
| Separate Crate | 新規 `pricer_shadow` クレート | 完全分離 | 依存関係複雑化、オーバーキル | 却下 |

## Design Decisions

### Decision: Shadow Trait 配置

- **Context**: Shadow trait を pricer_models (L2) vs pricer_risk (L4) のどちらに配置するか
- **Alternatives Considered**:
  1. pricer_models — マーケット構造体と同じ層
  2. pricer_risk::enzyme — Enzyme 機能と統合
- **Selected Approach**: `pricer_risk::enzyme::shadow` に配置
- **Rationale**:
  - Enzyme は L4 機能であり、Shadow は Enzyme 専用抽象化
  - L2 に Enzyme 関連コードを入れると依存方向が逆転
  - orphan rule 回避（trait と impl が同一クレート）
- **Trade-offs**: マーケット構造体の Shadow impl は pricer_risk 側で定義（ボイラープレート増加）
- **Follow-up**: proc-macro derive 検討（将来的な改善）

### Decision: カーネル関数シグネチャ

- **Context**: スライスベースカーネルの引数設計
- **Alternatives Considered**:
  1. `fn kernel(rates: &[f64], times: &[f64], ...) -> f64` — 戻り値で結果
  2. `fn kernel(rates: &[f64], ..., output: &mut f64)` — 参照渡しで結果
- **Selected Approach**: `fn kernel(..., output: &mut f64)` 参照渡し
- **Rationale**:
  - Enzyme Reverse mode は `Active` 戻り値にシードを設定
  - 出力参照の方が Enzyme の `Duplicated` パターンと整合
  - 副作用明示（純粋関数より AD 互換性優先）
- **Trade-offs**: Rust idiom からは外れる（通常は戻り値使用）
- **Follow-up**: 両パターンのベンチマーク実施

### Decision: `#[no_mangle]` 要件の緩和

- **Context**: 要件 2.5 で `#[no_mangle]` を要求しているが必要か
- **Alternatives Considered**:
  1. `#[no_mangle]` を必須とする
  2. `#[autodiff]` マクロのみで対応
- **Selected Approach**: `#[no_mangle]` は**不要**
- **Rationale**:
  - `#[autodiff]` は LLVM 内部で動作し、シンボル名は関係ない
  - Rust コンパイラが直接 LLVM IR を生成するため FFI 境界は存在しない
  - GSoC 2025 の作業で TypeTree がネイティブ対応
- **Trade-offs**: 要件 2.5 を設計フェーズで修正（仕様変更）
- **Follow-up**: 要件ドキュメントの更新を推奨

## Risks & Mitigations

- **Enzyme スライス対応のエッジケース** — 空スライス、非連続メモリへの対応検証 → 単体テストで網羅
- **パフォーマンス劣化** — `clone()` + `zero_out()` のオーバーヘッド → ベンチマーク早期導入
- **Nightly 依存** — `#[autodiff]` は nightly 専用 → feature flag で分離、stable fallback 維持

## References

- [rust-lang/rust PR #144197 - TypeTree support](https://github.com/rust-lang/rust/pull/144197) — スライス対応の実装詳細
- [GSoC 2025 Final Report](https://sa4dus.github.io/posts/gsoc2025-final-report/) — Enzyme Rust 安定化状況
- [Enzyme Rust Motivation](https://enzyme.mit.edu/rust/motivation.html) — Enzyme Rust 概要
- [Rust Unstable Book - autodiff](https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/autodiff.html) — コンパイラフラグ仕様
