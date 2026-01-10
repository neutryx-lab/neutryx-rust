//! File writer for report output.

use super::{Report, ReportSink};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::info;

/// File writer for reports
pub struct FileWriter {
    /// Output directory
    output_dir: PathBuf,
    /// Written files log
    written_files: Arc<RwLock<Vec<WrittenFile>>>,
}

/// Record of a written file
#[derive(Debug, Clone)]
pub struct WrittenFile {
    /// File path
    pub path: PathBuf,
    /// Report ID
    pub report_id: String,
    /// Size in bytes
    pub size: usize,
    /// Written timestamp
    pub written_at: String,
}

impl FileWriter {
    /// Create a new file writer
    pub fn new(output_dir: impl AsRef<Path>) -> Self {
        let output_dir = output_dir.as_ref().to_path_buf();
        
        // Create directory if it doesn't exist
        if !output_dir.exists() {
            fs::create_dir_all(&output_dir).ok();
        }

        Self {
            output_dir,
            written_files: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Write a report to file
    pub fn write(&self, report: &Report) -> Result<PathBuf, String> {
        let filename = format!(
            "{}_{}.{}",
            report.report_id,
            chrono::Utc::now().format("%Y%m%d_%H%M%S"),
            report.report_type.extension()
        );

        let path = self.output_dir.join(&filename);

        fs::write(&path, &report.content)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        let written = WrittenFile {
            path: path.clone(),
            report_id: report.report_id.clone(),
            size: report.content.len(),
            written_at: chrono::Utc::now().to_rfc3339(),
        };

        {
            let mut files = self.written_files.write().unwrap();
            files.push(written);
        }

        info!(
            path = %path.display(),
            report_id = %report.report_id,
            size = report.content.len(),
            "Report written to file"
        );

        Ok(path)
    }

    /// Get list of written files
    pub fn get_written_files(&self) -> Vec<WrittenFile> {
        let files = self.written_files.read().unwrap();
        files.clone()
    }

    /// Get output directory
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Clean old files (older than days)
    pub fn clean_old_files(&self, days: u32) -> Result<usize, String> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let mut removed = 0;

        let entries = fs::read_dir(&self.output_dir)
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    let modified: chrono::DateTime<chrono::Utc> = modified.into();
                    if modified < cutoff {
                        if fs::remove_file(entry.path()).is_ok() {
                            removed += 1;
                        }
                    }
                }
            }
        }

        Ok(removed)
    }
}

impl ReportSink for FileWriter {
    fn send(&self, report: &Report) -> Result<(), String> {
        self.write(report)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report_sink::ReportFormat;
    use std::env;

    #[test]
    fn test_file_writer() {
        let temp_dir = env::temp_dir().join("neutryx_test");
        let writer = FileWriter::new(&temp_dir);

        let report = Report {
            report_id: "TEST001".to_string(),
            title: "Test Report".to_string(),
            report_type: ReportFormat::Json,
            content: r#"{"test": true}"#.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            recipients: vec![],
        };

        let path = writer.write(&report).unwrap();
        assert!(path.exists());

        // Cleanup
        fs::remove_file(path).ok();
    }
}
