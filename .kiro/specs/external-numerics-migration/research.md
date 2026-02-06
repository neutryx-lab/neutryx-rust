# Research & Design Decisions

## Summary
- **Feature**: `external-numerics-migration`
- **Discovery Scope**: Complex Integration
- **Key Findings**:
  - `argmin` (0.10) と `argmin-math` (0.4) は既にワークスペース依存関係に存在し、nalgebra サポートが設定済み
  - `roots` クレートは Brent、Newton-Raphson を提供するが、f32/f64 の FloatType trait 経由のジェネリックサポートのみ（Dual64 非対応）
  - `faer` は中〜大規模行列向けに最適化され、nalgebra より高性能だが AD 型サポートが限定的

## Research Log

### argmin クレートの評価

- **Context**: 最適化アルゴリズム（Nelder-Mead、L-BFGS）の外部クレート移行先として評価
- **Sources Consulted**:
  - [argmin 公式ドキュメント](https://docs.rs/argmin/latest/argmin/)
  - [argmin GitHub](https://github.com/argmin-rs/argmin)
  - [argmin book](https://www.argmin-rs.org/book/)
- **Findings**:
  - Nelder-Mead: `argmin::solver::neldermead::NelderMead` で提供
  - L-BFGS: `argmin::solver::quasinewton::LBFGS` と More-Thuente line search で提供
  - Executor パターン: `Solver` trait 実装で checkpointing、observers 機能を活用可能
  - 型非依存設計: nalgebra、ndarray、カスタムバックエンドをサポート
  - `CostFunction` trait: 目的関数の定義に使用（`cost()` メソッド）
  - `Gradient` trait: 勾配計算に使用（`gradient()` メソッド）
  - 数値勾配: `finitediff` feature で有限差分近似を提供
- **Implications**:
  - 既存の `minimize_nelder_mead`、`minimize_lbfgs` API を薄いラッパーとして維持可能
  - `OptimisationResult` を argmin の `IterState` から変換するアダプターが必要
  - AD 互換性: `CostFunction` trait を Dual64 対応型で実装することで対応可能

### roots クレートの評価

- **Context**: 求根アルゴリズム（Brent、Newton-Raphson、Bisection）の外部クレート移行先として評価
- **Sources Consulted**:
  - [roots 公式ドキュメント](https://docs.rs/roots/latest/roots/)
  - [roots crates.io](https://crates.io/crates/roots)
- **Findings**:
  - `find_root_brent`: ブラケット法、導関数不要
  - `find_root_newton_raphson`: 二次収束、導関数必要
  - `find_root_secant`: 割線法
  - `find_root_regula_falsi`: Illinois 変形
  - **注意**: Bisection は明示的に提供されていない（Brent が代替）
  - `Convergency` trait: 収束条件のカスタマイズ（`SimpleConvergency`、`DebugConvergency`）
  - `FloatType` trait: f32、f64 をサポートするが Dual64 は非対応
  - `SearchError` enum: エラーハンドリング
- **Implications**:
  - Newton-Raphson の AD モード（`find_root_ad`）は roots クレートでは直接サポートされない
  - AD ユースケースでは自前実装を維持するか、Dual64 用のラッパーを提供する必要あり
  - `BacktrackingNewtonSolver` は roots に存在しないため自前実装を維持

### faer クレートの評価

- **Context**: 線形代数ライブラリの性能評価（nalgebra との比較）
- **Sources Consulted**:
  - [faer 公式ドキュメント](https://docs.rs/faer/latest/faer/)
  - [faer GitHub リリースノート](https://users.rust-lang.org/t/release-faer-0-4-a-high-performance-linear-algebra-library/85715)
  - [rust_linalg_bench](https://github.com/sebcrozet/rust_linalg_bench)
- **Findings**:
  - 中〜大規模行列向けに最適化（ゲーム/グラフィックス用途には nalgebra 推奨）
  - SIMD 最適化: AVX512 サポート（nightly feature）
  - 分解: Cholesky、LU（full pivoting 含む）、QR、SVD
  - **制限事項**:
    - AD 型（Dual64）のネイティブサポートなし
    - Entity trait のカスタム設計が非標準型との統合を複雑化
- **Implications**:
  - Requirement 4（ベンチマーク評価）で検証必要
  - AD 互換性維持のため nalgebra をデフォルトバックエンドとして維持
  - faer は `faer-backend` feature flag で optional 提供（AD 不要のユースケース向け）

### 既存実装の分析

- **Context**: 現在の pricer_core 実装パターンの把握
- **Sources Consulted**: crates/pricer_core/src/math/optimisers/、crates/pricer_core/src/math/solvers/
- **Findings**:
  - **Optimisers**: ~600 行（nelder_mead.rs: ~330 行、lbfgs.rs: ~440 行）
  - **Solvers**: ~1200 行（brent.rs: ~470 行、newton_raphson.rs: ~450 行、levenberg_marquardt.rs: ~740 行、bisection.rs: ~200 行、backtracking_newton.rs: ~250 行）
  - **公開 API**:
    - `minimize_nelder_mead(f, x0, config) -> Result<OptimisationResult, OptimisationError>`
    - `minimize_lbfgs(f_grad, x0, config) -> Result<OptimisationResult, OptimisationError>`
    - `minimize_lbfgs_numerical(f, x0, config, h) -> Result<OptimisationResult, OptimisationError>`
    - `BrentSolver::find_root(f, a, b) -> Result<T, SolverError>`
    - `NewtonRaphsonSolver::find_root(f, f_prime, x0) -> Result<T, SolverError>`
    - `NewtonRaphsonSolver::find_root_ad(f, x0) -> Result<f64, SolverError>`
    - `LevenbergMarquardtSolver::solve(residuals, initial_params) -> Result<LMResult, SolverError>`
  - **使用箇所**:
    - `pricer_models::market::calibration::model_calibrator` → LevenbergMarquardtSolver
    - `pricer_models::market::calibration::bootstrapping::adjoint_solver` → BrentSolver、NewtonRaphsonSolver
- **Implications**:
  - API シグネチャ維持が必須（downstream 互換性）
  - AD モード（`find_root_ad`）は自前実装を維持する必要あり

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Full Replacement | 全アルゴリズムを外部クレートに置換 | 最大のコード削減、保守コスト最小化 | AD 互換性喪失、API 変更必要 | roots の Dual64 非対応が障壁 |
| Wrapper Pattern | 外部クレートを薄いラッパーで包み、既存 API 維持 | API 互換性維持、段階的移行可能 | 追加の抽象化レイヤー | **推奨**: 最もリスクが低い |
| Hybrid | 標準型は外部クレート、AD 型は自前実装維持 | AD 互換性と保守コスト削減の両立 | 2 つの実装を維持 | AD ユースケースに最適 |

## Design Decisions

### Decision: Wrapper Pattern の採用

- **Context**: 外部クレート移行において API 互換性と AD 互換性を維持する必要がある
- **Alternatives Considered**:
  1. Full Replacement — 全面的な外部クレート移行
  2. Wrapper Pattern — 薄いラッパーで既存 API 維持
  3. Hybrid — 標準型と AD 型で分離
- **Selected Approach**: Wrapper Pattern + AD Fallback
  - 標準型（f64）: argmin/roots への委譲
  - AD 型（Dual64）: 自前実装を維持（`find_root_ad` メソッド）
- **Rationale**:
  - Downstream 互換性を維持しつつ保守コストを削減
  - roots クレートが Dual64 をサポートしないため AD 用フォールバックが必要
- **Trade-offs**:
  - 利点: API 互換性維持、段階的移行、テスト容易性
  - 欠点: 薄いラッパーレイヤーの追加、一部自前実装の維持
- **Follow-up**: 回帰テストで数値精度と収束特性を検証

### Decision: faer は Optional Feature Flag として提供

- **Context**: faer は高性能だが AD 型をサポートしない
- **Alternatives Considered**:
  1. faer をデフォルトに — nalgebra を完全置換
  2. nalgebra をデフォルト維持、faer を optional — AD 互換性優先
  3. 両方維持せず — nalgebra のみ
- **Selected Approach**: nalgebra デフォルト、`faer-backend` feature flag で optional
- **Rationale**:
  - AD 互換性（Dual64）が pricer_core の基本要件
  - faer は AD 不要のユースケース（大規模行列計算）で性能向上を提供
- **Trade-offs**:
  - 利点: AD 互換性維持、性能オプション提供
  - 欠点: 2 つのバックエンドの維持コスト
- **Follow-up**: Requirement 4 のベンチマークで ≥20% 性能向上を検証

### Decision: BacktrackingNewtonSolver の自前実装維持

- **Context**: roots クレートは backtracking line search 付き Newton-Raphson を提供しない
- **Alternatives Considered**:
  1. roots の標準 Newton-Raphson を使用 — backtracking なし
  2. rootfind クレートの hybrid 実装を使用
  3. 自前実装を維持
- **Selected Approach**: 自前実装を維持し、roots の Newton-Raphson を内部で使用可能に
- **Rationale**:
  - Backtracking line search は遠い初期推定からの収束に重要
  - 金融アプリケーション（インプライド・ボラティリティ計算）で必須
- **Trade-offs**:
  - 利点: 堅牢な収束特性維持
  - 欠点: ~250 行の自前コード維持
- **Follow-up**: calibration 統合テストで収束特性を検証

## Risks & Mitigations

- **Risk 1**: argmin API 変更による破壊的変更 — ラッパー層で吸収、バージョン固定
- **Risk 2**: roots の Dual64 非対応 — AD 用フォールバック実装を維持
- **Risk 3**: 数値精度の差異 — 回帰テストで baseline との比較（許容範囲: 10x tolerance）
- **Risk 4**: faer の AD 非互換 — optional feature として分離、デフォルトは nalgebra
- **Risk 5**: 収束特性の変化 — 反復回数の検証（許容範囲: 2x）

## References

- [argmin 公式ドキュメント](https://docs.rs/argmin/latest/argmin/) — 最適化アルゴリズム詳細
- [roots 公式ドキュメント](https://docs.rs/roots/latest/roots/) — 求根アルゴリズム詳細
- [faer 公式ドキュメント](https://docs.rs/faer/latest/faer/) — 線形代数パフォーマンス
- [argmin book](https://www.argmin-rs.org/book/) — 使用ガイドとベストプラクティス
- [rootfind クレート](https://lib.rs/crates/rootfind) — hybrid Newton/bracketing 実装の参考
