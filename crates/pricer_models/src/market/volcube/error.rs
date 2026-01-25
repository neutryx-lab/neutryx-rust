//! VolCube固有のエラー型。
//!
//! # Requirements: 7.1-7.5, 4.4, 4.5, 4.7
//!
//! このモジュールはVolCubeカリブレーションと操作に関する
//! 構造化エラーを定義する。

use thiserror::Error;

use crate::market::{error::MarketDataError, CalibrationError};

/// スライスの収束状態。
///
/// # Requirements: 4.4, 4.5
///
/// 各expiry-tenorスライスのカリブレーション結果を表す。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConvergenceStatus {
    /// カリブレーション成功。
    #[default]
    Success,
    /// カリブレーション失敗（収束しなかった）。
    Failed,
    /// 収束したが警告あり（パラメータ境界付近など）。
    Warning,
}

/// SABRパラメータの種類。
///
/// # Requirements: 4.7
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SabrParameter {
    /// α（ボラティリティレベル）。
    Alpha,
    /// β（CEV指数）。
    Beta,
    /// ρ（相関）。
    Rho,
    /// ν（ボラティリティのボラティリティ）。
    Nu,
}

impl std::fmt::Display for SabrParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SabrParameter::Alpha => write!(f, "α"),
            SabrParameter::Beta => write!(f, "β"),
            SabrParameter::Rho => write!(f, "ρ"),
            SabrParameter::Nu => write!(f, "ν"),
        }
    }
}

/// パラメータ境界違反情報。
///
/// # Requirements: 4.7
///
/// カリブレーション中にパラメータが境界に達した場合の詳細情報。
#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryViolation {
    /// 違反したパラメータ。
    pub parameter: SabrParameter,
    /// カリブレーション後の値。
    pub value: f64,
    /// 許容範囲の下限。
    pub lower_bound: f64,
    /// 許容範囲の上限。
    pub upper_bound: f64,
    /// 警告メッセージ。
    pub message: String,
}

impl BoundaryViolation {
    /// 新しい境界違反情報を作成。
    pub fn new(
        parameter: SabrParameter,
        value: f64,
        lower_bound: f64,
        upper_bound: f64,
    ) -> Self {
        let message = if value <= lower_bound {
            format!(
                "{} が下限 {} に達しました (値: {})",
                parameter, lower_bound, value
            )
        } else if value >= upper_bound {
            format!(
                "{} が上限 {} に達しました (値: {})",
                parameter, upper_bound, value
            )
        } else {
            format!(
                "{} が境界付近です (値: {}, 範囲: [{}, {}])",
                parameter, value, lower_bound, upper_bound
            )
        };

        Self {
            parameter,
            value,
            lower_bound,
            upper_bound,
            message,
        }
    }

    /// パラメータが実際に境界を超えているか。
    pub fn is_violated(&self) -> bool {
        self.value < self.lower_bound || self.value > self.upper_bound
    }

    /// パラメータが境界付近（境界から10%以内）か。
    pub fn is_near_boundary(&self) -> bool {
        let range = self.upper_bound - self.lower_bound;
        let threshold = range * 0.1;
        (self.value - self.lower_bound) < threshold || (self.upper_bound - self.value) < threshold
    }
}

/// スライス別診断情報。
///
/// # Requirements: 4.4, 4.5, 4.7
///
/// 個々のexpiry-tenorスライスのカリブレーション結果を保持する。
#[derive(Debug, Clone, PartialEq)]
pub struct SliceDiagnostics {
    /// Expiry（年単位）。
    pub expiry: f64,
    /// Tenor（年単位）。
    pub tenor: f64,
    /// 収束状態。
    pub status: ConvergenceStatus,
    /// 反復回数。
    pub iterations: usize,
    /// 最終残差（RMSE）。
    pub final_residual: f64,
    /// カリブレーションされたSABRパラメータ [α, β, ρ, ν]。
    pub parameters: [f64; 4],
    /// Forward rate（このスライスで使用）。
    pub forward: f64,
    /// パラメータ境界違反情報（ある場合）。
    pub boundary_violations: Vec<BoundaryViolation>,
}

impl SliceDiagnostics {
    /// 新しいスライス診断情報を作成。
    pub fn new(expiry: f64, tenor: f64) -> Self {
        Self {
            expiry,
            tenor,
            status: ConvergenceStatus::default(),
            iterations: 0,
            final_residual: 0.0,
            parameters: [0.0; 4],
            forward: 0.0,
            boundary_violations: Vec::new(),
        }
    }

    /// 収束状態を設定。
    pub fn with_status(mut self, status: ConvergenceStatus) -> Self {
        self.status = status;
        self
    }

    /// 反復回数を設定。
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// 最終残差を設定。
    pub fn with_residual(mut self, residual: f64) -> Self {
        self.final_residual = residual;
        self
    }

    /// SABRパラメータを設定。
    pub fn with_parameters(mut self, alpha: f64, beta: f64, rho: f64, nu: f64) -> Self {
        self.parameters = [alpha, beta, rho, nu];
        self
    }

    /// Forward rateを設定。
    pub fn with_forward(mut self, forward: f64) -> Self {
        self.forward = forward;
        self
    }

    /// 境界違反を追加。
    pub fn add_boundary_violation(&mut self, violation: BoundaryViolation) {
        self.boundary_violations.push(violation);
        // 境界違反があれば警告状態に
        if self.status == ConvergenceStatus::Success {
            self.status = ConvergenceStatus::Warning;
        }
    }

    /// このスライスが成功したか。
    pub fn is_success(&self) -> bool {
        self.status == ConvergenceStatus::Success
    }

    /// このスライスに警告があるか。
    pub fn has_warnings(&self) -> bool {
        !self.boundary_violations.is_empty() || self.status == ConvergenceStatus::Warning
    }
}

/// VolCubeカリブレーション診断情報。
///
/// # Requirements: 7.5, 4.4, 4.5, 4.7
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
    /// スライス別診断情報。
    ///
    /// # Requirements: 4.4, 4.5
    pub slice_diagnostics: Vec<SliceDiagnostics>,
    /// 全体の収束状態。
    ///
    /// # Requirements: 4.4
    pub overall_status: ConvergenceStatus,
}

impl CalibrationDiagnostics {
    /// 新しい診断情報を作成。
    pub fn new() -> Self { Self::default() }

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

    /// スライス別診断情報を追加。
    ///
    /// # Requirements: 4.4, 4.5
    pub fn add_slice_diagnostics(&mut self, slice: SliceDiagnostics) {
        // 全体のステータスを更新
        match slice.status {
            ConvergenceStatus::Failed => {
                self.overall_status = ConvergenceStatus::Failed;
            }
            ConvergenceStatus::Warning => {
                if self.overall_status != ConvergenceStatus::Failed {
                    self.overall_status = ConvergenceStatus::Warning;
                }
            }
            ConvergenceStatus::Success => {}
        }
        self.slice_diagnostics.push(slice);
    }

    /// スライス別診断情報を設定。
    pub fn with_slice_diagnostics(mut self, slices: Vec<SliceDiagnostics>) -> Self {
        for slice in slices {
            self.add_slice_diagnostics(slice);
        }
        self
    }

    /// 全体の収束状態を設定。
    pub fn with_overall_status(mut self, status: ConvergenceStatus) -> Self {
        self.overall_status = status;
        self
    }

    /// 総残差（二乗和の平方根）を計算。
    pub fn total_residual(&self) -> f64 { self.residuals.iter().map(|r| r * r).sum::<f64>().sqrt() }

    /// 収束率を計算。
    pub fn convergence_rate(&self) -> f64 {
        if self.slice_count == 0 {
            0.0
        } else {
            self.converged_slices as f64 / self.slice_count as f64
        }
    }

    /// 全てのスライスが成功したか。
    pub fn all_success(&self) -> bool {
        self.overall_status == ConvergenceStatus::Success
    }

    /// 警告があるスライスの数を取得。
    pub fn warning_count(&self) -> usize {
        self.slice_diagnostics
            .iter()
            .filter(|s| s.status == ConvergenceStatus::Warning)
            .count()
    }

    /// 失敗したスライスの数を取得。
    pub fn failed_count(&self) -> usize {
        self.slice_diagnostics
            .iter()
            .filter(|s| s.status == ConvergenceStatus::Failed)
            .count()
    }

    /// 全ての境界違反を取得。
    pub fn all_boundary_violations(&self) -> Vec<&BoundaryViolation> {
        self.slice_diagnostics
            .iter()
            .flat_map(|s| s.boundary_violations.iter())
            .collect()
    }

    /// サマリーレポートを生成。
    ///
    /// # Requirements: 4.4
    pub fn summary_report(&self) -> String {
        let status_str = match self.overall_status {
            ConvergenceStatus::Success => "成功",
            ConvergenceStatus::Warning => "警告あり",
            ConvergenceStatus::Failed => "失敗",
        };

        let mut report = format!(
            "=== カリブレーション診断サマリー ===\n\
             状態: {}\n\
             総スライス数: {}\n\
             成功: {}\n\
             警告: {}\n\
             失敗: {}\n\
             総反復回数: {}\n\
             総残差: {:.6e}\n",
            status_str,
            self.slice_count,
            self.converged_slices,
            self.warning_count(),
            self.failed_count(),
            self.iterations,
            self.total_residual()
        );

        let violations = self.all_boundary_violations();
        if !violations.is_empty() {
            report.push_str("\n--- パラメータ境界違反 ---\n");
            for violation in violations {
                report.push_str(&format!("  - {}\n", violation.message));
            }
        }

        report
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
    #[error("Arbitrage-free条件違反: {condition} (expiry={expiry:.4}, strike={strike:.4})")]
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
    pub fn not_converged_with_diagnostics(diagnostics: &CalibrationDiagnostics) -> Self {
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
        let diag = CalibrationDiagnostics::new().with_residuals(vec![3.0, 4.0]);
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
            VolCubeError::NotConverged {
                iterations, params, ..
            } => {
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
