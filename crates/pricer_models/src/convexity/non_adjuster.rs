//! No-op convexity adjuster returning zero adjustment.

use std::marker::PhantomData;

use pricer_core::traits::Float;

use super::{ConvexityAdjuster, ConvexityAdjustmentError};

/// No-op convexity adjuster that always returns zero adjustment.
///
/// Used as a default when no convexity correction is needed.
#[derive(Debug, Clone)]
pub struct NonConvexityAdjuster<T: Float> {
    _phantom: PhantomData<T>,
}

impl<T: Float> Default for NonConvexityAdjuster<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Float> NonConvexityAdjuster<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T: Float> ConvexityAdjuster<T> for NonConvexityAdjuster<T> {
    fn does_apply(&self, _end_date_yf: T, _payment_date_yf: T) -> bool {
        false
    }

    fn compute_adjustment(
        &self,
        _ref_term: T,
        _pay_freq: T,
        _fwd_swap: T,
        _lo_spread: T,
        _effective_date_yf: T,
        _end_date_yf: T,
        _pay_date_yf: T,
        _option_price_fn: &dyn Fn(T, bool) -> T,
        _time_value_fn: &dyn Fn(T) -> T,
        _normal_vol: T,
        _option_term: T,
        _daycount_adjust: T,
    ) -> Result<T, ConvexityAdjustmentError> {
        Ok(T::zero())
    }

    fn calc_swaplet_value(
        &self,
        _ref_term: T,
        _pay_freq: T,
        fwd_swap: T,
        _lo_spread: T,
        _effective_date_yf: T,
        _first_payment_date_yf: T,
        _pay_date_yf: T,
        _annuity: T,
        _discount_factor_pay: T,
        _normal_vol: T,
        _sln_vol: T,
        _shift_size: T,
        _option_term: T,
        _daycount_adjust: T,
        _option_price_fn: &dyn Fn(T, bool) -> T,
        _time_value_fn: &dyn Fn(T) -> T,
    ) -> Result<T, ConvexityAdjustmentError> {
        Ok(fwd_swap)
    }

    fn calc_caplet_value(
        &self,
        _ref_term: T,
        _pay_freq: T,
        fwd_swap: T,
        strike: T,
        _lo_spread: T,
        _effective_date_yf: T,
        _first_payment_date_yf: T,
        _pay_date_yf: T,
        _annuity: T,
        _discount_factor_pay: T,
        _normal_vol: T,
        _sln_vol: T,
        _shift_size: T,
        _option_term: T,
        _daycount_adjust: T,
        call_price_at_strike: T,
        _option_price_fn: &dyn Fn(T, bool) -> T,
        _time_value_fn: &dyn Fn(T) -> T,
    ) -> Result<T, ConvexityAdjustmentError> {
        let _ = (fwd_swap, strike);
        Ok(call_price_at_strike)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_apply_always_false() {
        let adj = NonConvexityAdjuster::<f64>::new();
        assert!(!adj.does_apply(1.0, 2.0));
        assert!(!adj.does_apply(1.0, 1.0));
    }

    #[test]
    fn compute_adjustment_returns_zero() {
        let adj = NonConvexityAdjuster::<f64>::new();
        let price_fn = |_k: f64, _is_call: bool| 0.0;
        let tv_fn = |_k: f64| 0.0;
        let result = adj
            .compute_adjustment(10.0, 2.0, 0.03, 0.0, 0.0, 10.0, 10.5, &price_fn, &tv_fn, 0.005, 1.0, 1.0)
            .unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn swaplet_returns_forward() {
        let adj = NonConvexityAdjuster::<f64>::new();
        let price_fn = |_k: f64, _is_call: bool| 0.0;
        let tv_fn = |_k: f64| 0.0;
        let result = adj
            .calc_swaplet_value(
                10.0, 2.0, 0.03, 0.0, 0.0, 0.5, 10.5, 9.5, 0.98, 0.005, 0.20, 0.0, 1.0, 1.0,
                &price_fn, &tv_fn,
            )
            .unwrap();
        assert_eq!(result, 0.03);
    }

    #[test]
    fn caplet_returns_call_price() {
        let adj = NonConvexityAdjuster::<f64>::new();
        let price_fn = |_k: f64, _is_call: bool| 0.0;
        let tv_fn = |_k: f64| 0.0;
        let result = adj
            .calc_caplet_value(
                10.0, 2.0, 0.03, 0.02, 0.0, 0.0, 0.5, 10.5, 9.5, 0.98, 0.005, 0.20, 0.0, 1.0,
                1.0, 0.0042, &price_fn, &tv_fn,
            )
            .unwrap();
        assert_eq!(result, 0.0042);
    }
}
