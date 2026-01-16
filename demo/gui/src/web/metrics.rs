//! Prometheus metrics module for FrictionalBank demo.
//!
//! Provides Prometheus-compatible metrics endpoint for monitoring:
//! - Counter: API request counts, error counts
//! - Histogram: Response times
//! - Gauge: Active WebSocket connections, memory usage
//!
//! ## Requirements Coverage
//!
//! - Requirement 8.1: 計算時間、メモリ使用量、キャッシュヒット率のメトリクス収集・表示
//! - Requirement 8.2: `/metrics` エンドポイントで Prometheus 形式のメトリクス出力
//! - Requirement 8.3: エラー発生件数とエラータイプの統計表示
//! - Requirement 8.4: API レスポンスタイム閾値超過時の警告ログ出力

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::warn;

// =========================================================================
// Task 9.1: Prometheus Metrics Data Structures
// =========================================================================

/// Response time threshold in milliseconds for warning logs (Requirement 8.4)
pub const RESPONSE_TIME_THRESHOLD_MS: u64 = 500;

/// Counter metric for tracking request counts and errors
#[derive(Debug)]
pub struct Counter {
    value: AtomicU64,
    name: String,
    help: String,
    labels: Vec<(String, String)>,
}

impl Counter {
    /// Create a new counter with the given name and help text
    pub fn new(name: &str, help: &str) -> Self {
        Self {
            value: AtomicU64::new(0),
            name: name.to_string(),
            help: help.to_string(),
            labels: Vec::new(),
        }
    }

    /// Create a counter with labels
    pub fn with_labels(name: &str, help: &str, labels: Vec<(&str, &str)>) -> Self {
        Self {
            value: AtomicU64::new(0),
            name: name.to_string(),
            help: help.to_string(),
            labels: labels
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    /// Increment the counter by 1
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the counter by a specific value
    pub fn inc_by(&self, v: u64) {
        self.value.fetch_add(v, Ordering::Relaxed);
    }

    /// Get the current value
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset the counter to 0
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }

    /// Format as Prometheus text
    pub fn to_prometheus(&self) -> String {
        let labels_str = if self.labels.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> = self
                .labels
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect();
            format!("{{{}}}", pairs.join(","))
        };
        format!(
            "# HELP {} {}\n# TYPE {} counter\n{}{} {}\n",
            self.name,
            self.help,
            self.name,
            self.name,
            labels_str,
            self.get()
        )
    }
}

/// Gauge metric for current values (connections, memory)
#[derive(Debug)]
pub struct Gauge {
    value: AtomicU64,
    name: String,
    help: String,
    labels: Vec<(String, String)>,
}

impl Gauge {
    /// Create a new gauge with the given name and help text
    pub fn new(name: &str, help: &str) -> Self {
        Self {
            value: AtomicU64::new(0),
            name: name.to_string(),
            help: help.to_string(),
            labels: Vec::new(),
        }
    }

    /// Create a gauge with labels
    pub fn with_labels(name: &str, help: &str, labels: Vec<(&str, &str)>) -> Self {
        Self {
            value: AtomicU64::new(0),
            name: name.to_string(),
            help: help.to_string(),
            labels: labels
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    /// Set the gauge value
    pub fn set(&self, v: u64) {
        self.value.store(v, Ordering::Relaxed);
    }

    /// Increment the gauge
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement the gauge
    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get the current value
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Format as Prometheus text
    pub fn to_prometheus(&self) -> String {
        let labels_str = if self.labels.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> = self
                .labels
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect();
            format!("{{{}}}", pairs.join(","))
        };
        format!(
            "# HELP {} {}\n# TYPE {} gauge\n{}{} {}\n",
            self.name,
            self.help,
            self.name,
            self.name,
            labels_str,
            self.get()
        )
    }
}

/// Histogram metric for response time distribution
#[derive(Debug)]
pub struct Histogram {
    name: String,
    help: String,
    /// Bucket boundaries in milliseconds
    buckets: Vec<f64>,
    /// Count of observations per bucket
    bucket_counts: Vec<AtomicU64>,
    /// Total sum of all observations
    sum: AtomicU64,
    /// Total count of observations
    count: AtomicU64,
}

impl Histogram {
    /// Default bucket boundaries for response times (in milliseconds)
    pub const DEFAULT_BUCKETS: [f64; 11] = [
        5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
    ];

    /// Create a new histogram with default buckets
    pub fn new(name: &str, help: &str) -> Self {
        Self::with_buckets(name, help, Self::DEFAULT_BUCKETS.to_vec())
    }

    /// Create a histogram with custom buckets
    pub fn with_buckets(name: &str, help: &str, mut buckets: Vec<f64>) -> Self {
        buckets.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let bucket_counts = (0..buckets.len() + 1) // +1 for +Inf
            .map(|_| AtomicU64::new(0))
            .collect();
        Self {
            name: name.to_string(),
            help: help.to_string(),
            buckets,
            bucket_counts,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    /// Observe a value (in milliseconds)
    pub fn observe(&self, value_ms: f64) {
        // Update sum and count
        self.sum
            .fetch_add((value_ms * 1000.0) as u64, Ordering::Relaxed); // Store as microseconds for precision
        self.count.fetch_add(1, Ordering::Relaxed);

        // Update bucket counts
        for (i, &bound) in self.buckets.iter().enumerate() {
            if value_ms <= bound {
                self.bucket_counts[i].fetch_add(1, Ordering::Relaxed);
            }
        }
        // Always increment +Inf bucket
        self.bucket_counts
            .last()
            .unwrap()
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get the sum of all observations (in milliseconds)
    pub fn sum(&self) -> f64 {
        self.sum.load(Ordering::Relaxed) as f64 / 1000.0
    }

    /// Get the total count of observations
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Format as Prometheus text
    pub fn to_prometheus(&self) -> String {
        let mut output = format!(
            "# HELP {} {}\n# TYPE {} histogram\n",
            self.name, self.help, self.name
        );

        // Output bucket counts
        let mut cumulative = 0u64;
        for (i, &bound) in self.buckets.iter().enumerate() {
            cumulative += self.bucket_counts[i].load(Ordering::Relaxed);
            output.push_str(&format!(
                "{}_bucket{{le=\"{}\"}} {}\n",
                self.name, bound, cumulative
            ));
        }
        // +Inf bucket
        cumulative += self.bucket_counts.last().unwrap().load(Ordering::Relaxed);
        output.push_str(&format!(
            "{}_bucket{{le=\"+Inf\"}} {}\n",
            self.name, cumulative
        ));

        // Sum and count
        output.push_str(&format!("{}_sum {}\n", self.name, self.sum()));
        output.push_str(&format!("{}_count {}\n", self.name, self.count()));

        output
    }
}

// =========================================================================
// Task 9.1: PrometheusMetrics Registry
// =========================================================================

/// Error type labels for categorising API errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    /// Client error (4xx)
    Client,
    /// Server error (5xx)
    Server,
    /// Computation error (NaN, Inf)
    Computation,
    /// Timeout error
    Timeout,
}

impl ErrorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorType::Client => "client",
            ErrorType::Server => "server",
            ErrorType::Computation => "computation",
            ErrorType::Timeout => "timeout",
        }
    }
}

/// Prometheus metrics registry for FrictionalBank
///
/// Collects and exports metrics in Prometheus text format.
pub struct PrometheusMetrics {
    // Counters
    /// Total API requests
    pub api_requests_total: Counter,
    /// API requests by endpoint
    pub api_requests_by_endpoint: RwLock<Vec<Counter>>,
    /// Total API errors
    pub api_errors_total: Counter,
    /// API errors by type
    pub api_errors_by_type: RwLock<Vec<Counter>>,
    /// Response time threshold exceedances
    pub response_time_threshold_exceeded: Counter,

    // Histograms
    /// API response time histogram
    pub api_response_time_ms: Histogram,
    /// Computation time histogram (Greeks calculation)
    pub computation_time_ms: Histogram,

    // Gauges
    /// Active WebSocket connections
    pub websocket_connections: Gauge,
    /// Memory usage in bytes
    pub memory_usage_bytes: Gauge,
    /// Cache hit rate (percentage * 100)
    pub cache_hit_rate: Gauge,
    /// Server uptime in seconds
    pub uptime_seconds: Gauge,

    /// Start time for uptime calculation
    start_time: Instant,
}

impl PrometheusMetrics {
    /// Create a new metrics registry
    pub fn new() -> Self {
        Self {
            api_requests_total: Counter::new(
                "neutryx_api_requests_total",
                "Total number of API requests",
            ),
            api_requests_by_endpoint: RwLock::new(Vec::new()),
            api_errors_total: Counter::new(
                "neutryx_api_errors_total",
                "Total number of API errors",
            ),
            api_errors_by_type: RwLock::new(Vec::new()),
            response_time_threshold_exceeded: Counter::new(
                "neutryx_response_time_threshold_exceeded_total",
                "Number of requests exceeding response time threshold",
            ),
            api_response_time_ms: Histogram::new(
                "neutryx_api_response_time_ms",
                "API response time in milliseconds",
            ),
            computation_time_ms: Histogram::new(
                "neutryx_computation_time_ms",
                "Greeks computation time in milliseconds",
            ),
            websocket_connections: Gauge::new(
                "neutryx_websocket_connections",
                "Number of active WebSocket connections",
            ),
            memory_usage_bytes: Gauge::new(
                "neutryx_memory_usage_bytes",
                "Current memory usage in bytes",
            ),
            cache_hit_rate: Gauge::new(
                "neutryx_cache_hit_rate",
                "Cache hit rate (percentage * 100)",
            ),
            uptime_seconds: Gauge::new("neutryx_uptime_seconds", "Server uptime in seconds"),
            start_time: Instant::now(),
        }
    }

    /// Record an API request
    pub async fn record_request(&self, endpoint: &str) {
        self.api_requests_total.inc();
        // Add endpoint-specific counter if needed
        self.ensure_endpoint_counter(endpoint).await;
    }

    /// Ensure endpoint counter exists and increment it
    async fn ensure_endpoint_counter(&self, endpoint: &str) {
        let mut counters = self.api_requests_by_endpoint.write().await;
        let endpoint_clean = endpoint
            .replace('/', "_")
            .trim_start_matches('_')
            .to_string();
        let counter_name = format!("neutryx_api_requests_{}", endpoint_clean);

        // Find existing or create new
        if let Some(counter) = counters.iter().find(|c| c.name == counter_name) {
            counter.inc();
        } else {
            let counter = Counter::with_labels(
                &counter_name,
                "API requests by endpoint",
                vec![("endpoint", endpoint)],
            );
            counter.inc();
            counters.push(counter);
        }
    }

    /// Record an API error
    pub async fn record_error(&self, error_type: ErrorType) {
        self.api_errors_total.inc();

        // Add error type-specific counter
        let mut counters = self.api_errors_by_type.write().await;
        let counter_name = format!("neutryx_api_errors_{}", error_type.as_str());

        if let Some(counter) = counters.iter().find(|c| c.name == counter_name) {
            counter.inc();
        } else {
            let counter = Counter::with_labels(
                &counter_name,
                "API errors by type",
                vec![("error_type", error_type.as_str())],
            );
            counter.inc();
            counters.push(counter);
        }
    }

    /// Record API response time and check threshold (Requirement 8.4)
    pub fn record_response_time(&self, duration_ms: f64, endpoint: &str) {
        self.api_response_time_ms.observe(duration_ms);

        // Task 9.3: Check threshold and warn if exceeded
        if duration_ms > RESPONSE_TIME_THRESHOLD_MS as f64 {
            self.response_time_threshold_exceeded.inc();
            warn!(
                endpoint = endpoint,
                duration_ms = duration_ms,
                threshold_ms = RESPONSE_TIME_THRESHOLD_MS,
                "API response time exceeded threshold"
            );
        }
    }

    /// Record computation time (Greeks calculation)
    pub fn record_computation_time(&self, duration_ms: f64) {
        self.computation_time_ms.observe(duration_ms);
    }

    /// Update WebSocket connection count
    pub fn set_websocket_connections(&self, count: u32) {
        self.websocket_connections.set(count as u64);
    }

    /// Increment WebSocket connections
    pub fn inc_websocket_connections(&self) {
        self.websocket_connections.inc();
    }

    /// Decrement WebSocket connections
    pub fn dec_websocket_connections(&self) {
        self.websocket_connections.dec();
    }

    /// Update memory usage
    pub fn set_memory_usage(&self, bytes: u64) {
        self.memory_usage_bytes.set(bytes);
    }

    /// Update cache hit rate (0-100)
    pub fn set_cache_hit_rate(&self, rate_percent: u64) {
        self.cache_hit_rate.set(rate_percent);
    }

    /// Update uptime
    pub fn update_uptime(&self) {
        self.uptime_seconds.set(self.start_time.elapsed().as_secs());
    }

    /// Get uptime in seconds
    pub fn get_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Export all metrics in Prometheus text format
    pub async fn to_prometheus_text(&self) -> String {
        self.update_uptime();

        let mut output = String::new();

        // Counters
        output.push_str(&self.api_requests_total.to_prometheus());
        output.push_str(&self.api_errors_total.to_prometheus());
        output.push_str(&self.response_time_threshold_exceeded.to_prometheus());

        // Endpoint-specific counters
        for counter in self.api_requests_by_endpoint.read().await.iter() {
            output.push_str(&counter.to_prometheus());
        }

        // Error type counters
        for counter in self.api_errors_by_type.read().await.iter() {
            output.push_str(&counter.to_prometheus());
        }

        // Histograms
        output.push_str(&self.api_response_time_ms.to_prometheus());
        output.push_str(&self.computation_time_ms.to_prometheus());

        // Gauges
        output.push_str(&self.websocket_connections.to_prometheus());
        output.push_str(&self.memory_usage_bytes.to_prometheus());
        output.push_str(&self.cache_hit_rate.to_prometheus());
        output.push_str(&self.uptime_seconds.to_prometheus());

        output
    }
}

impl Default for PrometheusMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================
// Task 9.2: Dashboard Metrics Response
// =========================================================================

/// Dashboard metrics response structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DashboardMetrics {
    /// Total API request count
    pub total_requests: u64,
    /// Total API error count
    pub total_errors: u64,
    /// Error rate (percentage)
    pub error_rate: f64,
    /// Average response time in ms
    pub avg_response_time_ms: f64,
    /// P95 response time approximation in ms
    pub p95_response_time_ms: f64,
    /// Active WebSocket connections
    pub websocket_connections: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Cache hit rate (percentage)
    pub cache_hit_rate: f64,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Computation metrics
    pub computation: ComputationMetrics,
    /// Error breakdown by type
    pub errors_by_type: Vec<ErrorTypeCount>,
    /// Response time threshold exceedances
    pub threshold_exceedances: u64,
}

/// Computation-specific metrics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComputationMetrics {
    /// Average computation time in ms
    pub avg_computation_time_ms: f64,
    /// Total computations
    pub total_computations: u64,
}

/// Error count by type
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorTypeCount {
    pub error_type: String,
    pub count: u64,
}

impl PrometheusMetrics {
    /// Get dashboard metrics summary
    pub async fn get_dashboard_metrics(&self) -> DashboardMetrics {
        let total_requests = self.api_requests_total.get();
        let total_errors = self.api_errors_total.get();
        let error_rate = if total_requests > 0 {
            (total_errors as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let count = self.api_response_time_ms.count();
        let avg_response_time_ms = if count > 0 {
            self.api_response_time_ms.sum() / count as f64
        } else {
            0.0
        };

        // P95 approximation from histogram
        let p95_response_time_ms = self.estimate_percentile(95.0);

        let computation_count = self.computation_time_ms.count();
        let avg_computation_time_ms = if computation_count > 0 {
            self.computation_time_ms.sum() / computation_count as f64
        } else {
            0.0
        };

        // Collect error types
        let error_counters = self.api_errors_by_type.read().await;
        let errors_by_type: Vec<ErrorTypeCount> = error_counters
            .iter()
            .map(|c| ErrorTypeCount {
                error_type: c.labels.first().map(|(_, v)| v.clone()).unwrap_or_default(),
                count: c.get(),
            })
            .collect();

        DashboardMetrics {
            total_requests,
            total_errors,
            error_rate,
            avg_response_time_ms,
            p95_response_time_ms,
            websocket_connections: self.websocket_connections.get(),
            memory_usage_bytes: self.memory_usage_bytes.get(),
            cache_hit_rate: self.cache_hit_rate.get() as f64,
            uptime_seconds: self.get_uptime_seconds(),
            computation: ComputationMetrics {
                avg_computation_time_ms,
                total_computations: computation_count,
            },
            errors_by_type,
            threshold_exceedances: self.response_time_threshold_exceeded.get(),
        }
    }

    /// Estimate percentile from histogram (approximation)
    fn estimate_percentile(&self, percentile: f64) -> f64 {
        let count = self.api_response_time_ms.count();
        if count == 0 {
            return 0.0;
        }

        let target = (percentile / 100.0 * count as f64) as u64;
        let mut cumulative = 0u64;

        // Find the bucket containing the target percentile
        for (i, &bound) in Histogram::DEFAULT_BUCKETS.iter().enumerate() {
            cumulative += self.api_response_time_ms.bucket_counts[i].load(Ordering::Relaxed);
            if cumulative >= target {
                return bound;
            }
        }

        // Return highest bucket if not found
        *Histogram::DEFAULT_BUCKETS.last().unwrap_or(&0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 9.1: Counter Tests
    // =========================================================================

    #[test]
    fn test_counter_new() {
        let counter = Counter::new("test_counter", "A test counter");
        assert_eq!(counter.get(), 0);
        assert_eq!(counter.name, "test_counter");
    }

    #[test]
    fn test_counter_inc() {
        let counter = Counter::new("test_counter", "A test counter");
        counter.inc();
        assert_eq!(counter.get(), 1);
        counter.inc();
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn test_counter_inc_by() {
        let counter = Counter::new("test_counter", "A test counter");
        counter.inc_by(5);
        assert_eq!(counter.get(), 5);
        counter.inc_by(10);
        assert_eq!(counter.get(), 15);
    }

    #[test]
    fn test_counter_reset() {
        let counter = Counter::new("test_counter", "A test counter");
        counter.inc_by(100);
        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_counter_with_labels() {
        let counter = Counter::with_labels(
            "test_counter",
            "A test counter",
            vec![("endpoint", "/api/test")],
        );
        counter.inc();
        assert_eq!(counter.get(), 1);
        assert_eq!(counter.labels.len(), 1);
    }

    #[test]
    fn test_counter_to_prometheus() {
        let counter = Counter::new("test_counter", "A test counter");
        counter.inc_by(42);
        let output = counter.to_prometheus();
        assert!(output.contains("# HELP test_counter A test counter"));
        assert!(output.contains("# TYPE test_counter counter"));
        assert!(output.contains("test_counter 42"));
    }

    #[test]
    fn test_counter_to_prometheus_with_labels() {
        let counter = Counter::with_labels(
            "test_counter",
            "A test counter",
            vec![("endpoint", "/api/test"), ("method", "GET")],
        );
        counter.inc_by(10);
        let output = counter.to_prometheus();
        assert!(output.contains("test_counter{endpoint=\"/api/test\",method=\"GET\"} 10"));
    }

    // =========================================================================
    // Task 9.1: Gauge Tests
    // =========================================================================

    #[test]
    fn test_gauge_new() {
        let gauge = Gauge::new("test_gauge", "A test gauge");
        assert_eq!(gauge.get(), 0);
    }

    #[test]
    fn test_gauge_set() {
        let gauge = Gauge::new("test_gauge", "A test gauge");
        gauge.set(100);
        assert_eq!(gauge.get(), 100);
        gauge.set(50);
        assert_eq!(gauge.get(), 50);
    }

    #[test]
    fn test_gauge_inc_dec() {
        let gauge = Gauge::new("test_gauge", "A test gauge");
        gauge.inc();
        assert_eq!(gauge.get(), 1);
        gauge.inc();
        assert_eq!(gauge.get(), 2);
        gauge.dec();
        assert_eq!(gauge.get(), 1);
    }

    #[test]
    fn test_gauge_to_prometheus() {
        let gauge = Gauge::new("test_gauge", "A test gauge");
        gauge.set(42);
        let output = gauge.to_prometheus();
        assert!(output.contains("# HELP test_gauge A test gauge"));
        assert!(output.contains("# TYPE test_gauge gauge"));
        assert!(output.contains("test_gauge 42"));
    }

    // =========================================================================
    // Task 9.1: Histogram Tests
    // =========================================================================

    #[test]
    fn test_histogram_new() {
        let histogram = Histogram::new("test_histogram", "A test histogram");
        assert_eq!(histogram.count(), 0);
        assert_eq!(histogram.sum(), 0.0);
    }

    #[test]
    fn test_histogram_observe() {
        let histogram = Histogram::new("test_histogram", "A test histogram");
        histogram.observe(100.0);
        assert_eq!(histogram.count(), 1);
        assert!((histogram.sum() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_histogram_observe_multiple() {
        let histogram = Histogram::new("test_histogram", "A test histogram");
        histogram.observe(10.0);
        histogram.observe(20.0);
        histogram.observe(30.0);
        assert_eq!(histogram.count(), 3);
        assert!((histogram.sum() - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_histogram_to_prometheus() {
        let histogram = Histogram::new("test_histogram", "A test histogram");
        histogram.observe(50.0);
        histogram.observe(150.0);
        let output = histogram.to_prometheus();
        assert!(output.contains("# HELP test_histogram A test histogram"));
        assert!(output.contains("# TYPE test_histogram histogram"));
        assert!(output.contains("test_histogram_bucket{le=\"50\"} 1"));
        assert!(output.contains("test_histogram_bucket{le=\"+Inf\"} 2"));
        assert!(output.contains("test_histogram_count 2"));
    }

    #[test]
    fn test_histogram_buckets_cumulative() {
        let histogram = Histogram::with_buckets("test", "test", vec![10.0, 50.0, 100.0]);
        histogram.observe(5.0); // goes in 10, 50, 100, +Inf
        histogram.observe(25.0); // goes in 50, 100, +Inf
        histogram.observe(75.0); // goes in 100, +Inf
        histogram.observe(150.0); // goes in +Inf only

        let output = histogram.to_prometheus();
        assert!(output.contains("test_bucket{le=\"10\"} 1"));
        assert!(output.contains("test_bucket{le=\"50\"} 2"));
        assert!(output.contains("test_bucket{le=\"100\"} 3"));
        assert!(output.contains("test_bucket{le=\"+Inf\"} 4"));
    }

    // =========================================================================
    // Task 9.1: PrometheusMetrics Tests
    // =========================================================================

    #[tokio::test]
    async fn test_prometheus_metrics_new() {
        let metrics = PrometheusMetrics::new();
        assert_eq!(metrics.api_requests_total.get(), 0);
        assert_eq!(metrics.api_errors_total.get(), 0);
    }

    #[tokio::test]
    async fn test_prometheus_metrics_record_request() {
        let metrics = PrometheusMetrics::new();
        metrics.record_request("/api/test").await;
        assert_eq!(metrics.api_requests_total.get(), 1);
    }

    #[tokio::test]
    async fn test_prometheus_metrics_record_error() {
        let metrics = PrometheusMetrics::new();
        metrics.record_error(ErrorType::Client).await;
        metrics.record_error(ErrorType::Server).await;
        assert_eq!(metrics.api_errors_total.get(), 2);
    }

    #[tokio::test]
    async fn test_prometheus_metrics_record_response_time() {
        let metrics = PrometheusMetrics::new();
        metrics.record_response_time(50.0, "/api/test");
        assert_eq!(metrics.api_response_time_ms.count(), 1);
    }

    #[tokio::test]
    async fn test_prometheus_metrics_threshold_exceeded() {
        let metrics = PrometheusMetrics::new();
        // Under threshold
        metrics.record_response_time(100.0, "/api/fast");
        assert_eq!(metrics.response_time_threshold_exceeded.get(), 0);

        // Over threshold (500ms)
        metrics.record_response_time(600.0, "/api/slow");
        assert_eq!(metrics.response_time_threshold_exceeded.get(), 1);
    }

    #[tokio::test]
    async fn test_prometheus_metrics_websocket_connections() {
        let metrics = PrometheusMetrics::new();
        metrics.inc_websocket_connections();
        metrics.inc_websocket_connections();
        assert_eq!(metrics.websocket_connections.get(), 2);
        metrics.dec_websocket_connections();
        assert_eq!(metrics.websocket_connections.get(), 1);
    }

    #[tokio::test]
    async fn test_prometheus_metrics_to_text() {
        let metrics = PrometheusMetrics::new();
        metrics.record_request("/api/test").await;
        metrics.record_response_time(50.0, "/api/test");

        let output = metrics.to_prometheus_text().await;
        assert!(output.contains("neutryx_api_requests_total"));
        assert!(output.contains("neutryx_api_response_time_ms"));
        assert!(output.contains("neutryx_uptime_seconds"));
    }

    // =========================================================================
    // Task 9.2: Dashboard Metrics Tests
    // =========================================================================

    #[tokio::test]
    async fn test_dashboard_metrics() {
        let metrics = PrometheusMetrics::new();
        metrics.record_request("/api/test").await;
        metrics.record_request("/api/test").await;
        metrics.record_error(ErrorType::Client).await;
        metrics.record_response_time(100.0, "/api/test");
        metrics.record_response_time(200.0, "/api/test");
        metrics.set_memory_usage(1024 * 1024);
        metrics.set_cache_hit_rate(85);

        let dashboard = metrics.get_dashboard_metrics().await;
        assert_eq!(dashboard.total_requests, 2);
        assert_eq!(dashboard.total_errors, 1);
        assert!((dashboard.error_rate - 50.0).abs() < 0.1);
        assert!((dashboard.avg_response_time_ms - 150.0).abs() < 0.1);
        assert_eq!(dashboard.memory_usage_bytes, 1024 * 1024);
        assert_eq!(dashboard.cache_hit_rate, 85.0);
    }

    #[tokio::test]
    async fn test_dashboard_metrics_empty() {
        let metrics = PrometheusMetrics::new();
        let dashboard = metrics.get_dashboard_metrics().await;
        assert_eq!(dashboard.total_requests, 0);
        assert_eq!(dashboard.error_rate, 0.0);
        assert_eq!(dashboard.avg_response_time_ms, 0.0);
    }

    // =========================================================================
    // Task 9.3: Response Time Warning Tests
    // =========================================================================

    #[test]
    fn test_response_time_threshold_constant() {
        assert_eq!(RESPONSE_TIME_THRESHOLD_MS, 500);
    }

    #[tokio::test]
    async fn test_multiple_threshold_exceedances() {
        let metrics = PrometheusMetrics::new();

        // Multiple slow responses
        for i in 0..5 {
            metrics.record_response_time(600.0 + i as f64 * 100.0, "/api/slow");
        }

        assert_eq!(metrics.response_time_threshold_exceeded.get(), 5);
    }

    // =========================================================================
    // Task 9.2: Computation Metrics Tests
    // =========================================================================

    #[tokio::test]
    async fn test_computation_time_recording() {
        let metrics = PrometheusMetrics::new();
        metrics.record_computation_time(150.0);
        metrics.record_computation_time(250.0);

        let dashboard = metrics.get_dashboard_metrics().await;
        assert_eq!(dashboard.computation.total_computations, 2);
        assert!((dashboard.computation.avg_computation_time_ms - 200.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_error_type_breakdown() {
        let metrics = PrometheusMetrics::new();
        metrics.record_error(ErrorType::Client).await;
        metrics.record_error(ErrorType::Client).await;
        metrics.record_error(ErrorType::Server).await;
        metrics.record_error(ErrorType::Computation).await;

        let dashboard = metrics.get_dashboard_metrics().await;
        assert_eq!(dashboard.total_errors, 4);
        assert_eq!(dashboard.errors_by_type.len(), 3); // 3 different error types

        // Find client errors
        let client_errors = dashboard
            .errors_by_type
            .iter()
            .find(|e| e.error_type == "client");
        assert!(client_errors.is_some());
        assert_eq!(client_errors.unwrap().count, 2);
    }
}
