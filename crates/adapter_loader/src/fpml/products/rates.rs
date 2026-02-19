//! Interest rate product parsers.

use infra_domain::{
    market::RateIndex,
    time::Date,
    trade::{
        Cashflow, CashflowType, Direction, EventLeg, ExerciseEvent, ExerciseType, Leg, LegType,
        Payoff, SettlementType, Trade, TradeType,
    },
};

use crate::fpml::{
    common::{
        build_metadata, parse_currency, parse_date, parse_decimal, parse_trade_header, xml_date,
        xml_decimal, xml_decimal_or, xml_text, XmlNavigator,
    },
    error::FpmlError,
};

/// Parse an interest rate swap from FpML.
pub fn parse_swap(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    let header = parse_trade_header(xml)?;

    let swap_section = nav
        .extract_section("swap")
        .ok_or_else(|| FpmlError::MissingElement("swap".to_string()))?;

    let swap_nav = XmlNavigator::new(&swap_section);

    let mut legs = Vec::new();

    for stream_xml in swap_nav.extract_all_sections("swapStream") {
        let leg = parse_swap_stream(&stream_xml)?;
        legs.push(leg);
    }

    let trade_type = determine_swap_type(&legs);

    let metadata = build_metadata(&header);

    Ok(Trade::builder()
        .id(header.trade_id)
        .legs(legs)
        .trade_type(trade_type)
        .metadata(metadata)
        .build())
}

/// Parse a single swap stream (leg).
fn parse_swap_stream(xml: &str) -> Result<Leg, FpmlError> {
    let nav = XmlNavigator::new(xml);

    let is_fixed = nav.extract_section("fixedRateSchedule").is_some();

    let direction = if is_fixed {
        Direction::Receiver
    } else {
        Direction::Payer
    };

    let leg_type = if is_fixed {
        LegType::Fixed
    } else {
        LegType::Floating
    };

    let calc_section = nav
        .extract_section("calculationPeriodDates")
        .unwrap_or_default();
    let calc_nav = XmlNavigator::new(&calc_section);

    let effective_date = xml_date!(
        calc_nav,
        "unadjustedDate",
        Date::from_ymd(2024, 1, 1).unwrap()
    );

    let notional = xml_decimal_or!(nav, "initialValue", "notionalAmount"; 0.0);

    let currency = parse_currency(&xml_text!(nav, "currency", "USD"));

    let payoff = if is_fixed {
        let rate = nav
            .find_text("initialValue")
            .or_else(|| {
                let schedule = nav.extract_section("fixedRateSchedule")?;
                XmlNavigator::new(&schedule).find_text("initialValue")
            })
            .map(|r| parse_decimal(&r))
            .transpose()?
            .unwrap_or(0.0);
        Payoff::fixed(rate)
    } else {
        let index = parse_floating_rate_index(&nav)?;
        let spread = xml_decimal!(nav, "spread", 0.0);
        Payoff::floating_with_spread(index.into(), spread)
    };

    let _dcf = xml_text!(nav, "dayCountFraction", "ACT/360");

    let cashflows = vec![Cashflow::new(
        CashflowType::Coupon,
        effective_date,
        effective_date,
        effective_date,
        0.5,
        notional,
        payoff,
        currency,
    )];

    Ok(Leg::new(cashflows, direction, leg_type, currency))
}

/// Parse floating rate index from FpML.
#[allow(clippy::unnecessary_wraps)]
fn parse_floating_rate_index(nav: &XmlNavigator) -> Result<RateIndex, FpmlError> {
    let index_name = xml_text!(nav, "floatingRateIndex", "USD-SOFR");

    let index = match index_name.to_uppercase().as_str() {
        s if s.contains("SOFR") => RateIndex::Sofr,
        s if s.contains("SONIA") => RateIndex::Sonia,
        s if s.contains("ESTR") || s.contains("€STR") => RateIndex::Estr,
        s if s.contains("TONAR") || s.contains("TONA") => RateIndex::Tonar,
        s if s.contains("EURIBOR") => {
            if s.contains("6M") {
                RateIndex::Euribor6M
            } else {
                RateIndex::Euribor3M
            }
        }
        s if s.contains("TIBOR") => RateIndex::Tonar,
        _ => RateIndex::Sofr,
    };

    Ok(index)
}

/// Determine swap type based on leg characteristics.
fn determine_swap_type(legs: &[Leg]) -> TradeType {
    let has_overnight = legs.iter().any(|leg| {
        leg.cashflows().any(|cf| {
            if let Some(index) = cf.payoff.required_index() {
                if let Some(rate) = index.as_rate() {
                    return rate.is_overnight();
                }
            }
            false
        })
    });

    if has_overnight {
        TradeType::Ois
    } else {
        TradeType::Swap
    }
}

/// Parse a swaption from FpML.
pub fn parse_swaption(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    let header = parse_trade_header(xml)?;

    let swaption_section = nav
        .extract_section("swaption")
        .ok_or_else(|| FpmlError::MissingElement("swaption".to_string()))?;

    let swaption_nav = XmlNavigator::new(&swaption_section);

    let exercise_type = if swaption_nav.extract_section("europeanExercise").is_some() {
        ExerciseType::European
    } else if swaption_nav.extract_section("americanExercise").is_some() {
        ExerciseType::American
    } else if swaption_nav.extract_section("bermudaExercise").is_some() {
        ExerciseType::Bermudan
    } else {
        ExerciseType::European
    };

    let mut exercise_dates = Vec::new();
    if let Some(expiry_section) = swaption_nav.extract_section("expirationDate") {
        let expiry_nav = XmlNavigator::new(&expiry_section);
        if let Some(date_str) = expiry_nav.find_text("unadjustedDate") {
            if let Ok(date) = parse_date(&date_str) {
                exercise_dates.push(date);
            }
        }
    }

    let settlement_type = if swaption_nav.extract_section("cashSettlement").is_some() {
        SettlementType::Cash
    } else {
        SettlementType::Physical
    };

    let underlying_swap = if let Some(swap_section) = swaption_nav.extract_section("swap") {
        parse_swap(&format!(
            "<trade><tradeHeader><tradeId>{}</tradeId></tradeHeader>{}</trade>",
            header.trade_id, swap_section
        ))?
    } else {
        return Err(FpmlError::MissingElement("underlying swap".to_string()));
    };

    let metadata = build_metadata(&header);

    let underlying_legs: Vec<Leg> = underlying_swap.legs().cloned().collect();
    let exercise = ExerciseEvent {
        exercise_dates,
        exercise_type,
        settlement_type,
    };
    let event_leg = EventLeg::new(exercise, underlying_legs);

    Ok(Trade::builder()
        .id(header.trade_id)
        .event_legs(vec![event_leg])
        .trade_type(TradeType::Swaption)
        .metadata(metadata)
        .build())
}

/// Parse a cap/floor from FpML.
pub fn parse_cap_floor(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    let header = parse_trade_header(xml)?;

    let capfloor_section = nav
        .extract_section("capFloor")
        .ok_or_else(|| FpmlError::MissingElement("capFloor".to_string()))?;

    let cf_nav = XmlNavigator::new(&capfloor_section);

    let notional = xml_decimal_or!(cf_nav, "notionalStepAmount", "initialValue"; 0.0);

    let currency = parse_currency(&xml_text!(cf_nav, "currency", "USD"));

    let strike = xml_decimal_or!(cf_nav, "capRate", "floorRate"; 0.0);

    let is_cap = cf_nav.find_text("capRate").is_some();

    let index = parse_floating_rate_index(&cf_nav)?;

    let effective_date = xml_date!(
        cf_nav,
        "unadjustedDate",
        Date::from_ymd(2024, 1, 1).unwrap()
    );

    let payoff = if is_cap {
        Payoff::cap(index.into(), strike)
    } else {
        Payoff::floor(index.into(), strike)
    };

    let cashflows = vec![Cashflow::new(
        CashflowType::Coupon,
        effective_date,
        effective_date,
        effective_date,
        0.25,
        notional,
        payoff,
        currency,
    )];

    let leg = Leg::new(cashflows, Direction::Receiver, LegType::CapFloor, currency);

    let metadata = build_metadata(&header);

    Ok(Trade::builder()
        .id(header.trade_id)
        .legs(vec![leg])
        .trade_type(TradeType::CapFloor)
        .metadata(metadata)
        .build())
}

#[cfg(test)]
mod tests {
    use infra_domain::market::Currency;

    use super::*;

    const SAMPLE_SWAP_XML: &str = r#"
        <trade>
            <tradeHeader>
                <tradeId>IRS-USD-001</tradeId>
                <tradeDate>2024-01-15</tradeDate>
            </tradeHeader>
            <swap>
                <swapStream id="fixedLeg">
                    <calculationPeriodDates>
                        <effectiveDate>
                            <unadjustedDate>2024-01-17</unadjustedDate>
                        </effectiveDate>
                    </calculationPeriodDates>
                    <calculationPeriodAmount>
                        <calculation>
                            <notionalSchedule>
                                <notionalStepSchedule>
                                    <initialValue>50000000</initialValue>
                                    <currency>USD</currency>
                                </notionalStepSchedule>
                            </notionalSchedule>
                            <fixedRateSchedule>
                                <initialValue>0.0425</initialValue>
                            </fixedRateSchedule>
                            <dayCountFraction>ACT/360</dayCountFraction>
                        </calculation>
                    </calculationPeriodAmount>
                </swapStream>
                <swapStream id="floatLeg">
                    <calculationPeriodDates>
                        <effectiveDate>
                            <unadjustedDate>2024-01-17</unadjustedDate>
                        </effectiveDate>
                    </calculationPeriodDates>
                    <calculationPeriodAmount>
                        <calculation>
                            <notionalSchedule>
                                <notionalStepSchedule>
                                    <initialValue>50000000</initialValue>
                                    <currency>USD</currency>
                                </notionalStepSchedule>
                            </notionalSchedule>
                            <floatingRateCalculation>
                                <floatingRateIndex>USD-SOFR-COMPOUND</floatingRateIndex>
                            </floatingRateCalculation>
                            <dayCountFraction>ACT/360</dayCountFraction>
                        </calculation>
                    </calculationPeriodAmount>
                </swapStream>
            </swap>
        </trade>
    "#;

    #[test]
    fn test_parse_swap() {
        let trade = parse_swap(SAMPLE_SWAP_XML).unwrap();

        assert_eq!(trade.id.as_str(), "IRS-USD-001");
        assert_eq!(trade.num_legs(), 2);
        assert!(trade.trade_type.is_swap());
    }

    #[test]
    fn test_parse_currency() {
        assert_eq!(parse_currency("USD"), Currency::USD);
        assert_eq!(parse_currency("EUR"), Currency::EUR);
        assert_eq!(parse_currency("GBP"), Currency::GBP);
        assert_eq!(parse_currency("usd"), Currency::USD);
    }
}
