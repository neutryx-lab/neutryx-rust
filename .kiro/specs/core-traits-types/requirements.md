# Requirements Document

## Project Description (Input)
Phase 1: Core Traits & Types - Date, Currency, and Pricing interfaces

## Introduction

Phase 1は、XVA価格計算ライブラリの基盤となるLayer 1（pricer_core）の中核機能を確立します。このフェーズでは、数値型、トレイト、時間型、通貨型、および微分可能性を保証するスムージング関数を実装し、stable Rustで動作する依存関係ゼロの基礎レイヤーを構築します。

本要件書は、Enzyme自動微分との統合を見据えた設計（Phase 3で実装）を考慮しつつ、num-dualベースの二重数演算をデフォルトモードとして採用し、正確性検証を優先します。

## Requirements

### Requirement 1: 数値型システム（Dual Number Support）

**Objective:** As a quantitative developer, I want type-safe dual number operations, so that I can compute derivatives with automatic differentiation

#### Acceptance Criteria

1. The pricer_core shall provide a `DualNumber<T>` type wrapping num-dual for first-order automatic differentiation
2. The pricer_core shall implement standard arithmetic operations (+, -, *, /) for `DualNumber<T>` where T implements `num_traits::Float`
3. When a user constructs a dual number with value and perturbation, the pricer_core shall preserve both components for derivative propagation
4. The pricer_core shall provide conversion traits (`From`, `Into`) between `f64` and `DualNumber<f64>`
5. The pricer_core shall implement `Debug`, `Display`, `Clone`, and `Copy` traits for `DualNumber<T>`

### Requirement 2: コアトレイト定義（Pricing and Differentiability Traits）

**Objective:** As a pricing model developer, I want abstract interfaces for priceable instruments, so that I can implement uniform pricing logic across different asset classes

#### Acceptance Criteria

1. The pricer_core shall define a `Priceable` trait with a `price(&self, market_data: &M) -> f64` method signature
2. The pricer_core shall define a `Differentiable` trait with a `gradient(&self, market_data: &M) -> Vec<f64>` method for parameter sensitivities
3. When a type implements `Differentiable`, the pricer_core shall provide default implementation for common Greeks (delta, gamma, vega) via trait methods
4. The pricer_core shall enforce that all trait methods are generic over market data type `M` for flexibility
5. The pricer_core shall use static dispatch (no `Box<dyn Trait>`) for Enzyme LLVM optimization compatibility

### Requirement 3: 時間型システム（Date and Time Handling）

**Objective:** As a derivatives pricing developer, I want robust date arithmetic and day count conventions, so that I can accurately calculate time fractions for discounting and rate calculations

#### Acceptance Criteria

1. The pricer_core shall provide a `Date` type wrapping `chrono::NaiveDate` for calendar date representation
2. When a user calculates the difference between two dates, the pricer_core shall return a `Duration` type representing the time interval
3. The pricer_core shall implement an `ActualActual` day count convention calculating year fractions as `(date2 - date1).num_days() as f64 / 365.25`
4. The pricer_core shall implement a `Thirty360` day count convention for bond pricing with the 30/360 day count method
5. The pricer_core shall provide a `DayCountConvention` enum supporting at least `ActualActual`, `Thirty360`, and `Actual365Fixed` variants
6. When a user converts a date pair to a year fraction, the pricer_core shall provide a `year_fraction(start: Date, end: Date, convention: DayCountConvention) -> f64` function

### Requirement 4: 通貨型システム（Currency Types）

**Objective:** As a multi-currency derivatives trader, I want type-safe currency handling, so that I can prevent currency mismatch errors at compile time

#### Acceptance Criteria

1. The pricer_core shall provide a `Currency` enum with at least `USD`, `EUR`, `JPY`, `GBP`, `CHF` variants
2. The pricer_core shall implement `Display` trait for `Currency` outputting ISO 4217 currency codes
3. The pricer_core shall provide a `Money` type parameterized by currency as `Money<C: Currency>` containing `(f64, C)` for amount and currency
4. When a user attempts to add two `Money` values with different currencies, the pricer_core shall enforce currency matching at compile time via type system
5. The pricer_core shall implement arithmetic operations for `Money<C>` (addition, subtraction, scalar multiplication) preserving currency type

### Requirement 5: スムージング関数（Differentiable Smoothing Functions）

**Objective:** As an algorithmic differentiation engineer, I want smooth approximations of discontinuous functions, so that I can ensure differentiability throughout pricing calculations

#### Acceptance Criteria

1. The pricer_core shall provide a `smooth_max(x: f64, y: f64, epsilon: f64) -> f64` function implementing `x + epsilon * ln(1 + exp((y - x) / epsilon))`
2. When epsilon approaches zero in `smooth_max`, the pricer_core shall approximate `max(x, y)` with controllable smoothness
3. The pricer_core shall provide a `smooth_indicator(x: f64, threshold: f64, epsilon: f64) -> f64` function implementing sigmoid-based indicator approximation `1.0 / (1.0 + exp(-(x - threshold) / epsilon))`
4. The pricer_core shall provide a `smooth_abs(x: f64, epsilon: f64) -> f64` function implementing `sqrt(x * x + epsilon * epsilon)`
5. When epsilon is set to `1e-8`, the pricer_core shall ensure smooth functions maintain numerical stability (no NaN, no Inf for normal input ranges)
6. The pricer_core shall document the mathematical derivatives of all smoothing functions in doc comments for verification purposes

### Requirement 6: 型安全性とエラー処理（Type Safety and Error Handling）

**Objective:** As a library maintainer, I want comprehensive type safety and error handling, so that I can catch errors at compile time and provide clear runtime diagnostics

#### Acceptance Criteria

1. The pricer_core shall use Rust's type system to enforce unit correctness (no raw `f64` for dimensioned quantities)
2. When invalid inputs occur (e.g., negative epsilon in smoothing functions), the pricer_core shall return `Result<T, PricerError>` with descriptive error messages
3. The pricer_core shall define a `PricerError` enum with at least `InvalidParameter`, `NumericalInstability`, and `DimensionMismatch` variants
4. The pricer_core shall implement `std::error::Error` and `Display` traits for `PricerError`
5. Where floating-point operations might produce NaN or Inf, the pricer_core shall validate results and return errors for invalid values

### Requirement 7: テストとドキュメント（Testing and Documentation）

**Objective:** As a library consumer, I want comprehensive tests and documentation, so that I can trust the correctness and understand how to use the library

#### Acceptance Criteria

1. The pricer_core shall provide unit tests for all public functions with at least 90% code coverage
2. When testing smoothing functions, the pricer_core shall verify convergence to non-smooth counterparts as epsilon → 0 using `approx::assert_relative_eq!`
3. The pricer_core shall provide property-based tests using `proptest` for mathematical invariants (e.g., dual number arithmetic properties)
4. The pricer_core shall include rustdoc comments with mathematical formulas (using LaTeX in doc comments) for all public types and functions
5. When running `cargo test -p pricer_core`, the pricer_core shall execute all tests successfully without warnings on stable Rust

### Requirement 8: 依存関係とツールチェーン（Dependencies and Toolchain）

**Objective:** As a build engineer, I want minimal, stable dependencies, so that I can ensure reproducible builds and long-term stability

#### Acceptance Criteria

1. The pricer_core shall compile on stable Rust (Edition 2021) without nightly features
2. The pricer_core shall depend only on `num-traits`, `num-dual`, `chrono`, `thiserror` (no other pricer_* crates)
3. When building pricer_core, the pricer_core shall produce zero warnings with `cargo clippy -- -D warnings`
4. The pricer_core shall format all code according to `cargo fmt --all -- --check` standards
5. The pricer_core shall pass `cargo build --release` with LTO and `codegen-units = 1` optimization settings

### Requirement 9: 将来のEnzyme統合準備（Enzyme Integration Preparation）

**Objective:** As an AD backend architect, I want design patterns compatible with future Enzyme integration, so that I can seamlessly transition to LLVM-level automatic differentiation in Phase 3

#### Acceptance Criteria

1. The pricer_core shall use static dispatch (concrete types, enum) instead of dynamic dispatch (`Box<dyn Trait>`) for all core types
2. When implementing mathematical functions, the pricer_core shall avoid runtime polymorphism that would inhibit LLVM inlining
3. The pricer_core shall provide feature flags `num-dual-mode` (default) and `enzyme-mode` for conditional compilation of AD backends
4. Where `enzyme-mode` is enabled, the pricer_core shall conditionally compile Enzyme-compatible dual number implementations (stub for Phase 1, full implementation in Phase 3)
5. The pricer_core shall document all functions requiring Enzyme compatibility with `#[inline]` attributes and LLVM optimization hints

### Requirement 10: パフォーマンス特性（Performance Characteristics）

**Objective:** As a quantitative researcher, I want high-performance numerical operations, so that I can run large-scale Monte Carlo simulations efficiently

#### Acceptance Criteria

1. The pricer_core shall implement all arithmetic operations for `DualNumber<T>` with zero-cost abstractions (no heap allocation in hot paths)
2. When benchmarked with `criterion`, the pricer_core shall demonstrate that `smooth_max` overhead is within 10% of standard `f64::max` for epsilon = 1e-8
3. The pricer_core shall use `#[inline]` attributes for all small functions (< 10 lines) to enable cross-crate inlining
4. The pricer_core shall avoid unnecessary allocations in day count fraction calculations (stack-only operations)
5. When compiled with `--release`, the pricer_core shall produce LLVM IR suitable for vectorization (verified with `cargo asm` or similar tools)

