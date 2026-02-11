//! Commodity product parsers.
//!
//! Handles parsing for:
//! - Commodity Swap (commoditySwap)
//! - Commodity Option (commodityOption)
//! - Commodity Forward (commodityForward)

use infra_domain::{
    time::Date,
    trade::{
        Cashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade, TradeMetadata, TradeType,
    },
};

use crate::fpml::{
    common::{parse_currency, parse_trade_header, xml_date, xml_decimal, xml_text, XmlNavigator},
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

/// Parse a commodity swap from FpML.
pub fn parse_commodity_swap(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract commoditySwap section
    let swap_section = nav
        .extract_section("commoditySwap")
        .ok_or_else(|| FpmlError::MissingElement("commoditySwap".to_string()))?;

    let swap_nav = XmlNavigator::new(&swap_section);

    // Parse effective date
    let effective_date = xml_date!(
        swap_nav,
        "unadjustedDate",
        Date::from_ymd(2024, 1, 1).unwrap()
    );

    // Parse fixed leg
    let fixed_section = swap_nav.extract_section("fixedLeg").unwrap_or_default();
    let fixed_nav = XmlNavigator::new(&fixed_section);

    let fixed_price = xml_decimal!(fixed_nav, "price", 0.0);
    let price_currency = xml_text!(fixed_nav, "priceCurrency", "USD");
    let price_unit = xml_text!(fixed_nav, "priceUnit", "BBL");
    let quantity = xml_decimal!(fixed_nav, "quantity", 0.0);
    let quantity_unit = xml_text!(fixed_nav, "quantityUnit", "BBL");
    let total_quantity = xml_decimal!(fixed_nav, "totalNotionalQuantity", quantity);

    // Parse floating leg for commodity reference
    let floating_section = swap_nav.extract_section("floatingLeg").unwrap_or_default();
    let floating_nav = XmlNavigator::new(&floating_section);

    let commodity = floating_nav
        .find_text("instrumentId")
        .or_else(|| swap_nav.find_text("commodity"))
        .unwrap_or_else(|| "OIL-BRENT".to_string());

    let currency = parse_currency(&price_currency);

    // Create fixed leg
    let fixed_notional = fixed_price * total_quantity;
    let fixed_cf = Cashflow::new(
        CashflowType::Coupon,
        effective_date,
        effective_date,
        effective_date,
        1.0,
        fixed_notional,
        Payoff::fixed(fixed_price),
        currency,
    );

    let fixed_leg = Leg::new(vec![fixed_cf], Direction::Payer, LegType::Fixed, currency);

    // Create floating leg
    let floating_cf = Cashflow::new(
        CashflowType::Coupon,
        effective_date,
        effective_date,
        effective_date,
        1.0,
        total_quantity,
        Payoff::fixed(1.0), // Commodity index reference
        currency,
    );

    let floating_leg = Leg::new(
        vec![floating_cf],
        Direction::Receiver,
        LegType::Floating,
        currency,
    );

    let trade_type = TradeType::CommoditySwap {
        commodity,
        fixed_price,
        price_unit,
        total_quantity,
        quantity_unit,
    };

    let metadata = build_metadata(&header);

    Ok(Trade::builder()
        .id(header.trade_id)
        .legs(vec![fixed_leg, floating_leg])
        .trade_type(trade_type)
        .metadata(metadata)
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_COMMODITY_SWAP_XML: &str = r#"
        <trade>
            <tradeHeader>
                <tradeId>COMSWAP-BRENT-001</tradeId>
                <tradeDate>2024-07-01</tradeDate>
            </tradeHeader>
            <commoditySwap>
                <effectiveDate>
                    <adjustableDate>
                        <unadjustedDate>2024-08-01</unadjustedDate>
                    </adjustableDate>
                </effectiveDate>
                <terminationDate>
                    <adjustableDate>
                        <unadjustedDate>2025-07-31</unadjustedDate>
                    </adjustableDate>
                </terminationDate>
                <fixedLeg>
                    <fixedPrice>
                        <price>80.00</price>
                        <priceCurrency>USD</priceCurrency>
                        <priceUnit>BBL</priceUnit>
                    </fixedPrice>
                    <notionalQuantity>
                        <quantityUnit>BBL</quantityUnit>
                        <quantity>25000</quantity>
                    </notionalQuantity>
                    <totalNotionalQuantity>300000</totalNotionalQuantity>
                </fixedLeg>
                <floatingLeg>
                    <commodity>
                        <instrumentId>OIL-BRENT-IPE</instrumentId>
                    </commodity>
                </floatingLeg>
            </commoditySwap>
        </trade>
    "#;

    #[test]
    fn test_parse_commodity_swap() {
        let trade = parse_commodity_swap(SAMPLE_COMMODITY_SWAP_XML).unwrap();

        assert_eq!(trade.id.as_str(), "COMSWAP-BRENT-001");
        assert!(trade.trade_type.is_commodity());
        assert_eq!(trade.num_legs(), 2);
    }
}
