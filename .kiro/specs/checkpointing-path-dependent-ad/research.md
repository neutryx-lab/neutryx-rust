# Research & Design Decisions: checkpointing-path-dependent-ad

---
**Purpose**: Enzyme ADチェックポイント、パス依存オプション、L1/L2統合に関する調査結果とアーキテクチャ決定の記録
**Discovery Date**: 2026-01-01
---

## Summary

- **Feature**: checkpointing-path-dependent-ad
- **Discovery Scope**: Complex Integration（L3とL1/L2の統合、新規チェックポイント機構、パス依存オプション拡張）
- **Key Findings**:
  - Enzyme ADはLLVMエイリアス解析を使用してキャッシュ vs 再計算のトレードオフを決定
  - 幾何平均アジアンオプションには閉形式解が存在（σ_G = σ/√3）
  - smooth_indicator関数によるバリア条件のスムージングが有効

## Research Log

### Enzyme Checkpointing Best Practices

- **Context**: 長いモンテカルロパスでの逆方向ADのメモリ使用量削減
- **Sources Consulted**:
  - [Enzyme AD GitHub](https://github.com/EnzymeAD/Enzyme)
  - [Enzyme MIT](https://enzyme.mit.edu/)
  - [Reverse-Mode AD and GPU Kernels](https://c.wsmoses.com/papers/EnzymeGPU.pdf)
- **Findings**:
  - Enzymeはforward passとreverse passの2パートで勾配を計算
  - エイリアス解析により、値が再計算可能か、キャッシュ必要かを判定
  - メモリアクセスしない命令は再計算可能
  - グローバルメモリ使用 vs レジスタ/共有メモリのトレードオフ
  - Rust Enzyme統合は2025年にnightlyコンパイラに追加
- **Implications**:
  - チェックポイント戦略はEnzymeの再計算ヒューリスティックと連携すべき
  - 明示的なチェックポイントにより、ユーザー制御可能なメモリ/計算トレードオフを提供
  - 状態保存は単純なコピー操作（Enzymeはメモリ操作を追跡）

### Smooth Barrier Approximation

- **Context**: バリアオプションのペイオフ関数を微分可能にする
- **Sources Consulted**:
  - [Smoothing Methods for AD Across Conditional Branches](https://arxiv.org/html/2310.03585v2)
  - [Sigmoid Functions for Smooth Approximation](https://www.researchgate.net/publication/348131737_Sigmoid_functions_for_the_smooth_approximation_to_the_absolute_value_function)
- **Findings**:
  - 条件分岐は不連続性を導入し、ADの課題となる
  - シグモイド関数による平滑解釈（Smooth Interpretation）が有効
  - `smooth_indicator(x, ε) = 1/(1 + exp(-x/ε))` がヘビサイド関数の近似
  - DiscoGradツールがC++プログラムの自動スムージングを提供
- **Implications**:
  - 既存の`pricer_core::math::smoothing::smooth_indicator`を活用
  - バリア条件: `smooth_indicator(S - B, ε)` でUp barrier、`smooth_indicator(B - S, ε)` でDown barrier
  - ε→0で真のバリア条件に収束

### Geometric Asian Option Analytical Formula

- **Context**: 幾何平均アジアンオプションの解析解による検証
- **Sources Consulted**:
  - [Asian Option - Wikipedia](https://en.wikipedia.org/wiki/Asian_option)
  - [NTU Asian Option Lecture](http://homepage.ntu.edu.tw/~jryanwang/courses/Financial%20Computation%20or%20Financial%20Engineering%20(graduate%20level)/FE_Ch10%20Asian%20Option.pdf)
  - [Pricing and Hedging Asian Options - USU](https://digitalcommons.usu.edu/cgi/viewcontent.cgi?article=1315&context=gradreports)
- **Findings**:
  - 幾何平均は対数正規分布を保持（Black-Scholes仮定を満たす）
  - 閉形式解:
    ```
    C_G = S₀ × e^{(b-r)T} × Φ(d₁) - K × e^{-rT} × Φ(d₂)
    where:
      σ_G = σ / √3
      b = ½(r - σ²/6)
      d₁ = [ln(S₀/K) + (b + σ_G²/2)T] / (σ_G × √T)
      d₂ = d₁ - σ_G × √T
    ```
  - 算術平均アジアンには閉形式解が存在しない
  - 幾何平均をコントロール変量として算術平均の分散削減に使用可能
- **Implications**:
  - 幾何平均アジアンのMC価格を解析解と比較してテスト
  - 算術平均アジアンの検証には収束テストを使用
  - Kemna-Vorst近似も参考可能

### Barrier Option Analytical Formula

- **Context**: バリアオプションの解析解による検証
- **Sources Consulted**:
  - 標準的なBlack-Scholesバリア公式（Rebonato, Merton）
- **Findings**:
  - 連続監視バリアには閉形式解が存在
  - 8種類: Up-In/Out Call/Put, Down-In/Out Call/Put
  - 離散監視バリアには連続監視への調整が必要
  - Rubinstein-Reiner公式が標準
- **Implications**:
  - 解析解実装を`pricer_models::analytical::barriers`に追加
  - 連続監視バリアでMC結果を検証
  - 離散監視には別途調整係数が必要

### L1/L2統合パターン

- **Context**: pricer_kernel（L3）とpricer_core/pricer_models（L1/L2）の統合
- **Sources Consulted**: 既存コードベース分析
- **Findings**:
  - L3は現在完全分離（pricer_*依存ゼロ）
  - L1提供: `Float` trait、smoothing関数、`YieldCurve` trait
  - L2提供: `StochasticModel` trait、`Instrument` enum、analytical module
  - Cargo.toml依存追加 + import変更で統合可能
  - payoff.rs内のローカル`soft_plus`は`pricer_core::math::smoothing::smooth_max`で置換可能
- **Implications**:
  - Phase 4.1でL1/L2統合を最優先
  - 既存APIの変更は最小限（型パラメータのFloat制約追加程度）
  - nightly Rustでのpricer_core/pricer_models互換性確認が必要

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: 既存拡張 | PathWorkspace/payoff.rs拡張 | 開発速度、ファイル数抑制 | 責務肥大化 | 短期的には有効だが長期保守性に懸念 |
| B: 新規モジュール | checkpoint/, path_dependent/新設 | 責務分離、テスト容易性 | 初期設計コスト | 長期的に推奨 |
| **C: ハイブリッド** | L1/L2統合は既存変更、checkpoint/path_dependentは新規 | リスク分散、段階的検証 | 計画複雑性 | **推奨アプローチ** |

## Design Decisions

### Decision: ハイブリッドアプローチの採用

- **Context**: 既存コードへの影響最小化と新機能の責務分離を両立
- **Alternatives Considered**:
  1. Option A: 全て既存ファイルに拡張 — 責務肥大化リスク
  2. Option B: 全て新規モジュール — 初期設計コスト高
- **Selected Approach**: Option C ハイブリッド
  - L1/L2統合: Cargo.toml + import変更（既存拡張）
  - チェックポイント: 新規`checkpoint/`モジュール
  - パス依存オプション: 新規trait + payoff.rsからの関数移行
- **Rationale**: 段階的実装によりリスク分散、各フェーズで動作確認可能
- **Trade-offs**: 計画が複雑になるが、各マイルストーンで品質確認可能
- **Follow-up**: Phase 4.1完了後にL1/L2統合の成功を確認してから次フェーズへ

### Decision: PathDependentPayoff traitの設計

- **Context**: パス依存オプション（Asian/Barrier/Lookback）の統一インターフェース
- **Alternatives Considered**:
  1. 個別関数 — 拡張性低、パターン重複
  2. enum dispatch — 既存Instrument enumと同様
  3. trait + enum wrapper — 柔軟性と静的ディスパッチの両立
- **Selected Approach**: trait + enum wrapper
  ```rust
  pub trait PathDependentPayoff<T: Float> {
      fn compute(&self, path: &[T]) -> T;
      fn observe(&mut self, step: usize, price: T);
      fn reset(&mut self);
  }

  pub enum PathPayoffType<T: Float> {
      AsianArithmetic(AsianArithmeticParams<T>),
      AsianGeometric(AsianGeometricParams<T>),
      BarrierUpIn(BarrierParams<T>),
      // ...
  }
  ```
- **Rationale**: Enzyme互換の静的ディスパッチ維持、将来の拡張容易
- **Trade-offs**: enum拡張時に網羅的match必要
- **Follow-up**: 全オプションタイプの実装後にインターフェース最終化

### Decision: CheckpointStrategyの選択

- **Context**: チェックポイント間隔の決定戦略
- **Alternatives Considered**:
  1. 均等間隔 — シンプル、予測可能
  2. 対数間隔 — 最適なメモリ/計算トレードオフ（Griewank理論）
  3. 適応的 — ランタイム調整
- **Selected Approach**: 均等間隔をデフォルト、対数間隔をオプション
- **Rationale**: 均等間隔が実装最シンプル、多くのケースで十分
- **Trade-offs**: 最適メモリ効率ではないが、実装/デバッグ容易
- **Follow-up**: ベンチマークで対数間隔の優位性を確認後、デフォルト変更検討

### Decision: PathObserverのストリーミング設計

- **Context**: 経路統計（max, min, avg）の蓄積方法
- **Alternatives Considered**:
  1. バッチ処理 — 全パス保存後に計算
  2. ストリーミング — 各ステップで逐次更新
- **Selected Approach**: ストリーミング
  ```rust
  struct PathObserver {
      running_sum: f64,
      running_max: f64,
      running_min: f64,
      count: usize,
  }
  ```
- **Rationale**: メモリ効率、チェックポイント状態保存が容易
- **Trade-offs**: 一部の統計（中央値など）は計算不可
- **Follow-up**: 必要に応じて特殊統計用のフルパス保存モードを追加

## Risks & Mitigations

- **Enzyme + チェックポイント相互作用（高リスク）**
  - 緩和策: Phase 4.3開始前にプロトタイプ検証、Enzymeコミュニティへの問い合わせ

- **nightly Rust互換性（中リスク）**
  - 緩和策: `rust-toolchain.toml`でnightly-2025-01-15固定済み

- **L1/L2統合コンパイルエラー（低リスク）**
  - 緩和策: 段階的統合、CI確認

- **パフォーマンス目標未達（中リスク）**
  - 緩和策: 早期ベンチマーク、チェックポイント間隔の動的調整

## References

- [Enzyme AD GitHub](https://github.com/EnzymeAD/Enzyme) — Enzyme公式リポジトリ
- [Enzyme MIT](https://enzyme.mit.edu/) — Enzyme公式サイト
- [Smoothing Methods for AD](https://arxiv.org/html/2310.03585v2) — 条件分岐のスムージング手法
- [Asian Option Wikipedia](https://en.wikipedia.org/wiki/Asian_option) — アジアンオプション概要
- [NTU Asian Option Lecture](http://homepage.ntu.edu.tw/~jryanwang/courses/Financial%20Computation%20or%20Financial%20Engineering%20(graduate%20level)/FE_Ch10%20Asian%20Option.pdf) — 幾何平均解析解の導出

---
_生成日: 2026-01-01_
_分析者: Claude Code_
