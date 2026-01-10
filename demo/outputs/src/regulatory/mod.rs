//! Regulatory reporting systems.
//!
//! This module provides mock implementations of regulatory reporting
//! APIs and audit trail storage.

mod audit_store;
mod regulator_api;

pub use audit_store::AuditStore;
pub use regulator_api::RegulatorApi;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Regulatory report types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    /// SA-CCR (Standardised Approach for Counterparty Credit Risk)
    SaCcr,
    /// FRTB (Fundamental Review of the Trading Book)
    Frtb,
    /// CVA Capital
    CvaCapital,
    /// Large Exposure
    LargeExposure,
    /// Liquidity Coverage Ratio
    Lcr,
    /// Net Stable Funding Ratio
    Nsfr,
}

/// A regulatory report submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryReport {
    /// Report ID
    pub report_id: String,
    /// Report type
    pub report_type: ReportType,
    /// Reporting date
    pub reporting_date: DateTime<Utc>,
    /// Submission timestamp
    pub submitted_at: DateTime<Utc>,
    /// Report payload (JSON)
    pub payload: serde_json::Value,
    /// Status
    pub status: ReportStatus,
}

/// Report submission status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportStatus {
    /// Pending submission
    Pending,
    /// Successfully submitted
    Submitted,
    /// Acknowledged by regulator
    Acknowledged,
    /// Rejected with errors
    Rejected,
}
