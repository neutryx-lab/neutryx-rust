# Requirements Document

## Introduction

本仕様書は、FXボラティリティサーフェスのカリブレーションシステムおよびその前提となるFXフォワードカーブ構築システムに関する要件を定義する。EURUSD、USDJPYなどの主要通貨ペアを対象に、以下の3つの主要機能を提供する：

1. **ディスカウントカーブ構築**: 既存の`CurveEngine`を活用し、USD OIS (SOFR)、EUR OIS (ESTR) 等のディスカウントカーブを構築
2. **FXフォワードカーブ構築**: 短期（ON〜1Y）はFXスワップ、中長期（2Y〜30Y）はクロスカレンシーベーシススワップから、マーケットコンシステントなフォワードカーブを構築（ディスカウントカーブに依存）
3. **FXボラティリティサーフェスカリブレーション**: Butterfly (BF) およびRisk Reversal (RR) インストルメントを用いてSABR等の補間器を持つVolSurfaceを構築（FXカーブに依存）

**依存関係チェーン**:
```
OIS Instruments → Discount Curves → FX Forward Curve → Vol Surface
```

CurveBuilder（イールドカーブブートストラップ）と同様のアーキテクチャを採用し、遅延評価・キャッシュ最適化・AAD計算グラフをサポートする。エンドツーエンドのワークフローを`FxMarketBuilder`で統合し、個別コンポーネントの再利用性も維持する。

## Requirements

### Requirement 1: FX Vol Instrument定義

**Objective:** As a クオンツ開発者, I want EURUSD/USDJPYのBF・RRインストルメントを標準的なマーケットコンベンションで定義できる, so that マーケットクォートから直接VolSurface構築が可能になる.

#### Acceptance Criteria

*[4 additional criteria omitted]*
1. The FX Vol Calibration Engine shall provide `FxVolInstrument` enum with variants for ATM, BF (Butterfly), RR (Risk Reversal), and Delta-quoted options.
2. When creating a BF instrument, the FX Vol Calibration Engine shall require expiry, delta (e.g., 25D, 10D), and market convention (premium-adjusted delta vs spot delta).
3. When creating an RR instrument, the FX Vol Calibration Engine shall require expiry, delta, and quote convention (call vol minus put vol).
### Requirement 2: VolSurface設定と補間器構成

**Objective:** As a クオンツ開発者, I want VolSurfaceの補間方法（SABR、Flat、Linear等）を設定で指定できる, so that 市場慣行やリスク管理要件に応じた補間戦略を選択できる.

#### Acceptance Criteria

*[4 additional criteria omitted]*
1. The FX Vol Calibration Engine shall provide `FxVolSurfaceConfig` struct specifying interpolator type, extrapolation method, and calibration parameters.
2. When SABR interpolator is selected, the FX Vol Calibration Engine shall configure SABR parameters (alpha, beta, rho, nu) per expiry with beta constraint options (fixed or calibrated).
3. The FX Vol Calibration Engine shall support `InterpolatorType` enum with variants: `Sabr`, `SviRaw`, `Flat`, `Linear`, `CubicSpline`.
### Requirement 3: FxVolSurfaceBuilderによるカリブレーションワークフロー

**Objective:** As a クオンツ開発者, I want CurveBuilderと同様のAPIでVolSurfaceをカリブレーションできる, so that 一貫したワークフローでマーケットデータ処理が可能になる.

#### Acceptance Criteria

*[4 additional criteria omitted]*
1. The FX Vol Calibration Engine shall provide `FxVolSurfaceBuilder` with method chain: `new()` -> `with_instruments()` -> `with_config()` -> `with_fx_curve()` -> `build()`.
2. When `build()` is called, the FX Vol Calibration Engine shall perform calibration and return `Result<CalibratedFxVolSurface, CalibrationError>`.
3. The FX Vol Calibration Engine shall require `FxCurve` (forward curve) as input for delta-to-strike conversion.
### Requirement 4: CalibratedFxVolSurface（補間器としての機能）

**Objective:** As a プライシングエンジン, I want カリブレーション済みVolSurfaceからボラティリティを補間取得できる, so that オプションプライシングで使用できる.

#### Acceptance Criteria

*[4 additional criteria omitted]*
1. The `CalibratedFxVolSurface` shall implement `VolatilitySurface` trait with `vol(expiry: f64, strike: f64) -> T` method.
2. When querying volatility at calibrated point, the `CalibratedFxVolSurface` shall return exact calibrated value.
3. When querying volatility at interpolated point, the `CalibratedFxVolSurface` shall use configured interpolator (SABR, SVI, etc.).
### Requirement 5: 遅延評価とキャッシュ最適化

**Objective:** As a パフォーマンス最適化エンジニア, I want VolSurfaceの評価を遅延実行しキャッシュできる, so that 繰り返し評価時のパフォーマンスが向上する.

#### Acceptance Criteria

*[4 additional criteria omitted]*
1. The FX Vol Calibration Engine shall provide `LazyFxVolSurface` wrapper with deferred calibration execution.
2. When `vol()` is first called on `LazyFxVolSurface`, the FX Vol Calibration Engine shall trigger calibration and cache result.
3. While calibration result is cached, the `LazyFxVolSurface` shall return cached values without recalibration.
### Requirement 6: AAD計算グラフサポート

**Objective:** As a リスクエンジン, I want VolSurface評価のAAD計算グラフをインストルメントまで拡張できる, so that マーケットクォートに対する感応度を自動微分で計算できる.

#### Acceptance Criteria

*[4 additional criteria omitted]*
1. The `CalibratedFxVolSurface` shall implement `Differentiable` trait for AD-enabled volatility queries.
2. When AD mode is enabled, the FX Vol Calibration Engine shall build computation graph from instrument quotes through calibration to vol output.
3. The FX Vol Calibration Engine shall support gradient computation with respect to input BF/RR quotes.
### Requirement 7: FXスワップインストルメント定義

**Objective:** As a マーケットデータエンジニア, I want 短期FXフォワードカーブ構築のためのFXスワップインストルメントを定義できる, so that ON〜1Yのフォワードポイントを正確にブートストラップできる.

#### Acceptance Criteria

*[4 additional criteria omitted]*
1. The FX Curve Engine shall provide `FxSwapInstrument` struct with: currency pair, near date, far date, near rate (spot), far rate (forward), and swap points.
2. When creating an FX swap, the FX Curve Engine shall support standard tenors: ON, TN, SN, 1W, 2W, 1M, 2M, 3M, 6M, 9M, 1Y.
3. The FX Curve Engine shall define `FxSwapConvention` struct containing: spot lag, settlement calendar, and business day convention.
### Requirement 8: クロスカレンシーベーシススワップ定義

**Objective:** As a マーケットデータエンジニア, I want 中長期FXフォワードカーブ構築のためのクロスカレンシーベーシススワップを定義できる, so that 2Y〜30Yのフォワードレートを正確に構築できる.

#### Acceptance Criteria

*[4 additional criteria omitted]*
1. The FX Curve Engine shall provide `CrossCurrencyBasisSwap` struct with: domestic currency, foreign currency, notional, maturity, domestic leg details, foreign leg details, and basis spread.
2. When creating a XCCY basis swap, the FX Curve Engine shall require two floating legs with respective rate indices (e.g., SOFR vs EURIBOR).
3. The FX Curve Engine shall support standard tenors: 2Y, 3Y, 4Y, 5Y, 7Y, 10Y, 15Y, 20Y, 25Y, 30Y.
### Requirement 9: FxCurveトレイトとフォワードカーブ抽象化

**Objective:** As a プライシングエンジン, I want 統一されたFXフォワードカーブインターフェースを使用できる, so that Delta-Strike変換やフォワードプライシングで一貫したAPIを利用できる.

#### Acceptance Criteria

*[4 additional criteria omitted]*
1. The FX Curve Engine shall provide `FxCurve<T>` trait with methods: `forward_rate(expiry: T) -> Result<T>`, `forward_points(expiry: T) -> Result<T>`, `spot_rate() -> T`.
2. The `FxCurve<T>` shall provide `discount_factor_domestic(t: T)` and `discount_factor_foreign(t: T)` for underlying yield curve access.
3. When querying forward rate, the `FxCurve<T>` shall compute: F(T) = S × exp((r_d - r_f) × T) or use bootstrapped forward points.
### Requirement 10: FxForwardCurveBuilder（短期＋長期統合）

**Objective:** As a クオンツ開発者, I want FXスワップとXCCYベーシススワップからFXフォワードカーブを構築できる, so that 全テナー範囲でマーケットコンシステントなカーブを取得できる.

#### Acceptance Criteria

*[5 additional criteria omitted]*
1. The FX Curve Engine shall provide `FxForwardCurveBuilder` with method chain: `new(currency_pair)` -> `with_spot_rate()` -> `with_domestic_curve()` -> `with_foreign_curve()` -> `with_fx_swaps()` -> `with_xccy_basis_swaps()` -> `build()`.
2. When `build()` is called, the FX Curve Engine shall bootstrap forward points from FX swaps (short-term) and XCCY basis swaps (long-term).
3. The FX Curve Engine shall blend short-term and long-term instruments at the transition tenor (typically 1Y-2Y) with smooth interpolation.
### Requirement 11: FxMarketBuilder（エンドツーエンドオーケストレーション）

**Objective:** As a クオンツ開発者, I want OISカーブ構築からFXフォワードカーブ、VolSurfaceまでを一括で構築できる, so that 依存関係を意識せず完全なFXマーケットを構築できる.

#### Acceptance Criteria

*[5 additional criteria omitted]*
1. The FX Market Engine shall provide `FxMarketBuilder` with method chain: `new(currency_pair)` -> `with_domestic_ois_instruments()` -> `with_foreign_ois_instruments()` -> `with_fx_instruments()` -> `with_vol_instruments()` -> `build()`.
2. When `build()` is called, the FX Market Engine shall execute the following dependency-ordered steps: (1) bootstrap domestic OIS curve, (2) bootstrap foreign OIS curve, (3) build FX forward curve using bootstrapped OIS curves, (4) calibrate vol surface using FX curve.
3. The FX Market Engine shall use existing `CurveEngine` (from `pricer_models::market::calibration::bootstrapping`) for OIS curve construction.
### Requirement 12: Demo WebApp統合（FXカーブ＆VolSurface）

**Objective:** As a デモユーザー, I want WebAppでFXフォワードカーブとVolSurfaceをインタラクティブに構築・可視化できる, so that カリブレーション結果を確認できる.

#### Acceptance Criteria

*[6 additional criteria omitted]*
1. The Demo WebApp shall provide `/api/fxcurve/build` endpoint accepting FX swaps, XCCY basis swaps, and discount curves.
2. When FX curve build completes, the WebApp shall return forward points by tenor in JSON format.
3. The WebApp shall provide `/api/fxvol/calibrate` endpoint accepting vol instrument list, config, and FX curve reference.
### Requirement 13: 既存実装のクリーンアップ

**Objective:** As a コードベースメンテナー, I want 本実装で不要になった既存コードを削除できる, so that コードベースがシンプルに保たれる.

#### Acceptance Criteria

*[5 additional criteria omitted]*
1. The implementation shall identify and remove deprecated `FxVolatilitySurface` implementations that are superseded.
2. When removing code, the implementation shall update all dependent modules to use new API.
3. The implementation shall remove unused `fxvol_types.rs` and `fxvol_handlers.rs` if functionality is replaced.
### Requirement 14: 型安全性とエラーハンドリング

**Objective:** As a 堅牢性重視の開発者, I want 型安全なAPIとstructuredエラーを使用できる, so that 実行時エラーをコンパイル時に検出できる.

#### Acceptance Criteria
