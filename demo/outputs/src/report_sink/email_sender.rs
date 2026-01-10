//! Mock email sender for reports.

use super::{Report, ReportSink};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tracing::info;

/// Mock email sender
pub struct EmailSender {
    /// SMTP server (mock)
    smtp_server: String,
    /// From address
    from_address: String,
    /// Sent emails log
    sent_emails: Arc<RwLock<Vec<SentEmail>>>,
}

/// Record of a sent email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentEmail {
    /// Email ID
    pub email_id: String,
    /// From address
    pub from: String,
    /// To addresses
    pub to: Vec<String>,
    /// Subject
    pub subject: String,
    /// Body preview
    pub body_preview: String,
    /// Attachment name
    pub attachment: Option<String>,
    /// Sent timestamp
    pub sent_at: String,
}

impl EmailSender {
    /// Create a new email sender
    pub fn new(smtp_server: &str, from_address: &str) -> Self {
        Self {
            smtp_server: smtp_server.to_string(),
            from_address: from_address.to_string(),
            sent_emails: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create with default settings (mock)
    pub fn mock() -> Self {
        Self::new("smtp.frictionalbank.local", "reports@frictionalbank.local")
    }

    /// Send a report via email
    pub fn send_report(&self, report: &Report) -> Result<String, String> {
        if report.recipients.is_empty() {
            return Err("No recipients specified".to_string());
        }

        let email_id = format!("EMAIL-{}", chrono::Utc::now().timestamp_millis());

        let email = SentEmail {
            email_id: email_id.clone(),
            from: self.from_address.clone(),
            to: report.recipients.clone(),
            subject: format!("[FrictionalBank] {}", report.title),
            body_preview: report.content.chars().take(100).collect(),
            attachment: Some(format!(
                "{}.{}",
                report.report_id,
                report.report_type.extension()
            )),
            sent_at: chrono::Utc::now().to_rfc3339(),
        };

        {
            let mut emails = self.sent_emails.write().unwrap();
            emails.push(email.clone());
        }

        info!(
            email_id = %email_id,
            to = ?report.recipients,
            subject = %email.subject,
            "Email sent (mock)"
        );

        Ok(email_id)
    }

    /// Get sent emails
    pub fn get_sent_emails(&self) -> Vec<SentEmail> {
        let emails = self.sent_emails.read().unwrap();
        emails.clone()
    }

    /// Get email by ID
    pub fn get_email(&self, email_id: &str) -> Option<SentEmail> {
        let emails = self.sent_emails.read().unwrap();
        emails.iter().find(|e| e.email_id == email_id).cloned()
    }

    /// Get SMTP server
    pub fn smtp_server(&self) -> &str {
        &self.smtp_server
    }

    /// Get from address
    pub fn from_address(&self) -> &str {
        &self.from_address
    }

    /// Get email count
    pub fn email_count(&self) -> usize {
        let emails = self.sent_emails.read().unwrap();
        emails.len()
    }
}

impl ReportSink for EmailSender {
    fn send(&self, report: &Report) -> Result<(), String> {
        self.send_report(report)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report_sink::ReportFormat;

    #[test]
    fn test_email_sender() {
        let sender = EmailSender::mock();

        let report = Report {
            report_id: "RPT001".to_string(),
            title: "Daily Risk Report".to_string(),
            report_type: ReportFormat::Html,
            content: "<html><body>Test</body></html>".to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            recipients: vec!["risk@bank.com".to_string()],
        };

        let email_id = sender.send_report(&report).unwrap();
        assert!(email_id.starts_with("EMAIL-"));
        assert_eq!(sender.email_count(), 1);
    }
}
