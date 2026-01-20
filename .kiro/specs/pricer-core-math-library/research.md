# Research Document: pricer-core-math-library

## 概要

本ドキュメントは、`pricer_core::math`モジュール拡張に必要な数値アルゴリズムと実装パターンの調査結果をまとめたものである。

## 1. 既存コードベースパターン分析

### 1.1 ジェネリック型パターン

既存の`pricer_core`では、すべての数値関数が`T: num_traits::Float`でジェネリックに実装されている。

```rust
// 標準パターン（smoothing.rsより）
pub fn smooth_max<T: Float>(a: T, b: T, epsilon: T) -> T {
    // 定数変換にはfrom_f64を使用
    let two: T = from_f64(2.0);
    // ...
}
```

**重要な設計原則**:
- `from_f64`、`from_i32`、`from_usize`ヘルパーで定数を変換
- `T::one()`、`T::zero()`でリテラル0/1を表現
- `T::exp()`、`T::ln()`、`T::sqrt()`等の組み込みメソッドを使用

### 1.2 エラーハンドリングパターン

`thiserror`を使用した構造化エラー型が標準：

```rust
// error.rsより
#[derive(Error, Debug, Clone, PartialEq)]
pub enum InterpolationError {
    #[error("Query point {x} outside valid domain [{min}, {max}]")]
    OutOfBounds { x: f64, min: f64, max: f64 },
    // ...
}
```

**新規モジュールのエラー型命名規約**:
- `DistributionError` - 分布計算エラー
- `IntegrationError` - 数値積分エラー
- `OptimisationError` - 最適化エラー
- `LinearAlgebraError` - 線形代数エラー

### 1.3 テストパターン

```rust
// 精度検証：approx crateを使用
use approx::assert_relative_eq;
assert_relative_eq!(result, expected, epsilon = 1e-10);

// プロパティベーステスト：proptestを使用
proptest! {
    #[test]
    fn test_property(x in finite_f64_strategy()) {
        // ...
    }
}
```

### 1.4 ドキュメントパターン

モジュールレベルと関数レベルで詳細なドキュメントを提供：
- 数学的定義（Mathematical Definition）
- AD互換性に関する注記
- 使用例（Examples）
- 精度保証（Accuracy）

## 2. 数値アルゴリズム調査

### 2.1 正規分布関数

#### 累積分布関数（CDF）

**現状実装**（`pricer_models::analytical::distributions`）:
- Abramowitz-Stegun近似（式7.1.26）
- 精度：最大誤差1.5e-7

**改善オプション**:

| アルゴリズム | 精度 | 計算コスト | AD互換性 |
|-------------|------|-----------|---------|
| Abramowitz-Stegun | 1e-7 | 低 | 良好 |
| Hart近似 | 1e-15 | 中 | 良好 |
| Cody近似 | 1e-18 | 高 | 良好 |

**推奨**: Hart近似を新規実装。精度要件1e-15を満たし、有理多項式のためAD互換。

#### 逆累積分布関数（Inverse CDF / Quantile）

| アルゴリズム | 精度 | 計算コスト | 備考 |
|-------------|------|-----------|------|
| Moro近似 | 1e-8 | 低 | 三分岐 |
| Acklam近似 | 1e-9 | 低 | 二分岐 |
| Wichura AS241 | 1e-15 | 中 | 高精度 |

**推奨**: Acklam近似を採用。精度要件1e-9を満たし、実装が単純。

#### 二変量正規分布

**Drezner-Wesolowsky (1990)近似**:
- Gauss-Legendre求積法による数値積分
- 精度：1e-10〜1e-12
- 相関係数ρの全範囲[-1, 1]で安定

**実装戦略**:
```rust
pub fn bivariate_normal_cdf<T: Float>(
    x: T, y: T, rho: T
) -> T {
    // 特殊ケース処理（ρ = ±1, 0）
    // Gauss-Legendre求積法で一般ケース
}
```

### 2.2 数値積分

#### Gauss-Legendre求積法

**節点と重みの事前計算**:
```rust
// 7点Gauss-Legendre（精度O(h^14)）
const GAUSS_LEGENDRE_7_NODES: [f64; 7] = [
    -0.949107912342759, -0.741531185599394, -0.405845151377397,
    0.0,
    0.405845151377397, 0.741531185599394, 0.949107912342759
];
const GAUSS_LEGENDRE_7_WEIGHTS: [f64; 7] = [
    0.129484966168870, 0.279705391489277, 0.381830050505119,
    0.417959183673469,
    0.381830050505119, 0.279705391489277, 0.129484966168870
];
```

**推奨**: 7点、15点、21点をconst配列として提供。

#### Gauss-Kronrod求積法

**G7-K15（7点Gauss + 15点Kronrod）**:
- 積分値と誤差推定を同時計算
- 適応的積分の基礎

**G10-K21（10点Gauss + 21点Kronrod）**:
- より高精度な誤差推定

**実装戦略**:
```rust
pub struct IntegrationResult<T> {
    pub value: T,
    pub error_estimate: T,
    pub evaluations: usize,
}

pub fn integrate_gauss_kronrod<T, F>(
    f: F, a: T, b: T, rule: GKRule
) -> IntegrationResult<T>
where
    T: Float,
    F: Fn(T) -> T;
```

#### Runge-Kutta法

**RK4（古典的4次ルンゲクッタ）**:
```rust
pub fn rk4_step<T, F>(f: F, t: T, y: T, h: T) -> T
where
    T: Float,
    F: Fn(T, T) -> T
{
    let k1 = f(t, y);
    let k2 = f(t + h/2, y + h*k1/2);
    let k3 = f(t + h/2, y + h*k2/2);
    let k4 = f(t + h, y + h*k3);
    y + h * (k1 + 2*k2 + 2*k3 + k4) / 6
}
```

**RK45（Dormand-Prince）**:
- 適応的ステップサイズ制御
- 誤差推定付き

### 2.3 最適化アルゴリズム

#### L-BFGS（Limited-memory BFGS）

**アルゴリズム概要**:
- 準ニュートン法の一種
- Hessian行列の近似を履歴ベクトルから構築
- メモリ効率が高い（O(nm)、m = 履歴数）

**2ループ再帰**:
```rust
pub struct LBFGSOptimiser<T: Float> {
    m: usize,          // 履歴数（通常3-20）
    s_history: VecDeque<Vec<T>>,  // 位置差分履歴
    y_history: VecDeque<Vec<T>>,  // 勾配差分履歴
}

impl<T: Float> LBFGSOptimiser<T> {
    pub fn compute_direction(&self, grad: &[T]) -> Vec<T> {
        // 2ループ再帰でHessian^{-1} * gradを計算
    }
}
```

**直線探索オプション**:
- Backtracking（Armijo条件）
- More-Thuente（Wolfe条件）
- Nocedal-Wright（強Wolfe条件）

#### Nelder-Mead（Amoeba）

**アルゴリズム概要**:
- 導関数不要のシンプレックス法
- 反射、膨張、収縮、縮小の4操作

**パラメータ**:
```rust
pub struct NelderMeadConfig<T: Float> {
    pub alpha: T,  // 反射係数（通常1.0）
    pub gamma: T,  // 膨張係数（通常2.0）
    pub rho: T,    // 収縮係数（通常0.5）
    pub sigma: T,  // 縮小係数（通常0.5）
}
```

### 2.4 補間アルゴリズム

#### 対数線形補間

ディスカウントファクターに最適:
```rust
pub fn log_linear_interpolate<T: Float>(
    x: T, x0: T, x1: T, y0: T, y1: T
) -> T {
    let log_y0 = y0.ln();
    let log_y1 = y1.ln();
    let t = (x - x0) / (x1 - x0);
    (log_y0 + t * (log_y1 - log_y0)).exp()
}
```

#### Hermiteスプライン

導関数値を指定した補間:
```rust
pub fn hermite_interpolate<T: Float>(
    t: T, p0: T, p1: T, m0: T, m1: T
) -> T {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2*t3 - 3*t2 + 1;
    let h10 = t3 - 2*t2 + t;
    let h01 = -2*t3 + 3*t2;
    let h11 = t3 - t2;
    h00*p0 + h10*m0 + h01*p1 + h11*m1
}
```

#### SVI（Stochastic Volatility Inspired）

ボラティリティスマイル補間:
```rust
/// SVI raw parameterisation
/// w(k) = a + b * (ρ*(k-m) + sqrt((k-m)² + σ²))
pub fn svi_total_variance<T: Float>(
    k: T,  // log-moneyness
    a: T, b: T, rho: T, m: T, sigma: T
) -> T {
    let km = k - m;
    a + b * (rho * km + (km * km + sigma * sigma).sqrt())
}
```

### 2.5 線形代数

#### コレスキー分解

正定値対称行列のLL^T分解:
```rust
pub fn cholesky_decompose<T: Float>(
    a: &[Vec<T>]
) -> Result<Vec<Vec<T>>, LinearAlgebraError> {
    let n = a.len();
    let mut l = vec![vec![T::zero(); n]; n];

    for i in 0..n {
        for j in 0..=i {
            let mut sum = a[i][j];
            for k in 0..j {
                sum = sum - l[i][k] * l[j][k];
            }
            if i == j {
                if sum <= T::zero() {
                    return Err(LinearAlgebraError::NotPositiveDefinite);
                }
                l[i][j] = sum.sqrt();
            } else {
                l[i][j] = sum / l[j][j];
            }
        }
    }
    Ok(l)
}
```

#### 外部クレートvs自前実装

| 観点 | nalgebra | 自前実装 |
|-----|----------|---------|
| 機能 | 豊富 | 必要最小限 |
| Enzyme AD | 要検証 | 完全対応 |
| 依存関係 | 追加 | なし |
| メンテナンス | 外部依存 | 内部 |

**推奨**: コレスキー、LU分解は自前実装。小規模行列（10x10以下）に特化。

### 2.6 金融関数

#### Black-Scholesの配置検討

**現状**: `pricer_models::analytical::black_scholes`

**検討事項**:
- `pricer_core`（L1）は純粋数学ライブラリ
- Black-Scholesは金融ロジックを含む（コール/プット判定等）
- 依存関係: BS → norm_cdf, norm_pdf

**推奨**:
- `norm_cdf`、`norm_pdf`は`pricer_core::math::distributions`に移動
- Black-Scholes、Bachelier、SABRは`pricer_models`に残す
- `pricer_models`から`pricer_core::math::distributions`を利用

#### SABR Hagan近似

```rust
/// Hagan (2002) SABR implied volatility approximation
pub fn sabr_implied_vol<T: Float>(
    f: T,      // forward
    k: T,      // strike
    t: T,      // time to expiry
    alpha: T,  // initial vol
    beta: T,   // CEV exponent
    rho: T,    // correlation
    nu: T,     // vol of vol
) -> T {
    // ATM case: K ≈ F
    // OTM/ITM case: 一般公式
}
```

#### Normal SABR（Antonov近似）

正規SABRモデルのボラティリティ計算。Antonov & Spector (2012)の近似公式を使用。

### 2.7 乱数生成

**現状**: `pricer_pricing::rng::PricerRng`

**検討事項**:
- `pricer_pricing`はMonte Carloシミュレーション専用
- Enzyme AD互換性の観点で`pricer_pricing`に残すのが適切
- 他クレートでRNGが必要な場合は、`pricer_core`にトレイト定義のみ配置

**推奨**:
- `pricer_core::math::rng`にトレイト定義（`RandomNumberGenerator`）のみ配置
- 実装は`pricer_pricing::rng`に残す

## 3. Enzyme AD互換性要件

### 3.1 避けるべきパターン

1. **動的ディスパッチ**: `Box<dyn Trait>`は使用不可
2. **条件分岐**: 可能な限りsmooth関数で代替
3. **非決定的操作**: 内部状態を持つイテレータ等

### 3.2 推奨パターン

1. **静的ディスパッチ**: ジェネリクスとmonomorphization
2. **ブランチフリー演算**: smooth_max, smooth_min等
3. **純粋関数**: 副作用なし、参照透過性

```rust
// 良い例：AD互換
pub fn smooth_relu<T: Float>(x: T, epsilon: T) -> T {
    smooth_max(x, T::zero(), epsilon)
}

// 悪い例：AD非互換（条件分岐）
pub fn relu<T: Float>(x: T) -> T {
    if x > T::zero() { x } else { T::zero() }
}
```

## 4. 実装優先度

### Phase 1: 基盤（高優先度）

| モジュール | 内容 | 工数 |
|-----------|------|------|
| distributions | norm_cdf/pdf移動、inverse_cdf、bivariate | M |
| calculus | 有限差分（前方/後方/中心/2階） | S |
| utilities | sign, clamp, lerp, factorial, log_gamma | S |

### Phase 2: 数値計算（中優先度）

| モジュール | 内容 | 工数 |
|-----------|------|------|
| integrators | Gauss-Legendre, Gauss-Kronrod, RK4 | L |
| interpolators拡張 | flat, log_linear, hermite, binary_search | M |
| solvers拡張 | bisection, backtracking_newton | M |

### Phase 3: 高度機能（低優先度）

| モジュール | 内容 | 工数 |
|-----------|------|------|
| optimisers | L-BFGS, Nelder-Mead | L |
| linalg | Matrix, Cholesky, LU | L |
| fitting | least_squares, gaussian_fit | M |
| mesh | 1D/2D grid generation | S |

## 5. 参考文献

1. Abramowitz, M. & Stegun, I.A. (1964). Handbook of Mathematical Functions.
2. Drezner, Z. & Wesolowsky, G.O. (1990). On the computation of the bivariate normal integral.
3. Nocedal, J. & Wright, S.J. (2006). Numerical Optimization.
4. Hagan, P.S. et al. (2002). Managing Smile Risk.
5. Gatheral, J. (2004). A parsimonious arbitrage-free implied volatility parameterization with application to the valuation of volatility derivatives.
