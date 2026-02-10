# Research & Design Decisions: mc-memory-layout-optimisation

## Summary
- **Feature**: `mc-memory-layout-optimisation`
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  1. `aligned-vec` クレートにより64バイトアラインメントが実現可能
  2. 既存の `PathObserver` パターンがストリーミング統計の基盤として活用可能
  3. `T: Float` ジェネリクスを維持しつつ、特殊化によりSIMD最適化が可能

## Research Log

### SIMD アラインメント実現方法
- **Findings**:
  - `aligned-vec` クレートが最も簡潔な解決策
  - `AVec<T, A>` 型でコンパイル時アラインメント指定可能
  - AVX-512 は 64バイト（512ビット）、AVX2 は 32バイト（256ビット）
- **Implications**: `aligned-vec` を optional dependency として追加、`AlignedPathBuffer<T, const ALIGN: usize>` 型を設計

### 既存 PathWorkspace 分析
- **Findings**:
  - Path First レイアウト: `index = path_idx * (n_steps + 1) + step_idx`
  - capacity/size 分離パターンで再利用をサポート
- **Implications**: 新規 `TimeStepFirstWorkspace` を並行導入、共通トレイト `PathWorkspaceTrait` でポリモーフィズム実現

### PathObserver ストリーミングパターン
- **Findings**:
  - `observe(price: T)` でインクリメンタル統計更新
  - `running_sum`, `running_max`, `running_min` パターン
  - フルパス保存不要（メモリ効率的）
- **Implications**: StreamingEngine は PathObserver を内部利用

### Enzyme AD 互換性
- **Findings**:
  - Enzyme は静的ディスパッチ（enum）を要求
  - `smooth_max`, `smooth_indicator` で分岐排除
  - `T: Float` ジェネリクスは維持可能
- **Implications**: Time Step First でも固定ステップ数ループなら互換

## Design Decisions

### Decision: ハイブリッドアプローチ採用
- **Selected Approach**: Config拡張 + 新規Workspace + トレイト抽象化
- **Rationale**: 既存API（MonteCarloPricer）を変更せず維持、新機能はオプトイン、Feature flag でコンパイル時制御可能

### Decision: aligned-vec クレート採用
- **Selected Approach**: `aligned-vec` クレート（optional dependency）
- **Rationale**: コンパイル時/ランタイム両方のアラインメントサポート、既存コードへの影響最小

### Decision: ダブルバッファリング戦略
- **Selected Approach**: ダブルバッファ（current/previous）をスワップ
- **Rationale**: GBM パス生成は前ステップのみ参照、スワップはポインタ交換のみ（O(1)）、PathObserver と自然に統合

## Risks & Mitigations
- **Risk 1: SIMD アラインメントの性能効果不確実** — Mitigation: 早期ベンチマーク（Criterion）で検証
- **Risk 2: Enzyme AD との互換性問題** — Mitigation: 既存テストスイートで継続検証
- **Risk 3: 性能目標（メモリ90%削減）未達** — Mitigation: ストリーミングモードは理論的に達成可能
