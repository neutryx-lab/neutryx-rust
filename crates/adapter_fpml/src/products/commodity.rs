//! Commodity product parsers.
//!
//! Handles parsing for:
//! - Commodity Swap (commoditySwap)
//! - Commodity Option (commodityOption)
//! - Commodity Forward (commodityForward)

use crate::common::{parse_date, parse_decimal, parse_trade_header, XmlNavigator};
use crate::error::FpmlError;
use infra_master::{
    trade::{
        Cashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade, TradeMetadata, TradeType,
    },
    Currency, Date,
};

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

    // Parse effective and termination dates
    let effective_date = swap_nav
        .find_text("unadjustedDate")
        .map(|d| parse_date(&d))
        .transpose()?
        .unwrap_or_else(|| Date::from_ymd(2024, 1, 1).unwrap());

    // Parse fixed leg
    let fixed_section = swap_nav.extract_section("fixedLeg").unwrap_or_default();
    let fixed_nav = XmlNavigator::new(&fixed_section);

    let fixed_price = fixed_nav
        .find_text("price")
        .map(|p| parse_decimal(&p))
        .transpose()?
        .unwrap_or(0.0);

    let price_currency = fixed_nav
        .find_text("priceCurrency")
        .unwrap_or_else(|| "USD".to_string());

    let price_unit = fixed_nav
        .find_text("priceUnit")
        .unwrap_or_else(|| "BBL".to_string());

    let quantity = fixed_nav
        .find_text("quantity")
        .map(|q| parse_decimal(&q))
        .transpose()?
        .unwrap_or(0.0);

    let quantity_unit = fixed_nav
        .find_text("quantityUnit")
        .unwrap_or_else(|| "BBL".to_string());

    let total_quantity = fixed_nav
        .find_text("totalNotionalQuantity")
        .map(|q| parse_decimal(&q))
        .transpose()?
        .unwrap_or(quantity);

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
        .legs(vec![fixed_leg, floating_leg])
        .trade_type(trade_type)
        .metadata(metadata)
        .build())
}

/// Parse a commodity forward from FpML.
pub fn parse_commodity_forward(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract commodityForward section
    let fwd_section = nav
        .extract_section("commodityForward")
        .ok_or_else(|| FpmlError::MissingElement("commodityForward".to_string()))?;

    let fwd_nav = XmlNavigator::new(&fwd_section);

    // Parse commodity reference
    let commodity = fwd_nav
        .find_text("instrumentId")
        .or_else(|| fwd_nav.find_text("commodity"))
        .unwrap_or_else(|| "UNKNOWN".to_string());

    // Parse delivery date
    let delivery_date = fwd_nav
        .find_text("deliveryDate")
        .map(|d| parse_date(&d))
        .transpose()?
        .unwrap_or_else(|| Date::from_ymd(2025, 1, 1).unwrap());

    // Parse forward price
    let forward_price = fwd_nav
        .find_text("forwardPrice")
        .or_else(|| fwd_nav.find_text("price"))
        .map(|p| parse_decimal(&p))
        .transpose()?
        .unwrap_or(0.0);

    // Parse quantity
    let quantity = fwd_nav
        .find_text("quantity")
        .map(|q| parse_decimal(&q))
        .transpose()?
        .unwrap_or(0.0);

    let quantity_unit = fwd_nav
        .find_text("quantityUnit")
        .unwrap_or_else(|| "BBL".to_string());

    let currency_str = fwd_nav
        .find_text("currency")
        .or_else(|| fwd_nav.find_text("priceCurrency"))
        .unwrap_or_else(|| "USD".to_string());
    let currency = parse_currency(&currency_str);

    let notional = forward_price * quantity;
    let cf = Cashflow::new(
        CashflowType::Settlement,
        delivery_date,
        delivery_date,
        delivery_date,
        0.0,
        notional,
        Payoff::fixed(1.0),
        currency,
    );

    let leg = Leg::new(vec![cf], Direction::Receiver, LegType::Generic, currency);

    let trade_type = TradeType::CommodityForward {
        commodity,
        delivery_date,
        forward_price,
        quantity,
        quantity_unit,
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
