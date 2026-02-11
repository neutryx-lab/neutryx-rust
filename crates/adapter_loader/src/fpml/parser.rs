//! FpML parser implementation.

use infra_domain::trade::Trade;

use super::{common::XmlNavigator, error::FpmlError, products};

/// FpML parser for trade definitions.
pub struct FpmlParser;

impl FpmlParser {
    /// Parse an FpML XML document into a Trade.
    pub fn parse(xml: &str) -> Result<Trade, FpmlError> {
        let product_type = Self::detect_product_type(xml)?;

        match product_type {
            ProductType::Swap => products::parse_swap(xml),
            ProductType::Swaption => products::parse_swaption(xml),
            ProductType::CapFloor => products::parse_cap_floor(xml),

            ProductType::FxForward => products::parse_fx_forward(xml),
            ProductType::FxSwap => products::parse_fx_swap(xml),
            ProductType::FxOption => products::parse_fx_option(xml),

            ProductType::EquityOption => products::parse_equity_option(xml),

            ProductType::CreditDefaultSwap => {
                if Self::is_index_cds(xml) {
                    products::parse_credit_default_swap_index(xml)
                } else {
                    products::parse_credit_default_swap(xml)
                }
            }

            ProductType::CommoditySwap => products::parse_commodity_swap(xml),
        }
    }

    /// Parse multiple FpML documents from a single XML file.
    pub fn parse_multiple(xml: &str) -> Result<Vec<Trade>, FpmlError> {
        let nav = XmlNavigator::new(xml);
        let trade_sections = nav.extract_all_sections("trade");

        if trade_sections.is_empty() {
            return Ok(vec![Self::parse(xml)?]);
        }

        let mut trades = Vec::with_capacity(trade_sections.len());
        for trade_xml in trade_sections {
            let full_xml = format!(
                r#"<?xml version="1.0"?><dataDocument>{}</dataDocument>"#,
                trade_xml
            );
            trades.push(Self::parse(&full_xml)?);
        }

        Ok(trades)
    }

    /// Detect the product type from FpML XML.
    fn detect_product_type(xml: &str) -> Result<ProductType, FpmlError> {
        if xml.contains("<swaption>") || xml.contains("<swaption ") {
            return Ok(ProductType::Swaption);
        }
        if xml.contains("<swap>") || xml.contains("<swap ") {
            return Ok(ProductType::Swap);
        }
        if xml.contains("<capFloor>") || xml.contains("<capFloor ") {
            return Ok(ProductType::CapFloor);
        }
        if xml.contains("<fxSingleLeg>") || xml.contains("<fxSingleLeg ") {
            return Ok(ProductType::FxForward);
        }
        if xml.contains("<fxSwap>") || xml.contains("<fxSwap ") {
            return Ok(ProductType::FxSwap);
        }
        if xml.contains("<fxOption>") || xml.contains("<fxOption ") {
            return Ok(ProductType::FxOption);
        }
        if xml.contains("<equityOption>") || xml.contains("<equityOption ") {
            return Ok(ProductType::EquityOption);
        }
        if xml.contains("<creditDefaultSwap>") || xml.contains("<creditDefaultSwap ") {
            return Ok(ProductType::CreditDefaultSwap);
        }
        if xml.contains("<commoditySwap>") || xml.contains("<commoditySwap ") {
            return Ok(ProductType::CommoditySwap);
        }

        Err(FpmlError::UnsupportedProduct(
            "Could not detect FpML product type".to_string(),
        ))
    }

    /// Check if a CDS is an index CDS (CDX, iTraxx).
    fn is_index_cds(xml: &str) -> bool {
        xml.contains("<indexReferenceInformation>")
            || xml.contains("<indexName>")
            || xml.contains("<indexSeries>")
    }
}

/// Detected FpML product type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProductType {
    Swap,
    Swaption,
    CapFloor,

    FxForward,
    FxSwap,
    FxOption,

    EquityOption,

    CreditDefaultSwap,

    CommoditySwap,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_swap() {
        let xml = r#"<trade><swap><swapStream/></swap></trade>"#;
        assert_eq!(
            FpmlParser::detect_product_type(xml).unwrap(),
            ProductType::Swap
        );
    }

    #[test]
    fn test_detect_fx_forward() {
        let xml = r#"<trade><fxSingleLeg><valueDate>2024-01-15</valueDate></fxSingleLeg></trade>"#;
        assert_eq!(
            FpmlParser::detect_product_type(xml).unwrap(),
            ProductType::FxForward
        );
    }

    #[test]
    fn test_detect_cds() {
        let xml = r#"<trade><creditDefaultSwap><generalTerms/></creditDefaultSwap></trade>"#;
        assert_eq!(
            FpmlParser::detect_product_type(xml).unwrap(),
            ProductType::CreditDefaultSwap
        );
    }

    #[test]
    fn test_is_index_cds() {
        let single_name = r#"<creditDefaultSwap><referenceEntity/></creditDefaultSwap>"#;
        let index = r#"<creditDefaultSwap><indexReferenceInformation><indexName>CDX.NA.IG</indexName></indexReferenceInformation></creditDefaultSwap>"#;

        assert!(!FpmlParser::is_index_cds(single_name));
        assert!(FpmlParser::is_index_cds(index));
    }

    #[test]
    fn test_detect_unsupported() {
        let xml = r#"<trade><unknownProduct/></trade>"#;
        assert!(FpmlParser::detect_product_type(xml).is_err());
    }

    #[test]
    fn test_parse_swap() {
        let xml = r#"
            <trade>
                <tradeHeader>
                    <tradeId>TEST-001</tradeId>
                    <tradeDate>2024-01-15</tradeDate>
                </tradeHeader>
                <swap>
                    <swapStream id="fixed">
                        <calculationPeriodDates>
                            <effectiveDate>
                                <unadjustedDate>2024-01-17</unadjustedDate>
                            </effectiveDate>
                        </calculationPeriodDates>
                        <calculationPeriodAmount>
                            <calculation>
                                <notionalSchedule>
                                    <notionalStepSchedule>
                                        <initialValue>10000000</initialValue>
                                        <currency>USD</currency>
                                    </notionalStepSchedule>
                                </notionalSchedule>
                                <fixedRateSchedule>
                                    <initialValue>0.05</initialValue>
                                </fixedRateSchedule>
                            </calculation>
                        </calculationPeriodAmount>
                    </swapStream>
                </swap>
            </trade>
        "#;

        let trade = FpmlParser::parse(xml).unwrap();
        assert_eq!(trade.id.as_str(), "TEST-001");
    }
}
