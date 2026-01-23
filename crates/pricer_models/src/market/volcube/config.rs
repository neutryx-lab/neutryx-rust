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
        }
    }
}

impl VolCubeConfig {
    /// 新しい設定を作成（デフォルト値）。
    pub fn new() -> Self {
        Self::default()
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
    pub fn with_sabr_beta(mut self, beta: Option<f64>) -> Self {
        self.sabr_beta = beta;
        self
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

    /// 設定を検証。
    pub fn validate(&self) -> Result<(), String> {
        if let Some(beta) = self.sabr_beta {
            if !(0.0..=1.0).contains(&beta) {
                return Err(format!(
                    "SABR beta must be in [0, 1], got: {}",
                    beta
                ));
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
        let variants = [
            OptimizerType::LevenbergMarquardt,
            OptimizerType::NelderMead,
        ];
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
        let config = VolCubeConfig::default()
            .with_interpolation(InterpolationMethod::Svi);
        assert_eq!(config.interpolation, InterpolationMethod::Svi);
    }

    #[test]
    fn test_volcube_config_builder_extrapolation() {
        let config = VolCubeConfig::default()
            .with_extrapolation(ExtrapolationMethod::Error);
        assert_eq!(config.extrapolation, ExtrapolationMethod::Error);
    }

    #[test]
    fn test_volcube_config_builder_strike_axis() {
        let config = VolCubeConfig::default()
            .with_strike_axis(StrikeAxisType::Delta);
        assert_eq!(config.strike_axis, StrikeAxisType::Delta);
    }

    #[test]
    fn test_volcube_config_builder_optimizer() {
        let config = VolCubeConfig::default()
            .with_optimizer(OptimizerType::NelderMead);
        assert_eq!(config.optimizer, OptimizerType::NelderMead);
    }

    #[test]
    fn test_volcube_config_builder_validate_arbitrage_free() {
        let config = VolCubeConfig::default()
            .with_validate_arbitrage_free(true);
        assert!(config.validate_arbitrage_free);
    }

    #[test]
    fn test_volcube_config_builder_sabr_beta() {
        let config = VolCubeConfig::default()
            .with_sabr_beta(Some(0.25));
        assert_eq!(config.sabr_beta, Some(0.25));

        let config_none = VolCubeConfig::default()
            .with_sabr_beta(None);
        assert_eq!(config_none.sabr_beta, None);
    }

    #[test]
    fn test_volcube_config_builder_sabr_shift() {
        let config = VolCubeConfig::default()
            .with_sabr_shift(0.03);
        assert!((config.sabr_shift - 0.03).abs() < 1e-15);
    }

    #[test]
    fn test_volcube_config_builder_max_iterations() {
        let config = VolCubeConfig::default()
            .with_max_iterations(500);
        assert_eq!(config.max_iterations, 500);
    }

    #[test]
    fn test_volcube_config_builder_tolerance() {
        let config = VolCubeConfig::default()
            .with_tolerance(1e-10);
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
        let config = VolCubeConfig::default()
            .with_sabr_beta(Some(1.5));
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("beta"));
    }

    #[test]
    fn test_volcube_config_validate_beta_out_of_range_low() {
        let config = VolCubeConfig::default()
            .with_sabr_beta(Some(-0.1));
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("beta"));
    }

    #[test]
    fn test_volcube_config_validate_beta_none() {
        let config = VolCubeConfig::default()
            .with_sabr_beta(None);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_volcube_config_validate_tolerance_zero() {
        let config = VolCubeConfig::default()
            .with_tolerance(0.0);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Tolerance"));
    }

    #[test]
    fn test_volcube_config_validate_tolerance_negative() {
        let config = VolCubeConfig::default()
            .with_tolerance(-1e-8);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Tolerance"));
    }

    #[test]
    fn test_volcube_config_validate_max_iterations_zero() {
        let config = VolCubeConfig::default()
            .with_max_iterations(0);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("iterations"));
    }

    #[test]
    fn test_volcube_config_clone() {
        let config = VolCubeConfig::default()
            .with_interpolation(InterpolationMethod::Svi);
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
}
