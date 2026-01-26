# Requirements Document

## Introduction

本仕様は、PricerにおけるMarketデータアクセスをIndex単位で統一化するための要件を定義する。現状、Curve（含Builder）やVolCube（含Builder）がリスト形式で保持されているが、Pricing時には`get_df(index, term)`や`get_bs_vol(index, tenor, term)`のようにIndexをキーとしたアクセスパターンが求められる。本仕様では、すべてのMarket構成要素がIndex単位で正しく定義・アクセス可能であることを保証する。

## Project Description (Input)
Pricerに渡されるMarketはCurve(含Builder)やVolCube(含Builder)がリストされている感じだが、Pricingの際にはgetDF(index, term)?やgetBSvol(index, tenor, term)?というようにIndexがキーとなるように設計したい。Curve(含Builder)やVolCube(含Builder)などがきちんとIndex単位で定義されているかを全体漏らさず実装したい。

## Requirements

### Requirement 1: Index型定義の標準化

**Objective:** As a クオンツ開発者, I want Marketデータアクセスに使用するIndex型が明確に定義されていること, so that 型安全性を確保しつつ一貫したAPIを提供できる

#### Acceptance Criteria
1. The Market module shall RateIndexを主キーとしてYieldCurveを一意に特定できること
2. The Market module shall FX通貨ペアをキーとしてFxCurveを一意に特定できること
3. The Market module shall VolatilityIndexを定義し、VolCube/VolSurfaceを一意に特定できること
4. When 存在しないIndexが指定された場合, the Market module shall 適切なエラー型（IndexNotFound）を返却すること
5. The infra_master module shall RateIndex、FxIndex、VolatilityIndexのID型を定義すること

### Requirement 2: Curve Index-Keyed Access API

**Objective:** As a Pricingエンジン, I want Index経由でCurveデータにアクセスできるAPI, so that Pricingロジック内で効率的にDF取得が可能となる

#### Acceptance Criteria
1. The Market module shall `get_df(index: &RateIndex, term: Date) -> Result<f64, MarketError>` APIを提供すること
2. The Market module shall `get_forward_rate(index: &RateIndex, start: Date, end: Date) -> Result<f64, MarketError>` APIを提供すること
3. The Market module shall `get_zero_rate(index: &RateIndex, term: Date) -> Result<f64, MarketError>` APIを提供すること
4. When OIS/IBORマルチカーブ環境において, the Market module shall Discounting CurveとProjection Curveを別々のIndexで管理できること
5. The CurveBuilder shall 構築結果を対応するRateIndexに紐づけて登録できること
6. While Curveが未構築の状態で, when DFが要求された場合, the Market module shall CurveNotBuiltエラーを返却すること

### Requirement 3: VolCube/VolSurface Index-Keyed Access API

**Objective:** As a オプションPricer, I want Index経由でVolatilityデータにアクセスできるAPI, so that Pricingロジック内で効率的にVol取得が可能となる

#### Acceptance Criteria
1. The Market module shall `get_bs_vol(index: &VolatilityIndex, expiry: Date, strike: f64) -> Result<f64, MarketError>` APIを提供すること
2. The Market module shall `get_swaption_vol(index: &RateIndex, expiry: Period, tenor: Period, strike: f64) -> Result<f64, MarketError>` APIを提供すること
3. The Market module shall `get_fx_vol(ccy_pair: &CurrencyPair, expiry: Date, strike: f64) -> Result<f64, MarketError>` APIを提供すること
4. The VolCubeBuilder shall 構築結果を対応するIndexに紐づけて登録できること
5. When VolCubeがキャリブレーション未完了の状態で, when Volが要求された場合, the Market module shall VolCubeNotCalibratedエラーを返却すること
6. Where SABR補間が有効な場合, the Market module shall SABR parameterを使用して任意Strikeの補間を行うこと

### Requirement 4: IndexCurveMapper統合

**Objective:** As a Market構築担当者, I want Index→Curve/VolCubeのマッピングを管理するMapperを持つこと, so that 複数のMarketコンポーネントを一元管理できる

#### Acceptance Criteria
1. The IndexCurveMapper shall RateIndex→YieldCurveのマッピングを管理できること
2. The IndexCurveMapper shall RateIndex→VolCubeのマッピングを管理できること（Swaption用）
3. The IndexCurveMapper shall CurrencyPair→FxCurveのマッピングを管理できること
4. The IndexCurveMapper shall CurrencyPair→FxVolSurfaceのマッピングを管理できること
5. When マッピングが重複して登録された場合, the IndexCurveMapper shall DuplicateIndexMappingエラーを返却すること
6. The Market module shall IndexCurveMapperを介してすべてのMarketデータアクセスを統一すること

### Requirement 5: Market構造体のIndex-Keyed設計

**Objective:** As a システムアーキテクト, I want Market構造体がIndex-Keyedなデータ構造を内部で使用すること, so that O(1)のルックアップ性能を達成できる

#### Acceptance Criteria
1. The Market struct shall `HashMap<RateIndex, Arc<YieldCurve>>`形式でCurveを保持すること
2. The Market struct shall `HashMap<RateIndex, Arc<VolCube>>`形式でSwaption VolCubeを保持すること
3. The Market struct shall `HashMap<CurrencyPair, Arc<FxCurve>>`形式でFX Curveを保持すること
4. The Market struct shall `HashMap<CurrencyPair, Arc<FxVolSurface>>`形式でFX Vol Surfaceを保持すること
5. The Market struct shall 評価日（valuation_date）を保持し、すべてのterm計算の基準とすること
6. While MarketがImmutableな状態で, the Pricer shall thread-safeに並列アクセス可能であること

### Requirement 6: Builder APIのIndex対応

**Objective:** As a Marketデータ構築担当者, I want Builder APIがIndexを明示的に指定する設計, so that 構築時点でIndex紐づけが保証される

#### Acceptance Criteria
1. The CurveBuilder shall `for_index(index: RateIndex)`メソッドを提供すること
2. The VolCubeBuilder shall `for_index(index: RateIndex)`メソッドを提供すること
3. The FxForwardCurveBuilder shall `for_pair(pair: CurrencyPair)`メソッドを提供すること
4. The FxVolSurfaceBuilder shall `for_pair(pair: CurrencyPair)`メソッドを提供すること
5. When Builderが`build()`呼び出し時にIndexが未設定の場合, the Builder shall IndexNotSpecifiedエラーを返却すること
6. The MarketBuilder shall 複数のCurve/VolCube Builderを集約し、一括でMarketを構築できること

### Requirement 7: 網羅性検証機能

**Objective:** As a リスク管理者, I want Pricingに必要なすべてのIndexがMarketに定義されているかを検証する機能, so that Pricing失敗を事前に防止できる

#### Acceptance Criteria
1. The Market module shall `validate_completeness(required_indices: &[RateIndex]) -> Result<(), Vec<MissingIndex>>` APIを提供すること
2. When Tradeが参照するIndexがMarketに存在しない場合, the validation shall そのIndexをMissingIndexとして報告すること
3. The Trade struct shall `required_indices() -> Vec<IndexRequirement>` APIを提供し、必要なIndexを列挙できること
4. The Portfolio struct shall 配下の全Tradeに対して必要なIndexを集約し、Market検証を実行できること
5. If Marketの網羅性検証が失敗した場合, the system shall 不足しているIndexの一覧を含む詳細なエラーメッセージを出力すること

### Requirement 8: 既存コードとの互換性

**Objective:** As a 開発者, I want 既存のMarketアクセスパターンとの後方互換性, so that 段階的な移行が可能となる

#### Acceptance Criteria
1. The Market module shall 既存のCurve直接アクセスAPIを非推奨（deprecated）としつつ維持すること
2. The Market module shall Index-Keyed APIを推奨APIとしてドキュメント化すること
3. When 非推奨APIが使用された場合, the compiler shall deprecation warningを出力すること
4. The migration guide shall 既存コードからIndex-Keyed APIへの移行手順を記載すること
5. While 移行期間中, the Market module shall 両方のアクセスパターンをサポートすること

