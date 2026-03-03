// SPDX-License-Identifier: AGPL-3.0-or-later
//! Job execution backends for UniversalScheduler
//!
//! Handles execution routing for Native, WASM, Primal, and BiomeOS job types.

use std::collections::HashMap;
use std::time::Duration;

use toadstool_config::defaults::network::BIND_ADDRESS_DEFAULT;
use tracing::{debug, info, warn};

/// Discovers the primal's own IP/host for `PrimalContext.network_location`.
///
/// Resolution order: `TOADSTOOL_BIND_ADDRESS` (host part) → `TOADSTOOL_BIND_HOST` →
/// `BIND_HOST` → `HOST` → `HOSTNAME` → `0.0.0.0` (any interface).
///
/// This is the self-discovery default when no explicit bind address is configured.
#[must_use]
fn discover_self_ip_address() -> String {
    // 1. TOADSTOOL_BIND_ADDRESS (host:port) — extract host
    if let Ok(addr) = std::env::var("TOADSTOOL_BIND_ADDRESS") {
        let host = addr.split(':').next().unwrap_or(&addr).trim();
        if !host.is_empty() {
            return host.to_string();
        }
    }
    // 2. TOADSTOOL_BIND_HOST
    if let Ok(h) = std::env::var("TOADSTOOL_BIND_HOST") {
        if !h.is_empty() {
            return h;
        }
    }
    // 3. BIND_HOST
    if let Ok(h) = std::env::var("BIND_HOST") {
        if !h.is_empty() {
            return h;
        }
    }
    // 4. HOST
    if let Ok(h) = std::env::var("HOST") {
        if !h.is_empty() {
            return h;
        }
    }
    // 5. HOSTNAME
    if let Ok(h) = std::env::var("HOSTNAME") {
        if !h.is_empty() {
            return h;
        }
    }
    // 6. Fallback: any interface (not loopback-only)
    BIND_ADDRESS_DEFAULT.to_string()
}
use uuid::Uuid;

use toadstool_common::constants::PRIMAL_NAME;

use crate::execution::{ExecutionInput, ExecutionRequest, ExecutionResponse, RuntimeType};
use crate::resources::ResourceRequirements;
use crate::workload::{ExecutableSource, WasiConfig, WasmModuleSource};
use crate::{SecurityContext, ToadStoolResult, WorkloadSpec};

use super::UniversalScheduler;
use crate::universal::requests::{PrimalRequest, ResponseStatus};
use crate::universal::types::{NetworkLocation, PrimalCapability, PrimalContext, SecurityLevel};

impl UniversalScheduler {
    /// Execute a native job (binary/script)
    #[allow(clippy::cast_possible_truncation)]
    pub(super) async fn execute_native(
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
            .primal_registry()
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
                        ip_address: discover_self_ip_address(),
                        subnet: None,
                        network_id: None,
                        geo_location: None,
                    },
                    security_level: SecurityLevel::Standard,
                    metadata: HashMap::new(),
                },
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
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
                        .map(|i| i as i32), // i64 to i32 truncation acceptable for exit codes
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
            let engines = self.runtime_engines().read().await;
            if let Some(native_engine) = engines.get(&RuntimeType::Native) {
                info!("Using local native runtime engine for execution");

                // Build execution request (clone env once, reuse for both fields)
                let env_owned = env.clone();
                let request = ExecutionRequest {
                    execution_id: Uuid::new_v4(),
                    workload: WorkloadSpec::Native {
                        executable: ExecutableSource::File {
                            path: std::path::PathBuf::from(executable),
                        },
                        args: Some(args.to_vec()),
                        working_dir: None,
                        env_vars: env_owned.clone(),
                        user: None,
                    },
                    runtime_hint: Some(RuntimeType::Native),
                    resources: ResourceRequirements::default(),
                    security_context: SecurityContext::default(),
                    timeout: Some(Duration::from_secs(300)),
                    environment: env_owned,
                    input_data: ExecutionInput::default(),
                    callback_config: None,
                    encryption_config: None,
                };

                return native_engine.execute(request).await;
            }

            // No primal provider and no registered runtime engine.
            // Sovereign fallback: attempt direct local process execution.
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
                    let error_msg = format!("Failed to spawn '{executable}': {e}");
                    warn!("{}", error_msg);
                    let error_for_stderr = error_msg.clone();
                    Ok(ExecutionResponse {
                        execution_id,
                        status: crate::execution::ExecutionStatus::Failed { error: error_msg },
                        output: crate::execution::ExecutionOutput {
                            data: bytes::Bytes::new(),
                            stdout: None,
                            stderr: Some(error_for_stderr),
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

    /// Execute a WASM job
    pub(super) async fn execute_wasm(
        &self,
        module: &[u8],
        args: &[String],
        env: &HashMap<String, String>,
    ) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing WASM job ({} bytes)", module.len());

        // Check if we have a WASM runtime engine registered
        let engines = self.runtime_engines().read().await;
        if let Some(wasm_engine) = engines.get(&RuntimeType::Wasm) {
            info!("Using registered WASM runtime engine for execution");

            // Build execution request (clone env once, reuse for both fields)
            let env_owned = env.clone();
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
                    env_vars: env_owned.clone(),
                },
                runtime_hint: Some(RuntimeType::Wasm),
                resources: ResourceRequirements::default(),
                security_context: SecurityContext::default(),
                timeout: Some(Duration::from_secs(300)),
                environment: env_owned,
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
        let error_for_stderr = error_msg.clone();
        Ok(ExecutionResponse {
            execution_id: Uuid::new_v4(),
            status: crate::execution::ExecutionStatus::Failed {
                error: error_msg,
            },
            output: crate::execution::ExecutionOutput {
                data: bytes::Bytes::new(),
                stdout: None,
                stderr: Some(error_for_stderr),
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

    /// Execute a primal job (remote capability)
    pub(super) async fn execute_primal(
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
        let providers = self.primal_registry().get_all_providers().await;
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
                        ip_address: discover_self_ip_address(),
                        subnet: None,
                        network_id: None,
                        geo_location: None,
                    },
                    security_level: SecurityLevel::Standard,
                    metadata: HashMap::new(),
                },
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            };

            match self.primal_registry().route_request(request).await {
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
                            stdout: Some(format!("Primal '{primal_type}' executed successfully")),
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
                    let error_msg = format!("Primal '{primal_type}' execution failed: {e}");
                    warn!("{}", error_msg);
                    let error_for_stderr = error_msg.clone();
                    Ok(ExecutionResponse {
                        execution_id,
                        status: crate::execution::ExecutionStatus::Failed { error: error_msg },
                        output: crate::execution::ExecutionOutput {
                            data: bytes::Bytes::new(),
                            stdout: None,
                            stderr: Some(error_for_stderr),
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
            let error_for_stderr = error_msg.clone();
            Ok(ExecutionResponse {
                execution_id,
                status: crate::execution::ExecutionStatus::Failed { error: error_msg },
                output: crate::execution::ExecutionOutput {
                    data: bytes::Bytes::new(),
                    stdout: None,
                    stderr: Some(error_for_stderr),
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

    /// Execute a BiomeOS job
    pub(super) async fn execute_biome_os(
        &self,
        biome_manifest: &serde_json::Value,
        team_id: &str,
    ) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing BiomeOS job for team: {}", team_id);
        let execution_id = Uuid::new_v4();
        let start_time = std::time::Instant::now();

        // BiomeOS integration: Look for a BiomeOS primal provider
        let providers = self.primal_registry().get_all_providers().await;
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
                        ip_address: discover_self_ip_address(),
                        subnet: None,
                        network_id: None,
                        geo_location: None,
                    },
                    security_level: SecurityLevel::Standard,
                    metadata: HashMap::new(),
                },
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            };

            match self.primal_registry().route_request(request).await {
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
                                "BiomeOS execution for team '{team_id}' completed"
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
                    let error_msg = format!("BiomeOS execution failed for team '{team_id}': {e}");
                    warn!("{}", error_msg);
                    let error_for_stderr = error_msg.clone();
                    Ok(ExecutionResponse {
                        execution_id,
                        status: crate::execution::ExecutionStatus::Failed { error: error_msg },
                        output: crate::execution::ExecutionOutput {
                            data: bytes::Bytes::new(),
                            stdout: None,
                            stderr: Some(error_for_stderr),
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
                "BiomeOS integration not available: no BiomeOS primal provider registered for team '{team_id}'"
            );
            warn!("{}", error_msg);
            let error_for_stderr = error_msg.clone();
            Ok(ExecutionResponse {
                execution_id,
                status: crate::execution::ExecutionStatus::Failed { error: error_msg },
                output: crate::execution::ExecutionOutput {
                    data: bytes::Bytes::new(),
                    stdout: None,
                    stderr: Some(error_for_stderr),
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
