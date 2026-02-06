//! Interest rate product parsers.
//!
//! Handles parsing for:
//! - Interest Rate Swaps (IRS)
//! - Overnight Index Swaps (OIS)
//! - Swaptions
//! - Cap/Floor

use crate::common::{parse_date, parse_decimal, parse_trade_header, XmlNavigator};
use crate::error::FpmlError;
use infra_domain::{
    trade::{
        Cashflow, CashflowType, Direction, ExerciseType, Leg, LegType, Payoff, SettlementType,
        Trade, TradeMetadata, TradeType,
    },
    Currency, Date, RateIndex,
};

/// Parse an interest rate swap from FpML.
pub fn parse_swap(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract swap section
    let swap_section = nav
        .extract_section("swap")
        .ok_or_else(|| FpmlError::MissingElement("swap".to_string()))?;

    let swap_nav = XmlNavigator::new(&swap_section);

    // Parse swap streams (legs)
    let mut legs = Vec::new();

    for stream_xml in swap_nav.extract_all_sections("swapStream") {
        let leg = parse_swap_stream(&stream_xml)?;
        legs.push(leg);
    }

    // Determine if this is OIS or regular swap based on floating rate index
    let trade_type = determine_swap_type(&legs);

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
        .trade_type(trade_type)
        .metadata(metadata)
        .build())
}

/// Parse a single swap stream (leg).
fn parse_swap_stream(xml: &str) -> Result<Leg, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Determine if this is fixed or floating
    let is_fixed = nav.extract_section("fixedRateSchedule").is_some();

    // Get direction from payer/receiver party refs
    // Note: In a full implementation, we'd resolve the party refs to determine direction
    // For now, we'll use a heuristic: fixed leg is receiver, floating is payer
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

    // Parse calculation period dates
    let calc_section = nav
        .extract_section("calculationPeriodDates")
        .unwrap_or_default();
    let calc_nav = XmlNavigator::new(&calc_section);

    let effective_date = calc_nav
        .find_text("unadjustedDate")
        .map(|d| parse_date(&d))
        .transpose()?
        .unwrap_or_else(|| Date::from_ymd(2024, 1, 1).unwrap());

    // Parse notional
    let notional = nav
        .find_text("initialValue")
        .or_else(|| nav.find_text("notionalAmount"))
        .map(|n| parse_decimal(&n))
        .transpose()?
        .unwrap_or(0.0);

    // Parse currency
    let currency_str = nav.find_text("currency").unwrap_or_else(|| "USD".to_string());
    let currency = parse_currency(&currency_str);

    // Parse fixed rate or floating index
    let payoff = if is_fixed {
        let rate = nav
            .find_text("initialValue")
            .or_else(|| {
                // Look in fixedRateSchedule section
                let schedule = nav.extract_section("fixedRateSchedule")?;
                XmlNavigator::new(&schedule).find_text("initialValue")
            })
            .map(|r| parse_decimal(&r))
            .transpose()?
            .unwrap_or(0.0);
        Payoff::fixed(rate)
    } else {
        let index = parse_floating_rate_index(&nav)?;
        let spread = nav
            .find_text("spread")
            .map(|s| parse_decimal(&s))
            .transpose()?
            .unwrap_or(0.0);
        Payoff::floating_with_spread(index.into(), spread)
    };

    // Parse day count fraction
    let _dcf = nav
        .find_text("dayCountFraction")
        .unwrap_or_else(|| "ACT/360".to_string());

    // Create a simplified single cashflow for now
    // A full implementation would generate the full schedule
    let cashflows = vec![Cashflow::new(
        CashflowType::Coupon,
        effective_date,
        effective_date,
        effective_date,
        0.5, // Simplified year fraction
        notional,
        payoff,
        currency,
    )];

    Ok(Leg::new(cashflows, direction, leg_type, currency))
}

/// Parse floating rate index from FpML.
fn parse_floating_rate_index(nav: &XmlNavigator) -> Result<RateIndex, FpmlError> {
    // Look for floatingRateIndex element
    let index_name = nav
        .find_text("floatingRateIndex")
        .unwrap_or_else(|| "USD-SOFR".to_string());

    // Map FpML index names to internal RateIndex
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
        s if s.contains("TIBOR") => RateIndex::Tonar, // Map TIBOR to TONAR (JPY)
        _ => RateIndex::Sofr, // Default fallback
    };

    Ok(index)
}

/// Determine swap type based on leg characteristics.
fn determine_swap_type(legs: &[Leg]) -> TradeType {
    // Check if any leg references an overnight index
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

/// Parse currency string to Currency enum.
fn parse_currency(s: &str) -> Currency {
    match s.to_uppercase().as_str() {
        "USD" => Currency::USD,
        "EUR" => Currency::EUR,
        "GBP" => Currency::GBP,
        "JPY" => Currency::JPY,
        "CHF" => Currency::CHF,
        _ => Currency::USD, // Default fallback
    }
}

/// Parse a swaption from FpML.
pub fn parse_swaption(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract swaption section
    let swaption_section = nav
        .extract_section("swaption")
        .ok_or_else(|| FpmlError::MissingElement("swaption".to_string()))?;

    let swaption_nav = XmlNavigator::new(&swaption_section);

    // Parse exercise type
    let exercise_type = if swaption_nav.extract_section("europeanExercise").is_some() {
        ExerciseType::European
    } else if swaption_nav.extract_section("americanExercise").is_some() {
        ExerciseType::American
    } else if swaption_nav.extract_section("bermudaExercise").is_some() {
        ExerciseType::Bermudan
    } else {
        ExerciseType::European // Default
    };

    // Parse exercise dates (look inside expirationDate for unadjustedDate)
    let mut exercise_dates = Vec::new();
    if let Some(expiry_section) = swaption_nav.extract_section("expirationDate") {
        let expiry_nav = XmlNavigator::new(&expiry_section);
        if let Some(date_str) = expiry_nav.find_text("unadjustedDate") {
            if let Ok(date) = parse_date(&date_str) {
                exercise_dates.push(date);
            }
        }
    }

    // Parse settlement type
    let settlement_type = if swaption_nav.extract_section("cashSettlement").is_some() {
        SettlementType::Cash
    } else {
        SettlementType::Physical
    };

    // Parse the underlying swap
    let underlying_swap = if let Some(swap_section) = swaption_nav.extract_section("swap") {
        parse_swap(&format!(
            "<trade><tradeHeader><tradeId>{}</tradeId></tradeHeader>{}</trade>",
            header.trade_id, swap_section
        ))?
    } else {
        return Err(FpmlError::MissingElement("underlying swap".to_string()));
    };

    let trade_type = TradeType::Swaption {
        exercise_dates,
        exercise_type,
        settlement_type,
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

    // Get legs from underlying swap
    let legs: Vec<Leg> = underlying_swap.legs().cloned().collect();

    Ok(Trade::builder()
        .id(header.trade_id)
        .legs(legs)
        .trade_type(trade_type)
        .metadata(metadata)
        .build())
}

/// Parse a cap/floor from FpML.
pub fn parse_cap_floor(xml: &str) -> Result<Trade, FpmlError> {
    let nav = XmlNavigator::new(xml);

    // Parse trade header (includes counterparty resolution)
    let header = parse_trade_header(xml)?;

    // Extract capFloor section
    let capfloor_section = nav
        .extract_section("capFloor")
        .ok_or_else(|| FpmlError::MissingElement("capFloor".to_string()))?;

    let cf_nav = XmlNavigator::new(&capfloor_section);

    // Parse notional
    let notional = cf_nav
        .find_text("notionalStepAmount")
        .or_else(|| cf_nav.find_text("initialValue"))
        .map(|n| parse_decimal(&n))
        .transpose()?
        .unwrap_or(0.0);

    // Parse currency
    let currency_str = cf_nav.find_text("currency").unwrap_or_else(|| "USD".to_string());
    let currency = parse_currency(&currency_str);

    // Parse strike (cap or floor rate)
    let strike = cf_nav
        .find_text("capRate")
        .or_else(|| cf_nav.find_text("floorRate"))
        .map(|s| parse_decimal(&s))
        .transpose()?
        .unwrap_or(0.0);

    // Determine if cap or floor
    let is_cap = cf_nav.find_text("capRate").is_some();

    // Parse floating rate index
    let index = parse_floating_rate_index(&cf_nav)?;

    // Parse effective and termination dates
    let effective_date = cf_nav
        .find_text("unadjustedDate")
        .map(|d| parse_date(&d))
        .transpose()?
        .unwrap_or_else(|| Date::from_ymd(2024, 1, 1).unwrap());

    // Create payoff
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
        0.25, // Quarterly
        notional,
        payoff,
        currency,
    )];

    let leg = Leg::new(cashflows, Direction::Receiver, LegType::CapFloor, currency);

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
        .trade_type(TradeType::CapFloor)
        .metadata(metadata)
        .build())
}

#[cfg(test)]
mod tests {
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
        assert_eq!(parse_currency("usd"), Currency::USD); // Case insensitive
    }
}
