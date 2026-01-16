//! Async job management for long-running computations.
//!
//! Provides infrastructure for managing long-running calculations
//! such as scenario analysis and portfolio Greeks computation.
//!
//! ## Design (from design.md)
//!
//! - UUID-based job ID generation
//! - State management: Pending, Running, Completed, Failed
//! - TTL-based job result cache (5 minutes)
//! - WebSocket integration for progress notifications
//!
//! ## Requirements Coverage
//!
//! - Requirements: 7.6 (非同期ジョブ管理)
//! - Requirements: 6.5 (5秒以上の計算の非同期化)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

// =========================================================================
// Task 7.1: Job Status Enum (Requirements: 7.6)
// =========================================================================

/// Job execution status.
///
/// Represents the current state of an async job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum JobStatus {
    /// Job is waiting to be executed.
    Pending,

    /// Job is currently running with progress percentage.
    Running {
        /// Progress percentage (0-100).
        progress_percent: u8,
    },

    /// Job completed successfully with result.
    Completed {
        /// JSON-serialized result.
        result: serde_json::Value,
    },

    /// Job failed with error message.
    Failed {
        /// Error description.
        error: String,
    },
}

impl JobStatus {
    /// Check if the job is in a terminal state (Completed or Failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, JobStatus::Completed { .. } | JobStatus::Failed { .. })
    }

    /// Check if the job is pending.
    pub fn is_pending(&self) -> bool {
        matches!(self, JobStatus::Pending)
    }

    /// Check if the job is running.
    pub fn is_running(&self) -> bool {
        matches!(self, JobStatus::Running { .. })
    }

    /// Get the progress percentage if running.
    pub fn progress(&self) -> Option<u8> {
        match self {
            JobStatus::Running { progress_percent } => Some(*progress_percent),
            JobStatus::Completed { .. } => Some(100),
            _ => None,
        }
    }
}

// =========================================================================
// Task 7.1: Job Entry (internal representation)
// =========================================================================

/// Internal job entry with status and metadata.
#[derive(Debug, Clone)]
struct JobEntry {
    /// Current job status.
    status: JobStatus,
    /// Job creation time.
    created_at: Instant,
    /// Last update time.
    updated_at: Instant,
    /// Optional job description.
    description: Option<String>,
}

impl JobEntry {
    /// Create a new pending job entry.
    fn new(description: Option<String>) -> Self {
        let now = Instant::now();
        Self {
            status: JobStatus::Pending,
            created_at: now,
            updated_at: now,
            description,
        }
    }

    /// Check if the job has expired (exceeded TTL).
    fn is_expired(&self, ttl: Duration) -> bool {
        self.status.is_terminal() && self.updated_at.elapsed() > ttl
    }
}

// =========================================================================
// Task 7.1: Job Manager Configuration
// =========================================================================

/// Configuration for the job manager.
#[derive(Debug, Clone)]
pub struct JobManagerConfig {
    /// Time-to-live for completed/failed jobs (default: 5 minutes).
    pub ttl: Duration,
    /// Maximum number of jobs to keep (default: 1000).
    pub max_jobs: usize,
    /// Cleanup interval for expired jobs (default: 1 minute).
    pub cleanup_interval: Duration,
}

impl Default for JobManagerConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(300), // 5 minutes
            max_jobs: 1000,
            cleanup_interval: Duration::from_secs(60), // 1 minute
        }
    }
}

// =========================================================================
// Task 7.1: Job Manager (Requirements: 7.6)
// =========================================================================

/// Manages async job lifecycle and state.
///
/// Thread-safe job manager for tracking long-running computations.
/// Supports TTL-based cleanup and progress tracking.
///
/// # Example
///
/// ```ignore
/// let manager = JobManager::new();
/// let job_id = manager.create_job(Some("Scenario Analysis")).await;
///
/// // Update progress
/// manager.update_progress(job_id, 50).await;
///
/// // Complete the job
/// let result = serde_json::json!({"pnl": 1234.56});
/// manager.complete_job(job_id, result).await;
///
/// // Get status
/// let status = manager.get_status(job_id).await;
/// ```
#[derive(Debug)]
pub struct JobManager {
    /// Job storage.
    jobs: Arc<RwLock<HashMap<Uuid, JobEntry>>>,
    /// Configuration.
    config: JobManagerConfig,
}

impl JobManager {
    /// Create a new job manager with default configuration.
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            config: JobManagerConfig::default(),
        }
    }

    /// Create a new job manager with custom configuration.
    pub fn with_config(config: JobManagerConfig) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Create a new job and return its ID.
    ///
    /// # Arguments
    ///
    /// * `description` - Optional description of the job.
    ///
    /// # Returns
    ///
    /// The UUID of the newly created job.
    pub async fn create_job(&self, description: Option<&str>) -> Uuid {
        let job_id = Uuid::new_v4();
        let entry = JobEntry::new(description.map(String::from));

        let mut jobs = self.jobs.write().await;

        // Enforce max jobs limit by removing oldest expired jobs
        if jobs.len() >= self.config.max_jobs {
            self.cleanup_expired_internal(&mut jobs);
        }

        jobs.insert(job_id, entry);
        job_id
    }

    /// Update the progress of a running job.
    ///
    /// # Arguments
    ///
    /// * `job_id` - The job UUID.
    /// * `percent` - Progress percentage (0-100, clamped to valid range).
    ///
    /// # Returns
    ///
    /// `true` if the job was found and updated, `false` otherwise.
    pub async fn update_progress(&self, job_id: Uuid, percent: u8) -> bool {
        let mut jobs = self.jobs.write().await;

        if let Some(entry) = jobs.get_mut(&job_id) {
            // Only update if job is pending or running
            if !entry.status.is_terminal() {
                let clamped_percent = percent.min(100);
                entry.status = JobStatus::Running {
                    progress_percent: clamped_percent,
                };
                entry.updated_at = Instant::now();
                return true;
            }
        }
        false
    }

    /// Mark a job as completed with a result.
    ///
    /// # Arguments
    ///
    /// * `job_id` - The job UUID.
    /// * `result` - The result as a JSON value.
    ///
    /// # Returns
    ///
    /// `true` if the job was found and updated, `false` otherwise.
    pub async fn complete_job(&self, job_id: Uuid, result: serde_json::Value) -> bool {
        let mut jobs = self.jobs.write().await;

        if let Some(entry) = jobs.get_mut(&job_id) {
            if !entry.status.is_terminal() {
                entry.status = JobStatus::Completed { result };
                entry.updated_at = Instant::now();
                return true;
            }
        }
        false
    }

    /// Mark a job as failed with an error message.
    ///
    /// # Arguments
    ///
    /// * `job_id` - The job UUID.
    /// * `error` - Error description.
    ///
    /// # Returns
    ///
    /// `true` if the job was found and updated, `false` otherwise.
    pub async fn fail_job(&self, job_id: Uuid, error: impl Into<String>) -> bool {
        let mut jobs = self.jobs.write().await;

        if let Some(entry) = jobs.get_mut(&job_id) {
            if !entry.status.is_terminal() {
                entry.status = JobStatus::Failed {
                    error: error.into(),
                };
                entry.updated_at = Instant::now();
                return true;
            }
        }
        false
    }

    /// Get the current status of a job.
    ///
    /// # Arguments
    ///
    /// * `job_id` - The job UUID.
    ///
    /// # Returns
    ///
    /// The job status if found, `None` otherwise.
    pub async fn get_status(&self, job_id: Uuid) -> Option<JobStatus> {
        let jobs = self.jobs.read().await;
        jobs.get(&job_id).map(|entry| entry.status.clone())
    }

    /// Get the number of active jobs (not in terminal state).
    pub async fn active_count(&self) -> usize {
        let jobs = self.jobs.read().await;
        jobs.values().filter(|e| !e.status.is_terminal()).count()
    }

    /// Get the total number of jobs.
    pub async fn total_count(&self) -> usize {
        let jobs = self.jobs.read().await;
        jobs.len()
    }

    /// Cleanup expired jobs.
    ///
    /// # Returns
    ///
    /// The number of jobs removed.
    pub async fn cleanup_expired(&self) -> usize {
        let mut jobs = self.jobs.write().await;
        self.cleanup_expired_internal(&mut jobs)
    }

    /// Internal cleanup helper (must be called with write lock held).
    fn cleanup_expired_internal(&self, jobs: &mut HashMap<Uuid, JobEntry>) -> usize {
        let before = jobs.len();
        jobs.retain(|_, entry| !entry.is_expired(self.config.ttl));
        before - jobs.len()
    }

    /// Check if a job exists.
    pub async fn exists(&self, job_id: Uuid) -> bool {
        let jobs = self.jobs.read().await;
        jobs.contains_key(&job_id)
    }

    /// Remove a job by ID.
    ///
    /// # Returns
    ///
    /// `true` if the job was found and removed, `false` otherwise.
    pub async fn remove_job(&self, job_id: Uuid) -> bool {
        let mut jobs = self.jobs.write().await;
        jobs.remove(&job_id).is_some()
    }

    /// Get all job IDs.
    pub async fn list_jobs(&self) -> Vec<Uuid> {
        let jobs = self.jobs.read().await;
        jobs.keys().copied().collect()
    }

    /// Get the configuration.
    pub fn config(&self) -> &JobManagerConfig {
        &self.config
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for JobManager {
    fn clone(&self) -> Self {
        Self {
            jobs: Arc::clone(&self.jobs),
            config: self.config.clone(),
        }
    }
}

// =========================================================================
// Task 7.1: Job Response (API response type)
// =========================================================================

/// API response for job status queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResponse {
    /// Job ID.
    pub job_id: String,
    /// Current status.
    #[serde(flatten)]
    pub status: JobStatus,
}

impl JobResponse {
    /// Create a new job response.
    pub fn new(job_id: Uuid, status: JobStatus) -> Self {
        Self {
            job_id: job_id.to_string(),
            status,
        }
    }
}

// =========================================================================
// Task 7.1: Job Creation Response
// =========================================================================

/// API response when a job is created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCreatedResponse {
    /// The job ID.
    pub job_id: String,
    /// Message.
    pub message: String,
}

impl JobCreatedResponse {
    /// Create a new job created response.
    pub fn new(job_id: Uuid) -> Self {
        Self {
            job_id: job_id.to_string(),
            message: "Job created. Poll /api/v1/jobs/{id} for status.".to_string(),
        }
    }
}

// =========================================================================
// Tests (TDD - RED phase)
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // JobStatus Tests
    // ================================================================

    #[test]
    fn test_job_status_pending() {
        let status = JobStatus::Pending;
        assert!(status.is_pending());
        assert!(!status.is_running());
        assert!(!status.is_terminal());
        assert_eq!(status.progress(), None);
    }

    #[test]
    fn test_job_status_running() {
        let status = JobStatus::Running {
            progress_percent: 50,
        };
        assert!(!status.is_pending());
        assert!(status.is_running());
        assert!(!status.is_terminal());
        assert_eq!(status.progress(), Some(50));
    }

    #[test]
    fn test_job_status_completed() {
        let status = JobStatus::Completed {
            result: serde_json::json!({"value": 42}),
        };
        assert!(!status.is_pending());
        assert!(!status.is_running());
        assert!(status.is_terminal());
        assert_eq!(status.progress(), Some(100));
    }

    #[test]
    fn test_job_status_failed() {
        let status = JobStatus::Failed {
            error: "Something went wrong".to_string(),
        };
        assert!(!status.is_pending());
        assert!(!status.is_running());
        assert!(status.is_terminal());
        assert_eq!(status.progress(), None);
    }

    #[test]
    fn test_job_status_serialization() {
        let status = JobStatus::Running {
            progress_percent: 75,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("running"));
        assert!(json.contains("75"));

        let deserialized: JobStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }

    // ================================================================
    // JobManagerConfig Tests
    // ================================================================

    #[test]
    fn test_job_manager_config_default() {
        let config = JobManagerConfig::default();
        assert_eq!(config.ttl, Duration::from_secs(300));
        assert_eq!(config.max_jobs, 1000);
        assert_eq!(config.cleanup_interval, Duration::from_secs(60));
    }

    // ================================================================
    // JobManager Tests
    // ================================================================

    #[tokio::test]
    async fn test_job_manager_new() {
        let manager = JobManager::new();
        assert_eq!(manager.total_count().await, 0);
        assert_eq!(manager.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_job_manager_create_job() {
        let manager = JobManager::new();
        let job_id = manager.create_job(Some("Test job")).await;

        assert!(manager.exists(job_id).await);
        assert_eq!(manager.total_count().await, 1);
        assert_eq!(manager.active_count().await, 1);

        let status = manager.get_status(job_id).await.unwrap();
        assert!(status.is_pending());
    }

    #[tokio::test]
    async fn test_job_manager_create_job_no_description() {
        let manager = JobManager::new();
        let job_id = manager.create_job(None).await;

        assert!(manager.exists(job_id).await);
        let status = manager.get_status(job_id).await.unwrap();
        assert!(status.is_pending());
    }

    #[tokio::test]
    async fn test_job_manager_update_progress() {
        let manager = JobManager::new();
        let job_id = manager.create_job(None).await;

        let updated = manager.update_progress(job_id, 50).await;
        assert!(updated);

        let status = manager.get_status(job_id).await.unwrap();
        assert!(status.is_running());
        assert_eq!(status.progress(), Some(50));
    }

    #[tokio::test]
    async fn test_job_manager_update_progress_clamps() {
        let manager = JobManager::new();
        let job_id = manager.create_job(None).await;

        // Progress > 100 should be clamped
        let updated = manager.update_progress(job_id, 150).await;
        assert!(updated);

        let status = manager.get_status(job_id).await.unwrap();
        assert_eq!(status.progress(), Some(100));
    }

    #[tokio::test]
    async fn test_job_manager_complete_job() {
        let manager = JobManager::new();
        let job_id = manager.create_job(None).await;

        let result = serde_json::json!({"pnl": 1234.56});
        let completed = manager.complete_job(job_id, result.clone()).await;
        assert!(completed);

        let status = manager.get_status(job_id).await.unwrap();
        assert!(status.is_terminal());
        if let JobStatus::Completed { result: r } = status {
            assert_eq!(r["pnl"], 1234.56);
        } else {
            panic!("Expected Completed status");
        }

        // Active count should decrease
        assert_eq!(manager.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_job_manager_fail_job() {
        let manager = JobManager::new();
        let job_id = manager.create_job(None).await;

        let failed = manager.fail_job(job_id, "Computation error").await;
        assert!(failed);

        let status = manager.get_status(job_id).await.unwrap();
        assert!(status.is_terminal());
        if let JobStatus::Failed { error } = status {
            assert_eq!(error, "Computation error");
        } else {
            panic!("Expected Failed status");
        }

        assert_eq!(manager.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_job_manager_cannot_update_terminal() {
        let manager = JobManager::new();
        let job_id = manager.create_job(None).await;

        // Complete the job
        manager.complete_job(job_id, serde_json::json!({})).await;

        // Cannot update progress after completion
        let updated = manager.update_progress(job_id, 50).await;
        assert!(!updated);

        // Cannot complete again
        let completed = manager.complete_job(job_id, serde_json::json!({})).await;
        assert!(!completed);

        // Cannot fail after completion
        let failed = manager.fail_job(job_id, "error").await;
        assert!(!failed);
    }

    #[tokio::test]
    async fn test_job_manager_get_nonexistent() {
        let manager = JobManager::new();
        let fake_id = Uuid::new_v4();

        assert!(!manager.exists(fake_id).await);
        assert!(manager.get_status(fake_id).await.is_none());
    }

    #[tokio::test]
    async fn test_job_manager_update_nonexistent() {
        let manager = JobManager::new();
        let fake_id = Uuid::new_v4();

        assert!(!manager.update_progress(fake_id, 50).await);
        assert!(!manager.complete_job(fake_id, serde_json::json!({})).await);
        assert!(!manager.fail_job(fake_id, "error").await);
    }

    #[tokio::test]
    async fn test_job_manager_remove_job() {
        let manager = JobManager::new();
        let job_id = manager.create_job(None).await;

        assert!(manager.exists(job_id).await);

        let removed = manager.remove_job(job_id).await;
        assert!(removed);
        assert!(!manager.exists(job_id).await);

        // Removing again should return false
        let removed_again = manager.remove_job(job_id).await;
        assert!(!removed_again);
    }

    #[tokio::test]
    async fn test_job_manager_list_jobs() {
        let manager = JobManager::new();
        let job1 = manager.create_job(None).await;
        let job2 = manager.create_job(None).await;
        let job3 = manager.create_job(None).await;

        let jobs = manager.list_jobs().await;
        assert_eq!(jobs.len(), 3);
        assert!(jobs.contains(&job1));
        assert!(jobs.contains(&job2));
        assert!(jobs.contains(&job3));
    }

    #[tokio::test]
    async fn test_job_manager_cleanup_expired() {
        // Create manager with very short TTL for testing
        let config = JobManagerConfig {
            ttl: Duration::from_millis(10),
            max_jobs: 1000,
            cleanup_interval: Duration::from_secs(60),
        };
        let manager = JobManager::with_config(config);

        let job_id = manager.create_job(None).await;
        manager.complete_job(job_id, serde_json::json!({})).await;

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(20)).await;

        let removed = manager.cleanup_expired().await;
        assert_eq!(removed, 1);
        assert!(!manager.exists(job_id).await);
    }

    #[tokio::test]
    async fn test_job_manager_cleanup_keeps_active() {
        let config = JobManagerConfig {
            ttl: Duration::from_millis(10),
            max_jobs: 1000,
            cleanup_interval: Duration::from_secs(60),
        };
        let manager = JobManager::with_config(config);

        let active_job = manager.create_job(None).await;
        let completed_job = manager.create_job(None).await;
        manager
            .complete_job(completed_job, serde_json::json!({}))
            .await;

        // Wait for TTL
        tokio::time::sleep(Duration::from_millis(20)).await;

        let removed = manager.cleanup_expired().await;
        assert_eq!(removed, 1);

        // Active job should still exist
        assert!(manager.exists(active_job).await);
        assert!(!manager.exists(completed_job).await);
    }

    #[tokio::test]
    async fn test_job_manager_clone_shares_state() {
        let manager1 = JobManager::new();
        let manager2 = manager1.clone();

        let job_id = manager1.create_job(None).await;

        // Both managers should see the job
        assert!(manager1.exists(job_id).await);
        assert!(manager2.exists(job_id).await);

        // Update from one manager should be visible in the other
        manager2.update_progress(job_id, 50).await;
        let status = manager1.get_status(job_id).await.unwrap();
        assert_eq!(status.progress(), Some(50));
    }

    // ================================================================
    // JobResponse Tests
    // ================================================================

    #[test]
    fn test_job_response_new() {
        let job_id = Uuid::new_v4();
        let status = JobStatus::Pending;
        let response = JobResponse::new(job_id, status);

        assert_eq!(response.job_id, job_id.to_string());
        assert!(response.status.is_pending());
    }

    #[test]
    fn test_job_response_serialization() {
        let job_id = Uuid::new_v4();
        let response = JobResponse::new(
            job_id,
            JobStatus::Running {
                progress_percent: 42,
            },
        );

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(&job_id.to_string()));
        assert!(json.contains("running"));
        assert!(json.contains("42"));
    }

    // ================================================================
    // JobCreatedResponse Tests
    // ================================================================

    #[test]
    fn test_job_created_response() {
        let job_id = Uuid::new_v4();
        let response = JobCreatedResponse::new(job_id);

        assert_eq!(response.job_id, job_id.to_string());
        assert!(response.message.contains("Poll"));
    }
}
