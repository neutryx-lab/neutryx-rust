//! Credit product parsers.
//!
//! Handles parsing for:
//! - Credit Default Swap (creditDefaultSwap)
//! - Credit Default Swap Index (CDX, iTraxx)

use infra_domain::{
    time::Date,
    trade::{
        Cashflow, CashflowType, Direction, Leg, LegType, Payoff, ProtectionSide, Trade,
        TradeMetadata, TradeType,
    },
};

use crate::fpml::{
    common::{
        parse_currency, parse_date, parse_decimal, parse_trade_header, xml_decimal,
        xml_text, XmlNavigator,
    },
    error::FpmlError,
};

/// Build trade metadata from header.
fn build_metadata(header: &crate::fpml::common::TradeHeader) -> TradeMetadata {
    let mut metadata = TradeMetadata::new();
    if let Some(td) = header.trade_date {
        metadata = metadata.with_trade_date(td);
    }
    if let Some(ref cp) = header.counterparty {
        metadata = metadata.with_counterparty(cp.clone());
    }
    if let Some(ref book) = header.book {
        metadata = metadata.with_book(book.clone());
    }
    metadata
}

/// Extract a date from a nested section like
/// `<effectiveDate><unadjustedDate>...`.
fn extract_nested_date(
    nav: &XmlNavigator,
    section_name: &str,
    fallback_elem: &str,
    default: Date,
) -> Result<Date, FpmlError> {
    nav.extract_section(section_name)
        .and_then(|section| XmlNavigator::new(&section).find_text("unadjustedDate"))
        .or_else(|| nav.find_text(fallback_elem))
        .map(|d| parse_date(&d))
        .transpose()
        .map(|opt| opt.unwrap_or(default))
}

/// Extract notional from a nested section like
/// `<calculationAmount><amount>...`.
fn extract_nested_amount(
    nav: &XmlNavigator,
    section_name: &str,
    fallback_elem: &str,
    default: f64,
) -> Result<f64, FpmlError> {
    nav.extract_section(section_name)
        .and_then(|section| XmlNavigator::new(&section).find_text("amount"))
        .or_else(|| nav.find_text(fallback_elem))
        .map(|n| parse_decimal(&n))
        .transpose()
        .map(|opt| opt.unwrap_or(default))
}

/// Parse a credit default swap from FpML.
pub fn parse_credit_default_swap(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract creditDefaultSwap section
    let cds_section = nav
        .extract_section("creditDefaultSwap")
        .ok_or_else(|| FpmlError::MissingElement("creditDefaultSwap".to_string()))?;

    let cds_nav = XmlNavigator::new(&cds_section);

    // Parse reference entity
    let reference_entity = xml_text!(cds_nav, "entityName", "UNKNOWN");

    // Parse entity ID (RED code)
    let entity_id = cds_nav.find_text("entityId");

    // Determine protection side
    let protection_side = if cds_nav.extract_section("buyerPartyReference").is_some() {
        ProtectionSide::Buyer
    } else {
        ProtectionSide::Seller
    };

    // Parse dates
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

    // Parse notional
    let notional = extract_nested_amount(&cds_nav, "calculationAmount", "amount", 0.0)?;

    // Parse currency
    let currency = parse_currency(&xml_text!(cds_nav, "currency", "USD"));

    // Parse fixed rate (premium)
    let fixed_rate = xml_decimal!(cds_nav, "fixedRate", 0.01);

    // Create fee leg (premium payments)
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

    // Create protection leg (contingent payment)
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

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract creditDefaultSwap section (same element for index CDS)
    let cds_section = nav
        .extract_section("creditDefaultSwap")
        .ok_or_else(|| FpmlError::MissingElement("creditDefaultSwap".to_string()))?;

    let cds_nav = XmlNavigator::new(&cds_section);

    // Parse index reference information
    let index_name = xml_text!(cds_nav, "indexName", "UNKNOWN INDEX");

    let series: u32 = cds_nav
        .find_text("indexSeries")
        .map(|s| s.parse().unwrap_or(1))
        .unwrap_or(1);

    let version: Option<u32> = cds_nav
        .find_text("indexAnnexVersion")
        .and_then(|v| v.parse().ok());

    // Determine protection side
    let protection_side = if cds_nav.extract_section("buyerPartyReference").is_some() {
        ProtectionSide::Buyer
    } else {
        ProtectionSide::Seller
    };

    // Parse dates
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

    // Parse notional
    let notional = extract_nested_amount(&cds_nav, "calculationAmount", "amount", 0.0)?;

    let currency = parse_currency(&xml_text!(cds_nav, "currency", "USD"));

    // Parse fixed rate
    let fixed_rate = xml_decimal!(cds_nav, "fixedRate", 0.01);

    // Create fee leg
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

    // Create protection leg
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
                    <buyerPartyReference href="FB_NA"/>
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
