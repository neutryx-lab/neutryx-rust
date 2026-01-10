//! Metrics store for risk dashboard.

use super::{CounterpartyRisk, MetricType, MetricUpdate, RiskSummary};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory metrics store for the risk dashboard
pub struct MetricsStore {
    /// Current metrics by entity and type
    metrics: Arc<RwLock<HashMap<(String, MetricType), MetricUpdate>>>,
    /// Historical metrics (last N updates per entity/type)
    history: Arc<RwLock<HashMap<(String, MetricType), Vec<MetricUpdate>>>>,
    /// Maximum history per metric
    max_history: usize,
}

impl MetricsStore {
    /// Create a new metrics store
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(HashMap::new())),
            max_history: 1000,
        }
    }

    /// Set maximum history per metric
    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    /// Update a metric
    pub fn update(&self, metric: MetricUpdate) {
        let key = (metric.entity_id.clone(), metric.metric_type);

        // Update current value
        {
            let mut metrics = self.metrics.write().unwrap();
            metrics.insert(key.clone(), metric.clone());
        }

        // Update history
        {
            let mut history = self.history.write().unwrap();
            let entry = history.entry(key).or_default();
            if entry.len() >= self.max_history {
                entry.remove(0);
            }
            entry.push(metric);
        }
    }

    /// Update multiple metrics
    pub fn update_batch(&self, metrics: Vec<MetricUpdate>) {
        for metric in metrics {
            self.update(metric);
        }
    }

    /// Get current metric value
    pub fn get(&self, entity_id: &str, metric_type: MetricType) -> Option<MetricUpdate> {
        let metrics = self.metrics.read().unwrap();
        metrics.get(&(entity_id.to_string(), metric_type)).cloned()
    }

    /// Get all metrics for an entity
    pub fn get_entity_metrics(&self, entity_id: &str) -> Vec<MetricUpdate> {
        let metrics = self.metrics.read().unwrap();
        metrics
            .iter()
            .filter(|(k, _)| k.0 == entity_id)
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Get all metrics of a specific type
    pub fn get_metrics_by_type(&self, metric_type: MetricType) -> Vec<MetricUpdate> {
        let metrics = self.metrics.read().unwrap();
        metrics
            .iter()
            .filter(|(k, _)| k.1 == metric_type)
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Get metric history
    pub fn get_history(&self, entity_id: &str, metric_type: MetricType) -> Vec<MetricUpdate> {
        let history = self.history.read().unwrap();
        history
            .get(&(entity_id.to_string(), metric_type))
            .cloned()
            .unwrap_or_default()
    }

    /// Generate a risk summary
    pub fn generate_summary(&self) -> RiskSummary {
        let metrics = self.metrics.read().unwrap();

        // Aggregate by metric type
        let mut total_cva = 0.0;
        let mut total_dva = 0.0;
        let mut total_fva = 0.0;
        let mut var_99_1d = 0.0;
        let mut max_epe = 0.0;
        let mut portfolio_value = 0.0;

        // Track counterparty metrics
        let mut cp_exposure: HashMap<String, f64> = HashMap::new();
        let mut cp_cva: HashMap<String, f64> = HashMap::new();
        let mut cp_pfe: HashMap<String, f64> = HashMap::new();

        for ((entity_id, metric_type), update) in metrics.iter() {
            match metric_type {
                MetricType::CVA => {
                    total_cva += update.value;
                    *cp_cva.entry(entity_id.clone()).or_insert(0.0) += update.value;
                }
                MetricType::DVA => {
                    total_dva += update.value;
                }
                MetricType::FVA => {
                    total_fva += update.value;
                }
                MetricType::VaR => {
                    var_99_1d += update.value;
                }
                MetricType::EPE => {
                    if update.value > max_epe {
                        max_epe = update.value;
                    }
                    *cp_exposure.entry(entity_id.clone()).or_insert(0.0) += update.value;
                }
                MetricType::PFE => {
                    *cp_pfe.entry(entity_id.clone()).or_insert(0.0) += update.value;
                }
                MetricType::PnL => {
                    portfolio_value += update.value;
                }
                _ => {}
            }
        }

        // Build counterparty summaries
        let mut by_counterparty: Vec<CounterpartyRisk> = cp_exposure
            .keys()
            .map(|cp_id| CounterpartyRisk {
                counterparty_id: cp_id.clone(),
                exposure: *cp_exposure.get(cp_id).unwrap_or(&0.0),
                cva: *cp_cva.get(cp_id).unwrap_or(&0.0),
                pfe_95: *cp_pfe.get(cp_id).unwrap_or(&0.0),
            })
            .collect();

        by_counterparty.sort_by(|a, b| b.exposure.partial_cmp(&a.exposure).unwrap());

        RiskSummary {
            portfolio_value,
            total_cva,
            total_dva,
            total_fva,
            var_99_1d,
            max_epe,
            by_counterparty,
            as_of: Utc::now(),
        }
    }

    /// Get store statistics
    pub fn statistics(&self) -> StoreStatistics {
        let metrics = self.metrics.read().unwrap();
        let history = self.history.read().unwrap();

        StoreStatistics {
            current_metric_count: metrics.len(),
            history_entries: history.values().map(|v| v.len()).sum(),
            unique_entities: metrics.keys().map(|(e, _)| e.clone()).collect::<std::collections::HashSet<_>>().len(),
        }
    }

    /// Clear all metrics
    pub fn clear(&self) {
        let mut metrics = self.metrics.write().unwrap();
        let mut history = self.history.write().unwrap();
        metrics.clear();
        history.clear();
    }
}

impl Default for MetricsStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Store statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStatistics {
    /// Current metric count
    pub current_metric_count: usize,
    /// Total history entries
    pub history_entries: usize,
    /// Unique entities
    pub unique_entities: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_store() {
        let store = MetricsStore::new();
        store.update(MetricUpdate {
            metric_type: MetricType::CVA,
            entity_id: "CP001".to_string(),
            value: 1000.0,
            currency: "USD".to_string(),
            confidence: None,
            horizon_days: None,
            timestamp: Utc::now(),
        });

        let metric = store.get("CP001", MetricType::CVA);
        assert!(metric.is_some());
        assert!((metric.unwrap().value - 1000.0).abs() < 0.01);
    }
}
