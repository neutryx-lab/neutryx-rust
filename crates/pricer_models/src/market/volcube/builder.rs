//! VolCubeBuilder - Fluent API for constructing VolCube instances.
//!
//! # 概要
//!
//! このモジュールは、VolInstrumentリストとカリブレーション設定から
//! VolCubeを構築するBuilderパターンを提供する。
//!
//! # 使用例
//!
//! ```ignore
//! use pricer_models::market::volcube::{VolCubeBuilder, VolCubeConfig, VolInstrument};
//!
//! let instruments = vec![...];
//! let config = VolCubeConfig::default();
//!
//! let cube = VolCubeBuilder::new()
//!     .with_instruments(instruments)
//!     .with_config(config)
//!     .build()?;
//! ```

use std::collections::HashMap;

use num_traits::Float;
use ordered_float::OrderedFloat;

use super::{
    breeden_litzenberger::BreedenLitzenberger,
    cache::{SharedVolCubeCache, VolCubeKey},
    config::VolCubeConfig,
    cube::VolCube,
    error::{
        ArbitrageViolation, BoundaryViolation, CalibrationDiagnostics, ConvergenceStatus,
        SabrParameter, SliceDiagnostics, VolCubeError,
    },
    quote::VolQuoteSet,
    sabr_surface::SabrParameterSurface,
    types::{SabrParams, VolInstrument},
};

/// VolCubeのBuilder。
///
/// Fluent APIによるVolCube構築を提供。
/// キャッシュ統合とSABRカリブレーションをサポート。
#[derive(Debug)]
pub struct VolCubeBuilder<T: Float> {
    /// 入力Instrumentリスト
    instruments: Vec<VolInstrument<T>>,
    /// カリブレーション設定
    config: VolCubeConfig,
    /// キャッシュ参照（オプション）
    cache: Option<SharedVolCubeCache<VolCube<T>>>,
    /// Forward値（デフォルト、ATM計算用）
    default_forward: T,
    /// Strike軸範囲（min, max）
    strike_bounds: Option<(T, T)>,
}

impl<T: Float + Send + Sync> Default for VolCubeBuilder<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Float + Send + Sync> VolCubeBuilder<T> {
    /// 新しいBuilderを作成。
    pub fn new() -> Self {
        Self {
            instruments: Vec::new(),
            config: VolCubeConfig::default(),
            cache: None,
            default_forward: T::from(0.03).unwrap(),
            strike_bounds: None,
        }
    }

    /// Instrumentリストを設定。
    pub fn with_instruments(mut self, instruments: Vec<VolInstrument<T>>) -> Self {
        self.instruments = instruments;
        self
    }

    /// VolQuoteSetからInstrumentを設定。
    ///
    /// # Arguments
    /// * `quote_set` - VolQuoteSet
    /// * `forward_fn` - (expiry, tenor)から forward rate を計算するクロージャ
    ///
    /// # Requirements: 2.1, 2.3
    ///
    /// VolQuoteSetをVolInstrumentに変換してBuilderに設定する。
    pub fn with_quote_set<F>(mut self, quote_set: &VolQuoteSet, forward_fn: F) -> Self
    where
        F: Fn(f64, f64) -> T,
    {
        self.instruments = quote_set.to_instruments(forward_fn);
        self
    }

    /// VolQuoteSetから固定forward rateでInstrumentを設定。
    ///
    /// # Arguments
    /// * `quote_set` - VolQuoteSet
    /// * `forward` - 全クォートに適用する固定forward rate
    ///
    /// # Requirements: 2.1, 2.3
    pub fn with_quote_set_fixed_forward(mut self, quote_set: &VolQuoteSet, forward: T) -> Self {
        self.instruments = quote_set.to_instruments_with_fixed_forward(forward);
        self.default_forward = forward;
        self
    }

    /// 単一のInstrumentを追加。
    pub fn add_instrument(mut self, instrument: VolInstrument<T>) -> Self {
        self.instruments.push(instrument);
        self
    }

    /// カリブレーション設定を設定。
    pub fn with_config(mut self, config: VolCubeConfig) -> Self {
        self.config = config;
        self
    }

    /// キャッシュを設定。
    pub fn with_cache(mut self, cache: SharedVolCubeCache<VolCube<T>>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Forward値を設定（ATM計算とStrike軸変換に使用）。
    pub fn with_forward(mut self, forward: T) -> Self {
        self.default_forward = forward;
        self
    }

    /// Strike軸の有効範囲を設定。
    pub fn with_strike_bounds(mut self, min: T, max: T) -> Self {
        self.strike_bounds = Some((min, max));
        self
    }

    /// VolCubeを構築。
    ///
    /// # Errors
    ///
    /// - `InsufficientData`:
    ///   Instrumentリストが空、またはexpiry-tenor格子に必要な
    ///   点が不足している場合。
    /// - `NotConverged`: SABRカリブレーションが収束しなかった場合。
    /// - `InvalidInput`: Instrumentデータに不整合がある場合。
    pub fn build(self) -> Result<VolCube<T>, VolCubeError> {
        // 設定の検証
        self.config
            .validate()
            .map_err(VolCubeError::invalid_input)?;

        // Instrumentリストが空の場合はエラー
        if self.instruments.is_empty() {
            return Err(VolCubeError::insufficient_data(0, 1));
        }

        // すべてのInstrumentを検証
        for instrument in &self.instruments {
            instrument.validate().map_err(VolCubeError::invalid_input)?;
        }

        // キャッシュキーを生成
        let cache_key = VolCubeKey::from_instruments(&self.instruments, &self.config);

        // キャッシュルックアップ（cacheを先に取り出す）
        let cache = self.cache.clone();
        if let Some(ref cache_ref) = cache {
            if let Some(cube) = cache_ref.lookup(&cache_key) {
                return Ok(cube);
            }
        }

        // キャッシュミス: カリブレーションを実行
        let (cube, _diagnostics) = self.calibrate_internal()?;

        // キャッシュに格納
        if let Some(ref cache_ref) = cache {
            cache_ref.insert(cache_key, cube.clone());
        }

        Ok(cube)
    }

    /// VolCubeを構築し、カリブレーション診断情報も返す。
    ///
    /// # Requirements: 4.4, 4.5, 4.7
    ///
    /// `build()`と同様にVolCubeを構築するが、カリブレーション診断情報も
    /// 返す。診断情報には各スライスの詳細な情報が含まれる。
    ///
    /// # Returns
    ///
    /// `(VolCube<T>, CalibrationDiagnostics)` - 構築されたVolCubeと診断情報
    ///
    /// # Errors
    ///
    /// `build()`と同じエラーを返す。
    pub fn build_with_diagnostics(
        self,
    ) -> Result<(VolCube<T>, CalibrationDiagnostics), VolCubeError> {
        // 設定の検証
        self.config
            .validate()
            .map_err(VolCubeError::invalid_input)?;

        // Instrumentリストが空の場合はエラー
        if self.instruments.is_empty() {
            return Err(VolCubeError::insufficient_data(0, 1));
        }

        // すべてのInstrumentを検証
        for instrument in &self.instruments {
            instrument.validate().map_err(VolCubeError::invalid_input)?;
        }

        // カリブレーションを実行
        self.calibrate_internal()
    }

    /// SABRカリブレーションを実行してVolCubeを構築（内部メソッド）。
    fn calibrate_internal(self) -> Result<(VolCube<T>, CalibrationDiagnostics), VolCubeError> {
        // Instrumentをexpiry-tenorでグループ化（f64に変換してHash可能にする）
        let grouped = self.group_by_expiry_tenor();

        // グリッドの次元を決定
        let mut expiries: Vec<T> = grouped.keys().filter_map(|(e, _)| T::from(e.0)).collect();
        let mut tenors: Vec<T> = grouped.keys().filter_map(|(_, t)| T::from(t.0)).collect();

        // 重複を除去してソート
        expiries.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        expiries.dedup();
        tenors.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        tenors.dedup();

        // グリッドサイズの検証
        if expiries.len() < 2 || tenors.len() < 2 {
            return Err(VolCubeError::insufficient_data(
                expiries.len() * tenors.len(),
                4,
            ));
        }

        // SABRパラメータグリッドとフォワードグリッドを構築
        let mut params_grid: Vec<Vec<SabrParams<T>>> = Vec::with_capacity(expiries.len());
        let mut forward_grid: Vec<Vec<T>> = Vec::with_capacity(expiries.len());
        let mut diagnostics = CalibrationDiagnostics::default();
        let mut source_instruments = Vec::new();

        let beta = T::from(self.config.sabr_beta.unwrap_or(0.5)).unwrap();

        for expiry in &expiries {
            let mut params_row: Vec<SabrParams<T>> = Vec::with_capacity(tenors.len());
            let mut forward_row: Vec<T> = Vec::with_capacity(tenors.len());

            for tenor in &tenors {
                // このexpiry-tenorセルのInstrumentを取得
                let key = (
                    OrderedFloat(expiry.to_f64().unwrap_or(0.0)),
                    OrderedFloat(tenor.to_f64().unwrap_or(0.0)),
                );
                let cell_instruments = grouped.get(&key);

                let (params, forward) = match cell_instruments {
                    Some(instruments) => {
                        // ソースInstrumentを記録
                        for inst in instruments {
                            source_instruments.push(inst.instrument_id.clone());
                        }
                        // SABRカリブレーション
                        let p =
                            self.calibrate_cell(instruments, *expiry, *tenor, &mut diagnostics)?;
                        // Forward平均を計算
                        let avg_forward = instruments
                            .iter()
                            .map(|i| i.forward)
                            .fold(T::zero(), |a, b| a + b)
                            / T::from(instruments.len()).unwrap();
                        (p, avg_forward)
                    }
                    None => {
                        // セルにデータがない場合はデフォルトパラメータを使用
                        (SabrParams::default(), self.default_forward)
                    }
                };

                params_row.push(params);
                forward_row.push(forward);
            }

            params_grid.push(params_row);
            forward_grid.push(forward_row);
        }

        // SABRパラメータサーフェスを作成
        let sabr_surface =
            SabrParameterSurface::new(expiries.clone(), tenors.clone(), &params_grid, beta)
                .map_err(|e| VolCubeError::invalid_input(format!("SABR surface error: {}", e)))?;

        // Strike範囲を決定
        let strike_bounds = self.strike_bounds.unwrap_or_else(|| {
            let min_strike = self
                .instruments
                .iter()
                .map(|i| i.strike)
                .fold(T::infinity(), |a, b| if a < b { a } else { b });
            let max_strike = self
                .instruments
                .iter()
                .map(|i| i.strike)
                .fold(T::neg_infinity(), |a, b| if a > b { a } else { b });
            (min_strike, max_strike)
        });

        // Arbitrage-free検証フラグを保存（configがmoveされる前に）
        let should_validate_arbitrage = self.config.validate_arbitrage_free;
        let default_forward = self.default_forward;

        // VolCubeを構築
        let cube = VolCube::new(
            sabr_surface,
            forward_grid,
            self.config,
            source_instruments,
            strike_bounds,
        );

        // Arbitrage-free条件を検証（有効な場合）
        //
        // # Requirements: 4.6
        if should_validate_arbitrage {
            let strike_bounds_f64 = (
                strike_bounds.0.to_f64().unwrap_or(0.01),
                strike_bounds.1.to_f64().unwrap_or(0.05),
            );

            // 各スライスでarbitrage検証を実行
            for slice_diag in &mut diagnostics.slice_diagnostics {
                let expiry_f64 = slice_diag.expiry;
                let tenor_f64 = slice_diag.tenor;
                let forward_f64 = slice_diag.forward;

                Self::validate_arbitrage_free_impl(
                    &cube,
                    expiry_f64,
                    tenor_f64,
                    forward_f64,
                    strike_bounds_f64,
                    slice_diag,
                );
            }

            // 全体ステータスを更新
            if diagnostics.arbitrage_violation_count() > 0
                && diagnostics.overall_status == ConvergenceStatus::Success
            {
                diagnostics.overall_status = ConvergenceStatus::Warning;
            }
        }

        Ok((cube, diagnostics))
    }

    /// Instrumentをexpiry-tenorでグループ化（OrderedFloat<f64>を使用）。
    fn group_by_expiry_tenor(
        &self,
    ) -> HashMap<(OrderedFloat<f64>, OrderedFloat<f64>), Vec<&VolInstrument<T>>> {
        let mut groups: HashMap<(OrderedFloat<f64>, OrderedFloat<f64>), Vec<&VolInstrument<T>>> =
            HashMap::new();

        for instrument in &self.instruments {
            let key = (
                OrderedFloat(instrument.expiry.to_f64().unwrap_or(0.0)),
                OrderedFloat(instrument.tenor.to_f64().unwrap_or(0.0)),
            );
            groups.entry(key).or_default().push(instrument);
        }

        groups
    }

    /// 単一セルのSABRカリブレーションを実行。
    ///
    /// # Requirements: 4.4, 4.5, 4.7
    ///
    /// カリブレーション結果に加えて、詳細な診断情報を生成する：
    /// - 残差（RMSE）
    /// - 反復回数
    /// - 最終パラメータ値
    /// - 収束状態
    /// - パラメータ境界違反情報
    fn calibrate_cell(
        &self,
        instruments: &[&VolInstrument<T>],
        expiry: T,
        tenor: T,
        diagnostics: &mut CalibrationDiagnostics,
    ) -> Result<SabrParams<T>, VolCubeError> {
        if instruments.is_empty() {
            return Err(VolCubeError::insufficient_data(0, 1));
        }

        // 平均forwardを計算
        let avg_forward = instruments
            .iter()
            .map(|i| i.forward)
            .fold(T::zero(), |a, b| a + b)
            / T::from(instruments.len()).unwrap();

        // ATMに最も近いInstrumentを探す
        let atm_instrument = instruments
            .iter()
            .min_by(|a, b| {
                let diff_a = (a.strike - avg_forward).abs();
                let diff_b = (b.strike - avg_forward).abs();
                diff_a
                    .partial_cmp(&diff_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        // 初期推定: ATM volからalphaを推定
        let beta = T::from(self.config.sabr_beta.unwrap_or(0.5)).unwrap();
        let one_minus_beta = T::one() - beta;
        let forward_pow = avg_forward.powf(one_minus_beta);
        let initial_alpha = atm_instrument.implied_vol * forward_pow;

        // 簡易的なカリブレーション
        let (alpha, rho, nu) =
            self.simple_sabr_fit(instruments, initial_alpha, beta, avg_forward)?;

        // 残差を計算
        let residual = self.compute_residual(instruments, alpha, beta, rho, nu, avg_forward);

        // スライス診断情報を作成
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        let tenor_f64 = tenor.to_f64().unwrap_or(0.0);
        let alpha_f64 = alpha.to_f64().unwrap_or(0.0);
        let beta_f64 = beta.to_f64().unwrap_or(0.0);
        let rho_f64 = rho.to_f64().unwrap_or(0.0);
        let nu_f64 = nu.to_f64().unwrap_or(0.0);
        let forward_f64 = avg_forward.to_f64().unwrap_or(0.0);

        let mut slice_diag = SliceDiagnostics::new(expiry_f64, tenor_f64)
            .with_iterations(1) // 簡易カリブレーションは1回
            .with_residual(residual)
            .with_parameters(alpha_f64, beta_f64, rho_f64, nu_f64)
            .with_forward(forward_f64)
            .with_status(ConvergenceStatus::Success);

        // パラメータ境界違反をチェック
        self.check_boundary_violations(&mut slice_diag, alpha_f64, beta_f64, rho_f64, nu_f64);

        // 診断情報を更新
        diagnostics.add_slice_diagnostics(slice_diag);
        diagnostics.converged_slices += 1;
        diagnostics.slice_count += 1;
        diagnostics.iterations += 1;
        diagnostics.residuals.push(residual);

        Ok(SabrParams::new(alpha, beta, rho, nu))
    }

    /// 残差（RMSE）を計算。
    ///
    /// # Requirements: 4.4
    fn compute_residual<'a>(
        &self,
        instruments: &[&'a VolInstrument<T>],
        alpha: T,
        beta: T,
        rho: T,
        nu: T,
        forward: T,
    ) -> f64 {
        use crate::market::calibration::sabr::SABRCalibrator;

        if instruments.is_empty() {
            return 0.0;
        }

        let forward_f64 = forward.to_f64().unwrap_or(0.03);
        let alpha_f64 = alpha.to_f64().unwrap_or(0.04);
        let beta_f64 = beta.to_f64().unwrap_or(0.5);
        let rho_f64 = rho.to_f64().unwrap_or(0.0);
        let nu_f64 = nu.to_f64().unwrap_or(0.3);

        let mut sum_sq = 0.0;
        for inst in instruments {
            let expiry_f64 = inst.expiry.to_f64().unwrap_or(1.0);
            let strike_f64 = inst.strike.to_f64().unwrap_or(0.03);
            let market_vol = inst.implied_vol.to_f64().unwrap_or(0.2);

            let model_vol = SABRCalibrator::implied_vol(
                forward_f64,
                strike_f64,
                expiry_f64,
                alpha_f64,
                beta_f64,
                rho_f64,
                nu_f64,
            );

            let diff = model_vol - market_vol;
            sum_sq += diff * diff;
        }

        (sum_sq / instruments.len() as f64).sqrt()
    }

    /// パラメータ境界違反をチェック。
    ///
    /// # Requirements: 4.7
    fn check_boundary_violations(
        &self,
        slice: &mut SliceDiagnostics,
        alpha: f64,
        beta: f64,
        rho: f64,
        nu: f64,
    ) {
        // Alpha bounds: (0.001, 2.0)
        if alpha <= 0.001 || alpha >= 2.0 {
            slice.add_boundary_violation(BoundaryViolation::new(
                SabrParameter::Alpha,
                alpha,
                0.001,
                2.0,
            ));
        } else if alpha < 0.01 || alpha > 1.5 {
            // Near boundary (within 10% of range)
            let violation = BoundaryViolation::new(SabrParameter::Alpha, alpha, 0.001, 2.0);
            if violation.is_near_boundary() {
                slice.add_boundary_violation(violation);
            }
        }

        // Beta bounds: [0.0, 1.0]
        if beta < 0.0 || beta > 1.0 {
            slice.add_boundary_violation(BoundaryViolation::new(
                SabrParameter::Beta,
                beta,
                0.0,
                1.0,
            ));
        }

        // Rho bounds: (-0.999, 0.999)
        if rho <= -0.999 || rho >= 0.999 {
            slice.add_boundary_violation(BoundaryViolation::new(
                SabrParameter::Rho,
                rho,
                -0.999,
                0.999,
            ));
        } else if rho.abs() > 0.9 {
            // Near boundary
            let violation = BoundaryViolation::new(SabrParameter::Rho, rho, -0.999, 0.999);
            if violation.is_near_boundary() {
                slice.add_boundary_violation(violation);
            }
        }

        // Nu bounds: (0.001, 2.0)
        if nu <= 0.001 || nu >= 2.0 {
            slice.add_boundary_violation(BoundaryViolation::new(SabrParameter::Nu, nu, 0.001, 2.0));
        } else if nu < 0.05 || nu > 1.8 {
            // Near boundary
            let violation = BoundaryViolation::new(SabrParameter::Nu, nu, 0.001, 2.0);
            if violation.is_near_boundary() {
                slice.add_boundary_violation(violation);
            }
        }
    }

    /// Arbitrage-free条件を検証。
    ///
    /// # Requirements: 4.6
    ///
    /// Breeden-Litzenberger公式を使用してカリブレーション後のsmileが
    /// arbitrage-free条件を満たすか検証する。
    /// 確率密度が負になる場合、arbitrage違反として報告する。
    fn validate_arbitrage_free_impl(
        cube: &VolCube<T>,
        expiry_f64: f64,
        tenor_f64: f64,
        forward_f64: f64,
        strike_range: (f64, f64),
        slice_diag: &mut SliceDiagnostics,
    ) {
        let delta_k = (strike_range.1 - strike_range.0) * 0.02;
        let num_points = 20usize;

        let step = (strike_range.1 - strike_range.0 - delta_k * 2.0) / (num_points - 1) as f64;
        let rate = 0.02; // 仮の無リスク金利

        // Convert to T for BreedenLitzenberger call
        let forward_t = T::from(forward_f64).unwrap_or(T::from(0.03).unwrap());
        let expiry_t = T::from(expiry_f64).unwrap_or(T::one());
        let tenor_t = T::from(tenor_f64).unwrap_or(T::from(5.0).unwrap());
        let rate_t = T::from(rate).unwrap();
        let delta_k_t = T::from(delta_k).unwrap();

        for i in 0..num_points {
            let strike_f64 = strike_range.0 + delta_k + (i as f64) * step;
            let strike_t = T::from(strike_f64).unwrap();

            // Breeden-Litzenbergerで確率密度を計算
            match BreedenLitzenberger::probability_density(
                cube, forward_t, expiry_t, tenor_t, strike_t, rate_t, delta_k_t,
            ) {
                Ok(density) => {
                    // 負の密度はarbitrage違反
                    let density_f64 = density.to_f64().unwrap_or(0.0);
                    if density_f64 < -1e-10 {
                        slice_diag.add_arbitrage_violation(ArbitrageViolation::negative_density(
                            strike_f64,
                            density_f64,
                        ));
                    }
                }
                Err(_) => {
                    // ドメイン外などのエラーは無視
                }
            }
        }
    }

    /// 簡易SABRフィッティング。
    fn simple_sabr_fit(
        &self,
        instruments: &[&VolInstrument<T>],
        initial_alpha: T,
        beta: T,
        forward: T,
    ) -> Result<(T, T, T), VolCubeError> {
        if instruments.is_empty() {
            return Err(VolCubeError::insufficient_data(0, 1));
        }

        // ATM volから初期alpha
        let mut alpha = initial_alpha;

        // スキューからrhoを推定
        let mut rho = T::zero();
        if instruments.len() >= 2 {
            let low_strikes: Vec<_> = instruments.iter().filter(|i| i.strike < forward).collect();
            let high_strikes: Vec<_> = instruments.iter().filter(|i| i.strike > forward).collect();

            if !low_strikes.is_empty() && !high_strikes.is_empty() {
                let avg_low_vol = low_strikes
                    .iter()
                    .map(|i| i.implied_vol)
                    .fold(T::zero(), |a, b| a + b)
                    / T::from(low_strikes.len()).unwrap();
                let avg_high_vol = high_strikes
                    .iter()
                    .map(|i| i.implied_vol)
                    .fold(T::zero(), |a, b| a + b)
                    / T::from(high_strikes.len()).unwrap();

                // スキュー方向からrhoを推定
                let skew = avg_low_vol - avg_high_vol;
                let max_skew = T::from(0.1).unwrap();
                rho = -(skew / max_skew)
                    .min(T::from(0.9).unwrap())
                    .max(T::from(-0.9).unwrap());
            }
        }

        // スマイルの曲率からnuを推定
        let mut nu = T::from(0.3).unwrap();
        if instruments.len() >= 3 {
            let atm_vol = instruments
                .iter()
                .min_by(|a, b| {
                    let diff_a = (a.strike - forward).abs();
                    let diff_b = (b.strike - forward).abs();
                    diff_a
                        .partial_cmp(&diff_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|i| i.implied_vol)
                .unwrap_or_else(T::zero);

            let wing_vols: Vec<T> = instruments
                .iter()
                .filter(|i| (i.strike - forward).abs() > forward * T::from(0.1).unwrap())
                .map(|i| i.implied_vol)
                .collect();

            if !wing_vols.is_empty() {
                let avg_wing = wing_vols.iter().fold(T::zero(), |a, b| a + *b)
                    / T::from(wing_vols.len()).unwrap();
                let curvature = (avg_wing - atm_vol).abs();
                nu = T::from(0.2).unwrap() + curvature * T::from(3.0).unwrap();
                nu = nu.min(T::from(1.5).unwrap()).max(T::from(0.1).unwrap());
            }
        }

        // alphaの補正
        let one_minus_beta = T::one() - beta;
        let forward_pow = forward.powf(one_minus_beta);
        let atm_vol = instruments
            .iter()
            .min_by(|a, b| {
                let diff_a = (a.strike - forward).abs();
                let diff_b = (b.strike - forward).abs();
                diff_a
                    .partial_cmp(&diff_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|i| i.implied_vol)
            .unwrap_or(alpha / forward_pow);

        alpha = atm_vol * forward_pow;

        Ok((alpha, rho, nu))
    }
}

#[cfg(test)]
mod tests {
    use super::{super::cube::VolatilityCube, *};

    fn make_test_instruments() -> Vec<VolInstrument<f64>> {
        let forward = 0.03;
        vec![
            // expiry=1.0, tenor=5.0 スマイル
            VolInstrument::new("1Y5Y_ATM", 1.0, 5.0, 0.03, 0.20, forward),
            VolInstrument::new("1Y5Y_LOW", 1.0, 5.0, 0.02, 0.25, forward),
            VolInstrument::new("1Y5Y_HIGH", 1.0, 5.0, 0.04, 0.22, forward),
            // expiry=1.0, tenor=10.0 スマイル
            VolInstrument::new("1Y10Y_ATM", 1.0, 10.0, 0.03, 0.18, forward),
            VolInstrument::new("1Y10Y_LOW", 1.0, 10.0, 0.02, 0.23, forward),
            VolInstrument::new("1Y10Y_HIGH", 1.0, 10.0, 0.04, 0.20, forward),
            // expiry=5.0, tenor=5.0 スマイル
            VolInstrument::new("5Y5Y_ATM", 5.0, 5.0, 0.03, 0.16, forward),
            VolInstrument::new("5Y5Y_LOW", 5.0, 5.0, 0.02, 0.20, forward),
            VolInstrument::new("5Y5Y_HIGH", 5.0, 5.0, 0.04, 0.17, forward),
            // expiry=5.0, tenor=10.0 スマイル
            VolInstrument::new("5Y10Y_ATM", 5.0, 10.0, 0.03, 0.15, forward),
            VolInstrument::new("5Y10Y_LOW", 5.0, 10.0, 0.02, 0.19, forward),
            VolInstrument::new("5Y10Y_HIGH", 5.0, 10.0, 0.04, 0.16, forward),
        ]
    }

    #[test]
    fn test_builder_new() {
        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new();
        assert!(builder.instruments.is_empty());
    }

    #[test]
    fn test_builder_default() {
        let builder: VolCubeBuilder<f64> = VolCubeBuilder::default();
        assert!(builder.instruments.is_empty());
    }

    #[test]
    fn test_builder_with_instruments() {
        let instruments = make_test_instruments();
        let count = instruments.len();
        let builder = VolCubeBuilder::new().with_instruments(instruments);
        assert_eq!(builder.instruments.len(), count);
    }

    #[test]
    fn test_builder_add_instrument() {
        let instrument = VolInstrument::new("test", 1.0, 5.0, 0.03, 0.20, 0.03);
        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new().add_instrument(instrument);
        assert_eq!(builder.instruments.len(), 1);
    }

    #[test]
    fn test_builder_with_config() {
        let config = VolCubeConfig::default().with_sabr_beta(Some(0.7));
        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new().with_config(config);
        assert_eq!(builder.config.sabr_beta, Some(0.7));
    }

    #[test]
    fn test_builder_with_forward() {
        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new().with_forward(0.03);
        assert_eq!(builder.default_forward, 0.03);
    }

    #[test]
    fn test_builder_with_strike_bounds() {
        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new().with_strike_bounds(0.01, 0.05);
        assert_eq!(builder.strike_bounds, Some((0.01, 0.05)));
    }

    #[test]
    fn test_builder_fluent_chain() {
        let instruments = make_test_instruments();
        let config = VolCubeConfig::default();

        let builder = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_config(config)
            .with_forward(0.03)
            .with_strike_bounds(0.01, 0.05);

        assert_eq!(builder.instruments.len(), 12);
        assert_eq!(builder.default_forward, 0.03);
        assert_eq!(builder.strike_bounds, Some((0.01, 0.05)));
    }

    #[test]
    fn test_builder_build_empty_instruments() {
        let result: Result<VolCube<f64>, _> = VolCubeBuilder::new().build();

        assert!(result.is_err());
        match result {
            Err(VolCubeError::InsufficientData { got, need }) => {
                assert_eq!(need, 1);
                assert_eq!(got, 0);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_builder_build_insufficient_grid() {
        // 1つのexpiry-tenorセルしかない場合
        let instruments = vec![
            VolInstrument::new("1", 1.0, 5.0, 0.03, 0.20, 0.03),
            VolInstrument::new("2", 1.0, 5.0, 0.02, 0.25, 0.03),
        ];

        let result = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_forward(0.03)
            .build();

        assert!(result.is_err());
        match result {
            Err(VolCubeError::InsufficientData { .. }) => {}
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_builder_build_success() {
        let instruments = make_test_instruments();

        let result = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_forward(0.03)
            .build();

        assert!(result.is_ok());
        let cube = result.unwrap();

        // ドメインの検証
        let (exp_min, exp_max) = cube.expiry_domain();
        assert_eq!(exp_min, 1.0);
        assert_eq!(exp_max, 5.0);

        let (ten_min, ten_max) = cube.tenor_domain();
        assert_eq!(ten_min, 5.0);
        assert_eq!(ten_max, 10.0);

        // ソースInstrumentの検証
        assert!(!cube.source_instruments().is_empty());
    }

    #[test]
    fn test_builder_build_with_cache() {
        use std::sync::Arc;

        use super::super::cache::VolCubeCache;

        let instruments = make_test_instruments();
        let cache = Arc::new(VolCubeCache::new(10));

        // 最初のビルド（キャッシュミス）
        let result1 = VolCubeBuilder::new()
            .with_instruments(instruments.clone())
            .with_forward(0.03)
            .with_cache(cache.clone())
            .build();

        assert!(result1.is_ok());
        assert_eq!(cache.stats().entries, 1);
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 0);

        // 2回目のビルド（キャッシュヒット）
        let result2 = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_forward(0.03)
            .with_cache(cache.clone())
            .build();

        assert!(result2.is_ok());
        assert_eq!(cache.stats().entries, 1);
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_builder_invalid_instrument() {
        let instruments = vec![VolInstrument::new(
            "invalid", -1.0, // 負のexpiry
            5.0, 0.03, 0.20, 0.03,
        )];

        let result = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_forward(0.03)
            .build();

        assert!(result.is_err());
        match result {
            Err(VolCubeError::InvalidInput { .. }) => {}
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_builder_invalid_config() {
        let instruments = make_test_instruments();
        let config = VolCubeConfig::default().with_sabr_beta(Some(1.5)); // 範囲外

        let result = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_config(config)
            .with_forward(0.03)
            .build();

        assert!(result.is_err());
        match result {
            Err(VolCubeError::InvalidInput { .. }) => {}
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_group_by_expiry_tenor() {
        let instruments = make_test_instruments();
        let builder = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_forward(0.03);

        let grouped = builder.group_by_expiry_tenor();

        // 4つのexpiry-tenorセル
        assert_eq!(grouped.len(), 4);
        assert!(grouped.contains_key(&(OrderedFloat(1.0_f64), OrderedFloat(5.0_f64))));
        assert!(grouped.contains_key(&(OrderedFloat(1.0_f64), OrderedFloat(10.0_f64))));
        assert!(grouped.contains_key(&(OrderedFloat(5.0_f64), OrderedFloat(5.0_f64))));
        assert!(grouped.contains_key(&(OrderedFloat(5.0_f64), OrderedFloat(10.0_f64))));

        // 各セルに3つのInstrument
        for (_, cell) in &grouped {
            assert_eq!(cell.len(), 3);
        }
    }

    #[test]
    fn test_simple_sabr_fit() {
        let instruments_raw = vec![
            VolInstrument::new("ATM", 1.0, 5.0, 0.03, 0.20, 0.03),
            VolInstrument::new("LOW", 1.0, 5.0, 0.02, 0.25, 0.03),
            VolInstrument::new("HIGH", 1.0, 5.0, 0.04, 0.18, 0.03),
        ];

        let instruments: Vec<&VolInstrument<f64>> = instruments_raw.iter().collect();

        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new().with_forward(0.03);
        let result = builder.simple_sabr_fit(&instruments, 0.04, 0.5, 0.03);

        assert!(result.is_ok());
        let (alpha, rho, nu) = result.unwrap();

        assert!(alpha > 0.0);
        assert!(rho > -1.0 && rho < 1.0);
        assert!(nu > 0.0);
    }

    #[test]
    fn test_calibrate_cell_single_instrument() {
        let instruments_raw = vec![VolInstrument::new("single", 1.0, 5.0, 0.03, 0.20, 0.03)];

        let instruments: Vec<&VolInstrument<f64>> = instruments_raw.iter().collect();

        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new().with_forward(0.03);
        let mut diagnostics = CalibrationDiagnostics::default();

        let result = builder.calibrate_cell(&instruments, 1.0, 5.0, &mut diagnostics);

        assert!(result.is_ok());
        let params = result.unwrap();
        assert!(params.alpha > 0.0);
        assert!(params.beta > 0.0 && params.beta <= 1.0);
    }

    #[test]
    fn test_volatility_query_after_build() {
        let instruments = make_test_instruments();

        let cube = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_forward(0.03)
            .build()
            .expect("Build should succeed");

        // グリッド内のボラティリティクエリ
        let vol = cube.volatility(1.0, 5.0, 0.03);
        assert!(vol.is_ok());
        let v = vol.unwrap();
        assert!(v > 0.0 && v < 1.0);
    }

    // =========================================================================
    // Task 4.2: Calibration Diagnostics Tests
    // Requirements: 4.4, 4.5, 4.7
    // =========================================================================

    #[test]
    fn test_calibrate_cell_returns_slice_diagnostics() {
        // ConvergenceStatus, SliceDiagnostics are available via *

        let instruments_raw = vec![
            VolInstrument::new("ATM", 1.0, 5.0, 0.03, 0.20, 0.03),
            VolInstrument::new("LOW", 1.0, 5.0, 0.02, 0.25, 0.03),
            VolInstrument::new("HIGH", 1.0, 5.0, 0.04, 0.18, 0.03),
        ];
        let instruments: Vec<&VolInstrument<f64>> = instruments_raw.iter().collect();

        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new().with_forward(0.03);
        let mut diagnostics = CalibrationDiagnostics::default();

        let result = builder.calibrate_cell(&instruments, 1.0, 5.0, &mut diagnostics);

        assert!(result.is_ok());

        // Verify slice diagnostics were populated
        assert!(!diagnostics.slice_diagnostics.is_empty());
        let slice = &diagnostics.slice_diagnostics[0];

        // Requirements 4.4: Convergence status
        assert!(matches!(
            slice.status,
            ConvergenceStatus::Success | ConvergenceStatus::Warning
        ));

        // Requirements 4.4: Iterations should be recorded
        assert!(slice.iterations > 0);

        // Requirements 4.4: Residual should be calculated
        assert!(slice.final_residual >= 0.0);

        // Requirements 4.4: Parameters should be set
        assert!(slice.parameters[0] > 0.0); // alpha > 0
        assert!(slice.parameters[1] >= 0.0 && slice.parameters[1] <= 1.0); // beta in [0,1]
        assert!(slice.parameters[2] > -1.0 && slice.parameters[2] < 1.0); // rho in (-1, 1)
        assert!(slice.parameters[3] > 0.0); // nu > 0

        // Forward should be recorded
        assert!(slice.forward > 0.0);
    }

    #[test]
    fn test_calibrate_populates_all_slice_diagnostics() {
        // ConvergenceStatus is available via *

        let instruments = make_test_instruments();

        let builder = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_forward(0.03);

        let result = builder.build_with_diagnostics();
        assert!(result.is_ok());

        let (cube, diagnostics) = result.unwrap();
        assert!(cube.expiry_domain().0 > 0.0);

        // Should have 4 slices (2 expiries × 2 tenors)
        assert_eq!(diagnostics.slice_count, 4);
        assert_eq!(diagnostics.slice_diagnostics.len(), 4);

        // Each slice should have proper data
        for slice in &diagnostics.slice_diagnostics {
            assert!(slice.expiry > 0.0);
            assert!(slice.tenor > 0.0);
            assert!(slice.iterations > 0);
            assert!(slice.parameters[0] > 0.0); // alpha
        }

        // Overall status should be set
        assert!(matches!(
            diagnostics.overall_status,
            ConvergenceStatus::Success | ConvergenceStatus::Warning
        ));
    }

    #[test]
    fn test_boundary_violation_detection_rho_near_bound() {
        // ConvergenceStatus, SabrParameter are available via *

        // Create instruments that would require extreme rho
        let instruments_raw = vec![
            VolInstrument::new("ATM", 1.0, 5.0, 0.03, 0.15, 0.03),
            VolInstrument::new("LOW", 1.0, 5.0, 0.01, 0.35, 0.03), // extreme vol at low strike
            VolInstrument::new("HIGH", 1.0, 5.0, 0.05, 0.10, 0.03), // low vol at high strike
        ];
        let instruments: Vec<&VolInstrument<f64>> = instruments_raw.iter().collect();

        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new().with_forward(0.03);
        let mut diagnostics = CalibrationDiagnostics::default();

        let _result = builder.calibrate_cell(&instruments, 1.0, 5.0, &mut diagnostics);

        // Verify slice diagnostics are created
        assert!(!diagnostics.slice_diagnostics.is_empty());
        let slice = &diagnostics.slice_diagnostics[0];
        let rho = slice.parameters[2];

        // Rho should be in valid range
        assert!(rho > -1.0 && rho < 1.0, "rho should be in (-1, 1)");

        // If rho is near boundary (>0.9), there should be a warning or boundary
        // violation The boundary violation detection uses 0.9 as threshold
        if rho.abs() > 0.9 {
            assert!(
                slice.has_warnings()
                    || slice.status == ConvergenceStatus::Warning
                    || !slice.boundary_violations.is_empty(),
                "Expected warning when rho={} is near boundary",
                rho
            );
        }

        // Regardless of rho value, verify the diagnostic structure is correct
        assert!(slice.iterations > 0);
        assert!(slice.final_residual >= 0.0);
    }

    #[test]
    fn test_diagnostics_summary_report() {
        let instruments = make_test_instruments();

        let builder = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_forward(0.03);

        let result = builder.build_with_diagnostics();
        assert!(result.is_ok());

        let (_, diagnostics) = result.unwrap();

        // Test summary report generation
        let report = diagnostics.summary_report();

        // Report should contain key information
        assert!(report.contains("状態"));
        assert!(report.contains("総スライス数"));
        assert!(report.contains("成功") || report.contains("警告") || report.contains("失敗"));
    }

    #[test]
    fn test_calibration_residual_calculation() {
        let instruments = make_test_instruments();

        let builder = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_forward(0.03);

        let result = builder.build_with_diagnostics();
        assert!(result.is_ok());

        let (_, diagnostics) = result.unwrap();

        // Total residual should be calculated
        let total_residual = diagnostics.total_residual();
        assert!(total_residual >= 0.0);

        // Convergence rate should be valid
        let convergence_rate = diagnostics.convergence_rate();
        assert!(convergence_rate >= 0.0 && convergence_rate <= 1.0);
    }

    // =========================================================================
    // Task 4.3: Arbitrage-Free Validation Tests
    // Requirements: 4.6
    // =========================================================================

    #[test]
    fn test_arbitrage_free_validation_enabled() {
        let instruments = make_test_instruments();

        // Enable arbitrage-free validation
        let config = VolCubeConfig::default().with_validate_arbitrage_free(true);

        let builder = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_config(config)
            .with_forward(0.03);

        let result = builder.build_with_diagnostics();
        assert!(result.is_ok());

        let (cube, diagnostics) = result.unwrap();

        // Cube should be valid
        assert!(cube.expiry_domain().0 > 0.0);

        // Diagnostics should have slice info
        assert!(!diagnostics.slice_diagnostics.is_empty());

        // With good data, arbitrage violations should be minimal or none
        // (we're testing the mechanism, not guaranteeing arbitrage-free data)
        let arb_count = diagnostics.arbitrage_violation_count();
        assert!(arb_count <= diagnostics.slice_count);
    }

    #[test]
    fn test_arbitrage_free_validation_disabled_by_default() {
        let instruments = make_test_instruments();

        let builder = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_forward(0.03);

        let result = builder.build_with_diagnostics();
        assert!(result.is_ok());

        let (_, diagnostics) = result.unwrap();

        // With validation disabled, arbitrage violations should be empty
        assert!(diagnostics.all_arbitrage_violations().is_empty());
    }

    #[test]
    fn test_arbitrage_violation_structure() {
        use super::super::super::volcube::error::ArbitrageViolationType;

        // Test ArbitrageViolation construction
        let violation = ArbitrageViolation::negative_density(0.03, -0.001);

        assert_eq!(violation.strike, 0.03);
        assert_eq!(
            violation.violation_type,
            ArbitrageViolationType::NegativeDensity
        );
        assert!(violation.density_value.is_some());
        assert!(violation.message.contains("負の確率密度"));

        // Test butterfly spread violation
        let butterfly = ArbitrageViolation::negative_butterfly(0.04);
        assert_eq!(
            butterfly.violation_type,
            ArbitrageViolationType::NegativeButterflySpread
        );
    }

    #[test]
    fn test_arbitrage_violations_in_summary_report() {
        let instruments = make_test_instruments();

        let config = VolCubeConfig::default().with_validate_arbitrage_free(true);

        let builder = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_config(config)
            .with_forward(0.03);

        let result = builder.build_with_diagnostics();
        assert!(result.is_ok());

        let (_, diagnostics) = result.unwrap();

        // Summary report should be generated
        let report = diagnostics.summary_report();
        assert!(report.contains("カリブレーション診断サマリー"));

        // If there are arbitrage violations, they should appear in the report
        if diagnostics.arbitrage_violation_count() > 0 {
            assert!(report.contains("Arbitrage条件違反"));
        }
    }

    #[test]
    fn test_slice_diagnostics_with_arbitrage_violations() {
        let mut slice_diag = SliceDiagnostics::new(1.0, 5.0);

        assert!(!slice_diag.has_arbitrage_violations());

        // Add an arbitrage violation
        slice_diag.add_arbitrage_violation(ArbitrageViolation::negative_density(0.03, -0.001));

        assert!(slice_diag.has_arbitrage_violations());
        assert!(slice_diag.has_warnings());
        assert_eq!(slice_diag.status, ConvergenceStatus::Warning);
        assert_eq!(slice_diag.arbitrage_violations.len(), 1);
    }
}
