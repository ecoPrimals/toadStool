// SPDX-License-Identifier: AGPL-3.0-or-later
//! BiomeOS manifest execution (`execute_biome_os`).

use std::collections::HashMap;

use bytes::Bytes;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::ToadStoolResult;
use crate::execution::{ExecutionResponse, RuntimeEngine};

use super::super::UniversalScheduler;
use super::discover::discover_self_ip_address;
use crate::universal::requests::{PrimalRequest, ResponseStatus};
use crate::universal::traits::UniversalPrimalProvider;
use crate::universal::types::{NetworkLocation, PrimalContext, SecurityLevel};

impl<P, E: RuntimeEngine> UniversalScheduler<P, E>
where
    P: UniversalPrimalProvider + Send + Sync + 'static,
{
    /// Execute a BiomeOS job
    pub(in crate::universal::scheduler) async fn execute_biome_os(
        &self,
        biome_manifest: &serde_json::Value,
        team_id: &str,
    ) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing BiomeOS job for team: {}", team_id);
        let execution_id = crate::generate_uuid();
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
                        crate::execution::ExecutionStatus::Failed {
                            error: error_msg.into(),
                        }
                    };

                    Ok(ExecutionResponse {
                        execution_id,
                        status,
                        output: crate::execution::ExecutionOutput {
                            data: Bytes::from(
                                serde_json::to_vec(&response.payload).unwrap_or_default(),
                            ),
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
                    Ok(ExecutionResponse {
                        execution_id,
                        status: crate::execution::ExecutionStatus::Failed {
                            error: error_msg.clone().into(),
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
                "BiomeOS integration not available: no BiomeOS primal provider registered for team '{team_id}'"
            );
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
                duration: start_time.elapsed(),
                runtime_used: crate::execution::RuntimeType::Native,
                warnings: vec![
                    "BiomeOS execution requires a registered BiomeOS primal provider".to_string(),
                ],
            })
        }
    }
}
