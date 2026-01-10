//! Mock regulator API for receiving regulatory reports.

use super::{RegulatoryReport, ReportStatus, ReportType};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;

/// Mock regulatory authority API
pub struct RegulatorApi {
    /// Received reports
    reports: Arc<RwLock<HashMap<String, RegulatoryReport>>>,
    /// Validation rules (simple mock)
    validation_enabled: bool,
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
        }
    }

    /// Disable validation (for testing)
    pub fn without_validation(mut self) -> Self {
        self.validation_enabled = false;
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

        let report = RegulatoryReport {
            report_id: report_id.clone(),
            report_type: request.report_type,
            reporting_date: Utc::now(), // Simplified
            submitted_at: Utc::now(),
            payload: request.data,
            status,
        };

        // Store report
        {
            let mut reports = self.reports.write().unwrap();
            reports.insert(report_id.clone(), report);
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
            timestamp: Utc::now().to_rfc3339(),
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
}
