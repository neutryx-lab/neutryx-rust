//! VolCubeカリブレーション設定。
//!
//! # Requirements: 6.1-6.5, 10.1-10.2
//!
//! このモジュールはVolCubeカリブレーションの設定型を定義する。
//! Smile補間方式、外挿方式、Strike軸表現、最適化アルゴリズムを指定できる。

/// Smile補間方式。
///
/// # Requirements: 6.1, 10.1, 10.2
///
/// VolCube内でのSmile軸の補間方法を指定する。
/// 各expiry-tenorスライスでのstrike軸補間に使用される。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterpolationMethod {
    /// SABR stochastic volatility model。
    /// Hagan公式によるimplied volatility計算。
    #[default]
    Sabr,
    /// Stochastic Volatility Inspired (SVI) parameterisation。
    /// Jim Gatheral (2004) の平滑なarbitrage-free smile。
    Svi,
    /// 線形補間。
    /// 単純だが計算が高速。
    Linear,
    /// 自然三次スプライン補間。
    /// C2連続性を保証。
    CubicSpline,
    /// 一定ボラティリティ（flat smile）。
    /// テスト・デバッグ用途。
    FlatVol,
}

/// 外挿方式。
///
/// # Requirements: 6.2
///
/// VolCubeドメイン外のクエリに対する振る舞いを指定する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrapolationMethod {
    /// 境界値で一定（flat extrapolation）。
    /// 安全だがsmileの形状を考慮しない。
    #[default]
    Flat,
    /// 線形外挿。
    /// 境界での勾配を維持。
    Linear,
    /// エラーを返す（外挿禁止）。
    /// 厳密なドメイン制約が必要な場合。
    Error,
}

/// Strike軸表現方式。
///
/// # Requirements: 6.3
///
/// Smile軸のstrike表現を指定する。
/// 市場慣行やモデル要件に応じて選択。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrikeAxisType {
    /// 絶対strike値（K）。
    Absolute,
    /// Moneyness（K/F）。
    Moneyness,
    /// Log-moneyness（ln(K/F)）。
    /// SABRやSVIで一般的。
    #[default]
    LogMoneyness,
    /// Delta（オプションdelta）。
    /// FX市場慣行。
    Delta,
}

/// 最適化アルゴリズム。
///
/// # Requirements: 6.4
///
/// カリブレーション時の最適化アルゴリズムを指定する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptimizerType {
    /// Levenberg-Marquardt法。
    /// 非線形最小二乗問題に適した勾配ベース手法。
    #[default]
    LevenbergMarquardt,
    /// Nelder-Mead法（シンプレックス法）。
    /// 勾配不要の直接探索法。
    NelderMead,
}

/// カリブレーション順序。
///
/// # Requirements: 5.8
///
/// VolCubeグリッドをどの順序でカリブレーションするかを指定する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalibrationOrder {
    /// Expiry優先（各expiryで全tenorをカリブレーション後、次のexpiryへ）。
    #[default]
    ExpiryFirst,
    /// Tenor優先（各tenorで全expiryをカリブレーション後、次のtenorへ）。
    TenorFirst,
    /// 並列カリブレーション（順序は実装依存）。
    Parallel,
}

/// SABRパラメータ軸別補間設定。
///
/// # Requirements: 3.4, 3.5
///
/// 各SABRパラメータ（α、β、ρ、ν）をExpiry軸とTenor軸で
/// どの補間方式を使用するかを設定する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SabrParameterInterpolation {
    /// α（ボラティリティレベル）のExpiry軸補間。
    pub alpha_expiry: AxisInterpolationMethod,
    /// α（ボラティリティレベル）のTenor軸補間。
    pub alpha_tenor: AxisInterpolationMethod,
    /// β（CEV指数）のExpiry軸補間。
    pub beta_expiry: AxisInterpolationMethod,
    /// β（CEV指数）のTenor軸補間。
    pub beta_tenor: AxisInterpolationMethod,
    /// ρ（相関）のExpiry軸補間。
    pub rho_expiry: AxisInterpolationMethod,
    /// ρ（相関）のTenor軸補間。
    pub rho_tenor: AxisInterpolationMethod,
    /// ν（ボラティリティのボラティリティ）のExpiry軸補間。
    pub nu_expiry: AxisInterpolationMethod,
    /// ν（ボラティリティのボラティリティ）のTenor軸補間。
    pub nu_tenor: AxisInterpolationMethod,
}

impl Default for SabrParameterInterpolation {
    fn default() -> Self {
        // デフォルトは全パラメータで線形補間
        Self {
            alpha_expiry: AxisInterpolationMethod::Linear,
            alpha_tenor: AxisInterpolationMethod::Linear,
            beta_expiry: AxisInterpolationMethod::Flat,  // βは通常固定なのでFlat
            beta_tenor: AxisInterpolationMethod::Flat,
            rho_expiry: AxisInterpolationMethod::Linear,
            rho_tenor: AxisInterpolationMethod::Linear,
            nu_expiry: AxisInterpolationMethod::Linear,
            nu_tenor: AxisInterpolationMethod::Linear,
        }
    }
}

impl SabrParameterInterpolation {
    /// 新しい補間設定を作成（デフォルト値）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 全パラメータを同じ補間方式に設定。
    pub fn uniform(method: AxisInterpolationMethod) -> Self {
        Self {
            alpha_expiry: method,
            alpha_tenor: method,
            beta_expiry: method,
            beta_tenor: method,
            rho_expiry: method,
            rho_tenor: method,
            nu_expiry: method,
            nu_tenor: method,
        }
    }

    /// αの補間方式を設定。
    pub fn with_alpha(mut self, expiry: AxisInterpolationMethod, tenor: AxisInterpolationMethod) -> Self {
        self.alpha_expiry = expiry;
        self.alpha_tenor = tenor;
        self
    }

    /// βの補間方式を設定。
    pub fn with_beta(mut self, expiry: AxisInterpolationMethod, tenor: AxisInterpolationMethod) -> Self {
        self.beta_expiry = expiry;
        self.beta_tenor = tenor;
        self
    }

    /// ρの補間方式を設定。
    pub fn with_rho(mut self, expiry: AxisInterpolationMethod, tenor: AxisInterpolationMethod) -> Self {
        self.rho_expiry = expiry;
        self.rho_tenor = tenor;
        self
    }

    /// νの補間方式を設定。
    pub fn with_nu(mut self, expiry: AxisInterpolationMethod, tenor: AxisInterpolationMethod) -> Self {
        self.nu_expiry = expiry;
        self.nu_tenor = tenor;
        self
    }
}

/// 軸方向の補間方式。
///
/// # Requirements: 3.4, 3.5
///
/// Expiry軸またはTenor軸でのパラメータ補間方式を指定する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AxisInterpolationMethod {
    /// 一定値（最近傍）。
    Flat,
    /// 線形補間。
    #[default]
    Linear,
    /// 対数線形補間（αなど正の値に適切）。
    LogLinear,
    /// 自然三次スプライン補間。
    CubicSpline,
}

/// VolCubeカリブレーション設定。
///
/// # Requirements: 6.5
///
/// VolCubeBuilder用の包括的な設定構造体。
/// Builderパターンで設定を構築できる。
///
/// # 例
///
/// ```
/// use pricer_models::market::volcube::{
///     VolCubeConfig, InterpolationMethod, ExtrapolationMethod
/// };
///
/// let config = VolCubeConfig::default()
///     .with_interpolation(InterpolationMethod::Sabr)
///     .with_extrapolation(ExtrapolationMethod::Flat)
///     .with_validate_arbitrage_free(true);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct VolCubeConfig {
    /// Smile補間方式。
    pub interpolation: InterpolationMethod,
    /// 外挿方式。
    pub extrapolation: ExtrapolationMethod,
    /// Strike軸表現方式。
    pub strike_axis: StrikeAxisType,
    /// 最適化アルゴリズム。
    pub optimizer: OptimizerType,
    /// Arbitrage-free条件検証フラグ。
    pub validate_arbitrage_free: bool,
    /// SABR beta固定値（Noneの場合はカリブレーション対象）。
    pub sabr_beta: Option<f64>,
    /// Shifted SABRのshift値（負金利対応）。
    pub sabr_shift: f64,
    /// カリブレーション最大反復回数。
    pub max_iterations: usize,
    /// カリブレーション収束判定閾値。
    pub tolerance: f64,
    /// Discount curve名（forward rate計算に使用）。
    ///
    /// # Requirements: 5.4
    pub discount_curve: Option<String>,
    /// Projection curve名（forward swap rate計算に使用）。
    ///
    /// # Requirements: 5.4
    pub projection_curve: Option<String>,
    /// カリブレーション順序。
    ///
    /// # Requirements: 5.8
    pub calibration_order: CalibrationOrder,
    /// SABRパラメータ軸別補間設定。
    ///
    /// # Requirements: 3.4, 3.5
    pub sabr_param_interpolation: SabrParameterInterpolation,
}

impl Default for VolCubeConfig {
    fn default() -> Self {
        Self {
            interpolation: InterpolationMethod::default(),
            extrapolation: ExtrapolationMethod::default(),
            strike_axis: StrikeAxisType::default(),
            optimizer: OptimizerType::default(),
            validate_arbitrage_free: false,
            sabr_beta: Some(0.5), // 一般的なデフォルト値
            sabr_shift: 0.0,
            max_iterations: 100,
            tolerance: 1e-8,
            discount_curve: None,
            projection_curve: None,
            calibration_order: CalibrationOrder::default(),
            sabr_param_interpolation: SabrParameterInterpolation::default(),
        }
    }
}

impl VolCubeConfig {
    /// 新しい設定を作成（デフォルト値）。
    pub fn new() -> Self { Self::default() }

    /// 通貨別デフォルト設定を作成。
    ///
    /// # Requirements: 5.10
    ///
    /// 通貨に応じたデフォルトのdiscount curveとprojection curveを設定する。
    /// - USD → SOFR-OIS (discount), SOFR-3M (projection)
    /// - EUR → ESTR-OIS (discount), ESTR-3M (projection)
    /// - JPY → TONA-OIS (discount), TONA-3M (projection)
    /// - GBP → SONIA-OIS (discount), SONIA-3M (projection)
    /// - CHF → SARON-OIS (discount), SARON-3M (projection)
    ///
    /// # 例
    ///
    /// ```
    /// use pricer_models::market::volcube::{VolCubeConfig, Currency};
    ///
    /// let config = VolCubeConfig::default_for_currency(Currency::Usd);
    /// assert_eq!(config.discount_curve, Some("USD-SOFR-OIS".to_string()));
    /// ```
    pub fn default_for_currency(currency: super::quote::Currency) -> Self {
        use super::quote::Currency;

        let (discount, projection) = match currency {
            Currency::Usd => ("USD-SOFR-OIS", "USD-SOFR-3M"),
            Currency::Eur => ("EUR-ESTR-OIS", "EUR-ESTR-3M"),
            Currency::Jpy => ("JPY-TONA-OIS", "JPY-TONA-3M"),
            Currency::Gbp => ("GBP-SONIA-OIS", "GBP-SONIA-3M"),
            Currency::Chf => ("CHF-SARON-OIS", "CHF-SARON-3M"),
            Currency::Other(_) => ("OTHER-OIS", "OTHER-3M"),
        };

        Self::default()
            .with_discount_curve(discount)
            .with_projection_curve(projection)
    }

    /// 補間方式を設定。
    pub fn with_interpolation(mut self, method: InterpolationMethod) -> Self {
        self.interpolation = method;
        self
    }

    /// 外挿方式を設定。
    pub fn with_extrapolation(mut self, method: ExtrapolationMethod) -> Self {
        self.extrapolation = method;
        self
    }

    /// Strike軸表現を設定。
    pub fn with_strike_axis(mut self, axis: StrikeAxisType) -> Self {
        self.strike_axis = axis;
        self
    }

    /// 最適化アルゴリズムを設定。
    pub fn with_optimizer(mut self, optimizer: OptimizerType) -> Self {
        self.optimizer = optimizer;
        self
    }

    /// Arbitrage-free検証を設定。
    pub fn with_validate_arbitrage_free(mut self, validate: bool) -> Self {
        self.validate_arbitrage_free = validate;
        self
    }

    /// SABR beta固定値を設定。
    ///
    /// # Arguments
    ///
    /// * `beta` - β値（0.0〜1.0）。Noneの場合はカリブレーション対象。
    ///
    /// # 一般的なβ値
    ///
    /// * `0.0` - Normal model (Bachelier) - 負金利環境でよく使用
    /// * `0.5` - CIR (Cox-Ingersoll-Ross) - 最も一般的なデフォルト
    /// * `1.0` - Lognormal model (Black-Scholes) - 伝統的な金利モデル
    pub fn with_sabr_beta(mut self, beta: Option<f64>) -> Self {
        self.sabr_beta = beta;
        self
    }

    /// SABR beta = 0.0 を設定（Normal model / Bachelier）。
    ///
    /// # Requirements: 4.2, 9.2
    ///
    /// 負金利環境で一般的に使用されるNormal SABRモデル。
    /// ボラティリティは絶対値（bps単位）として解釈される。
    pub fn with_normal_sabr(self) -> Self {
        self.with_sabr_beta(Some(0.0))
    }

    /// SABR beta = 0.5 を設定（CIR / Square-root model）。
    ///
    /// # Requirements: 4.2, 9.2
    ///
    /// 金利デリバティブで最も一般的に使用されるデフォルト設定。
    /// CIRプロセスに対応し、正の金利を仮定。
    pub fn with_cir_sabr(self) -> Self {
        self.with_sabr_beta(Some(0.5))
    }

    /// SABR beta = 1.0 を設定（Lognormal model / Black）。
    ///
    /// # Requirements: 4.2, 9.2
    ///
    /// 伝統的なBlack-Scholesタイプのログノーマルモデル。
    /// 正の金利を強く仮定し、負金利環境では不適切。
    pub fn with_lognormal_sabr(self) -> Self {
        self.with_sabr_beta(Some(1.0))
    }

    /// βをカリブレーション対象に設定（固定しない）。
    ///
    /// # Requirements: 4.2, 9.2
    ///
    /// βを他のSABRパラメータと共にカリブレーションする。
    /// より多くのデータ点が必要だが、最適なフィットを得られる可能性がある。
    pub fn with_calibrated_beta(self) -> Self {
        self.with_sabr_beta(None)
    }

    /// SABR shiftを設定（負金利対応）。
    pub fn with_sabr_shift(mut self, shift: f64) -> Self {
        self.sabr_shift = shift;
        self
    }

    /// 最大反復回数を設定。
    pub fn with_max_iterations(mut self, iterations: usize) -> Self {
        self.max_iterations = iterations;
        self
    }

    /// 収束判定閾値を設定。
    pub fn with_tolerance(mut self, tol: f64) -> Self {
        self.tolerance = tol;
        self
    }

    /// Discount curve名を設定。
    ///
    /// # Requirements: 5.4
    pub fn with_discount_curve(mut self, curve_name: impl Into<String>) -> Self {
        self.discount_curve = Some(curve_name.into());
        self
    }

    /// Projection curve名を設定。
    ///
    /// # Requirements: 5.4
    pub fn with_projection_curve(mut self, curve_name: impl Into<String>) -> Self {
        self.projection_curve = Some(curve_name.into());
        self
    }

    /// Discount curveとProjection curveを同時に設定。
    ///
    /// # Requirements: 5.4
    pub fn with_curves(
        mut self,
        discount: impl Into<String>,
        projection: impl Into<String>,
    ) -> Self {
        self.discount_curve = Some(discount.into());
        self.projection_curve = Some(projection.into());
        self
    }

    /// カリブレーション順序を設定。
    ///
    /// # Requirements: 5.8
    pub fn with_calibration_order(mut self, order: CalibrationOrder) -> Self {
        self.calibration_order = order;
        self
    }

    /// SABRパラメータ軸別補間設定を設定。
    ///
    /// # Requirements: 3.4, 3.5
    ///
    /// # 例
    ///
    /// ```
    /// use pricer_models::market::volcube::{
    ///     VolCubeConfig, SabrParameterInterpolation, AxisInterpolationMethod
    /// };
    ///
    /// let param_interp = SabrParameterInterpolation::default()
    ///     .with_alpha(AxisInterpolationMethod::LogLinear, AxisInterpolationMethod::Linear)
    ///     .with_nu(AxisInterpolationMethod::CubicSpline, AxisInterpolationMethod::CubicSpline);
    ///
    /// let config = VolCubeConfig::default()
    ///     .with_sabr_param_interpolation(param_interp);
    /// ```
    pub fn with_sabr_param_interpolation(mut self, interp: SabrParameterInterpolation) -> Self {
        self.sabr_param_interpolation = interp;
        self
    }

    /// 設定を検証。
    pub fn validate(&self) -> Result<(), String> {
        if let Some(beta) = self.sabr_beta {
            if !(0.0..=1.0).contains(&beta) {
                return Err(format!("SABR beta must be in [0, 1], got: {}", beta));
            }
        }
        if self.tolerance <= 0.0 {
            return Err(format!(
                "Tolerance must be positive, got: {}",
                self.tolerance
            ));
        }
        if self.max_iterations == 0 {
            return Err("Max iterations must be at least 1".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // InterpolationMethod Tests
    // =========================================================================

    #[test]
    fn test_interpolation_method_default() {
        let method = InterpolationMethod::default();
        assert_eq!(method, InterpolationMethod::Sabr);
    }

    #[test]
    fn test_interpolation_method_debug() {
        let method = InterpolationMethod::Svi;
        let debug_str = format!("{:?}", method);
        assert_eq!(debug_str, "Svi");
    }

    #[test]
    fn test_interpolation_method_clone() {
        let method = InterpolationMethod::CubicSpline;
        let cloned = method.clone();
        assert_eq!(method, cloned);
    }

    #[test]
    fn test_interpolation_method_copy() {
        let method = InterpolationMethod::Linear;
        let copied = method;
        assert_eq!(method, copied);
    }

    #[test]
    fn test_interpolation_method_all_variants() {
        let variants = [
            InterpolationMethod::Sabr,
            InterpolationMethod::Svi,
            InterpolationMethod::Linear,
            InterpolationMethod::CubicSpline,
            InterpolationMethod::FlatVol,
        ];
        assert_eq!(variants.len(), 5);
    }

    // =========================================================================
    // ExtrapolationMethod Tests
    // =========================================================================

    #[test]
    fn test_extrapolation_method_default() {
        let method = ExtrapolationMethod::default();
        assert_eq!(method, ExtrapolationMethod::Flat);
    }

    #[test]
    fn test_extrapolation_method_debug() {
        let method = ExtrapolationMethod::Error;
        let debug_str = format!("{:?}", method);
        assert_eq!(debug_str, "Error");
    }

    #[test]
    fn test_extrapolation_method_clone() {
        let method = ExtrapolationMethod::Linear;
        let cloned = method.clone();
        assert_eq!(method, cloned);
    }

    #[test]
    fn test_extrapolation_method_copy() {
        let method = ExtrapolationMethod::Flat;
        let copied = method;
        assert_eq!(method, copied);
    }

    #[test]
    fn test_extrapolation_method_all_variants() {
        let variants = [
            ExtrapolationMethod::Flat,
            ExtrapolationMethod::Linear,
            ExtrapolationMethod::Error,
        ];
        assert_eq!(variants.len(), 3);
    }

    // =========================================================================
    // StrikeAxisType Tests
    // =========================================================================

    #[test]
    fn test_strike_axis_type_default() {
        let axis = StrikeAxisType::default();
        assert_eq!(axis, StrikeAxisType::LogMoneyness);
    }

    #[test]
    fn test_strike_axis_type_debug() {
        let axis = StrikeAxisType::Delta;
        let debug_str = format!("{:?}", axis);
        assert_eq!(debug_str, "Delta");
    }

    #[test]
    fn test_strike_axis_type_clone() {
        let axis = StrikeAxisType::Moneyness;
        let cloned = axis.clone();
        assert_eq!(axis, cloned);
    }

    #[test]
    fn test_strike_axis_type_copy() {
        let axis = StrikeAxisType::Absolute;
        let copied = axis;
        assert_eq!(axis, copied);
    }

    #[test]
    fn test_strike_axis_type_all_variants() {
        let variants = [
            StrikeAxisType::Absolute,
            StrikeAxisType::Moneyness,
            StrikeAxisType::LogMoneyness,
            StrikeAxisType::Delta,
        ];
        assert_eq!(variants.len(), 4);
    }

    // =========================================================================
    // OptimizerType Tests
    // =========================================================================

    #[test]
    fn test_optimizer_type_default() {
        let opt = OptimizerType::default();
        assert_eq!(opt, OptimizerType::LevenbergMarquardt);
    }

    #[test]
    fn test_optimizer_type_debug() {
        let opt = OptimizerType::NelderMead;
        let debug_str = format!("{:?}", opt);
        assert_eq!(debug_str, "NelderMead");
    }

    #[test]
    fn test_optimizer_type_clone() {
        let opt = OptimizerType::LevenbergMarquardt;
        let cloned = opt.clone();
        assert_eq!(opt, cloned);
    }

    #[test]
    fn test_optimizer_type_copy() {
        let opt = OptimizerType::NelderMead;
        let copied = opt;
        assert_eq!(opt, copied);
    }

    #[test]
    fn test_optimizer_type_all_variants() {
        let variants = [OptimizerType::LevenbergMarquardt, OptimizerType::NelderMead];
        assert_eq!(variants.len(), 2);
    }

    // =========================================================================
    // VolCubeConfig Tests
    // =========================================================================

    #[test]
    fn test_volcube_config_default() {
        let config = VolCubeConfig::default();
        assert_eq!(config.interpolation, InterpolationMethod::Sabr);
        assert_eq!(config.extrapolation, ExtrapolationMethod::Flat);
        assert_eq!(config.strike_axis, StrikeAxisType::LogMoneyness);
        assert_eq!(config.optimizer, OptimizerType::LevenbergMarquardt);
        assert!(!config.validate_arbitrage_free);
        assert_eq!(config.sabr_beta, Some(0.5));
        assert_eq!(config.sabr_shift, 0.0);
        assert_eq!(config.max_iterations, 100);
        assert!((config.tolerance - 1e-8).abs() < 1e-15);
    }

    #[test]
    fn test_volcube_config_new() {
        let config = VolCubeConfig::new();
        assert_eq!(config, VolCubeConfig::default());
    }

    #[test]
    fn test_volcube_config_builder_interpolation() {
        let config = VolCubeConfig::default().with_interpolation(InterpolationMethod::Svi);
        assert_eq!(config.interpolation, InterpolationMethod::Svi);
    }

    #[test]
    fn test_volcube_config_builder_extrapolation() {
        let config = VolCubeConfig::default().with_extrapolation(ExtrapolationMethod::Error);
        assert_eq!(config.extrapolation, ExtrapolationMethod::Error);
    }

    #[test]
    fn test_volcube_config_builder_strike_axis() {
        let config = VolCubeConfig::default().with_strike_axis(StrikeAxisType::Delta);
        assert_eq!(config.strike_axis, StrikeAxisType::Delta);
    }

    #[test]
    fn test_volcube_config_builder_optimizer() {
        let config = VolCubeConfig::default().with_optimizer(OptimizerType::NelderMead);
        assert_eq!(config.optimizer, OptimizerType::NelderMead);
    }

    #[test]
    fn test_volcube_config_builder_validate_arbitrage_free() {
        let config = VolCubeConfig::default().with_validate_arbitrage_free(true);
        assert!(config.validate_arbitrage_free);
    }

    #[test]
    fn test_volcube_config_builder_sabr_beta() {
        let config = VolCubeConfig::default().with_sabr_beta(Some(0.25));
        assert_eq!(config.sabr_beta, Some(0.25));

        let config_none = VolCubeConfig::default().with_sabr_beta(None);
        assert_eq!(config_none.sabr_beta, None);
    }

    #[test]
    fn test_volcube_config_with_normal_sabr() {
        let config = VolCubeConfig::default().with_normal_sabr();
        assert_eq!(config.sabr_beta, Some(0.0));
    }

    #[test]
    fn test_volcube_config_with_cir_sabr() {
        let config = VolCubeConfig::default().with_cir_sabr();
        assert_eq!(config.sabr_beta, Some(0.5));
    }

    #[test]
    fn test_volcube_config_with_lognormal_sabr() {
        let config = VolCubeConfig::default().with_lognormal_sabr();
        assert_eq!(config.sabr_beta, Some(1.0));
    }

    #[test]
    fn test_volcube_config_with_calibrated_beta() {
        let config = VolCubeConfig::default().with_calibrated_beta();
        assert_eq!(config.sabr_beta, None);
    }

    #[test]
    fn test_volcube_config_sabr_beta_chaining() {
        // Normal → CIR
        let config = VolCubeConfig::default()
            .with_normal_sabr()
            .with_cir_sabr();
        assert_eq!(config.sabr_beta, Some(0.5));

        // Lognormal → Calibrated
        let config = VolCubeConfig::default()
            .with_lognormal_sabr()
            .with_calibrated_beta();
        assert_eq!(config.sabr_beta, None);
    }

    #[test]
    fn test_volcube_config_builder_sabr_shift() {
        let config = VolCubeConfig::default().with_sabr_shift(0.03);
        assert!((config.sabr_shift - 0.03).abs() < 1e-15);
    }

    #[test]
    fn test_volcube_config_builder_max_iterations() {
        let config = VolCubeConfig::default().with_max_iterations(500);
        assert_eq!(config.max_iterations, 500);
    }

    #[test]
    fn test_volcube_config_builder_tolerance() {
        let config = VolCubeConfig::default().with_tolerance(1e-10);
        assert!((config.tolerance - 1e-10).abs() < 1e-15);
    }

    #[test]
    fn test_volcube_config_builder_chain() {
        let config = VolCubeConfig::default()
            .with_interpolation(InterpolationMethod::Svi)
            .with_extrapolation(ExtrapolationMethod::Linear)
            .with_strike_axis(StrikeAxisType::Moneyness)
            .with_optimizer(OptimizerType::NelderMead)
            .with_validate_arbitrage_free(true)
            .with_sabr_beta(Some(1.0))
            .with_sabr_shift(0.02)
            .with_max_iterations(200)
            .with_tolerance(1e-6);

        assert_eq!(config.interpolation, InterpolationMethod::Svi);
        assert_eq!(config.extrapolation, ExtrapolationMethod::Linear);
        assert_eq!(config.strike_axis, StrikeAxisType::Moneyness);
        assert_eq!(config.optimizer, OptimizerType::NelderMead);
        assert!(config.validate_arbitrage_free);
        assert_eq!(config.sabr_beta, Some(1.0));
        assert!((config.sabr_shift - 0.02).abs() < 1e-15);
        assert_eq!(config.max_iterations, 200);
        assert!((config.tolerance - 1e-6).abs() < 1e-15);
    }

    #[test]
    fn test_volcube_config_validate_valid() {
        let config = VolCubeConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_volcube_config_validate_beta_out_of_range_high() {
        let config = VolCubeConfig::default().with_sabr_beta(Some(1.5));
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("beta"));
    }

    #[test]
    fn test_volcube_config_validate_beta_out_of_range_low() {
        let config = VolCubeConfig::default().with_sabr_beta(Some(-0.1));
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("beta"));
    }

    #[test]
    fn test_volcube_config_validate_beta_none() {
        let config = VolCubeConfig::default().with_sabr_beta(None);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_volcube_config_validate_tolerance_zero() {
        let config = VolCubeConfig::default().with_tolerance(0.0);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Tolerance"));
    }

    #[test]
    fn test_volcube_config_validate_tolerance_negative() {
        let config = VolCubeConfig::default().with_tolerance(-1e-8);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Tolerance"));
    }

    #[test]
    fn test_volcube_config_validate_max_iterations_zero() {
        let config = VolCubeConfig::default().with_max_iterations(0);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("iterations"));
    }

    #[test]
    fn test_volcube_config_clone() {
        let config = VolCubeConfig::default().with_interpolation(InterpolationMethod::Svi);
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn test_volcube_config_debug() {
        let config = VolCubeConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("VolCubeConfig"));
        assert!(debug_str.contains("Sabr"));
    }

    // =========================================================================
    // CalibrationOrder Tests
    // =========================================================================

    #[test]
    fn test_calibration_order_default() {
        let order = CalibrationOrder::default();
        assert_eq!(order, CalibrationOrder::ExpiryFirst);
    }

    #[test]
    fn test_calibration_order_all_variants() {
        let variants = [
            CalibrationOrder::ExpiryFirst,
            CalibrationOrder::TenorFirst,
            CalibrationOrder::Parallel,
        ];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn test_calibration_order_clone_copy() {
        let order = CalibrationOrder::TenorFirst;
        let cloned = order.clone();
        let copied = order;
        assert_eq!(order, cloned);
        assert_eq!(order, copied);
    }

    // =========================================================================
    // VolCubeConfig Curve Settings Tests
    // =========================================================================

    #[test]
    fn test_volcube_config_default_curve_settings() {
        let config = VolCubeConfig::default();
        assert!(config.discount_curve.is_none());
        assert!(config.projection_curve.is_none());
        assert_eq!(config.calibration_order, CalibrationOrder::ExpiryFirst);
    }

    #[test]
    fn test_volcube_config_with_discount_curve() {
        let config = VolCubeConfig::default().with_discount_curve("USD-SOFR-OIS");
        assert_eq!(config.discount_curve, Some("USD-SOFR-OIS".to_string()));
    }

    #[test]
    fn test_volcube_config_with_projection_curve() {
        let config = VolCubeConfig::default().with_projection_curve("USD-SOFR-3M");
        assert_eq!(config.projection_curve, Some("USD-SOFR-3M".to_string()));
    }

    #[test]
    fn test_volcube_config_with_curves() {
        let config = VolCubeConfig::default().with_curves("USD-SOFR-OIS", "USD-SOFR-3M");
        assert_eq!(config.discount_curve, Some("USD-SOFR-OIS".to_string()));
        assert_eq!(config.projection_curve, Some("USD-SOFR-3M".to_string()));
    }

    #[test]
    fn test_volcube_config_with_calibration_order() {
        let config = VolCubeConfig::default().with_calibration_order(CalibrationOrder::TenorFirst);
        assert_eq!(config.calibration_order, CalibrationOrder::TenorFirst);

        let config_parallel =
            VolCubeConfig::default().with_calibration_order(CalibrationOrder::Parallel);
        assert_eq!(config_parallel.calibration_order, CalibrationOrder::Parallel);
    }

    #[test]
    fn test_volcube_config_full_chain_with_curves() {
        let config = VolCubeConfig::default()
            .with_interpolation(InterpolationMethod::Sabr)
            .with_extrapolation(ExtrapolationMethod::Flat)
            .with_sabr_beta(Some(0.5))
            .with_curves("USD-SOFR-OIS", "USD-SOFR-3M")
            .with_calibration_order(CalibrationOrder::ExpiryFirst)
            .with_max_iterations(200)
            .with_tolerance(1e-10);

        assert_eq!(config.discount_curve, Some("USD-SOFR-OIS".to_string()));
        assert_eq!(config.projection_curve, Some("USD-SOFR-3M".to_string()));
        assert_eq!(config.calibration_order, CalibrationOrder::ExpiryFirst);
        assert_eq!(config.max_iterations, 200);
    }

    // =========================================================================
    // VolCubeConfig::default_for_currency Tests
    // =========================================================================

    #[test]
    fn test_volcube_config_default_for_currency_usd() {
        use super::super::quote::Currency;

        let config = VolCubeConfig::default_for_currency(Currency::Usd);
        assert_eq!(config.discount_curve, Some("USD-SOFR-OIS".to_string()));
        assert_eq!(config.projection_curve, Some("USD-SOFR-3M".to_string()));
    }

    #[test]
    fn test_volcube_config_default_for_currency_eur() {
        use super::super::quote::Currency;

        let config = VolCubeConfig::default_for_currency(Currency::Eur);
        assert_eq!(config.discount_curve, Some("EUR-ESTR-OIS".to_string()));
        assert_eq!(config.projection_curve, Some("EUR-ESTR-3M".to_string()));
    }

    #[test]
    fn test_volcube_config_default_for_currency_jpy() {
        use super::super::quote::Currency;

        let config = VolCubeConfig::default_for_currency(Currency::Jpy);
        assert_eq!(config.discount_curve, Some("JPY-TONA-OIS".to_string()));
        assert_eq!(config.projection_curve, Some("JPY-TONA-3M".to_string()));
    }

    #[test]
    fn test_volcube_config_default_for_currency_gbp() {
        use super::super::quote::Currency;

        let config = VolCubeConfig::default_for_currency(Currency::Gbp);
        assert_eq!(config.discount_curve, Some("GBP-SONIA-OIS".to_string()));
        assert_eq!(config.projection_curve, Some("GBP-SONIA-3M".to_string()));
    }

    #[test]
    fn test_volcube_config_default_for_currency_chf() {
        use super::super::quote::Currency;

        let config = VolCubeConfig::default_for_currency(Currency::Chf);
        assert_eq!(config.discount_curve, Some("CHF-SARON-OIS".to_string()));
        assert_eq!(config.projection_curve, Some("CHF-SARON-3M".to_string()));
    }

    #[test]
    fn test_volcube_config_default_for_currency_other() {
        use super::super::quote::Currency;

        let config = VolCubeConfig::default_for_currency(Currency::Other(999));
        assert_eq!(config.discount_curve, Some("OTHER-OIS".to_string()));
        assert_eq!(config.projection_curve, Some("OTHER-3M".to_string()));
    }

    #[test]
    fn test_volcube_config_default_for_currency_then_customize() {
        use super::super::quote::Currency;

        let config = VolCubeConfig::default_for_currency(Currency::Usd)
            .with_sabr_beta(Some(0.7))
            .with_calibration_order(CalibrationOrder::TenorFirst);

        assert_eq!(config.discount_curve, Some("USD-SOFR-OIS".to_string()));
        assert_eq!(config.sabr_beta, Some(0.7));
        assert_eq!(config.calibration_order, CalibrationOrder::TenorFirst);
    }

    // =========================================================================
    // AxisInterpolationMethod Tests
    // =========================================================================

    #[test]
    fn test_axis_interpolation_method_default() {
        let method = AxisInterpolationMethod::default();
        assert_eq!(method, AxisInterpolationMethod::Linear);
    }

    #[test]
    fn test_axis_interpolation_method_all_variants() {
        let variants = [
            AxisInterpolationMethod::Flat,
            AxisInterpolationMethod::Linear,
            AxisInterpolationMethod::LogLinear,
            AxisInterpolationMethod::CubicSpline,
        ];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn test_axis_interpolation_method_clone_copy() {
        let method = AxisInterpolationMethod::LogLinear;
        let cloned = method.clone();
        let copied = method;
        assert_eq!(method, cloned);
        assert_eq!(method, copied);
    }

    // =========================================================================
    // SabrParameterInterpolation Tests
    // =========================================================================

    #[test]
    fn test_sabr_param_interp_default() {
        let interp = SabrParameterInterpolation::default();

        // αは線形補間
        assert_eq!(interp.alpha_expiry, AxisInterpolationMethod::Linear);
        assert_eq!(interp.alpha_tenor, AxisInterpolationMethod::Linear);

        // βはFlat（通常固定）
        assert_eq!(interp.beta_expiry, AxisInterpolationMethod::Flat);
        assert_eq!(interp.beta_tenor, AxisInterpolationMethod::Flat);

        // ρは線形補間
        assert_eq!(interp.rho_expiry, AxisInterpolationMethod::Linear);
        assert_eq!(interp.rho_tenor, AxisInterpolationMethod::Linear);

        // νは線形補間
        assert_eq!(interp.nu_expiry, AxisInterpolationMethod::Linear);
        assert_eq!(interp.nu_tenor, AxisInterpolationMethod::Linear);
    }

    #[test]
    fn test_sabr_param_interp_new() {
        let interp = SabrParameterInterpolation::new();
        assert_eq!(interp, SabrParameterInterpolation::default());
    }

    #[test]
    fn test_sabr_param_interp_uniform() {
        let interp = SabrParameterInterpolation::uniform(AxisInterpolationMethod::CubicSpline);

        assert_eq!(interp.alpha_expiry, AxisInterpolationMethod::CubicSpline);
        assert_eq!(interp.alpha_tenor, AxisInterpolationMethod::CubicSpline);
        assert_eq!(interp.beta_expiry, AxisInterpolationMethod::CubicSpline);
        assert_eq!(interp.beta_tenor, AxisInterpolationMethod::CubicSpline);
        assert_eq!(interp.rho_expiry, AxisInterpolationMethod::CubicSpline);
        assert_eq!(interp.rho_tenor, AxisInterpolationMethod::CubicSpline);
        assert_eq!(interp.nu_expiry, AxisInterpolationMethod::CubicSpline);
        assert_eq!(interp.nu_tenor, AxisInterpolationMethod::CubicSpline);
    }

    #[test]
    fn test_sabr_param_interp_with_alpha() {
        let interp = SabrParameterInterpolation::default()
            .with_alpha(AxisInterpolationMethod::LogLinear, AxisInterpolationMethod::CubicSpline);

        assert_eq!(interp.alpha_expiry, AxisInterpolationMethod::LogLinear);
        assert_eq!(interp.alpha_tenor, AxisInterpolationMethod::CubicSpline);
        // 他のパラメータはデフォルト値のまま
        assert_eq!(interp.rho_expiry, AxisInterpolationMethod::Linear);
    }

    #[test]
    fn test_sabr_param_interp_with_beta() {
        let interp = SabrParameterInterpolation::default()
            .with_beta(AxisInterpolationMethod::Linear, AxisInterpolationMethod::Linear);

        assert_eq!(interp.beta_expiry, AxisInterpolationMethod::Linear);
        assert_eq!(interp.beta_tenor, AxisInterpolationMethod::Linear);
    }

    #[test]
    fn test_sabr_param_interp_with_rho() {
        let interp = SabrParameterInterpolation::default()
            .with_rho(AxisInterpolationMethod::CubicSpline, AxisInterpolationMethod::Flat);

        assert_eq!(interp.rho_expiry, AxisInterpolationMethod::CubicSpline);
        assert_eq!(interp.rho_tenor, AxisInterpolationMethod::Flat);
    }

    #[test]
    fn test_sabr_param_interp_with_nu() {
        let interp = SabrParameterInterpolation::default()
            .with_nu(AxisInterpolationMethod::LogLinear, AxisInterpolationMethod::LogLinear);

        assert_eq!(interp.nu_expiry, AxisInterpolationMethod::LogLinear);
        assert_eq!(interp.nu_tenor, AxisInterpolationMethod::LogLinear);
    }

    #[test]
    fn test_sabr_param_interp_chain() {
        let interp = SabrParameterInterpolation::default()
            .with_alpha(AxisInterpolationMethod::LogLinear, AxisInterpolationMethod::LogLinear)
            .with_beta(AxisInterpolationMethod::Flat, AxisInterpolationMethod::Flat)
            .with_rho(AxisInterpolationMethod::CubicSpline, AxisInterpolationMethod::Linear)
            .with_nu(AxisInterpolationMethod::Linear, AxisInterpolationMethod::CubicSpline);

        assert_eq!(interp.alpha_expiry, AxisInterpolationMethod::LogLinear);
        assert_eq!(interp.alpha_tenor, AxisInterpolationMethod::LogLinear);
        assert_eq!(interp.beta_expiry, AxisInterpolationMethod::Flat);
        assert_eq!(interp.beta_tenor, AxisInterpolationMethod::Flat);
        assert_eq!(interp.rho_expiry, AxisInterpolationMethod::CubicSpline);
        assert_eq!(interp.rho_tenor, AxisInterpolationMethod::Linear);
        assert_eq!(interp.nu_expiry, AxisInterpolationMethod::Linear);
        assert_eq!(interp.nu_tenor, AxisInterpolationMethod::CubicSpline);
    }

    // =========================================================================
    // VolCubeConfig SABRパラメータ補間設定 Tests
    // =========================================================================

    #[test]
    fn test_volcube_config_default_sabr_param_interpolation() {
        let config = VolCubeConfig::default();
        assert_eq!(
            config.sabr_param_interpolation,
            SabrParameterInterpolation::default()
        );
    }

    #[test]
    fn test_volcube_config_with_sabr_param_interpolation() {
        let interp = SabrParameterInterpolation::uniform(AxisInterpolationMethod::CubicSpline);
        let config = VolCubeConfig::default().with_sabr_param_interpolation(interp.clone());

        assert_eq!(config.sabr_param_interpolation, interp);
    }

    #[test]
    fn test_volcube_config_full_chain_with_sabr_param_interp() {
        let interp = SabrParameterInterpolation::default()
            .with_alpha(AxisInterpolationMethod::LogLinear, AxisInterpolationMethod::Linear);

        let config = VolCubeConfig::default()
            .with_interpolation(InterpolationMethod::Sabr)
            .with_sabr_beta(Some(0.5))
            .with_sabr_param_interpolation(interp)
            .with_curves("USD-SOFR-OIS", "USD-SOFR-3M");

        assert_eq!(config.interpolation, InterpolationMethod::Sabr);
        assert_eq!(config.sabr_beta, Some(0.5));
        assert_eq!(
            config.sabr_param_interpolation.alpha_expiry,
            AxisInterpolationMethod::LogLinear
        );
        assert_eq!(
            config.sabr_param_interpolation.alpha_tenor,
            AxisInterpolationMethod::Linear
        );
        assert_eq!(config.discount_curve, Some("USD-SOFR-OIS".to_string()));
    }
}
