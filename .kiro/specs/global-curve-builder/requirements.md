# Requirements Document

## Project Description (Input)
「カーブ構築を行列計算（連立方程式）として解き、かつAAD（Adjoint Algorithmic Differentiation）のJacobian逆行列アプローチと整合させる」

このアプローチは、逐次的なブートストラップ（1期間ずつ解く）ではなく、**グローバル・ソルバー（全期間同時推定）**として実装することを意味します。これにより、すべての観測商品がすべてのカーブパラメータに依存する構造をJacobianとして明示的に扱うことができ、Implicit Function Theorem（陰関数定理）を用いたAADの高速化が可能になります。

### 1. 実装の全体像

実装は主に以下の3つのレイヤーに分割するべきです。

1. **Math Layer (`pricer_core`)**: 金融ロジックに依存しない、汎用的な「多次元ニュートン・ラフソン法」ソルバー。ここでJacobianの逆行列（またはLU分解）を扱います。
2. **Model Logic (`pricer_models`)**: カーブ構築問題を  という形式の残差関数として定義するアダプター。
3. **AAD Integration (`pricer_risk` / Enzyme)**: ソルバーの逆伝播を定義するカスタム微分ルール（Custom Gradient）。

---

### 2. 各レイヤーの詳細設計

#### Phase 1: Math Layer (汎用ソルバーの実装)

**場所**: `crates/pricer_core/src/math/solvers/multidimensional.rs` (新規作成)

既存の`pricer_core/src/math/solvers/`には1次元ソルバー（Brent, Newton等）がありますが、ここに行列対応のソルバーを追加します。

**設計のポイント**:

* 入力: 初期値 、関数 、Jacobian計算 。
* 出力: 解  と、収束時点での **Jacobianの逆行列（またはFactorization）**。
* 数値安定性: 行列計算には `ndarray` と `ndarray-linalg` (LAPACK backend) を使用することを推奨します。

```rust
// crates/pricer_core/src/math/solvers/multidimensional.rs

use ndarray::{Array1, Array2};
use crate::math::linalg::error::LinalgError;

pub struct SolverResult {
    pub solution: Array1<f64>,
    pub jacobian_inv: Array2<f64>, // AADのために保持
    pub iterations: usize,
}

pub trait SystemOfEquations {
    fn evaluate(&self, x: &Array1<f64>) -> Result<Array1<f64>, LinalgError>;
    fn jacobian(&self, x: &Array1<f64>) -> Result<Array2<f64>, LinalgError>;
}

pub fn newton_raphson_global<S: SystemOfEquations>(
    system: &S,
    initial_guess: Array1<f64>,
    tolerance: f64,
    max_iter: usize,
) -> Result<SolverResult, LinalgError> {
    let mut x = initial_guess;

    for i in 0..max_iter {
        let f_val = system.evaluate(&x)?;
        let norm = f_val.mapv(|v| v.abs()).sum(); // 簡易的なノルム

        if norm < tolerance {
            // 収束時のJacobian逆行列を計算して返す
            let j = system.jacobian(&x)?;
            // ※実務ではinv()よりLU分解等を返すが、ここでは概念的に逆行列とする
            let j_inv = crate::math::linalg::wrappers::inverse(&j)?;

            return Ok(SolverResult {
                solution: x,
                jacobian_inv: j_inv,
                iterations: i,
            });
        }

        let j = system.jacobian(&x)?;
        let delta = crate::math::linalg::wrappers::solve(&j, &(-f_val))?; // J * delta = -F
        x = x + delta;
    }

    Err(LinalgError::ConvergenceFailure)
}
```

#### Phase 2: Model Logic (カーブ構築ロジック)

**場所**: `crates/pricer_models/src/market/global_solver.rs` (新規作成)

既存モジュール内に、上記`SystemOfEquations`トレイトを実装する構造体を配置します。

**設計のポイント**:

* **パラメータ **: カーブのピラー（各日付のDiscount Factorの対数、あるいはZero Rate）。
* **ターゲット **: 市場レート（Swap Rate, Futures Price等）。
* **関数 **: 現在のパラメータ  からカーブを構築し、各商品の理論価格（またはレート）を計算する。
* **Jacobian **: 各ピラーを微小変動させたときのレート感応度（Finite DifferenceまたはAADで計算）。

```rust
// crates/pricer_models/src/market/global_solver.rs

use crate::market::calibration::Instrument;
use pricer_core::math::solvers::multidimensional::SystemOfEquations;
use ndarray::{Array1, Array2};

pub struct CurveCalibrationProblem {
    instruments: Vec<Box<dyn Instrument>>, // Swap, Deposit等
    market_quotes: Array1<f64>,
    // 補間ロジック等の設定を持つ
}

impl SystemOfEquations for CurveCalibrationProblem {
    fn evaluate(&self, params: &Array1<f64>) -> Result<Array1<f64>, _> {
        // 1. paramsからCurveオブジェクトを一時的に構築
        let temp_curve = self.build_curve(params);

        // 2. 各商品の理論価格を計算
        let mut model_prices = Array1::zeros(self.instruments.len());
        for (i, inst) in self.instruments.iter().enumerate() {
            model_prices[i] = inst.price(&temp_curve)?;
        }

        // 3. 残差 = 理論値 - 市場値
        Ok(model_prices - &self.market_quotes)
    }

    fn jacobian(&self, params: &Array1<f64>) -> Result<Array2<f64>, _> {
        // ここでAAD (Enzyme) を呼ぶか、数値微分を行う
        // 行列計算アプローチの場合、ここが密行列(Dense Matrix)になる
        self.calculate_jacobian_ad(params)
    }
}
```

#### Phase 3: AAD Integration (陰関数定理の適用)

**場所**: `crates/pricer_risk/src/enzyme/implicit.rs` (概念的な配置) または `pricer_core` 内

これが最も重要な要件「AADに有用」な部分です。
ソルバーの反復計算（`for`ループ）をAADでそのまま微分すると、ステップ数分のメモリと計算時間を消費します。
代わりに、**Implicit Function Theorem** を利用し、収束点における Jacobian 逆行列を使って、市場データへの勾配を直接計算します。

Enzyme等のAADツールに対して、`solve`関数に対するカスタム勾配（Shadow）を定義します。

**数学的背景**:

（ より  のため）

**実装イメージ**:

```rust
// Enzymeのカスタムルール（擬似コード）
#[enzyme_rules]
fn shadow_solve_system(
    system: &CurveCalibrationProblem,
    market_quotes_gradient: &mut Array1<f64>, // 出力：ここへ勾配を蓄積
    output_adjoint: &Array1<f64>,             // 入力：解x*に対する感応度
    solver_result: &SolverResult              // 元関数の結果（J_invを含む）
) {
    // Implicit Function Theorem:
    // d(Objective) / d(MarketParams) = (d(Objective)/d(x*)) * (d(x*)/d(MarketParams))
    //                                = adjoint^T * J^{-1}

    // 1. adjoint ベクトルに Jacobian逆行列を掛ける
    // x_bar = J^{-T} * y_bar
    let implicit_adjoint = solver_result.jacobian_inv.t().dot(output_adjoint);

    // 2. 市場クオートへの感応度として加算
    // ターゲット値 m は F(x) - m = 0 なので、符号は状況によるが通常は直結
    *market_quotes_gradient += &implicit_adjoint;
}
```

### 3. 具体的な実装手順の推奨

このライブラリの現状に即して、以下の順序で実装することを推奨します。

1. **`pricer_core` の拡張**:
* `multidimensional.rs` を追加し、`ndarray` ベースの Newton-Raphson を実装してください。このとき、返り値に `inverse_jacobian` を含める設計にすることが必須です。


2. **`bootstrapping` のリファクタリング**:
* 現在の `crates/pricer_models/src/market/calibration/bootstrapping/` は、おそらく1商品ずつ解く逐次法（Bootstrap）が前提になっている可能性があります。
* これを `GlobalBootstrapper` 構造体としてラップし、全商品をベクトルとして扱う実装を追加してください。


3. **線形代数バックエンドの選定**:
* `pricer_core/Cargo.toml` を確認し、`ndarray-linalg` (要OpenBLAS/Intel MKL) が有効か確認してください。大規模なカーブ（数千ピラー）でなければ、Rust純粋実装の `linfa-linalg` や `nalgebra` でも十分ですが、`neutryx` の方向性（科学計算重視）からは `ndarray` + LAPACK が適切です。

## Requirements
<!-- Will be generated in /kiro:spec-requirements phase -->
