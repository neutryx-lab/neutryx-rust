# Gap Analysis: external-numerics-migration

## Summary

本分析は、pricer_core の数値計算モジュール（optimisers/solvers/linalg）を外部クレートへ移行する際の現状と要件のギャップを調査した結果である。

### Key Findings

1. **argmin は既に workspace 依存関係に存在**するが、pricer_core では使用されていない
2. **自前実装は約 1,500 行**（Nelder-Mead ~200行、L-BFGS ~300行、Brent ~230行、Newton-Raphson ~150行、LM ~350行、その他）
3. **公開 API は安定**しており、既存の呼び出し元への影響を最小化できる設計
4. **roots クレートは未導入**だが、必要な機能（Brent、Bisection、Newton-Raphson）を提供
5. **AD 互換性**は num-dual フィーチャーフラグで管理され、find_root_ad メソッドが存在

---

## 1. Current State Investigation

### 1.1 Existing Assets

| モジュール | ファイル | 行数(概算) | 機能 |
|-----------|----------|-----------|------|
| `optimisers/nelder_mead.rs` | 自前実装 | ~200 | Nelder-Mead simplex |
| `optimisers/lbfgs.rs` | 自前実装 | ~310 | L-BFGS + numerical gradient |
| `optimisers/config.rs` | 設定型 | ~180 | OptimisationConfig, LbfgsConfig, NelderMeadConfig |
| `optimisers/result.rs` | 結果型 | ~60 | OptimisationResult |
| `optimisers/error.rs` | エラー型 | ~50 | OptimisationError |
| `solvers/brent.rs` | 自前実装 | ~230 | Brent's method |
| `solvers/newton_raphson.rs` | 自前実装 | ~150 | Newton-Raphson + AD support |
| `solvers/bisection.rs` | 自前実装 | ~200 | Bisection method |
| `solvers/backtracking_newton.rs` | 自前実装 | ~250 | Backtracking line search |
| `solvers/levenberg_marquardt.rs` | 自前実装 | ~350 | LM with Cholesky solver |
| `solvers/config.rs` | 設定型 | ~80 | SolverConfig |
| `linalg/mod.rs` | nalgebra wrapper | ~150 | Cholesky, LU, QR decompositions |

### 1.2 Dependency Status

```toml
# Workspace Cargo.toml (既存)
argmin = "0.10"
argmin-math = { version = "0.4", features = ["nalgebra_latest"] }

# pricer_core/Cargo.toml (現状: 未使用)
# argmin は依存関係に含まれていない
```

**発見**: argmin は workspace レベルで利用可能だが、pricer_core の Cargo.toml には追加されていない。

### 1.3 Integration Surfaces

| 呼び出し元 | 使用 API | 用途 |
|-----------|---------|------|
| `pricer_models::calibration::ModelCalibrator` | `LevenbergMarquardtSolver` | モデルキャリブレーション |
| `pricer_models::calibration::AdjointSolver` | `BrentSolver`, `NewtonRaphsonSolver` | AAD 互換求根 |
| `pricer_core/tests/math_integration.rs` | `minimize_nelder_mead` | 統合テスト |

### 1.4 Public API Surface

**Optimisers:**
```rust
// 公開関数
pub fn minimize_nelder_mead<F>(f: F, x0: &[f64], config: NelderMeadConfig) -> Result<OptimisationResult, OptimisationError>
pub fn minimize_lbfgs<F>(f: F, x0: &[f64], config: LbfgsConfig) -> Result<OptimisationResult, OptimisationError>
pub fn minimize_lbfgs_numerical<F>(f: F, x0: &[f64], config: LbfgsConfig, h: f64) -> Result<OptimisationResult, OptimisationError>

// 公開型
pub struct OptimisationConfig { abs_tol, rel_tol, max_iterations, verbose }
pub struct LbfgsConfig { base, m, c1, c2 }
pub struct NelderMeadConfig { base, alpha, gamma, rho, sigma, initial_scale }
pub struct OptimisationResult { params, value, iterations, func_evals, converged, message }
```

**Solvers:**
```rust
// 公開構造体
pub struct BrentSolver<T: Float> { config: SolverConfig<T> }
pub struct NewtonRaphsonSolver<T: Float> { config: SolverConfig<T> }
pub struct BisectionSolver<T: Float> { config: SolverConfig<T> }
pub struct BacktrackingNewtonSolver<T: Float> { config: SolverConfig<T> }
pub struct LevenbergMarquardtSolver { config: LMConfig }

// 公開型
pub struct SolverConfig<T: Float> { tolerance, max_iterations }
pub struct LMConfig { tolerance, max_iterations, initial_lambda, lambda_up, lambda_down, ... }
pub struct LMResult { params, residual_ss, iterations, converged, final_lambda }
```

---

## 2. Requirements Feasibility Analysis

### 2.1 Requirement-to-Asset Map

| Requirement | 現状 | Gap | 備考 |
|-------------|------|-----|------|
| **Req 1**: argmin への最適化移行 | 自前実装 | **Replace** | argmin::solver::neldermead, argmin::solver::quasinewton::LBFGS |
| **Req 2**: roots への求根移行 | 自前実装 | **Add dep + Replace** | roots crate 未導入 |
| **Req 3**: LM の外部化 | 自前実装 | **Replace** | argmin::solver::leastsquares 検討 |
| **Req 4**: faer 評価 | nalgebra 使用中 | **Research** | ベンチマーク必要 |
| **Req 5**: AD 互換性維持 | num-dual feature 対応済 | **Verify** | 外部クレートとの整合性確認 |
| **Req 6**: テスト互換性 | 既存テスト ~50件 | **Maintain** | 回帰テスト追加 |
| **Req 7**: 依存関係管理 | workspace 設定済 | **Extend** | roots, faer (optional) 追加 |
| **Req 8**: ドキュメント | 既存 doc comments | **Update** | 移行ガイド追加 |

### 2.2 External Crate Compatibility

#### argmin (v0.10)

| 機能 | argmin 提供 | 互換性 |
|------|------------|--------|
| Nelder-Mead | `argmin::solver::neldermead::NelderMead` | ✅ |
| L-BFGS | `argmin::solver::quasinewton::LBFGS` | ✅ |
| Line Search | `argmin::solver::linesearch::MoreThuente` | ✅ |
| Levenberg-Marquardt | `argmin::solver::leastsquares::LevenbergMarquardt` | ⚠️ API 差異あり |
| Numerical Gradient | `argmin::core::FiniteDiff` | ✅ |
| AD Support | `argmin-math` + num-dual | ⚠️ 要調査 |

**参考**: [argmin BrentRoot](https://argmin-rs.github.io/argmin/argmin/solver/brent/struct.BrentRoot.html) - argmin にも Brent solver が存在

#### roots (v0.0.8)

| 機能 | roots 提供 | 互換性 |
|------|------------|--------|
| Brent | `find_root_brent` | ✅ |
| Bisection | `find_root_bisection` | ✅ |
| Newton-Raphson | `find_root_newton_raphson` | ⚠️ API 差異 |
| Secant | `find_root_secant` | ✅ (追加機能) |
| Generic Float | `F: Float` | ⚠️ Convergency trait 必要 |

**注意**: roots は Rust 2015 edition を使用しており、やや古い設計。ただし安定しており広く使用されている。

#### faer (v0.21)

| 機能 | faer 提供 | nalgebra 比較 |
|------|----------|--------------|
| Cholesky | `faer::linalg::cholesky` | 高速（SIMD 最適化） |
| LU | `faer::linalg::lu` | 高速（SIMD 最適化） |
| QR | `faer::linalg::qr` | 高速（SIMD 最適化） |
| AD Support | ❌ | nalgebra は num-dual 互換 |
| Low-dim optimization | ❌ (大行列向け) | nalgebra が適切 |

**評価**: faer は大規模行列向け。pricer_core の典型的ユースケース（キャリブレーション: 10-50 パラメータ）では nalgebra が適切。大規模ポートフォリオリスク計算時に faer のメリットが出る可能性あり。

### 2.3 Complexity Signals

| 要件 | 複雑度 | 理由 |
|------|--------|------|
| Optimisers 移行 | **Medium** | API 維持しつつ内部実装を argmin に委譲 |
| Solvers 移行 | **Medium** | roots の Convergency trait との適合 |
| LM 移行 | **Medium** | Jacobian 計算のインターフェース差異 |
| faer 評価 | **Low-Medium** | ベンチマーク設計・実行 |
| AD 互換性 | **High** | 外部クレートの AD サポート検証 |
| テスト維持 | **Low** | 既存テスト流用可能 |

---

## 3. Implementation Approach Options

### Option A: Extend Existing Components (Wrapper Approach)

**概要**: 既存の公開 API を維持し、内部実装のみを外部クレートへ委譲する。

**対象ファイル変更**:
- `optimisers/nelder_mead.rs`: 自前アルゴリズム削除 → argmin ラッパー
- `optimisers/lbfgs.rs`: 自前アルゴリズム削除 → argmin ラッパー
- `solvers/brent.rs`: 自前アルゴリズム削除 → roots/argmin ラッパー
- `solvers/newton_raphson.rs`: 自前アルゴリズム削除 → roots ラッパー
- `solvers/bisection.rs`: 自前アルゴリズム削除 → roots ラッパー
- `solvers/levenberg_marquardt.rs`: 自前アルゴリズム削除 → argmin ラッパー

**Trade-offs**:
- ✅ 公開 API 完全互換
- ✅ 呼び出し元コード変更不要
- ✅ 段階的移行可能
- ❌ ラッパー層によるわずかなオーバーヘッド
- ❌ 外部クレートの挙動差異を吸収する変換コード必要

### Option B: Create New Components (Direct Integration)

**概要**: 新しい外部クレート直接利用モジュールを作成し、既存モジュールを deprecated 化。

**新規作成**:
- `optimisers/argmin_adapters.rs`: argmin 直接利用のアダプター
- `solvers/roots_adapters.rs`: roots 直接利用のアダプター
- `linalg/faer_backend.rs`: faer バックエンド (feature-gated)

**Trade-offs**:
- ✅ クリーンな設計
- ✅ 外部クレートの全機能にアクセス可能
- ❌ 呼び出し元コードの更新必要
- ❌ 移行期間中の二重実装

### Option C: Hybrid Approach (Recommended)

**概要**: Option A をベースに、AD 互換性が必要な部分のみ自前実装を保持。

**Phase 1**: 非 AD 機能を外部クレートへ移行
- `minimize_nelder_mead` → argmin
- `minimize_lbfgs` → argmin
- `minimize_lbfgs_numerical` → argmin with FiniteDiff
- `BrentSolver::find_root` → roots または argmin::BrentRoot
- `BisectionSolver::find_root` → roots
- `LevenbergMarquardtSolver::solve` → argmin

**Phase 2**: AD 機能の互換性検証
- `NewtonRaphsonSolver::find_root_ad` → 自前実装を保持 or roots 検証後に移行
- `BacktrackingNewtonSolver` → 自前実装保持（roots 非対応）

**Phase 3**: faer 評価（オプション）
- ベンチマーク実施後に決定

**Trade-offs**:
- ✅ リスク最小化（AD 機能を段階的に移行）
- ✅ 公開 API 互換維持
- ✅ 明確なフォールバック（AD 非対応時は自前実装）
- ❌ 一部重複コード残存の可能性

---

## 4. Effort & Risk Assessment

| 項目 | Effort | Risk | 備考 |
|------|--------|------|------|
| Optimisers argmin 移行 | **M** (3-5日) | **Low** | argmin API 成熟、既存パターンあり |
| Solvers roots 移行 | **M** (3-5日) | **Medium** | Convergency trait 適合検証必要 |
| LM argmin 移行 | **S** (1-2日) | **Low** | argmin LM 実装あり |
| AD 互換性検証 | **M** (3-5日) | **High** | 外部クレートの num-dual 対応状況未確定 |
| faer ベンチマーク | **S** (1-2日) | **Low** | 評価のみ、実装決定は後続フェーズ |
| 回帰テスト | **S** (1-2日) | **Low** | 既存テスト流用 |
| ドキュメント更新 | **S** (1日) | **Low** | API 互換なら軽微 |
| **合計** | **L** (1-2週) | **Medium** | AD 互換性が主要リスク |

---

## 5. Research Items for Design Phase

### 5.1 Immediate Research Needed

1. **argmin-math と num-dual の統合方法**
   - `argmin-math` の `numdual` feature の有無・動作確認
   - `CostFunction` / `Gradient` trait を Dual64 で実装する方法

2. **roots クレートの型制約**
   - `Convergency` trait の実装要件
   - `Float` bound と `num-dual::Dual64` の互換性

3. **argmin::solver::brent::BrentRoot vs roots::find_root_brent**
   - API 比較、どちらを採用するか決定

### 5.2 Deferred Research (Design Phase)

1. **faer ベンチマーク設計**
   - 代表的な行列サイズ（10×10, 100×100, 1000×1000）
   - Cholesky, LU, QR の実行時間比較

2. **BacktrackingNewtonSolver の代替**
   - argmin の line search アルゴリズムで代替可能か

---

## 6. Recommendations for Design Phase

### 6.1 Preferred Approach

**Option C (Hybrid)** を推奨。理由:
1. 公開 API 互換性を維持しつつ、内部実装を段階的に置換
2. AD 互換性が不確実な部分のリスクを限定
3. 既存テストを回帰テストとして最大活用

### 6.2 Key Design Decisions

1. **求根ソルバー**: `argmin::solver::brent::BrentRoot` vs `roots` クレート
   - argmin に Brent が存在するため、依存関係統一の観点から argmin 優先を検討

2. **AD モード**: 外部クレート非対応時は自前実装をフォールバックとして保持

3. **faer 採用**: ベンチマーク結果次第で決定、初期リリースでは nalgebra 維持

### 6.3 Integration Points to Address

1. `ModelCalibrator` → `LevenbergMarquardtSolver` 移行後の互換性
2. `AdjointSolver` → `BrentSolver` / `NewtonRaphsonSolver` の AD 互換維持

---

## Sources

- [roots crate - crates.io](https://crates.io/crates/roots)
- [roots::find_root_brent documentation](https://docs.rs/roots/latest/roots/fn.find_root_brent.html)
- [argmin BrentRoot](https://argmin-rs.github.io/argmin/argmin/solver/brent/struct.BrentRoot.html)
- [faer-rs GitHub](https://github.com/sarah-quinones/faer-rs)
- [nalgebra](https://nalgebra.rs/)
- [faer vs nalgebra discussion](https://users.rust-lang.org/t/release-faer-0-4-a-high-performance-linear-algebra-library/85715)
