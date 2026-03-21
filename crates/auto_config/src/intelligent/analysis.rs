// SPDX-License-Identifier: AGPL-3.0-only
//! Pattern recognition and usage learning (Pipeline Stage 2)

use std::time::Duration;

use tracing::debug;

use crate::ToadStoolResult;
use crate::hardware::{PerformanceClass, SystemCapabilities};
use toadstool_config::ToadStoolConfig;

/// Usage pattern learning and prediction from environment.
pub struct UsageLearner {
    /// Hints extracted from the environment.
    pub environment_hints: Vec<EnvironmentHint>,
}

impl Default for UsageLearner {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageLearner {
    /// Creates a new usage learner.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            environment_hints: Vec::new(),
        }
    }

    /// Analyze the environment to predict usage patterns
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if environment detection fails or
    /// file system operations encounter errors.
    #[must_use = "Environment analysis result should be checked"]
    pub async fn analyze_environment(&mut self) -> ToadStoolResult<UsageHints> {
        let mut usage_hints = UsageHints::default();

        // Check for development environment indicators
        if self.is_development_environment().await? {
            usage_hints
                .predicted_workload_types
                .push("development".to_string());
            usage_hints.expected_cpu_usage = 0.3; // Moderate CPU usage
            usage_hints.expected_memory_usage = 0.4; // Moderate memory usage
        }

        // Check for machine learning environment indicators
        if self.is_ml_environment().await? {
            usage_hints
                .predicted_workload_types
                .push("machine_learning".to_string());
            usage_hints.expected_cpu_usage = 0.8; // High CPU usage
            usage_hints.expected_memory_usage = 0.7; // High memory usage
            usage_hints.prefers_gpu = true;
        }

        // Check for web development indicators
        if self.is_web_development_environment().await? {
            usage_hints
                .predicted_workload_types
                .push("web_development".to_string());
            usage_hints.expected_cpu_usage = 0.4; // Moderate CPU usage
            usage_hints.expected_memory_usage = 0.3; // Lower memory usage
            usage_hints.prefers_containers = true;
        }

        // Check for data processing indicators
        if self.is_data_processing_environment().await? {
            usage_hints
                .predicted_workload_types
                .push("data_processing".to_string());
            usage_hints.expected_cpu_usage = 0.6; // Moderate-high CPU usage
            usage_hints.expected_memory_usage = 0.8; // High memory usage
        }

        debug!(
            "Usage pattern analysis complete: {:?}",
            usage_hints.predicted_workload_types
        );
        Ok(usage_hints)
    }

    /// Check if this appears to be a development environment
    async fn is_development_environment(&self) -> ToadStoolResult<bool> {
        // Look for common development tools and directories
        let dev_indicators = [
            ".git",
            ".gitignore",
            "package.json",
            "Cargo.toml",
            "requirements.txt",
            "node_modules",
            "target",
            "__pycache__",
            ".vscode",
            ".idea",
        ];

        for indicator in &dev_indicators {
            if tokio::fs::metadata(indicator).await.is_ok() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if this appears to be a machine learning environment
    async fn is_ml_environment(&self) -> ToadStoolResult<bool> {
        // Look for ML-specific files and tools
        let ml_indicators = [
            "requirements.txt",
            "environment.yml",
            "conda-meta",
            "jupyter",
            ".ipynb",
            "model.pkl",
            "data.csv",
        ];

        for indicator in &ml_indicators {
            if tokio::fs::metadata(indicator).await.is_ok() {
                return Ok(true);
            }
        }

        // Check for Python ML packages
        if tokio::process::Command::new("python")
            .arg("-c")
            .arg("import torch, tensorflow, scikit-learn")
            .output()
            .await
            .is_ok()
        {
            return Ok(true);
        }

        Ok(false)
    }

    /// Check if this appears to be a web development environment
    async fn is_web_development_environment(&self) -> ToadStoolResult<bool> {
        // Look for web development indicators
        let web_indicators = [
            "package.json",
            "yarn.lock",
            "package-lock.json",
            "webpack.config.js",
            "rollup.config.js",
            "vite.config.js",
            "src",
            "public",
            "dist",
            "build",
        ];

        for indicator in &web_indicators {
            if tokio::fs::metadata(indicator).await.is_ok() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if this appears to be a data processing environment
    async fn is_data_processing_environment(&self) -> ToadStoolResult<bool> {
        // Look for data processing indicators
        let data_indicators = [
            "data",
            "datasets",
            "*.csv",
            "*.parquet",
            "*.json",
            "Pipfile",
            "dask",
            "spark",
        ];

        for indicator in &data_indicators {
            if tokio::fs::metadata(indicator).await.is_ok() {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// Usage pattern hints for optimization.
#[derive(Debug, Clone, Default)]
pub struct UsageHints {
    /// Predicted workload types (e.g. development, machine_learning).
    pub predicted_workload_types: Vec<String>,
    /// Expected CPU usage (0–1).
    pub expected_cpu_usage: f64,
    /// Expected memory usage (0–1).
    pub expected_memory_usage: f64,
    /// Whether GPU is preferred.
    pub prefers_gpu: bool,
    /// Whether containers are preferred.
    pub prefers_containers: bool,
}

impl UsageHints {
    /// Returns true if expected CPU usage exceeds 70%.
    #[must_use]
    pub fn is_cpu_intensive(&self) -> bool {
        self.expected_cpu_usage > 0.7
    }

    /// Returns true if expected memory usage exceeds 70%.
    #[must_use]
    pub fn is_memory_intensive(&self) -> bool {
        self.expected_memory_usage > 0.7
    }
}

/// Environment hint for usage pattern detection.
#[derive(Debug, Clone)]
pub struct EnvironmentHint {
    /// Hint type identifier.
    pub hint_type: String,
    /// Confidence score (0–1).
    pub confidence: f64,
    /// Human-readable description.
    pub description: String,
}

/// Configuration snapshot for learning and optimization.
#[derive(Debug, Clone)]
pub struct ConfigSnapshot {
    /// When the snapshot was taken.
    pub timestamp: std::time::SystemTime,
    /// Current configuration.
    pub config: ToadStoolConfig,
    /// Detected hardware capabilities.
    pub hardware: SystemCapabilities,
    /// Usage pattern hints.
    pub usage_hints: UsageHints,
    /// Performance metrics (if available).
    pub performance_metrics: Option<PerformanceMetrics>,
}

/// Performance metrics for configuration optimization.
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Average execution time.
    pub avg_execution_time: Duration,
    /// Peak memory usage (fraction of total).
    pub memory_usage_peak: f64,
    /// Average CPU usage (0–1).
    pub cpu_usage_avg: f64,
    /// Throughput in executions per second.
    pub throughput_executions_per_sec: f64,
}

/// Classify performance based on hardware capabilities
#[must_use]
pub fn classify_performance(hardware: &SystemCapabilities) -> PerformanceClass {
    if hardware.cpu_cores >= 16.0 && hardware.memory_gb >= 32.0 && hardware.gpu_count > 0 {
        PerformanceClass::HighEnd
    } else if hardware.cpu_cores >= 8.0 && hardware.memory_gb >= 16.0 {
        PerformanceClass::Mainstream
    } else {
        PerformanceClass::LowEnd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_hints_is_cpu_intensive() {
        let hints = UsageHints {
            expected_cpu_usage: 0.5,
            ..Default::default()
        };
        assert!(!hints.is_cpu_intensive());
        let hints = UsageHints {
            expected_cpu_usage: 0.8,
            ..Default::default()
        };
        assert!(hints.is_cpu_intensive());
    }

    #[test]
    fn test_usage_hints_is_memory_intensive() {
        let hints = UsageHints {
            expected_memory_usage: 0.5,
            ..Default::default()
        };
        assert!(!hints.is_memory_intensive());
        let hints = UsageHints {
            expected_memory_usage: 0.8,
            ..Default::default()
        };
        assert!(hints.is_memory_intensive());
    }

    #[test]
    fn test_classify_performance_high_end() {
        let hardware = SystemCapabilities {
            cpu_cores: 32.0,
            memory_gb: 64.0,
            gpu_count: 2,
            ..Default::default()
        };
        assert_eq!(classify_performance(&hardware), PerformanceClass::HighEnd);
    }

    #[test]
    fn test_classify_performance_mainstream() {
        let hardware = SystemCapabilities {
            cpu_cores: 8.0,
            memory_gb: 16.0,
            gpu_count: 0,
            ..Default::default()
        };
        assert_eq!(
            classify_performance(&hardware),
            PerformanceClass::Mainstream
        );
    }

    #[test]
    fn test_classify_performance_low_end() {
        let hardware = SystemCapabilities {
            cpu_cores: 2.0,
            memory_gb: 4.0,
            gpu_count: 0,
            ..Default::default()
        };
        assert_eq!(classify_performance(&hardware), PerformanceClass::LowEnd);
    }
}
