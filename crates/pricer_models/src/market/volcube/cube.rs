//! VolatilityCube 3次元ボラティリティ構造。
//!
//! # Requirements: 1.5, 2.1-2.5, 3.1-3.4, 4.1-4.4
//!
//! 3次元（Expiry, Tenor, Strike）でのボラティリティ補間と
//! 確率密度関数計算を提供する。

use num_traits::Float;

use super::{
    breeden_litzenberger::BreedenLitzenberger,
    config::{ExtrapolationMethod, VolCubeConfig},
    sabr_surface::SabrParameterSurface,
    types::InstrumentId,
};
use crate::market::error::MarketDataError;

/// 3次元ボラティリティキューブトレイト。
///
/// # Requirements: 2.1-2.5
///
/// 任意の(expiry, tenor, strike)でのボラティリティ取得と
/// ドメイン情報のインターフェースを定義する。
pub trait VolatilityCube<T: Float>: Send + Sync {
    /// 3D vol補間。
    ///
    /// # Arguments
    ///
    /// * `expiry` - オプションの満期（年単位、> 0）
    /// * `tenor` - Underlyingのtenor（年単位、> 0、Swaptionなど）
    /// * `strike` - Strike価格/レート（> 0）
    ///
    /// # Returns
    ///
    /// 補間されたimplied volatility、またはエラー。
    fn volatility(&self, expiry: T, tenor: T, strike: T) -> Result<T, MarketDataError>;

    /// 確率密度関数。
    ///
    /// # Requirements: 3.1
    ///
    /// Breeden-Litzenberger公式に基づくリスクニュートラル密度。
    fn probability_density(&self, expiry: T, strike: T) -> Result<T, MarketDataError>;

    /// 累積確率分布。
    ///
    /// # Requirements: 3.2
    fn cumulative_probability(&self, expiry: T, strike: T) -> Result<T, MarketDataError>;

    /// Expiry軸の有効範囲。
    fn expiry_domain(&self) -> (T, T);

    /// Tenor軸の有効範囲。
    fn tenor_domain(&self) -> (T, T);

    /// Strike軸の有効範囲。
    fn strike_domain(&self) -> (T, T);

    /// ソースInstrument IDリスト。
    ///
    /// # Requirements: 4.1
    fn source_instruments(&self) -> &[InstrumentId];
}

/// VolatilityCube実装。
///
/// # Requirements: 1.5, 2.1-2.5, 4.1
///
/// SABR パラメータ平面に基づく3Dボラティリティキューブ。
/// Expiry-Tenor平面でSABRパラメータをBilinear補間し、
/// Strike軸でHagan公式を適用してimplied volatilityを計算する。
#[derive(Debug, Clone)]
pub struct VolCube<T: Float> {
    /// SABRパラメータ平面。
    sabr_params: SabrParameterSurface<T>,
    /// 設定。
    config: VolCubeConfig,
    /// ソースInstrument IDリスト。
    source_instruments: Vec<InstrumentId>,
    /// Forward rate平面（expiry-tenor格子上）。
    /// forwardsは(expiry, tenor)格子に対応。
    forwards: Vec<Vec<T>>,
    /// Expiry格子点。
    expiries: Vec<T>,
    /// Tenor格子点。
    tenors: Vec<T>,
    /// Strike下限。
    strike_min: T,
    /// Strike上限。
    strike_max: T,
    /// 確率密度計算用デフォルトテナー（年単位）。
    default_tenor: T,
    /// 確率密度計算用無リスク金利。
    risk_free_rate: T,
    /// 確率密度計算用ストライク差分。
    delta_k: T,
}

impl<T: Float> VolCube<T> {
    /// 新しいVolCubeを構築。
    ///
    /// # Arguments
    ///
    /// * `sabr_params` - カリブレーション済みSABRパラメータ平面
    /// * `forwards` - Forward rate行列 `forwards\[i\]\[j\]` = (expiry\[i\],
    ///   tenor\[j\])でのforward
    /// * `config` - VolCube設定
    /// * `source_instruments` - ソースInstrument IDリスト
    /// * `strike_domain` - (strike_min, strike_max)
    pub fn new(
        sabr_params: SabrParameterSurface<T>,
        forwards: Vec<Vec<T>>,
        config: VolCubeConfig,
        source_instruments: Vec<InstrumentId>,
        strike_domain: (T, T),
    ) -> Self {
        let expiries = sabr_params.expiries().to_vec();
        let tenors = sabr_params.tenors().to_vec();

        // 確率密度計算用のデフォルト値
        let default_tenor = *tenors.first().unwrap_or(&T::from(5.0).unwrap());
        let risk_free_rate = T::from(0.02).unwrap(); // 2%
        let delta_k = (strike_domain.1 - strike_domain.0) * T::from(0.01).unwrap(); // strike範囲の1%

        Self {
            sabr_params,
            config,
            source_instruments,
            forwards,
            expiries,
            tenors,
            strike_min: strike_domain.0,
            strike_max: strike_domain.1,
            default_tenor,
            risk_free_rate,
            delta_k,
        }
    }

    /// 確率密度計算用パラメータを設定。
    pub fn with_density_params(mut self, tenor: T, risk_free_rate: T, delta_k: T) -> Self {
        self.default_tenor = tenor;
        self.risk_free_rate = risk_free_rate;
        self.delta_k = delta_k;
        self
    }

    /// デフォルトテナーを取得。
    pub fn default_tenor(&self) -> T { self.default_tenor }

    /// 無リスク金利を取得。
    pub fn risk_free_rate(&self) -> T { self.risk_free_rate }

    /// 差分計算用ストライク変位を取得。
    pub fn delta_k(&self) -> T { self.delta_k }

    /// Forward rateを補間取得。
    ///
    /// 簡易的なBilinear補間を使用。
    fn interpolate_forward(&self, expiry: T, tenor: T) -> Result<T, MarketDataError> {
        // 格子点インデックスを探索
        let (ei_lo, ei_hi, e_frac) = self.find_bracket(&self.expiries, expiry)?;
        let (ti_lo, ti_hi, t_frac) = self.find_bracket(&self.tenors, tenor)?;

        // Bilinear補間
        let f00 = self.forwards[ei_lo][ti_lo];
        let f01 = self.forwards[ei_lo][ti_hi];
        let f10 = self.forwards[ei_hi][ti_lo];
        let f11 = self.forwards[ei_hi][ti_hi];

        let one = T::one();
        let f0 = f00 * (one - t_frac) + f01 * t_frac;
        let f1 = f10 * (one - t_frac) + f11 * t_frac;
        let forward = f0 * (one - e_frac) + f1 * e_frac;

        Ok(forward)
    }

    /// ブラケットインデックスと補間係数を取得。
    fn find_bracket(&self, grid: &[T], x: T) -> Result<(usize, usize, T), MarketDataError> {
        if grid.is_empty() {
            return Err(MarketDataError::InsufficientData { got: 0, need: 2 });
        }
        if grid.len() == 1 {
            return Ok((0, 0, T::zero()));
        }

        let x_min = grid[0];
        let x_max = *grid.last().unwrap();

        // 範囲外処理
        if x < x_min {
            match self.config.extrapolation {
                ExtrapolationMethod::Flat => return Ok((0, 1, T::zero())),
                ExtrapolationMethod::Linear => {
                    // 線形外挿: 下側
                    let dx = grid[1] - grid[0];
                    if dx <= T::zero() {
                        return Ok((0, 1, T::zero()));
                    }
                    let frac = (x - x_min) / dx; // 負の値
                    return Ok((0, 1, frac));
                }
                ExtrapolationMethod::Error => {
                    return Err(MarketDataError::OutOfBounds {
                        x: x.to_f64().unwrap_or(0.0),
                        min: x_min.to_f64().unwrap_or(0.0),
                        max: x_max.to_f64().unwrap_or(0.0),
                    });
                }
            }
        }
        if x > x_max {
            match self.config.extrapolation {
                ExtrapolationMethod::Flat => return Ok((grid.len() - 2, grid.len() - 1, T::one())),
                ExtrapolationMethod::Linear => {
                    let n = grid.len();
                    let dx = grid[n - 1] - grid[n - 2];
                    if dx <= T::zero() {
                        return Ok((n - 2, n - 1, T::one()));
                    }
                    let frac = T::one() + (x - x_max) / dx;
                    return Ok((n - 2, n - 1, frac));
                }
                ExtrapolationMethod::Error => {
                    return Err(MarketDataError::OutOfBounds {
                        x: x.to_f64().unwrap_or(0.0),
                        min: x_min.to_f64().unwrap_or(0.0),
                        max: x_max.to_f64().unwrap_or(0.0),
                    });
                }
            }
        }

        // 内側: バイナリサーチでブラケットを見つける
        let mut lo = 0;
        let mut hi = grid.len() - 1;
        while hi - lo > 1 {
            let mid = usize::midpoint(lo, hi);
            if grid[mid] <= x {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        let dx = grid[hi] - grid[lo];
        let frac = if dx <= T::zero() {
            T::zero()
        } else {
            (x - grid[lo]) / dx
        };

        Ok((lo, hi, frac))
    }

    /// SABR Hagan公式でimplied volatilityを計算。
    fn sabr_implied_vol(
        &self,
        forward: T,
        strike: T,
        expiry: T,
        alpha: T,
        beta: T,
        rho: T,
        nu: T,
    ) -> T {
        let eps = T::from(1e-10).unwrap();
        let one = T::one();
        let two = T::from(2.0).unwrap();
        let three = T::from(3.0).unwrap();
        let twentyfour = T::from(24.0).unwrap();
        let quarter = T::from(0.25).unwrap();
        let nineteen_twenty = T::from(1920.0).unwrap();

        // ATM近傍の処理
        if (forward - strike).abs() < eps * forward {
            return self.sabr_atm_vol(forward, expiry, alpha, beta, rho, nu);
        }

        let one_minus_beta = one - beta;
        let log_fk = (forward / strike).ln();
        let fk_mid = (forward * strike).powf(one_minus_beta / two);

        // z = (nu/alpha) * (FK)^((1-beta)/2) * ln(F/K)
        let z = (nu / alpha) * fk_mid * log_fk;

        // chi(z) = ln[(sqrt(1-2*rho*z+z^2)+z-rho)/(1-rho)]
        let sqrt_term = (one - two * rho * z + z * z).sqrt();
        let chi_z = ((sqrt_term + z - rho) / (one - rho)).ln();

        // Handle chi(z) near zero
        let z_over_chi = if chi_z.abs() < eps { one } else { z / chi_z };

        // Denominator from log-moneyness expansion
        let log_fk_2 = log_fk * log_fk;
        let log_fk_4 = log_fk_2 * log_fk_2;
        let one_minus_beta_2 = one_minus_beta * one_minus_beta;
        let one_minus_beta_4 = one_minus_beta_2 * one_minus_beta_2;

        let denom = one
            + one_minus_beta_2 * log_fk_2 / twentyfour
            + one_minus_beta_4 * log_fk_4 / nineteen_twenty;

        // Higher-order corrections
        let fk_mid_2 = fk_mid * fk_mid;
        let term1 = one_minus_beta_2 * alpha * alpha / (twentyfour * fk_mid_2);
        let term2 = quarter * rho * beta * nu * alpha / fk_mid;
        let term3 = (two - three * rho * rho) * nu * nu / twentyfour;

        let higher_order = one + (term1 + term2 + term3) * expiry;

        // Final volatility
        (alpha / fk_mid) * z_over_chi * higher_order / denom
    }

    /// SABR ATM volatility approximation。
    fn sabr_atm_vol(&self, forward: T, expiry: T, alpha: T, beta: T, rho: T, nu: T) -> T {
        let one = T::one();
        let two = T::from(2.0).unwrap();
        let three = T::from(3.0).unwrap();
        let twentyfour = T::from(24.0).unwrap();
        let quarter = T::from(0.25).unwrap();

        let one_minus_beta = one - beta;
        let f_pow = forward.powf(one_minus_beta);

        // Base ATM vol
        let vol_0 = alpha / f_pow;

        // Higher-order ATM corrections
        let f_pow_2 = f_pow * f_pow;
        let term1 = one_minus_beta * one_minus_beta * alpha * alpha / (twentyfour * f_pow_2);
        let term2 = quarter * rho * beta * nu * alpha / f_pow;
        let term3 = (two - three * rho * rho) * nu * nu / twentyfour;

        vol_0 * (one + (term1 + term2 + term3) * expiry)
    }

    /// 設定への参照を取得。
    pub fn config(&self) -> &VolCubeConfig { &self.config }

    /// SABRパラメータ平面への参照を取得。
    pub fn sabr_params(&self) -> &SabrParameterSurface<T> { &self.sabr_params }
}

impl<T: Float + Send + Sync> VolatilityCube<T> for VolCube<T> {
    fn volatility(&self, expiry: T, tenor: T, strike: T) -> Result<T, MarketDataError> {
        let zero = T::zero();

        // 入力検証
        if expiry <= zero {
            return Err(MarketDataError::InvalidExpiry {
                expiry: expiry.to_f64().unwrap_or(0.0),
            });
        }
        if tenor <= zero {
            return Err(MarketDataError::InvalidMaturity {
                t: tenor.to_f64().unwrap_or(0.0),
            });
        }
        if strike <= zero {
            return Err(MarketDataError::InvalidStrike {
                strike: strike.to_f64().unwrap_or(0.0),
            });
        }

        // Strike範囲チェック
        if strike < self.strike_min || strike > self.strike_max {
            match self.config.extrapolation {
                ExtrapolationMethod::Error => {
                    return Err(MarketDataError::OutOfBounds {
                        x: strike.to_f64().unwrap_or(0.0),
                        min: self.strike_min.to_f64().unwrap_or(0.0),
                        max: self.strike_max.to_f64().unwrap_or(0.0),
                    });
                }
                ExtrapolationMethod::Flat => {
                    // Flat extrapolation: 境界値を使用
                    let clamped_strike = if strike < self.strike_min {
                        self.strike_min
                    } else {
                        self.strike_max
                    };
                    return self.volatility(expiry, tenor, clamped_strike);
                }
                ExtrapolationMethod::Linear => {
                    // Linear extrapolation: 許容（SABRが自然に外挿）
                }
            }
        }

        // SABRパラメータを補間取得
        let sabr = self
            .sabr_params
            .interpolate(expiry, tenor)
            .map_err(MarketDataError::Interpolation)?;

        // Forward rateを補間取得
        let forward = self.interpolate_forward(expiry, tenor)?;

        // SABR Hagan公式でimplied volatilityを計算
        let vol = self.sabr_implied_vol(
            forward, strike, expiry, sabr.alpha, sabr.beta, sabr.rho, sabr.nu,
        );

        Ok(vol)
    }

    fn probability_density(&self, expiry: T, strike: T) -> Result<T, MarketDataError> {
        let forward = self.interpolate_forward(expiry, self.default_tenor)?;

        BreedenLitzenberger::probability_density(
            self,
            forward,
            expiry,
            self.default_tenor,
            strike,
            self.risk_free_rate,
            self.delta_k,
        )
        .map_err(|e| MarketDataError::InterpolationFailed {
            reason: e.to_string(),
        })
    }

    fn cumulative_probability(&self, expiry: T, strike: T) -> Result<T, MarketDataError> {
        let forward = self.interpolate_forward(expiry, self.default_tenor)?;

        BreedenLitzenberger::cumulative_probability(
            self,
            forward,
            expiry,
            self.default_tenor,
            strike,
            self.risk_free_rate,
            self.delta_k,
        )
        .map_err(|e| MarketDataError::InterpolationFailed {
            reason: e.to_string(),
        })
    }

    fn expiry_domain(&self) -> (T, T) { self.sabr_params.expiry_domain() }

    fn tenor_domain(&self) -> (T, T) { self.sabr_params.tenor_domain() }

    fn strike_domain(&self) -> (T, T) { (self.strike_min, self.strike_max) }

    fn source_instruments(&self) -> &[InstrumentId] { &self.source_instruments }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::volcube::types::SabrParams;

    fn create_test_cube() -> VolCube<f64> {
        // 2x2の最小グリッド
        let expiries = vec![0.5, 1.0];
        let tenors = vec![2.0, 5.0];
        let beta = 0.5;

        let params = vec![
            vec![
                SabrParams::new(0.04, beta, -0.3, 0.4),
                SabrParams::new(0.05, beta, -0.25, 0.35),
            ],
            vec![
                SabrParams::new(0.045, beta, -0.35, 0.45),
                SabrParams::new(0.055, beta, -0.2, 0.3),
            ],
        ];

        let sabr_surface = SabrParameterSurface::new(expiries, tenors, &params, beta).unwrap();

        let forwards = vec![
            vec![0.03, 0.035],  // expiry = 0.5
            vec![0.032, 0.038], // expiry = 1.0
        ];

        let config = VolCubeConfig::default();
        let source_instruments = vec![InstrumentId::new("INST-1"), InstrumentId::new("INST-2")];
        let strike_domain = (0.01, 0.10);

        VolCube::new(
            sabr_surface,
            forwards,
            config,
            source_instruments,
            strike_domain,
        )
    }

    // =========================================================================
    // VolCube Construction Tests
    // =========================================================================

    #[test]
    fn test_volcube_new() {
        let cube = create_test_cube();
        assert_eq!(cube.expiries.len(), 2);
        assert_eq!(cube.tenors.len(), 2);
        assert_eq!(cube.source_instruments().len(), 2);
    }

    #[test]
    fn test_volcube_domain() {
        let cube = create_test_cube();

        let (exp_min, exp_max) = cube.expiry_domain();
        assert_eq!(exp_min, 0.5);
        assert_eq!(exp_max, 1.0);

        let (ten_min, ten_max) = cube.tenor_domain();
        assert_eq!(ten_min, 2.0);
        assert_eq!(ten_max, 5.0);

        let (k_min, k_max) = cube.strike_domain();
        assert_eq!(k_min, 0.01);
        assert_eq!(k_max, 0.10);
    }

    // =========================================================================
    // Volatility Lookup Tests
    // =========================================================================

    #[test]
    fn test_volcube_volatility_atm() {
        let cube = create_test_cube();

        // ATMでのvol (forward ≈ 0.03)
        let vol = cube.volatility(0.5, 2.0, 0.03).unwrap();

        // Positive volatility
        assert!(vol > 0.0);
        assert!(vol < 1.0); // 合理的な範囲
    }

    #[test]
    fn test_volcube_volatility_smile() {
        let cube = create_test_cube();
        let forward = 0.03;

        // ATM vol
        let vol_atm = cube.volatility(0.5, 2.0, forward).unwrap();

        // OTM put
        let vol_low = cube.volatility(0.5, 2.0, 0.02).unwrap();

        // OTM call
        let vol_high = cube.volatility(0.5, 2.0, 0.04).unwrap();

        // すべて正
        assert!(vol_atm > 0.0);
        assert!(vol_low > 0.0);
        assert!(vol_high > 0.0);

        // Smileが存在（異なるvol）
        assert!((vol_low - vol_atm).abs() > 1e-6 || (vol_high - vol_atm).abs() > 1e-6);
    }

    #[test]
    fn test_volcube_volatility_interpolated_expiry_tenor() {
        let cube = create_test_cube();

        // グリッド間の点での補間
        let vol = cube.volatility(0.75, 3.5, 0.03).unwrap();

        assert!(vol > 0.0);
        assert!(vol < 1.0);
    }

    #[test]
    fn test_volcube_volatility_invalid_expiry() {
        let cube = create_test_cube();
        let result = cube.volatility(-1.0, 2.0, 0.03);
        assert!(result.is_err());
        assert!(matches!(result, Err(MarketDataError::InvalidExpiry { .. })));
    }

    #[test]
    fn test_volcube_volatility_invalid_tenor() {
        let cube = create_test_cube();
        let result = cube.volatility(0.5, 0.0, 0.03);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(MarketDataError::InvalidMaturity { .. })
        ));
    }

    #[test]
    fn test_volcube_volatility_invalid_strike() {
        let cube = create_test_cube();
        let result = cube.volatility(0.5, 2.0, -0.01);
        assert!(result.is_err());
        assert!(matches!(result, Err(MarketDataError::InvalidStrike { .. })));
    }

    #[test]
    fn test_volcube_volatility_strike_out_of_bounds_flat_extrap() {
        let cube = create_test_cube();
        // デフォルトはFlat extrapolation

        // 下限以下
        let vol_low = cube.volatility(0.5, 2.0, 0.005);
        assert!(vol_low.is_ok());

        // 上限以上
        let vol_high = cube.volatility(0.5, 2.0, 0.15);
        assert!(vol_high.is_ok());
    }

    #[test]
    fn test_volcube_volatility_strike_out_of_bounds_error_extrap() {
        let sabr_surface = {
            let expiries = vec![0.5, 1.0];
            let tenors = vec![2.0, 5.0];
            let beta = 0.5;
            let params = vec![
                vec![
                    SabrParams::new(0.04, beta, -0.3, 0.4),
                    SabrParams::new(0.05, beta, -0.25, 0.35),
                ],
                vec![
                    SabrParams::new(0.045, beta, -0.35, 0.45),
                    SabrParams::new(0.055, beta, -0.2, 0.3),
                ],
            ];
            SabrParameterSurface::new(expiries, tenors, &params, beta).unwrap()
        };

        let config = VolCubeConfig::default().with_extrapolation(ExtrapolationMethod::Error);

        let cube = VolCube::new(
            sabr_surface,
            vec![vec![0.03, 0.035], vec![0.032, 0.038]],
            config,
            vec![],
            (0.01, 0.10),
        );

        // 範囲外はエラー
        let result = cube.volatility(0.5, 2.0, 0.005);
        assert!(result.is_err());
        assert!(matches!(result, Err(MarketDataError::OutOfBounds { .. })));
    }

    // =========================================================================
    // Source Instruments Tests
    // =========================================================================

    #[test]
    fn test_volcube_source_instruments() {
        let cube = create_test_cube();
        let instruments = cube.source_instruments();

        assert_eq!(instruments.len(), 2);
        assert_eq!(instruments[0].as_str(), "INST-1");
        assert_eq!(instruments[1].as_str(), "INST-2");
    }

    // =========================================================================
    // Config Tests
    // =========================================================================

    #[test]
    fn test_volcube_config() {
        let cube = create_test_cube();
        let config = cube.config();

        assert_eq!(config.extrapolation, ExtrapolationMethod::Flat);
    }

    // =========================================================================
    // Probability Density Tests
    // =========================================================================

    #[test]
    fn test_volcube_probability_density() {
        let cube = create_test_cube();
        let result = cube.probability_density(0.75, 0.03);

        // Should succeed and return non-negative density
        assert!(result.is_ok());
        let density = result.unwrap();
        assert!(
            density >= 0.0,
            "Density should be non-negative: {}",
            density
        );
    }

    #[test]
    fn test_volcube_cumulative_probability() {
        let cube = create_test_cube();
        let result = cube.cumulative_probability(0.75, 0.03);

        // Should succeed and return value in [0, 1]
        assert!(result.is_ok());
        let cdf = result.unwrap();
        assert!(cdf >= 0.0 && cdf <= 1.0, "CDF should be in [0, 1]: {}", cdf);
    }

    #[test]
    fn test_volcube_probability_density_with_custom_params() {
        let cube = create_test_cube().with_density_params(3.5, 0.03, 0.002);

        assert_eq!(cube.default_tenor(), 3.5);
        assert_eq!(cube.risk_free_rate(), 0.03);
        assert_eq!(cube.delta_k(), 0.002);

        let result = cube.probability_density(0.75, 0.03);
        assert!(result.is_ok());
    }

    // =========================================================================
    // SABR Formula Tests
    // =========================================================================

    #[test]
    fn test_sabr_atm_vol_consistency() {
        let cube = create_test_cube();
        let forward = 0.03;

        // ATM near forward
        let vol_atm = cube.volatility(0.5, 2.0, forward).unwrap();
        let vol_near = cube.volatility(0.5, 2.0, forward * 1.0001).unwrap();

        // ATM付近では連続
        assert!((vol_atm - vol_near).abs() < 0.01);
    }

    #[test]
    fn test_sabr_vol_positive() {
        let cube = create_test_cube();

        // 様々なstrikeで正のvol
        for strike in [0.015, 0.02, 0.025, 0.03, 0.035, 0.04, 0.05, 0.08] {
            let vol = cube.volatility(0.75, 3.5, strike).unwrap();
            assert!(vol > 0.0, "Negative vol at strike {}", strike);
        }
    }
}
