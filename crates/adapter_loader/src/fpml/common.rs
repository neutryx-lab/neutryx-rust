//! Common FpML parsing utilities.
//!
//! Provides helpers for parsing common FpML elements like dates, parties,
//! and schedule definitions.

use infra_domain::{market::Currency, time::Date};

use super::error::FpmlError;

// =============================================================================
// Field Extraction Macros
// =============================================================================

/// Extract a text field from an `XmlNavigator`, returning a default if absent.
///
/// Usage: `xml_text!(nav, "elementName", "default")`
macro_rules! xml_text {
    ($nav:expr, $elem:expr, $default:expr) => {
        $nav.find_text($elem)
            .unwrap_or_else(|| $default.to_string())
    };
}

/// Extract a decimal (f64) field from an `XmlNavigator`, returning a default if
/// absent.
///
/// Usage: `xml_decimal!(nav, "elementName", 0.0)`
macro_rules! xml_decimal {
    ($nav:expr, $elem:expr, $default:expr) => {
        $nav.find_text($elem)
            .map(|v| $crate::fpml::common::parse_decimal(&v))
            .transpose()?
            .unwrap_or($default)
    };
}

/// Extract a date field from an `XmlNavigator`, returning a default if absent.
///
/// Usage: `xml_date!(nav, "elementName", Date::from_ymd(2024,1,1).unwrap())`
macro_rules! xml_date {
    ($nav:expr, $elem:expr, $default:expr) => {
        $nav.find_text($elem)
            .map(|d| $crate::fpml::common::parse_date(&d))
            .transpose()?
            .unwrap_or_else(|| $default)
    };
}

/// Extract a decimal from one of several candidate elements, returning a
/// default.
///
/// Usage: `xml_decimal_or!(nav, "primary", "fallback", 0.0)`
macro_rules! xml_decimal_or {
    ($nav:expr, $( $elem:expr ),+; $default:expr) => {{
        let text = None
            $( .or_else(|| $nav.find_text($elem)) )+;
        text.map(|v| $crate::fpml::common::parse_decimal(&v))
            .transpose()?
            .unwrap_or($default)
    }};
}

pub(crate) use xml_date;
pub(crate) use xml_decimal;
pub(crate) use xml_decimal_or;
pub(crate) use xml_text;

/// Parse currency string to Currency enum.
pub fn parse_currency(s: &str) -> Currency {
    match s.to_uppercase().as_str() {
        "USD" => Currency::USD,
        "EUR" => Currency::EUR,
        "GBP" => Currency::GBP,
        "JPY" => Currency::JPY,
        "CHF" => Currency::CHF,
        _ => Currency::USD, // Default fallback
    }
}

/// Parse a date from FpML format (YYYY-MM-DD).
pub fn parse_date(date_str: &str) -> Result<Date, FpmlError> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return Err(FpmlError::DateError(format!(
            "Invalid date format: {}",
            date_str
        )));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| FpmlError::DateError(format!("Invalid year in date: {}", date_str)))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| FpmlError::DateError(format!("Invalid month in date: {}", date_str)))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| FpmlError::DateError(format!("Invalid day in date: {}", date_str)))?;

    Date::from_ymd(year, month, day)
        .map_err(|_| FpmlError::DateError(format!("Invalid date: {}", date_str)))
}

/// Parse a decimal value from string.
pub fn parse_decimal(value_str: &str) -> Result<f64, FpmlError> {
    value_str.parse().map_err(|_| FpmlError::InvalidValue {
        element: "decimal".to_string(),
        message: format!("Could not parse '{}' as decimal", value_str),
    })
}

/// XML element navigator for easier tree traversal.
pub struct XmlNavigator<'a> {
    content: &'a str,
}

impl<'a> XmlNavigator<'a> {
    /// Creates a new navigator from XML content.
    pub fn new(content: &'a str) -> Self { Self { content } }

    /// Finds the first occurrence of an element and returns its text content.
    ///
    /// If the element contains nested elements, returns the raw content.
    /// For leaf elements (no nested elements), returns the text.
    pub fn find_text(&self, element_name: &str) -> Option<String> {
        // First try to find a simple element (leaf with no nested elements)
        if let Some(text) = self.find_leaf_text(element_name) {
            return Some(text);
        }

        // Fall back to extracting raw content
        let start_tag = format!("<{}", element_name);
        let end_tag = format!("</{}>", element_name);

        let start_idx = self.content.find(&start_tag)?;
        let after_start = &self.content[start_idx..];

        // Find the > that closes the opening tag
        let content_start = after_start.find('>')? + 1;

        let end_idx = after_start.find(&end_tag)?;

        if content_start >= end_idx {
            return Some(String::new());
        }

        let text = &after_start[content_start..end_idx];
        Some(text.trim().to_string())
    }

    /// Finds a leaf element (one with no nested elements) and returns its text.
    pub fn find_leaf_text(&self, element_name: &str) -> Option<String> {
        // Pattern: <elementName>text</elementName> or <elementName
        // ...>text</elementName> where text contains no < characters
        let start_tag = format!("<{}", element_name);
        let end_tag = format!("</{}>", element_name);

        let mut search_from = 0;
        while let Some(start_idx) = self.content[search_from..].find(&start_tag) {
            let abs_start = search_from + start_idx;
            let after_start = &self.content[abs_start..];

            // Find the > that closes the opening tag
            if let Some(tag_close) = after_start.find('>') {
                let content_start = tag_close + 1;

                // Find the corresponding end tag
                if let Some(end_idx) = after_start.find(&end_tag) {
                    if content_start < end_idx {
                        let text = &after_start[content_start..end_idx];

                        // Check if this is a leaf element (no nested elements)
                        if !text.contains('<') {
                            return Some(text.trim().to_string());
                        }
                    }
                }
            }

            // Move to next occurrence
            search_from = abs_start + 1;
        }

        None
    }

    /// Extracts a subsection of XML by element name.
    pub fn extract_section(&self, element_name: &str) -> Option<String> {
        let start_tag = format!("<{}", element_name);
        let end_tag = format!("</{}>", element_name);

        let start_idx = self.content.find(&start_tag)?;
        let after_start = &self.content[start_idx..];
        let end_idx = after_start.find(&end_tag)? + end_tag.len();

        Some(after_start[..end_idx].to_string())
    }

    /// Extracts all subsections of XML by element name.
    pub fn extract_all_sections(&self, element_name: &str) -> Vec<String> {
        let mut results = Vec::new();
        let start_tag = format!("<{}", element_name);
        let end_tag = format!("</{}>", element_name);

        let mut search_from = 0;
        while let Some(start_idx) = self.content[search_from..].find(&start_tag) {
            let abs_start = search_from + start_idx;
            let after_start = &self.content[abs_start..];

            if let Some(end_idx) = after_start.find(&end_tag) {
                let section = &after_start[..end_idx + end_tag.len()];
                results.push(section.to_string());
                search_from = abs_start + end_idx + end_tag.len();
            } else {
                break;
            }
        }

        results
    }

    /// Gets an attribute value from the first matching element.
    pub fn get_attribute(&self, element_name: &str, attr_name: &str) -> Option<String> {
        let start_tag = format!("<{}", element_name);

        let start_idx = self.content.find(&start_tag)?;
        let after_start = &self.content[start_idx..];
        let tag_end = after_start.find('>')?;
        let tag_content = &after_start[..tag_end];

        // Look for attribute
        let attr_pattern = format!("{}=\"", attr_name);
        let attr_start = tag_content.find(&attr_pattern)?;
        let value_start = attr_start + attr_pattern.len();
        let after_value = &tag_content[value_start..];
        let value_end = after_value.find('"')?;

        Some(after_value[..value_end].to_string())
    }
}

/// Parsed party information from FpML.
#[derive(Debug, Clone)]
pub struct Party {
    /// Party ID (href reference).
    pub id: String,
    /// Party identifier (e.g., LEI).
    pub party_id: Option<String>,
    /// Party name.
    pub name: Option<String>,
}

/// Parse all parties from an FpML document.
pub fn parse_parties(xml: &str) -> Vec<Party> {
    let mut parties = Vec::new();
    let nav = XmlNavigator::new(xml);

    for section in nav.extract_all_sections("party") {
        let section_nav = XmlNavigator::new(&section);

        let id = section_nav.get_attribute("party", "id").unwrap_or_default();
        let party_id = section_nav.find_text("partyId");
        let name = section_nav.find_text("partyName");

        parties.push(Party { id, party_id, name });
    }

    parties
}

/// Trade header information.
#[derive(Debug, Clone)]
pub struct TradeHeader {
    /// Trade ID.
    pub trade_id: String,
    /// Trade date.
    pub trade_date: Option<Date>,
    /// Party references.
    pub parties: Vec<String>,
    /// Counterparty name (extracted from party definitions).
    pub counterparty: Option<String>,
    /// Book identifier (if available).
    pub book: Option<String>,
}

/// Parse trade header from FpML.
///
/// Extracts trade ID, date, party references, and resolves the counterparty
/// from the party definitions in the document.
pub fn parse_trade_header(xml: &str) -> Result<TradeHeader, FpmlError> {
    let nav = XmlNavigator::new(xml);

    let header_section = nav
        .extract_section("tradeHeader")
        .ok_or_else(|| FpmlError::MissingElement("tradeHeader".to_string()))?;

    let header_nav = XmlNavigator::new(&header_section);

    let trade_id = header_nav
        .find_text("tradeId")
        .ok_or_else(|| FpmlError::MissingElement("tradeId".to_string()))?;

    let trade_date = header_nav
        .find_text("tradeDate")
        .map(|d| parse_date(&d))
        .transpose()?;

    // Extract party references
    let mut parties = Vec::new();
    for section in header_nav.extract_all_sections("partyTradeIdentifier") {
        let section_nav = XmlNavigator::new(&section);
        if let Some(party_ref) = section_nav.get_attribute("partyReference", "href") {
            parties.push(party_ref);
        }
    }

    // Parse all party definitions to resolve counterparty
    let all_parties = parse_parties(xml);

    // Find counterparty (party that is not "our" bank - FB_NA, FrictionalBank,
    // etc.)
    let counterparty = find_counterparty(&parties, &all_parties);

    // Extract book from tradeIdentifierExtension if present
    let book = nav.find_text("book").or_else(|| nav.find_text("bookId"));

    Ok(TradeHeader {
        trade_id,
        trade_date,
        parties,
        counterparty,
        book,
    })
}

/// Find the counterparty from the list of party references.
///
/// Returns the party ID (partyReference href value) directly, e.g., "GOLDMAN",
/// "BARCLAYS". Assumes our bank ID starts with "FB" or contains
/// "FrictionalBank".
fn find_counterparty(party_refs: &[String], _all_parties: &[Party]) -> Option<String> {
    for party_ref in party_refs {
        // Skip our own bank
        if party_ref.starts_with("FB") || party_ref.contains("FRICTIONAL") {
            continue;
        }
        // Return the party reference ID directly (e.g., "GOLDMAN", "BARCLAYS")
        return Some(party_ref.clone());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        let date = parse_date("2024-12-15").unwrap();
        assert_eq!(date, Date::from_ymd(2024, 12, 15).unwrap());
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("invalid").is_err());
        assert!(parse_date("2024-13-01").is_err()); // Invalid month
    }

    #[test]
    fn test_parse_decimal() {
        assert_eq!(parse_decimal("0.05").unwrap(), 0.05);
        assert_eq!(parse_decimal("1000000").unwrap(), 1_000_000.0);
    }

    #[test]
    fn test_xml_navigator_find_text() {
        let xml = r#"<root><name>Test Value</name><amount>100</amount></root>"#;
        let nav = XmlNavigator::new(xml);

        assert_eq!(nav.find_text("name"), Some("Test Value".to_string()));
        assert_eq!(nav.find_text("amount"), Some("100".to_string()));
        assert_eq!(nav.find_text("missing"), None);
    }

    #[test]
    fn test_xml_navigator_extract_section() {
        let xml = r#"<root><swap><leg>fixed</leg></swap></root>"#;
        let nav = XmlNavigator::new(xml);

        let section = nav.extract_section("swap").unwrap();
        assert!(section.contains("<leg>fixed</leg>"));
    }

    #[test]
    fn test_xml_navigator_get_attribute() {
        let xml = r#"<root><party id="FB_NA">Test</party></root>"#;
        let nav = XmlNavigator::new(xml);

        assert_eq!(nav.get_attribute("party", "id"), Some("FB_NA".to_string()));
    }
}
