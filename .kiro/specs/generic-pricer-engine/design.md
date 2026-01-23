# Technical Design: Generic Pricer Engine

## Overview

**Purpose**: 本機能は、`pricer_pricing`クレート（L3層）に統一されたプライシングAPIを提供し、クオンツ開発者が様々な金融商品を一貫した方法で評価できるようにする。

**Users**: クオンツ開発者およびリスク管理者が、単一商品プライシング、ポートフォリオレベルのバッチプライシング、Greeks計算に使用する。

**Impact**: 既存の3-stage rocketパターンを拡張し、`generic_pricer/`モジュールとして新規追加。既存コードへの影響は最小限。

### Goals

- Trade/Leg/Cashflow階層に対応した統一プライシングAPI（`get_pv`, `get_greeks`）の提供
- `MarketProvider`、`StochasticModelEnum`、`CurveEnum`、`VolSurfaceEnum`との統合
- 任意の粒度（Cashflow、Leg、Trade、Path）でのPV内訳アクセス
- Rayon並列処理によるバッチプライシングサポート
- Enzyme AD互換性の維持

### Non-Goals

- 新しい金融商品定義の追加（既存の`InstrumentEnum`拡張は別スコープ）
- XVA計算の統合（`pricer_risk`の責務）
- WebSocket/RESTエンドポイントの追加（`service_gateway`の責務）

---

## Architecture

### Existing Architecture Analysis

**現行パターン**:
- 3-stage rocket: Definition (L2) → Linking (PricingContext) → Execution (pure kernel)
- `PricingContext`は`discount_curve`と`adjustment_vol`のみ保持
- 静的ディスパッチ（enum）でEnzyme最適化を維持

**尊重すべき既存境界**:
- A-I-P-S依存方向（pricer_pricingはinfra_master、pricer_modelsに依存可、逆は不可）
- pricer_pricingはnightly Rust + Enzyme必須
- pricer_modelsのマーケットデータ構造（CurveEnum, VolSurfaceEnum, MarketProvider）

### Architecture Pattern & Boundary Map

```mermaid
graph TB
    subgraph Service["S: Service Layer"]
        CLI[service_cli]
        Gateway[service_gateway]
    end

    subgraph Pricer["P: Pricer Layer"]
        subgraph L4["L4: pricer_risk"]
            Portfolio[Portfolio Analytics]
        end

        subgraph L3["L3: pricer_pricing"]
            GenericPricer[GenericPricer Module]
            MC[Monte Carlo]
            Greeks[Greeks Module]
            Context[PricingContext]
        end

        subgraph L2["L2: pricer_models"]
            Market[MarketProvider]
            Models[StochasticModelEnum]
            Curves[CurveEnum]
            Surfaces[VolSurfaceEnum]
            FxRates[FxRate Cache]
        end

        subgraph L1["L1: pricer_core"]
            Math[Math Utils]
            Types[Core Types]
        end
    end

    subgraph Infra["I: Infra Layer"]
        Master[infra_master]
        Trade[Trade Leg Cashflow]
        Time[Calendar DayCounter]
        Currency[Currency]
    end

    CLI --> GenericPricer
    Gateway --> GenericPricer
    Portfolio --> GenericPricer

    GenericPricer --> Market
    GenericPricer --> Models
    GenericPricer --> MC
    GenericPricer --> Greeks
    GenericPricer --> Context

    Market --> Curves
    Market --> Surfaces
    Market --> FxRates
    Models --> Math

    GenericPricer --> Trade
    GenericPricer --> Time
    GenericPricer --> Currency
```

**Architecture Integration**:
- **Selected pattern**: 新規モジュール追加（Option B）、既存の3-stage rocketを拡張
- **Domain boundaries**: `generic_pricer/`モジュールがTrade → PV変換の責務を持つ
- **Existing patterns preserved**: 静的ディスパッチ、Arc-cached market data、Builderパターン
- **New components rationale**: `GenericPricer`トレイト、`PricingResult`階層、`ModelConfig`/`PricerConfig`
- **Steering compliance**: A-I-P-S依存方向維持、Enzyme互換静的ディスパッチ

### Technology Stack

| Layer | Choice / Version | Role in Feature | Notes |
|-------|------------------|-----------------|-------|
| Core Language | Rust nightly-2025-01-15 | 全コンポーネント | Enzyme AD必須 |
| Parallelisation | rayon ^1.10 | バッチプライシング (8.1, 8.2) | 既存依存 |
| Time | chrono ^0.4 | 日付計算 (7.1-7.5) | 既存依存 |
| AD Backend | Enzyme LLVM-18 | Greeks計算 (4.2) | feature-gated |
| Numeric | num-traits ^0.2 | Float trait bound | 既存依存 |

---

## System Flows

### Single Trade Pricing Flow

```mermaid
sequenceDiagram
    participant Client
    participant GenericPricer
    participant Market as MarketProvider
    participant Model as ModelConfig
    participant Kernel as PricingKernel

    Client->>GenericPricer: get_pv_with_currency(trade, date, target_ccy)
    GenericPricer->>Market: resolve_curves(trade.currencies)
    Market-->>GenericPricer: Arc<CurveEnum>
    GenericPricer->>Market: resolve_surfaces(trade.underlyings)
    Market-->>GenericPricer: Arc<VolSurfaceEnum>
    GenericPricer->>Market: get_fx_rate(leg.currency, target_ccy)
    Market-->>GenericPricer: FxRate
    GenericPricer->>Model: get_model_or_default(trade_type)
    Model-->>GenericPricer: StochasticModelEnum
    GenericPricer->>Kernel: price_trade(trade, curves, surfaces, model)
    loop For each Leg
        Kernel->>Kernel: price_leg(leg, curves)
        loop For each Cashflow
            Kernel->>Kernel: price_cashflow(cf, discount_curve)
        end
    end
    Kernel-->>GenericPricer: PricingResult<T>
    GenericPricer->>GenericPricer: convert_to_target_currency(result, fx_rates)
    GenericPricer-->>Client: Result<PricingResult<T>, PricingError>
```

**Key Decisions**:
- Stage 2（リンキング）でマーケットデータ解決（カーブ、サーフェス、**FxRate**）、Stage 3（カーネル）ではHashMap lookupなし
- 通貨換算用のFxRateも`MarketProvider`から取得（統一されたマーケットデータソース）
- 失敗時は`PricingError`で具体的なエラーコンテキストを返す

### Batch Pricing Flow

```mermaid
sequenceDiagram
    participant Client
    participant BatchPricer
    participant ThreadPool as Rayon ThreadPool
    participant Market as MarketProvider
    participant Kernel as PricingKernel

    Client->>BatchPricer: price_batch(trades, config)
    BatchPricer->>Market: preload_all_curves(currencies)
    Market-->>BatchPricer: Arc-cached curves
    BatchPricer->>ThreadPool: par_iter(trades)
    par [Parallel Processing]
        ThreadPool->>Kernel: price_trade(trade_i)
        Kernel-->>ThreadPool: PricingResult<T>
    end
    ThreadPool-->>BatchPricer: Vec<Result<PricingResult>>
    BatchPricer->>BatchPricer: aggregate_results()
    BatchPricer-->>Client: BatchPricingResult
```

**Key Decisions**:
- マーケットデータは`Arc`で共有、各スレッドはread-onlyアクセス
- 部分失敗を許容し、成功/失敗を商品別に返す

---

## Requirements Traceability

| Requirement | Summary | Components | Interfaces | Flows |
|-------------|---------|------------|------------|-------|
| 1.1 | get_pv(valuation_date) | GenericPricer, PricingKernel | GenericPricer::get_pv | Single Trade Pricing |
| 1.2 | get_pv_with_currency(date, ccy) | GenericPricer, FxConverter | GenericPricer::get_pv_with_currency | Single Trade Pricing |
| 1.3 | get_greeks(date, config) | GenericPricer, GreeksCalculator | GenericPricer::get_greeks | Greeks Calculation |
| 1.4 | PricingResult<T>型 | PricingResult | - | - |
| 1.5 | MissingMarketDataエラー | PricingError | - | - |
| 2.1 | MarketProvider/MarketSnapshot | GenericPricerContext | GenericPricerContext::new | Single Trade Pricing |
| 2.2 | CurveEnumサポート | PricingKernel | - | - |
| 2.3 | VolSurfaceEnumサポート | PricingKernel | - | - |
| 2.4 | CurveSet解決 | MarketResolver | MarketProvider::resolve_curve | Single Trade Pricing |
| 2.5 | CurveNotFound/SurfaceNotFoundエラー | MarketDataError | - | - |
| 3.1 | ModelConfig構造体 | ModelConfig | ModelConfigBuilder | - |
| 3.2 | StochasticModelEnumサポート | PricingKernel | - | - |
| 3.3 | デフォルトモデル選択 | ModelSelector | ModelSelector::default_for | - |
| 3.4 | キャリブレーション連携 | ModelConfig | ModelConfig::from_calibration | - |
| 3.5 | ModelConfig Builder | ModelConfigBuilder | ModelConfigBuilder::build | - |
| 3.6 | InvalidModelParameterエラー | ConfigError | - | - |
| 4.1 | PricerConfig構造体 | PricerConfig | PricerConfigBuilder | - |
| 4.2 | AADモード | GreeksCalculator | GreeksConfig::mode | Greeks Calculation |
| 4.3 | BumpAndRevalueモード | GreeksCalculator | GreeksConfig::mode | Greeks Calculation |
| 4.4 | PricerConfig Builder | PricerConfigBuilder | PricerConfigBuilder::build | - |
| 4.5 | スレッドローカルバッファ | BufferPool | ThreadLocalPool | Batch Pricing |
| 5.1 | Trade入力 | GenericPricer | GenericPricer::price_trade | Single Trade Pricing |
| 5.2 | InstrumentEnum入力 | GenericPricer | GenericPricer::price_instrument | Single Trade Pricing |
| 5.3 | Leg/Cashflowパース | PricingKernel | - | Single Trade Pricing |
| 5.4 | 静的ディスパッチ | All enums | - | - |
| 5.5 | UnsupportedInstrumentエラー | PricingError | - | - |
| 6.1 | Currency列挙型 | All components | - | - |
| 6.2 | 為替レート換算 | FxConverter | FxConverter::convert | Single Trade Pricing |
| 6.3 | PricingResult階層構造 | PricingResult, LegPricingResult, CashflowPricingResult | PricingResult::by_leg, by_cashflow | - |
| 6.4 | CurrencyBreakdown | CurrencyBreakdown | - | - |
| 6.5 | by_leg() | PricingResult | PricingResult::by_leg | - |
| 6.6 | by_cashflow() | PricingResult | PricingResult::by_cashflow | - |
| 6.7 | by_path() | PricingResult | PricingResult::by_path | - |
| 6.8 | FxRateNotFoundエラー | MarketDataError | - | - |
| 6.9 | デフォルト通貨設定 | PricerConfig | PricerConfig::default_currency | - |
| 7.1 | Calendar/DayCounter/Frequency | DateUtils | - | - |
| 7.2 | 営業日調整 | DateUtils | Calendar::adjust | - |
| 7.3 | time_to_maturity計算 | DateUtils | DateUtils::year_fraction | - |
| 7.4 | カーブテナー補間 | CurveEnum | CurveEnum::discount_factor | - |
| 7.5 | NaiveDate使用 | All components | - | - |
| 8.1 | price_batch() | BatchPricer | BatchPricer::price_batch | Batch Pricing |
| 8.2 | Rayon並列処理 | BatchPricer | - | Batch Pricing |
| 8.3 | Arc-cachedマーケット | MarketProvider | - | Batch Pricing |
| 8.4 | BatchPricingResult | BatchPricingResult | - | - |
| 8.5 | 部分エラー継続 | BatchPricer | - | Batch Pricing |

---

## Components and Interfaces

### Summary Table

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies | Contracts |
|-----------|--------------|--------|--------------|------------------|-----------|
| GenericPricer | L3/generic_pricer | 統一プライシングAPI提供 | 1.1-1.5, 5.1-5.5 | MarketProvider(P0), ModelConfig(P0), PricerConfig(P1) | Service |
| PricingKernel | L3/generic_pricer | Trade→PV変換の純粋計算 | 1.1, 5.3, 5.4 | CurveEnum(P0), VolSurfaceEnum(P1) | Service |
| ModelConfig | L3/generic_pricer | モデル・シミュレーション設定 | 3.1-3.6 | StochasticModelEnum(P0) | State |
| PricerConfig | L3/generic_pricer | プライサー設定 | 4.1-4.5, 6.9 | GreeksConfig(P0), Currency(P1) | State |
| PricingResult | L3/generic_pricer | 階層的プライシング結果 | 1.4, 6.3-6.7 | LegPricingResult(P0), CurrencyBreakdown(P0) | State |
| BatchPricer | L3/generic_pricer | バッチプライシング | 8.1-8.5 | GenericPricer(P0), Rayon(P0) | Service |
| FxConverter | L3/generic_pricer | 為替換算 | 6.2, 6.8 | MarketProvider(P0) | Service |
| DateUtils | L3/generic_pricer | 日付計算ヘルパー | 7.1-7.5 | infra_master::time(P0) | Service |

---

### L3: pricer_pricing/generic_pricer

#### GenericPricer (Trait)

| Field | Detail |
|-------|--------|
| Intent | Trade/InstrumentEnumに対する統一プライシングAPI |
| Requirements | 1.1, 1.2, 1.3, 1.4, 1.5, 5.1, 5.2 |

**Responsibilities & Constraints**
- TradeまたはInstrumentEnumを受け取り、PricingResult<T>を返す
- マーケットデータ解決（Stage 2）とカーネル実行（Stage 3）の調整
- Enzyme互換性のため`T: Float`ジェネリック

**Dependencies**
- Inbound: service_cli, service_gateway, pricer_risk — プライシングAPI呼び出し (P0)
- Outbound: MarketProvider — マーケットデータ取得 (P0)
- Outbound: PricingKernel — 実際の計算実行 (P0)
- External: rayon — 並列処理 (P1)

**Contracts**: Service [x]

##### Service Interface

```rust
/// 汎用プライサートレイト
/// T: Float制約でEnzyme AD互換
pub trait GenericPricer<T: Float> {
    /// 評価日時点のPVを計算
    fn get_pv(
        &self,
        trade: &Trade,
        valuation_date: Date,
    ) -> Result<PricingResult<T>, PricingError>;

    /// 指定通貨建てでPVを計算
    fn get_pv_with_currency(
        &self,
        trade: &Trade,
        valuation_date: Date,
        target_currency: Currency,
    ) -> Result<PricingResult<T>, PricingError>;

    /// Greeks計算
    fn get_greeks(
        &self,
        trade: &Trade,
        valuation_date: Date,
        greeks_config: &GreeksConfig,
    ) -> Result<GreeksResult<T>, GreeksError>;
}
```

- Preconditions: `trade`が有効なTrade構造、`valuation_date`が妥当な日付
- Postconditions: 成功時はPricingResult<T>を返却、失敗時はPricingErrorで具体的理由
- Invariants: 同一入力に対して同一結果（決定論的、seedが同じ場合）

**Implementation Notes**
- Integration: `GenericPricerEngine`構造体がトレイトを実装
- Validation: Trade有効性チェック、マーケットデータ存在確認
- Risks: マーケットデータ欠落時のエラーハンドリング

---

#### ModelConfig

| Field | Detail |
|-------|--------|
| Intent | モデルタイプとシミュレーションパラメータの設定 |
| Requirements | 3.1, 3.2, 3.3, 3.4, 3.5, 3.6 |

**Responsibilities & Constraints**
- `StochasticModelEnum`選択とシミュレーションパラメータ（パス数、ステップ数、シード）の保持
- Builderパターンで構築
- パラメータ検証（num_paths > 0、num_steps > 0等）

**Dependencies**
- Inbound: GenericPricer — モデル設定取得 (P0)
- Outbound: StochasticModelEnum — モデル定義 (P0)
- Outbound: pricer_models::market::calibration — キャリブレーション結果取得 (P1)

**Contracts**: State [x]

##### State Management

```rust
/// モデル構成
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// 使用するモデル（None時はデフォルト選択）
    pub model: Option<StochasticModelEnum<f64>>,
    /// シミュレーションパス数
    pub num_paths: usize,
    /// 時間ステップ数
    pub num_steps: usize,
    /// 乱数シード（再現性確保用）
    pub seed: Option<u64>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model: None,
            num_paths: 10_000,
            num_steps: 100,
            seed: None,
        }
    }
}

/// ModelConfig Builder
#[derive(Debug, Default)]
pub struct ModelConfigBuilder {
    model: Option<StochasticModelEnum<f64>>,
    num_paths: Option<usize>,
    num_steps: Option<usize>,
    seed: Option<u64>,
}

impl ModelConfigBuilder {
    pub fn model(mut self, model: StochasticModelEnum<f64>) -> Self;
    pub fn num_paths(mut self, n: usize) -> Self;
    pub fn num_steps(mut self, n: usize) -> Self;
    pub fn seed(mut self, seed: u64) -> Self;
    pub fn build(self) -> Result<ModelConfig, ConfigError>;
}
```

- Persistence: メモリ内のみ（永続化なし）
- Consistency: 不変（immutable after build）
- Concurrency: Clone可能、スレッド間共有安全

---

#### PricerConfig

| Field | Detail |
|-------|--------|
| Intent | Greeks計算モードと出力設定 |
| Requirements | 4.1, 4.2, 4.3, 4.4, 4.5, 6.9 |

**Responsibilities & Constraints**
- GreeksConfig（計算モード、バンプ幅）の保持
- デフォルト出力通貨の設定
- スレッドローカルバッファ使用フラグ

**Dependencies**
- Inbound: GenericPricer — 設定取得 (P0)
- Outbound: GreeksConfig — Greeks設定 (P0)
- Outbound: Currency — 通貨定義 (P1)

**Contracts**: State [x]

##### State Management

```rust
/// プライサー構成
#[derive(Debug, Clone)]
pub struct PricerConfig {
    /// Greeks計算設定
    pub greeks_config: GreeksConfig,
    /// デフォルト出力通貨
    pub default_currency: Currency,
    /// スレッドローカルバッファ使用
    pub use_thread_local_buffers: bool,
}

impl Default for PricerConfig {
    fn default() -> Self {
        Self {
            greeks_config: GreeksConfig::default(),
            default_currency: Currency::USD,
            use_thread_local_buffers: true,
        }
    }
}

/// PricerConfig Builder
#[derive(Debug, Default)]
pub struct PricerConfigBuilder {
    greeks_config: Option<GreeksConfig>,
    default_currency: Option<Currency>,
    use_thread_local_buffers: Option<bool>,
}

impl PricerConfigBuilder {
    pub fn greeks_config(mut self, config: GreeksConfig) -> Self;
    pub fn default_currency(mut self, currency: Currency) -> Self;
    pub fn use_thread_local_buffers(mut self, use_buffers: bool) -> Self;
    pub fn build(self) -> Result<PricerConfig, ConfigError>;
}
```

---

#### PricingResult

| Field | Detail |
|-------|--------|
| Intent | 階層的プライシング結果（Trade → Leg → Cashflow） |
| Requirements | 1.4, 6.3, 6.4, 6.5, 6.6, 6.7 |

**Responsibilities & Constraints**
- Trade/Leg/Cashflow階層に対応したPV内訳の保持
- 通貨別集計（CurrencyBreakdown）
- MCシミュレーション時のパス分布（オプション）
- `T: Float`ジェネリックでAD互換

**Dependencies**
- Inbound: GenericPricer — 結果返却 (P0)
- Outbound: LegPricingResult — Leg単位結果 (P0)
- Outbound: CurrencyBreakdown — 通貨別集計 (P0)

**Contracts**: State [x]

##### State Management

```rust
/// プライシング結果（Trade単位）
#[derive(Debug, Clone)]
pub struct PricingResult<T: Float> {
    /// 合計PV（デフォルト通貨建て）
    pub total_pv: T,
    /// 各Legの結果
    pub legs: Vec<LegPricingResult<T>>,
    /// 通貨別内訳
    pub currency_breakdown: CurrencyBreakdown<T>,
    /// パス分布（MC計算時のみ）
    pub path_distribution: Option<PathDistribution<T>>,
    /// 報告通貨
    pub reporting_currency: Currency,
}

impl<T: Float> PricingResult<T> {
    /// Leg単位のPV集計を返す
    pub fn by_leg(&self) -> &[LegPricingResult<T>];

    /// Cashflow単位のPV詳細を返す
    pub fn by_cashflow(&self) -> Vec<&CashflowPricingResult<T>>;

    /// パス単位のPV分布を返す（MC計算時のみ）
    pub fn by_path(&self) -> Option<&PathDistribution<T>>;
}

/// Leg単位プライシング結果
#[derive(Debug, Clone)]
pub struct LegPricingResult<T: Float> {
    pub pv: T,
    pub direction: Direction,
    pub currency: Currency,
    pub cashflows: Vec<CashflowPricingResult<T>>,
}

/// Cashflow単位プライシング結果
#[derive(Debug, Clone)]
pub struct CashflowPricingResult<T: Float> {
    pub pv: T,
    pub payment_date: Date,
    pub discount_factor: T,
    pub currency: Currency,
}

/// 通貨別集計
#[derive(Debug, Clone)]
pub struct CurrencyBreakdown<T: Float> {
    pub pv_by_currency: HashMap<Currency, T>,
    pub total_pv: T,
    pub reporting_currency: Currency,
}

/// パス分布（MC用）
#[derive(Debug, Clone)]
pub struct PathDistribution<T: Float> {
    pub mean: T,
    pub std_dev: T,
    pub percentiles: Vec<(f64, T)>, // (percentile, value)
    pub path_count: usize,
}
```

---

#### BatchPricer

| Field | Detail |
|-------|--------|
| Intent | 複数商品の並列バッチプライシング |
| Requirements | 8.1, 8.2, 8.3, 8.4, 8.5 |

**Responsibilities & Constraints**
- Rayon並列処理による複数商品同時プライシング
- Arc-cachedマーケットデータの共有
- 部分失敗時の継続処理
- スレッドローカルバッファプールの管理

**Dependencies**
- Inbound: service_cli, pricer_risk — バッチ呼び出し (P0)
- Outbound: GenericPricer — 個別プライシング (P0)
- Outbound: MarketProvider — マーケットデータ共有 (P0)
- External: rayon — 並列処理 (P0)

**Contracts**: Service [x]

##### Service Interface

```rust
/// バッチプライサー
pub struct BatchPricer<'a> {
    market: Arc<MarketProvider>,
    model_config: &'a ModelConfig,
    pricer_config: &'a PricerConfig,
}

impl<'a> BatchPricer<'a> {
    /// バッチプライシング実行
    pub fn price_batch(
        &self,
        trades: &[Trade],
        valuation_date: Date,
    ) -> BatchPricingResult;
}

/// バッチプライシング結果
#[derive(Debug)]
pub struct BatchPricingResult {
    /// 成功したプライシング結果
    pub successes: Vec<(TradeId, PricingResult<f64>)>,
    /// 失敗したプライシング
    pub failures: Vec<(TradeId, PricingError)>,
    /// 処理統計
    pub stats: BatchStats,
}

/// バッチ処理統計
#[derive(Debug)]
pub struct BatchStats {
    pub total_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub elapsed_ms: u64,
}
```

- Preconditions: `trades`が空でない、`market`が有効
- Postconditions: 全商品が成功または失敗として分類
- Invariants: 部分失敗でも他商品の処理は継続

---

#### FxConverter

| Field | Detail |
|-------|--------|
| Intent | MarketProviderからFxRateを取得し、通貨間換算を実行 |
| Requirements | 6.2, 6.8 |

**Responsibilities & Constraints**
- `MarketProvider`からFxRateを取得（統一されたマーケットデータソース）
- 通貨ペア間のレート換算
- FxRate不在時のエラーハンドリング

**Dependencies**
- Inbound: GenericPricer — 通貨換算要求 (P0)
- Outbound: MarketProvider — FxRate取得 (P0)

**Contracts**: Service [x]

##### Service Interface

```rust
/// 為替換算ユーティリティ
/// MarketProviderからFxRateを取得して換算を実行
pub struct FxConverter<'a> {
    market: &'a MarketProvider,
}

impl<'a> FxConverter<'a> {
    /// 新しいFxConverterを作成
    pub fn new(market: &'a MarketProvider) -> Self;

    /// 金額を換算
    /// MarketProviderからFxRateを取得し、base通貨からquote通貨へ換算
    pub fn convert<T: Float>(
        &self,
        amount: T,
        from: Currency,
        to: Currency,
        valuation_date: Date,
    ) -> Result<T, MarketDataError>;

    /// PricingResultを指定通貨に換算
    pub fn convert_result<T: Float>(
        &self,
        result: PricingResult<T>,
        target_currency: Currency,
        valuation_date: Date,
    ) -> Result<PricingResult<T>, MarketDataError>;
}
```

- Preconditions: `market`が有効、通貨ペアがサポート対象
- Postconditions: 換算後の金額を返却、またはFxRateNotFoundエラー
- Invariants: 同一通貨間の換算はレート1.0を使用

**Implementation Notes**
- Integration: `MarketProvider::get_fx_rate(base, quote)`を使用
- Validation: 通貨ペアの存在確認
- Risks: クロスレート計算が必要な場合の対応（USD経由等）

---

## Data Models

### Domain Model

```mermaid
erDiagram
    Trade ||--o{ Leg : contains
    Leg ||--o{ Cashflow : contains

    PricingResult ||--o{ LegPricingResult : contains
    LegPricingResult ||--o{ CashflowPricingResult : contains
    PricingResult ||--|| CurrencyBreakdown : has
    PricingResult ||--o| PathDistribution : may_have

    ModelConfig ||--o| StochasticModelEnum : uses
    PricerConfig ||--|| GreeksConfig : contains

    GenericPricerEngine ||--|| ModelConfig : configured_by
    GenericPricerEngine ||--|| PricerConfig : configured_by
    GenericPricerEngine ||--|| MarketProvider : uses
```

**Aggregates**:
- `PricingResult`: 単一プライシングの結果集約
- `BatchPricingResult`: バッチプライシングの結果集約

**Entities**:
- `Trade`, `Leg`, `Cashflow`（既存、infra_master）

**Value Objects**:
- `ModelConfig`, `PricerConfig`, `CurrencyBreakdown`, `PathDistribution`

**Invariants**:
- `PricingResult.total_pv` = Σ(legs.pv * direction.sign())
- `CurrencyBreakdown.total_pv` = Σ(pv_by_currency.values())変換後

---

## Error Handling

### Error Strategy

Generic Pricer Engineは以下のエラー戦略を採用:

1. **Fail Fast**: 入力検証（Trade構造、設定パラメータ）は早期に実行
2. **Graceful Degradation**: バッチ処理では部分失敗を許容
3. **Contextual Errors**: エラーメッセージに具体的なコンテキスト（TradeId、通貨ペア等）を含む

### Error Categories and Responses

**User Errors (4xx equivalent)**:
- `ConfigError::InvalidModelParameter` — 不正なモデルパラメータ（num_paths = 0等）
- `PricingError::UnsupportedInstrument` — 未対応の商品タイプ

**System Errors (5xx equivalent)**:
- `PricingError::MissingMarketData` — 必要なマーケットデータが欠落
- `MarketDataError::CurveNotFound` — 指定されたカーブが存在しない
- `MarketDataError::SurfaceNotFound` — 指定されたサーフェスが存在しない
- `MarketDataError::FxRateNotFound` — 為替レートが利用不可

**Business Logic Errors**:
- モデルキャリブレーション失敗 — CalibrationError

### Error Types Extension

```rust
/// ConfigError: 設定関連エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid model parameter: {0}")]
    InvalidModelParameter(String),

    #[error("Invalid pricer config: {0}")]
    InvalidPricerConfig(String),
}

/// MarketDataError拡張（既存に追加）
#[derive(Debug, Clone, thiserror::Error)]
pub enum MarketDataError {
    // ... 既存のvariant ...

    #[error("FX rate not found: {base}/{quote}")]
    FxRateNotFound { base: Currency, quote: Currency },

    #[error("Volatility surface not found: {name}")]
    SurfaceNotFound { name: String },
}
```

---

## Testing Strategy

### Unit Tests

- `ModelConfig::validate()` — パラメータ検証（num_paths > 0、num_steps > 0）
- `PricerConfig::validate()` — 設定検証（default_currency有効性）
- `PricingResult::by_leg()` — Leg単位集計の正確性
- `PricingResult::by_cashflow()` — Cashflow単位集計の正確性
- `CurrencyBreakdown::from_legs()` — 通貨別集計の正確性

### Integration Tests

- `GenericPricer::get_pv()` — Trade → PV変換のE2E
- `GenericPricer::get_pv_with_currency()` — 通貨換算を含むプライシング
- `BatchPricer::price_batch()` — 並列バッチプライシング
- `FxConverter::convert()` — 為替換算の正確性
- Market data resolution — CurveSet/VolSurfaceEnum解決

### Performance Tests

- バッチプライシング並列効率 — 8コアで80%以上のスケーリング
- メモリ使用量 — 1000商品バッチで許容範囲内
- スレッドローカルバッファ効果 — アロケーション削減の確認

---

## Optional Sections

### Performance & Scalability

**Target Metrics**:
- 単一商品プライシング: < 1ms（解析解）、< 100ms（MC 10,000パス）
- バッチプライシング: 線形スケーリング（商品数に対して）
- 並列効率: 8コアで80%以上

**Scaling Approaches**:
- 水平: Rayon work-stealingによる自動負荷分散
- メモリ: Arc-cachedマーケットデータ、スレッドローカルバッファ

**Caching Strategy**:
- MarketProvider: `RwLock<HashMap<Currency, Arc<CurveEnum>>>`
- プライシング結果: キャッシュなし（都度計算）

---

## Supporting References

詳細な調査結果とアーキテクチャ評価は[research.md](.kiro/specs/generic-pricer-engine/research.md)を参照。

- 3-Stage Rocketパターン分析
- Trade/Leg/Cashflow階層構造調査
- Enzyme AD互換性制約
- 為替レート統合オプション評価
