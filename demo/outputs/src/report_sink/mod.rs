//! Report output destinations.
//!
//! This module provides mock implementations of report
//! output destinations (files, email, etc.).

mod email_sender;
mod file_writer;

pub use email_sender::EmailSender;
pub use file_writer::FileWriter;

use serde::{Deserialize, Serialize};

/// Report output destination trait
pub trait ReportSink: Send + Sync {
    /// Send a report
    fn send(&self, report: &Report) -> Result<(), String>;
}

/// Report structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Report ID
    pub report_id: String,
    /// Report title
    pub title: String,
    /// Report type
    pub report_type: ReportFormat,
    /// Content
    pub content: String,
    /// Generated timestamp
    pub generated_at: String,
    /// Recipients (for email)
    pub recipients: Vec<String>,
}

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    /// CSV format
    Csv,
    /// JSON format
    Json,
    /// HTML format
    Html,
    /// PDF format (mock)
    Pdf,
    /// Excel format (mock)
    Excel,
}

impl ReportFormat {
    /// Get file extension
    pub fn extension(&self) -> &'static str {
        match self {
            ReportFormat::Csv => "csv",
            ReportFormat::Json => "json",
            ReportFormat::Html => "html",
            ReportFormat::Pdf => "pdf",
            ReportFormat::Excel => "xlsx",
        }
    }

    /// Get MIME type
    pub fn mime_type(&self) -> &'static str {
        match self {
            ReportFormat::Csv => "text/csv",
            ReportFormat::Json => "application/json",
            ReportFormat::Html => "text/html",
            ReportFormat::Pdf => "application/pdf",
            ReportFormat::Excel => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        }
    }
}
