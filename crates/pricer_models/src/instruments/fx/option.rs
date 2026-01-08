//! FX Option instrument definitions.
//!
//! This module provides FX option structures for foreign exchange
//! derivative pricing.
//!
//! # Examples
//!
//! ```
//! use pricer_models::instruments::fx::{FxOption, FxOptionType};
//! use pricer_core::types::{Currency, CurrencyPair};
//!
//! // Create a EUR/USD call option
//! let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
//! let option = FxOption::new(
//!     pair,
//!     1.12,              // strike
//!     1.0,               // expiry (1 year)
//!     1_000_000.0,       // notional (1M EUR)
//!     FxOptionType::Call,
//!     1e-6,              // smoothing epsilon
//! ).unwrap();
//!
//! assert_eq!(option.strike(), 1.12);
//! ```

use num_traits::Float;
use pricer_core::types::{Currency, CurrencyPair, CurrencyError};

use crate::instruments::traits::InstrumentTrait;

/// FX option type (Call or Put).
///
/// - Call: Right to buy base currency, sell quote currency
/// - Put: Right to sell base currency, buy quote currency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxOptionType {
    /// Right to buy base currency at strike price in quote currency.
    Call,
    /// Right to sell base currency at strike price in quote currency.
    Put,
}

impl FxOptionType {
    /// Returns whether this is a call option.
    #[inline]
    pub fn is_call(&self) -> bool {
        matches!(self, FxOptionType::Call)
    }

    /// Returns whether this is a put option.
    #[inline]
    pub fn is_put(&self) -> bool {
        matches!(self, FxOptionType::Put)
    }
}

impl std::fmt::Display for FxOptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FxOptionType::Call => write!(f, "Call"),
            FxOptionType::Put => write!(f, "Put"),
        }
    }
}

/// Error types for FX option operations.
#[derive(Debug, Clone, PartialEq)]
pub enum FxOptionError {
    /// Invalid strike price.
    InvalidStrike,
    /// Invalid expiry.
    InvalidExpiry,
    /// Invalid notional.
    InvalidNotional,
    /// Currency error.
    Currency(CurrencyError),
}

impl std::fmt::Display for FxOptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FxOptionError::InvalidStrike => write!(f, "Strike must be positive"),
            FxOptionError::InvalidExpiry => write!(f, "Expiry must be positive"),
            FxOptionError::InvalidNotional => write!(f, "Notional must be positive"),
            FxOptionError::Currency(e) => write!(f, "Currency error: {}", e),
        }
    }
}

impl std::error::Error for FxOptionError {}

impl From<CurrencyError> for FxOptionError {
    fn from(e: CurrencyError) -> Self {
        FxOptionError::Currency(e)
    }
}

/// FX option instrument.
///
/// Represents a vanilla FX option on a currency pair.
/// The option gives the holder the right (but not obligation) to
/// buy (call) or sell (put) the base currency at the strike price.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Conventions
///
/// - Notional is in base currency
/// - Strike is quoted as units of quote currency per 1 unit of base currency
/// - Settlement is in quote currency
///
/// # Examples
///
/// ```
/// use pricer_models::instruments::fx::{FxOption, FxOptionType};
/// use pricer_core::types::{Currency, CurrencyPair};
///
/// // EUR/USD call at 1.12, notional 1M EUR
/// let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10_f64).unwrap();
/// let call = FxOption::new(
///     pair,
///     1.12,
///     0.5,
///     1_000_000.0,
///     FxOptionType::Call,
///     1e-6,
/// ).unwrap();
///
/// // At spot 1.15, call payoff = 1M * max(1.15 - 1.12, 0) = 30,000 USD
/// let payoff = call.payoff(1.15);
/// assert!((payoff - 30_000.0_f64).abs() < 100.0);
/// ```
#[derive(Debug, Clone)]
pub struct FxOption<T: Float> {
    /// Currency pair (BASE/QUOTE).
    currency_pair: CurrencyPair<T>,
    /// Strike price (quote per base).
    strike: T,
    /// Time to expiry in years.
    expiry: T,
    /// Notional amount in base currency.
    notional: T,
    /// Option type (Call/Put).
    option_type: FxOptionType,
    /// Smoothing epsilon for AD-compatible payoff.
    epsilon: T,
}

impl<T: Float> FxOption<T> {
    /// Creates a new FX option.
    ///
    /// # Arguments
    ///
    /// * `currency_pair` - The underlying currency pair
    /// * `strike` - Strike price (must be positive)
    /// * `expiry` - Time to expiry in years (must be positive)
    /// * `notional` - Notional amount in base currency (must be positive)
    /// * `option_type` - Call or Put
    /// * `epsilon` - Smoothing parameter for AD compatibility
    ///
    /// # Errors
    ///
    /// Returns `FxOptionError` if strike, expiry, or notional is not positive.
    pub fn new(
        currency_pair: CurrencyPair<T>,
        strike: T,
        expiry: T,
        notional: T,
        option_type: FxOptionType,
        epsilon: T,
    ) -> Result<Self, FxOptionError> {
        if strike <= T::zero() {
            return Err(FxOptionError::InvalidStrike);
        }
        if expiry <= T::zero() {
            return Err(FxOptionError::InvalidExpiry);
        }
        if notional <= T::zero() {
            return Err(FxOptionError::InvalidNotional);
        }

        Ok(Self {
            currency_pair,
            strike,
            expiry,
            notional,
            option_type,
            epsilon,
        })
    }

    /// Returns the currency pair.
    #[inline]
    pub fn currency_pair(&self) -> &CurrencyPair<T> {
        &self.currency_pair
    }

    /// Returns the base currency.
    #[inline]
    pub fn base_currency(&self) -> Currency {
        self.currency_pair.base()
    }

    /// Returns the quote currency (settlement currency).
    #[inline]
    pub fn quote_currency(&self) -> Currency {
        self.currency_pair.quote()
    }

    /// Returns the strike price.
    #[inline]
    pub fn strike(&self) -> T {
        self.strike
    }

    /// Returns the time to expiry in years.
    #[inline]
    pub fn expiry_time(&self) -> T {
        self.expiry
    }

    /// Returns the notional amount in base currency.
    #[inline]
    pub fn notional_amount(&self) -> T {
        self.notional
    }

    /// Returns the option type.
    #[inline]
    pub fn option_type(&self) -> FxOptionType {
        self.option_type
    }

    /// Returns the smoothing epsilon.
    #[inline]
    pub fn epsilon(&self) -> T {
        self.epsilon
    }

    /// Returns whether this is a call option.
    #[inline]
    pub fn is_call(&self) -> bool {
        self.option_type.is_call()
    }

    /// Returns whether this is a put option.
    #[inline]
    pub fn is_put(&self) -> bool {
        self.option_type.is_put()
    }

    /// Calculates the payoff at expiry for a given spot rate.
    ///
    /// The payoff is calculated in quote currency:
    /// - Call: notional * max(spot - strike, 0)
    /// - Put: notional * max(strike - spot, 0)
    ///
    /// Uses smooth_max for AD compatibility.
    ///
    /// # Arguments
    ///
    /// * `spot` - Spot exchange rate at expiry
    ///
    /// # Returns
    ///
    /// Payoff in quote currency.
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        let intrinsic = match self.option_type {
            FxOptionType::Call => spot - self.strike,
            FxOptionType::Put => self.strike - spot,
        };

        // Use smooth_max for AD compatibility
        let payoff_per_unit = smooth_max(intrinsic, T::zero(), self.epsilon);
        self.notional * payoff_per_unit
    }

    /// Returns the currency pair code (e.g., "EUR/USD").
    pub fn pair_code(&self) -> String {
        self.currency_pair.code()
    }
}

/// Smooth approximation of max(x, y) for AD compatibility.
///
/// Uses the log-sum-exp trick with numerical stability.
#[inline]
fn smooth_max<T: Float>(x: T, y: T, epsilon: T) -> T {
    if epsilon <= T::zero() {
        if x > y { x } else { y }
    } else {
        // Numerically stable log-sum-exp: max(x,y) + eps * ln(1 + exp(-(|x-y|)/eps))
        let max_val = if x > y { x } else { y };
        let min_val = if x > y { y } else { x };
        let diff = (max_val - min_val) / epsilon;

        // For large diff, the correction term is negligible
        let threshold = T::from(30.0).unwrap();
        if diff > threshold {
            max_val
        } else {
            max_val + epsilon * (T::one() + (-diff).exp()).ln()
        }
    }
}

impl<T: Float> InstrumentTrait<T> for FxOption<T> {
    #[inline]
    fn payoff(&self, spot: T) -> T {
        self.payoff(spot)
    }

    #[inline]
    fn expiry(&self) -> T {
        self.expiry
    }

    #[inline]
    fn currency(&self) -> Currency {
        // Settlement is in quote currency
        self.quote_currency()
    }

    #[inline]
    fn notional(&self) -> T {
        self.notional
    }

    fn type_name(&self) -> &'static str {
        match self.option_type {
            FxOptionType::Call => "FxCall",
            FxOptionType::Put => "FxPut",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pair() -> CurrencyPair<f64> {
        CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap()
    }

    #[test]
    fn test_fx_option_new_call() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.12,
            1.0,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();

        assert_eq!(option.base_currency(), Currency::EUR);
        assert_eq!(option.quote_currency(), Currency::USD);
        assert!((option.strike() - 1.12).abs() < 1e-10);
        assert!((option.expiry_time() - 1.0).abs() < 1e-10);
        assert!((option.notional_amount() - 1_000_000.0).abs() < 1e-10);
        assert!(option.is_call());
        assert!(!option.is_put());
    }

    #[test]
    fn test_fx_option_new_put() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.08,
            0.5,
            500_000.0,
            FxOptionType::Put,
            1e-6,
        ).unwrap();

        assert!(option.is_put());
        assert!(!option.is_call());
    }

    #[test]
    fn test_fx_option_invalid_strike() {
        let pair = create_test_pair();
        let result = FxOption::new(pair, 0.0, 1.0, 1_000_000.0, FxOptionType::Call, 1e-6);
        assert!(matches!(result, Err(FxOptionError::InvalidStrike)));

        let pair = create_test_pair();
        let result = FxOption::new(pair, -1.0, 1.0, 1_000_000.0, FxOptionType::Call, 1e-6);
        assert!(matches!(result, Err(FxOptionError::InvalidStrike)));
    }

    #[test]
    fn test_fx_option_invalid_expiry() {
        let pair = create_test_pair();
        let result = FxOption::new(pair, 1.12, 0.0, 1_000_000.0, FxOptionType::Call, 1e-6);
        assert!(matches!(result, Err(FxOptionError::InvalidExpiry)));
    }

    #[test]
    fn test_fx_option_invalid_notional() {
        let pair = create_test_pair();
        let result = FxOption::new(pair, 1.12, 1.0, 0.0, FxOptionType::Call, 1e-6);
        assert!(matches!(result, Err(FxOptionError::InvalidNotional)));
    }

    #[test]
    fn test_fx_call_payoff_itm() {
        let pair = create_test_pair();
        let call = FxOption::new(
            pair,
            1.10,              // strike
            1.0,
            1_000_000.0,       // notional
            FxOptionType::Call,
            1e-6,
        ).unwrap();

        // Spot at 1.15, payoff = 1M * (1.15 - 1.10) = 50,000 USD
        let payoff = call.payoff(1.15);
        assert!((payoff - 50_000.0).abs() < 100.0);
    }

    #[test]
    fn test_fx_call_payoff_otm() {
        let pair = create_test_pair();
        let call = FxOption::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();

        // Spot at 1.05, call is OTM, payoff ≈ 0
        let payoff = call.payoff(1.05);
        assert!(payoff < 100.0);
        assert!(payoff >= 0.0);
    }

    #[test]
    fn test_fx_put_payoff_itm() {
        let pair = create_test_pair();
        let put = FxOption::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxOptionType::Put,
            1e-6,
        ).unwrap();

        // Spot at 1.05, payoff = 1M * (1.10 - 1.05) = 50,000 USD
        let payoff = put.payoff(1.05);
        assert!((payoff - 50_000.0).abs() < 100.0);
    }

    #[test]
    fn test_fx_put_payoff_otm() {
        let pair = create_test_pair();
        let put = FxOption::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxOptionType::Put,
            1e-6,
        ).unwrap();

        // Spot at 1.15, put is OTM, payoff ≈ 0
        let payoff = put.payoff(1.15);
        assert!(payoff < 100.0);
        assert!(payoff >= 0.0);
    }

    #[test]
    fn test_fx_option_pair_code() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.12,
            1.0,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();

        assert_eq!(option.pair_code(), "EUR/USD");
    }

    #[test]
    fn test_fx_option_instrument_trait() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.12,
            0.5,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();

        // Test InstrumentTrait methods
        assert!((option.expiry() - 0.5).abs() < 1e-10);
        assert_eq!(option.currency(), Currency::USD); // Settlement currency
        assert!((option.notional() - 1_000_000.0).abs() < 1e-10);
        assert_eq!(option.type_name(), "FxCall");
    }

    #[test]
    fn test_fx_option_type_display() {
        assert_eq!(format!("{}", FxOptionType::Call), "Call");
        assert_eq!(format!("{}", FxOptionType::Put), "Put");
    }

    #[test]
    fn test_fx_option_clone() {
        let pair = create_test_pair();
        let option1 = FxOption::new(
            pair,
            1.12,
            1.0,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();
        let option2 = option1.clone();

        assert!((option1.strike() - option2.strike()).abs() < 1e-10);
        assert_eq!(option1.option_type(), option2.option_type());
    }

    #[test]
    fn test_fx_option_debug() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.12,
            1.0,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();

        let debug_str = format!("{:?}", option);
        assert!(debug_str.contains("FxOption"));
        assert!(debug_str.contains("Call"));
    }

    #[test]
    fn test_usdjpy_option() {
        let pair = CurrencyPair::new(Currency::USD, Currency::JPY, 150.0).unwrap();
        let call = FxOption::new(
            pair,
            152.0,              // strike
            0.25,               // 3 months
            100_000.0,          // 100k USD notional
            FxOptionType::Call,
            1e-6,
        ).unwrap();

        assert_eq!(call.base_currency(), Currency::USD);
        assert_eq!(call.quote_currency(), Currency::JPY);
        assert_eq!(call.pair_code(), "USD/JPY");

        // At spot 155, payoff = 100k * (155 - 152) = 300,000 JPY
        let payoff = call.payoff(155.0);
        assert!((payoff - 300_000.0).abs() < 1000.0);
    }
}
