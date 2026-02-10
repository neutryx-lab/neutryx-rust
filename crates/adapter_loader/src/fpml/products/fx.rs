//! FX product parsers.
//!
//! Handles parsing for:
//! - FX Spot/Forward (fxSingleLeg)
//! - FX Swap (fxSwap)
//! - FX Option (fxOption)

use infra_domain::{
    market::Currency,
    time::Date,
    trade::{
        Cashflow, CashflowType, Direction, ExerciseType, Leg, LegType, OptionType, Payoff,
        SettlementType, Trade, TradeMetadata, TradeType,
    },
};

use crate::fpml::{
    common::{parse_date, parse_decimal, parse_trade_header, XmlNavigator},
    error::FpmlError,
};

/// Parse an FX forward (fxSingleLeg) from FpML.
pub fn parse_fx_forward(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract fxSingleLeg section
    let fx_section = nav
        .extract_section("fxSingleLeg")
        .ok_or_else(|| FpmlError::MissingElement("fxSingleLeg".to_string()))?;

    let fx_nav = XmlNavigator::new(&fx_section);

    // Parse value date
    let value_date = fx_nav
        .find_text("valueDate")
        .map(|d| parse_date(&d))
        .transpose()?
        .unwrap_or_else(|| Date::from_ymd(2024, 1, 1).unwrap());

    // Parse exchanged currencies
    let ccy1_section = fx_nav
        .extract_section("exchangedCurrency1")
        .unwrap_or_default();
    let ccy2_section = fx_nav
        .extract_section("exchangedCurrency2")
        .unwrap_or_default();

    let ccy1_nav = XmlNavigator::new(&ccy1_section);
    let ccy2_nav = XmlNavigator::new(&ccy2_section);

    let ccy1_str = ccy1_nav
        .find_text("currency")
        .unwrap_or_else(|| "EUR".to_string());
    let ccy2_str = ccy2_nav
        .find_text("currency")
        .unwrap_or_else(|| "USD".to_string());

    let amount1 = ccy1_nav
        .find_text("amount")
        .map(|a| parse_decimal(&a))
        .transpose()?
        .unwrap_or(0.0);

    let amount2 = ccy2_nav
        .find_text("amount")
        .map(|a| parse_decimal(&a))
        .transpose()?
        .unwrap_or(0.0);

    let currency1 = parse_currency(&ccy1_str);
    let currency2 = parse_currency(&ccy2_str);

    // Parse exchange rate
    let _rate = fx_nav
        .find_text("rate")
        .map(|r| parse_decimal(&r))
        .transpose()?
        .unwrap_or(1.0);

    // Create legs for each currency
    // Leg 1: Pay currency 1
    let leg1_cf = Cashflow::new(
        CashflowType::Principal,
        value_date,
        value_date,
        value_date,
        0.0,
        amount1,
        Payoff::fixed(1.0),
        currency1,
    );
    let leg1 = Leg::new(
        vec![leg1_cf],
        Direction::Payer,
        LegType::Principal,
        currency1,
    );

    // Leg 2: Receive currency 2
    let leg2_cf = Cashflow::new(
        CashflowType::Principal,
        value_date,
        value_date,
        value_date,
        0.0,
        amount2,
        Payoff::fixed(1.0),
        currency2,
    );
    let leg2 = Leg::new(
        vec![leg2_cf],
        Direction::Receiver,
        LegType::Principal,
        currency2,
    );

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
        .legs(vec![leg1, leg2])
        .trade_type(TradeType::FxForward)
        .metadata(metadata)
        .build())
}

/// Parse an FX swap from FpML.
pub fn parse_fx_swap(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract fxSwap section
    let fx_section = nav
        .extract_section("fxSwap")
        .ok_or_else(|| FpmlError::MissingElement("fxSwap".to_string()))?;

    let fx_nav = XmlNavigator::new(&fx_section);

    // Parse near leg
    let near_section = fx_nav.extract_section("nearLeg").unwrap_or_default();
    let _near_nav = XmlNavigator::new(&near_section);

    // Parse far leg
    let far_section = fx_nav.extract_section("farLeg").unwrap_or_default();
    let _far_nav = XmlNavigator::new(&far_section);

    // For simplicity, we'll parse this as a single FxSwap with combined legs
    // A full implementation would parse near and far legs separately

    let mut legs = Vec::new();

    // Parse all exchangedCurrency sections
    for ccy_section in fx_nav.extract_all_sections("exchangedCurrency1") {
        let ccy_nav = XmlNavigator::new(&ccy_section);
        let currency_str = ccy_nav
            .find_text("currency")
            .unwrap_or_else(|| "USD".to_string());
        let amount = ccy_nav
            .find_text("amount")
            .map(|a| parse_decimal(&a))
            .transpose()?
            .unwrap_or(0.0);

        let currency = parse_currency(&currency_str);

        // Get value date from nearest ancestor
        let value_date = fx_nav
            .find_text("valueDate")
            .map(|d| parse_date(&d))
            .transpose()?
            .unwrap_or_else(|| Date::from_ymd(2024, 1, 1).unwrap());

        let cf = Cashflow::new(
            CashflowType::Principal,
            value_date,
            value_date,
            value_date,
            0.0,
            amount,
            Payoff::fixed(1.0),
            currency,
        );
        legs.push(Leg::new(
            vec![cf],
            Direction::Payer,
            LegType::Principal,
            currency,
        ));
    }

    for ccy_section in fx_nav.extract_all_sections("exchangedCurrency2") {
        let ccy_nav = XmlNavigator::new(&ccy_section);
        let currency_str = ccy_nav
            .find_text("currency")
            .unwrap_or_else(|| "USD".to_string());
        let amount = ccy_nav
            .find_text("amount")
            .map(|a| parse_decimal(&a))
            .transpose()?
            .unwrap_or(0.0);

        let currency = parse_currency(&currency_str);
        let value_date = fx_nav
            .find_text("valueDate")
            .map(|d| parse_date(&d))
            .transpose()?
            .unwrap_or_else(|| Date::from_ymd(2024, 1, 1).unwrap());

        let cf = Cashflow::new(
            CashflowType::Principal,
            value_date,
            value_date,
            value_date,
            0.0,
            amount,
            Payoff::fixed(1.0),
            currency,
        );
        legs.push(Leg::new(
            vec![cf],
            Direction::Receiver,
            LegType::Principal,
            currency,
        ));
    }

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
        .legs(legs)
        .trade_type(TradeType::FxSwap)
        .metadata(metadata)
        .build())
}

/// Parse an FX option from FpML.
pub fn parse_fx_option(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract fxOption section
    let option_section = nav
        .extract_section("fxOption")
        .ok_or_else(|| FpmlError::MissingElement("fxOption".to_string()))?;

    let opt_nav = XmlNavigator::new(&option_section);

    // Parse option type (Call/Put)
    let option_type_str = opt_nav
        .find_text("optionType")
        .or_else(|| {
            opt_nav
                .find_text("callCurrencyAmount")
                .map(|_| "Call".to_string())
        })
        .unwrap_or_else(|| "Call".to_string());

    let option_type = if option_type_str.to_lowercase().contains("put") {
        OptionType::Put
    } else {
        OptionType::Call
    };

    // Parse strike
    let strike = opt_nav
        .find_text("rate")
        .or_else(|| opt_nav.find_text("strikePrice"))
        .map(|s| parse_decimal(&s))
        .transpose()?
        .unwrap_or(1.0);

    // Parse expiry date
    let expiry_date = opt_nav
        .find_text("expiryDate")
        .map(|d| parse_date(&d))
        .transpose()?
        .unwrap_or_else(|| Date::from_ymd(2025, 1, 1).unwrap());

    // Parse exercise type
    let exercise_type = if opt_nav.extract_section("europeanExercise").is_some() {
        ExerciseType::European
    } else if opt_nav.extract_section("americanExercise").is_some() {
        ExerciseType::American
    } else {
        ExerciseType::European
    };

    // Parse settlement type
    let settlement_type = if opt_nav.find_text("cashSettlement").is_some() {
        SettlementType::Cash
    } else {
        SettlementType::Physical
    };

    // Parse notional (look inside callCurrencyAmount/putCurrencyAmount for amount)
    let notional = opt_nav
        .extract_section("callCurrencyAmount")
        .and_then(|section| XmlNavigator::new(&section).find_text("amount"))
        .or_else(|| {
            opt_nav
                .extract_section("putCurrencyAmount")
                .and_then(|section| XmlNavigator::new(&section).find_text("amount"))
        })
        .or_else(|| opt_nav.find_text("amount"))
        .map(|a| parse_decimal(&a))
        .transpose()?
        .unwrap_or(0.0);

    // Parse currency (look inside callCurrencyAmount/putCurrencyAmount for
    // currency)
    let currency_str = opt_nav
        .extract_section("callCurrencyAmount")
        .and_then(|section| XmlNavigator::new(&section).find_text("currency"))
        .or_else(|| {
            opt_nav
                .extract_section("putCurrencyAmount")
                .and_then(|section| XmlNavigator::new(&section).find_text("currency"))
        })
        .or_else(|| opt_nav.find_text("callCurrency"))
        .or_else(|| opt_nav.find_text("putCurrency"))
        .or_else(|| opt_nav.find_text("currency"))
        .unwrap_or_else(|| "USD".to_string());
    let currency = parse_currency(&currency_str);

    let cf = Cashflow::new(
        CashflowType::Settlement,
        expiry_date,
        expiry_date,
        expiry_date,
        0.0,
        notional,
        Payoff::fixed(1.0), // Option payoff is handled by TradeType
        currency,
    );

    let leg = Leg::new(vec![cf], Direction::Receiver, LegType::Generic, currency);

    let trade_type = TradeType::FxOption {
        option_type,
        strike,
        exercise_type,
        settlement_type,
        expiry_date,
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

    const SAMPLE_FX_FORWARD_XML: &str = r#"
        <trade>
            <tradeHeader>
                <tradeId>FXFWD-EURUSD-001</tradeId>
                <tradeDate>2024-01-15</tradeDate>
            </tradeHeader>
            <fxSingleLeg>
                <exchangedCurrency1>
                    <paymentAmount>
                        <currency>EUR</currency>
                        <amount>25000000</amount>
                    </paymentAmount>
                </exchangedCurrency1>
                <exchangedCurrency2>
                    <paymentAmount>
                        <currency>USD</currency>
                        <amount>27250000</amount>
                    </paymentAmount>
                </exchangedCurrency2>
                <valueDate>2024-07-15</valueDate>
                <exchangeRate>
                    <rate>1.0900</rate>
                </exchangeRate>
            </fxSingleLeg>
        </trade>
    "#;

    #[test]
    fn test_parse_fx_forward() {
        let trade = parse_fx_forward(SAMPLE_FX_FORWARD_XML).unwrap();

        assert_eq!(trade.id.as_str(), "FXFWD-EURUSD-001");
        assert_eq!(trade.num_legs(), 2);
        assert!(trade.trade_type.is_fx());
    }

    #[test]
    fn test_parse_currency() {
        assert_eq!(parse_currency("EUR"), Currency::EUR);
        assert_eq!(parse_currency("eur"), Currency::EUR);
        assert_eq!(parse_currency("JPY"), Currency::JPY);
    }
}
