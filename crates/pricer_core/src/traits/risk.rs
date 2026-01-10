//! Risk factor traits and types for scenario-based risk analysis.
//!
//! This module provides the core abstractions for risk factor management:
//! - `RiskFactorType`: Classification of risk factors (rates, credit, FX, etc.)
//! - `RiskFactor`: Trait for bumpable risk factors
//! - `ShiftType`: Types of market data shifts (absolute, relative, parallel, etc.)
//!
//! ## Design Philosophy
//!
//! - **Static dispatch**: All types designed for enum-based dispatch (Enzyme compatible)
//! - **Flexible bumping**: Support for absolute, relative, and structured shifts
//! - **Type safety**: Strong typing prevents misuse of incompatible factor types
//!
//! ## Example
//!
//! ```rust
//! use pricer_core::traits::risk::{RiskFactorType, ShiftType};
//!
//! let factor_type = RiskFactorType::InterestRate;
//! assert!(factor_type.affects_discounting());
//!
//! let shift = ShiftType::parallel(0.0001_f64); // 1bp parallel shift
//! assert!(shift.is_curve_shift());
//! ```

use super::Float;

/// Classification of risk factor types.
///
/// Each risk factor type represents a category of market variables
/// that can be shocked or shifted in scenario analysis.
///
/// # Variants
///
/// - `InterestRate`: Interest rate curve factors (yield curves, swap curves)
/// - `Credit`: Credit spread and default probability factors
/// - `Fx`: Foreign exchange rate factors
/// - `Equity`: Equity price and dividend factors
/// - `Commodity`: Commodity price factors
/// - `Volatility`: Implied volatility surface factors
///
/// # Requirements
///
/// - Requirements: 10.1
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RiskFactorType {
    /// Interest rate curve factors (OIS, SOFR, swap curves, etc.)
    InterestRate,
    /// Credit spread and default probability factors
    Credit,
    /// Foreign exchange rate factors
    Fx,
    /// Equity price and dividend factors
    Equity,
    /// Commodity price factors
    Commodity,
    /// Implied volatility surface factors
    Volatility,
}

impl RiskFactorType {
    /// Returns true if this factor type affects discounting calculations.
    ///
    /// Interest rate and credit factors directly impact discount factors.
    #[inline]
    pub fn affects_discounting(&self) -> bool {
        matches!(self, RiskFactorType::InterestRate | RiskFactorType::Credit)
    }

    /// Returns true if this is a price-based factor (spot prices).
    #[inline]
    pub fn is_price_factor(&self) -> bool {
        matches!(
            self,
            RiskFactorType::Fx | RiskFactorType::Equity | RiskFactorType::Commodity
        )
    }

    /// Returns true if this factor typically requires surface shifts.
    #[inline]
    pub fn is_surface_factor(&self) -> bool {
        matches!(self, RiskFactorType::Volatility)
    }

    /// Get the name of this factor type.
    pub fn name(&self) -> &'static str {
        match self {
            RiskFactorType::InterestRate => "InterestRate",
            RiskFactorType::Credit => "Credit",
            RiskFactorType::Fx => "FX",
            RiskFactorType::Equity => "Equity",
            RiskFactorType::Commodity => "Commodity",
            RiskFactorType::Volatility => "Volatility",
        }
    }
}

/// Types of market data shifts for scenario analysis.
///
/// These represent different ways to perturb market data factors
/// for sensitivity analysis and stress testing.
///
/// # Requirements
///
/// - Requirements: 10.2
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShiftType<T: Float> {
    /// Absolute shift: add a fixed value.
    ///
    /// `new_value = old_value + shift_amount`
    Absolute(T),

    /// Relative shift: multiply by (1 + percentage).
    ///
    /// `new_value = old_value * (1 + shift_percentage)`
    Relative(T),

    /// Parallel shift: uniform shift across all tenors/strikes.
    ///
    /// Used for curve-level shocks (e.g., +1bp parallel shift).
    Parallel(T),

    /// Twist shift: short end moves opposite to long end.
    ///
    /// `short_shift = +amount, long_shift = -amount`
    /// Used for yield curve steepening/flattening analysis.
    Twist {
        /// Shift amount for short end (positive = up)
        short_shift: T,
        /// Shift amount for long end (positive = up)
        long_shift: T,
    },

    /// Butterfly shift: middle of curve moves opposite to wings.
    ///
    /// Used for curvature analysis.
    Butterfly {
        /// Shift for short and long ends
        wing_shift: T,
        /// Shift for middle of curve
        belly_shift: T,
    },
}

impl<T: Float> ShiftType<T> {
    /// Create a new absolute shift.
    pub fn absolute(amount: T) -> Self {
        ShiftType::Absolute(amount)
    }

    /// Create a new relative shift.
    pub fn relative(percentage: T) -> Self {
        ShiftType::Relative(percentage)
    }

    /// Create a new parallel shift.
    pub fn parallel(amount: T) -> Self {
        ShiftType::Parallel(amount)
    }

    /// Create a new twist shift.
    pub fn twist(short_shift: T, long_shift: T) -> Self {
        ShiftType::Twist {
            short_shift,
            long_shift,
        }
    }

    /// Create a new butterfly shift.
    pub fn butterfly(wing_shift: T, belly_shift: T) -> Self {
        ShiftType::Butterfly {
            wing_shift,
            belly_shift,
        }
    }

    /// Returns true if this is a curve-level shift (parallel, twist, butterfly).
    pub fn is_curve_shift(&self) -> bool {
        matches!(
            self,
            ShiftType::Parallel(_) | ShiftType::Twist { .. } | ShiftType::Butterfly { .. }
        )
    }

    /// Apply this shift to a value.
    ///
    /// For curve shifts (Parallel, Twist, Butterfly), applies the parallel component.
    pub fn apply(&self, value: T) -> T {
        match *self {
            ShiftType::Absolute(amount) => value + amount,
            ShiftType::Relative(pct) => value * (T::one() + pct),
            ShiftType::Parallel(amount) => value + amount,
            ShiftType::Twist { short_shift, .. } => value + short_shift, // Simplified
            ShiftType::Butterfly { wing_shift, .. } => value + wing_shift, // Simplified
        }
    }
}

/// Trait for risk factors that can be bumped in scenario analysis.
///
/// Implementors of this trait represent market data that can be
/// perturbed for sensitivity analysis, stress testing, and Greeks calculation.
///
/// # Design
///
/// - Designed for static dispatch (implement on concrete types, not dyn)
/// - Supports multiple shift types through `ShiftType` enum
/// - Returns new shifted value rather than mutating in place (functional style)
///
/// # Requirements
///
/// - Requirements: 10.1
pub trait RiskFactor<T: Float>: Clone {
    /// Get the type of this risk factor.
    fn factor_type(&self) -> RiskFactorType;

    /// Get the current value of this factor.
    fn value(&self) -> T;

    /// Create a new factor with a shifted value.
    ///
    /// Returns a new instance with the shift applied.
    fn with_shift(&self, shift: ShiftType<T>) -> Self;

    /// Get factor identifier for reporting.
    fn identifier(&self) -> &str;
}

/// A simple risk factor representing a single market observable.
///
/// This is a basic implementation of `RiskFactor` for scalar values.
#[derive(Clone, Debug)]
pub struct SimpleRiskFactor<T: Float> {
    /// Factor type
    factor_type: RiskFactorType,
    /// Current value
    value: T,
    /// Identifier
    identifier: String,
}

impl<T: Float> SimpleRiskFactor<T> {
    /// Create a new simple risk factor.
    pub fn new(factor_type: RiskFactorType, value: T, identifier: impl Into<String>) -> Self {
        Self {
            factor_type,
            value,
            identifier: identifier.into(),
        }
    }

    /// Create an interest rate factor.
    pub fn interest_rate(value: T, identifier: impl Into<String>) -> Self {
        Self::new(RiskFactorType::InterestRate, value, identifier)
    }

    /// Create a credit spread factor.
    pub fn credit(value: T, identifier: impl Into<String>) -> Self {
        Self::new(RiskFactorType::Credit, value, identifier)
    }

    /// Create an FX rate factor.
    pub fn fx(value: T, identifier: impl Into<String>) -> Self {
        Self::new(RiskFactorType::Fx, value, identifier)
    }

    /// Create an equity price factor.
    pub fn equity(value: T, identifier: impl Into<String>) -> Self {
        Self::new(RiskFactorType::Equity, value, identifier)
    }

    /// Create a volatility factor.
    pub fn volatility(value: T, identifier: impl Into<String>) -> Self {
        Self::new(RiskFactorType::Volatility, value, identifier)
    }
}

impl<T: Float> RiskFactor<T> for SimpleRiskFactor<T> {
    fn factor_type(&self) -> RiskFactorType {
        self.factor_type
    }

    fn value(&self) -> T {
        self.value
    }

    fn with_shift(&self, shift: ShiftType<T>) -> Self {
        Self {
            factor_type: self.factor_type,
            value: shift.apply(self.value),
            identifier: self.identifier.clone(),
        }
    }

    fn identifier(&self) -> &str {
        &self.identifier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 11.1: RiskFactor trait tests (TDD)
    // ================================================================

    // Test: RiskFactorType enum
    #[test]
    fn test_risk_factor_type_affects_discounting() {
        assert!(RiskFactorType::InterestRate.affects_discounting());
        assert!(RiskFactorType::Credit.affects_discounting());
        assert!(!RiskFactorType::Fx.affects_discounting());
        assert!(!RiskFactorType::Equity.affects_discounting());
        assert!(!RiskFactorType::Commodity.affects_discounting());
        assert!(!RiskFactorType::Volatility.affects_discounting());
    }

    #[test]
    fn test_risk_factor_type_is_price_factor() {
        assert!(!RiskFactorType::InterestRate.is_price_factor());
        assert!(!RiskFactorType::Credit.is_price_factor());
        assert!(RiskFactorType::Fx.is_price_factor());
        assert!(RiskFactorType::Equity.is_price_factor());
        assert!(RiskFactorType::Commodity.is_price_factor());
        assert!(!RiskFactorType::Volatility.is_price_factor());
    }

    #[test]
    fn test_risk_factor_type_is_surface_factor() {
        assert!(!RiskFactorType::InterestRate.is_surface_factor());
        assert!(RiskFactorType::Volatility.is_surface_factor());
    }

    #[test]
    fn test_risk_factor_type_name() {
        assert_eq!(RiskFactorType::InterestRate.name(), "InterestRate");
        assert_eq!(RiskFactorType::Credit.name(), "Credit");
        assert_eq!(RiskFactorType::Fx.name(), "FX");
        assert_eq!(RiskFactorType::Equity.name(), "Equity");
        assert_eq!(RiskFactorType::Commodity.name(), "Commodity");
        assert_eq!(RiskFactorType::Volatility.name(), "Volatility");
    }

    #[test]
    fn test_risk_factor_type_clone_copy() {
        let factor = RiskFactorType::InterestRate;
        let cloned = factor.clone();
        let copied = factor;
        assert_eq!(factor, cloned);
        assert_eq!(factor, copied);
    }

    #[test]
    fn test_risk_factor_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(RiskFactorType::InterestRate);
        set.insert(RiskFactorType::Credit);
        set.insert(RiskFactorType::Fx);
        assert_eq!(set.len(), 3);
    }

    // Test: ShiftType enum
    #[test]
    fn test_shift_type_absolute() {
        let shift = ShiftType::absolute(0.01_f64);
        assert_eq!(shift.apply(1.0), 1.01);
    }

    #[test]
    fn test_shift_type_relative() {
        let shift = ShiftType::relative(0.10_f64); // 10%
        assert!((shift.apply(100.0) - 110.0).abs() < 1e-10);
    }

    #[test]
    fn test_shift_type_parallel() {
        let shift = ShiftType::parallel(0.0001_f64); // 1bp
        assert!((shift.apply(0.05) - 0.0501).abs() < 1e-10);
    }

    #[test]
    fn test_shift_type_twist() {
        let shift = ShiftType::<f64>::twist(0.0010, -0.0005);
        assert!(shift.is_curve_shift());
    }

    #[test]
    fn test_shift_type_butterfly() {
        let shift = ShiftType::<f64>::butterfly(-0.0005, 0.0010);
        assert!(shift.is_curve_shift());
    }

    #[test]
    fn test_shift_type_is_curve_shift() {
        assert!(!ShiftType::absolute(0.01_f64).is_curve_shift());
        assert!(!ShiftType::relative(0.10_f64).is_curve_shift());
        assert!(ShiftType::parallel(0.01_f64).is_curve_shift());
        assert!(ShiftType::<f64>::twist(0.01, -0.01).is_curve_shift());
        assert!(ShiftType::<f64>::butterfly(0.01, -0.01).is_curve_shift());
    }

    // Test: SimpleRiskFactor
    #[test]
    fn test_simple_risk_factor_new() {
        let factor = SimpleRiskFactor::new(RiskFactorType::InterestRate, 0.05_f64, "USD.OIS.5Y");
        assert_eq!(factor.factor_type(), RiskFactorType::InterestRate);
        assert_eq!(factor.value(), 0.05);
        assert_eq!(factor.identifier(), "USD.OIS.5Y");
    }

    #[test]
    fn test_simple_risk_factor_constructors() {
        let rate = SimpleRiskFactor::interest_rate(0.03_f64, "JPY.TONAR.1Y");
        assert_eq!(rate.factor_type(), RiskFactorType::InterestRate);

        let credit = SimpleRiskFactor::credit(0.005_f64, "XYZ.CDS.5Y");
        assert_eq!(credit.factor_type(), RiskFactorType::Credit);

        let fx = SimpleRiskFactor::fx(150.0_f64, "USDJPY");
        assert_eq!(fx.factor_type(), RiskFactorType::Fx);

        let equity = SimpleRiskFactor::equity(4500.0_f64, "SPX");
        assert_eq!(equity.factor_type(), RiskFactorType::Equity);

        let vol = SimpleRiskFactor::volatility(0.20_f64, "SPX.ATM.1Y");
        assert_eq!(vol.factor_type(), RiskFactorType::Volatility);
    }

    #[test]
    fn test_simple_risk_factor_with_shift_absolute() {
        let factor = SimpleRiskFactor::interest_rate(0.03_f64, "USD.OIS.5Y");
        let shifted = factor.with_shift(ShiftType::absolute(0.01));

        assert_eq!(factor.value(), 0.03); // Original unchanged
        assert!((shifted.value() - 0.04).abs() < 1e-10);
        assert_eq!(shifted.factor_type(), RiskFactorType::InterestRate);
        assert_eq!(shifted.identifier(), "USD.OIS.5Y");
    }

    #[test]
    fn test_simple_risk_factor_with_shift_relative() {
        let factor = SimpleRiskFactor::equity(100.0_f64, "AAPL");
        let shifted = factor.with_shift(ShiftType::relative(0.05)); // +5%

        assert!((shifted.value() - 105.0).abs() < 1e-10);
    }

    #[test]
    fn test_simple_risk_factor_clone() {
        let factor = SimpleRiskFactor::fx(1.10_f64, "EURUSD");
        let cloned = factor.clone();
        assert_eq!(factor.value(), cloned.value());
        assert_eq!(factor.identifier(), cloned.identifier());
    }

    // Test: f32 compatibility
    #[test]
    fn test_simple_risk_factor_f32() {
        let factor = SimpleRiskFactor::interest_rate(0.03_f32, "EUR.ESTR.1Y");
        assert_eq!(factor.value(), 0.03_f32);

        let shifted = factor.with_shift(ShiftType::absolute(0.01_f32));
        assert!((shifted.value() - 0.04_f32).abs() < 1e-6);
    }
}
