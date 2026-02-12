//! Data source identification and priority.

use std::{cmp::Ordering, fmt};

/// Identification of market data sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum DataSource {
    /// Reuters/Refinitiv market data.
    Reuters,
    /// Bloomberg market data.
    Bloomberg,
    /// Internally computed or sourced data.
    Internal,
    /// Manually entered data.
    Manual,
}

impl DataSource {
    /// Returns a short code for this data source.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            DataSource::Reuters => "RTR",
            DataSource::Bloomberg => "BBG",
            DataSource::Internal => "INT",
            DataSource::Manual => "MAN",
        }
    }

    /// Returns the full name of this data source.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            DataSource::Reuters => "Reuters",
            DataSource::Bloomberg => "Bloomberg",
            DataSource::Internal => "Internal",
            DataSource::Manual => "Manual",
        }
    }
}

impl fmt::Display for DataSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.name()) }
}

/// Priority ordering for data sources.
#[derive(Debug, Clone)]
pub struct SourcePriority {
    /// Priority order (first = highest priority).
    priorities: Vec<DataSource>,
}

impl SourcePriority {
    /// Creates a new `SourcePriority` with the given order.
    #[must_use]
    pub fn new(priorities: Vec<DataSource>) -> Self { Self { priorities } }

    /// Creates the default priority order.
    #[must_use]
    pub fn default_priority() -> Self {
        Self {
            priorities: vec![
                DataSource::Bloomberg,
                DataSource::Reuters,
                DataSource::Internal,
                DataSource::Manual,
            ],
        }
    }

    /// Compares two data sources by priority.
    #[must_use]
    pub fn compare(&self, a: DataSource, b: DataSource) -> Ordering {
        let pos_a = self.position(a);
        let pos_b = self.position(b);
        pos_a.cmp(&pos_b)
    }

    /// Returns the position of a source in the priority list.
    fn position(&self, source: DataSource) -> usize {
        self.priorities
            .iter()
            .position(|&s| s == source)
            .unwrap_or(usize::MAX)
    }

    /// Returns true if `a` has higher priority than `b`.
    #[must_use]
    pub fn is_higher_priority(&self, a: DataSource, b: DataSource) -> bool {
        self.compare(a, b) == Ordering::Less
    }

    /// Returns the source list in priority order.
    #[must_use]
    pub fn sources(&self) -> &[DataSource] { &self.priorities }
}

impl Default for SourcePriority {
    fn default() -> Self { Self::default_priority() }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[test]
    fn test_data_source_variants() {
        let all_sources = [
            DataSource::Reuters,
            DataSource::Bloomberg,
            DataSource::Internal,
            DataSource::Manual,
        ];
        assert_eq!(all_sources.len(), 4);
    }

    #[test]
    fn test_data_source_code() {
        assert_eq!(DataSource::Reuters.code(), "RTR");
        assert_eq!(DataSource::Bloomberg.code(), "BBG");
        assert_eq!(DataSource::Internal.code(), "INT");
        assert_eq!(DataSource::Manual.code(), "MAN");
    }

    #[test]
    fn test_data_source_name() {
        assert_eq!(DataSource::Reuters.name(), "Reuters");
        assert_eq!(DataSource::Bloomberg.name(), "Bloomberg");
        assert_eq!(DataSource::Internal.name(), "Internal");
        assert_eq!(DataSource::Manual.name(), "Manual");
    }

    #[test]
    fn test_data_source_display() {
        assert_eq!(format!("{}", DataSource::Bloomberg), "Bloomberg");
        assert_eq!(format!("{}", DataSource::Reuters), "Reuters");
    }

    #[test]
    fn test_data_source_copy() {
        let original = DataSource::Bloomberg;
        let copied = original;
        assert_eq!(original, copied);
    }

    #[test]
    fn test_data_source_clone() {
        let original = DataSource::Reuters;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_data_source_eq() {
        assert_eq!(DataSource::Bloomberg, DataSource::Bloomberg);
        assert_ne!(DataSource::Bloomberg, DataSource::Reuters);
    }

    #[test]
    fn test_data_source_hash() {
        let mut set = HashSet::new();
        set.insert(DataSource::Bloomberg);
        set.insert(DataSource::Reuters);
        set.insert(DataSource::Bloomberg);

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_data_source_as_hashmap_key() {
        let mut map: HashMap<DataSource, &str> = HashMap::new();
        map.insert(DataSource::Bloomberg, "Primary");
        map.insert(DataSource::Reuters, "Secondary");

        assert_eq!(map.get(&DataSource::Bloomberg), Some(&"Primary"));
        assert_eq!(map.get(&DataSource::Reuters), Some(&"Secondary"));
    }

    #[test]
    fn test_data_source_debug() {
        assert_eq!(format!("{:?}", DataSource::Bloomberg), "Bloomberg");
        assert_eq!(format!("{:?}", DataSource::Reuters), "Reuters");
    }

    #[test]
    fn test_source_priority_new() {
        let priority = SourcePriority::new(vec![
            DataSource::Internal,
            DataSource::Bloomberg,
            DataSource::Reuters,
        ]);

        assert_eq!(priority.sources().len(), 3);
    }

    #[test]
    fn test_source_priority_default() {
        let priority = SourcePriority::default_priority();

        assert_eq!(priority.sources().len(), 4);
        assert_eq!(priority.sources()[0], DataSource::Bloomberg);
        assert_eq!(priority.sources()[1], DataSource::Reuters);
        assert_eq!(priority.sources()[2], DataSource::Internal);
        assert_eq!(priority.sources()[3], DataSource::Manual);
    }

    #[test]
    fn test_source_priority_default_trait() {
        let priority = SourcePriority::default();

        assert_eq!(priority.sources().len(), 4);
        assert_eq!(priority.sources()[0], DataSource::Bloomberg);
    }

    #[test]
    fn test_source_priority_compare() {
        let priority = SourcePriority::default_priority();

        assert_eq!(
            priority.compare(DataSource::Bloomberg, DataSource::Reuters),
            Ordering::Less
        );

        assert_eq!(
            priority.compare(DataSource::Reuters, DataSource::Bloomberg),
            Ordering::Greater
        );

        assert_eq!(
            priority.compare(DataSource::Bloomberg, DataSource::Bloomberg),
            Ordering::Equal
        );
    }

    #[test]
    fn test_source_priority_compare_all_pairs() {
        let priority = SourcePriority::default_priority();

        assert_eq!(
            priority.compare(DataSource::Bloomberg, DataSource::Reuters),
            Ordering::Less
        );
        assert_eq!(
            priority.compare(DataSource::Bloomberg, DataSource::Internal),
            Ordering::Less
        );
        assert_eq!(
            priority.compare(DataSource::Bloomberg, DataSource::Manual),
            Ordering::Less
        );

        assert_eq!(
            priority.compare(DataSource::Reuters, DataSource::Internal),
            Ordering::Less
        );
        assert_eq!(
            priority.compare(DataSource::Reuters, DataSource::Manual),
            Ordering::Less
        );

        assert_eq!(
            priority.compare(DataSource::Internal, DataSource::Manual),
            Ordering::Less
        );
    }

    #[test]
    fn test_source_priority_is_higher_priority() {
        let priority = SourcePriority::default_priority();

        assert!(priority.is_higher_priority(DataSource::Bloomberg, DataSource::Reuters));
        assert!(priority.is_higher_priority(DataSource::Reuters, DataSource::Internal));
        assert!(priority.is_higher_priority(DataSource::Internal, DataSource::Manual));

        assert!(!priority.is_higher_priority(DataSource::Reuters, DataSource::Bloomberg));
        assert!(!priority.is_higher_priority(DataSource::Manual, DataSource::Internal));
    }

    #[test]
    fn test_source_priority_custom_order() {
        let priority = SourcePriority::new(vec![
            DataSource::Internal,
            DataSource::Manual,
            DataSource::Bloomberg,
            DataSource::Reuters,
        ]);

        assert!(priority.is_higher_priority(DataSource::Internal, DataSource::Manual));
        assert!(priority.is_higher_priority(DataSource::Manual, DataSource::Bloomberg));
        assert!(priority.is_higher_priority(DataSource::Bloomberg, DataSource::Reuters));
    }

    #[test]
    fn test_source_priority_clone() {
        let original = SourcePriority::default_priority();
        let cloned = original.clone();

        assert_eq!(original.sources(), cloned.sources());
    }

    #[test]
    fn test_source_priority_debug() {
        let priority = SourcePriority::default_priority();
        let debug_str = format!("{:?}", priority);
        assert!(debug_str.contains("SourcePriority"));
        assert!(debug_str.contains("Bloomberg"));
    }
}
