//! AAD検証とSmooth Approximationモジュール。
//!
//! # Requirements: 7.6, 7.7
//!
//! このモジュールはAAD（自動微分）とbump-and-revalueのクロス検証、
//! および不連続点でのsmooth approximation適用を提供する。
//!
//! # アーキテクチャ
//!
//! ```text
//! AADValidation
//! ├── CrossValidator: AAD vs bump-and-revalue比較
//! ├── SmoothingConfig: 平滑化パラメータ設定
//! └── ValidationReport: 検証結果レポート
//! ```
//!
//! # 使用例
//!
//! ```ignore
//! use pricer_models::market::volcube::{AADCrossValidator, ValidationReport};
//!
//! let validator = AADCrossValidator::new();
//! let report = validator.validate_vega(&cube, &pricing_fn, 0.5, 2.0, 0.03)?;
//! assert!(report.is_valid());
//! ```

use serde::{Deserialize, Serialize};

// =============================================================================
// Smoothing Configuration
// =============================================================================

/// 平滑化設定。
///
/// # Requirements: 7.7
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmoothingConfig {
    /// 平滑化パラメータ ε。
    /// デフォルト: 1e-6
    pub epsilon: f64,
    /// Smooth maxを使用するかどうか。
    pub use_smooth_max: bool,
    /// Smooth minを使用するかどうか。
    pub use_smooth_min: bool,
    /// Smooth absを使用するかどうか。
    pub use_smooth_abs: bool,
    /// Smooth sqrtを使用するかどうか。
    pub use_smooth_sqrt: bool,
    /// Smooth indicatorを使用するかどうか。
    pub use_smooth_indicator: bool,
}

impl Default for SmoothingConfig {
    fn default() -> Self {
        Self {
            epsilon: 1e-6,
            use_smooth_max: true,
            use_smooth_min: true,
            use_smooth_abs: true,
            use_smooth_sqrt: true,
            use_smooth_indicator: true,
        }
    }
}

impl SmoothingConfig {
    /// 新しい設定を作成。
    pub fn new(epsilon: f64) -> Self {
        Self {
            epsilon,
            ..Default::default()
        }
    }

    /// すべての平滑化を無効化。
    pub fn disabled() -> Self {
        Self {
            epsilon: 0.0,
            use_smooth_max: false,
            use_smooth_min: false,
            use_smooth_abs: false,
            use_smooth_sqrt: false,
            use_smooth_indicator: false,
        }
    }

    /// εを設定。
    pub fn with_epsilon(mut self, epsilon: f64) -> Self {
        self.epsilon = epsilon;
        self
    }
}

// =============================================================================
// Validation Results
// =============================================================================

/// 単一点でのAAD検証結果。
///
/// # Requirements: 7.6
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointValidation {
    /// Expiry。
    pub expiry: f64,
    /// Tenor。
    pub tenor: f64,
    /// Strike。
    pub strike: f64,
    /// AAD計算によるVega。
    pub aad_vega: f64,
    /// Bump-and-revalueによるVega。
    pub bump_vega: f64,
    /// 絶対誤差。
    pub absolute_error: f64,
    /// 相対誤差。
    pub relative_error: f64,
    /// 検証パス判定。
    pub passed: bool,
}

impl PointValidation {
    /// 新しい検証結果を作成。
    pub fn new(
        expiry: f64,
        tenor: f64,
        strike: f64,
        aad_vega: f64,
        bump_vega: f64,
        tolerance: f64,
    ) -> Self {
        let absolute_error = (aad_vega - bump_vega).abs();
        let relative_error = if bump_vega.abs() > 1e-12 {
            absolute_error / bump_vega.abs()
        } else {
            absolute_error
        };
        let passed = relative_error < tolerance;

        Self {
            expiry,
            tenor,
            strike,
            aad_vega,
            bump_vega,
            absolute_error,
            relative_error,
            passed,
        }
    }
}

/// 検証レポート。
///
/// # Requirements: 7.6
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationReport {
    /// 個別点の検証結果。
    pub point_validations: Vec<PointValidation>,
    /// 総テスト数。
    pub total_tests: usize,
    /// パスしたテスト数。
    pub passed_tests: usize,
    /// 失敗したテスト数。
    pub failed_tests: usize,
    /// 最大相対誤差。
    pub max_relative_error: f64,
    /// 平均相対誤差。
    pub mean_relative_error: f64,
    /// 許容誤差。
    pub tolerance: f64,
    /// 全体のパス判定。
    pub overall_passed: bool,
}

impl ValidationReport {
    /// 新しい空のレポートを作成。
    pub fn new(tolerance: f64) -> Self {
        Self {
            tolerance,
            ..Default::default()
        }
    }

    /// 点検証結果を追加。
    pub fn add_point(&mut self, validation: PointValidation) {
        self.total_tests += 1;
        if validation.passed {
            self.passed_tests += 1;
        } else {
            self.failed_tests += 1;
        }
        self.max_relative_error = self.max_relative_error.max(validation.relative_error);
        self.point_validations.push(validation);
    }

    /// レポートを完成させる。
    pub fn finalize(&mut self) {
        if self.total_tests > 0 {
            self.mean_relative_error = self.point_validations
                .iter()
                .map(|p| p.relative_error)
                .sum::<f64>() / self.total_tests as f64;
        }
        self.overall_passed = self.failed_tests == 0;
    }

    /// 検証がパスしたかどうか。
    pub fn is_valid(&self) -> bool {
        self.overall_passed
    }

    /// パス率を取得。
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests > 0 {
            self.passed_tests as f64 / self.total_tests as f64
        } else {
            1.0
        }
    }

    /// 失敗した点を取得。
    pub fn failed_points(&self) -> Vec<&PointValidation> {
        self.point_validations.iter().filter(|p| !p.passed).collect()
    }
}

// =============================================================================
// AAD Cross Validator
// =============================================================================

/// AADクロス検証器。
///
/// # Requirements: 7.6
///
/// AAD計算とbump-and-revalueの結果を比較し、
/// 許容誤差範囲内で一致することを検証する。
#[derive(Debug, Clone)]
pub struct AADCrossValidator {
    /// 許容相対誤差。
    tolerance: f64,
    /// バンプサイズ。
    bump_size: f64,
    /// 中心差分を使用するか。
    use_central_difference: bool,
    /// 平滑化設定。
    smoothing_config: SmoothingConfig,
}

impl AADCrossValidator {
    /// 新しい検証器を作成。
    pub fn new() -> Self {
        Self {
            tolerance: 1e-4,
            bump_size: 1e-4, // 1bp
            use_central_difference: true,
            smoothing_config: SmoothingConfig::default(),
        }
    }

    /// 許容誤差を設定。
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// バンプサイズを設定。
    pub fn with_bump_size(mut self, bump: f64) -> Self {
        self.bump_size = bump;
        self
    }

    /// 片側差分を使用。
    pub fn with_one_sided_difference(mut self) -> Self {
        self.use_central_difference = false;
        self
    }

    /// 平滑化設定を設定。
    pub fn with_smoothing(mut self, config: SmoothingConfig) -> Self {
        self.smoothing_config = config;
        self
    }

    /// 許容誤差を取得。
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }

    /// 単一点でのVegaを検証。
    ///
    /// # Arguments
    ///
    /// * `base_vol` - 基準ボラティリティ
    /// * `pricing_fn` - 価格計算関数 f(vol) -> price
    /// * `aad_vega` - AAD計算によるVega（既知の場合）
    ///
    /// # Returns
    ///
    /// 検証結果。
    pub fn validate_point<F>(
        &self,
        expiry: f64,
        tenor: f64,
        strike: f64,
        base_vol: f64,
        pricing_fn: F,
        aad_vega: f64,
    ) -> PointValidation
    where
        F: Fn(f64) -> f64,
    {
        // Bump-and-revalue計算
        let bump_vega = if self.use_central_difference {
            let price_up = pricing_fn(base_vol + self.bump_size);
            let price_down = pricing_fn((base_vol - self.bump_size).max(1e-10));
            (price_up - price_down) / (2.0 * self.bump_size)
        } else {
            let price_base = pricing_fn(base_vol);
            let price_up = pricing_fn(base_vol + self.bump_size);
            (price_up - price_base) / self.bump_size
        };

        PointValidation::new(expiry, tenor, strike, aad_vega, bump_vega, self.tolerance)
    }

    /// グリッド全体を検証。
    ///
    /// # Arguments
    ///
    /// * `points` - 検証点リスト [(expiry, tenor, strike, base_vol, aad_vega), ...]
    /// * `pricing_fn` - 価格計算関数
    ///
    /// # Returns
    ///
    /// 検証レポート。
    pub fn validate_grid<F>(
        &self,
        points: &[(f64, f64, f64, f64, f64)],
        pricing_fn: F,
    ) -> ValidationReport
    where
        F: Fn(f64) -> f64,
    {
        let mut report = ValidationReport::new(self.tolerance);

        for &(expiry, tenor, strike, base_vol, aad_vega) in points {
            let validation = self.validate_point(
                expiry, tenor, strike, base_vol, &pricing_fn, aad_vega
            );
            report.add_point(validation);
        }

        report.finalize();
        report
    }

    /// 数値微分でVegaを計算（AAD結果がない場合のフォールバック）。
    pub fn compute_bump_vega<F>(&self, base_vol: f64, pricing_fn: F) -> f64
    where
        F: Fn(f64) -> f64,
    {
        if self.use_central_difference {
            let price_up = pricing_fn(base_vol + self.bump_size);
            let price_down = pricing_fn((base_vol - self.bump_size).max(1e-10));
            (price_up - price_down) / (2.0 * self.bump_size)
        } else {
            let price_base = pricing_fn(base_vol);
            let price_up = pricing_fn(base_vol + self.bump_size);
            (price_up - price_base) / self.bump_size
        }
    }
}

impl Default for AADCrossValidator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Discontinuity Detector
// =============================================================================

/// 不連続点検出器。
///
/// # Requirements: 7.7
///
/// 微分が不連続点を通過する可能性がある箇所を検出し、
/// smooth approximationの適用を推奨する。
#[derive(Debug, Clone, Default)]
pub struct DiscontinuityDetector {
    /// 検出された不連続点。
    discontinuities: Vec<DiscontinuityPoint>,
    /// 検出閾値。
    threshold: f64,
}

/// 不連続点情報。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscontinuityPoint {
    /// 不連続点のタイプ。
    pub kind: DiscontinuityKind,
    /// 場所（Expiry, Tenor, Strike）。
    pub location: (f64, f64, f64),
    /// 重大度（0.0-1.0）。
    pub severity: f64,
    /// 推奨される平滑化関数。
    pub recommended_smoothing: &'static str,
}

/// 不連続点の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscontinuityKind {
    /// ATM付近のmax/min操作。
    AtmPayoff,
    /// バリアクロッシング。
    BarrierCrossing,
    /// Digital payoff。
    DigitalPayoff,
    /// Knockin/Knockout条件。
    KnockCondition,
    /// その他。
    Other,
}

impl DiscontinuityDetector {
    /// 新しい検出器を作成。
    pub fn new() -> Self {
        Self {
            discontinuities: Vec::new(),
            threshold: 0.01,
        }
    }

    /// 検出閾値を設定。
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// ATM不連続点を検出。
    ///
    /// # Arguments
    ///
    /// * `forward` - フォワードレート
    /// * `strike` - ストライク
    /// * `expiry` - 満期
    /// * `tenor` - テナー
    pub fn detect_atm(
        &mut self,
        forward: f64,
        strike: f64,
        expiry: f64,
        tenor: f64,
    ) {
        let moneyness = (strike / forward - 1.0).abs();
        if moneyness < self.threshold {
            self.discontinuities.push(DiscontinuityPoint {
                kind: DiscontinuityKind::AtmPayoff,
                location: (expiry, tenor, strike),
                severity: 1.0 - moneyness / self.threshold,
                recommended_smoothing: "smooth_max / smooth_abs",
            });
        }
    }

    /// Digital payoff不連続点を検出。
    pub fn detect_digital(
        &mut self,
        barrier: f64,
        spot: f64,
        expiry: f64,
        tenor: f64,
    ) {
        let distance = (spot / barrier - 1.0).abs();
        if distance < self.threshold {
            self.discontinuities.push(DiscontinuityPoint {
                kind: DiscontinuityKind::DigitalPayoff,
                location: (expiry, tenor, barrier),
                severity: 1.0 - distance / self.threshold,
                recommended_smoothing: "smooth_indicator",
            });
        }
    }

    /// 検出された不連続点を取得。
    pub fn discontinuities(&self) -> &[DiscontinuityPoint] {
        &self.discontinuities
    }

    /// 不連続点が検出されたかどうか。
    pub fn has_discontinuities(&self) -> bool {
        !self.discontinuities.is_empty()
    }

    /// クリア。
    pub fn clear(&mut self) {
        self.discontinuities.clear();
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoothing_config_default() {
        let config = SmoothingConfig::default();
        assert!((config.epsilon - 1e-6).abs() < 1e-10);
        assert!(config.use_smooth_max);
        assert!(config.use_smooth_min);
    }

    #[test]
    fn test_smoothing_config_disabled() {
        let config = SmoothingConfig::disabled();
        assert!((config.epsilon - 0.0).abs() < 1e-10);
        assert!(!config.use_smooth_max);
        assert!(!config.use_smooth_min);
    }

    #[test]
    fn test_point_validation_creation() {
        let validation = PointValidation::new(1.0, 5.0, 0.03, 100.0, 99.99, 0.01);

        assert!((validation.expiry - 1.0).abs() < 1e-10);
        assert!((validation.aad_vega - 100.0).abs() < 1e-10);
        assert!((validation.bump_vega - 99.99).abs() < 1e-10);
        assert!(validation.passed); // 0.01% error < 1% tolerance
    }

    #[test]
    fn test_point_validation_failure() {
        let validation = PointValidation::new(1.0, 5.0, 0.03, 100.0, 90.0, 0.01);

        assert!(!validation.passed); // 10% error > 1% tolerance
        assert!((validation.relative_error - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_validation_report() {
        let mut report = ValidationReport::new(0.01);

        report.add_point(PointValidation::new(1.0, 5.0, 0.03, 100.0, 99.99, 0.01));
        report.add_point(PointValidation::new(2.0, 5.0, 0.03, 200.0, 199.98, 0.01));
        report.add_point(PointValidation::new(3.0, 5.0, 0.03, 300.0, 250.0, 0.01));
        report.finalize();

        assert_eq!(report.total_tests, 3);
        assert_eq!(report.passed_tests, 2);
        assert_eq!(report.failed_tests, 1);
        assert!(!report.is_valid());
        assert!((report.pass_rate() - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_aad_cross_validator_creation() {
        let validator = AADCrossValidator::new();
        assert!((validator.tolerance() - 1e-4).abs() < 1e-10);
    }

    #[test]
    fn test_aad_cross_validator_linear_function() {
        let validator = AADCrossValidator::new().with_tolerance(0.01);

        // Linear function: price = vol * 100
        let pricing_fn = |vol: f64| vol * 100.0;
        let aad_vega = 100.0; // Exact derivative

        let validation = validator.validate_point(1.0, 5.0, 0.03, 0.20, pricing_fn, aad_vega);

        assert!(validation.passed);
        assert!((validation.bump_vega - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_aad_cross_validator_quadratic_function() {
        let validator = AADCrossValidator::new().with_tolerance(0.01);

        // Quadratic function: price = vol^2 * 100
        let pricing_fn = |vol: f64| vol * vol * 100.0;
        let base_vol = 0.20;
        // Exact derivative: 2 * vol * 100 = 40
        let aad_vega = 2.0 * base_vol * 100.0;

        let validation = validator.validate_point(1.0, 5.0, 0.03, base_vol, pricing_fn, aad_vega);

        assert!(validation.passed);
    }

    #[test]
    fn test_aad_cross_validator_grid() {
        let validator = AADCrossValidator::new().with_tolerance(0.01);

        let pricing_fn = |vol: f64| vol * 100.0;
        let points = vec![
            (1.0, 5.0, 0.03, 0.20, 100.0),
            (2.0, 5.0, 0.03, 0.25, 100.0),
            (1.0, 10.0, 0.03, 0.22, 100.0),
        ];

        let report = validator.validate_grid(&points, pricing_fn);

        assert_eq!(report.total_tests, 3);
        assert!(report.is_valid());
    }

    #[test]
    fn test_discontinuity_detector_atm() {
        let mut detector = DiscontinuityDetector::new().with_threshold(0.02);

        // Strike very close to forward (1% away)
        detector.detect_atm(0.03, 0.0303, 1.0, 5.0);

        assert!(detector.has_discontinuities());
        assert_eq!(detector.discontinuities().len(), 1);
        assert_eq!(detector.discontinuities()[0].kind, DiscontinuityKind::AtmPayoff);
    }

    #[test]
    fn test_discontinuity_detector_no_discontinuity() {
        let mut detector = DiscontinuityDetector::new().with_threshold(0.02);

        // Strike far from forward (10% away)
        detector.detect_atm(0.03, 0.033, 1.0, 5.0);

        assert!(!detector.has_discontinuities());
    }

    #[test]
    fn test_discontinuity_detector_digital() {
        let mut detector = DiscontinuityDetector::new().with_threshold(0.02);

        // Spot close to barrier
        detector.detect_digital(100.0, 100.5, 1.0, 5.0);

        assert!(detector.has_discontinuities());
        assert_eq!(detector.discontinuities()[0].kind, DiscontinuityKind::DigitalPayoff);
    }

    #[test]
    fn test_compute_bump_vega() {
        let validator = AADCrossValidator::new();

        let pricing_fn = |vol: f64| vol * vol * 100.0;
        let vega = validator.compute_bump_vega(0.20, pricing_fn);

        // For vol^2 * 100, derivative = 2 * vol * 100 = 40 at vol=0.20
        assert!((vega - 40.0).abs() < 1.0);
    }

    #[test]
    fn test_failed_points() {
        let mut report = ValidationReport::new(0.01);

        report.add_point(PointValidation::new(1.0, 5.0, 0.03, 100.0, 99.99, 0.01));
        report.add_point(PointValidation::new(2.0, 5.0, 0.03, 200.0, 150.0, 0.01));
        report.finalize();

        let failed = report.failed_points();
        assert_eq!(failed.len(), 1);
        assert!((failed[0].expiry - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_discontinuity_detector_clear() {
        let mut detector = DiscontinuityDetector::new();
        detector.detect_atm(0.03, 0.0301, 1.0, 5.0);

        assert!(detector.has_discontinuities());

        detector.clear();
        assert!(!detector.has_discontinuities());
    }
}
