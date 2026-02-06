//! Aggregation hierarchy and configuration structures.
//!
//! This module provides types for hierarchical aggregation of exposure
//! and XVA calculations across different dimensions.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use thiserror::Error;

// ============================================================================
// AggregationHierarchy
// ============================================================================

/// Aggregation hierarchy level.
///
/// Defines the level at which exposure or XVA values are aggregated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AggregationHierarchy {
    /// Individual trade level (no aggregation).
    Trade,
    /// Netting set level.
    #[default]
    NettingSet,
    /// Trading book level.
    Book,
    /// Counterparty level.
    Counterparty,
    /// Legal entity level.
    LegalEntity,
    /// Full portfolio level.
    Portfolio,
}

impl AggregationHierarchy {
    /// Returns the next level up in the hierarchy.
    ///
    /// Returns `None` if already at the top level (Portfolio).
    pub fn parent(&self) -> Option<Self> {
        match self {
            AggregationHierarchy::Trade => Some(AggregationHierarchy::NettingSet),
            AggregationHierarchy::NettingSet => Some(AggregationHierarchy::Book),
            AggregationHierarchy::Book => Some(AggregationHierarchy::Counterparty),
            AggregationHierarchy::Counterparty => Some(AggregationHierarchy::LegalEntity),
            AggregationHierarchy::LegalEntity => Some(AggregationHierarchy::Portfolio),
            AggregationHierarchy::Portfolio => None,
        }
    }

    /// Returns the next level down in the hierarchy.
    ///
    /// Returns `None` if already at the bottom level (Trade).
    pub fn child(&self) -> Option<Self> {
        match self {
            AggregationHierarchy::Portfolio => Some(AggregationHierarchy::LegalEntity),
            AggregationHierarchy::LegalEntity => Some(AggregationHierarchy::Counterparty),
            AggregationHierarchy::Counterparty => Some(AggregationHierarchy::Book),
            AggregationHierarchy::Book => Some(AggregationHierarchy::NettingSet),
            AggregationHierarchy::NettingSet => Some(AggregationHierarchy::Trade),
            AggregationHierarchy::Trade => None,
        }
    }

    /// Returns the depth (0 = Portfolio, higher = more granular).
    pub fn depth(&self) -> u8 {
        match self {
            AggregationHierarchy::Portfolio => 0,
            AggregationHierarchy::LegalEntity => 1,
            AggregationHierarchy::Counterparty => 2,
            AggregationHierarchy::Book => 3,
            AggregationHierarchy::NettingSet => 4,
            AggregationHierarchy::Trade => 5,
        }
    }

    /// Returns true if this level is more granular than the other.
    pub fn is_more_granular_than(&self, other: &Self) -> bool { self.depth() > other.depth() }
}

impl std::fmt::Display for AggregationHierarchy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AggregationHierarchy::Trade => "Trade",
            AggregationHierarchy::NettingSet => "NettingSet",
            AggregationHierarchy::Book => "Book",
            AggregationHierarchy::Counterparty => "Counterparty",
            AggregationHierarchy::LegalEntity => "LegalEntity",
            AggregationHierarchy::Portfolio => "Portfolio",
        };
        write!(f, "{}", s)
    }
}

// ============================================================================
// AggregationMethod
// ============================================================================

/// Aggregation method for combining values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AggregationMethod {
    /// Sum all values.
    #[default]
    Sum,
    /// Take the average.
    Average,
    /// Take the maximum.
    Max,
    /// Take the minimum.
    Min,
    /// Weighted average (requires weights).
    WeightedAverage,
}

impl std::fmt::Display for AggregationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AggregationMethod::Sum => "Sum",
            AggregationMethod::Average => "Average",
            AggregationMethod::Max => "Max",
            AggregationMethod::Min => "Min",
            AggregationMethod::WeightedAverage => "WeightedAverage",
        };
        write!(f, "{}", s)
    }
}

// ============================================================================
// GroupingKey
// ============================================================================

/// Grouping key for multi-dimensional aggregation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GroupingKey {
    /// Group by hierarchy level.
    #[default]
    Hierarchy,
    /// Group by currency.
    Currency,
    /// Group by product type.
    ProductType,
    /// Group by asset class.
    AssetClass,
    /// Group by desk.
    Desk,
    /// Group by region.
    Region,
    /// Custom grouping key.
    Custom(String),
}

impl std::fmt::Display for GroupingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupingKey::Hierarchy => write!(f, "Hierarchy"),
            GroupingKey::Currency => write!(f, "Currency"),
            GroupingKey::ProductType => write!(f, "ProductType"),
            GroupingKey::AssetClass => write!(f, "AssetClass"),
            GroupingKey::Desk => write!(f, "Desk"),
            GroupingKey::Region => write!(f, "Region"),
            GroupingKey::Custom(s) => write!(f, "Custom({})", s),
        }
    }
}

// ============================================================================
// AggregationError
// ============================================================================

/// Errors that can occur during aggregation.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum AggregationError {
    /// Incompatible dimensions for aggregation.
    #[error("Incompatible dimensions: {message}")]
    IncompatibleDimensions {
        /// Error details.
        message: String,
    },

    /// Missing weights for weighted average.
    #[error("Missing weights for weighted average aggregation")]
    MissingWeights,

    /// Invalid hierarchy path.
    #[error("Invalid hierarchy path: {0}")]
    InvalidHierarchyPath(String),

    /// Empty data set for aggregation.
    #[error("Cannot aggregate empty data set")]
    EmptyDataSet,
}

// ============================================================================
// AggregationConfig
// ============================================================================

/// Configuration for aggregation operations.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AggregationConfig {
    /// Target hierarchy level.
    target_level: AggregationHierarchy,
    /// Primary aggregation method.
    method: AggregationMethod,
    /// Grouping keys for multi-dimensional aggregation.
    grouping_keys: Vec<GroupingKey>,
    /// Whether to include sub-totals at intermediate levels.
    include_subtotals: bool,
    /// Whether to preserve original granular data.
    preserve_granular: bool,
}

impl AggregationConfig {
    /// Creates a new aggregation config with default values.
    pub fn new(target_level: AggregationHierarchy) -> Self {
        Self {
            target_level,
            method: AggregationMethod::default(),
            grouping_keys: vec![GroupingKey::Hierarchy],
            include_subtotals: false,
            preserve_granular: false,
        }
    }

    /// Sets the aggregation method.
    pub fn with_method(mut self, method: AggregationMethod) -> Self {
        self.method = method;
        self
    }

    /// Adds a grouping key.
    pub fn add_grouping_key(mut self, key: GroupingKey) -> Self {
        if !self.grouping_keys.contains(&key) {
            self.grouping_keys.push(key);
        }
        self
    }

    /// Sets the grouping keys.
    pub fn with_grouping_keys(mut self, keys: Vec<GroupingKey>) -> Self {
        self.grouping_keys = keys;
        self
    }

    /// Sets whether to include sub-totals.
    pub fn with_include_subtotals(mut self, include: bool) -> Self {
        self.include_subtotals = include;
        self
    }

    /// Sets whether to preserve granular data.
    pub fn with_preserve_granular(mut self, preserve: bool) -> Self {
        self.preserve_granular = preserve;
        self
    }

    /// Returns the target hierarchy level.
    pub fn target_level(&self) -> AggregationHierarchy { self.target_level }

    /// Returns the aggregation method.
    pub fn method(&self) -> AggregationMethod { self.method }

    /// Returns the grouping keys.
    pub fn grouping_keys(&self) -> &[GroupingKey] { &self.grouping_keys }

    /// Returns whether sub-totals are included.
    pub fn include_subtotals(&self) -> bool { self.include_subtotals }

    /// Returns whether granular data is preserved.
    pub fn preserve_granular(&self) -> bool { self.preserve_granular }
}

impl Default for AggregationConfig {
    fn default() -> Self { Self::new(AggregationHierarchy::default()) }
}

// ============================================================================
// DrillDownPath
// ============================================================================

/// Path for drill-down navigation from aggregated to detailed data.
///
/// Represents a hierarchical path through the aggregation structure.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DrillDownPath {
    /// Path segments from top to current level.
    segments: Vec<DrillDownSegment>,
}

impl DrillDownPath {
    /// Creates a new empty path.
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Creates a path starting from a specific level.
    pub fn from_level(level: AggregationHierarchy, id: impl Into<String>) -> Self {
        Self {
            segments: vec![DrillDownSegment {
                level,
                id: id.into(),
            }],
        }
    }

    /// Adds a segment to the path.
    pub fn push(&mut self, level: AggregationHierarchy, id: impl Into<String>) {
        self.segments.push(DrillDownSegment {
            level,
            id: id.into(),
        });
    }

    /// Returns the current level.
    pub fn current_level(&self) -> Option<AggregationHierarchy> {
        self.segments.last().map(|s| s.level)
    }

    /// Returns the current ID.
    pub fn current_id(&self) -> Option<&str> { self.segments.last().map(|s| s.id.as_str()) }

    /// Returns the path segments.
    pub fn segments(&self) -> &[DrillDownSegment] { &self.segments }

    /// Returns the depth of the path.
    pub fn depth(&self) -> usize { self.segments.len() }

    /// Returns whether the path is empty.
    pub fn is_empty(&self) -> bool { self.segments.is_empty() }

    /// Pops the last segment (moves up one level).
    pub fn pop(&mut self) -> Option<DrillDownSegment> { self.segments.pop() }

    /// Returns the parent path (without the last segment).
    pub fn parent(&self) -> Option<Self> {
        if self.segments.len() <= 1 {
            return None;
        }
        let mut parent = self.clone();
        parent.pop();
        Some(parent)
    }
}

impl Default for DrillDownPath {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Display for DrillDownPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path_str = self
            .segments
            .iter()
            .map(|s| format!("{}:{}", s.level, s.id))
            .collect::<Vec<_>>()
            .join("/");
        write!(f, "{}", path_str)
    }
}

// ============================================================================
// DrillDownSegment
// ============================================================================

/// A single segment in a drill-down path.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DrillDownSegment {
    /// Hierarchy level of this segment.
    pub level: AggregationHierarchy,
    /// Identifier at this level.
    pub id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // AggregationHierarchy tests
    // ========================================================================

    #[test]
    fn test_aggregation_hierarchy_default() {
        assert_eq!(
            AggregationHierarchy::default(),
            AggregationHierarchy::NettingSet
        );
    }

    #[test]
    fn test_aggregation_hierarchy_parent() {
        assert_eq!(
            AggregationHierarchy::Trade.parent(),
            Some(AggregationHierarchy::NettingSet)
        );
        assert_eq!(
            AggregationHierarchy::NettingSet.parent(),
            Some(AggregationHierarchy::Book)
        );
        assert_eq!(AggregationHierarchy::Portfolio.parent(), None);
    }

    #[test]
    fn test_aggregation_hierarchy_child() {
        assert_eq!(
            AggregationHierarchy::Portfolio.child(),
            Some(AggregationHierarchy::LegalEntity)
        );
        assert_eq!(
            AggregationHierarchy::NettingSet.child(),
            Some(AggregationHierarchy::Trade)
        );
        assert_eq!(AggregationHierarchy::Trade.child(), None);
    }

    #[test]
    fn test_aggregation_hierarchy_depth() {
        assert_eq!(AggregationHierarchy::Portfolio.depth(), 0);
        assert_eq!(AggregationHierarchy::Trade.depth(), 5);
    }

    #[test]
    fn test_aggregation_hierarchy_granularity() {
        assert!(
            AggregationHierarchy::Trade.is_more_granular_than(&AggregationHierarchy::NettingSet)
        );
        assert!(!AggregationHierarchy::Portfolio
            .is_more_granular_than(&AggregationHierarchy::Counterparty));
    }

    #[test]
    fn test_aggregation_hierarchy_display() {
        assert_eq!(format!("{}", AggregationHierarchy::Trade), "Trade");
        assert_eq!(format!("{}", AggregationHierarchy::Portfolio), "Portfolio");
    }

    // ========================================================================
    // AggregationMethod tests
    // ========================================================================

    #[test]
    fn test_aggregation_method_default() {
        assert_eq!(AggregationMethod::default(), AggregationMethod::Sum);
    }

    #[test]
    fn test_aggregation_method_display() {
        assert_eq!(format!("{}", AggregationMethod::Sum), "Sum");
        assert_eq!(
            format!("{}", AggregationMethod::WeightedAverage),
            "WeightedAverage"
        );
    }

    // ========================================================================
    // GroupingKey tests
    // ========================================================================

    #[test]
    fn test_grouping_key_default() {
        assert_eq!(GroupingKey::default(), GroupingKey::Hierarchy);
    }

    #[test]
    fn test_grouping_key_custom() {
        let key = GroupingKey::Custom("MyKey".to_string());
        assert_eq!(format!("{}", key), "Custom(MyKey)");
    }

    // ========================================================================
    // AggregationError tests
    // ========================================================================

    #[test]
    fn test_aggregation_error_display() {
        let err = AggregationError::IncompatibleDimensions {
            message: "test".to_string(),
        };
        assert!(err.to_string().contains("Incompatible dimensions"));

        let err = AggregationError::MissingWeights;
        assert!(err.to_string().contains("weights"));
    }

    // ========================================================================
    // AggregationConfig tests
    // ========================================================================

    #[test]
    fn test_aggregation_config_new() {
        let config = AggregationConfig::new(AggregationHierarchy::Counterparty);
        assert_eq!(config.target_level(), AggregationHierarchy::Counterparty);
        assert_eq!(config.method(), AggregationMethod::Sum);
        assert!(!config.include_subtotals());
    }

    #[test]
    fn test_aggregation_config_builder() {
        let config = AggregationConfig::new(AggregationHierarchy::Book)
            .with_method(AggregationMethod::Average)
            .add_grouping_key(GroupingKey::Currency)
            .add_grouping_key(GroupingKey::ProductType)
            .with_include_subtotals(true)
            .with_preserve_granular(true);

        assert_eq!(config.target_level(), AggregationHierarchy::Book);
        assert_eq!(config.method(), AggregationMethod::Average);
        assert_eq!(config.grouping_keys().len(), 3); // Hierarchy + 2 added
        assert!(config.include_subtotals());
        assert!(config.preserve_granular());
    }

    #[test]
    fn test_aggregation_config_dedup_keys() {
        let config = AggregationConfig::new(AggregationHierarchy::Portfolio)
            .add_grouping_key(GroupingKey::Currency)
            .add_grouping_key(GroupingKey::Currency); // Duplicate

        assert_eq!(config.grouping_keys().len(), 2); // Hierarchy + Currency
    }

    // ========================================================================
    // DrillDownPath tests
    // ========================================================================

    #[test]
    fn test_drill_down_path_new() {
        let path = DrillDownPath::new();
        assert!(path.is_empty());
        assert_eq!(path.depth(), 0);
    }

    #[test]
    fn test_drill_down_path_from_level() {
        let path = DrillDownPath::from_level(AggregationHierarchy::Portfolio, "GLOBAL");
        assert_eq!(path.depth(), 1);
        assert_eq!(path.current_level(), Some(AggregationHierarchy::Portfolio));
        assert_eq!(path.current_id(), Some("GLOBAL"));
    }

    #[test]
    fn test_drill_down_path_push_pop() {
        let mut path = DrillDownPath::new();
        path.push(AggregationHierarchy::Portfolio, "GLOBAL");
        path.push(AggregationHierarchy::Counterparty, "CP001");
        path.push(AggregationHierarchy::NettingSet, "NS001");

        assert_eq!(path.depth(), 3);
        assert_eq!(path.current_level(), Some(AggregationHierarchy::NettingSet));

        let popped = path.pop();
        assert_eq!(popped.as_ref().map(|s| s.id.as_str()), Some("NS001"));
        assert_eq!(path.depth(), 2);
    }

    #[test]
    fn test_drill_down_path_parent() {
        let mut path = DrillDownPath::new();
        path.push(AggregationHierarchy::Portfolio, "GLOBAL");
        path.push(AggregationHierarchy::Counterparty, "CP001");

        let parent = path.parent().unwrap();
        assert_eq!(parent.depth(), 1);
        assert_eq!(parent.current_id(), Some("GLOBAL"));
    }

    #[test]
    fn test_drill_down_path_display() {
        let mut path = DrillDownPath::new();
        path.push(AggregationHierarchy::Portfolio, "GLOBAL");
        path.push(AggregationHierarchy::Counterparty, "CP001");

        let display = format!("{}", path);
        assert!(display.contains("Portfolio:GLOBAL"));
        assert!(display.contains("Counterparty:CP001"));
    }
}
