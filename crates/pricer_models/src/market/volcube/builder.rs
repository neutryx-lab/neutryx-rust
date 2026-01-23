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
    cache::{SharedVolCubeCache, VolCubeKey},
    config::VolCubeConfig,
    cube::VolCube,
    error::{CalibrationDiagnostics, VolCubeError},
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

impl<T: Float> Default for VolCubeBuilder<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Float> VolCubeBuilder<T> {
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
        let cube = self.calibrate()?;

        // キャッシュに格納
        if let Some(ref cache_ref) = cache {
            cache_ref.insert(cache_key, cube.clone());
        }

        Ok(cube)
    }

    /// SABRカリブレーションを実行してVolCubeを構築。
    fn calibrate(self) -> Result<VolCube<T>, VolCubeError> {
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
                        let p = self.calibrate_cell(instruments, *expiry, &mut diagnostics)?;
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

        // VolCubeを構築
        let cube = VolCube::new(
            sabr_surface,
            forward_grid,
            self.config,
            source_instruments,
            strike_bounds,
        );

        Ok(cube)
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
    fn calibrate_cell(
        &self,
        instruments: &[&VolInstrument<T>],
        _expiry: T,
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

        // 診断情報を更新
        diagnostics.converged_slices += 1;
        diagnostics.slice_count += 1;
        diagnostics.iterations += 1;

        Ok(SabrParams::new(alpha, beta, rho, nu))
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
            VolInstrument::new(("1Y5Y_ATM"), 1.0, 5.0, 0.03, 0.20, forward),
            VolInstrument::new(("1Y5Y_LOW"), 1.0, 5.0, 0.02, 0.25, forward),
            VolInstrument::new(("1Y5Y_HIGH"), 1.0, 5.0, 0.04, 0.22, forward),
            // expiry=1.0, tenor=10.0 スマイル
            VolInstrument::new(("1Y10Y_ATM"), 1.0, 10.0, 0.03, 0.18, forward),
            VolInstrument::new(("1Y10Y_LOW"), 1.0, 10.0, 0.02, 0.23, forward),
            VolInstrument::new(("1Y10Y_HIGH"), 1.0, 10.0, 0.04, 0.20, forward),
            // expiry=5.0, tenor=5.0 スマイル
            VolInstrument::new(("5Y5Y_ATM"), 5.0, 5.0, 0.03, 0.16, forward),
            VolInstrument::new(("5Y5Y_LOW"), 5.0, 5.0, 0.02, 0.20, forward),
            VolInstrument::new(("5Y5Y_HIGH"), 5.0, 5.0, 0.04, 0.17, forward),
            // expiry=5.0, tenor=10.0 スマイル
            VolInstrument::new(("5Y10Y_ATM"), 5.0, 10.0, 0.03, 0.15, forward),
            VolInstrument::new(("5Y10Y_LOW"), 5.0, 10.0, 0.02, 0.19, forward),
            VolInstrument::new(("5Y10Y_HIGH"), 5.0, 10.0, 0.04, 0.16, forward),
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
        let instrument = VolInstrument::new(("test"), 1.0, 5.0, 0.03, 0.20, 0.03);
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
            VolInstrument::new(("1"), 1.0, 5.0, 0.03, 0.20, 0.03),
            VolInstrument::new(("2"), 1.0, 5.0, 0.02, 0.25, 0.03),
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
            ("invalid"),
            -1.0, // 負のexpiry
            5.0,
            0.03,
            0.20,
            0.03,
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
            VolInstrument::new(("ATM"), 1.0, 5.0, 0.03, 0.20, 0.03),
            VolInstrument::new(("LOW"), 1.0, 5.0, 0.02, 0.25, 0.03),
            VolInstrument::new(("HIGH"), 1.0, 5.0, 0.04, 0.18, 0.03),
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
        let instruments_raw = vec![VolInstrument::new(("single"), 1.0, 5.0, 0.03, 0.20, 0.03)];

        let instruments: Vec<&VolInstrument<f64>> = instruments_raw.iter().collect();

        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new().with_forward(0.03);
        let mut diagnostics = CalibrationDiagnostics::default();

        let result = builder.calibrate_cell(&instruments, 1.0, &mut diagnostics);

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
}
