//! Vanilla option definitions.
//!
//! This module provides vanilla option structures combining
//! common parameters, payoff types, and exercise styles.

use num_traits::Float;

use super::exercise::ExerciseStyle;
use super::params::InstrumentParams;
use super::payoff::PayoffType;

/// Vanilla option instrument.
///
/// Combines instrument parameters, payoff type, and exercise style
/// to represent a complete vanilla option contract.
///
/// # Type Parameters
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Examples
/// ```
/// use pricer_models::instruments::{
///     VanillaOption, InstrumentParams, PayoffType, ExerciseStyle,
/// };
///
/// let params = InstrumentParams::new(100.0_f64, 1.0, 1_000_000.0).unwrap();
/// let option = VanillaOption::new(
///     params,
///     PayoffType::Call,
///     ExerciseStyle::European,
///     1e-6,
/// );
///
/// // Calculate payoff at expiry
/// let payoff = option.payoff(110.0);
/// assert!((payoff - 10_000_000.0).abs() < 1000.0); // notional * (S - K)
/// ```
#[derive(Debug, Clone)]
pub struct VanillaOption<T: Float> {
    params: InstrumentParams<T>,
    payoff_type: PayoffType,
    exercise_style: ExerciseStyle<T>,
    epsilon: T,
}

impl<T: Float> VanillaOption<T> {
    /// Creates a new vanilla option.
    ///
    /// # Arguments
    /// * `params` - Instrument parameters (strike, expiry, notional)
    /// * `payoff_type` - Type of payoff (Call, Put, Digital)
    /// * `exercise_style` - Exercise style (European, American, etc.)
    /// * `epsilon` - Smoothing parameter for AD-compatible payoff
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::{
    ///     VanillaOption, InstrumentParams, PayoffType, ExerciseStyle,
    /// };
    ///
    /// let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
    /// let option = VanillaOption::new(
    ///     params,
    ///     PayoffType::Call,
    ///     ExerciseStyle::European,
    ///     1e-6,
    /// );
    /// ```
    pub fn new(
        params: InstrumentParams<T>,
        payoff_type: PayoffType,
        exercise_style: ExerciseStyle<T>,
        epsilon: T,
    ) -> Self {
        Self {
            params,
            payoff_type,
            exercise_style,
            epsilon,
        }
    }

    /// Calculates the payoff at expiry for a given spot price.
    ///
    /// Returns `notional * payoff_per_unit` where payoff_per_unit
    /// is computed using smooth approximations for AD compatibility.
    ///
    /// # Arguments
    /// * `spot` - Current spot price
    ///
    /// # Returns
    /// Total payoff scaled by notional amount.
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::{
    ///     VanillaOption, InstrumentParams, PayoffType, ExerciseStyle,
    /// };
    ///
    /// let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
    /// let call = VanillaOption::new(
    ///     params,
    ///     PayoffType::Call,
    ///     ExerciseStyle::European,
    ///     1e-6,
    /// );
    ///
    /// // ITM call: payoff ≈ 10
    /// let payoff = call.payoff(110.0);
    /// assert!((payoff - 10.0).abs() < 0.01);
    /// ```
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        let unit_payoff = self
            .payoff_type
            .evaluate(spot, self.params.strike(), self.epsilon);
        self.params.notional() * unit_payoff
    }

    /// Returns a reference to the instrument parameters.
    #[inline]
    pub fn params(&self) -> &InstrumentParams<T> {
        &self.params
    }

    /// Returns the payoff type.
    #[inline]
    pub fn payoff_type(&self) -> PayoffType {
        self.payoff_type
    }

    /// Returns a reference to the exercise style.
    #[inline]
    pub fn exercise_style(&self) -> &ExerciseStyle<T> {
        &self.exercise_style
    }

    /// Returns the smoothing epsilon.
    #[inline]
    pub fn epsilon(&self) -> T {
        self.epsilon
    }

    /// Returns the strike price.
    #[inline]
    pub fn strike(&self) -> T {
        self.params.strike()
    }

    /// Returns the time to expiry.
    #[inline]
    pub fn expiry(&self) -> T {
        self.params.expiry()
    }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> T {
        self.params.notional()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn create_test_params() -> InstrumentParams<f64> {
        InstrumentParams::new(100.0, 1.0, 1.0).unwrap()
    }

    #[test]
    fn test_new_european_call() {
        let params = create_test_params();
        let option = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);

        assert_eq!(option.payoff_type(), PayoffType::Call);
        assert!(option.exercise_style().is_european());
        assert_eq!(option.strike(), 100.0);
        assert_eq!(option.expiry(), 1.0);
        assert_eq!(option.notional(), 1.0);
    }

    #[test]
    fn test_call_payoff_itm() {
        let params = create_test_params();
        let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);

        let payoff = call.payoff(110.0);
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_call_payoff_otm() {
        let params = create_test_params();
        let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);

        let payoff = call.payoff(90.0);
        assert!(payoff < 0.01);
        assert!(payoff >= 0.0);
    }

    #[test]
    fn test_put_payoff_itm() {
        let params = create_test_params();
        let put = VanillaOption::new(params, PayoffType::Put, ExerciseStyle::European, 1e-6);

        let payoff = put.payoff(90.0);
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_put_payoff_otm() {
        let params = create_test_params();
        let put = VanillaOption::new(params, PayoffType::Put, ExerciseStyle::European, 1e-6);

        let payoff = put.payoff(110.0);
        assert!(payoff < 0.01);
        assert!(payoff >= 0.0);
    }

    #[test]
    fn test_payoff_with_notional() {
        let params = InstrumentParams::new(100.0, 1.0, 1_000_000.0).unwrap();
        let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);

        let payoff = call.payoff(110.0);
        // notional * (spot - strike) = 1_000_000 * 10 = 10_000_000
        assert_relative_eq!(payoff, 10_000_000.0, epsilon = 1000.0);
    }

    #[test]
    fn test_digital_call_payoff() {
        let params = create_test_params();
        let digital = VanillaOption::new(
            params,
            PayoffType::DigitalCall,
            ExerciseStyle::European,
            1e-6,
        );

        // ITM digital call
        let payoff_itm = digital.payoff(110.0);
        assert!(payoff_itm > 0.99);

        // OTM digital call
        let payoff_otm = digital.payoff(90.0);
        assert!(payoff_otm < 0.01);
    }

    #[test]
    fn test_american_option() {
        let params = create_test_params();
        let american = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::American, 1e-6);

        assert!(american.exercise_style().is_american());
        assert!(american.exercise_style().allows_early_exercise());
    }

    #[test]
    fn test_bermudan_option() {
        let params = create_test_params();
        let bermudan = VanillaOption::new(
            params,
            PayoffType::Put,
            ExerciseStyle::bermudan(vec![0.25, 0.5, 0.75]),
            1e-6,
        );

        assert!(bermudan.exercise_style().is_bermudan());
        assert!(bermudan.exercise_style().allows_early_exercise());
    }

    #[test]
    fn test_asian_option() {
        let params = create_test_params();
        let asian = VanillaOption::new(
            params,
            PayoffType::Call,
            ExerciseStyle::asian(0.0, 1.0, 12),
            1e-6,
        );

        assert!(asian.exercise_style().is_asian());
        assert!(asian.exercise_style().is_path_dependent());
    }

    #[test]
    fn test_accessors() {
        let params = InstrumentParams::new(105.0, 0.5, 100.0).unwrap();
        let option = VanillaOption::new(params, PayoffType::Put, ExerciseStyle::European, 1e-8);

        assert_eq!(option.strike(), 105.0);
        assert_eq!(option.expiry(), 0.5);
        assert_eq!(option.notional(), 100.0);
        assert_eq!(option.epsilon(), 1e-8);
        assert_eq!(option.payoff_type(), PayoffType::Put);
        assert_eq!(option.params().strike(), 105.0);
    }

    #[test]
    fn test_clone() {
        let params = create_test_params();
        let option1 = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
        let option2 = option1.clone();

        assert_eq!(option1.strike(), option2.strike());
        assert_eq!(option1.payoff_type(), option2.payoff_type());
    }

    #[test]
    fn test_debug() {
        let params = create_test_params();
        let option = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
        let debug_str = format!("{:?}", option);
        assert!(debug_str.contains("VanillaOption"));
    }
}
