//! Performance metrics with O(1) ring buffer implementation.
//!
//! This module provides efficient metrics collection using a fixed-size
//! ring buffer that avoids the O(n) cost of removing elements from a Vec.
//!
//! # Example
//!
//! ```rust,ignore
//! use demo_gui::web::state::metrics::{PerformanceMetrics, RingBuffer};
//!
//! let metrics = PerformanceMetrics::new();
//! metrics.record_portfolio_time(1500); // 1.5ms in microseconds
//!
//! let avg = metrics.portfolio_avg_ms();
//! ```

use std::{
    sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering},
    time::Instant,
};

// =============================================================================
// Ring Buffer Implementation
// =============================================================================

/// A lock-free, fixed-size ring buffer for efficient metrics collection.
///
/// This implementation uses atomic operations for thread-safe access without
/// requiring locks. It provides O(1) insertion and O(n) average calculation.
///
/// # Type Parameters
///
/// * `N` - The capacity of the ring buffer (must be a power of 2 for optimal
///   performance)
///
/// # Thread Safety
///
/// All operations are thread-safe and lock-free. Multiple threads can push
/// values concurrently without blocking.
pub struct RingBuffer<const N: usize> {
    /// The data storage using atomic u64 for thread-safe access
    data: Box<[AtomicU64]>,
    /// The write index (wraps around using modulo)
    write_idx: AtomicUsize,
    /// The current count of elements (capped at N)
    count: AtomicUsize,
}

impl<const N: usize> RingBuffer<N> {
    /// Create a new ring buffer with the specified capacity.
    ///
    /// # Panics
    ///
    /// Panics if N is 0.
    pub fn new() -> Self {
        assert!(N > 0, "Ring buffer capacity must be greater than 0");

        let data: Vec<AtomicU64> = (0..N).map(|_| AtomicU64::new(0)).collect();

        Self {
            data: data.into_boxed_slice(),
            write_idx: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
        }
    }

    /// Push a value into the ring buffer.
    ///
    /// This operation is O(1) and lock-free. If the buffer is full,
    /// the oldest value is overwritten.
    pub fn push(&self, value: u64) {
        // Get the next write position and wrap around
        let idx = self.write_idx.fetch_add(1, Ordering::Relaxed) % N;

        // Store the value
        self.data[idx].store(value, Ordering::Relaxed);

        // Update count if not yet at capacity
        let current_count = self.count.load(Ordering::Relaxed);
        if current_count < N {
            // Use compare_exchange to avoid race conditions
            let _ = self.count.compare_exchange(
                current_count,
                current_count + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            );
        }
    }

    /// Get the current number of elements in the buffer.
    pub fn len(&self) -> usize { self.count.load(Ordering::Relaxed).min(N) }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    /// Calculate the average of all values in the buffer.
    ///
    /// Returns 0.0 if the buffer is empty.
    pub fn average(&self) -> f64 {
        let count = self.len();
        if count == 0 {
            return 0.0;
        }

        let sum: u64 = self.data[..count]
            .iter()
            .map(|a| a.load(Ordering::Relaxed))
            .sum();

        sum as f64 / count as f64
    }

    /// Calculate the average in milliseconds (assuming values are in
    /// microseconds).
    pub fn average_ms(&self) -> f64 { self.average() / 1000.0 }

    /// Get all values as a vector (for debugging or export).
    pub fn to_vec(&self) -> Vec<u64> {
        let count = self.len();
        self.data[..count]
            .iter()
            .map(|a| a.load(Ordering::Relaxed))
            .collect()
    }

    /// Clear all values in the buffer.
    pub fn clear(&self) {
        self.count.store(0, Ordering::Release);
        self.write_idx.store(0, Ordering::Release);
    }

    /// Get the minimum value in the buffer.
    pub fn min(&self) -> Option<u64> {
        let count = self.len();
        if count == 0 {
            return None;
        }

        self.data[..count]
            .iter()
            .map(|a| a.load(Ordering::Relaxed))
            .min()
    }

    /// Get the maximum value in the buffer.
    pub fn max(&self) -> Option<u64> {
        let count = self.len();
        if count == 0 {
            return None;
        }

        self.data[..count]
            .iter()
            .map(|a| a.load(Ordering::Relaxed))
            .max()
    }

    /// Calculate the percentile value.
    ///
    /// # Arguments
    ///
    /// * `percentile` - The percentile to calculate (0.0 to 100.0)
    ///
    /// Returns `None` if the buffer is empty.
    pub fn percentile(&self, percentile: f64) -> Option<u64> {
        let count = self.len();
        if count == 0 {
            return None;
        }

        let mut values = self.to_vec();
        values.sort_unstable();

        let idx = ((percentile / 100.0) * (count - 1) as f64).round() as usize;
        Some(values[idx.min(count - 1)])
    }
}

impl<const N: usize> Default for RingBuffer<N> {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// Performance Metrics
// =============================================================================

/// Performance metrics for API response times and WebSocket connections.
///
/// This is an improved implementation using ring buffers for O(1) insertion
/// instead of the previous Vec-based O(n) implementation.
///
/// # Metrics Collected
///
/// - Portfolio API response times
/// - Exposure API response times
/// - Risk API response times
/// - Graph API response times
/// - WebSocket message latencies
/// - WebSocket connection count
/// - Server uptime
pub struct PerformanceMetrics {
    /// Portfolio API response times in microseconds
    pub portfolio_times: RingBuffer<1000>,
    /// Exposure API response times in microseconds
    pub exposure_times: RingBuffer<1000>,
    /// Risk API response times in microseconds
    pub risk_times: RingBuffer<1000>,
    /// Graph API response times in microseconds
    pub graph_times: RingBuffer<1000>,
    /// WebSocket message latencies in microseconds
    pub ws_message_latencies: RingBuffer<1000>,
    /// Number of active WebSocket connections
    pub ws_connections: AtomicU32,
    /// Server start time for uptime calculation
    pub start_time: Instant,
}

impl PerformanceMetrics {
    /// Maximum number of timing entries to keep per metric type.
    pub const MAX_ENTRIES: usize = 1000;

    /// Create a new performance metrics instance.
    pub fn new() -> Self {
        Self {
            portfolio_times: RingBuffer::new(),
            exposure_times: RingBuffer::new(),
            risk_times: RingBuffer::new(),
            graph_times: RingBuffer::new(),
            ws_message_latencies: RingBuffer::new(),
            ws_connections: AtomicU32::new(0),
            start_time: Instant::now(),
        }
    }

    // =========================================================================
    // Recording Methods (O(1) - no async needed)
    // =========================================================================

    /// Record portfolio API response time in microseconds.
    ///
    /// This is now a synchronous O(1) operation.
    pub fn record_portfolio_time(&self, duration_us: u64) {
        self.portfolio_times.push(duration_us);
    }

    /// Record exposure API response time in microseconds.
    pub fn record_exposure_time(&self, duration_us: u64) { self.exposure_times.push(duration_us); }

    /// Record risk API response time in microseconds.
    pub fn record_risk_time(&self, duration_us: u64) { self.risk_times.push(duration_us); }

    /// Record graph API response time in microseconds.
    pub fn record_graph_time(&self, duration_us: u64) { self.graph_times.push(duration_us); }

    /// Record WebSocket message latency in microseconds.
    pub fn record_ws_latency(&self, latency_us: u64) { self.ws_message_latencies.push(latency_us); }

    // =========================================================================
    // Query Methods
    // =========================================================================

    /// Get average portfolio response time in milliseconds.
    pub fn portfolio_avg_ms(&self) -> f64 { self.portfolio_times.average_ms() }

    /// Get average exposure response time in milliseconds.
    pub fn exposure_avg_ms(&self) -> f64 { self.exposure_times.average_ms() }

    /// Get average risk response time in milliseconds.
    pub fn risk_avg_ms(&self) -> f64 { self.risk_times.average_ms() }

    /// Get average graph response time in milliseconds.
    pub fn graph_avg_ms(&self) -> f64 { self.graph_times.average_ms() }

    /// Get average WebSocket message latency in milliseconds.
    pub fn ws_latency_avg_ms(&self) -> f64 { self.ws_message_latencies.average_ms() }

    /// Get the 95th percentile portfolio response time in milliseconds.
    pub fn portfolio_p95_ms(&self) -> Option<f64> {
        self.portfolio_times
            .percentile(95.0)
            .map(|v| v as f64 / 1000.0)
    }

    /// Get server uptime in seconds.
    pub fn uptime_seconds(&self) -> u64 { self.start_time.elapsed().as_secs() }

    /// Get current WebSocket connection count.
    pub fn ws_connection_count(&self) -> u32 { self.ws_connections.load(Ordering::Relaxed) }

    /// Increment WebSocket connection count.
    pub fn increment_ws_connections(&self) { self.ws_connections.fetch_add(1, Ordering::Relaxed); }

    /// Decrement WebSocket connection count.
    pub fn decrement_ws_connections(&self) { self.ws_connections.fetch_sub(1, Ordering::Relaxed); }

    // =========================================================================
    // Async Compatibility Methods
    // =========================================================================

    /// Record portfolio API response time (async compatibility wrapper).
    ///
    /// This method exists for backward compatibility with existing code that
    /// uses `await`. The actual operation is synchronous.
    pub async fn record_portfolio_time_async(&self, duration_us: u64) {
        self.record_portfolio_time(duration_us);
    }

    /// Record exposure API response time (async compatibility wrapper).
    pub async fn record_exposure_time_async(&self, duration_us: u64) {
        self.record_exposure_time(duration_us);
    }

    /// Record risk API response time (async compatibility wrapper).
    pub async fn record_risk_time_async(&self, duration_us: u64) {
        self.record_risk_time(duration_us);
    }

    /// Record graph API response time (async compatibility wrapper).
    pub async fn record_graph_time_async(&self, duration_us: u64) {
        self.record_graph_time(duration_us);
    }

    /// Record WebSocket message latency (async compatibility wrapper).
    pub async fn record_ws_latency_async(&self, latency_us: u64) {
        self.record_ws_latency(latency_us);
    }

    /// Get average portfolio response time (async compatibility wrapper).
    pub async fn portfolio_avg_ms_async(&self) -> f64 { self.portfolio_avg_ms() }

    /// Get average exposure response time (async compatibility wrapper).
    pub async fn exposure_avg_ms_async(&self) -> f64 { self.exposure_avg_ms() }

    /// Get average risk response time (async compatibility wrapper).
    pub async fn risk_avg_ms_async(&self) -> f64 { self.risk_avg_ms() }

    /// Get average graph response time (async compatibility wrapper).
    pub async fn graph_avg_ms_async(&self) -> f64 { self.graph_avg_ms() }

    /// Get average WebSocket latency (async compatibility wrapper).
    pub async fn ws_latency_avg_ms_async(&self) -> f64 { self.ws_latency_avg_ms() }
}

impl Default for PerformanceMetrics {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod ring_buffer_tests {
        use super::*;

        #[test]
        fn test_new_buffer_is_empty() {
            let buffer: RingBuffer<100> = RingBuffer::new();
            assert!(buffer.is_empty());
            assert_eq!(buffer.len(), 0);
        }

        #[test]
        fn test_push_increments_count() {
            let buffer: RingBuffer<100> = RingBuffer::new();
            buffer.push(42);
            assert_eq!(buffer.len(), 1);
            buffer.push(43);
            assert_eq!(buffer.len(), 2);
        }

        #[test]
        fn test_average_calculation() {
            let buffer: RingBuffer<100> = RingBuffer::new();
            buffer.push(10);
            buffer.push(20);
            buffer.push(30);
            assert_eq!(buffer.average(), 20.0);
        }

        #[test]
        fn test_average_ms() {
            let buffer: RingBuffer<100> = RingBuffer::new();
            buffer.push(1000); // 1000 microseconds = 1 ms
            buffer.push(2000); // 2000 microseconds = 2 ms
            assert_eq!(buffer.average_ms(), 1.5);
        }

        #[test]
        fn test_wrapping_at_capacity() {
            let buffer: RingBuffer<3> = RingBuffer::new();
            buffer.push(1);
            buffer.push(2);
            buffer.push(3);
            assert_eq!(buffer.len(), 3);

            // Push one more - should wrap around
            buffer.push(4);
            assert_eq!(buffer.len(), 3); // Still 3, not 4
        }

        #[test]
        fn test_min_max() {
            let buffer: RingBuffer<100> = RingBuffer::new();
            buffer.push(30);
            buffer.push(10);
            buffer.push(20);

            assert_eq!(buffer.min(), Some(10));
            assert_eq!(buffer.max(), Some(30));
        }

        #[test]
        fn test_empty_buffer_returns_none() {
            let buffer: RingBuffer<100> = RingBuffer::new();
            assert_eq!(buffer.min(), None);
            assert_eq!(buffer.max(), None);
            assert_eq!(buffer.percentile(50.0), None);
        }

        #[test]
        fn test_percentile() {
            let buffer: RingBuffer<100> = RingBuffer::new();
            for i in 1..=100 {
                buffer.push(i);
            }

            // 50th percentile should be around 50
            let p50 = buffer.percentile(50.0).unwrap();
            assert!((49..=51).contains(&p50));

            // 95th percentile should be around 95
            let p95 = buffer.percentile(95.0).unwrap();
            assert!((94..=96).contains(&p95));
        }

        #[test]
        fn test_clear() {
            let buffer: RingBuffer<100> = RingBuffer::new();
            buffer.push(1);
            buffer.push(2);
            buffer.clear();
            assert!(buffer.is_empty());
        }

        #[test]
        fn test_to_vec() {
            let buffer: RingBuffer<100> = RingBuffer::new();
            buffer.push(1);
            buffer.push(2);
            buffer.push(3);

            let vec = buffer.to_vec();
            assert_eq!(vec.len(), 3);
        }
    }

    mod performance_metrics_tests {
        use super::*;

        #[test]
        fn test_new_metrics() {
            let metrics = PerformanceMetrics::new();
            assert_eq!(metrics.ws_connection_count(), 0);
            assert!(metrics.uptime_seconds() < 1);
        }

        #[test]
        fn test_record_and_query() {
            let metrics = PerformanceMetrics::new();

            metrics.record_portfolio_time(1000);
            metrics.record_portfolio_time(2000);
            metrics.record_portfolio_time(3000);

            assert_eq!(metrics.portfolio_avg_ms(), 2.0);
        }

        #[test]
        fn test_ws_connection_tracking() {
            let metrics = PerformanceMetrics::new();

            metrics.increment_ws_connections();
            metrics.increment_ws_connections();
            assert_eq!(metrics.ws_connection_count(), 2);

            metrics.decrement_ws_connections();
            assert_eq!(metrics.ws_connection_count(), 1);
        }

        #[test]
        fn test_p95() {
            let metrics = PerformanceMetrics::new();

            // Add 100 values from 1000 to 100000 microseconds
            for i in 1..=100 {
                metrics.record_portfolio_time(i * 1000);
            }

            let p95 = metrics.portfolio_p95_ms();
            assert!(p95.is_some());
            // 95th percentile of 1-100ms should be around 95ms
            let p95_val = p95.unwrap();
            assert!(p95_val > 90.0 && p95_val < 100.0);
        }
    }

    #[tokio::test]
    async fn test_async_compatibility() {
        let metrics = PerformanceMetrics::new();

        metrics.record_portfolio_time_async(1500).await;
        let avg = metrics.portfolio_avg_ms_async().await;

        assert_eq!(avg, 1.5);
    }
}
