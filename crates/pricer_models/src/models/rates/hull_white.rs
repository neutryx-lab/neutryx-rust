//! Hull-White one-factor interest rate model.
//!
//! The Hull-White model is a short-rate model described by:
//! ```text
//! dr(t) = [theta(t) - a * r(t)] * dt + sigma * dW(t)
//! ```
//! where:
//! - r(t) = short rate at time t
//! - a = mean reversion speed (must be positive)
//! - sigma = volatility (must be positive)
//! - theta(t) = time-dependent drift (calibrated to initial yield curve)
//! - dW(t) = Wiener process increment
//!
//! ## Key Properties
//!
//! - **Mean reversion**: Rates tend to revert to a long-term mean
//! - **Analytical tractability**: Closed-form solutions for zero-coupon bonds
//! - **Negative rates**: Model allows for negative interest rates
//!
//! ## Usage
//!
//! ```
//! use pricer_models::models::rates::hull_white::{HullWhiteModel, HullWhiteParams};
//! use pricer_models::models::stochastic::StochasticModel;
//! use pricer_core::market_data::curves::FlatCurve;
//!
//! // Create parameters with flat initial curve
//! let params = HullWhiteParams::new(0.05_f64, 0.01, FlatCurve::new(0.03)).unwrap();
//!
//! // Get initial state (short rate from curve)
//! let state = HullWhiteModel::initial_state(&params);
//! assert!((state.0 - 0.03_f64).abs() < 1e-10);
//!
//! // Evolve one step
//! let dt = 1.0_f64 / 252.0;
//! let dw = [0.0_f64];
//! let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);
//! ```

use pricer_core::market_data::curves::{FlatCurve, YieldCurve};
use pricer_core::traits::priceable::Differentiable;
use pricer_core::traits::Float;

use crate::models::stochastic::{SingleState, StochasticModel};

// ================================================================
// Task 2.1: ThetaFunction 型の実装
// ================================================================

/// theta(t) 計算方法を指定するための列挙型
///
/// Hull-White モデルの時間依存ドリフト theta(t) を
/// 異なる方法で計算するための型
///
/// # Requirement: 4.1, 4.2
///
/// # バリアント
///
/// - `Constant`: 定数 theta（フラットカーブ向け）
/// - `TimeDependentFlat`: フラットカーブからの時間依存 theta
/// - `Custom`: カスタムテーブル（補間付き）
#[derive(Clone, Debug)]
pub enum ThetaFunction<T: Float> {
    /// 定数 theta（theta = a * r_star）
    ///
    /// フラットカーブの簡易ケースで使用
    Constant {
        /// 定数 theta 値
        value: T,
    },

    /// フラットカーブからの時間依存 theta
    ///
    /// theta(t) = a * r* + (sigma^2 / 2a) * (1 - exp(-2at))
    ///
    /// # Requirement: 4.2
    TimeDependentFlat {
        /// 平均回帰速度
        mean_reversion: T,
        /// ボラティリティ
        volatility: T,
        /// 初期短期金利
        initial_short_rate: T,
    },

    /// カスタムテーブル（時間と theta のペア）
    ///
    /// 任意のイールドカーブから導出された theta を
    /// 線形補間でサポート
    ///
    /// # Requirement: 4.3
    CustomTable {
        /// 時間ポイント（昇順）
        times: Vec<T>,
        /// 対応する theta 値
        values: Vec<T>,
    },
}

impl<T: Float> ThetaFunction<T> {
    /// 定数 theta を作成
    pub fn constant(value: T) -> Self {
        ThetaFunction::Constant { value }
    }

    /// フラットカーブからの時間依存 theta を作成
    ///
    /// # Requirement: 4.2
    pub fn from_flat_curve(mean_reversion: T, volatility: T, initial_short_rate: T) -> Self {
        ThetaFunction::TimeDependentFlat {
            mean_reversion,
            volatility,
            initial_short_rate,
        }
    }

    /// カスタムテーブルから theta 関数を作成
    ///
    /// # 引数
    ///
    /// * `times` - 時間ポイント（昇順）
    /// * `values` - 対応する theta 値
    ///
    /// # Requirement: 4.3
    pub fn from_table(times: Vec<T>, values: Vec<T>) -> Option<Self> {
        if times.len() != values.len() || times.is_empty() {
            return None;
        }
        Some(ThetaFunction::CustomTable { times, values })
    }

    /// 時刻 t における theta(t) を計算
    ///
    /// # 引数
    ///
    /// * `t` - 時刻
    ///
    /// # 戻り値
    ///
    /// theta(t) の値
    pub fn evaluate(&self, t: T) -> T {
        match self {
            ThetaFunction::Constant { value } => *value,

            ThetaFunction::TimeDependentFlat {
                mean_reversion,
                volatility,
                initial_short_rate,
            } => {
                let a = *mean_reversion;
                let sigma = *volatility;
                let r_star = *initial_short_rate;

                // theta(t) = a * r* + (sigma^2 / 2a) * (1 - exp(-2at))
                let two = T::from(2.0).unwrap_or(T::one() + T::one());
                let term = T::one() - (-(two * a * t)).exp();
                a * r_star + (sigma * sigma / (two * a)) * term
            }

            ThetaFunction::CustomTable { times, values } => {
                // 線形補間
                if t <= times[0] {
                    return values[0];
                }
                if t >= times[times.len() - 1] {
                    return values[values.len() - 1];
                }

                // 補間区間を探索
                for i in 0..times.len() - 1 {
                    if t >= times[i] && t < times[i + 1] {
                        // 線形補間
                        let dt = times[i + 1] - times[i];
                        if dt <= T::zero() {
                            return values[i];
                        }
                        let w = (t - times[i]) / dt;
                        return values[i] * (T::one() - w) + values[i + 1] * w;
                    }
                }

                // フォールバック
                values[values.len() - 1]
            }
        }
    }
}

/// Hull-White model parameters.
///
/// # Type Parameters
///
/// * `T` - Float type (f64 or DualNumber for AD compatibility)
///
/// # Fields
///
/// * `mean_reversion` - Mean reversion speed (a), must be positive
/// * `volatility` - Short rate volatility (sigma), must be positive
/// * `initial_short_rate` - Initial short rate r(0) from the initial curve
/// * `initial_curve` - Initial yield curve for calibration
/// * `theta_function` - Time-dependent theta function (Requirement 4.1-4.3)
/// * `current_time` - Current simulation time for time-dependent theta
#[derive(Clone, Debug)]
pub struct HullWhiteParams<T: Float> {
    /// Mean reversion speed (a > 0)
    pub mean_reversion: T,
    /// Volatility of short rate (sigma > 0)
    pub volatility: T,
    /// Initial short rate r(0)
    pub initial_short_rate: T,
    /// Initial yield curve (for theta calibration)
    pub initial_curve: FlatCurve<T>,
    /// Time-dependent theta function (Requirement 4.1-4.3)
    pub theta_function: ThetaFunction<T>,
    /// Current simulation time for time-dependent theta
    pub current_time: T,
}

impl<T: Float> HullWhiteParams<T> {
    /// Create new Hull-White parameters with validation.
    ///
    /// # Arguments
    ///
    /// * `mean_reversion` - Mean reversion speed (must be positive)
    /// * `volatility` - Short rate volatility (must be positive)
    /// * `initial_curve` - Initial yield curve
    ///
    /// # Returns
    ///
    /// `Some(HullWhiteParams)` if parameters are valid, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_models::models::rates::hull_white::HullWhiteParams;
    /// use pricer_core::market_data::curves::FlatCurve;
    ///
    /// let params = HullWhiteParams::new(0.05, 0.01, FlatCurve::new(0.03));
    /// assert!(params.is_some());
    ///
    /// // Invalid: negative mean reversion
    /// let invalid = HullWhiteParams::new(-0.05, 0.01, FlatCurve::new(0.03));
    /// assert!(invalid.is_none());
    /// ```
    pub fn new(mean_reversion: T, volatility: T, initial_curve: FlatCurve<T>) -> Option<Self> {
        if mean_reversion <= T::zero() || volatility <= T::zero() {
            return None;
        }

        // Extract initial short rate from curve (instantaneous forward rate at t=0)
        // For a flat curve, this equals the constant rate
        let initial_short_rate = initial_curve
            .zero_rate(T::from(0.01).unwrap_or(T::one()))
            .unwrap_or(T::zero());

        // Create time-dependent theta function from flat curve
        let theta_function =
            ThetaFunction::from_flat_curve(mean_reversion, volatility, initial_short_rate);

        Some(Self {
            mean_reversion,
            volatility,
            initial_short_rate,
            initial_curve,
            theta_function,
            current_time: T::zero(),
        })
    }

    /// Create Hull-White parameters with a custom theta function.
    ///
    /// # Requirement: 4.3
    ///
    /// # Arguments
    ///
    /// * `mean_reversion` - Mean reversion speed (must be positive)
    /// * `volatility` - Short rate volatility (must be positive)
    /// * `initial_short_rate` - Initial short rate
    /// * `theta_function` - Custom theta function
    pub fn with_theta_function(
        mean_reversion: T,
        volatility: T,
        initial_short_rate: T,
        theta_function: ThetaFunction<T>,
    ) -> Option<Self> {
        if mean_reversion <= T::zero() || volatility <= T::zero() {
            return None;
        }

        Some(Self {
            mean_reversion,
            volatility,
            initial_short_rate,
            initial_curve: FlatCurve::new(initial_short_rate),
            theta_function,
            current_time: T::zero(),
        })
    }

    /// Get the long-term mean rate (approximate).
    ///
    /// For a flat initial curve with rate r*, the long-term mean
    /// is approximately r* + (sigma^2) / (2 * a^2).
    pub fn long_term_mean(&self) -> T {
        let sigma = self.volatility;
        let a = self.mean_reversion;
        let two = T::from(2.0).unwrap_or(T::one());
        self.initial_short_rate + (sigma * sigma) / (two * a * a)
    }

    /// Calculate theta(t) using the configured theta function.
    ///
    /// # Requirement: 4.1, 4.2, 4.3
    ///
    /// # Arguments
    ///
    /// * `t` - Time at which to evaluate theta
    ///
    /// # Returns
    ///
    /// theta(t) value
    pub fn theta(&self, t: T) -> T {
        self.theta_function.evaluate(t)
    }

    /// Advance the current simulation time by dt.
    ///
    /// Used internally for time-dependent theta evaluation.
    pub fn advance_time(&mut self, dt: T) {
        self.current_time = self.current_time + dt;
    }

    /// Reset the current simulation time to zero.
    pub fn reset_time(&mut self) {
        self.current_time = T::zero();
    }

    /// Get the current simulation time.
    pub fn current_time(&self) -> T {
        self.current_time
    }
}

/// Hull-White one-factor model for short rate dynamics.
///
/// Implements the StochasticModel trait for Monte Carlo simulation.
///
/// # Discretization
///
/// Uses Euler-Maruyama discretization:
/// ```text
/// r(t+dt) = r(t) + [theta(t) - a * r(t)] * dt + sigma * sqrt(dt) * dW
/// ```
#[derive(Clone, Debug, Default)]
pub struct HullWhiteModel<T: Float> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Float> HullWhiteModel<T> {
    /// Create a new Hull-White model instance.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Float> Differentiable for HullWhiteModel<T> {}

impl<T: Float + Default> StochasticModel<T> for HullWhiteModel<T> {
    type State = SingleState<T>;
    type Params = HullWhiteParams<T>;

    /// Hull-White の 1 ステップ時間発展
    ///
    /// # Requirement: 4.1, 4.2, 4.4
    ///
    /// Euler-Maruyama 離散化:
    /// ```text
    /// r(t+dt) = r(t) + [theta(t) - a * r(t)] * dt + sigma * sqrt(dt) * dW
    /// ```
    ///
    /// # Note
    ///
    /// 時間依存 theta を使用するには、呼び出し側で `params.current_time` を
    /// 更新する必要があります。または、このメソッドは `params.current_time` を
    /// 使用して theta(t) を評価します。
    fn evolve_step(state: Self::State, dt: T, dw: &[T], params: &Self::Params) -> Self::State {
        // Euler-Maruyama discretization:
        // r(t+dt) = r(t) + [theta(t) - a * r(t)] * dt + sigma * sqrt(dt) * dW
        let r = state.0;
        let a = params.mean_reversion;
        let sigma = params.volatility;

        // Use time-dependent theta evaluated at current simulation time
        // Requirement 4.2: Time-dependent theta for yield curve fitting
        let theta = params.theta(params.current_time);

        // Drift: [theta - a * r] * dt
        let drift = (theta - a * r) * dt;

        // Diffusion: sigma * sqrt(dt) * dW
        let diffusion = sigma * dt.sqrt() * dw[0];

        SingleState(r + drift + diffusion)
    }

    fn initial_state(params: &Self::Params) -> Self::State {
        SingleState(params.initial_short_rate)
    }

    fn brownian_dim() -> usize {
        1
    }

    fn model_name() -> &'static str {
        "HullWhite1F"
    }

    fn num_factors() -> usize {
        1 // Hull-White 1F is a single-factor model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 9.2: Hull-White 1F Model Tests (TDD)
    // ================================================================

    // Test: Parameter validation
    #[test]
    fn test_hull_white_params_new_valid() {
        let params = HullWhiteParams::new(0.05_f64, 0.01, FlatCurve::new(0.03));
        assert!(params.is_some());
        let p = params.unwrap();
        assert_eq!(p.mean_reversion, 0.05);
        assert_eq!(p.volatility, 0.01);
        assert!((p.initial_short_rate - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_hull_white_params_new_invalid_mean_reversion() {
        // Negative mean reversion
        let params = HullWhiteParams::new(-0.05_f64, 0.01, FlatCurve::new(0.03));
        assert!(params.is_none());

        // Zero mean reversion
        let params = HullWhiteParams::new(0.0_f64, 0.01, FlatCurve::new(0.03));
        assert!(params.is_none());
    }

    #[test]
    fn test_hull_white_params_new_invalid_volatility() {
        // Negative volatility
        let params = HullWhiteParams::new(0.05_f64, -0.01, FlatCurve::new(0.03));
        assert!(params.is_none());

        // Zero volatility
        let params = HullWhiteParams::new(0.05_f64, 0.0, FlatCurve::new(0.03));
        assert!(params.is_none());
    }

    #[test]
    fn test_hull_white_params_long_term_mean() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let ltm = params.long_term_mean();
        // Expected: 0.03 + (0.01^2) / (2 * 0.1^2) = 0.03 + 0.0001 / 0.02 = 0.03 + 0.005 = 0.035
        assert!((ltm - 0.035).abs() < 1e-10);
    }

    #[test]
    fn test_hull_white_params_theta() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        // theta at t=0: a * r* = 0.1 * 0.03 = 0.003
        let theta_0 = params.theta(0.0);
        assert!((theta_0 - 0.003).abs() < 1e-10);
    }

    // Test: Model properties
    #[test]
    fn test_hull_white_model_new() {
        let model: HullWhiteModel<f64> = HullWhiteModel::new();
        assert_eq!(HullWhiteModel::<f64>::model_name(), "HullWhite1F");
        assert_eq!(HullWhiteModel::<f64>::brownian_dim(), 1);
        assert_eq!(HullWhiteModel::<f64>::num_factors(), 1);
        let _ = model; // Suppress unused warning
    }

    #[test]
    fn test_hull_white_initial_state() {
        let params = HullWhiteParams::new(0.05_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        assert!((state.0 - 0.03).abs() < 1e-10);
    }

    // Test: Evolution step
    #[test]
    fn test_hull_white_evolve_step_no_shock() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        let dt = 1.0 / 252.0; // Daily step
        let dw = [0.0];

        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);

        // With zero shock, should move towards long-term mean
        // drift = [theta - a * r] * dt = [0.003 - 0.1 * 0.03] * dt = 0 * dt = 0
        // For flat curve at initial state, drift is approximately zero
        assert!((next_state.0 - state.0).abs() < 1e-8);
    }

    #[test]
    fn test_hull_white_evolve_step_positive_shock() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [1.0]; // Positive shock

        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);

        // With positive shock, rate should increase
        assert!(next_state.0 > state.0);
    }

    #[test]
    fn test_hull_white_evolve_step_negative_shock() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let dw = [-1.0]; // Negative shock

        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);

        // With negative shock, rate should decrease
        assert!(next_state.0 < state.0);
    }

    #[test]
    fn test_hull_white_mean_reversion() {
        // Start above long-term mean, should trend down
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.02)).unwrap();
        let state = SingleState(0.08_f64); // Well above 0.02
        let dt = 0.1; // Larger time step
        let dw = [0.0];

        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);

        // Should decrease towards long-term mean
        assert!(next_state.0 < state.0);
    }

    #[test]
    fn test_hull_white_allows_negative_rates() {
        // Start near zero, with large negative shock
        let params = HullWhiteParams::new(0.1_f64, 0.05, FlatCurve::new(0.001)).unwrap();
        let state = SingleState(0.001_f64);
        let dt = 1.0 / 12.0; // Monthly step for larger diffusion effect
        let dw = [-3.0]; // Large negative shock

        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);

        // Rate can go negative in Hull-White model
        // Diffusion: 0.05 * sqrt(1/12) * (-3) = -0.0433
        // This should push the rate negative
        assert!(next_state.0 < 0.0);
    }

    // Test: Multiple steps path generation
    #[test]
    fn test_hull_white_path_generation() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let mut state = HullWhiteModel::initial_state(&params);
        let dt = 1.0 / 252.0;
        let n_steps = 252; // One year

        // Generate path with zero shocks
        for _ in 0..n_steps {
            state = HullWhiteModel::evolve_step(state, dt, &[0.0], &params);
        }

        // After one year with zero shocks, should be close to initial
        // (since we started at the long-term mean for flat curve)
        assert!(state.0.is_finite());
    }

    // Test: Differentiable marker
    #[test]
    fn test_hull_white_is_differentiable() {
        let model: HullWhiteModel<f64> = HullWhiteModel::new();
        let _: &dyn Differentiable = &model;
    }

    // Test: f32 compatibility
    #[test]
    fn test_hull_white_f32_compatibility() {
        let params = HullWhiteParams::new(0.1_f32, 0.01, FlatCurve::new(0.03)).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        assert!((state.0 - 0.03_f32).abs() < 1e-6);

        let dt = 1.0_f32 / 252.0;
        let dw = [0.0_f32];
        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);
        assert!(next_state.0.is_finite());
    }

    // Test: Clone
    #[test]
    fn test_hull_white_params_clone() {
        let params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let cloned = params.clone();
        assert_eq!(params.mean_reversion, cloned.mean_reversion);
        assert_eq!(params.volatility, cloned.volatility);
    }

    // ================================================================
    // Task 6.2: ThetaFunction Tests
    // ================================================================

    #[test]
    fn test_theta_function_constant() {
        let theta = ThetaFunction::constant(0.05_f64);
        assert!((theta.evaluate(0.0) - 0.05).abs() < 1e-10);
        assert!((theta.evaluate(1.0) - 0.05).abs() < 1e-10);
        assert!((theta.evaluate(10.0) - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_theta_function_time_dependent_flat() {
        let theta = ThetaFunction::from_flat_curve(0.1_f64, 0.01, 0.03);

        // At t=0: theta(0) = a * r* + 0 = 0.1 * 0.03 = 0.003
        let theta_0 = theta.evaluate(0.0);
        assert!((theta_0 - 0.003).abs() < 1e-10);

        // At t>0: theta(t) = a * r* + (sigma^2 / 2a) * (1 - exp(-2at))
        // At t=1: theta(1) = 0.003 + (0.0001 / 0.2) * (1 - exp(-0.2))
        //       = 0.003 + 0.0005 * (1 - 0.8187...) = 0.003 + 0.0005 * 0.1813
        //       = 0.003 + 0.0000906... ≈ 0.0030906
        let theta_1 = theta.evaluate(1.0);
        assert!(theta_1 > theta_0); // Should increase with time

        // At large t: theta approaches a * r* + sigma^2 / 2a
        // = 0.003 + 0.0001 / 0.2 = 0.003 + 0.0005 = 0.0035
        let theta_large = theta.evaluate(100.0);
        assert!((theta_large - 0.0035).abs() < 1e-6);
    }

    #[test]
    fn test_theta_function_custom_table_basic() {
        let times = vec![0.0_f64, 1.0, 2.0, 3.0];
        let values = vec![0.01_f64, 0.02, 0.03, 0.04];
        let theta = ThetaFunction::from_table(times, values).unwrap();

        // At exact time points
        assert!((theta.evaluate(0.0) - 0.01).abs() < 1e-10);
        assert!((theta.evaluate(1.0) - 0.02).abs() < 1e-10);
        assert!((theta.evaluate(2.0) - 0.03).abs() < 1e-10);
        assert!((theta.evaluate(3.0) - 0.04).abs() < 1e-10);
    }

    #[test]
    fn test_theta_function_custom_table_interpolation() {
        let times = vec![0.0_f64, 1.0, 2.0];
        let values = vec![0.01_f64, 0.03, 0.05];
        let theta = ThetaFunction::from_table(times, values).unwrap();

        // Interpolate at t=0.5: (0.01 + 0.03) / 2 = 0.02
        let theta_05 = theta.evaluate(0.5);
        assert!((theta_05 - 0.02).abs() < 1e-10);

        // Interpolate at t=1.5: (0.03 + 0.05) / 2 = 0.04
        let theta_15 = theta.evaluate(1.5);
        assert!((theta_15 - 0.04).abs() < 1e-10);

        // Interpolate at t=0.25: 0.01 + 0.25 * (0.03 - 0.01) = 0.01 + 0.005 = 0.015
        let theta_025 = theta.evaluate(0.25);
        assert!((theta_025 - 0.015).abs() < 1e-10);
    }

    #[test]
    fn test_theta_function_custom_table_extrapolation() {
        let times = vec![1.0_f64, 2.0, 3.0];
        let values = vec![0.02_f64, 0.03, 0.04];
        let theta = ThetaFunction::from_table(times, values).unwrap();

        // Before first time point: use first value
        assert!((theta.evaluate(0.0) - 0.02).abs() < 1e-10);
        assert!((theta.evaluate(0.5) - 0.02).abs() < 1e-10);

        // After last time point: use last value
        assert!((theta.evaluate(3.5) - 0.04).abs() < 1e-10);
        assert!((theta.evaluate(10.0) - 0.04).abs() < 1e-10);
    }

    #[test]
    fn test_theta_function_from_table_invalid() {
        // Empty vectors
        let result = ThetaFunction::<f64>::from_table(vec![], vec![]);
        assert!(result.is_none());

        // Mismatched lengths
        let result = ThetaFunction::from_table(vec![0.0_f64, 1.0], vec![0.01_f64]);
        assert!(result.is_none());
    }

    #[test]
    fn test_hull_white_params_with_theta_function_constant() {
        let theta = ThetaFunction::constant(0.005_f64);
        let params =
            HullWhiteParams::with_theta_function(0.1, 0.01, 0.03, theta).unwrap();

        assert_eq!(params.mean_reversion, 0.1);
        assert_eq!(params.volatility, 0.01);
        assert_eq!(params.initial_short_rate, 0.03);
        assert!((params.theta(0.0) - 0.005).abs() < 1e-10);
        assert!((params.theta(1.0) - 0.005).abs() < 1e-10);
    }

    #[test]
    fn test_hull_white_params_with_theta_function_custom() {
        let times = vec![0.0_f64, 1.0, 2.0];
        let values = vec![0.003_f64, 0.004, 0.005];
        let theta = ThetaFunction::from_table(times, values).unwrap();
        let params =
            HullWhiteParams::with_theta_function(0.1, 0.01, 0.03, theta).unwrap();

        assert!((params.theta(0.5) - 0.0035).abs() < 1e-10);
    }

    #[test]
    fn test_hull_white_params_with_theta_function_invalid() {
        let theta = ThetaFunction::constant(0.005_f64);

        // Invalid mean reversion
        let result = HullWhiteParams::with_theta_function(-0.1_f64, 0.01, 0.03, theta.clone());
        assert!(result.is_none());

        // Invalid volatility
        let result = HullWhiteParams::with_theta_function(0.1_f64, -0.01, 0.03, theta);
        assert!(result.is_none());
    }

    #[test]
    fn test_hull_white_params_advance_time() {
        let mut params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        assert!((params.current_time() - 0.0).abs() < 1e-10);

        params.advance_time(0.1);
        assert!((params.current_time() - 0.1).abs() < 1e-10);

        params.advance_time(0.2);
        assert!((params.current_time() - 0.3).abs() < 1e-10);

        params.reset_time();
        assert!((params.current_time() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_hull_white_evolve_with_time_dependent_theta() {
        // Create params with time-dependent theta
        let mut params = HullWhiteParams::new(0.1_f64, 0.01, FlatCurve::new(0.03)).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        let dt = 0.1;
        let dw = [0.0];

        // Evolve at t=0
        let state_1 = HullWhiteModel::evolve_step(state, dt, &dw, &params);

        // Advance time and evolve at t=0.1
        params.advance_time(dt);
        let state_2 = HullWhiteModel::evolve_step(state_1, dt, &dw, &params);

        // Both states should be finite
        assert!(state_1.0.is_finite());
        assert!(state_2.0.is_finite());
    }

    #[test]
    fn test_hull_white_custom_theta_in_evolve_step() {
        // Use constant theta that differs from what the curve would give
        let theta = ThetaFunction::constant(0.01_f64); // Much higher than a * r* = 0.003
        let params =
            HullWhiteParams::with_theta_function(0.1, 0.01, 0.03, theta).unwrap();
        let state = HullWhiteModel::initial_state(&params);
        let dt = 1.0;
        let dw = [0.0];

        // With theta = 0.01, drift = [0.01 - 0.1 * 0.03] * 1 = [0.01 - 0.003] * 1 = 0.007
        // Expected: 0.03 + 0.007 = 0.037
        let next_state = HullWhiteModel::evolve_step(state, dt, &dw, &params);
        assert!((next_state.0 - 0.037).abs() < 1e-10);
    }

    #[test]
    fn test_theta_function_clone() {
        let theta = ThetaFunction::from_flat_curve(0.1_f64, 0.01, 0.03);
        let cloned = theta.clone();
        assert!((theta.evaluate(1.0) - cloned.evaluate(1.0)).abs() < 1e-10);

        let times = vec![0.0_f64, 1.0];
        let values = vec![0.01_f64, 0.02];
        let theta_table = ThetaFunction::from_table(times, values).unwrap();
        let cloned_table = theta_table.clone();
        assert!((theta_table.evaluate(0.5) - cloned_table.evaluate(0.5)).abs() < 1e-10);
    }

    #[test]
    fn test_theta_function_debug() {
        let theta = ThetaFunction::constant(0.05_f64);
        let debug_str = format!("{:?}", theta);
        assert!(debug_str.contains("Constant"));

        let theta_flat = ThetaFunction::from_flat_curve(0.1_f64, 0.01, 0.03);
        let debug_str = format!("{:?}", theta_flat);
        assert!(debug_str.contains("TimeDependentFlat"));
    }
}
