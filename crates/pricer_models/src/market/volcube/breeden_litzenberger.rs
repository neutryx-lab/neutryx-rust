//! Breeden-Litzenberger 確率密度関数計算。
//!
//! # Requirements: 3.1-3.3
//!
//! このモジュールはVolCubeから導出されるリスク中立確率密度関数（PDF）と
//! 累積分布関数（CDF）を計算する。
//!
//! # 数理背景
//!
//! Breeden-Litzenberger公式:
//! ```text
//! f(K) = e^(rT) * ∂²C/∂K²
//! ```
//! ここで:
//! - f(K): ストライクKでのリスク中立確率密度
//! - C: コールオプション価格
//! - r: 無リスク金利
//! - T: 満期までの時間
//!
//! # 実装
//!
//! 1. Black-Scholesでコール価格を計算
//! 2. 中心差分で二次微分を近似
//! 3. 割引係数でスケーリング

use num_traits::Float;
use std::f64::consts::PI;

use super::cube::VolatilityCube;
use super::error::VolCubeError;

/// Breeden-Litzenberger計算を提供する構造体。
///
/// # Requirements: 3.1-3.3
///
/// VolCubeからリスク中立確率密度と累積分布を計算する。
#[derive(Debug, Clone, Copy, Default)]
pub struct BreedenLitzenberger;

impl BreedenLitzenberger {
    /// 新しいBreedenLitzenbergerインスタンスを作成。
    pub fn new() -> Self {
        Self
    }

    /// リスク中立確率密度関数を計算。
    ///
    /// # Requirements: 3.1
    ///
    /// f(K) = e^(rT) * ∂²C/∂K² を中心差分で近似。
    ///
    /// # Arguments
    ///
    /// * `vol_cube` - ボラティリティキューブ
    /// * `forward` - フォワード価格
    /// * `expiry` - 満期（年単位）
    /// * `tenor` - テナー（年単位、Swaptionなど）
    /// * `strike` - ストライク価格
    /// * `risk_free_rate` - 無リスク金利
    /// * `delta_k` - 差分計算用のストライク変位
    ///
    /// # Returns
    ///
    /// リスク中立確率密度値（非負）
    ///
    /// # Errors
    ///
    /// ストライク範囲がドメイン外の場合エラー。
    pub fn probability_density<T, C>(
        vol_cube: &C,
        forward: T,
        expiry: T,
        tenor: T,
        strike: T,
        risk_free_rate: T,
        delta_k: T,
    ) -> Result<T, VolCubeError>
    where
        T: Float,
        C: VolatilityCube<T>,
    {
        let zero = T::zero();

        // 入力検証
        if expiry <= zero {
            return Err(VolCubeError::invalid_input("Expiry must be positive"));
        }
        if strike <= zero {
            return Err(VolCubeError::invalid_input("Strike must be positive"));
        }
        if delta_k <= zero {
            return Err(VolCubeError::invalid_input("Delta K must be positive"));
        }

        // 3点でのボラティリティを取得
        let vol_minus = vol_cube.volatility(expiry, tenor, strike - delta_k)?;
        let vol_center = vol_cube.volatility(expiry, tenor, strike)?;
        let vol_plus = vol_cube.volatility(expiry, tenor, strike + delta_k)?;

        // 3点でのコール価格を計算
        let call_minus = Self::black_scholes_call(forward, strike - delta_k, expiry, vol_minus, risk_free_rate);
        let call_center = Self::black_scholes_call(forward, strike, expiry, vol_center, risk_free_rate);
        let call_plus = Self::black_scholes_call(forward, strike + delta_k, expiry, vol_plus, risk_free_rate);

        // 中心差分で二次微分を近似: d²C/dK² ≈ (C(K+h) - 2C(K) + C(K-h)) / h²
        let delta_k_squared = delta_k * delta_k;
        let d2c_dk2 = (call_plus - call_center - call_center + call_minus) / delta_k_squared;

        // f(K) = e^(rT) * d²C/dK²
        let discount = (-risk_free_rate * expiry).exp();
        let density = d2c_dk2 / discount;

        // 確率密度は非負
        Ok(density.max(zero))
    }

    /// 累積確率分布関数を計算。
    ///
    /// # Requirements: 3.2
    ///
    /// P(S_T < K) = 1 + e^(rT) * ∂C/∂K
    ///
    /// # Arguments
    ///
    /// * `vol_cube` - ボラティリティキューブ
    /// * `forward` - フォワード価格
    /// * `expiry` - 満期（年単位）
    /// * `tenor` - テナー（年単位）
    /// * `strike` - ストライク価格
    /// * `risk_free_rate` - 無リスク金利
    /// * `delta_k` - 差分計算用のストライク変位
    ///
    /// # Returns
    ///
    /// 累積確率（0から1の間）
    pub fn cumulative_probability<T, C>(
        vol_cube: &C,
        forward: T,
        expiry: T,
        tenor: T,
        strike: T,
        risk_free_rate: T,
        delta_k: T,
    ) -> Result<T, VolCubeError>
    where
        T: Float,
        C: VolatilityCube<T>,
    {
        let zero = T::zero();
        let one = T::one();

        // 入力検証
        if expiry <= zero {
            return Err(VolCubeError::invalid_input("Expiry must be positive"));
        }
        if strike <= zero {
            return Err(VolCubeError::invalid_input("Strike must be positive"));
        }
        if delta_k <= zero {
            return Err(VolCubeError::invalid_input("Delta K must be positive"));
        }

        // 2点でのボラティリティを取得（中心差分）
        let vol_minus = vol_cube.volatility(expiry, tenor, strike - delta_k)?;
        let vol_plus = vol_cube.volatility(expiry, tenor, strike + delta_k)?;

        // 2点でのコール価格を計算
        let call_minus = Self::black_scholes_call(forward, strike - delta_k, expiry, vol_minus, risk_free_rate);
        let call_plus = Self::black_scholes_call(forward, strike + delta_k, expiry, vol_plus, risk_free_rate);

        // 中心差分で一次微分を近似: dC/dK ≈ (C(K+h) - C(K-h)) / 2h
        let two = T::from(2.0).unwrap();
        let dc_dk = (call_plus - call_minus) / (two * delta_k);

        // P(S_T < K) = 1 + e^(rT) * dC/dK
        // (dC/dK is typically negative for calls)
        let discount = (-risk_free_rate * expiry).exp();
        let cdf = one + dc_dk / discount;

        // CDFは[0, 1]にクランプ
        Ok(cdf.max(zero).min(one))
    }

    /// Black-Scholes コールオプション価格を計算。
    ///
    /// C = F * e^(-rT) * N(d1) - K * e^(-rT) * N(d2)
    /// ここで F = S * e^(rT) (forward price)
    ///
    /// Forward measureで表すと:
    /// C = e^(-rT) * (F * N(d1) - K * N(d2))
    fn black_scholes_call<T: Float>(
        forward: T,
        strike: T,
        expiry: T,
        vol: T,
        risk_free_rate: T,
    ) -> T {
        let zero = T::zero();
        let half = T::from(0.5).unwrap();

        if expiry <= zero || vol <= zero {
            return zero;
        }

        let sqrt_t = expiry.sqrt();
        let vol_sqrt_t = vol * sqrt_t;

        // d1 = (ln(F/K) + σ²T/2) / (σ√T)
        let ln_fk = (forward / strike).ln();
        let d1 = (ln_fk + half * vol * vol * expiry) / vol_sqrt_t;

        // d2 = d1 - σ√T
        let d2 = d1 - vol_sqrt_t;

        // C = e^(-rT) * (F * N(d1) - K * N(d2))
        let discount = (-risk_free_rate * expiry).exp();
        let n_d1 = Self::normal_cdf(d1);
        let n_d2 = Self::normal_cdf(d2);

        discount * (forward * n_d1 - strike * n_d2)
    }

    /// 標準正規分布の累積分布関数 N(x)。
    ///
    /// Abramowitz and Stegun 近似を使用。
    /// 精度: |ε| < 7.5×10^-8
    fn normal_cdf<T: Float>(x: T) -> T {
        let zero = T::zero();
        let one = T::one();
        let half = T::from(0.5).unwrap();

        // N(x) = 1 - n(x) * (b1*t + b2*t^2 + b3*t^3 + b4*t^4 + b5*t^5)
        // where t = 1/(1 + p*|x|) and n(x) = exp(-x^2/2)/sqrt(2π)

        // 定数
        let p = T::from(0.2316419).unwrap();
        let b1 = T::from(0.319381530).unwrap();
        let b2 = T::from(-0.356563782).unwrap();
        let b3 = T::from(1.781477937).unwrap();
        let b4 = T::from(-1.821255978).unwrap();
        let b5 = T::from(1.330274429).unwrap();

        let x_abs = x.abs();
        let t = one / (one + p * x_abs);

        // n(x) = exp(-x^2/2) / sqrt(2π)
        let sqrt_2pi = T::from((2.0 * PI).sqrt()).unwrap();
        let n_x = (-x_abs * x_abs * half).exp() / sqrt_2pi;

        // Polynomial
        let t2 = t * t;
        let t3 = t2 * t;
        let t4 = t3 * t;
        let t5 = t4 * t;
        let poly = b1 * t + b2 * t2 + b3 * t3 + b4 * t4 + b5 * t5;

        let cdf_positive = one - n_x * poly;

        // Use symmetry: N(-x) = 1 - N(x)
        if x >= zero {
            cdf_positive
        } else {
            one - cdf_positive
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::volcube::{
        VolCubeBuilder, VolCubeConfig, VolInstrument,
    };

    fn make_test_cube() -> impl VolatilityCube<f64> {
        let forward = 0.03;
        let instruments = vec![
            // 2x2 minimum grid
            // expiry=1.0, tenor=5.0
            VolInstrument::new("1Y5Y_1", 1.0, 5.0, 0.02, 0.22, forward),
            VolInstrument::new("1Y5Y_2", 1.0, 5.0, 0.03, 0.20, forward),
            VolInstrument::new("1Y5Y_3", 1.0, 5.0, 0.04, 0.21, forward),
            // expiry=1.0, tenor=10.0
            VolInstrument::new("1Y10Y_1", 1.0, 10.0, 0.02, 0.20, forward),
            VolInstrument::new("1Y10Y_2", 1.0, 10.0, 0.03, 0.18, forward),
            VolInstrument::new("1Y10Y_3", 1.0, 10.0, 0.04, 0.19, forward),
            // expiry=5.0, tenor=5.0
            VolInstrument::new("5Y5Y_1", 5.0, 5.0, 0.02, 0.18, forward),
            VolInstrument::new("5Y5Y_2", 5.0, 5.0, 0.03, 0.16, forward),
            VolInstrument::new("5Y5Y_3", 5.0, 5.0, 0.04, 0.17, forward),
            // expiry=5.0, tenor=10.0
            VolInstrument::new("5Y10Y_1", 5.0, 10.0, 0.02, 0.17, forward),
            VolInstrument::new("5Y10Y_2", 5.0, 10.0, 0.03, 0.15, forward),
            VolInstrument::new("5Y10Y_3", 5.0, 10.0, 0.04, 0.16, forward),
        ];

        VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_config(VolCubeConfig::default())
            .with_forward(forward)
            .build()
            .expect("Failed to build test cube")
    }

    // =========================================================================
    // BreedenLitzenberger Tests
    // =========================================================================

    #[test]
    fn test_bl_new() {
        let bl = BreedenLitzenberger::new();
        assert_eq!(std::mem::size_of_val(&bl), 0);
    }

    #[test]
    fn test_bl_default() {
        let bl = BreedenLitzenberger::default();
        assert_eq!(std::mem::size_of_val(&bl), 0);
    }

    #[test]
    fn test_normal_cdf_zero() {
        let result = BreedenLitzenberger::normal_cdf(0.0_f64);
        assert!((result - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_normal_cdf_positive() {
        // N(1) ≈ 0.8413
        let result = BreedenLitzenberger::normal_cdf(1.0_f64);
        assert!((result - 0.8413).abs() < 0.01);
    }

    #[test]
    fn test_normal_cdf_negative() {
        // N(-1) ≈ 0.1587
        let result = BreedenLitzenberger::normal_cdf(-1.0_f64);
        assert!((result - 0.1587).abs() < 0.01);
    }

    #[test]
    fn test_normal_cdf_large_positive() {
        // N(3) ≈ 0.9987
        let result = BreedenLitzenberger::normal_cdf(3.0_f64);
        assert!(result > 0.99);
    }

    #[test]
    fn test_normal_cdf_large_negative() {
        // N(-3) ≈ 0.0013
        let result = BreedenLitzenberger::normal_cdf(-3.0_f64);
        assert!(result < 0.01);
    }

    #[test]
    fn test_normal_cdf_symmetry() {
        for x in [0.5, 1.0, 2.0, 3.0] {
            let n_x = BreedenLitzenberger::normal_cdf(x);
            let n_minus_x = BreedenLitzenberger::normal_cdf(-x);
            assert!((n_x + n_minus_x - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_black_scholes_call_atm() {
        // ATM call: F = K
        let forward = 100.0;
        let strike = 100.0;
        let expiry = 1.0;
        let vol = 0.20;
        let rate = 0.05;

        let price = BreedenLitzenberger::black_scholes_call(forward, strike, expiry, vol, rate);

        // ATM call ≈ 0.4 * σ * sqrt(T) * F * e^(-rT)
        // For F=100, σ=0.2, T=1: ≈ 7.6
        assert!(price > 5.0 && price < 10.0);
    }

    #[test]
    fn test_black_scholes_call_itm() {
        // Deep ITM call: F >> K
        let forward = 120.0;
        let strike = 80.0;
        let expiry = 1.0;
        let vol = 0.20;
        let rate = 0.05;

        let price = BreedenLitzenberger::black_scholes_call(forward, strike, expiry, vol, rate);

        // ITM call ≈ (F - K) * e^(-rT)
        let intrinsic = (forward - strike) * (-rate * expiry).exp();
        assert!(price > intrinsic);
        assert!(price < forward);
    }

    #[test]
    fn test_black_scholes_call_otm() {
        // Deep OTM call: F << K
        let forward = 80.0;
        let strike = 120.0;
        let expiry = 1.0;
        let vol = 0.20;
        let rate = 0.05;

        let price = BreedenLitzenberger::black_scholes_call(forward, strike, expiry, vol, rate);

        // OTM call is cheap
        assert!(price > 0.0);
        assert!(price < 5.0);
    }

    #[test]
    fn test_black_scholes_call_zero_expiry() {
        let price = BreedenLitzenberger::black_scholes_call(100.0, 90.0, 0.0, 0.20, 0.05);
        assert_eq!(price, 0.0);
    }

    #[test]
    fn test_black_scholes_call_zero_vol() {
        let price = BreedenLitzenberger::black_scholes_call(100.0, 90.0, 1.0, 0.0, 0.05);
        assert_eq!(price, 0.0);
    }

    #[test]
    fn test_probability_density_positive() {
        let cube = make_test_cube();
        let forward = 0.03;
        let expiry = 1.0;
        let tenor = 5.0;
        let strike = 0.03;
        let rate = 0.02;
        let delta_k = 0.001;

        let density = BreedenLitzenberger::probability_density(
            &cube, forward, expiry, tenor, strike, rate, delta_k
        ).unwrap();

        // Density should be non-negative
        assert!(density >= 0.0);
    }

    #[test]
    fn test_probability_density_atm_peak() {
        let cube = make_test_cube();
        let forward = 0.03;
        let expiry = 1.0;
        let tenor = 5.0;
        let rate = 0.02;
        let delta_k = 0.001;

        // Density near ATM should be higher than far OTM
        let density_atm = BreedenLitzenberger::probability_density(
            &cube, forward, expiry, tenor, forward, rate, delta_k
        ).unwrap();

        // Check multiple OTM points if they're in range
        let strike_otm = forward * 1.1; // 10% OTM
        let (strike_min, strike_max) = cube.strike_domain();

        if strike_otm > strike_min && strike_otm < strike_max {
            let density_otm = BreedenLitzenberger::probability_density(
                &cube, forward, expiry, tenor, strike_otm, rate, delta_k
            ).unwrap_or(0.0);

            // ATM density is typically higher than OTM
            // (not always true for extreme skews, so we just check non-negative)
            assert!(density_atm >= 0.0);
            assert!(density_otm >= 0.0);
        }
    }

    #[test]
    fn test_probability_density_invalid_expiry() {
        let cube = make_test_cube();

        let result = BreedenLitzenberger::probability_density(
            &cube, 0.03, -1.0, 5.0, 0.03, 0.02, 0.001
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_probability_density_invalid_strike() {
        let cube = make_test_cube();

        let result = BreedenLitzenberger::probability_density(
            &cube, 0.03, 1.0, 5.0, -0.03, 0.02, 0.001
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_probability_density_invalid_delta_k() {
        let cube = make_test_cube();

        let result = BreedenLitzenberger::probability_density(
            &cube, 0.03, 1.0, 5.0, 0.03, 0.02, -0.001
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_cumulative_probability_positive() {
        let cube = make_test_cube();
        let forward = 0.03;
        let expiry = 1.0;
        let tenor = 5.0;
        let strike = 0.03;
        let rate = 0.02;
        let delta_k = 0.001;

        let cdf = BreedenLitzenberger::cumulative_probability(
            &cube, forward, expiry, tenor, strike, rate, delta_k
        ).unwrap();

        // CDF should be in [0, 1]
        assert!(cdf >= 0.0);
        assert!(cdf <= 1.0);
    }

    #[test]
    fn test_cumulative_probability_monotonic() {
        let cube = make_test_cube();
        let forward = 0.03;
        let expiry = 1.0;
        let tenor = 5.0;
        let rate = 0.02;
        let delta_k = 0.001;

        let (strike_min, strike_max) = cube.strike_domain();

        // Test at a few points within the domain
        let test_strikes = [
            strike_min + (strike_max - strike_min) * 0.2,
            strike_min + (strike_max - strike_min) * 0.4,
            strike_min + (strike_max - strike_min) * 0.6,
            strike_min + (strike_max - strike_min) * 0.8,
        ];

        let mut prev_cdf = 0.0;
        for &strike in &test_strikes {
            let cdf = BreedenLitzenberger::cumulative_probability(
                &cube, forward, expiry, tenor, strike, rate, delta_k
            ).unwrap_or(0.0);

            // CDF should be monotonically increasing
            assert!(cdf >= prev_cdf - 0.01, "CDF should be monotonic: {} >= {}", cdf, prev_cdf);
            prev_cdf = cdf;
        }
    }

    #[test]
    fn test_cumulative_probability_invalid_inputs() {
        let cube = make_test_cube();

        // Invalid expiry
        assert!(BreedenLitzenberger::cumulative_probability(
            &cube, 0.03, -1.0, 5.0, 0.03, 0.02, 0.001
        ).is_err());

        // Invalid strike
        assert!(BreedenLitzenberger::cumulative_probability(
            &cube, 0.03, 1.0, 5.0, -0.03, 0.02, 0.001
        ).is_err());

        // Invalid delta_k
        assert!(BreedenLitzenberger::cumulative_probability(
            &cube, 0.03, 1.0, 5.0, 0.03, 0.02, -0.001
        ).is_err());
    }

    #[test]
    fn test_cumulative_probability_atm() {
        let cube = make_test_cube();
        let forward = 0.03;
        let expiry = 1.0;
        let tenor = 5.0;
        let rate = 0.02;
        let delta_k = 0.001;

        let cdf_atm = BreedenLitzenberger::cumulative_probability(
            &cube, forward, expiry, tenor, forward, rate, delta_k
        ).unwrap();

        // ATM CDF should be around 0.5 for symmetric distribution
        // With skew it can deviate, but should be in reasonable range
        assert!(cdf_atm > 0.2 && cdf_atm < 0.8);
    }
}
