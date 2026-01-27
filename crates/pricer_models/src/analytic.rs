//! Analytical pricing utilities with instrument integration.
//!
//! This module provides:
//! - Re-exports of pure mathematical formulas from `pricer_core::math::formulas`
//! - Error types with `PricingError` conversion
//! - Convenience methods for pricing instruments
//!
//! ## Pure Formulas (from pricer_core)
//!
//! - [`BlackScholes`] - Black-Scholes model for lognormal dynamics
//! - [`Bachelier`] - Bachelier model for normal dynamics
//! - [`GarmanKohlhagen`] - Garman-Kohlhagen model for FX options
//! - [`sabr_implied_vol`] - SABR Hagan implied volatility
//!
//! ## Instrument Wrappers
//!
//! Extension traits and wrappers that integrate pure formulas with
//! `VanillaOption` and other instrument types.

use num_traits::Float;
use thiserror::Error;

use pricer_core::types::PricingError;

use crate::instruments::{FxOptionType, PayoffType, VanillaOption};

// Re-export pure math formulas from pricer_core
pub use pricer_core::math::formulas::{
    // Black-Scholes
    BlackScholes,
    // Bachelier
    Bachelier,
    // Garman-Kohlhagen
    fx_call_price, fx_put_price, GarmanKohlhagen, GarmanKohlhagenParams,
    // SABR
    sabr_atm_vol, sabr_implied_vol, sabr_implied_vol_with_floor,
    SabrImpliedVolError, SabrImpliedVolParams,
    // Error
    FormulaError,
};

/// Analytical pricing errors with instrument context.
///
/// Extends `FormulaError` with additional variants for instrument-specific
/// error conditions.
///
/// # Variants
/// - `FormulaError`: Wrapped error from pure formula calculation
/// - `UnsupportedExerciseStyle`: Exercise style not supported by analytical model
///
/// # Examples
/// ```
/// use pricer_models::analytic::AnalyticalError;
///
/// let err = AnalyticalError::UnsupportedExerciseStyle {
///     style: "American".to_string(),
/// };
/// assert!(format!("{}", err).contains("American"));
/// ```
#[derive(Debug, Clone, Error, PartialEq)]
pub enum AnalyticalError {
    /// Error from formula calculation.
    #[error("{0}")]
    Formula(#[from] FormulaError),

    /// Unsupported exercise style.
    #[error("Unsupported exercise style: {style}")]
    UnsupportedExerciseStyle {
        /// Description of the unsupported exercise style
        style: String,
    },
}

impl From<AnalyticalError> for PricingError {
    fn from(err: AnalyticalError) -> Self {
        match err {
            AnalyticalError::Formula(e) => match e {
                FormulaError::InvalidVolatility { .. } | FormulaError::InvalidSpot { .. } => {
                    PricingError::InvalidInput(e.to_string())
                }
                FormulaError::InvalidExpiry { .. } => {
                    PricingError::InvalidInput(e.to_string())
                }
                FormulaError::NumericalInstability { .. } => {
                    PricingError::NumericalInstability(e.to_string())
                }
            },
            AnalyticalError::UnsupportedExerciseStyle { .. } => {
                PricingError::UnsupportedInstrument(err.to_string())
            }
        }
    }
}

/// Extension trait for pricing vanilla options with Black-Scholes.
pub trait BlackScholesExt<T: Float> {
    /// Prices a VanillaOption using Black-Scholes.
    ///
    /// # Arguments
    /// * `option` - The vanilla option to price
    ///
    /// # Returns
    /// The option price scaled by notional, or an error if the
    /// exercise style is not European.
    ///
    /// # Errors
    /// - `AnalyticalError::UnsupportedExerciseStyle` if not European
    fn price_option(&self, option: &VanillaOption<T>) -> Result<T, AnalyticalError>;
}

impl<T: Float> BlackScholesExt<T> for BlackScholes<T> {
    fn price_option(&self, option: &VanillaOption<T>) -> Result<T, AnalyticalError> {
        // Verify exercise style is European
        if !option.exercise_style().is_european() {
            let style = if option.exercise_style().is_american() {
                "American".to_string()
            } else if option.exercise_style().is_bermudan() {
                "Bermudan".to_string()
            } else if option.exercise_style().is_asian() {
                "Asian".to_string()
            } else {
                "non-European".to_string()
            };
            return Err(AnalyticalError::UnsupportedExerciseStyle { style });
        }

        let strike = option.strike();
        let expiry = option.expiry();
        let notional = option.notional();

        let unit_price = match option.payoff_type() {
            PayoffType::Call => self.price_call(strike, expiry),
            PayoffType::Put => self.price_put(strike, expiry),
            PayoffType::DigitalCall | PayoffType::DigitalPut => {
                return Err(AnalyticalError::UnsupportedExerciseStyle {
                    style: "Digital options require different pricing".to_string(),
                });
            }
        };

        Ok(notional * unit_price)
    }
}

/// Extension trait for pricing vanilla options with Bachelier.
pub trait BachelierExt<T: Float> {
    /// Prices a VanillaOption using Bachelier model.
    ///
    /// # Arguments
    /// * `option` - The vanilla option to price
    ///
    /// # Returns
    /// The option price scaled by notional, or an error if the
    /// exercise style is not European.
    fn price_option(&self, option: &VanillaOption<T>) -> Result<T, AnalyticalError>;
}

impl<T: Float> BachelierExt<T> for Bachelier<T> {
    fn price_option(&self, option: &VanillaOption<T>) -> Result<T, AnalyticalError> {
        // Verify exercise style is European
        if !option.exercise_style().is_european() {
            let style = if option.exercise_style().is_american() {
                "American".to_string()
            } else if option.exercise_style().is_bermudan() {
                "Bermudan".to_string()
            } else if option.exercise_style().is_asian() {
                "Asian".to_string()
            } else {
                "non-European".to_string()
            };
            return Err(AnalyticalError::UnsupportedExerciseStyle { style });
        }

        let strike = option.strike();
        let expiry = option.expiry();
        let notional = option.notional();

        let unit_price = match option.payoff_type() {
            PayoffType::Call => self.price_call(strike, expiry),
            PayoffType::Put => self.price_put(strike, expiry),
            PayoffType::DigitalCall | PayoffType::DigitalPut => {
                return Err(AnalyticalError::UnsupportedExerciseStyle {
                    style: "Digital options require different pricing".to_string(),
                });
            }
        };

        Ok(notional * unit_price)
    }
}

/// Extension trait for Garman-Kohlhagen to work with FxOptionType.
pub trait GarmanKohlhagenExt<T: Float> {
    /// Computes the option price using FxOptionType enum.
    ///
    /// # Arguments
    /// * `option_type` - Call or Put
    fn price_fx(&self, option_type: FxOptionType) -> T;

    /// Computes Delta using FxOptionType enum.
    fn delta_fx(&self, option_type: FxOptionType) -> T;

    /// Computes Theta using FxOptionType enum.
    fn theta_fx(&self, option_type: FxOptionType) -> T;

    /// Computes Rho (domestic) using FxOptionType enum.
    fn rho_domestic_fx(&self, option_type: FxOptionType) -> T;

    /// Computes Rho (foreign) using FxOptionType enum.
    fn rho_foreign_fx(&self, option_type: FxOptionType) -> T;
}

impl<T: Float> GarmanKohlhagenExt<T> for GarmanKohlhagen<T> {
    fn price_fx(&self, option_type: FxOptionType) -> T {
        match option_type {
            FxOptionType::Call => self.price(true),
            FxOptionType::Put => self.price(false),
        }
    }

    fn delta_fx(&self, option_type: FxOptionType) -> T {
        match option_type {
            FxOptionType::Call => self.delta(true),
            FxOptionType::Put => self.delta(false),
        }
    }

    fn theta_fx(&self, option_type: FxOptionType) -> T {
        match option_type {
            FxOptionType::Call => self.theta(true),
            FxOptionType::Put => self.theta(false),
        }
    }

    fn rho_domestic_fx(&self, option_type: FxOptionType) -> T {
        match option_type {
            FxOptionType::Call => self.rho_domestic(true),
            FxOptionType::Put => self.rho_domestic(false),
        }
    }

    fn rho_foreign_fx(&self, option_type: FxOptionType) -> T {
        match option_type {
            FxOptionType::Call => self.rho_foreign(true),
            FxOptionType::Put => self.rho_foreign(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::{ExerciseStyle, InstrumentParams};

    #[test]
    fn test_black_scholes_price_option_call() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let params = InstrumentParams::new(100.0, 1.0, 1000.0).unwrap();
        let option = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);

        let price = bs.price_option(&option).unwrap();
        let expected = 1000.0 * bs.price_call(100.0, 1.0);
        assert!((price - expected).abs() < 1e-10);
    }

    #[test]
    fn test_black_scholes_price_option_put() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let params = InstrumentParams::new(100.0, 1.0, 1000.0).unwrap();
        let option = VanillaOption::new(params, PayoffType::Put, ExerciseStyle::European, 1e-6);

        let price = bs.price_option(&option).unwrap();
        let expected = 1000.0 * bs.price_put(100.0, 1.0);
        assert!((price - expected).abs() < 1e-10);
    }

    #[test]
    fn test_black_scholes_price_option_american_rejected() {
        let bs = BlackScholes::new(100.0_f64, 0.05, 0.2).unwrap();
        let params = InstrumentParams::new(100.0, 1.0, 1000.0).unwrap();
        let option = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::American, 1e-6);

        let result = bs.price_option(&option);
        assert!(result.is_err());
        match result.unwrap_err() {
            AnalyticalError::UnsupportedExerciseStyle { .. } => {}
            _ => panic!("Expected UnsupportedExerciseStyle error"),
        }
    }

    #[test]
    fn test_bachelier_price_option_call() {
        let model = Bachelier::new(0.03_f64, 0.01).unwrap();
        let params = InstrumentParams::new(0.03, 1.0, 10_000_000.0).unwrap();
        let option = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);

        let price = model.price_option(&option).unwrap();
        let expected = 10_000_000.0 * model.price_call(0.03, 1.0);
        assert!((price - expected).abs() < 1e-10);
    }

    #[test]
    fn test_garman_kohlhagen_fx_extension() {
        let params = GarmanKohlhagenParams::new(1.10, 1.12, 0.03, 0.01, 0.15, 1.0).unwrap();
        let model = GarmanKohlhagen::new(params);

        let call_price = model.price_fx(FxOptionType::Call);
        let put_price = model.price_fx(FxOptionType::Put);

        assert_eq!(call_price, model.price(true));
        assert_eq!(put_price, model.price(false));
    }

    #[test]
    fn test_analytical_error_to_pricing_error() {
        let err = AnalyticalError::UnsupportedExerciseStyle {
            style: "American".to_string(),
        };
        let pricing_err: PricingError = err.into();
        match pricing_err {
            PricingError::UnsupportedInstrument(msg) => {
                assert!(msg.contains("American"));
            }
            _ => panic!("Expected UnsupportedInstrument"),
        }
    }

    #[test]
    fn test_formula_error_to_pricing_error() {
        let err = AnalyticalError::Formula(FormulaError::InvalidVolatility { volatility: -0.1 });
        let pricing_err: PricingError = err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => {
                assert!(msg.contains("volatility"));
            }
            _ => panic!("Expected InvalidInput"),
        }
    }
}
