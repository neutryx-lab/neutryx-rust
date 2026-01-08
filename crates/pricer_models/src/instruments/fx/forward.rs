//! FX Forward instrument definitions.
//!
//! This module provides FX forward structures for foreign exchange
//! derivative pricing.
//!
//! # Examples
//!
//! ```
//! use pricer_models::instruments::fx::{FxForward, FxForwardDirection};
//! use pricer_core::types::{Currency, CurrencyPair};
//!
//! // Create a EUR/USD forward - buy EUR, sell USD
//! let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
//! let forward = FxForward::new(
//!     pair,
//!     1.12,              // forward rate
//!     1.0,               // maturity (1 year)
//!     1_000_000.0,       // notional (1M EUR)
//!     FxForwardDirection::Buy,
//! ).unwrap();
//!
//! assert_eq!(forward.forward_rate(), 1.12);
//! ```

use num_traits::Float;
use pricer_core::types::{Currency, CurrencyPair, CurrencyError};

use crate::instruments::traits::InstrumentTrait;

/// FX forward direction.
///
/// - Buy: Buy base currency, sell quote currency (long position)
/// - Sell: Sell base currency, buy quote currency (short position)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxForwardDirection {
    /// Buy base currency at forward rate (long position).
    Buy,
    /// Sell base currency at forward rate (short position).
    Sell,
}

impl FxForwardDirection {
    /// Returns the sign multiplier for payoff calculation.
    #[inline]
    pub fn sign<T: Float>(&self) -> T {
        match self {
            FxForwardDirection::Buy => T::one(),
            FxForwardDirection::Sell => -T::one(),
        }
    }

    /// Returns whether this is a buy (long) position.
    #[inline]
    pub fn is_buy(&self) -> bool {
        matches!(self, FxForwardDirection::Buy)
    }

    /// Returns whether this is a sell (short) position.
    #[inline]
    pub fn is_sell(&self) -> bool {
        matches!(self, FxForwardDirection::Sell)
    }
}

impl std::fmt::Display for FxForwardDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FxForwardDirection::Buy => write!(f, "Buy"),
            FxForwardDirection::Sell => write!(f, "Sell"),
        }
    }
}

/// Error types for FX forward operations.
#[derive(Debug, Clone, PartialEq)]
pub enum FxForwardError {
    /// Invalid forward rate.
    InvalidForwardRate,
    /// Invalid maturity.
    InvalidMaturity,
    /// Invalid notional.
    InvalidNotional,
    /// Currency error.
    Currency(CurrencyError),
}

impl std::fmt::Display for FxForwardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FxForwardError::InvalidForwardRate => write!(f, "Forward rate must be positive"),
            FxForwardError::InvalidMaturity => write!(f, "Maturity must be positive"),
            FxForwardError::InvalidNotional => write!(f, "Notional must be positive"),
            FxForwardError::Currency(e) => write!(f, "Currency error: {}", e),
        }
    }
}

impl std::error::Error for FxForwardError {}

impl From<CurrencyError> for FxForwardError {
    fn from(e: CurrencyError) -> Self {
        FxForwardError::Currency(e)
    }
}

/// FX forward instrument.
///
/// Represents an FX forward contract to exchange currencies at a
/// predetermined rate on a future date.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Conventions
///
/// - Notional is in base currency
/// - Forward rate is quoted as units of quote currency per 1 unit of base currency
/// - Settlement is in quote currency
///
/// # Examples
///
/// ```
/// use pricer_models::instruments::fx::{FxForward, FxForwardDirection};
/// use pricer_core::types::{Currency, CurrencyPair};
///
/// // EUR/USD forward to buy 1M EUR at 1.12
/// let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10_f64).unwrap();
/// let forward = FxForward::new(
///     pair,
///     1.12,           // forward rate
///     1.0,            // 1 year maturity
///     1_000_000.0,    // notional
///     FxForwardDirection::Buy,
/// ).unwrap();
///
/// // At spot 1.15, payoff = 1M * (1.15 - 1.12) = 30,000 USD
/// let payoff = forward.payoff(1.15);
/// assert!((payoff - 30_000.0_f64).abs() < 1e-6);
/// ```
#[derive(Debug, Clone)]
pub struct FxForward<T: Float> {
    /// Currency pair (BASE/QUOTE).
    currency_pair: CurrencyPair<T>,
    /// Forward exchange rate (quote per base).
    forward_rate: T,
    /// Time to maturity in years.
    maturity: T,
    /// Notional amount in base currency.
    notional: T,
    /// Direction (Buy or Sell base currency).
    direction: FxForwardDirection,
}

impl<T: Float> FxForward<T> {
    /// Creates a new FX forward.
    ///
    /// # Arguments
    ///
    /// * `currency_pair` - The underlying currency pair
    /// * `forward_rate` - Forward exchange rate (must be positive)
    /// * `maturity` - Time to maturity in years (must be positive)
    /// * `notional` - Notional amount in base currency (must be positive)
    /// * `direction` - Buy or Sell direction
    ///
    /// # Errors
    ///
    /// Returns `FxForwardError` if forward_rate, maturity, or notional is not positive.
    pub fn new(
        currency_pair: CurrencyPair<T>,
        forward_rate: T,
        maturity: T,
        notional: T,
        direction: FxForwardDirection,
    ) -> Result<Self, FxForwardError> {
        if forward_rate <= T::zero() {
            return Err(FxForwardError::InvalidForwardRate);
        }
        if maturity <= T::zero() {
            return Err(FxForwardError::InvalidMaturity);
        }
        if notional <= T::zero() {
            return Err(FxForwardError::InvalidNotional);
        }

        Ok(Self {
            currency_pair,
            forward_rate,
            maturity,
            notional,
            direction,
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

    /// Returns the forward rate.
    #[inline]
    pub fn forward_rate(&self) -> T {
        self.forward_rate
    }

    /// Returns the time to maturity in years.
    #[inline]
    pub fn maturity(&self) -> T {
        self.maturity
    }

    /// Returns the notional amount in base currency.
    #[inline]
    pub fn notional_amount(&self) -> T {
        self.notional
    }

    /// Returns the direction.
    #[inline]
    pub fn direction(&self) -> FxForwardDirection {
        self.direction
    }

    /// Returns whether this is a buy (long) position.
    #[inline]
    pub fn is_buy(&self) -> bool {
        self.direction.is_buy()
    }

    /// Returns whether this is a sell (short) position.
    #[inline]
    pub fn is_sell(&self) -> bool {
        self.direction.is_sell()
    }

    /// Calculates the payoff at maturity for a given spot rate.
    ///
    /// The payoff is calculated in quote currency:
    /// - Buy: notional * (spot - forward_rate)
    /// - Sell: notional * (forward_rate - spot)
    ///
    /// # Arguments
    ///
    /// * `spot` - Spot exchange rate at maturity
    ///
    /// # Returns
    ///
    /// Payoff in quote currency.
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        let diff = spot - self.forward_rate;
        self.notional * diff * self.direction.sign()
    }

    /// Returns the currency pair code (e.g., "EUR/USD").
    pub fn pair_code(&self) -> String {
        self.currency_pair.code()
    }

    /// Calculates the mark-to-market value given current forward rate.
    ///
    /// # Arguments
    ///
    /// * `current_forward` - Current market forward rate
    /// * `discount_factor` - Discount factor to maturity
    ///
    /// # Returns
    ///
    /// Present value in quote currency.
    pub fn mark_to_market(&self, current_forward: T, discount_factor: T) -> T {
        let diff = current_forward - self.forward_rate;
        self.notional * diff * self.direction.sign() * discount_factor
    }
}

impl<T: Float> InstrumentTrait<T> for FxForward<T> {
    #[inline]
    fn payoff(&self, spot: T) -> T {
        self.payoff(spot)
    }

    #[inline]
    fn expiry(&self) -> T {
        self.maturity
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
        match self.direction {
            FxForwardDirection::Buy => "FxForwardBuy",
            FxForwardDirection::Sell => "FxForwardSell",
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
    fn test_fx_forward_new_buy() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.12,
            1.0,
            1_000_000.0,
            FxForwardDirection::Buy,
        ).unwrap();

        assert_eq!(forward.base_currency(), Currency::EUR);
        assert_eq!(forward.quote_currency(), Currency::USD);
        assert!((forward.forward_rate() - 1.12).abs() < 1e-10);
        assert!((forward.maturity() - 1.0).abs() < 1e-10);
        assert!((forward.notional_amount() - 1_000_000.0).abs() < 1e-10);
        assert!(forward.is_buy());
        assert!(!forward.is_sell());
    }

    #[test]
    fn test_fx_forward_new_sell() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.12,
            0.5,
            500_000.0,
            FxForwardDirection::Sell,
        ).unwrap();

        assert!(forward.is_sell());
        assert!(!forward.is_buy());
    }

    #[test]
    fn test_fx_forward_invalid_forward_rate() {
        let pair = create_test_pair();
        let result = FxForward::new(
            pair,
            0.0,
            1.0,
            1_000_000.0,
            FxForwardDirection::Buy,
        );
        assert!(matches!(result, Err(FxForwardError::InvalidForwardRate)));

        let pair = create_test_pair();
        let result = FxForward::new(
            pair,
            -1.0,
            1.0,
            1_000_000.0,
            FxForwardDirection::Buy,
        );
        assert!(matches!(result, Err(FxForwardError::InvalidForwardRate)));
    }

    #[test]
    fn test_fx_forward_invalid_maturity() {
        let pair = create_test_pair();
        let result = FxForward::new(
            pair,
            1.12,
            0.0,
            1_000_000.0,
            FxForwardDirection::Buy,
        );
        assert!(matches!(result, Err(FxForwardError::InvalidMaturity)));
    }

    #[test]
    fn test_fx_forward_invalid_notional() {
        let pair = create_test_pair();
        let result = FxForward::new(
            pair,
            1.12,
            1.0,
            0.0,
            FxForwardDirection::Buy,
        );
        assert!(matches!(result, Err(FxForwardError::InvalidNotional)));
    }

    #[test]
    fn test_fx_forward_buy_payoff_positive() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.10,              // forward rate
            1.0,
            1_000_000.0,       // notional
            FxForwardDirection::Buy,
        ).unwrap();

        // Spot at 1.15, payoff = 1M * (1.15 - 1.10) = 50,000 USD
        let payoff = forward.payoff(1.15);
        assert!((payoff - 50_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_fx_forward_buy_payoff_negative() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxForwardDirection::Buy,
        ).unwrap();

        // Spot at 1.05, payoff = 1M * (1.05 - 1.10) = -50,000 USD
        let payoff = forward.payoff(1.05);
        assert!((payoff - (-50_000.0)).abs() < 1e-6);
    }

    #[test]
    fn test_fx_forward_sell_payoff_positive() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxForwardDirection::Sell,
        ).unwrap();

        // Spot at 1.05, payoff = 1M * (1.10 - 1.05) = 50,000 USD
        let payoff = forward.payoff(1.05);
        assert!((payoff - 50_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_fx_forward_sell_payoff_negative() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxForwardDirection::Sell,
        ).unwrap();

        // Spot at 1.15, payoff = 1M * (1.10 - 1.15) = -50,000 USD
        let payoff = forward.payoff(1.15);
        assert!((payoff - (-50_000.0)).abs() < 1e-6);
    }

    #[test]
    fn test_fx_forward_mark_to_market() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxForwardDirection::Buy,
        ).unwrap();

        // Current forward at 1.12, discount factor 0.95
        let mtm = forward.mark_to_market(1.12, 0.95);
        // Expected: 1M * (1.12 - 1.10) * 0.95 = 19,000 USD
        assert!((mtm - 19_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_fx_forward_pair_code() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.12,
            1.0,
            1_000_000.0,
            FxForwardDirection::Buy,
        ).unwrap();

        assert_eq!(forward.pair_code(), "EUR/USD");
    }

    #[test]
    fn test_fx_forward_instrument_trait() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.12,
            0.5,
            1_000_000.0,
            FxForwardDirection::Buy,
        ).unwrap();

        // Test InstrumentTrait methods
        assert!((forward.expiry() - 0.5).abs() < 1e-10);
        assert_eq!(forward.currency(), Currency::USD); // Settlement currency
        assert!((forward.notional() - 1_000_000.0).abs() < 1e-10);
        assert_eq!(forward.type_name(), "FxForwardBuy");
    }

    #[test]
    fn test_fx_forward_direction_display() {
        assert_eq!(format!("{}", FxForwardDirection::Buy), "Buy");
        assert_eq!(format!("{}", FxForwardDirection::Sell), "Sell");
    }

    #[test]
    fn test_fx_forward_clone() {
        let pair = create_test_pair();
        let forward1 = FxForward::new(
            pair,
            1.12,
            1.0,
            1_000_000.0,
            FxForwardDirection::Buy,
        ).unwrap();
        let forward2 = forward1.clone();

        assert!((forward1.forward_rate() - forward2.forward_rate()).abs() < 1e-10);
        assert_eq!(forward1.direction(), forward2.direction());
    }

    #[test]
    fn test_fx_forward_debug() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.12,
            1.0,
            1_000_000.0,
            FxForwardDirection::Buy,
        ).unwrap();

        let debug_str = format!("{:?}", forward);
        assert!(debug_str.contains("FxForward"));
        assert!(debug_str.contains("Buy"));
    }

    #[test]
    fn test_usdjpy_forward() {
        let pair = CurrencyPair::new(Currency::USD, Currency::JPY, 150.0).unwrap();
        let forward = FxForward::new(
            pair,
            152.0,              // forward rate
            0.5,                // 6 months
            100_000.0,          // 100k USD notional
            FxForwardDirection::Buy,
        ).unwrap();

        assert_eq!(forward.base_currency(), Currency::USD);
        assert_eq!(forward.quote_currency(), Currency::JPY);
        assert_eq!(forward.pair_code(), "USD/JPY");

        // At spot 155, payoff = 100k * (155 - 152) = 300,000 JPY
        let payoff = forward.payoff(155.0);
        assert!((payoff - 300_000.0).abs() < 1e-6);
    }
}
