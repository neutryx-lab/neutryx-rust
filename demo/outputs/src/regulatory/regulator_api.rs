//! Mock regulator API for receiving regulatory reports.

use super::{RegulatoryReport, ReportStatus, ReportType};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;

/// Submission log entry for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionLog {
    /// Report ID assigned by the regulator
    pub report_id: String,
    /// Report type
    pub report_type: ReportType,
    /// Reporting date from request
    pub reporting_date: String,
    /// Timestamp when submitted
    pub submitted_at: String,
    /// Submission status
    pub status: ReportStatus,
    /// Original request data
    pub request_data: serde_json::Value,
    /// Error messages if any
    pub errors: Vec<String>,
}

/// Mock regulatory authority API
pub struct RegulatorApi {
    /// Received reports
    reports: Arc<RwLock<HashMap<String, RegulatoryReport>>>,
    /// Validation rules (simple mock)
    validation_enabled: bool,
    /// Whether to log submissions
    logging_enabled: bool,
    /// Submission logs for audit trail
    submission_logs: Arc<RwLock<Vec<SubmissionLog>>>,
}

/// Report submission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionRequest {
    /// Report type
    pub report_type: ReportType,
    /// Reporting date (YYYY-MM-DD)
    pub reporting_date: String,
    /// Report data
    pub data: serde_json::Value,
}

/// Report submission response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResponse {
    /// Assigned report ID
    pub report_id: String,
    /// Submission status
    pub status: ReportStatus,
    /// Error messages if rejected
    pub errors: Vec<String>,
    /// Timestamp
    pub timestamp: String,
}

impl RegulatorApi {
    /// Create a new regulator API
    pub fn new() -> Self {
        Self {
            reports: Arc::new(RwLock::new(HashMap::new())),
            validation_enabled: true,
            logging_enabled: false,
            submission_logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Disable validation (for testing)
    pub fn without_validation(mut self) -> Self {
        self.validation_enabled = false;
        self
    }

    /// Enable or disable submission logging
    pub fn with_logging(mut self, enabled: bool) -> Self {
        self.logging_enabled = enabled;
        self
    }

    /// Submit a regulatory report
    pub fn submit(&self, request: SubmissionRequest) -> SubmissionResponse {
        let report_id = format!(
            "{}-{}-{}",
            format!("{:?}", request.report_type).to_uppercase(),
            request.reporting_date.replace('-', ""),
            Utc::now().timestamp_millis() % 100000
        );

        // Simple validation
        let (status, errors) = if self.validation_enabled {
            self.validate(&request)
        } else {
            (ReportStatus::Acknowledged, vec![])
        };

        let submitted_at = Utc::now();

        let report = RegulatoryReport {
            report_id: report_id.clone(),
            report_type: request.report_type,
            reporting_date: submitted_at, // Simplified
            submitted_at,
            payload: request.data.clone(),
            status,
        };

        // Store report
        {
            let mut reports = self.reports.write().unwrap();
            reports.insert(report_id.clone(), report);
        }

        // Record submission log if logging is enabled
        if self.logging_enabled {
            let log_entry = SubmissionLog {
                report_id: report_id.clone(),
                report_type: request.report_type,
                reporting_date: request.reporting_date.clone(),
                submitted_at: submitted_at.to_rfc3339(),
                status,
                request_data: request.data,
                errors: errors.clone(),
            };
            let mut logs = self.submission_logs.write().unwrap();
            logs.push(log_entry);
        }

        info!(
            report_id = %report_id,
            report_type = ?request.report_type,
            status = ?status,
            "Regulatory report received"
        );

        SubmissionResponse {
            report_id,
            status,
            errors,
            timestamp: submitted_at.to_rfc3339(),
        }
    }

    /// Validate a report submission
    fn validate(&self, request: &SubmissionRequest) -> (ReportStatus, Vec<String>) {
        let mut errors = Vec::new();

        // Check required fields based on report type
        match request.report_type {
            ReportType::SaCcr => {
                if request.data.get("ead").is_none() {
                    errors.push("Missing required field: ead".to_string());
                }
                if request.data.get("netting_sets").is_none() {
                    errors.push("Missing required field: netting_sets".to_string());
                }
            }
            ReportType::CvaCapital => {
                if request.data.get("cva_capital").is_none() {
                    errors.push("Missing required field: cva_capital".to_string());
                }
            }
            _ => {}
        }

        if errors.is_empty() {
            (ReportStatus::Acknowledged, errors)
        } else {
            (ReportStatus::Rejected, errors)
        }
    }

    /// Get a submitted report by ID
    pub fn get_report(&self, report_id: &str) -> Option<RegulatoryReport> {
        let reports = self.reports.read().unwrap();
        reports.get(report_id).cloned()
    }

    /// Get all reports of a specific type
    pub fn get_reports_by_type(&self, report_type: ReportType) -> Vec<RegulatoryReport> {
        let reports = self.reports.read().unwrap();
        reports
            .values()
            .filter(|r| r.report_type == report_type)
            .cloned()
            .collect()
    }

    /// Get report statistics
    pub fn get_statistics(&self) -> ReportStatistics {
        let reports = self.reports.read().unwrap();
        let total = reports.len();
        let acknowledged = reports
            .values()
            .filter(|r| r.status == ReportStatus::Acknowledged)
            .count();
        let rejected = reports
            .values()
            .filter(|r| r.status == ReportStatus::Rejected)
            .count();

        ReportStatistics {
            total,
            acknowledged,
            rejected,
            pending: total - acknowledged - rejected,
        }
    }

    /// Get all submission logs
    pub fn get_submission_logs(&self) -> Vec<SubmissionLog> {
        let logs = self.submission_logs.read().unwrap();
        logs.clone()
    }

    /// Export submission logs as JSON string
    pub fn export_logs_json(&self) -> String {
        let logs = self.submission_logs.read().unwrap();
        serde_json::to_string_pretty(&*logs).unwrap_or_default()
    }

    /// Clear submission logs
    pub fn clear_logs(&self) {
        let mut logs = self.submission_logs.write().unwrap();
        logs.clear();
    }
}

impl Default for RegulatorApi {
    fn default() -> Self {
        Self::new()
    }
}

/// Report statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportStatistics {
    /// Total reports
    pub total: usize,
    /// Acknowledged reports
    pub acknowledged: usize,
    /// Rejected reports
    pub rejected: usize,
    /// Pending reports
    pub pending: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regulator_api_submit() {
        let api = RegulatorApi::new().without_validation();
        let request = SubmissionRequest {
            report_type: ReportType::SaCcr,
            reporting_date: "2026-01-09".to_string(),
            data: serde_json::json!({"ead": 1000000}),
        };
        let response = api.submit(request);
        assert_eq!(response.status, ReportStatus::Acknowledged);
    }

    #[test]
    fn test_regulator_api_with_logging() {
        let api = RegulatorApi::new()
            .without_validation()
            .with_logging(true);

        let request = SubmissionRequest {
            report_type: ReportType::SaCcr,
            reporting_date: "2026-01-10".to_string(),
            data: serde_json::json!({"ead": 500000, "netting_sets": []}),
        };

        let response = api.submit(request);
        assert_eq!(response.status, ReportStatus::Acknowledged);

        // Check that the log entry was recorded
        let logs = api.get_submission_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].report_id, response.report_id);
        assert_eq!(logs[0].report_type, ReportType::SaCcr);
        assert_eq!(logs[0].status, ReportStatus::Acknowledged);
    }

    #[test]
    fn test_regulator_api_export_logs() {
        let api = RegulatorApi::new()
            .without_validation()
            .with_logging(true);

        // Submit multiple reports
        for i in 0..3 {
            let request = SubmissionRequest {
                report_type: ReportType::CvaCapital,
                reporting_date: format!("2026-01-{:02}", 10 + i),
                data: serde_json::json!({"cva_capital": 100000 * (i + 1)}),
            };
            api.submit(request);
        }

        let logs = api.get_submission_logs();
        assert_eq!(logs.len(), 3);

        // Export logs as JSON
        let json = api.export_logs_json();
        assert!(json.contains("CvaCapital"));
        assert!(json.contains("report_id"));
    }

    #[test]
    fn test_regulator_api_log_entry_details() {
        let api = RegulatorApi::new()
            .without_validation()
            .with_logging(true);

        let request = SubmissionRequest {
            report_type: ReportType::Frtb,
            reporting_date: "2026-01-10".to_string(),
            data: serde_json::json!({"market_risk": 250000}),
        };

        let response = api.submit(request);
        let logs = api.get_submission_logs();

        assert_eq!(logs[0].report_id, response.report_id);
        assert_eq!(logs[0].reporting_date, "2026-01-10");
        assert!(logs[0].submitted_at.len() > 0);
        assert!(logs[0].request_data.get("market_risk").is_some());
    }
}
