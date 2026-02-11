//! Central bank definitions.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Central bank identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CentralBank {
    /// Bank code (e.g., "FED", "ECB", "BOJ", "BOE").
    pub code: String,
    /// Full name (e.g., "Federal Reserve", "European Central Bank").
    pub name: String,
    /// Associated currency code (e.g., "USD", "EUR").
    pub currency: String,
    /// Country or region (e.g., "United States", "Eurozone").
    pub region: String,
}

impl CentralBank {
    /// Create a new central bank.
    pub fn new(
        code: impl Into<String>,
        name: impl Into<String>,
        currency: impl Into<String>,
        region: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            currency: currency.into(),
            region: region.into(),
        }
    }

    /// Federal Reserve (United States).
    pub fn fed() -> Self { Self::new("FED", "Federal Reserve", "USD", "United States") }

    /// European Central Bank.
    pub fn ecb() -> Self { Self::new("ECB", "European Central Bank", "EUR", "Eurozone") }

    /// Bank of Japan.
    pub fn boj() -> Self { Self::new("BOJ", "Bank of Japan", "JPY", "Japan") }

    /// Bank of England.
    pub fn boe() -> Self { Self::new("BOE", "Bank of England", "GBP", "United Kingdom") }

    /// Swiss National Bank.
    pub fn snb() -> Self { Self::new("SNB", "Swiss National Bank", "CHF", "Switzerland") }

    /// Reserve Bank of Australia.
    pub fn rba() -> Self { Self::new("RBA", "Reserve Bank of Australia", "AUD", "Australia") }

    /// Bank of Canada.
    pub fn boc() -> Self { Self::new("BOC", "Bank of Canada", "CAD", "Canada") }
}

impl std::fmt::Display for CentralBank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.code)
    }
}
