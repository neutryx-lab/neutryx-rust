//! Memory monitoring and automatic checkpointing.
//!
//! This module provides [`MemoryMonitor`] for tracking memory usage and
//! automatically activating checkpointing when memory thresholds are exceeded.
//!
//! # Requirements
//!
//! - Requirement 3.3: Memory efficiency and checkpoint mechanism

use pricer_pricing::checkpoint::MemoryBudget;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Memory usage statistics.
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Current memory usage in bytes.
    pub current_bytes: usize,
    /// Peak memory usage in bytes.
    pub peak_bytes: usize,
    /// Number of memory allocations tracked.
    pub allocations: usize,
    /// Number of times checkpoint was triggered.
    pub checkpoint_triggers: usize,
    /// Whether checkpointing is currently active.
    pub checkpoint_active: bool,
}

impl MemoryStats {
    /// Creates new empty statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns current memory usage in megabytes.
    #[inline]
    pub fn current_mb(&self) -> f64 {
        self.current_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Returns peak memory usage in megabytes.
    #[inline]
    pub fn peak_mb(&self) -> f64 {
        self.peak_bytes as f64 / (1024.0 * 1024.0)
    }
}

/// Configuration for memory monitoring.
#[derive(Clone, Debug)]
pub struct MemoryMonitorConfig {
    /// Memory budget for threshold calculations.
    pub budget: MemoryBudget,
    /// Whether to enable automatic checkpointing.
    pub auto_checkpoint: bool,
    /// Threshold percentage (0-100) for enabling checkpointing.
    pub checkpoint_threshold_pct: f64,
    /// Estimated bytes per trade for memory calculation.
    pub bytes_per_trade: usize,
}

impl Default for MemoryMonitorConfig {
    fn default() -> Self {
        Self {
            budget: MemoryBudget::from_mb(512), // 512 MB default
            auto_checkpoint: true,
            checkpoint_threshold_pct: 70.0, // Enable checkpoint at 70% usage
            bytes_per_trade: 8192,          // ~8KB per trade estimate
        }
    }
}

impl MemoryMonitorConfig {
    /// Creates a new configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the memory budget in megabytes.
    pub fn with_budget_mb(mut self, mb: usize) -> Self {
        self.budget = MemoryBudget::from_mb(mb);
        self
    }

    /// Sets the memory budget in gigabytes.
    pub fn with_budget_gb(mut self, gb: usize) -> Self {
        self.budget = MemoryBudget::from_gb(gb);
        self
    }

    /// Sets whether auto-checkpointing is enabled.
    pub fn with_auto_checkpoint(mut self, enabled: bool) -> Self {
        self.auto_checkpoint = enabled;
        self
    }

    /// Sets the checkpoint threshold percentage.
    pub fn with_checkpoint_threshold_pct(mut self, pct: f64) -> Self {
        self.checkpoint_threshold_pct = pct.clamp(0.0, 100.0);
        self
    }

    /// Sets the estimated bytes per trade.
    pub fn with_bytes_per_trade(mut self, bytes: usize) -> Self {
        self.bytes_per_trade = bytes.max(1);
        self
    }
}

/// Memory monitor for tracking usage and triggering checkpoints.
///
/// Provides real-time memory tracking and automatic checkpoint activation
/// when memory usage exceeds configured thresholds.
///
/// # Requirements Coverage
///
/// - Requirement 3.3: Memory efficiency and checkpoint mechanism
///
/// # Examples
///
/// ```rust
/// use pricer_risk::parallel::{MemoryMonitor, MemoryMonitorConfig};
///
/// let config = MemoryMonitorConfig::new()
///     .with_budget_mb(256)
///     .with_checkpoint_threshold_pct(70.0)
///     .with_auto_checkpoint(true);
///
/// let monitor = MemoryMonitor::new(config);
///
/// // Track memory for a portfolio
/// let n_trades = 1000;
/// let should_checkpoint = monitor.should_enable_checkpoint(n_trades);
/// println!("Checkpoint needed: {}", should_checkpoint);
///
/// // Record allocation
/// monitor.record_allocation(1024 * 1024); // 1 MB
/// let stats = monitor.stats();
/// println!("Current usage: {:.2} MB", stats.current_mb());
/// ```
#[derive(Debug)]
pub struct MemoryMonitor {
    config: MemoryMonitorConfig,
    current_bytes: AtomicUsize,
    peak_bytes: AtomicUsize,
    allocations: AtomicUsize,
    checkpoint_triggers: AtomicUsize,
    checkpoint_active: std::sync::atomic::AtomicBool,
}

impl MemoryMonitor {
    /// Creates a new memory monitor with the given configuration.
    pub fn new(config: MemoryMonitorConfig) -> Self {
        Self {
            config,
            current_bytes: AtomicUsize::new(0),
            peak_bytes: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
            checkpoint_triggers: AtomicUsize::new(0),
            checkpoint_active: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Creates a monitor with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(MemoryMonitorConfig::default())
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &MemoryMonitorConfig {
        &self.config
    }

    /// Records a memory allocation.
    ///
    /// Updates current and peak memory usage atomically.
    pub fn record_allocation(&self, bytes: usize) {
        let new_current = self.current_bytes.fetch_add(bytes, Ordering::Relaxed) + bytes;
        self.allocations.fetch_add(1, Ordering::Relaxed);

        // Update peak if needed (using compare-and-swap loop)
        loop {
            let current_peak = self.peak_bytes.load(Ordering::Relaxed);
            if new_current <= current_peak {
                break;
            }
            if self
                .peak_bytes
                .compare_exchange_weak(current_peak, new_current, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }

        // Check if we need to trigger checkpoint
        if self.config.auto_checkpoint && self.should_trigger_checkpoint() {
            self.activate_checkpoint();
        }
    }

    /// Records a memory deallocation.
    pub fn record_deallocation(&self, bytes: usize) {
        self.current_bytes.fetch_sub(bytes.min(self.current_bytes.load(Ordering::Relaxed)), Ordering::Relaxed);
    }

    /// Returns current memory usage statistics.
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            current_bytes: self.current_bytes.load(Ordering::Relaxed),
            peak_bytes: self.peak_bytes.load(Ordering::Relaxed),
            allocations: self.allocations.load(Ordering::Relaxed),
            checkpoint_triggers: self.checkpoint_triggers.load(Ordering::Relaxed),
            checkpoint_active: self.checkpoint_active.load(Ordering::Relaxed),
        }
    }

    /// Checks if checkpointing should be triggered based on current usage.
    #[inline]
    fn should_trigger_checkpoint(&self) -> bool {
        if !self.config.auto_checkpoint {
            return false;
        }

        let current = self.current_bytes.load(Ordering::Relaxed);
        let threshold_bytes = (self.config.budget.max_bytes() as f64
            * (self.config.checkpoint_threshold_pct / 100.0)) as usize;

        current > threshold_bytes
    }

    /// Activates checkpointing.
    fn activate_checkpoint(&self) {
        if !self.checkpoint_active.swap(true, Ordering::Relaxed) {
            self.checkpoint_triggers.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Deactivates checkpointing.
    pub fn deactivate_checkpoint(&self) {
        self.checkpoint_active.store(false, Ordering::Relaxed);
    }

    /// Resets all counters and statistics.
    pub fn reset(&self) {
        self.current_bytes.store(0, Ordering::Relaxed);
        self.peak_bytes.store(0, Ordering::Relaxed);
        self.allocations.store(0, Ordering::Relaxed);
        self.checkpoint_triggers.store(0, Ordering::Relaxed);
        self.checkpoint_active.store(false, Ordering::Relaxed);
    }

    /// Returns whether checkpointing is currently active.
    #[inline]
    pub fn is_checkpoint_active(&self) -> bool {
        self.checkpoint_active.load(Ordering::Relaxed)
    }

    /// Determines if checkpointing should be enabled for a portfolio.
    ///
    /// Uses the estimated bytes per trade to predict memory usage
    /// and compare against the threshold.
    ///
    /// # Arguments
    ///
    /// * `n_trades` - Number of trades in the portfolio
    ///
    /// # Returns
    ///
    /// `true` if checkpointing should be enabled.
    pub fn should_enable_checkpoint(&self, n_trades: usize) -> bool {
        if !self.config.auto_checkpoint {
            return false;
        }

        let estimated_bytes = n_trades * self.config.bytes_per_trade;
        let threshold_bytes = (self.config.budget.max_bytes() as f64
            * (self.config.checkpoint_threshold_pct / 100.0)) as usize;

        estimated_bytes > threshold_bytes
    }

    /// Calculates recommended checkpoint interval for a portfolio.
    ///
    /// # Arguments
    ///
    /// * `n_trades` - Number of trades in the portfolio
    /// * `n_steps` - Number of time steps (if applicable)
    ///
    /// # Returns
    ///
    /// Recommended checkpoint interval in number of trades.
    pub fn recommended_checkpoint_interval(&self, n_trades: usize, n_steps: usize) -> usize {
        self.config.budget.recommended_interval(
            n_trades,
            n_steps.max(1),
            self.config.bytes_per_trade,
        )
    }

    /// Returns current memory usage percentage.
    #[inline]
    pub fn usage_percentage(&self) -> f64 {
        self.config
            .budget
            .usage_percentage(self.current_bytes.load(Ordering::Relaxed))
    }

    /// Returns remaining memory budget in bytes.
    #[inline]
    pub fn remaining_bytes(&self) -> usize {
        self.config
            .budget
            .remaining(self.current_bytes.load(Ordering::Relaxed))
    }

    /// Returns remaining memory budget in megabytes.
    #[inline]
    pub fn remaining_mb(&self) -> f64 {
        self.remaining_bytes() as f64 / (1024.0 * 1024.0)
    }
}

impl Clone for MemoryMonitor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            current_bytes: AtomicUsize::new(self.current_bytes.load(Ordering::Relaxed)),
            peak_bytes: AtomicUsize::new(self.peak_bytes.load(Ordering::Relaxed)),
            allocations: AtomicUsize::new(self.allocations.load(Ordering::Relaxed)),
            checkpoint_triggers: AtomicUsize::new(self.checkpoint_triggers.load(Ordering::Relaxed)),
            checkpoint_active: std::sync::atomic::AtomicBool::new(
                self.checkpoint_active.load(Ordering::Relaxed),
            ),
        }
    }
}

/// Thread-safe shared memory monitor.
pub type SharedMemoryMonitor = Arc<MemoryMonitor>;

/// Creates a shared memory monitor.
pub fn create_shared_monitor(config: MemoryMonitorConfig) -> SharedMemoryMonitor {
    Arc::new(MemoryMonitor::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 3.3: MemoryMonitor tests (TDD)
    // ================================================================

    // -----------------------------------------------------------------
    // Configuration tests
    // -----------------------------------------------------------------

    #[test]
    fn test_memory_monitor_config_default() {
        let config = MemoryMonitorConfig::default();
        assert_eq!(config.budget.max_bytes(), 512 * 1024 * 1024);
        assert!(config.auto_checkpoint);
        assert!((config.checkpoint_threshold_pct - 70.0).abs() < 0.001);
    }

    #[test]
    fn test_memory_monitor_config_builder() {
        let config = MemoryMonitorConfig::new()
            .with_budget_mb(256)
            .with_auto_checkpoint(false)
            .with_checkpoint_threshold_pct(80.0)
            .with_bytes_per_trade(4096);

        assert_eq!(config.budget.max_bytes(), 256 * 1024 * 1024);
        assert!(!config.auto_checkpoint);
        assert!((config.checkpoint_threshold_pct - 80.0).abs() < 0.001);
        assert_eq!(config.bytes_per_trade, 4096);
    }

    #[test]
    fn test_memory_monitor_config_gb() {
        let config = MemoryMonitorConfig::new().with_budget_gb(2);
        assert_eq!(config.budget.max_bytes(), 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_checkpoint_threshold_clamping() {
        let config = MemoryMonitorConfig::new().with_checkpoint_threshold_pct(150.0);
        assert!((config.checkpoint_threshold_pct - 100.0).abs() < 0.001);

        let config2 = MemoryMonitorConfig::new().with_checkpoint_threshold_pct(-10.0);
        assert!(config2.checkpoint_threshold_pct.abs() < 0.001);
    }

    // -----------------------------------------------------------------
    // Monitor creation tests
    // -----------------------------------------------------------------

    #[test]
    fn test_memory_monitor_new() {
        let config = MemoryMonitorConfig::default();
        let monitor = MemoryMonitor::new(config);

        let stats = monitor.stats();
        assert_eq!(stats.current_bytes, 0);
        assert_eq!(stats.peak_bytes, 0);
        assert_eq!(stats.allocations, 0);
    }

    #[test]
    fn test_memory_monitor_with_defaults() {
        let monitor = MemoryMonitor::with_defaults();
        assert_eq!(monitor.config().budget.max_bytes(), 512 * 1024 * 1024);
    }

    // -----------------------------------------------------------------
    // Allocation tracking tests
    // -----------------------------------------------------------------

    #[test]
    fn test_record_allocation() {
        let monitor = MemoryMonitor::with_defaults();

        monitor.record_allocation(1024);
        let stats = monitor.stats();

        assert_eq!(stats.current_bytes, 1024);
        assert_eq!(stats.peak_bytes, 1024);
        assert_eq!(stats.allocations, 1);
    }

    #[test]
    fn test_multiple_allocations() {
        let monitor = MemoryMonitor::with_defaults();

        monitor.record_allocation(1024);
        monitor.record_allocation(2048);
        monitor.record_allocation(512);

        let stats = monitor.stats();
        assert_eq!(stats.current_bytes, 1024 + 2048 + 512);
        assert_eq!(stats.peak_bytes, 1024 + 2048 + 512);
        assert_eq!(stats.allocations, 3);
    }

    #[test]
    fn test_record_deallocation() {
        let monitor = MemoryMonitor::with_defaults();

        monitor.record_allocation(1024);
        monitor.record_deallocation(512);

        let stats = monitor.stats();
        assert_eq!(stats.current_bytes, 512);
        assert_eq!(stats.peak_bytes, 1024); // Peak preserved
    }

    #[test]
    fn test_deallocation_underflow_protection() {
        let monitor = MemoryMonitor::with_defaults();

        monitor.record_allocation(100);
        monitor.record_deallocation(200); // More than allocated

        let stats = monitor.stats();
        assert_eq!(stats.current_bytes, 0); // Should saturate at 0
    }

    #[test]
    fn test_peak_memory_tracking() {
        let monitor = MemoryMonitor::with_defaults();

        monitor.record_allocation(1000);
        monitor.record_allocation(500);
        assert_eq!(monitor.stats().peak_bytes, 1500);

        monitor.record_deallocation(700);
        assert_eq!(monitor.stats().current_bytes, 800);
        assert_eq!(monitor.stats().peak_bytes, 1500); // Peak unchanged

        monitor.record_allocation(200);
        assert_eq!(monitor.stats().peak_bytes, 1500); // Still unchanged

        monitor.record_allocation(800);
        assert_eq!(monitor.stats().peak_bytes, 1800); // New peak
    }

    // -----------------------------------------------------------------
    // Checkpoint trigger tests
    // -----------------------------------------------------------------

    #[test]
    fn test_checkpoint_trigger_on_threshold() {
        let config = MemoryMonitorConfig::new()
            .with_budget_mb(1) // 1 MB budget
            .with_checkpoint_threshold_pct(50.0); // Trigger at 50%

        let monitor = MemoryMonitor::new(config);

        // Below threshold
        monitor.record_allocation(400 * 1024); // ~400 KB
        assert!(!monitor.is_checkpoint_active());

        // Above threshold (> 512 KB)
        monitor.record_allocation(200 * 1024); // Total ~600 KB
        assert!(monitor.is_checkpoint_active());
        assert_eq!(monitor.stats().checkpoint_triggers, 1);
    }

    #[test]
    fn test_checkpoint_not_triggered_when_disabled() {
        let config = MemoryMonitorConfig::new()
            .with_budget_mb(1)
            .with_checkpoint_threshold_pct(50.0)
            .with_auto_checkpoint(false); // Disabled

        let monitor = MemoryMonitor::new(config);

        monitor.record_allocation(800 * 1024); // Above threshold
        assert!(!monitor.is_checkpoint_active());
        assert_eq!(monitor.stats().checkpoint_triggers, 0);
    }

    #[test]
    fn test_checkpoint_single_trigger() {
        let config = MemoryMonitorConfig::new()
            .with_budget_mb(1)
            .with_checkpoint_threshold_pct(50.0);

        let monitor = MemoryMonitor::new(config);

        // Multiple allocations above threshold
        monitor.record_allocation(600 * 1024);
        monitor.record_allocation(100 * 1024);
        monitor.record_allocation(100 * 1024);

        // Should only trigger once
        assert_eq!(monitor.stats().checkpoint_triggers, 1);
    }

    #[test]
    fn test_deactivate_checkpoint() {
        let config = MemoryMonitorConfig::new()
            .with_budget_mb(1)
            .with_checkpoint_threshold_pct(50.0);

        let monitor = MemoryMonitor::new(config);

        monitor.record_allocation(600 * 1024);
        assert!(monitor.is_checkpoint_active());

        monitor.deactivate_checkpoint();
        assert!(!monitor.is_checkpoint_active());
    }

    // -----------------------------------------------------------------
    // Should enable checkpoint tests
    // -----------------------------------------------------------------

    #[test]
    fn test_should_enable_checkpoint_small_portfolio() {
        let config = MemoryMonitorConfig::new()
            .with_budget_mb(100)
            .with_checkpoint_threshold_pct(70.0)
            .with_bytes_per_trade(8192); // 8 KB per trade

        let monitor = MemoryMonitor::new(config);

        // 100 trades * 8 KB = 800 KB, well below 70 MB threshold
        assert!(!monitor.should_enable_checkpoint(100));
    }

    #[test]
    fn test_should_enable_checkpoint_large_portfolio() {
        let config = MemoryMonitorConfig::new()
            .with_budget_mb(100)
            .with_checkpoint_threshold_pct(70.0)
            .with_bytes_per_trade(8192);

        let monitor = MemoryMonitor::new(config);

        // 10000 trades * 8 KB = 80 MB, above 70 MB threshold
        assert!(monitor.should_enable_checkpoint(10000));
    }

    #[test]
    fn test_should_enable_checkpoint_disabled() {
        let config = MemoryMonitorConfig::new()
            .with_auto_checkpoint(false);

        let monitor = MemoryMonitor::new(config);
        assert!(!monitor.should_enable_checkpoint(100000));
    }

    // -----------------------------------------------------------------
    // Recommended interval tests
    // -----------------------------------------------------------------

    #[test]
    fn test_recommended_checkpoint_interval() {
        let config = MemoryMonitorConfig::new()
            .with_budget_mb(10)
            .with_bytes_per_trade(8192);

        let monitor = MemoryMonitor::new(config);

        let interval = monitor.recommended_checkpoint_interval(1000, 100);
        assert!(interval >= 1);
    }

    // -----------------------------------------------------------------
    // Usage statistics tests
    // -----------------------------------------------------------------

    #[test]
    fn test_usage_percentage() {
        let config = MemoryMonitorConfig::new().with_budget_mb(100);
        let monitor = MemoryMonitor::new(config);

        monitor.record_allocation(50 * 1024 * 1024); // 50 MB

        let usage = monitor.usage_percentage();
        assert!((usage - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_remaining_bytes() {
        let config = MemoryMonitorConfig::new().with_budget_mb(100);
        let monitor = MemoryMonitor::new(config);

        monitor.record_allocation(30 * 1024 * 1024); // 30 MB

        let remaining = monitor.remaining_bytes();
        assert_eq!(remaining, 70 * 1024 * 1024);
    }

    #[test]
    fn test_remaining_mb() {
        let config = MemoryMonitorConfig::new().with_budget_mb(100);
        let monitor = MemoryMonitor::new(config);

        monitor.record_allocation(40 * 1024 * 1024);

        let remaining_mb = monitor.remaining_mb();
        assert!((remaining_mb - 60.0).abs() < 0.1);
    }

    // -----------------------------------------------------------------
    // Reset tests
    // -----------------------------------------------------------------

    #[test]
    fn test_reset() {
        let config = MemoryMonitorConfig::new()
            .with_budget_mb(1)
            .with_checkpoint_threshold_pct(50.0);

        let monitor = MemoryMonitor::new(config);

        monitor.record_allocation(600 * 1024);
        assert!(monitor.is_checkpoint_active());

        monitor.reset();

        let stats = monitor.stats();
        assert_eq!(stats.current_bytes, 0);
        assert_eq!(stats.peak_bytes, 0);
        assert_eq!(stats.allocations, 0);
        assert!(!stats.checkpoint_active);
    }

    // -----------------------------------------------------------------
    // Clone tests
    // -----------------------------------------------------------------

    #[test]
    fn test_clone() {
        let monitor = MemoryMonitor::with_defaults();
        monitor.record_allocation(1024);

        let cloned = monitor.clone();

        assert_eq!(cloned.stats().current_bytes, 1024);
        assert_eq!(cloned.config().budget.max_bytes(), monitor.config().budget.max_bytes());
    }

    // -----------------------------------------------------------------
    // Shared monitor tests
    // -----------------------------------------------------------------

    #[test]
    fn test_create_shared_monitor() {
        let shared = create_shared_monitor(MemoryMonitorConfig::default());
        shared.record_allocation(1024);

        // Clone the Arc
        let shared2 = Arc::clone(&shared);
        shared2.record_allocation(2048);

        // Both should see the same state
        assert_eq!(shared.stats().current_bytes, 3072);
        assert_eq!(shared2.stats().current_bytes, 3072);
    }

    // -----------------------------------------------------------------
    // Memory stats helper tests
    // -----------------------------------------------------------------

    #[test]
    fn test_memory_stats_mb_conversion() {
        let mut stats = MemoryStats::new();
        stats.current_bytes = 10 * 1024 * 1024;
        stats.peak_bytes = 20 * 1024 * 1024;

        assert!((stats.current_mb() - 10.0).abs() < 0.001);
        assert!((stats.peak_mb() - 20.0).abs() < 0.001);
    }
}
