//! Common FpML parsing utilities.

use infra_domain::{market::Currency, time::Date, trade::TradeMetadata};
use quick_xml::{events::Event, Reader};

use super::error::FpmlError;

/// Extract a text field from an `XmlNavigator`, returning a default if absent.
macro_rules! xml_text {
    ($nav:expr, $elem:expr, $default:expr) => {
        $nav.find_text($elem)
            .unwrap_or_else(|| $default.to_string())
    };
}

/// Extract a decimal (f64) field from an `XmlNavigator`, returning a default
/// if.
macro_rules! xml_decimal {
    ($nav:expr, $elem:expr, $default:expr) => {
        $nav.find_text($elem)
            .map(|v| $crate::fpml::common::parse_decimal(&v))
            .transpose()?
            .unwrap_or($default)
    };
}

/// Extract a date field from an `XmlNavigator`, returning a default if absent.
macro_rules! xml_date {
    ($nav:expr, $elem:expr, $default:expr) => {
        $nav.find_text($elem)
            .map(|d| $crate::fpml::common::parse_date(&d))
            .transpose()?
            .unwrap_or_else(|| $default)
    };
}

/// Extract a decimal from one of several candidate elements, returning a.
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
        _ => Currency::USD,
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

/// XML element navigator backed by quick-xml for robust tree traversal.
pub struct XmlNavigator<'a> {
    content: &'a str,
}

/// Check if an element name matches the target, ignoring namespace prefixes.
fn name_matches(name: &[u8], target: &[u8]) -> bool {
    if name == target {
        return true;
    }
    if let Some(pos) = name.iter().position(|&b| b == b':') {
        return &name[pos + 1..] == target;
    }
    false
}

impl<'a> XmlNavigator<'a> {
    /// Creates a new navigator from XML content.
    pub fn new(content: &'a str) -> Self { Self { content } }

    /// Finds the first occurrence of an element and returns its text content.
    ///
    /// Tries leaf-only match first, then falls back to raw inner content.
    pub fn find_text(&self, element_name: &str) -> Option<String> {
        if let Some(text) = self.find_leaf_text(element_name) {
            return Some(text);
        }

        // Fallback: extract the section and strip the outer tags.
        let section = self.extract_section(element_name)?;
        let content_start = section.find('>')? + 1;
        let end_tag = format!("</{element_name}>");
        let content_end = section.rfind(&end_tag)?;
        if content_start >= content_end {
            return Some(String::new());
        }
        Some(section[content_start..content_end].trim().to_string())
    }

    /// Finds a leaf element (one with no nested elements) and returns its text.
    pub fn find_leaf_text(&self, element_name: &str) -> Option<String> {
        let mut reader = Reader::from_str(self.content);
        let target = element_name.as_bytes();

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    if name_matches(e.name().as_ref(), target) {
                        let mut text = String::new();
                        let mut is_leaf = true;
                        let mut depth = 1u32;

                        loop {
                            match reader.read_event() {
                                Ok(Event::Start(_)) => {
                                    is_leaf = false;
                                    depth += 1;
                                }
                                Ok(Event::Empty(_)) => {
                                    is_leaf = false;
                                }
                                Ok(Event::End(_)) => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                Ok(Event::Text(t)) => {
                                    if depth == 1 {
                                        if let Ok(s) = t.unescape() {
                                            text.push_str(&s);
                                        }
                                    }
                                }
                                Ok(Event::Eof) | Err(_) => return None,
                                _ => {}
                            }
                        }

                        if is_leaf {
                            let trimmed = text.trim().to_string();
                            if !trimmed.is_empty() {
                                return Some(trimmed);
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }
        None
    }

    /// Extracts a subsection of XML by element name.
    pub fn extract_section(&self, element_name: &str) -> Option<String> {
        let mut reader = Reader::from_str(self.content);
        let target = element_name.as_bytes();

        loop {
            let start_pos = reader.buffer_position() as usize;
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    if name_matches(e.name().as_ref(), target) {
                        reader.read_to_end(e.to_end().name()).ok()?;
                        let end_pos = reader.buffer_position() as usize;
                        return Some(self.content[start_pos..end_pos].to_string());
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }
        None
    }

    /// Extracts all subsections of XML by element name.
    pub fn extract_all_sections(&self, element_name: &str) -> Vec<String> {
        let mut results = Vec::new();
        let mut reader = Reader::from_str(self.content);
        let target = element_name.as_bytes();

        loop {
            let start_pos = reader.buffer_position() as usize;
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    if name_matches(e.name().as_ref(), target)
                        && reader.read_to_end(e.to_end().name()).is_ok()
                    {
                        let end_pos = reader.buffer_position() as usize;
                        results.push(self.content[start_pos..end_pos].to_string());
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }
        results
    }

    /// Gets an attribute value from the first matching element.
    pub fn get_attribute(&self, element_name: &str, attr_name: &str) -> Option<String> {
        let mut reader = Reader::from_str(self.content);
        let target = element_name.as_bytes();
        let attr_target = attr_name.as_bytes();

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if name_matches(e.name().as_ref(), target) {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == attr_target {
                                return attr.unescape_value().ok().map(|v| v.to_string());
                            }
                        }
                        return None;
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }
        None
    }
}

/// Build trade metadata from a parsed trade header.
pub fn build_metadata(header: &TradeHeader) -> TradeMetadata {
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

/// Extract a date from a nested section like.
pub fn extract_nested_date(
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

/// Extract notional from a nested section like.
pub fn extract_nested_amount(
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

    let mut parties = Vec::new();
    for section in header_nav.extract_all_sections("partyTradeIdentifier") {
        let section_nav = XmlNavigator::new(&section);
        if let Some(party_ref) = section_nav.get_attribute("partyReference", "href") {
            parties.push(party_ref);
        }
    }

    let all_parties = parse_parties(xml);

    let counterparty = find_counterparty(&parties, &all_parties);

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
fn find_counterparty(party_refs: &[String], _all_parties: &[Party]) -> Option<String> {
    for party_ref in party_refs {
        if party_ref.starts_with("FB") || party_ref.contains("FRICTIONAL") {
            continue;
        }
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
        assert!(parse_date("2024-13-01").is_err());
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
