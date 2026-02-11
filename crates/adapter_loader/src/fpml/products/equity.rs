//! Equity product parsers.

use infra_domain::{
    time::Date,
    trade::{
        Cashflow, CashflowType, Direction, ExerciseType, Leg, LegType, OptionType, Payoff,
        SettlementType, Trade, TradeType,
    },
};

use crate::fpml::{
    common::{
        build_metadata, parse_currency, parse_date, parse_trade_header, xml_decimal,
        xml_decimal_or, xml_text, XmlNavigator,
    },
    error::FpmlError,
};

/// Parse an equity option from FpML.
pub fn parse_equity_option(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    let header = parse_trade_header(xml)?;

    let option_section = nav
        .extract_section("equityOption")
        .ok_or_else(|| FpmlError::MissingElement("equityOption".to_string()))?;

    let opt_nav = XmlNavigator::new(&option_section);

    let option_type_str = xml_text!(opt_nav, "optionType", "Call");
    let option_type = if option_type_str.to_lowercase().contains("put") {
        OptionType::Put
    } else {
        OptionType::Call
    };

    let underlyer = opt_nav
        .find_text("instrumentId")
        .or_else(|| opt_nav.find_text("description"))
        .unwrap_or_else(|| "UNKNOWN".to_string());

    let strike = xml_decimal!(opt_nav, "strikePrice", 0.0);

    let notional = xml_decimal_or!(opt_nav, "amount", "numberOfOptions"; 0.0);

    let contract_multiplier = xml_decimal_or!(opt_nav, "optionEntitlement", "openUnits"; 1.0);

    let currency = parse_currency(&xml_text!(opt_nav, "currency", "USD"));

    let exercise_type = if opt_nav.extract_section("equityEuropeanExercise").is_some() {
        ExerciseType::European
    } else if opt_nav.extract_section("equityAmericanExercise").is_some() {
        ExerciseType::American
    } else if opt_nav.extract_section("equityBermudaExercise").is_some() {
        ExerciseType::Bermudan
    } else {
        ExerciseType::European
    };

    let expiry_date = opt_nav
        .extract_section("expirationDate")
        .and_then(|section| XmlNavigator::new(&section).find_text("unadjustedDate"))
        .or_else(|| opt_nav.find_text("unadjustedDate"))
        .map(|d| parse_date(&d))
        .transpose()?
        .unwrap_or_else(|| Date::from_ymd(2025, 1, 1).unwrap());

    let settlement_type_str = xml_text!(opt_nav, "settlementType", "Cash");
    let settlement_type = if settlement_type_str.to_lowercase().contains("physical") {
        SettlementType::Physical
    } else {
        SettlementType::Cash
    };

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

    let metadata = build_metadata(&header);

    Ok(Trade::builder()
        .id(header.trade_id)
        .legs(vec![leg])
        .trade_type(trade_type)
        .metadata(metadata)
        .build())
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
