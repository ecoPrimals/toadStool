//! Universal Scheduler for cross-platform job execution
//!
//! The scheduler routes jobs to appropriate execution backends:
//! 1. **Primal Registry**: Discovers remote primals with capabilities
//! 2. **Runtime Engines**: Local engines for direct execution (WASM, Native)
//!
//! ## Execution Flow
//!
//! ```text
//! Job → Try Primal Registry → Found? → Execute via Primal
//!                            ↓ Not Found
//!                     Try Runtime Engine → Found? → Execute Locally
//!                            ↓ Not Found
//!                     Return Fallback/Error
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use toadstool_common::constants::PRIMAL_NAME;
use toadstool_config::defaults;

use crate::execution::{
    ExecutionInput, ExecutionRequest, ExecutionResponse, RuntimeEngine, RuntimeType,
};
use crate::resources::ResourceRequirements;
use crate::workload::{ExecutableSource, WasiConfig, WasmModuleSource};
use crate::{SecurityContext, ToadStoolResult, WorkloadSpec};

use super::jobs::{UniversalJob, UniversalJobType};
use super::registry::UniversalPrimalRegistry;
use super::requests::{PrimalRequest, ResponseStatus};
use super::resources::ResourceCoordinator;
use super::types::{NetworkLocation, PrimalCapability, PrimalContext, SecurityLevel};

/// Universal scheduler for any substrate
pub struct UniversalScheduler {
    /// Primal registry
    primal_registry: Arc<UniversalPrimalRegistry>,
    /// Resource coordinator
    resource_coordinator: Arc<ResourceCoordinator>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, UniversalJob>>>,
    /// Runtime engines for local execution (optional)
    runtime_engines: Arc<RwLock<HashMap<RuntimeType, Box<dyn RuntimeEngine>>>>,
}

impl UniversalScheduler {
    /// Create new scheduler
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if resource coordinator initialization fails.
    #[must_use = "Scheduler creation should be checked"]
    pub async fn new(primal_registry: Arc<UniversalPrimalRegistry>) -> ToadStoolResult<Self> {
        Ok(Self {
            primal_registry,
            resource_coordinator: Arc::new(ResourceCoordinator::new().await?),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create scheduler with runtime engines for local execution
    ///
    /// # Arguments
    /// * `primal_registry` - Registry for discovering remote primals
    /// * `runtime_engines` - Map of runtime type to engine for local execution
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if resource coordinator initialization fails.
    pub async fn with_runtime_engines(
        primal_registry: Arc<UniversalPrimalRegistry>,
        runtime_engines: HashMap<RuntimeType, Box<dyn RuntimeEngine>>,
    ) -> ToadStoolResult<Self> {
        info!(
            "Creating scheduler with {} runtime engines: {:?}",
            runtime_engines.len(),
            runtime_engines.keys().collect::<Vec<_>>()
        );
        Ok(Self {
            primal_registry,
            resource_coordinator: Arc::new(ResourceCoordinator::new().await?),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            runtime_engines: Arc::new(RwLock::new(runtime_engines)),
        })
    }

    /// Register a runtime engine for local execution
    ///
    /// Allows adding runtime engines after scheduler creation.
    pub async fn register_runtime_engine(
        &self,
        runtime_type: RuntimeType,
        engine: Box<dyn RuntimeEngine>,
    ) {
        info!("Registering runtime engine: {:?}", runtime_type);
        self.runtime_engines
            .write()
            .await
            .insert(runtime_type, engine);
    }

    /// Get available runtime types
    pub async fn available_runtimes(&self) -> Vec<RuntimeType> {
        self.runtime_engines.read().await.keys().cloned().collect()
    }

    /// Schedule a job
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if:
    /// - Resource allocation fails.
    /// - Job execution fails.
    /// - No suitable primal can be found for the job.
    #[must_use = "Job scheduling result should be checked"]
    pub async fn schedule_job(&self, job: UniversalJob) -> ToadStoolResult<ExecutionResponse> {
        let job_id = job.id;
        info!("Scheduling job: {}", job_id);

        // Add to active jobs
        self.active_jobs.write().await.insert(job_id, job.clone());

        // Allocate resources
        let _allocation = self
            .resource_coordinator
            .allocate_resources(&job.resources)
            .await?;

        // Execute based on job type
        let result = match &job.job_type {
            UniversalJobType::Native {
                executable,
                args,
                env,
            } => self.execute_native(executable, args, env).await,
            UniversalJobType::Wasm { module, args, env } => {
                self.execute_wasm(module, args, env).await
            }
            UniversalJobType::Primal {
                primal_type,
                endpoint,
                payload,
            } => self.execute_primal(primal_type, endpoint, payload).await,
            UniversalJobType::BiomeOS {
                biome_manifest,
                team_id,
            } => self.execute_biome_os(biome_manifest, team_id).await,
        };

        // Remove from active jobs
        self.active_jobs.write().await.remove(&job_id);

        result
    }

    /// Get active job count
    pub async fn get_active_job_count(&self) -> usize {
        self.active_jobs.read().await.len()
    }

    /// Find primals by capability using the registry
    pub async fn find_primals_by_capability(
        &self,
        capability: &PrimalCapability,
    ) -> Vec<Arc<dyn super::traits::UniversalPrimalProvider>> {
        self.primal_registry.find_by_capability(capability).await
    }

    async fn execute_native(
        &self,
        executable: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing native job: {} with args: {:?}", executable, args);

        // Try to find a native runtime engine through the primal registry
        let native_capability = PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
        };

        let providers = self
            .primal_registry
            .find_by_capability(&native_capability)
            .await;

        if let Some(provider) = providers.first() {
            // Create a primal request for native execution
            let request = PrimalRequest {
                id: Uuid::new_v4(),
                source: PRIMAL_NAME.to_string(),
                target: provider.primal_id().to_string(),
                request_type: "execute_native".to_string(),
                payload: serde_json::json!({
                    "executable": executable,
                    "args": args,
                    "env": env
                }),
                context: PrimalContext {
                    user_id: "system".to_string(),
                    device_id: "local".to_string(),
                    session_id: Uuid::new_v4().to_string(),
                    network_location: NetworkLocation {
                        ip_address: defaults::network::LOCALHOST.to_string(),
                        subnet: None,
                        network_id: None,
                        geo_location: None,
                    },
                    security_level: SecurityLevel::Standard,
                    metadata: HashMap::new(),
                },
                metadata: HashMap::new(),
                timestamp: chrono::Utc::now(),
            };

            let response = provider.handle_primal_request(request).await?;

            // Convert primal response to execution response
            Ok(ExecutionResponse {
                execution_id: response.request_id,
                status: match response.status {
                    ResponseStatus::Success => crate::execution::ExecutionStatus::Success,
                    ResponseStatus::Error { message, .. } => {
                        crate::execution::ExecutionStatus::Failed { error: message }
                    }
                    ResponseStatus::Timeout => crate::execution::ExecutionStatus::TimedOut,
                    ResponseStatus::ServiceUnavailable => {
                        crate::execution::ExecutionStatus::Failed {
                            error: "Service unavailable".to_string(),
                        }
                    }
                },
                output: crate::execution::ExecutionOutput {
                    data: response.payload.to_string().into_bytes().into(),
                    stdout: response
                        .payload
                        .get("stdout")
                        .and_then(|v| v.as_str())
                        .map(std::string::ToString::to_string),
                    stderr: response
                        .payload
                        .get("stderr")
                        .and_then(|v| v.as_str())
                        .map(std::string::ToString::to_string),
                    exit_code: response
                        .payload
                        .get("exit_code")
                        .and_then(serde_json::Value::as_i64)
                        .map(|i| i as i32),
                    format: Some("application/json".to_string()),
                    result: HashMap::new(),
                    metadata: response.metadata,
                },
                metrics: crate::RuntimeMetrics::default(),
                duration: Duration::from_millis(100),
                runtime_used: crate::execution::RuntimeType::Native,
                warnings: Vec::new(),
            })
        } else {
            // No primal found - try local native runtime engine
            let engines = self.runtime_engines.read().await;
            if let Some(native_engine) = engines.get(&RuntimeType::Native) {
                info!("Using local native runtime engine for execution");

                // Build execution request
                let request = ExecutionRequest {
                    execution_id: Uuid::new_v4(),
                    workload: WorkloadSpec::Native {
                        executable: ExecutableSource::File {
                            path: std::path::PathBuf::from(executable),
                        },
                        args: Some(args.to_vec()),
                        working_dir: None,
                        env_vars: env.clone(),
                        user: None,
                    },
                    runtime_hint: Some(RuntimeType::Native),
                    resources: ResourceRequirements::default(),
                    security_context: SecurityContext::default(),
                    timeout: Some(Duration::from_secs(300)),
                    environment: env.clone(),
                    input_data: ExecutionInput::default(),
                    callback_config: None,
                    encryption_config: None,
                };

                return native_engine.execute(request).await;
            }

            // No primal provider and no registered runtime engine.
            // Sovereign fallback: attempt direct local process execution.
            // This preserves autonomy — the scheduler can run binaries itself
            // without requiring any registered capability provider.
            let start = std::time::Instant::now();
            let execution_id = Uuid::new_v4();
            match tokio::process::Command::new(executable)
                .args(args)
                .envs(env)
                .output()
                .await
            {
                Ok(output) => {
                    let duration = start.elapsed();
                    let stdout_text = String::from_utf8_lossy(&output.stdout).into_owned();
                    let stderr_text = String::from_utf8_lossy(&output.stderr).into_owned();
                    let exit_code = output.status.code().unwrap_or(-1);
                    let status = if output.status.success() {
                        crate::execution::ExecutionStatus::Success
                    } else {
                        crate::execution::ExecutionStatus::Failed {
                            error: format!("process exited with code {exit_code}"),
                        }
                    };
                    Ok(ExecutionResponse {
                        execution_id,
                        status,
                        output: crate::execution::ExecutionOutput {
                            data: output.stdout.into(),
                            stdout: Some(stdout_text),
                            stderr: if stderr_text.is_empty() {
                                None
                            } else {
                                Some(stderr_text)
                            },
                            exit_code: Some(exit_code),
                            format: Some("text/plain".to_string()),
                            result: HashMap::new(),
                            metadata: HashMap::new(),
                        },
                        metrics: crate::RuntimeMetrics::default(),
                        duration,
                        runtime_used: crate::execution::RuntimeType::Native,
                        warnings: Vec::new(),
                    })
                }
                Err(e) => {
                    let duration = start.elapsed();
                    let error_msg = format!("Failed to spawn '{}': {}", executable, e);
                    warn!("{}", error_msg);
                    Ok(ExecutionResponse {
                        execution_id,
                        status: crate::execution::ExecutionStatus::Failed {
                            error: error_msg.clone(),
                        },
                        output: crate::execution::ExecutionOutput {
                            data: bytes::Bytes::new(),
                            stdout: None,
                            stderr: Some(error_msg),
                            exit_code: Some(127),
                            format: Some("text/plain".to_string()),
                            result: HashMap::new(),
                            metadata: HashMap::new(),
                        },
                        metrics: crate::RuntimeMetrics::default(),
                        duration,
                        runtime_used: crate::execution::RuntimeType::Native,
                        warnings: Vec::new(),
                    })
                }
            }
        }
    }

    async fn execute_wasm(
        &self,
        module: &[u8],
        args: &[String],
        env: &HashMap<String, String>,
    ) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing WASM job ({} bytes)", module.len());

        // Check if we have a WASM runtime engine registered
        let engines = self.runtime_engines.read().await;
        if let Some(wasm_engine) = engines.get(&RuntimeType::Wasm) {
            info!("Using registered WASM runtime engine for execution");

            // Build execution request
            let request = ExecutionRequest {
                execution_id: Uuid::new_v4(),
                workload: WorkloadSpec::Wasm {
                    module: WasmModuleSource::Bytes {
                        data: bytes::Bytes::copy_from_slice(module),
                    },
                    args: Some(args.to_vec()),
                    wasi_config: Some(WasiConfig {
                        inherit_env: true,
                        inherit_stdio: true,
                        allowed_dirs: Vec::new(),
                        preopened_dirs: Vec::new(),
                        args: args.to_vec(),
                    }),
                    env_vars: env.clone(),
                },
                runtime_hint: Some(RuntimeType::Wasm),
                resources: ResourceRequirements::default(),
                security_context: SecurityContext::default(),
                timeout: Some(Duration::from_secs(300)),
                environment: env.clone(),
                input_data: ExecutionInput::default(),
                callback_config: None,
                encryption_config: None,
            };

            // Execute via the WASM runtime engine
            return wasm_engine.execute(request).await;
        }

        // No WASM engine registered - return proper error
        let error_msg = format!(
            "No WASM execution capability: no runtime engine registered for WASM modules ({} bytes)",
            module.len()
        );
        warn!("{}", error_msg);
        Ok(ExecutionResponse {
            execution_id: Uuid::new_v4(),
            status: crate::execution::ExecutionStatus::Failed {
                error: error_msg.clone(),
            },
            output: crate::execution::ExecutionOutput {
                data: bytes::Bytes::new(),
                stdout: None,
                stderr: Some(error_msg),
                exit_code: Some(126), // Command not executable
                format: Some("text/plain".to_string()),
                result: HashMap::new(),
                metadata: HashMap::new(),
            },
            metrics: crate::RuntimeMetrics::default(),
            duration: Duration::from_millis(0),
            runtime_used: crate::execution::RuntimeType::Wasm,
            warnings: vec!["Register a WASM runtime engine via register_runtime_engine(RuntimeType::Wasm, engine)".to_string()],
        })
    }

    async fn execute_primal(
        &self,
        primal_type: &str,
        endpoint: &str,
        payload: &serde_json::Value,
    ) -> ToadStoolResult<ExecutionResponse> {
        debug!(
            "Executing primal job: {} at endpoint: {}",
            primal_type, endpoint
        );
        let start_time = std::time::Instant::now();
        let execution_id = Uuid::new_v4();

        // Find a primal provider that matches the requested type
        let providers = self.primal_registry.get_all_providers().await;
        let matching_provider = providers
            .iter()
            .find(|p| p.primal_type().as_str() == primal_type);

        if let Some(provider) = matching_provider {
            // Build and route the request through the primal registry
            let request = PrimalRequest {
                id: execution_id,
                source: "scheduler".to_string(),
                target: provider.instance_id().to_string(),
                request_type: endpoint.to_string(),
                payload: payload.clone(),
                context: PrimalContext {
                    user_id: "scheduler".to_string(),
                    device_id: "local".to_string(),
                    session_id: execution_id.to_string(),
                    network_location: NetworkLocation {
                        ip_address: "127.0.0.1".to_string(),
                        subnet: None,
                        network_id: None,
                        geo_location: None,
                    },
                    security_level: SecurityLevel::Standard,
                    metadata: HashMap::new(),
                },
                metadata: HashMap::new(),
                timestamp: chrono::Utc::now(),
            };

            match self.primal_registry.route_request(request).await {
                Ok(response) => {
                    let duration = start_time.elapsed();
                    let status = if response.status == ResponseStatus::Success {
                        crate::execution::ExecutionStatus::Success
                    } else {
                        let error_msg = match &response.status {
                            ResponseStatus::Error { message, .. } => message.clone(),
                            ResponseStatus::Timeout => "Request timed out".to_string(),
                            ResponseStatus::ServiceUnavailable => "Service unavailable".to_string(),
                            ResponseStatus::Success => "Unknown error".to_string(),
                        };
                        crate::execution::ExecutionStatus::Failed { error: error_msg }
                    };

                    Ok(ExecutionResponse {
                        execution_id,
                        status,
                        output: crate::execution::ExecutionOutput {
                            data: response.payload.to_string().into_bytes().into(),
                            stdout: Some(format!("Primal '{}' executed successfully", primal_type)),
                            stderr: None,
                            exit_code: Some(0),
                            format: Some("application/json".to_string()),
                            result: HashMap::new(),
                            metadata: HashMap::new(),
                        },
                        metrics: crate::RuntimeMetrics::default(),
                        duration,
                        runtime_used: crate::execution::RuntimeType::Native,
                        warnings: Vec::new(),
                    })
                }
                Err(e) => {
                    let duration = start_time.elapsed();
                    let error_msg = format!("Primal '{}' execution failed: {}", primal_type, e);
                    warn!("{}", error_msg);
                    Ok(ExecutionResponse {
                        execution_id,
                        status: crate::execution::ExecutionStatus::Failed {
                            error: error_msg.clone(),
                        },
                        output: crate::execution::ExecutionOutput {
                            data: bytes::Bytes::new(),
                            stdout: None,
                            stderr: Some(error_msg),
                            exit_code: Some(1),
                            format: Some("text/plain".to_string()),
                            result: HashMap::new(),
                            metadata: HashMap::new(),
                        },
                        metrics: crate::RuntimeMetrics::default(),
                        duration,
                        runtime_used: crate::execution::RuntimeType::Native,
                        warnings: Vec::new(),
                    })
                }
            }
        } else {
            // No matching primal provider found
            let error_msg = format!(
                "No primal provider registered for type '{}'. Available providers: {}",
                primal_type,
                if providers.is_empty() {
                    "none".to_string()
                } else {
                    providers
                        .iter()
                        .map(|p| p.primal_type().as_str().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            );
            warn!("{}", error_msg);
            Ok(ExecutionResponse {
                execution_id,
                status: crate::execution::ExecutionStatus::Failed {
                    error: error_msg.clone(),
                },
                output: crate::execution::ExecutionOutput {
                    data: bytes::Bytes::new(),
                    stdout: None,
                    stderr: Some(error_msg),
                    exit_code: Some(127),
                    format: Some("text/plain".to_string()),
                    result: HashMap::new(),
                    metadata: HashMap::new(),
                },
                metrics: crate::RuntimeMetrics::default(),
                duration: start_time.elapsed(),
                runtime_used: crate::execution::RuntimeType::Native,
                warnings: vec![
                    "Register a primal provider via primal_registry.register_primal()".to_string(),
                ],
            })
        }
    }

    async fn execute_biome_os(
        &self,
        biome_manifest: &serde_json::Value,
        team_id: &str,
    ) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing BiomeOS job for team: {}", team_id);
        let execution_id = Uuid::new_v4();
        let start_time = std::time::Instant::now();

        // BiomeOS integration: Look for a BiomeOS primal provider
        let providers = self.primal_registry.get_all_providers().await;
        let biome_provider = providers.iter().find(|p| p.primal_type().as_str() == "os");

        if let Some(provider) = biome_provider {
            // Route to BiomeOS primal
            let request = PrimalRequest {
                id: execution_id,
                source: "scheduler".to_string(),
                target: provider.instance_id().to_string(),
                request_type: "execute".to_string(),
                payload: serde_json::json!({
                    "manifest": biome_manifest,
                    "team_id": team_id,
                }),
                context: PrimalContext {
                    user_id: team_id.to_string(),
                    device_id: "local".to_string(),
                    session_id: execution_id.to_string(),
                    network_location: NetworkLocation {
                        ip_address: "127.0.0.1".to_string(),
                        subnet: None,
                        network_id: None,
                        geo_location: None,
                    },
                    security_level: SecurityLevel::Standard,
                    metadata: HashMap::new(),
                },
                metadata: HashMap::new(),
                timestamp: chrono::Utc::now(),
            };

            match self.primal_registry.route_request(request).await {
                Ok(response) => {
                    let duration = start_time.elapsed();
                    let status = if response.status == ResponseStatus::Success {
                        crate::execution::ExecutionStatus::Success
                    } else {
                        let error_msg = match &response.status {
                            ResponseStatus::Error { message, .. } => message.clone(),
                            ResponseStatus::Timeout => "BiomeOS request timed out".to_string(),
                            ResponseStatus::ServiceUnavailable => {
                                "BiomeOS service unavailable".to_string()
                            }
                            ResponseStatus::Success => "Unknown error".to_string(),
                        };
                        crate::execution::ExecutionStatus::Failed { error: error_msg }
                    };

                    Ok(ExecutionResponse {
                        execution_id,
                        status,
                        output: crate::execution::ExecutionOutput {
                            data: response.payload.to_string().into_bytes().into(),
                            stdout: Some(format!(
                                "BiomeOS execution for team '{}' completed",
                                team_id
                            )),
                            stderr: None,
                            exit_code: Some(0),
                            format: Some("application/json".to_string()),
                            result: HashMap::new(),
                            metadata: HashMap::new(),
                        },
                        metrics: crate::RuntimeMetrics::default(),
                        duration,
                        runtime_used: crate::execution::RuntimeType::Native,
                        warnings: Vec::new(),
                    })
                }
                Err(e) => {
                    let duration = start_time.elapsed();
                    let error_msg =
                        format!("BiomeOS execution failed for team '{}': {}", team_id, e);
                    warn!("{}", error_msg);
                    Ok(ExecutionResponse {
                        execution_id,
                        status: crate::execution::ExecutionStatus::Failed {
                            error: error_msg.clone(),
                        },
                        output: crate::execution::ExecutionOutput {
                            data: bytes::Bytes::new(),
                            stdout: None,
                            stderr: Some(error_msg),
                            exit_code: Some(1),
                            format: Some("text/plain".to_string()),
                            result: HashMap::new(),
                            metadata: HashMap::new(),
                        },
                        metrics: crate::RuntimeMetrics::default(),
                        duration,
                        runtime_used: crate::execution::RuntimeType::Native,
                        warnings: Vec::new(),
                    })
                }
            }
        } else {
            // No BiomeOS primal provider registered
            let error_msg = format!(
                "BiomeOS integration not available: no BiomeOS primal provider registered for team '{}'",
                team_id
            );
            warn!("{}", error_msg);
            Ok(ExecutionResponse {
                execution_id,
                status: crate::execution::ExecutionStatus::Failed {
                    error: error_msg.clone(),
                },
                output: crate::execution::ExecutionOutput {
                    data: bytes::Bytes::new(),
                    stdout: None,
                    stderr: Some(error_msg),
                    exit_code: Some(127),
                    format: Some("text/plain".to_string()),
                    result: HashMap::new(),
                    metadata: HashMap::new(),
                },
                metrics: crate::RuntimeMetrics::default(),
                duration: start_time.elapsed(),
                runtime_used: crate::execution::RuntimeType::Native,
                warnings: vec![
                    "BiomeOS execution requires a registered BiomeOS primal provider".to_string(),
                ],
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobPriority;
    use std::collections::HashMap;
    use std::future::Future;
    use std::pin::Pin;

    use crate::execution::{
        ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeConfig,
        RuntimeEngine, RuntimeType,
    };
    use crate::resources::{ResourceRequirements, RuntimeMetrics};
    use crate::workload::WorkloadType;

    fn make_test_context() -> PrimalContext {
        PrimalContext {
            user_id: "test-user".to_string(),
            device_id: "test-device".to_string(),
            session_id: "test-session".to_string(),
            network_location: NetworkLocation {
                ip_address: "127.0.0.1".to_string(),
                subnet: None,
                network_id: None,
                geo_location: None,
            },
            security_level: SecurityLevel::Standard,
            metadata: HashMap::new(),
        }
    }

    fn make_universal_job(job_type: UniversalJobType) -> UniversalJob {
        UniversalJob {
            id: Uuid::new_v4(),
            job_type,
            priority: JobPriority::Normal,
            resources: ResourceRequirements::default(),
            timeout: Some(Duration::from_secs(60)),
            created_at: chrono::Utc::now(),
            context: make_test_context(),
        }
    }

    /// Minimal mock RuntimeEngine for scheduler tests
    struct SimpleMockRuntimeEngine;

    impl RuntimeEngine for SimpleMockRuntimeEngine {
        fn initialize(
            &mut self,
            _config: RuntimeConfig,
        ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
            Box::pin(async { Ok(()) })
        }

        fn execute(
            &self,
            request: ExecutionRequest,
        ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
            Box::pin(async move {
                let runtime_used = request.runtime_hint.unwrap_or(RuntimeType::Native);
                Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::Success,
                    output: ExecutionOutput::default(),
                    metrics: RuntimeMetrics::default(),
                    duration: Duration::from_millis(10),
                    runtime_used,
                    warnings: vec![],
                })
            })
        }

        fn get_capabilities(&self) -> crate::RuntimeCapabilities {
            crate::RuntimeCapabilities {
                supported_workloads: vec![WorkloadType::Native, WorkloadType::Wasm],
                max_concurrent_executions: Some(10),
                supported_architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
                platform_features: HashMap::new(),
                version: "1.0-test".to_string(),
            }
        }

        fn supports_workload(&self, _: &WorkloadType) -> bool {
            true
        }

        fn get_metrics(
            &self,
        ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
            Box::pin(async { Ok(RuntimeMetrics::default()) })
        }

        fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
            Box::pin(async { Ok(()) })
        }
    }

    #[tokio::test]
    async fn test_scheduler_new_basic_construction() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await;
        assert!(scheduler.is_ok());
        let scheduler = scheduler.unwrap();
        assert!(scheduler.available_runtimes().await.is_empty());
    }

    #[tokio::test]
    async fn test_scheduler_with_runtime_engines() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let mut engines = HashMap::new();
        engines.insert(
            RuntimeType::Native,
            Box::new(SimpleMockRuntimeEngine) as Box<dyn RuntimeEngine>,
        );
        let scheduler = UniversalScheduler::with_runtime_engines(registry, engines)
            .await
            .unwrap();
        let runtimes = scheduler.available_runtimes().await;
        assert_eq!(runtimes.len(), 1);
        assert_eq!(runtimes[0], RuntimeType::Native);
    }

    #[tokio::test]
    async fn test_scheduler_register_runtime_engine_and_available_runtimes() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        assert!(scheduler.available_runtimes().await.is_empty());

        scheduler
            .register_runtime_engine(RuntimeType::Native, Box::new(SimpleMockRuntimeEngine))
            .await;

        let runtimes = scheduler.available_runtimes().await;
        assert_eq!(runtimes.len(), 1);
        assert_eq!(runtimes[0], RuntimeType::Native);

        scheduler
            .register_runtime_engine(RuntimeType::Wasm, Box::new(SimpleMockRuntimeEngine))
            .await;

        let runtimes = scheduler.available_runtimes().await;
        assert_eq!(runtimes.len(), 2);
        assert!(runtimes.contains(&RuntimeType::Native));
        assert!(runtimes.contains(&RuntimeType::Wasm));
    }

    #[tokio::test]
    async fn test_scheduler_get_active_job_count_starts_at_zero() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();
        assert_eq!(scheduler.get_active_job_count().await, 0);
    }

    #[tokio::test]
    async fn test_scheduler_find_primals_by_capability_empty_registry() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();
        let capability = PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string()],
        };
        let providers = scheduler.find_primals_by_capability(&capability).await;
        assert!(providers.is_empty());
    }

    #[tokio::test]
    async fn test_schedule_job_native_echo_hello() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        let job = make_universal_job(UniversalJobType::Native {
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
            env: HashMap::new(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok(), "native echo job should succeed");
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Success);
        assert_eq!(
            response.output.stdout.as_deref(),
            Some("hello\n"),
            "stdout should contain 'hello'"
        );
    }

    #[tokio::test]
    async fn test_schedule_job_native_nonexistent_binary_returns_failed() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        let job = make_universal_job(UniversalJobType::Native {
            executable: "/nonexistent/binary/that/does/not/exist".to_string(),
            args: vec![],
            env: HashMap::new(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(
            result.is_ok(),
            "schedule_job returns Ok even when process fails"
        );
        let response = result.unwrap();
        assert!(
            matches!(response.status, ExecutionStatus::Failed { .. }),
            "status should be Failed"
        );
        assert_eq!(response.output.exit_code, Some(127));
    }

    #[tokio::test]
    async fn test_schedule_job_wasm_no_engine_returns_error() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        let job = make_universal_job(UniversalJobType::Wasm {
            module: vec![0x00, 0x61, 0x73, 0x6d],
            args: vec![],
            env: HashMap::new(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(
            result.is_ok(),
            "schedule_job returns Ok with Failed status in response"
        );
        let response = result.unwrap();
        assert!(matches!(
            response.status,
            ExecutionStatus::Failed { ref error } if error.contains("No WASM execution capability")
        ));
        assert_eq!(response.runtime_used, RuntimeType::Wasm);
    }

    #[tokio::test]
    async fn test_schedule_job_primal_no_provider_returns_error() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        let job = make_universal_job(UniversalJobType::Primal {
            primal_type: "nonexistent_primal".to_string(),
            endpoint: "http://localhost:8080/run".to_string(),
            payload: serde_json::json!({}),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(matches!(
            response.status,
            ExecutionStatus::Failed { ref error } if error.contains("No primal provider")
        ));
    }

    #[tokio::test]
    async fn test_schedule_job_biome_os_no_provider_returns_error() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        let job = make_universal_job(UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({"version": "1"}),
            team_id: "team-42".to_string(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(matches!(
            response.status,
            ExecutionStatus::Failed { ref error } if error.contains("BiomeOS integration not available")
        ));
    }

    #[tokio::test]
    async fn test_get_active_job_count_during_execution() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let mut engines = HashMap::new();
        engines.insert(
            RuntimeType::Native,
            Box::new(SimpleMockRuntimeEngine) as Box<dyn RuntimeEngine>,
        );
        let scheduler = UniversalScheduler::with_runtime_engines(registry, engines)
            .await
            .unwrap();

        let job = make_universal_job(UniversalJobType::Native {
            executable: "echo".to_string(),
            args: vec!["hi".to_string()],
            env: HashMap::new(),
        });

        let scheduler_clone = std::sync::Arc::new(scheduler);
        let job_clone = job.clone();

        let handle = tokio::spawn({
            let s = Arc::clone(&scheduler_clone);
            async move { s.schedule_job(job_clone).await }
        });

        // Give the job a moment to start (it gets added to active_jobs at the start)
        tokio::time::sleep(Duration::from_millis(50)).await;
        let count = scheduler_clone.get_active_job_count().await;
        // Job may have already completed; either 0 or 1 is valid
        assert!(count <= 1, "active job count should be 0 or 1");

        let _ = handle.await.unwrap();
        assert_eq!(scheduler_clone.get_active_job_count().await, 0);
    }

    #[tokio::test]
    async fn test_schedule_job_with_native_engine_prefers_engine_over_direct_exec() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let mut engines = HashMap::new();
        engines.insert(
            RuntimeType::Native,
            Box::new(SimpleMockRuntimeEngine) as Box<dyn RuntimeEngine>,
        );
        let scheduler = UniversalScheduler::with_runtime_engines(registry, engines)
            .await
            .unwrap();

        let job = make_universal_job(UniversalJobType::Native {
            executable: "echo".to_string(),
            args: vec!["via-engine".to_string()],
            env: HashMap::new(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Success);
        assert_eq!(response.runtime_used, RuntimeType::Native);
    }

    #[tokio::test]
    async fn test_schedule_job_wasm_with_engine_succeeds() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let mut engines = HashMap::new();
        engines.insert(
            RuntimeType::Wasm,
            Box::new(SimpleMockRuntimeEngine) as Box<dyn RuntimeEngine>,
        );
        let scheduler = UniversalScheduler::with_runtime_engines(registry, engines)
            .await
            .unwrap();

        let job = make_universal_job(UniversalJobType::Wasm {
            module: vec![0x00, 0x61, 0x73, 0x6d],
            args: vec![],
            env: HashMap::new(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Success);
        assert_eq!(response.runtime_used, RuntimeType::Wasm);
    }

    #[tokio::test]
    async fn test_find_primals_by_capability_with_registered_provider() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let provider = Arc::new(crate::universal::ToadStoolPrimalProvider::new(
            make_test_context(),
        ));
        registry.register_primal(provider).await.unwrap();

        let scheduler = UniversalScheduler::new(registry).await.unwrap();
        let capability = PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
        };
        let providers = scheduler.find_primals_by_capability(&capability).await;
        assert!(!providers.is_empty());
    }

    #[tokio::test]
    async fn test_schedule_job_native_with_env_vars() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        let mut env = HashMap::new();
        env.insert(
            "SCHEDULER_TEST_VAR".to_string(),
            "hello_from_test".to_string(),
        );

        let job = make_universal_job(UniversalJobType::Native {
            executable: "env".to_string(),
            args: vec![],
            env,
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Success);
        let stdout = response.output.stdout.unwrap_or_default();
        assert!(
            stdout.contains("SCHEDULER_TEST_VAR=hello_from_test"),
            "stdout should contain env var: {stdout}"
        );
    }

    #[tokio::test]
    async fn test_schedule_job_native_stderr_output() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        // sh -c 'echo "stderr message" >&2' writes to stderr
        let job = make_universal_job(UniversalJobType::Native {
            executable: "sh".to_string(),
            args: vec!["-c".to_string(), "echo 'stderr_message' >&2".to_string()],
            env: HashMap::new(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Success);
        let stderr = response.output.stderr.unwrap_or_default();
        assert!(
            stderr.contains("stderr_message"),
            "stderr should contain message: {stderr}"
        );
    }

    #[tokio::test]
    async fn test_schedule_job_native_failed_process_nonzero_exit() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        // `false` exits with code 1
        let job = make_universal_job(UniversalJobType::Native {
            executable: "false".to_string(),
            args: vec![],
            env: HashMap::new(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(
            matches!(response.status, ExecutionStatus::Failed { .. }),
            "status should be Failed for nonzero exit"
        );
        assert_eq!(response.output.exit_code, Some(1));
    }

    #[tokio::test]
    async fn test_schedule_job_native_exit_code_42() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        let job = make_universal_job(UniversalJobType::Native {
            executable: "sh".to_string(),
            args: vec!["-c".to_string(), "exit 42".to_string()],
            env: HashMap::new(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(matches!(response.status, ExecutionStatus::Failed { .. }));
        assert_eq!(response.output.exit_code, Some(42));
    }

    #[tokio::test]
    async fn test_schedule_multiple_concurrent_native_jobs() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = Arc::new(UniversalScheduler::new(registry).await.unwrap());

        let mut handles = vec![];
        for i in 0..5 {
            let s = Arc::clone(&scheduler);
            let job = make_universal_job(UniversalJobType::Native {
                executable: "echo".to_string(),
                args: vec![format!("job_{}", i)],
                env: HashMap::new(),
            });
            handles.push(tokio::spawn(async move { s.schedule_job(job).await }));
        }

        let mut results = Vec::with_capacity(5);
        for h in handles {
            results.push(h.await.expect("task panicked"));
        }

        for (i, result) in results.into_iter().enumerate() {
            assert!(result.is_ok(), "job {} should succeed", i);
            let response = result.unwrap();
            assert_eq!(response.status, ExecutionStatus::Success);
            assert_eq!(
                response.output.stdout.as_deref(),
                Some(format!("job_{}\n", i).as_str())
            );
        }
    }

    #[tokio::test]
    async fn test_schedule_job_native_stdout_and_stderr() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        let job = make_universal_job(UniversalJobType::Native {
            executable: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "echo 'stdout_line'; echo 'stderr_line' >&2".to_string(),
            ],
            env: HashMap::new(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Success);
        assert!(response
            .output
            .stdout
            .unwrap_or_default()
            .contains("stdout_line"));
        assert!(response
            .output
            .stderr
            .unwrap_or_default()
            .contains("stderr_line"));
    }

    #[tokio::test]
    async fn test_schedule_job_native_with_args() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        let job = make_universal_job(UniversalJobType::Native {
            executable: "echo".to_string(),
            args: vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()],
            env: HashMap::new(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Success);
        assert!(response
            .output
            .stdout
            .unwrap_or_default()
            .contains("arg1 arg2 arg3"));
    }

    #[tokio::test]
    async fn test_schedule_job_native_true_succeeds() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        let job = make_universal_job(UniversalJobType::Native {
            executable: "true".to_string(),
            args: vec![],
            env: HashMap::new(),
        });

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Success);
        assert_eq!(response.output.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_schedule_job_active_count_after_completion() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        let job = make_universal_job(UniversalJobType::Native {
            executable: "echo".to_string(),
            args: vec!["done".to_string()],
            env: HashMap::new(),
        });

        let _ = scheduler.schedule_job(job).await;
        let count = scheduler.get_active_job_count().await;
        assert_eq!(count, 0, "active jobs should be 0 after completion");
    }

    #[tokio::test]
    async fn test_register_runtime_engine_replaces_existing() {
        let registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = UniversalScheduler::new(registry).await.unwrap();

        scheduler
            .register_runtime_engine(RuntimeType::Native, Box::new(SimpleMockRuntimeEngine))
            .await;
        scheduler
            .register_runtime_engine(RuntimeType::Native, Box::new(SimpleMockRuntimeEngine))
            .await;

        let runtimes = scheduler.available_runtimes().await;
        assert_eq!(runtimes.len(), 1, "should replace not duplicate");
    }
}
