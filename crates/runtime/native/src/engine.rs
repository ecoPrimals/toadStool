// SPDX-License-Identifier: AGPL-3.0-only
//! Native process runtime engine for executing native executables

use bytes::Bytes;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{info, warn};
use uuid::Uuid;

use toadstool::{
    WorkloadType,
    error::{ToadStoolError, ToadStoolResult},
    execution::{
        ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities,
        RuntimeConfig, RuntimeEngine, RuntimeType,
    },
    resources::{ResourceMonitor, RuntimeMetrics},
    workload::WorkloadSpec,
};

use crate::capabilities;
use crate::process::{self, ProcessHandle};
use crate::security;
use crate::validation;

/// Native process runtime engine; executes native binaries as child processes
pub struct NativeRuntimeEngine {
    pub(crate) config: RuntimeConfig,
    active_processes: Arc<RwLock<HashMap<Uuid, ProcessHandle>>>,
    resource_monitor: Option<Arc<dyn ResourceMonitor>>,
    capabilities: RuntimeCapabilities,
}

impl std::fmt::Debug for NativeRuntimeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeRuntimeEngine")
            .field("config", &self.config)
            .field("active_processes", &self.active_processes)
            .field(
                "resource_monitor",
                &self.resource_monitor.as_ref().map(|_| "ResourceMonitor"),
            )
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl NativeRuntimeEngine {
    /// Create a new native runtime engine with default config
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
            active_processes: Arc::new(RwLock::new(HashMap::new())),
            resource_monitor: None,
            capabilities: capabilities::default_capabilities(),
        }
    }

    /// Attach resource monitor for execution metrics
    #[must_use]
    pub fn with_resource_monitor(mut self, monitor: Arc<dyn ResourceMonitor>) -> Self {
        self.resource_monitor = Some(monitor);
        self
    }

    async fn execute_workload(
        &self,
        request: &ExecutionRequest,
        executable_path: PathBuf,
    ) -> ToadStoolResult<ExecutionResponse> {
        let start_time = Instant::now();

        let args = match &request.workload {
            WorkloadSpec::Native { args, .. } => args.clone().unwrap_or_default(),
            _ => {
                return Err(ToadStoolError::validation(
                    "Invalid workload type for native runtime",
                ));
            }
        };

        let mut command = TokioCommand::new(&executable_path);
        command.args(&args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        for (key, value) in &request.environment {
            command.env(key, value);
        }

        command = security::apply_security_context(command, &request.security_context);

        if let Some(monitor) = &self.resource_monitor {
            monitor.start_monitoring(&request.execution_id.to_string())?;
        }

        let active_processes = Arc::clone(&self.active_processes);
        let execution_future = async move {
            let child = command
                .spawn()
                .map_err(|e| ToadStoolError::runtime(format!("Failed to spawn process: {e}")))?;

            {
                let mut processes = active_processes.write().await;
                processes.insert(
                    request.execution_id,
                    ProcessHandle {
                        child: None,
                        _start_time: start_time,
                        _workload_id: request.execution_id.to_string(),
                        _executable_path: executable_path.clone(),
                    },
                );
            }

            let output = child
                .wait_with_output()
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Process execution failed: {e}")))?;

            Ok::<_, ToadStoolError>(output)
        };

        let output = if let Some(timeout_duration) = request.timeout {
            if let Ok(result) = timeout(timeout_duration, execution_future).await {
                result?
            } else {
                process::cleanup_process(&self.active_processes, &request.execution_id).await;

                return Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::TimedOut,
                    output: ExecutionOutput::default(),
                    metrics: RuntimeMetrics::default(),
                    duration: start_time.elapsed(),
                    runtime_used: RuntimeType::Native,
                    warnings: vec!["Execution timed out".to_string()],
                });
            }
        } else {
            execution_future.await?
        };

        let duration = start_time.elapsed();

        let metrics = if let Some(monitor) = &self.resource_monitor {
            monitor.stop_monitoring(&request.execution_id.to_string())?;
            match monitor.get_metrics(&request.execution_id.to_string()).await {
                Ok(metrics) => metrics,
                Err(e) => {
                    warn!(
                        "Failed to get metrics for execution {}: {}",
                        request.execution_id, e
                    );
                    RuntimeMetrics::default()
                }
            }
        } else {
            RuntimeMetrics::default()
        };

        process::cleanup_process(&self.active_processes, &request.execution_id).await;

        let status = if output.status.success() {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Failed {
                error: format!("Process exited with code: {:?}", output.status.code()).into(),
            }
        };

        Ok(ExecutionResponse {
            execution_id: request.execution_id,
            status,
            output: ExecutionOutput {
                data: Bytes::from(output.stdout.clone()),
                stdout: Some(String::from_utf8_lossy(&output.stdout).to_string()),
                stderr: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                exit_code: output.status.code(),
                format: Some("text/plain".to_string()),
                ..Default::default()
            },
            metrics,
            duration,
            runtime_used: RuntimeType::Native,
            warnings: Vec::new(),
        })
    }
}

impl RuntimeEngine for NativeRuntimeEngine {
    fn initialize(
        &mut self,
        config: RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            info!("Initializing Native Runtime Engine");
            self.config = config;

            let test_command = if cfg!(windows) { "cmd" } else { "echo" };

            let test_result = std::process::Command::new(test_command)
                .arg("test")
                .output();

            match test_result {
                Ok(_) => {
                    info!("Native runtime engine initialized successfully");
                    Ok(())
                }
                Err(e) => Err(ToadStoolError::runtime(format!(
                    "Failed to initialize native runtime: {e}"
                ))),
            }
        })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            info!("Executing native workload: {}", request.execution_id);

            let processes = self.active_processes.read().await;
            if processes.len()
                >= self.capabilities.max_concurrent_executions.unwrap_or(100) as usize
            {
                return Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::Failed {
                        error: std::borrow::Cow::Borrowed("Maximum concurrent processes exceeded"),
                    },
                    output: ExecutionOutput::default(),
                    metrics: RuntimeMetrics::default(),
                    duration: Duration::from_secs(0),
                    runtime_used: RuntimeType::Native,
                    warnings: vec!["Maximum concurrent processes exceeded".to_string()],
                });
            }
            drop(processes);

            request.workload.validate()?;
            validation::validate_resource_requirements(&request)?;

            let WorkloadSpec::Native {
                executable: executable_source,
                ..
            } = &request.workload
            else {
                return Err(ToadStoolError::validation(
                    "Invalid workload type for native runtime",
                ));
            };

            let executable_path = validation::resolve_executable(&self.config, executable_source)?;

            self.execute_workload(&request, executable_path).await
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        self.capabilities.clone()
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Native)
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async {
            if self.active_processes.read().await.is_empty() {
                return Ok(RuntimeMetrics::default());
            }

            Ok(RuntimeMetrics::default())
        })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
            info!("Shutting down Native Runtime Engine");

            let to_kill: Vec<(Uuid, Option<Child>)> = {
                let mut processes = self.active_processes.write().await;
                processes
                    .drain()
                    .map(|(id, mut h)| (id, h.child.take()))
                    .collect()
            };

            for (execution_id, child_opt) in to_kill {
                if let Some(mut child) = child_opt
                    && let Err(e) = child.kill().await
                {
                    warn!("Failed to kill process {}: {}", execution_id, e);
                }
            }

            info!("Native runtime engine shut down successfully");
            Ok(())
        })
    }
}

impl Default for NativeRuntimeEngine {
    fn default() -> Self {
        Self::new()
    }
}
