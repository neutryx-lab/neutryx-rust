//! Definition registry for curve construction.
//!
//! This module provides [`DefinitionRegistry`] which aggregates and validates
//! [`InstrumentDefinition`], [`RateIndexDefinition`], and [`CurveDefinition`]
//! instances, ensuring referential integrity.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::{
//!     DefinitionRegistry, InstrumentDefinition, RateIndexDefinition,
//!     CurveDefinition, Currency, RateType, RateIndex,
//! };
//!
//! let mut registry = DefinitionRegistry::new();
//!
//! // Register instrument
//! registry.register_instrument(InstrumentDefinition::new(
//!     "USD-Depo-ON", Currency::USD, RateType::Deposit, "O/N",
//! ));
//!
//! // Register rate index
//! registry.register_rate_index(RateIndexDefinition::new(
//!     "USD-SOFR", Currency::USD, RateIndex::Sofr,
//! )).unwrap();
//!
//! // Register curve (validates references)
//! registry.register_curve(CurveDefinition::new(
//!     "USD-SOFR-Discount",
//!     "USD-SOFR",
//!     vec!["USD-Depo-ON".to_string()],
//! )).unwrap();
//! ```

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::definition::{
    CurveDefError, CurveDefinition, InstrumentDefError, InstrumentDefinition,
    InstrumentTemplate, RateIndexDefError, RateIndexDefinition,
};

/// Error type for registry operations.
#[derive(Debug, Clone)]
pub enum RegistryError {
    /// Instrument definition error
    Instrument(InstrumentDefError),
    /// Rate index definition error
    RateIndex(RateIndexDefError),
    /// Curve definition error
    Curve(CurveDefError),
    /// Duplicate ID
    DuplicateId {
        /// Entity type (e.g., "instrument", "curve")
        entity: &'static str,
        /// The duplicate ID
        id: String,
    },
    /// Reference not found
    ReferenceNotFound {
        /// Source entity that has the reference
        from: String,
        /// Target ID that was not found
        to: String,
        /// Target entity type
        entity: &'static str,
    },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Instrument(e) => write!(f, "Instrument error: {}", e),
            Self::RateIndex(e) => write!(f, "Rate index error: {}", e),
            Self::Curve(e) => write!(f, "Curve error: {}", e),
            Self::DuplicateId { entity, id } => {
                write!(f, "Duplicate {} ID: {}", entity, id)
            }
            Self::ReferenceNotFound { from, to, entity } => {
                write!(f, "{} '{}' references unknown {}: '{}'", entity, from, entity, to)
            }
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<InstrumentDefError> for RegistryError {
    fn from(e: InstrumentDefError) -> Self {
        Self::Instrument(e)
    }
}

impl From<RateIndexDefError> for RegistryError {
    fn from(e: RateIndexDefError) -> Self {
        Self::RateIndex(e)
    }
}

impl From<CurveDefError> for RegistryError {
    fn from(e: CurveDefError) -> Self {
        Self::Curve(e)
    }
}

/// Registry for curve construction definitions.
///
/// Aggregates instruments, rate indices, and curve definitions,
/// validating referential integrity on registration.
#[derive(Debug, Clone, Default)]
pub struct DefinitionRegistry {
    instruments: HashMap<String, InstrumentDefinition>,
    rate_indices: HashMap<String, RateIndexDefinition>,
    curves: HashMap<String, CurveDefinition>,
}

/// JSON-serializable bundle of all definitions for loading.
///
/// Supports both individual instrument definitions and templates.
/// Templates are expanded to individual instruments during loading.
///
/// # Example JSON
///
/// ```json
/// {
///   "templates": [
///     {
///       "idPattern": "{currency}-OIS-{tenor}",
///       "currency": "USD",
///       "convention": "USD-SOFR-OIS",
///       "rateIndex": "USD-SOFR",
///       "tenors": ["1M", "3M", "6M", "1Y", "5Y", "10Y", "30Y"]
///     }
///   ],
///   "instruments": [
///     { "id": "USD-Custom", "currency": "USD", "convention": "USD-DEPO", "tenor": "O/N" }
///   ],
///   "rateIndices": [...],
///   "curves": [...]
/// }
/// ```
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionBundle {
    /// Instrument templates for bulk generation
    #[serde(default)]
    pub templates: Vec<InstrumentTemplate>,

    /// Individual instrument definitions
    #[serde(default)]
    pub instruments: Vec<InstrumentDefinition>,

    /// Rate index definitions
    #[serde(default, rename = "rateIndices")]
    pub rate_indices: Vec<RateIndexDefinition>,

    /// Curve definitions
    #[serde(default)]
    pub curves: Vec<CurveDefinition>,
}

#[cfg(feature = "serde")]
impl DefinitionBundle {
    /// Expands all templates and returns the combined list of instruments.
    ///
    /// This includes both individually defined instruments and those
    /// generated from templates.
    #[must_use]
    pub fn expand_instruments(&self) -> Vec<InstrumentDefinition> {
        let mut result = Vec::new();

        // First, expand all templates
        for template in &self.templates {
            result.extend(template.expand());
        }

        // Then add individual instruments
        result.extend(self.instruments.iter().cloned());

        result
    }

    /// Returns the total count of instruments after template expansion.
    #[must_use]
    pub fn total_instrument_count(&self) -> usize {
        self.templates.iter().map(|t| t.count()).sum::<usize>() + self.instruments.len()
    }
}

impl DefinitionRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an instrument definition.
    ///
    /// Validates the definition and checks for duplicate IDs.
    ///
    /// # Errors
    ///
    /// Returns error if validation fails or ID already exists.
    pub fn register_instrument(
        &mut self,
        def: InstrumentDefinition,
    ) -> Result<(), RegistryError> {
        def.validate()?;

        if self.instruments.contains_key(&def.id) {
            return Err(RegistryError::DuplicateId {
                entity: "instrument",
                id: def.id.clone(),
            });
        }

        self.instruments.insert(def.id.clone(), def);
        Ok(())
    }

    /// Registers a rate index definition.
    ///
    /// Validates the definition and checks for duplicate IDs.
    ///
    /// # Errors
    ///
    /// Returns error if validation fails or ID already exists.
    pub fn register_rate_index(
        &mut self,
        def: RateIndexDefinition,
    ) -> Result<(), RegistryError> {
        def.validate()?;

        if self.rate_indices.contains_key(&def.id) {
            return Err(RegistryError::DuplicateId {
                entity: "rate_index",
                id: def.id.clone(),
            });
        }

        self.rate_indices.insert(def.id.clone(), def);
        Ok(())
    }

    /// Registers a curve definition.
    ///
    /// Validates the definition and checks that all referenced
    /// instruments and rate index exist.
    ///
    /// # Errors
    ///
    /// Returns error if validation fails, references are missing,
    /// or name already exists.
    pub fn register_curve(&mut self, def: CurveDefinition) -> Result<(), RegistryError> {
        def.validate()?;

        // Check for duplicate
        if self.curves.contains_key(&def.name) {
            return Err(RegistryError::DuplicateId {
                entity: "curve",
                id: def.name.clone(),
            });
        }

        // Verify rate_index reference
        if !self.rate_indices.contains_key(&def.rate_index) {
            return Err(RegistryError::ReferenceNotFound {
                from: def.name.clone(),
                to: def.rate_index.clone(),
                entity: "rate_index",
            });
        }

        // Verify all instrument references
        for inst_id in &def.instruments {
            if !self.instruments.contains_key(inst_id) {
                return Err(RegistryError::ReferenceNotFound {
                    from: def.name.clone(),
                    to: inst_id.clone(),
                    entity: "instrument",
                });
            }
        }

        self.curves.insert(def.name.clone(), def);
        Ok(())
    }

    /// Gets an instrument definition by ID.
    #[must_use]
    pub fn get_instrument(&self, id: &str) -> Option<&InstrumentDefinition> {
        self.instruments.get(id)
    }

    /// Gets a rate index definition by ID.
    #[must_use]
    pub fn get_rate_index(&self, id: &str) -> Option<&RateIndexDefinition> {
        self.rate_indices.get(id)
    }

    /// Gets a curve definition by name.
    #[must_use]
    pub fn get_curve(&self, name: &str) -> Option<&CurveDefinition> {
        self.curves.get(name)
    }

    /// Returns the number of registered instruments.
    #[must_use]
    pub fn instrument_count(&self) -> usize {
        self.instruments.len()
    }

    /// Returns the number of registered rate indices.
    #[must_use]
    pub fn rate_index_count(&self) -> usize {
        self.rate_indices.len()
    }

    /// Returns the number of registered curves.
    #[must_use]
    pub fn curve_count(&self) -> usize {
        self.curves.len()
    }

    /// Returns an iterator over all instrument definitions.
    pub fn instruments(&self) -> impl Iterator<Item = &InstrumentDefinition> {
        self.instruments.values()
    }

    /// Returns an iterator over all rate index definitions.
    pub fn rate_indices(&self) -> impl Iterator<Item = &RateIndexDefinition> {
        self.rate_indices.values()
    }

    /// Returns an iterator over all curve definitions.
    pub fn curves(&self) -> impl Iterator<Item = &CurveDefinition> {
        self.curves.values()
    }

    /// Gets the instrument definitions for a curve.
    ///
    /// Returns the definitions in the order specified by the curve.
    ///
    /// # Panics
    ///
    /// Panics if the curve references an instrument that doesn't exist.
    /// This should not happen if the curve was registered via [`register_curve`].
    #[must_use]
    pub fn curve_instruments(&self, curve_name: &str) -> Option<Vec<&InstrumentDefinition>> {
        let curve = self.curves.get(curve_name)?;
        Some(
            curve
                .instruments
                .iter()
                .map(|id| {
                    self.instruments
                        .get(id)
                        .expect("curve references unknown instrument")
                })
                .collect(),
        )
    }

    /// Gets the rate index definition for a curve.
    ///
    /// # Panics
    ///
    /// Panics if the curve references a rate index that doesn't exist.
    /// This should not happen if the curve was registered via [`register_curve`].
    #[must_use]
    pub fn curve_rate_index(&self, curve_name: &str) -> Option<&RateIndexDefinition> {
        let curve = self.curves.get(curve_name)?;
        self.rate_indices.get(&curve.rate_index)
    }

    /// Loads definitions from a JSON bundle.
    ///
    /// Registers all definitions in order:
    /// 1. Instruments from templates (expanded)
    /// 2. Individual instruments
    /// 3. Rate indices
    /// 4. Curves
    ///
    /// # Errors
    ///
    /// Returns error if any definition fails validation or registration.
    #[cfg(feature = "serde")]
    pub fn load_bundle(&mut self, bundle: DefinitionBundle) -> Result<(), RegistryError> {
        // First, expand and register instruments from templates
        for template in &bundle.templates {
            template.validate()?;
            for inst in template.expand() {
                self.register_instrument(inst)?;
            }
        }

        // Then register individual instruments
        for inst in bundle.instruments {
            self.register_instrument(inst)?;
        }

        // Then rate indices
        for idx in bundle.rate_indices {
            self.register_rate_index(idx)?;
        }

        // Finally curves (which reference the above)
        for curve in bundle.curves {
            self.register_curve(curve)?;
        }

        Ok(())
    }

    /// Loads definitions from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns error if JSON parsing fails or any definition fails validation.
    #[cfg(feature = "serde")]
    pub fn load_from_json(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let bundle: DefinitionBundle = serde_json::from_str(json)?;
        let mut registry = Self::new();
        registry.load_bundle(bundle)?;
        Ok(registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::{Currency, RateIndex, RateType};

    fn create_test_instrument(id: &str) -> InstrumentDefinition {
        InstrumentDefinition::new(id, Currency::USD, RateType::Deposit, "O/N")
    }

    fn create_test_rate_index(id: &str) -> RateIndexDefinition {
        RateIndexDefinition::new(id, Currency::USD, RateIndex::Sofr)
    }

    #[test]
    fn test_registry_new() {
        let registry = DefinitionRegistry::new();
        assert_eq!(registry.instrument_count(), 0);
        assert_eq!(registry.rate_index_count(), 0);
        assert_eq!(registry.curve_count(), 0);
    }

    #[test]
    fn test_register_instrument() {
        let mut registry = DefinitionRegistry::new();
        let inst = create_test_instrument("USD-Depo-ON");

        assert!(registry.register_instrument(inst).is_ok());
        assert_eq!(registry.instrument_count(), 1);
        assert!(registry.get_instrument("USD-Depo-ON").is_some());
    }

    #[test]
    fn test_register_instrument_duplicate() {
        let mut registry = DefinitionRegistry::new();
        let inst1 = create_test_instrument("USD-Depo-ON");
        let inst2 = create_test_instrument("USD-Depo-ON");

        assert!(registry.register_instrument(inst1).is_ok());
        assert!(matches!(
            registry.register_instrument(inst2),
            Err(RegistryError::DuplicateId { .. })
        ));
    }

    #[test]
    fn test_register_rate_index() {
        let mut registry = DefinitionRegistry::new();
        let idx = create_test_rate_index("USD-SOFR");

        assert!(registry.register_rate_index(idx).is_ok());
        assert_eq!(registry.rate_index_count(), 1);
        assert!(registry.get_rate_index("USD-SOFR").is_some());
    }

    #[test]
    fn test_register_curve_success() {
        let mut registry = DefinitionRegistry::new();

        // Register dependencies first
        registry
            .register_instrument(create_test_instrument("USD-Depo-ON"))
            .unwrap();
        registry
            .register_rate_index(create_test_rate_index("USD-SOFR"))
            .unwrap();

        // Now register curve
        let curve = CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string()],
        );

        assert!(registry.register_curve(curve).is_ok());
        assert_eq!(registry.curve_count(), 1);
    }

    #[test]
    fn test_register_curve_missing_rate_index() {
        let mut registry = DefinitionRegistry::new();
        registry
            .register_instrument(create_test_instrument("USD-Depo-ON"))
            .unwrap();

        let curve = CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR", // Not registered
            vec!["USD-Depo-ON".to_string()],
        );

        assert!(matches!(
            registry.register_curve(curve),
            Err(RegistryError::ReferenceNotFound { entity: "rate_index", .. })
        ));
    }

    #[test]
    fn test_register_curve_missing_instrument() {
        let mut registry = DefinitionRegistry::new();
        registry
            .register_rate_index(create_test_rate_index("USD-SOFR"))
            .unwrap();

        let curve = CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string()], // Not registered
        );

        assert!(matches!(
            registry.register_curve(curve),
            Err(RegistryError::ReferenceNotFound { entity: "instrument", .. })
        ));
    }

    #[test]
    fn test_curve_instruments() {
        let mut registry = DefinitionRegistry::new();

        let inst1 = create_test_instrument("USD-Depo-ON");
        let inst2 = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y");

        registry.register_instrument(inst1).unwrap();
        registry.register_instrument(inst2).unwrap();
        registry
            .register_rate_index(create_test_rate_index("USD-SOFR"))
            .unwrap();

        let curve = CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string(), "USD-OIS-5Y".to_string()],
        );
        registry.register_curve(curve).unwrap();

        let instruments = registry.curve_instruments("USD-SOFR-Discount").unwrap();
        assert_eq!(instruments.len(), 2);
        assert_eq!(instruments[0].id, "USD-Depo-ON");
        assert_eq!(instruments[1].id, "USD-OIS-5Y");
    }

    #[test]
    fn test_curve_rate_index() {
        let mut registry = DefinitionRegistry::new();

        registry
            .register_instrument(create_test_instrument("USD-Depo-ON"))
            .unwrap();
        registry
            .register_rate_index(create_test_rate_index("USD-SOFR"))
            .unwrap();

        let curve = CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string()],
        );
        registry.register_curve(curve).unwrap();

        let rate_index = registry.curve_rate_index("USD-SOFR-Discount").unwrap();
        assert_eq!(rate_index.id, "USD-SOFR");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_load_from_json() {
        // Using convention-based format (preferred)
        let json = r#"{
            "instruments": [
                { "id": "USD-Depo-ON", "currency": "USD", "convention": "USD-DEPO", "tenor": "O/N" },
                { "id": "USD-OIS-5Y", "currency": "USD", "convention": "USD-SOFR-OIS", "tenor": "5Y", "rateIndex": "USD-SOFR" }
            ],
            "rateIndices": [
                { "id": "USD-SOFR", "currency": "USD", "indexType": "Sofr", "tenor": "O/N" }
            ],
            "curves": [
                {
                    "name": "USD-SOFR-Discount",
                    "rateIndex": "USD-SOFR",
                    "instruments": ["USD-Depo-ON", "USD-OIS-5Y"]
                }
            ]
        }"#;

        let registry = DefinitionRegistry::load_from_json(json).unwrap();
        assert_eq!(registry.instrument_count(), 2);
        assert_eq!(registry.rate_index_count(), 1);
        assert_eq!(registry.curve_count(), 1);

        let curve = registry.get_curve("USD-SOFR-Discount").unwrap();
        assert_eq!(curve.instruments.len(), 2);

        // Verify rate type is correctly derived from convention
        let depo = registry.get_instrument("USD-Depo-ON").unwrap();
        assert_eq!(depo.rate_type(), RateType::Deposit);
        let ois = registry.get_instrument("USD-OIS-5Y").unwrap();
        assert_eq!(ois.rate_type(), RateType::Ois);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_load_from_json_legacy_format() {
        // Legacy format with rateType field (backwards compatible)
        let json = r#"{
            "instruments": [
                { "id": "USD-Depo-ON", "currency": "USD", "rateTypeOverride": "Deposit", "tenor": "O/N" }
            ],
            "rateIndices": [
                { "id": "USD-SOFR", "currency": "USD", "indexType": "Sofr", "tenor": "O/N" }
            ],
            "curves": [
                {
                    "name": "USD-SOFR-Discount",
                    "rateIndex": "USD-SOFR",
                    "instruments": ["USD-Depo-ON"]
                }
            ]
        }"#;

        let registry = DefinitionRegistry::load_from_json(json).unwrap();
        let depo = registry.get_instrument("USD-Depo-ON").unwrap();
        assert_eq!(depo.rate_type(), RateType::Deposit);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_load_from_json_validation_error() {
        // Missing rate index reference
        let json = r#"{
            "instruments": [
                { "id": "USD-Depo-ON", "currency": "USD", "convention": "USD-DEPO", "tenor": "O/N" }
            ],
            "rateIndices": [],
            "curves": [
                {
                    "name": "USD-SOFR-Discount",
                    "rateIndex": "USD-SOFR",
                    "instruments": ["USD-Depo-ON"]
                }
            ]
        }"#;

        let result = DefinitionRegistry::load_from_json(json);
        assert!(result.is_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_load_from_json_with_templates() {
        let json = r#"{
            "templates": [
                {
                    "idPattern": "{currency}-OIS-{tenor}",
                    "currency": "USD",
                    "convention": "USD-SOFR-OIS",
                    "rateIndex": "USD-SOFR",
                    "tenors": ["1M", "3M", "6M", "1Y", "5Y"]
                },
                {
                    "idPattern": "{currency}-Depo-{tenor}",
                    "currency": "USD",
                    "convention": "USD-DEPO",
                    "rateIndex": "USD-SOFR",
                    "tenors": ["O/N", "1W"]
                }
            ],
            "instruments": [
                { "id": "USD-Custom", "currency": "USD", "convention": "USD-DEPO", "tenor": "2W" }
            ],
            "rateIndices": [
                { "id": "USD-SOFR", "currency": "USD", "indexType": "Sofr", "tenor": "O/N" }
            ],
            "curves": [
                {
                    "name": "USD-SOFR-Discount",
                    "rateIndex": "USD-SOFR",
                    "instruments": ["USD-Depo-O/N", "USD-OIS-1M", "USD-OIS-5Y"]
                }
            ]
        }"#;

        let registry = DefinitionRegistry::load_from_json(json).unwrap();

        // 5 OIS + 2 Depo (from templates) + 1 Custom = 8 instruments
        assert_eq!(registry.instrument_count(), 8);

        // Check template-generated instruments
        let ois_1m = registry.get_instrument("USD-OIS-1M").unwrap();
        assert_eq!(ois_1m.rate_type(), RateType::Ois);
        assert_eq!(ois_1m.rate_index, Some("USD-SOFR".to_string()));

        let ois_5y = registry.get_instrument("USD-OIS-5Y").unwrap();
        assert_eq!(ois_5y.tenor, "5Y");

        let depo_on = registry.get_instrument("USD-Depo-O/N").unwrap();
        assert_eq!(depo_on.rate_type(), RateType::Deposit);

        // Check individual instrument
        let custom = registry.get_instrument("USD-Custom").unwrap();
        assert_eq!(custom.tenor, "2W");

        // Check curve references work
        let curve = registry.get_curve("USD-SOFR-Discount").unwrap();
        assert_eq!(curve.instruments.len(), 3);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_definition_bundle_expand_instruments() {
        use super::InstrumentTemplate;

        let bundle = DefinitionBundle {
            templates: vec![
                InstrumentTemplate::new(
                    "{currency}-OIS-{tenor}",
                    Currency::USD,
                    "USD-SOFR-OIS",
                    vec!["1M".into(), "3M".into(), "6M".into()],
                ),
            ],
            instruments: vec![
                InstrumentDefinition::from_convention(
                    "USD-Custom",
                    Currency::USD,
                    "USD-DEPO",
                    "O/N",
                ),
            ],
            rate_indices: vec![],
            curves: vec![],
        };

        assert_eq!(bundle.total_instrument_count(), 4);

        let expanded = bundle.expand_instruments();
        assert_eq!(expanded.len(), 4);
        assert_eq!(expanded[0].id, "USD-OIS-1M");
        assert_eq!(expanded[1].id, "USD-OIS-3M");
        assert_eq!(expanded[2].id, "USD-OIS-6M");
        assert_eq!(expanded[3].id, "USD-Custom");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_load_from_json_fra_templates() {
        let json = r#"{
            "templates": [
                {
                    "idPattern": "{currency}-FRA-{tenor}",
                    "currency": "USD",
                    "convention": "USD-FRA",
                    "rateIndex": "USD-SOFR",
                    "tenors": ["1x4", "3x6", "6x9"]
                }
            ],
            "rateIndices": [
                { "id": "USD-SOFR", "currency": "USD", "indexType": "Sofr", "tenor": "O/N" }
            ],
            "curves": []
        }"#;

        let registry = DefinitionRegistry::load_from_json(json).unwrap();
        assert_eq!(registry.instrument_count(), 3);

        let fra = registry.get_instrument("USD-FRA-1x4").unwrap();
        assert_eq!(fra.rate_type(), RateType::Fra);
        assert_eq!(fra.tenor, "1x4");
    }
}
