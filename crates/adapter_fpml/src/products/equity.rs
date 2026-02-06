//! Equity product parsers.
//!
//! Handles parsing for:
//! - Equity Option (equityOption)
//! - Equity Forward (equityForward)
//! - Equity Swap (returnSwap)

use crate::common::{parse_date, parse_decimal, parse_trade_header, XmlNavigator};
use crate::error::FpmlError;
use infra_domain::{
    trade::{
        Cashflow, CashflowType, Direction, ExerciseType, Leg, LegType, OptionType, Payoff,
        SettlementType, Trade, TradeMetadata, TradeType,
    },
    Currency, Date,
};

/// Parse an equity option from FpML.
pub fn parse_equity_option(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract equityOption section
    let option_section = nav
        .extract_section("equityOption")
        .ok_or_else(|| FpmlError::MissingElement("equityOption".to_string()))?;

    let opt_nav = XmlNavigator::new(&option_section);

    // Parse option type
    let option_type_str = opt_nav
        .find_text("optionType")
        .unwrap_or_else(|| "Call".to_string());
    let option_type = if option_type_str.to_lowercase().contains("put") {
        OptionType::Put
    } else {
        OptionType::Call
    };

    // Parse underlyer
    let underlyer = opt_nav
        .find_text("instrumentId")
        .or_else(|| opt_nav.find_text("description"))
        .unwrap_or_else(|| "UNKNOWN".to_string());

    // Parse strike
    let strike = opt_nav
        .find_text("strikePrice")
        .map(|s| parse_decimal(&s))
        .transpose()?
        .unwrap_or(0.0);

    // Parse notional/contract size
    let notional = opt_nav
        .find_text("amount")
        .or_else(|| opt_nav.find_text("numberOfOptions"))
        .map(|n| parse_decimal(&n))
        .transpose()?
        .unwrap_or(0.0);

    // Parse number of shares per contract
    let contract_multiplier = opt_nav
        .find_text("optionEntitlement")
        .or_else(|| opt_nav.find_text("openUnits"))
        .map(|n| parse_decimal(&n))
        .transpose()?
        .unwrap_or(1.0);

    // Parse currency
    let currency_str = opt_nav.find_text("currency").unwrap_or_else(|| "USD".to_string());
    let currency = parse_currency(&currency_str);

    // Parse exercise type
    let exercise_type = if opt_nav.extract_section("equityEuropeanExercise").is_some() {
        ExerciseType::European
    } else if opt_nav.extract_section("equityAmericanExercise").is_some() {
        ExerciseType::American
    } else if opt_nav.extract_section("equityBermudaExercise").is_some() {
        ExerciseType::Bermudan
    } else {
        ExerciseType::European
    };

    // Parse expiry date (look inside expirationDate for unadjustedDate)
    let expiry_date = opt_nav
        .extract_section("expirationDate")
        .and_then(|section| XmlNavigator::new(&section).find_text("unadjustedDate"))
        .or_else(|| opt_nav.find_text("unadjustedDate"))
        .map(|d| parse_date(&d))
        .transpose()?
        .unwrap_or_else(|| Date::from_ymd(2025, 1, 1).unwrap());

    // Parse settlement type
    let settlement_type_str = opt_nav
        .find_text("settlementType")
        .unwrap_or_else(|| "Cash".to_string());
    let settlement_type = if settlement_type_str.to_lowercase().contains("physical") {
        SettlementType::Physical
    } else {
        SettlementType::Cash
    };

    // Create cashflow for the option
    let cf = Cashflow::new(
        CashflowType::Settlement,
        expiry_date,
        expiry_date,
        expiry_date,
        0.0,
        notional * contract_multiplier,
        Payoff::fixed(1.0),
        currency,
    );

    let leg = Leg::new(vec![cf], Direction::Receiver, LegType::Generic, currency);

    let trade_type = TradeType::EquityOption {
        underlyer,
        option_type,
        strike,
        exercise_type,
        settlement_type,
        expiry_date,
        contract_multiplier,
    };

    let mut metadata = TradeMetadata::new();
    if let Some(td) = header.trade_date {
        metadata = metadata.with_trade_date(td);
    }
    if let Some(cp) = header.counterparty {
        metadata = metadata.with_counterparty(cp);
    }
    if let Some(book) = header.book {
        metadata = metadata.with_book(book);
    }

    Ok(Trade::builder()
        .id(header.trade_id)
        .legs(vec![leg])
        .trade_type(trade_type)
        .metadata(metadata)
        .build())
}

/// Parse an equity forward from FpML.
pub fn parse_equity_forward(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract equityForward section
    let fwd_section = nav
        .extract_section("equityForward")
        .ok_or_else(|| FpmlError::MissingElement("equityForward".to_string()))?;

    let fwd_nav = XmlNavigator::new(&fwd_section);

    // Parse underlyer
    let underlyer = fwd_nav
        .find_text("instrumentId")
        .or_else(|| fwd_nav.find_text("description"))
        .unwrap_or_else(|| "UNKNOWN".to_string());

    // Parse forward price
    let forward_price = fwd_nav
        .find_text("forwardPrice")
        .or_else(|| fwd_nav.find_text("price"))
        .map(|p| parse_decimal(&p))
        .transpose()?
        .unwrap_or(0.0);

    // Parse settlement date
    let settlement_date = fwd_nav
        .find_text("settlementDate")
        .map(|d| parse_date(&d))
        .transpose()?
        .unwrap_or_else(|| Date::from_ymd(2025, 1, 1).unwrap());

    // Parse notional
    let notional = fwd_nav
        .find_text("amount")
        .or_else(|| fwd_nav.find_text("numberOfShares"))
        .map(|n| parse_decimal(&n))
        .transpose()?
        .unwrap_or(0.0);

    let currency_str = fwd_nav.find_text("currency").unwrap_or_else(|| "USD".to_string());
    let currency = parse_currency(&currency_str);

    let cf = Cashflow::new(
        CashflowType::Settlement,
        settlement_date,
        settlement_date,
        settlement_date,
        0.0,
        notional * forward_price,
        Payoff::fixed(1.0),
        currency,
    );

    let leg = Leg::new(vec![cf], Direction::Receiver, LegType::Generic, currency);

    let trade_type = TradeType::EquityForward {
        underlyer,
        forward_price,
        settlement_date,
    };

    let mut metadata = TradeMetadata::new();
    if let Some(td) = header.trade_date {
        metadata = metadata.with_trade_date(td);
    }
    if let Some(cp) = header.counterparty {
        metadata = metadata.with_counterparty(cp);
    }
    if let Some(book) = header.book {
        metadata = metadata.with_book(book);
    }

    Ok(Trade::builder()
        .id(header.trade_id)
        .legs(vec![leg])
        .trade_type(trade_type)
        .metadata(metadata)
        .build())
}

/// Parse currency string to Currency enum.
fn parse_currency(s: &str) -> Currency {
    match s.to_uppercase().as_str() {
        "USD" => Currency::USD,
        "EUR" => Currency::EUR,
        "GBP" => Currency::GBP,
        "JPY" => Currency::JPY,
        "CHF" => Currency::CHF,
        _ => Currency::USD,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_EQUITY_OPTION_XML: &str = r#"
        <trade>
            <tradeHeader>
                <tradeId>EQOPT-AAPL-001</tradeId>
                <tradeDate>2024-11-01</tradeDate>
            </tradeHeader>
            <equityOption>
                <optionType>Call</optionType>
                <underlyer>
                    <singleUnderlyer>
                        <equity>
                            <instrumentId>AAPL.O</instrumentId>
                            <description>Apple Inc.</description>
                        </equity>
                        <openUnits>100000</openUnits>
                    </singleUnderlyer>
                </underlyer>
                <notional>
                    <currency>USD</currency>
                    <amount>20000000</amount>
                </notional>
                <equityExercise>
                    <equityAmericanExercise>
                        <expirationDate>
                            <adjustableDate>
                                <unadjustedDate>2025-11-01</unadjustedDate>
                            </adjustableDate>
                        </expirationDate>
                    </equityAmericanExercise>
                </equityExercise>
                <strike>
                    <strikePrice>200.00</strikePrice>
                    <currency>USD</currency>
                </strike>
                <settlementType>Cash</settlementType>
                <numberOfOptions>100000</numberOfOptions>
                <optionEntitlement>1</optionEntitlement>
            </equityOption>
        </trade>
    "#;

    #[test]
    fn test_parse_equity_option() {
        let trade = parse_equity_option(SAMPLE_EQUITY_OPTION_XML).unwrap();

        assert_eq!(trade.id.as_str(), "EQOPT-AAPL-001");
        assert!(trade.trade_type.is_equity());
        assert!(trade.trade_type.is_option());
    }
}
