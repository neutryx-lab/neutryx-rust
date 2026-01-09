//! SABRモデル実装
//!
//! SABR (Stochastic Alpha, Beta, Rho) モデルは以下のSDEで記述される
//! 確率的ボラティリティモデル:
//! ```text
//! dF = alpha * F^beta * dW_F
//! d(alpha) = nu * alpha * dW_alpha
//! E[dW_F * dW_alpha] = rho * dt
//! ```
//! ここで:
//! - F = フォワード価格
//! - alpha = 瞬間ボラティリティ
//! - beta = CEVパラメータ (0: Normal, 1: Lognormal)
//! - nu = ボラティリティのボラティリティ (vol-of-vol)
//! - rho = フォワード価格とボラティリティの相関
//!
//! ## Hagan公式
//!
//! 本実装ではHagan et al. (2002) のインプライドボラティリティ近似公式を使用。
//! ATM近傍では展開公式を使用して数値安定性を確保。
//!
//! ## 使用例
//!
//! ```
//! use pricer_models::models::sabr::{SABRParams, SABRError};
//!
//! // パラメータを作成
//! let params = SABRParams::new(
//!     100.0,  // フォワード価格
//!     0.2,    // 初期ボラティリティ (alpha)
//!     0.4,    // vol-of-vol (nu)
//!     -0.3,   // 相関 (rho)
//!     0.5,    // ベータ
//!     1.0,    // 満期
//! );
//! assert!(params.is_ok());
//! ```

use pricer_core::math::smoothing::{smooth_log, smooth_pow};
use pricer_core::traits::Float;
use thiserror::Error;

/// SABRモデルエラー型
///
/// パラメータ検証と数値計算時のエラーを表現する。
/// `thiserror`クレートを使用して構造化されたエラー情報を提供。
///
/// # バリアント
///
/// - `InvalidForward`: フォワード価格が正でない
/// - `InvalidAlpha`: 初期ボラティリティが正でない
/// - `InvalidNu`: vol-of-volが負
/// - `InvalidBeta`: ベータが[0, 1]の範囲外
/// - `InvalidRho`: 相関が(-1, 1)の範囲外
/// - `InvalidMaturity`: 満期が正でない
/// - `InvalidStrike`: ストライクが正でない
/// - `NegativeImpliedVol`: 負のインプライドボラティリティが計算された
/// - `NumericalInstability`: 数値計算の不安定性
/// - `NonFinite`: NaNまたは無限大が検出された
///
/// # 例
///
/// ```
/// use pricer_models::models::sabr::SABRError;
///
/// let err = SABRError::InvalidAlpha(-0.1);
/// assert!(format!("{}", err).contains("-0.1"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SABRError {
    /// 無効なフォワード価格（正でなければならない）
    #[error("無効なフォワード価格: F = {0} (正の値が必要)")]
    InvalidForward(f64),

    /// 無効な初期ボラティリティ（正でなければならない）
    #[error("無効な初期ボラティリティ: alpha = {0} (正の値が必要)")]
    InvalidAlpha(f64),

    /// 無効なvol-of-vol（非負でなければならない）
    #[error("無効なvol-of-vol: nu = {0} (非負の値が必要)")]
    InvalidNu(f64),

    /// 無効なベータ（0から1の範囲内でなければならない）
    #[error("無効なベータ: beta = {0} ([0, 1]の範囲が必要)")]
    InvalidBeta(f64),

    /// 無効な相関係数（-1から1の開区間内でなければならない）
    #[error("無効な相関係数: rho = {0} ((-1, 1)の範囲が必要)")]
    InvalidRho(f64),

    /// 無効な満期（正でなければならない）
    #[error("無効な満期: T = {0} (正の値が必要)")]
    InvalidMaturity(f64),

    /// 無効なATM閾値（正でなければならない）
    #[error("無効なATM閾値: {0} (正の値が必要)")]
    InvalidAtmThreshold(f64),

    /// 無効なsmoothing epsilon（正でなければならない）
    #[error("無効なsmoothing epsilon: {0} (正の値が必要)")]
    InvalidEpsilon(f64),

    /// 無効なストライク価格（正でなければならない）
    #[error("無効なストライク価格: K = {0} (正の値が必要)")]
    InvalidStrike(f64),

    /// 負のインプライドボラティリティが計算された
    #[error("ストライク {0} で負のインプライドボラティリティが計算されました")]
    NegativeImpliedVol(f64),

    /// 数値的不安定性が検出された
    #[error("数値的不安定性: {0}")]
    NumericalInstability(String),

    /// NaNまたは無限大が検出された
    #[error("{0}でNaNまたはInfinityが検出されました")]
    NonFinite(String),
}

/// SABRモデルパラメータ
///
/// # 型パラメータ
///
/// * `T` - Float型（f64またはAD互換のDualNumber）
///
/// # フィールド
///
/// * `forward` - フォワード価格 (F > 0)
/// * `alpha` - 初期ボラティリティ (alpha > 0)
/// * `nu` - ボラティリティのボラティリティ (nu >= 0)
/// * `rho` - 相関係数 (-1 < rho < 1)
/// * `beta` - CEVパラメータ (0 <= beta <= 1)
/// * `maturity` - 満期までの時間 (T > 0)
/// * `atm_threshold` - ATM近傍判定閾値
/// * `smoothing_epsilon` - smooth approximation用のepsilon
///
/// # 例
///
/// ```
/// use pricer_models::models::sabr::SABRParams;
///
/// let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0);
/// assert!(params.is_ok());
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SABRParams<T: Float> {
    /// フォワード価格 (F)
    pub forward: T,
    /// 初期ボラティリティ (alpha)
    pub alpha: T,
    /// ボラティリティのボラティリティ (nu)
    pub nu: T,
    /// 相関係数 (rho)
    pub rho: T,
    /// CEVパラメータ (beta): 0 = Normal, 1 = Lognormal
    pub beta: T,
    /// 満期までの時間
    pub maturity: T,
    /// ATM近傍判定閾値
    pub atm_threshold: T,
    /// smooth approximation epsilon
    pub smoothing_epsilon: T,
}

impl<T: Float> SABRParams<T> {
    /// 新しいSABRパラメータを作成（検証付き）
    ///
    /// # 引数
    ///
    /// * `forward` - フォワード価格（正でなければならない）
    /// * `alpha` - 初期ボラティリティ（正でなければならない）
    /// * `nu` - vol-of-vol（非負でなければならない）
    /// * `rho` - 相関係数（-1から1の開区間）
    /// * `beta` - CEVパラメータ（0から1の範囲）
    /// * `maturity` - 満期（正でなければならない）
    ///
    /// # 戻り値
    ///
    /// パラメータが有効な場合は`Ok(SABRParams)`、無効な場合は`Err(SABRError)`
    ///
    /// # 例
    ///
    /// ```
    /// use pricer_models::models::sabr::SABRParams;
    ///
    /// // 有効なパラメータ
    /// let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0);
    /// assert!(params.is_ok());
    ///
    /// // 無効なalpha
    /// let invalid = SABRParams::new(100.0, -0.2, 0.4, -0.3, 0.5, 1.0);
    /// assert!(invalid.is_err());
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        forward: T,
        alpha: T,
        nu: T,
        rho: T,
        beta: T,
        maturity: T,
    ) -> Result<Self, SABRError> {
        let params = Self {
            forward,
            alpha,
            nu,
            rho,
            beta,
            maturity,
            atm_threshold: T::from(1e-4).unwrap_or(T::zero()),
            smoothing_epsilon: T::from(1e-8).unwrap_or(T::zero()),
        };
        params.validate()?;
        Ok(params)
    }

    /// カスタムATM閾値を設定
    ///
    /// # 引数
    ///
    /// * `threshold` - ATM近傍判定閾値（正でなければならない）
    ///
    /// # 戻り値
    ///
    /// 更新されたパラメータ
    pub fn with_atm_threshold(mut self, threshold: T) -> Result<Self, SABRError> {
        if threshold <= T::zero() {
            return Err(SABRError::InvalidAtmThreshold(
                threshold.to_f64().unwrap_or(f64::NAN),
            ));
        }
        self.atm_threshold = threshold;
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
    pub fn with_epsilon(mut self, epsilon: T) -> Result<Self, SABRError> {
        if epsilon <= T::zero() {
            return Err(SABRError::InvalidEpsilon(
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
    /// パラメータが有効な場合は`Ok(())`、無効な場合は`Err(SABRError)`
    pub fn validate(&self) -> Result<(), SABRError> {
        // フォワード価格は正でなければならない
        if self.forward <= T::zero() {
            return Err(SABRError::InvalidForward(
                self.forward.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // 初期ボラティリティは正でなければならない
        if self.alpha <= T::zero() {
            return Err(SABRError::InvalidAlpha(
                self.alpha.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // vol-of-volは非負でなければならない
        if self.nu < T::zero() {
            return Err(SABRError::InvalidNu(
                self.nu.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // ベータは[0, 1]の範囲でなければならない
        if self.beta < T::zero() || self.beta > T::one() {
            return Err(SABRError::InvalidBeta(
                self.beta.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // 相関は(-1, 1)の開区間でなければならない
        if self.rho <= -T::one() || self.rho >= T::one() {
            return Err(SABRError::InvalidRho(
                self.rho.to_f64().unwrap_or(f64::NAN),
            ));
        }

        // 満期は正でなければならない
        if self.maturity <= T::zero() {
            return Err(SABRError::InvalidMaturity(
                self.maturity.to_f64().unwrap_or(f64::NAN),
            ));
        }

        Ok(())
    }

    /// Normal SABRモード (beta = 0) かどうか
    ///
    /// # 戻り値
    ///
    /// beta = 0 の場合は true
    pub fn is_normal(&self) -> bool {
        self.beta.abs() < self.smoothing_epsilon
    }

    /// Lognormal SABRモード (beta = 1) かどうか
    ///
    /// # 戻り値
    ///
    /// beta = 1 の場合は true
    pub fn is_lognormal(&self) -> bool {
        (self.beta - T::one()).abs() < self.smoothing_epsilon
    }

    /// フォワード価格を取得
    pub fn forward(&self) -> T {
        self.forward
    }

    /// 初期ボラティリティを取得
    pub fn alpha(&self) -> T {
        self.alpha
    }

    /// vol-of-volを取得
    pub fn nu(&self) -> T {
        self.nu
    }

    /// 相関係数を取得
    pub fn rho(&self) -> T {
        self.rho
    }

    /// ベータを取得
    pub fn beta(&self) -> T {
        self.beta
    }

    /// 満期を取得
    pub fn maturity(&self) -> T {
        self.maturity
    }

    /// ATM閾値を取得
    pub fn atm_threshold(&self) -> T {
        self.atm_threshold
    }

    /// smoothing epsilonを取得
    pub fn smoothing_epsilon(&self) -> T {
        self.smoothing_epsilon
    }
}

/// SABRモデル（インプライドボラティリティ計算用）
///
/// Hagan et al. (2002) のインプライドボラティリティ近似公式を実装。
/// ATM近傍では展開公式を使用して数値安定性を確保する。
///
/// # 型パラメータ
///
/// * `T` - Float型（f64またはAD互換のDualNumber）
///
/// # Hagan公式
///
/// インプライドボラティリティは以下の式で近似される:
/// ```text
/// σ_B(K,F) = α / [(FK)^((1-β)/2) * D(F/K)]
///            × (z/x(z))
///            × [1 + expansion_terms * T]
/// ```
/// ここで:
/// - D(F/K) = 1 + ((1-β)²/24)*ln²(F/K) + ((1-β)⁴/1920)*ln⁴(F/K)
/// - z = (ν/α)*(FK)^((1-β)/2)*ln(F/K)
/// - x(z) = ln((√(1-2ρz+z²)+z-ρ)/(1-ρ))
///
/// # 例
///
/// ```
/// use pricer_models::models::sabr::{SABRModel, SABRParams};
///
/// let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
/// let model = SABRModel::new(params).unwrap();
///
/// // ATMインプライドボラティリティ
/// let atm_vol = model.atm_vol();
/// assert!(atm_vol > 0.0);
///
/// // OTMストライクのインプライドボラティリティ
/// let otm_vol = model.implied_vol(110.0);
/// assert!(otm_vol.is_ok());
/// ```
#[derive(Clone, Debug)]
pub struct SABRModel<T: Float> {
    /// モデルパラメータ
    params: SABRParams<T>,
}

impl<T: Float> SABRModel<T> {
    /// 新しいSABRモデルを作成
    ///
    /// # 引数
    ///
    /// * `params` - SABRパラメータ
    ///
    /// # 戻り値
    ///
    /// 検証済みのSABRモデル
    ///
    /// # エラー
    ///
    /// パラメータが無効な場合はエラーを返す
    pub fn new(params: SABRParams<T>) -> Result<Self, SABRError> {
        params.validate()?;
        Ok(Self { params })
    }

    /// パラメータへの参照を取得
    pub fn params(&self) -> &SABRParams<T> {
        &self.params
    }

    /// ATM（At-The-Money）インプライドボラティリティを計算
    ///
    /// ATMではストライク K = フォワード F なので、Hagan公式は簡略化される:
    /// ```text
    /// σ_ATM = α / F^(1-β) * [1 + expansion_terms * T]
    /// ```
    ///
    /// # 戻り値
    ///
    /// ATMインプライドボラティリティ
    pub fn atm_vol(&self) -> T {
        let f = self.params.forward;
        let alpha = self.params.alpha;
        let beta = self.params.beta;
        let nu = self.params.nu;
        let rho = self.params.rho;
        let t = self.params.maturity;
        let eps = self.params.smoothing_epsilon;

        let one = T::one();
        let one_minus_beta = one - beta;

        // F^(1-β) using smooth_pow for AD compatibility
        let f_pow = smooth_pow(f, one_minus_beta, eps);

        // Base term: α / F^(1-β)
        let base = alpha / f_pow;

        // Higher order expansion terms
        // term1 = (1-β)² / 24 * α² / F^(2(1-β))
        let two = T::from(2.0).unwrap();
        let twenty_four = T::from(24.0).unwrap();

        let f_pow_2 = smooth_pow(f, two * one_minus_beta, eps);
        let term1 = one_minus_beta * one_minus_beta / twenty_four * alpha * alpha / f_pow_2;

        // term2 = ρ * β * ν * α / (4 * F^(1-β))
        let four = T::from(4.0).unwrap();
        let term2 = rho * beta * nu * alpha / (four * f_pow);

        // term3 = (2 - 3ρ²) / 24 * ν²
        let three = T::from(3.0).unwrap();
        let term3 = (two - three * rho * rho) / twenty_four * nu * nu;

        // Total expansion
        let expansion = one + (term1 + term2 + term3) * t;

        base * expansion
    }

    /// 任意のストライクに対するインプライドボラティリティを計算
    ///
    /// Hagan et al. (2002) の近似公式を使用。
    /// ATM近傍（|ln(F/K)| < atm_threshold）では展開公式を使用して
    /// 数値安定性を確保する。
    ///
    /// beta=0の場合はNormal SABR公式、beta=1の場合はLognormal SABR公式を使用。
    ///
    /// # 引数
    ///
    /// * `strike` - ストライク価格（正でなければならない）
    ///
    /// # 戻り値
    ///
    /// インプライドボラティリティ（Black-Scholes volatility）
    ///
    /// # エラー
    ///
    /// - `InvalidStrike`: ストライクが正でない
    /// - `NegativeImpliedVol`: 負のボラティリティが計算された
    /// - `NonFinite`: NaN/Infinityが検出された
    pub fn implied_vol(&self, strike: T) -> Result<T, SABRError> {
        // ストライク検証
        if strike <= T::zero() {
            return Err(SABRError::InvalidStrike(
                strike.to_f64().unwrap_or(f64::NAN),
            ));
        }

        let f = self.params.forward;
        let log_fk = smooth_log(f / strike, self.params.smoothing_epsilon);

        // ATM近傍判定
        let vol = if log_fk.abs() < self.params.atm_threshold {
            self.implied_vol_atm_expansion(strike)
        } else if self.params.is_normal() {
            // Normal SABR (beta=0) specialized formula
            self.implied_vol_normal(strike)
        } else if self.params.is_lognormal() {
            // Lognormal SABR (beta=1) specialized formula
            self.implied_vol_lognormal(strike)
        } else {
            // General Hagan formula
            self.implied_vol_hagan(strike)
        };

        // 結果の検証
        if !vol.is_finite() {
            return Err(SABRError::NonFinite("implied_vol".to_string()));
        }

        // 負のボラティリティチェック
        if vol <= T::zero() {
            return Err(SABRError::NegativeImpliedVol(
                strike.to_f64().unwrap_or(f64::NAN),
            ));
        }

        Ok(vol)
    }

    /// ATM近傍の展開公式によるインプライドボラティリティ計算
    ///
    /// K ≈ F の場合、Hagan公式の特異点を回避するために
    /// Taylor展開を使用する。
    fn implied_vol_atm_expansion(&self, strike: T) -> T {
        let f = self.params.forward;
        let alpha = self.params.alpha;
        let beta = self.params.beta;
        let nu = self.params.nu;
        let rho = self.params.rho;
        let t = self.params.maturity;
        let eps = self.params.smoothing_epsilon;

        let one = T::one();
        let two = T::from(2.0).unwrap();
        let three = T::from(3.0).unwrap();
        let four = T::from(4.0).unwrap();
        let twenty_four = T::from(24.0).unwrap();

        let one_minus_beta = one - beta;

        // 幾何平均 (F*K)^((1-β)/2)
        let fk = f * strike;
        let fk_pow = smooth_pow(fk, one_minus_beta / two, eps);

        // Base term: α / (FK)^((1-β)/2)
        let base = alpha / fk_pow;

        // Higher order expansion terms
        // term1 = (1-β)² / 24 * α² / (FK)^(1-β)
        let fk_pow_full = smooth_pow(fk, one_minus_beta, eps);
        let term1 = one_minus_beta * one_minus_beta / twenty_four * alpha * alpha / fk_pow_full;

        // term2 = ρ * β * ν * α / (4 * (FK)^((1-β)/2))
        let term2 = rho * beta * nu * alpha / (four * fk_pow);

        // term3 = (2 - 3ρ²) / 24 * ν²
        let term3 = (two - three * rho * rho) / twenty_four * nu * nu;

        // Total expansion
        let expansion = one + (term1 + term2 + term3) * t;

        base * expansion
    }

    /// 一般的なHagan公式によるインプライドボラティリティ計算
    ///
    /// 完全なHagan et al. (2002) 公式:
    /// σ_B(K,F) = α / [(FK)^((1-β)/2) * D] × (z/x(z)) × [1 + expansion * T]
    fn implied_vol_hagan(&self, strike: T) -> T {
        let f = self.params.forward;
        let k = strike;
        let alpha = self.params.alpha;
        let beta = self.params.beta;
        let nu = self.params.nu;
        let rho = self.params.rho;
        let t = self.params.maturity;
        let eps = self.params.smoothing_epsilon;

        let one = T::one();
        let two = T::from(2.0).unwrap();
        let three = T::from(3.0).unwrap();
        let four = T::from(4.0).unwrap();
        let twenty_four = T::from(24.0).unwrap();
        let one_thousand_nine_twenty = T::from(1920.0).unwrap();

        let one_minus_beta = one - beta;

        // ln(F/K)
        let log_fk = smooth_log(f / k, eps);

        // (FK)^((1-β)/2)
        let fk = f * k;
        let fk_pow_half = smooth_pow(fk, one_minus_beta / two, eps);

        // D(F/K) = 1 + ((1-β)²/24)*ln²(F/K) + ((1-β)⁴/1920)*ln⁴(F/K)
        let log_fk_2 = log_fk * log_fk;
        let log_fk_4 = log_fk_2 * log_fk_2;
        let one_minus_beta_2 = one_minus_beta * one_minus_beta;
        let one_minus_beta_4 = one_minus_beta_2 * one_minus_beta_2;

        let d_term1 = one_minus_beta_2 / twenty_four * log_fk_2;
        let d_term2 = one_minus_beta_4 / one_thousand_nine_twenty * log_fk_4;
        let d = one + d_term1 + d_term2;

        // z = (ν/α) * (FK)^((1-β)/2) * ln(F/K)
        let z = (nu / alpha) * fk_pow_half * log_fk;

        // x(z) = ln((√(1-2ρz+z²) + z - ρ) / (1-ρ))
        let x_z = self.compute_x_of_z(z, rho);

        // z/x(z) coefficient
        let z_over_x = if z.abs() < eps {
            // z → 0 のとき z/x(z) → 1
            one
        } else {
            z / x_z
        };

        // Base term: α / [(FK)^((1-β)/2) * D]
        let base = alpha / (fk_pow_half * d);

        // Higher order expansion terms (at FK midpoint)
        let fk_pow_full = smooth_pow(fk, one_minus_beta, eps);

        // term1 = (1-β)² / 24 * α² / (FK)^(1-β)
        let term1 = one_minus_beta_2 / twenty_four * alpha * alpha / fk_pow_full;

        // term2 = ρ * β * ν * α / (4 * (FK)^((1-β)/2))
        let term2 = rho * beta * nu * alpha / (four * fk_pow_half);

        // term3 = (2 - 3ρ²) / 24 * ν²
        let term3 = (two - three * rho * rho) / twenty_four * nu * nu;

        // Total expansion
        let expansion = one + (term1 + term2 + term3) * t;

        base * z_over_x * expansion
    }

    /// x(z) 関数の計算
    ///
    /// x(z) = ln((√(1-2ρz+z²) + z - ρ) / (1-ρ))
    ///
    /// 数値安定性のために smooth_log を使用
    fn compute_x_of_z(&self, z: T, rho: T) -> T {
        let eps = self.params.smoothing_epsilon;
        let one = T::one();
        let two = T::from(2.0).unwrap();

        // √(1 - 2ρz + z²)
        let discriminant = one - two * rho * z + z * z;
        // 数値安定性のため、discriminant が負にならないようにフロアを設定
        let safe_discriminant = if discriminant < eps * eps {
            eps * eps
        } else {
            discriminant
        };
        let sqrt_disc = safe_discriminant.sqrt();

        // numerator = √(1-2ρz+z²) + z - ρ
        let numerator = sqrt_disc + z - rho;

        // denominator = 1 - ρ
        let denominator = one - rho;

        // x(z) = ln(numerator / denominator)
        // smooth_log を使用して数値安定性を確保
        smooth_log(numerator / denominator, eps)
    }

    /// パラメータを検証
    pub fn validate(&self) -> Result<(), SABRError> {
        self.params.validate()
    }

    /// フロア付きインプライドボラティリティを計算
    ///
    /// 負のインプライドボラティリティが計算された場合、指定されたフロア値を適用する。
    /// これは極端なストライクやパラメータ組み合わせで発生しうる数値的問題を回避するため。
    ///
    /// # 引数
    ///
    /// * `strike` - ストライク価格（正でなければならない）
    /// * `floor` - 最小ボラティリティフロア値（通常は小さな正の値、例: 0.0001）
    ///
    /// # 戻り値
    ///
    /// インプライドボラティリティ（floor以上の値が保証される）
    ///
    /// # エラー
    ///
    /// - `InvalidStrike`: ストライクが正でない
    /// - `NonFinite`: NaN/Infinityが検出された
    ///
    /// # 例
    ///
    /// ```
    /// use pricer_models::models::sabr::{SABRModel, SABRParams};
    ///
    /// let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
    /// let model = SABRModel::new(params).unwrap();
    ///
    /// // フロア付きでIVを計算
    /// let iv = model.implied_vol_with_floor(100.0, 0.0001);
    /// assert!(iv.is_ok());
    /// assert!(iv.unwrap() >= 0.0001);
    /// ```
    pub fn implied_vol_with_floor(&self, strike: T, floor: T) -> Result<T, SABRError> {
        // ストライク検証
        if strike <= T::zero() {
            return Err(SABRError::InvalidStrike(
                strike.to_f64().unwrap_or(f64::NAN),
            ));
        }

        let f = self.params.forward;
        let log_fk = smooth_log(f / strike, self.params.smoothing_epsilon);

        // ATM近傍判定
        let vol = if log_fk.abs() < self.params.atm_threshold {
            self.implied_vol_atm_expansion(strike)
        } else if self.params.is_normal() {
            // Normal SABR (beta=0) specialized formula
            self.implied_vol_normal(strike)
        } else if self.params.is_lognormal() {
            // Lognormal SABR (beta=1) specialized formula
            self.implied_vol_lognormal(strike)
        } else {
            // General Hagan formula
            self.implied_vol_hagan(strike)
        };

        // 結果の検証
        if !vol.is_finite() {
            return Err(SABRError::NonFinite("implied_vol".to_string()));
        }

        // フロアを適用
        let floored_vol = if vol < floor { floor } else { vol };

        Ok(floored_vol)
    }

    /// Normal SABR (beta=0) のインプライドボラティリティ計算
    ///
    /// beta=0の場合、Hagan公式は以下のように簡略化される:
    /// ```text
    /// σ_N = α * [1 + ((2-3ρ²)/24 * ν² + ρνα/(4F))*T] * z/x(z)
    /// ```
    /// ここで z = ν/α * (F-K)、ATMでは z/x(z) → 1
    ///
    /// Normal SABRでは結果は「Normal volatility」（絶対値単位）となる。
    fn implied_vol_normal(&self, strike: T) -> T {
        let f = self.params.forward;
        let k = strike;
        let alpha = self.params.alpha;
        let nu = self.params.nu;
        let rho = self.params.rho;
        let t = self.params.maturity;
        let eps = self.params.smoothing_epsilon;

        let one = T::one();
        let two = T::from(2.0).unwrap();
        let three = T::from(3.0).unwrap();
        let four = T::from(4.0).unwrap();
        let twenty_four = T::from(24.0).unwrap();

        // z = ν/α * (F - K) for Normal SABR
        let f_minus_k = f - k;
        let z = if alpha > eps {
            (nu / alpha) * f_minus_k
        } else {
            T::zero()
        };

        // x(z) calculation
        let x_z = self.compute_x_of_z(z, rho);

        // z/x(z) coefficient
        let z_over_x = if z.abs() < eps {
            // z → 0 のとき z/x(z) → 1
            one
        } else {
            z / x_z
        };

        // Base term: α (for Normal SABR, no FK power adjustment)
        let base = alpha;

        // Higher order expansion terms for Normal SABR
        // term1 = 0 when beta = 0 (since (1-beta)^2 * alpha^2 / (FK)^(1-beta) involves log terms)
        // For Normal SABR, the expansion simplifies:
        // term1 = 0 (no contribution from this term in Normal model)
        let term1 = T::zero();

        // term2 = ρ * ν * α / (4 * F) when beta=0
        // Note: For Normal SABR, this term uses F directly
        let avg_f = (f + k) / two; // Use average for OTM consistency
        let term2 = if avg_f > eps {
            rho * nu * alpha / (four * avg_f)
        } else {
            T::zero()
        };

        // term3 = (2 - 3ρ²) / 24 * ν²
        let term3 = (two - three * rho * rho) / twenty_four * nu * nu;

        // Total expansion
        let expansion = one + (term1 + term2 + term3) * t;

        base * z_over_x * expansion
    }

    /// Lognormal SABR (beta=1) のインプライドボラティリティ計算
    ///
    /// beta=1の場合、(FK)^((1-β)/2) = 1 となり、Hagan公式は簡略化される:
    /// ```text
    /// σ_B = α * (z/x(z)) * [1 + ((2-3ρ²)/24 * ν² + ρνα/4)*T]
    /// ```
    /// ここで z = ν/α * ln(F/K)
    ///
    /// Lognormal SABRでは結果は「Black volatility」（対数単位）となる。
    fn implied_vol_lognormal(&self, strike: T) -> T {
        let f = self.params.forward;
        let k = strike;
        let alpha = self.params.alpha;
        let nu = self.params.nu;
        let rho = self.params.rho;
        let t = self.params.maturity;
        let eps = self.params.smoothing_epsilon;

        let one = T::one();
        let two = T::from(2.0).unwrap();
        let three = T::from(3.0).unwrap();
        let four = T::from(4.0).unwrap();
        let twenty_four = T::from(24.0).unwrap();

        // ln(F/K)
        let log_fk = smooth_log(f / k, eps);

        // z = ν/α * ln(F/K) for Lognormal SABR (since (FK)^0 = 1)
        let z = if alpha > eps {
            (nu / alpha) * log_fk
        } else {
            T::zero()
        };

        // x(z) calculation
        let x_z = self.compute_x_of_z(z, rho);

        // z/x(z) coefficient
        let z_over_x = if z.abs() < eps {
            // z → 0 のとき z/x(z) → 1
            one
        } else {
            z / x_z
        };

        // Base term: α (for Lognormal SABR, (FK)^((1-1)/2) = 1)
        // D(F/K) = 1 when beta=1 (since (1-beta) = 0)
        let base = alpha;

        // Higher order expansion terms for Lognormal SABR
        // term1 = 0 when beta = 1 (since (1-beta)^2 = 0)
        let term1 = T::zero();

        // term2 = ρ * ν * α / 4 when beta=1
        let term2 = rho * nu * alpha / four;

        // term3 = (2 - 3ρ²) / 24 * ν²
        let term3 = (two - three * rho * rho) / twenty_four * nu * nu;

        // Total expansion
        let expansion = one + (term1 + term2 + term3) * t;

        base * z_over_x * expansion
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 4.1: SABRParams and SABRError Tests (TDD - RED phase)
    // ================================================================

    // ----------------------------------------------------------------
    // SABRError tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_error_invalid_forward_message() {
        let err = SABRError::InvalidForward(-100.0);
        let msg = format!("{}", err);
        assert!(msg.contains("-100"));
        assert!(msg.contains("フォワード価格"));
    }

    #[test]
    fn test_sabr_error_invalid_alpha_message() {
        let err = SABRError::InvalidAlpha(-0.1);
        let msg = format!("{}", err);
        assert!(msg.contains("-0.1"));
        assert!(msg.contains("alpha"));
    }

    #[test]
    fn test_sabr_error_invalid_nu_message() {
        let err = SABRError::InvalidNu(-0.2);
        let msg = format!("{}", err);
        assert!(msg.contains("-0.2"));
        assert!(msg.contains("nu"));
    }

    #[test]
    fn test_sabr_error_invalid_beta_message() {
        let err = SABRError::InvalidBeta(1.5);
        let msg = format!("{}", err);
        assert!(msg.contains("1.5"));
        assert!(msg.contains("beta"));
    }

    #[test]
    fn test_sabr_error_invalid_rho_message() {
        let err = SABRError::InvalidRho(1.0);
        let msg = format!("{}", err);
        assert!(msg.contains("1"));
        assert!(msg.contains("rho"));
    }

    #[test]
    fn test_sabr_error_invalid_maturity_message() {
        let err = SABRError::InvalidMaturity(-1.0);
        let msg = format!("{}", err);
        assert!(msg.contains("-1"));
        assert!(msg.contains("満期"));
    }

    #[test]
    fn test_sabr_error_invalid_strike_message() {
        let err = SABRError::InvalidStrike(-50.0);
        let msg = format!("{}", err);
        assert!(msg.contains("-50"));
        assert!(msg.contains("ストライク"));
    }

    #[test]
    fn test_sabr_error_negative_implied_vol_message() {
        let err = SABRError::NegativeImpliedVol(80.0);
        let msg = format!("{}", err);
        assert!(msg.contains("80"));
        assert!(msg.contains("インプライドボラティリティ"));
    }

    #[test]
    fn test_sabr_error_numerical_instability_message() {
        let err = SABRError::NumericalInstability("test instability".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("test instability"));
    }

    #[test]
    fn test_sabr_error_non_finite_message() {
        let err = SABRError::NonFinite("implied_vol".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("implied_vol"));
        assert!(msg.contains("NaN"));
    }

    #[test]
    fn test_sabr_error_clone() {
        let err = SABRError::InvalidAlpha(-0.1);
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_sabr_error_debug() {
        let err = SABRError::InvalidBeta(2.0);
        let debug = format!("{:?}", err);
        assert!(debug.contains("InvalidBeta"));
        assert!(debug.contains("2.0"));
    }

    // ----------------------------------------------------------------
    // SABRParams creation tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_params_new_valid() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0);
        assert!(params.is_ok());
        let p = params.unwrap();
        assert_eq!(p.forward, 100.0);
        assert_eq!(p.alpha, 0.2);
        assert_eq!(p.nu, 0.4);
        assert_eq!(p.rho, -0.3);
        assert_eq!(p.beta, 0.5);
        assert_eq!(p.maturity, 1.0);
    }

    #[test]
    fn test_sabr_params_new_invalid_forward() {
        let params = SABRParams::new(-100.0, 0.2, 0.4, -0.3, 0.5, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidForward(_))));
    }

    #[test]
    fn test_sabr_params_new_zero_forward() {
        let params = SABRParams::new(0.0, 0.2, 0.4, -0.3, 0.5, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidForward(_))));
    }

    #[test]
    fn test_sabr_params_new_invalid_alpha() {
        let params = SABRParams::new(100.0, -0.2, 0.4, -0.3, 0.5, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidAlpha(_))));
    }

    #[test]
    fn test_sabr_params_new_zero_alpha() {
        let params = SABRParams::new(100.0, 0.0, 0.4, -0.3, 0.5, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidAlpha(_))));
    }

    #[test]
    fn test_sabr_params_new_invalid_nu() {
        let params = SABRParams::new(100.0, 0.2, -0.1, -0.3, 0.5, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidNu(_))));
    }

    #[test]
    fn test_sabr_params_new_zero_nu_valid() {
        // nu = 0 is valid (no vol-of-vol)
        let params = SABRParams::new(100.0, 0.2, 0.0, -0.3, 0.5, 1.0);
        assert!(params.is_ok());
    }

    #[test]
    fn test_sabr_params_new_invalid_beta_negative() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, -0.1, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidBeta(_))));
    }

    #[test]
    fn test_sabr_params_new_invalid_beta_above_one() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 1.1, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidBeta(_))));
    }

    #[test]
    fn test_sabr_params_new_beta_zero_valid() {
        // beta = 0 (Normal SABR) is valid
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.0, 1.0);
        assert!(params.is_ok());
    }

    #[test]
    fn test_sabr_params_new_beta_one_valid() {
        // beta = 1 (Lognormal SABR) is valid
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 1.0, 1.0);
        assert!(params.is_ok());
    }

    #[test]
    fn test_sabr_params_new_invalid_rho_minus_one() {
        // rho = -1 is invalid (open interval)
        let params = SABRParams::new(100.0, 0.2, 0.4, -1.0, 0.5, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidRho(_))));
    }

    #[test]
    fn test_sabr_params_new_invalid_rho_plus_one() {
        // rho = 1 is invalid (open interval)
        let params = SABRParams::new(100.0, 0.2, 0.4, 1.0, 0.5, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidRho(_))));
    }

    #[test]
    fn test_sabr_params_new_invalid_rho_below_minus_one() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -1.1, 0.5, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidRho(_))));
    }

    #[test]
    fn test_sabr_params_new_invalid_rho_above_plus_one() {
        let params = SABRParams::new(100.0, 0.2, 0.4, 1.1, 0.5, 1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidRho(_))));
    }

    #[test]
    fn test_sabr_params_new_rho_near_bounds_valid() {
        // rho = 0.999 is valid
        let params = SABRParams::new(100.0, 0.2, 0.4, 0.999, 0.5, 1.0);
        assert!(params.is_ok());

        // rho = -0.999 is valid
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.999, 0.5, 1.0);
        assert!(params.is_ok());
    }

    #[test]
    fn test_sabr_params_new_invalid_maturity() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, -1.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidMaturity(_))));
    }

    #[test]
    fn test_sabr_params_new_zero_maturity() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 0.0);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidMaturity(_))));
    }

    // ----------------------------------------------------------------
    // SABRParams builder methods tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_params_with_atm_threshold() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0)
            .unwrap()
            .with_atm_threshold(1e-3);
        assert!(params.is_ok());
        let p = params.unwrap();
        assert!((p.atm_threshold - 1e-3).abs() < 1e-10);
    }

    #[test]
    fn test_sabr_params_with_atm_threshold_invalid() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0)
            .unwrap()
            .with_atm_threshold(-0.001);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidAtmThreshold(_))));
    }

    #[test]
    fn test_sabr_params_with_epsilon() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0)
            .unwrap()
            .with_epsilon(1e-6);
        assert!(params.is_ok());
        let p = params.unwrap();
        assert!((p.smoothing_epsilon - 1e-6).abs() < 1e-12);
    }

    #[test]
    fn test_sabr_params_with_epsilon_invalid() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0)
            .unwrap()
            .with_epsilon(-1e-6);
        assert!(params.is_err());
        assert!(matches!(params, Err(SABRError::InvalidEpsilon(_))));
    }

    // ----------------------------------------------------------------
    // SABRParams accessor tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_params_accessors() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        assert_eq!(params.forward(), 100.0);
        assert_eq!(params.alpha(), 0.2);
        assert_eq!(params.nu(), 0.4);
        assert_eq!(params.rho(), -0.3);
        assert_eq!(params.beta(), 0.5);
        assert_eq!(params.maturity(), 1.0);
    }

    // ----------------------------------------------------------------
    // SABRParams special modes tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_params_is_normal() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.0, 1.0).unwrap();
        assert!(params.is_normal());
        assert!(!params.is_lognormal());
    }

    #[test]
    fn test_sabr_params_is_lognormal() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 1.0, 1.0).unwrap();
        assert!(params.is_lognormal());
        assert!(!params.is_normal());
    }

    #[test]
    fn test_sabr_params_mixed_beta() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        assert!(!params.is_normal());
        assert!(!params.is_lognormal());
    }

    // ----------------------------------------------------------------
    // SABRParams trait implementations tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_params_clone() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let cloned = params.clone();
        assert_eq!(params, cloned);
    }

    #[test]
    fn test_sabr_params_copy() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let copied = params; // Copy
        assert_eq!(params.forward, copied.forward);
    }

    #[test]
    fn test_sabr_params_debug() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let debug = format!("{:?}", params);
        assert!(debug.contains("SABRParams"));
        assert!(debug.contains("forward"));
        assert!(debug.contains("100"));
    }

    #[test]
    fn test_sabr_params_partial_eq() {
        let params1 = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let params2 = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let params3 = SABRParams::new(100.0, 0.3, 0.4, -0.3, 0.5, 1.0).unwrap();
        assert_eq!(params1, params2);
        assert_ne!(params1, params3);
    }

    // ----------------------------------------------------------------
    // SABRParams default values tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_params_default_atm_threshold() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        // Default ATM threshold should be 1e-4
        assert!(params.atm_threshold > 0.0);
        assert!(params.atm_threshold < 1e-3);
    }

    #[test]
    fn test_sabr_params_default_smoothing_epsilon() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        // Default smoothing epsilon should be small and positive
        assert!(params.smoothing_epsilon > 0.0);
        assert!(params.smoothing_epsilon < 1e-6);
    }

    // ----------------------------------------------------------------
    // SABRParams with f32 type tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_params_f32_type() {
        let params = SABRParams::<f32>::new(100.0_f32, 0.2_f32, 0.4_f32, -0.3_f32, 0.5_f32, 1.0_f32);
        assert!(params.is_ok());
        let p = params.unwrap();
        assert_eq!(p.forward, 100.0_f32);
    }

    // ----------------------------------------------------------------
    // SABRParams edge cases tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_params_very_small_maturity() {
        // Very small but positive maturity should be valid
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1e-10);
        assert!(params.is_ok());
    }

    #[test]
    fn test_sabr_params_very_large_maturity() {
        // Large maturity should be valid
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 30.0);
        assert!(params.is_ok());
    }

    #[test]
    fn test_sabr_params_typical_fx_values() {
        // Typical FX SABR parameters
        let params = SABRParams::new(
            1.0850,  // EUR/USD forward
            0.05,    // alpha (ATM vol ~ 5%)
            0.30,    // nu
            -0.20,   // rho (negative for FX)
            0.5,     // beta (mixed model)
            0.25,    // 3 months
        );
        assert!(params.is_ok());
    }

    #[test]
    fn test_sabr_params_typical_rates_values() {
        // Typical rates SABR parameters
        let params = SABRParams::new(
            0.03,    // 3% forward rate
            0.005,   // alpha (low for rates)
            0.50,    // nu (high vol-of-vol for rates)
            0.10,    // rho (positive for rates)
            0.0,     // beta = 0 (Normal SABR for rates)
            10.0,    // 10 years
        );
        assert!(params.is_ok());
    }

    // ================================================================
    // Task 4.2: SABRModel Hagan Implied Volatility Tests (TDD)
    // ================================================================

    // ----------------------------------------------------------------
    // SABRModel creation tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_model_new_valid() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params);
        assert!(model.is_ok());
    }

    #[test]
    fn test_sabr_model_params_accessor() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();
        assert_eq!(model.params().forward(), 100.0);
        assert_eq!(model.params().alpha(), 0.2);
    }

    #[test]
    fn test_sabr_model_validate() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();
        assert!(model.validate().is_ok());
    }

    #[test]
    fn test_sabr_model_clone() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();
        let cloned = model.clone();
        assert_eq!(model.params().forward(), cloned.params().forward());
    }

    #[test]
    fn test_sabr_model_debug() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();
        let debug = format!("{:?}", model);
        assert!(debug.contains("SABRModel"));
    }

    // ----------------------------------------------------------------
    // ATM implied volatility tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_model_atm_vol_positive() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();
        let atm_vol = model.atm_vol();
        assert!(atm_vol > 0.0, "ATM vol should be positive: {}", atm_vol);
    }

    #[test]
    fn test_sabr_model_atm_vol_finite() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();
        let atm_vol = model.atm_vol();
        assert!(atm_vol.is_finite(), "ATM vol should be finite");
    }

    #[test]
    fn test_sabr_model_atm_vol_lognormal() {
        // beta = 1 (Lognormal SABR): ATM vol ~ alpha
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 1.0, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();
        let atm_vol = model.atm_vol();
        // For beta=1, ATM vol should be close to alpha
        assert!(
            (atm_vol - 0.2).abs() < 0.05,
            "Lognormal ATM vol should be close to alpha: {}",
            atm_vol
        );
    }

    #[test]
    fn test_sabr_model_atm_vol_increases_with_alpha() {
        let params1 = SABRParams::new(100.0, 0.1, 0.4, -0.3, 0.5, 1.0).unwrap();
        let params2 = SABRParams::new(100.0, 0.3, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model1 = SABRModel::new(params1).unwrap();
        let model2 = SABRModel::new(params2).unwrap();

        assert!(
            model2.atm_vol() > model1.atm_vol(),
            "Higher alpha should give higher ATM vol"
        );
    }

    #[test]
    fn test_sabr_model_atm_vol_short_maturity() {
        // Short maturity: expansion terms should be small
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 0.01).unwrap();
        let model = SABRModel::new(params).unwrap();
        let atm_vol = model.atm_vol();
        assert!(atm_vol > 0.0 && atm_vol.is_finite());
    }

    #[test]
    fn test_sabr_model_atm_vol_long_maturity() {
        // Long maturity: expansion terms more significant
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 10.0).unwrap();
        let model = SABRModel::new(params).unwrap();
        let atm_vol = model.atm_vol();
        assert!(atm_vol > 0.0 && atm_vol.is_finite());
    }

    // ----------------------------------------------------------------
    // Implied volatility for any strike tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_model_implied_vol_atm() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        // implied_vol at ATM should be close to atm_vol
        let iv_atm = model.implied_vol(100.0).unwrap();
        let atm_vol = model.atm_vol();

        assert!(
            (iv_atm - atm_vol).abs() < 0.01,
            "IV at ATM ({}) should match ATM vol ({})",
            iv_atm,
            atm_vol
        );
    }

    #[test]
    fn test_sabr_model_implied_vol_otm_call() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv = model.implied_vol(110.0);
        assert!(iv.is_ok(), "OTM call IV should compute successfully");
        assert!(iv.unwrap() > 0.0, "OTM call IV should be positive");
    }

    #[test]
    fn test_sabr_model_implied_vol_otm_put() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv = model.implied_vol(90.0);
        assert!(iv.is_ok(), "OTM put IV should compute successfully");
        assert!(iv.unwrap() > 0.0, "OTM put IV should be positive");
    }

    #[test]
    fn test_sabr_model_implied_vol_deep_itm() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv = model.implied_vol(50.0);
        assert!(iv.is_ok(), "Deep ITM IV should compute successfully");
    }

    #[test]
    fn test_sabr_model_implied_vol_deep_otm() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv = model.implied_vol(150.0);
        assert!(iv.is_ok(), "Deep OTM IV should compute successfully");
    }

    #[test]
    fn test_sabr_model_implied_vol_invalid_strike_zero() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv = model.implied_vol(0.0);
        assert!(iv.is_err());
        assert!(matches!(iv, Err(SABRError::InvalidStrike(_))));
    }

    #[test]
    fn test_sabr_model_implied_vol_invalid_strike_negative() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv = model.implied_vol(-50.0);
        assert!(iv.is_err());
        assert!(matches!(iv, Err(SABRError::InvalidStrike(_))));
    }

    // ----------------------------------------------------------------
    // Smile shape tests (negative rho gives skew)
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_model_smile_negative_rho_skew() {
        // Negative rho should create a downward sloping skew (higher IV for lower strikes)
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.5, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv_90 = model.implied_vol(90.0).unwrap();
        let iv_100 = model.implied_vol(100.0).unwrap();
        let iv_110 = model.implied_vol(110.0).unwrap();

        // With negative rho, IV at lower strikes should be higher
        assert!(
            iv_90 > iv_100,
            "Negative rho: IV at 90 ({}) should be > IV at 100 ({})",
            iv_90,
            iv_100
        );
        assert!(
            iv_100 > iv_110,
            "Negative rho: IV at 100 ({}) should be > IV at 110 ({})",
            iv_100,
            iv_110
        );
    }

    #[test]
    fn test_sabr_model_smile_positive_rho_skew() {
        // Positive rho should create an upward sloping skew
        let params = SABRParams::new(100.0, 0.2, 0.4, 0.5, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv_90 = model.implied_vol(90.0).unwrap();
        let iv_100 = model.implied_vol(100.0).unwrap();
        let iv_110 = model.implied_vol(110.0).unwrap();

        // With positive rho, IV at higher strikes should be higher
        assert!(
            iv_110 > iv_100,
            "Positive rho: IV at 110 ({}) should be > IV at 100 ({})",
            iv_110,
            iv_100
        );
        assert!(
            iv_100 > iv_90,
            "Positive rho: IV at 100 ({}) should be > IV at 90 ({})",
            iv_100,
            iv_90
        );
    }

    #[test]
    fn test_sabr_model_smile_zero_rho_symmetric() {
        // Zero rho should give symmetric smile (approximately)
        let params = SABRParams::new(100.0, 0.2, 0.4, 0.0, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv_90 = model.implied_vol(90.0).unwrap();
        let iv_110 = model.implied_vol(110.0).unwrap();

        // With zero rho, smile should be approximately symmetric
        let asymmetry = (iv_90 - iv_110).abs() / ((iv_90 + iv_110) / 2.0);
        assert!(
            asymmetry < 0.1,
            "Zero rho: smile asymmetry ({}) should be small",
            asymmetry
        );
    }

    // ----------------------------------------------------------------
    // Nu (vol-of-vol) effects tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_model_higher_nu_more_convexity() {
        // Higher nu should increase smile convexity (wings higher relative to ATM)
        let params_low_nu = SABRParams::new(100.0, 0.2, 0.1, -0.3, 0.5, 1.0).unwrap();
        let params_high_nu = SABRParams::new(100.0, 0.2, 0.6, -0.3, 0.5, 1.0).unwrap();

        let model_low = SABRModel::new(params_low_nu).unwrap();
        let model_high = SABRModel::new(params_high_nu).unwrap();

        let wing_low = model_low.implied_vol(80.0).unwrap() - model_low.atm_vol();
        let wing_high = model_high.implied_vol(80.0).unwrap() - model_high.atm_vol();

        assert!(
            wing_high > wing_low,
            "Higher nu should increase wing vol relative to ATM"
        );
    }

    #[test]
    fn test_sabr_model_zero_nu_flat_smile() {
        // Zero nu should give almost flat smile
        let params = SABRParams::new(100.0, 0.2, 0.0, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv_90 = model.implied_vol(90.0).unwrap();
        let iv_100 = model.implied_vol(100.0).unwrap();
        let iv_110 = model.implied_vol(110.0).unwrap();

        // With nu=0, smile should be nearly flat
        let variation = ((iv_90 - iv_100).abs().max((iv_110 - iv_100).abs())) / iv_100;
        assert!(
            variation < 0.02,
            "Zero nu: smile variation ({}) should be very small",
            variation
        );
    }

    // ----------------------------------------------------------------
    // Beta effects tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_model_beta_zero_normal() {
        // beta=0 (Normal SABR)
        let params = SABRParams::new(100.0, 20.0, 0.4, -0.3, 0.0, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv = model.implied_vol(100.0);
        assert!(iv.is_ok(), "Normal SABR should compute IV successfully");
        assert!(iv.unwrap() > 0.0);
    }

    #[test]
    fn test_sabr_model_beta_one_lognormal() {
        // beta=1 (Lognormal SABR)
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 1.0, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv = model.implied_vol(100.0);
        assert!(iv.is_ok(), "Lognormal SABR should compute IV successfully");
        assert!(iv.unwrap() > 0.0);
    }

    #[test]
    fn test_sabr_model_mixed_beta() {
        // Mixed beta (0 < beta < 1)
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv = model.implied_vol(100.0);
        assert!(iv.is_ok(), "Mixed beta SABR should compute IV successfully");
        assert!(iv.unwrap() > 0.0);
    }

    // ----------------------------------------------------------------
    // Numerical stability tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_model_implied_vol_near_atm() {
        // Strike very close to forward
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv = model.implied_vol(100.001);
        assert!(iv.is_ok(), "Near-ATM strike should compute IV successfully");
        let iv_val = iv.unwrap();
        assert!(iv_val.is_finite(), "Near-ATM IV should be finite");
        assert!(iv_val > 0.0, "Near-ATM IV should be positive");
    }

    #[test]
    fn test_sabr_model_implied_vol_continuity_across_atm() {
        // IV should be continuous across ATM threshold
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0)
            .unwrap()
            .with_atm_threshold(0.01)
            .unwrap();
        let model = SABRModel::new(params).unwrap();

        let strikes = [99.0, 99.5, 99.9, 100.0, 100.1, 100.5, 101.0];
        let ivs: Vec<f64> = strikes
            .iter()
            .map(|k| model.implied_vol(*k).unwrap())
            .collect();

        // Check that IVs are continuous (no jumps)
        for i in 1..ivs.len() {
            let jump = (ivs[i] - ivs[i - 1]).abs();
            let avg = (ivs[i] + ivs[i - 1]) / 2.0;
            assert!(
                jump / avg < 0.05,
                "IV should be continuous: jump from {} to {} at strikes {} to {}",
                ivs[i - 1],
                ivs[i],
                strikes[i - 1],
                strikes[i]
            );
        }
    }

    #[test]
    fn test_sabr_model_implied_vol_extreme_strikes() {
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        // Very low strike
        let iv_low = model.implied_vol(10.0);
        assert!(iv_low.is_ok() || matches!(iv_low, Err(SABRError::NegativeImpliedVol(_))));

        // Very high strike
        let iv_high = model.implied_vol(500.0);
        assert!(iv_high.is_ok() || matches!(iv_high, Err(SABRError::NegativeImpliedVol(_))));
    }

    // ----------------------------------------------------------------
    // Type compatibility tests (f32)
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_model_f32_type() {
        let params =
            SABRParams::<f32>::new(100.0_f32, 0.2_f32, 0.4_f32, -0.3_f32, 0.5_f32, 1.0_f32)
                .unwrap();
        let model = SABRModel::new(params).unwrap();

        let atm_vol = model.atm_vol();
        assert!(atm_vol > 0.0_f32 && atm_vol.is_finite());

        let iv = model.implied_vol(110.0_f32);
        assert!(iv.is_ok());
    }

    // ----------------------------------------------------------------
    // Literature comparison tests (known values)
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_model_hagan_formula_sanity() {
        // Test case from Hagan et al. (2002) paper approximation
        // These are not exact values but sanity checks
        let params = SABRParams::new(
            0.03,  // 3% forward rate
            0.02,  // 2% ATM vol (approximately)
            0.3,   // 30% vol-of-vol
            -0.25, // negative correlation
            0.0,   // beta = 0 (Normal SABR for rates)
            1.0,   // 1 year
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        let atm_vol = model.atm_vol();
        // For Normal SABR with these params, ATM vol should be reasonable
        assert!(
            atm_vol > 0.01 && atm_vol < 0.1,
            "ATM vol ({}) should be in reasonable range for rates",
            atm_vol
        );
    }

    #[test]
    fn test_sabr_model_lognormal_approximation() {
        // For beta=1, T small, ATM vol should be approximately alpha
        let params = SABRParams::new(
            100.0, // forward
            0.20,  // 20% vol
            0.0,   // no vol-of-vol
            0.0,   // no correlation
            1.0,   // lognormal
            0.01,  // very short maturity
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        let atm_vol = model.atm_vol();
        // With nu=0, rho=0, short maturity, ATM vol should be very close to alpha
        assert!(
            (atm_vol - 0.20).abs() < 0.01,
            "Lognormal ATM vol ({}) should be close to alpha (0.20)",
            atm_vol
        );
    }

    // ================================================================
    // Task 4.3: Special Cases and Parameter Validation Tests (TDD)
    // ================================================================

    // ----------------------------------------------------------------
    // Normal SABR (beta = 0) specialized formula tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_normal_implied_vol_atm() {
        // Normal SABR (beta=0): ATM vol formula is alpha * [1 + ((2-3rho^2)/24 * nu^2)*T]
        let params = SABRParams::new(
            0.03,  // 3% forward rate (typical for rates)
            0.005, // Normal vol alpha (in rate units, not %)
            0.4,   // vol-of-vol
            -0.3,  // correlation
            0.0,   // beta = 0 (Normal SABR)
            1.0,   // 1 year
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        // ATM vol should be positive and finite
        let atm_vol = model.atm_vol();
        assert!(atm_vol > 0.0, "Normal SABR ATM vol should be positive");
        assert!(atm_vol.is_finite(), "Normal SABR ATM vol should be finite");
    }

    #[test]
    fn test_sabr_normal_implied_vol_otm() {
        // Normal SABR with OTM strikes
        let params = SABRParams::new(
            0.03,  // 3% forward rate
            0.005, // Normal vol alpha
            0.4,   // vol-of-vol
            -0.3,  // correlation
            0.0,   // beta = 0
            1.0,   // 1 year
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        // OTM call (higher strike)
        let iv_otm_call = model.implied_vol(0.04);
        assert!(iv_otm_call.is_ok(), "Normal SABR should compute OTM call IV");
        assert!(
            iv_otm_call.unwrap() > 0.0,
            "Normal SABR OTM call IV should be positive"
        );

        // OTM put (lower strike)
        let iv_otm_put = model.implied_vol(0.02);
        assert!(iv_otm_put.is_ok(), "Normal SABR should compute OTM put IV");
        assert!(
            iv_otm_put.unwrap() > 0.0,
            "Normal SABR OTM put IV should be positive"
        );
    }

    #[test]
    fn test_sabr_normal_uses_specialized_formula() {
        // Verify Normal SABR uses the specialized formula
        // For beta=0, the formula simplifies significantly
        let params = SABRParams::new(
            100.0, // forward
            20.0,  // Normal vol alpha (absolute units)
            0.0,   // no vol-of-vol (simplifies formula)
            0.0,   // no correlation
            0.0,   // beta = 0
            1.0,   // 1 year
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        // With nu=0, rho=0, Normal SABR ATM vol = alpha
        let atm_vol = model.atm_vol();
        assert!(
            (atm_vol - 20.0).abs() < 1.0,
            "Normal SABR with nu=0 ATM vol ({}) should be close to alpha (20.0)",
            atm_vol
        );
    }

    #[test]
    fn test_sabr_normal_smile_with_negative_rho() {
        // Normal SABR smile shape with negative rho
        let params = SABRParams::new(
            0.03,  // forward
            0.005, // alpha
            0.4,   // nu
            -0.5,  // negative rho
            0.0,   // beta = 0
            1.0,   // maturity
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv_low = model.implied_vol(0.02).unwrap();
        let iv_atm = model.implied_vol(0.03).unwrap();
        let iv_high = model.implied_vol(0.04).unwrap();

        // With negative rho in Normal SABR, lower strikes should have higher vol
        assert!(
            iv_low > iv_atm,
            "Normal SABR with negative rho: IV at low strike ({}) should be > ATM ({})",
            iv_low,
            iv_atm
        );
    }

    #[test]
    fn test_sabr_normal_f32_support() {
        // Test Normal SABR with f32 type
        let params = SABRParams::<f32>::new(
            0.03_f32, // forward
            0.005_f32, // alpha
            0.4_f32,  // nu
            -0.3_f32, // rho
            0.0_f32,  // beta = 0
            1.0_f32,  // maturity
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        let atm_vol = model.atm_vol();
        assert!(atm_vol > 0.0_f32 && atm_vol.is_finite());

        let iv = model.implied_vol(0.035_f32);
        assert!(iv.is_ok());
    }

    // ----------------------------------------------------------------
    // Lognormal SABR (beta = 1) specialized formula tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_lognormal_implied_vol_atm() {
        // Lognormal SABR (beta=1): ATM vol formula simplifies
        // since (FK)^((1-beta)/2) = (FK)^0 = 1
        let params = SABRParams::new(
            100.0, // forward
            0.2,   // Black-Scholes style vol
            0.4,   // vol-of-vol
            -0.3,  // correlation
            1.0,   // beta = 1 (Lognormal)
            1.0,   // 1 year
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        let atm_vol = model.atm_vol();
        assert!(atm_vol > 0.0, "Lognormal SABR ATM vol should be positive");
        assert!(
            atm_vol.is_finite(),
            "Lognormal SABR ATM vol should be finite"
        );

        // For beta=1, ATM vol should be close to alpha (with expansion corrections)
        assert!(
            (atm_vol - 0.2).abs() < 0.05,
            "Lognormal ATM vol ({}) should be close to alpha (0.2)",
            atm_vol
        );
    }

    #[test]
    fn test_sabr_lognormal_implied_vol_otm() {
        // Lognormal SABR with OTM strikes
        let params = SABRParams::new(
            100.0, // forward
            0.2,   // alpha
            0.4,   // vol-of-vol
            -0.3,  // correlation
            1.0,   // beta = 1
            1.0,   // 1 year
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        // OTM call
        let iv_otm_call = model.implied_vol(120.0);
        assert!(
            iv_otm_call.is_ok(),
            "Lognormal SABR should compute OTM call IV"
        );
        assert!(
            iv_otm_call.unwrap() > 0.0,
            "Lognormal SABR OTM call IV should be positive"
        );

        // OTM put
        let iv_otm_put = model.implied_vol(80.0);
        assert!(
            iv_otm_put.is_ok(),
            "Lognormal SABR should compute OTM put IV"
        );
        assert!(
            iv_otm_put.unwrap() > 0.0,
            "Lognormal SABR OTM put IV should be positive"
        );
    }

    #[test]
    fn test_sabr_lognormal_simplified_formula() {
        // Verify Lognormal SABR uses simplified formula
        // For beta=1: D(F/K) term simplifies, (FK)^((1-beta)/2) = 1
        let params = SABRParams::new(
            100.0, // forward
            0.25,  // alpha
            0.0,   // no vol-of-vol
            0.0,   // no correlation
            1.0,   // beta = 1
            0.01,  // very short maturity
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        // With nu=0, rho=0, short T, Lognormal SABR ATM vol = alpha
        let atm_vol = model.atm_vol();
        assert!(
            (atm_vol - 0.25).abs() < 0.01,
            "Lognormal SABR with nu=0 ATM vol ({}) should be close to alpha (0.25)",
            atm_vol
        );
    }

    #[test]
    fn test_sabr_lognormal_smile_shape() {
        // Lognormal SABR smile with typical equity parameters
        let params = SABRParams::new(
            100.0, // forward
            0.2,   // alpha
            0.5,   // high vol-of-vol
            -0.5,  // negative correlation (typical for equity)
            1.0,   // beta = 1
            1.0,   // maturity
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv_80 = model.implied_vol(80.0).unwrap();
        let iv_100 = model.implied_vol(100.0).unwrap();
        let iv_120 = model.implied_vol(120.0).unwrap();

        // With negative rho, smile should be skewed
        assert!(
            iv_80 > iv_100,
            "Lognormal SABR skew: IV at 80 ({}) should be > ATM ({})",
            iv_80,
            iv_100
        );
    }

    #[test]
    fn test_sabr_lognormal_f32_support() {
        // Test Lognormal SABR with f32 type
        let params = SABRParams::<f32>::new(
            100.0_f32, // forward
            0.2_f32,   // alpha
            0.4_f32,   // nu
            -0.3_f32,  // rho
            1.0_f32,   // beta = 1
            1.0_f32,   // maturity
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        let atm_vol = model.atm_vol();
        assert!(atm_vol > 0.0_f32 && atm_vol.is_finite());

        let iv = model.implied_vol(110.0_f32);
        assert!(iv.is_ok());
    }

    // ----------------------------------------------------------------
    // Parameter validation tests (Requirements 10.2, 10.3, 10.4)
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_validate_alpha_positive() {
        // alpha must be strictly positive
        let result = SABRParams::new(100.0, 0.0, 0.4, -0.3, 0.5, 1.0);
        assert!(matches!(result, Err(SABRError::InvalidAlpha(_))));

        let result = SABRParams::new(100.0, -0.1, 0.4, -0.3, 0.5, 1.0);
        assert!(matches!(result, Err(SABRError::InvalidAlpha(_))));
    }

    #[test]
    fn test_sabr_validate_beta_range() {
        // beta must be in [0, 1]
        let result = SABRParams::new(100.0, 0.2, 0.4, -0.3, -0.1, 1.0);
        assert!(matches!(result, Err(SABRError::InvalidBeta(_))));

        let result = SABRParams::new(100.0, 0.2, 0.4, -0.3, 1.1, 1.0);
        assert!(matches!(result, Err(SABRError::InvalidBeta(_))));

        // Edge cases: exactly 0 and 1 should be valid
        let result = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.0, 1.0);
        assert!(result.is_ok());

        let result = SABRParams::new(100.0, 0.2, 0.4, -0.3, 1.0, 1.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sabr_validate_rho_strict_bounds() {
        // |rho| < 1 (strict inequality)
        let result = SABRParams::new(100.0, 0.2, 0.4, -1.0, 0.5, 1.0);
        assert!(matches!(result, Err(SABRError::InvalidRho(_))));

        let result = SABRParams::new(100.0, 0.2, 0.4, 1.0, 0.5, 1.0);
        assert!(matches!(result, Err(SABRError::InvalidRho(_))));

        // Values very close to boundary should be valid
        let result = SABRParams::new(100.0, 0.2, 0.4, 0.9999, 0.5, 1.0);
        assert!(result.is_ok());

        let result = SABRParams::new(100.0, 0.2, 0.4, -0.9999, 0.5, 1.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sabr_validate_all_params_comprehensive() {
        // Test all validation conditions at once
        // Valid parameters
        let valid = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0);
        assert!(valid.is_ok());

        // Invalid forward
        let invalid_forward = SABRParams::new(0.0, 0.2, 0.4, -0.3, 0.5, 1.0);
        assert!(matches!(invalid_forward, Err(SABRError::InvalidForward(_))));

        // Invalid alpha
        let invalid_alpha = SABRParams::new(100.0, 0.0, 0.4, -0.3, 0.5, 1.0);
        assert!(matches!(invalid_alpha, Err(SABRError::InvalidAlpha(_))));

        // Invalid nu (negative)
        let invalid_nu = SABRParams::new(100.0, 0.2, -0.1, -0.3, 0.5, 1.0);
        assert!(matches!(invalid_nu, Err(SABRError::InvalidNu(_))));

        // Invalid rho
        let invalid_rho = SABRParams::new(100.0, 0.2, 0.4, 1.5, 0.5, 1.0);
        assert!(matches!(invalid_rho, Err(SABRError::InvalidRho(_))));

        // Invalid beta
        let invalid_beta = SABRParams::new(100.0, 0.2, 0.4, -0.3, 1.5, 1.0);
        assert!(matches!(invalid_beta, Err(SABRError::InvalidBeta(_))));

        // Invalid maturity
        let invalid_maturity = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 0.0);
        assert!(matches!(invalid_maturity, Err(SABRError::InvalidMaturity(_))));
    }

    // ----------------------------------------------------------------
    // Negative implied volatility floor tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_negative_iv_floor_applied() {
        // Test that negative IV is handled properly with floor
        // This can occur at extreme strikes with certain parameter combinations
        let params = SABRParams::new(
            100.0, // forward
            0.1,   // low alpha
            0.8,   // high vol-of-vol
            0.7,   // high positive rho
            0.5,   // beta
            5.0,   // long maturity
        )
        .unwrap();
        let model = SABRModel::new(params).unwrap();

        // Test with extreme low strike where Hagan formula might produce negative IV
        let result = model.implied_vol_with_floor(5.0, 0.0001);

        // Result should either be Ok with positive value or use the floor
        match result {
            Ok(vol) => {
                assert!(vol >= 0.0001, "IV should be at least the floor value");
                assert!(vol.is_finite(), "IV should be finite");
            }
            Err(_) => {
                // Error is acceptable if IV can't be computed
            }
        }
    }

    #[test]
    fn test_sabr_iv_floor_default_behavior() {
        // Default implied_vol should error on negative IV
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        // Normal strikes should not hit negative IV
        let iv = model.implied_vol(100.0);
        assert!(iv.is_ok());
        assert!(iv.unwrap() > 0.0);
    }

    #[test]
    fn test_sabr_iv_floor_with_custom_value() {
        // Test implied_vol_with_floor with custom floor
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let custom_floor = 0.01; // 1% floor
        let iv = model.implied_vol_with_floor(100.0, custom_floor);

        assert!(iv.is_ok());
        assert!(iv.unwrap() >= custom_floor);
    }

    #[test]
    fn test_sabr_iv_floor_preserves_valid_values() {
        // Floor should not affect valid positive IV values
        let params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        let model = SABRModel::new(params).unwrap();

        let iv_no_floor = model.implied_vol(100.0).unwrap();
        let iv_with_floor = model.implied_vol_with_floor(100.0, 0.0001).unwrap();

        // Both should give the same result when IV is well above floor
        assert!(
            (iv_no_floor - iv_with_floor).abs() < 1e-10,
            "Floor should not affect valid IV: {} vs {}",
            iv_no_floor,
            iv_with_floor
        );
    }

    // ----------------------------------------------------------------
    // Mode detection and specialized handling tests
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_is_normal_detection() {
        let normal_params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.0, 1.0).unwrap();
        assert!(normal_params.is_normal());
        assert!(!normal_params.is_lognormal());

        // Very small beta should also be detected as normal
        let near_normal_params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 1e-10, 1.0).unwrap();
        assert!(near_normal_params.is_normal());
    }

    #[test]
    fn test_sabr_is_lognormal_detection() {
        let lognormal_params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 1.0, 1.0).unwrap();
        assert!(lognormal_params.is_lognormal());
        assert!(!lognormal_params.is_normal());

        // Beta very close to 1 should also be detected as lognormal
        let near_lognormal_params =
            SABRParams::new(100.0, 0.2, 0.4, -0.3, 1.0 - 1e-10, 1.0).unwrap();
        assert!(near_lognormal_params.is_lognormal());
    }

    #[test]
    fn test_sabr_mixed_beta_not_special_mode() {
        let mixed_params = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.5, 1.0).unwrap();
        assert!(!mixed_params.is_normal());
        assert!(!mixed_params.is_lognormal());
    }

    // ----------------------------------------------------------------
    // Consistency tests across beta values
    // ----------------------------------------------------------------

    #[test]
    fn test_sabr_consistency_beta_zero_vs_small() {
        // IV should be continuous as beta approaches 0
        let params_zero = SABRParams::new(100.0, 10.0, 0.4, -0.3, 0.0, 1.0).unwrap();
        let params_small = SABRParams::new(100.0, 10.0, 0.4, -0.3, 0.01, 1.0).unwrap();

        let model_zero = SABRModel::new(params_zero).unwrap();
        let model_small = SABRModel::new(params_small).unwrap();

        let iv_zero = model_zero.implied_vol(100.0).unwrap();
        let iv_small = model_small.implied_vol(100.0).unwrap();

        // Should be reasonably close (within 10%)
        let relative_diff = (iv_zero - iv_small).abs() / iv_zero;
        assert!(
            relative_diff < 0.1,
            "IV should be continuous as beta->0: {} vs {} (diff: {}%)",
            iv_zero,
            iv_small,
            relative_diff * 100.0
        );
    }

    #[test]
    fn test_sabr_consistency_beta_one_vs_near_one() {
        // IV should be continuous as beta approaches 1
        let params_one = SABRParams::new(100.0, 0.2, 0.4, -0.3, 1.0, 1.0).unwrap();
        let params_near = SABRParams::new(100.0, 0.2, 0.4, -0.3, 0.99, 1.0).unwrap();

        let model_one = SABRModel::new(params_one).unwrap();
        let model_near = SABRModel::new(params_near).unwrap();

        let iv_one = model_one.implied_vol(100.0).unwrap();
        let iv_near = model_near.implied_vol(100.0).unwrap();

        // Should be reasonably close
        let relative_diff = (iv_one - iv_near).abs() / iv_one;
        assert!(
            relative_diff < 0.05,
            "IV should be continuous as beta->1: {} vs {} (diff: {}%)",
            iv_one,
            iv_near,
            relative_diff * 100.0
        );
    }
}
