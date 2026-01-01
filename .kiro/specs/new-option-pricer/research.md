# Research & Design Decisions: new-option-pricer

## Summary
- **Feature**: `new-option-pricer`
- **Discovery Scope**: Extension（既存システムへの機能追加）
- **Key Findings**:
  1. Hestonモデルの離散化にはFull Truncationスキームが最適（シンプルさとEnzyme AD互換性のバランス）
  2. Cholesky分解には`nalgebra`クレートの純Rust実装を使用（追加依存なし、十分な性能）
  3. 既存のソルバー/分布関数インフラを最大限活用可能

## Research Log

### Hestonモデルの離散化スキーム
- **Context**: Hestonモデルのモンテカルロシミュレーションで分散プロセスが負になる問題への対処
- **Sources Consulted**:
  - [Simulating from the Heston Model: A Gamma Approximation Scheme](https://dms.umontreal.ca/~bedard/Heston.pdf)
  - [Efficient Simulation of the Heston Stochastic Volatility Model - Leif Andersen](https://www.ressources-actuarielles.net/EXT/ISFA/1226.nsf/0/1826b88b152e65a7c12574b000347c74/$FILE/LeifAndersenHeston.pdf)
  - [Euler and Milstein Discretization](http://www.frouah.com/finance%20notes/Euler%20and%20Milstein%20Discretization.pdf)
- **Findings**:
  - QE（Quadratic Exponential）スキームが精度最高だが、実装複雑性が高い
  - Full Truncationスキームは精度とシンプルさのバランスが良い
  - MilsteinはEulerより負の分散発生が少ない
  - Full Truncation: V(t) < 0 の場合に max(V(t), 0) を使用
- **Implications**:
  - Full Truncationスキームを採用（Enzyme AD互換性を考慮）
  - smooth_max関数で非負性を保証（AD微分可能性維持）

### Cholesky分解の実装方法
- **Context**: バスケットオプションの相関付きブラウン運動生成に必要
- **Sources Consulted**:
  - [nalgebra Cholesky documentation](https://docs.rs/nalgebra/latest/nalgebra/linalg/struct.Cholesky.html)
  - [faer-cholesky benchmarks](https://lib.rs/crates/faer-cholesky)
  - [nalgebra decompositions guide](https://www.nalgebra.rs/docs/user_guide/decompositions_and_lapack/)
- **Findings**:
  - `nalgebra`: 純Rust実装、追加依存なし、512x512で15.6ms
  - `faer`: 高速（512x512で1.1ms）だが追加依存
  - バスケットオプションの相関行列は通常小さい（10x10以下）ため性能差は無視できる
  - `nalgebra::linalg::Cholesky::new()`は正定値でない場合Noneを返す
- **Implications**:
  - `nalgebra`の既存実装を使用（追加依存なし）
  - 正定値チェックはnalgebraのNone戻り値で自動検出

### Black-Scholes公式の実装
- **Context**: 既存のnorm_cdf/norm_pdf関数を活用
- **Sources Consulted**: 既存コードベース調査（gap-analysis.md参照）
- **Findings**:
  - `pricer_models::analytical::distributions`にnorm_cdf/norm_pdfが実装済み
  - Float型ジェネリックでAD互換
  - Abramowitz-Stegun近似で高精度
- **Implications**:
  - 公式実装のみで対応可能
  - 既存パターンに完全準拠

### IVソルバーの実装
- **Context**: 既存のNewtonRaphson/BrentソルバーをIV計算に適用
- **Sources Consulted**: 既存コードベース調査
- **Findings**:
  - `NewtonRaphsonSolver::find_root_ad()`でDual64自動微分サポート
  - `BrentSolver`はブラケット法でロバスト
  - `SolverConfig`で許容誤差・最大反復回数を設定可能
- **Implications**:
  - Newton-Raphson（AD使用）を主ソルバー、Brentをフォールバック
  - 既存エラー型にArbitrageBoundsViolationを追加

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 既存モジュール拡張 | pricer_models/analyticalに追加 | 最小限の変更、既存パターン活用 | モジュール肥大化 | Req 1, 2に適用 |
| 新規モジュール追加 | pricer_kernel/multi_asset新設 | 責務分離、単一資産への影響なし | 設計・統合コスト | Req 4に適用 |
| 静的ディスパッチ維持 | StochasticModelEnumパターン踏襲 | Enzyme互換、ゼロコスト | 列挙型の拡張が必要 | Req 3に適用 |

## Design Decisions

### Decision: Hestonモデルの離散化スキーム
- **Context**: 分散プロセスの負値問題への対処とAD互換性の両立
- **Alternatives Considered**:
  1. Euler-Maruyama + Truncation — シンプルだがバイアス大
  2. Milstein + Full Truncation — バイアス小、実装適度
  3. QE (Quadratic Exponential) — 最高精度だが複雑、AD非互換の可能性
- **Selected Approach**: Milstein + Full Truncation（smooth_max使用）
- **Rationale**: AD互換性を維持しつつ十分な精度を達成。smooth_max関数で非負性を微分可能に保証。
- **Trade-offs**: QEほどの精度は得られないが、実装・保守性で優位
- **Follow-up**: ベンチマークでGBMとの収束比較を実施

### Decision: Cholesky分解の依存ライブラリ
- **Context**: バスケットオプションの相関付きBM生成
- **Alternatives Considered**:
  1. 自前実装 — 依存なし、バグリスク
  2. nalgebra — 純Rust、既に間接依存
  3. faer — 高速だが追加依存
  4. nalgebra-lapack — BLAS依存、ビルド複雑化
- **Selected Approach**: nalgebra（純Rust実装）
- **Rationale**: 小規模行列（通常10x10以下）では性能差は無視できる。追加依存なし。
- **Trade-offs**: 大規模行列では遅いが、ユースケース外
- **Follow-up**: Cargo.tomlにnalgebra依存追加

### Decision: IVソルバーのフォールバック戦略
- **Context**: Newton-Raphsonが収束しない場合の対処
- **Alternatives Considered**:
  1. Newton-Raphsonのみ — シンプルだが失敗時エラー
  2. Newton-Raphson + Brent — 確実性向上
  3. Brentのみ — 常に収束だが遅い
- **Selected Approach**: Newton-Raphson（AD付き）→ Brentフォールバック
- **Rationale**: 大部分のケースはNewton-Raphsonで高速収束。エッジケースのみBrent使用。
- **Trade-offs**: 実装が少し複雑になるが、ロバスト性向上
- **Follow-up**: フォールバック発生頻度をログ記録

## Risks & Mitigations
- **Hestonの数値不安定性** — Full Truncationとsmooth_maxで緩和。Feller条件バリデーション追加。
- **バスケットオプションの相関行列検証** — nalgebraのCholesky::new()がNone返却で検出。カスタムエラー型追加。
- **AD互換性の破壊** — すべての新実装でFloat型ジェネリックを使用。Enzyme ADテストを追加。

## References
- [Leif Andersen - Efficient Simulation of the Heston Model](https://www.ressources-actuarielles.net/EXT/ISFA/1226.nsf/0/1826b88b152e65a7c12574b000347c74/$FILE/LeifAndersenHeston.pdf) — QE/TGスキームの原論文
- [nalgebra Cholesky](https://docs.rs/nalgebra/latest/nalgebra/linalg/struct.Cholesky.html) — Rust Cholesky実装のドキュメント
- [Black-Scholes Formula](https://en.wikipedia.org/wiki/Black%E2%80%93Scholes_model) — BS公式の参照
