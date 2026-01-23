//! VolCube固有のエラー型。
//!
//! # Requirements: 7.1-7.5
//!
//! このモジュールはVolCubeカリブレーションと操作に関する
//! 構造化エラーを定義する。

use thiserror::Error;

use crate::market::error::MarketDataError;
use crate::market::CalibrationError;

/// VolCubeカリブレーション診断情報。
///
/// # Requirements: 7.5
///
/// カリブレーション結果に付随する診断情報を保持する。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CalibrationDiagnostics {
    /// 実行した反復回数。
    pub iterations: usize,
    /// 残差（各expiry-tenorスライスの残差ベクトル）。
    pub residuals: Vec<f64>,
    /// 最終パラメータ値。
    pub parameter_values: Vec<f64>,
    /// カリブレーションに使用したスライス数。
    pub slice_count: usize,
    /// 収束したスライス数。
    pub converged_slices: usize,
}

impl CalibrationDiagnostics {
    /// 新しい診断情報を作成。
    pub fn new() -> Self {
        Self::default()
    }

    /// 反復回数を設定。
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// 残差を設定。
    pub fn with_residuals(mut self, residuals: Vec<f64>) -> Self {
        self.residuals = residuals;
        self
    }

    /// パラメータ値を設定。
    pub fn with_parameter_values(mut self, values: Vec<f64>) -> Self {
        self.parameter_values = values;
        self
    }

    /// スライス数を設定。
    pub fn with_slice_count(mut self, count: usize) -> Self {
        self.slice_count = count;
        self
    }

    /// 収束スライス数を設定。
    pub fn with_converged_slices(mut self, count: usize) -> Self {
        self.converged_slices = count;
        self
    }

    /// 総残差（二乗和の平方根）を計算。
    pub fn total_residual(&self) -> f64 {
        self.residuals.iter().map(|r| r * r).sum::<f64>().sqrt()
    }

    /// 収束率を計算。
    pub fn convergence_rate(&self) -> f64 {
        if self.slice_count == 0 {
            0.0
        } else {
            self.converged_slices as f64 / self.slice_count as f64
        }
    }
}

/// VolCube操作エラー。
///
/// # Requirements: 7.1-7.4
///
/// VolCubeのカリブレーションと使用に関する包括的なエラー型。
/// `thiserror`を使用して構造化エラーを提供する。
#[derive(Error, Debug, Clone)]
pub enum VolCubeError {
    /// カリブレーションが収束しなかった。
    ///
    /// # Requirements: 7.1
    #[error(
        "カリブレーションが収束しませんでした (iterations: {iterations}, residual: {residual:.6e})"
    )]
    NotConverged {
        /// 実行した反復回数。
        iterations: usize,
        /// 最終残差。
        residual: f64,
        /// パラメータ値（デバッグ用）。
        params: Vec<f64>,
    },

    /// 入力データ不足。
    ///
    /// # Requirements: 7.2
    #[error("入力データ不足: {got} instruments (最低 {need} 必要)")]
    InsufficientData {
        /// 提供されたinstrument数。
        got: usize,
        /// 必要な最低数。
        need: usize,
    },

    /// 入力データ不正。
    ///
    /// # Requirements: 7.2
    #[error("入力データ不正: {message}")]
    InvalidInput {
        /// エラー詳細。
        message: String,
    },

    /// Arbitrage-free条件違反。
    ///
    /// # Requirements: 7.3
    #[error(
        "Arbitrage-free条件違反: {condition} (expiry={expiry:.4}, strike={strike:.4})"
    )]
    ArbitrageFreeViolation {
        /// 違反した条件の説明。
        condition: String,
        /// 違反が検出されたexpiry。
        expiry: f64,
        /// 違反が検出されたstrike。
        strike: f64,
    },

    /// 市場データエラー（既存MarketDataErrorのラップ）。
    #[error("市場データエラー: {0}")]
    MarketData(#[from] MarketDataError),

    /// カリブレーションエラー（既存CalibrationErrorのラップ）。
    #[error("カリブレーションエラー: {0}")]
    Calibration(#[from] CalibrationError),

    /// キャッシュエラー。
    #[error("キャッシュエラー: {message}")]
    CacheError {
        /// エラー詳細。
        message: String,
    },

    /// 数値不安定性。
    #[error("数値不安定性: {message}")]
    NumericalInstability {
        /// エラー詳細。
        message: String,
    },
}

impl VolCubeError {
    /// 収束失敗エラーを作成。
    pub fn not_converged(iterations: usize, residual: f64, params: Vec<f64>) -> Self {
        VolCubeError::NotConverged {
            iterations,
            residual,
            params,
        }
    }

    /// 入力データ不足エラーを作成。
    pub fn insufficient_data(got: usize, need: usize) -> Self {
        VolCubeError::InsufficientData { got, need }
    }

    /// 入力不正エラーを作成。
    pub fn invalid_input(message: impl Into<String>) -> Self {
        VolCubeError::InvalidInput {
            message: message.into(),
        }
    }

    /// Arbitrage-free違反エラーを作成。
    pub fn arbitrage_violation(condition: impl Into<String>, expiry: f64, strike: f64) -> Self {
        VolCubeError::ArbitrageFreeViolation {
            condition: condition.into(),
            expiry,
            strike,
        }
    }

    /// キャッシュエラーを作成。
    pub fn cache_error(message: impl Into<String>) -> Self {
        VolCubeError::CacheError {
            message: message.into(),
        }
    }

    /// 数値不安定性エラーを作成。
    pub fn numerical_instability(message: impl Into<String>) -> Self {
        VolCubeError::NumericalInstability {
            message: message.into(),
        }
    }

    /// 診断情報付きの収束失敗を返す。
    pub fn not_converged_with_diagnostics(
        diagnostics: &CalibrationDiagnostics,
    ) -> Self {
        VolCubeError::NotConverged {
            iterations: diagnostics.iterations,
            residual: diagnostics.total_residual(),
            params: diagnostics.parameter_values.clone(),
        }
    }

    /// エラーが回復可能かどうかを判定。
    ///
    /// 回復可能なエラーは、異なる初期パラメータや設定で
    /// 再試行することで成功する可能性がある。
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            VolCubeError::NotConverged { .. } | VolCubeError::NumericalInstability { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // CalibrationDiagnostics Tests
    // =========================================================================

    #[test]
    fn test_diagnostics_default() {
        let diag = CalibrationDiagnostics::default();
        assert_eq!(diag.iterations, 0);
        assert!(diag.residuals.is_empty());
        assert!(diag.parameter_values.is_empty());
        assert_eq!(diag.slice_count, 0);
        assert_eq!(diag.converged_slices, 0);
    }

    #[test]
    fn test_diagnostics_new() {
        let diag = CalibrationDiagnostics::new();
        assert_eq!(diag, CalibrationDiagnostics::default());
    }

    #[test]
    fn test_diagnostics_builder() {
        let diag = CalibrationDiagnostics::new()
            .with_iterations(100)
            .with_residuals(vec![0.001, 0.002, 0.003])
            .with_parameter_values(vec![0.04, 0.5, -0.3, 0.4])
            .with_slice_count(10)
            .with_converged_slices(8);

        assert_eq!(diag.iterations, 100);
        assert_eq!(diag.residuals.len(), 3);
        assert_eq!(diag.parameter_values.len(), 4);
        assert_eq!(diag.slice_count, 10);
        assert_eq!(diag.converged_slices, 8);
    }

    #[test]
    fn test_diagnostics_total_residual() {
        let diag = CalibrationDiagnostics::new()
            .with_residuals(vec![3.0, 4.0]);
        let total = diag.total_residual();
        assert!((total - 5.0).abs() < 1e-10); // sqrt(9 + 16) = 5
    }

    #[test]
    fn test_diagnostics_total_residual_empty() {
        let diag = CalibrationDiagnostics::new();
        let total = diag.total_residual();
        assert_eq!(total, 0.0);
    }

    #[test]
    fn test_diagnostics_convergence_rate() {
        let diag = CalibrationDiagnostics::new()
            .with_slice_count(10)
            .with_converged_slices(8);
        let rate = diag.convergence_rate();
        assert!((rate - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_diagnostics_convergence_rate_zero_slices() {
        let diag = CalibrationDiagnostics::new();
        let rate = diag.convergence_rate();
        assert_eq!(rate, 0.0);
    }

    // =========================================================================
    // VolCubeError Tests
    // =========================================================================

    #[test]
    fn test_error_not_converged() {
        let err = VolCubeError::not_converged(100, 0.001, vec![0.04, -0.3, 0.4]);
        let msg = format!("{}", err);
        assert!(msg.contains("100"));
        assert!(msg.contains("収束"));
    }

    #[test]
    fn test_error_insufficient_data() {
        let err = VolCubeError::insufficient_data(3, 10);
        let msg = format!("{}", err);
        assert!(msg.contains("3"));
        assert!(msg.contains("10"));
        assert!(msg.contains("不足"));
    }

    #[test]
    fn test_error_invalid_input() {
        let err = VolCubeError::invalid_input("negative volatility");
        let msg = format!("{}", err);
        assert!(msg.contains("negative volatility"));
        assert!(msg.contains("不正"));
    }

    #[test]
    fn test_error_arbitrage_violation() {
        let err = VolCubeError::arbitrage_violation("Butterfly spread negative", 1.0, 0.03);
        let msg = format!("{}", err);
        assert!(msg.contains("Butterfly"));
        assert!(msg.contains("1.0"));
        assert!(msg.contains("0.03"));
        assert!(msg.contains("Arbitrage"));
    }

    #[test]
    fn test_error_cache_error() {
        let err = VolCubeError::cache_error("cache full");
        let msg = format!("{}", err);
        assert!(msg.contains("cache full"));
        assert!(msg.contains("キャッシュ"));
    }

    #[test]
    fn test_error_numerical_instability() {
        let err = VolCubeError::numerical_instability("NaN in Hagan formula");
        let msg = format!("{}", err);
        assert!(msg.contains("NaN"));
        assert!(msg.contains("数値不安定性"));
    }

    #[test]
    fn test_error_from_market_data() {
        let mkt_err = MarketDataError::InvalidStrike { strike: -1.0 };
        let err: VolCubeError = mkt_err.into();
        assert!(matches!(err, VolCubeError::MarketData(_)));
    }

    #[test]
    fn test_error_from_calibration() {
        let calib_err = CalibrationError::convergence_failure(50, 0.01);
        let err: VolCubeError = calib_err.into();
        assert!(matches!(err, VolCubeError::Calibration(_)));
    }

    #[test]
    fn test_error_is_recoverable() {
        assert!(VolCubeError::not_converged(100, 0.001, vec![]).is_recoverable());
        assert!(VolCubeError::numerical_instability("test").is_recoverable());
        assert!(!VolCubeError::insufficient_data(1, 10).is_recoverable());
        assert!(!VolCubeError::invalid_input("test").is_recoverable());
        assert!(!VolCubeError::arbitrage_violation("test", 1.0, 1.0).is_recoverable());
    }

    #[test]
    fn test_error_not_converged_with_diagnostics() {
        let diag = CalibrationDiagnostics::new()
            .with_iterations(50)
            .with_residuals(vec![0.001, 0.002])
            .with_parameter_values(vec![0.04, -0.3, 0.4]);

        let err = VolCubeError::not_converged_with_diagnostics(&diag);

        match err {
            VolCubeError::NotConverged { iterations, params, .. } => {
                assert_eq!(iterations, 50);
                assert_eq!(params.len(), 3);
            }
            _ => panic!("Expected NotConverged variant"),
        }
    }

    #[test]
    fn test_error_debug() {
        let err = VolCubeError::insufficient_data(5, 10);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InsufficientData"));
    }

    #[test]
    fn test_error_clone() {
        let err = VolCubeError::not_converged(100, 0.001, vec![0.04]);
        let cloned = err.clone();
        assert!(matches!(cloned, VolCubeError::NotConverged { .. }));
    }
}
