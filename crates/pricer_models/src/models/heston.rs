//! Heston確率的ボラティリティモデル実装
//!
//! Hestonモデルは以下のSDEで記述される確率的ボラティリティモデル:
//! ```text
//! dS = r * S * dt + sqrt(V) * S * dW_S
//! dV = kappa * (theta - V) * dt + xi * sqrt(V) * dW_V
//! E[dW_S * dW_V] = rho * dt
//! ```
//! ここで:
//! - S = 資産価格
//! - V = 瞬間分散
//! - r = リスクフリーレート
//! - kappa = 平均回帰速度
//! - theta = 長期分散
//! - xi = ボラティリティのボラティリティ (vol-of-vol)
//! - rho = 資産価格と分散の相関
//! - dW_S, dW_V = 相関ブラウン運動
//!
//! ## Feller条件
//!
//! 分散が常に正を保つための十分条件:
//! ```text
//! 2 * kappa * theta > xi^2
//! ```
//!
//! ## QE離散化スキーム
//!
//! Andersen (2008) のQuadratic Exponential離散化スキームを使用。
//! psi値に基づいて二次スキームと指数スキームを滑らかに切り替える。
//!
//! ## 使用例
//!
//! ```
//! use pricer_models::models::heston::{HestonParams, HestonError};
//!
//! // パラメータを作成
//! let params = HestonParams::new(
//!     100.0,  // スポット価格
//!     0.04,   // 初期分散
//!     0.04,   // 長期分散
//!     1.5,    // 平均回帰速度
//!     0.3,    // vol-of-vol
//!     -0.7,   // 相関
//!     0.05,   // リスクフリーレート
//!     1.0,    // 満期
//! );
//! assert!(params.is_ok());
//!
//! let p = params.unwrap();
//! assert!(p.satisfies_feller()); // Feller条件をチェック
//! ```

use pricer_core::math::smoothing::{smooth_indicator, smooth_max, smooth_sqrt};
use pricer_core::traits::priceable::Differentiable;
use pricer_core::traits::Float;
use thiserror::Error;

/// Hestonモデルエラー型
///
/// パラメータ検証と数値計算時のエラーを表現する。
/// `thiserror`クレートを使用して構造化されたエラー情報を提供。
///
/// # バリアント
///
/// - `InvalidSpot`: スポット価格が正でない
/// - `InvalidV0`: 初期分散が正でない
/// - `InvalidTheta`: 長期分散が正でない
/// - `InvalidKappa`: 平均回帰速度が正でない
/// - `InvalidXi`: vol-of-volが正でない
/// - `InvalidRho`: 相関が[-1, 1]の範囲外
/// - `InvalidMaturity`: 満期が正でない
/// - `NumericalInstability`: 数値計算の不安定性
/// - `NonFinite`: NaNまたは無限大が検出された
///
/// # 例
///
/// ```
/// use pricer_models::models::heston::HestonError;
///
/// let err = HestonError::InvalidSpot(-100.0);
/// assert!(format!("{}", err).contains("-100"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum HestonError {
    /// 無効なスポット価格（正でなければならない）
    #[error("無効なスポット価格: S0 = {0} (正の値が必要)")]
    InvalidSpot(f64),

    /// 無効な初期分散（正でなければならない）
    #[error("無効な初期分散: v0 = {0} (正の値が必要)")]
    InvalidV0(f64),

    /// 無効な長期分散（正でなければならない）
    #[error("無効な長期分散: theta = {0} (正の値が必要)")]
    InvalidTheta(f64),

    /// 無効な平均回帰速度（正でなければならない）
    #[error("無効な平均回帰速度: kappa = {0} (正の値が必要)")]
    InvalidKappa(f64),

    /// 無効なvol-of-vol（正でなければならない）
    #[error("無効なvol-of-vol: xi = {0} (正の値が必要)")]
    InvalidXi(f64),

    /// 無効な相関係数（-1から1の範囲内でなければならない）
    #[error("無効な相関係数: rho = {0} ([-1, 1]の範囲が必要)")]
    InvalidRho(f64),

    /// 無効な満期（正でなければならない）
    #[error("無効な満期: T = {0} (正の値が必要)")]
    InvalidMaturity(f64),

    /// 無効なQE切り替え閾値（正でなければならない）
    #[error("無効なQE閾値: psi_c = {0} (正の値が必要)")]
    InvalidPsiC(f64),

    /// 無効なsmoothing epsilon（正でなければならない）
    #[error("無効なsmoothing epsilon: {0} (正の値が必要)")]
    InvalidEpsilon(f64),

    /// 数値的不安定性が検出された
    #[error("数値的不安定性: {0}")]
    NumericalInstability(String),

    /// NaNまたは無限大が検出された
    #[error("{0}でNaNまたはInfinityが検出されました")]
    NonFinite(String),
}

/// Hestonモデルパラメータ
///
/// # 型パラメータ
///
/// * `T` - Float型（f64またはAD互換のDualNumber）
///
/// # フィールド
///
/// * `spot` - スポット価格 (S0 > 0)
/// * `v0` - 初期分散 (v0 > 0)
/// * `theta` - 長期分散 (theta > 0)
/// * `kappa` - 平均回帰速度 (kappa > 0)
/// * `xi` - ボラティリティのボラティリティ (xi > 0)
/// * `rho` - 相関係数 (-1 <= rho <= 1)
/// * `rate` - リスクフリーレート
/// * `maturity` - 満期までの時間 (T > 0)
/// * `psi_c` - QE切り替え閾値 (推奨値: 1.5)
/// * `smoothing_epsilon` - smooth approximation用のepsilon
///
/// # 例
///
/// ```
/// use pricer_models::models::heston::HestonParams;
///
/// let params = HestonParams::new(
///     100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0
/// );
/// assert!(params.is_ok());
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HestonParams<T: Float> {
    /// スポット価格 (S0)
    pub spot: T,
    /// 初期分散 (v0)
    pub v0: T,
    /// 長期分散 (theta)
    pub theta: T,
    /// 平均回帰速度 (kappa)
    pub kappa: T,
    /// ボラティリティのボラティリティ (xi/sigma)
    pub xi: T,
    /// 相関係数 (rho)
    pub rho: T,
    /// リスクフリーレート
    pub rate: T,
    /// 満期までの時間
    pub maturity: T,
    /// QE切り替え閾値 (psi_c, デフォルト: 1.5)
    pub psi_c: T,
    /// smooth approximation epsilon
    pub smoothing_epsilon: T,
}

impl<T: Float> HestonParams<T> {
    /// 新しいHestonパラメータを作成（検証付き）
    ///
    /// # 引数
    ///
    /// * `spot` - スポット価格（正でなければならない）
    /// * `v0` - 初期分散（正でなければならない）
    /// * `theta` - 長期分散（正でなければならない）
    /// * `kappa` - 平均回帰速度（正でなければならない）
    /// * `xi` - vol-of-vol（正でなければならない）
    /// * `rho` - 相関係数（-1から1の範囲）
    /// * `rate` - リスクフリーレート
    /// * `maturity` - 満期（正でなければならない）
    ///
    /// # 戻り値
    ///
    /// パラメータが有効な場合は`Ok(HestonParams)`、無効な場合は`Err(HestonError)`
    ///
    /// # 例
    ///
    /// ```
    /// use pricer_models::models::heston::HestonParams;
    ///
    /// // 有効なパラメータ
    /// let params = HestonParams::new(100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0);
    /// assert!(params.is_ok());
    ///
    /// // 無効なスポット価格
    /// let invalid = HestonParams::new(-100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0);
    /// assert!(invalid.is_err());
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spot: T,
        v0: T,
        theta: T,
        kappa: T,
        xi: T,
        rho: T,
        rate: T,
        maturity: T,
    ) -> Result<Self, HestonError> {
        let params = Self {
            spot,
            v0,
            theta,
            kappa,
            xi,
            rho,
            rate,
            maturity,
            psi_c: T::from(1.5).unwrap_or(T::one()),
            smoothing_epsilon: T::from(1e-8).unwrap_or(T::zero()),
        };
        params.validate()?;
        Ok(params)
    }

    /// カスタムQE閾値を設定
    ///
    /// # 引数
    ///
    /// * `psi_c` - QE切り替え閾値（正でなければならない）
    ///
    /// # 戻り値
    ///
    /// 更新されたパラメータ
    pub fn with_psi_c(mut self, psi_c: T) -> Result<Self, HestonError> {
        if psi_c <= T::zero() {
            return Err(HestonError::InvalidPsiC(
                psi_c.to_f64().unwrap_or(f64::NAN),
            ));
        }
        self.psi_c = psi_c;
        Ok(self)
    }

    /// カスタムsmoothing epsilonを設定
    ///
    /// # 引数
    ///
    /// * `epsilon` - smoothing epsilon（正でなければならない）
    ///
    /// # 戻り値
    ///
    /// 更新されたパラメータ
    pub fn with_epsilon(mut self, epsilon: T) -> Result<Self, HestonError> {
        if epsilon <= T::zero() {
            return Err(HestonError::InvalidEpsilon(
                epsilon.to_f64().unwrap_or(f64::NAN),
            ));
        }
        self.smoothing_epsilon = epsilon;
        Ok(self)
    }

    /// パラメータを検証
    ///
    /// # 戻り値
    ///
    /// パラメータが有効な場合は`Ok(())`、無効な場合は`Err(HestonError)`
    pub fn validate(&self) -> Result<(), HestonError> {
        // スポット価格は正でなければならない
        if self.spot <= T::zero() {
            return Err(HestonError::InvalidSpot(
                self.spot.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // 初期分散は正でなければならない
        if self.v0 <= T::zero() {
            return Err(HestonError::InvalidV0(
                self.v0.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // 長期分散は正でなければならない
        if self.theta <= T::zero() {
            return Err(HestonError::InvalidTheta(
                self.theta.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // 平均回帰速度は正でなければならない
        if self.kappa <= T::zero() {
            return Err(HestonError::InvalidKappa(
                self.kappa.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // vol-of-volは正でなければならない
        if self.xi <= T::zero() {
            return Err(HestonError::InvalidXi(
                self.xi.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // 相関は[-1, 1]の範囲内でなければならない
        let one = T::one();
        if self.rho < -one || self.rho > one {
            return Err(HestonError::InvalidRho(
                self.rho.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // 満期は正でなければならない
        if self.maturity <= T::zero() {
            return Err(HestonError::InvalidMaturity(
                self.maturity.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // QE閾値は正でなければならない
        if self.psi_c <= T::zero() {
            return Err(HestonError::InvalidPsiC(
                self.psi_c.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // smoothing epsilonは正でなければならない
        if self.smoothing_epsilon <= T::zero() {
            return Err(HestonError::InvalidEpsilon(
                self.smoothing_epsilon.to_f64().unwrap_or(f64::NAN),
            ));
        }

        Ok(())
    }

    /// Feller条件をチェック (2 * kappa * theta > xi^2)
    ///
    /// Feller条件が満たされる場合、分散過程は常に正を保つ。
    ///
    /// # 戻り値
    ///
    /// Feller条件が満たされる場合は`true`、そうでない場合は`false`
    ///
    /// # 例
    ///
    /// ```
    /// use pricer_models::models::heston::HestonParams;
    ///
    /// // Feller条件を満たすパラメータ: 2 * 1.5 * 0.04 = 0.12 > 0.3^2 = 0.09
    /// let params = HestonParams::new(100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
    /// assert!(params.satisfies_feller());
    ///
    /// // Feller条件を満たさないパラメータ: 2 * 0.5 * 0.04 = 0.04 < 0.5^2 = 0.25
    /// let params2 = HestonParams::new(100.0, 0.04, 0.04, 0.5, 0.5, -0.7, 0.05, 1.0).unwrap();
    /// assert!(!params2.satisfies_feller());
    /// ```
    pub fn satisfies_feller(&self) -> bool {
        let two = T::from(2.0).unwrap_or(T::one());
        let lhs = two * self.kappa * self.theta;
        let rhs = self.xi * self.xi;
        lhs > rhs
    }

    /// Feller比率を計算 (2 * kappa * theta / xi^2)
    ///
    /// 値が1.0以上の場合、Feller条件が満たされる。
    ///
    /// # 戻り値
    ///
    /// Feller比率
    pub fn feller_ratio(&self) -> T {
        let two = T::from(2.0).unwrap_or(T::one());
        let numerator = two * self.kappa * self.theta;
        let denominator = self.xi * self.xi;
        if denominator > T::zero() {
            numerator / denominator
        } else {
            T::infinity()
        }
    }
}

impl<T: Float> Default for HestonParams<T> {
    /// デフォルトのHestonパラメータを作成
    ///
    /// 標準的なパラメータセット:
    /// - spot = 100.0
    /// - v0 = 0.04 (20% vol)
    /// - theta = 0.04 (20% vol long-term)
    /// - kappa = 1.5
    /// - xi = 0.3
    /// - rho = -0.7
    /// - rate = 0.05 (5%)
    /// - maturity = 1.0 (1 year)
    fn default() -> Self {
        Self {
            spot: T::from(100.0).unwrap_or(T::one()),
            v0: T::from(0.04).unwrap_or(T::zero()),
            theta: T::from(0.04).unwrap_or(T::zero()),
            kappa: T::from(1.5).unwrap_or(T::one()),
            xi: T::from(0.3).unwrap_or(T::zero()),
            rho: T::from(-0.7).unwrap_or(T::zero()),
            rate: T::from(0.05).unwrap_or(T::zero()),
            maturity: T::from(1.0).unwrap_or(T::one()),
            psi_c: T::from(1.5).unwrap_or(T::one()),
            smoothing_epsilon: T::from(1e-8).unwrap_or(T::zero()),
        }
    }
}

/// Hestonモデル
///
/// 確率的ボラティリティモデルの実装。以下のSDEで記述される:
/// ```text
/// dS = r * S * dt + sqrt(V) * S * dW_S
/// dV = kappa * (theta - V) * dt + xi * sqrt(V) * dW_V
/// E[dW_S * dW_V] = rho * dt
/// ```
///
/// # 型パラメータ
///
/// * `T` - Float型（f64またはAD互換のDualNumber）
///
/// # 特徴
///
/// - **QE離散化**: Andersen (2008) のQuadratic Exponential離散化スキームを使用
/// - **Feller条件**: 2*kappa*theta > xi^2 を満たす場合、分散は常に正
/// - **分散フロア**: Feller条件が満たされない場合、分散フロアを適用
/// - **AD互換**: ジェネリックFloat型によりnum-dual互換
///
/// # 例
///
/// ```
/// use pricer_models::models::heston::{HestonModel, HestonParams};
///
/// let params = HestonParams::new(
///     100.0,  // スポット価格
///     0.04,   // 初期分散
///     0.04,   // 長期分散
///     1.5,    // 平均回帰速度
///     0.3,    // vol-of-vol
///     -0.7,   // 相関
///     0.05,   // リスクフリーレート
///     1.0,    // 満期
/// ).unwrap();
///
/// let model = HestonModel::new(params).unwrap();
/// assert!(model.check_feller_condition());
/// ```
#[derive(Clone)]
pub struct HestonModel<T: Float> {
    /// モデルパラメータ
    params: HestonParams<T>,
    /// 分散フロア（Feller条件が満たされない場合に使用）
    variance_floor: T,
}

impl<T: Float> std::fmt::Debug for HestonModel<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HestonModel")
            .field("params", &self.params)
            .field(
                "variance_floor",
                &self.variance_floor.to_f64().unwrap_or(0.0),
            )
            .finish()
    }
}

impl<T: Float> HestonModel<T> {
    /// 新しいHestonモデルを作成
    ///
    /// パラメータを検証し、Feller条件をチェックする。
    /// Feller条件が満たされない場合、警告ログを出力し、分散フロアを適用する。
    ///
    /// # 引数
    ///
    /// * `params` - Hestonモデルパラメータ
    ///
    /// # 戻り値
    ///
    /// パラメータが有効な場合は`Ok(HestonModel)`、無効な場合は`Err(HestonError)`
    ///
    /// # 例
    ///
    /// ```
    /// use pricer_models::models::heston::{HestonModel, HestonParams};
    ///
    /// let params = HestonParams::new(100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
    /// let model = HestonModel::new(params);
    /// assert!(model.is_ok());
    /// ```
    pub fn new(params: HestonParams<T>) -> Result<Self, HestonError> {
        // パラメータ検証
        params.validate()?;

        // 分散フロアを計算（smoothing_epsilonを使用）
        let variance_floor = params.smoothing_epsilon;

        // Feller条件をチェックし、違反時は警告
        let model = Self {
            params,
            variance_floor,
        };

        if !model.check_feller_condition() {
            // Feller条件違反の警告をログ出力
            // NOTE: ログクレートが利用可能な場合はlog::warn!を使用
            #[cfg(feature = "std")]
            {
                eprintln!(
                    "警告: Feller条件 (2*kappa*theta > xi^2) が満たされていません。\
                     分散フロア {} が適用されます。\
                     kappa={}, theta={}, xi={}",
                    model.variance_floor.to_f64().unwrap_or(0.0),
                    model.params.kappa.to_f64().unwrap_or(0.0),
                    model.params.theta.to_f64().unwrap_or(0.0),
                    model.params.xi.to_f64().unwrap_or(0.0),
                );
            }
        }

        Ok(model)
    }

    /// パラメータを検証
    ///
    /// # 戻り値
    ///
    /// パラメータが有効な場合は`Ok(())`、無効な場合は`Err(HestonError)`
    pub fn validate(&self) -> Result<(), HestonError> {
        self.params.validate()
    }

    /// モデルパラメータへの参照を取得
    ///
    /// # 戻り値
    ///
    /// HestonParamsへの参照
    pub fn params(&self) -> &HestonParams<T> {
        &self.params
    }

    /// Feller条件をチェック (2 * kappa * theta > xi^2)
    ///
    /// Feller条件が満たされる場合、分散過程は常に正を保つ。
    ///
    /// # 戻り値
    ///
    /// Feller条件が満たされる場合は`true`、そうでない場合は`false`
    ///
    /// # 例
    ///
    /// ```
    /// use pricer_models::models::heston::{HestonModel, HestonParams};
    ///
    /// // Feller条件を満たすパラメータ
    /// let params = HestonParams::new(100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
    /// let model = HestonModel::new(params).unwrap();
    /// assert!(model.check_feller_condition());
    /// ```
    pub fn check_feller_condition(&self) -> bool {
        self.params.satisfies_feller()
    }

    /// 分散フロアを取得
    ///
    /// Feller条件が満たされない場合、分散がこの値を下回らないようにする。
    ///
    /// # 戻り値
    ///
    /// 分散フロア値
    pub fn variance_floor(&self) -> T {
        self.variance_floor
    }

    /// 分散フロアを設定
    ///
    /// # 引数
    ///
    /// * `floor` - 新しい分散フロア値（正でなければならない）
    ///
    /// # 戻り値
    ///
    /// 更新されたモデル（ビルダーパターン）
    pub fn with_variance_floor(mut self, floor: T) -> Self {
        if floor > T::zero() {
            self.variance_floor = floor;
        }
        self
    }

    // ================================================================
    // QE離散化スキーム (Andersen 2008)
    // ================================================================

    /// QEスキームの条件付きモーメントを計算
    ///
    /// 分散過程 V_{t+dt} | V_t の条件付き期待値と分散を計算する。
    ///
    /// # Arguments
    /// * `v_current` - 現在の分散
    /// * `dt` - タイムステップ
    ///
    /// # Returns
    /// (m, s2, psi) where:
    /// - m: 条件付き期待値 E[V_{t+dt} | V_t]
    /// - s2: 条件付き分散 Var[V_{t+dt} | V_t]
    /// - psi: s2 / m^2 (スキーム切り替え指標)
    ///
    /// # Mathematical Background
    /// For the CIR variance process:
    /// dV = kappa * (theta - V) * dt + xi * sqrt(V) * dW
    ///
    /// The conditional distribution is non-central chi-squared.
    /// QE scheme matches the first two moments:
    /// - E[V_{t+dt}|V_t] = theta + (V_t - theta) * exp(-kappa * dt)
    /// - Var[V_{t+dt}|V_t] = (V_t * xi^2 * exp(-kappa*dt) / kappa) * (1 - exp(-kappa*dt))
    ///                     + (theta * xi^2 / (2*kappa)) * (1 - exp(-kappa*dt))^2
    pub fn compute_qe_moments(&self, v_current: T, dt: T) -> (T, T, T) {
        let kappa = self.params.kappa;
        let theta = self.params.theta;
        let xi = self.params.xi;

        // exp(-kappa * dt)
        let exp_neg_kappa_dt = (-kappa * dt).exp();
        let one_minus_exp = T::one() - exp_neg_kappa_dt;

        // Conditional mean: E[V_{t+dt} | V_t] = theta + (v_current - theta) * exp(-kappa * dt)
        let m = theta + (v_current - theta) * exp_neg_kappa_dt;

        // Conditional variance (Andersen 2008, Eq. 17)
        // s2 = V_t * (xi^2 * exp(-kappa*dt) / kappa) * (1 - exp(-kappa*dt))
        //    + (theta * xi^2 / (2*kappa)) * (1 - exp(-kappa*dt))^2
        let two = T::from(2.0).unwrap_or(T::one() + T::one());
        let xi_squared = xi * xi;

        let term1 = v_current * xi_squared * exp_neg_kappa_dt * one_minus_exp / kappa;
        let term2 = theta * xi_squared * one_minus_exp * one_minus_exp / (two * kappa);

        let s2 = term1 + term2;

        // psi = s2 / m^2 (ratio of variance to mean squared)
        // Add small epsilon to avoid division by zero
        let eps = self.params.smoothing_epsilon;
        let m_safe = smooth_max(m, eps, eps);
        let psi = s2 / (m_safe * m_safe);

        (m, s2, psi)
    }

    /// QE二次スキーム（psi < psi_c の場合）
    ///
    /// 分散が比較的大きい場合に使用。二次多項式の根として次の分散を計算。
    ///
    /// # Arguments
    /// * `m` - 条件付き期待値
    /// * `s2` - 条件付き分散
    /// * `psi` - s2 / m^2
    /// * `uv` - 一様乱数 U ~ Uniform(0, 1)
    ///
    /// # Returns
    /// 次の分散値 V_{t+dt}
    ///
    /// # Mathematical Background
    /// For psi < psi_c, use moment-matched quadratic:
    /// V_{t+dt} = a * (b + Z_v)^2 where Z_v ~ N(0,1)
    /// with a, b chosen to match m and s2.
    pub fn qe_quadratic_step(&self, m: T, s2: T, psi: T, uv: T) -> T {
        let eps = self.params.smoothing_epsilon;
        let one = T::one();
        let two = T::from(2.0).unwrap_or(one + one);

        // Compute b^2 based on psi
        // For psi in (0, 1]: b^2 = 2/psi - 1 + sqrt(2/psi) * sqrt(2/psi - 1)
        // For psi in (1, psi_c]: b^2 = 2 * (1/psi - 1 + sqrt(1/psi) * sqrt(1/psi - 1))
        //
        // Simplified unified formula (Andersen 2008, Eq. 27):
        // b^2 = 2 * psi^{-1} - 1 + sqrt(2 * psi^{-1}) * sqrt(2 * psi^{-1} - 1)
        //     = (2/psi - 1) + sqrt(2/psi * (2/psi - 1))

        let psi_inv = one / smooth_max(psi, eps, eps);
        let two_psi_inv = two * psi_inv;
        let term_inner = two_psi_inv - one;

        // Ensure non-negative argument for sqrt using smooth_max
        let term_inner_safe = smooth_max(term_inner, T::zero(), eps);
        let sqrt_term = smooth_sqrt(two_psi_inv * term_inner_safe, eps);

        let b_squared = term_inner_safe + sqrt_term;

        // b = sqrt(b^2) - ensure non-negative
        let b = smooth_sqrt(b_squared, eps);

        // a = m / (1 + b^2)
        let a = m / (one + b_squared);

        // Convert uniform to normal using inverse CDF approximation
        // Z_v = Phi^{-1}(uv) ≈ rational approximation
        let z_v = self.inverse_normal_cdf(uv);

        // V_{t+dt} = a * (b + Z_v)^2
        let b_plus_z = b + z_v;
        let v_next = a * b_plus_z * b_plus_z;

        // Ensure non-negative using smooth_max
        smooth_max(v_next, T::zero(), eps)
    }

    /// QE指数スキーム（psi >= psi_c の場合）
    ///
    /// 分散が小さい場合に使用。ゼロ質量を持つ混合分布で次の分散を計算。
    ///
    /// # Arguments
    /// * `m` - 条件付き期待値
    /// * `psi` - s2 / m^2
    /// * `uv` - 一様乱数 U ~ Uniform(0, 1)
    ///
    /// # Returns
    /// 次の分散値 V_{t+dt}
    ///
    /// # Mathematical Background
    /// For psi >= psi_c, use exponential martingale with probability mass at zero:
    /// V_{t+dt} = psi^{-1}(U_v; p, beta) where
    /// - p = (psi - 1) / (psi + 1)
    /// - beta = (1 - p) / m = 2 / (m * (psi + 1))
    pub fn qe_exponential_step(&self, m: T, psi: T, uv: T) -> T {
        let eps = self.params.smoothing_epsilon;
        let one = T::one();
        let two = T::from(2.0).unwrap_or(one + one);

        // p = (psi - 1) / (psi + 1) - probability of zero
        let psi_safe = smooth_max(psi, eps, eps);
        let p = (psi_safe - one) / (psi_safe + one);

        // Clamp p to [0, 1)
        let p_clamped = smooth_max(p, T::zero(), eps);
        let _p_final = smooth_max(one - p_clamped, eps, eps); // 1 - p > 0 (used for validation)

        // beta = 2 / (m * (psi + 1))
        let m_safe = smooth_max(m, eps, eps);
        let beta = two / (m_safe * (psi_safe + one));

        // Inverse CDF of the mixture distribution:
        // if U <= p: V = 0
        // else: V = -ln((1 - U) / (1 - p)) / beta
        //
        // Using smooth_indicator for differentiability:
        // V = smooth_indicator(uv - p) * (-ln((1 - uv) / (1 - p)) / beta)

        let one_minus_uv = smooth_max(one - uv, eps, eps);
        let one_minus_p = smooth_max(one - p_clamped, eps, eps);
        let log_ratio = (one_minus_uv / one_minus_p).ln();

        // Exponential inverse CDF: -ln(U') / beta where U' = (1-U)/(1-p)
        let v_exp = smooth_max(-log_ratio / beta, T::zero(), eps);

        // Smooth blend: if uv <= p, return 0; else return v_exp
        let indicator = smooth_indicator(uv - p_clamped, eps);
        let v_next = indicator * v_exp;

        // Ensure non-negative
        smooth_max(v_next, T::zero(), eps)
    }

    /// QE分散ステップ（二次/指数スキームの滑らかなブレンド）
    ///
    /// psi値に基づいてスキームを滑らかに切り替える。
    ///
    /// # Arguments
    /// * `v_current` - 現在の分散
    /// * `dt` - タイムステップ
    /// * `uv` - 一様乱数 U ~ Uniform(0, 1)
    ///
    /// # Returns
    /// 次の分散値 V_{t+dt}
    pub fn qe_variance_step(&self, v_current: T, dt: T, uv: T) -> T {
        let (m, s2, psi) = self.compute_qe_moments(v_current, dt);
        let psi_c = self.params.psi_c;
        let eps = self.params.smoothing_epsilon;

        // Compute both schemes
        let v_quadratic = self.qe_quadratic_step(m, s2, psi, uv);
        let v_exponential = self.qe_exponential_step(m, psi, uv);

        // Smooth blend based on psi relative to psi_c
        // indicator = 0 when psi < psi_c (use quadratic)
        // indicator = 1 when psi >= psi_c (use exponential)
        let indicator = smooth_indicator(psi - psi_c, eps);

        // Blend: v_next = (1 - indicator) * v_quadratic + indicator * v_exponential
        let one = T::one();
        let v_next = (one - indicator) * v_quadratic + indicator * v_exponential;

        // Apply variance floor using smooth_max
        smooth_max(v_next, self.variance_floor, eps)
    }

    /// 相関ブラウン運動を生成（Cholesky分解）
    ///
    /// 独立な標準正規乱数から相関のあるブラウン運動増分を生成する。
    ///
    /// # Arguments
    /// * `z1` - 独立標準正規乱数（価格用）
    /// * `z2` - 独立標準正規乱数（分散用）
    ///
    /// # Returns
    /// (dW_S, dW_V) - 相関のあるブラウン運動増分
    ///
    /// # Mathematical Background
    /// Using Cholesky decomposition:
    /// dW_S = z1
    /// dW_V = rho * z1 + sqrt(1 - rho^2) * z2
    pub fn generate_correlated_brownian(&self, z1: T, z2: T) -> (T, T) {
        let rho = self.params.rho;
        let eps = self.params.smoothing_epsilon;

        // dW_S = z1 (unchanged)
        let dw_s = z1;

        // dW_V = rho * z1 + sqrt(1 - rho^2) * z2
        let one = T::one();
        let one_minus_rho_sq = one - rho * rho;

        // Use smooth_sqrt to handle rho = +/- 1 case smoothly
        let sqrt_term = smooth_sqrt(one_minus_rho_sq, eps);
        let dw_v = rho * z1 + sqrt_term * z2;

        (dw_s, dw_v)
    }

    /// QE価格ステップ（中間点規則）
    ///
    /// 分散の中間点を使用して価格を更新する。
    ///
    /// # Arguments
    /// * `s_current` - 現在の価格
    /// * `v_current` - 現在の分散
    /// * `v_next` - 次の分散
    /// * `dt` - タイムステップ
    /// * `dw_s` - 価格用ブラウン運動増分
    ///
    /// # Returns
    /// 次の価格 S_{t+dt}
    ///
    /// # Mathematical Background
    /// Mid-point discretization for log-price:
    /// ln(S_{t+dt}) = ln(S_t) + (r - V_avg/2) * dt + sqrt(V_avg * dt) * dW_S
    /// where V_avg = (V_t + V_{t+dt}) / 2
    pub fn qe_price_step(&self, s_current: T, v_current: T, v_next: T, dt: T, dw_s: T) -> T {
        let rate = self.params.rate;
        let eps = self.params.smoothing_epsilon;

        // Mid-point variance averaging
        let two = T::from(2.0).unwrap_or(T::one() + T::one());
        let v_avg = (v_current + v_next) / two;

        // Ensure positive variance using smooth_max
        let v_avg_safe = smooth_max(v_avg, eps, eps);

        // Log-price dynamics:
        // d(ln S) = (r - V/2) * dt + sqrt(V) * dW_S
        let drift = (rate - v_avg_safe / two) * dt;
        let volatility = smooth_sqrt(v_avg_safe, eps);
        let diffusion = volatility * dt.sqrt() * dw_s;

        // S_{t+dt} = S_t * exp(drift + diffusion)
        let log_return = drift + diffusion;
        let s_next = s_current * log_return.exp();

        // Ensure positive price using smooth_max
        smooth_max(s_next, eps, eps)
    }

    /// QE離散化の1ステップ（分散 + 価格）
    ///
    /// Andersen (2008) QEスキームによる完全な1ステップシミュレーション。
    ///
    /// # Arguments
    /// * `s_current` - 現在の価格
    /// * `v_current` - 現在の分散
    /// * `dt` - タイムステップ
    /// * `z1` - 標準正規乱数（価格用）
    /// * `z2` - 標準正規乱数（分散用）
    /// * `uv` - 一様乱数（QEスキーム用）
    ///
    /// # Returns
    /// (S_{t+dt}, V_{t+dt}) - 次のステップの価格と分散
    ///
    /// # Example
    /// ```
    /// use pricer_models::models::heston::{HestonModel, HestonParams};
    ///
    /// let params = HestonParams::new(100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
    /// let model = HestonModel::new(params).unwrap();
    ///
    /// let (s_next, v_next) = model.qe_step(100.0, 0.04, 1.0/252.0, 0.5, 0.0, 0.5);
    /// assert!(s_next > 0.0);
    /// assert!(v_next >= 0.0);
    /// ```
    pub fn qe_step(
        &self,
        s_current: T,
        v_current: T,
        dt: T,
        z1: T,
        z2: T,
        uv: T,
    ) -> (T, T) {
        // Step 1: Update variance using QE scheme
        let v_next = self.qe_variance_step(v_current, dt, uv);

        // Step 2: Generate correlated Brownian motion
        let (dw_s, _dw_v) = self.generate_correlated_brownian(z1, z2);

        // Step 3: Update price using mid-point rule
        let s_next = self.qe_price_step(s_current, v_current, v_next, dt, dw_s);

        (s_next, v_next)
    }

    /// 標準正規分布の逆CDF（近似）
    ///
    /// Beasley-Springer-Moro近似を使用して一様乱数を標準正規乱数に変換。
    ///
    /// # Arguments
    /// * `u` - 一様乱数 U ~ Uniform(0, 1)
    ///
    /// # Returns
    /// Z ~ N(0, 1)
    fn inverse_normal_cdf(&self, u: T) -> T {
        // Beasley-Springer-Moro algorithm coefficients
        let a = [
            T::from(2.50662823884).unwrap_or(T::zero()),
            T::from(-18.61500062529).unwrap_or(T::zero()),
            T::from(41.39119773534).unwrap_or(T::zero()),
            T::from(-25.44106049637).unwrap_or(T::zero()),
        ];
        let b = [
            T::from(-8.47351093090).unwrap_or(T::zero()),
            T::from(23.08336743743).unwrap_or(T::zero()),
            T::from(-21.06224101826).unwrap_or(T::zero()),
            T::from(3.13082909833).unwrap_or(T::zero()),
        ];
        let c = [
            T::from(0.3374754822726147).unwrap_or(T::zero()),
            T::from(0.9761690190917186).unwrap_or(T::zero()),
            T::from(0.1607979714918209).unwrap_or(T::zero()),
            T::from(0.0276438810333863).unwrap_or(T::zero()),
            T::from(0.0038405729373609).unwrap_or(T::zero()),
            T::from(0.0003951896511919).unwrap_or(T::zero()),
            T::from(0.0000321767881768).unwrap_or(T::zero()),
            T::from(0.0000002888167364).unwrap_or(T::zero()),
            T::from(0.0000003960315187).unwrap_or(T::zero()),
        ];

        let half = T::from(0.5).unwrap_or(T::zero());
        let eps = self.params.smoothing_epsilon;

        // Clamp u to (eps, 1-eps) to avoid infinities
        let one = T::one();
        let u_clamped = smooth_max(smooth_max(u, eps, eps), one - eps, eps);
        let u_safe = if u_clamped > one - eps {
            one - eps
        } else if u_clamped < eps {
            eps
        } else {
            u_clamped
        };

        let y = u_safe - half;

        // Central region: |y| <= 0.42
        let threshold = T::from(0.42).unwrap_or(half);
        let y_abs = if y < T::zero() { -y } else { y };

        if y_abs <= threshold {
            // Rational approximation for central region
            let r = y * y;
            let numer = a[0] + r * (a[1] + r * (a[2] + r * a[3]));
            let denom = one + r * (b[0] + r * (b[1] + r * (b[2] + r * b[3])));
            y * numer / denom
        } else {
            // Tail approximation
            let r = if y < T::zero() { u_safe } else { one - u_safe };
            let r_safe = smooth_max(r, eps, eps);
            let s = (-r_safe.ln()).ln();

            let z = c[0]
                + s * (c[1]
                    + s * (c[2]
                        + s * (c[3]
                            + s * (c[4] + s * (c[5] + s * (c[6] + s * (c[7] + s * c[8])))))));

            if y < T::zero() {
                -z
            } else {
                z
            }
        }
    }
}

/// DifferentiableマーカートレイトをHestonModelに実装
///
/// これによりStochasticModelトレイトの実装が可能になる。
impl<T: Float> Differentiable for HestonModel<T> {}

// ================================================================
// Task 3.4: StochasticModelトレイト実装
// ================================================================

use super::stochastic::{StochasticModel, TwoFactorState};

/// HestonModelのStochasticModelトレイト実装
///
/// Hestonモデルは2ファクターモデルとして実装される:
/// - State: `TwoFactorState<T>` - (価格, 分散)
/// - Params: `HestonParams<T>` - モデルパラメータ
/// - brownian_dim: 2 (相関ブラウン運動用に2次元 + 1次元一様乱数)
///
/// # evolve_stepの入力形式
///
/// `dw`スライスは以下の3要素を期待:
/// - `dw[0]`: 標準正規乱数 z1 (価格用)
/// - `dw[1]`: 標準正規乱数 z2 (分散用、相関変換に使用)
/// - `dw[2]`: 一様乱数 uv (QEスキーム用、0-1の範囲)
///
/// # 使用例
///
/// ```
/// use pricer_models::models::heston::{HestonModel, HestonParams};
/// use pricer_models::models::stochastic::StochasticModel;
///
/// let params = HestonParams::new(100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
/// let state = HestonModel::initial_state(&params);
///
/// let dt = 1.0 / 252.0;
/// let dw = [0.5, 0.0, 0.5]; // z1, z2, uv
///
/// let next_state = HestonModel::evolve_step(state, dt, &dw, &params);
/// assert!(next_state.first > 0.0);  // 価格は正
/// assert!(next_state.second >= 0.0); // 分散は非負
/// ```
impl<T: Float + Default> StochasticModel<T> for HestonModel<T> {
    /// 2ファクター状態: (価格, 分散)
    type State = TwoFactorState<T>;

    /// Hestonモデルパラメータ
    type Params = HestonParams<T>;

    /// 1タイムステップの状態遷移（QE離散化スキーム使用）
    ///
    /// # Arguments
    /// * `state` - 現在の状態 (price, variance)
    /// * `dt` - タイムステップ
    /// * `dw` - 乱数スライス: [z1 (normal), z2 (normal), uv (uniform)]
    /// * `params` - モデルパラメータ
    ///
    /// # Returns
    /// 次の状態 (next_price, next_variance)
    fn evolve_step(state: Self::State, dt: T, dw: &[T], params: &Self::Params) -> Self::State {
        // 一時的なモデルインスタンスを作成してQEステップを実行
        // NOTE: HestonModel::newはResultを返すが、paramsは既に検証済みと仮定
        let variance_floor = params.smoothing_epsilon;

        // dw[0] = z1 (price normal)
        // dw[1] = z2 (variance normal)
        // dw[2] = uv (uniform for QE scheme)
        let z1 = dw.first().copied().unwrap_or(T::zero());
        let z2 = dw.get(1).copied().unwrap_or(T::zero());
        let uv = dw.get(2).copied().unwrap_or(T::from(0.5).unwrap_or(T::zero()));

        // QEステップを実行
        let model = HestonModel {
            params: *params,
            variance_floor,
        };

        let (next_price, next_variance) =
            model.qe_step(state.first, state.second, dt, z1, z2, uv);

        TwoFactorState {
            first: next_price,
            second: next_variance,
        }
    }

    /// 初期状態を返す (spot, v0)
    ///
    /// # Arguments
    /// * `params` - モデルパラメータ
    ///
    /// # Returns
    /// 初期状態: (spot price, initial variance)
    fn initial_state(params: &Self::Params) -> Self::State {
        TwoFactorState {
            first: params.spot,
            second: params.v0,
        }
    }

    /// 必要なブラウン運動の次元数
    ///
    /// Hestonモデルは2次元のブラウン運動を使用:
    /// - 1次元: 価格過程
    /// - 1次元: 分散過程（相関付き）
    ///
    /// 実際の`dw`スライスは3要素（z1, z2, uv）を期待するが、
    /// brownian_dimは独立なブラウン運動の数を返す。
    fn brownian_dim() -> usize {
        2
    }

    /// モデル名
    fn model_name() -> &'static str {
        "Heston"
    }

    /// 確率ファクター数
    ///
    /// Hestonモデルは2ファクターモデル:
    /// - 1: 資産価格過程
    /// - 2: 分散過程
    fn num_factors() -> usize {
        2
    }
}

// PricingErrorへの変換を実装
impl From<HestonError> for pricer_core::types::PricingError {
    fn from(err: HestonError) -> Self {
        match err {
            HestonError::InvalidSpot(v) => {
                pricer_core::types::PricingError::InvalidInput(format!(
                    "無効なスポット価格: S0 = {}",
                    v
                ))
            }
            HestonError::InvalidV0(v) => {
                pricer_core::types::PricingError::InvalidInput(format!(
                    "無効な初期分散: v0 = {}",
                    v
                ))
            }
            HestonError::InvalidTheta(v) => {
                pricer_core::types::PricingError::InvalidInput(format!(
                    "無効な長期分散: theta = {}",
                    v
                ))
            }
            HestonError::InvalidKappa(v) => {
                pricer_core::types::PricingError::InvalidInput(format!(
                    "無効な平均回帰速度: kappa = {}",
                    v
                ))
            }
            HestonError::InvalidXi(v) => {
                pricer_core::types::PricingError::InvalidInput(format!(
                    "無効なvol-of-vol: xi = {}",
                    v
                ))
            }
            HestonError::InvalidRho(v) => {
                pricer_core::types::PricingError::InvalidInput(format!(
                    "無効な相関係数: rho = {}",
                    v
                ))
            }
            HestonError::InvalidMaturity(v) => {
                pricer_core::types::PricingError::InvalidInput(format!(
                    "無効な満期: T = {}",
                    v
                ))
            }
            HestonError::InvalidPsiC(v) => {
                pricer_core::types::PricingError::InvalidInput(format!(
                    "無効なQE閾値: psi_c = {}",
                    v
                ))
            }
            HestonError::InvalidEpsilon(v) => {
                pricer_core::types::PricingError::InvalidInput(format!(
                    "無効なsmoothing epsilon: {}",
                    v
                ))
            }
            HestonError::NumericalInstability(msg) => {
                pricer_core::types::PricingError::NumericalInstability(msg)
            }
            HestonError::NonFinite(msg) => {
                pricer_core::types::PricingError::NumericalInstability(format!(
                    "非有限値検出: {}",
                    msg
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 3.1: HestonParams構造体とエラー型のテスト (TDD)
    // ================================================================

    // テスト: HestonError型の定義とDisplay実装

    #[test]
    fn test_heston_error_invalid_spot_display() {
        let err = HestonError::InvalidSpot(-100.0);
        let msg = format!("{}", err);
        assert!(msg.contains("-100"));
        assert!(msg.contains("スポット価格"));
    }

    #[test]
    fn test_heston_error_invalid_v0_display() {
        let err = HestonError::InvalidV0(-0.04);
        let msg = format!("{}", err);
        assert!(msg.contains("-0.04"));
        assert!(msg.contains("初期分散"));
    }

    #[test]
    fn test_heston_error_invalid_theta_display() {
        let err = HestonError::InvalidTheta(-0.04);
        let msg = format!("{}", err);
        assert!(msg.contains("-0.04"));
        assert!(msg.contains("長期分散"));
    }

    #[test]
    fn test_heston_error_invalid_kappa_display() {
        let err = HestonError::InvalidKappa(-1.5);
        let msg = format!("{}", err);
        assert!(msg.contains("-1.5"));
        assert!(msg.contains("平均回帰速度"));
    }

    #[test]
    fn test_heston_error_invalid_xi_display() {
        let err = HestonError::InvalidXi(-0.3);
        let msg = format!("{}", err);
        assert!(msg.contains("-0.3"));
        assert!(msg.contains("vol-of-vol"));
    }

    #[test]
    fn test_heston_error_invalid_rho_display() {
        let err = HestonError::InvalidRho(1.5);
        let msg = format!("{}", err);
        assert!(msg.contains("1.5"));
        assert!(msg.contains("相関係数"));
    }

    #[test]
    fn test_heston_error_invalid_maturity_display() {
        let err = HestonError::InvalidMaturity(-1.0);
        let msg = format!("{}", err);
        assert!(msg.contains("-1"));
        assert!(msg.contains("満期"));
    }

    #[test]
    fn test_heston_error_numerical_instability_display() {
        let err = HestonError::NumericalInstability("分散が発散".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("分散が発散"));
    }

    #[test]
    fn test_heston_error_non_finite_display() {
        let err = HestonError::NonFinite("価格計算".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("価格計算"));
        assert!(msg.contains("NaN") || msg.contains("Infinity"));
    }

    #[test]
    fn test_heston_error_clone_and_equality() {
        let err1 = HestonError::InvalidSpot(-100.0);
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_heston_error_trait_implementation() {
        let err = HestonError::InvalidSpot(-100.0);
        // Error traitが実装されていることを確認
        let _: &dyn std::error::Error = &err;
    }

    // テスト: HestonParams構造体のフィールド

    #[test]
    fn test_heston_params_new_valid() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0);
        assert!(params.is_ok());
        let p = params.unwrap();
        assert_eq!(p.spot, 100.0);
        assert_eq!(p.v0, 0.04);
        assert_eq!(p.theta, 0.04);
        assert_eq!(p.kappa, 1.5);
        assert_eq!(p.xi, 0.3);
        assert_eq!(p.rho, -0.7);
        assert_eq!(p.rate, 0.05);
        assert_eq!(p.maturity, 1.0);
        assert_eq!(p.psi_c, 1.5); // デフォルト値
        assert!((p.smoothing_epsilon - 1e-8).abs() < 1e-15);
    }

    #[test]
    fn test_heston_params_new_invalid_spot_negative() {
        let params = HestonParams::new(-100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidSpot(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_spot_zero() {
        let params = HestonParams::new(0.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidSpot(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_v0_negative() {
        let params = HestonParams::new(100.0_f64, -0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidV0(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_v0_zero() {
        let params = HestonParams::new(100.0_f64, 0.0, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidV0(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_theta_negative() {
        let params = HestonParams::new(100.0_f64, 0.04, -0.04, 1.5, 0.3, -0.7, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidTheta(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_theta_zero() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.0, 1.5, 0.3, -0.7, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidTheta(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_kappa_negative() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, -1.5, 0.3, -0.7, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidKappa(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_kappa_zero() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 0.0, 0.3, -0.7, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidKappa(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_xi_negative() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, -0.3, -0.7, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidXi(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_xi_zero() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.0, -0.7, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidXi(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_rho_too_large() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, 1.5, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidRho(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_rho_too_small() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -1.5, 0.05, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidRho(_))));
    }

    #[test]
    fn test_heston_params_new_valid_rho_boundaries() {
        // rho = -1.0 は有効
        let params1 = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -1.0, 0.05, 1.0);
        assert!(params1.is_ok());

        // rho = 1.0 は有効
        let params2 = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, 1.0, 0.05, 1.0);
        assert!(params2.is_ok());

        // rho = 0.0 は有効
        let params3 = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, 0.0, 0.05, 1.0);
        assert!(params3.is_ok());
    }

    #[test]
    fn test_heston_params_new_invalid_maturity_negative() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, -1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidMaturity(_))));
    }

    #[test]
    fn test_heston_params_new_invalid_maturity_zero() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 0.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(HestonError::InvalidMaturity(_))));
    }

    #[test]
    fn test_heston_params_default() {
        let params: HestonParams<f64> = Default::default();
        assert_eq!(params.spot, 100.0);
        assert_eq!(params.v0, 0.04);
        assert_eq!(params.theta, 0.04);
        assert_eq!(params.kappa, 1.5);
        assert_eq!(params.xi, 0.3);
        assert_eq!(params.rho, -0.7);
        assert_eq!(params.rate, 0.05);
        assert_eq!(params.maturity, 1.0);
        assert_eq!(params.psi_c, 1.5);
    }

    // テスト: with_psi_cとwith_epsilon

    #[test]
    fn test_heston_params_with_psi_c_valid() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0)
            .unwrap()
            .with_psi_c(2.0);
        assert!(params.is_ok());
        assert_eq!(params.unwrap().psi_c, 2.0);
    }

    #[test]
    fn test_heston_params_with_psi_c_invalid() {
        let result = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0)
            .unwrap()
            .with_psi_c(-1.0);
        assert!(result.is_err());
        assert!(matches!(result, Err(HestonError::InvalidPsiC(_))));
    }

    #[test]
    fn test_heston_params_with_epsilon_valid() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0)
            .unwrap()
            .with_epsilon(1e-6);
        assert!(params.is_ok());
        assert!((params.unwrap().smoothing_epsilon - 1e-6).abs() < 1e-15);
    }

    #[test]
    fn test_heston_params_with_epsilon_invalid() {
        let result = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0)
            .unwrap()
            .with_epsilon(-1e-6);
        assert!(result.is_err());
        assert!(matches!(result, Err(HestonError::InvalidEpsilon(_))));
    }

    // テスト: Feller条件

    #[test]
    fn test_heston_params_satisfies_feller() {
        // 2 * 1.5 * 0.04 = 0.12 > 0.3^2 = 0.09 → Feller条件満たす
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        assert!(params.satisfies_feller());
    }

    #[test]
    fn test_heston_params_violates_feller() {
        // 2 * 0.5 * 0.04 = 0.04 < 0.5^2 = 0.25 → Feller条件満たさない
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 0.5, 0.5, -0.7, 0.05, 1.0).unwrap();
        assert!(!params.satisfies_feller());
    }

    #[test]
    fn test_heston_params_feller_ratio() {
        // Feller比率 = 2 * 1.5 * 0.04 / 0.3^2 = 0.12 / 0.09 = 1.333...
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let ratio = params.feller_ratio();
        assert!((ratio - 4.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_heston_params_feller_ratio_boundary() {
        // Feller条件のちょうど境界: 2 * kappa * theta = xi^2
        // kappa = 1.0, theta = 0.02, xi = 0.2
        // 2 * 1.0 * 0.02 = 0.04 = 0.2^2 = 0.04
        let params = HestonParams::new(100.0_f64, 0.04, 0.02, 1.0, 0.2, -0.7, 0.05, 1.0).unwrap();
        let ratio = params.feller_ratio();
        assert!((ratio - 1.0).abs() < 1e-10);
        // 厳密にはFeller条件は > なので、= では満たさない
        assert!(!params.satisfies_feller());
    }

    // テスト: Clone, Copy, Debug

    #[test]
    fn test_heston_params_clone() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let cloned = params.clone();
        assert_eq!(params, cloned);
    }

    #[test]
    fn test_heston_params_copy() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let copied = params; // Copy
        assert_eq!(params.spot, copied.spot);
    }

    #[test]
    fn test_heston_params_debug() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let debug_str = format!("{:?}", params);
        assert!(debug_str.contains("HestonParams"));
        assert!(debug_str.contains("100"));
    }

    // テスト: f32互換性

    #[test]
    fn test_heston_params_f32_support() {
        let params = HestonParams::new(100.0_f32, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0);
        assert!(params.is_ok());
        let p = params.unwrap();
        assert_eq!(p.spot, 100.0_f32);
    }

    // テスト: PricingErrorへの変換

    #[test]
    fn test_heston_error_to_pricing_error_invalid_spot() {
        let heston_err = HestonError::InvalidSpot(-100.0);
        let pricing_err: pricer_core::types::PricingError = heston_err.into();
        match pricing_err {
            pricer_core::types::PricingError::InvalidInput(msg) => {
                assert!(msg.contains("-100"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_heston_error_to_pricing_error_numerical_instability() {
        let heston_err = HestonError::NumericalInstability("テストエラー".to_string());
        let pricing_err: pricer_core::types::PricingError = heston_err.into();
        match pricing_err {
            pricer_core::types::PricingError::NumericalInstability(msg) => {
                assert!(msg.contains("テストエラー"));
            }
            _ => panic!("Expected NumericalInstability variant"),
        }
    }

    #[test]
    fn test_heston_error_to_pricing_error_non_finite() {
        let heston_err = HestonError::NonFinite("価格".to_string());
        let pricing_err: pricer_core::types::PricingError = heston_err.into();
        match pricing_err {
            pricer_core::types::PricingError::NumericalInstability(msg) => {
                assert!(msg.contains("価格"));
            }
            _ => panic!("Expected NumericalInstability variant"),
        }
    }

    // テスト: 検証メソッド

    #[test]
    fn test_heston_params_validate_valid() {
        let params = HestonParams {
            spot: 100.0_f64,
            v0: 0.04,
            theta: 0.04,
            kappa: 1.5,
            xi: 0.3,
            rho: -0.7,
            rate: 0.05,
            maturity: 1.0,
            psi_c: 1.5,
            smoothing_epsilon: 1e-8,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_heston_params_validate_all_errors() {
        // 各エラーケースをテスト
        let test_cases = vec![
            (
                HestonParams {
                    spot: -1.0_f64,
                    ..Default::default()
                },
                "InvalidSpot",
            ),
            (
                HestonParams {
                    v0: -1.0_f64,
                    ..Default::default()
                },
                "InvalidV0",
            ),
            (
                HestonParams {
                    theta: -1.0_f64,
                    ..Default::default()
                },
                "InvalidTheta",
            ),
            (
                HestonParams {
                    kappa: -1.0_f64,
                    ..Default::default()
                },
                "InvalidKappa",
            ),
            (
                HestonParams {
                    xi: -1.0_f64,
                    ..Default::default()
                },
                "InvalidXi",
            ),
            (
                HestonParams {
                    rho: 2.0_f64,
                    ..Default::default()
                },
                "InvalidRho",
            ),
            (
                HestonParams {
                    maturity: -1.0_f64,
                    ..Default::default()
                },
                "InvalidMaturity",
            ),
        ];

        for (params, expected_error) in test_cases {
            let result = params.validate();
            assert!(
                result.is_err(),
                "Expected error for {}",
                expected_error
            );
        }
    }

    // ================================================================
    // Task 3.2: HestonModel構造体の基本実装テスト (TDD - RED phase)
    // ================================================================

    // テスト: HestonModel構造体の作成
    #[test]
    fn test_heston_model_new_valid() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params);
        assert!(model.is_ok());
    }

    #[test]
    fn test_heston_model_new_invalid_params() {
        // 無効なパラメータでnewを呼ぶとエラー
        let invalid_params = HestonParams {
            spot: -100.0_f64,
            ..Default::default()
        };
        let model = HestonModel::new(invalid_params);
        assert!(model.is_err());
    }

    #[test]
    fn test_heston_model_validate() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        assert!(model.validate().is_ok());
    }

    #[test]
    fn test_heston_model_params_accessor() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        assert_eq!(model.params().spot, 100.0);
        assert_eq!(model.params().v0, 0.04);
    }

    // テスト: Feller条件チェック
    #[test]
    fn test_heston_model_check_feller_condition_satisfied() {
        // 2 * 1.5 * 0.04 = 0.12 > 0.3^2 = 0.09 → 満たす
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        assert!(model.check_feller_condition());
    }

    #[test]
    fn test_heston_model_check_feller_condition_violated() {
        // 2 * 0.5 * 0.04 = 0.04 < 0.5^2 = 0.25 → 満たさない
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 0.5, 0.5, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        assert!(!model.check_feller_condition());
    }

    // テスト: Feller条件違反時の警告とフロア適用
    #[test]
    fn test_heston_model_new_with_feller_warning() {
        // Feller条件を満たさないパラメータでも作成可能だが、警告が出る
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 0.5, 0.5, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params);
        assert!(model.is_ok()); // 警告は出るが、作成は成功
        let m = model.unwrap();
        assert!(!m.check_feller_condition()); // Feller条件は満たさない
    }

    // テスト: 分散フロアの適用
    #[test]
    fn test_heston_model_variance_floor() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        // 分散フロアはデフォルトで1e-8
        assert!(model.variance_floor() > 0.0);
    }

    // テスト: ジェネリックFloat型サポート
    #[test]
    fn test_heston_model_generic_f32() {
        let params = HestonParams::new(100.0_f32, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params);
        assert!(model.is_ok());
    }

    // テスト: Clone, Debug
    #[test]
    fn test_heston_model_clone() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model1 = HestonModel::new(params).unwrap();
        let model2 = model1.clone();
        assert_eq!(model1.params().spot, model2.params().spot);
    }

    #[test]
    fn test_heston_model_debug() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let debug_str = format!("{:?}", model);
        assert!(debug_str.contains("HestonModel"));
    }

    // テスト: Differentiableマーカートレイト
    #[test]
    fn test_heston_model_differentiable() {
        use pricer_core::traits::priceable::Differentiable;
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        // Differentiableトレイトが実装されていることを確認
        let _: &dyn Differentiable = &model;
    }

    // テスト: 境界条件の処理
    #[test]
    fn test_heston_model_boundary_rho_minus_one() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -1.0, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params);
        assert!(model.is_ok());
    }

    #[test]
    fn test_heston_model_boundary_rho_plus_one() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, 1.0, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params);
        assert!(model.is_ok());
    }

    #[test]
    fn test_heston_model_boundary_rho_zero() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, 0.0, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params);
        assert!(model.is_ok());
    }

    // ================================================================
    // Task 3.3: QE離散化スキームのテスト (TDD - RED phase)
    // ================================================================

    // テスト: QEモーメント計算
    #[test]
    fn test_qe_moments_computation() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let v_current = 0.04_f64;
        let dt = 1.0 / 252.0;

        let (m, s2, psi) = model.compute_qe_moments(v_current, dt);

        // m (conditional mean) should be positive
        assert!(m > 0.0, "QE mean m = {} should be positive", m);

        // s2 (conditional variance) should be non-negative
        assert!(s2 >= 0.0, "QE variance s2 = {} should be non-negative", s2);

        // psi = s2/m^2 should be non-negative
        assert!(psi >= 0.0, "QE psi = {} should be non-negative", psi);
    }

    #[test]
    fn test_qe_moments_mean_reversion_effect() {
        // Test that variance mean-reverts toward theta
        let params = HestonParams::new(100.0_f64, 0.01, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let dt = 1.0 / 252.0;

        // v0 < theta: mean should be > v0 (reverting up)
        let (m_low, _, _) = model.compute_qe_moments(0.01, dt);
        assert!(m_low > 0.01, "Mean should revert up when v < theta");

        // v0 > theta: mean should be < v0 (reverting down)
        let (m_high, _, _) = model.compute_qe_moments(0.08, dt);
        assert!(m_high < 0.08, "Mean should revert down when v > theta");
    }

    // テスト: 二次スキーム（psi < psi_c）
    #[test]
    fn test_qe_quadratic_scheme() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let v_current = 0.04_f64;
        let dt = 1.0 / 252.0;

        // Get QE moments
        let (m, s2, psi) = model.compute_qe_moments(v_current, dt);

        // Verify we're in quadratic regime (psi < psi_c typically)
        // For typical parameters, psi should be small
        if psi < 1.5 {
            // Apply quadratic scheme
            let uv = 0.5_f64; // uniform random (median value)
            let v_next = model.qe_quadratic_step(m, s2, psi, uv);

            // Result should be positive
            assert!(v_next >= 0.0, "QE quadratic v_next = {} should be non-negative", v_next);

            // Result should be in reasonable range
            assert!(
                v_next < 1.0,
                "QE quadratic v_next = {} should be reasonable",
                v_next
            );
        }
    }

    // テスト: 指数スキーム（psi >= psi_c）
    #[test]
    fn test_qe_exponential_scheme() {
        // Create params with high vol-of-vol to get high psi
        let params = HestonParams::new(100.0_f64, 0.001, 0.04, 0.5, 0.8, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let v_current = 0.001_f64; // small variance
        let dt = 1.0 / 12.0; // larger time step

        // Get QE moments (s2 used for validation only)
        let (m, _s2, psi) = model.compute_qe_moments(v_current, dt);

        // Apply exponential scheme (works for any psi)
        let uv = 0.5_f64; // uniform random
        let v_next = model.qe_exponential_step(m, psi, uv);

        // Result should be non-negative
        assert!(v_next >= 0.0, "QE exponential v_next = {} should be non-negative", v_next);
    }

    // テスト: smooth_indicatorによる滑らかなスキーム切り替え
    #[test]
    fn test_qe_smooth_scheme_switching() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let dt = 1.0 / 252.0;

        // Test variance evolution with smooth blending
        let v_current = 0.04_f64;
        let uv = 0.5_f64;

        let v_next = model.qe_variance_step(v_current, dt, uv);

        // Should be positive due to smooth_max
        assert!(v_next >= 0.0, "QE variance v_next = {} should be non-negative", v_next);

        // Should be in reasonable range
        assert!(
            v_next < 1.0 && v_next > 0.0,
            "QE variance v_next = {} should be in reasonable range",
            v_next
        );
    }

    // テスト: 分散の非負性保証（smooth_max使用）
    #[test]
    fn test_qe_variance_positivity_smooth_max() {
        let params = HestonParams::new(100.0_f64, 0.001, 0.04, 0.5, 0.8, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let dt = 1.0 / 252.0;

        // Test with various uniform randoms
        for uv in [0.01, 0.1, 0.5, 0.9, 0.99] {
            let v_next = model.qe_variance_step(0.001, dt, uv);
            assert!(
                v_next >= 0.0,
                "Variance should be non-negative for uv={}, got {}",
                uv,
                v_next
            );
        }
    }

    // テスト: 相関ブラウン運動生成（Cholesky分解）
    #[test]
    fn test_correlated_brownian_generation() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();

        // Independent standard normals
        let z1 = 0.5_f64;
        let z2 = -0.3_f64;

        let (dw_s, dw_v) = model.generate_correlated_brownian(z1, z2);

        // dW_S = z1 (unchanged)
        assert!((dw_s - z1).abs() < 1e-10, "dW_S should equal z1");

        // dW_V = rho * z1 + sqrt(1 - rho^2) * z2
        let rho = -0.7_f64;
        let expected_dw_v = rho * z1 + (1.0 - rho * rho).sqrt() * z2;
        assert!(
            (dw_v - expected_dw_v).abs() < 1e-10,
            "dW_V should follow Cholesky: expected {}, got {}",
            expected_dw_v,
            dw_v
        );
    }

    #[test]
    fn test_correlated_brownian_extreme_rho() {
        // Test with rho = -1.0 (perfect negative correlation)
        let params_neg =
            HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -1.0, 0.05, 1.0).unwrap();
        let model_neg = HestonModel::new(params_neg).unwrap();

        let z1 = 1.0_f64;
        let z2 = 0.5_f64;
        let (dw_s, dw_v) = model_neg.generate_correlated_brownian(z1, z2);

        // With rho = -1, dW_V = -z1
        assert!((dw_s - z1).abs() < 1e-10);
        assert!((dw_v - (-z1)).abs() < 1e-10);

        // Test with rho = 1.0 (perfect positive correlation)
        let params_pos =
            HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, 1.0, 0.05, 1.0).unwrap();
        let model_pos = HestonModel::new(params_pos).unwrap();

        let (dw_s2, dw_v2) = model_pos.generate_correlated_brownian(z1, z2);

        // With rho = 1, dW_V = z1
        assert!((dw_s2 - z1).abs() < 1e-10);
        assert!((dw_v2 - z1).abs() < 1e-10);
    }

    #[test]
    fn test_correlated_brownian_zero_rho() {
        // Test with rho = 0 (independent)
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, 0.0, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();

        let z1 = 0.5_f64;
        let z2 = -0.3_f64;
        let (dw_s, dw_v) = model.generate_correlated_brownian(z1, z2);

        // With rho = 0, dW_S = z1, dW_V = z2
        assert!((dw_s - z1).abs() < 1e-10);
        assert!((dw_v - z2).abs() < 1e-10);
    }

    // テスト: QEステップ全体（分散 + 価格）
    #[test]
    fn test_qe_full_step() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();

        let s_current = 100.0_f64;
        let v_current = 0.04_f64;
        let dt = 1.0 / 252.0;
        let z1 = 0.5_f64;  // normal for price
        let z2 = -0.3_f64; // normal for variance
        let uv = 0.5_f64;  // uniform for variance scheme

        let (s_next, v_next) = model.qe_step(s_current, v_current, dt, z1, z2, uv);

        // Price should be positive
        assert!(s_next > 0.0, "Price s_next = {} should be positive", s_next);

        // Variance should be non-negative
        assert!(v_next >= 0.0, "Variance v_next = {} should be non-negative", v_next);

        // Price should be in reasonable range (within 50% of original for small dt)
        assert!(
            s_next > s_current * 0.5 && s_next < s_current * 1.5,
            "Price change should be reasonable: s_next = {}",
            s_next
        );
    }

    #[test]
    fn test_qe_full_step_multiple_paths() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();

        let dt = 1.0 / 252.0;
        let s0 = 100.0_f64;
        let v0 = 0.04_f64;

        // Simulate multiple steps
        let test_cases = [
            (0.5, 0.0, 0.5),    // neutral
            (1.5, 0.5, 0.9),    // positive shock
            (-1.5, -0.5, 0.1),  // negative shock
            (0.0, 0.0, 0.5),    // zero shock
        ];

        for (z1, z2, uv) in test_cases {
            let (s_next, v_next) = model.qe_step(s0, v0, dt, z1, z2, uv);

            assert!(s_next > 0.0, "Price should be positive for z1={}, z2={}", z1, z2);
            assert!(v_next >= 0.0, "Variance should be non-negative for z1={}, z2={}", z1, z2);
        }
    }

    // テスト: smooth_sqrt使用の検証
    #[test]
    fn test_qe_uses_smooth_sqrt() {
        // Test that QE step handles near-zero variance smoothly
        let params = HestonParams::new(100.0_f64, 0.001, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0)
            .unwrap()
            .with_epsilon(1e-6)
            .unwrap();
        let model = HestonModel::new(params).unwrap();

        let s0 = 100.0_f64;
        let v0 = 1e-8_f64; // Very small variance
        let dt = 1.0 / 252.0;

        // Should not panic or produce NaN
        let (s_next, v_next) = model.qe_step(s0, v0, dt, 0.5, 0.0, 0.5);

        assert!(
            s_next.is_finite(),
            "Price should be finite for small v0, got {}",
            s_next
        );
        assert!(
            v_next.is_finite(),
            "Variance should be finite for small v0, got {}",
            v_next
        );
    }

    // テスト: モーメント整合性（統計テスト）
    #[test]
    fn test_qe_moments_consistency() {
        // Test that QE scheme preserves correct conditional moments
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let dt = 1.0 / 252.0;
        let v_current = 0.04_f64;

        // Analytical conditional mean: E[V_{t+dt} | V_t] = theta + (v_t - theta) * exp(-kappa * dt)
        let kappa = 1.5_f64;
        let theta = 0.04_f64;
        let exp_factor = (-kappa * dt).exp();
        let analytical_mean = theta + (v_current - theta) * exp_factor;

        // QE moments should match analytical
        let (qe_mean, _, _) = model.compute_qe_moments(v_current, dt);

        assert!(
            (qe_mean - analytical_mean).abs() < 1e-10,
            "QE mean {} should match analytical mean {}",
            qe_mean,
            analytical_mean
        );
    }

    // テスト: psi_c閾値でのスムーズな切り替え
    #[test]
    fn test_qe_smooth_transition_at_psi_c() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let dt = 1.0 / 252.0;

        // Test near the psi_c threshold (psi_c = 1.5 is default)
        // Find variance that gives psi near psi_c
        // This is a numerical search - just test continuity
        let uvs = [0.3, 0.5, 0.7];
        for uv in uvs {
            // Test multiple variances
            for v in [0.01, 0.04, 0.1] {
                let v_next1 = model.qe_variance_step(v, dt, uv);
                let v_next2 = model.qe_variance_step(v * 1.01, dt, uv);

                // Results should be continuous (small change in input -> small change in output)
                let diff = (v_next2 - v_next1).abs();
                assert!(
                    diff < 0.01,
                    "QE should be smooth: v={}, diff={}",
                    v,
                    diff
                );
            }
        }
    }

    // テスト: ジェネリックFloat型サポート
    #[test]
    fn test_qe_generic_f32() {
        let params = HestonParams::new(100.0_f32, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();

        let s0 = 100.0_f32;
        let v0 = 0.04_f32;
        let dt = 1.0_f32 / 252.0;

        let (s_next, v_next) = model.qe_step(s0, v0, dt, 0.5_f32, 0.0_f32, 0.5_f32);

        assert!(s_next > 0.0_f32, "f32 price should be positive");
        assert!(v_next >= 0.0_f32, "f32 variance should be non-negative");
    }

    // テスト: 中間点規則（mid-point rule）による価格更新
    #[test]
    fn test_qe_price_mid_point_rule() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();

        let s0 = 100.0_f64;
        let v_current = 0.04_f64;
        let v_next = 0.05_f64;
        let dt = 1.0 / 252.0;
        let dw_s = 0.5_f64;

        // Mid-point rule uses average variance: (v_current + v_next) / 2
        let s_next = model.qe_price_step(s0, v_current, v_next, dt, dw_s);

        // Price should follow log-normal dynamics
        assert!(s_next > 0.0, "Price should be positive");

        // Verify mid-point averaging is used
        let v_avg = (v_current + v_next) / 2.0;
        let rate = 0.05_f64;
        let expected_drift = (rate - 0.5 * v_avg) * dt;
        let expected_diffusion = v_avg.sqrt() * (dt.sqrt()) * dw_s;
        let expected_s_next = s0 * (expected_drift + expected_diffusion).exp();

        assert!(
            (s_next - expected_s_next).abs() < 1e-10,
            "Price should follow mid-point rule: expected {}, got {}",
            expected_s_next,
            s_next
        );
    }

    // ================================================================
    // Task 3.4: StochasticModelトレイト実装のテスト (TDD - RED phase)
    // ================================================================

    use super::super::stochastic::{StochasticModel, StochasticState, TwoFactorState};

    // テスト: State型が(T, T)として定義されている
    #[test]
    fn test_heston_stochastic_model_state_type() {
        // HestonModelのState型はTwoFactorState<T>（価格、分散）
        let state: <HestonModel<f64> as StochasticModel<f64>>::State = TwoFactorState {
            first: 100.0,
            second: 0.04,
        };
        assert_eq!(state.first, 100.0);
        assert_eq!(state.second, 0.04);
    }

    // テスト: initial_stateがパラメータから正しく初期状態を返す
    #[test]
    fn test_heston_stochastic_model_initial_state() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let state = HestonModel::initial_state(&params);

        // 初期状態は(spot, v0)
        assert_eq!(state.first, 100.0, "Initial price should be spot");
        assert_eq!(state.second, 0.04, "Initial variance should be v0");
    }

    // テスト: brownian_dimが2を返す（価格と分散の2次元ブラウン運動）
    #[test]
    fn test_heston_stochastic_model_brownian_dim() {
        assert_eq!(
            HestonModel::<f64>::brownian_dim(),
            2,
            "Heston requires 2 Brownian dimensions"
        );
    }

    // テスト: model_nameが"Heston"を返す
    #[test]
    fn test_heston_stochastic_model_model_name() {
        assert_eq!(
            HestonModel::<f64>::model_name(),
            "Heston",
            "Model name should be 'Heston'"
        );
    }

    // テスト: num_factorsが2を返す（価格と分散の2ファクター）
    #[test]
    fn test_heston_stochastic_model_num_factors() {
        assert_eq!(
            HestonModel::<f64>::num_factors(),
            2,
            "Heston is a 2-factor model"
        );
    }

    // テスト: evolve_stepが正しく状態を遷移させる
    #[test]
    fn test_heston_stochastic_model_evolve_step() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let state = HestonModel::initial_state(&params);
        let dt = 1.0 / 252.0;

        // dw = [z1, z2, uv] - 2つの正規乱数と1つの一様乱数
        let dw = [0.5_f64, 0.0, 0.5];

        let next_state = HestonModel::evolve_step(state, dt, &dw, &params);

        // 価格は正であること
        assert!(
            next_state.first > 0.0,
            "Price should be positive, got {}",
            next_state.first
        );

        // 分散は非負であること
        assert!(
            next_state.second >= 0.0,
            "Variance should be non-negative, got {}",
            next_state.second
        );
    }

    // テスト: evolve_stepが複数ステップで安定している
    #[test]
    fn test_heston_stochastic_model_evolve_multiple_steps() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let mut state = HestonModel::initial_state(&params);
        let dt = 1.0 / 252.0;

        // 複数ステップをシミュレート
        for _ in 0..252 {
            let dw = [0.0_f64, 0.0, 0.5]; // ゼロショック
            state = HestonModel::evolve_step(state, dt, &dw, &params);

            // 各ステップで有効な値であること
            assert!(state.first > 0.0, "Price should remain positive");
            assert!(state.second >= 0.0, "Variance should remain non-negative");
            assert!(
                state.first.is_finite(),
                "Price should be finite"
            );
            assert!(
                state.second.is_finite(),
                "Variance should be finite"
            );
        }
    }

    // テスト: evolve_stepがポジティブショックで価格を上昇させる
    #[test]
    fn test_heston_stochastic_model_positive_shock() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let state = HestonModel::initial_state(&params);
        let dt = 1.0 / 252.0;

        // 正のショック
        let dw = [2.0_f64, 0.0, 0.5];
        let next_state = HestonModel::evolve_step(state, dt, &dw, &params);

        // 正のショックで価格は上昇するはず
        assert!(
            next_state.first > state.first,
            "Price should increase with positive shock"
        );
    }

    // テスト: evolve_stepがネガティブショックで価格を下落させる
    #[test]
    fn test_heston_stochastic_model_negative_shock() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let state = HestonModel::initial_state(&params);
        let dt = 1.0 / 252.0;

        // 負のショック
        let dw = [-2.0_f64, 0.0, 0.5];
        let next_state = HestonModel::evolve_step(state, dt, &dw, &params);

        // 負のショックで価格は下落するはず
        assert!(
            next_state.first < state.first,
            "Price should decrease with negative shock"
        );
    }

    // テスト: f32型でも動作する
    #[test]
    fn test_heston_stochastic_model_f32() {
        let params = HestonParams::new(100.0_f32, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let state = HestonModel::initial_state(&params);
        let dt = 1.0_f32 / 252.0;

        let dw = [0.5_f32, 0.0, 0.5];
        let next_state = HestonModel::evolve_step(state, dt, &dw, &params);

        assert!(next_state.first > 0.0_f32, "f32 price should be positive");
        assert!(next_state.second >= 0.0_f32, "f32 variance should be non-negative");
    }

    // テスト: TwoFactorStateがStochasticStateトレイトを実装している
    #[test]
    fn test_heston_state_implements_stochastic_state() {
        let state: TwoFactorState<f64> = TwoFactorState {
            first: 100.0,
            second: 0.04,
        };

        // StochasticStateのメソッドが使える
        assert_eq!(TwoFactorState::<f64>::dimension(), 2);
        assert_eq!(state.get(0), Some(100.0));
        assert_eq!(state.get(1), Some(0.04));
        assert_eq!(state.to_array(), vec![100.0, 0.04]);
    }

    // テスト: Params型がHestonParams<T>である
    #[test]
    fn test_heston_stochastic_model_params_type() {
        // Params型の確認（コンパイル時チェック）
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let _: <HestonModel<f64> as StochasticModel<f64>>::Params = params;
    }

    // テスト: Differentiableマーカートレイトが実装されている
    #[test]
    fn test_heston_implements_differentiable() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let _: &dyn Differentiable = &model;
    }

    // テスト: evolve_stepがdwの長さ3を期待する
    #[test]
    fn test_heston_evolve_step_dw_format() {
        let params = HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let state = HestonModel::initial_state(&params);
        let dt = 1.0 / 252.0;

        // dw = [z1 (price normal), z2 (variance normal), uv (uniform for QE)]
        let dw = [0.5_f64, -0.3, 0.5];
        let next_state = HestonModel::evolve_step(state, dt, &dw, &params);

        assert!(next_state.first.is_finite());
        assert!(next_state.second.is_finite());
    }
}
