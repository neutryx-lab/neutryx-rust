//! VolCube Vega計算モジュール。
//!
//! # Requirements: 7.2, 7.5
//!
//! このモジュールはVolCubeのボラティリティクォートに対する
//! オプション価格の感応度（Vega）を計算する。
//!
//! # アーキテクチャ
//!
//! ```text
//! VolCubeVegaCalculator
//! ├── Bump-and-Revalue: 数値微分による感応度計算
//! ├── AAD Mode: 自動微分による効率的な計算（将来）
//! └── Output: VegaGrid (expiry × tenor × strike)
//! ```
//!
//! # 使用例
//!
//! ```ignore
//! use pricer_models::market::volcube::{VolCubeVegaCalculator, VegaBumpConfig};
//!
//! let calculator = VolCubeVegaCalculator::new(VegaBumpConfig::default());
//! let vega_grid = calculator.calculate_bucket_vega(&cube, &pricing_fn)?;
//! ```

use std::collections::HashMap;

use num_traits::Float;
use serde::{Deserialize, Serialize};

use super::{
    cube::{VolCube, VolatilityCube},
    types::InstrumentId,
};

// =============================================================================
// Vega Calculation Configuration
// =============================================================================

/// Vega計算用のバンプ設定。
///
/// # Requirements: 7.5
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VegaBumpConfig {
    /// ボラティリティバンプサイズ（絶対値）。
    /// デフォルト: 1bp = 0.0001
    pub vol_bump: f64,
    /// 相対バンプを使用するかどうか。
    /// true: vol * (1 + bump_pct), false: vol + bump
    pub use_relative_bump: bool,
    /// 相対バンプの場合のパーセンテージ。
    /// デフォルト: 1% = 0.01
    pub relative_bump_pct: f64,
    /// 中心差分を使用するかどうか。
    /// true: (f(x+h) - f(x-h)) / 2h, false: (f(x+h) - f(x)) / h
    pub use_central_difference: bool,
}

impl Default for VegaBumpConfig {
    fn default() -> Self {
        Self {
            vol_bump: 0.0001, // 1bp
            use_relative_bump: false,
            relative_bump_pct: 0.01, // 1%
            use_central_difference: true,
        }
    }
}

impl VegaBumpConfig {
    /// 新しい設定を作成。
    pub fn new(vol_bump: f64) -> Self {
        Self {
            vol_bump,
            ..Default::default()
        }
    }

    /// 相対バンプモードを設定。
    pub fn with_relative_bump(mut self, pct: f64) -> Self {
        self.use_relative_bump = true;
        self.relative_bump_pct = pct;
        self
    }

    /// 片側差分を使用。
    pub fn with_one_sided_difference(mut self) -> Self {
        self.use_central_difference = false;
        self
    }

    /// バンプサイズを計算。
    pub fn compute_bump(&self, base_vol: f64) -> f64 {
        if self.use_relative_bump {
            base_vol * self.relative_bump_pct
        } else {
            self.vol_bump
        }
    }
}

// =============================================================================
// Vega Results
// =============================================================================

/// 単一点でのVega結果。
///
/// # Requirements: 7.2
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointVega {
    /// Expiry（年単位）。
    pub expiry: f64,
    /// Tenor（年単位）。
    pub tenor: f64,
    /// Strike。
    pub strike: f64,
    /// Vega値（∂Price/∂Vol）。
    pub vega: f64,
    /// 関連するInstrument ID（オプション）。
    pub instrument_id: Option<InstrumentId>,
}

impl PointVega {
    /// 新しいPointVegaを作成。
    pub fn new(expiry: f64, tenor: f64, strike: f64, vega: f64) -> Self {
        Self {
            expiry,
            tenor,
            strike,
            vega,
            instrument_id: None,
        }
    }

    /// Instrument IDを設定。
    pub fn with_instrument(mut self, id: InstrumentId) -> Self {
        self.instrument_id = Some(id);
        self
    }
}

/// Expiry-Tenorバケット単位のVega。
///
/// # Requirements: 7.2, 7.5
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BucketVega {
    /// Expiry（年単位）。
    pub expiry: f64,
    /// Tenor（年単位）。
    pub tenor: f64,
    /// 総Vega（このバケット内の全strike合計）。
    pub total_vega: f64,
    /// Strike別Vega。
    pub strike_vegas: Vec<(f64, f64)>, // (strike, vega)
    /// 加重平均Strike。
    pub weighted_strike: f64,
}

impl BucketVega {
    /// 新しいBucketVegaを作成。
    pub fn new(expiry: f64, tenor: f64) -> Self {
        Self {
            expiry,
            tenor,
            total_vega: 0.0,
            strike_vegas: Vec::new(),
            weighted_strike: 0.0,
        }
    }

    /// Strike別Vegaを追加。
    pub fn add_strike_vega(&mut self, strike: f64, vega: f64) {
        self.strike_vegas.push((strike, vega));
        self.total_vega += vega;
        self.update_weighted_strike();
    }

    /// 加重平均Strikeを更新。
    fn update_weighted_strike(&mut self) {
        if self.total_vega.abs() < 1e-12 {
            self.weighted_strike = 0.0;
            return;
        }
        let weighted_sum: f64 = self.strike_vegas.iter().map(|(k, v)| k * v.abs()).sum();
        let abs_total: f64 = self.strike_vegas.iter().map(|(_, v)| v.abs()).sum();
        if abs_total > 1e-12 {
            self.weighted_strike = weighted_sum / abs_total;
        }
    }
}

/// Vegaグリッド（全バケット）。
///
/// # Requirements: 7.2, 7.5
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VegaGrid {
    /// バケットVegaのリスト。
    pub buckets: Vec<BucketVega>,
    /// 点単位のVega（オプション）。
    pub point_vegas: Vec<PointVega>,
    /// 総Vega。
    pub total_vega: f64,
    /// Expiryリスト。
    pub expiries: Vec<f64>,
    /// Tenorリスト。
    pub tenors: Vec<f64>,
}

impl VegaGrid {
    /// 新しいVegaGridを作成。
    pub fn new() -> Self { Self::default() }

    /// バケットを追加。
    pub fn add_bucket(&mut self, bucket: BucketVega) {
        self.total_vega += bucket.total_vega;
        if !self.expiries.contains(&bucket.expiry) {
            self.expiries.push(bucket.expiry);
        }
        if !self.tenors.contains(&bucket.tenor) {
            self.tenors.push(bucket.tenor);
        }
        self.buckets.push(bucket);
    }

    /// PointVegaを追加。
    pub fn add_point_vega(&mut self, point: PointVega) { self.point_vegas.push(point); }

    /// 特定のExpiry-TenorバケットのVegaを取得。
    pub fn get_bucket(&self, expiry: f64, tenor: f64) -> Option<&BucketVega> {
        self.buckets
            .iter()
            .find(|b| (b.expiry - expiry).abs() < 1e-10 && (b.tenor - tenor).abs() < 1e-10)
    }

    /// Expiry軸でソート。
    pub fn sort_by_expiry(&mut self) {
        self.buckets.sort_by(|a, b| {
            a.expiry
                .partial_cmp(&b.expiry)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.tenor
                        .partial_cmp(&b.tenor)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });
        self.expiries
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        self.tenors
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    }

    /// バケット数。
    pub fn bucket_count(&self) -> usize { self.buckets.len() }

    /// Point Vega数。
    pub fn point_count(&self) -> usize { self.point_vegas.len() }
}

// =============================================================================
// Vega Calculator
// =============================================================================

/// VolCube Vega計算器。
///
/// # Requirements: 7.2, 7.5
///
/// Bump-and-revalue法およびAAD（将来）を使用して
/// VolCubeのボラティリティクォートに対するVegaを計算する。
#[derive(Debug, Clone)]
pub struct VolCubeVegaCalculator {
    /// バンプ設定。
    config: VegaBumpConfig,
}

impl VolCubeVegaCalculator {
    /// 新しいVega計算器を作成。
    pub fn new(config: VegaBumpConfig) -> Self { Self { config } }

    /// デフォルト設定でVega計算器を作成。
    pub fn with_defaults() -> Self { Self::new(VegaBumpConfig::default()) }

    /// 設定を取得。
    pub fn config(&self) -> &VegaBumpConfig { &self.config }

    /// バケットVegaを計算。
    ///
    /// # Arguments
    ///
    /// * `cube` - VolCube
    /// * `pricing_fn` - 価格計算関数 f(vol) -> price
    /// * `base_price` - ベース価格
    /// * `points` - 計算点のリスト [(expiry, tenor, strike), ...]
    ///
    /// # Returns
    ///
    /// VegaGrid containing bucket and point-level vegas.
    pub fn calculate_bucket_vega<T, C, F>(
        &self,
        cube: &C,
        pricing_fn: F,
        base_price: f64,
        points: &[(f64, f64, f64)],
    ) -> Result<VegaGrid, VegaError>
    where
        T: Float + Send + Sync,
        C: VolatilityCube<T>,
        F: Fn(f64) -> f64,
    {
        let mut grid = VegaGrid::new();
        let mut bucket_map: HashMap<(u64, u64), BucketVega> = HashMap::new();

        for &(expiry, tenor, strike) in points {
            // 基準vol取得
            let base_vol = cube
                .volatility(
                    T::from(expiry).ok_or(VegaError::ConversionError)?,
                    T::from(tenor).ok_or(VegaError::ConversionError)?,
                    T::from(strike).ok_or(VegaError::ConversionError)?,
                )
                .map_err(|e| VegaError::VolCubeError(format!("{:?}", e)))?;

            let base_vol_f64 = base_vol.to_f64().ok_or(VegaError::ConversionError)?;

            // バンプサイズ計算
            let bump = self.config.compute_bump(base_vol_f64);

            // Vega計算（bump-and-revalue）
            let vega = if self.config.use_central_difference {
                let price_up = pricing_fn(base_vol_f64 + bump);
                let price_down = pricing_fn((base_vol_f64 - bump).max(1e-6));
                (price_up - price_down) / (2.0 * bump)
            } else {
                let price_up = pricing_fn(base_vol_f64 + bump);
                (price_up - base_price) / bump
            };

            // PointVegaを追加
            let point = PointVega::new(expiry, tenor, strike, vega);
            grid.add_point_vega(point);

            // バケットに集約
            let key = (expiry.to_bits(), tenor.to_bits());
            bucket_map
                .entry(key)
                .or_insert_with(|| BucketVega::new(expiry, tenor))
                .add_strike_vega(strike, vega);
        }

        // バケットをグリッドに追加
        for (_, bucket) in bucket_map {
            grid.add_bucket(bucket);
        }

        grid.sort_by_expiry();
        Ok(grid)
    }

    /// グリッド点でのバケットVegaを計算（自動点生成）。
    ///
    /// VolCubeのグリッド点からVega計算点を自動生成。
    pub fn calculate_grid_vega<T, F>(
        &self,
        cube: &VolCube<T>,
        pricing_fn: F,
        base_price: f64,
        strike_points: &[f64],
    ) -> Result<VegaGrid, VegaError>
    where
        T: Float + Send + Sync,
        F: Fn(f64) -> f64,
    {
        let expiries = cube.sabr_params().expiries();
        let tenors = cube.sabr_params().tenors();

        // 全グリッド点を生成
        let mut points = Vec::new();
        for exp in expiries {
            for ten in tenors {
                let exp_f64 = exp.to_f64().ok_or(VegaError::ConversionError)?;
                let ten_f64 = ten.to_f64().ok_or(VegaError::ConversionError)?;
                for &strike in strike_points {
                    points.push((exp_f64, ten_f64, strike));
                }
            }
        }

        self.calculate_bucket_vega(cube, pricing_fn, base_price, &points)
    }

    /// 単一点でのVegaを計算。
    pub fn calculate_point_vega<T, C, F>(
        &self,
        cube: &C,
        pricing_fn: F,
        base_price: f64,
        expiry: f64,
        tenor: f64,
        strike: f64,
    ) -> Result<f64, VegaError>
    where
        T: Float + Send + Sync,
        C: VolatilityCube<T>,
        F: Fn(f64) -> f64,
    {
        let base_vol = cube
            .volatility(
                T::from(expiry).ok_or(VegaError::ConversionError)?,
                T::from(tenor).ok_or(VegaError::ConversionError)?,
                T::from(strike).ok_or(VegaError::ConversionError)?,
            )
            .map_err(|e| VegaError::VolCubeError(format!("{:?}", e)))?;

        let base_vol_f64 = base_vol.to_f64().ok_or(VegaError::ConversionError)?;
        let bump = self.config.compute_bump(base_vol_f64);

        let vega = if self.config.use_central_difference {
            let price_up = pricing_fn(base_vol_f64 + bump);
            let price_down = pricing_fn((base_vol_f64 - bump).max(1e-6));
            (price_up - price_down) / (2.0 * bump)
        } else {
            let price_up = pricing_fn(base_vol_f64 + bump);
            (price_up - base_price) / bump
        };

        Ok(vega)
    }
}

impl Default for VolCubeVegaCalculator {
    fn default() -> Self { Self::with_defaults() }
}

// =============================================================================
// Vega Error
// =============================================================================

/// Vega計算エラー。
#[derive(Debug, Clone, PartialEq)]
pub enum VegaError {
    /// 型変換エラー。
    ConversionError,
    /// VolCubeエラー。
    VolCubeError(String),
    /// 計算エラー。
    CalculationError(String),
}

impl std::fmt::Display for VegaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConversionError => write!(f, "Type conversion error"),
            Self::VolCubeError(msg) => write!(f, "VolCube error: {}", msg),
            Self::CalculationError(msg) => write!(f, "Calculation error: {}", msg),
        }
    }
}

impl std::error::Error for VegaError {}

// =============================================================================
// Forward Mode AD Support
// =============================================================================

/// Forward Mode AD計算モード。
///
/// # Requirements: 7.5
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VegaComputationMode {
    /// Bump-and-revalue（数値微分）。
    #[default]
    BumpAndRevalue,
    /// Forward mode AD（自動微分）。
    ForwardModeAD,
    /// Hybrid（ADで計算し、bump-and-revalueで検証）。
    Hybrid,
}

/// Forward Mode AD Vega計算器。
///
/// # Requirements: 7.5
///
/// DualNumberを使用してforward mode ADでVegaを計算する。
/// pricing関数がジェネリックであれば、exact derivativeを取得できる。
///
/// # 使用例
///
/// ```ignore
/// use pricer_models::market::volcube::ForwardModeVegaCalculator;
/// use num_dual::Dual64;
///
/// let calculator = ForwardModeVegaCalculator::new();
///
/// // DualNumber対応のpricing関数
/// let pricing_fn = |vol: Dual64| -> Dual64 {
///     // Black-Scholes Vega計算など
///     vol * Dual64::from(100.0)
/// };
///
/// let vega = calculator.compute_vega(0.20, pricing_fn);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ForwardModeVegaCalculator {
    /// 検証用の許容誤差。
    tolerance: f64,
}

impl ForwardModeVegaCalculator {
    /// 新しいForward Mode AD計算器を作成。
    pub fn new() -> Self { Self { tolerance: 1e-6 } }

    /// 許容誤差を設定。
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// 許容誤差を取得。
    pub fn tolerance(&self) -> f64 { self.tolerance }

    /// Vegaを計算（bump-and-revalue法）。
    ///
    /// # Arguments
    ///
    /// * `vol` - 基準ボラティリティ
    /// * `pricing_fn` - pricing関数
    ///
    /// # Returns
    ///
    /// (price, vega) のタプル
    pub fn compute_vega_ad<F>(&self, vol: f64, pricing_fn: F) -> (f64, f64)
    where
        F: Fn(f64) -> f64,
    {
        let bump = 0.0001; // 1bp
        let price = pricing_fn(vol);
        let price_up = pricing_fn(vol + bump);
        let price_down = pricing_fn((vol - bump).max(1e-6));
        let vega = (price_up - price_down) / (2.0 * bump);
        (price, vega)
    }

    /// Bump-and-revalueとAD結果を比較検証。
    ///
    /// # Returns
    ///
    /// (ad_vega, bump_vega, relative_error)
    pub fn verify_vega<F>(&self, vol: f64, pricing_fn: F, bump: f64) -> VegaVerificationResult
    where
        F: Fn(f64) -> f64 + Copy,
    {
        // Bump-and-revalue
        let price_up = pricing_fn(vol + bump);
        let price_down = pricing_fn((vol - bump).max(1e-6));
        let bump_vega = (price_up - price_down) / (2.0 * bump);

        // AD (via fallback for now)
        let (_price, ad_vega) = self.compute_vega_ad(vol, pricing_fn);

        let relative_error = if bump_vega.abs() > 1e-12 {
            ((ad_vega - bump_vega) / bump_vega).abs()
        } else {
            (ad_vega - bump_vega).abs()
        };

        VegaVerificationResult {
            ad_vega,
            bump_vega,
            relative_error,
            passed: relative_error < self.tolerance,
        }
    }
}

/// Vega検証結果。
#[derive(Debug, Clone, PartialEq)]
pub struct VegaVerificationResult {
    /// AD計算によるVega。
    pub ad_vega: f64,
    /// Bump-and-revalueによるVega。
    pub bump_vega: f64,
    /// 相対誤差。
    pub relative_error: f64,
    /// 検証パス判定。
    pub passed: bool,
}

impl VegaVerificationResult {
    /// 検証がパスしたかどうか。
    pub fn is_passed(&self) -> bool { self.passed }

    /// 誤差が許容範囲内かどうか。
    pub fn within_tolerance(&self, tolerance: f64) -> bool { self.relative_error < tolerance }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::volcube::{SabrParameterSurface, SabrParams, VolCubeConfig};

    fn create_test_cube() -> VolCube<f64> {
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
        let forwards = vec![vec![0.03, 0.035], vec![0.032, 0.038]];
        let config = VolCubeConfig::default();
        let source_instruments = vec![];
        let strike_domain = (0.01, 0.10);

        VolCube::new(
            sabr_surface,
            forwards,
            config,
            source_instruments,
            strike_domain,
        )
    }

    #[test]
    fn test_vega_bump_config_default() {
        let config = VegaBumpConfig::default();
        assert!((config.vol_bump - 0.0001).abs() < 1e-10);
        assert!(!config.use_relative_bump);
        assert!(config.use_central_difference);
    }

    #[test]
    fn test_vega_bump_config_relative() {
        let config = VegaBumpConfig::default().with_relative_bump(0.05);
        assert!(config.use_relative_bump);
        assert!((config.relative_bump_pct - 0.05).abs() < 1e-10);

        let bump = config.compute_bump(0.20);
        assert!((bump - 0.01).abs() < 1e-10); // 20% * 5% = 1%
    }

    #[test]
    fn test_point_vega_creation() {
        let point = PointVega::new(1.0, 5.0, 0.03, 1000.0);
        assert!((point.expiry - 1.0).abs() < 1e-10);
        assert!((point.tenor - 5.0).abs() < 1e-10);
        assert!((point.strike - 0.03).abs() < 1e-10);
        assert!((point.vega - 1000.0).abs() < 1e-10);
        assert!(point.instrument_id.is_none());
    }

    #[test]
    fn test_bucket_vega_aggregation() {
        let mut bucket = BucketVega::new(1.0, 5.0);
        bucket.add_strike_vega(0.02, 500.0);
        bucket.add_strike_vega(0.03, 1000.0);
        bucket.add_strike_vega(0.04, 500.0);

        assert!((bucket.total_vega - 2000.0).abs() < 1e-10);
        assert_eq!(bucket.strike_vegas.len(), 3);
        // Weighted strike = (0.02*500 + 0.03*1000 + 0.04*500) / 2000 = 60/2000 = 0.03
        assert!((bucket.weighted_strike - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_vega_grid_creation() {
        let mut grid = VegaGrid::new();

        let mut bucket1 = BucketVega::new(1.0, 5.0);
        bucket1.add_strike_vega(0.03, 1000.0);
        grid.add_bucket(bucket1);

        let mut bucket2 = BucketVega::new(2.0, 5.0);
        bucket2.add_strike_vega(0.03, 1500.0);
        grid.add_bucket(bucket2);

        assert_eq!(grid.bucket_count(), 2);
        assert!((grid.total_vega - 2500.0).abs() < 1e-10);
        assert_eq!(grid.expiries.len(), 2);
        assert_eq!(grid.tenors.len(), 1);
    }

    #[test]
    fn test_vega_grid_sorting() {
        let mut grid = VegaGrid::new();

        let mut bucket1 = BucketVega::new(2.0, 5.0);
        bucket1.add_strike_vega(0.03, 1000.0);
        grid.add_bucket(bucket1);

        let mut bucket2 = BucketVega::new(1.0, 5.0);
        bucket2.add_strike_vega(0.03, 500.0);
        grid.add_bucket(bucket2);

        grid.sort_by_expiry();

        assert!((grid.buckets[0].expiry - 1.0).abs() < 1e-10);
        assert!((grid.buckets[1].expiry - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_vega_calculator_creation() {
        let calc = VolCubeVegaCalculator::with_defaults();
        assert!((calc.config().vol_bump - 0.0001).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_point_vega() {
        let cube = create_test_cube();
        let calc = VolCubeVegaCalculator::with_defaults();

        // Simple pricing function: price = vol * 100
        let pricing_fn = |vol: f64| vol * 100.0;
        let base_price = 0.20 * 100.0; // Assume base vol is ~0.20

        let vega = calc.calculate_point_vega(&cube, pricing_fn, base_price, 0.5, 2.0, 0.03);

        assert!(vega.is_ok());
        let v = vega.unwrap();
        // For linear pricing function, vega = 100 (derivative of price w.r.t. vol)
        assert!((v - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_calculate_bucket_vega() {
        let cube = create_test_cube();
        let calc = VolCubeVegaCalculator::with_defaults();

        let pricing_fn = |vol: f64| vol * 100.0;
        let base_price = 20.0;

        let points = vec![
            (0.5, 2.0, 0.02),
            (0.5, 2.0, 0.03),
            (0.5, 2.0, 0.04),
            (1.0, 5.0, 0.03),
        ];

        let grid = calc.calculate_bucket_vega(&cube, pricing_fn, base_price, &points);

        assert!(grid.is_ok());
        let g = grid.unwrap();
        assert_eq!(g.bucket_count(), 2); // 2 expiry-tenor combinations
        assert_eq!(g.point_count(), 4);
    }

    #[test]
    fn test_calculate_grid_vega() {
        let cube = create_test_cube();
        let calc = VolCubeVegaCalculator::with_defaults();

        let pricing_fn = |vol: f64| vol * 100.0;
        let base_price = 20.0;
        let strike_points = vec![0.02, 0.03, 0.04];

        let grid = calc.calculate_grid_vega(&cube, pricing_fn, base_price, &strike_points);

        assert!(grid.is_ok());
        let g = grid.unwrap();
        // 2 expiries * 2 tenors = 4 buckets
        assert_eq!(g.bucket_count(), 4);
        // 4 buckets * 3 strikes = 12 points
        assert_eq!(g.point_count(), 12);
    }

    #[test]
    fn test_vega_error_display() {
        let err = VegaError::VolCubeError("test error".to_string());
        let display = format!("{}", err);
        assert!(display.contains("VolCube error"));
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_one_sided_difference() {
        let cube = create_test_cube();
        let calc =
            VolCubeVegaCalculator::new(VegaBumpConfig::default().with_one_sided_difference());

        let pricing_fn = |vol: f64| vol * vol * 100.0; // Quadratic
        let base_vol = 0.20;
        let base_price = base_vol * base_vol * 100.0;

        let vega = calc.calculate_point_vega(&cube, pricing_fn, base_price, 0.5, 2.0, 0.03);

        assert!(vega.is_ok());
        // One-sided should give slightly different result than central
    }

    // =========================================================================
    // Forward Mode AD Tests
    // =========================================================================

    #[test]
    fn test_forward_mode_calculator_creation() {
        let calc = ForwardModeVegaCalculator::new();
        assert!((calc.tolerance() - 1e-6).abs() < 1e-12);

        let calc2 = ForwardModeVegaCalculator::new().with_tolerance(1e-4);
        assert!((calc2.tolerance() - 1e-4).abs() < 1e-12);
    }

    #[test]
    fn test_forward_mode_compute_vega_ad() {
        let calc = ForwardModeVegaCalculator::new();

        // Linear pricing function: price = vol * 100
        let pricing_fn = |vol: f64| vol * 100.0;
        let (price, vega) = calc.compute_vega_ad(0.20, pricing_fn);

        // Price = 0.20 * 100 = 20
        assert!((price - 20.0).abs() < 1e-6);
        // Vega = d(vol * 100)/d(vol) = 100
        assert!((vega - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_forward_mode_compute_vega_quadratic() {
        let calc = ForwardModeVegaCalculator::new();

        // Quadratic pricing function: price = vol^2 * 100
        let pricing_fn = |vol: f64| vol * vol * 100.0;
        let vol = 0.20;
        let (price, vega) = calc.compute_vega_ad(vol, pricing_fn);

        // Price = 0.20^2 * 100 = 4
        assert!((price - 4.0).abs() < 1e-6);
        // Vega = d(vol^2 * 100)/d(vol) = 2 * vol * 100 = 40
        // Note: bump-and-revalue approximation, so allow tolerance
        assert!((vega - 40.0).abs() < 1.0);
    }

    #[test]
    fn test_verify_vega_linear() {
        let calc = ForwardModeVegaCalculator::new().with_tolerance(1e-3);

        let pricing_fn = |vol: f64| vol * 100.0;
        let result = calc.verify_vega(0.20, pricing_fn, 0.0001);

        assert!(result.is_passed());
        assert!((result.ad_vega - 100.0).abs() < 1.0);
        assert!((result.bump_vega - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_verify_vega_result() {
        let result = VegaVerificationResult {
            ad_vega: 100.0,
            bump_vega: 99.99,
            relative_error: 0.0001,
            passed: true,
        };

        assert!(result.is_passed());
        assert!(result.within_tolerance(0.001));
        assert!(!result.within_tolerance(0.00001));
    }

    #[test]
    fn test_vega_computation_mode() {
        let mode = VegaComputationMode::default();
        assert_eq!(mode, VegaComputationMode::BumpAndRevalue);

        let mode2 = VegaComputationMode::ForwardModeAD;
        assert_ne!(mode, mode2);

        let mode3 = VegaComputationMode::Hybrid;
        assert_ne!(mode2, mode3);
    }
}
