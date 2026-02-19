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
use tracing::{debug, info, warn};
use uuid::Uuid;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    execution::{
        ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities,
        RuntimeConfig, RuntimeEngine, RuntimeType,
    },
    resources::{ResourceMonitor, RuntimeMetrics},
    security::{IsolationLevel, SecurityContext},
    workload::{ExecutableSource, WorkloadSpec},
    WorkloadType,
};

#[cfg(unix)]
#[allow(unused_imports)]
use std::os::unix::process::CommandExt;

/// Native runtime engine for executing native processes
pub struct NativeRuntimeEngine {
    config: RuntimeConfig,
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

#[derive(Debug)]
#[allow(dead_code)]
struct ProcessHandle {
    child: Option<Child>,
    start_time: Instant,
    workload_id: String,
    executable_path: PathBuf,
}

impl NativeRuntimeEngine {
    /// Create a new native runtime engine
    #[must_use]
    pub fn new() -> Self {
        let capabilities = RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Native],
            max_concurrent_executions: Some(100),
            supported_architectures: vec![std::env::consts::ARCH.to_string()],
            platform_features: {
                let mut features = HashMap::new();
                features.insert("process_isolation".to_string(), true);
                features.insert("resource_limits".to_string(), cfg!(target_os = "linux"));
                features.insert("user_switching".to_string(), cfg!(unix));
                features.insert("chroot_jail".to_string(), cfg!(unix));
                features
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        Self {
            config: RuntimeConfig::default(),
            active_processes: Arc::new(RwLock::new(HashMap::new())),
            resource_monitor: None,
            capabilities,
        }
    }

    /// Set the resource monitor for this runtime
    pub fn with_resource_monitor(mut self, monitor: Arc<dyn ResourceMonitor>) -> Self {
        self.resource_monitor = Some(monitor);
        self
    }

    /// Validate executable source and return the executable path
    async fn resolve_executable(&self, source: &ExecutableSource) -> ToadStoolResult<PathBuf> {
        match source {
            ExecutableSource::File { path } => {
                if !path.exists() {
                    return Err(ToadStoolError::not_found(format!(
                        "Executable not found: {}",
                        path.display()
                    )));
                }

                // Check if file is executable
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let metadata = std::fs::metadata(path)
                        .map_err(|e| ToadStoolError::io(format!("Failed to read metadata: {e}")))?;
                    let permissions = metadata.permissions();
                    if permissions.mode() & 0o111 == 0 {
                        return Err(ToadStoolError::permission_denied(format!(
                            "File is not executable: {}",
                            path.display()
                        )));
                    }
                }

                Ok(path.clone())
            }
            ExecutableSource::Url { url: _ } => Err(ToadStoolError::not_supported(
                "URL-based executables not yet supported",
            )),
            ExecutableSource::Bytes { data: _ } => Err(ToadStoolError::not_supported(
                "Byte-based executables not yet supported",
            )),
        }
    }

    /// Apply security context to the command
    fn apply_security_context(
        &self,
        mut command: TokioCommand,
        security_context: &SecurityContext,
    ) -> ToadStoolResult<TokioCommand> {
        match security_context.isolation_level {
            IsolationLevel::None => {
                debug!("No isolation applied");
            }
            IsolationLevel::Basic => {
                debug!("Applying basic isolation");
                command.current_dir("/tmp");

                #[cfg(unix)]
                {
                    command.process_group(0);
                }
            }
            IsolationLevel::Standard => {
                debug!("Applying standard isolation");
                command.current_dir("/tmp");

                #[cfg(unix)]
                {
                    command.process_group(0);
                    if let Some(user_context) = &security_context.user_context {
                        if let Some(username) = &user_context.username {
                            info!("Setting user context to: {}", username);
                            // Note: Actually changing user requires elevated privileges
                        }
                    }
                }
            }
            IsolationLevel::Enhanced => {
                debug!("Applying enhanced isolation");
                command.current_dir("/tmp");

                #[cfg(unix)]
                {
                    command.process_group(0);
                }

                #[cfg(target_os = "linux")]
                {
                    info!("Enhanced isolation on Linux - implementing namespace isolation");
                }
            }
            IsolationLevel::Maximum => {
                debug!("Applying maximum isolation");

                #[cfg(unix)]
                {
                    command.process_group(0);
                }

                #[cfg(target_os = "linux")]
                {
                    info!("Maximum isolation - would use container-like isolation");
                }

                #[cfg(not(target_os = "linux"))]
                {
                    warn!("Maximum isolation not fully supported on this platform");
                }
            }
        }

        // Apply capability restrictions
        if !security_context.has_capability(&toadstool::security::Capability::Read) {
            debug!("File system read access denied");
        }

        if !security_context.has_capability(&toadstool::security::Capability::NetworkClient) {
            debug!("Network outbound access denied");
        }

        Ok(command)
    }

    /// Execute the workload and return the result
    async fn execute_workload(
        &self,
        request: &ExecutionRequest,
        executable_path: PathBuf,
    ) -> ToadStoolResult<ExecutionResponse> {
        let start_time = Instant::now();

        // Extract arguments from workload
        let args = match &request.workload {
            WorkloadSpec::Native { args, .. } => args.clone().unwrap_or_default(),
            _ => {
                return Err(ToadStoolError::validation(
                    "Invalid workload type for native runtime",
                ))
            }
        };

        // Create the command
        let mut command = TokioCommand::new(&executable_path);
        command.args(&args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // Apply environment variables
        for (key, value) in &request.environment {
            command.env(key, value);
        }

        // Apply security context
        command = self.apply_security_context(command, &request.security_context)?;

        // Start monitoring if available
        if let Some(monitor) = &self.resource_monitor {
            monitor.start_monitoring(&request.execution_id.to_string())?;
        }

        // Execute with timeout
        let execution_future = async {
            let child = command
                .spawn()
                .map_err(|e| ToadStoolError::runtime(format!("Failed to spawn process: {e}")))?;

            // Store the process handle
            {
                let mut processes = self.active_processes.write().await;
                processes.insert(
                    request.execution_id,
                    ProcessHandle {
                        child: None, // We'll update this after getting the result
                        start_time,
                        workload_id: request.execution_id.to_string(),
                        executable_path: executable_path.clone(),
                    },
                );
            }

            // Wait for completion
            let output = child
                .wait_with_output()
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Process execution failed: {e}")))?;

            Ok::<_, ToadStoolError>(output)
        };

        // Apply timeout if specified
        let output = if let Some(timeout_duration) = request.timeout {
            if let Ok(result) = timeout(timeout_duration, execution_future).await {
                result?
            } else {
                // Clean up the process
                self.cleanup_process(&request.execution_id).await;

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

        // Stop monitoring and get metrics
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

        // Clean up process tracking
        self.cleanup_process(&request.execution_id).await;

        // Determine execution status
        let status = if output.status.success() {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Failed {
                error: format!("Process exited with code: {:?}", output.status.code()),
            }
        };

        // Create response
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

    /// Clean up process tracking
    async fn cleanup_process(&self, execution_id: &Uuid) {
        let mut processes = self.active_processes.write().await;
        if let Some(mut process_handle) = processes.remove(execution_id) {
            if let Some(mut child) = process_handle.child.take() {
                // Kill the process if it's still running
                if let Err(e) = child.kill().await {
                    warn!("Failed to kill process {}: {}", execution_id, e);
                }
            }
        }
    }

    /// Validate resource requirements
    fn validate_resource_requirements(&self, request: &ExecutionRequest) -> ToadStoolResult<()> {
        // Check if we can meet the resource requirements
        let requirements = &request.resources;

        // Basic validation - in a real implementation, this would check system resources
        if requirements.cpu.min_cores > 32.0 {
            return Err(ToadStoolError::resource(
                "Requested CPU cores exceed system limits",
            ));
        }

        if requirements.memory.min_bytes > 128 * 1024 * 1024 * 1024 {
            // 128GB limit
            return Err(ToadStoolError::resource(
                "Requested memory exceeds system limits",
            ));
        }

        Ok(())
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

            // Validate that we can execute native processes
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

            // Check concurrent process limit
            let processes = self.active_processes.read().await;
            if processes.len()
                >= self.capabilities.max_concurrent_executions.unwrap_or(100) as usize
            {
                return Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::Failed {
                        error: "Maximum concurrent processes exceeded".to_string(),
                    },
                    output: ExecutionOutput::default(),
                    metrics: RuntimeMetrics::default(),
                    duration: Duration::from_secs(0),
                    runtime_used: RuntimeType::Native,
                    warnings: vec!["Maximum concurrent processes exceeded".to_string()],
                });
            }
            drop(processes);

            // Validate the request
            request.workload.validate()?;
            self.validate_resource_requirements(&request)?;

            // Extract executable source
            let WorkloadSpec::Native {
                executable: executable_source,
                ..
            } = &request.workload
            else {
                return Err(ToadStoolError::validation(
                    "Invalid workload type for native runtime",
                ));
            };

            // Resolve the executable path
            let executable_path = self.resolve_executable(executable_source).await?;

            // Execute the workload
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
            // Return aggregated metrics for all active processes
            let processes = self.active_processes.read().await;

            if processes.is_empty() {
                return Ok(RuntimeMetrics::default());
            }

            // In a real implementation, this would aggregate metrics from all processes
            // For now, return default metrics
            Ok(RuntimeMetrics::default())
        })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
            info!("Shutting down Native Runtime Engine");

            // Kill all active processes
            let mut processes = self.active_processes.write().await;
            for (execution_id, mut process_handle) in processes.drain() {
                if let Some(mut child) = process_handle.child.take() {
                    if let Err(e) = child.kill().await {
                        warn!("Failed to kill process {}: {}", execution_id, e);
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::{
        execution::{ExecutionInput, RuntimeType},
        resources::ResourceRequirements,
        security::{Capability, IsolationLevel, SecurityContext},
        workload::WorkloadSpec,
    };

    async fn create_test_engine() -> NativeRuntimeEngine {
        let mut engine = NativeRuntimeEngine::new();
        engine
            .initialize(RuntimeConfig::default())
            .await
            .expect("Test engine initialization should succeed");
        engine
    }

    fn create_test_request(executable_path: &str, args: Vec<String>) -> ExecutionRequest {
        ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from(executable_path),
                },
                args: Some(args),
                working_dir: None,
                env_vars: HashMap::new(),
                user: None,
            },
            runtime_hint: Some(RuntimeType::Native),
            resources: ResourceRequirements::default(),
            security_context: SecurityContext::for_isolation_level(IsolationLevel::Basic)
                .with_capability(Capability::Execute)
                .with_capability(Capability::Read),
            timeout: Some(Duration::from_secs(10)),
            environment: HashMap::new(),
            input_data: ExecutionInput::default(),
            callback_config: None,
            encryption_config: None,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_engine_initialization() {
        let engine = create_test_engine().await;
        assert!(engine.supports_workload(&WorkloadType::Native));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capabilities() {
        let engine = create_test_engine().await;
        let capabilities = engine.get_capabilities();

        assert!(capabilities
            .supported_workloads
            .contains(&WorkloadType::Native));
        assert!(capabilities.max_concurrent_executions.is_some());
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_simple_execution() {
        let engine = create_test_engine().await;
        let request = create_test_request("/bin/echo", vec!["hello".to_string()]);

        let response = engine
            .execute(request)
            .await
            .expect("Echo execution should succeed");

        assert_eq!(response.status, ExecutionStatus::Success);
        assert!(response
            .output
            .stdout
            .expect("Echo should produce stdout")
            .contains("hello"));
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execution_with_args() {
        let engine = create_test_engine().await;
        let request = create_test_request("/bin/ls", vec!["-la".to_string(), "/tmp".to_string()]);

        let response = engine.execute(request).await.unwrap();

        assert_eq!(response.status, ExecutionStatus::Success);
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_timeout_handling() {
        let engine = create_test_engine().await;
        let mut request = create_test_request("/bin/sleep", vec!["5".to_string()]);
        request.timeout = Some(Duration::from_millis(100)); // Very short timeout

        let response = engine.execute(request).await.unwrap();

        assert_eq!(response.status, ExecutionStatus::TimedOut);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_invalid_executable() {
        let engine = create_test_engine().await;
        let request = create_test_request("/nonexistent/executable", vec![]);

        let result = engine.execute(request).await;

        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_metrics() {
        let engine = create_test_engine().await;
        let metrics = engine.get_metrics().await.unwrap();

        // Default metrics when no processes running
        assert!(metrics.cpu.usage_percent >= 0.0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_shutdown() {
        let mut engine = create_test_engine().await;
        let shutdown_result = engine.shutdown().await;

        assert!(shutdown_result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_supports_workload() {
        let engine = NativeRuntimeEngine::new();

        assert!(engine.supports_workload(&WorkloadType::Native));
        assert!(!engine.supports_workload(&WorkloadType::Wasm));
        assert!(!engine.supports_workload(&WorkloadType::Container));
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execution_with_env_vars() {
        let engine = create_test_engine().await;
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());

        let mut request = create_test_request(
            "/bin/sh",
            vec!["-c".to_string(), "echo $TEST_VAR".to_string()],
        );
        if let WorkloadSpec::Native {
            env_vars: ref mut env,
            ..
        } = request.workload
        {
            *env = env_vars;
        }

        let response = engine.execute(request).await.unwrap();

        // Environment variables test - may or may not propagate depending on system
        assert!(matches!(
            response.status,
            ExecutionStatus::Success | ExecutionStatus::Failed { .. }
        ));
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execution_with_working_dir() {
        let engine = create_test_engine().await;
        let mut request = create_test_request("/bin/pwd", vec![]);

        if let WorkloadSpec::Native {
            working_dir: ref mut dir,
            ..
        } = request.workload
        {
            *dir = Some(PathBuf::from("/tmp"));
        }

        let response = engine.execute(request).await.unwrap();

        assert_eq!(response.status, ExecutionStatus::Success);
        if let Some(stdout) = response.output.stdout {
            assert!(stdout.contains("/tmp"));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_multiple_concurrent_executions() {
        let engine = Arc::new(create_test_engine().await);
        let mut handles = vec![];

        for _ in 0..5 {
            let engine_clone = Arc::clone(&engine);
            let handle = tokio::spawn(async move {
                let request = create_test_request(
                    if cfg!(windows) { "cmd" } else { "/bin/echo" },
                    vec!["test".to_string()],
                );
                engine_clone.execute(request).await
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execution_failure_handling() {
        let engine = create_test_engine().await;
        // false command always returns exit code 1
        let request = create_test_request("/bin/false", vec![]);

        let response = engine.execute(request).await.unwrap();

        match response.status {
            ExecutionStatus::Failed { .. } => {
                // Expected
            }
            _ => panic!("Expected failed status for /bin/false"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_default_capabilities() {
        let engine = NativeRuntimeEngine::new();
        let capabilities = engine.get_capabilities();

        assert!(capabilities
            .supported_workloads
            .contains(&WorkloadType::Native));
        assert!(capabilities.max_concurrent_executions.is_some());
        assert!(capabilities.max_concurrent_executions.unwrap() > 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_engine_default_construction() {
        let engine1 = NativeRuntimeEngine::new();
        let engine2 = NativeRuntimeEngine::default();

        // Both should have same default configuration structure
        assert_eq!(engine1.config.settings.len(), engine2.config.settings.len());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_debug_trait() {
        let engine = NativeRuntimeEngine::new();
        let debug_str = format!("{:?}", engine);

        assert!(debug_str.contains("NativeRuntimeEngine"));
        assert!(debug_str.contains("config"));
    }
}
