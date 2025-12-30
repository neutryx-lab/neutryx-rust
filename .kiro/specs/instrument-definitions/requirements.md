# Requirements Document

## Introduction

本仕様は、定量金融プライシングのための金融商品定義を規定する。trait object (`Box<dyn Trait>`) を使用せず、enum dispatch アーキテクチャによりEnzyme AD互換性を確保する。全構造体は `T: Float` ジェネリクスを使用し、自動微分をサポートする。

## Requirements

### Requirement 1: Instrument Enum Definition

**Objective:** As a quant developer, I want a unified instrument enum with variants for common derivatives, so that I can price different instruments without trait objects.

#### Acceptance Criteria

1. The Instrument module shall define a generic enum `Instrument<T: Float>` with variants for European, American, Asian, Bermudan options, forward contracts, and swaps.
2. When an Instrument variant is constructed, the Instrument shall store the corresponding instrument-specific parameters.
3. The Instrument enum shall implement static dispatch for pricing operations without dynamic allocation.
4. The Instrument enum shall be generic over `T: Float` to support both `f64` and `Dual64` types.
5. The Instrument enum shall derive Clone, Debug for diagnostic purposes.

### Requirement 2: PayoffType Enum Definition

**Objective:** As a quant developer, I want a payoff type enum with smooth implementations, so that I can price options with differentiable payoffs.

#### Acceptance Criteria

1. The PayoffType module shall define an enum `PayoffType` with variants for Call, Put, and Digital payoffs.
2. When `PayoffType::Call` is evaluated with spot price S and strike K, the PayoffType shall return `max(S - K, 0)` using smooth approximation.
3. When `PayoffType::Put` is evaluated with spot price S and strike K, the PayoffType shall return `max(K - S, 0)` using smooth approximation.
4. When `PayoffType::Digital` is evaluated, the PayoffType shall return a smoothed indicator function using `smooth_indicator`.
5. The PayoffType shall be generic over `T: Float` for AD compatibility.

### Requirement 3: InstrumentParams Struct

**Objective:** As a quant developer, I want a common parameters struct for shared instrument properties, so that I can avoid duplication across instrument types.

#### Acceptance Criteria

1. The InstrumentParams struct shall contain common fields: strike, expiry, notional.
2. When InstrumentParams is constructed, the struct shall validate that strike > 0 and expiry > 0.
3. The InstrumentParams shall provide accessor methods for each field.
4. The InstrumentParams shall be generic over `T: Float` for AD compatibility.
5. If invalid parameters are provided (non-positive strike or expiry), then the InstrumentParams shall return `InstrumentError::InvalidParameter`.

### Requirement 4: VanillaOption Struct

**Objective:** As a quant developer, I want a vanilla option struct combining common params and payoff type, so that I can represent European and American options.

#### Acceptance Criteria

1. The VanillaOption struct shall contain InstrumentParams, PayoffType, and ExerciseStyle.
2. The VanillaOption shall implement a `payoff(spot)` method that delegates to PayoffType with smooth approximations.
3. When payoff is called with a valid spot price, the VanillaOption shall return the appropriate payoff value.
4. The VanillaOption shall be generic over `T: Float` for AD compatibility.
5. The VanillaOption shall derive Clone, Debug.

### Requirement 5: ExerciseStyle Enum

**Objective:** As a quant developer, I want an exercise style enum to distinguish option exercise types, so that I can model European, American, Bermudan, and Asian options correctly.

#### Acceptance Criteria

1. The ExerciseStyle module shall define an enum with variants: European, American, Bermudan, Asian.
2. When `ExerciseStyle::Bermudan` is used, the ExerciseStyle shall store exercise dates as a vector.
3. When `ExerciseStyle::Asian` is used, the ExerciseStyle shall store averaging parameters (start, end, frequency).
4. The ExerciseStyle shall be generic over `T: Float` where date parameters require floating-point times.
5. The ExerciseStyle shall derive Clone, Debug, PartialEq.

### Requirement 6: Forward Contract Struct

**Objective:** As a quant developer, I want a forward contract struct for non-option derivatives, so that I can price linear instruments.

#### Acceptance Criteria

1. The Forward struct shall contain strike price, expiry time, notional amount, and direction (long/short).
2. When the forward payoff is computed with spot S and strike K, the Forward shall return `notional * (S - K)` for long, `notional * (K - S)` for short.
3. The Forward shall be generic over `T: Float` for AD compatibility.
4. The Forward shall derive Clone, Debug.
5. If expiry is non-positive, then the Forward construction shall return an error.

### Requirement 7: Swap Contract Struct

**Objective:** As a quant developer, I want a swap contract struct for interest rate products, so that I can price basic swap structures.

#### Acceptance Criteria

1. The Swap struct shall contain notional, fixed rate, payment schedule, and currency.
2. When Swap is constructed with payment dates, the Swap shall validate date ordering and non-negative notional.
3. The Swap shall store payment frequency (Annual, SemiAnnual, Quarterly, Monthly).
4. The Swap shall be generic over `T: Float` for rate calculations.
5. The Swap shall derive Clone, Debug.

### Requirement 8: Instrument Error Handling

**Objective:** As a quant developer, I want consistent error handling for instrument operations, so that I can gracefully handle construction and pricing errors.

#### Acceptance Criteria

1. The instruments module shall define `InstrumentError` enum covering all failure modes.
2. When an invalid strike (non-positive) is provided, the module shall return `InstrumentError::InvalidStrike`.
3. When an invalid expiry (non-positive) is provided, the module shall return `InstrumentError::InvalidExpiry`.
4. When payoff computation fails, the module shall return `InstrumentError::PayoffError`.
5. The InstrumentError shall integrate with `PricingError` for unified error handling.

### Requirement 9: Smooth Payoff Functions

**Objective:** As a quant developer, I want smooth payoff implementations using existing smoothing infrastructure, so that I can ensure AD tape consistency.

#### Acceptance Criteria

1. The payoff implementations shall use `smooth_max` from `pricer_core::math::smoothing` for max operations.
2. When a digital payoff is computed, the implementation shall use `smooth_indicator` for discontinuity smoothing.
3. The smoothing epsilon shall be configurable per instrument instance.
4. While computing payoffs, the implementations shall maintain AD tape consistency for Enzyme compatibility.
5. The smooth payoff functions shall produce numerically stable results near discontinuity boundaries.

### Requirement 10: Generic Type Compatibility

**Objective:** As a quant developer, I want all instrument types to work with both f64 and Dual64, so that I can compute sensitivities via automatic differentiation.

#### Acceptance Criteria

1. The Instrument enum and all structs shall be generic over `T: Float`.
2. When instruments are instantiated with `Dual64`, the instruments shall propagate derivatives correctly through payoff calculations.
3. The implementations shall avoid branching operations that break AD tape consistency.
4. While computing Greeks (delta, gamma, vega), the instruments shall maintain differentiability.
5. The instruments module shall include tests verifying derivative propagation through payoff functions.
