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
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use anyhow::Result;

/// Integration test configuration
#[derive(Debug, Clone)]
pub struct IntegrationTestConfig {
    pub test_name: String,
    pub timeout: Duration,
    pub setup_cleanup: bool,
    pub parallel_execution: bool,
    pub resource_limits: ResourceLimits,
    pub environment_variables: HashMap<String, String>,
}

/// Resource limits for integration tests
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory_mb: Option<u32>,
    pub max_cpu_percent: Option<u32>,
    pub max_execution_time: Duration,
    pub max_disk_usage_mb: Option<u32>,
}

/// Integration test result
#[derive(Debug, Clone)]
pub struct IntegrationTestResult {
    pub test_name: String,
    pub success: bool,
    pub duration: Duration,
    pub error_message: Option<String>,
    pub metrics: TestMetrics,
    pub artifacts: Vec<TestArtifact>,
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
    pub path: PathBuf,
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
    config: IntegrationTestConfig,
    results: Arc<RwLock<Vec<IntegrationTestResult>>>,
    active_tests: Arc<RwLock<HashMap<String, TestContext>>>,
}

/// Context for a running integration test
#[derive(Debug)]
pub struct TestContext {
    pub test_name: String,
    pub start_time: std::time::Instant,
    pub temp_dir: PathBuf,
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
    RemoveDirectory(PathBuf),
    KillProcess(u32),
    CloseConnection(String),
    RestoreFile(PathBuf, PathBuf),
    Custom(Box<dyn Fn() -> Result<()> + Send + Sync>),
}

impl std::fmt::Debug for CleanupAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CleanupAction::RemoveDirectory(path) => f.debug_tuple("RemoveDirectory").field(path).finish(),
            CleanupAction::KillProcess(pid) => f.debug_tuple("KillProcess").field(pid).finish(),
            CleanupAction::CloseConnection(conn) => f.debug_tuple("CloseConnection").field(conn).finish(),
            CleanupAction::RestoreFile(backup, original) => f.debug_tuple("RestoreFile").field(backup).field(original).finish(),
            CleanupAction::Custom(_) => f.debug_tuple("Custom").field(&"<function>").finish(),
        }
    }
}

/// Metrics collector for integration tests
#[derive(Debug)]
pub struct MetricsCollector {
    start_time: std::time::Instant,
    memory_samples: Vec<u32>,
    cpu_samples: Vec<f32>,
    custom_metrics: HashMap<String, Vec<f64>>,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            test_name: "unnamed_test".to_string(),
            timeout: Duration::from_secs(300), // 5 minutes default
            setup_cleanup: true,
            parallel_execution: false,
            resource_limits: ResourceLimits::default(),
            environment_variables: HashMap::new(),
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: Some(1024), // 1GB default
            max_cpu_percent: Some(80),
            max_execution_time: Duration::from_secs(300),
            max_disk_usage_mb: Some(500),
        }
    }
}

impl IntegrationTestManager {
    /// Create a new integration test manager
    pub fn new(config: IntegrationTestConfig) -> Self {
        Self {
            config,
            results: Arc::new(RwLock::new(Vec::new())),
            active_tests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute an integration test
    pub async fn execute_test<F, Fut>(&self, test_fn: F) -> Result<IntegrationTestResult>
    where
        F: FnOnce(TestContext) -> Fut + Send,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let start_time = std::time::Instant::now();
        let test_name = self.config.test_name.clone();
        
        // Create test context
        let temp_dir = std::env::temp_dir().join(format!("toadstool_test_{}", test_name));
        std::fs::create_dir_all(&temp_dir)?;
        
        let context = TestContext {
            test_name: test_name.clone(),
            start_time,
            temp_dir: temp_dir.clone(),
            cleanup_tasks: Vec::new(),
            metrics_collector: MetricsCollector::new(),
        };

        // Register active test
        {
            let mut active = self.active_tests.write().await;
            active.insert(test_name.clone(), context);
        }

        // Execute test with timeout  
        let result = match tokio::time::timeout(self.config.timeout, async {
            // Create context directly for the test function
            let context = TestContext {
                test_name: test_name.clone(),
                start_time,
                temp_dir: temp_dir.clone(),
                cleanup_tasks: Vec::new(),
                metrics_collector: MetricsCollector::new(),
            };
            test_fn(context).await
        }).await {
            Ok(Ok(())) => IntegrationTestResult {
                test_name: test_name.clone(),
                success: true,
                duration: start_time.elapsed(),
                error_message: None,
                metrics: TestMetrics::default(),
                artifacts: Vec::new(),
            },
            Ok(Err(e)) => IntegrationTestResult {
                test_name: test_name.clone(),
                success: false,
                duration: start_time.elapsed(),
                error_message: Some(e.to_string()),
                metrics: TestMetrics::default(),
                artifacts: Vec::new(),
            },
            Err(_) => IntegrationTestResult {
                test_name: test_name.clone(),
                success: false,
                duration: start_time.elapsed(),
                error_message: Some("Test timed out".to_string()),
                metrics: TestMetrics::default(),
                artifacts: Vec::new(),
            },
        };

        // Cleanup
        if self.config.setup_cleanup {
            self.cleanup_test(&test_name).await?;
        }

        // Remove from active tests
        {
            let mut active = self.active_tests.write().await;
            active.remove(&test_name);
        }

        // Store result
        {
            let mut results = self.results.write().await;
            results.push(result.clone());
        }

        Ok(result)
    }

    /// Get all test results
    pub async fn get_results(&self) -> Vec<IntegrationTestResult> {
        self.results.read().await.clone()
    }

    /// Generate test report
    pub async fn generate_report(&self) -> TestReport {
        let results = self.get_results().await;
        let total_tests = results.len();
        let passed_tests = results.iter().filter(|r| r.success).count();
        let failed_tests = total_tests - passed_tests;
        
        TestReport {
            total_tests,
            passed_tests,
            failed_tests,
            total_duration: results.iter().map(|r| r.duration).sum(),
            results,
        }
    }

    async fn cleanup_test(&self, test_name: &str) -> Result<()> {
        if let Some(context) = self.active_tests.read().await.get(test_name) {
            // Remove temp directory
            if context.temp_dir.exists() {
                std::fs::remove_dir_all(&context.temp_dir)?;
            }
            
            // Execute cleanup tasks
            for task in &context.cleanup_tasks {
                if let Err(e) = self.execute_cleanup_task(task).await {
                    eprintln!("Cleanup task '{}' failed: {}", task.name, e);
                }
            }
        }
        Ok(())
    }

    async fn execute_cleanup_task(&self, task: &CleanupTask) -> Result<()> {
        match &task.action {
            CleanupAction::RemoveDirectory(path) => {
                if path.exists() {
                    std::fs::remove_dir_all(path)?;
                }
            }
            CleanupAction::KillProcess(_pid) => {
                // Platform-specific process killing
#[cfg(all(unix, feature = "integration-tests"))]
                {
                    unsafe {
                        libc::kill(*pid as i32, libc::SIGTERM);
                    }
                }
                #[cfg(not(all(unix, feature = "integration-tests")))]
                {
                    eprintln!("Process killing not supported on this platform");
                }
            }
            CleanupAction::CloseConnection(_) => {
                // Close network connections
            }
            CleanupAction::RestoreFile(backup, original) => {
                if backup.exists() {
                    std::fs::copy(backup, original)?;
                    std::fs::remove_file(backup)?;
                }
            }
            CleanupAction::Custom(action) => {
                action()?;
            }
        }
        Ok(())
    }
}

impl TestContext {
    /// Add a cleanup task to be executed after the test
    pub fn add_cleanup_task(&mut self, task: CleanupTask) {
        self.cleanup_tasks.push(task);
    }

    /// Get elapsed time since test start
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Create a test artifact
    pub fn create_artifact(&self, name: &str, artifact_type: ArtifactType, content: &[u8]) -> Result<TestArtifact> {
        let path = self.temp_dir.join(name);
        std::fs::write(&path, content)?;
        
        Ok(TestArtifact {
            name: name.to_string(),
            path,
            artifact_type,
            size_bytes: content.len() as u64,
        })
    }
}

impl MetricsCollector {
    fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            memory_samples: Vec::new(),
            cpu_samples: Vec::new(),
            custom_metrics: HashMap::new(),
        }
    }

    /// Record a memory sample
    pub fn record_memory(&mut self, memory_mb: u32) {
        self.memory_samples.push(memory_mb);
    }

    /// Record a CPU sample
    pub fn record_cpu(&mut self, cpu_percent: f32) {
        self.cpu_samples.push(cpu_percent);
    }

    /// Record a custom metric
    pub fn record_custom_metric(&mut self, name: &str, value: f64) {
        self.custom_metrics.entry(name.to_string()).or_insert_with(Vec::new).push(value);
    }
}

impl Default for TestMetrics {
    fn default() -> Self {
        Self {
            memory_peak_mb: 0,
            cpu_usage_percent: 0.0,
            disk_io_mb: 0,
            network_requests: 0,
            custom_metrics: HashMap::new(),
        }
    }
}

/// Test report summarizing all integration test results
#[derive(Debug, Clone)]
pub struct TestReport {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub total_duration: Duration,
    pub results: Vec<IntegrationTestResult>,
}

impl TestReport {
    /// Get success rate as percentage
    pub fn success_rate(&self) -> f32 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed_tests as f32 / self.total_tests as f32) * 100.0
        }
    }

    /// Generate human-readable report
    pub fn to_string(&self) -> String {
        format!(
            "Integration Test Report\n\
             ======================\n\
             Total Tests: {}\n\
             Passed: {}\n\
             Failed: {}\n\
             Success Rate: {:.1}%\n\
             Total Duration: {:.2}s\n",
            self.total_tests,
            self.passed_tests,
            self.failed_tests,
            self.success_rate(),
            self.total_duration.as_secs_f64()
        )
    }
}
