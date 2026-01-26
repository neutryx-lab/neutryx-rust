//! Payoff evaluation for cashflow calculations.
//!
//! This module provides the `PayoffEvaluator` struct for computing
//! cashflow amounts based on different payoff types (Fixed, Linear,
//! VanillaOption, Digital).
//!
//! **Requires the `l1l2-integration` feature.**

use infra_master::trade::{IndexType, OptionType, Payoff};
use num_traits::Float;
use pricer_models::market::CurveSet;

use super::error::PricingError;

/// Payoff evaluator for computing cashflow amounts.
///
/// This struct evaluates different payoff types (Fixed, Linear, VanillaOption,
/// Digital) to compute the actual cashflow amount given market data.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Example
///
/// ```
/// use pricer_pricing::generic_pricer::PayoffEvaluator;
/// use pricer_models::market::curves::{CurveSet, CurveName, CurveEnum};
/// use infra_master::trade::Payoff;
///
/// let mut curves = CurveSet::new();
/// curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
///
/// let evaluator = PayoffEvaluator::new(&curves);
///
/// let payoff = Payoff::fixed(0.03);
/// let amount = evaluator.evaluate(&payoff, 1_000_000.0, 0.5, 0.0, 0.5).unwrap();
/// assert!((amount - 15_000.0).abs() < 1e-6); // 1M * 0.03 * 0.5
/// ```
pub struct PayoffEvaluator<'a, T: Float> {
    curve_set: &'a CurveSet<T>,
}

impl<'a, T: Float + 'static> PayoffEvaluator<'a, T> {
    /// Creates a new `PayoffEvaluator` with the given curve set.
    pub fn new(curve_set: &'a CurveSet<T>) -> Self { Self { curve_set } }

    /// Evaluates a payoff and returns the cashflow amount.
    ///
    /// # Arguments
    ///
    /// * `payoff` - The payoff type to evaluate
    /// * `notional` - The notional principal amount
    /// * `year_fraction` - The accrual period year fraction
    /// * `start_time` - Start time (year fraction from valuation date)
    /// * `end_time` - End time (year fraction from valuation date)
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - The computed cashflow amount
    /// * `Err(PricingError)` - If evaluation fails (e.g., missing curve)
    ///
    /// # Payoff Types
    ///
    /// - **Fixed**: `notional * rate * year_fraction`
    /// - **Linear**: `notional * (forward_rate + spread) * multiplier *
    ///   year_fraction`
    /// - **VanillaOption**: Cap/Floor pricing (using Black's model)
    /// - **Digital**: Binary option (not yet implemented, returns 0)
    pub fn evaluate(
        &self,
        payoff: &Payoff,
        notional: T,
        year_fraction: T,
        start_time: T,
        end_time: T,
    ) -> Result<T, PricingError> {
        match payoff {
            Payoff::Fixed { rate } => self.evaluate_fixed(*rate, notional, year_fraction),
            Payoff::Linear {
                index,
                spread,
                multiplier,
            } => self.evaluate_linear(
                index,
                notional,
                year_fraction,
                start_time,
                end_time,
                *spread,
                *multiplier,
            ),
            Payoff::VanillaOption {
                index,
                strike,
                option_type,
            } => self.evaluate_vanilla_option(
                index,
                notional,
                year_fraction,
                start_time,
                end_time,
                *strike,
                option_type,
            ),
            Payoff::Digital { .. } => {
                // Digital options not yet implemented
                Ok(T::zero())
            }
        }
    }

    /// Evaluates a fixed rate payoff.
    fn evaluate_fixed(&self, rate: f64, notional: T, year_fraction: T) -> Result<T, PricingError> {
        let rate_t = T::from(rate).ok_or_else(|| PricingError::InvalidInput {
            reason: "Failed to convert rate to T".to_string(),
        })?;
        Ok(notional * rate_t * year_fraction)
    }

    /// Evaluates a linear (floating) payoff.
    fn evaluate_linear(
        &self,
        index: &IndexType,
        notional: T,
        year_fraction: T,
        start_time: T,
        end_time: T,
        spread: f64,
        multiplier: f64,
    ) -> Result<T, PricingError> {
        let rate_index = index.as_rate().ok_or(PricingError::InvalidInput {
            reason: "Linear payoff requires a rate index".to_string(),
        })?;

        let fwd_rate = self
            .curve_set
            .forward_rate_for_index(*rate_index, start_time, end_time)
            .map_err(|e| PricingError::InvalidInput {
                reason: format!("Failed to get forward rate: {}", e),
            })?;

        let spread_t = T::from(spread).ok_or_else(|| PricingError::InvalidInput {
            reason: "Failed to convert spread to T".to_string(),
        })?;
        let multiplier_t = T::from(multiplier).ok_or_else(|| PricingError::InvalidInput {
            reason: "Failed to convert multiplier to T".to_string(),
        })?;

        let rate_with_spread = fwd_rate + spread_t;
        Ok(notional * rate_with_spread * multiplier_t * year_fraction)
    }

    /// Evaluates a vanilla option (cap/floor) payoff using Black's model.
    ///
    /// For caps/floors, this computes the intrinsic value (max(rate - strike,
    /// 0) for caps, max(strike - rate, 0) for floors) when no vol surface
    /// is available.
    fn evaluate_vanilla_option(
        &self,
        index: &IndexType,
        notional: T,
        year_fraction: T,
        start_time: T,
        end_time: T,
        strike: f64,
        option_type: &OptionType,
    ) -> Result<T, PricingError> {
        let rate_index = index.as_rate().ok_or(PricingError::InvalidInput {
            reason: "VanillaOption payoff requires a rate index".to_string(),
        })?;

        let fwd_rate = self
            .curve_set
            .forward_rate_for_index(*rate_index, start_time, end_time)
            .map_err(|e| PricingError::InvalidInput {
                reason: format!("Failed to get forward rate: {}", e),
            })?;

        let strike_t = T::from(strike).ok_or_else(|| PricingError::InvalidInput {
            reason: "Failed to convert strike to T".to_string(),
        })?;

        // Compute intrinsic value (simplified - no time value without vol surface)
        let intrinsic = match option_type {
            OptionType::Call => {
                // Cap: max(rate - strike, 0)
                if fwd_rate > strike_t {
                    fwd_rate - strike_t
                } else {
                    T::zero()
                }
            }
            OptionType::Put => {
                // Floor: max(strike - rate, 0)
                if strike_t > fwd_rate {
                    strike_t - fwd_rate
                } else {
                    T::zero()
                }
            }
            OptionType::DigitalCall => {
                // Digital call: 1 if rate > strike, else 0
                if fwd_rate > strike_t {
                    T::one()
                } else {
                    T::zero()
                }
            }
            OptionType::DigitalPut => {
                // Digital put: 1 if rate < strike, else 0
                if fwd_rate < strike_t {
                    T::one()
                } else {
                    T::zero()
                }
            }
        };

        Ok(notional * intrinsic * year_fraction)
    }
}

#[cfg(test)]
mod tests {
    use infra_master::RateIndex;
    use pricer_models::market::curves::{CurveEnum, CurveName};
    use OptionType;

    use super::*;

    fn create_curve_set() -> CurveSet<f64> {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
        curves.insert(CurveName::Euribor, CurveEnum::flat(0.04_f64));
        curves
    }

    // ========================================
    // Fixed Payoff Tests
    // ========================================

    #[test]
    fn test_evaluate_fixed_payoff() {
        let curves = create_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        let payoff = Payoff::fixed(0.03);
        let amount = evaluator
            .evaluate(&payoff, 1_000_000.0, 0.5, 0.0, 0.5)
            .unwrap();

        // 1M * 0.03 * 0.5 = 15,000
        assert!((amount - 15_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_evaluate_fixed_payoff_zero_rate() {
        let curves = create_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        let payoff = Payoff::fixed(0.0);
        let amount = evaluator
            .evaluate(&payoff, 1_000_000.0, 0.5, 0.0, 0.5)
            .unwrap();

        assert!((amount - 0.0).abs() < 1e-10);
    }

    // ========================================
    // Linear Payoff Tests
    // ========================================

    #[test]
    fn test_evaluate_linear_payoff_no_spread() {
        let curves = create_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        let payoff = Payoff::floating(IndexType::Rate(RateIndex::Sofr));
        let amount = evaluator
            .evaluate(&payoff, 1_000_000.0, 0.5, 0.5, 1.0)
            .unwrap();

        // 1M * 0.035 * 1.0 * 0.5 = 17,500
        assert!((amount - 17_500.0).abs() < 1.0);
    }

    #[test]
    fn test_evaluate_linear_payoff_with_spread() {
        let curves = create_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        let payoff = Payoff::floating_with_spread(IndexType::Rate(RateIndex::Sofr), 0.005); // 50bp spread
        let amount = evaluator
            .evaluate(&payoff, 1_000_000.0, 0.5, 0.5, 1.0)
            .unwrap();

        // 1M * (0.035 + 0.005) * 1.0 * 0.5 = 20,000
        assert!((amount - 20_000.0).abs() < 1.0);
    }

    #[test]
    fn test_evaluate_linear_payoff_euribor() {
        let curves = create_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        let payoff = Payoff::floating(IndexType::Rate(RateIndex::Euribor3M));
        let amount = evaluator
            .evaluate(&payoff, 1_000_000.0, 0.25, 0.25, 0.5)
            .unwrap();

        // 1M * 0.04 * 1.0 * 0.25 = 10,000
        assert!((amount - 10_000.0).abs() < 1.0);
    }

    #[test]
    fn test_evaluate_linear_payoff_missing_curve() {
        let curves = CurveSet::<f64>::new(); // Empty curve set
        let evaluator = PayoffEvaluator::new(&curves);

        let payoff = Payoff::floating(IndexType::Rate(RateIndex::Sofr));
        let result = evaluator.evaluate(&payoff, 1_000_000.0, 0.5, 0.5, 1.0);

        assert!(result.is_err());
    }

    // ========================================
    // VanillaOption Payoff Tests (Cap/Floor)
    // ========================================

    #[test]
    fn test_evaluate_cap_in_the_money() {
        let curves = create_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        // Cap with strike 3% (SOFR is 3.5%, so ITM)
        let payoff = Payoff::cap(IndexType::Rate(RateIndex::Sofr), 0.03);
        let amount = evaluator
            .evaluate(&payoff, 1_000_000.0, 0.5, 0.5, 1.0)
            .unwrap();

        // 1M * (0.035 - 0.03) * 0.5 = 2,500 (intrinsic only)
        assert!((amount - 2_500.0).abs() < 100.0);
    }

    #[test]
    fn test_evaluate_cap_out_of_the_money() {
        let curves = create_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        // Cap with strike 4% (SOFR is 3.5%, so OTM)
        let payoff = Payoff::cap(IndexType::Rate(RateIndex::Sofr), 0.04);
        let amount = evaluator
            .evaluate(&payoff, 1_000_000.0, 0.5, 0.5, 1.0)
            .unwrap();

        // OTM cap has zero intrinsic value
        assert!((amount - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_evaluate_floor_in_the_money() {
        let curves = create_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        // Floor with strike 4% (SOFR is 3.5%, so ITM)
        let payoff = Payoff::floor(IndexType::Rate(RateIndex::Sofr), 0.04);
        let amount = evaluator
            .evaluate(&payoff, 1_000_000.0, 0.5, 0.5, 1.0)
            .unwrap();

        // 1M * (0.04 - 0.035) * 0.5 = 2,500 (intrinsic only)
        assert!((amount - 2_500.0).abs() < 100.0);
    }

    #[test]
    fn test_evaluate_floor_out_of_the_money() {
        let curves = create_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        // Floor with strike 3% (SOFR is 3.5%, so OTM)
        let payoff = Payoff::floor(IndexType::Rate(RateIndex::Sofr), 0.03);
        let amount = evaluator
            .evaluate(&payoff, 1_000_000.0, 0.5, 0.5, 1.0)
            .unwrap();

        // OTM floor has zero intrinsic value
        assert!((amount - 0.0).abs() < 1e-6);
    }

    // ========================================
    // Digital Payoff Tests
    // ========================================

    #[test]
    fn test_evaluate_digital_returns_zero() {
        let curves = create_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        let payoff = Payoff::Digital {
            index: IndexType::Rate(RateIndex::Sofr),
            strike: 0.03,
            option_type: OptionType::Call,
            payout: 10000.0,
        };
        let amount = evaluator
            .evaluate(&payoff, 1_000_000.0, 0.5, 0.5, 1.0)
            .unwrap();

        // Digital not implemented, returns zero
        assert!((amount - 0.0).abs() < 1e-10);
    }
}
