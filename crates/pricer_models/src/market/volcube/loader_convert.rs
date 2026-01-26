//! Loader conversion module for VolQuoteSet.
//!
//! # Requirements: 10.7
//!
//! Provides conversion from adapter_loader types to `VolQuoteSet`.
//! This module bridges the Adapter layer (adapter_loader) with the Pricer layer.
//!
//! # Example
//!
//! ```rust,ignore
//! use adapter_loader::{VolSurfaceLoader, VolQuoteSetJson};
//! use pricer_models::market::volcube::convert_vol_quote_set_json;
//!
//! let json_data = VolSurfaceLoader::load_json("vol_quotes.json")?;
//! let quote_set = convert_vol_quote_set_json(&json_data)?;
//! ```

use chrono::NaiveDate;

use super::quote::{Currency, QuoteType, Tenor, UnderlyingIndex, VolQuote, VolQuoteSet, VolStrike};
use super::types::InstrumentId;

/// Error type for conversion operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConversionError {
    /// Invalid date format.
    #[error("Invalid date format: {0}")]
    InvalidDate(String),
    /// Invalid tenor format.
    #[error("Invalid tenor format: {0}")]
    InvalidTenor(String),
    /// Invalid currency.
    #[error("Unknown currency: {0}")]
    UnknownCurrency(String),
    /// Invalid underlying index.
    #[error("Unknown underlying index: {0}")]
    UnknownIndex(String),
    /// Invalid quote type.
    #[error("Invalid quote type: {0}")]
    InvalidQuoteType(String),
    /// Invalid strike type.
    #[error("Invalid strike type: {0}")]
    InvalidStrikeType(String),
    /// Validation error.
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Result type for conversions.
pub type ConversionResult<T> = Result<T, ConversionError>;

/// Parse currency string to Currency enum.
pub fn parse_currency(s: &str) -> ConversionResult<Currency> {
    match s.to_uppercase().as_str() {
        "USD" => Ok(Currency::Usd),
        "EUR" => Ok(Currency::Eur),
        "JPY" => Ok(Currency::Jpy),
        "GBP" => Ok(Currency::Gbp),
        "CHF" => Ok(Currency::Chf),
        _ => Err(ConversionError::UnknownCurrency(s.to_string())),
    }
}

/// Parse underlying index string to UnderlyingIndex enum.
pub fn parse_underlying_index(s: &str) -> ConversionResult<UnderlyingIndex> {
    match s.to_uppercase().as_str() {
        "SOFR" => Ok(UnderlyingIndex::Sofr),
        "ESTR" | "€STR" => Ok(UnderlyingIndex::Estr),
        "TONA" => Ok(UnderlyingIndex::Tona),
        "EURIBOR" => Ok(UnderlyingIndex::Euribor),
        "LIBOR" => Ok(UnderlyingIndex::Libor),
        _ => Err(ConversionError::UnknownIndex(s.to_string())),
    }
}

/// Parse quote type string to QuoteType enum.
pub fn parse_quote_type(s: &str, shift: Option<f64>) -> ConversionResult<QuoteType> {
    match s.to_lowercase().as_str() {
        "normal" => Ok(QuoteType::Normal),
        "lognormal" | "black" => Ok(QuoteType::LogNormal),
        "shifted_lognormal" | "shifted-lognormal" | "shifted" => {
            let shift = shift.unwrap_or(0.0);
            Ok(QuoteType::ShiftedLogNormal { shift })
        }
        _ => Err(ConversionError::InvalidQuoteType(s.to_string())),
    }
}

/// Parse strike type string to VolStrike.
pub fn parse_strike(value: f64, strike_type: &str) -> ConversionResult<VolStrike> {
    match strike_type.to_lowercase().as_str() {
        "absolute" | "abs" => Ok(VolStrike::Absolute(value)),
        "relative" | "rel" | "relative_to_atm" => Ok(VolStrike::RelativeToAtm(value)),
        "moneyness" => Ok(VolStrike::Moneyness(value)),
        "log_moneyness" | "logmoneyness" => Ok(VolStrike::LogMoneyness(value)),
        _ => Err(ConversionError::InvalidStrikeType(strike_type.to_string())),
    }
}

/// Parse tenor string to years (f64).
pub fn parse_tenor_to_years(s: &str) -> ConversionResult<f64> {
    let s = s.trim().to_uppercase();

    if s.ends_with('Y') {
        let num_str = &s[..s.len() - 1];
        num_str
            .parse::<f64>()
            .map_err(|_| ConversionError::InvalidTenor(s.clone()))
    } else if s.ends_with('M') {
        let num_str = &s[..s.len() - 1];
        num_str
            .parse::<f64>()
            .map(|m| m / 12.0)
            .map_err(|_| ConversionError::InvalidTenor(s.clone()))
    } else if s.ends_with('W') {
        let num_str = &s[..s.len() - 1];
        num_str
            .parse::<f64>()
            .map(|w| w / 52.0)
            .map_err(|_| ConversionError::InvalidTenor(s.clone()))
    } else if s.ends_with('D') {
        let num_str = &s[..s.len() - 1];
        num_str
            .parse::<f64>()
            .map(|d| d / 365.0)
            .map_err(|_| ConversionError::InvalidTenor(s.clone()))
    } else {
        s.parse::<f64>()
            .map_err(|_| ConversionError::InvalidTenor(s.clone()))
    }
}

/// Convert expiry string to NaiveDate.
pub fn parse_expiry_to_date(
    expiry_str: &str,
    as_of_date: NaiveDate,
) -> ConversionResult<NaiveDate> {
    // Try ISO date format first
    if let Ok(date) = NaiveDate::parse_from_str(expiry_str, "%Y-%m-%d") {
        return Ok(date);
    }

    // Try tenor format
    let years = parse_tenor_to_years(expiry_str)?;
    let days = (years * 365.0).round() as i64;
    as_of_date
        .checked_add_signed(chrono::Duration::days(days))
        .ok_or_else(|| ConversionError::InvalidDate(expiry_str.to_string()))
}

// =============================================================================
// Converter Traits
// =============================================================================

/// Trait for converting loader data to VolQuoteSet.
pub trait ToVolQuoteSet {
    /// Convert to VolQuoteSet.
    fn to_vol_quote_set(&self) -> ConversionResult<VolQuoteSet>;
}

/// Trait for converting a single row/quote to VolQuote.
pub trait ToVolQuote {
    /// Convert to VolQuote.
    fn to_vol_quote(&self, as_of_date: NaiveDate, index: usize) -> ConversionResult<VolQuote>;
}

// =============================================================================
// Generic Conversion Functions
// =============================================================================

/// Convert a JSON-style vol quote set to VolQuoteSet.
///
/// # Arguments
///
/// * `currency` - Currency string (e.g., "USD")
/// * `underlying_index` - Optional underlying index string
/// * `as_of_date` - As-of date string (YYYY-MM-DD)
/// * `quotes` - Iterator of quote data (expiry, tenor_years, strike_value, strike_type, mid, bid, ask, quote_type, shift)
pub fn convert_json_quotes<'a, I>(
    currency: &str,
    underlying_index: Option<&str>,
    as_of_date_str: &str,
    quotes: I,
) -> ConversionResult<VolQuoteSet>
where
    I: IntoIterator<
        Item = (
            &'a str,       // expiry
            f64,           // tenor_years
            f64,           // strike_value
            &'a str,       // strike_type
            f64,           // mid
            Option<f64>,   // bid
            Option<f64>,   // ask
            Option<&'a str>, // quote_type
            Option<f64>,   // shift
            Option<&'a str>, // instrument_id
        ),
    >,
{
    let ccy = parse_currency(currency)?;
    let index = match underlying_index {
        Some(idx) => parse_underlying_index(idx)?,
        None => ccy.default_index(),
    };
    let as_of_date = NaiveDate::parse_from_str(as_of_date_str, "%Y-%m-%d")
        .map_err(|_| ConversionError::InvalidDate(as_of_date_str.to_string()))?;

    let mut quote_set = VolQuoteSet::new(ccy, index, as_of_date);

    for (i, (expiry, tenor_years, strike_value, strike_type, mid, bid, ask, quote_type, shift, inst_id)) in quotes.into_iter().enumerate() {
        let expiry_date = parse_expiry_to_date(expiry, as_of_date)?;
        let tenor = Tenor::years(tenor_years);
        let strike = parse_strike(strike_value, strike_type)?;
        let qt = match quote_type {
            Some(qt_str) => parse_quote_type(qt_str, shift)?,
            None => QuoteType::LogNormal,
        };

        let instrument_id: InstrumentId = match inst_id {
            Some(id) => id.into(),
            None => format!("VOL-{}", i).into(),
        };

        let mut quote = VolQuote::new(instrument_id, expiry_date, tenor, strike, mid)
            .with_quote_type(qt);

        if let Some(b) = bid {
            quote = quote.with_bid(b);
        }
        if let Some(a) = ask {
            quote = quote.with_ask(a);
        }

        quote_set.add_quote(quote);
    }

    Ok(quote_set)
}

/// Convert CSV swaption rows to VolQuoteSet.
///
/// # Arguments
///
/// * `currency` - Currency
/// * `underlying_index` - Underlying index
/// * `as_of_date` - As-of date
/// * `rows` - Iterator of (expiry_str, tenor_str, strike, mid, bid, ask, quote_type, strike_type, shift)
pub fn convert_swaption_csv_rows<'a, I>(
    currency: Currency,
    underlying_index: UnderlyingIndex,
    as_of_date: NaiveDate,
    rows: I,
) -> ConversionResult<VolQuoteSet>
where
    I: IntoIterator<
        Item = (
            &'a str,       // expiry
            &'a str,       // tenor
            f64,           // strike
            f64,           // mid
            Option<f64>,   // bid
            Option<f64>,   // ask
            &'a str,       // quote_type
            &'a str,       // strike_type
            Option<f64>,   // shift
        ),
    >,
{
    let mut quote_set = VolQuoteSet::new(currency, underlying_index, as_of_date);

    for (i, (expiry_str, tenor_str, strike_val, mid, bid, ask, quote_type_str, strike_type_str, shift)) in rows.into_iter().enumerate() {
        let expiry_date = parse_expiry_to_date(expiry_str, as_of_date)?;
        let tenor_years = parse_tenor_to_years(tenor_str)?;
        let tenor = Tenor::years(tenor_years);
        let strike = parse_strike(strike_val, strike_type_str)?;
        let qt = parse_quote_type(quote_type_str, shift)?;

        let instrument_id: InstrumentId = format!("SWAPTION-{}", i).into();

        let mut quote = VolQuote::new(instrument_id, expiry_date, tenor, strike, mid)
            .with_quote_type(qt);

        if let Some(b) = bid {
            quote = quote.with_bid(b);
        }
        if let Some(a) = ask {
            quote = quote.with_ask(a);
        }

        quote_set.add_quote(quote);
    }

    Ok(quote_set)
}

/// Convert CSV capfloor rows to VolQuoteSet.
///
/// Cap/Floor quotes typically don't have an underlying tenor in the same way swaptions do.
/// We use a default tenor or derive it from the cap structure.
pub fn convert_capfloor_csv_rows<'a, I>(
    currency: Currency,
    underlying_index: UnderlyingIndex,
    as_of_date: NaiveDate,
    default_tenor: f64,
    rows: I,
) -> ConversionResult<VolQuoteSet>
where
    I: IntoIterator<
        Item = (
            &'a str,       // expiry
            f64,           // strike
            f64,           // mid
            Option<f64>,   // bid
            Option<f64>,   // ask
            &'a str,       // quote_type
            &'a str,       // strike_type
            Option<f64>,   // shift
            &'a str,       // cap_floor
        ),
    >,
{
    let mut quote_set = VolQuoteSet::new(currency, underlying_index, as_of_date);

    for (i, (expiry_str, strike_val, mid, bid, ask, quote_type_str, strike_type_str, shift, cap_floor)) in rows.into_iter().enumerate() {
        let expiry_date = parse_expiry_to_date(expiry_str, as_of_date)?;
        let tenor = Tenor::years(default_tenor);
        let strike = parse_strike(strike_val, strike_type_str)?;
        let qt = parse_quote_type(quote_type_str, shift)?;

        let instrument_id: InstrumentId = format!("{}-{}", cap_floor.to_uppercase(), i).into();

        let mut quote = VolQuote::new(instrument_id, expiry_date, tenor, strike, mid)
            .with_quote_type(qt);

        if let Some(b) = bid {
            quote = quote.with_bid(b);
        }
        if let Some(a) = ask {
            quote = quote.with_ask(a);
        }

        quote_set.add_quote(quote);
    }

    Ok(quote_set)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_currency() {
        assert!(matches!(parse_currency("USD").unwrap(), Currency::Usd));
        assert!(matches!(parse_currency("eur").unwrap(), Currency::Eur));
        assert!(matches!(parse_currency("JPY").unwrap(), Currency::Jpy));
        assert!(parse_currency("XYZ").is_err());
    }

    #[test]
    fn test_parse_underlying_index() {
        assert!(matches!(
            parse_underlying_index("SOFR").unwrap(),
            UnderlyingIndex::Sofr
        ));
        assert!(matches!(
            parse_underlying_index("estr").unwrap(),
            UnderlyingIndex::Estr
        ));
        assert!(matches!(
            parse_underlying_index("TONA").unwrap(),
            UnderlyingIndex::Tona
        ));
        assert!(parse_underlying_index("XYZ").is_err());
    }

    #[test]
    fn test_parse_quote_type() {
        assert!(matches!(
            parse_quote_type("normal", None).unwrap(),
            QuoteType::Normal
        ));
        assert!(matches!(
            parse_quote_type("lognormal", None).unwrap(),
            QuoteType::LogNormal
        ));
        assert!(matches!(
            parse_quote_type("shifted_lognormal", Some(0.02)).unwrap(),
            QuoteType::ShiftedLogNormal { shift } if (shift - 0.02).abs() < 1e-10
        ));
        assert!(parse_quote_type("invalid", None).is_err());
    }

    #[test]
    fn test_parse_strike() {
        let s = parse_strike(0.03, "absolute").unwrap();
        assert!(matches!(s, VolStrike::Absolute(v) if (v - 0.03).abs() < 1e-10));

        let s = parse_strike(50.0, "relative").unwrap();
        assert!(matches!(s, VolStrike::RelativeToAtm(v) if (v - 50.0).abs() < 1e-10));

        let s = parse_strike(1.1, "moneyness").unwrap();
        assert!(matches!(s, VolStrike::Moneyness(v) if (v - 1.1).abs() < 1e-10));

        let s = parse_strike(0.1, "log_moneyness").unwrap();
        assert!(matches!(s, VolStrike::LogMoneyness(v) if (v - 0.1).abs() < 1e-10));

        assert!(parse_strike(0.03, "invalid").is_err());
    }

    #[test]
    fn test_parse_tenor_to_years() {
        assert!((parse_tenor_to_years("1Y").unwrap() - 1.0).abs() < 1e-10);
        assert!((parse_tenor_to_years("5Y").unwrap() - 5.0).abs() < 1e-10);
        assert!((parse_tenor_to_years("6M").unwrap() - 0.5).abs() < 1e-10);
        assert!((parse_tenor_to_years("3M").unwrap() - 0.25).abs() < 1e-10);
        assert!(parse_tenor_to_years("XYZ").is_err());
    }

    #[test]
    fn test_parse_expiry_to_date() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();

        // ISO date
        let date = parse_expiry_to_date("2027-01-25", as_of).unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2027, 1, 25).unwrap());

        // Tenor format
        let date = parse_expiry_to_date("1Y", as_of).unwrap();
        let days_diff = (date - as_of).num_days();
        assert!(days_diff >= 364 && days_diff <= 366);
    }

    #[test]
    fn test_convert_json_quotes() {
        let quotes = vec![(
            "2027-01-25",  // expiry
            5.0,           // tenor_years
            0.03,          // strike_value
            "absolute",    // strike_type
            0.20,          // mid
            Some(0.19),    // bid
            Some(0.21),    // ask
            Some("lognormal"), // quote_type
            None,          // shift
            Some("TEST-1"), // instrument_id
        )];

        let quote_set = convert_json_quotes(
            "USD",
            Some("SOFR"),
            "2026-01-25",
            quotes,
        ).unwrap();

        assert_eq!(quote_set.len(), 1);
        assert!(matches!(quote_set.currency, Currency::Usd));
        assert!(matches!(quote_set.underlying_index, UnderlyingIndex::Sofr));
        assert_eq!(quote_set.quotes[0].mid, 0.20);
        assert_eq!(quote_set.quotes[0].bid, Some(0.19));
        assert_eq!(quote_set.quotes[0].ask, Some(0.21));
    }

    #[test]
    fn test_convert_swaption_csv_rows() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let rows = vec![
            ("1Y", "5Y", 0.03, 0.20, Some(0.19), Some(0.21), "lognormal", "absolute", None),
            ("2Y", "10Y", 0.035, 0.22, None, None, "lognormal", "absolute", None),
        ];

        let quote_set = convert_swaption_csv_rows(
            Currency::Usd,
            UnderlyingIndex::Sofr,
            as_of,
            rows,
        ).unwrap();

        assert_eq!(quote_set.len(), 2);
        assert!((quote_set.quotes[0].mid - 0.20).abs() < 1e-10);
        assert!((quote_set.quotes[1].mid - 0.22).abs() < 1e-10);
    }

    #[test]
    fn test_convert_capfloor_csv_rows() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let rows = vec![
            ("5Y", 0.03, 0.25, None, None, "lognormal", "absolute", None, "cap"),
            ("10Y", 0.04, 0.22, None, None, "lognormal", "absolute", None, "floor"),
        ];

        let quote_set = convert_capfloor_csv_rows(
            Currency::Usd,
            UnderlyingIndex::Sofr,
            as_of,
            0.25, // 3M default tenor
            rows,
        ).unwrap();

        assert_eq!(quote_set.len(), 2);
        assert!(quote_set.quotes[0].instrument_id.as_str().starts_with("CAP"));
        assert!(quote_set.quotes[1].instrument_id.as_str().starts_with("FLOOR"));
    }

    #[test]
    fn test_convert_json_quotes_default_index() {
        let quotes = vec![(
            "1Y",
            5.0,
            0.03,
            "absolute",
            0.20,
            None,
            None,
            None::<&str>,
            None,
            None::<&str>,
        )];

        let quote_set = convert_json_quotes(
            "EUR",
            None, // No index specified, should default to ESTR
            "2026-01-25",
            quotes,
        ).unwrap();

        assert!(matches!(quote_set.underlying_index, UnderlyingIndex::Estr));
    }

    #[test]
    fn test_conversion_error_display() {
        let err = ConversionError::InvalidDate("bad-date".to_string());
        assert!(err.to_string().contains("bad-date"));

        let err = ConversionError::UnknownCurrency("XYZ".to_string());
        assert!(err.to_string().contains("XYZ"));
    }
}
