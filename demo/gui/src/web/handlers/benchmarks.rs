//! Benchmark-related handlers.
//!
//! This module provides benchmark handlers:
//! - `/api/benchmark/speed-comparison` - Get speed comparison chart data

use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};

// =============================================================================
// Types
// =============================================================================

/// Speed comparison benchmark data
struct SpeedComparisonData {
    aad_mean_ns: f64,
    bump_mean_ns: f64,
    speedup_ratio: f64,
    tenor_count: usize,
}

impl SpeedComparisonData {
    fn new(aad_ns: f64, bump_ns: f64, tenor_count: usize) -> Self {
        let speedup = bump_ns / aad_ns.max(1.0);
        Self {
            aad_mean_ns: aad_ns,
            bump_mean_ns: bump_ns,
            speedup_ratio: speedup,
            tenor_count,
        }
    }

    fn sample() -> Self {
        Self::new(150_000.0, 2_500_000.0, 20)
    }

    fn aad_mean_us(&self) -> f64 {
        self.aad_mean_ns / 1000.0
    }

    fn bump_mean_us(&self) -> f64 {
        self.bump_mean_ns / 1000.0
    }
}

/// Query parameters for speed comparison endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct SpeedComparisonQueryParams {
    /// AAD mean time in nanoseconds (optional, uses sample data if not provided)
    pub aad_mean_ns: Option<f64>,
    /// Bump mean time in nanoseconds (optional, uses sample data if not provided)
    pub bump_mean_ns: Option<f64>,
    /// Number of tenor points (optional, defaults to 20)
    pub tenor_count: Option<usize>,
}

/// Speed comparison chart response (Chart.js compatible)
#[derive(Debug, Clone, Serialize)]
pub struct SpeedComparisonResponse {
    /// Chart type (always "bar")
    #[serde(rename = "type")]
    pub chart_type: String,
    /// Chart data
    pub data: SpeedComparisonChartData,
    /// Chart options
    pub options: SpeedComparisonChartOptions,
    /// Raw benchmark data for additional processing
    pub benchmark: SpeedComparisonBenchmarkData,
}

/// Chart.js compatible data structure
#[derive(Debug, Clone, Serialize)]
pub struct SpeedComparisonChartData {
    /// X-axis labels
    pub labels: Vec<String>,
    /// Chart datasets
    pub datasets: Vec<SpeedComparisonDataset>,
}

/// Chart.js compatible dataset
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedComparisonDataset {
    /// Dataset label
    pub label: String,
    /// Data values
    pub data: Vec<f64>,
    /// Background colours
    pub background_color: Vec<String>,
}

/// Chart.js options
#[derive(Debug, Clone, Serialize)]
pub struct SpeedComparisonChartOptions {
    /// Title configuration
    pub title: SpeedComparisonTitleOptions,
}

/// Chart.js title options
#[derive(Debug, Clone, Serialize)]
pub struct SpeedComparisonTitleOptions {
    /// Whether to display the title
    pub display: bool,
    /// Title text
    pub text: String,
}

/// Raw benchmark data for client-side processing
#[derive(Debug, Clone, Serialize)]
pub struct SpeedComparisonBenchmarkData {
    /// AAD mean time in microseconds
    pub aad_mean_us: f64,
    /// Bump mean time in microseconds
    pub bump_mean_us: f64,
    /// Speedup ratio (bump / aad)
    pub speedup_ratio: f64,
    /// Number of tenor points
    pub tenor_count: usize,
}

// =============================================================================
// Handlers
// =============================================================================

/// Get speed comparison chart data endpoint
///
/// GET /api/benchmark/speed-comparison
pub async fn get_speed_comparison(
    Query(params): Query<SpeedComparisonQueryParams>,
) -> Json<SpeedComparisonResponse> {
    let data = if let (Some(aad_ns), Some(bump_ns)) = (params.aad_mean_ns, params.bump_mean_ns) {
        let tenor_count = params.tenor_count.unwrap_or(20);
        SpeedComparisonData::new(aad_ns, bump_ns, tenor_count)
    } else {
        SpeedComparisonData::sample()
    };

    Json(SpeedComparisonResponse {
        chart_type: "bar".to_string(),
        data: SpeedComparisonChartData {
            labels: vec!["AAD".to_string(), "Bump".to_string()],
            datasets: vec![SpeedComparisonDataset {
                label: "Computation Time (μs)".to_string(),
                data: vec![data.aad_mean_us(), data.bump_mean_us()],
                background_color: vec![
                    "rgba(54, 162, 235, 0.8)".to_string(),
                    "rgba(255, 99, 132, 0.8)".to_string(),
                ],
            }],
        },
        options: SpeedComparisonChartOptions {
            title: SpeedComparisonTitleOptions {
                display: true,
                text: format!("AAD vs Bump: {:.1}x faster", data.speedup_ratio),
            },
        },
        benchmark: SpeedComparisonBenchmarkData {
            aad_mean_us: data.aad_mean_us(),
            bump_mean_us: data.bump_mean_us(),
            speedup_ratio: data.speedup_ratio,
            tenor_count: data.tenor_count,
        },
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speed_comparison_data_new() {
        let data = SpeedComparisonData::new(100_000.0, 1_000_000.0, 10);
        assert_eq!(data.tenor_count, 10);
        assert!((data.speedup_ratio - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_speed_comparison_data_sample() {
        let data = SpeedComparisonData::sample();
        assert_eq!(data.tenor_count, 20);
        assert!(data.speedup_ratio > 1.0);
    }

    #[test]
    fn test_aad_mean_us() {
        let data = SpeedComparisonData::new(1_000_000.0, 10_000_000.0, 20);
        assert_eq!(data.aad_mean_us(), 1000.0);
    }
}
