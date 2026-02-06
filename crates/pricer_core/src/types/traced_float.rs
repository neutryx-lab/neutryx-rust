//! # TracedFloat - Computation Graph Building Numeric Type
//!
//! `TracedFloat` is a floating-point type that automatically builds a
//! computation graph while performing calculations. It implements
//! `num_traits::Float` so it can be used with existing generic code (`T:
//! Float`).
//!
//! ## Feature Gate
//!
//! This module is only available when the `execution-trace` feature is enabled.
//!
//! ## Design Note
//!
//! `TracedFloat` is `Copy` (required by `num_traits::Float`) and accesses the
//! execution trace through a thread-local context. You must set up the trace
//! context before creating TracedFloat values.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use std::cell::RefCell;
//! use std::rc::Rc;
//! use pricer_core::types::traced::{ExecutionTrace, set_trace_context, clear_trace_context};
//! use pricer_core::types::TracedFloat;
//!
//! // Set up trace context
//! let trace = Rc::new(RefCell::new(ExecutionTrace::new()));
//! set_trace_context(Rc::clone(&trace));
//!
//! // Create traced values
//! let x = TracedFloat::input(100.0, "spot");
//! let y = TracedFloat::input(0.2, "vol");
//! let z = x * y;  // Automatically records Mul operation
//!
//! assert_eq!(z.value(), 20.0);
//! assert_eq!(trace.borrow().node_count(), 3);
//!
//! // Clean up
//! clear_trace_context();
//! ```

use std::{
    cmp::Ordering,
    fmt,
    ops::{Add, Div, Mul, Neg, Rem, Sub},
    panic::Location,
};

use num_traits::{Float, FloatConst, FromPrimitive, Num, NumCast, One, ToPrimitive, Zero};

use super::traced::{get_trace_context, NodeId, Operation, SourceLocation};

/// A floating-point value that tracks its computation graph.
///
/// `TracedFloat` wraps an `f64` value and a node ID. Operations are recorded
/// to the thread-local trace context.
///
/// This type is `Copy` as required by `num_traits::Float`.
#[derive(Clone, Copy)]
pub struct TracedFloat {
    value: f64,
    node_id: NodeId,
}

impl TracedFloat {
    /// Creates a new input TracedFloat with a label.
    ///
    /// Input nodes are the starting points of the computation graph,
    /// typically representing market data or model parameters.
    ///
    /// # Panics
    ///
    /// Panics if no trace context is set.
    #[track_caller]
    pub fn input(value: f64, label: &str) -> Self {
        let trace = get_trace_context().expect("TracedFloat::input() requires an active trace context. Call set_trace_context() first.");
        let location = SourceLocation::from_location(Location::caller());
        let node_id = trace.borrow_mut().add_input(value, label, location);
        Self { value, node_id }
    }

    /// Creates a new constant TracedFloat (no label).
    ///
    /// # Panics
    ///
    /// Panics if no trace context is set.
    #[track_caller]
    pub fn constant(value: f64) -> Self {
        let trace = get_trace_context().expect("TracedFloat::constant() requires an active trace context. Call set_trace_context() first.");
        let location = SourceLocation::from_location(Location::caller());
        let node_id = trace.borrow_mut().add_constant(value, location);
        Self { value, node_id }
    }

    /// Creates a TracedFloat from a value without recording in trace.
    ///
    /// This is an internal method used for operations that don't need tracing.
    #[inline]
    const fn from_raw(value: f64, node_id: NodeId) -> Self { Self { value, node_id } }

    /// Returns the wrapped f64 value.
    #[must_use]
    pub const fn value(self) -> f64 { self.value }

    /// Returns the node ID in the computation graph.
    #[must_use]
    pub const fn node_id(self) -> NodeId { self.node_id }

    /// Creates a new TracedFloat from a unary operation.
    #[track_caller]
    fn unary_op(self, op: Operation, result: f64) -> Self {
        let trace =
            get_trace_context().expect("TracedFloat operations require an active trace context");
        let location = SourceLocation::from_location(Location::caller());
        let node_id = trace
            .borrow_mut()
            .add_node(op, result, location, vec![self.node_id]);
        Self::from_raw(result, node_id)
    }

    /// Creates a new TracedFloat from a binary operation.
    #[track_caller]
    fn binary_op(self, other: Self, op: Operation, result: f64) -> Self {
        let trace =
            get_trace_context().expect("TracedFloat operations require an active trace context");
        let location = SourceLocation::from_location(Location::caller());
        let node_id =
            trace
                .borrow_mut()
                .add_node(op, result, location, vec![self.node_id, other.node_id]);
        Self::from_raw(result, node_id)
    }

    /// Creates a new TracedFloat from a ternary operation.
    #[track_caller]
    fn ternary_op(self, b: Self, c: Self, op: Operation, result: f64) -> Self {
        let trace =
            get_trace_context().expect("TracedFloat operations require an active trace context");
        let location = SourceLocation::from_location(Location::caller());
        let node_id = trace.borrow_mut().add_node(
            op,
            result,
            location,
            vec![self.node_id, b.node_id, c.node_id],
        );
        Self::from_raw(result, node_id)
    }

    /// Internal helper to add a constant node.
    #[track_caller]
    fn add_constant_node(value: f64) -> Self {
        let trace =
            get_trace_context().expect("TracedFloat operations require an active trace context");
        let location = SourceLocation::from_location(Location::caller());
        let node_id = trace.borrow_mut().add_constant(value, location);
        Self::from_raw(value, node_id)
    }
}

// =============================================================================
// Debug and Display
// =============================================================================

impl fmt::Debug for TracedFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TracedFloat")
            .field("value", &self.value)
            .field("node_id", &self.node_id)
            .finish()
    }
}

impl fmt::Display for TracedFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.value) }
}

// =============================================================================
// Comparison (PartialEq, PartialOrd)
// =============================================================================

impl PartialEq for TracedFloat {
    fn eq(&self, other: &Self) -> bool { self.value == other.value }
}

impl PartialOrd for TracedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { self.value.partial_cmp(&other.value) }
}

// =============================================================================
// Arithmetic Operators (Add, Sub, Mul, Div, Neg, Rem)
// =============================================================================

impl Add for TracedFloat {
    type Output = Self;

    #[track_caller]
    fn add(self, rhs: Self) -> Self::Output {
        let result = self.value + rhs.value;
        self.binary_op(rhs, Operation::Add, result)
    }
}

impl Sub for TracedFloat {
    type Output = Self;

    #[track_caller]
    fn sub(self, rhs: Self) -> Self::Output {
        let result = self.value - rhs.value;
        self.binary_op(rhs, Operation::Sub, result)
    }
}

impl Mul for TracedFloat {
    type Output = Self;

    #[track_caller]
    fn mul(self, rhs: Self) -> Self::Output {
        let result = self.value * rhs.value;
        self.binary_op(rhs, Operation::Mul, result)
    }
}

impl Div for TracedFloat {
    type Output = Self;

    #[track_caller]
    fn div(self, rhs: Self) -> Self::Output {
        let result = self.value / rhs.value;
        self.binary_op(rhs, Operation::Div, result)
    }
}

impl Neg for TracedFloat {
    type Output = Self;

    #[track_caller]
    fn neg(self) -> Self::Output {
        let result = -self.value;
        self.unary_op(Operation::Neg, result)
    }
}

impl Rem for TracedFloat {
    type Output = Self;

    #[track_caller]
    fn rem(self, rhs: Self) -> Self::Output {
        let result = self.value % rhs.value;
        self.binary_op(rhs, Operation::Rem, result)
    }
}

// =============================================================================
// Zero and One traits
// =============================================================================

impl Zero for TracedFloat {
    #[track_caller]
    fn zero() -> Self { Self::add_constant_node(0.0) }

    fn is_zero(&self) -> bool { self.value == 0.0 }
}

impl One for TracedFloat {
    #[track_caller]
    fn one() -> Self { Self::add_constant_node(1.0) }
}

// =============================================================================
// Num trait
// =============================================================================

impl Num for TracedFloat {
    type FromStrRadixErr = <f64 as Num>::FromStrRadixErr;

    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        f64::from_str_radix(str, radix).map(Self::add_constant_node)
    }
}

// =============================================================================
// NumCast, ToPrimitive, FromPrimitive
// =============================================================================

impl ToPrimitive for TracedFloat {
    fn to_i64(&self) -> Option<i64> { self.value.to_i64() }

    fn to_u64(&self) -> Option<u64> { self.value.to_u64() }

    fn to_f64(&self) -> Option<f64> { Some(self.value) }
}

impl FromPrimitive for TracedFloat {
    #[track_caller]
    fn from_i64(n: i64) -> Option<Self> { f64::from_i64(n).map(Self::add_constant_node) }

    #[track_caller]
    fn from_u64(n: u64) -> Option<Self> { f64::from_u64(n).map(Self::add_constant_node) }

    #[track_caller]
    fn from_f64(n: f64) -> Option<Self> { Some(Self::add_constant_node(n)) }
}

impl NumCast for TracedFloat {
    #[track_caller]
    fn from<T: ToPrimitive>(n: T) -> Option<Self> { n.to_f64().and_then(FromPrimitive::from_f64) }
}

// =============================================================================
// FloatConst trait
// =============================================================================

impl FloatConst for TracedFloat {
    #[track_caller]
    fn E() -> Self { Self::add_constant_node(std::f64::consts::E) }

    #[track_caller]
    fn FRAC_1_PI() -> Self { Self::add_constant_node(std::f64::consts::FRAC_1_PI) }

    #[track_caller]
    fn FRAC_1_SQRT_2() -> Self { Self::add_constant_node(std::f64::consts::FRAC_1_SQRT_2) }

    #[track_caller]
    fn FRAC_2_PI() -> Self { Self::add_constant_node(std::f64::consts::FRAC_2_PI) }

    #[track_caller]
    fn FRAC_2_SQRT_PI() -> Self { Self::add_constant_node(std::f64::consts::FRAC_2_SQRT_PI) }

    #[track_caller]
    fn FRAC_PI_2() -> Self { Self::add_constant_node(std::f64::consts::FRAC_PI_2) }

    #[track_caller]
    fn FRAC_PI_3() -> Self { Self::add_constant_node(std::f64::consts::FRAC_PI_3) }

    #[track_caller]
    fn FRAC_PI_4() -> Self { Self::add_constant_node(std::f64::consts::FRAC_PI_4) }

    #[track_caller]
    fn FRAC_PI_6() -> Self { Self::add_constant_node(std::f64::consts::FRAC_PI_6) }

    #[track_caller]
    fn FRAC_PI_8() -> Self { Self::add_constant_node(std::f64::consts::FRAC_PI_8) }

    #[track_caller]
    fn LN_10() -> Self { Self::add_constant_node(std::f64::consts::LN_10) }

    #[track_caller]
    fn LN_2() -> Self { Self::add_constant_node(std::f64::consts::LN_2) }

    #[track_caller]
    fn LOG10_E() -> Self { Self::add_constant_node(std::f64::consts::LOG10_E) }

    #[track_caller]
    fn LOG2_E() -> Self { Self::add_constant_node(std::f64::consts::LOG2_E) }

    #[track_caller]
    fn PI() -> Self { Self::add_constant_node(std::f64::consts::PI) }

    #[track_caller]
    fn SQRT_2() -> Self { Self::add_constant_node(std::f64::consts::SQRT_2) }
}

// =============================================================================
// Float trait (the main implementation)
// =============================================================================

impl Float for TracedFloat {
    #[track_caller]
    fn nan() -> Self { Self::add_constant_node(f64::NAN) }

    #[track_caller]
    fn infinity() -> Self { Self::add_constant_node(f64::INFINITY) }

    #[track_caller]
    fn neg_infinity() -> Self { Self::add_constant_node(f64::NEG_INFINITY) }

    #[track_caller]
    fn neg_zero() -> Self { Self::add_constant_node(-0.0) }

    #[track_caller]
    fn min_value() -> Self { Self::add_constant_node(f64::MIN) }

    #[track_caller]
    fn min_positive_value() -> Self { Self::add_constant_node(f64::MIN_POSITIVE) }

    #[track_caller]
    fn max_value() -> Self { Self::add_constant_node(f64::MAX) }

    fn is_nan(self) -> bool { self.value.is_nan() }

    fn is_infinite(self) -> bool { self.value.is_infinite() }

    fn is_finite(self) -> bool { self.value.is_finite() }

    fn is_normal(self) -> bool { self.value.is_normal() }

    fn classify(self) -> std::num::FpCategory { self.value.classify() }

    #[track_caller]
    fn floor(self) -> Self { self.unary_op(Operation::Floor, self.value.floor()) }

    #[track_caller]
    fn ceil(self) -> Self { self.unary_op(Operation::Ceil, self.value.ceil()) }

    #[track_caller]
    fn round(self) -> Self { self.unary_op(Operation::Round, self.value.round()) }

    #[track_caller]
    fn trunc(self) -> Self { self.unary_op(Operation::Trunc, self.value.trunc()) }

    #[track_caller]
    fn fract(self) -> Self { self.unary_op(Operation::Fract, self.value.fract()) }

    #[track_caller]
    fn abs(self) -> Self { self.unary_op(Operation::Abs, self.value.abs()) }

    #[track_caller]
    fn signum(self) -> Self { self.unary_op(Operation::Signum, self.value.signum()) }

    fn is_sign_positive(self) -> bool { self.value.is_sign_positive() }

    fn is_sign_negative(self) -> bool { self.value.is_sign_negative() }

    #[track_caller]
    fn mul_add(self, a: Self, b: Self) -> Self {
        let result = self.value.mul_add(a.value, b.value);
        self.ternary_op(a, b, Operation::MulAdd, result)
    }

    #[track_caller]
    fn recip(self) -> Self { self.unary_op(Operation::Recip, self.value.recip()) }

    #[track_caller]
    fn powi(self, n: i32) -> Self {
        let result = self.value.powi(n);
        let trace =
            get_trace_context().expect("TracedFloat operations require an active trace context");
        let location = SourceLocation::from_location(Location::caller());
        let n_node = trace
            .borrow_mut()
            .add_constant(<f64 as From<i32>>::from(n), location.clone());
        let node_id = trace.borrow_mut().add_node(
            Operation::Powi,
            result,
            location,
            vec![self.node_id, n_node],
        );
        Self::from_raw(result, node_id)
    }

    #[track_caller]
    fn powf(self, n: Self) -> Self {
        let result = self.value.powf(n.value);
        self.binary_op(n, Operation::Powf, result)
    }

    #[track_caller]
    fn sqrt(self) -> Self { self.unary_op(Operation::Sqrt, self.value.sqrt()) }

    #[track_caller]
    fn exp(self) -> Self { self.unary_op(Operation::Exp, self.value.exp()) }

    #[track_caller]
    fn exp2(self) -> Self { self.unary_op(Operation::Exp2, self.value.exp2()) }

    #[track_caller]
    fn ln(self) -> Self { self.unary_op(Operation::Ln, self.value.ln()) }

    #[track_caller]
    fn log(self, base: Self) -> Self {
        // log_base(x) = ln(x) / ln(base)
        let ln_self = self.value.ln();
        let ln_base = base.value.ln();
        let result = ln_self / ln_base;

        let trace =
            get_trace_context().expect("TracedFloat operations require an active trace context");
        let location = SourceLocation::from_location(Location::caller());

        let ln_self_id = trace.borrow_mut().add_node(
            Operation::Ln,
            ln_self,
            location.clone(),
            vec![self.node_id],
        );
        let ln_base_id = trace.borrow_mut().add_node(
            Operation::Ln,
            ln_base,
            location.clone(),
            vec![base.node_id],
        );
        let node_id = trace.borrow_mut().add_node(
            Operation::Div,
            result,
            location,
            vec![ln_self_id, ln_base_id],
        );
        Self::from_raw(result, node_id)
    }

    #[track_caller]
    fn log2(self) -> Self { self.unary_op(Operation::Log2, self.value.log2()) }

    #[track_caller]
    fn log10(self) -> Self { self.unary_op(Operation::Log10, self.value.log10()) }

    #[track_caller]
    fn max(self, other: Self) -> Self {
        let result = self.value.max(other.value);
        self.binary_op(other, Operation::Max, result)
    }

    #[track_caller]
    fn min(self, other: Self) -> Self {
        let result = self.value.min(other.value);
        self.binary_op(other, Operation::Min, result)
    }

    #[track_caller]
    fn abs_sub(self, other: Self) -> Self {
        let result = (self.value - other.value).abs();
        self.binary_op(other, Operation::AbsDiffEq, result)
    }

    #[track_caller]
    fn cbrt(self) -> Self {
        // cbrt(x) = x^(1/3)
        let result = self.value.cbrt();
        let trace =
            get_trace_context().expect("TracedFloat operations require an active trace context");
        let location = SourceLocation::from_location(Location::caller());
        let third_id = trace.borrow_mut().add_constant(1.0 / 3.0, location.clone());
        let node_id = trace.borrow_mut().add_node(
            Operation::Powf,
            result,
            location,
            vec![self.node_id, third_id],
        );
        Self::from_raw(result, node_id)
    }

    #[track_caller]
    fn hypot(self, other: Self) -> Self {
        let result = self.value.hypot(other.value);
        self.binary_op(other, Operation::Hypot, result)
    }

    #[track_caller]
    fn sin(self) -> Self { self.unary_op(Operation::Sin, self.value.sin()) }

    #[track_caller]
    fn cos(self) -> Self { self.unary_op(Operation::Cos, self.value.cos()) }

    #[track_caller]
    fn tan(self) -> Self { self.unary_op(Operation::Tan, self.value.tan()) }

    #[track_caller]
    fn asin(self) -> Self { self.unary_op(Operation::Asin, self.value.asin()) }

    #[track_caller]
    fn acos(self) -> Self { self.unary_op(Operation::Acos, self.value.acos()) }

    #[track_caller]
    fn atan(self) -> Self { self.unary_op(Operation::Atan, self.value.atan()) }

    #[track_caller]
    fn atan2(self, other: Self) -> Self {
        let result = self.value.atan2(other.value);
        self.binary_op(other, Operation::Atan2, result)
    }

    #[track_caller]
    fn sin_cos(self) -> (Self, Self) {
        let (sin_val, cos_val) = self.value.sin_cos();
        let trace =
            get_trace_context().expect("TracedFloat operations require an active trace context");
        let location = SourceLocation::from_location(Location::caller());

        let sin_id = trace.borrow_mut().add_node(
            Operation::Sin,
            sin_val,
            location.clone(),
            vec![self.node_id],
        );
        let cos_id =
            trace
                .borrow_mut()
                .add_node(Operation::Cos, cos_val, location, vec![self.node_id]);

        (
            Self::from_raw(sin_val, sin_id),
            Self::from_raw(cos_val, cos_id),
        )
    }

    #[track_caller]
    fn exp_m1(self) -> Self { self.unary_op(Operation::ExpM1, self.value.exp_m1()) }

    #[track_caller]
    fn ln_1p(self) -> Self { self.unary_op(Operation::Ln1p, self.value.ln_1p()) }

    #[track_caller]
    fn sinh(self) -> Self { self.unary_op(Operation::Sinh, self.value.sinh()) }

    #[track_caller]
    fn cosh(self) -> Self { self.unary_op(Operation::Cosh, self.value.cosh()) }

    #[track_caller]
    fn tanh(self) -> Self { self.unary_op(Operation::Tanh, self.value.tanh()) }

    #[track_caller]
    fn asinh(self) -> Self { self.unary_op(Operation::Asinh, self.value.asinh()) }

    #[track_caller]
    fn acosh(self) -> Self { self.unary_op(Operation::Acosh, self.value.acosh()) }

    #[track_caller]
    fn atanh(self) -> Self { self.unary_op(Operation::Atanh, self.value.atanh()) }

    fn integer_decode(self) -> (u64, i16, i8) { self.value.integer_decode() }

    #[track_caller]
    fn epsilon() -> Self { Self::add_constant_node(f64::EPSILON) }

    #[track_caller]
    fn copysign(self, sign: Self) -> Self {
        let result = self.value.copysign(sign.value);
        self.binary_op(sign, Operation::Copysign, result)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use approx::assert_relative_eq;

    use super::*;
    use crate::types::traced::{clear_trace_context, set_trace_context, ExecutionTrace};

    fn setup_trace() -> Rc<RefCell<ExecutionTrace>> {
        let trace = Rc::new(RefCell::new(ExecutionTrace::new()));
        set_trace_context(Rc::clone(&trace));
        trace
    }

    fn teardown() { clear_trace_context(); }

    mod basic_tests {
        use super::*;

        #[test]
        fn test_input_creation() {
            let trace = setup_trace();
            let x = TracedFloat::input(100.0, "spot");

            assert_eq!(x.value(), 100.0);
            assert_eq!(x.node_id(), NodeId::new(0));
            assert_eq!(trace.borrow().node_count(), 1);

            teardown();
        }

        #[test]
        fn test_constant_creation() {
            let trace = setup_trace();
            let c = TracedFloat::constant(3.14);

            assert_relative_eq!(c.value(), 3.14);
            assert_eq!(c.node_id(), NodeId::new(0));

            teardown();
        }

        #[test]
        fn test_display() {
            let trace = setup_trace();
            let x = TracedFloat::input(42.5, "x");

            assert_eq!(format!("{x}"), "42.5");

            teardown();
        }

        #[test]
        fn test_debug() {
            let trace = setup_trace();
            let x = TracedFloat::input(1.0, "x");

            let debug_str = format!("{x:?}");
            assert!(debug_str.contains("TracedFloat"));
            assert!(debug_str.contains("value"));
            assert!(debug_str.contains("node_id"));

            teardown();
        }

        #[test]
        fn test_copy() {
            let trace = setup_trace();
            let a = TracedFloat::input(5.0, "a");
            let b = a; // Copy
            let c = a; // Copy again

            assert_eq!(a.value(), b.value());
            assert_eq!(a.value(), c.value());
            assert_eq!(a.node_id(), b.node_id());

            teardown();
        }
    }

    mod arithmetic_tests {
        use super::*;

        #[test]
        fn test_add() {
            let trace = setup_trace();
            let a = TracedFloat::input(10.0, "a");
            let b = TracedFloat::input(5.0, "b");
            let c = a + b;

            assert_eq!(c.value(), 15.0);
            assert_eq!(trace.borrow().node_count(), 3);
            assert_eq!(trace.borrow().edge_count(), 2);

            teardown();
        }

        #[test]
        fn test_sub() {
            let trace = setup_trace();
            let a = TracedFloat::input(10.0, "a");
            let b = TracedFloat::input(3.0, "b");
            let c = a - b;

            assert_eq!(c.value(), 7.0);

            teardown();
        }

        #[test]
        fn test_mul() {
            let trace = setup_trace();
            let a = TracedFloat::input(4.0, "a");
            let b = TracedFloat::input(5.0, "b");
            let c = a * b;

            assert_eq!(c.value(), 20.0);

            teardown();
        }

        #[test]
        fn test_div() {
            let trace = setup_trace();
            let a = TracedFloat::input(20.0, "a");
            let b = TracedFloat::input(4.0, "b");
            let c = a / b;

            assert_eq!(c.value(), 5.0);

            teardown();
        }

        #[test]
        fn test_neg() {
            let trace = setup_trace();
            let a = TracedFloat::input(5.0, "a");
            let b = -a;

            assert_eq!(b.value(), -5.0);
            assert_eq!(trace.borrow().node_count(), 2);

            teardown();
        }

        #[test]
        fn test_rem() {
            let trace = setup_trace();
            let a = TracedFloat::input(17.0, "a");
            let b = TracedFloat::input(5.0, "b");
            let c = a % b;

            assert_eq!(c.value(), 2.0);

            teardown();
        }

        #[test]
        fn test_chained_operations() {
            let trace = setup_trace();
            let a = TracedFloat::input(2.0, "a");
            let b = TracedFloat::input(3.0, "b");
            let c = TracedFloat::input(4.0, "c");

            // (a + b) * c = (2 + 3) * 4 = 20
            let result = (a + b) * c;

            assert_eq!(result.value(), 20.0);
            // 3 inputs + 1 add + 1 mul = 5 nodes
            assert_eq!(trace.borrow().node_count(), 5);
            // add has 2 edges, mul has 2 edges = 4 edges
            assert_eq!(trace.borrow().edge_count(), 4);

            teardown();
        }
    }

    mod comparison_tests {
        use super::*;

        #[test]
        fn test_partial_eq() {
            let trace = setup_trace();
            let a = TracedFloat::input(5.0, "a");
            let b = TracedFloat::input(5.0, "b");
            let c = TracedFloat::input(3.0, "c");

            assert_eq!(a, b);
            assert_ne!(a, c);

            teardown();
        }

        #[test]
        fn test_partial_ord() {
            let trace = setup_trace();
            let a = TracedFloat::input(5.0, "a");
            let b = TracedFloat::input(3.0, "b");

            assert!(a > b);
            assert!(b < a);

            teardown();
        }
    }

    mod float_operations_tests {
        use super::*;

        #[test]
        fn test_sqrt() {
            let trace = setup_trace();
            let a = TracedFloat::input(16.0, "a");
            let b = a.sqrt();

            assert_eq!(b.value(), 4.0);

            teardown();
        }

        #[test]
        fn test_exp_ln() {
            let trace = setup_trace();
            let a = TracedFloat::input(1.0, "a");
            let exp_a = a.exp();
            let ln_exp_a = exp_a.ln();

            assert_relative_eq!(ln_exp_a.value(), 1.0, epsilon = 1e-10);

            teardown();
        }

        #[test]
        fn test_sin_cos() {
            let trace = setup_trace();
            let a = TracedFloat::input(0.0, "a");
            let sin_a = a.sin();
            let cos_a = a.cos();

            assert_relative_eq!(sin_a.value(), 0.0, epsilon = 1e-10);
            assert_relative_eq!(cos_a.value(), 1.0, epsilon = 1e-10);

            teardown();
        }

        #[test]
        fn test_powf() {
            let trace = setup_trace();
            let base = TracedFloat::input(2.0, "base");
            let exp = TracedFloat::input(3.0, "exp");
            let result = base.powf(exp);

            assert_eq!(result.value(), 8.0);

            teardown();
        }

        #[test]
        fn test_powi() {
            let trace = setup_trace();
            let base = TracedFloat::input(2.0, "base");
            let result = base.powi(4);

            assert_eq!(result.value(), 16.0);

            teardown();
        }

        #[test]
        fn test_abs() {
            let trace = setup_trace();
            let a = TracedFloat::input(-5.0, "a");
            let b = a.abs();

            assert_eq!(b.value(), 5.0);

            teardown();
        }

        #[test]
        fn test_floor_ceil_round() {
            let trace = setup_trace();
            let a = TracedFloat::input(3.7, "a");

            assert_eq!(a.floor().value(), 3.0);
            assert_eq!(a.ceil().value(), 4.0);
            assert_eq!(a.round().value(), 4.0);

            teardown();
        }

        #[test]
        fn test_max_min() {
            let trace = setup_trace();
            let a = TracedFloat::input(5.0, "a");
            let b = TracedFloat::input(3.0, "b");

            assert_eq!(a.max(b).value(), 5.0);
            assert_eq!(a.min(b).value(), 3.0);

            teardown();
        }

        #[test]
        fn test_recip() {
            let trace = setup_trace();
            let a = TracedFloat::input(4.0, "a");
            let b = a.recip();

            assert_eq!(b.value(), 0.25);

            teardown();
        }

        #[test]
        fn test_mul_add() {
            let trace = setup_trace();
            let a = TracedFloat::input(2.0, "a");
            let b = TracedFloat::input(3.0, "b");
            let c = TracedFloat::input(4.0, "c");
            // a * b + c = 2 * 3 + 4 = 10
            let result = a.mul_add(b, c);

            assert_eq!(result.value(), 10.0);

            teardown();
        }

        #[test]
        fn test_hypot() {
            let trace = setup_trace();
            let a = TracedFloat::input(3.0, "a");
            let b = TracedFloat::input(4.0, "b");
            let c = a.hypot(b);

            assert_eq!(c.value(), 5.0);

            teardown();
        }
    }

    mod special_values_tests {
        use super::*;

        #[test]
        fn test_is_nan() {
            let trace = setup_trace();
            let a = TracedFloat::input(f64::NAN, "nan");

            assert!(a.is_nan());

            teardown();
        }

        #[test]
        fn test_is_infinite() {
            let trace = setup_trace();
            let a = TracedFloat::input(f64::INFINITY, "inf");

            assert!(a.is_infinite());
            assert!(!a.is_finite());

            teardown();
        }

        #[test]
        fn test_is_finite() {
            let trace = setup_trace();
            let a = TracedFloat::input(42.0, "a");

            assert!(a.is_finite());
            assert!(!a.is_infinite());

            teardown();
        }
    }

    mod zero_one_tests {
        use super::*;

        #[test]
        fn test_zero() {
            let _trace = setup_trace();
            let zero: TracedFloat = Zero::zero();

            assert!(zero.is_zero());
            assert_eq!(zero.value(), 0.0);

            teardown();
        }

        #[test]
        fn test_one() {
            let _trace = setup_trace();
            let one: TracedFloat = One::one();

            assert_eq!(one.value(), 1.0);

            teardown();
        }
    }

    mod from_primitive_tests {
        use super::*;

        #[test]
        fn test_from_f64() {
            let _trace = setup_trace();
            let a: TracedFloat = FromPrimitive::from_f64(3.14).unwrap();

            assert_relative_eq!(a.value(), 3.14);

            teardown();
        }

        #[test]
        fn test_from_i64() {
            let _trace = setup_trace();
            let a: TracedFloat = FromPrimitive::from_i64(42).unwrap();

            assert_eq!(a.value(), 42.0);

            teardown();
        }

        #[test]
        fn test_num_cast() {
            let _trace = setup_trace();
            let a: TracedFloat = NumCast::from(100i32).unwrap();

            assert_eq!(a.value(), 100.0);

            teardown();
        }
    }

    mod graph_structure_tests {
        use super::*;
        use crate::types::traced::Operation;

        #[test]
        fn test_graph_captures_operations() {
            let trace = setup_trace();
            let x = TracedFloat::input(10.0, "x");
            let y = TracedFloat::input(20.0, "y");
            let _z = x * y;

            let nodes = trace.borrow();
            assert_eq!(nodes.node_count(), 3);

            // Check operations
            assert_eq!(nodes.nodes()[0].operation, Operation::Input);
            assert_eq!(nodes.nodes()[1].operation, Operation::Input);
            assert_eq!(nodes.nodes()[2].operation, Operation::Mul);

            // Check labels
            assert_eq!(nodes.nodes()[0].label, Some("x".to_string()));
            assert_eq!(nodes.nodes()[1].label, Some("y".to_string()));
            assert!(nodes.nodes()[2].label.is_none());

            teardown();
        }

        #[test]
        fn test_graph_captures_edges() {
            let trace = setup_trace();
            let a = TracedFloat::input(1.0, "a");
            let b = TracedFloat::input(2.0, "b");
            let c = TracedFloat::input(3.0, "c");

            // a + b => node 3, then (a+b) * c => node 4
            let ab = a + b;
            let _result = ab * c;

            let edges = trace.borrow().edges().to_vec();
            assert_eq!(edges.len(), 4);

            // Edges for a + b
            assert_eq!(edges[0].source, NodeId::new(0));
            assert_eq!(edges[0].target, NodeId::new(3));
            assert_eq!(edges[1].source, NodeId::new(1));
            assert_eq!(edges[1].target, NodeId::new(3));

            // Edges for (a+b) * c
            assert_eq!(edges[2].source, NodeId::new(3));
            assert_eq!(edges[2].target, NodeId::new(4));
            assert_eq!(edges[3].source, NodeId::new(2));
            assert_eq!(edges[3].target, NodeId::new(4));

            teardown();
        }
    }

    mod generic_function_test {
        use super::*;

        /// A generic function that works with any `T: Float`.
        /// This demonstrates that TracedFloat can be used with existing generic
        /// code.
        fn compute_price<T: Float>(spot: T, vol: T, time: T) -> T {
            let sqrt_t = time.sqrt();
            spot * vol * sqrt_t
        }

        #[test]
        fn test_generic_function_with_traced_float() {
            let trace = setup_trace();

            let spot = TracedFloat::input(100.0, "spot");
            let vol = TracedFloat::input(0.2, "vol");
            let time = TracedFloat::input(1.0, "time");

            let price = compute_price(spot, vol, time);

            // spot=100, vol=0.2, time=1 => 100 * 0.2 * sqrt(1) = 20
            assert_relative_eq!(price.value(), 20.0, epsilon = 1e-10);

            // Check graph structure: 3 inputs + sqrt + 2 muls = 6 nodes
            assert_eq!(trace.borrow().node_count(), 6);

            teardown();
        }

        #[test]
        fn test_same_result_as_f64() {
            let trace = setup_trace();

            // Compute with f64
            let f64_result = compute_price(100.0_f64, 0.2_f64, 1.0_f64);

            // Compute with TracedFloat
            let spot = TracedFloat::input(100.0, "spot");
            let vol = TracedFloat::input(0.2, "vol");
            let time = TracedFloat::input(1.0, "time");
            let traced_result = compute_price(spot, vol, time);

            assert_relative_eq!(traced_result.value(), f64_result, epsilon = 1e-10);

            teardown();
        }
    }
}
