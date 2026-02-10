//! Heston確率的ボラティリティモデル実装
//!
//! ```text
//! dS = r * S * dt + sqrt(V) * S * dW_S
//! dV = kappa * (theta - V) * dt + xi * sqrt(V) * dW_V
//! E[dW_S * dW_V] = rho * dt
//! ```
//!
//! Andersen (2008) のQuadratic Exponential (QE) 離散化スキームを使用。
//! Feller条件 `2 * kappa * theta > xi^2` が満たされない場合、分散フロアを適用。

use pricer_core::{
    math::{
        distributions::norm_inv_cdf,
        smoothing::{smooth_indicator, smooth_max, smooth_sqrt},
    },
    traits::{priceable::Differentiable, Float},
};
use thiserror::Error;

use super::validation::{
    validate_correlation, validate_positive, ComputationError, ParamValidationError,
};

// ================================================================
// エラー型
// ================================================================

/// Hestonモデルエラー型
#[derive(Error, Debug, Clone, PartialEq)]
pub enum HestonError {
    /// パラメータ検証エラー
    #[error("パラメータ検証エラー: {0}")]
    Param(#[from] ParamValidationError),

    /// 数値計算エラー
    #[error("数値計算エラー: {0}")]
    Computation(#[from] ComputationError),
}

impl From<HestonError> for pricer_core::types::PricingError {
    fn from(err: HestonError) -> Self {
        match err {
            HestonError::Param(e) => pricer_core::types::PricingError::InvalidInput(e.to_string()),
            HestonError::Computation(e) => {
                pricer_core::types::PricingError::NumericalInstability(e.to_string())
            }
        }
    }
}

// ================================================================
// パラメータ
// ================================================================

/// Hestonモデルパラメータ
///
/// # 型パラメータ
/// * `T` - Float型（f64またはAD互換のDualNumber）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HestonParams<T: Float> {
    /// スポット価格 (S0 > 0)
    pub spot: T,
    /// 初期分散 (v0 > 0)
    pub v0: T,
    /// 長期分散 (theta > 0)
    pub theta: T,
    /// 平均回帰速度 (kappa > 0)
    pub kappa: T,
    /// ボラティリティのボラティリティ (xi > 0)
    pub xi: T,
    /// 相関係数 (-1 <= rho <= 1)
    pub rho: T,
    /// リスクフリーレート
    pub rate: T,
    /// 満期までの時間 (T > 0)
    pub maturity: T,
    /// QE切り替え閾値 (推奨値: 1.5)
    pub psi_c: T,
    /// smooth approximation epsilon
    pub smoothing_epsilon: T,
}

impl<T: Float> HestonParams<T> {
    /// 新しいHestonパラメータを作成（検証付き）
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
    pub fn with_psi_c(mut self, psi_c: T) -> Result<Self, HestonError> {
        validate_positive("psi_c", psi_c.to_f64().unwrap_or(f64::NAN))?;
        self.psi_c = psi_c;
        Ok(self)
    }

    /// カスタムsmoothing epsilonを設定
    pub fn with_epsilon(mut self, epsilon: T) -> Result<Self, HestonError> {
        validate_positive("smoothing_epsilon", epsilon.to_f64().unwrap_or(f64::NAN))?;
        self.smoothing_epsilon = epsilon;
        Ok(self)
    }

    /// パラメータを検証
    pub fn validate(&self) -> Result<(), HestonError> {
        let f = |v: T| v.to_f64().unwrap_or(f64::NAN);
        validate_positive("spot", f(self.spot))?;
        validate_positive("v0", f(self.v0))?;
        validate_positive("theta", f(self.theta))?;
        validate_positive("kappa", f(self.kappa))?;
        validate_positive("xi", f(self.xi))?;
        validate_correlation("rho", f(self.rho))?;
        validate_positive("maturity", f(self.maturity))?;
        validate_positive("psi_c", f(self.psi_c))?;
        validate_positive("smoothing_epsilon", f(self.smoothing_epsilon))?;
        Ok(())
    }

    /// Feller条件をチェック: 2 * kappa * theta > xi^2
    pub fn satisfies_feller(&self) -> bool {
        let two = T::from(2.0).unwrap_or(T::one());
        two * self.kappa * self.theta > self.xi * self.xi
    }

    /// Feller比率: 2 * kappa * theta / xi^2 (>= 1.0 で条件充足)
    pub fn feller_ratio(&self) -> T {
        let two = T::from(2.0).unwrap_or(T::one());
        let denom = self.xi * self.xi;
        if denom > T::zero() {
            two * self.kappa * self.theta / denom
        } else {
            T::infinity()
        }
    }
}

impl<T: Float> Default for HestonParams<T> {
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

// ================================================================
// モデル
// ================================================================

/// Hestonモデル
///
/// QE離散化スキーム (Andersen 2008) による2ファクターモデル。
/// AD互換のジェネリックFloat型をサポート。
#[derive(Clone)]
pub struct HestonModel<T: Float> {
    params: HestonParams<T>,
    variance_floor: T,
}

impl<T: Float + std::fmt::Debug> std::fmt::Debug for HestonModel<T> {
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
    pub fn new(params: HestonParams<T>) -> Result<Self, HestonError> {
        params.validate()?;

        let model = Self {
            variance_floor: params.smoothing_epsilon,
            params,
        };

        if !model.check_feller_condition() {
            eprintln!(
                "警告: Feller条件 (2*kappa*theta > xi^2) 未充足。分散フロア {} を適用。",
                model.variance_floor.to_f64().unwrap_or(0.0),
            );
        }

        Ok(model)
    }

    /// パラメータを検証
    pub fn validate(&self) -> Result<(), HestonError> { self.params.validate() }
    /// モデルパラメータへの参照を取得
    pub fn params(&self) -> &HestonParams<T> { &self.params }
    /// Feller条件をチェック
    pub fn check_feller_condition(&self) -> bool { self.params.satisfies_feller() }
    /// 分散フロアを取得
    pub fn variance_floor(&self) -> T { self.variance_floor }

    /// 分散フロアを設定
    pub fn with_variance_floor(mut self, floor: T) -> Self {
        if floor > T::zero() {
            self.variance_floor = floor;
        }
        self
    }

    // ================================================================
    // QE離散化スキーム (Andersen 2008)
    // ================================================================

    /// QEモーメント計算: (m, s2, psi) = (条件付き平均, 条件付き分散, s2/m²)
    ///
    /// CIR分散過程: E[V_{t+dt}|V_t] = theta + (V_t - theta) * exp(-kappa * dt)
    pub fn compute_qe_moments(&self, v_current: T, dt: T) -> (T, T, T) {
        let kappa = self.params.kappa;
        let theta = self.params.theta;
        let xi = self.params.xi;

        let exp_neg_kappa_dt = (-kappa * dt).exp();
        let one_minus_exp = T::one() - exp_neg_kappa_dt;

        // 条件付き平均
        let m = theta + (v_current - theta) * exp_neg_kappa_dt;

        // 条件付き分散 (Andersen 2008, Eq. 17)
        let two = T::from(2.0).unwrap_or(T::one() + T::one());
        let xi_sq = xi * xi;
        let term1 = v_current * xi_sq * exp_neg_kappa_dt * one_minus_exp / kappa;
        let term2 = theta * xi_sq * one_minus_exp * one_minus_exp / (two * kappa);
        let s2 = term1 + term2;

        // psi = s2 / m^2
        let eps = self.params.smoothing_epsilon;
        let m_safe = smooth_max(m, eps, eps);
        let psi = s2 / (m_safe * m_safe);

        (m, s2, psi)
    }

    /// QE二次スキーム (psi < psi_c): V_{t+dt} = a * (b + Z_v)^2
    pub fn qe_quadratic_step(&self, m: T, _s2: T, psi: T, uv: T) -> T {
        let eps = self.params.smoothing_epsilon;
        let one = T::one();
        let two = T::from(2.0).unwrap_or(one + one);

        // b^2 = (2/psi - 1) + sqrt(2/psi * (2/psi - 1))
        let psi_inv = one / smooth_max(psi, eps, eps);
        let two_psi_inv = two * psi_inv;
        let term_inner = two_psi_inv - one;
        let term_inner_safe = smooth_max(term_inner, T::zero(), eps);
        let sqrt_term = smooth_sqrt(two_psi_inv * term_inner_safe, eps);
        let b_squared = term_inner_safe + sqrt_term;
        let b = smooth_sqrt(b_squared, eps);
        let a = m / (one + b_squared);

        // 一様乱数→正規乱数変換
        let u_clamped = uv.max(eps).min(one - eps);
        let z_v = norm_inv_cdf(u_clamped).unwrap_or(T::zero());

        let b_plus_z = b + z_v;
        let v_next = a * b_plus_z * b_plus_z;
        smooth_max(v_next, T::zero(), eps)
    }

    /// QE指数スキーム (psi >= psi_c): ゼロ質量混合分布
    pub fn qe_exponential_step(&self, m: T, psi: T, uv: T) -> T {
        let eps = self.params.smoothing_epsilon;
        let one = T::one();
        let two = T::from(2.0).unwrap_or(one + one);

        let psi_safe = smooth_max(psi, eps, eps);
        let p = (psi_safe - one) / (psi_safe + one);
        let p_clamped = smooth_max(p, T::zero(), eps);

        let m_safe = smooth_max(m, eps, eps);
        let beta = two / (m_safe * (psi_safe + one));

        let one_minus_uv = smooth_max(one - uv, eps, eps);
        let one_minus_p = smooth_max(one - p_clamped, eps, eps);
        let log_ratio = (one_minus_uv / one_minus_p).ln();
        let v_exp = smooth_max(-log_ratio / beta, T::zero(), eps);

        let indicator = smooth_indicator(uv - p_clamped, eps);
        let v_next = indicator * v_exp;
        smooth_max(v_next, T::zero(), eps)
    }

    /// QE分散ステップ: psi値で二次/指数スキームを滑らかにブレンド
    pub fn qe_variance_step(&self, v_current: T, dt: T, uv: T) -> T {
        let (m, s2, psi) = self.compute_qe_moments(v_current, dt);
        let psi_c = self.params.psi_c;
        let eps = self.params.smoothing_epsilon;

        let v_quadratic = self.qe_quadratic_step(m, s2, psi, uv);
        let v_exponential = self.qe_exponential_step(m, psi, uv);

        let indicator = smooth_indicator(psi - psi_c, eps);
        let one = T::one();
        let v_next = (one - indicator) * v_quadratic + indicator * v_exponential;
        smooth_max(v_next, self.variance_floor, eps)
    }

    /// 相関ブラウン運動生成 (Cholesky): dW_V = rho * z1 + sqrt(1 - rho^2) * z2
    pub fn generate_correlated_brownian(&self, z1: T, z2: T) -> (T, T) {
        let rho = self.params.rho;
        let eps = self.params.smoothing_epsilon;
        let one_minus_rho_sq = T::one() - rho * rho;
        let sqrt_term = smooth_sqrt(one_minus_rho_sq, eps);
        (z1, rho * z1 + sqrt_term * z2)
    }

    /// QE価格ステップ (中間点規則)
    ///
    /// ln(S_{t+dt}) = ln(S_t) + (r - V_avg/2) * dt + sqrt(V_avg * dt) * dW_S
    pub fn qe_price_step(&self, s_current: T, v_current: T, v_next: T, dt: T, dw_s: T) -> T {
        let rate = self.params.rate;
        let eps = self.params.smoothing_epsilon;
        let two = T::from(2.0).unwrap_or(T::one() + T::one());

        let v_avg = (v_current + v_next) / two;
        let v_avg_safe = smooth_max(v_avg, eps, eps);

        let drift = (rate - v_avg_safe / two) * dt;
        let volatility = smooth_sqrt(v_avg_safe, eps);
        let diffusion = volatility * dt.sqrt() * dw_s;

        let s_next = s_current * (drift + diffusion).exp();
        smooth_max(s_next, eps, eps)
    }

    /// QE離散化1ステップ: (S_{t+dt}, V_{t+dt})
    pub fn qe_step(&self, s_current: T, v_current: T, dt: T, z1: T, z2: T, uv: T) -> (T, T) {
        let v_next = self.qe_variance_step(v_current, dt, uv);
        let (dw_s, _dw_v) = self.generate_correlated_brownian(z1, z2);
        let s_next = self.qe_price_step(s_current, v_current, v_next, dt, dw_s);
        (s_next, v_next)
    }
}

// ================================================================
// トレイト実装
// ================================================================

impl<T: Float> Differentiable for HestonModel<T> {}

use crate::stochastic::stochastic::{EquityModel, StochasticModel, TwoFactorState};

impl<T: Float + Default> StochasticModel<T> for HestonModel<T> {
    type State = TwoFactorState<T>;
    type Params = HestonParams<T>;

    /// QE離散化による1ステップ遷移
    /// dw: [z1 (price normal), z2 (variance normal), uv (uniform for QE)]
    fn evolve_step(state: Self::State, dt: T, dw: &[T], params: &Self::Params) -> Self::State {
        let z1 = dw.first().copied().unwrap_or(T::zero());
        let z2 = dw.get(1).copied().unwrap_or(T::zero());
        let uv = dw
            .get(2)
            .copied()
            .unwrap_or(T::from(0.5).unwrap_or(T::zero()));

        let model = HestonModel {
            params: *params,
            variance_floor: params.smoothing_epsilon,
        };
        let (next_price, next_variance) = model.qe_step(state.first, state.second, dt, z1, z2, uv);
        TwoFactorState {
            first: next_price,
            second: next_variance,
        }
    }

    fn initial_state(params: &Self::Params) -> Self::State {
        TwoFactorState {
            first: params.spot,
            second: params.v0,
        }
    }

    fn brownian_dim() -> usize { 2 }
    fn model_name() -> &'static str { "Heston" }
    fn num_factors() -> usize { 2 }
}

impl<T: Float + Default> EquityModel<T> for HestonModel<T> {}

// ================================================================
// テスト
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stochastic::stochastic::{StochasticModel, StochasticState, TwoFactorState};

    // ----------------------------------------------------------------
    // ヘルパー
    // ----------------------------------------------------------------

    fn default_params() -> HestonParams<f64> {
        HestonParams::new(100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap()
    }

    fn default_model() -> HestonModel<f64> { HestonModel::new(default_params()).unwrap() }

    // ----------------------------------------------------------------
    // エラー型
    // ----------------------------------------------------------------

    #[test]
    fn test_heston_error_display() {
        let param_err =
            HestonError::Param(ParamValidationError::must_be_positive("spot", -100.0));
        assert!(param_err.to_string().contains("spot"));
        assert!(param_err.to_string().contains("-100"));

        let comp_err = HestonError::Computation(ComputationError::non_finite("price"));
        assert!(comp_err.to_string().contains("price"));
    }

    #[test]
    fn test_heston_error_to_pricing_error() {
        let param_err: pricer_core::types::PricingError =
            HestonError::Param(ParamValidationError::must_be_positive("spot", -100.0)).into();
        assert!(matches!(
            param_err,
            pricer_core::types::PricingError::InvalidInput(_)
        ));

        let comp_err: pricer_core::types::PricingError =
            HestonError::Computation(ComputationError::non_finite("price")).into();
        assert!(matches!(
            comp_err,
            pricer_core::types::PricingError::NumericalInstability(_)
        ));
    }

    // ----------------------------------------------------------------
    // HestonParams
    // ----------------------------------------------------------------

    #[test]
    fn test_params_new_valid() {
        let p = default_params();
        assert_eq!(p.spot, 100.0);
        assert_eq!(p.v0, 0.04);
        assert_eq!(p.theta, 0.04);
        assert_eq!(p.kappa, 1.5);
        assert_eq!(p.xi, 0.3);
        assert_eq!(p.rho, -0.7);
        assert_eq!(p.rate, 0.05);
        assert_eq!(p.maturity, 1.0);
        assert_eq!(p.psi_c, 1.5);
    }

    #[test]
    fn test_params_rejects_invalid() {
        let cases: [(f64, f64, f64, f64, f64, f64, f64, f64); 14] = [
            (-100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0), // negative spot
            (0.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0),    // zero spot
            (100.0, -0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0),  // negative v0
            (100.0, 0.0, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0),    // zero v0
            (100.0, 0.04, -0.04, 1.5, 0.3, -0.7, 0.05, 1.0),  // negative theta
            (100.0, 0.04, 0.0, 1.5, 0.3, -0.7, 0.05, 1.0),    // zero theta
            (100.0, 0.04, 0.04, -1.5, 0.3, -0.7, 0.05, 1.0),  // negative kappa
            (100.0, 0.04, 0.04, 0.0, 0.3, -0.7, 0.05, 1.0),   // zero kappa
            (100.0, 0.04, 0.04, 1.5, -0.3, -0.7, 0.05, 1.0),  // negative xi
            (100.0, 0.04, 0.04, 1.5, 0.0, -0.7, 0.05, 1.0),   // zero xi
            (100.0, 0.04, 0.04, 1.5, 0.3, 1.5, 0.05, 1.0),    // rho > 1
            (100.0, 0.04, 0.04, 1.5, 0.3, -1.5, 0.05, 1.0),   // rho < -1
            (100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, -1.0),  // negative maturity
            (100.0, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 0.0),   // zero maturity
        ];
        for (s, v0, th, k, xi, rho, r, t) in cases {
            assert!(
                HestonParams::new(s, v0, th, k, xi, rho, r, t).is_err(),
                "Should reject ({s}, {v0}, {th}, {k}, {xi}, {rho}, {r}, {t})"
            );
        }
    }

    #[test]
    fn test_params_valid_rho_boundaries() {
        for rho in [-1.0, 0.0, 1.0] {
            assert!(HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, rho, 0.05, 1.0).is_ok());
        }
    }

    #[test]
    fn test_params_default() {
        let p: HestonParams<f64> = Default::default();
        assert_eq!(p.spot, 100.0);
        assert_eq!(p.v0, 0.04);
        assert_eq!(p.theta, 0.04);
        assert_eq!(p.kappa, 1.5);
        assert_eq!(p.xi, 0.3);
        assert_eq!(p.rho, -0.7);
        assert_eq!(p.maturity, 1.0);
    }

    #[test]
    fn test_params_with_psi_c() {
        let p = default_params().with_psi_c(2.0);
        assert!(p.is_ok());
        assert_eq!(p.unwrap().psi_c, 2.0);
        assert!(default_params().with_psi_c(-1.0).is_err());
    }

    #[test]
    fn test_params_with_epsilon() {
        let p = default_params().with_epsilon(1e-6);
        assert!(p.is_ok());
        assert!((p.unwrap().smoothing_epsilon - 1e-6).abs() < 1e-15);
        assert!(default_params().with_epsilon(-1e-6).is_err());
    }

    #[test]
    fn test_feller_condition() {
        // 2 * 1.5 * 0.04 = 0.12 > 0.3^2 = 0.09 → satisfied
        assert!(default_params().satisfies_feller());

        // 2 * 0.5 * 0.04 = 0.04 < 0.5^2 = 0.25 → violated
        let violated =
            HestonParams::new(100.0_f64, 0.04, 0.04, 0.5, 0.5, -0.7, 0.05, 1.0).unwrap();
        assert!(!violated.satisfies_feller());
    }

    #[test]
    fn test_feller_ratio() {
        let ratio = default_params().feller_ratio();
        assert!((ratio - 4.0 / 3.0).abs() < 1e-10);

        // 境界: 2 * 1.0 * 0.02 = 0.04 = 0.2^2
        let boundary =
            HestonParams::new(100.0_f64, 0.04, 0.02, 1.0, 0.2, -0.7, 0.05, 1.0).unwrap();
        assert!((boundary.feller_ratio() - 1.0).abs() < 1e-10);
        assert!(!boundary.satisfies_feller()); // strictly >
    }

    #[test]
    fn test_params_f32() {
        let p = HestonParams::new(100.0_f32, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0);
        assert!(p.is_ok());
    }

    // ----------------------------------------------------------------
    // HestonModel
    // ----------------------------------------------------------------

    #[test]
    fn test_model_new_valid_and_invalid() {
        assert!(HestonModel::new(default_params()).is_ok());

        let invalid = HestonParams {
            spot: -100.0_f64,
            ..Default::default()
        };
        assert!(HestonModel::new(invalid).is_err());
    }

    #[test]
    fn test_model_feller_warning() {
        let params =
            HestonParams::new(100.0_f64, 0.04, 0.04, 0.5, 0.5, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        assert!(!model.check_feller_condition());
        assert!(model.variance_floor() > 0.0);
    }

    // ----------------------------------------------------------------
    // QEモーメント
    // ----------------------------------------------------------------

    #[test]
    fn test_qe_moments_computation() {
        let model = default_model();
        let (m, s2, psi) = model.compute_qe_moments(0.04, 1.0 / 252.0);
        assert!(m > 0.0);
        assert!(s2 >= 0.0);
        assert!(psi >= 0.0);
    }

    #[test]
    fn test_qe_moments_mean_reversion() {
        let params =
            HestonParams::new(100.0_f64, 0.01, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let dt = 1.0 / 252.0;

        let (m_low, _, _) = model.compute_qe_moments(0.01, dt);
        assert!(m_low > 0.01, "Mean should revert up when v < theta");

        let (m_high, _, _) = model.compute_qe_moments(0.08, dt);
        assert!(m_high < 0.08, "Mean should revert down when v > theta");
    }

    #[test]
    fn test_qe_moments_analytical_match() {
        let model = default_model();
        let dt = 1.0 / 252.0;
        let v = 0.04_f64;

        let analytical_mean = 0.04 + (v - 0.04) * (-1.5 * dt).exp();
        let (qe_mean, _, _) = model.compute_qe_moments(v, dt);
        assert!((qe_mean - analytical_mean).abs() < 1e-10);
    }

    #[test]
    fn test_qe_psi_numerical_stability() {
        let model = default_model();
        let dt = 1.0 / 252.0;
        for v in [1e-10, 1e-8, 1e-4, 0.04, 0.5, 1.0] {
            let (m, s2, psi) = model.compute_qe_moments(v, dt);
            assert!(m.is_finite(), "Mean non-finite for v={v}");
            assert!(s2.is_finite() && s2 >= 0.0, "Var invalid for v={v}");
            assert!(psi.is_finite() && psi >= 0.0, "Psi invalid for v={v}");
        }
    }

    // ----------------------------------------------------------------
    // QEスキーム
    // ----------------------------------------------------------------

    #[test]
    fn test_qe_quadratic_scheme() {
        let model = default_model();
        let (m, s2, psi) = model.compute_qe_moments(0.04, 1.0 / 252.0);
        if psi < 1.5 {
            let v_next = model.qe_quadratic_step(m, s2, psi, 0.5);
            assert!(v_next >= 0.0 && v_next < 1.0);
        }
    }

    #[test]
    fn test_qe_exponential_scheme() {
        let params =
            HestonParams::new(100.0_f64, 0.001, 0.04, 0.5, 0.8, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let (m, _s2, psi) = model.compute_qe_moments(0.001, 1.0 / 12.0);
        let v_next = model.qe_exponential_step(m, psi, 0.5);
        assert!(v_next >= 0.0);
    }

    #[test]
    fn test_qe_variance_positivity() {
        let params =
            HestonParams::new(100.0_f64, 0.001, 0.04, 0.5, 0.8, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let dt = 1.0 / 252.0;
        for uv in [0.01, 0.1, 0.5, 0.9, 0.99] {
            let v_next = model.qe_variance_step(0.001, dt, uv);
            assert!(v_next >= 0.0, "Variance negative for uv={uv}: {v_next}");
        }
    }

    #[test]
    fn test_qe_smooth_transition_at_psi_c() {
        let model = default_model();
        let dt = 1.0 / 252.0;
        for uv in [0.3, 0.5, 0.7] {
            for v in [0.01, 0.04, 0.1] {
                let v1 = model.qe_variance_step(v, dt, uv);
                let v2 = model.qe_variance_step(v * 1.01, dt, uv);
                assert!((v2 - v1).abs() < 0.01, "Discontinuity at v={v}: {}", (v2 - v1).abs());
            }
        }
    }

    // ----------------------------------------------------------------
    // 相関ブラウン運動
    // ----------------------------------------------------------------

    #[test]
    fn test_correlated_brownian() {
        let model = default_model();
        let (dw_s, dw_v) = model.generate_correlated_brownian(0.5, -0.3);

        assert!((dw_s - 0.5).abs() < 1e-10);
        let expected = -0.7 * 0.5 + (1.0 - 0.49_f64).sqrt() * (-0.3);
        assert!((dw_v - expected).abs() < 1e-8);
    }

    #[test]
    fn test_correlated_brownian_extreme_rho() {
        // rho = -1: dW_V = -z1
        let params_neg =
            HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -1.0, 0.05, 1.0).unwrap();
        let model_neg = HestonModel::new(params_neg).unwrap();
        let (_, dw_v) = model_neg.generate_correlated_brownian(1.0, 0.5);
        assert!((dw_v - (-1.0)).abs() < 1e-10);

        // rho = 1: dW_V = z1
        let params_pos =
            HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, 1.0, 0.05, 1.0).unwrap();
        let model_pos = HestonModel::new(params_pos).unwrap();
        let (_, dw_v2) = model_pos.generate_correlated_brownian(1.0, 0.5);
        assert!((dw_v2 - 1.0).abs() < 1e-10);

        // rho = 0: dW_V = z2
        let params_zero =
            HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, 0.0, 0.05, 1.0).unwrap();
        let model_zero = HestonModel::new(params_zero).unwrap();
        let (_, dw_v3) = model_zero.generate_correlated_brownian(0.5, -0.3);
        assert!((dw_v3 - (-0.3)).abs() < 1e-8);
    }

    // ----------------------------------------------------------------
    // QEフルステップ
    // ----------------------------------------------------------------

    #[test]
    fn test_qe_full_step() {
        let model = default_model();
        let (s, v) = model.qe_step(100.0, 0.04, 1.0 / 252.0, 0.5, -0.3, 0.5);
        assert!(s > 0.0 && s > 50.0 && s < 150.0);
        assert!(v >= 0.0);
    }

    #[test]
    fn test_qe_step_multiple_shocks() {
        let model = default_model();
        let dt = 1.0 / 252.0;
        for (z1, z2, uv) in [(0.5, 0.0, 0.5), (1.5, 0.5, 0.9), (-1.5, -0.5, 0.1), (0.0, 0.0, 0.5)] {
            let (s, v) = model.qe_step(100.0, 0.04, dt, z1, z2, uv);
            assert!(s > 0.0, "Price must be positive for z1={z1}");
            assert!(v >= 0.0, "Variance must be non-negative for z1={z1}");
        }
    }

    #[test]
    fn test_qe_step_near_zero_variance() {
        let params = HestonParams::new(100.0_f64, 0.001, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0)
            .unwrap()
            .with_epsilon(1e-6)
            .unwrap();
        let model = HestonModel::new(params).unwrap();
        let (s, v) = model.qe_step(100.0, 1e-8, 1.0 / 252.0, 0.5, 0.0, 0.5);
        assert!(s.is_finite() && v.is_finite());
    }

    #[test]
    fn test_qe_price_mid_point_rule() {
        let model = default_model();
        let s_next = model.qe_price_step(100.0, 0.04, 0.05, 1.0 / 252.0, 0.5);
        assert!(s_next > 0.0);

        // Verify mid-point rule
        let dt = 1.0 / 252.0;
        let v_avg = (0.04 + 0.05) / 2.0;
        let expected = 100.0 * ((0.05 - 0.5 * v_avg) * dt + v_avg.sqrt() * dt.sqrt() * 0.5).exp();
        assert!((s_next - expected).abs() < 1e-7);
    }

    // ----------------------------------------------------------------
    // StochasticModel トレイト
    // ----------------------------------------------------------------

    #[test]
    fn test_stochastic_model_basics() {
        assert_eq!(HestonModel::<f64>::brownian_dim(), 2);
        assert_eq!(HestonModel::<f64>::model_name(), "Heston");
        assert_eq!(HestonModel::<f64>::num_factors(), 2);
    }

    #[test]
    fn test_stochastic_model_initial_state() {
        let params = default_params();
        let state = HestonModel::initial_state(&params);
        assert_eq!(state.first, 100.0);
        assert_eq!(state.second, 0.04);
    }

    #[test]
    fn test_stochastic_model_evolve_step() {
        let params = default_params();
        let state = HestonModel::initial_state(&params);
        let next = HestonModel::evolve_step(state, 1.0 / 252.0, &[0.5, 0.0, 0.5], &params);
        assert!(next.first > 0.0);
        assert!(next.second >= 0.0);
    }

    #[test]
    fn test_stochastic_model_shock_direction() {
        let params = default_params();
        let state = HestonModel::initial_state(&params);
        let dt = 1.0 / 252.0;

        let pos = HestonModel::evolve_step(state, dt, &[2.0, 0.0, 0.5], &params);
        assert!(pos.first > state.first, "Positive shock should increase price");

        let neg = HestonModel::evolve_step(state, dt, &[-2.0, 0.0, 0.5], &params);
        assert!(neg.first < state.first, "Negative shock should decrease price");
    }

    #[test]
    fn test_stochastic_model_multi_step_stability() {
        let params = default_params();
        let mut state = HestonModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        for _ in 0..252 {
            state = HestonModel::evolve_step(state, dt, &[0.0, 0.0, 0.5], &params);
            assert!(state.first > 0.0 && state.first.is_finite());
            assert!(state.second >= 0.0 && state.second.is_finite());
        }
    }

    #[test]
    fn test_stochastic_state_trait() {
        let state = TwoFactorState::<f64> {
            first: 100.0,
            second: 0.04,
        };
        assert_eq!(TwoFactorState::<f64>::dimension(), 2);
        assert_eq!(state.get(0), Some(100.0));
        assert_eq!(state.get(1), Some(0.04));
        assert_eq!(state.to_array(), vec![100.0, 0.04]);
    }

    #[test]
    fn test_stochastic_model_f32() {
        let params = HestonParams::new(100.0_f32, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 1.0).unwrap();
        let state = HestonModel::initial_state(&params);
        let next = HestonModel::evolve_step(state, 1.0_f32 / 252.0, &[0.5_f32, 0.0, 0.5], &params);
        assert!(next.first > 0.0_f32 && next.second >= 0.0_f32);
    }

    // ----------------------------------------------------------------
    // ロバストネス・安定性
    // ----------------------------------------------------------------

    #[test]
    fn test_feller_violation_variance_floor() {
        let params =
            HestonParams::new(100.0_f64, 0.04, 0.04, 0.5, 0.5, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        assert!(!params.satisfies_feller());

        let dt = 1.0 / 252.0;
        let mut v = 0.001;
        for _ in 0..100 {
            v = model.qe_variance_step(v, dt, 0.01);
            assert!(v >= 0.0, "Variance must never go negative: {v}");
        }
    }

    #[test]
    fn test_feller_violation_evolve_robustness() {
        let params =
            HestonParams::new(100.0_f64, 0.01, 0.01, 0.1, 0.8, -0.9, 0.05, 1.0).unwrap();
        assert!(!params.satisfies_feller());

        let mut state = HestonModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        for i in 0..500 {
            let z1 = (i as f64 * 0.1).sin();
            let z2 = (i as f64 * 0.17).cos();
            let uv = (i as f64 * 0.07).sin().abs();
            state = HestonModel::evolve_step(state, dt, &[z1, z2, uv], &params);
            assert!(state.first > 0.0 && state.first.is_finite(), "Price invalid at step {i}");
            assert!(state.second >= 0.0 && state.second.is_finite(), "Var invalid at step {i}");
        }
    }

    #[test]
    fn test_extreme_rho_path_stability() {
        for rho in [-0.99_f64, -0.5, 0.0, 0.5, 0.99] {
            let params =
                HestonParams::new(100.0, 0.04, 0.04, 1.5, 0.3, rho, 0.05, 1.0).unwrap();
            let mut state = HestonModel::initial_state(&params);
            let dt = 1.0 / 252.0;
            for i in 0..252 {
                state = HestonModel::evolve_step(state, dt, &[0.1, 0.1, 0.5], &params);
                assert!(state.first > 0.0 && state.first.is_finite(), "rho={rho} step {i}");
                assert!(state.second >= 0.0 && state.second.is_finite(), "rho={rho} step {i}");
            }
        }
    }

    #[test]
    fn test_extreme_mean_reversion_moments() {
        let params =
            HestonParams::new(100.0_f64, 0.04, 0.08, 10.0, 0.2, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();
        let (mean, var, _) = model.compute_qe_moments(0.01, 0.1);

        let expected = 0.08 + (0.01 - 0.08) * (-10.0 * 0.1_f64).exp();
        assert!((mean - expected).abs() < 0.01);
        assert!(var > 0.0);
    }

    #[test]
    fn test_boundary_variance_values() {
        let model = default_model();
        let dt = 1.0 / 252.0;
        for v in [1e-8, 1.0] {
            let v_next = model.qe_variance_step(v, dt, 0.5);
            assert!(v_next >= 0.0 && v_next.is_finite(), "Failed for v={v}: {v_next}");
        }
    }

    #[test]
    fn test_long_simulation_stability() {
        let params =
            HestonParams::new(100.0_f64, 0.04, 0.04, 1.5, 0.3, -0.7, 0.05, 5.0).unwrap();
        let mut state = HestonModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        for i in 0..(252 * 5) {
            let phase = i as f64 * 0.1;
            let dw = [0.1 * phase.sin(), 0.1 * phase.cos(), (phase * 0.7).sin().abs()];
            state = HestonModel::evolve_step(state, dt, &dw, &params);
        }
        assert!(state.first > 0.0 && state.first.is_finite());
        assert!(state.second >= 0.0 && state.second.is_finite());
    }

    #[test]
    fn test_variance_mean_reversion_convergence() {
        let params =
            HestonParams::new(100.0_f64, 0.04, 0.06, 2.0, 0.3, -0.7, 0.05, 1.0).unwrap();
        let model = HestonModel::new(params).unwrap();

        let mut mean = 0.04_f64;
        let dt = 0.01;
        for _ in 0..100 {
            let (m, _, _) = model.compute_qe_moments(mean, dt);
            mean = m;
        }
        let expected = 0.06 + (0.04 - 0.06) * (-2.0 * 1.0_f64).exp();
        assert!((mean - expected).abs() < 1e-6);
    }
}
