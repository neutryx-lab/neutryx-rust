# Technical Design: IR Vol Cube Calibration Engine

## 1. 概要

本設計書は、IR Vol Cube Calibration Engineの技術アーキテクチャを定義する。CurveBuilderパターンを参照し、Curve→VolCubeの統合依存グラフを持つカリブレーションフレームワークを構築する。

### 1.1 スコープ

- European Swaption / CapFloor商品定義（USD/EUR/JPY）
- SABRモデルカリブレーション
- CalibrationGraph（Curve→VolCube依存管理）
- LazyValuation・キャッシュ最適化
- AAD（Adjoint Algorithmic Differentiation）統合
- WebApp統合・データローダー

### 1.2 設計原則

1. **既存コード再利用**: volcubeモジュール拡張、新規ファイル最小化
2. **Static Dispatch**: enum-based trait objectでEnzyme互換性維持
3. **段階的実装**: Phase 1-4で複雑性を分散
4. **一貫したパターン**: IrsLazyEvaluator、MarketProviderパターン踏襲

---

## 2. アーキテクチャ

### 2.1 全体構成

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                          Service Layer (demo/gui)                        │
│  volcube_handlers.rs ─── /api/volcube/calibrate ─── 3D Visualization    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Pricing Layer (pricer_pricing)                    │
│  ┌─────────────────┐    ┌─────────────────┐    ┌──────────────────┐    │
│  │ VolLazyEvaluator│◄───│ CalibrationGraph│◄───│ AAD Integration  │    │
│  │ (キャッシュ)    │    │ (依存解決)      │    │ (感応度計算)    │    │
│  └─────────────────┘    └─────────────────┘    └──────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Models Layer (pricer_models)                      │
│  ┌─────────────────┐    ┌─────────────────┐    ┌──────────────────┐    │
│  │ VolCube<T>      │◄───│VolCubeCalibration│◄───│ MarketProvider   │    │
│  │ (3D構造)        │    │Engine (カリブレ)│    │ (Arc cache)      │    │
│  └─────────────────┘    └─────────────────┘    └──────────────────┘    │
│            ▲                      ▲                                      │
│            │                      │                                      │
│  ┌─────────────────┐    ┌─────────────────┐                             │
│  │ SabrCalibrator  │    │ YieldCurve      │                             │
│  │ (SABR最適化)    │    │ (forward rate)  │                             │
│  └─────────────────┘    └─────────────────┘                             │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Infra Layer (infra_domain)                        │
│  ┌─────────────────┐    ┌─────────────────┐                             │
│  │ Swaption        │    │ SwaptionConvention │                          │
│  │ CapFloor        │    │ CapFloorConvention │                          │
│  └─────────────────┘    └─────────────────┘                             │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Adapter Layer (adapter_loader)                    │
│  ┌─────────────────┐                                                    │
│  │ VolSurfaceLoader│◄─── JSON/CSV swaption/capfloor quotes             │
│  └─────────────────┘                                                    │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 データフロー

```text
Market Quotes (JSON/CSV)
        │
        ▼
┌───────────────┐
│VolSurfaceLoader│
└───────────────┘
        │ VolQuoteSet
        ▼
┌───────────────┐     ┌───────────────┐
│VolCubeBuilder │◄────│  YieldCurve   │ (forward rate計算)
└───────────────┘     └───────────────┘
        │ RawVolCube
        ▼
┌───────────────────────┐
│VolCubeCalibrationEngine│
└───────────────────────┘
        │ per-slice SABR calibration
        ▼
┌───────────────┐
│CalibratedVolCube│──► AAD Graph ──► Sensitivities
└───────────────┘
        │
        ▼
    WebApp Visualization
```

---

## 3. コンポーネント設計

### 3.1 VolCubeConfig拡張

**ファイル**: `pricer_models/src/market/volcube/config.rs`

```rust
use crate::types::CurveName;
use infra_domain::currency::Currency;

/// VolCubeカリブレーション設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolCubeConfig {
    // 既存フィールド
    pub interpolation: InterpolationMethod,
    pub extrapolation: ExtrapolationMethod,
    pub strike_axis: StrikeAxisType,
    pub optimizer: OptimizerType,
    pub validate_arbitrage_free: bool,
    pub sabr_beta: Option<f64>,
    pub sabr_shift: f64,
    pub max_iterations: usize,
    pub tolerance: f64,

    // 新規フィールド (Curve依存)
    pub discount_curve: CurveName,
    pub projection_curve: CurveName,
    pub calibration_order: CalibrationOrder,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum CalibrationOrder {
    #[default]
    ExpiryFirst,  // 各expiryでtenor方向にカリブレーション
    TenorFirst,   // 各tenorでexpiry方向にカリブレーション
}

impl VolCubeConfig {
    /// 通貨別デフォルト設定
    pub fn default_for_currency(ccy: Currency) -> Self {
        let (discount, projection) = match ccy {
            Currency::USD => ("USD-SOFR-OIS".into(), "USD-SOFR".into()),
            Currency::EUR => ("EUR-ESTR-OIS".into(), "EUR-ESTR".into()),
            Currency::JPY => ("JPY-TONA-OIS".into(), "JPY-TONA".into()),
            _ => ("DEFAULT-OIS".into(), "DEFAULT".into()),
        };
        Self {
            interpolation: InterpolationMethod::Sabr,
            extrapolation: ExtrapolationMethod::Flat,
            strike_axis: StrikeAxisType::Moneyness,
            optimizer: OptimizerType::LevenbergMarquardt,
            validate_arbitrage_free: true,
            sabr_beta: Some(0.5),
            sabr_shift: 0.0,
            max_iterations: 100,
            tolerance: 1e-8,
            discount_curve: discount,
            projection_curve: projection,
            calibration_order: CalibrationOrder::ExpiryFirst,
        }
    }
}
```

**要件対応**: Req 5.8, 5.10

### 3.2 VolQuote構造

**ファイル**: `pricer_models/src/market/volcube/quote.rs` (新規)

```rust
use chrono::NaiveDate;
use crate::types::InstrumentId;

/// マーケットボラティリティクォート
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolQuote {
    pub instrument_id: InstrumentId,
    pub expiry: NaiveDate,
    pub tenor: Tenor,
    pub strike: Strike,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub mid: f64,
    pub quote_type: QuoteType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QuoteType {
    Normal,       // 正規ボラティリティ
    LogNormal,    // 対数正規ボラティリティ
    ShiftedLogNormal { shift: f64 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Strike {
    Absolute(f64),
    RelativeToAtm(f64),      // ATMからの乖離 (bps)
    Moneyness(f64),          // K/F
    LogMoneyness(f64),       // ln(K/F)
}

/// クォートセット（VolCubeBuilder入力）
#[derive(Debug, Clone, Default)]
pub struct VolQuoteSet {
    pub quotes: Vec<VolQuote>,
    pub currency: Currency,
    pub underlying_index: UnderlyingIndex,
    pub as_of_date: NaiveDate,
}
```

**要件対応**: Req 2.2, 10.3

### 3.3 VolCubeCalibrationEngine

**ファイル**: `pricer_models/src/market/volcube/engine.rs` (新規)

```rust
use std::sync::Arc;
use parking_lot::RwLock;

/// カリブレーション結果
#[derive(Debug, Clone)]
pub struct VolCubeCalibrationResult<T: Float> {
    pub calibrated_cube: VolCube<T>,
    pub slice_results: Vec<SliceCalibrationResult<T>>,
    pub total_residual: T,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone)]
pub struct SliceCalibrationResult<T: Float> {
    pub expiry: T,
    pub tenor: T,
    pub sabr_params: SabrParams<T>,
    pub residuals: Vec<T>,
    pub iterations: usize,
    pub converged: bool,
    pub arbitrage_free: bool,
}

/// VolCubeカリブレーションエンジン
pub struct VolCubeCalibrationEngine<T: Float> {
    config: VolCubeConfig,
    calibrator: CalibratorKind,
    progress_callback: Option<Box<dyn Fn(CalibrationProgress) + Send + Sync>>,
    _phantom: PhantomData<T>,
}

#[derive(Debug, Clone)]
pub struct CalibrationProgress {
    pub current_slice: (usize, usize),  // (expiry_idx, tenor_idx)
    pub total_slices: usize,
    pub current_iteration: usize,
    pub current_residual: f64,
}

impl<T: Float> VolCubeCalibrationEngine<T> {
    pub fn new(config: VolCubeConfig) -> Self {
        let calibrator = match config.interpolation {
            InterpolationMethod::Sabr => CalibratorKind::Sabr(SabrCalibrator::new(&config)),
            InterpolationMethod::Svi => CalibratorKind::Svi(SviCalibrator::new(&config)),
            InterpolationMethod::FlatVol => CalibratorKind::Flat(FlatCalibrator::new()),
            InterpolationMethod::Linear => CalibratorKind::Linear(LinearCalibrator::new()),
            _ => CalibratorKind::Sabr(SabrCalibrator::new(&config)),
        };
        Self {
            config,
            calibrator,
            progress_callback: None,
            _phantom: PhantomData,
        }
    }

    pub fn with_progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(CalibrationProgress) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }

    /// カリブレーション実行
    pub fn calibrate(
        &self,
        quotes: &VolQuoteSet,
        curve_set: &CurveSet,
    ) -> Result<VolCubeCalibrationResult<T>, CalibrationError> {
        // 1. Curve解決
        let discount_curve = curve_set
            .get(&self.config.discount_curve)
            .ok_or_else(|| CalibrationError::CurveNotFound(self.config.discount_curve.clone()))?;
        let projection_curve = curve_set
            .get(&self.config.projection_curve)
            .ok_or_else(|| CalibrationError::CurveNotFound(self.config.projection_curve.clone()))?;

        // 2. Forward rate計算
        let forwards = self.compute_forwards(quotes, projection_curve)?;

        // 3. スライス毎カリブレーション
        let slices = self.organize_slices(quotes);
        let mut slice_results = Vec::with_capacity(slices.len());

        for (idx, slice) in slices.iter().enumerate() {
            if let Some(ref cb) = self.progress_callback {
                cb(CalibrationProgress {
                    current_slice: (slice.expiry_idx, slice.tenor_idx),
                    total_slices: slices.len(),
                    current_iteration: 0,
                    current_residual: f64::MAX,
                });
            }

            let result = self.calibrate_slice(slice, &forwards)?;
            slice_results.push(result);
        }

        // 4. VolCube構築
        let calibrated_cube = self.build_cube(&slice_results, &forwards)?;

        // 5. Arbitrage-free検証
        if self.config.validate_arbitrage_free {
            self.validate_arbitrage_free(&calibrated_cube)?;
        }

        Ok(VolCubeCalibrationResult {
            calibrated_cube,
            slice_results,
            total_residual: self.compute_total_residual(&slice_results),
            elapsed_ms: 0, // TODO: timing
        })
    }
}
```

**要件対応**: Req 5.1-5.7, 5.9

### 3.4 CalibrationGraph（依存管理）

**ファイル**: `pricer_pricing/src/calibration_graph.rs` (新規)

```rust
use std::collections::{HashMap, HashSet, VecDeque};
use parking_lot::RwLock;

/// カリブレーションノードID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

/// ノード種別
#[derive(Debug, Clone)]
pub enum CalibrationNodeKind {
    Curve { name: CurveName },
    VolCube { currency: Currency, index: UnderlyingIndex },
}

/// カリブレーション状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationStatus {
    Pending,     // 未カリブレーション
    Computing,   // カリブレーション中
    Calibrated,  // カリブレーション済み
    Stale,       // 依存元更新により無効
}

/// カリブレーションノード
#[derive(Debug, Clone)]
pub struct CalibrationNode {
    pub id: NodeId,
    pub kind: CalibrationNodeKind,
    pub status: CalibrationStatus,
    pub last_calibrated: Option<chrono::DateTime<chrono::Utc>>,
}

/// Curve→VolCube依存グラフ
pub struct CalibrationGraph {
    nodes: HashMap<NodeId, CalibrationNode>,
    /// child → parents (VolCube → Curves it depends on)
    dependencies: HashMap<NodeId, Vec<NodeId>>,
    /// parent → children (Curve → VolCubes that depend on it)
    reverse_deps: HashMap<NodeId, Vec<NodeId>>,
    next_id: u64,
}

impl CalibrationGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            dependencies: HashMap::new(),
            reverse_deps: HashMap::new(),
            next_id: 0,
        }
    }

    /// ノード追加
    pub fn add_node(&mut self, kind: CalibrationNodeKind) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        self.nodes.insert(id, CalibrationNode {
            id,
            kind,
            status: CalibrationStatus::Pending,
            last_calibrated: None,
        });
        id
    }

    /// 依存関係追加 (child depends on parent)
    pub fn add_dependency(&mut self, child: NodeId, parent: NodeId) {
        self.dependencies.entry(child).or_default().push(parent);
        self.reverse_deps.entry(parent).or_default().push(child);
    }

    /// トポロジカルソート（カリブレーション順序）
    pub fn topological_order(&self) -> Result<Vec<NodeId>, GraphError> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        for id in self.nodes.keys() {
            in_degree.insert(*id, 0);
        }
        for deps in self.dependencies.values() {
            for dep in deps {
                // depはparent、親が先にカリブレーションされる必要
                // childのin_degreeを増やす
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<NodeId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut result = Vec::new();
        while let Some(node) = queue.pop_front() {
            result.push(node);
            if let Some(children) = self.reverse_deps.get(&node) {
                for child in children {
                    if let Some(deg) = in_degree.get_mut(child) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(*child);
                        }
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            return Err(GraphError::CyclicDependency);
        }
        Ok(result)
    }

    /// ノード無効化（カスケード）
    pub fn invalidate(&mut self, node_id: NodeId) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.status = CalibrationStatus::Stale;
        }
        // 依存する子ノードも無効化
        if let Some(children) = self.reverse_deps.get(&node_id).cloned() {
            for child in children {
                self.invalidate(child);
            }
        }
    }

    /// 指定ノードのカリブレーションに必要なノード一覧
    pub fn required_calibrations(&self, target: NodeId) -> Vec<NodeId> {
        let mut required = Vec::new();
        let mut visited = HashSet::new();
        self.collect_required(target, &mut required, &mut visited);
        required
    }

    fn collect_required(
        &self,
        node: NodeId,
        required: &mut Vec<NodeId>,
        visited: &mut HashSet<NodeId>,
    ) {
        if visited.contains(&node) {
            return;
        }
        visited.insert(node);

        // 親を先に処理
        if let Some(parents) = self.dependencies.get(&node) {
            for parent in parents {
                self.collect_required(*parent, required, visited);
            }
        }

        // 自身をPending/Staleなら追加
        if let Some(n) = self.nodes.get(&node) {
            if matches!(n.status, CalibrationStatus::Pending | CalibrationStatus::Stale) {
                required.push(node);
            }
        }
    }
}
```

**要件対応**: Req 6.8-6.12

### 3.5 VolLazyEvaluator

**ファイル**: `pricer_pricing/src/vol_lazy_evaluator.rs` (新規)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use dashmap::DashMap;

/// キャッシュキー
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VolCacheKey {
    pub currency: Currency,
    pub index: UnderlyingIndex,
    pub expiry_tenor: Option<(OrderedFloat<f64>, OrderedFloat<f64>)>,
}

/// キャッシュ状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    Fresh,
    Stale,
    Computing,
}

/// キャッシュ結果
#[derive(Debug, Clone)]
pub struct CachedVolCube<T: Float> {
    pub cube: Arc<VolCube<T>>,
    pub calibration_result: Arc<VolCubeCalibrationResult<T>>,
    pub cached_at: chrono::DateTime<chrono::Utc>,
}

/// キャッシュ統計
#[derive(Debug, Default)]
pub struct VolCacheStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub invalidations: AtomicU64,
    pub calibrations: AtomicU64,
}

/// VolCube遅延評価器
pub struct VolLazyEvaluator<T: Float> {
    cache: DashMap<VolCacheKey, CachedVolCube<T>>,
    cache_state: DashMap<VolCacheKey, CacheState>,
    calibration_graph: Arc<RwLock<CalibrationGraph>>,
    stats: VolCacheStats,
    market_provider: Arc<MarketProvider>,
}

impl<T: Float> VolLazyEvaluator<T> {
    pub fn new(market_provider: Arc<MarketProvider>) -> Self {
        Self {
            cache: DashMap::new(),
            cache_state: DashMap::new(),
            calibration_graph: Arc::new(RwLock::new(CalibrationGraph::new())),
            stats: VolCacheStats::default(),
            market_provider,
        }
    }

    /// VolCube取得（遅延カリブレーション）
    pub fn get_volcube(
        &self,
        currency: Currency,
        index: UnderlyingIndex,
        config: &VolCubeConfig,
    ) -> Result<Arc<VolCube<T>>, EvaluatorError> {
        let key = VolCacheKey {
            currency,
            index,
            expiry_tenor: None,
        };

        // 1. キャッシュチェック
        if let Some(cached) = self.cache.get(&key) {
            if self.cache_state.get(&key).map(|s| *s == CacheState::Fresh).unwrap_or(false) {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(cached.cube.clone());
            }
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);

        // 2. Computing状態に設定（double-check locking）
        self.cache_state.insert(key.clone(), CacheState::Computing);

        // 3. 依存Curveのカリブレーション確認
        self.ensure_curves_calibrated(config)?;

        // 4. VolCubeカリブレーション
        let result = self.calibrate_volcube(currency, index, config)?;

        // 5. キャッシュ格納
        self.cache.insert(key.clone(), CachedVolCube {
            cube: Arc::new(result.calibrated_cube.clone()),
            calibration_result: Arc::new(result),
            cached_at: chrono::Utc::now(),
        });
        self.cache_state.insert(key.clone(), CacheState::Fresh);

        self.stats.calibrations.fetch_add(1, Ordering::Relaxed);

        Ok(self.cache.get(&key).unwrap().cube.clone())
    }

    /// Quote更新時の無効化
    pub fn invalidate_on_quote_update(&self, currency: Currency, index: UnderlyingIndex) {
        let key = VolCacheKey {
            currency,
            index,
            expiry_tenor: None,
        };
        self.cache_state.insert(key, CacheState::Stale);
        self.stats.invalidations.fetch_add(1, Ordering::Relaxed);

        // CalibrationGraphでカスケード無効化
        // ...
    }

    /// 統計取得
    pub fn stats(&self) -> &VolCacheStats {
        &self.stats
    }

    fn ensure_curves_calibrated(&self, config: &VolCubeConfig) -> Result<(), EvaluatorError> {
        // MarketProviderからCurve取得（lazy calibration）
        let _ = self.market_provider.get_curve(&config.discount_curve)?;
        let _ = self.market_provider.get_curve(&config.projection_curve)?;
        Ok(())
    }

    fn calibrate_volcube(
        &self,
        currency: Currency,
        index: UnderlyingIndex,
        config: &VolCubeConfig,
    ) -> Result<VolCubeCalibrationResult<T>, EvaluatorError> {
        let engine = VolCubeCalibrationEngine::new(config.clone());
        let quotes = self.market_provider.get_vol_quotes(currency, index)?;
        let curve_set = self.market_provider.get_curve_set()?;

        engine
            .calibrate(&quotes, &curve_set)
            .map_err(EvaluatorError::Calibration)
    }
}
```

**要件対応**: Req 6.1-6.7

### 3.6 AAD統合

**ファイル**: `pricer_pricing/src/aad/vol_sensitivities.rs` (新規)

```rust
use crate::graph::{ComputationGraph, GraphNode, GraphEdge, GraphExtractable};

/// VolCube感応度計算コンテキスト
pub struct VolSensitivityContext<T: Float> {
    computation_graph: ComputationGraph,
    tape_cache: AadTapeCache,
}

impl<T: Float> VolSensitivityContext<T> {
    /// Vega計算（∂Price/∂VolQuote）
    pub fn compute_vega(
        &self,
        volcube: &VolCube<T>,
        instrument: &impl Priceable<T>,
    ) -> Result<VegaResult<T>, SensitivityError> {
        // AAD adjoint mode
        // ...
    }

    /// Curve経由の間接感応度（∂Price/∂CurveQuote via VolCube）
    pub fn compute_curve_sensitivity(
        &self,
        volcube: &VolCube<T>,
        curve: &YieldCurve<T>,
        instrument: &impl Priceable<T>,
    ) -> Result<CurveSensitivityResult<T>, SensitivityError> {
        // Full AAD path: CurveQuote → Curve → ForwardRate → VolCube → Price
        // ...
    }

    /// Bump-and-revalue検証
    pub fn validate_against_bump_revalue(
        &self,
        aad_result: &SensitivityResult<T>,
        bump_size: f64,
    ) -> ValidationResult {
        // 数値微分との比較
        // ...
    }
}

impl<T: Float> GraphExtractable for VolCube<T> {
    fn extract_graph(&self) -> ComputationGraph {
        let mut graph = ComputationGraph::new();

        // ノード: VolQuotes → SABRParams → InterpolatedVol
        for (expiry_idx, expiry) in self.expiries.iter().enumerate() {
            for (tenor_idx, tenor) in self.tenors.iter().enumerate() {
                let quote_node = graph.add_node(GraphNode::VolQuote {
                    expiry: expiry_idx,
                    tenor: tenor_idx,
                });
                let sabr_node = graph.add_node(GraphNode::SabrParams {
                    expiry: expiry_idx,
                    tenor: tenor_idx,
                });
                graph.add_edge(quote_node, sabr_node);
            }
        }

        graph
    }
}
```

**要件対応**: Req 7.1-7.10

### 3.7 WebApp統合

**ファイル**: `demo/gui/src/web/volcube_handlers.rs` (新規)

```rust
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CalibrateRequest {
    pub currency: String,
    pub index: String,
    pub config: Option<VolCubeConfigDto>,
}

#[derive(Serialize)]
pub struct CalibrateResponse {
    pub success: bool,
    pub cube_id: String,
    pub sabr_params: Vec<SabrParamDto>,
    pub diagnostics: CalibrationDiagnosticsDto,
    pub surface_data: SurfaceDataDto,  // 3D可視化用
}

#[derive(Serialize)]
pub struct SabrParamDto {
    pub expiry: f64,
    pub tenor: f64,
    pub alpha: f64,
    pub beta: f64,
    pub rho: f64,
    pub nu: f64,
}

#[derive(Serialize)]
pub struct SurfaceDataDto {
    pub expiries: Vec<f64>,
    pub tenors: Vec<f64>,
    pub strikes: Vec<f64>,
    pub vols: Vec<Vec<Vec<f64>>>,  // [expiry][tenor][strike]
    pub market_vols: Vec<VolQuoteDto>,
    pub fitted_vols: Vec<VolQuoteDto>,
}

/// POST /api/volcube/calibrate
pub async fn calibrate_volcube(
    State(app_state): State<AppState>,
    Json(request): Json<CalibrateRequest>,
) -> Result<Json<CalibrateResponse>, AppError> {
    let currency = request.currency.parse()?;
    let index = request.index.parse()?;

    let config = request
        .config
        .map(|c| c.into())
        .unwrap_or_else(|| VolCubeConfig::default_for_currency(currency));

    let result = app_state
        .vol_lazy_evaluator
        .get_volcube_with_result(currency, index, &config)
        .await?;

    Ok(Json(CalibrateResponse {
        success: true,
        cube_id: format!("{}-{}", currency, index),
        sabr_params: result.slice_results.iter().map(|s| s.into()).collect(),
        diagnostics: (&result).into(),
        surface_data: build_surface_data(&result),
    }))
}

/// GET /api/volcube/{id}/density
pub async fn get_density(
    State(app_state): State<AppState>,
    Path(cube_id): Path<String>,
    Query(params): Query<DensityParams>,
) -> Result<Json<DensityResponse>, AppError> {
    // Breeden-Litzenberger密度抽出
    // ...
}
```

**要件対応**: Req 8.1-8.8

### 3.8 VolSurfaceLoader

**ファイル**: `adapter_loader/src/volsurface.rs` (新規)

```rust
use std::path::Path;
use csv::ReaderBuilder;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct VolQuoteRow {
    expiry: String,      // "1Y", "2Y", etc.
    tenor: String,       // "1Y", "5Y", "10Y"
    strike: f64,         // bps from ATM or absolute
    strike_type: String, // "absolute", "relative", "moneyness"
    bid: Option<f64>,
    ask: Option<f64>,
    mid: f64,
    quote_type: String,  // "normal", "lognormal"
}

pub struct VolSurfaceLoader;

impl VolSurfaceLoader {
    /// CSV読み込み
    pub fn load_csv(path: impl AsRef<Path>) -> Result<VolQuoteSet, LoaderError> {
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_path(path.as_ref())
            .map_err(|e| LoaderError::FileNotFound(path.as_ref().to_path_buf()))?;

        let mut quotes = Vec::new();
        for (line_num, result) in reader.deserialize().enumerate() {
            let row: VolQuoteRow = result.map_err(|e| LoaderError::ParseError {
                line: line_num + 2,  // +1 for 0-index, +1 for header
                message: e.to_string(),
            })?;

            quotes.push(Self::convert_row(row)?);
        }

        Ok(VolQuoteSet {
            quotes,
            ..Default::default()
        })
    }

    /// JSON読み込み
    pub fn load_json(path: impl AsRef<Path>) -> Result<VolQuoteSet, LoaderError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|_| LoaderError::FileNotFound(path.as_ref().to_path_buf()))?;

        let data: VolSurfaceJson = serde_json::from_str(&content)
            .map_err(|e| LoaderError::ParseError {
                line: 0,
                message: e.to_string(),
            })?;

        Ok(data.into())
    }

    fn convert_row(row: VolQuoteRow) -> Result<VolQuote, LoaderError> {
        let expiry = parse_tenor_to_date(&row.expiry)?;
        let tenor = parse_tenor(&row.tenor)?;
        let strike = match row.strike_type.as_str() {
            "absolute" => Strike::Absolute(row.strike),
            "relative" => Strike::RelativeToAtm(row.strike),
            "moneyness" => Strike::Moneyness(row.strike),
            _ => return Err(LoaderError::InvalidStrikeType(row.strike_type)),
        };

        Ok(VolQuote {
            instrument_id: InstrumentId::default(),
            expiry,
            tenor,
            strike,
            bid: row.bid,
            ask: row.ask,
            mid: row.mid,
            quote_type: row.quote_type.parse()?,
        })
    }
}
```

**要件対応**: Req 10.1-10.7

---

## 4. エラーハンドリング

### 4.1 エラー型定義

**ファイル**: `pricer_models/src/market/volcube/error.rs` (既存拡張)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CalibrationError {
    #[error("Curve not found: {0}")]
    CurveNotFound(CurveName),

    #[error("Calibration did not converge for slice ({expiry}, {tenor}): residual={residual}")]
    NotConverged {
        expiry: f64,
        tenor: f64,
        residual: f64,
        iterations: usize,
    },

    #[error("Parameter out of bounds: {param}={value} (bounds: [{min}, {max}])")]
    ParameterOutOfBounds {
        param: String,
        value: f64,
        min: f64,
        max: f64,
    },

    #[error("Arbitrage detected at expiry={expiry}: {description}")]
    ArbitrageDetected {
        expiry: f64,
        description: String,
    },

    #[error("Insufficient quotes for slice ({expiry}, {tenor}): found {found}, required {required}")]
    InsufficientQuotes {
        expiry: f64,
        tenor: f64,
        found: usize,
        required: usize,
    },
}

#[derive(Error, Debug)]
pub enum GraphError {
    #[error("Cyclic dependency detected in calibration graph")]
    CyclicDependency,

    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeId),
}

#[derive(Error, Debug)]
pub enum LoaderError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },

    #[error("Invalid strike type: {0}")]
    InvalidStrikeType(String),
}
```

---

## 5. 実装フェーズ

### Phase 1: VolCube拡張（Req 1-4, 9）

| タスク | ファイル | 依存 |
|--------|---------|------|
| VolCubeConfig拡張 | config.rs | なし |
| VolQuote構造追加 | quote.rs (新規) | なし |
| VolCubeCalibrationEngine | engine.rs (新規) | config.rs |
| Flat/LinearInterpolator追加 | calibrator.rs | なし |
| EUR ESTR convention追加 | convention/swaption.rs | なし |

### Phase 2: 統合依存グラフ（Req 5, 6）

| タスク | ファイル | 依存 |
|--------|---------|------|
| CalibrationGraph | calibration_graph.rs (新規) | Phase 1 |
| VolLazyEvaluator | vol_lazy_evaluator.rs (新規) | CalibrationGraph |
| MarketProvider拡張 | provider.rs | VolLazyEvaluator |

### Phase 3: AAD統合（Req 7）

| タスク | ファイル | 依存 |
|--------|---------|------|
| VolCube GraphExtractable | aad/vol_sensitivities.rs | Phase 2 |
| Vega計算 | aad/vol_sensitivities.rs | GraphExtractable |
| Curve間接感応度 | aad/vol_sensitivities.rs | Vega計算 |
| Bump-and-revalue検証 | aad/validation.rs (新規) | 感応度計算 |

### Phase 4: WebApp・Loader（Req 8, 10）

| タスク | ファイル | 依存 |
|--------|---------|------|
| VolSurfaceLoader | volsurface.rs (新規) | なし |
| WebApp handlers | volcube_handlers.rs (新規) | Phase 2 |
| 3D可視化 | static/volcube.js (新規) | handlers |
| Breeden-Litzenberger表示 | static/density.js (新規) | handlers |

---

## 6. テスト戦略

### 6.1 単体テスト

| コンポーネント | テストファイル | カバレッジ目標 |
|---------------|---------------|---------------|
| VolCubeConfig | config_test.rs | 100% |
| CalibrationGraph | graph_test.rs | 90% |
| VolCubeCalibrationEngine | engine_test.rs | 85% |
| VolLazyEvaluator | lazy_test.rs | 85% |
| VolSurfaceLoader | loader_test.rs | 95% |

### 6.2 統合テスト

| シナリオ | テストファイル |
|---------|---------------|
| Curve→VolCubeカリブレーション | integration/calibration_flow.rs |
| Quote更新→キャッシュ無効化 | integration/cache_invalidation.rs |
| AAD Vega計算 | integration/aad_vega.rs |
| WebApp E2E | integration/webapp_e2e.rs |

### 6.3 ベンチマーク

| 測定項目 | 目標 |
|---------|------|
| 単一VolCubeカリブレーション | < 500ms |
| キャッシュヒット時取得 | < 1ms |
| AAD Vega計算（100 instrument） | < 100ms |

---

## 7. 要件トレーサビリティ

| 要件ID | 設計セクション | コンポーネント |
|--------|---------------|---------------|
| Req 1 | 3.2 | VolQuote, infra_domain |
| Req 2 | 3.2, 3.3 | VolQuote, VolCube |
| Req 3 | 3.1, 3.3 | VolCubeConfig, CalibratorKind |
| Req 4 | 3.3 | VolCubeCalibrationEngine |
| Req 5 | 3.1, 3.3, 3.4 | VolCubeConfig, Engine, CalibrationGraph |
| Req 6 | 3.4, 3.5 | CalibrationGraph, VolLazyEvaluator |
| Req 7 | 3.6 | VolSensitivityContext, GraphExtractable |
| Req 8 | 3.7 | volcube_handlers.rs |
| Req 9 | - | Phase 1で既存コード活用 |
| Req 10 | 3.8 | VolSurfaceLoader |

---

_Generated: 2026-01-25_
_Design follows gap-analysis.md Option C (Hybrid Approach)_
