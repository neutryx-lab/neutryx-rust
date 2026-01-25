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
1. The FX Vol Calibration Engine shall provide `FxVolInstrument` enum with variants for ATM, BF (Butterfly), RR (Risk Reversal), and Delta-quoted options.
2. When creating a BF instrument, the FX Vol Calibration Engine shall require expiry, delta (e.g., 25D, 10D), and market convention (premium-adjusted delta vs spot delta).
3. When creating an RR instrument, the FX Vol Calibration Engine shall require expiry, delta, and quote convention (call vol minus put vol).
4. The FX Vol Calibration Engine shall support premium-adjusted delta convention for USDJPY and spot delta convention for EURUSD by default.
5. The FX Vol Calibration Engine shall define `FxVolConvention` struct containing: delta type, premium currency, cut-off time, calendar, and day count convention.
6. If an instrument is created with invalid delta (outside 0-50 range), then the FX Vol Calibration Engine shall return `FxVolInstrumentError::InvalidDelta`.
7. The FX Vol Calibration Engine shall provide builder pattern for `FxVolInstrument` construction with fluent API.

### Requirement 2: VolSurface設定と補間器構成

**Objective:** As a クオンツ開発者, I want VolSurfaceの補間方法（SABR、Flat、Linear等）を設定で指定できる, so that 市場慣行やリスク管理要件に応じた補間戦略を選択できる.

#### Acceptance Criteria
1. The FX Vol Calibration Engine shall provide `FxVolSurfaceConfig` struct specifying interpolator type, extrapolation method, and calibration parameters.
2. When SABR interpolator is selected, the FX Vol Calibration Engine shall configure SABR parameters (alpha, beta, rho, nu) per expiry with beta constraint options (fixed or calibrated).
3. The FX Vol Calibration Engine shall support `InterpolatorType` enum with variants: `Sabr`, `SviRaw`, `Flat`, `Linear`, `CubicSpline`.
4. When constructing VolSurface with SABR, the FX Vol Calibration Engine shall treat SABR as a parametric interpolator in strike/delta dimension.
5. The FX Vol Calibration Engine shall support expiry interpolation methods: `Linear`, `FlatForward`, `CubicSpline`.
6. If incompatible interpolator combination is specified, then the FX Vol Calibration Engine shall return `ConfigError::IncompatibleInterpolators`.
7. The FX Vol Calibration Engine shall provide sensible default configuration for major currency pairs (G10 currencies).

### Requirement 3: FxVolSurfaceBuilderによるカリブレーションワークフロー

**Objective:** As a クオンツ開発者, I want CurveBuilderと同様のAPIでVolSurfaceをカリブレーションできる, so that 一貫したワークフローでマーケットデータ処理が可能になる.

#### Acceptance Criteria
1. The FX Vol Calibration Engine shall provide `FxVolSurfaceBuilder` with method chain: `new()` -> `with_instruments()` -> `with_config()` -> `with_fx_curve()` -> `build()`.
2. When `build()` is called, the FX Vol Calibration Engine shall perform calibration and return `Result<CalibratedFxVolSurface, CalibrationError>`.
3. The FX Vol Calibration Engine shall require `FxCurve` (forward curve) as input for delta-to-strike conversion.
4. While calibration is in progress, the FX Vol Calibration Engine shall track iteration count and residual for diagnostics.
5. When calibration fails to converge, the FX Vol Calibration Engine shall return `CalibrationError` with diagnostic information including final residual and parameter values.
6. The FX Vol Calibration Engine shall support incremental calibration (adding instruments to existing surface).
7. When instruments with overlapping expiries are provided, the FX Vol Calibration Engine shall use latest quote for each tenor-delta combination.

### Requirement 4: CalibratedFxVolSurface（補間器としての機能）

**Objective:** As a プライシングエンジン, I want カリブレーション済みVolSurfaceからボラティリティを補間取得できる, so that オプションプライシングで使用できる.

#### Acceptance Criteria
1. The `CalibratedFxVolSurface` shall implement `VolatilitySurface` trait with `vol(expiry: f64, strike: f64) -> T` method.
2. When querying volatility at calibrated point, the `CalibratedFxVolSurface` shall return exact calibrated value.
3. When querying volatility at interpolated point, the `CalibratedFxVolSurface` shall use configured interpolator (SABR, SVI, etc.).
4. The `CalibratedFxVolSurface` shall provide `vol_by_delta(expiry: f64, delta: f64) -> T` for delta-space queries.
5. If query is outside extrapolation bounds, then the `CalibratedFxVolSurface` shall apply configured extrapolation policy (flat, linear, or error).
6. The `CalibratedFxVolSurface` shall be generic over `T: Float` for AD compatibility.
7. The `CalibratedFxVolSurface` shall provide `smile(expiry: f64) -> VolSmile<T>` for single-expiry smile extraction.

### Requirement 5: 遅延評価とキャッシュ最適化

**Objective:** As a パフォーマンス最適化エンジニア, I want VolSurfaceの評価を遅延実行しキャッシュできる, so that 繰り返し評価時のパフォーマンスが向上する.

#### Acceptance Criteria
1. The FX Vol Calibration Engine shall provide `LazyFxVolSurface` wrapper with deferred calibration execution.
2. When `vol()` is first called on `LazyFxVolSurface`, the FX Vol Calibration Engine shall trigger calibration and cache result.
3. While calibration result is cached, the `LazyFxVolSurface` shall return cached values without recalibration.
4. The `LazyFxVolSurface` shall provide `invalidate()` method to clear cache and force recalibration on next access.
5. When underlying instrument quotes change, the `LazyFxVolSurface` shall automatically invalidate affected cache entries.
6. The FX Vol Calibration Engine shall support `Arc<RwLock<>>` based thread-safe caching for concurrent access.
7. The `LazyFxVolSurface` shall track cache hit/miss statistics via `CacheStats` struct.

### Requirement 6: AAD計算グラフサポート

**Objective:** As a リスクエンジン, I want VolSurface評価のAAD計算グラフをインストルメントまで拡張できる, so that マーケットクォートに対する感応度を自動微分で計算できる.

#### Acceptance Criteria
1. The `CalibratedFxVolSurface` shall implement `Differentiable` trait for AD-enabled volatility queries.
2. When AD mode is enabled, the FX Vol Calibration Engine shall build computation graph from instrument quotes through calibration to vol output.
3. The FX Vol Calibration Engine shall support gradient computation with respect to input BF/RR quotes.
4. The FX Vol Calibration Engine shall provide `VolSurfaceSensitivity` struct containing dVol/dATM, dVol/dBF, dVol/dRR.
5. While performing AAD, the FX Vol Calibration Engine shall use smooth approximations for any discontinuous operations.
6. The FX Vol Calibration Engine shall support both forward-mode (tangent) and reverse-mode (adjoint) differentiation.
7. When extracting computation graph, the FX Vol Calibration Engine shall produce D3.js-compatible JSON for visualisation.

### Requirement 7: FXスワップインストルメント定義

**Objective:** As a マーケットデータエンジニア, I want 短期FXフォワードカーブ構築のためのFXスワップインストルメントを定義できる, so that ON〜1Yのフォワードポイントを正確にブートストラップできる.

#### Acceptance Criteria
1. The FX Curve Engine shall provide `FxSwapInstrument` struct with: currency pair, near date, far date, near rate (spot), far rate (forward), and swap points.
2. When creating an FX swap, the FX Curve Engine shall support standard tenors: ON, TN, SN, 1W, 2W, 1M, 2M, 3M, 6M, 9M, 1Y.
3. The FX Curve Engine shall define `FxSwapConvention` struct containing: spot lag, settlement calendar, and business day convention.
4. When swap points are quoted, the FX Curve Engine shall convert to forward rate: F = S + swap_points / scaling_factor.
5. The FX Curve Engine shall support both outright forward quote and swap points quote formats.
6. If FX swap dates are invalid (near >= far), then the FX Curve Engine shall return `FxSwapError::InvalidDates`.
7. The FX Curve Engine shall provide `implied_forward_rate()` method to extract forward rate from swap structure.

### Requirement 8: クロスカレンシーベーシススワップ定義

**Objective:** As a マーケットデータエンジニア, I want 中長期FXフォワードカーブ構築のためのクロスカレンシーベーシススワップを定義できる, so that 2Y〜30Yのフォワードレートを正確に構築できる.

#### Acceptance Criteria
1. The FX Curve Engine shall provide `CrossCurrencyBasisSwap` struct with: domestic currency, foreign currency, notional, maturity, domestic leg details, foreign leg details, and basis spread.
2. When creating a XCCY basis swap, the FX Curve Engine shall require two floating legs with respective rate indices (e.g., SOFR vs EURIBOR).
3. The FX Curve Engine shall support standard tenors: 2Y, 3Y, 4Y, 5Y, 7Y, 10Y, 15Y, 20Y, 25Y, 30Y.
4. The FX Curve Engine shall define `XccyBasisConvention` struct containing: notional exchange flag, mark-to-market flag, payment frequency per leg, and spread leg indicator.
5. When basis spread is quoted, the FX Curve Engine shall apply spread to the designated leg (typically foreign leg).
6. The FX Curve Engine shall support both resettable (mark-to-market) and non-resettable XCCY swaps.
7. If XCCY swap has mismatched currencies on same leg, then the FX Curve Engine shall return `XccySwapError::CurrencyMismatch`.

### Requirement 9: FxCurveトレイトとフォワードカーブ抽象化

**Objective:** As a プライシングエンジン, I want 統一されたFXフォワードカーブインターフェースを使用できる, so that Delta-Strike変換やフォワードプライシングで一貫したAPIを利用できる.

#### Acceptance Criteria
1. The FX Curve Engine shall provide `FxCurve<T>` trait with methods: `forward_rate(expiry: T) -> Result<T>`, `forward_points(expiry: T) -> Result<T>`, `spot_rate() -> T`.
2. The `FxCurve<T>` shall provide `discount_factor_domestic(t: T)` and `discount_factor_foreign(t: T)` for underlying yield curve access.
3. When querying forward rate, the `FxCurve<T>` shall compute: F(T) = S × exp((r_d - r_f) × T) or use bootstrapped forward points.
4. The `FxCurve<T>` shall be generic over `T: Float` for AD compatibility.
5. The FX Curve Engine shall provide `CalibratedFxCurve<T>` implementing `FxCurve<T>` with interpolated forward points.
6. If forward rate is queried beyond curve tenor, then the `FxCurve<T>` shall apply configured extrapolation policy.
7. The `FxCurve<T>` shall provide `currency_pair()` method returning the `CurrencyPair` for the curve.

### Requirement 10: FxForwardCurveBuilder（短期＋長期統合）

**Objective:** As a クオンツ開発者, I want FXスワップとXCCYベーシススワップからFXフォワードカーブを構築できる, so that 全テナー範囲でマーケットコンシステントなカーブを取得できる.

#### Acceptance Criteria
1. The FX Curve Engine shall provide `FxForwardCurveBuilder` with method chain: `new(currency_pair)` -> `with_spot_rate()` -> `with_domestic_curve()` -> `with_foreign_curve()` -> `with_fx_swaps()` -> `with_xccy_basis_swaps()` -> `build()`.
2. When `build()` is called, the FX Curve Engine shall bootstrap forward points from FX swaps (short-term) and XCCY basis swaps (long-term).
3. The FX Curve Engine shall blend short-term and long-term instruments at the transition tenor (typically 1Y-2Y) with smooth interpolation.
4. While bootstrapping, the FX Curve Engine shall solve for implied forward points that reprice input instruments to par.
5. When FX swap and XCCY basis swap tenors overlap, the FX Curve Engine shall prioritise based on configured preference (default: FX swap for ≤1Y).
6. If domestic or foreign discount curve is missing, then the FX Curve Engine shall return `FxCurveError::MissingDiscountCurve`.
7. The FX Curve Engine shall provide diagnostic output including repricing errors for each input instrument.
8. The FX Curve Engine shall reuse existing `SequentialBootstrapper<T>` from `pricer_models::market::calibration::bootstrapping` for the solving algorithm.

### Requirement 11: FxMarketBuilder（エンドツーエンドオーケストレーション）

**Objective:** As a クオンツ開発者, I want OISカーブ構築からFXフォワードカーブ、VolSurfaceまでを一括で構築できる, so that 依存関係を意識せず完全なFXマーケットを構築できる.

#### Acceptance Criteria
1. The FX Market Engine shall provide `FxMarketBuilder` with method chain: `new(currency_pair)` -> `with_domestic_ois_instruments()` -> `with_foreign_ois_instruments()` -> `with_fx_instruments()` -> `with_vol_instruments()` -> `build()`.
2. When `build()` is called, the FX Market Engine shall execute the following dependency-ordered steps: (1) bootstrap domestic OIS curve, (2) bootstrap foreign OIS curve, (3) build FX forward curve using bootstrapped OIS curves, (4) calibrate vol surface using FX curve.
3. The FX Market Engine shall use existing `CurveEngine` (from `pricer_models::market::calibration::bootstrapping`) for OIS curve construction.
4. The `FxMarketBuilder` shall provide partial build methods: `build_discount_curves()`, `build_fx_curve()`, `build_vol_surface()` for step-by-step construction.
5. When any intermediate step fails, the FX Market Engine shall return `FxMarketError` with the failing step and partial results already computed.
6. The FX Market Engine shall support lazy evaluation mode where intermediate curves are built on-demand.
7. The `FxMarketBuilder` shall provide `with_prebuilt_domestic_curve()` and `with_prebuilt_foreign_curve()` to skip OIS bootstrapping when curves are already available.
8. The FX Market Engine shall provide `FxMarket` result struct containing: domestic discount curve, foreign discount curve, FX forward curve, and optionally vol surface.

### Requirement 12: Demo WebApp統合（FXカーブ＆VolSurface）

**Objective:** As a デモユーザー, I want WebAppでFXフォワードカーブとVolSurfaceをインタラクティブに構築・可視化できる, so that カリブレーション結果を確認できる.

#### Acceptance Criteria
1. The Demo WebApp shall provide `/api/fxcurve/build` endpoint accepting FX swaps, XCCY basis swaps, and discount curves.
2. When FX curve build completes, the WebApp shall return forward points by tenor in JSON format.
3. The WebApp shall provide `/api/fxvol/calibrate` endpoint accepting vol instrument list, config, and FX curve reference.
4. When vol calibration completes, the WebApp shall return surface data in JSON format suitable for 3D visualisation.
5. The WebApp shall provide `/api/fxvol/smile` endpoint returning vol smile data for specified expiry.
6. The WebApp shall display calibration diagnostics (iterations, residual, convergence status) for both curve and surface.
7. If calibration fails, then the WebApp shall return HTTP 422 with detailed error message and diagnostic data.
8. The WebApp shall support real-time surface update via WebSocket when instrument quotes change.
9. The WebApp shall provide UI for editing FX swap points, basis spreads, BF/RR quotes and immediately see recalibrated results.

### Requirement 13: 既存実装のクリーンアップ

**Objective:** As a コードベースメンテナー, I want 本実装で不要になった既存コードを削除できる, so that コードベースがシンプルに保たれる.

#### Acceptance Criteria
1. The implementation shall identify and remove deprecated `FxVolatilitySurface` implementations that are superseded.
2. When removing code, the implementation shall update all dependent modules to use new API.
3. The implementation shall remove unused `fxvol_types.rs` and `fxvol_handlers.rs` if functionality is replaced.
4. The implementation shall consolidate `FxSwap` in `infra_master` with enhanced definition if needed.
5. The implementation shall ensure all existing tests pass or are updated for new API.
6. If code removal would break public API, then the implementation shall provide migration guide in commit message.
7. The implementation shall run `cargo clippy --all-targets` and `cargo test --workspace` to verify no regressions.
8. The implementation shall update steering documents (`structure.md`, `roadmap.md`) to reflect architectural changes.

### Requirement 14: 型安全性とエラーハンドリング

**Objective:** As a 堅牢性重視の開発者, I want 型安全なAPIとstructuredエラーを使用できる, so that 実行時エラーをコンパイル時に検出できる.

#### Acceptance Criteria
1. The FX Calibration Engine shall use newtype pattern for domain values: `Delta(f64)`, `Strike(f64)`, `Vol(f64)`, `ForwardPoints(f64)`, `BasisSpread(f64)`.
2. The FX Calibration Engine shall define `FxCalibrationError` enum with variants for all failure modes (curve and vol).
3. When invalid input is detected, the FX Calibration Engine shall return typed error, not panic.
4. The FX Calibration Engine shall implement `thiserror::Error` for all error types.
5. If numerical instability is detected during calibration, then the FX Calibration Engine shall return `CalibrationError::NumericalInstability` with context.
6. The FX Calibration Engine shall validate all inputs at API boundary before proceeding.
7. The FX Calibration Engine shall support `serde::Serialize` for error types to enable JSON error responses.
