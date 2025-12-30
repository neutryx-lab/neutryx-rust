# Requirements Document

## Introduction

本仕様は、ヨーロピアンオプションのための解析的プライシングモデルを定義する。Black-Scholes モデルと Bachelier（正規）モデルの閉形式解を提供し、オプション価格とグリークス（デルタ、ガンマ、ベガ、シータ、ロー）の計算をサポートする。全構造体は `T: Float` ジェネリクスを使用し、Enzyme AD 互換性を確保する。

## Requirements

### Requirement 1: Black-Scholes Model Definition

**Objective:** As a quant developer, I want a Black-Scholes pricing model for European options, so that I can compute option prices using the industry-standard lognormal model.

#### Acceptance Criteria

1. The BlackScholes module shall define a generic struct `BlackScholes<T: Float>` with market parameters (spot, rate, volatility).
2. When `price_call(strike, expiry)` is called, the BlackScholes shall return the European call option price using the closed-form Black-Scholes formula.
3. When `price_put(strike, expiry)` is called, the BlackScholes shall return the European put option price using the closed-form Black-Scholes formula.
4. The BlackScholes shall implement put-call parity such that `C - P = S - K * exp(-r * T)` holds within numerical tolerance.
5. If expiry is non-positive, then the BlackScholes shall return the intrinsic value for the option.

### Requirement 2: Bachelier Model Definition

**Objective:** As a quant developer, I want a Bachelier (normal) pricing model for European options, so that I can price options under normal dynamics (e.g., interest rate options with negative rates).

#### Acceptance Criteria

1. The Bachelier module shall define a generic struct `Bachelier<T: Float>` with market parameters (forward, volatility).
2. When `price_call(strike, expiry)` is called, the Bachelier shall return the European call price using the normal model formula.
3. When `price_put(strike, expiry)` is called, the Bachelier shall return the European put price using the normal model formula.
4. The Bachelier shall support negative forward prices (common in interest rate markets).
5. The Bachelier shall derive Clone, Debug for diagnostic purposes.

### Requirement 3: Greeks Calculation via Analytical Formulas

**Objective:** As a quant developer, I want analytical Greeks calculation for Black-Scholes model, so that I can compute sensitivities efficiently without numerical differentiation.

#### Acceptance Criteria

1. The BlackScholes shall provide `delta(strike, expiry)` method returning the option's sensitivity to spot price.
2. The BlackScholes shall provide `gamma(strike, expiry)` method returning the option's second-order sensitivity to spot price.
3. The BlackScholes shall provide `vega(strike, expiry)` method returning the option's sensitivity to volatility.
4. The BlackScholes shall provide `theta(strike, expiry)` method returning the option's sensitivity to time decay.
5. The BlackScholes shall provide `rho(strike, expiry)` method returning the option's sensitivity to interest rate.

### Requirement 4: Cumulative Normal Distribution

**Objective:** As a quant developer, I want a differentiable cumulative normal distribution function, so that I can compute Black-Scholes prices with AD compatibility.

#### Acceptance Criteria

1. The analytical module shall provide `norm_cdf<T: Float>(x)` function computing the standard normal CDF.
2. The norm_cdf shall use a numerically stable approximation accurate to at least 1e-7.
3. The analytical module shall provide `norm_pdf<T: Float>(x)` function computing the standard normal PDF.
4. While computing norm_cdf, the implementation shall avoid branching operations for AD compatibility.
5. The norm_cdf shall produce correct results for extreme values (|x| > 8).

### Requirement 5: Generic Type Compatibility

**Objective:** As a quant developer, I want all analytical models to work with both f64 and Dual64, so that I can compute sensitivities via automatic differentiation.

#### Acceptance Criteria

1. The BlackScholes and Bachelier structs shall be generic over `T: Float`.
2. When models are instantiated with `Dual64`, the models shall propagate derivatives correctly through pricing calculations.
3. The implementations shall use smooth approximations from `pricer_core::math::smoothing` where applicable.
4. While computing prices and Greeks, the implementations shall maintain AD tape consistency.
5. The analytical module shall include tests verifying derivative propagation against analytical Greeks.

### Requirement 6: Option Pricing Interface

**Objective:** As a quant developer, I want a unified pricing interface for analytical models, so that I can price VanillaOption instruments consistently.

#### Acceptance Criteria

1. The BlackScholes shall provide `price_option(option: &VanillaOption<T>)` method accepting instrument definitions.
2. When price_option is called with a VanillaOption, the model shall extract strike, expiry, and payoff_type to compute the price.
3. The price_option shall apply notional scaling from the VanillaOption parameters.
4. If the VanillaOption exercise_style is not European, then the model shall return an appropriate error.
5. The pricing interface shall support both Call and Put payoff types.

### Requirement 7: Model Error Handling

**Objective:** As a quant developer, I want consistent error handling for analytical pricing operations, so that I can gracefully handle invalid inputs and edge cases.

#### Acceptance Criteria

1. The analytical module shall define `AnalyticalError` enum covering all failure modes.
2. If volatility is non-positive, then the module shall return `AnalyticalError::InvalidVolatility`.
3. If spot price is non-positive for Black-Scholes, then the module shall return `AnalyticalError::InvalidSpot`.
4. When an unsupported exercise style is provided, the module shall return `AnalyticalError::UnsupportedExerciseStyle`.
5. The AnalyticalError shall integrate with `PricingError` for unified error handling.

### Requirement 8: d1/d2 Term Calculation

**Objective:** As a quant developer, I want reusable d1/d2 term calculations for Black-Scholes, so that I can compute prices and Greeks efficiently with shared intermediate values.

#### Acceptance Criteria

1. The BlackScholes shall provide internal `d1(strike, expiry)` method computing the d1 term.
2. The BlackScholes shall provide internal `d2(strike, expiry)` method computing the d2 term.
3. When d1 and d2 are computed, the implementation shall cache sqrt(T) for efficiency.
4. If expiry approaches zero, then the d1/d2 computation shall handle the limiting case correctly.
5. The d1/d2 calculations shall maintain numerical stability for extreme moneyness levels.
