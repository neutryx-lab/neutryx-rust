//! Bootstrap instrument definitions.
//!
//! This module provides the `BootstrapInstrument<T>` enum representing
//! market instruments used for yield curve bootstrapping: OIS, IRS, FRA, and Futures.
//!
//! All instrument types are generic over `T: Float` for AD compatibility
//! and use static dispatch (enum-based) for Enzyme optimisation.

use num_traits::Float;

/// Payment frequency for fixed-income instruments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Frequency {
    /// Annual payments (1 per year)
    #[default]
    Annual,
    /// Semi-annual payments (2 per year)
    SemiAnnual,
    /// Quarterly payments (4 per year)
    Quarterly,
    /// Monthly payments (12 per year)
    Monthly,
    /// Daily payments (365 per year)
    Daily,
}

impl Frequency {
    /// Get the number of payments per year.
    pub fn payments_per_year(&self) -> usize {
        match self {
            Frequency::Annual => 1,
            Frequency::SemiAnnual => 2,
            Frequency::Quarterly => 4,
            Frequency::Monthly => 12,
            Frequency::Daily => 365,
        }
    }

    /// Get the period length in years.
    pub fn period_years<T: Float>(&self) -> T {
        T::from(1.0).unwrap() / T::from(self.payments_per_year()).unwrap()
    }
}

/// Market instruments for yield curve bootstrapping.
///
/// Each variant represents a different instrument type used to construct
/// the yield curve, with instrument-specific parameters.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
///
/// # Variants
///
/// - `Ois`: Overnight Index Swap
/// - `Irs`: Interest Rate Swap
/// - `Fra`: Forward Rate Agreement
/// - `Future`: Interest Rate Future
///
/// # Examples
///
/// ```
/// use pricer_optimiser::bootstrapping::BootstrapInstrument;
///
/// // Create a 5-year OIS at 3%
/// let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
/// assert!((ois.maturity() - 5.0).abs() < 1e-10);
/// assert!((ois.rate() - 0.03).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum BootstrapInstrument<T: Float> {
    /// Overnight Index Swap.
    ///
    /// Exchanges fixed rate for compounded overnight rate.
    /// Primary instrument for discount curve construction.
    Ois {
        /// Maturity in years from today
        maturity: T,
        /// Fixed rate (as decimal, e.g., 0.03 for 3%)
        rate: T,
        /// Payment frequency (typically Annual)
        payment_frequency: Frequency,
    },

    /// Interest Rate Swap.
    ///
    /// Exchanges fixed rate for floating rate (e.g., SOFR + spread).
    /// Used for tenor curve construction.
    Irs {
        /// Maturity in years from today
        maturity: T,
        /// Fixed rate (as decimal)
        rate: T,
        /// Fixed leg payment frequency
        fixed_frequency: Frequency,
        /// Floating leg payment frequency
        float_frequency: Frequency,
    },

    /// Forward Rate Agreement.
    ///
    /// Single-period forward contract on an interest rate.
    Fra {
        /// Start time in years from today
        start: T,
        /// End time in years from today
        end: T,
        /// Forward rate (as decimal)
        rate: T,
    },

    /// Interest Rate Future.
    ///
    /// Exchange-traded future on an interest rate.
    /// Price is quoted as 100 - rate (e.g., 97.50 = 2.50% rate).
    Future {
        /// Maturity in years from today
        maturity: T,
        /// Quoted price (100 - rate)
        price: T,
        /// Convexity adjustment (typically small positive value)
        convexity_adjustment: T,
    },
}

impl<T: Float> BootstrapInstrument<T> {
    // ========================================
    // Factory Methods
    // ========================================

    /// Create an OIS instrument with default annual payments.
    pub fn ois(maturity: T, rate: T) -> Self {
        Self::Ois {
            maturity,
            rate,
            payment_frequency: Frequency::Annual,
        }
    }

    /// Create an OIS instrument with custom frequency.
    pub fn ois_with_frequency(maturity: T, rate: T, frequency: Frequency) -> Self {
        Self::Ois {
            maturity,
            rate,
            payment_frequency: frequency,
        }
    }

    /// Create an IRS instrument with typical conventions (annual fixed, quarterly float).
    pub fn irs(maturity: T, rate: T) -> Self {
        Self::Irs {
            maturity,
            rate,
            fixed_frequency: Frequency::Annual,
            float_frequency: Frequency::Quarterly,
        }
    }

    /// Create an IRS instrument with custom frequencies.
    pub fn irs_with_frequencies(
        maturity: T,
        rate: T,
        fixed_frequency: Frequency,
        float_frequency: Frequency,
    ) -> Self {
        Self::Irs {
            maturity,
            rate,
            fixed_frequency,
            float_frequency,
        }
    }

    /// Create a FRA instrument.
    pub fn fra(start: T, end: T, rate: T) -> Self {
        Self::Fra { start, end, rate }
    }

    /// Create a Future instrument from quoted price.
    ///
    /// The rate is derived as (100 - price) / 100.
    pub fn future(maturity: T, price: T, convexity_adjustment: T) -> Self {
        Self::Future {
            maturity,
            price,
            convexity_adjustment,
        }
    }

    /// Create a Future instrument from rate (converts to price internally).
    pub fn future_from_rate(maturity: T, rate: T, convexity_adjustment: T) -> Self {
        let price = T::from(100.0).unwrap() - rate * T::from(100.0).unwrap();
        Self::Future {
            maturity,
            price,
            convexity_adjustment,
        }
    }

    // ========================================
    // Common Accessors
    // ========================================

    /// Get the maturity (in years) of this instrument.
    ///
    /// For FRA, returns the end date.
    pub fn maturity(&self) -> T {
        match self {
            Self::Ois { maturity, .. } => *maturity,
            Self::Irs { maturity, .. } => *maturity,
            Self::Fra { end, .. } => *end,
            Self::Future { maturity, .. } => *maturity,
        }
    }

    /// Get the rate of this instrument.
    ///
    /// For Futures, returns the implied rate from price minus convexity adjustment.
    pub fn rate(&self) -> T {
        match self {
            Self::Ois { rate, .. } => *rate,
            Self::Irs { rate, .. } => *rate,
            Self::Fra { rate, .. } => *rate,
            Self::Future {
                price,
                convexity_adjustment,
                ..
            } => {
                let hundred = T::from(100.0).unwrap();
                (hundred - *price) / hundred - *convexity_adjustment
            }
        }
    }

    /// Get the instrument type as a string.
    pub fn instrument_type(&self) -> &'static str {
        match self {
            Self::Ois { .. } => "OIS",
            Self::Irs { .. } => "IRS",
            Self::Fra { .. } => "FRA",
            Self::Future { .. } => "Future",
        }
    }

    /// Check if this is an OIS instrument.
    pub fn is_ois(&self) -> bool {
        matches!(self, Self::Ois { .. })
    }

    /// Check if this is an IRS instrument.
    pub fn is_irs(&self) -> bool {
        matches!(self, Self::Irs { .. })
    }

    /// Check if this is a FRA instrument.
    pub fn is_fra(&self) -> bool {
        matches!(self, Self::Fra { .. })
    }

    /// Check if this is a Future instrument.
    pub fn is_future(&self) -> bool {
        matches!(self, Self::Future { .. })
    }

    /// Get the start time for FRA, or 0 for other instruments.
    pub fn start(&self) -> T {
        match self {
            Self::Fra { start, .. } => *start,
            _ => T::zero(),
        }
    }

    // ========================================
    // Residual Functions (for bootstrapping)
    // ========================================

    /// Compute the residual function for root-finding.
    ///
    /// The residual is the difference between the implied rate (from the
    /// discount factor at maturity) and the market rate. The solver finds
    /// the discount factor that makes this residual zero.
    ///
    /// # Arguments
    ///
    /// * `df_maturity` - Discount factor at the instrument's maturity
    /// * `partial_curve_df` - Function to get discount factor from partial curve
    ///
    /// # Returns
    ///
    /// The residual value (implied_rate - market_rate)
    pub fn residual<F>(&self, df_maturity: T, partial_curve_df: F) -> T
    where
        F: Fn(T) -> T,
    {
        let market_rate = self.rate();
        let implied_rate = self.implied_rate(df_maturity, &partial_curve_df);
        implied_rate - market_rate
    }

    /// Compute the derivative of the residual with respect to df_maturity.
    ///
    /// This is used by Newton-Raphson for efficient convergence.
    ///
    /// # Arguments
    ///
    /// * `df_maturity` - Discount factor at the instrument's maturity
    /// * `partial_curve_df` - Function to get discount factor from partial curve
    ///
    /// # Returns
    ///
    /// The derivative d(residual)/d(df_maturity)
    pub fn residual_derivative<F>(&self, df_maturity: T, partial_curve_df: F) -> T
    where
        F: Fn(T) -> T,
    {
        match self {
            Self::Ois {
                maturity,
                payment_frequency,
                ..
            } => {
                // For single-period OIS: implied_rate = (1/df - 1) / T
                // d(implied_rate)/d(df) = -1 / (df^2 * T)
                //
                // For multi-period OIS, we use simplified approach
                let num_periods = (*maturity
                    * T::from(payment_frequency.payments_per_year()).unwrap())
                .ceil()
                .to_usize()
                .unwrap_or(1)
                .max(1);

                if num_periods == 1 {
                    let neg_one = -T::one();
                    neg_one / (df_maturity * df_maturity * *maturity)
                } else {
                    // Multi-period: more complex derivative
                    self.numerical_residual_derivative(df_maturity, partial_curve_df)
                }
            }
            Self::Irs {
                maturity,
                fixed_frequency,
                ..
            } => {
                // For IRS: the residual derivative is more complex
                // Use simplified form for annual payments
                let dt = fixed_frequency.period_years::<T>();
                let num_periods = (*maturity / dt).ceil().to_usize().unwrap_or(1).max(1);

                if num_periods == 1 {
                    // Single period swap
                    let neg_one = -T::one();
                    neg_one / (df_maturity * df_maturity * dt)
                } else {
                    self.numerical_residual_derivative(df_maturity, partial_curve_df)
                }
            }
            Self::Fra { start, end, .. } => {
                // For FRA: forward_rate = (df_start / df_end - 1) / tau
                // d(forward_rate)/d(df_end) = -df_start / (df_end^2 * tau)
                let df_start = partial_curve_df(*start);
                let tau = *end - *start;
                let neg_one = -T::one();
                neg_one * df_start / (df_maturity * df_maturity * tau)
            }
            Self::Future { maturity, .. } => {
                // Similar to FRA
                // Typically for short-dated futures, we use simple discounting
                let neg_one = -T::one();
                neg_one / (df_maturity * df_maturity * *maturity)
            }
        }
    }

    /// Compute the implied rate from the discount factor.
    ///
    /// This is the rate implied by the given discount factor at maturity.
    fn implied_rate<F>(&self, df_maturity: T, partial_curve_df: &F) -> T
    where
        F: Fn(T) -> T,
    {
        match self {
            Self::Ois {
                maturity,
                payment_frequency,
                ..
            } => {
                // For OIS, we compute the par swap rate
                let dt = payment_frequency.period_years::<T>();
                let num_periods = (*maturity / dt).ceil().to_usize().unwrap_or(1).max(1);

                if num_periods == 1 {
                    // Single period: rate = (1/df - 1) / T
                    (T::one() / df_maturity - T::one()) / *maturity
                } else {
                    // Multi-period OIS par rate
                    // rate = (1 - df_maturity) / sum(df_i * dt_i)
                    let mut annuity = T::zero();
                    for i in 1..=num_periods {
                        let t_i = dt * T::from(i).unwrap();
                        if t_i < *maturity {
                            annuity = annuity + partial_curve_df(t_i) * dt;
                        }
                    }
                    // Add final period
                    let final_dt = *maturity - dt * T::from(num_periods - 1).unwrap();
                    annuity = annuity + df_maturity * final_dt;

                    if annuity > T::zero() {
                        (T::one() - df_maturity) / annuity
                    } else {
                        T::zero()
                    }
                }
            }
            Self::Irs {
                maturity,
                fixed_frequency,
                ..
            } => {
                // For IRS, compute par swap rate
                // rate = (1 - df_maturity) / annuity
                let dt = fixed_frequency.period_years::<T>();
                let num_periods = (*maturity / dt).ceil().to_usize().unwrap_or(1).max(1);

                let mut annuity = T::zero();
                for i in 1..num_periods {
                    let t_i = dt * T::from(i).unwrap();
                    if t_i < *maturity {
                        annuity = annuity + partial_curve_df(t_i) * dt;
                    }
                }
                // Final period (may be shorter)
                let final_dt = if num_periods > 1 {
                    *maturity - dt * T::from(num_periods - 1).unwrap()
                } else {
                    *maturity
                };
                annuity = annuity + df_maturity * final_dt;

                if annuity > T::zero() {
                    (T::one() - df_maturity) / annuity
                } else {
                    T::zero()
                }
            }
            Self::Fra { start, end, .. } => {
                // Forward rate from discount factors
                // forward_rate = (df_start / df_end - 1) / tau
                let df_start = partial_curve_df(*start);
                let tau = *end - *start;
                (df_start / df_maturity - T::one()) / tau
            }
            Self::Future {
                maturity,
                convexity_adjustment,
                ..
            } => {
                // For futures, compute implied rate similar to FRA
                // but with convexity adjustment already factored in
                // Simple discounting: rate = (1/df - 1) / T + convexity_adj
                (T::one() / df_maturity - T::one()) / *maturity + *convexity_adjustment
            }
        }
    }

    /// Compute numerical derivative using finite difference.
    fn numerical_residual_derivative<F>(&self, df_maturity: T, partial_curve_df: F) -> T
    where
        F: Fn(T) -> T,
    {
        let epsilon = T::from(1e-8).unwrap();
        let df_plus = df_maturity + epsilon;
        let df_minus = df_maturity - epsilon;

        let r_plus = self.residual(df_plus, &partial_curve_df);
        let r_minus = self.residual(df_minus, &partial_curve_df);

        (r_plus - r_minus) / (T::from(2.0).unwrap() * epsilon)
    }

    // ========================================
    // Validation Methods
    // ========================================

    /// Validate instrument parameters.
    ///
    /// Returns `Ok(())` if valid, or a descriptive error message.
    pub fn validate(&self, max_maturity: T) -> Result<(), String> {
        // Check maturity
        let mat = self.maturity();
        if mat <= T::zero() {
            return Err(format!("Maturity must be positive, got {:?}", mat.to_f64()));
        }
        if mat > max_maturity {
            return Err(format!(
                "Maturity {:?} exceeds maximum {:?}",
                mat.to_f64(),
                max_maturity.to_f64()
            ));
        }

        // Specific checks per instrument type
        match self {
            Self::Fra { start, end, .. } => {
                if *start >= *end {
                    return Err(format!(
                        "FRA start {:?} must be before end {:?}",
                        start.to_f64(),
                        end.to_f64()
                    ));
                }
                if *start < T::zero() {
                    return Err(format!(
                        "FRA start must be non-negative, got {:?}",
                        start.to_f64()
                    ));
                }
            }
            Self::Future { price, .. } => {
                if *price <= T::zero() || *price >= T::from(200.0).unwrap() {
                    return Err(format!(
                        "Future price {:?} is unreasonable (expected 0-200)",
                        price.to_f64()
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Frequency Tests
    // ========================================

    #[test]
    fn test_frequency_payments_per_year() {
        assert_eq!(Frequency::Annual.payments_per_year(), 1);
        assert_eq!(Frequency::SemiAnnual.payments_per_year(), 2);
        assert_eq!(Frequency::Quarterly.payments_per_year(), 4);
        assert_eq!(Frequency::Monthly.payments_per_year(), 12);
        assert_eq!(Frequency::Daily.payments_per_year(), 365);
    }

    #[test]
    fn test_frequency_period_years() {
        assert!((Frequency::Annual.period_years::<f64>() - 1.0).abs() < 1e-10);
        assert!((Frequency::SemiAnnual.period_years::<f64>() - 0.5).abs() < 1e-10);
        assert!((Frequency::Quarterly.period_years::<f64>() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_frequency_default() {
        let freq: Frequency = Default::default();
        assert_eq!(freq, Frequency::Annual);
    }

    // ========================================
    // OIS Tests
    // ========================================

    #[test]
    fn test_ois_creation() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
        assert!(ois.is_ois());
        assert!((ois.maturity() - 5.0).abs() < 1e-10);
        assert!((ois.rate() - 0.03).abs() < 1e-10);
        assert_eq!(ois.instrument_type(), "OIS");
    }

    #[test]
    fn test_ois_with_frequency() {
        let ois: BootstrapInstrument<f64> =
            BootstrapInstrument::ois_with_frequency(5.0, 0.03, Frequency::SemiAnnual);
        if let BootstrapInstrument::Ois {
            payment_frequency, ..
        } = ois
        {
            assert_eq!(payment_frequency, Frequency::SemiAnnual);
        } else {
            panic!("Expected OIS variant");
        }
    }

    // ========================================
    // IRS Tests
    // ========================================

    #[test]
    fn test_irs_creation() {
        let irs: BootstrapInstrument<f64> = BootstrapInstrument::irs(10.0, 0.035);
        assert!(irs.is_irs());
        assert!((irs.maturity() - 10.0).abs() < 1e-10);
        assert!((irs.rate() - 0.035).abs() < 1e-10);
        assert_eq!(irs.instrument_type(), "IRS");
    }

    #[test]
    fn test_irs_with_frequencies() {
        let irs: BootstrapInstrument<f64> = BootstrapInstrument::irs_with_frequencies(
            10.0,
            0.035,
            Frequency::SemiAnnual,
            Frequency::Monthly,
        );
        if let BootstrapInstrument::Irs {
            fixed_frequency,
            float_frequency,
            ..
        } = irs
        {
            assert_eq!(fixed_frequency, Frequency::SemiAnnual);
            assert_eq!(float_frequency, Frequency::Monthly);
        } else {
            panic!("Expected IRS variant");
        }
    }

    // ========================================
    // FRA Tests
    // ========================================

    #[test]
    fn test_fra_creation() {
        let fra: BootstrapInstrument<f64> = BootstrapInstrument::fra(0.25, 0.5, 0.025);
        assert!(fra.is_fra());
        assert!((fra.maturity() - 0.5).abs() < 1e-10); // maturity is end
        assert!((fra.rate() - 0.025).abs() < 1e-10);
        assert!((fra.start() - 0.25).abs() < 1e-10);
        assert_eq!(fra.instrument_type(), "FRA");
    }

    #[test]
    fn test_fra_start() {
        let fra: BootstrapInstrument<f64> = BootstrapInstrument::fra(0.5, 0.75, 0.03);
        assert!((fra.start() - 0.5).abs() < 1e-10);
    }

    // ========================================
    // Future Tests
    // ========================================

    #[test]
    fn test_future_creation() {
        let future: BootstrapInstrument<f64> = BootstrapInstrument::future(0.25, 97.5, 0.0001);
        assert!(future.is_future());
        assert!((future.maturity() - 0.25).abs() < 1e-10);
        // rate = (100 - 97.5) / 100 - 0.0001 = 0.025 - 0.0001 = 0.0249
        assert!((future.rate() - 0.0249).abs() < 1e-10);
        assert_eq!(future.instrument_type(), "Future");
    }

    #[test]
    fn test_future_from_rate() {
        let future: BootstrapInstrument<f64> =
            BootstrapInstrument::future_from_rate(0.25, 0.025, 0.0001);
        // price = 100 - 0.025 * 100 = 97.5
        // rate = (100 - 97.5) / 100 - 0.0001 = 0.0249
        assert!((future.rate() - 0.0249).abs() < 1e-10);
    }

    // ========================================
    // Common Method Tests
    // ========================================

    #[test]
    fn test_start_for_non_fra() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
        assert!((ois.start() - 0.0).abs() < 1e-10);

        let irs: BootstrapInstrument<f64> = BootstrapInstrument::irs(10.0, 0.035);
        assert!((irs.start() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_is_type_checks() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
        assert!(ois.is_ois());
        assert!(!ois.is_irs());
        assert!(!ois.is_fra());
        assert!(!ois.is_future());

        let irs: BootstrapInstrument<f64> = BootstrapInstrument::irs(10.0, 0.035);
        assert!(!irs.is_ois());
        assert!(irs.is_irs());

        let fra: BootstrapInstrument<f64> = BootstrapInstrument::fra(0.25, 0.5, 0.025);
        assert!(fra.is_fra());

        let future: BootstrapInstrument<f64> = BootstrapInstrument::future(0.25, 97.5, 0.0001);
        assert!(future.is_future());
    }

    // ========================================
    // Clone and Equality Tests
    // ========================================

    #[test]
    fn test_clone_ois() {
        let ois1: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
        let ois2 = ois1.clone();
        assert_eq!(ois1, ois2);
    }

    #[test]
    fn test_clone_irs() {
        let irs1: BootstrapInstrument<f64> = BootstrapInstrument::irs(10.0, 0.035);
        let irs2 = irs1.clone();
        assert_eq!(irs1, irs2);
    }

    #[test]
    fn test_clone_fra() {
        let fra1: BootstrapInstrument<f64> = BootstrapInstrument::fra(0.25, 0.5, 0.025);
        let fra2 = fra1.clone();
        assert_eq!(fra1, fra2);
    }

    #[test]
    fn test_clone_future() {
        let future1: BootstrapInstrument<f64> = BootstrapInstrument::future(0.25, 97.5, 0.0001);
        let future2 = future1.clone();
        assert_eq!(future1, future2);
    }

    // ========================================
    // Debug Tests
    // ========================================

    #[test]
    fn test_debug_ois() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
        let debug_str = format!("{:?}", ois);
        assert!(debug_str.contains("Ois"));
        assert!(debug_str.contains("5"));
    }

    // ========================================
    // f32 Tests
    // ========================================

    #[test]
    fn test_with_f32() {
        let ois: BootstrapInstrument<f32> = BootstrapInstrument::ois(5.0_f32, 0.03_f32);
        assert!((ois.maturity() - 5.0_f32).abs() < 1e-5);
        assert!((ois.rate() - 0.03_f32).abs() < 1e-5);
    }

    // ========================================
    // Residual Function Tests
    // ========================================

    #[test]
    fn test_ois_residual_zero_at_correct_df() {
        // For a 1-year OIS at 3%, the correct DF is approximately 1 / (1 + 0.03) ≈ 0.9709
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(1.0, 0.03);
        let df_correct = 1.0 / (1.0 + 0.03 * 1.0);

        // Partial curve returns 1.0 at t=0 (not used for single-period)
        let partial_df = |_t: f64| 1.0;

        let residual = ois.residual(df_correct, partial_df);
        assert!(
            residual.abs() < 1e-10,
            "Expected residual near zero, got {}",
            residual
        );
    }

    #[test]
    fn test_ois_residual_nonzero_at_wrong_df() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(1.0, 0.03);
        let partial_df = |_t: f64| 1.0;

        // Using wrong DF should give non-zero residual
        let residual = ois.residual(0.95, partial_df);
        assert!(
            residual.abs() > 0.01,
            "Expected non-zero residual at wrong DF"
        );
    }

    #[test]
    fn test_fra_residual_zero_at_correct_df() {
        // FRA from 0.25 to 0.5 at 2.5%
        // Forward rate = (df_start / df_end - 1) / tau
        // If df_0.25 = 0.9925 and rate = 2.5%, then:
        // df_0.5 = df_0.25 / (1 + rate * tau) = 0.9925 / (1 + 0.025 * 0.25) ≈ 0.9863
        let fra: BootstrapInstrument<f64> = BootstrapInstrument::fra(0.25, 0.5, 0.025);
        let df_start = 0.99; // Assume we know df at 0.25
        let tau = 0.25;
        let df_end = df_start / (1.0 + 0.025 * tau);

        let partial_df = |t: f64| {
            if t <= 0.25 + 1e-10 {
                df_start
            } else {
                df_end
            }
        };

        let residual = fra.residual(df_end, partial_df);
        assert!(
            residual.abs() < 1e-10,
            "Expected residual near zero, got {}",
            residual
        );
    }

    #[test]
    fn test_residual_derivative_negative() {
        // For most instruments, the derivative should be negative
        // (higher DF means lower implied rate)
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(1.0, 0.03);
        let partial_df = |_t: f64| 1.0;

        let deriv = ois.residual_derivative(0.97, partial_df);
        assert!(deriv < 0.0, "Expected negative derivative, got {}", deriv);
    }

    #[test]
    fn test_residual_derivative_fra() {
        let fra: BootstrapInstrument<f64> = BootstrapInstrument::fra(0.25, 0.5, 0.025);
        let df_start = 0.99;
        let partial_df = |t: f64| if t <= 0.25 { df_start } else { 0.98 };

        let deriv = fra.residual_derivative(0.98, partial_df);
        assert!(deriv < 0.0, "Expected negative derivative for FRA");
    }

    // ========================================
    // Validation Tests
    // ========================================

    #[test]
    fn test_validate_valid_ois() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(5.0, 0.03);
        assert!(ois.validate(50.0).is_ok());
    }

    #[test]
    fn test_validate_maturity_too_large() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(60.0, 0.03);
        let result = ois.validate(50.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds maximum"));
    }

    #[test]
    fn test_validate_negative_maturity() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(-1.0, 0.03);
        let result = ois.validate(50.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("positive"));
    }

    #[test]
    fn test_validate_fra_start_after_end() {
        let fra: BootstrapInstrument<f64> = BootstrapInstrument::fra(0.5, 0.25, 0.03);
        let result = fra.validate(50.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("before end"));
    }

    #[test]
    fn test_validate_fra_negative_start() {
        let fra: BootstrapInstrument<f64> = BootstrapInstrument::fra(-0.1, 0.25, 0.03);
        let result = fra.validate(50.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-negative"));
    }

    #[test]
    fn test_validate_future_unreasonable_price() {
        let future: BootstrapInstrument<f64> = BootstrapInstrument::future(0.25, 250.0, 0.0001);
        let result = future.validate(50.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unreasonable"));
    }

    #[test]
    fn test_validate_valid_future() {
        let future: BootstrapInstrument<f64> = BootstrapInstrument::future(0.25, 97.5, 0.0001);
        assert!(future.validate(50.0).is_ok());
    }
}
