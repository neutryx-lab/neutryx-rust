# Requirements Document

## Introduction

本仕様は、PricerにおけるMarketデータアクセスをIndex単位で統一化するための要件を定義する。現状、Curve（含Builder）やVolCube（含Builder）がリスト形式で保持されているが、Pricing時には`get_df(index, term)`や`get_bs_vol(index, tenor, term)`のようにIndexをキーとしたアクセスパターンが求められる。

## Requirements

### Requirement 1: Index型定義の標準化

**Objective:** As a クオンツ開発者, I want Marketデータアクセスに使用するIndex型が明確に定義されていること, so that 型安全性を確保しつつ一貫したAPIを提供できる

#### Acceptance Criteria
1. The Market module shall RateIndexを主キーとしてYieldCurveを一意に特定できること
2. The Market module shall FX通貨ペアをキーとしてFxCurveを一意に特定できること
3. The Market module shall VolatilityIndexを定義し、VolCube/VolSurfaceを一意に特定できること
4. The infra_domain module shall RateIndex、FxIndex、VolatilityIndexのID型を定義すること

### Requirement 2: Curve Index-Keyed Access API

**Objective:** As a Pricingエンジン, I want Index経由でCurveデータにアクセスできるAPI, so that Pricingロジック内で効率的にDF取得が可能となる

#### Acceptance Criteria
1. The Market module shall `get_df(index: &RateIndex, term: Date) -> Result<f64, MarketError>` APIを提供すること
2. The Market module shall `get_forward_rate(index: &RateIndex, start: Date, end: Date) -> Result<f64, MarketError>` APIを提供すること
3. The Market module shall `get_zero_rate(index: &RateIndex, term: Date) -> Result<f64, MarketError>` APIを提供すること

### Requirement 3: VolCube/VolSurface Index-Keyed Access API

**Objective:** As a オプションPricer, I want Index経由でVolatilityデータにアクセスできるAPI, so that Pricingロジック内で効率的にVol取得が可能となる

#### Acceptance Criteria
1. The Market module shall `get_bs_vol(index: &VolatilityIndex, expiry: Date, strike: f64) -> Result<f64, MarketError>` APIを提供すること
2. The Market module shall `get_swaption_vol(index: &RateIndex, expiry: Period, tenor: Period, strike: f64) -> Result<f64, MarketError>` APIを提供すること
3. The Market module shall `get_fx_vol(ccy_pair: &CurrencyPair, expiry: Date, strike: f64) -> Result<f64, MarketError>` APIを提供すること

### Requirement 4: IndexCurveMapper統合

**Objective:** As a Market構築担当者, I want Index→Curve/VolCubeのマッピングを管理するMapperを持つこと, so that 複数のMarketコンポーネントを一元管理できる

#### Acceptance Criteria
1. The IndexCurveMapper shall RateIndex→YieldCurveのマッピングを管理できること
2. The IndexCurveMapper shall RateIndex→VolCubeのマッピングを管理できること（Swaption用）
3. The IndexCurveMapper shall CurrencyPair→FxCurveのマッピングを管理できること

### Requirement 5: Market構造体のIndex-Keyed設計

**Objective:** As a システムアーキテクト, I want Market構造体がIndex-Keyedなデータ構造を内部で使用すること, so that O(1)のルックアップ性能を達成できる

#### Acceptance Criteria
1. The Market struct shall `HashMap<RateIndex, Arc<YieldCurve>>`形式でCurveを保持すること
2. The Market struct shall `HashMap<RateIndex, Arc<VolCube>>`形式でSwaption VolCubeを保持すること
3. The Market struct shall `HashMap<CurrencyPair, Arc<FxCurve>>`形式でFX Curveを保持すること

### Requirement 6: Builder APIのIndex対応

**Objective:** As a Marketデータ構築担当者, I want Builder APIがIndexを明示的に指定する設計, so that 構築時点でIndex紐づけが保証される

#### Acceptance Criteria
1. The CurveBuilder shall `for_index(index: RateIndex)`メソッドを提供すること
2. The VolCubeBuilder shall `for_index(index: RateIndex)`メソッドを提供すること
3. The FxForwardCurveBuilder shall `for_pair(pair: CurrencyPair)`メソッドを提供すること

### Requirement 7: 網羅性検証機能

**Objective:** As a リスク管理者, I want Pricingに必要なすべてのIndexがMarketに定義されているかを検証する機能, so that Pricing失敗を事前に防止できる

#### Acceptance Criteria
1. The Market module shall `validate_completeness(required_indices: &[RateIndex]) -> Result<(), Vec<MissingIndex>>` APIを提供すること
2. When Tradeが参照するIndexがMarketに存在しない場合, the validation shall そのIndexをMissingIndexとして報告すること
3. The Trade struct shall `required_indices() -> Vec<IndexRequirement>` APIを提供し、必要なIndexを列挙できること

### Requirement 8: 既存コードとの互換性

**Objective:** As a 開発者, I want 既存のMarketアクセスパターンとの後方互換性, so that 段階的な移行が可能となる

#### Acceptance Criteria
1. The Market module shall 既存のCurve直接アクセスAPIを非推奨（deprecated）としつつ維持すること
2. The Market module shall Index-Keyed APIを推奨APIとしてドキュメント化すること
3. When 非推奨APIが使用された場合, the compiler shall deprecation warningを出力すること
