# Requirements Document

## Introduction

本仕様は、定量金融プライシングのための市場データ構造を定義する。イールドカーブとボラティリティサーフェスの抽象化を提供し、割引因子・フォワードレート計算、およびストライク/満期によるインプライドボラティリティ検索をサポートする。全構造体は `T: Float` ジェネリクスを使用し、Enzyme AD互換性を確保する。

## Requirements

### Requirement 1: YieldCurve Trait Definition

**Objective:** As a quant developer, I want a generic yield curve trait that defines discount factor and forward rate calculations, so that I can price interest rate sensitive instruments consistently.

#### Acceptance Criteria

1. The YieldCurve module shall define a generic trait `YieldCurve<T: Float>` with methods for discount factor and forward rate calculations.
2. When `discount_factor(t)` is called with time `t`, the YieldCurve shall return the discount factor `D(t) = exp(-r * t)` for the given maturity.
3. When `forward_rate(t1, t2)` is called with times `t1 < t2`, the YieldCurve shall return the continuously compounded forward rate between `t1` and `t2`.
4. If `t < 0` is provided to discount_factor, then the YieldCurve shall return an appropriate error indicating invalid maturity.

### Requirement 2: FlatCurve Implementation

**Objective:** As a quant developer, I want a flat yield curve implementation for simple constant-rate scenarios, so that I can quickly prototype and test pricing models.

#### Acceptance Criteria

1. The FlatCurve struct shall implement `YieldCurve<T>` with a single constant rate parameter.
2. When FlatCurve is constructed with rate `r`, the FlatCurve shall use this rate for all maturity calculations.
3. The FlatCurve shall compute `discount_factor(t)` as `exp(-r * t)` using the constant rate.
4. The FlatCurve shall be generic over `T: Float` to support both `f64` and `Dual64` types.

### Requirement 3: InterpolatedCurve Implementation

**Objective:** As a quant developer, I want an interpolated yield curve that uses market data points, so that I can build realistic term structures from observable rates.

#### Acceptance Criteria

1. The InterpolatedCurve struct shall implement `YieldCurve<T>` using pillar points (tenors and rates).
2. When InterpolatedCurve is constructed with pillar points, the InterpolatedCurve shall store and interpolate between these points.
3. The InterpolatedCurve shall integrate with `pricer_core::math::interpolators` for interpolation logic.
4. If `t` is outside the pillar range, then the InterpolatedCurve shall either extrapolate (flat) or return an error based on configuration.

### Requirement 4: VolatilitySurface Trait Definition

**Objective:** As a quant developer, I want a generic volatility surface trait that supports implied volatility lookup, so that I can price options with market-consistent volatilities.

#### Acceptance Criteria

1. The VolatilitySurface module shall define a generic trait `VolatilitySurface<T: Float>` for implied volatility lookup.
2. When `volatility(strike, expiry)` is called, the VolatilitySurface shall return the implied volatility for the given strike and expiry.
3. The VolatilitySurface shall provide domain bounds for valid strike and expiry ranges.
4. If strike or expiry is outside valid bounds, then the VolatilitySurface shall return an appropriate error.

### Requirement 5: FlatVol Implementation

**Objective:** As a quant developer, I want a flat volatility surface for simple constant-vol scenarios, so that I can test option pricing models with known analytical solutions.

#### Acceptance Criteria

1. The FlatVol struct shall implement `VolatilitySurface<T>` with a single constant volatility parameter.
2. When FlatVol is constructed with volatility `sigma`, the FlatVol shall return this value for all strike/expiry queries.
3. The FlatVol shall be generic over `T: Float` to support both `f64` and `Dual64` types.

### Requirement 6: InterpolatedVolSurface Implementation

**Objective:** As a quant developer, I want an interpolated volatility surface using market smile data, so that I can capture the volatility smile/skew from market quotes.

#### Acceptance Criteria

1. The InterpolatedVolSurface struct shall implement `VolatilitySurface<T>` using a grid of (strike, expiry, vol) points.
2. When InterpolatedVolSurface is constructed with grid data, the InterpolatedVolSurface shall organise data by expiry slices.
3. The InterpolatedVolSurface shall use 2D interpolation (bilinear or bicubic) for strike-expiry lookup.
4. If extrapolation is required, then the InterpolatedVolSurface shall use flat extrapolation or configurable boundary behaviour.

### Requirement 7: Market Data Error Handling

**Objective:** As a quant developer, I want consistent error handling for market data operations, so that I can gracefully handle edge cases and invalid inputs.

#### Acceptance Criteria

1. The market data module shall define `MarketDataError` enum covering all failure modes.
2. When an invalid maturity (negative time) is provided, the module shall return `MarketDataError::InvalidMaturity`.
3. When interpolation fails due to out-of-bounds query, the module shall return `MarketDataError::OutOfBounds`.
4. The MarketDataError shall integrate with existing `PricingError` for unified error handling.

### Requirement 8: Generic Type Compatibility

**Objective:** As a quant developer, I want all market data structures to work with both f64 and Dual64, so that I can compute sensitivities via automatic differentiation.

#### Acceptance Criteria

1. The YieldCurve and VolatilitySurface traits shall be generic over `T: Float`.
2. When market data structures are instantiated with `Dual64`, the structures shall propagate derivatives correctly.
3. The implementations shall use `smooth_max` / `smooth_abs` from `pricer_core::math::smoothing` instead of branching operations where applicable.
