//! Convexity adjustment for CMS and in-arrears rate products.
//!
//! Provides a [`ConvexityAdjuster`] trait and concrete implementations
//! for computing CMS convexity adjustments under different model assumptions.
//!
//! ## Implementations
//!
//! | Adjuster | Description |
//! |----------|-------------|
//! | [`ConundrumIntegrandConvexityAdjuster`] | CMS replication via numerical integration or analytic formulae |
//! | [`NonConvexityAdjuster`] | No-op (zero adjustment) |

pub mod conundrum;
pub mod non_adjuster;
pub mod params;
pub mod support;

pub use conundrum::ConundrumIntegrandConvexityAdjuster;
use enum_dispatch::enum_dispatch;
pub use non_adjuster::NonConvexityAdjuster;
pub use params::{ConvexityAdjustCalcMethod, ConvexityAdjusterParams};
use pricer_core::traits::Float;

// ─── Error type ─────────────────────────────────────────────────────

/// Errors arising from convexity adjustment operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ConvexityAdjustmentError {
    /// The option term is non-positive when a positive value is required.
    NonPositiveOptionTerm { term: f64 },
    /// Numerical integration failed to converge.
    IntegrationFailure { iterations: usize },
    /// Invalid parameters supplied.
    InvalidParameters { message: String },
}

impl std::fmt::Display for ConvexityAdjustmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonPositiveOptionTerm { term } => {
                write!(f, "option term is non-positive: {}", term)
            }
            Self::IntegrationFailure { iterations } => {
                write!(
                    f,
                    "integration failed to converge after {} iterations",
                    iterations
                )
            }
            Self::InvalidParameters { message } => {
                write!(f, "invalid parameters: {}", message)
            }
        }
    }
}

impl std::error::Error for ConvexityAdjustmentError {}

// ─── Trait ──────────────────────────────────────────────────────────

/// Trait for convexity adjustment computation.
///
/// Implementations compute CMS convexity adjustments, in-arrears
/// adjustments, and CMS-linked option prices under different model
/// assumptions.
///
/// All dates are expressed as year-fractions. Market data (vol,
/// discount factor, annuity) is passed as pre-computed values.
/// Option prices are provided via closures for AD compatibility.
#[enum_dispatch]
pub trait ConvexityAdjuster<T: Float> {
    /// Whether a convexity adjustment should be applied based on the
    /// gap between index end date and payment date.
    fn does_apply(&self, end_date_yf: T, payment_date_yf: T) -> bool;

    /// Computes the additive convexity adjustment for a rate index.
    ///
    /// # Arguments
    /// * `option_price_fn` - `fn(strike, is_call) -> price`
    /// * `time_value_fn` - `fn(strike) -> time_value`
    fn compute_adjustment(
        &self,
        ref_term: T,
        pay_freq: T,
        fwd_swap: T,
        lo_spread: T,
        effective_date_yf: T,
        end_date_yf: T,
        pay_date_yf: T,
        option_price_fn: &dyn Fn(T, bool) -> T,
        time_value_fn: &dyn Fn(T) -> T,
        normal_vol: T,
        option_term: T,
        daycount_adjust: T,
    ) -> Result<T, ConvexityAdjustmentError>;

    /// Computes the CMS swaplet value (adjusted forward rate for a CMS swap).
    ///
    /// # Arguments
    /// * `option_price_fn` - `fn(strike, is_call) -> price` (used by numerical
    ///   method)
    /// * `time_value_fn` - `fn(strike) -> time_value` (used by numerical
    ///   method)
    fn calc_swaplet_value(
        &self,
        ref_term: T,
        pay_freq: T,
        fwd_swap: T,
        lo_spread: T,
        effective_date_yf: T,
        first_payment_date_yf: T,
        pay_date_yf: T,
        annuity: T,
        discount_factor_pay: T,
        normal_vol: T,
        sln_vol: T,
        shift_size: T,
        option_term: T,
        daycount_adjust: T,
        option_price_fn: &dyn Fn(T, bool) -> T,
        time_value_fn: &dyn Fn(T) -> T,
    ) -> Result<T, ConvexityAdjustmentError>;

    /// Computes the CMS caplet value.
    ///
    /// # Arguments
    /// * `call_price_at_strike` - Pre-computed call option price at the given
    ///   strike
    /// * `option_price_fn` - `fn(strike, is_call) -> price` (used by numerical
    ///   method)
    /// * `time_value_fn` - `fn(strike) -> time_value` (used by numerical
    ///   method)
    fn calc_caplet_value(
        &self,
        ref_term: T,
        pay_freq: T,
        fwd_swap: T,
        strike: T,
        lo_spread: T,
        effective_date_yf: T,
        first_payment_date_yf: T,
        pay_date_yf: T,
        annuity: T,
        discount_factor_pay: T,
        normal_vol: T,
        sln_vol: T,
        shift_size: T,
        option_term: T,
        daycount_adjust: T,
        call_price_at_strike: T,
        option_price_fn: &dyn Fn(T, bool) -> T,
        time_value_fn: &dyn Fn(T) -> T,
    ) -> Result<T, ConvexityAdjustmentError>;
}

// ─── Dispatch enum ──────────────────────────────────────────────────

/// Static-dispatch enum wrapping all convexity adjuster variants.
///
/// Uses `enum_dispatch` for zero-cost dynamic polymorphism,
/// keeping the code Enzyme-friendly (no trait objects).
#[derive(Debug, Clone)]
#[enum_dispatch(ConvexityAdjuster<T>)]
pub enum ConvexityAdjusterEnum<T: Float> {
    /// Conundrum integrand method (CMS convexity via replication).
    Conundrum(ConundrumIntegrandConvexityAdjuster<T>),
    /// No-op adjuster (returns zero adjustment).
    None(NonConvexityAdjuster<T>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_dispatch_none_variant() {
        let adjuster = ConvexityAdjusterEnum::<f64>::None(NonConvexityAdjuster::new());
        assert!(!adjuster.does_apply(1.0, 2.0));
    }

    #[test]
    fn enum_dispatch_conundrum_variant() {
        let adjuster = ConvexityAdjusterEnum::<f64>::Conundrum(
            ConundrumIntegrandConvexityAdjuster::default_normal_analytic(),
        );
        // Outside grace period
        assert!(adjuster.does_apply(10.0, 10.5));
    }

    #[test]
    fn error_display() {
        let e = ConvexityAdjustmentError::NonPositiveOptionTerm { term: -0.5 };
        assert_eq!(format!("{}", e), "option term is non-positive: -0.5");

        let e = ConvexityAdjustmentError::IntegrationFailure { iterations: 1000 };
        assert_eq!(
            format!("{}", e),
            "integration failed to converge after 1000 iterations"
        );

        let e = ConvexityAdjustmentError::InvalidParameters {
            message: "bad value".to_string(),
        };
        assert_eq!(format!("{}", e), "invalid parameters: bad value");
    }
}
