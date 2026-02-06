# Research & Design Decisions: mc-memory-layout-optimisation

---
**Purpose**: モンテカルロ・シミュレーションのメモリレイアウト最適化に関するディスカバリ調査結果と設計判断の記録。

---

## Summary

- **Feature**: `mc-memory-layout-optimisation`
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  1. `aligned-vec` クレートにより64バイトアラインメントが実現可能
  2. 既存の `PathObserver` パターンがストリーミング統計の基盤として活用可能
  3. `T: Float` ジェネリクスを維持しつつ、特殊化によりSIMD最適化が可能

## Research Log

### SIMD アラインメント実現方法

- **Context**: 要件3（ベクトル化対応）で64バイトアラインメントが必要
- **Sources Consulted**:
  - [aligned-vec crate](https://docs.rs/aligned-vec/latest/aligned_vec/) — ランタイム/コンパイル時アラインメントをサポート
  - [Rust SIMD Tutorial 2025](https://medium.com/@bartekwinter3/rust-simd-a-tutorial-bb9826f98e81) — AVX-512 zmm レジスタ（512ビット）
  - [State of SIMD in Rust 2025](https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d) — std::simd vs wide vs pulp
  - [Rust Forum: 64 byte alignment](https://users.rust-lang.org/t/easy-way-to-allocate-vec-with-64-byte-alignment/95696) — キャッシュライン境界
- **Findings**:
  - `aligned-vec` クレートが最も簡潔な解決策
  - `AVec<T, A>` 型でコンパイル時アラインメント指定可能
  - `std::alloc::Layout` による手動実装も可能だが複雑
  - AVX-512 は 64バイト（512ビット）、AVX2 は 32バイト（256ビット）
- **Implications**:
  - `aligned-vec` を optional dependency として追加
  - `AlignedPathBuffer<T, const ALIGN: usize>` 型を設計
  - デフォルトは64バイト、設定で変更可能

### 既存 PathWorkspace 分析

- **Context**: 拡張ポイントと後方互換性の確認
- **Sources Consulted**:
  - `crates/pricer_pricing/src/mc/workspace.rs` — 現行実装
  - `crates/pricer_pricing/src/mc/paths.rs` — パス生成ループ
- **Findings**:
  - Path First レイアウト: `index = path_idx * (n_steps + 1) + step_idx`
  - capacity/size 分離パターンで再利用をサポート
  - `get_path_slice`, `get_path_slice_mut` がパス単位アクセスを提供
  - `randoms`, `paths`, `payoffs` の3バッファ構成
- **Implications**:
  - 新規 `TimeStepFirstWorkspace` を並行導入（PathWorkspace は変更しない）
  - 共通トレイト `PathWorkspaceTrait` でポリモーフィズム実現
  - MonteCarloPricer にレイアウト選択オプション追加

### PathObserver ストリーミングパターン

- **Context**: ストリーミングエンジンの基盤設計
- **Sources Consulted**:
  - `crates/pricer_pricing/src/path_dependent/observer.rs` — 既存実装
- **Findings**:
  - `observe(price: T)` でインクリメンタル統計更新
  - `running_sum`, `running_max`, `running_min` パターン
  - フルパス保存不要（メモリ効率的）
  - `snapshot()`, `restore()` でチェックポイント対応
- **Implications**:
  - StreamingEngine は PathObserver を内部利用
  - 各パスに対して PathObserver インスタンスを並列管理
  - ストリーミング処理と PathObserver の自然な統合

### Enzyme AD 互換性

- **Context**: Time Step First レイアウトでのAD動作確認
- **Sources Consulted**:
  - `.kiro/steering/tech.md` — Enzyme 要件
- **Findings**:
  - Enzyme は静的ディスパッチ（enum）を要求
  - `smooth_max`, `smooth_indicator` で分岐排除
  - 固定サイズ `for` ループ推奨
  - `T: Float` ジェネリクスは維持可能
- **Implications**:
  - Time Step First でも固定ステップ数ループなら互換
  - ストリーミング処理も固定ステップ数なら問題なし
  - レイアウト変更は AD に影響しない（データ順序のみ変更）

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: PathWorkspace 拡張 | 既存構造体にレイアウトモード追加 | 最小変更、既存インフラ活用 | 複雑化、ランタイムオーバーヘッド | 却下 |
| B: 新規コンポーネント | TimeStepFirstWorkspace + StreamingEngine 新規作成 | 責任分離、テスト容易 | ファイル増加、インターフェース設計 | 候補 |
| **C: ハイブリッド** | Config拡張 + 新規コンポーネント + トレイト抽象化 | 段階的実装、既存API維持、Feature flag制御 | 計画複雑性 | **採用** |

## Design Decisions

### Decision: ハイブリッドアプローチ採用

- **Context**: 後方互換性維持しつつ新レイアウト導入
- **Alternatives Considered**:
  1. PathWorkspace 直接変更 — 既存コード破壊リスク
  2. 完全新規実装 — 既存パターン無視
- **Selected Approach**: Config拡張 + 新規Workspace + トレイト抽象化
- **Rationale**:
  - 既存API（MonteCarloPricer）を変更せず維持
  - 新機能はオプトイン（デフォルトは従来動作）
  - Feature flag でコンパイル時制御可能
- **Trade-offs**:
  - ✅ 後方互換性完全維持
  - ✅ 段階的マイグレーション可能
  - ❌ コードベース拡大
- **Follow-up**: PathWorkspaceTrait の設計詳細を design.md で確定

### Decision: aligned-vec クレート採用

- **Context**: 64バイトアラインメント実現
- **Alternatives Considered**:
  1. `std::alloc::Layout` 手動実装 — 複雑、unsafe 必要
  2. `simd_aligned` クレート — API が限定的
- **Selected Approach**: `aligned-vec` クレート（optional dependency）
- **Rationale**:
  - コンパイル時/ランタイム両方のアラインメントサポート
  - `AVec<T, A>` でジェネリック対応
  - 既存コードへの影響最小
- **Trade-offs**:
  - ✅ 簡潔な実装
  - ✅ メンテナンスされているクレート
  - ❌ 外部依存追加（optional）
- **Follow-up**: Feature flag `simd-aligned` で制御

### Decision: ダブルバッファリング戦略

- **Context**: ストリーミング処理のメモリ管理
- **Alternatives Considered**:
  1. シングルバッファ + コピー — 単純だがコピーコスト
  2. リングバッファ — 複雑
- **Selected Approach**: ダブルバッファ（current/previous）をスワップ
- **Rationale**:
  - GBM パス生成は前ステップのみ参照
  - スワップはポインタ交換のみ（O(1)）
  - PathObserver と自然に統合
- **Trade-offs**:
  - ✅ メモリ O(2 * num_paths) のみ
  - ✅ ゼロコピー更新
  - ❌ 全履歴アクセス不可（ストリーミングモードの制約）
- **Follow-up**: 全履歴必要なペイオフは従来モード使用を推奨

## Risks & Mitigations

| Risk | Level | Mitigation |
|------|-------|------------|
| SIMD アラインメントの性能効果不確実 | 中 | 早期ベンチマーク（Criterion）で検証 |
| Enzyme AD との互換性問題 | 中 | 既存テストスイートで継続検証 |
| 性能目標（メモリ90%削減）未達 | 中 | ストリーミングモードは理論的に達成可能 |
| API 複雑化によるユーザビリティ低下 | 低 | デフォルト設定で従来動作、ビルダーパターン |

## References

- [aligned-vec crate docs](https://docs.rs/aligned-vec/latest/aligned_vec/) — アラインメント実装の参考
- [State of SIMD in Rust 2025](https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d) — SIMD 現状分析
- [Rust Forum: 64 byte alignment](https://users.rust-lang.org/t/easy-way-to-allocate-vec-with-64-byte-alignment/95696) — コミュニティ議論
- `.kiro/steering/tech.md` — プロジェクト技術標準
- `crates/pricer_pricing/src/mc/` — 既存 MC 実装
