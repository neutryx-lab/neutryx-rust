# Requirements Document

## Introduction

本仕様書は、neutryx-rustライブラリのクレート構成を全デリバティブ評価に対応できるよう再設計するための要件を定義する。現行の4層アーキテクチャを基盤としつつ、株式デリバティブに加えて金利・クレジット・為替・コモディティ・エキゾチックデリバティブをカバーする拡張性を確保する。

**クレート命名規則（変更）**:

- `pricer_core` → 基盤（数学・型・市場データ）
- `pricer_models` → 商品定義・確率モデル
- `pricer_pricing` → MC・AD・評価エンジン（旧pricer_kernel）
- `pricer_risk` → リスク計算・XVA・エクスポージャー（旧pricer_xva）

**命名規則の設計原則**:
アルファベット順（C < M < P < R）と依存順（L1 → L2 → L3 → L4）が一致するよう設計。
`ls`やIDEでの表示順が自然に依存階層を反映する。

**現状の課題**:

- 現行設計は株式オプション（VanillaOption, Asian, Barrier, Lookback）に特化
- 金利デリバティブ（IRS, Swaption）に必要なスケジュール生成・複数カーブ対応が不足
- クレジットデリバティブ（CDS）に必要なハザードレート・生存確率の基盤が未整備
- 為替デリバティブに必要なマルチカレンシー対応が限定的
- エキゾチックデリバティブ（Variance Swap, Autocallable等）の基盤が未整備

**目標**:

- アセットクラス非依存の商品階層設計
- 複数市場データソース（カーブ・サーフェス）の統一管理
- モデル選択とキャリブレーションの柔軟なフレームワーク
- エキゾチック商品への拡張性確保

## Requirements

### Requirement 1: アセットクラス別商品階層

**Objective:** As a クオンツ開発者, I want アセットクラスごとに商品を分類・整理できる階層構造, so that 新規商品の追加が既存コードに影響を与えず拡張可能になる

#### Acceptance Criteria 1

1. The pricer_models crate shall 商品を「equity」「rates」「credit」「fx」「commodity」「exotic」のアセットクラス別サブモジュールに分類する
2. When 新しいアセットクラスの商品を追加する場合, the Instrument enum shall 静的ディスパッチを維持しつつアセットクラス別のサブenumで拡張可能である
3. The pricer_models crate shall 共通の`Instrument`トレイトを提供し、全商品がprice(), greeks(), cashflows()メソッドを実装する
4. Where 金利デリバティブが含まれる場合, the pricer_models/schedules module shall Schedule構造体（支払日・計算期間・日数計算規約）を提供する
5. While 既存のVanillaOption, Forward, Swapが存在する間, the architecture shall 後方互換性を維持しAPIの破壊的変更を回避する

### Requirement 2: マルチカーブ市場データ基盤

**Objective:** As a リスク管理者, I want 複数のイールドカーブとクレジットカーブを統一的に管理できる基盤, so that 金利デリバティブとXVA計算で適切なディスカウント・フォワードレートを使用できる

#### Acceptance Criteria 2

1. The pricer_core/market_data module shall 複数のイールドカーブ（OIS, SOFR, TONAR等）を名前付きで登録・取得できるCurveSet構造体を提供する
2. When デリバティブを評価する場合, the pricing engine shall ディスカウントカーブとフォワードカーブを分離して指定可能である
3. The pricer_core crate shall CreditCurveトレイトを提供し、ハザードレート・生存確率・デフォルト確率の計算を抽象化する
4. If カーブ構築に必要なマーケットデータが不足している場合, the MarketDataError shall 欠損データの詳細を含むエラーメッセージを返す
5. The market_data module shall すべてのカーブ・サーフェスを`T: Float`でジェネリックに保ち、AD互換性を維持する

### Requirement 3: 確率モデル拡張フレームワーク

**Objective:** As a クオンツ研究者, I want 金利モデル（Hull-White, CIR）やジャンプ拡散モデルを追加できるフレームワーク, so that 様々なアセットクラスに適したモデルで評価できる

#### Acceptance Criteria 3

1. The StochasticModel trait shall `num_factors()`メソッドを提供し、1ファクター/2ファクター/マルチファクターモデルを統一的に扱う
2. When Hull-Whiteモデルを使用する場合, the pricer_models/models/rates module shall mean-reversion速度とボラティリティパラメータを受け取り、短期金利パスを生成する
3. The StochasticModelEnum shall 静的ディスパッチを維持しつつ、新規モデル追加時にenum variantの追加のみで拡張可能である
4. Where 相関を持つ複数ファクターモデルが必要な場合, the pricer_models/models/hybrid module shall 相関行列を受け取りコレスキー分解で相関ブラウン運動を生成する
5. The pricer_pricing crate shall モデルキャリブレーション用のインターフェース（Calibrator trait）を提供する

### Requirement 4: 金利デリバティブ対応

**Objective:** As a 金利トレーダー, I want IRS、Swaption、Cap/Floorの評価機能, so that 金利デリバティブのポートフォリオをXVA計算に含められる

#### Acceptance Criteria 4

1. The pricer_models/instruments/rates module shall InterestRateSwap構造体（固定レグ・変動レグ・ノーショナル・日数計算規約）を提供する
2. When IRSを評価する場合, the pricing engine shall 変動レグのフォワードレート計算とディスカウントを適切なカーブで実行する
3. The pricer_models/instruments/rates module shall Swaption構造体（underlying swap, expiry, strike, option type）を提供する
4. Where Black or Bachelierモデルが選択された場合, the pricer_models/analytical module shall Swaption価格の解析解を提供する
5. The pricer_models/schedules module shall IMM日付、Modified Following、Act/360等の標準的な日付規約をサポートする

### Requirement 5: クレジットデリバティブ対応

**Objective:** As a クレジットアナリスト, I want CDSの評価機能とハザードレート計算, so that クレジットエクスポージャーとCVA/DVA計算の精度を向上できる

#### Acceptance Criteria 5

1. The pricer_models/instruments/credit module shall CreditDefaultSwap構造体（参照エンティティ、ノーショナル、スプレッド、満期）を提供する
2. When CDSを評価する場合, the pricing engine shall ハザードレートカーブから生存確率を計算し、プロテクションレグとプレミアムレグのPVを算出する
3. The pricer_core/market_data module shall HazardRateCurve構造体（ハザードレートの期間構造）を提供する
4. If デフォルトイベントが発生した場合のシミュレーションにおいて, the Monte Carlo engine shall デフォルト時刻を生存確率の逆関数でサンプリングする
5. The pricer_risk crate shall Wrong-Way Risk（WWR）を考慮したCVA計算オプションを提供する

### Requirement 6: 為替デリバティブ対応

**Objective:** As a FXトレーダー, I want FXオプションとFXフォワードの評価機能, so that マルチカレンシーポートフォリオのリスク管理ができる

#### Acceptance Criteria 6

1. The pricer_models/instruments/fx module shall FxOption構造体（通貨ペア、ストライク、満期、オプションタイプ）を提供する
2. When FXオプションを評価する場合, the pricing engine shall 国内・外国の金利カーブを使用したGarman-Kohlhagenモデルを適用する
3. The pricer_core/types module shall CurrencyPair構造体を提供し、ベース通貨・クォート通貨・スポットレートを管理する
4. Where マルチカレンシーXVA計算が必要な場合, the pricer_risk crate shall 各取引の決済通貨と評価通貨の変換を自動的に処理する
5. The pricer_core/market_data module shall FxVolatilitySurface（デルタ・満期グリッドでのボラティリティ）を提供する

### Requirement 7: レイヤー構成とフォルダ構造

**Objective:** As a ライブラリメンテナー, I want 4層アーキテクチャの責務を明確化しフォルダ構造を整理, so that 将来のアセットクラス追加が容易になる

#### Acceptance Criteria 7

1. The workspace shall 以下の命名規則でクレートを構成する: pricer_core (L1), pricer_models (L2), pricer_pricing (L3, 旧kernel), pricer_risk (L4, 旧xva)
2. The pricer_models/instruments module shall アセットクラス別サブモジュール構成（equity/, rates/, credit/, fx/, commodity/, exotic/）を採用する
3. The pricer_models/models module shall モデルカテゴリ別サブモジュール構成（equity/, rates/, hybrid/）を採用する
4. The pricer_models crate shall feature flagによりアセットクラス別の条件付きコンパイルをサポートする（例: `features = ["rates", "credit"]`）
5. The dependency graph shall 常にL1→L2→L3→L4の方向のみを許可し、循環依存を禁止する
6. When 新規クレートを追加する場合, the workspace shall 将来的に`pricer_rates`, `pricer_credit`等のアセットクラス別クレート分割をサポートする構造を維持する

### Requirement 8: キャリブレーション基盤

**Objective:** As a クオンツ, I want モデルパラメータを市場データにキャリブレートする基盤, so that モデルが市場整合的な価格を出力する

#### Acceptance Criteria 8

1. The pricer_pricing crate shall Calibratorトレイト（calibrate(), objective_function(), constraints()）を提供する
2. When ボラティリティサーフェスにキャリブレートする場合, the calibrator shall 市場のオプション価格とモデル価格の差を最小化する
3. The pricer_core/math/solvers module shall Levenberg-Marquardtまたは他の非線形最小二乗法ソルバーを提供する
4. If キャリブレーションが収束しない場合, the CalibrationError shall 残差、イテレーション数、収束判定基準を含む詳細情報を返す
5. The calibration framework shall Enzyme ADを活用した勾配計算によるキャリブレーション高速化をサポートする

### Requirement 9: リスクファクター管理

**Objective:** As a リスクマネージャー, I want 複数のリスクファクター（金利、クレジット、FX）を統一的に管理, so that ポートフォリオ全体の感応度分析とストレステストができる

#### Acceptance Criteria 9

1. The pricer_core crate shall RiskFactorトレイト（factor_type(), bump(), scenario()）を提供する
2. When バンプシナリオを生成する場合, the risk framework shall 各リスクファクターを独立または同時にシフトできる
3. The pricer_risk crate shall ポートフォリオレベルのDelta、Gamma、Vegaを計算するGreeksAggregator構造体を提供する
4. Where ストレステストシナリオが定義された場合, the scenario engine shall 複数リスクファクターを同時にシフトしたPnLを計算する
5. The pricer_risk/risk_factors module shall 金利カーブバンプ（パラレル、ツイスト、バタフライ）のプリセットシナリオを提供する

### Requirement 10: パフォーマンスとメモリ効率

**Objective:** As a プロダクションエンジニア, I want 大規模ポートフォリオでも高速に評価できる性能, so that リアルタイムリスク計算とバッチ処理の両方に対応できる

#### Acceptance Criteria 10

1. The architecture shall Structure of Arrays (SoA)レイアウトをL4（pricer_risk）で維持し、ベクトル化最適化を可能にする
2. When 10,000件以上の取引を評価する場合, the parallel module shall Rayonによる自動並列化でCPUコアを効率的に使用する
3. The pricer_pricing crate shall メモリアロケーションを最小化するためのワークスペースバッファパターンを全評価パスで適用する
4. If メモリ制約がある環境で実行する場合, the checkpointing module shall メモリ使用量と再計算のトレードオフを設定可能にする
5. The benchmark suite shall 各アセットクラスの代表的な商品で`criterion`ベンチマークを提供し、パフォーマンス回帰を検出する

### Requirement 11: エキゾチックデリバティブ対応

**Objective:** As a ストラクチャラー, I want Variance Swap、Autocallable、Cliquetなどのエキゾチック商品の評価機能, so that 仕組商品のプライシングとリスク管理ができる

#### Acceptance Criteria 11

1. The pricer_models/instruments/exotic module shall VarianceSwap構造体（実現ボラティリティ vs ストライク、バリアンス・ノーショナル）を提供する
2. When Variance Swapを評価する場合, the pricing engine shall ログリターンの二乗和から実現バリアンスを計算し、レプリケーションまたはMCで公正バリアンスストライクを算出する
3. The pricer_models/instruments/exotic module shall Cliquet構造体（リセット日、ローカルキャップ/フロア、グローバルキャップ/フロア）を提供する
4. The pricer_models/instruments/exotic module shall Autocallable構造体（観測日、早期償還バリア、クーポン条件、ノックインプット）を提供する
5. Where 複数原資産オプション（Rainbow）が必要な場合, the pricer_models/instruments/exotic module shall BestOf/WorstOf構造体と相関パラメータを提供する
6. The pricer_models/instruments/exotic module shall QuantoOption構造体（原資産通貨、決済通貨、quanto調整）を提供する
7. When Bermudan Swaptionを評価する場合, the pricer_pricing crate shall Longstaff-Schwartz法による早期行使境界の推定を提供する
8. The pricer_models/instruments/exotic module shall VolatilitySwap構造体（実現ボラティリティのペイオフ）を提供し、バリアンススワップとの違いを明確化する

## Appendix: 推奨フォルダ構造

### A.1 pricer_core (L1: 基盤レイヤー)

```text
pricer_core/src/
├── lib.rs
├── market_data/
│   ├── mod.rs
│   ├── error.rs           → MarketDataError
│   ├── curves/
│   │   ├── mod.rs
│   │   ├── traits.rs      → YieldCurve trait
│   │   ├── flat.rs        → FlatCurve
│   │   ├── interpolated.rs → InterpolatedCurve (linear, log-linear, cubic)
│   │   ├── curve_set.rs   → CurveSet (OIS, SOFR, TONAR等の名前付き管理)
│   │   └── credit.rs      → CreditCurve trait, HazardRateCurve
│   ├── surfaces/
│   │   ├── mod.rs
│   │   ├── traits.rs      → VolatilitySurface trait
│   │   ├── flat.rs        → FlatVolSurface
│   │   ├── grid.rs        → GridVolSurface (strike×maturity)
│   │   └── fx.rs          → FxVolatilitySurface (delta×maturity)
│   └── interpolation/
│       ├── mod.rs
│       ├── linear.rs
│       ├── cubic_spline.rs
│       └── sabr.rs        → SABR補間
├── types/
│   ├── mod.rs
│   ├── currency.rs        → Currency enum (5通貨→拡張可能)
│   ├── currency_pair.rs   → CurrencyPair (base/quote/spot)
│   ├── time.rs            → Date, DayCountConvention
│   ├── schedule.rs        → ScheduleSpec (生成パラメータ)
│   └── error.rs           → CurrencyError, TimeError
├── math/
│   ├── mod.rs
│   ├── distributions/
│   │   ├── mod.rs
│   │   ├── normal.rs      → N(d), N'(d)
│   │   └── poisson.rs     → Poisson分布
│   ├── solvers/
│   │   ├── mod.rs
│   │   ├── newton.rs      → Newton-Raphson
│   │   ├── brent.rs       → Brent法
│   │   └── levenberg_marquardt.rs → LM法
│   └── special/
│       ├── mod.rs
│       └── gamma.rs       → Gamma関数
├── traits/
│   ├── mod.rs
│   └── risk_factor.rs     → RiskFactor trait (factor_type, bump, scenario)
└── error.rs               → 共通エラー型
```

### A.2 pricer_models (L2: 商品・モデル定義)

```text
pricer_models/src/
├── lib.rs
├── instruments/
│   ├── mod.rs             → Instrument trait, InstrumentEnum
│   ├── traits.rs          → 共通トレイト (price, greeks, cashflows)
│   ├── equity/
│   │   ├── mod.rs
│   │   ├── vanilla.rs     → VanillaOption
│   │   ├── barrier.rs     → BarrierOption (8 variants)
│   │   ├── asian.rs       → AsianOption (arithmetic/geometric)
│   │   └── lookback.rs    → LookbackOption (fixed/floating)
│   ├── rates/
│   │   ├── mod.rs
│   │   ├── swap.rs        → InterestRateSwap, CrossCurrencySwap
│   │   ├── swaption.rs    → Swaption
│   │   ├── capfloor.rs    → Cap, Floor, Collar
│   │   └── fra.rs         → ForwardRateAgreement
│   ├── credit/
│   │   ├── mod.rs
│   │   └── cds.rs         → CreditDefaultSwap
│   ├── fx/
│   │   ├── mod.rs
│   │   ├── option.rs      → FxOption
│   │   └── forward.rs     → FxForward
│   ├── commodity/
│   │   ├── mod.rs
│   │   ├── forward.rs     → CommodityForward
│   │   └── option.rs      → CommodityOption
│   └── exotic/
│       ├── mod.rs
│       ├── variance.rs    → VarianceSwap, VolatilitySwap
│       ├── cliquet.rs     → Cliquet (Ratchet)
│       ├── autocall.rs    → Autocallable
│       ├── rainbow.rs     → BestOf, WorstOf, Spread
│       ├── quanto.rs      → QuantoOption
│       └── bermudan.rs    → BermudanSwaption
├── models/
│   ├── mod.rs             → StochasticModel trait, StochasticModelEnum
│   ├── traits.rs          → State trait, num_factors()
│   ├── equity/
│   │   ├── mod.rs
│   │   ├── gbm.rs         → GeometricBrownianMotion
│   │   ├── local_vol.rs   → LocalVolatility
│   │   └── heston.rs      → Heston
│   ├── rates/
│   │   ├── mod.rs
│   │   ├── hull_white.rs  → HullWhite (1F)
│   │   ├── cir.rs         → CoxIngersollRoss
│   │   ├── g2pp.rs        → G2++ (2F)
│   │   └── lmm.rs         → LIBOR Market Model
│   ├── credit/
│   │   ├── mod.rs
│   │   └── jt.rs          → Jarrow-Turnbull (reduced-form)
│   └── hybrid/
│       ├── mod.rs
│       └── correlated.rs  → CorrelatedModels (Cholesky)
├── schedules/
│   ├── mod.rs
│   ├── schedule.rs        → Schedule, Period
│   ├── conventions.rs     → BusinessDayConvention, DateRoll
│   ├── calendars.rs       → Calendar, HolidayCalendar
│   └── generators.rs      → ScheduleGenerator (IMM, Quarterly等)
└── analytical/
    ├── mod.rs
    ├── black_scholes.rs   → BS formula
    ├── garman_kohlhagen.rs → FX option
    ├── black76.rs         → Swaption, Cap/Floor
    └── bachelier.rs       → Normal model
```

### A.3 pricer_pricing (L3: 評価エンジン、旧pricer_kernel)

```text
pricer_pricing/src/
├── lib.rs
├── mc/
│   ├── mod.rs             → MonteCarloEngine
│   ├── engine.rs          → MCEngine (main orchestrator)
│   ├── config.rs          → MCConfig (num_paths, time_steps)
│   ├── path_generator.rs  → PathGenerator trait
│   └── payoff.rs          → Payoff trait
├── rng/
│   ├── mod.rs
│   ├── traits.rs          → RandomNumberGenerator trait
│   ├── sobol.rs           → Sobol sequence (QMC)
│   ├── mersenne.rs        → MersenneTwister
│   └── brownian.rs        → BrownianBridge, AntitheticVariate
├── enzyme/
│   ├── mod.rs             → Enzyme AD統合
│   ├── wrappers.rs        → Dual64ラッパー
│   └── tape.rs            → AD tape管理
├── calibration/
│   ├── mod.rs
│   ├── traits.rs          → Calibrator trait (calibrate, objective, constraints)
│   ├── optimizer.rs       → CalibrationOptimizer
│   ├── targets.rs         → CalibrationTarget (価格、IV等)
│   └── error.rs           → CalibrationError
├── path_dependent/
│   ├── mod.rs
│   ├── asian.rs           → Asian averaging
│   ├── barrier.rs         → Barrier monitoring
│   ├── lookback.rs        → Lookback tracking
│   └── early_exercise.rs  → Early exercise detection
├── american/
│   ├── mod.rs
│   ├── lsm.rs             → Longstaff-Schwartz Method
│   ├── regression.rs      → Basis function regression
│   └── boundary.rs        → Exercise boundary
├── greeks/
│   ├── mod.rs
│   ├── config.rs          → GreeksConfig
│   ├── mode.rs            → GreeksMode (AAD, Bump, etc.)
│   └── result.rs          → GreeksResult<T>
├── checkpoint/
│   ├── mod.rs
│   ├── memory.rs          → Memory-compute tradeoff
│   └── binomial.rs        → Binomial checkpointing
└── analytical/
    ├── mod.rs
    └── closed_form.rs     → Closed-form pricer dispatch
```

### A.4 pricer_risk (L4: リスク計算、旧pricer_xva)

```text
pricer_risk/src/
├── lib.rs
├── portfolio/
│   ├── mod.rs
│   ├── portfolio.rs       → Portfolio struct
│   ├── position.rs        → Position (trade + metadata)
│   ├── netting.rs         → NettingSet
│   └── collateral.rs      → CollateralAgreement (CSA)
├── xva/
│   ├── mod.rs
│   ├── config.rs          → XvaConfig
│   ├── cva.rs             → CreditValueAdjustment
│   ├── dva.rs             → DebitValueAdjustment
│   ├── fva.rs             → FundingValueAdjustment
│   ├── colva.rs           → CollateralValueAdjustment
│   ├── kva.rs             → CapitalValueAdjustment
│   ├── mva.rs             → MarginValueAdjustment
│   └── wwr.rs             → WrongWayRisk
├── exposure/
│   ├── mod.rs
│   ├── profile.rs         → ExposureProfile
│   ├── epe.rs             → ExpectedPositiveExposure
│   ├── ene.rs             → ExpectedNegativeExposure
│   ├── pfe.rs             → PotentialFutureExposure
│   └── simulation.rs      → ExposureSimulator
├── risk_factors/
│   ├── mod.rs
│   ├── bump.rs            → BumpScenario (parallel, twist, butterfly)
│   ├── shift.rs           → RiskFactorShift
│   └── presets.rs         → Preset scenarios (regulatory, internal)
├── scenarios/
│   ├── mod.rs
│   ├── engine.rs          → ScenarioEngine
│   ├── historical.rs      → HistoricalScenario
│   ├── stress.rs          → StressScenario
│   └── pnl.rs             → ScenarioPnL
├── aggregation/
│   ├── mod.rs
│   ├── greeks_aggregator.rs → GreeksAggregator (portfolio Greeks)
│   ├── var.rs             → ValueAtRisk
│   └── expected_shortfall.rs → ES/CVaR
├── soa/
│   ├── mod.rs
│   ├── layout.rs          → SoA layout definitions
│   └── conversion.rs      → AoS ↔ SoA変換
└── parallel/
    ├── mod.rs
    ├── rayon.rs           → Rayon parallel iterators
    ├── batch.rs           → Batch processing
    └── scheduling.rs      → Work scheduling
```

### A.5 依存関係グラフ

```text
                     ┌─────────────────────────────────────┐
                     │                                     │
                     │   pricer_risk (L4)                  │
                     │   - XVA/CVA/DVA計算                 │
                     │   - ポートフォリオ管理              │
                     │   - リスクファクター管理            │
                     │                                     │
                     └───────────────┬─────────────────────┘
                                     │
                                     ▼
                     ┌─────────────────────────────────────┐
                     │                                     │
                     │   pricer_pricing (L3)               │
                     │   - Monte Carlo評価                 │
                     │   - キャリブレーション              │
                     │   - Greeks計算                      │
                     │   - Enzyme AD統合                   │
                     │                                     │
                     └───────────────┬─────────────────────┘
                                     │
                                     ▼
                     ┌─────────────────────────────────────┐
                     │                                     │
                     │   pricer_models (L2)                │
                     │   - 商品定義                        │
                     │   - 確率モデル                      │
                     │   - スケジュール生成                │
                     │   - 解析解                          │
                     │                                     │
                     └───────────────┬─────────────────────┘
                                     │
                                     ▼
                     ┌─────────────────────────────────────┐
                     │                                     │
                     │   pricer_core (L1)                  │
                     │   - 市場データ (curves, surfaces)   │
                     │   - 基本型 (Currency, Date)         │
                     │   - 数学ユーティリティ              │
                     │   - 共通トレイト                    │
                     │                                     │
                     └─────────────────────────────────────┘
```
