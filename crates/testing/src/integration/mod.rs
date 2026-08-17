// SPDX-License-Identifier: AGPL-3.0-or-later
// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Integration testing utilities

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use std::sync::RwLock;
use toadstool::ToadStoolResult as Result;
use tracing::info;

/// Integration test result
#[derive(Debug, Clone)]
pub struct IntegrationTestResult {
    /// Name of the integration test
    pub test_name: String,
    /// Outcome of the test run
    pub status: TestStatus,
    /// Time taken to execute the test
    pub duration: Duration,
    /// Human-readable status message
    pub message: String,
    /// Optional detailed breakdown of components and metrics
    pub details: Option<IntegrationTestDetails>,
}

/// Integration test details
#[derive(Debug, Clone)]
pub struct IntegrationTestDetails {
    /// List of components exercised by the test
    pub components_tested: Vec<String>,
    /// Key-value test data collected during execution
    pub test_data: HashMap<String, String>,
    /// Resource and performance metrics
    pub metrics: TestMetrics,
    /// Artifacts produced (logs, screenshots, etc.)
    pub artifacts: Vec<TestArtifact>,
}

/// Test status enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestStatus {
    /// All assertions passed
    Passed,
    /// One or more assertions failed
    Failed,
    /// Test was skipped (e.g. unsupported platform)
    Skipped,
    /// Test exceeded its timeout
    Timeout,
}

/// Test metrics collected during execution
#[derive(Debug, Clone)]
pub struct TestMetrics {
    /// Peak memory usage in megabytes
    pub memory_peak_mb: u32,
    /// Average CPU usage percentage
    pub cpu_usage_percent: f32,
    /// Disk I/O in megabytes
    pub disk_io_mb: u32,
    /// Number of network requests made
    pub network_requests: u32,
    /// Additional custom metrics (e.g. throughput, latency)
    pub custom_metrics: HashMap<String, f64>,
}

/// Test artifacts produced during execution
#[derive(Debug, Clone)]
pub struct TestArtifact {
    /// Artifact name for identification
    pub name: String,
    /// Filesystem path to the artifact
    pub path: std::path::PathBuf,
    /// Type of artifact for categorization
    pub artifact_type: ArtifactType,
    /// Size in bytes
    pub size_bytes: u64,
}

/// Type of test artifact produced
#[derive(Debug, Clone)]
pub enum ArtifactType {
    /// Log file output
    LogFile,
    /// Screenshot or visual capture
    Screenshot,
    /// Configuration file
    ConfigFile,
    /// Database dump or state
    Database,
    /// Compiled binary
    Binary,
    /// Other artifact type (custom string)
    Other(String),
}

/// Integration test manager
pub struct IntegrationTestManager {
    _config: IntegrationTestConfig,
    results: Arc<RwLock<Vec<IntegrationTestResult>>>,
    _active_tests: Arc<RwLock<HashMap<String, TestContext>>>,
}

/// Configuration for integration tests
#[derive(Debug, Clone)]
pub struct IntegrationTestConfig {
    /// Maximum number of tests that may run concurrently
    pub max_concurrent_tests: usize,
    /// Default timeout per test
    pub default_timeout: Duration,
    /// Whether to collect resource/performance metrics
    pub collect_metrics: bool,
    /// Whether to save test artifacts to disk
    pub save_artifacts: bool,
    /// Directory for storing test artifacts
    pub artifact_dir: std::path::PathBuf,
    /// Whether to cleanup resources when tests pass
    pub cleanup_on_success: bool,
    /// Whether to cleanup resources when tests fail
    pub cleanup_on_failure: bool,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tests: 10,
            default_timeout: Duration::from_secs(300),
            collect_metrics: true,
            save_artifacts: true,
            artifact_dir: std::path::PathBuf::from("./test_artifacts"),
            cleanup_on_success: true,
            cleanup_on_failure: false,
        }
    }
}

/// Context for a running integration test
#[derive(Debug)]
pub struct TestContext {
    /// Name of the test being executed
    pub test_name: String,
    /// When the test started (for duration calculation)
    pub start_time: std::time::Instant,
    /// Temporary directory for test files
    pub temp_dir: std::path::PathBuf,
    /// Tasks to run on test completion (cleanup)
    pub cleanup_tasks: Vec<CleanupTask>,
    /// Collector for resource and performance metrics
    pub metrics_collector: MetricsCollector,
}

/// Cleanup task to run after test completion
#[derive(Debug)]
pub struct CleanupTask {
    /// Descriptive name for the cleanup task
    pub name: String,
    /// Action to perform (e.g. remove dir, kill process)
    pub action: CleanupAction,
}

/// Action to perform during test cleanup
pub enum CleanupAction {
    /// Remove a directory and its contents
    RemoveDirectory(std::path::PathBuf),
    /// Terminate a process by PID
    KillProcess(u32),
    /// Close a named connection
    CloseConnection(String),
    /// Restore a file from backup (from, to)
    RestoreFile(std::path::PathBuf, std::path::PathBuf),
    /// Custom cleanup closure
    Custom(Box<dyn Fn() -> Result<()> + Send + Sync>),
}

impl std::fmt::Debug for CleanupAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RemoveDirectory(path) => write!(f, "RemoveDirectory({path:?})"),
            Self::KillProcess(pid) => write!(f, "KillProcess({pid})"),
            Self::CloseConnection(conn) => write!(f, "CloseConnection({conn})"),
            Self::RestoreFile(from, to) => write!(f, "RestoreFile({from:?}, {to:?})"),
            Self::Custom(_) => write!(f, "Custom(...)"),
        }
    }
}

/// Metrics collector for integration tests
#[derive(Debug)]
pub struct MetricsCollector {
    /// When collection started
    pub start_time: std::time::Instant,
    /// Accumulated metrics
    pub metrics: TestMetrics,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// Create a new metrics collector with empty metrics
    #[must_use]
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            metrics: TestMetrics {
                memory_peak_mb: 0,
                cpu_usage_percent: 0.0,
                disk_io_mb: 0,
                network_requests: 0,
                custom_metrics: HashMap::new(),
            },
        }
    }

    /// Record a custom metric value
    pub fn record_metric(&mut self, name: &str, value: f64) {
        self.metrics.custom_metrics.insert(name.to_string(), value);
    }

    /// Finalize and return collected metrics (adds duration_ms)
    #[must_use]
    pub fn finalize(self) -> TestMetrics {
        let mut metrics = self.metrics;
        metrics.custom_metrics.insert(
            "duration_ms".to_string(),
            self.start_time.elapsed().as_millis() as f64,
        );
        metrics
    }
}

// ===== Implementation Module =====
// Implementation extracted for better organization

/// Helper functions for integration test sub-components
mod helpers;
/// Integration test manager implementation and test runners
pub mod integration_impl;
