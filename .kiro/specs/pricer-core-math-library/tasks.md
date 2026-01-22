# Implementation Tasks: pricer-core-math-library

本ドキュメントは、`pricer_core::math`モジュール拡張の実装タスクを定義する。

## Phase 1: 基盤モジュール（distributions, calculus, utilities）

### Task 1: distributions モジュール - エラー型と正規分布

**要件マッピング**: REQ-1

**説明**: `distributions`モジュールを新規作成し、エラー型と標準正規分布関数を実装する。Hart近似による高精度CDFおよびAcklam近似によるinverse CDFを含む。

**受け入れ基準**:
- `distributions/mod.rs`、`distributions/error.rs`、`distributions/normal.rs`を作成
- `DistributionError`列挙型を定義（`InvalidProbability`、`InvalidCorrelation`等）
- `norm_cdf<T: Float>`を実装（精度1e-15）
- `norm_pdf<T: Float>`を実装
- `norm_inv_cdf<T: Float>`を実装（精度1e-9）
- 単体テストで精度を検証
- `pricer_models::analytical::distributions`からの移行パスを確認

**サブタスク**:
- [x] 1.1: `distributions/error.rs`を作成し`DistributionError`を定義
- [x] 1.2: `distributions/normal.rs`を作成し`norm_cdf`、`norm_pdf`を実装
- [x] 1.3: `norm_inv_cdf`をAcklam近似で実装
- [x] 1.4: 単体テストとプロパティベーステストを追加

---

### Task 2: distributions モジュール - 二変量正規分布と非心カイ二乗分布

**要件マッピング**: REQ-1

**説明**: 二変量正規分布CDFおよび非心カイ二乗分布CDFを実装する。Drezner-Wesolowsky近似とSeries展開を使用。

**受け入れ基準**:
- `distributions/bivariate_normal.rs`を作成
- `bivariate_norm_cdf<T: Float>`を実装（精度1e-10）
- `distributions/chi_squared.rs`を作成
- `noncentral_chi_squared_cdf<T: Float>`を実装（精度1e-8）
- 参照値との比較テストを追加

**サブタスク**:
- [x] 2.1: `bivariate_normal.rs`を作成しDrezner-Wesolowsky近似を実装
- [x] 2.2: `chi_squared.rs`を作成し非心カイ二乗CDFを実装
- [x] 2.3: テストを追加（R/SciPyの参照値と比較）

---

### Task 3: distributions モジュール - ガウシアンコピュラ

**要件マッピング**: REQ-1

**説明**: ガウシアンコピュラの結合確率計算を実装する。

**受け入れ基準**:
- `distributions/copula.rs`を作成
- `GaussianCopula`構造体を定義（相関行列を保持）
- `joint_probability`メソッドを実装
- 正定値検証を含むコンストラクタを実装

**サブタスク**:
- [x] 3.1: `copula.rs`を作成し`GaussianCopula`構造体を定義
- [x] 3.2: 2次元の場合の`joint_probability`を実装
- [x] 3.3: 多次元の場合の実装（オプション、`linalg`フィーチャーで有効化）

---

### Task 4: calculus モジュール - 有限差分

**要件マッピング**: REQ-3

**説明**: 有限差分法による数値微分を実装する。前方/後方/中心差分、2階導関数、偏微分を含む。

**受け入れ基準**:
- `calculus/mod.rs`、`calculus/finite_difference.rs`を作成
- `DifferenceType`列挙型（Forward, Backward, Central）を定義
- `finite_diff<T, F>`を実装
- `finite_diff_second<T, F>`を実装
- `partial_diff<T, F>`を実装

**サブタスク**:
- [x] 4.1: `calculus/mod.rs`と`finite_difference.rs`を作成
- [x] 4.2: 1階導関数（前方/後方/中心差分）を実装
- [x] 4.3: 2階導関数を実装
- [x] 4.4: 偏微分を実装
- [x] 4.5: 精度検証テストを追加

---

### Task 5: calculus モジュール - bump幅自動選択

**要件マッピング**: REQ-3

**説明**: 数値微分のbump幅を自動選択する機能を実装する。

**受け入れ基準**:
- `calculus/bump_selection.rs`を作成
- `suggest_bump_size<T: Float>`を実装（機械イプシロンに基づく）
- 中心差分用の最適bump幅計算を実装

**サブタスク**:
- [x] 5.1: `bump_selection.rs`を作成
- [x] 5.2: 1階導関数用の最適bump幅を実装
- [x] 5.3: 2階導関数用の最適bump幅を実装

---

### Task 6: utilities モジュール - 基本関数

**要件マッピング**: REQ-13

**説明**: 基本的なユーティリティ関数（sign, clamp, lerp）を実装する。

**受け入れ基準**:
- `utilities/mod.rs`、`utilities/basic.rs`を作成
- `sign<T: Float>`を実装
- `clamp<T: Float>`を実装
- `lerp<T: Float>`を実装
- 単体テストを追加

**サブタスク**:
- [x] 6.1: `utilities/mod.rs`と`basic.rs`を作成
- [x] 6.2: 基本関数を実装
- [x] 6.3: テストを追加

---

### Task 7: utilities モジュール - 組み合わせ論関数

**要件マッピング**: REQ-13

**説明**: 階乗、二項係数を実装する。

**受け入れ基準**:
- `utilities/combinatorics.rs`を作成
- `factorial<T: Float>`を実装（キャッシュ付き）
- `binomial<T: Float>`を実装

**サブタスク**:
- [x] 7.1: `combinatorics.rs`を作成
- [x] 7.2: `factorial`を実装
- [x] 7.3: `binomial`を実装
- [x] 7.4: オーバーフロー対策のテストを追加

---

### Task 8: utilities モジュール - 特殊関数

**要件マッピング**: REQ-13

**説明**: 対数ガンマ関数とベータ関数を実装する。

**受け入れ基準**:
- `utilities/special.rs`を作成
- `log_gamma<T: Float>`を実装（Lanczos近似）
- `beta<T: Float>`を実装

**サブタスク**:
- [x] 8.1: `special.rs`を作成
- [x] 8.2: `log_gamma`をLanczos近似で実装
- [x] 8.3: `beta`を実装
- [x] 8.4: 精度検証テストを追加

---

## Phase 2: 数値計算モジュール（integrators, interpolators拡張, solvers拡張）

### Task 9: integrators モジュール - Gauss-Legendre求積法 (P)

**要件マッピング**: REQ-2

**説明**: Gauss-Legendre求積法（7点、15点、21点）を実装する。

**受け入れ基準**:
- `integrators/mod.rs`、`integrators/gauss_legendre.rs`を作成
- `IntegrationResult<T>`構造体を定義
- `GaussLegendreOrder`列挙型（N7, N15, N21）を定義
- `integrate_gauss_legendre<T, F>`を実装
- 各次数の重みと節点を事前計算して定義

**サブタスク**:
- [x] 9.1: `integrators/mod.rs`と`IntegrationResult`を作成
- [x] 9.2: Gauss-Legendre重みと節点を定義
- [x] 9.3: `integrate_gauss_legendre`を実装
- [x] 9.4: 多項式積分の精度テストを追加

---

### Task 10: integrators モジュール - Gauss-Kronrod求積法 (P)

**要件マッピング**: REQ-2

**説明**: Gauss-Kronrod求積法（G7-K15、G10-K21）を実装する。誤差推定機能付き。

**受け入れ基準**:
- `integrators/gauss_kronrod.rs`を作成
- `GaussKronrodRule`列挙型を定義
- `integrate_gauss_kronrod<T, F>`を実装
- 誤差推定を`IntegrationResult.error_estimate`に格納

**サブタスク**:
- [x] 10.1: `gauss_kronrod.rs`を作成
- [x] 10.2: G7-K15の重みと節点を定義
- [x] 10.3: G10-K21の重みと節点を定義
- [x] 10.4: 誤差推定付き積分を実装
- [x] 10.5: テストを追加

---

### Task 11: integrators モジュール - 適応的積分

**要件マッピング**: REQ-2

**説明**: 適応的積分（区間分割）を実装する。

**受け入れ基準**:
- `integrators/adaptive.rs`を作成
- `integrate_adaptive<T, F>`を実装
- 許容誤差に基づく区間分割
- 無限区間の変数変換（tanh-sinh）を実装

**サブタスク**:
- [x] 11.1: `adaptive.rs`を作成
- [x] 11.2: 区間分割アルゴリズムを実装
- [x] 11.3: 無限区間変換を実装
- [x] 11.4: テストを追加

---

### Task 12: integrators モジュール - Runge-Kutta法 (P)

**要件マッピング**: REQ-2

**説明**: Runge-Kutta法（RK4、RK45）を実装する。

**受け入れ基準**:
- `integrators/runge_kutta.rs`を作成
- `rk4_step<T, F>`を実装
- `rk45_integrate<T, F>`を実装（適応的ステップ制御）
- ODEの数値解が正しいことをテスト

**サブタスク**:
- [x] 12.1: `runge_kutta.rs`を作成
- [x] 12.2: RK4の1ステップを実装
- [x] 12.3: RK45（Dormand-Prince）を実装
- [x] 12.4: テスト（指数関数のODEなど）を追加

---

### Task 13: interpolators 拡張 - フラット補間と対数線形補間 (P)

**要件マッピング**: REQ-5

**説明**: フラット補間（区分定数）と対数線形補間を実装する。

**受け入れ基準**:
- `interpolators/flat.rs`を作成
- `FlatInterpolator<T>`を実装（Left/Rightモード）
- `interpolators/log_linear.rs`を作成
- `LogLinearInterpolator<T>`を実装

**サブタスク**:
- [x] 13.1: `flat.rs`を作成し`FlatInterpolator`を実装
- [x] 13.2: `log_linear.rs`を作成し`LogLinearInterpolator`を実装
- [x] 13.3: テストを追加

---

### Task 14: interpolators 拡張 - Hermiteスプラインと二分探索 (P)

**要件マッピング**: REQ-5

**説明**: Hermiteスプライン補間と二分探索/線形探索を実装する。

**受け入れ基準**:
- `interpolators/hermite.rs`を作成
- `HermiteInterpolator<T>`を実装
- `interpolators/search.rs`を作成
- `binary_search<T>`、`linear_search<T>`を実装

**サブタスク**:
- [x] 14.1: `hermite.rs`を作成し`HermiteInterpolator`を実装
- [x] 14.2: `search.rs`を作成し検索関数を実装
- [x] 14.3: テストを追加

---

### Task 15: interpolators 拡張 - SVI補間

**要件マッピング**: REQ-5

**説明**: SVI（Stochastic Volatility Inspired）補間を実装する。

**受け入れ基準**:
- `interpolators/svi.rs`を作成
- `SviParams`構造体を定義
- `svi_total_variance<T: Float>`を実装
- `svi_implied_vol<T: Float>`を実装

**サブタスク**:
- [x] 15.1: `svi.rs`を作成し`SviParams`を定義
- [x] 15.2: SVI total variance公式を実装
- [x] 15.3: テストを追加

---

### Task 16: solvers 拡張 - 二分法

**要件マッピング**: REQ-11

**説明**: 二分法ソルバーを実装する。

**受け入れ基準**:
- `solvers/bisection.rs`を作成
- `BisectionSolver<T>`を実装
- `find_root<F>`メソッドを実装
- 収束保証と最大反復回数のチェック

**サブタスク**:
- [x] 16.1: `bisection.rs`を作成し`BisectionSolver`を実装
- [x] 16.2: 異符号チェックを実装
- [x] 16.3: テストを追加

---

### Task 17: solvers 拡張 - Backtracking Newton法

**要件マッピング**: REQ-11

**説明**: 直線探索付きNewton法を実装する。

**受け入れ基準**:
- `solvers/backtracking_newton.rs`を作成
- `BacktrackingNewtonSolver<T>`を実装
- Armijo条件による直線探索を実装

**サブタスク**:
- [x] 17.1: `backtracking_newton.rs`を作成
- [x] 17.2: Armijo条件を実装
- [x] 17.3: 直線探索付きNewton法を実装
- [x] 17.4: テストを追加

---

## Phase 3: 高度機能モジュール（optimisers, linalg, fitting, mesh）

### Task 18: optimisers モジュール - argminラッパー基盤 (P)

**要件マッピング**: REQ-4

**説明**: argminクレートのラッパー基盤を作成する。エラー型と設定型を定義。

**受け入れ基準**:
- `optimisers/mod.rs`を作成
- `optimisers/error.rs`を作成し`OptimisationError`を定義
- `optimisers/config.rs`を作成し`OptimisationConfig`を定義
- argminの主要型をre-export

**サブタスク**:
- [x] 18.1: `optimisers/mod.rs`を作成しargminをre-export
- [x] 18.2: `error.rs`を作成しエラー型を定義
- [x] 18.3: `config.rs`を作成し設定型を定義

---

### Task 19: optimisers モジュール - L-BFGSラッパー (P)

**要件マッピング**: REQ-4

**説明**: L-BFGSのラッパー関数を実装する。

**受け入れ基準**:
- `optimisers/wrappers.rs`を作成
- `minimize_lbfgs`関数を実装
- 使用例をドキュメントに記載

**サブタスク**:
- [x] 19.1: `lbfgs.rs`を作成
- [x] 19.2: `minimize_lbfgs`を実装
- [x] 19.3: Rosenbrock関数でテスト

---

### Task 20: optimisers モジュール - Nelder-Meadラッパー (P)

**要件マッピング**: REQ-4

**説明**: Nelder-Mead法のラッパー関数を実装する。

**受け入れ基準**:
- `minimize_nelder_mead`関数を実装
- 導関数不要の最適化テストを追加

**サブタスク**:
- [x] 20.1: `minimize_nelder_mead`を実装
- [x] 20.2: テストを追加

---

### Task 21: linalg モジュール - nalgebraラッパー基盤 (P)

**要件マッピング**: REQ-9

**説明**: nalgebraクレートのラッパー基盤を作成する。

**受け入れ基準**:
- `linalg/mod.rs`を作成
- `linalg/error.rs`を作成し`LinearAlgebraError`を定義
- nalgebraの主要型をre-export
- `Matrix<T>`、`Vector<T>`型エイリアスを定義

**サブタスク**:
- [x] 21.1: `linalg/mod.rs`を作成しnalgebraをre-export
- [x] 21.2: `error.rs`を作成しエラー型を定義
- [x] 21.3: 型エイリアスを定義

---

### Task 22: linalg モジュール - 分解ラッパー (P)

**要件マッピング**: REQ-9

**説明**: コレスキー分解、LU分解のラッパー関数を実装する。

**受け入れ基準**:
- `linalg/wrappers.rs`を作成
- `cholesky_solve<T>`を実装
- `lu_solve<T>`を実装
- `cholesky<T>`を実装

**サブタスク**:
- [x] 22.1: `wrappers.rs`を作成
- [x] 22.2: `cholesky_solve`を実装
- [x] 22.3: `lu_solve`を実装
- [x] 22.4: `cholesky`（分解のみ）を実装
- [x] 22.5: テストを追加

---

### Task 23: linalg モジュール - 行列演算ラッパー (P)

**要件マッピング**: REQ-9

**説明**: 行列式、逆行列のラッパー関数を実装する。

**受け入れ基準**:
- `determinant<T>`を実装
- `inverse<T>`を実装

**サブタスク**:
- [x] 23.1: `determinant`を実装
- [x] 23.2: `inverse`を実装
- [x] 23.3: テストを追加

---

### Task 24: fitting モジュール - 線形最小二乗

**要件マッピング**: REQ-8

**説明**: 線形最小二乗フィットを実装する（nalgebraのSVDを活用）。

**受け入れ基準**:
- `fitting/mod.rs`、`fitting/least_squares.rs`を作成
- `linear_least_squares<T>`を実装
- `FittingResult<T>`構造体（解、残差、R²）を定義

**サブタスク**:
- [x] 24.1: `fitting/mod.rs`と`least_squares.rs`を作成
- [x] 24.2: `FittingResult`を定義
- [x] 24.3: `linear_least_squares`を実装
- [x] 24.4: テストを追加

---

### Task 25: fitting モジュール - ガウシアンフィット

**要件マッピング**: REQ-8

**説明**: ガウシアン分布へのフィッティングを実装する。

**受け入れ基準**:
- `fitting/gaussian.rs`を作成
- `fit_gaussian<T>`を実装（平均、標準偏差の推定）

**サブタスク**:
- [x] 25.1: `gaussian.rs`を作成
- [x] 25.2: `fit_gaussian`を実装
- [x] 25.3: テストを追加

---

### Task 26: mesh モジュール - 1次元メッシュ (P)

**要件マッピング**: REQ-12

**説明**: 1次元メッシュ生成を実装する。

**受け入れ基準**:
- `mesh/mod.rs`、`mesh/grid_1d.rs`を作成
- `uniform_grid<T>`を実装（等間隔）
- `log_grid<T>`を実装（対数間隔）
- `refine_grid<T>`を実装（細分化）

**サブタスク**:
- [x] 26.1: `mesh/mod.rs`と`grid_1d.rs`を作成
- [x] 26.2: `uniform_grid`を実装
- [x] 26.3: `log_grid`を実装
- [x] 26.4: `refine_grid`を実装
- [x] 26.5: テストを追加

---

### Task 27: mesh モジュール - 2次元メッシュ (P)

**要件マッピング**: REQ-12

**説明**: 2次元メッシュ生成を実装する。

**受け入れ基準**:
- `mesh/grid_2d.rs`を作成
- `Grid2D<T>`構造体を定義
- `tensor_product_grid<T>`を実装

**サブタスク**:
- [x] 27.1: `grid_2d.rs`を作成
- [x] 27.2: `Grid2D`を定義
- [x] 27.3: `tensor_product_grid`を実装
- [x] 27.4: テストを追加

---

## Phase 4: 統合と移行

### Task 28: mod.rs 更新とエクスポート整理

**要件マッピング**: REQ-14

**説明**: `pricer_core::math`のmod.rsを更新し、全モジュールをエクスポートする。

**受け入れ基準**:
- `math/mod.rs`に全サブモジュールを追加
- 公開APIを整理
- ドキュメントコメントを更新

**サブタスク**:
- [x] 28.1: `mod.rs`にサブモジュール宣言を追加
- [x] 28.2: re-exportを整理
- [x] 28.3: モジュールレベルドキュメントを更新

---

### Task 29: pricer_models 移行 - distributions

**要件マッピング**: REQ-1, REQ-14

**説明**: `pricer_models::analytical::distributions`を`pricer_core::math::distributions`に移行する。

**受け入れ基準**:
- `pricer_models`の`norm_cdf`、`norm_pdf`を`pricer_core`からのインポートに変更
- `pricer_models::analytical::distributions`を削除または非推奨化
- `pricer_models`の既存テストがパス

**サブタスク**:
- [x] 29.1: `pricer_models`の該当ファイルを更新
- [x] 29.2: Black-Scholes等が新しいインポートで動作することを確認
- [x] 29.3: 既存テストを実行

---

### Task 30: Cargo.toml 依存関係更新

**要件マッピング**: REQ-4, REQ-9

**説明**: `pricer_core/Cargo.toml`に新しい依存関係を追加する。

**受け入れ基準**:
- `nalgebra = "0.33"`を追加
- `argmin = "0.10"`を追加
- `argmin-math = { version = "0.4", features = ["nalgebra_latest"] }`を追加
- ビルドが成功

**サブタスク**:
- [x] 30.1: `Cargo.toml`を更新
- [x] 30.2: `cargo build`を実行
- [x] 30.3: 依存関係の競合がないことを確認

---

### Task 31: 統合テストとドキュメント

**要件マッピング**: REQ-14

**説明**: 統合テストを追加し、ドキュメントを完成させる。

**受け入れ基準**:
- `pricer_core/tests/math_integration.rs`を作成
- 全モジュールの基本的な統合テストを追加
- 各モジュールのドキュメントコメントが完備

**サブタスク**:
- [x] 31.1: 統合テストファイルを作成
- [x] 31.2: 分布、積分、補間の統合テストを追加
- [x] 31.3: ドキュメントを確認・補完
- [x] 31.4: `cargo clippy --pedantic`を実行（スタイル警告のみ、エラーなし）

---

## 並列実行ガイド

以下のタスクは並列実行可能（依存関係なし）:

**Phase 1 内**:
- Task 1-3 (distributions) と Task 4-5 (calculus) と Task 6-8 (utilities) は並列可能 (P)

**Phase 2 内**:
- Task 9-12 (integrators) と Task 13-15 (interpolators) と Task 16-17 (solvers) は並列可能 (P)

**Phase 3 内**:
- Task 18-20 (optimisers) と Task 21-23 (linalg) と Task 24-25 (fitting) と Task 26-27 (mesh) は並列可能 (P)

**依存関係**:
- Task 29 (pricer_models移行) は Task 1 (distributions) 完了後
- Task 31 (統合テスト) は Task 28-30 完了後

---

## タスク一覧サマリ

| Phase | タスク | 要件 | 並列 |
|-------|-------|------|------|
| 1 | Task 1-3: distributions | REQ-1 | ✓ |
| 1 | Task 4-5: calculus | REQ-3 | ✓ |
| 1 | Task 6-8: utilities | REQ-13 | ✓ |
| 2 | Task 9-12: integrators | REQ-2 | ✓ |
| 2 | Task 13-15: interpolators | REQ-5 | ✓ |
| 2 | Task 16-17: solvers | REQ-11 | ✓ |
| 3 | Task 18-20: optimisers | REQ-4 | ✓ |
| 3 | Task 21-23: linalg | REQ-9 | ✓ |
| 3 | Task 24-25: fitting | REQ-8 | ✓ |
| 3 | Task 26-27: mesh | REQ-12 | ✓ |
| 4 | Task 28: mod.rs更新 | REQ-14 | - |
| 4 | Task 29: pricer_models移行 | REQ-1 | - |
| 4 | Task 30: Cargo.toml更新 | REQ-4,9 | - |
| 4 | Task 31: 統合テスト | REQ-14 | - |

**総タスク数**: 31タスク（Phase 1: 8, Phase 2: 9, Phase 3: 10, Phase 4: 4）
