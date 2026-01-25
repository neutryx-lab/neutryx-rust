# Research Log: IR Vol Cube Calibration

## 調査概要

本ドキュメントは、IR Vol Cube Calibration Engine設計フェーズにおける調査結果を記録する。

**調査日時**: 2026-01-25
**機能分類**: Complex Integration（既存volcubeモジュールの拡張 + Curve依存統合）

---

## 1. 発見事項

### 1.1 既存VolCubeアーキテクチャ

**ファイル**: [cube.rs](crates/pricer_models/src/market/volcube/cube.rs)

```rust
pub trait VolatilityCube<T: Float>: Send + Sync {
    fn volatility(&self, expiry: T, tenor: T, strike: T) -> Result<T, MarketDataError>;
    fn probability_density(&self, expiry: T, strike: T) -> Result<T, MarketDataError>;
    fn cumulative_probability(&self, expiry: T, strike: T) -> Result<T, MarketDataError>;
    fn expiry_domain(&self) -> (T, T);
    fn tenor_domain(&self) -> (T, T);
    fn strike_domain(&self) -> (T, T);
    fn source_instruments(&self) -> &[InstrumentId];
}

pub struct VolCube<T: Float> {
    sabr_params: SabrParameterSurface<T>,
    config: VolCubeConfig,
    source_instruments: Vec<InstrumentId>,
    forwards: Vec<Vec<T>>,
    expiries: Vec<T>,
    tenors: Vec<T>,
}
```

**所見**:
- `T: Float`ジェネリクスによるAAD互換性確保済み
- SABRパラメータサーフェスを内部保持
- source_instrumentsでtraceability確保
- forward rateをキャッシュ（Curve依存の出力点）

### 1.2 補間・カリブレーション設定

**ファイル**: [config.rs](crates/pricer_models/src/market/volcube/config.rs)

```rust
pub enum InterpolationMethod {
    Sabr, Svi, Linear, CubicSpline, FlatVol,
}

pub enum ExtrapolationMethod {
    Flat, Linear, Error,
}

pub struct VolCubeConfig {
    pub interpolation: InterpolationMethod,
    pub extrapolation: ExtrapolationMethod,
    pub strike_axis: StrikeAxisType,
    pub optimizer: OptimizerType,
    pub validate_arbitrage_free: bool,
    pub sabr_beta: Option<f64>,
    pub sabr_shift: f64,
    pub max_iterations: usize,
    pub tolerance: f64,
}
```

**所見**:
- 複数補間手法対応のenum-based static dispatch
- SABR β固定、shift対応
- arbitrage-free検証フラグ
- **ギャップ**: discount_curve_name、projection_curve_name設定が未存在

### 1.3 LazyEvaluatorパターン（参照実装）

**ファイル**: [lazy_evaluator.rs](crates/pricer_pricing/src/irs_greeks/lazy_evaluator.rs)

```rust
pub struct DependencyGraph {
    tenor_to_swaps: HashMap<TenorPoint, HashSet<SwapId>>,
    swap_to_tenors: HashMap<SwapId, HashSet<TenorPoint>>,
}

pub struct IrsLazyEvaluator<T: Float> {
    cache: HashMap<CacheKey, CachedResult<T>>,
    cache_state: HashMap<CacheKey, CacheState>,
    dependency_graph: DependencyGraph,
    stats: CacheStats,
    tape_cache: AadTapeCache,
}

pub struct AadTapeCache {
    tapes: HashMap<u64, CachedTape>,
    next_tape_id: u64,
    max_tapes: usize,
}
```

**所見**:
- 双方向依存マップ（tenor→swap、swap→tenor）
- CacheState enum（Fresh, Stale, Computing）
- AAD tape用の専用キャッシュ
- **適用方針**: Curve→VolCube依存グラフに同パターン適用

### 1.4 MarketProviderパターン

**ファイル**: [provider.rs](crates/pricer_models/src/market/provider.rs)

**所見**:
- Arc-wrapped lazy evaluation
- double-check lockingパターン
- curve cacheの既存実装
- **拡張点**: volcube cacheの追加、Curve→VolCube依存解決

### 1.5 SequentialBootstrapperパターン

**ファイル**: [engine.rs](crates/pricer_models/src/market/calibration/bootstrapping/engine.rs)

```rust
pub struct GenericBootstrapResult<T: Float> {
    pub curve: BootstrappedCurve<T>,
    pub pillars: Vec<T>,
    pub discount_factors: Vec<T>,
    pub residuals: Vec<T>,
    pub iterations: Vec<usize>,
}
```

**所見**:
- カリブレーション結果に診断データ（残差、反復回数）を含む
- pillar pointの明示
- **参考**: VolCubeCalibrationResultも同構造で設計

---

## 2. 設計判断

### 2.1 CalibrationGraph設計

**判断**: MarketProvider拡張 + VolLazyEvaluator新設

**根拠**:
- `IrsLazyEvaluator`パターンが成熟しており、VolCubeに適用可能
- `MarketProvider`への責任集約で一貫したAPI提供
- 完全新規システムより既存パターンの拡張がリスク低

**構造**:
```rust
pub struct CalibrationNode {
    id: NodeId,
    kind: CalibrationNodeKind,  // Curve | VolCube
    status: CalibrationStatus,   // Pending | Calibrated | Stale
}

pub struct CalibrationGraph {
    nodes: HashMap<NodeId, CalibrationNode>,
    dependencies: HashMap<NodeId, Vec<NodeId>>,  // child → parents
    reverse_deps: HashMap<NodeId, Vec<NodeId>>,  // parent → children
}
```

### 2.2 Curve→VolCube依存解決

**判断**: VolCubeConfig拡張 + MarketProvider統合

**根拠**:
- VolCubeはforward rate計算にCurveが必要
- 通貨別デフォルトCurve（USD=SOFR、EUR=ESTR、JPY=TONA）をconfig化
- MarketProviderでCurve→VolCubeのトポロジカル順序保証

**設計**:
```rust
// config.rs拡張
pub struct VolCubeConfig {
    // 既存フィールド...
    pub discount_curve: CurveName,
    pub projection_curve: CurveName,
}

impl VolCubeConfig {
    pub fn default_for_currency(ccy: Currency) -> Self {
        match ccy {
            Currency::USD => Self { projection_curve: "USD-SOFR".into(), .. },
            Currency::EUR => Self { projection_curve: "EUR-ESTR".into(), .. },
            Currency::JPY => Self { projection_curve: "JPY-TONA".into(), .. },
            _ => Self::default(),
        }
    }
}
```

### 2.3 Static Dispatch vs Dynamic Dispatch

**判断**: enum-based static dispatch維持

**根拠**:
- Enzyme AADとの互換性必須
- 既存`InterpolationMethod` enumパターンが確立
- trait objectはLLVM最適化阻害

**適用**:
```rust
pub enum CalibratorKind {
    Sabr(SabrCalibrator),
    Svi(SviCalibrator),
    Flat(FlatCalibrator),
    Linear(LinearCalibrator),
}
```

### 2.4 キャッシュ無効化戦略

**判断**: 依存グラフベースの選択的無効化

**根拠**:
- Curve更新時、依存VolCubeのみ無効化（全体再計算回避）
- `IrsLazyEvaluator`のCacheState enumを踏襲
- 逆依存マップで効率的な影響範囲特定

**実装方針**:
```rust
impl CalibrationGraph {
    pub fn invalidate(&mut self, node_id: NodeId) {
        // Mark node as stale
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.status = CalibrationStatus::Stale;
        }
        // Cascade to dependents
        if let Some(dependents) = self.reverse_deps.get(&node_id) {
            for dep_id in dependents {
                self.invalidate(*dep_id);  // recursive cascade
            }
        }
    }
}
```

### 2.5 AAD統合方針

**判断**: 段階的統合（Phase 3で完全パス実現）

**根拠**:
- CurveQuote→Price完全パスは複雑度高
- まずVolCube内部のAADを確立
- その後、Curve経由の間接感応度を追加

**パス設計**:
```
Level 1: VolQuote → SABRParams → InterpolatedVol
Level 2: ForwardRate → SABRCalibration → VolQuote → Price
Level 3: CurveQuote → Curve → ForwardRate → VolCube → Price
```

---

## 3. 技術的考慮事項

### 3.1 スレッドセーフティ

- `DashMap`を推奨（fine-grained locking）
- `RwLock<HashMap>`は代替案（read-heavy workloadで有効）
- カリブレーション実行中のCacheState::Computing状態管理必須

### 3.2 メモリ効率

- SABRパラメータは各(expiry, tenor)スライスで4値
- 典型的なキューブ: 15 expiry × 10 tenor × 11 strikes = 1,650点
- パラメータ格納: 15 × 10 × 4 = 600 f64 ≈ 4.8KB
- メモリ圧力は低い

### 3.3 エラーハンドリング

- `thiserror`ベースの構造化エラー
- `CalibrationError`に診断情報（残差、パラメータ境界違反）を含める
- Breeden-Litzenberger違反は警告（エラーではない）

### 3.4 WebApp可視化

- 3Dサーフェス: plotly.jsを推奨（既存curve-builderと一貫性）
- market vs fitted比較: 2Dチャート（expiry/tenor slice）
- SABRパラメータグリッド: HTML table

---

## 4. 参考ドキュメント

| ドキュメント | 場所 | 用途 |
|-------------|------|------|
| requirements.md | .kiro/specs/ir-vol-cube-calibration/ | 10要件のEARS定義 |
| gap-analysis.md | .kiro/specs/ir-vol-cube-calibration/ | 既存コードとのギャップ分析 |
| tech.md | .kiro/steering/ | 技術スタック・コーディング規約 |
| product.md | .kiro/steering/ | プロダクトビジョン |
| structure.md | .kiro/steering/ | A-I-P-Sアーキテクチャ |

---

## 5. 未解決事項

1. **Enzyme制約詳細**: VolCubeカリブレーション全体へのEnzyme適用の技術的検証が必要
2. **3D補間戦略**: per-slice SABR vs 完全3D補間のパフォーマンス比較未実施
3. **EUR ESTR Convention**: 既存EURIBOR conventionからの移行パス要確認

---

_Generated: 2026-01-25_
