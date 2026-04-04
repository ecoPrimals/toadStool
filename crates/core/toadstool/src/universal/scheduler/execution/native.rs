// SPDX-License-Identifier: AGPL-3.0-only
//! Native binary / script execution (`execute_native`).

use std::collections::HashMap;
use std::time::Duration;

use bytes::Bytes;
use toadstool_common::constants::PRIMAL_NAME;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::execution::{ExecutionInput, ExecutionRequest, ExecutionResponse, RuntimeType};
use crate::resources::ResourceRequirements;
use crate::workload::ExecutableSource;
use crate::{SecurityContext, ToadStoolResult, WorkloadSpec};

use super::super::UniversalScheduler;
use super::discover::discover_self_ip_address;
use crate::universal::requests::{PrimalRequest, ResponseStatus};
use crate::universal::types::{NetworkLocation, PrimalCapability, PrimalContext, SecurityLevel};

impl UniversalScheduler {
    /// Execute a native job (binary/script)
    #[expect(
        clippy::cast_possible_truncation,
        clippy::significant_drop_tightening,
        reason = "truncation acceptable for this conversion; drop order is intentional"
    )]
    pub(in crate::universal::scheduler) async fn execute_native(
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
                        crate::execution::ExecutionStatus::Failed {
                            error: message.into(),
                        }
                    }
                    ResponseStatus::Timeout => crate::execution::ExecutionStatus::TimedOut,
                    ResponseStatus::ServiceUnavailable => {
                        crate::execution::ExecutionStatus::Failed {
                            error: std::borrow::Cow::Borrowed("Service unavailable"),
                        }
                    }
                },
                output: crate::execution::ExecutionOutput {
                    data: Bytes::from(serde_json::to_vec(&response.payload).unwrap_or_default()),
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
                            error: format!("process exited with code {exit_code}").into(),
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
                    Ok(ExecutionResponse {
                        execution_id,
                        status: crate::execution::ExecutionStatus::Failed {
                            error: error_msg.clone().into(),
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
}
