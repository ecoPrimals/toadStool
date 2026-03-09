// SPDX-License-Identifier: AGPL-3.0-only
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

use toadstool::ToadStoolResult as Result;
use tokio::sync::RwLock;
use tracing::info;

/// Integration test result
#[derive(Debug, Clone)]
pub struct IntegrationTestResult {
    pub test_name: String,
    pub status: TestStatus,
    pub duration: Duration,
    pub message: String,
    pub details: Option<IntegrationTestDetails>,
}

/// Integration test details
#[derive(Debug, Clone)]
pub struct IntegrationTestDetails {
    pub components_tested: Vec<String>,
    pub test_data: HashMap<String, String>,
    pub metrics: TestMetrics,
    pub artifacts: Vec<TestArtifact>,
}

/// Test status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Timeout,
}

/// Test metrics collected during execution
#[derive(Debug, Clone)]
pub struct TestMetrics {
    pub memory_peak_mb: u32,
    pub cpu_usage_percent: f32,
    pub disk_io_mb: u32,
    pub network_requests: u32,
    pub custom_metrics: HashMap<String, f64>,
}

/// Test artifacts produced during execution
#[derive(Debug, Clone)]
pub struct TestArtifact {
    pub name: String,
    pub path: std::path::PathBuf,
    pub artifact_type: ArtifactType,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub enum ArtifactType {
    LogFile,
    Screenshot,
    ConfigFile,
    Database,
    Binary,
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
    pub max_concurrent_tests: usize,
    pub default_timeout: Duration,
    pub collect_metrics: bool,
    pub save_artifacts: bool,
    pub artifact_dir: std::path::PathBuf,
    pub cleanup_on_success: bool,
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
    pub test_name: String,
    pub start_time: std::time::Instant,
    pub temp_dir: std::path::PathBuf,
    pub cleanup_tasks: Vec<CleanupTask>,
    pub metrics_collector: MetricsCollector,
}

/// Cleanup task to run after test completion
#[derive(Debug)]
pub struct CleanupTask {
    pub name: String,
    pub action: CleanupAction,
}

pub enum CleanupAction {
    RemoveDirectory(std::path::PathBuf),
    KillProcess(u32),
    CloseConnection(String),
    RestoreFile(std::path::PathBuf, std::path::PathBuf),
    Custom(Box<dyn Fn() -> Result<()> + Send + Sync>),
}

impl std::fmt::Debug for CleanupAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CleanupAction::RemoveDirectory(path) => write!(f, "RemoveDirectory({path:?})"),
            CleanupAction::KillProcess(pid) => write!(f, "KillProcess({pid})"),
            CleanupAction::CloseConnection(conn) => write!(f, "CloseConnection({conn})"),
            CleanupAction::RestoreFile(from, to) => write!(f, "RestoreFile({from:?}, {to:?})"),
            CleanupAction::Custom(_) => write!(f, "Custom(...)"),
        }
    }
}

/// Metrics collector for integration tests
#[derive(Debug)]
pub struct MetricsCollector {
    pub start_time: std::time::Instant,
    pub metrics: TestMetrics,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
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

    pub fn record_metric(&mut self, name: &str, value: f64) {
        self.metrics.custom_metrics.insert(name.to_string(), value);
    }

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
mod helpers;
pub mod integration_impl;
