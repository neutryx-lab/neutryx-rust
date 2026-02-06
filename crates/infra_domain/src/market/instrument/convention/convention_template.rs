//! Convention template for generating multiple convention definitions.
//!
//! This module provides [`ConventionTemplate`] and [`CurrencyDefaults`] which
//! enable compact JSON representation of market conventions by extracting
//! common patterns.
//!
//! # Example JSON
//!
//! ```json
//! {
//!   "currencyDefaults": {
//!     "USD": { "dayCount": "ACT/360", "settlementDays": 2, "calendar": "New York", "index": "SOFR" },
//!     "GBP": { "dayCount": "ACT/365F", "settlementDays": 0, "calendar": "London", "index": "SONIA" }
//!   },
//!   "typeDefaults": {
//!     "OisConvention": { "paymentFrequency": "Annual", "businessDayConvention": "Modified Following" },
//!     "DepositConvention": { "businessDayConvention": "Modified Following" }
//!   },
//!   "templates": [
//!     {
//!       "type": "OisConvention",
//!       "idPattern": "{currency}-{index}-OIS",
//!       "currencies": ["USD", "EUR", "GBP", "JPY", "CHF"]
//!     }
//!   ]
//! }
//! ```

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Default values for a currency, used to reduce repetition in convention definitions.
///
/// These defaults are merged with type-specific defaults and individual overrides
/// to produce the final convention.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CurrencyDefaults {
    /// Default day count convention (e.g., "ACT/360", "ACT/365F")
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub day_count: Option<String>,

    /// Default settlement days
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub settlement_days: Option<u8>,

    /// Default calendar (e.g., "New York", "London", "TARGET")
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub calendar: Option<String>,

    /// Default overnight index (e.g., "SOFR", "SONIA", "ESTR")
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub index: Option<String>,

    /// Default business day convention
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub business_day_convention: Option<String>,
}

/// Template for generating multiple conventions across currencies.
///
/// The template defines common fields and generates one convention per currency
/// in the `currencies` list, with ID generated from `id_pattern`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ConventionTemplate {
    /// Convention type (e.g., "OisConvention", "DepositConvention")
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub convention_type: String,

    /// Pattern for generating IDs. Supports {currency}, {index} placeholders.
    pub id_pattern: String,

    /// List of currencies to generate conventions for
    pub currencies: Vec<String>,

    /// Whether this is the default convention for each currency
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_default: bool,

    /// Additional fields to include in all generated conventions
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub fields: Option<HashMap<String, Value>>,

    /// Per-currency overrides for fields
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub overrides: Option<HashMap<String, HashMap<String, Value>>>,
}

impl ConventionTemplate {
    /// Creates a new convention template.
    #[must_use]
    pub fn new(
        convention_type: impl Into<String>,
        id_pattern: impl Into<String>,
        currencies: Vec<String>,
    ) -> Self {
        Self {
            convention_type: convention_type.into(),
            id_pattern: id_pattern.into(),
            currencies,
            is_default: true,
            fields: None,
            overrides: None,
        }
    }

    /// Sets the default flag.
    #[must_use]
    pub fn with_is_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }

    /// Sets additional fields.
    #[must_use]
    pub fn with_fields(mut self, fields: HashMap<String, Value>) -> Self {
        self.fields = Some(fields);
        self
    }

    /// Sets per-currency overrides.
    #[must_use]
    pub fn with_overrides(mut self, overrides: HashMap<String, HashMap<String, Value>>) -> Self {
        self.overrides = Some(overrides);
        self
    }

    /// Returns the number of conventions this template will generate.
    #[must_use]
    pub fn count(&self) -> usize {
        self.currencies.len()
    }

    /// Expands the template into convention JSON objects.
    ///
    /// # Arguments
    ///
    /// * `currency_defaults` - Map of currency code to default values
    /// * `type_defaults` - Map of convention type to default fields
    #[must_use]
    pub fn expand(
        &self,
        currency_defaults: &HashMap<String, CurrencyDefaults>,
        type_defaults: &HashMap<String, HashMap<String, Value>>,
    ) -> Vec<(String, Value)> {
        self.currencies
            .iter()
            .map(|currency| self.expand_single(currency, currency_defaults, type_defaults))
            .collect()
    }

    /// Expands a single currency into a convention.
    fn expand_single(
        &self,
        currency: &str,
        currency_defaults: &HashMap<String, CurrencyDefaults>,
        type_defaults: &HashMap<String, HashMap<String, Value>>,
    ) -> (String, Value) {
        let ccy_defaults = currency_defaults.get(currency);
        let index = ccy_defaults
            .and_then(|d| d.index.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("");

        // Generate ID from pattern
        let id = self
            .id_pattern
            .replace("{currency}", currency)
            .replace("{index}", index);

        // Build fields by merging defaults and overrides
        let mut fields = serde_json::Map::new();

        // 1. Apply currency defaults
        if let Some(defaults) = ccy_defaults {
            if let Some(ref dc) = defaults.day_count {
                fields.insert("day_count".to_string(), Value::String(dc.clone()));
            }
            if let Some(sd) = defaults.settlement_days {
                fields.insert("settlement_days".to_string(), Value::Number(sd.into()));
            }
            if let Some(ref cal) = defaults.calendar {
                fields.insert("calendar".to_string(), Value::String(cal.clone()));
            }
            if let Some(ref idx) = defaults.index {
                fields.insert("index".to_string(), Value::String(idx.clone()));
            }
            if let Some(ref bdc) = defaults.business_day_convention {
                fields.insert("business_day_convention".to_string(), Value::String(bdc.clone()));
            }
        }

        // 2. Apply type defaults
        if let Some(type_def) = type_defaults.get(&self.convention_type) {
            for (k, v) in type_def {
                fields.insert(k.clone(), v.clone());
            }
        }

        // 3. Apply template-level fields
        if let Some(ref template_fields) = self.fields {
            for (k, v) in template_fields {
                fields.insert(k.clone(), v.clone());
            }
        }

        // 4. Apply per-currency overrides
        if let Some(ref overrides) = self.overrides {
            if let Some(ccy_overrides) = overrides.get(currency) {
                for (k, v) in ccy_overrides {
                    fields.insert(k.clone(), v.clone());
                }
            }
        }

        // Build the convention object
        let mut convention = serde_json::Map::new();
        convention.insert("type".to_string(), Value::String(self.convention_type.clone()));
        convention.insert("currency".to_string(), Value::String(currency.to_string()));
        convention.insert("is_default".to_string(), Value::Bool(self.is_default));
        convention.insert("fields".to_string(), Value::Object(fields));

        (id, Value::Object(convention))
    }
}

/// Bundle for loading conventions with template support.
///
/// This structure supports:
/// - `currencyDefaults`: Default values per currency
/// - `typeDefaults`: Default fields per convention type
/// - `templates`: Bulk generation of conventions
/// - `conventions`: Individual convention definitions
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ConventionBundle {
    /// Metadata about the bundle
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub metadata: Option<HashMap<String, Value>>,

    /// Default values per currency
    #[cfg_attr(feature = "serde", serde(default))]
    pub currency_defaults: HashMap<String, CurrencyDefaults>,

    /// Default fields per convention type
    #[cfg_attr(feature = "serde", serde(default))]
    pub type_defaults: HashMap<String, HashMap<String, Value>>,

    /// Templates for bulk convention generation
    #[cfg_attr(feature = "serde", serde(default))]
    pub templates: Vec<ConventionTemplate>,

    /// Individual convention definitions
    #[cfg_attr(feature = "serde", serde(default))]
    pub conventions: HashMap<String, Value>,
}

impl ConventionBundle {
    /// Creates a new empty bundle.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Expands all templates and merges with individual conventions.
    ///
    /// Returns a map of convention ID to convention object.
    #[must_use]
    pub fn expand_all(&self) -> HashMap<String, Value> {
        let mut result = HashMap::new();

        // First, expand all templates
        for template in &self.templates {
            for (id, convention) in template.expand(&self.currency_defaults, &self.type_defaults) {
                result.insert(id, convention);
            }
        }

        // Then add individual conventions (which can override templates)
        for (id, convention) in &self.conventions {
            result.insert(id.clone(), convention.clone());
        }

        result
    }

    /// Returns the total count of conventions after template expansion.
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.templates.iter().map(|t| t.count()).sum::<usize>() + self.conventions.len()
    }

    /// Converts the bundle to the legacy format (flat conventions map).
    ///
    /// This produces a JSON structure compatible with the original format:
    /// ```json
    /// {
    ///   "metadata": { ... },
    ///   "conventions": { "USD-SOFR-OIS": { ... }, ... }
    /// }
    /// ```
    #[must_use]
    pub fn to_legacy_format(&self) -> Value {
        let mut result = serde_json::Map::new();

        if let Some(ref metadata) = self.metadata {
            result.insert(
                "metadata".to_string(),
                serde_json::to_value(metadata).unwrap_or(Value::Null),
            );
        }

        let conventions = self.expand_all();
        result.insert(
            "conventions".to_string(),
            serde_json::to_value(conventions).unwrap_or(Value::Null),
        );

        Value::Object(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_defaults() {
        let defaults = CurrencyDefaults {
            day_count: Some("ACT/360".to_string()),
            settlement_days: Some(2),
            calendar: Some("New York".to_string()),
            index: Some("SOFR".to_string()),
            business_day_convention: Some("Modified Following".to_string()),
        };

        assert_eq!(defaults.day_count, Some("ACT/360".to_string()));
        assert_eq!(defaults.settlement_days, Some(2));
    }

    #[test]
    fn test_convention_template_expand() {
        let mut currency_defaults = HashMap::new();
        currency_defaults.insert(
            "USD".to_string(),
            CurrencyDefaults {
                day_count: Some("ACT/360".to_string()),
                settlement_days: Some(2),
                calendar: Some("New York".to_string()),
                index: Some("SOFR".to_string()),
                business_day_convention: None,
            },
        );
        currency_defaults.insert(
            "GBP".to_string(),
            CurrencyDefaults {
                day_count: Some("ACT/365F".to_string()),
                settlement_days: Some(0),
                calendar: Some("London".to_string()),
                index: Some("SONIA".to_string()),
                business_day_convention: None,
            },
        );

        let mut type_defaults = HashMap::new();
        let mut ois_defaults = HashMap::new();
        ois_defaults.insert("payment_frequency".to_string(), Value::String("Annual".to_string()));
        ois_defaults.insert(
            "business_day_convention".to_string(),
            Value::String("Modified Following".to_string()),
        );
        type_defaults.insert("OisConvention".to_string(), ois_defaults);

        let template = ConventionTemplate::new(
            "OisConvention",
            "{currency}-{index}-OIS",
            vec!["USD".to_string(), "GBP".to_string()],
        );

        let expanded = template.expand(&currency_defaults, &type_defaults);
        assert_eq!(expanded.len(), 2);

        let (usd_id, usd_conv) = &expanded[0];
        assert_eq!(usd_id, "USD-SOFR-OIS");
        assert_eq!(usd_conv["type"], "OisConvention");
        assert_eq!(usd_conv["currency"], "USD");
        assert_eq!(usd_conv["fields"]["day_count"], "ACT/360");
        assert_eq!(usd_conv["fields"]["settlement_days"], 2);
        assert_eq!(usd_conv["fields"]["payment_frequency"], "Annual");

        let (gbp_id, gbp_conv) = &expanded[1];
        assert_eq!(gbp_id, "GBP-SONIA-OIS");
        assert_eq!(gbp_conv["fields"]["day_count"], "ACT/365F");
        assert_eq!(gbp_conv["fields"]["settlement_days"], 0);
    }

    #[test]
    fn test_convention_template_with_overrides() {
        let mut currency_defaults = HashMap::new();
        currency_defaults.insert(
            "USD".to_string(),
            CurrencyDefaults {
                settlement_days: Some(2),
                ..Default::default()
            },
        );
        currency_defaults.insert(
            "GBP".to_string(),
            CurrencyDefaults {
                settlement_days: Some(2),
                ..Default::default()
            },
        );

        let mut overrides = HashMap::new();
        let mut gbp_override = HashMap::new();
        gbp_override.insert("settlement_days".to_string(), Value::Number(0.into()));
        overrides.insert("GBP".to_string(), gbp_override);

        let template = ConventionTemplate::new("DepositConvention", "{currency}-DEPO", vec!["USD".to_string(), "GBP".to_string()])
            .with_overrides(overrides);

        let expanded = template.expand(&currency_defaults, &HashMap::new());

        let (_, usd_conv) = &expanded[0];
        assert_eq!(usd_conv["fields"]["settlement_days"], 2);

        let (_, gbp_conv) = &expanded[1];
        assert_eq!(gbp_conv["fields"]["settlement_days"], 0);
    }

    #[test]
    fn test_convention_bundle_expand_all() {
        let mut currency_defaults = HashMap::new();
        currency_defaults.insert(
            "USD".to_string(),
            CurrencyDefaults {
                index: Some("SOFR".to_string()),
                ..Default::default()
            },
        );

        let template = ConventionTemplate::new(
            "OisConvention",
            "{currency}-{index}-OIS",
            vec!["USD".to_string()],
        );

        let mut individual = HashMap::new();
        individual.insert(
            "USD-CUSTOM".to_string(),
            serde_json::json!({
                "type": "CustomConvention",
                "currency": "USD"
            }),
        );

        let bundle = ConventionBundle {
            metadata: None,
            currency_defaults,
            type_defaults: HashMap::new(),
            templates: vec![template],
            conventions: individual,
        };

        let expanded = bundle.expand_all();
        assert_eq!(expanded.len(), 2);
        assert!(expanded.contains_key("USD-SOFR-OIS"));
        assert!(expanded.contains_key("USD-CUSTOM"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_convention_bundle_serde() {
        let json = r#"{
            "currencyDefaults": {
                "USD": { "dayCount": "ACT/360", "settlementDays": 2, "calendar": "New York", "index": "SOFR" },
                "GBP": { "dayCount": "ACT/365F", "settlementDays": 0, "calendar": "London", "index": "SONIA" }
            },
            "typeDefaults": {
                "OisConvention": { "payment_frequency": "Annual", "business_day_convention": "Modified Following" }
            },
            "templates": [
                {
                    "type": "OisConvention",
                    "idPattern": "{currency}-{index}-OIS",
                    "currencies": ["USD", "GBP"],
                    "isDefault": true
                }
            ],
            "conventions": {}
        }"#;

        let bundle: ConventionBundle = serde_json::from_str(json).unwrap();
        assert_eq!(bundle.currency_defaults.len(), 2);
        assert_eq!(bundle.templates.len(), 1);

        let expanded = bundle.expand_all();
        assert_eq!(expanded.len(), 2);
        assert!(expanded.contains_key("USD-SOFR-OIS"));
        assert!(expanded.contains_key("GBP-SONIA-OIS"));
    }
}
