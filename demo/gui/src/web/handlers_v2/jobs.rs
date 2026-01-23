//! Async job management endpoints.
//!
//! This module provides endpoints for:
//! - Job status retrieval (`/api/v1/jobs/{id}`)
//! - Job listing (`/api/v1/jobs`)

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::web::{jobs::JobResponse, AppState};

// =============================================================================
// Types
// =============================================================================

/// Path parameter for job ID.
#[derive(Debug, Deserialize)]
pub struct JobPathParams {
    /// Job ID (UUID format).
    pub id: String,
}

/// Job status error response.
#[derive(Debug, Serialize)]
pub struct JobErrorResponse {
    /// Error code.
    pub code: String,
    /// Error message.
    pub message: String,
}

/// Job list response.
#[derive(Debug, Serialize)]
pub struct JobListResponse {
    /// Total number of jobs.
    pub total: usize,
    /// Active (non-terminal) job count.
    pub active: usize,
    /// List of jobs.
    pub jobs: Vec<JobResponse>,
}

// =============================================================================
// Handlers
// =============================================================================

/// Get job status by ID.
///
/// GET /api/v1/jobs/{id}
///
/// Returns the status of a specific async job.
///
/// # Responses
///
/// - 200: Job status returned successfully
/// - 400: Invalid job ID format
/// - 404: Job not found
pub async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(params): Path<JobPathParams>,
) -> impl IntoResponse {
    // Parse job ID
    let job_id = match Uuid::parse_str(&params.id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(JobErrorResponse {
                    code: "INVALID_JOB_ID".to_string(),
                    message: format!("Invalid job ID format: {}", params.id),
                }),
            )
                .into_response();
        }
    };

    // Get job status
    match state.job_manager.get_status(job_id).await {
        Some(status) => {
            let response = JobResponse::new(job_id, status);
            (StatusCode::OK, Json(response)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(JobErrorResponse {
                code: "JOB_NOT_FOUND".to_string(),
                message: format!("Job not found: {}", params.id),
            }),
        )
            .into_response(),
    }
}

/// List all jobs.
///
/// GET /api/v1/jobs
///
/// Returns a list of all async jobs with their statuses.
pub async fn list_jobs(State(state): State<Arc<AppState>>) -> Json<JobListResponse> {
    let job_ids = state.job_manager.list_jobs().await;
    let active = state.job_manager.active_count().await;

    let mut jobs = Vec::with_capacity(job_ids.len());
    for job_id in job_ids {
        if let Some(status) = state.job_manager.get_status(job_id).await {
            jobs.push(JobResponse::new(job_id, status));
        }
    }

    Json(JobListResponse {
        total: jobs.len(),
        active,
        jobs,
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_error_response_serialisation() {
        let error = JobErrorResponse {
            code: "JOB_NOT_FOUND".to_string(),
            message: "Job not found".to_string(),
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"code\":\"JOB_NOT_FOUND\""));
    }

    #[test]
    fn test_job_list_response_serialisation() {
        let response = JobListResponse {
            total: 0,
            active: 0,
            jobs: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"active\":0"));
    }
}
