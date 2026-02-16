//! Credit product parsers.

use infra_domain::{
    time::Date,
    trade::{
        Cashflow, CashflowType, Direction, Leg, LegType, Payoff, ProtectionSide, Trade, TradeType,
    },
};

use crate::fpml::{
    common::{
        build_metadata, extract_nested_amount, extract_nested_date, parse_currency, parse_date,
        parse_trade_header, xml_decimal, xml_text, XmlNavigator,
    },
    error::FpmlError,
};

/// Parse a credit default swap from FpML.
pub fn parse_credit_default_swap(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    let header = parse_trade_header(xml)?;

    let cds_section = nav
        .extract_section("creditDefaultSwap")
        .ok_or_else(|| FpmlError::MissingElement("creditDefaultSwap".to_string()))?;

    let cds_nav = XmlNavigator::new(&cds_section);

    let reference_entity = xml_text!(cds_nav, "entityName", "UNKNOWN");

    let entity_id = cds_nav.find_text("entityId");

    let protection_side = if cds_nav.extract_section("buyerPartyReference").is_some() {
        ProtectionSide::Buyer
    } else {
        ProtectionSide::Seller
    };

    let effective_date = extract_nested_date(
        &cds_nav,
        "effectiveDate",
        "unadjustedDate",
        Date::from_ymd(2024, 1, 1).unwrap(),
    )?;

    let termination_date = extract_nested_date(
        &cds_nav,
        "scheduledTerminationDate",
        "scheduledTerminationDate",
        Date::from_ymd(2029, 1, 1).unwrap(),
    )?;

    let notional = extract_nested_amount(&cds_nav, "calculationAmount", "amount", 0.0)?;

    let currency = parse_currency(&xml_text!(cds_nav, "currency", "USD"));

    let fixed_rate = xml_decimal!(cds_nav, "fixedRate", 0.01);

    let fee_cf = Cashflow::new(
        CashflowType::Coupon,
        termination_date,
        effective_date,
        termination_date,
        1.0,
        notional,
        Payoff::fixed(fixed_rate),
        currency,
    );

    let fee_leg = Leg::new(
        vec![fee_cf],
        if protection_side == ProtectionSide::Buyer {
            Direction::Payer
        } else {
            Direction::Receiver
        },
        LegType::Fixed,
        currency,
    );

    let protection_cf = Cashflow::new(
        CashflowType::Settlement,
        termination_date,
        effective_date,
        termination_date,
        0.0,
        notional,
        Payoff::fixed(1.0),
        currency,
    );

    let protection_leg = Leg::new(
        vec![protection_cf],
        if protection_side == ProtectionSide::Buyer {
            Direction::Receiver
        } else {
            Direction::Payer
        },
        LegType::Generic,
        currency,
    );

    let trade_type = TradeType::CreditDefaultSwap {
        reference_entity,
        entity_id,
        protection_side,
    };

    let metadata = build_metadata(&header);

    Ok(Trade::builder()
        .id(header.trade_id)
        .legs(vec![fee_leg, protection_leg])
        .trade_type(trade_type)
        .metadata(metadata)
        .build())
}

/// Parse a credit default swap index from FpML.
pub fn parse_credit_default_swap_index(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    let header = parse_trade_header(xml)?;

    let cds_section = nav
        .extract_section("creditDefaultSwap")
        .ok_or_else(|| FpmlError::MissingElement("creditDefaultSwap".to_string()))?;

    let cds_nav = XmlNavigator::new(&cds_section);

    let index_name = xml_text!(cds_nav, "indexName", "UNKNOWN INDEX");

    let series: u32 = cds_nav
        .find_text("indexSeries")
        .map(|s| s.parse().unwrap_or(1))
        .unwrap_or(1);

    let version: Option<u32> = cds_nav
        .find_text("indexAnnexVersion")
        .and_then(|v| v.parse().ok());

    let protection_side = if cds_nav.extract_section("buyerPartyReference").is_some() {
        ProtectionSide::Buyer
    } else {
        ProtectionSide::Seller
    };

    let effective_date = extract_nested_date(
        &cds_nav,
        "effectiveDate",
        "unadjustedDate",
        Date::from_ymd(2024, 1, 1).unwrap(),
    )?;

    let termination_date = cds_nav
        .extract_section("scheduledTerminationDate")
        .and_then(|section| XmlNavigator::new(&section).find_text("unadjustedDate"))
        .map(|d| parse_date(&d))
        .transpose()?
        .unwrap_or_else(|| Date::from_ymd(2029, 1, 1).unwrap());

    let notional = extract_nested_amount(&cds_nav, "calculationAmount", "amount", 0.0)?;

    let currency = parse_currency(&xml_text!(cds_nav, "currency", "USD"));

    let fixed_rate = xml_decimal!(cds_nav, "fixedRate", 0.01);

    let fee_cf = Cashflow::new(
        CashflowType::Coupon,
        termination_date,
        effective_date,
        termination_date,
        1.0,
        notional,
        Payoff::fixed(fixed_rate),
        currency,
    );

    let fee_leg = Leg::new(
        vec![fee_cf],
        if protection_side == ProtectionSide::Buyer {
            Direction::Payer
        } else {
            Direction::Receiver
        },
        LegType::Fixed,
        currency,
    );

    let protection_cf = Cashflow::new(
        CashflowType::Settlement,
        termination_date,
        effective_date,
        termination_date,
        0.0,
        notional,
        Payoff::fixed(1.0),
        currency,
    );

    let protection_leg = Leg::new(
        vec![protection_cf],
        if protection_side == ProtectionSide::Buyer {
            Direction::Receiver
        } else {
            Direction::Payer
        },
        LegType::Generic,
        currency,
    );

    let trade_type = TradeType::CreditDefaultSwapIndex {
        index_name,
        series,
        version,
        protection_side,
    };

    let metadata = build_metadata(&header);

    Ok(Trade::builder()
        .id(header.trade_id)
        .legs(vec![fee_leg, protection_leg])
        .trade_type(trade_type)
        .metadata(metadata)
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CDS_XML: &str = r#"
        <trade>
            <tradeHeader>
                <tradeId>CDS-IBM-001</tradeId>
                <tradeDate>2024-06-20</tradeDate>
            </tradeHeader>
            <creditDefaultSwap>
                <generalTerms>
                    <effectiveDate>
                        <unadjustedDate>2024-06-21</unadjustedDate>
                    </effectiveDate>
                    <scheduledTerminationDate>
                        <unadjustedDate>2029-06-20</unadjustedDate>
                    </scheduledTerminationDate>
                    <buyerPartyReference href="EB_NA"/>
                    <sellerPartyReference href="GOLDMAN"/>
                    <referenceInformation>
                        <referenceEntity>
                            <entityName>International Business Machines Corporation</entityName>
                            <entityId>4B8BPH</entityId>
                        </referenceEntity>
                    </referenceInformation>
                </generalTerms>
                <feeLeg>
                    <periodicPayment>
                        <fixedAmountCalculation>
                            <calculationAmount>
                                <currency>USD</currency>
                                <amount>10000000</amount>
                            </calculationAmount>
                            <fixedRate>0.0065</fixedRate>
                        </fixedAmountCalculation>
                    </periodicPayment>
                </feeLeg>
                <protectionTerms>
                    <calculationAmount>
                        <currency>USD</currency>
                        <amount>10000000</amount>
                    </calculationAmount>
                </protectionTerms>
            </creditDefaultSwap>
        </trade>
    "#;

    #[test]
    fn test_parse_credit_default_swap() {
        let trade = parse_credit_default_swap(SAMPLE_CDS_XML).unwrap();

        assert_eq!(trade.id.as_str(), "CDS-IBM-001");
        assert!(trade.trade_type.is_credit());
        assert_eq!(trade.num_legs(), 2);
    }
}
