//! Definition registry for curve construction.

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
use super::definition::InstrumentTemplate;
use super::definition::{
    CurveDefError, CurveDefinition, InstrumentDefError, InstrumentDefinition, RateIndexDefError,
    RateIndexDefinition,
};

/// Error type for registry operations.
#[derive(Debug, Clone)]
pub enum RegistryError {
    /// Instrument definition error.
    Instrument(InstrumentDefError),
    /// Rate index definition error.
    RateIndex(RateIndexDefError),
    /// Curve definition error.
    Curve(CurveDefError),
    /// Duplicate ID.
    DuplicateId {
        /// Entity type (e.g., "instrument", "curve").
        entity: &'static str,
        /// The duplicate ID.
        id: String,
    },
    /// Reference not found.
    ReferenceNotFound {
        /// Source entity that has the reference.
        from: String,
        /// Target ID that was not found.
        to: String,
        /// Target entity type.
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
                write!(
                    f,
                    "{} '{}' references unknown {}: '{}'",
                    entity, from, entity, to
                )
            }
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<InstrumentDefError> for RegistryError {
    fn from(e: InstrumentDefError) -> Self { Self::Instrument(e) }
}

impl From<RateIndexDefError> for RegistryError {
    fn from(e: RateIndexDefError) -> Self { Self::RateIndex(e) }
}

impl From<CurveDefError> for RegistryError {
    fn from(e: CurveDefError) -> Self { Self::Curve(e) }
}

/// Registry for curve construction definitions.
#[derive(Debug, Clone, Default)]
pub struct DefinitionRegistry {
    instruments: HashMap<String, InstrumentDefinition>,
    rate_indices: HashMap<String, RateIndexDefinition>,
    curves: HashMap<String, CurveDefinition>,
}

/// JSON-serializable bundle of all definitions for loading.
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionBundle {
    /// Instrument templates for bulk generation.
    #[serde(default)]
    pub templates: Vec<InstrumentTemplate>,

    /// Individual instrument definitions.
    #[serde(default)]
    pub instruments: Vec<InstrumentDefinition>,

    /// Rate index definitions.
    #[serde(default, rename = "rateIndices")]
    pub rate_indices: Vec<RateIndexDefinition>,

    /// Curve definitions.
    #[serde(default)]
    pub curves: Vec<CurveDefinition>,
}

#[cfg(feature = "serde")]
impl DefinitionBundle {
    /// Expands all templates and returns the combined list of instruments.
    #[must_use]
    pub fn expand_instruments(&self) -> Vec<InstrumentDefinition> {
        let mut result = Vec::new();

        for template in &self.templates {
            result.extend(template.expand());
        }

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
    pub fn new() -> Self { Self::default() }

    /// Registers an instrument definition.
    pub fn register_instrument(&mut self, def: InstrumentDefinition) -> Result<(), RegistryError> {
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
    pub fn register_rate_index(&mut self, def: RateIndexDefinition) -> Result<(), RegistryError> {
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
    pub fn register_curve(&mut self, def: CurveDefinition) -> Result<(), RegistryError> {
        def.validate()?;

        if self.curves.contains_key(&def.name) {
            return Err(RegistryError::DuplicateId {
                entity: "curve",
                id: def.name.clone(),
            });
        }

        if !self.rate_indices.contains_key(&def.rate_index) {
            return Err(RegistryError::ReferenceNotFound {
                from: def.name.clone(),
                to: def.rate_index.clone(),
                entity: "rate_index",
            });
        }

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
    pub fn get_curve(&self, name: &str) -> Option<&CurveDefinition> { self.curves.get(name) }

    /// Returns the number of registered instruments.
    #[must_use]
    pub fn instrument_count(&self) -> usize { self.instruments.len() }

    /// Returns the number of registered rate indices.
    #[must_use]
    pub fn rate_index_count(&self) -> usize { self.rate_indices.len() }

    /// Returns the number of registered curves.
    #[must_use]
    pub fn curve_count(&self) -> usize { self.curves.len() }

    /// Returns an iterator over all instrument definitions.
    pub fn instruments(&self) -> impl Iterator<Item = &InstrumentDefinition> {
        self.instruments.values()
    }

    /// Returns an iterator over all rate index definitions.
    pub fn rate_indices(&self) -> impl Iterator<Item = &RateIndexDefinition> {
        self.rate_indices.values()
    }

    /// Returns an iterator over all curve definitions.
    pub fn curves(&self) -> impl Iterator<Item = &CurveDefinition> { self.curves.values() }

    /// Gets the instrument definitions for a curve.
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
    #[must_use]
    pub fn curve_rate_index(&self, curve_name: &str) -> Option<&RateIndexDefinition> {
        let curve = self.curves.get(curve_name)?;
        self.rate_indices.get(&curve.rate_index)
    }

    /// Loads definitions from a JSON bundle.
    #[cfg(feature = "serde")]
    pub fn load_bundle(&mut self, bundle: DefinitionBundle) -> Result<(), RegistryError> {
        for template in &bundle.templates {
            template.validate()?;
            for inst in template.expand() {
                self.register_instrument(inst)?;
            }
        }

        for inst in bundle.instruments {
            self.register_instrument(inst)?;
        }

        for idx in bundle.rate_indices {
            self.register_rate_index(idx)?;
        }

        for curve in bundle.curves {
            self.register_curve(curve)?;
        }

        Ok(())
    }

    /// Loads definitions from a JSON string.
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
    use crate::market::{Currency, RateIndex, QuoteCategory};

    fn inst(id: &str) -> InstrumentDefinition {
        InstrumentDefinition::new(id, Currency::USD, QuoteCategory::Deposit, "ON")
    }
    fn idx(id: &str) -> RateIndexDefinition {
        RateIndexDefinition::new(id, Currency::USD, RateIndex::Sofr)
    }

    #[test]
    fn test_registry_operations() {
        let mut r = DefinitionRegistry::new();
        assert_eq!(r.instrument_count(), 0);
        assert_eq!(r.rate_index_count(), 0);
        assert_eq!(r.curve_count(), 0);

        assert!(r.register_instrument(inst("USD-Depo-ON")).is_ok());
        assert_eq!(r.instrument_count(), 1);
        assert!(r.get_instrument("USD-Depo-ON").is_some());
        assert!(matches!(
            r.register_instrument(inst("USD-Depo-ON")),
            Err(RegistryError::DuplicateId { .. })
        ));

        assert!(r.register_rate_index(idx("USD-SOFR")).is_ok());
        assert_eq!(r.rate_index_count(), 1);
        assert!(r.get_rate_index("USD-SOFR").is_some());

        let curve = CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string()],
        );
        assert!(r.register_curve(curve).is_ok());
        assert_eq!(r.curve_count(), 1);

        let ri = r.curve_rate_index("USD-SOFR-Discount").unwrap();
        assert_eq!(ri.id, "USD-SOFR");

        let mut r2 = DefinitionRegistry::new();
        r2.register_instrument(inst("USD-Depo-ON")).unwrap();
        assert!(matches!(
            r2.register_curve(CurveDefinition::new(
                "c",
                "USD-SOFR",
                vec!["USD-Depo-ON".into()]
            )),
            Err(RegistryError::ReferenceNotFound {
                entity: "rate_index",
                ..
            })
        ));

        let mut r3 = DefinitionRegistry::new();
        r3.register_rate_index(idx("USD-SOFR")).unwrap();
        assert!(matches!(
            r3.register_curve(CurveDefinition::new(
                "c",
                "USD-SOFR",
                vec!["USD-Depo-ON".into()]
            )),
            Err(RegistryError::ReferenceNotFound {
                entity: "instrument",
                ..
            })
        ));

        let mut r4 = DefinitionRegistry::new();
        r4.register_instrument(inst("USD-Depo-ON")).unwrap();
        r4.register_instrument(InstrumentDefinition::new(
            "USD-OIS-5Y",
            Currency::USD,
            QuoteCategory::Ois,
            "5Y",
        ))
        .unwrap();
        r4.register_rate_index(idx("USD-SOFR")).unwrap();
        r4.register_curve(CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR",
            vec!["USD-Depo-ON".into(), "USD-OIS-5Y".into()],
        ))
        .unwrap();
        let insts = r4.curve_instruments("USD-SOFR-Discount").unwrap();
        assert_eq!(insts.len(), 2);
        assert_eq!(insts[0].id, "USD-Depo-ON");
        assert_eq!(insts[1].id, "USD-OIS-5Y");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_registry_json() {
        let json = r#"{"instruments":[
            {"id":"USD-Depo-ON","currency":"USD","convention":"USD-DEPO","tenor":"ON"},
            {"id":"USD-OIS-5Y","currency":"USD","convention":"USD-SOFR-OIS","tenor":"5Y","rateIndex":"USD-SOFR"}],
            "rateIndices":[{"id":"USD-SOFR","currency":"USD","indexType":"Sofr","tenor":"ON"}],
            "curves":[{"name":"USD-SOFR-Discount","rateIndex":"USD-SOFR","instruments":["USD-Depo-ON","USD-OIS-5Y"]}]}"#;
        let r = DefinitionRegistry::load_from_json(json).unwrap();
        assert_eq!(r.instrument_count(), 2);
        assert_eq!(r.rate_index_count(), 1);
        assert_eq!(r.curve_count(), 1);
        assert_eq!(
            r.get_instrument("USD-Depo-ON").unwrap().quote_category(),
            QuoteCategory::Deposit
        );
        assert_eq!(
            r.get_instrument("USD-OIS-5Y").unwrap().quote_category(),
            QuoteCategory::Ois
        );

        let legacy = r#"{"instruments":[{"id":"USD-Depo-ON","currency":"USD","quoteCategoryOverride":"Deposit","tenor":"ON"}],
            "rateIndices":[{"id":"USD-SOFR","currency":"USD","indexType":"Sofr","tenor":"ON"}],
            "curves":[{"name":"c","rateIndex":"USD-SOFR","instruments":["USD-Depo-ON"]}]}"#;
        assert_eq!(
            DefinitionRegistry::load_from_json(legacy)
                .unwrap()
                .get_instrument("USD-Depo-ON")
                .unwrap()
                .quote_category(),
            QuoteCategory::Deposit
        );

        let bad = r#"{"instruments":[{"id":"USD-Depo-ON","currency":"USD","convention":"USD-DEPO","tenor":"ON"}],
            "rateIndices":[],"curves":[{"name":"c","rateIndex":"USD-SOFR","instruments":["USD-Depo-ON"]}]}"#;
        assert!(DefinitionRegistry::load_from_json(bad).is_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_registry_templates() {
        let json = r#"{"templates":[
            {"idPattern":"{currency}-OIS-{tenor}","currency":"USD","convention":"USD-SOFR-OIS","rateIndex":"USD-SOFR","tenors":["1M","3M","6M","1Y","5Y"]},
            {"idPattern":"{currency}-Depo-{tenor}","currency":"USD","convention":"USD-DEPO","rateIndex":"USD-SOFR","tenors":["ON","1W"]}],
            "instruments":[{"id":"USD-Custom","currency":"USD","convention":"USD-DEPO","tenor":"2W"}],
            "rateIndices":[{"id":"USD-SOFR","currency":"USD","indexType":"Sofr","tenor":"ON"}],
            "curves":[{"name":"USD-SOFR-Discount","rateIndex":"USD-SOFR","instruments":["USD-Depo-ON","USD-OIS-1M","USD-OIS-5Y"]}]}"#;
        let r = DefinitionRegistry::load_from_json(json).unwrap();
        assert_eq!(r.instrument_count(), 8);
        assert_eq!(
            r.get_instrument("USD-OIS-1M").unwrap().quote_category(),
            QuoteCategory::Ois
        );
        assert_eq!(r.get_instrument("USD-OIS-5Y").unwrap().tenor, "5Y");
        assert_eq!(
            r.get_instrument("USD-Depo-ON").unwrap().quote_category(),
            QuoteCategory::Deposit
        );
        assert_eq!(r.get_instrument("USD-Custom").unwrap().tenor, "2W");
        assert_eq!(
            r.get_curve("USD-SOFR-Discount").unwrap().instruments.len(),
            3
        );

        let bundle = DefinitionBundle {
            templates: vec![InstrumentTemplate::new(
                "{currency}-OIS-{tenor}",
                Currency::USD,
                "USD-SOFR-OIS",
                vec!["1M".into(), "3M".into(), "6M".into()],
            )],
            instruments: vec![InstrumentDefinition::from_convention(
                "USD-Custom",
                Currency::USD,
                "USD-DEPO",
                "ON",
            )],
            rate_indices: vec![],
            curves: vec![],
        };
        assert_eq!(bundle.total_instrument_count(), 4);
        let expanded = bundle.expand_instruments();
        assert_eq!(expanded.len(), 4);
        assert_eq!(expanded[0].id, "USD-OIS-1M");

        let fra_json = r#"{"templates":[{"idPattern":"{currency}-FRA-{tenor}","currency":"USD","convention":"USD-FRA","rateIndex":"USD-SOFR","tenors":["1x4","3x6","6x9"]}],
            "rateIndices":[{"id":"USD-SOFR","currency":"USD","indexType":"Sofr","tenor":"ON"}],"curves":[]}"#;
        let r2 = DefinitionRegistry::load_from_json(fra_json).unwrap();
        assert_eq!(r2.instrument_count(), 3);
        assert_eq!(
            r2.get_instrument("USD-FRA-1x4").unwrap().quote_category(),
            QuoteCategory::Fra
        );
    }
}
