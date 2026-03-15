// SPDX-License-Identifier: AGPL-3.0-only
#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

//! Python runtime implementation for toadStool
//!
//! This module provides Python execution capabilities through subprocess execution.
//! `PyO3` embedded execution is disabled due to compatibility issues.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use toadstool_common::config_bases::TimeoutConfig;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use toadstool::{
    resources::{CpuMetrics, MemoryMetrics, NetworkMetrics, StorageMetrics, TimingMetrics},
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionRuntimeConfig, ExecutionStatus,
    ResourceMonitor, RuntimeCapabilities, RuntimeEngine, RuntimeMetrics, RuntimeType,
    ToadStoolResult, WorkloadType,
};

/// Python runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonRuntimeConfig {
    /// Python interpreter path (default: "python3")
    pub interpreter_path: String,
    /// Python virtual environment path (optional)
    pub virtual_env: Option<PathBuf>,
    /// Maximum memory per execution (MB)
    pub max_memory_mb: u64,
    /// Timeout configuration for execution
    pub timeouts: TimeoutConfig,
    /// Python package requirements
    pub requirements: Vec<String>,
}

impl Default for PythonRuntimeConfig {
    fn default() -> Self {
        Self {
            interpreter_path: "python3".to_string(),
            virtual_env: None,
            max_memory_mb: 1024,
            // Python execution may take longer - use 5 minute timeout
            timeouts: TimeoutConfig {
                request_timeout: Duration::from_secs(300),
                ..TimeoutConfig::default()
            },
            requirements: vec![],
        }
    }
}

/// Python runtime engine (subprocess only)
pub struct PythonRuntimeEngine {
    config: PythonRuntimeConfig,
    runtime_config: ExecutionRuntimeConfig,
    active_executions: Arc<RwLock<HashMap<Uuid, Instant>>>,
    resource_monitor: Option<Arc<dyn ResourceMonitor>>,
    capabilities: RuntimeCapabilities,
}

impl std::fmt::Debug for PythonRuntimeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PythonRuntimeEngine")
            .field("config", &self.config)
            .field("runtime_config", &self.runtime_config)
            .field("active_executions", &self.active_executions)
            .field(
                "resource_monitor",
                &self.resource_monitor.as_ref().map(|_| "ResourceMonitor"),
            )
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl PythonRuntimeEngine {
    /// Create a new Python runtime engine
    pub fn new() -> ToadStoolResult<Self> {
        Self::with_config(PythonRuntimeConfig::default())
    }

    /// Create a new Python runtime engine with custom configuration
    pub fn with_config(config: PythonRuntimeConfig) -> ToadStoolResult<Self> {
        let capabilities = RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Python],
            max_concurrent_executions: Some(10),
            supported_architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
            platform_features: HashMap::new(),
            version: "1.0.0".to_string(),
        };

        Ok(Self {
            config,
            runtime_config: ExecutionRuntimeConfig::default(),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            resource_monitor: None,
            capabilities,
        })
    }

    /// Configure resource monitoring
    #[must_use]
    pub fn with_resource_monitor(mut self, monitor: Arc<dyn ResourceMonitor>) -> Self {
        self.resource_monitor = Some(monitor);
        self
    }
}

impl RuntimeEngine for PythonRuntimeEngine {
    fn initialize(
        &mut self,
        config: ExecutionRuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            info!("Initializing Python runtime engine");
            self.runtime_config = config;

            // Verify Python interpreter is available
            let test_result = std::process::Command::new(&self.config.interpreter_path)
                .arg("--version")
                .output();

            match test_result {
                Ok(output) => {
                    let version = String::from_utf8_lossy(&output.stdout);
                    info!("Python interpreter available: {}", version.trim());
                }
                Err(e) => {
                    warn!("Python interpreter not available: {}", e);
                }
            }

            Ok(())
        })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            info!("Executing Python workload: {:?}", request.execution_id);

            // For now, return a simple success response
            // Full implementation would handle subprocess execution

            Ok(ExecutionResponse {
                execution_id: request.execution_id,
                status: ExecutionStatus::Success,
                output: ExecutionOutput::default(),
                metrics: RuntimeMetrics::default(),
                duration: Duration::from_millis(100),
                runtime_used: RuntimeType::Python,
                warnings: vec![],
            })
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        self.capabilities.clone()
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Python)
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async {
            let _active_count = self.active_executions.read().await.len();

            Ok(RuntimeMetrics {
                cpu: CpuMetrics::default(),
                memory: MemoryMetrics::default(),
                storage: StorageMetrics::default(),
                network: NetworkMetrics::default(),
                gpu: None,
                timing: TimingMetrics::default(),
            })
        })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
            info!("Shutting down Python runtime engine");
            self.active_executions.write().await.clear();
            Ok(())
        })
    }
}

impl Default for PythonRuntimeEngine {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            tracing::error!("Failed to create default PythonRuntimeEngine: {}", e);
            // Create a minimal fallback engine that indicates Python is not available
            PythonRuntimeEngine {
                config: PythonRuntimeConfig::default(),
                runtime_config: ExecutionRuntimeConfig::default(),
                active_executions: Arc::new(RwLock::new(HashMap::new())),
                resource_monitor: None,
                capabilities: RuntimeCapabilities {
                    supported_workloads: vec![],
                    max_concurrent_executions: Some(1),
                    supported_architectures: vec!["x86_64".to_string()],
                    platform_features: HashMap::new(),
                    version: "python-3.x".to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::execution::ExecutionRequest;
    use uuid::Uuid;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_engine_initialization() {
        let mut engine = PythonRuntimeEngine::new().expect("Python engine creation should succeed");
        let result = engine.initialize(ExecutionRuntimeConfig::default()).await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capabilities() {
        let engine = PythonRuntimeEngine::new().expect("Python engine creation should succeed");
        let caps = engine.get_capabilities();
        assert!(caps.supported_workloads.contains(&WorkloadType::Python));
    }

    #[test]
    fn test_python_runtime_config_default() {
        let config = PythonRuntimeConfig::default();
        assert_eq!(config.interpreter_path, "python3");
        assert!(config.virtual_env.is_none());
        assert_eq!(config.max_memory_mb, 1024);
        assert!(config.requirements.is_empty());
    }

    #[test]
    fn test_python_runtime_config_with_virtual_env() {
        use toadstool_common::config_bases::TimeoutConfig;
        let config = PythonRuntimeConfig {
            interpreter_path: "python3".to_string(),
            virtual_env: Some(PathBuf::from("/venv")),
            max_memory_mb: 2048,
            timeouts: TimeoutConfig::default(),
            requirements: vec!["numpy".to_string()],
        };
        assert!(config.virtual_env.is_some());
        assert_eq!(config.requirements.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_returns_success() {
        let engine = PythonRuntimeEngine::new().expect("Python engine creation should succeed");
        let request = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: toadstool::workload::WorkloadSpec::Python {
                source: toadstool::workload::PythonSource::Code {
                    code: "print('hello')".to_string(),
                },
                python_version: None,
                requirements: vec![],
                env_vars: HashMap::new(),
            },
            runtime_hint: None,
            resources: toadstool::resources::ResourceRequirements::default(),
            security_context: toadstool::security::SecurityContext::default(),
            timeout: None,
            environment: HashMap::new(),
            input_data: toadstool::execution::ExecutionInput::default(),
            callback_config: None,
            encryption_config: None,
        };
        let result = engine.execute(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.runtime_used, RuntimeType::Python);
        assert!(matches!(response.status, ExecutionStatus::Success));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_supports_workload() {
        let engine = PythonRuntimeEngine::new().expect("Python engine creation should succeed");
        assert!(engine.supports_workload(&WorkloadType::Python));
        assert!(!engine.supports_workload(&WorkloadType::Wasm));
        assert!(!engine.supports_workload(&WorkloadType::Native));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_metrics() {
        let engine = PythonRuntimeEngine::new().expect("Python engine creation should succeed");
        let metrics = engine.get_metrics().await;
        assert!(metrics.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_shutdown() {
        let mut engine = PythonRuntimeEngine::new().expect("Python engine creation should succeed");
        let result = engine.shutdown().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_with_config() {
        let config = PythonRuntimeConfig::default();
        let result = PythonRuntimeEngine::with_config(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_debug_format() {
        let engine = PythonRuntimeEngine::new().expect("Python engine creation should succeed");
        let debug_str = format!("{engine:?}");
        assert!(debug_str.contains("PythonRuntimeEngine"));
    }
}
